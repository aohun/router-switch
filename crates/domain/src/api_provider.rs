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
            Self::Anthropic | Self::GlmCoding | Self::MinimaxCoding => {
                vec![ApiCapability::native("anthropic.messages", true)]
            }
            Self::Gemini => vec![ApiCapability::native("google.generate_content", true)],
            // Custom + OpenAI Compatible: chat / legacy completions / models (AstrLink defaults).
            Self::Custom | Self::OpenaiCompatible => Self::openai_compatible_default_capabilities(),
            Self::CodexSubscription => Self::codex_native_capabilities(),
            Self::ClaudeSubscription => Self::claude_native_capabilities(),
            Self::GrokSubscription => Self::grok_native_capabilities(),
            _ => vec![
                ApiCapability::native("openai.chat", true),
                ApiCapability::native("openai.models", false),
            ],
        }
    }

    /// Default ingress protocols for Custom / OpenAI Compatible providers.
    pub fn openai_compatible_default_capabilities() -> Vec<ApiCapability> {
        vec![
            ApiCapability::native("openai.chat", true),
            ApiCapability::native("openai.completions", true),
            ApiCapability::native("openai.models", false),
        ]
    }

    /// Fixed native capabilities for Codex subscription (AstrLink openai_codex).
    pub fn codex_native_capabilities() -> Vec<ApiCapability> {
        vec![
            ApiCapability::native("openai.responses", true),
            ApiCapability::native("openai.responses.compact", false),
            ApiCapability::native("openai.models", false),
        ]
    }

    pub fn claude_native_capabilities() -> Vec<ApiCapability> {
        vec![
            ApiCapability::native("anthropic.messages", true),
            ApiCapability::native("openai.models", false),
        ]
    }

    pub fn grok_native_capabilities() -> Vec<ApiCapability> {
        vec![
            ApiCapability::native("openai.responses", true),
            ApiCapability::native("openai.chat", true),
            ApiCapability::native("openai.models", false),
        ]
    }

    /// Native protocol IDs locked on for this subscription kind (if any).
    pub fn subscription_native_protocol_ids(self) -> &'static [&'static str] {
        match self {
            Self::CodexSubscription => &[
                "openai.responses",
                "openai.responses.compact",
                "openai.models",
            ],
            Self::ClaudeSubscription => &["anthropic.messages", "openai.models"],
            Self::GrokSubscription => &["openai.responses", "openai.chat", "openai.models"],
            _ => &[],
        }
    }

    /// Upstream protocols a local conversion may emit for this subscription.
    pub fn subscription_conversion_targets(self) -> &'static [&'static str] {
        match self {
            Self::CodexSubscription => &["openai.responses"],
            Self::ClaudeSubscription => &["anthropic.messages"],
            Self::GrokSubscription => &["openai.responses", "openai.chat"],
            _ => &[],
        }
    }

    /// Rows shown on the protocols tab (natives ∪ convertible for subscriptions).
    pub fn protocol_editor_rows(self) -> Vec<&'static ApiProtocolDescriptor> {
        let natives: std::collections::HashSet<&str> = self
            .subscription_native_protocol_ids()
            .iter()
            .copied()
            .collect();
        if self.is_subscription() && !natives.is_empty() {
            return API_PROTOCOL_CATALOG
                .iter()
                .filter(|d| natives.contains(d.id) || supports_local_conversion(d.id))
                .collect();
        }
        // HTTP / coding-plan / gateway: show catalog entries that match defaults
        // plus common OpenAI-compatible set.
        let defaults: std::collections::HashSet<String> = self
            .default_capabilities()
            .into_iter()
            .map(|c| c.protocol)
            .collect();
        API_PROTOCOL_CATALOG
            .iter()
            .filter(|d| {
                defaults.contains(d.id)
                    || supports_local_conversion(d.id)
                    || d.id == "openai.models"
                    || d.id == "openai.completions"
            })
            .collect()
    }

    /// Ensure locked natives are present on a subscription provider's capability list.
    pub fn ensure_subscription_natives(self, capabilities: &mut Vec<ApiCapability>) {
        let natives = match self {
            Self::CodexSubscription => Self::codex_native_capabilities(),
            Self::ClaudeSubscription => Self::claude_native_capabilities(),
            Self::GrokSubscription => Self::grok_native_capabilities(),
            _ => return,
        };
        for native in natives {
            if !capabilities.iter().any(|c| c.protocol == native.protocol) {
                capabilities.push(native);
            }
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

impl ApiCapability {
    pub fn native(protocol: impl Into<String>, streaming: bool) -> Self {
        Self {
            protocol: protocol.into(),
            mode: "native".into(),
            streaming,
            convert_to: None,
        }
    }

    pub fn converted(
        protocol: impl Into<String>,
        streaming: bool,
        convert_to: impl Into<String>,
    ) -> Self {
        Self {
            protocol: protocol.into(),
            mode: "native".into(),
            streaming,
            convert_to: Some(convert_to.into()),
        }
    }
}

/// Descriptor for an ingress protocol row (AstrLink ProtocolDescriptor + labels/paths).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiProtocolDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub entry_path: &'static str,
    pub streaming: bool,
}

/// All known ingress protocols shown in the protocols editor.
pub const API_PROTOCOL_CATALOG: &[ApiProtocolDescriptor] = &[
    ApiProtocolDescriptor {
        id: "openai.responses",
        label: "OpenAI Responses",
        entry_path: "/v1/responses",
        streaming: true,
    },
    ApiProtocolDescriptor {
        id: "openai.responses.compact",
        label: "Responses Compact",
        entry_path: "/v1/responses/compact",
        streaming: false,
    },
    ApiProtocolDescriptor {
        id: "anthropic.messages",
        label: "Anthropic Messages",
        entry_path: "/v1/messages",
        streaming: true,
    },
    ApiProtocolDescriptor {
        id: "google.generate_content",
        label: "Gemini Generate Content",
        entry_path: "/v1beta/models/:model:generateContent",
        streaming: true,
    },
    ApiProtocolDescriptor {
        id: "openai.chat",
        label: "OpenAI Chat Completions",
        entry_path: "/v1/chat/completions",
        streaming: true,
    },
    ApiProtocolDescriptor {
        id: "openai.completions",
        label: "OpenAI Legacy Completions",
        entry_path: "/v1/completions",
        streaming: true,
    },
    ApiProtocolDescriptor {
        id: "openai.models",
        label: "OpenAI Models",
        entry_path: "/v1/models",
        streaming: false,
    },
    ApiProtocolDescriptor {
        id: "google.models",
        label: "Gemini Models",
        entry_path: "/v1beta/models",
        streaming: false,
    },
];

pub fn api_protocol_descriptor(id: &str) -> Option<&'static ApiProtocolDescriptor> {
    API_PROTOCOL_CATALOG.iter().find(|d| d.id == id)
}

/// Protocols the local conversion engine can bridge (AstrLink convertibleProtocolIDs).
pub const CONVERTIBLE_PROTOCOL_IDS: &[&str] = &[
    "openai.responses",
    "anthropic.messages",
    "google.generate_content",
    "openai.chat",
];

pub fn supports_local_conversion(protocol_id: &str) -> bool {
    CONVERTIBLE_PROTOCOL_IDS.contains(&protocol_id)
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

    /// Exact model allow-list for routing. Empty `models` means wildcard (any model).
    pub fn supports_model(&self, model: &str) -> bool {
        let model = model.trim();
        if model.is_empty() || self.models.is_empty() {
            return true;
        }
        self.models.iter().any(|m| m == model)
    }

    /// Whether this provider exposes an usable capability for `protocol`.
    pub fn supports_protocol(&self, protocol: &str) -> bool {
        let protocol = protocol.trim();
        if protocol.is_empty() {
            return true;
        }
        self.capabilities.iter().any(|c| {
            c.protocol == protocol && (!c.mode.trim().is_empty() || c.convert_to.is_some())
        })
    }
}

/// Merge a filtered reorder into the global priority list (AstrLink useServiceOrder).
///
/// Only slots belonging to `visible_reordered` are rewritten; other IDs keep position.
/// Returns `None` when the visible list is too short, has duplicates, or unknown IDs.
pub fn merge_visible_api_provider_order(
    global_ids: &[String],
    visible_reordered: &[String],
) -> Option<Vec<String>> {
    if visible_reordered.len() < 2 {
        return None;
    }
    let mut seen = std::collections::HashSet::with_capacity(visible_reordered.len());
    for id in visible_reordered {
        if !seen.insert(id.as_str()) {
            return None;
        }
        if !global_ids.iter().any(|g| g == id) {
            return None;
        }
    }
    let selected = seen;
    let mut index = 0usize;
    let merged: Vec<String> = global_ids
        .iter()
        .map(|id| {
            if selected.contains(id.as_str()) {
                let next = visible_reordered[index].clone();
                index += 1;
                next
            } else {
                id.clone()
            }
        })
        .collect();
    if index != visible_reordered.len() {
        return None;
    }
    Some(merged)
}

/// Resolve enabled upstream candidates in list order (AstrLink defaultCandidates).
///
/// - Providers are sorted by `sort_index` ascending (stable by id).
/// - Disabled providers are skipped (but still occupy sort slots in the list).
/// - `model`: exact match, or empty models list = wildcard.
/// - `protocol`: requires a matching capability with a usable mode.
pub fn resolve_upstream_candidates<'a>(
    providers: &'a [ApiProvider],
    model: Option<&str>,
    protocol: Option<&str>,
) -> Vec<&'a ApiProvider> {
    let mut ordered: Vec<&ApiProvider> = providers.iter().collect();
    ordered.sort_by(|a, b| {
        a.sort_index
            .cmp(&b.sort_index)
            .then_with(|| a.id.cmp(&b.id))
    });
    ordered
        .into_iter()
        .filter(|p| p.enabled)
        .filter(|p| model.map(|m| p.supports_model(m)).unwrap_or(true))
        .filter(|p| {
            protocol
                .map(|proto| p.supports_protocol(proto))
                .unwrap_or(true)
        })
        .collect()
}

fn uuid_like(now_ms: i64) -> String {
    format!("{:x}", now_ms.wrapping_mul(1_000_000_007) as u64)
}

impl ApiProviderKind {
    /// HTTP kinds that support AstrLink-style `GET /v1/models` discovery.
    pub fn supports_http_model_probe(self) -> bool {
        matches!(self, Self::OpenaiCompatible | Self::Custom)
    }
}

/// Merge current allow-list with discovered IDs (AstrLink `mergeDiscoveredServiceModels`).
pub fn merge_discovered_api_models(
    current: &[String],
    discovered: &[String],
) -> Option<Vec<String>> {
    let mut models: Vec<String> = current
        .iter()
        .cloned()
        .chain(discovered.iter().cloned())
        .collect();
    models.sort();
    models.dedup();
    if models.len() > 2_000 {
        return None;
    }
    Some(models)
}

/// Probe OpenAI-compatible `GET {base}/v1/models` (AstrLink `ProbeHTTP` / openai.models).
pub fn fetch_http_connection_models(http: &HttpConnection) -> Result<Vec<String>, String> {
    let trimmed_base = http.base_url.trim().trim_end_matches('/');
    if trimmed_base.is_empty() {
        return Err("API 地址为空".into());
    }

    let candidate_urls = if trimmed_base.ends_with("/v1") || trimmed_base.ends_with("/v1beta") {
        vec![
            format!("{trimmed_base}/models"),
            format!("{trimmed_base}/v1/models"),
        ]
    } else {
        vec![
            format!("{trimmed_base}/v1/models"),
            format!("{trimmed_base}/models"),
        ]
    };

    let mut last_error = String::new();
    for url in candidate_urls {
        let mut req = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .set("Accept", "application/json");
        req = apply_http_auth_headers(req, http);
        match req.call() {
            Ok(resp) => {
                let status = resp.status();
                if !(200..300).contains(&status) {
                    last_error = format!("HTTP {status}");
                    continue;
                }
                match resp.into_json::<serde_json::Value>() {
                    Ok(json_val) => {
                        let models = parse_openai_models_payload(&json_val);
                        if !models.is_empty() {
                            return Ok(models);
                        }
                        last_error = "服务商未返回可用模型列表或响应格式不兼容".into();
                    }
                    Err(e) => last_error = format!("解析响应失败: {e}"),
                }
            }
            Err(e) => last_error = e.to_string(),
        }
    }

    if last_error.is_empty() {
        Err("服务商未返回可用模型列表或响应格式不兼容".into())
    } else {
        Err(format!("请求模型列表失败: {last_error}"))
    }
}

fn apply_http_auth_headers(req: ureq::Request, http: &HttpConnection) -> ureq::Request {
    let key = http.api_key.as_deref().unwrap_or("").trim();
    if key.is_empty() {
        return req;
    }
    match http.auth_scheme {
        ApiAuthScheme::None => req,
        ApiAuthScheme::Bearer => req.set("Authorization", &format!("Bearer {key}")),
        ApiAuthScheme::AnthropicApiKey => req
            .set("x-api-key", key)
            .set("anthropic-version", "2023-06-01"),
        ApiAuthScheme::GoogleApiKey => req.set("x-goog-api-key", key),
        ApiAuthScheme::CustomHeader => {
            let name = http
                .header_name
                .as_deref()
                .unwrap_or("Authorization")
                .trim();
            if name.is_empty() {
                req
            } else {
                req.set(name, key)
            }
        }
    }
}

fn parse_openai_models_payload(json_val: &serde_json::Value) -> Vec<String> {
    let mut models = Vec::new();
    if let Some(arr) = json_val.get("data").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                let id = id.trim();
                if !id.is_empty() && id.chars().count() <= 256 {
                    models.push(id.to_string());
                }
            }
        }
    } else if let Some(arr) = json_val.get("models").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item
                .get("id")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("name").and_then(|v| v.as_str()))
                .or_else(|| item.as_str())
            {
                let id = id.trim().trim_start_matches("models/");
                if !id.is_empty() && id.chars().count() <= 256 {
                    models.push(id.to_string());
                }
            }
        }
    } else if let Some(arr) = json_val.as_array() {
        for item in arr {
            if let Some(id) = item
                .get("id")
                .and_then(|v| v.as_str())
                .or_else(|| item.as_str())
            {
                let id = id.trim();
                if !id.is_empty() && id.chars().count() <= 256 {
                    models.push(id.to_string());
                }
            }
        }
    }
    models.sort();
    models.dedup();
    models
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

    fn provider(id: &str, sort: i64, enabled: bool, models: &[&str]) -> ApiProvider {
        let mut p = ApiProvider::new_from_kind(ApiProviderKind::OpenaiCompatible, id, sort);
        p.id = id.into();
        p.sort_index = sort;
        p.enabled = enabled;
        p.models = models.iter().map(|m| (*m).to_string()).collect();
        p.capabilities = vec![ApiCapability::native("openai.chat", true)];
        p
    }

    #[test]
    fn merge_visible_reorder_only_rewrites_filtered_slots() {
        let global = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        // Visible subset was b,d — reorder to d,b
        let merged = merge_visible_api_provider_order(&global, &["d".into(), "b".into()]).unwrap();
        assert_eq!(merged, vec!["a", "d", "c", "b"]);
    }

    #[test]
    fn merge_visible_reorder_rejects_single_item() {
        assert!(
            merge_visible_api_provider_order(&["a".into(), "b".into()], &["a".into()]).is_none()
        );
    }

    #[test]
    fn resolve_upstream_prefers_lower_sort_index() {
        let providers = vec![
            provider("second", 20, true, &["gpt-4"]),
            provider("first", 10, true, &["gpt-4"]),
            provider("disabled", 5, false, &["gpt-4"]),
        ];
        let ids: Vec<&str> =
            resolve_upstream_candidates(&providers, Some("gpt-4"), Some("openai.chat"))
                .iter()
                .map(|p| p.id.as_str())
                .collect();
        assert_eq!(ids, vec!["first", "second"]);
    }

    #[test]
    fn resolve_upstream_empty_models_is_wildcard() {
        let mut p = provider("any", 1, true, &[]);
        p.models.clear();
        let providers = vec![p];
        assert_eq!(
            resolve_upstream_candidates(&providers, Some("whatever"), None).len(),
            1
        );
    }

    #[test]
    fn resolve_upstream_filters_protocol() {
        let mut p = provider("chat-only", 1, true, &["m"]);
        p.capabilities = vec![ApiCapability::native("openai.chat", true)];
        let providers = vec![p];
        assert!(
            resolve_upstream_candidates(&providers, Some("m"), Some("openai.responses")).is_empty()
        );
        assert_eq!(
            resolve_upstream_candidates(&providers, Some("m"), Some("openai.chat")).len(),
            1
        );
    }

    #[test]
    fn merge_discovered_models_unions_and_dedupes() {
        let merged =
            merge_discovered_api_models(&["b".into(), "a".into()], &["a".into(), "c".into()])
                .unwrap();
        assert_eq!(merged, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_openai_models_payload_reads_data_ids() {
        let json = serde_json::json!({
            "data": [{ "id": "gpt-4o" }, { "id": "o3" }, { "id": "gpt-4o" }]
        });
        assert_eq!(
            parse_openai_models_payload(&json),
            vec!["gpt-4o".to_string(), "o3".to_string()]
        );
    }

    #[test]
    fn custom_and_openai_compatible_support_http_probe() {
        assert!(ApiProviderKind::OpenaiCompatible.supports_http_model_probe());
        assert!(ApiProviderKind::Custom.supports_http_model_probe());
        assert!(!ApiProviderKind::Openai.supports_http_model_probe());
        let custom = ApiProvider::new_from_kind(ApiProviderKind::Custom, "C", 1);
        assert!(custom
            .capabilities
            .iter()
            .any(|c| c.protocol == "openai.models"));
    }

    #[test]
    fn custom_and_openai_compatible_default_three_protocols() {
        for kind in [ApiProviderKind::Custom, ApiProviderKind::OpenaiCompatible] {
            let caps = kind.default_capabilities();
            let ids: Vec<&str> = caps.iter().map(|c| c.protocol.as_str()).collect();
            assert_eq!(
                ids,
                vec!["openai.chat", "openai.completions", "openai.models",],
                "{kind:?}"
            );
        }
    }
}
