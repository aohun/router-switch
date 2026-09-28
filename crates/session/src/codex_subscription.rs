//! Codex subscription usage + model discovery via ChatGPT backend-api
//! (aligned with AstrLink / openai-codex `/wham/usage` and `/models`).

use serde::Deserialize;
use serde_json::Value;

const CODEX_API_BASE: &str = "https://chatgpt.com/backend-api/codex";
const CODEX_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_MODELS_CLIENT_VERSION: &str = "0.155.1";
const CODEX_ORIGINATOR: &str = "codex-tui";
const CODEX_UA_SUFFIX: &str = " (Ubuntu 22.4.0; x86_64) xterm-256color";

#[derive(Debug, Clone, Default)]
pub struct CodexRateLimitWindow {
    pub used_percent: f64,
    pub limit_window_seconds: Option<i64>,
    pub reset_after_seconds: Option<i64>,
    /// Unix epoch seconds when the window resets.
    pub reset_at: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct CodexSubscriptionUsage {
    pub plan_type: Option<String>,
    pub limit_reached: bool,
    pub primary: Option<CodexRateLimitWindow>,
    pub secondary: Option<CodexRateLimitWindow>,
}

#[derive(Debug, Clone)]
pub struct CodexStoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub account_id: Option<String>,
    pub id_token: Option<String>,
    pub last_refresh: Option<String>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap_or_default()
}

fn apply_codex_api_headers(
    req: reqwest::RequestBuilder,
    tokens: &CodexStoredTokens,
) -> reqwest::RequestBuilder {
    let ua = format!("{CODEX_ORIGINATOR}/{CODEX_MODELS_CLIENT_VERSION}{CODEX_UA_SUFFIX}");
    let mut req = req
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .header("OAI-Product-Sku", "codex")
        .header("originator", CODEX_ORIGINATOR)
        .header("User-Agent", ua)
        .header("version", CODEX_MODELS_CLIENT_VERSION)
        .header("Accept", "application/json");
    if let Some(account_id) = tokens
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        req = req.header("ChatGPT-Account-ID", account_id);
    }
    req
}

/// Mask account id like AstrLink: `abcd***xy` (or short → `a***`).
pub fn mask_account_hint(account_id: &str) -> String {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return String::new();
    }
    if account_id.len() <= 8 {
        return format!("{}***", &account_id[..1]);
    }
    format!(
        "{}***{}",
        &account_id[..4],
        &account_id[account_id.len() - 2..]
    )
}

/// Human plan badge label (AstrLink `planTypeLabel` for openai_codex).
pub fn plan_type_label(plan_type: &str) -> String {
    match plan_type.trim().to_ascii_lowercase().as_str() {
        "plus" => "Plus".into(),
        "pro" => "Pro 20x".into(),
        "prolite" => "Pro 5x".into(),
        "go" => "Go".into(),
        "free" => "Free".into(),
        "team" => "Team".into(),
        "business" => "Business".into(),
        "enterprise" => "Enterprise".into(),
        "edu" | "education" => "Edu".into(),
        other if !other.is_empty() => {
            let mut chars = other.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => other.to_string(),
            }
        }
        _ => String::new(),
    }
}

pub fn window_label(seconds: Option<i64>, is_secondary: bool, zh: bool) -> String {
    let Some(seconds) = seconds.filter(|s| *s > 0) else {
        return if zh {
            if is_secondary {
                "周期限额".into()
            } else {
                "滚动限额".into()
            }
        } else if is_secondary {
            "Period limit".into()
        } else {
            "Rolling limit".into()
        };
    };
    match seconds {
        18_000 => {
            if zh {
                "5 小时".into()
            } else {
                "5 hours".into()
            }
        }
        604_800 => {
            if zh {
                "7 天".into()
            } else {
                "7 days".into()
            }
        }
        86_400 => {
            if zh {
                "每天".into()
            } else {
                "Daily".into()
            }
        }
        3_600 => {
            if zh {
                "每小时".into()
            } else {
                "Hourly".into()
            }
        }
        s if s % 86_400 == 0 => {
            let days = s / 86_400;
            if zh {
                format!("{days} 天")
            } else if days == 1 {
                "1 day".into()
            } else {
                format!("{days} days")
            }
        }
        s if s % 3_600 == 0 => {
            let hours = s / 3_600;
            if zh {
                format!("{hours} 小时")
            } else if hours == 1 {
                "1 hour".into()
            } else {
                format!("{hours} hours")
            }
        }
        s if s % 60 == 0 => {
            let minutes = s / 60;
            if zh {
                format!("{minutes} 分钟")
            } else if minutes == 1 {
                "1 minute".into()
            } else {
                format!("{minutes} minutes")
            }
        }
        _ => {
            if zh {
                if is_secondary {
                    "周期限额".into()
                } else {
                    "滚动限额".into()
                }
            } else if is_secondary {
                "Period limit".into()
            } else {
                "Rolling limit".into()
            }
        }
    }
}

pub fn format_reset_countdown(window: &CodexRateLimitWindow, zh: bool) -> Option<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let target = if let Some(reset_at) = window.reset_at.filter(|t| *t > 0) {
        reset_at
    } else if let Some(after) = window.reset_after_seconds.filter(|t| *t >= 0) {
        now + after
    } else {
        return None;
    };
    let delta = (target - now).max(0);
    let minutes = delta / 60;
    if minutes < 1 {
        return Some(if zh {
            "即将重置".into()
        } else {
            "Resets soon".into()
        });
    }
    if minutes < 60 {
        return Some(if zh {
            format!("{minutes} 分钟后重置")
        } else {
            format!("Resets in {minutes}m")
        });
    }
    let hours = minutes / 60;
    if hours < 24 {
        return Some(if zh {
            format!("{hours} 小时后重置")
        } else {
            format!("Resets in {hours}h")
        });
    }
    let days = ((hours as f64) / 24.0).round() as i64;
    Some(if zh {
        format!("{days} 天后重置")
    } else {
        format!("Resets in {days}d")
    })
}

pub async fn fetch_codex_usage(
    tokens: &CodexStoredTokens,
) -> Result<CodexSubscriptionUsage, String> {
    let response = apply_codex_api_headers(http_client().get(CODEX_USAGE_URL), tokens)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Codex usage 请求失败: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Codex usage 读取失败: {e}"))?;
    if status.as_u16() == 401 {
        return Err("token_expired".into());
    }
    if !status.is_success() {
        return Err(format!("Codex usage 返回 {status}"));
    }
    decode_codex_usage(&body)
}

pub async fn fetch_codex_models(tokens: &CodexStoredTokens) -> Result<Vec<String>, String> {
    let url = format!("{CODEX_API_BASE}/models?client_version={CODEX_MODELS_CLIENT_VERSION}");
    let response = apply_codex_api_headers(http_client().get(&url), tokens)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Codex models 请求失败: {e}"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("Codex models 读取失败: {e}"))?;
    if status.as_u16() == 401 {
        return Err("token_expired".into());
    }
    if !status.is_success() {
        return Err(format!("Codex models 返回 {status}"));
    }
    decode_codex_models(&body)
}

/// True when the error indicates the access token must be refreshed / re-authed.
pub fn is_token_auth_error(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("token_expired")
        || e.contains("401")
        || e.contains("unauthorized")
        || e.contains("invalid_grant")
        || e.contains("refresh_token_reused")
        || e.contains("sign in again")
}

fn decode_codex_usage(body: &str) -> Result<CodexSubscriptionUsage, String> {
    let trimmed = body.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return Err("Codex usage payload 无效".into());
    }
    let value: Value =
        serde_json::from_str(trimmed).map_err(|e| format!("Codex usage 解析失败: {e}"))?;
    let plan_type = value
        .get("plan_type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.contains('@') && s.len() <= 64)
        .map(str::to_string);
    let rate = value.get("rate_limit");
    let limit_reached = rate
        .and_then(|r| r.get("limit_reached"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let primary = rate
        .and_then(|r| r.get("primary_window"))
        .and_then(decode_window);
    let secondary = rate
        .and_then(|r| r.get("secondary_window"))
        .and_then(decode_window);
    Ok(CodexSubscriptionUsage {
        plan_type,
        limit_reached,
        primary,
        secondary,
    })
}

fn decode_window(raw: &Value) -> Option<CodexRateLimitWindow> {
    let used = raw
        .get("used_percent")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .filter(|u| *u >= 0.0 && *u <= 1000.0)?;
    let limit_window_seconds = raw
        .get("limit_window_seconds")
        .and_then(Value::as_i64)
        .filter(|s| *s > 0);
    let reset_after_seconds = raw
        .get("reset_after_seconds")
        .and_then(Value::as_i64)
        .filter(|s| *s >= 0);
    let reset_at = raw
        .get("reset_at")
        .and_then(Value::as_i64)
        .filter(|s| *s > 0);
    Some(CodexRateLimitWindow {
        used_percent: used,
        limit_window_seconds,
        reset_after_seconds,
        reset_at,
    })
}

#[derive(Deserialize)]
struct OfficialCodexModel {
    slug: Option<String>,
    #[serde(default)]
    visibility: String,
}

fn decode_codex_models(body: &[u8]) -> Result<Vec<String>, String> {
    let value: Value =
        serde_json::from_slice(body).map_err(|e| format!("Codex models 解析失败: {e}"))?;
    if let Some(models) = value.get("models").and_then(Value::as_array) {
        let mut out = Vec::new();
        for item in models {
            let model: OfficialCodexModel =
                serde_json::from_value(item.clone()).unwrap_or(OfficialCodexModel {
                    slug: None,
                    visibility: String::new(),
                });
            let slug = model
                .slug
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(slug) = slug else { continue };
            let vis = model.visibility.trim().to_ascii_lowercase();
            if vis == "hide" || vis == "none" {
                continue;
            }
            out.push(slug.to_string());
        }
        out.sort();
        out.dedup();
        return Ok(out);
    }
    if let Some(data) = value.get("data").and_then(Value::as_array) {
        let mut out = Vec::new();
        for item in data {
            if let Some(id) = item
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                out.push(id.to_string());
            }
        }
        out.sort();
        out.dedup();
        return Ok(out);
    }
    Err("Codex models 缺少 models/data".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_account_like_astrlink() {
        assert_eq!(mask_account_hint("e259abcd46"), "e259***46");
        assert_eq!(mask_account_hint("short"), "s***");
    }

    #[test]
    fn decodes_usage_fixture() {
        let body = r#"{
            "plan_type": "free",
            "rate_limit": {
                "limit_reached": false,
                "primary_window": {
                    "used_percent": 0,
                    "limit_window_seconds": 2592000,
                    "reset_after_seconds": 2592000
                }
            }
        }"#;
        let usage = decode_codex_usage(body).unwrap();
        assert_eq!(usage.plan_type.as_deref(), Some("free"));
        assert_eq!(
            usage.primary.as_ref().unwrap().limit_window_seconds,
            Some(2_592_000)
        );
        assert_eq!(plan_type_label("free"), "Free");
        assert_eq!(window_label(Some(2_592_000), false, true), "30 天");
    }

    #[test]
    fn decodes_official_models() {
        let body = br#"{"models":[{"slug":"gpt-5","visibility":"listed"},{"slug":"hidden","visibility":"hide"}]}"#;
        let models = decode_codex_models(body).unwrap();
        assert_eq!(models, vec!["gpt-5".to_string()]);
    }
}
