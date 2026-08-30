use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use domain::AppKind;
use gpui::{rgb, Hsla};
use std::collections::HashMap;
use tokens_core::{generate_graph, GroupBy, ReportOptions};

use crate::assets::CustomIcon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UsageWindowChoice {
    #[default]
    Hours6,
    Hours24,
    Yesterday,
    Days7,
    Days30,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UsageRefreshInterval {
    Off,
    Sec10,
    Sec30,
    #[default]
    Sec60,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UsageMetric {
    Cost,
    #[default]
    Tokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UsageBreakdownTab {
    #[default]
    Model,
    Day,
}

#[derive(Debug, Clone)]
pub struct DashboardDailyPoint {
    pub date: String,
    pub label: String,
    pub cost: f64,
    pub tokens: i64,
}

/// One bucket of the rolling-window timeline (HH:MM label, local time).
#[derive(Debug, Clone)]
pub struct DashboardTimePoint {
    pub label: String,
    pub cost: f64,
    pub tokens: i64,
}

#[derive(Debug, Clone)]
pub struct DashboardModelRow {
    pub model_id: String,
    pub display_name: String,
    pub app_icon: CustomIcon,
    pub color: Hsla,
    pub cost: f64,
    pub cost_formatted: String,
    pub share_percent: f64,
    pub share_formatted: String,
    pub tokens: i64,
    pub tokens_formatted: String,
}

#[derive(Debug, Clone)]
pub struct DashboardDayRow {
    pub date: String,
    pub date_display: String,
    pub cost: f64,
    pub cost_formatted: String,
    pub share_percent: f64,
    pub share_formatted: String,
    pub tokens: i64,
    pub tokens_formatted: String,
}

#[derive(Debug, Clone)]
pub struct DashboardUsageData {
    pub total_cost: f64,
    pub total_cost_formatted: String,
    pub total_tokens: i64,
    pub total_tokens_formatted: String,
    pub active_days: i32,
    pub per_active_day_formatted: String,
    pub cached_input_tokens: i64,
    pub cached_input_formatted: String,
    pub uncached_input_tokens: i64,
    pub uncached_input_formatted: String,
    pub cache_write_tokens: i64,
    pub cache_write_formatted: String,
    pub output_tokens: i64,
    pub output_formatted: String,
    pub reasoning_tokens: i64,
    pub reasoning_formatted: String,
    pub cache_savings_cost: f64,
    pub cache_savings_cost_formatted: String,
    pub cache_savings_multiple: f64,
    pub cache_savings_multiple_formatted: String,
    pub observed_input_share_percent: f64,
    pub observed_input_share_formatted: String,
    pub daily_points: Vec<DashboardDailyPoint>,
    pub time_points: Vec<DashboardTimePoint>,
    pub max_time_cost: f64,
    pub max_time_tokens: i64,
    pub model_rows: Vec<DashboardModelRow>,
    pub day_rows: Vec<DashboardDayRow>,
    pub active_clients: Vec<String>,
    pub range_desc: String,
    pub max_daily_cost: f64,
    pub max_daily_tokens: i64,
}

impl Default for DashboardUsageData {
    fn default() -> Self {
        Self {
            total_cost: 0.0,
            total_cost_formatted: "$0.00".to_string(),
            total_tokens: 0,
            total_tokens_formatted: "0".to_string(),
            active_days: 0,
            per_active_day_formatted: "0".to_string(),
            cached_input_tokens: 0,
            cached_input_formatted: "0".to_string(),
            uncached_input_tokens: 0,
            uncached_input_formatted: "0".to_string(),
            cache_write_tokens: 0,
            cache_write_formatted: "0".to_string(),
            output_tokens: 0,
            output_formatted: "0".to_string(),
            reasoning_tokens: 0,
            reasoning_formatted: "0".to_string(),
            cache_savings_cost: 0.0,
            cache_savings_cost_formatted: "$0.00".to_string(),
            cache_savings_multiple: 1.0,
            cache_savings_multiple_formatted: "1.0".to_string(),
            observed_input_share_percent: 0.0,
            observed_input_share_formatted: "0.0%".to_string(),
            daily_points: Vec::new(),
            time_points: Vec::new(),
            max_time_cost: 0.0,
            max_time_tokens: 0,
            model_rows: Vec::new(),
            day_rows: Vec::new(),
            active_clients: Vec::new(),
            range_desc: String::new(),
            max_daily_cost: 0.0,
            max_daily_tokens: 0,
        }
    }
}

pub fn format_tokens(tokens: i64) -> String {
    let t = tokens.max(0) as f64;
    if t >= 1_000_000_000.0 {
        format!("{:.2} B", t / 1_000_000_000.0)
    } else if t >= 1_000_000.0 {
        let val = t / 1_000_000.0;
        if val >= 100.0 {
            format!("{:.0} M", val)
        } else if val >= 10.0 {
            format!("{:.1} M", val)
        } else {
            format!("{:.2} M", val)
        }
    } else if t >= 1_000.0 {
        let val = t / 1_000.0;
        if val >= 100.0 {
            format!("{:.0} K", val)
        } else {
            format!("{:.1} K", val)
        }
    } else {
        format!("{}", tokens.max(0))
    }
}

pub fn format_currency(cost: f64) -> String {
    let c = cost.max(0.0);
    if c >= 1000.0 {
        let thousands = (c / 1000.0).floor();
        let remainder = c - (thousands * 1000.0);
        format!("${:.0},{:06.2}", thousands, remainder)
    } else {
        format!("${:.2}", c)
    }
}

pub fn model_icon_and_color(model_id: &str) -> (CustomIcon, Hsla) {
    let lower = model_id.to_lowercase();
    if lower.contains("claude") || lower.contains("anthropic") {
        (CustomIcon::Claude, rgb(0xD97757).into())
    } else if lower.contains("gpt")
        || lower.contains("o1")
        || lower.contains("o3")
        || lower.contains("openai")
        || lower.contains("text-embedding")
    {
        (CustomIcon::OpenAI, rgb(0x10A37F).into())
    } else if lower.contains("grok") || lower.contains("x-ai") || lower.contains("xai") {
        (CustomIcon::Grok, rgb(0x8B5CF6).into())
    } else if lower.contains("deepseek") {
        (CustomIcon::DeepSeek, rgb(0x4D6BFE).into())
    } else if lower.contains("gemini") || lower.contains("google") {
        (CustomIcon::DeepSeek, rgb(0x3B82F6).into())
    } else if lower.contains("qwen") {
        (CustomIcon::OpenCode, rgb(0x6366F1).into())
    } else if lower.contains("kimi") || lower.contains("moonshot") {
        (CustomIcon::Kimi, rgb(0x0284C7).into())
    } else if lower.contains("pi") || lower.contains("inflection") {
        (CustomIcon::Pi, rgb(0x10B981).into())
    } else if lower.contains("cursor") || lower.contains("composer") {
        (CustomIcon::Cursor, rgb(0x06B6D4).into())
    } else {
        (CustomIcon::OpenCode, rgb(0x64748B).into())
    }
}

pub fn format_date_label(date_str: &str) -> String {
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        format!("{}月{}日", date.month(), date.day())
    } else {
        date_str.to_string()
    }
}

pub fn test_canvas() -> impl gpui::IntoElement {
    gpui::canvas(
        |_bounds, _window, _cx| (),
        |bounds, _state, window, _cx| {
            window.paint_quad(gpui::fill(
                gpui::Bounds::new(bounds.origin, gpui::size(gpui::px(10.0), gpui::px(10.0))),
                gpui::rgb(0x10A37F),
            ));
        },
    )
}

pub async fn load_dashboard_usage(
    app_filter: Option<AppKind>,
    window_choice: UsageWindowChoice,
    metric: UsageMetric,
) -> DashboardUsageData {
    let today = Local::now().date_naive();
    let (since_date, until_date, since_ts_ms) = match window_choice {
        UsageWindowChoice::Hours6 => {
            let cutoff = Local::now() - Duration::hours(6);
            (cutoff.date_naive(), today, Some(cutoff.timestamp_millis()))
        }
        UsageWindowChoice::Hours24 => {
            let cutoff = Local::now() - Duration::hours(24);
            (cutoff.date_naive(), today, Some(cutoff.timestamp_millis()))
        }
        UsageWindowChoice::Yesterday => {
            let y = today - Duration::days(1);
            (y, y, None)
        }
        UsageWindowChoice::Days7 => (today - Duration::days(6), today, None),
        UsageWindowChoice::Days30 => (today - Duration::days(29), today, None),
        UsageWindowChoice::Month => {
            let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
            (start, today, None)
        }
    };

    let since_str = since_date.format("%Y-%m-%d").to_string();
    let until_str = until_date.format("%Y-%m-%d").to_string();

    let client_filters: Option<Vec<String>> = match app_filter {
        None => None,
        Some(AppKind::Claude) => Some(vec!["claude".to_string()]),
        Some(AppKind::Codex) => Some(vec!["codex".to_string()]),
        Some(AppKind::Grok) => Some(vec!["grok".to_string()]),
        Some(AppKind::OpenCode) => Some(vec!["opencode".to_string()]),
        Some(AppKind::Pi) => Some(vec!["pi".to_string()]),
        Some(AppKind::Cursor) => Some(vec!["cursor".to_string()]),
        Some(AppKind::ZCode) => Some(vec!["zcode".to_string()]),
        Some(AppKind::WorkBuddy) => Some(vec!["workbuddy".to_string(), "codebuddy".to_string()]),
    };

    let options = ReportOptions {
        home_dir: None,
        use_env_roots: true,
        clients: client_filters,
        since: Some(since_str.clone()),
        until: Some(until_str.clone()),
        year: None,
        group_by: GroupBy::ClientModel,
        scanner_settings: Default::default(),
        today_only: false,
        since_ts_ms,
    };

    let graph_result = match session::tokio_runtime()
        .spawn(async move { generate_graph(options).await })
        .await
    {
        Ok(Ok(res)) => res,
        Ok(Err(err)) => {
            eprintln!("[router-switch] generate_graph error: {}", err);
            return DashboardUsageData::default();
        }
        Err(err) => {
            eprintln!("[router-switch] tokio task join error: {}", err);
            return DashboardUsageData::default();
        }
    };

    let contributions = graph_result.contributions;
    let summary = graph_result.summary;

    let time_points: Vec<DashboardTimePoint> = graph_result
        .time_points
        .unwrap_or_default()
        .into_iter()
        .map(|p| DashboardTimePoint {
            label: Local
                .timestamp_millis_opt(p.start_ts_ms)
                .single()
                .map(|t| t.format("%H:%M").to_string())
                .unwrap_or_default(),
            cost: p.cost,
            tokens: p.tokens,
        })
        .collect();
    let max_time_cost = time_points.iter().map(|p| p.cost).fold(0.0, f64::max);
    let max_time_tokens = time_points.iter().map(|p| p.tokens).fold(0i64, i64::max);

    let mut cached_input_tokens: i64 = 0;
    let mut uncached_input_tokens: i64 = 0;
    let mut cache_write_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut reasoning_tokens: i64 = 0;
    let mut cache_savings_cost: f64 = 0.0;

    // Per-model aggregations
    struct ModelAgg {
        cost: f64,
        tokens: i64,
    }
    let mut model_map: HashMap<String, ModelAgg> = HashMap::new();

    // Per-day aggregations
    struct DayAgg {
        cost: f64,
        tokens: i64,
    }
    let mut day_map: HashMap<String, DayAgg> = HashMap::new();

    for contrib in &contributions {
        cached_input_tokens += contrib.token_breakdown.cache_read;
        uncached_input_tokens += contrib.token_breakdown.input;
        cache_write_tokens += contrib.token_breakdown.cache_write;
        output_tokens += contrib.token_breakdown.output;
        reasoning_tokens += contrib.token_breakdown.reasoning;

        let entry = day_map.entry(contrib.date.clone()).or_insert(DayAgg {
            cost: 0.0,
            tokens: 0,
        });
        entry.cost += contrib.totals.cost;
        entry.tokens += contrib.totals.tokens;

        for client in &contrib.clients {
            let m_entry = model_map
                .entry(client.model_id.clone())
                .or_insert(ModelAgg {
                    cost: 0.0,
                    tokens: 0,
                });
            m_entry.cost += client.cost;
            m_entry.tokens += client.tokens.total();

            // Estimate cache savings: standard input price vs cache read rate (~90% savings)
            if client.tokens.cache_read > 0 {
                // If model has pricing, compute savings = cache_read * (input_price - cache_read_price)
                // Baseline: ~ $2.70 / 1M cached input tokens saved
                cache_savings_cost += (client.tokens.cache_read as f64) * 2.7e-6;
            }
        }
    }

    let total_cost = summary.total_cost;
    let total_tokens = summary.total_tokens;
    let active_days = summary.active_days.max(1);

    let per_active_day_formatted = match metric {
        UsageMetric::Cost => {
            let avg = if summary.active_days > 0 {
                total_cost / summary.active_days as f64
            } else {
                0.0
            };
            format!("${:.2}", avg)
        }
        UsageMetric::Tokens => {
            let avg = if summary.active_days > 0 {
                total_tokens / summary.active_days as i64
            } else {
                0
            };
            format_tokens(avg)
        }
    };

    let observed_input = uncached_input_tokens + cached_input_tokens;
    let observed_input_share_percent = if observed_input > 0 {
        (cached_input_tokens as f64 / observed_input as f64) * 100.0
    } else {
        0.0
    };

    let cache_savings_multiple = if total_cost > 0.001 {
        (total_cost + cache_savings_cost) / total_cost
    } else if cache_savings_cost > 0.0 {
        1.0 + (cache_savings_cost / 1.0)
    } else {
        1.0
    };

    // Build continuous timeline daily points
    let mut daily_points = Vec::new();
    let mut current_d = since_date;
    let mut max_daily_cost: f64 = 0.0;
    let mut max_daily_tokens: i64 = 0;

    while current_d <= until_date {
        let d_str = current_d.format("%Y-%m-%d").to_string();
        let label = format!("{}月{}日", current_d.month(), current_d.day());
        let (cost, tokens) = if let Some(agg) = day_map.get(&d_str) {
            (agg.cost, agg.tokens)
        } else {
            (0.0, 0)
        };

        if cost > max_daily_cost {
            max_daily_cost = cost;
        }
        if tokens > max_daily_tokens {
            max_daily_tokens = tokens;
        }

        daily_points.push(DashboardDailyPoint {
            date: d_str,
            label,
            cost,
            tokens,
        });

        current_d += Duration::days(1);
    }

    // Build Model Rows
    let mut model_rows: Vec<DashboardModelRow> = model_map
        .into_iter()
        .map(|(model_id, agg)| {
            let (app_icon, color) = model_icon_and_color(&model_id);
            let share_percent = if total_cost > 0.0001 {
                (agg.cost / total_cost) * 100.0
            } else if total_tokens > 0 {
                (agg.tokens as f64 / total_tokens as f64) * 100.0
            } else {
                0.0
            };
            DashboardModelRow {
                display_name: model_id.clone(),
                model_id,
                app_icon,
                color,
                cost: agg.cost,
                cost_formatted: format_currency(agg.cost),
                share_percent,
                share_formatted: format!("{:.1}%", share_percent),
                tokens: agg.tokens,
                tokens_formatted: format_tokens(agg.tokens),
            }
        })
        .collect();

    // Sort models descending by cost (or tokens)
    match metric {
        UsageMetric::Cost => {
            model_rows.sort_by(|a, b| {
                b.cost
                    .partial_cmp(&a.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        UsageMetric::Tokens => {
            model_rows.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        }
    }

    // Build Day Rows
    let mut day_rows: Vec<DashboardDayRow> = day_map
        .into_iter()
        .map(|(date, agg)| {
            let date_display = format_date_label(&date);
            let share_percent = if total_cost > 0.0001 {
                (agg.cost / total_cost) * 100.0
            } else if total_tokens > 0 {
                (agg.tokens as f64 / total_tokens as f64) * 100.0
            } else {
                0.0
            };
            DashboardDayRow {
                date,
                date_display,
                cost: agg.cost,
                cost_formatted: format_currency(agg.cost),
                share_percent,
                share_formatted: format!("{:.1}%", share_percent),
                tokens: agg.tokens,
                tokens_formatted: format_tokens(agg.tokens),
            }
        })
        .collect();

    // Sort days descending by date
    day_rows.sort_by(|a, b| b.date.cmp(&a.date));

    let range_desc = if since_str == until_str {
        format!(
            "{}年{}月{}日",
            since_date.year(),
            since_date.month(),
            since_date.day()
        )
    } else {
        format!(
            "{}年{}月{}日 至 {}月{}日",
            since_date.year(),
            since_date.month(),
            since_date.day(),
            until_date.month(),
            until_date.day()
        )
    };

    DashboardUsageData {
        total_cost,
        total_cost_formatted: format_currency(total_cost),
        total_tokens,
        total_tokens_formatted: format_tokens(total_tokens),
        active_days,
        per_active_day_formatted,
        cached_input_tokens,
        cached_input_formatted: format_tokens(cached_input_tokens),
        uncached_input_tokens,
        uncached_input_formatted: format_tokens(uncached_input_tokens),
        cache_write_tokens,
        cache_write_formatted: format_tokens(cache_write_tokens),
        output_tokens,
        output_formatted: format_tokens(output_tokens),
        reasoning_tokens,
        reasoning_formatted: format_tokens(reasoning_tokens),
        cache_savings_cost,
        cache_savings_cost_formatted: format_currency(cache_savings_cost),
        cache_savings_multiple,
        cache_savings_multiple_formatted: format!("{:.1}", cache_savings_multiple),
        observed_input_share_percent,
        observed_input_share_formatted: format!("{:.1}%", observed_input_share_percent),
        daily_points,
        time_points,
        max_time_cost,
        max_time_tokens,
        model_rows,
        day_rows,
        active_clients: summary.clients,
        range_desc,
        max_daily_cost,
        max_daily_tokens,
    }
}
