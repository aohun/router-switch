use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardProviderInfo {
    pub name: Option<String>,
    pub base_url: String,
    pub api_key: String,
    pub model: Option<String>,
    pub models: Vec<String>,
}

/// Attempts to parse clipboard text into structured provider information.
/// Supports NewAPI channel connection exports, standard JSON config objects,
/// and common provider connection strings.
pub fn parse_clipboard_provider_info(text: &str) -> Option<ClipboardProviderInfo> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip markdown code fences if present (```json ... ```)
    let json_text = if (trimmed.starts_with("```json") || trimmed.starts_with("```"))
        && trimmed.ends_with("```")
    {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 {
            lines[1..lines.len() - 1].join("\n")
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    };

    // 1. Try parsing as JSON
    if let Ok(value) = serde_json::from_str::<Value>(json_text.trim()) {
        if let Some(obj) = value.as_object() {
            // Check for key/api_key
            let key = obj
                .get("key")
                .or_else(|| obj.get("api_key"))
                .or_else(|| obj.get("apiKey"))
                .or_else(|| obj.get("token"))
                .or_else(|| obj.get("sk"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string());

            // Check for url/base_url/baseUrl/endpoint
            let url = obj
                .get("url")
                .or_else(|| obj.get("base_url"))
                .or_else(|| obj.get("baseUrl"))
                .or_else(|| obj.get("endpoint"))
                .or_else(|| obj.get("api_url"))
                .or_else(|| obj.get("apiUrl"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string());

            // Name
            let name = obj
                .get("name")
                .or_else(|| obj.get("channel_name"))
                .or_else(|| obj.get("channelName"))
                .or_else(|| obj.get("title"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // Single model or default_model
            let model = obj
                .get("model")
                .or_else(|| obj.get("default_model"))
                .or_else(|| obj.get("defaultModel"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // Models array
            let mut models = Vec::new();
            if let Some(models_val) = obj.get("models").or_else(|| obj.get("model_list")) {
                if let Some(arr) = models_val.as_array() {
                    for item in arr {
                        if let Some(m) = item.as_str() {
                            let trimmed_m = m.trim();
                            if !trimmed_m.is_empty() {
                                models.push(trimmed_m.to_string());
                            }
                        }
                    }
                } else if let Some(m_str) = models_val.as_str() {
                    for m in m_str.split(',') {
                        let trimmed_m = m.trim();
                        if !trimmed_m.is_empty() {
                            models.push(trimmed_m.to_string());
                        }
                    }
                }
            }

            if let (Some(api_key), Some(base_url)) = (key, url) {
                if !api_key.is_empty() && !base_url.is_empty() {
                    let inferred_name = name.or_else(|| infer_name_from_url(&base_url));
                    return Some(ClipboardProviderInfo {
                        name: inferred_name,
                        base_url,
                        api_key,
                        model,
                        models,
                    });
                }
            }
        }
    }

    None
}

/// Infers a friendly name from base_url, e.g. "https://api.example.com" -> "api.example.com"
fn infer_name_from_url(url: &str) -> Option<String> {
    let clean = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let host = clean.split('/').next().unwrap_or("").trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_newapi_channel_conn() {
        let text = r#"{"_type":"newapi_channel_conn","key":"sk-mock-channel-key-1234567890","url":"https://api.example.com"}"#;
        let info = parse_clipboard_provider_info(text).expect("should parse newapi conn");
        assert_eq!(info.api_key, "sk-mock-channel-key-1234567890");
        assert_eq!(info.base_url, "https://api.example.com");
        assert_eq!(info.name.as_deref(), Some("api.example.com"));
    }

    #[test]
    fn parses_newapi_with_name_and_models() {
        let text = r#"{
            "_type": "newapi_channel_conn",
            "name": "Custom Provider",
            "key": "sk-123456",
            "url": "https://api.example.com/v1",
            "model": "gpt-4o",
            "models": ["gpt-4o", "claude-3-5-sonnet", "deepseek-v3"]
        }"#;
        let info = parse_clipboard_provider_info(text).expect("should parse");
        assert_eq!(info.name.as_deref(), Some("Custom Provider"));
        assert_eq!(info.api_key, "sk-123456");
        assert_eq!(info.base_url, "https://api.example.com/v1");
        assert_eq!(info.model.as_deref(), Some("gpt-4o"));
        assert_eq!(info.models.len(), 3);
    }

    #[test]
    fn parses_markdown_fenced_json() {
        let text = "```json\n{\"apiKey\":\"sk-abc\",\"baseUrl\":\"https://api.openai.com/v1\"}\n```";
        let info = parse_clipboard_provider_info(text).expect("should parse fenced json");
        assert_eq!(info.api_key, "sk-abc");
        assert_eq!(info.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn returns_none_on_invalid() {
        assert_eq!(parse_clipboard_provider_info(""), None);
        assert_eq!(parse_clipboard_provider_info("hello world"), None);
        assert_eq!(parse_clipboard_provider_info(r#"{"other": "value"}"#), None);
    }
}
