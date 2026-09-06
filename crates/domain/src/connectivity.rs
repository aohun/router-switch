use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{
    extract_claude_api_key, extract_claude_base_url, extract_codex_api_key, extract_codex_base_url,
    extract_cursor_api_key, extract_cursor_base_url, extract_grok_api_key, extract_grok_base_url,
    extract_opencode_api_key, extract_opencode_base_url, extract_zcode_api_key,
    extract_zcode_base_url, Provider, ProviderSettings,
};

/// Health / Reachability status of a provider endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Operational,
    Degraded,
    Failed,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Operational | Self::Degraded)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Operational => "连通正常",
            Self::Degraded => "连通较慢",
            Self::Failed => "无法连通",
        }
    }
}

/// Configuration for connectivity checks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityCheckConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries on timeout-like transient errors
    pub max_retries: u32,
    /// Latency threshold in milliseconds after which status is marked as Degraded
    pub degraded_threshold_ms: u64,
}

impl Default for ConnectivityCheckConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 8,
            max_retries: 1,
            degraded_threshold_ms: 3000,
        }
    }
}

/// Result of a provider connectivity / reachability check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityCheckResult {
    pub status: HealthStatus,
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub http_status: Option<u16>,
    pub tested_at: i64,
    pub retry_count: u32,
}

/// Extract base_url and optional api_key for connectivity probing from a Provider
pub fn extract_provider_probe_target(
    provider: &Provider,
) -> Result<(String, Option<String>), String> {
    match &provider.settings {
        ProviderSettings::Codex(s) => {
            if s.kind.is_official() {
                Ok(("https://api.openai.com/v1".to_string(), None))
            } else {
                let url = extract_codex_base_url(&s.config_toml)
                    .ok_or_else(|| "Codex 服务商缺少 base_url".to_string())?;
                let key = extract_codex_api_key(&s.auth);
                Ok((url, key))
            }
        }
        ProviderSettings::Claude(s) => {
            if s.kind.is_official() {
                Ok(("https://api.anthropic.com".to_string(), None))
            } else {
                let url = extract_claude_base_url(&s.env)
                    .ok_or_else(|| "Claude 服务商缺少 base_url".to_string())?;
                let key = extract_claude_api_key(&s.env);
                Ok((url, key))
            }
        }
        ProviderSettings::Grok(s) => {
            if s.kind.is_official() {
                Ok(("https://api.x.ai/v1".to_string(), None))
            } else {
                let url = extract_grok_base_url(&s.config_toml)
                    .ok_or_else(|| "Grok 服务商缺少 base_url".to_string())?;
                let key = extract_grok_api_key(&s.config_toml);
                Ok((url, key))
            }
        }
        ProviderSettings::OpenCode(s) => {
            if s.kind.is_official() {
                Ok(("https://opencode.ai".to_string(), None))
            } else {
                let url = extract_opencode_base_url(&s.options)
                    .ok_or_else(|| "OpenCode 服务商缺少 options.baseURL".to_string())?;
                let key = extract_opencode_api_key(&s.options);
                Ok((url, key))
            }
        }
        ProviderSettings::Pi(s) => {
            if s.kind.is_official() {
                Ok(("https://pi.dev".to_string(), None))
            } else {
                let url = if !s.base_url.trim().is_empty() {
                    s.base_url.trim().to_string()
                } else {
                    return Err("Pi 服务商缺少 base_url".to_string());
                };
                let key = if !s.api_key.trim().is_empty() {
                    Some(s.api_key.trim().to_string())
                } else {
                    None
                };
                Ok((url, key))
            }
        }
        ProviderSettings::Cursor(s) => {
            if s.kind.is_official() {
                Ok(("https://api2.cursor.sh".to_string(), None))
            } else {
                let url = extract_cursor_base_url(s)
                    .ok_or_else(|| "Cursor 服务商缺少 base_url".to_string())?;
                let key = extract_cursor_api_key(s);
                Ok((url, key))
            }
        }
        ProviderSettings::ZCode(s) => {
            if s.kind.is_official() {
                Ok(("https://zcode.z.ai".to_string(), None))
            } else {
                let url = extract_zcode_base_url(&s.options)
                    .ok_or_else(|| "ZCode 服务商缺少 options.baseURL".to_string())?;
                let key = extract_zcode_api_key(&s.options);
                Ok((url, key))
            }
        }
        ProviderSettings::WorkBuddy(s) => {
            if s.kind.is_official() {
                Ok(("https://workbuddy.cn".to_string(), None))
            } else {
                if s.url.trim().is_empty() {
                    return Err("WorkBuddy 模型缺少接口地址 (url)".to_string());
                }
                let key = if !s.api_key.trim().is_empty() {
                    Some(s.api_key.trim().to_string())
                } else {
                    None
                };
                Ok((s.url.trim().to_string(), key))
            }
        }
        ProviderSettings::Unsupported { app } => {
            Err(format!("暂不支持应用 {} 的连通性测试", app.display_name()))
        }
    }
}

/// Perform reachability test with retry on timeout
pub fn check_reachability_with_retry(
    base_url: &str,
    api_key: Option<&str>,
    config: &ConnectivityCheckConfig,
) -> ConnectivityCheckResult {
    let trimmed_url = base_url.trim().trim_end_matches('/');
    if trimmed_url.is_empty() {
        return ConnectivityCheckResult {
            status: HealthStatus::Failed,
            success: false,
            message: "API 端点为空".to_string(),
            latency_ms: None,
            http_status: None,
            tested_at: now_unix_secs(),
            retry_count: 0,
        };
    }

    let target_url = if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://")
    {
        format!("https://{}", trimmed_url)
    } else {
        trimmed_url.to_string()
    };

    let mut last_result: Option<ConnectivityCheckResult> = None;
    for attempt in 0..=config.max_retries {
        let start = Instant::now();
        let mut req = ureq::get(&target_url)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .set("Accept", "*/*")
            .set("User-Agent", "RouterSwitch/0.1.2");

        if let Some(key) = api_key {
            let trimmed_key = key.trim();
            if !trimmed_key.is_empty() {
                req = req.set("Authorization", &format!("Bearer {trimmed_key}"));
                req = req.set("x-api-key", trimmed_key);
            }
        }

        match req.call() {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let status_code = response.status();
                let status = if latency_ms <= config.degraded_threshold_ms {
                    HealthStatus::Operational
                } else {
                    HealthStatus::Degraded
                };
                return ConnectivityCheckResult {
                    status,
                    success: true,
                    message: format!("HTTP {}", status_code),
                    latency_ms: Some(latency_ms),
                    http_status: Some(status_code),
                    tested_at: now_unix_secs(),
                    retry_count: attempt,
                };
            }
            Err(ureq::Error::Status(code, _resp)) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let status = if latency_ms <= config.degraded_threshold_ms {
                    HealthStatus::Operational
                } else {
                    HealthStatus::Degraded
                };
                return ConnectivityCheckResult {
                    status,
                    success: true,
                    message: format!("HTTP {}", code),
                    latency_ms: Some(latency_ms),
                    http_status: Some(code),
                    tested_at: now_unix_secs(),
                    retry_count: attempt,
                };
            }
            Err(ureq::Error::Transport(transport_err)) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let err_msg = transport_err.to_string();
                let is_timeout = err_msg.to_lowercase().contains("timeout")
                    || err_msg.to_lowercase().contains("timed out");

                let res = ConnectivityCheckResult {
                    status: HealthStatus::Failed,
                    success: false,
                    message: err_msg,
                    latency_ms: Some(latency_ms),
                    http_status: None,
                    tested_at: now_unix_secs(),
                    retry_count: attempt,
                };
                if is_timeout && attempt < config.max_retries {
                    last_result = Some(res);
                    continue;
                }
                return res;
            }
        }
    }

    last_result.unwrap_or_else(|| ConnectivityCheckResult {
        status: HealthStatus::Failed,
        success: false,
        message: "Check failed".to_string(),
        latency_ms: None,
        http_status: None,
        tested_at: now_unix_secs(),
        retry_count: config.max_retries,
    })
}

/// Perform reachability check for a provider
pub fn test_provider_connectivity(provider: &Provider) -> ConnectivityCheckResult {
    match extract_provider_probe_target(provider) {
        Ok((base_url, api_key)) => {
            let config = ConnectivityCheckConfig::default();
            check_reachability_with_retry(&base_url, api_key.as_deref(), &config)
        }
        Err(err) => ConnectivityCheckResult {
            status: HealthStatus::Failed,
            success: false,
            message: err,
            latency_ms: None,
            http_status: None,
            tested_at: now_unix_secs(),
            retry_count: 0,
        },
    }
}

/// Convenience function to test raw URL and Key
pub fn check_reachability(
    base_url: &str,
    api_key: Option<&str>,
    timeout_secs: u64,
) -> ConnectivityCheckResult {
    let mut config = ConnectivityCheckConfig::default();
    if timeout_secs > 0 {
        config.timeout_secs = timeout_secs;
    }
    check_reachability_with_retry(base_url, api_key, &config)
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate_claude_env, generate_grok_config_toml, generate_third_party_config, AppKind,
        ClaudeKind, ClaudeSettings, CodexKind, CodexSettings, GrokKind, GrokSettings, OpenCodeKind,
        OpenCodeSettings, PiKind, PiSettings, WorkBuddyKind, WorkBuddySettings, ZCodeKind,
        ZCodeSettings,
    };
    use serde_json::json;

    #[test]
    fn test_extract_probe_target_codex() {
        let p = Provider {
            id: "test-codex".into(),
            app: AppKind::Codex,
            name: "PackyCode".into(),
            website_url: None,
            settings: ProviderSettings::Codex(CodexSettings {
                kind: CodexKind::ResponsesThirdParty,
                auth: json!({ "OPENAI_API_KEY": "sk-mock-key-12345" }),
                config_toml: generate_third_party_config(
                    "PackyCode",
                    "https://api.example.com/v1",
                    "gpt-5.6-sol",
                ),
                request_protocol: String::new(),
                model_mappings: vec![],
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://api.example.com/v1");
        assert_eq!(key.as_deref(), Some("sk-mock-key-12345"));
    }

    #[test]
    fn test_extract_probe_target_claude() {
        let p = Provider {
            id: "test-claude".into(),
            app: AppKind::Claude,
            name: "Claude Relay".into(),
            website_url: None,
            settings: ProviderSettings::Claude(ClaudeSettings {
                kind: ClaudeKind::ThirdParty,
                env: generate_claude_env(
                    "sk-ant-test",
                    "https://relay.example.com",
                    "claude-3-7-sonnet",
                ),
                request_protocol: String::new(),
                model_mappings: vec![],
                desktop_mode: None,
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://relay.example.com");
        assert_eq!(key.as_deref(), Some("sk-ant-test"));
    }

    #[test]
    fn test_extract_probe_target_grok() {
        let p = Provider {
            id: "test-grok".into(),
            app: AppKind::Grok,
            name: "Grok Provider".into(),
            website_url: None,
            settings: ProviderSettings::Grok(GrokSettings {
                kind: GrokKind::ThirdParty,
                config_toml: generate_grok_config_toml(
                    "Grok Provider",
                    "xai-key",
                    "https://api.xai.example/v1",
                    "grok-4.5",
                ),
                request_protocol: String::new(),
                model_mappings: vec![],
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://api.xai.example/v1");
        assert_eq!(key.as_deref(), Some("xai-key"));
    }

    #[test]
    fn test_extract_probe_target_opencode() {
        let p = Provider {
            id: "test-opencode".into(),
            app: AppKind::OpenCode,
            name: "OpenCode Provider".into(),
            website_url: None,
            settings: ProviderSettings::OpenCode(OpenCodeSettings {
                kind: OpenCodeKind::ThirdParty,
                npm: "@ai-sdk/openai-compatible".into(),
                options: json!({ "baseURL": "https://opencode.example.com/v1", "apiKey": "sk-opencode-mock" }),
                models: json!({}),
                request_protocol: String::new(),
                model_mappings: vec![],
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://opencode.example.com/v1");
        assert_eq!(key.as_deref(), Some("sk-opencode-mock"));
    }

    #[test]
    fn test_extract_probe_target_pi() {
        let p = Provider {
            id: "test-pi".into(),
            app: AppKind::Pi,
            name: "Pi Provider".into(),
            website_url: None,
            settings: ProviderSettings::Pi(PiSettings {
                kind: PiKind::ThirdParty,
                api_type: "openai-completions".into(),
                base_url: "https://pi.example.com/v1".into(),
                api_key: "sk-pi-mock".into(),
                model: "gpt-4o".into(),
                request_protocol: String::new(),
                model_mappings: vec![],
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://pi.example.com/v1");
        assert_eq!(key.as_deref(), Some("sk-pi-mock"));
    }

    #[test]
    fn test_extract_probe_target_zcode() {
        let p = Provider {
            id: "test-zcode".into(),
            app: AppKind::ZCode,
            name: "ZCode Provider".into(),
            website_url: None,
            settings: ProviderSettings::ZCode(ZCodeSettings {
                kind: ZCodeKind::ThirdParty,
                provider_kind: "anthropic".into(),
                options: json!({ "baseURL": "https://zcode.example.com", "apiKey": "sk-zcode-mock" }),
                models: json!({}),
                model_mappings: vec![],
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://zcode.example.com");
        assert_eq!(key.as_deref(), Some("sk-zcode-mock"));
    }

    #[test]
    fn test_extract_probe_target_workbuddy() {
        let p = Provider {
            id: "test-workbuddy".into(),
            app: AppKind::WorkBuddy,
            name: "WorkBuddy Provider".into(),
            website_url: None,
            settings: ProviderSettings::WorkBuddy(WorkBuddySettings {
                kind: WorkBuddyKind::ThirdParty,
                model_id: "gemini-3.7-flash-high".into(),
                vendor: "Custom".into(),
                url: "https://cchost.example/v1".into(),
                api_key: "sk-wb-mock".into(),
                supports_tool_call: true,
                supports_images: true,
                supports_reasoning: false,
                reasoning_only: false,
                can_disable_reasoning: true,
                use_custom_protocol: false,
                request_protocol: String::new(),
                max_input_tokens: Some(262144),
                max_output_tokens: Some(65536),
                reasoning_effort: None,
                supported_reasoning_efforts: Vec::new(),
            }),
            created_at: 0,
            sort_index: 0,
        };
        let (url, key) = extract_provider_probe_target(&p).unwrap();
        assert_eq!(url, "https://cchost.example/v1");
        assert_eq!(key.as_deref(), Some("sk-wb-mock"));
    }

    #[test]
    fn test_empty_url_fails_gracefully() {
        let res = check_reachability("", None, 1);
        assert!(!res.success);
        assert_eq!(res.status, HealthStatus::Failed);
    }
}
