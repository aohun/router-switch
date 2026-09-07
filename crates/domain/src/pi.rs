use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::protocol::RequestProtocol;
use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_PI_ID: &str = "pi-official";
pub const DEFAULT_PI_MODEL: &str = "gpt-4o";
pub const DEFAULT_PI_API_TYPE: &str = "openai-completions";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PiKind {
    Official,
    ThirdParty,
}

impl PiKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiModelMapping {
    pub model_id: String,
    pub display_name: String,
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiSettings {
    pub kind: PiKind,
    pub api_type: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub request_protocol: String,
    #[serde(default)]
    pub model_mappings: Vec<PiModelMapping>,
}

impl PiSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> PiForm {
        PiForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            api_type: self.api_type.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            request_protocol: if self.request_protocol.is_empty() {
                RequestProtocol::default_for_app(AppKind::Pi)
                    .as_str()
                    .to_string()
            } else {
                self.request_protocol.clone()
            },
            model_mappings: self.model_mappings.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiForm {
    pub name: String,
    pub website_url: String,
    pub kind: PiKind,
    pub api_type: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub request_protocol: String,
    pub model_mappings: Vec<PiModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PiPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: PiKind,
    pub api_type: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_label: &'static str,
}

pub const PI_PRESETS: &[PiPreset] = &[
    PiPreset {
        id: "official",
        name: "Pi 官方",
        website_url: "https://pi.dev",
        kind: PiKind::Official,
        api_type: "openai-completions",
        base_url: "",
        model: "gpt-4o",
        provider_label: "Official",
    },
    PiPreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: "https://www.packyapi.ai/v1",
        model: "gpt-4o",
        provider_label: "Third-Party",
    },
    PiPreset {
        id: "zetaapi",
        name: "ZetaAPI / S2A",
        website_url: "https://s2a.ii.sb",
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: "https://s2a.ii.sb/v1",
        model: "grok-4.6",
        provider_label: "Third-Party",
    },
    PiPreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://deepseek.com",
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        provider_label: "Third-Party",
    },
    PiPreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: "https://openrouter.ai/api/v1",
        model: "anthropic/claude-3.7-sonnet",
        provider_label: "Third-Party",
    },
    PiPreset {
        id: crate::WORKBUDDY_CODEBUDDY_PRESET_ID,
        name: "WorkBuddy (CodeBuddy)",
        website_url: crate::WORKBUDDY_WEBSITE_URL,
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: crate::WORKBUDDY_UPSTREAM_BASE,
        model: crate::WORKBUDDY_DEFAULT_CODEBUDDY_MODEL,
        provider_label: "CodeBuddy",
    },
    PiPreset {
        id: "custom",
        name: "自定义模板",
        website_url: "",
        kind: PiKind::ThirdParty,
        api_type: "openai-completions",
        base_url: "",
        model: "",
        provider_label: "Custom",
    },
];

pub fn official_pi_settings() -> PiSettings {
    PiSettings {
        kind: PiKind::Official,
        api_type: DEFAULT_PI_API_TYPE.to_string(),
        base_url: String::new(),
        api_key: String::new(),
        model: DEFAULT_PI_MODEL.to_string(),
        request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
        model_mappings: Vec::new(),
    }
}

pub fn official_pi_provider() -> Provider {
    Provider {
        id: OFFICIAL_PI_ID.to_string(),
        app: AppKind::Pi,
        name: "Pi 官方".to_string(),
        website_url: Some("https://pi.dev".to_string()),
        settings: ProviderSettings::Pi(official_pi_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_pi_form(form: PiForm) -> Result<PiSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }

    match form.kind {
        PiKind::Official => Ok(official_pi_settings()),
        PiKind::ThirdParty => {
            let base_url = form.base_url.trim();
            if base_url.is_empty() {
                return Err(DomainError::Validation(
                    "API 端点 (Base URL) 不能为空".into(),
                ));
            }
            let api_key = form.api_key.trim();
            if api_key.is_empty() {
                return Err(DomainError::Validation("API Key 不能为空".into()));
            }
            let model = form.model.trim();
            if model.is_empty() {
                return Err(DomainError::Validation("模型名称不能为空".into()));
            }

            let api_type = if form.api_type.trim().is_empty() {
                DEFAULT_PI_API_TYPE.to_string()
            } else {
                form.api_type.trim().to_string()
            };

            Ok(PiSettings {
                kind: PiKind::ThirdParty,
                api_type,
                base_url: base_url.to_string(),
                api_key: api_key.to_string(),
                model: model.to_string(),
                request_protocol: RequestProtocol::parse(&form.request_protocol)
                    .as_str()
                    .into(),
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn generate_pi_models_json(settings: &PiSettings, provider_key: &str) -> Value {
    let mut models_list = Vec::new();
    models_list.push(json!({
        "id": settings.model,
        "name": settings.model
    }));

    for mapping in &settings.model_mappings {
        if !mapping.model_id.trim().is_empty() && mapping.model_id != settings.model {
            models_list.push(json!({
                "id": mapping.model_id,
                "name": if mapping.display_name.trim().is_empty() { &mapping.model_id } else { &mapping.display_name }
            }));
        }
    }

    let mut provider_obj = Map::new();
    provider_obj.insert("api".to_string(), json!(settings.api_type));
    provider_obj.insert("baseUrl".to_string(), json!(settings.base_url));
    provider_obj.insert("apiKey".to_string(), json!(settings.api_key));
    provider_obj.insert("models".to_string(), Value::Array(models_list));

    let mut providers_map = Map::new();
    providers_map.insert(provider_key.to_string(), Value::Object(provider_obj));

    let mut root = Map::new();
    root.insert("providers".to_string(), Value::Object(providers_map));
    Value::Object(root)
}

pub fn generate_pi_settings_json(provider_key: &str, model: &str) -> Value {
    json!({
        "defaultProvider": provider_key,
        "defaultModel": model
    })
}
