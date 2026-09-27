//! Gateway-centric API providers (AstrLink ServiceManager alignment).
//! Distinct from per-app Live `Provider` rows synced into ~/.codex etc.

use serde::{Deserialize, Serialize};

/// AstrLink selector group for the create-form kind dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiProviderKindGroup {
    Subscription,
    Gateway,
    PayAsYouGo,
    Advanced,
}

/// AstrLink-compatible provider kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiProviderKind {
    // Subscriptions
    CodexSubscription,
    ClaudeSubscription,
    GrokSubscription,
    // Coding plans (shown under Subscription group in AstrLink)
    OpencodeGo,
    KimiCoding,
    GlmCoding,
    MinimaxCoding,
    // Gateway
    Newapi,
    // Pay-as-you-go
    OpencodeZen,
    Openai,
    Anthropic,
    Gemini,
    Deepseek,
    Qwen,
    Moonshot,
    Glm,
    Minimax,
    Doubao,
    Xai,
    // Advanced
    OpenaiCompatible,
    Custom,
}

impl ApiProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CodexSubscription => "codex_subscription",
            Self::ClaudeSubscription => "claude_subscription",
            Self::GrokSubscription => "grok_subscription",
            Self::OpencodeGo => "opencode_go",
            Self::KimiCoding => "kimi_coding",
            Self::GlmCoding => "glm_coding",
            Self::MinimaxCoding => "minimax_coding",
            Self::Newapi => "newapi",
            Self::OpencodeZen => "opencode_zen",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Deepseek => "deepseek",
            Self::Qwen => "qwen",
            Self::Moonshot => "moonshot",
            Self::Glm => "glm",
            Self::Minimax => "minimax",
            Self::Doubao => "doubao",
            Self::Xai => "xai",
            Self::OpenaiCompatible => "openai_compatible",
            Self::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "codex_subscription" => Self::CodexSubscription,
            "claude_subscription" => Self::ClaudeSubscription,
            "grok_subscription" => Self::GrokSubscription,
            "opencode_go" => Self::OpencodeGo,
            "kimi_coding" => Self::KimiCoding,
            "glm_coding" => Self::GlmCoding,
            "minimax_coding" => Self::MinimaxCoding,
            "newapi" => Self::Newapi,
            "opencode_zen" => Self::OpencodeZen,
            "openai" => Self::Openai,
            "anthropic" => Self::Anthropic,
            "gemini" => Self::Gemini,
            "deepseek" => Self::Deepseek,
            "qwen" => Self::Qwen,
            "moonshot" => Self::Moonshot,
            "glm" => Self::Glm,
            "minimax" => Self::Minimax,
            "doubao" => Self::Doubao,
            "xai" => Self::Xai,
            "openai_compatible" => Self::OpenaiCompatible,
            "custom" => Self::Custom,
            _ => return None,
        })
    }

    pub fn is_subscription(self) -> bool {
        matches!(
            self,
            Self::CodexSubscription | Self::ClaudeSubscription | Self::GrokSubscription
        )
    }

    /// AstrLink dropdown group (coding plans live under Subscription).
    pub fn kind_group(self) -> ApiProviderKindGroup {
        match self {
            Self::CodexSubscription
            | Self::ClaudeSubscription
            | Self::GrokSubscription
            | Self::OpencodeGo
            | Self::KimiCoding
            | Self::GlmCoding
            | Self::MinimaxCoding => ApiProviderKindGroup::Subscription,
            Self::Newapi => ApiProviderKindGroup::Gateway,
            Self::OpenaiCompatible | Self::Custom => ApiProviderKindGroup::Advanced,
            _ => ApiProviderKindGroup::PayAsYouGo,
        }
    }

    /// Brand asset id for UI icons (`assets::brand_icon_path` / svg brands).
    pub fn brand_id(self) -> &'static str {
        match self {
            Self::CodexSubscription | Self::Openai | Self::OpenaiCompatible => "codex",
            Self::ClaudeSubscription | Self::Anthropic => "claude",
            Self::GrokSubscription | Self::Xai => "grok",
            Self::OpencodeGo | Self::OpencodeZen => "opencode",
            Self::KimiCoding | Self::Moonshot => "kimi",
            Self::Gemini => "gemini",
            Self::Deepseek => "deepseek",
            Self::Newapi => "newapi",
            Self::Qwen => "qwen",
            Self::GlmCoding | Self::Glm => "glm",
            Self::MinimaxCoding | Self::Minimax => "minimax",
            Self::Doubao => "doubao",
            Self::Custom => "custom",
        }
    }

    pub fn display_zh(self) -> &'static str {
        match self {
            Self::CodexSubscription => "Codex 订阅（OpenAI OAuth）",
            Self::ClaudeSubscription => "Claude Code 订阅",
            Self::GrokSubscription => "Grok 订阅（xAI OAuth）",
            Self::OpencodeGo => "OpenCode Go",
            Self::KimiCoding => "Kimi Coding",
            Self::GlmCoding => "GLM Coding Plan",
            Self::MinimaxCoding => "MiniMax Coding Plan",
            Self::Newapi => "New API",
            Self::OpencodeZen => "OpenCode Zen",
            Self::Openai => "OpenAI API",
            Self::Anthropic => "Anthropic API",
            Self::Gemini => "Gemini API",
            Self::Deepseek => "DeepSeek API",
            Self::Qwen => "通义千问（百炼） API",
            Self::Moonshot => "Kimi（Moonshot） API",
            Self::Glm => "智谱 GLM API",
            Self::Minimax => "MiniMax API",
            Self::Doubao => "豆包（火山方舟） API",
            Self::Xai => "xAI（Grok） API",
            Self::OpenaiCompatible => "OpenAI 兼容",
            Self::Custom => "自定义 API",
        }
    }

    pub fn display_en(self) -> &'static str {
        match self {
            Self::CodexSubscription => "Codex Subscription (OpenAI OAuth)",
            Self::ClaudeSubscription => "Claude Code Subscription",
            Self::GrokSubscription => "Grok Subscription (xAI OAuth)",
            Self::OpencodeGo => "OpenCode Go",
            Self::KimiCoding => "Kimi Coding",
            Self::GlmCoding => "GLM Coding Plan",
            Self::MinimaxCoding => "MiniMax Coding Plan",
            Self::Newapi => "New API",
            Self::OpencodeZen => "OpenCode Zen",
            Self::Openai => "OpenAI API",
            Self::Anthropic => "Anthropic API",
            Self::Gemini => "Gemini API",
            Self::Deepseek => "DeepSeek API",
            Self::Qwen => "Qwen (Bailian) API",
            Self::Moonshot => "Kimi (Moonshot) API",
            Self::Glm => "Zhipu GLM API",
            Self::Minimax => "MiniMax API",
            Self::Doubao => "Doubao (Volcengine) API",
            Self::Xai => "xAI (Grok) API",
            Self::OpenaiCompatible => "OpenAI Compatible",
            Self::Custom => "Custom API",
        }
    }

    pub fn hint_zh(self) -> &'static str {
        match self {
            Self::CodexSubscription => "使用 Codex 官方公开 OAuth 客户端接入真实订阅账户。",
            Self::ClaudeSubscription => "使用 Claude Code 官方 OAuth 接入订阅账户。",
            Self::GrokSubscription => {
                "使用 Grok CLI 公开 OAuth 客户端接入订阅账号，仅支持 Device Code。"
            }
            Self::OpencodeGo | Self::KimiCoding | Self::GlmCoding | Self::MinimaxCoding => {
                "Coding Plan：按厂商文档配置 Base URL 与 API Key。"
            }
            Self::Newapi => "对接 New API / One API 等网关。",
            Self::OpenaiCompatible | Self::Custom => "自定义 OpenAI 兼容或任意 HTTP 上游。",
            _ => "按量付费：配置官方或中转地址与 API Key。",
        }
    }

    pub fn hint_en(self) -> &'static str {
        match self {
            Self::CodexSubscription => {
                "Connect a real Codex subscription via the official public OAuth client."
            }
            Self::ClaudeSubscription => "Connect a Claude Code subscription via official OAuth.",
            Self::GrokSubscription => "Connect a Grok subscription via Device Code OAuth only.",
            Self::OpencodeGo | Self::KimiCoding | Self::GlmCoding | Self::MinimaxCoding => {
                "Coding Plan: configure Base URL and API key per vendor docs."
            }
            Self::Newapi => "Connect New API / One API style gateways.",
            Self::OpenaiCompatible | Self::Custom => {
                "Custom OpenAI-compatible or arbitrary HTTP upstream."
            }
            _ => "Pay-as-you-go: configure official or proxy endpoint and API key.",
        }
    }

    pub fn supports_responses_websocket(self) -> bool {
        matches!(
            self,
            Self::CodexSubscription | Self::Openai | Self::OpenaiCompatible | Self::Xai
        )
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Self::OpencodeGo => Some("https://opencode.ai/zen/go/v1"),
            Self::OpencodeZen => Some("https://opencode.ai/zen/v1"),
            Self::KimiCoding => Some("https://api.kimi.ai/coding"),
            Self::GlmCoding => Some("https://open.bigmodel.cn/api/anthropic"),
            Self::MinimaxCoding => Some("https://api.minimax.cn/anthropic"),
            Self::Openai => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com"),
            Self::Deepseek => Some("https://api.deepseek.com/v1"),
            Self::Qwen => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            Self::Moonshot => Some("https://api.moonshot.cn/v1"),
            Self::Glm => Some("https://open.bigmodel.cn/api/paas/v4"),
            Self::Minimax => Some("https://api.minimax.cn/v1"),
            Self::Doubao => Some("https://ark.cn-beijing.volces.com/api/v3"),
            Self::Xai => Some("https://api.x.ai/v1"),
            Self::OpenaiCompatible => Some("https://api.example.com/v1"),
            _ => None,
        }
    }

    pub fn default_auth_scheme(self) -> ApiAuthScheme {
        match self {
            Self::Anthropic | Self::GlmCoding | Self::MinimaxCoding => {
                ApiAuthScheme::AnthropicApiKey
            }
            Self::Gemini => ApiAuthScheme::GoogleApiKey,
            Self::CodexSubscription | Self::ClaudeSubscription | Self::GrokSubscription => {
                ApiAuthScheme::None
            }
            _ => ApiAuthScheme::Bearer,
        }
    }

    pub fn default_capabilities(self) -> Vec<ApiCapability> {
        match self {
            Self::Anthropic | Self::GlmCoding | Self::MinimaxCoding => vec![ApiCapability {
                protocol: "anthropic.messages".into(),
                mode: "native".into(),
                streaming: true,
                convert_to: None,
            }],
            Self::Gemini => vec![ApiCapability {
                protocol: "google.generate_content".into(),
                mode: "native".into(),
                streaming: true,
                convert_to: None,
            }],
            Self::Custom => Vec::new(),
            Self::CodexSubscription | Self::ClaudeSubscription | Self::GrokSubscription => {
                vec![ApiCapability {
                    protocol: match self {
                        Self::ClaudeSubscription => "anthropic.messages",
                        Self::GrokSubscription => "openai.responses",
                        _ => "openai.responses",
                    }
                    .into(),
                    mode: "native".into(),
                    streaming: true,
                    convert_to: None,
                }]
            }
            _ => vec![
                ApiCapability {
                    protocol: "openai.chat".into(),
                    mode: "native".into(),
                    streaming: true,
                    convert_to: None,
                },
                ApiCapability {
                    protocol: "openai.models".into(),
                    mode: "native".into(),
                    streaming: false,
                    convert_to: None,
                },
            ],
        }
    }

    /// Ordered presets for the create-form kind selector (AstrLink order).
    pub fn all_presets() -> &'static [ApiProviderKind] {
        &[
            // Subscription
            Self::CodexSubscription,
            Self::ClaudeSubscription,
            Self::GrokSubscription,
            Self::OpencodeGo,
            Self::KimiCoding,
            Self::GlmCoding,
            Self::MinimaxCoding,
            // Gateway
            Self::Newapi,
            // Pay-as-you-go
            Self::OpencodeZen,
            Self::Openai,
            Self::Anthropic,
            Self::Gemini,
            Self::Deepseek,
            Self::Qwen,
            Self::Moonshot,
            Self::Glm,
            Self::Minimax,
            Self::Doubao,
            Self::Xai,
            // Advanced
            Self::OpenaiCompatible,
            Self::Custom,
        ]
    }

    /// Preset groups for the kind dropdown (title keys live in UI i18n).
    pub fn preset_groups() -> &'static [(ApiProviderKindGroup, &'static [ApiProviderKind])] {
        &[
            (
                ApiProviderKindGroup::Subscription,
                &[
                    Self::CodexSubscription,
                    Self::ClaudeSubscription,
                    Self::GrokSubscription,
                    Self::OpencodeGo,
                    Self::KimiCoding,
                    Self::GlmCoding,
                    Self::MinimaxCoding,
                ],
            ),
            (ApiProviderKindGroup::Gateway, &[Self::Newapi]),
            (
                ApiProviderKindGroup::PayAsYouGo,
                &[
                    Self::OpencodeZen,
                    Self::Openai,
                    Self::Anthropic,
                    Self::Gemini,
                    Self::Deepseek,
                    Self::Qwen,
                    Self::Moonshot,
                    Self::Glm,
                    Self::Minimax,
                    Self::Doubao,
                    Self::Xai,
                ],
            ),
            (
                ApiProviderKindGroup::Advanced,
                &[Self::OpenaiCompatible, Self::Custom],
            ),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiAuthScheme {
    #[default]
    None,
    Bearer,
    AnthropicApiKey,
    GoogleApiKey,
    CustomHeader,
}

impl ApiAuthScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bearer => "bearer",
            Self::AnthropicApiKey => "anthropic_api_key",
            Self::GoogleApiKey => "google_api_key",
            Self::CustomHeader => "custom_header",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiCapability {
    pub protocol: String,
    pub mode: String,
    pub streaming: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    #[default]
    Disconnected,
    Authorizing,
    Connected,
    NeedsReauth,
    Error,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Authorizing => "authorizing",
            Self::Connected => "connected",
            Self::NeedsReauth => "needs_reauth",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpConnection {
    pub base_url: String,
    pub auth_scheme: ApiAuthScheme,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Stored API key (local DB only; never commit real keys).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscriptionConnection {
    pub status: SubscriptionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProvider {
    pub id: String,
    pub name: String,
    pub kind: ApiProviderKind,
    pub enabled: bool,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<ApiCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConnection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription: Option<SubscriptionConnection>,
    /// Codex / OpenAI Responses WebSocket (AstrLink alignment).
    #[serde(default)]
    pub responses_websocket: bool,
    /// `inherit` | `off` | `custom` — UI-first; gateway wiring later.
    #[serde(default = "default_proxy_mode")]
    pub proxy_mode: String,
    pub sort_index: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

fn default_proxy_mode() -> String {
    "inherit".into()
}

impl ApiProvider {
    pub fn new_from_kind(kind: ApiProviderKind, name: impl Into<String>, now_ms: i64) -> Self {
        let name = name.into();
        let id = format!("api_{}_{}", kind.as_str(), &uuid_like(now_ms));
        let http = if kind.is_subscription() {
            None
        } else {
            Some(HttpConnection {
                base_url: kind.default_base_url().unwrap_or("").to_string(),
                auth_scheme: kind.default_auth_scheme(),
                header_name: None,
                api_key: None,
            })
        };
        let subscription = if kind.is_subscription() {
            Some(SubscriptionConnection::default())
        } else {
            None
        };
        Self {
            id,
            name,
            kind,
            enabled: true,
            models: Vec::new(),
            capabilities: kind.default_capabilities(),
            http,
            subscription,
            responses_websocket: matches!(kind, ApiProviderKind::CodexSubscription),
            proxy_mode: "inherit".into(),
            sort_index: now_ms,
            created_at: now_ms,
            updated_at: now_ms,
        }
    }

    pub fn kind_label(&self, zh: bool) -> &'static str {
        if zh {
            self.kind.display_zh()
        } else {
            self.kind.display_en()
        }
    }

    pub fn connection_hint(&self) -> String {
        if let Some(sub) = &self.subscription {
            if let Some(hint) = &sub.account_hint {
                if !hint.is_empty() {
                    return hint.clone();
                }
            }
            return match sub.status {
                SubscriptionStatus::Connected => "OAuth 已连接".into(),
                SubscriptionStatus::Authorizing => "授权中…".into(),
                SubscriptionStatus::NeedsReauth => "需要重新登录".into(),
                SubscriptionStatus::Error => {
                    sub.last_error.clone().unwrap_or_else(|| "连接错误".into())
                }
                SubscriptionStatus::Disconnected => "未登录".into(),
            };
        }
        self.http
            .as_ref()
            .map(|h| h.base_url.clone())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| "—".into())
    }

    pub fn matches_query(&self, q: &str) -> bool {
        if q.is_empty() {
            return true;
        }
        let q = q.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.kind.as_str().contains(&q)
            || self.kind.display_zh().to_lowercase().contains(&q)
            || self.kind.display_en().to_lowercase().contains(&q)
            || self.connection_hint().to_lowercase().contains(&q)
            || self.models.iter().any(|m| m.to_lowercase().contains(&q))
    }

    pub fn matches_model_filter(&self, model: &str) -> bool {
        if model.is_empty() {
            return true;
        }
        let m = model.to_lowercase();
        self.models.iter().any(|x| x.to_lowercase().contains(&m))
    }
}

fn uuid_like(now_ms: i64) -> String {
    format!("{:x}", now_ms.wrapping_mul(1_000_000_007) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_roundtrip() {
        for k in ApiProviderKind::all_presets() {
            assert_eq!(ApiProviderKind::from_str(k.as_str()), Some(*k));
        }
    }

    #[test]
    fn new_http_provider_has_defaults() {
        let p = ApiProvider::new_from_kind(ApiProviderKind::Openai, "OpenAI", 1);
        assert!(p.enabled);
        assert!(p.http.is_some());
        assert!(p.subscription.is_none());
        assert!(!p.capabilities.is_empty());
        let zen = ApiProvider::new_from_kind(ApiProviderKind::OpencodeZen, "Zen", 2);
        assert_eq!(zen.kind.as_str(), "opencode_zen");
        assert_eq!(
            zen.http.as_ref().map(|h| h.base_url.as_str()),
            Some("https://opencode.ai/zen/v1")
        );
    }
}
