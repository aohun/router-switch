use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_CURSOR_ID: &str = "cursor-official";
pub const DEFAULT_CURSOR_MODEL: &str = "claude-3.7-sonnet";
pub const DEFAULT_CURSOR_PROVIDER_TYPE: &str = "openai-chat";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorKind {
    Official,
    ThirdParty,
}

impl CursorKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorModelMapping {
    pub display_name: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorSettings {
    pub kind: CursorKind,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub provider_type: String,
    #[serde(default)]
    pub options: Value,
    #[serde(default)]
    pub model_mappings: Vec<CursorModelMapping>,
}

impl CursorSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> CursorForm {
        CursorForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: if self.model.is_empty() {
                DEFAULT_CURSOR_MODEL.to_string()
            } else {
                self.model.clone()
            },
            provider_type: if self.provider_type.is_empty() {
                "openai-chat".to_string()
            } else {
                self.provider_type.clone()
            },
            model_mappings: self.model_mappings.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorForm {
    pub name: String,
    pub website_url: String,
    pub kind: CursorKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub provider_type: String,
    pub model_mappings: Vec<CursorModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: CursorKind,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_type: &'static str,
    pub provider_label: &'static str,
}

pub const CURSOR_PRESETS: &[CursorPreset] = &[
    CursorPreset {
        id: "official",
        name: "Cursor Official",
        website_url: "https://cursor.com",
        kind: CursorKind::Official,
        base_url: "https://api2.cursor.sh",
        model: "claude-3.7-sonnet",
        provider_type: "official",
        provider_label: "Official",
    },
    CursorPreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: CursorKind::ThirdParty,
        base_url: "https://www.packyapi.ai/v1",
        model: "gpt-4o",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: CursorKind::ThirdParty,
        base_url: "https://openrouter.ai/api/v1",
        model: "anthropic/claude-3.7-sonnet",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://deepseek.com",
        kind: CursorKind::ThirdParty,
        base_url: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "siliconflow",
        name: "SiliconFlow",
        website_url: "https://siliconflow.cn",
        kind: CursorKind::ThirdParty,
        base_url: "https://api.siliconflow.cn/v1",
        model: "deepseek-ai/DeepSeek-V3",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "openai",
        name: "OpenAI Direct",
        website_url: "https://openai.com",
        kind: CursorKind::ThirdParty,
        base_url: "https://api.openai.com/v1",
        model: "gpt-4o",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "anthropic",
        name: "Anthropic Direct",
        website_url: "https://anthropic.com",
        kind: CursorKind::ThirdParty,
        base_url: "https://api.anthropic.com/v1",
        model: "claude-3-7-sonnet-20250219",
        provider_type: "anthropic",
        provider_label: "Third-Party",
    },
    CursorPreset {
        id: "grok",
        name: "xAI / Grok",
        website_url: "https://x.ai",
        kind: CursorKind::ThirdParty,
        base_url: "https://api.x.ai/v1",
        model: "grok-2-latest",
        provider_type: "openai-chat",
        provider_label: "Third-Party",
    },
];

pub fn parse_cursor_form(form: CursorForm) -> Result<CursorSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }

    match form.kind {
        CursorKind::Official => Ok(official_cursor_settings()),
        CursorKind::ThirdParty => {
            let base_url = form.base_url.trim();
            if base_url.is_empty() {
                return Err(DomainError::Validation("API 地址不能为空".into()));
            }
            let api_key = form.api_key.trim();
            if api_key.is_empty() {
                return Err(DomainError::Validation("API Key 不能为空".into()));
            }
            let model = form.model.trim();
            let model = if model.is_empty() {
                DEFAULT_CURSOR_MODEL
            } else {
                model
            };
            let provider_type = form.provider_type.trim();
            let provider_type = if provider_type.is_empty() {
                "openai-chat"
            } else {
                provider_type
            };

            let options = json!({
                "apiKey": api_key,
                "baseURL": base_url,
                "model": model,
                "providerType": provider_type,
            });

            Ok(CursorSettings {
                kind: CursorKind::ThirdParty,
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                model: model.to_string(),
                provider_type: provider_type.to_string(),
                options,
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn official_cursor_provider() -> Provider {
    Provider {
        id: OFFICIAL_CURSOR_ID.into(),
        app: AppKind::Cursor,
        name: "Cursor Official".into(),
        website_url: Some("https://cursor.com".into()),
        settings: ProviderSettings::Cursor(official_cursor_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn official_cursor_settings() -> CursorSettings {
    CursorSettings {
        kind: CursorKind::Official,
        api_key: String::new(),
        base_url: "https://api2.cursor.sh".into(),
        model: DEFAULT_CURSOR_MODEL.into(),
        provider_type: "official".into(),
        options: json!({}),
        model_mappings: Vec::new(),
    }
}

pub fn extract_cursor_api_key(settings: &CursorSettings) -> Option<String> {
    if !settings.api_key.is_empty() {
        Some(settings.api_key.clone())
    } else {
        settings
            .options
            .get("apiKey")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

pub fn extract_cursor_base_url(settings: &CursorSettings) -> Option<String> {
    if !settings.base_url.is_empty() {
        Some(settings.base_url.clone())
    } else {
        settings
            .options
            .get("baseURL")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

pub fn extract_cursor_model(settings: &CursorSettings) -> Option<String> {
    if !settings.model.is_empty() {
        Some(settings.model.clone())
    } else {
        settings
            .options
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

pub fn extract_cursor_provider_type(settings: &CursorSettings) -> Option<String> {
    if !settings.provider_type.is_empty() {
        Some(settings.provider_type.clone())
    } else {
        settings
            .options
            .get("providerType")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

pub fn backfill_cursor_settings(settings: &mut CursorSettings) {
    if settings.api_key.is_empty() {
        if let Some(key) = extract_cursor_api_key(settings) {
            settings.api_key = key;
        }
    }
    if settings.base_url.is_empty() {
        if let Some(url) = extract_cursor_base_url(settings) {
            settings.base_url = url;
        }
    }
    if settings.model.is_empty() {
        if let Some(m) = extract_cursor_model(settings) {
            settings.model = m;
        }
    }
}
