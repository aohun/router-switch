//! Wake-style Insights / 统计 dashboard (pure GPUI div charts, no canvas).

use chrono::{Datelike, Days, NaiveDate};
use gpui::{
    div, prelude::FluentBuilder, px, rgb, AnyElement, App, Context, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    tooltip::Tooltip,
    v_flex, ActiveTheme, IconName, Sizable as _, WindowExt,
};
use rust_i18n::t;
use std::rc::Rc;
use tokens_core::{InsightsSnapshot, TrendSeries, TREND_WEEKS};

use crate::app_view::RouterApp;
use crate::assets::{brand_img, CustomIcon};
use crate::usage_service::format_tokens;

const WEEK_CELL: f32 = 9.;
const WEEK_GAP: f32 = 3.;
const WEEK_STEP: f32 = WEEK_CELL + WEEK_GAP;
const DOW_W: f32 = 26.;
const HEAT: [f32; 4] = [0.25, 0.5, 0.75, 1.];
const RADIUS_CELL: f32 = 2.;
const FONT_LABEL: f32 = 11.;
const FONT_CAPTION: f32 = 12.;
const FONT_TITLE: f32 = 22.;
const FONT_HEADING: f32 = 16.;
const SECTION_GAP: f32 = 28.;
const TREND_TOP: usize = 6;

/// Distribution chart dimension (‹ › cycle).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum InsightsRange {
    #[default]
    Hour,
    Weekday,
    Month,
}

impl InsightsRange {
    pub fn prev(self) -> Self {
        match self {
            Self::Hour => Self::Month,
            Self::Weekday => Self::Hour,
            Self::Month => Self::Weekday,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Hour => Self::Weekday,
            Self::Weekday => Self::Month,
            Self::Month => Self::Hour,
        }
    }

    fn title_key(self) -> &'static str {
        match self {
            Self::Hour => "stats.by_hour",
            Self::Weekday => "stats.by_weekday",
            Self::Month => "stats.by_month",
        }
    }
}

fn thousands(n: i64) -> String {
    let n = n.max(0);
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn month_year(ts_ms: i64) -> String {
    if ts_ms <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| {
            if rust_i18n::locale().starts_with("zh") {
                format!("{}年{}月", dt.year(), dt.month())
            } else {
                dt.format("%b %Y").to_string()
            }
        })
        .unwrap_or_default()
}

/// Wake Insights chart accent (macOS system blue). Page-local only — does not
/// touch the app-wide theme palette (RS primary stays neutral black/gray).
fn chart_accent(cx: &App) -> Hsla {
    let dark = cx.theme().is_dark();
    // Wake theme.rs: light primary 0x0A84FF, dark primary 0x4C8DFF
    rgb(if dark { 0x4C8DFF } else { 0x0A84FF }).into()
}

/// Wake `theme::agent_series_color` — brand hues for the stacked trend chart.
fn agent_series_color(client: &str) -> Hsla {
    let c = client.to_lowercase();
    let hex = if c.contains("claude") {
        0xD97757
    } else if c.contains("codex") {
        0x7A8DFF
    } else if c.contains("cursor") {
        0x2DB6C8
    } else if c.contains("grok") {
        0x8A7F73
    } else if c.contains("opencode") {
        0xC98A2E
    } else if c == "pi" || c.starts_with("pi-") {
        0x3AAE8C
    } else if c.contains("gemini") {
        0x3B8BD9
    } else if c.contains("kimi") {
        0xE4739E
    } else if c.contains("hermes") {
        0xE0B040
    } else if c.contains("openclaw") {
        0xE04A4A
    } else if c.contains("zcode") {
        0x8B95A5
    } else if c.contains("codebuddy") {
        0x6C4DFF
    } else if c.contains("workbuddy") {
        0x0EC8A9
    } else if c.contains("antigravity") {
        0x648AB5
    } else if c.contains("qoder") {
        0x2BB454
    } else if c.contains("deepseek") || c == "dsh" {
        0x4D6BFE
    } else if c.contains("omp") {
        0xB05CE6
    } else {
        0x4C8DFF // Wake SERIES_FALLBACK
    };
    rgb(hex).into()
}

fn display_agent_name(raw: &str) -> SharedString {
    if raw == "Other" {
        return t!("stats.other").to_string().into();
    }
    let lower = raw.to_lowercase();
    let label = match lower.as_str() {
        "claude" | "claude-code" => "Claude Code",
        "codex" => "Codex",
        "cursor" => "Cursor",
        "grok" => "Grok",
        "opencode" => "OpenCode",
        "pi" => "Pi",
        "zcode" => "ZCode",
        "workbuddy" => "WorkBuddy",
        "codebuddy" => "CodeBuddy",
        "gemini" => "Gemini",
        "kimi" => "Kimi",
        "hermes" => "Hermes",
        "openclaw" => "OpenClaw",
        "antigravity" => "Antigravity",
        "qoder" => "Qoder",
        "omp" => "Oh My Pi",
        _ => raw,
    };
    label.to_string().into()
}

fn brand_id_for_client(client: &str) -> Option<&'static str> {
    let c = client.to_lowercase();
    if c.contains("claude") {
        Some("claude")
    } else if c.contains("codex") {
        Some("codex")
    } else if c.contains("cursor") {
        Some("cursor")
    } else if c.contains("grok") {
        Some("grok")
    } else if c.contains("opencode") {
        Some("opencode")
    } else if c == "pi" || c.starts_with("pi-") {
        Some("pi")
    } else if c.contains("zcode") {
        Some("zcode")
    } else if c.contains("workbuddy") {
        Some("workbuddy")
    } else if c.contains("codebuddy") {
        Some("codebuddy")
    } else if c.contains("gemini") {
        Some("gemini")
    } else if c.contains("kimi") {
        Some("kimi")
    } else if c.contains("hermes") {
        Some("hermes")
    } else if c.contains("openclaw") {
        Some("openclaw")
    } else if c.contains("antigravity") {
        Some("antigravity")
    } else if c.contains("qoder") {
        Some("qoder")
    } else if c.contains("omp") {
        Some("omp")
    } else if c.contains("deepseek") {
        Some("deepseek")
    } else {
        None
    }
}

fn requests_label(n: i64) -> String {
    format!("{} {}", thousands(n), t!("stats.requests"))
}

fn dow_short() -> [&'static str; 7] {
    if rust_i18n::locale().starts_with("zh") {
        ["一", "二", "三", "四", "五", "六", "日"]
    } else {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    }
}

fn dow_plural() -> [&'static str; 7] {
    if rust_i18n::locale().starts_with("zh") {
        [
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ]
    } else {
        [
            "Mondays",
            "Tuesdays",
            "Wednesdays",
            "Thursdays",
            "Fridays",
            "Saturdays",
            "Sundays",
        ]
    }
}

fn month_short() -> [SharedString; 12] {
    if rust_i18n::locale().starts_with("zh") {
        [
            "1月".into(),
            "2月".into(),
            "3月".into(),
            "4月".into(),
            "5月".into(),
            "6月".into(),
            "7月".into(),
            "8月".into(),
            "9月".into(),
            "10月".into(),
            "11月".into(),
            "12月".into(),
        ]
    } else {
        [
            "Jan".into(),
            "Feb".into(),
            "Mar".into(),
            "Apr".into(),
            "May".into(),
            "Jun".into(),
            "Jul".into(),
            "Aug".into(),
            "Sep".into(),
            "Oct".into(),
            "Nov".into(),
            "Dec".into(),
        ]
    }
}

fn month_full() -> [SharedString; 12] {
    if rust_i18n::locale().starts_with("zh") {
        [
            "一月".into(),
            "二月".into(),
            "三月".into(),
            "四月".into(),
            "五月".into(),
            "六月".into(),
            "七月".into(),
            "八月".into(),
            "九月".into(),
            "十月".into(),
            "十一月".into(),
            "十二月".into(),
        ]
    } else {
        [
            "January".into(),
            "February".into(),
            "March".into(),
            "April".into(),
            "May".into(),
            "June".into(),
            "July".into(),
            "August".into(),
            "September".into(),
            "October".into(),
            "November".into(),
            "December".into(),
        ]
    }
}

fn hour_label(h: usize) -> String {
    format!("{:02}:00", h % 24)
}

fn format_day_short(day: NaiveDate) -> String {
    if rust_i18n::locale().starts_with("zh") {
        format!("{}月{}日", day.month(), day.day())
    } else {
        day.format("%b %-d").to_string()
    }
}

fn format_day_full(day: NaiveDate) -> String {
    if rust_i18n::locale().starts_with("zh") {
        format!("{}年{}月{}日", day.year(), day.month(), day.day())
    } else {
        day.format("%b %-d, %Y").to_string()
    }
}

fn section_head(
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    trailing: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    h_flex()
        .w_full()
        .items_end()
        .justify_between()
        .gap(px(12.))
        .child(
            v_flex()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title.into()),
                )
                .when_some(subtitle, |col, s| {
                    col.child(
                        div()
                            .text_size(px(FONT_CAPTION))
                            .text_color(theme.muted_foreground)
                            .child(s),
                    )
                }),
        )
        .when_some(trailing, |row, el| row.child(el))
}

fn stat_cell(
    value: String,
    label: impl Into<SharedString>,
    size: f32,
    note: Option<(SharedString, Hsla)>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(size))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(value),
        )
        .child(
            div()
                .text_size(px(FONT_CAPTION))
                .text_color(theme.muted_foreground)
                .child(label.into()),
        )
        .when_some(note, |cell, (text, color)| {
            cell.child(
                div()
                    .text_size(px(FONT_LABEL))
                    .text_color(color)
                    .child(text),
            )
        })
}

struct TrendLayer {
    name: SharedString,
    color: Hsla,
    weekly: Vec<i64>,
    other: bool,
}

fn trend_layers(series: &[TrendSeries], cx: &App) -> Rc<Vec<TrendLayer>> {
    let theme = cx.theme();
    let mut layers: Vec<TrendLayer> = series
        .iter()
        .take(TREND_TOP)
        .filter(|s| s.name != "Other")
        .map(|s| TrendLayer {
            name: display_agent_name(&s.name),
            color: agent_series_color(&s.name),
            weekly: s.weekly.clone(),
            other: false,
        })
        .collect();
    if let Some(other) = series.iter().find(|s| s.name == "Other") {
        layers.push(TrendLayer {
            name: t!("stats.other").to_string().into(),
            color: theme.muted_foreground.opacity(0.35),
            weekly: other.weekly.clone(),
            other: true,
        });
    } else if series.len() > TREND_TOP {
        let mut weekly = vec![0i64; TREND_WEEKS];
        for s in series.iter().skip(TREND_TOP) {
            for (o, n) in weekly.iter_mut().zip(&s.weekly) {
                *o += n;
            }
        }
        if weekly.iter().any(|&n| n > 0) {
            layers.push(TrendLayer {
                name: t!("stats.other").to_string().into(),
                color: theme.muted_foreground.opacity(0.35),
                weekly,
                other: true,
            });
        }
    }
    Rc::new(layers)
}

fn week_change_note(now: i64, before: i64, cx: &App) -> (SharedString, Hsla) {
    let theme = cx.theme();
    let accent = chart_accent(cx);
    match (now, before) {
        (0, 0) => (
            t!("stats.no_activity").to_string().into(),
            theme.muted_foreground,
        ),
        (_, 0) => (t!("stats.new_this_week").to_string().into(), accent),
        _ => {
            let pct = ((now - before) as f64 * 100. / before as f64).round() as i64;
            match pct {
                0 => (
                    t!("stats.same_as_last_week").to_string().into(),
                    theme.muted_foreground,
                ),
                p => (
                    t!("stats.vs_last_week", pct = format!("{p:+}").as_str())
                        .to_string()
                        .into(),
                    if p > 0 {
                        accent
                    } else {
                        theme.muted_foreground
                    },
                ),
            }
        }
    }
}

fn month_ticks(start: NaiveDate, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let names = month_short();
    let mut months = div()
        .relative()
        .w_full()
        .h(px(14.))
        .text_size(px(FONT_LABEL))
        .text_color(theme.muted_foreground);
    for c in 0..TREND_WEEKS as u64 {
        let monday = start + Days::new(c * 7);
        if monday.month() != (monday - Days::new(7)).month() {
            months = months.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(DOW_W + WEEK_GAP + c as f32 * WEEK_STEP))
                    .child(names[monday.month0() as usize].clone()),
            );
        }
    }
    months
}

fn render_week_section(d: &InsightsSnapshot, cx: &App) -> AnyElement {
    let (cur, prev) = d.last_week_pair();
    v_flex()
        .gap(px(14.))
        .child(section_head(
            t!("stats.last_7_days").to_string(),
            Some(t!("stats.compared_prev_week").to_string().into()),
            None,
            cx,
        ))
        .child(
            h_flex()
                .gap(px(40.))
                .items_start()
                .child(stat_cell(
                    thousands(cur.sessions),
                    t!("stats.sessions").to_string(),
                    FONT_HEADING,
                    Some(week_change_note(cur.sessions, prev.sessions, cx)),
                    cx,
                ))
                .child(stat_cell(
                    thousands(cur.requests),
                    t!("stats.requests").to_string(),
                    FONT_HEADING,
                    Some(week_change_note(cur.requests, prev.requests, cx)),
                    cx,
                ))
                .child(stat_cell(
                    thousands(cur.active_days),
                    t!("stats.active_days").to_string(),
                    FONT_HEADING,
                    Some(week_change_note(cur.active_days, prev.active_days, cx)),
                    cx,
                )),
        )
        .into_any_element()
}

fn render_heatmap(d: &InsightsSnapshot, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let today = d.as_of;
    let start = d.trend_start();
    let today_ix = (today - start).num_days();

    const DAYS: usize = TREND_WEEKS * 7;
    let mut window = [0i64; DAYS];
    let mut heat_max = 1i64;
    let from = d.daily.partition_point(|(day, _)| *day < start);
    for &(day, n) in &d.daily[from..] {
        let ix = (day - start).num_days();
        if (0..DAYS as i64).contains(&ix) {
            window[ix as usize] = n;
            heat_max = heat_max.max(n);
        }
    }
    let accent = chart_accent(cx);
    let heat_color = |n: i64| -> Hsla {
        if n == 0 {
            return theme.muted;
        }
        let quartile = ((n as f32 / heat_max as f32) * 4.).ceil().clamp(1., 4.) as usize;
        accent.opacity(HEAT[quartile - 1])
    };

    let dow_col = div()
        .relative()
        .w(px(DOW_W))
        .h(px(7. * WEEK_STEP - WEEK_GAP))
        .flex_shrink_0()
        .text_size(px(FONT_LABEL))
        .text_color(theme.muted_foreground)
        .children({
            let names = dow_short();
            [0usize, 2, 4].map(|r| {
                div()
                    .absolute()
                    .top(px(r as f32 * WEEK_STEP - 2.))
                    .left_0()
                    .child(names[r])
            })
        });

    let mut grid = h_flex().gap(px(WEEK_GAP)).items_start().child(dow_col);
    for c in 0..TREND_WEEKS {
        let mut col = v_flex().gap(px(WEEK_GAP));
        for r in 0..7usize {
            let ix = c * 7 + r;
            if ix as i64 > today_ix {
                col = col.child(div().size(px(WEEK_CELL)));
                continue;
            }
            let n = window[ix];
            col = col.child(
                div()
                    .id(("hm", ix))
                    .size(px(WEEK_CELL))
                    .rounded(px(RADIUS_CELL))
                    .bg(heat_color(n))
                    .tooltip(move |window, cx| {
                        let day = start + Days::new(ix as u64);
                        let label = format!("{} · {}", requests_label(n), format_day_full(day));
                        Tooltip::new(SharedString::from(label)).build(window, cx)
                    }),
            );
        }
        grid = grid.child(col);
    }

    let mut notes: Vec<String> = Vec::new();
    if d.current_streak > 0 {
        notes.push(t!("stats.streak", days = d.current_streak.to_string().as_str()).to_string());
    }
    if d.longest_streak > 0 {
        notes.push(
            t!(
                "stats.longest_streak",
                days = d.longest_streak.to_string().as_str()
            )
            .to_string(),
        );
    }
    if let Some((day, n)) = d.busiest_day() {
        notes.push(
            t!(
                "stats.busiest",
                day = format_day_short(day).as_str(),
                count = requests_label(n).as_str()
            )
            .to_string(),
        );
    }

    let legend = h_flex()
        .justify_between()
        .items_center()
        .text_size(px(FONT_LABEL))
        .text_color(theme.muted_foreground)
        .child(div().min_w_0().truncate().child(notes.join(" · ")))
        .child(
            h_flex()
                .gap(px(WEEK_GAP))
                .items_center()
                .flex_shrink_0()
                .child(t!("stats.less").to_string())
                .children(std::iter::once(0.).chain(HEAT).map(|a: f32| {
                    div()
                        .size(px(WEEK_CELL))
                        .rounded(px(RADIUS_CELL))
                        .bg(if a == 0. {
                            theme.muted
                        } else {
                            accent.opacity(a)
                        })
                }))
                .child(t!("stats.more").to_string()),
        );

    v_flex()
        .gap(px(6.))
        .child(month_ticks(start, cx))
        .child(grid)
        .child(div().pt(px(4.)).child(legend))
        .into_any_element()
}

fn render_trend(start: NaiveDate, layers: Rc<Vec<TrendLayer>>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    const CHART_H: f32 = 72.;
    let totals: Vec<i64> = (0..TREND_WEEKS)
        .map(|w| {
            layers
                .iter()
                .map(|l| l.weekly.get(w).copied().unwrap_or(0))
                .sum()
        })
        .collect();
    let max = totals.iter().copied().max().unwrap_or(0).max(1);

    let mut columns = h_flex()
        .items_end()
        .gap(px(WEEK_GAP))
        .h(px(CHART_H))
        .child(div().w(px(DOW_W)).flex_shrink_0());
    for w in 0..TREND_WEEKS {
        let total = totals[w];
        let column = if total == 0 {
            div()
                .w(px(WEEK_CELL))
                .h(px(2.))
                .rounded(px(RADIUS_CELL))
                .bg(theme.muted)
        } else {
            let col_h = ((total as f32 / max as f32) * CHART_H).max(3.);
            let mut col = v_flex()
                .w(px(WEEK_CELL))
                .h(px(col_h))
                .justify_end()
                .rounded(px(RADIUS_CELL))
                .overflow_hidden();
            for l in layers
                .iter()
                .rev()
                .filter(|l| l.weekly.get(w).copied().unwrap_or(0) > 0)
            {
                let h = (l.weekly[w] as f32 / total as f32) * col_h;
                col = col.child(div().w_full().h(px(h)).bg(l.color));
            }
            col
        };
        let layers = Rc::clone(&layers);
        columns = columns.child(column.id(("trend", w)).flex_shrink_0().tooltip(
            move |window, cx| {
                let monday = start + Days::new(w as u64 * 7);
                let mut label = format!(
                    "{} · {}",
                    t!("stats.week_of", day = format_day_short(monday).as_str()),
                    requests_label(total)
                );
                let breakdown: Vec<String> = layers
                    .iter()
                    .filter(|l| l.weekly.get(w).copied().unwrap_or(0) > 0)
                    .take(3)
                    .map(|l| format!("{} {}", l.name, l.weekly[w]))
                    .collect();
                if !breakdown.is_empty() {
                    label.push('\n');
                    label.push_str(&breakdown.join(" · "));
                }
                Tooltip::new(SharedString::from(label)).build(window, cx)
            },
        ));
    }

    let legend = h_flex()
        .flex_wrap()
        .gap_x(px(14.))
        .gap_y(px(4.))
        .pl(px(DOW_W + WEEK_GAP))
        .text_size(px(FONT_LABEL))
        .text_color(theme.muted_foreground)
        .children(layers.iter().map(|l| {
            h_flex()
                .gap(px(5.))
                .items_center()
                .child(
                    div()
                        .size(px(WEEK_CELL))
                        .rounded(px(RADIUS_CELL))
                        .bg(l.color),
                )
                .child(l.name.clone())
        }));

    v_flex()
        .gap(px(6.))
        .child(columns)
        .child(month_ticks(start, cx))
        .child(legend)
        .into_any_element()
}

fn trend_caption(layers: &[TrendLayer]) -> String {
    let leader = [TREND_WEEKS - 1, TREND_WEEKS - 2]
        .into_iter()
        .find_map(|w| {
            layers
                .iter()
                .filter(|l| !l.other && l.weekly.get(w).copied().unwrap_or(0) > 0)
                .max_by_key(|l| l.weekly[w])
                .map(|l| l.name.to_string())
        });
    match leader {
        Some(name) => t!("stats.over_time_lead", name = name.as_str()).to_string(),
        None => t!("stats.over_time_subtitle").to_string(),
    }
}

fn dist_caption(range: InsightsRange, peak: usize, _peak_n: i64) -> String {
    match range {
        InsightsRange::Hour => {
            t!("stats.most_active_hour", hour = hour_label(peak).as_str()).to_string()
        }
        InsightsRange::Weekday => {
            t!("stats.most_active_weekday", day = dow_plural()[peak]).to_string()
        }
        InsightsRange::Month => t!(
            "stats.most_active_month",
            month = month_full()[peak].as_ref()
        )
        .to_string(),
    }
}

fn render_distribution(range: InsightsRange, values: &[i64], peak: usize, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let accent = chart_accent(cx);
    let max = values.iter().copied().max().unwrap_or(0).max(1);
    let gap = match range {
        InsightsRange::Hour => px(4.),
        InsightsRange::Weekday => px(8.),
        InsightsRange::Month => px(6.),
    };
    const CHART_H: f32 = 72.;
    let long_names: Vec<SharedString> = match range {
        InsightsRange::Hour => Vec::new(),
        InsightsRange::Weekday => dow_plural().map(SharedString::from).into(),
        InsightsRange::Month => month_full().into(),
    };
    let tick_names: Vec<SharedString> = match range {
        InsightsRange::Hour => Vec::new(),
        InsightsRange::Weekday => dow_short().map(SharedString::from).into(),
        InsightsRange::Month => month_short().into(),
    };

    v_flex()
        .gap(px(6.))
        .child(
            h_flex()
                .items_end()
                .gap(gap)
                .h(px(CHART_H))
                .children((0..values.len()).map(|i| {
                    let n = values[i];
                    let (height, bg) = if n == 0 {
                        (px(2.), theme.muted)
                    } else {
                        let frac = (n as f32 / max as f32).max(0.05);
                        (
                            px((frac * CHART_H).max(3.)),
                            if i == peak {
                                accent
                            } else {
                                accent.opacity(0.55)
                            },
                        )
                    };
                    let label: SharedString = match range {
                        InsightsRange::Hour => format!(
                            "{} · {} – {}",
                            requests_label(n),
                            hour_label(i),
                            hour_label(i + 1)
                        ),
                        _ => format!("{} · {}", requests_label(n), long_names[i]),
                    }
                    .into();
                    div()
                        .id(("dist", i))
                        .flex_1()
                        .h(height)
                        .rounded(px(RADIUS_CELL))
                        .bg(bg)
                        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
                })),
        )
        .child(
            h_flex()
                .gap(gap)
                .text_size(px(FONT_LABEL))
                .text_color(theme.muted_foreground)
                .children((0..values.len()).map(|i| {
                    let tick = tick_names.get(i).cloned();
                    div().flex_1().whitespace_nowrap().map(|slot| match tick {
                        Some(name) => slot.flex().justify_center().child(name),
                        None if i % 6 == 0 => slot.child(hour_label(i)),
                        None => slot,
                    })
                })),
        )
        .into_any_element()
}

fn render_overview(d: &InsightsSnapshot, cx: &App) -> AnyElement {
    h_flex()
        .gap(px(40.))
        .flex_wrap()
        .child(stat_cell(
            thousands(d.sessions),
            t!("stats.sessions").to_string(),
            FONT_TITLE,
            None,
            cx,
        ))
        .when(d.tokens > 0, |row| {
            row.child(stat_cell(
                format_tokens(d.tokens),
                t!("stats.tokens").to_string(),
                FONT_TITLE,
                None,
                cx,
            ))
        })
        .child(stat_cell(
            thousands(d.requests),
            t!("stats.requests").to_string(),
            FONT_TITLE,
            None,
            cx,
        ))
        .child(stat_cell(
            thousands(d.agent_count),
            t!("stats.agents").to_string(),
            FONT_TITLE,
            None,
            cx,
        ))
        .child(stat_cell(
            thousands(d.project_count),
            t!("stats.projects").to_string(),
            FONT_TITLE,
            None,
            cx,
        ))
        .child(stat_cell(
            thousands(d.active_days()),
            t!("stats.active_days").to_string(),
            FONT_TITLE,
            None,
            cx,
        ))
        .into_any_element()
}

fn render_usage_board(
    title: SharedString,
    rows: &[tokens_core::UsageTally],
    dark: bool,
    show_brand: bool,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    // Wake Tokens metric: skip groups with no reported usage (0 ≠ "used zero").
    let mut sorted: Vec<&tokens_core::UsageTally> =
        rows.iter().filter(|r| r.tokens > 0).collect();
    sorted.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    if show_brand {
        // Agents: Wake lists all that reported tokens.
    } else {
        sorted.truncate(6);
    }
    let max_tok = sorted.iter().map(|r| r.tokens).max().unwrap_or(0).max(1);

    if sorted.is_empty() {
        return div().into_any_element();
    }

    v_flex()
        .gap(px(14.))
        .child(section_head(
            title,
            None,
            Some(
                div()
                    .text_size(px(FONT_CAPTION))
                    .text_color(theme.muted_foreground)
                    .child(t!("stats.tokens").to_string())
                    .into_any_element(),
            ),
            cx,
        ))
        .child(
            v_flex()
                .gap(px(8.))
                .children(sorted.into_iter().map(|row| {
                    let frac = (row.tokens as f32 / max_tok as f32).clamp(0.02, 1.);
                    let brand = show_brand.then(|| brand_id_for_client(&row.name)).flatten();
                    let name = if show_brand {
                        display_agent_name(&row.name)
                    } else {
                        row.name.clone().into()
                    };
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            h_flex()
                                .w(px(140.))
                                .items_center()
                                .gap(px(6.))
                                .flex_shrink_0()
                                .when_some(brand, |row, id| row.child(brand_img(id, dark, px(15.))))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.foreground)
                                        .truncate()
                                        .child(name),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(6.))
                                .rounded(px(3.))
                                .bg(theme.muted)
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(frac))
                                        .rounded(px(3.))
                                        .bg(chart_accent(cx)),
                                ),
                        )
                        .child(
                            div()
                                .w(px(72.))
                                .text_right()
                                .text_size(px(FONT_LABEL))
                                .text_color(theme.muted_foreground)
                                .child(format_tokens(row.tokens)),
                        )
                })),
        )
        .into_any_element()
}

fn relative(frac: f32) -> gpui::Length {
    gpui::Length::Definite(gpui::relative(frac))
}

/// Full Insights page: header + scrollable Wake-style charts.
pub fn render_insights_page(app: &RouterApp, cx: &mut Context<RouterApp>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let dark = theme.is_dark();
    let loading = app.is_loading_dashboard;
    let range = app.insights_range;
    let data = app.insights_data.as_ref();

    let subtitle: SharedString = match data {
        Some(d) if d.sessions > 0 => {
            let my = month_year(d.first_ts);
            if my.is_empty() {
                t!("stats.subtitle_default").to_string().into()
            } else {
                t!("stats.since", month = my.as_str()).to_string().into()
            }
        }
        _ => t!("stats.subtitle_default").to_string().into(),
    };

    let body: AnyElement = match data {
        Some(d) if d.sessions > 0 || d.requests > 0 => {
            let layers = trend_layers(&d.trend_agents, cx);
            let values: &[i64] = match range {
                InsightsRange::Hour => &d.hourly,
                InsightsRange::Weekday => &d.weekday,
                InsightsRange::Month => &d.monthly,
            };
            let (peak, peak_n) = values
                .iter()
                .enumerate()
                .max_by_key(|(_, n)| **n)
                .map(|(i, n)| (i, *n))
                .unwrap_or((0, 0));

            let arrows = h_flex()
                .gap(px(2.))
                .child(
                    Button::new("insights-range-prev")
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronLeft)
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.insights_range = this.insights_range.prev();
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("insights-range-next")
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronRight)
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.insights_range = this.insights_range.next();
                            cx.notify();
                        })),
                );

            div()
                .id("insights-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div().w_full().flex().justify_center().px(px(8.)).child(
                        v_flex()
                            .w_full()
                            .max_w(px(720.))
                            .pt(px(8.))
                            .pb(px(40.))
                            .gap(px(SECTION_GAP))
                            .child(render_overview(d, cx))
                            .child(render_week_section(d, cx))
                            .child(
                                v_flex()
                                    .gap(px(14.))
                                    .child(section_head(
                                        t!("stats.activity").to_string(),
                                        Some(t!("stats.activity_subtitle").to_string().into()),
                                        None,
                                        cx,
                                    ))
                                    .child(render_heatmap(d, cx)),
                            )
                            .child(
                                v_flex()
                                    .gap(px(14.))
                                    .child(section_head(
                                        t!("stats.over_time").to_string(),
                                        Some(trend_caption(&layers).into()),
                                        None,
                                        cx,
                                    ))
                                    .child(render_trend(d.trend_start(), layers, cx)),
                            )
                            .child(
                                v_flex()
                                    .gap(px(14.))
                                    .child(section_head(
                                        t!(range.title_key()).to_string(),
                                        Some(dist_caption(range, peak, peak_n).into()),
                                        Some(arrows.into_any_element()),
                                        cx,
                                    ))
                                    .child(render_distribution(range, values, peak, cx)),
                            )
                            .when(!d.agents.is_empty(), |col| {
                                col.child(render_usage_board(
                                    t!("stats.agents").to_string().into(),
                                    &d.agents,
                                    dark,
                                    true,
                                    cx,
                                ))
                            })
                            .when(!d.projects.is_empty(), |col| {
                                col.child(render_usage_board(
                                    t!("stats.projects").to_string().into(),
                                    &d.projects,
                                    dark,
                                    false,
                                    cx,
                                ))
                            })
                            .when(!d.models.is_empty(), |col| {
                                col.child(render_usage_board(
                                    t!("stats.models").to_string().into(),
                                    &d.models,
                                    dark,
                                    false,
                                    cx,
                                ))
                            }),
                    ),
                )
                .into_any_element()
        }
        _ if loading => div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child(t!("stats.loading").to_string())
            .into_any_element(),
        _ => v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.foreground)
                    .child(t!("stats.empty").to_string()),
            )
            .child(
                div()
                    .text_size(px(FONT_CAPTION))
                    .text_color(theme.muted_foreground)
                    .child(t!("stats.empty_hint").to_string()),
            )
            .into_any_element(),
    };

    v_flex()
        .w_full()
        .h_full()
        .gap(px(12.))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    v_flex()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.foreground)
                                .child(t!("stats.title").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(FONT_CAPTION))
                                .text_color(theme.muted_foreground)
                                .child(subtitle),
                        ),
                )
                .child(
                    h_flex()
                        .gap(px(8.))
                        .items_center()
                        .child(div().w(px(80.)).child(
                            gpui_component::select::Select::new(&app.usage_refresh_select).small(),
                        ))
                        .child(
                            Button::new("insights-refresh-btn")
                                .outline()
                                .small()
                                .icon(CustomIcon::RotateCw)
                                .tooltip(t!("stats.refresh").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.refresh_dashboard_data(cx);
                                    let msg = t!("stats.refreshed").to_string();
                                    window.push_notification(
                                        gpui_component::notification::Notification::success(msg),
                                        cx,
                                    );
                                })),
                        ),
                ),
        )
        .child(body)
}
