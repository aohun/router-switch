use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::protocol::RequestProtocol;
use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_CLAUDE_ID: &str = "claude-official";
pub const OFFICIAL_CLAUDE_DESKTOP_ID: &str = "claude-desktop-official";
pub const DEFAULT_CLAUDE_MODEL: &str = "claude-3-7-sonnet-20250219";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaudeKind {
    Official,
    ThirdParty,
}

impl ClaudeKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeModelMapping {
    pub display_name: String,
    pub model: String,
    pub context_window: Option<u64>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaudeSettings {
    pub kind: ClaudeKind,
    pub env: Value,
    #[serde(default)]
    pub request_protocol: String,
    #[serde(default)]
    pub model_mappings: Vec<ClaudeModelMapping>,
    /// Claude Desktop 接入方式: "mapping" = 模型映射(四档角色经本地网关
    /// 路由到实际模型)；None/"direct" = 直连。仅 Claude Desktop 使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_mode: Option<String>,
}

/// Claude Desktop 接入方式取值
pub const CLAUDE_DESKTOP_MODE_DIRECT: &str = "direct";
pub const CLAUDE_DESKTOP_MODE_MAPPING: &str = "mapping";

/// `context_window` 用 1M 值表示「声明支持 1M 上下文」
pub const CLAUDE_DESKTOP_ONE_M_WINDOW: u64 = 1_000_000;

/// 模型映射模式的固定四档角色: (角色名, 暴露给 Claude Desktop 的安全路由 ID)。
/// Claude Desktop 只接受 claude-sonnet-* / claude-opus-* / claude-fable-* /
/// claude-haiku-* 形态的角色 ID。
pub const CLAUDE_DESKTOP_ROUTES: [(&str, &str); 4] = [
    ("Sonnet", "claude-sonnet-5"),
    ("Opus", "claude-opus-5"),
    ("Fable", "claude-fable-5"),
    ("Haiku", "claude-haiku-4-5"),
];

impl ClaudeSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> ClaudeForm {
        ClaudeForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            api_key: extract_claude_api_key(&self.env).unwrap_or_default(),
            base_url: extract_claude_base_url(&self.env).unwrap_or_default(),
            model: extract_claude_model(&self.env)
                .unwrap_or_else(|| DEFAULT_CLAUDE_MODEL.to_string()),
            request_protocol: if self.request_protocol.is_empty() {
                RequestProtocol::default_for_app(AppKind::Claude)
                    .as_str()
                    .to_string()
            } else {
                self.request_protocol.clone()
            },
            model_mappings: self.model_mappings.clone(),
            desktop_mode: self.desktop_mode.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeForm {
    pub name: String,
    pub website_url: String,
    pub kind: ClaudeKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub request_protocol: String,
    pub model_mappings: Vec<ClaudeModelMapping>,
    /// Claude Desktop 接入方式(None = 直连)
    pub desktop_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaudePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: ClaudeKind,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_label: &'static str,
}

pub const CLAUDE_PRESETS: &[ClaudePreset] = &[
    ClaudePreset {
        id: "official",
        name: "Anthropic Official",
        website_url: "https://anthropic.com",
        kind: ClaudeKind::Official,
        base_url: "https://api.anthropic.com",
        model: "claude-3-7-sonnet-20250219",
        provider_label: "Official",
    },
    ClaudePreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://openrouter.ai/api",
        model: "anthropic/claude-3.7-sonnet",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://deepseek.com",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://api.deepseek.com",
        model: "deepseek-chat",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "kimi",
        name: "Moonshot Kimi",
        website_url: "https://moonshot.cn",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://api.moonshot.cn/v1",
        model: "moonshot-v1-auto",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "minimax",
        name: "MiniMax",
        website_url: "https://minimax.chat",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://api.minimax.chat/v1",
        model: "MiniMax-Text-01",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "glm",
        name: "Zhipu GLM",
        website_url: "https://zhipuai.cn",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        model: "glm-4-plus",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: ClaudeKind::ThirdParty,
        base_url: "https://www.packyapi.ai/v1",
        model: "claude-3-7-sonnet-20250219",
        provider_label: "Third-Party",
    },
    ClaudePreset {
        id: "custom",
        name: "自定义服务商",
        website_url: "",
        kind: ClaudeKind::ThirdParty,
        base_url: "",
        model: "claude-3-7-sonnet-20250219",
        provider_label: "Custom",
    },
];

pub fn official_claude_settings() -> ClaudeSettings {
    ClaudeSettings {
        kind: ClaudeKind::Official,
        env: json!({}),
        request_protocol: RequestProtocol::Anthropic.as_str().into(),
        model_mappings: Vec::new(),
        desktop_mode: None,
    }
}

pub fn official_claude_provider() -> Provider {
    Provider {
        id: OFFICIAL_CLAUDE_ID.into(),
        app: AppKind::Claude,
        name: "Anthropic Official".into(),
        website_url: Some("https://anthropic.com".into()),
        settings: ProviderSettings::Claude(official_claude_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

/// Claude Desktop 官方订阅入口(1P 部署模式，还原官方登录态)
pub fn official_claude_desktop_provider() -> Provider {
    Provider {
        id: OFFICIAL_CLAUDE_DESKTOP_ID.into(),
        app: AppKind::ClaudeDesktop,
        name: "Claude Official".into(),
        website_url: Some("https://claude.ai".into()),
        settings: ProviderSettings::Claude(official_claude_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_claude_form(form: ClaudeForm) -> Result<ClaudeSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }
    match form.kind {
        ClaudeKind::Official => Ok(ClaudeSettings {
            kind: ClaudeKind::Official,
            env: json!({}),
            request_protocol: RequestProtocol::Anthropic.as_str().into(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        }),
        ClaudeKind::ThirdParty => {
            let api_key = form.api_key.trim();
            if api_key.is_empty() {
                return Err(DomainError::Validation(
                    "第三方服务商 API 密钥不能为空".into(),
                ));
            }
            let base_url = form.base_url.trim();
            if base_url.is_empty() {
                return Err(DomainError::Validation(
                    "第三方服务商 API 端点不能为空".into(),
                ));
            }
            let model = form.model.trim();
            let model = if model.is_empty() {
                DEFAULT_CLAUDE_MODEL
            } else {
                model
            };
            let env = generate_claude_env(api_key, base_url, model);
            Ok(ClaudeSettings {
                kind: ClaudeKind::ThirdParty,
                env,
                request_protocol: RequestProtocol::parse(&form.request_protocol)
                    .as_str()
                    .into(),
                model_mappings: form.model_mappings,
                desktop_mode: form.desktop_mode,
            })
        }
    }
}

pub fn generate_claude_env(api_key: &str, base_url: &str, model: &str) -> Value {
    json!({
        "ANTHROPIC_BASE_URL": base_url,
        "ANTHROPIC_AUTH_TOKEN": api_key,
        "ANTHROPIC_API_KEY": api_key,
        "ANTHROPIC_MODEL": model,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL": model,
        "ANTHROPIC_DEFAULT_SONNET_MODEL": model,
        "ANTHROPIC_DEFAULT_OPUS_MODEL": model,
    })
}

pub fn extract_claude_api_key(env: &Value) -> Option<String> {
    env.get("ANTHROPIC_AUTH_TOKEN")
        .or_else(|| env.get("ANTHROPIC_API_KEY"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

pub fn extract_claude_base_url(env: &Value) -> Option<String> {
    env.get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

pub fn extract_claude_model(env: &Value) -> Option<String> {
    env.get("ANTHROPIC_MODEL")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

pub fn extract_claude_provider_name(_env: &Value) -> Option<String> {
    None
}

pub fn backfill_claude_settings(stored: &ClaudeSettings, live_env: &Value) -> ClaudeSettings {
    if stored.kind.is_official() {
        return stored.clone();
    }
    let mut updated = stored.clone();
    if let (Some(obj), Some(live_obj)) = (updated.env.as_object_mut(), live_env.as_object()) {
        for (k, v) in live_obj {
            if let Some(str_val) = v.as_str() {
                if !str_val.trim().is_empty() {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }
    updated
}
