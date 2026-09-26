//! Wake-style Insights aggregation over scanned [`UnifiedMessage`]s.
//!
//! Activity unit for heatmap / hourly / trend / week comparison:
//! prefer `is_turn_start` messages; if a client has zero turn-starts in the
//! whole set, fall back to counting every message for that client (so heatmaps
//! are not empty). UI labels these as "请求" (requests), not prompts.

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Days, NaiveDate};

use crate::bucket_timezone;
use crate::sessions::UnifiedMessage;

/// Trend / heatmap window: 52 full weeks + the current week.
pub const TREND_WEEKS: usize = 53;

/// Named agent series kept before collapsing the rest into "其他".
const TREND_TOP: usize = 6;

/// Full Insights snapshot computed from message scan.
#[derive(Debug, Clone)]
pub struct InsightsSnapshot {
    pub as_of: NaiveDate,
    pub sessions: i64,
    /// Activity unit (turn-starts, or all messages for clients without them).
    pub requests: i64,
    pub tokens: i64,
    pub agent_count: i64,
    pub project_count: i64,
    /// Earliest message timestamp (ms); 0 if none.
    pub first_ts: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    /// Ascending (day, activity count) — only days with activity ≤ as_of.
    pub daily: Vec<(NaiveDate, i64)>,
    /// Sessions by first-seen day (ascending).
    pub daily_sessions: Vec<(NaiveDate, i64)>,
    pub hourly: [i64; 24],
    /// Monday-first weekday distribution.
    pub weekday: [i64; 7],
    pub monthly: [i64; 12],
    /// Top agents by request total (+ optional "其他"), weekly length [`TREND_WEEKS`].
    pub trend_agents: Vec<TrendSeries>,
    pub agents: Vec<UsageTally>,
    pub models: Vec<UsageTally>,
    pub projects: Vec<UsageTally>,
}

impl Default for InsightsSnapshot {
    fn default() -> Self {
        Self {
            as_of: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            sessions: 0,
            requests: 0,
            tokens: 0,
            agent_count: 0,
            project_count: 0,
            first_ts: 0,
            current_streak: 0,
            longest_streak: 0,
            daily: Vec::new(),
            daily_sessions: Vec::new(),
            hourly: [0; 24],
            weekday: [0; 7],
            monthly: [0; 12],
            trend_agents: Vec::new(),
            agents: Vec::new(),
            models: Vec::new(),
            projects: Vec::new(),
        }
    }
}

/// Time-window metrics (Last 7 days vs previous 7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowStats {
    pub sessions: i64,
    pub requests: i64,
    pub active_days: i64,
}

/// One agent (or "其他") weekly request series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendSeries {
    pub name: String,
    pub weekly: Vec<i64>,
}

impl TrendSeries {
    pub fn total(&self) -> i64 {
        self.weekly.iter().sum()
    }
}

/// Agent / model / project usage row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageTally {
    pub name: String,
    pub sessions: i64,
    pub requests: i64,
    pub tokens: i64,
}

impl InsightsSnapshot {
    /// Active days = length of [`Self::daily`].
    pub fn active_days(&self) -> i64 {
        self.daily.len() as i64
    }

    pub fn busiest_day(&self) -> Option<(NaiveDate, i64)> {
        self.daily.iter().max_by_key(|(_, n)| *n).copied()
    }

    /// Metrics for the closed 7-day window ending on `ending`.
    pub fn week_ending(&self, ending: NaiveDate) -> WindowStats {
        let start = week_window_start(ending);
        let mut w = WindowStats::default();
        let from = self.daily.partition_point(|(d, _)| *d < start);
        for (_, n) in self.daily[from..].iter().take_while(|(d, _)| *d <= ending) {
            w.requests += n;
            w.active_days += 1;
        }
        let from = self.daily_sessions.partition_point(|(d, _)| *d < start);
        for (_, sessions) in self.daily_sessions[from..]
            .iter()
            .take_while(|(d, _)| *d <= ending)
        {
            w.sessions += sessions;
        }
        w
    }

    /// Recent 7 days and the 7 days before that.
    pub fn last_week_pair(&self) -> (WindowStats, WindowStats) {
        (
            self.week_ending(self.as_of),
            self.week_ending(self.as_of - Days::new(7)),
        )
    }

    /// First Monday of the trend / heatmap window (last column = as_of's week).
    pub fn trend_start(&self) -> NaiveDate {
        week_start(self.as_of) - Days::new((TREND_WEEKS as u64 - 1) * 7)
    }
}

/// Week start = Monday.
pub fn week_start(day: NaiveDate) -> NaiveDate {
    day - Days::new(day.weekday().num_days_from_monday() as u64)
}

/// First day of a closed 7-day window ending on `ending`.
pub fn week_window_start(ending: NaiveDate) -> NaiveDate {
    ending - Days::new(6)
}

/// Index into the trend window (last column = as_of's week); outside → None.
pub fn trend_week_index(as_of: NaiveDate, day: NaiveDate) -> Option<usize> {
    let weeks_back = (week_start(as_of) - week_start(day)).num_days() / 7;
    (0..TREND_WEEKS as i64)
        .contains(&weeks_back)
        .then(|| TREND_WEEKS - 1 - weeks_back as usize)
}

fn parse_day(date_str: &str, as_of: NaiveDate) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .filter(|d| *d <= as_of)
}

fn hour_of_ms(timestamp_ms: i64) -> Option<usize> {
    let formatted = bucket_timezone().date_hour_of_ms(timestamp_ms)?;
    // "YYYY-MM-DD HH:00"
    formatted
        .get(11..13)?
        .parse::<usize>()
        .ok()
        .filter(|h| *h < 24)
}

fn compute_streaks(daily: &[(NaiveDate, i64)], today: NaiveDate) -> (i64, i64) {
    let mut longest = 0i64;
    let mut run = 0i64;
    let mut prev: Option<NaiveDate> = None;
    for &(d, _) in daily {
        run = match prev {
            Some(p) if (d - p).num_days() == 1 => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        prev = Some(d);
    }
    // GitHub / Wake: if today is empty, current streak may still include yesterday.
    let current = match prev {
        Some(last) if (today - last).num_days() <= 1 => run,
        _ => 0,
    };
    (current, longest)
}

/// Clients with zero `is_turn_start` messages fall back to counting all messages.
fn clients_without_turn_starts(messages: &[UnifiedMessage]) -> HashSet<String> {
    let mut has_turn = HashSet::new();
    let mut all_clients = HashSet::new();
    for m in messages {
        all_clients.insert(m.client.clone());
        if m.is_turn_start {
            has_turn.insert(m.client.clone());
        }
    }
    all_clients
        .into_iter()
        .filter(|c| !has_turn.contains(c))
        .collect()
}

fn counts_as_request(m: &UnifiedMessage, fallback_all: &HashSet<String>) -> bool {
    if fallback_all.contains(&m.client) {
        true
    } else {
        m.is_turn_start
    }
}

/// Aggregate Insights over a message slice.
pub fn compute_insights(messages: &[UnifiedMessage], as_of: NaiveDate) -> InsightsSnapshot {
    let fallback_all = clients_without_turn_starts(messages);

    let mut sessions: HashSet<(String, String)> = HashSet::new();
    let mut projects: HashSet<String> = HashSet::new();
    let mut agents: HashSet<String> = HashSet::new();
    let mut first_ts = 0i64;
    let mut tokens = 0i64;
    let mut requests = 0i64;
    let mut hourly = [0i64; 24];
    let mut weekday = [0i64; 7];
    let mut monthly = [0i64; 12];
    let mut daily_map: HashMap<NaiveDate, i64> = HashMap::new();

    // Per-entity tallies
    struct Acc {
        sessions: HashSet<String>,
        requests: i64,
        tokens: i64,
    }
    impl Acc {
        fn new() -> Self {
            Self {
                sessions: HashSet::new(),
                requests: 0,
                tokens: 0,
            }
        }
    }
    let mut by_agent: HashMap<String, Acc> = HashMap::new();
    let mut by_model: HashMap<String, Acc> = HashMap::new();
    let mut by_project: HashMap<String, Acc> = HashMap::new();
    let mut weekly_by_agent: HashMap<String, Vec<i64>> = HashMap::new();

    // Session first-seen day: (client, session_id) → earliest day
    let mut session_first: HashMap<(String, String), NaiveDate> = HashMap::new();

    for m in messages {
        if !m.session_id.is_empty() {
            sessions.insert((m.client.clone(), m.session_id.clone()));
            let key = (m.client.clone(), m.session_id.clone());
            if let Some(day) = parse_day(&m.date, as_of) {
                session_first
                    .entry(key)
                    .and_modify(|d| {
                        if day < *d {
                            *d = day;
                        }
                    })
                    .or_insert(day);
            }
        }
        if let Some(wk) = m.workspace_key.as_deref().filter(|s| !s.is_empty()) {
            projects.insert(wk.to_string());
        }
        agents.insert(m.client.clone());

        if m.timestamp > 0 && (first_ts == 0 || m.timestamp < first_ts) {
            first_ts = m.timestamp;
        }

        let tok = m.tokens.total();
        tokens = tokens.saturating_add(tok);

        let is_req = counts_as_request(m, &fallback_all);
        if is_req {
            requests += 1;
        }

        let agent_acc = by_agent.entry(m.client.clone()).or_insert_with(Acc::new);
        if !m.session_id.is_empty() {
            agent_acc.sessions.insert(m.session_id.clone());
        }
        agent_acc.tokens = agent_acc.tokens.saturating_add(tok);
        if is_req {
            agent_acc.requests += 1;
        }

        if !m.model_id.is_empty() {
            let model_acc = by_model.entry(m.model_id.clone()).or_insert_with(Acc::new);
            if !m.session_id.is_empty() {
                model_acc.sessions.insert(m.session_id.clone());
            }
            model_acc.tokens = model_acc.tokens.saturating_add(tok);
            if is_req {
                model_acc.requests += 1;
            }
        }

        if let Some(wk) = m.workspace_key.as_deref().filter(|s| !s.is_empty()) {
            let label = m
                .workspace_label
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(wk);
            let proj_acc = by_project.entry(label.to_string()).or_insert_with(Acc::new);
            if !m.session_id.is_empty() {
                proj_acc
                    .sessions
                    .insert(format!("{}:{}", m.client, m.session_id));
            }
            proj_acc.tokens = proj_acc.tokens.saturating_add(tok);
            if is_req {
                proj_acc.requests += 1;
            }
        }

        if !is_req {
            continue;
        }

        let Some(day) = parse_day(&m.date, as_of) else {
            continue;
        };

        *daily_map.entry(day).or_default() += 1;
        weekday[day.weekday().num_days_from_monday() as usize] += 1;
        monthly[day.month0() as usize] += 1;
        if let Some(h) = hour_of_ms(m.timestamp) {
            hourly[h] += 1;
        }

        if let Some(ix) = trend_week_index(as_of, day) {
            weekly_by_agent
                .entry(m.client.clone())
                .or_insert_with(|| vec![0; TREND_WEEKS])[ix] += 1;
        }
    }

    let mut daily: Vec<(NaiveDate, i64)> = daily_map.into_iter().collect();
    daily.sort_by_key(|(d, _)| *d);

    let mut daily_sessions_map: HashMap<NaiveDate, i64> = HashMap::new();
    for day in session_first.values() {
        *daily_sessions_map.entry(*day).or_default() += 1;
    }
    let mut daily_sessions: Vec<(NaiveDate, i64)> = daily_sessions_map.into_iter().collect();
    daily_sessions.sort_by_key(|(d, _)| *d);

    let to_tallies = |map: HashMap<String, Acc>| -> Vec<UsageTally> {
        let mut rows: Vec<UsageTally> = map
            .into_iter()
            .map(|(name, acc)| UsageTally {
                name,
                sessions: acc.sessions.len() as i64,
                requests: acc.requests,
                tokens: acc.tokens,
            })
            .collect();
        rows.sort_by(|a, b| {
            b.sessions
                .cmp(&a.sessions)
                .then_with(|| b.requests.cmp(&a.requests))
                .then_with(|| a.name.cmp(&b.name))
        });
        rows
    };

    let mut trend_series: Vec<TrendSeries> = weekly_by_agent
        .into_iter()
        .map(|(name, weekly)| TrendSeries { name, weekly })
        .collect();
    trend_series.sort_by(|a, b| b.total().cmp(&a.total()).then_with(|| a.name.cmp(&b.name)));

    let trend_agents = if trend_series.len() <= TREND_TOP {
        trend_series
    } else {
        let mut kept: Vec<TrendSeries> = trend_series.drain(..TREND_TOP).collect();
        let mut other = vec![0i64; TREND_WEEKS];
        for s in &trend_series {
            for (o, n) in other.iter_mut().zip(&s.weekly) {
                *o += n;
            }
        }
        if other.iter().any(|&n| n > 0) {
            kept.push(TrendSeries {
                name: "Other".to_string(),
                weekly: other,
            });
        }
        kept
    };

    let (current_streak, longest_streak) = compute_streaks(&daily, as_of);

    InsightsSnapshot {
        as_of,
        sessions: sessions.len() as i64,
        requests,
        tokens,
        agent_count: agents.len() as i64,
        project_count: projects.len() as i64,
        first_ts,
        current_streak,
        longest_streak,
        daily,
        daily_sessions,
        hourly,
        weekday,
        monthly,
        trend_agents,
        agents: to_tallies(by_agent),
        models: to_tallies(by_model),
        projects: to_tallies(by_project),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenBreakdown;

    fn msg(
        client: &str,
        session: &str,
        day: NaiveDate,
        hour: u32,
        turn_start: bool,
        tokens: i64,
    ) -> UnifiedMessage {
        use chrono::TimeZone;
        let ts = chrono::Local
            .from_local_datetime(&day.and_hms_opt(hour, 0, 0).unwrap())
            .single()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);
        // Override date so bucketing tests stay stable even if Local parse fails.
        let mut m = UnifiedMessage::new(
            client,
            "model-a",
            "provider",
            session,
            ts,
            TokenBreakdown {
                input: tokens,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                reasoning: 0,
            },
            0.0,
        );
        m.date = day.format("%Y-%m-%d").to_string();
        m.is_turn_start = turn_start;
        m.workspace_key = Some("/proj/demo".into());
        m.workspace_label = Some("demo".into());
        m
    }

    #[test]
    fn week_helpers_align() {
        let wed = NaiveDate::from_ymd_opt(2026, 1, 14).unwrap(); // Wednesday
        assert_eq!(
            week_start(wed),
            NaiveDate::from_ymd_opt(2026, 1, 12).unwrap()
        );
        assert_eq!(
            week_window_start(wed),
            NaiveDate::from_ymd_opt(2026, 1, 8).unwrap()
        );
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 14).unwrap();
        let same_week = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        assert_eq!(trend_week_index(as_of, same_week), Some(TREND_WEEKS - 1));
        let prev = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert_eq!(trend_week_index(as_of, prev), Some(TREND_WEEKS - 2));
    }

    #[test]
    fn buckets_turn_starts_and_fallback() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 13).unwrap();
        let d10 = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let d11 = NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();
        let d12 = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();

        // Claude: turn starts only
        let mut messages = vec![
            msg("claude", "s1", d10, 9, true, 100),
            msg("claude", "s1", d10, 9, false, 50), // not a turn
            msg("claude", "s1", d11, 14, true, 100),
            msg("claude", "s1", d12, 22, true, 100),
        ];
        // Codex: no turn starts → count all messages
        messages.push(msg("codex", "s2", d11, 10, false, 200));
        messages.push(msg("codex", "s2", d11, 11, false, 200));

        let snap = compute_insights(&messages, as_of);
        assert_eq!(snap.sessions, 2);
        assert_eq!(snap.requests, 5); // 3 claude turns + 2 codex msgs
        assert_eq!(snap.tokens, 750);
        assert_eq!(snap.agent_count, 2);
        assert_eq!(snap.project_count, 1);
        assert_eq!(snap.active_days(), 3);
        assert_eq!(snap.busiest_day(), Some((d11, 3))); // 1 claude + 2 codex
        assert_eq!((snap.current_streak, snap.longest_streak), (3, 3));
        assert!(snap.hourly.iter().sum::<i64>() >= 3);

        let (cur, _prev) = snap.last_week_pair();
        assert_eq!(cur.requests, 5);
        assert_eq!(cur.sessions, 2);
        assert_eq!(cur.active_days, 3);

        assert_eq!(snap.trend_start(), week_start(as_of) - Days::new(52 * 7));
    }

    #[test]
    fn daily_sessions_use_first_seen_day() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let d10 = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let d15 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let messages = vec![
            msg("claude", "s1", d10, 9, true, 10),
            msg("claude", "s1", d15, 9, true, 10), // same session, later day
            msg("claude", "s2", d15, 10, true, 10),
        ];
        let snap = compute_insights(&messages, as_of);
        assert_eq!(
            snap.daily_sessions,
            vec![(d10, 1), (d15, 1)],
            "s1 counted on first day only"
        );
    }
}
