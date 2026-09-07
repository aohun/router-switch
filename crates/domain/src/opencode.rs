use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::protocol::RequestProtocol;
use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_OPENCODE_ID: &str = "opencode-official";
pub const DEFAULT_OPENCODE_MODEL: &str = "claude-3-7-sonnet";
pub const DEFAULT_OPENCODE_NPM: &str = "@ai-sdk/openai-compatible";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenCodeKind {
    Official,
    ThirdParty,
}

impl OpenCodeKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeModelMapping {
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<u64>,
    pub output_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenCodeSettings {
    pub kind: OpenCodeKind,
    pub npm: String,
    pub options: Value,
    pub models: Value,
    #[serde(default)]
    pub request_protocol: String,
    #[serde(default)]
    pub model_mappings: Vec<OpenCodeModelMapping>,
}

impl OpenCodeSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> OpenCodeForm {
        let (api_key, base_url) = extract_opencode_options(&self.options);
        let model = extract_opencode_model(&self.models)
            .unwrap_or_else(|| DEFAULT_OPENCODE_MODEL.to_string());

        OpenCodeForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            npm: self.npm.clone(),
            api_key,
            base_url,
            model,
            request_protocol: if self.request_protocol.is_empty() {
                RequestProtocol::default_for_app(AppKind::OpenCode)
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
pub struct OpenCodeForm {
    pub name: String,
    pub website_url: String,
    pub kind: OpenCodeKind,
    pub npm: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub request_protocol: String,
    pub model_mappings: Vec<OpenCodeModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCodePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: OpenCodeKind,
    pub npm: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_label: &'static str,
}

pub const OPENCODE_PRESETS: &[OpenCodePreset] = &[
    OpenCodePreset {
        id: "official",
        name: "OpenCode 官方",
        website_url: "https://opencode.ai",
        kind: OpenCodeKind::Official,
        npm: "@ai-sdk/openai-compatible",
        base_url: "",
        model: "claude-3-7-sonnet",
        provider_label: "Official",
    },
    OpenCodePreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "https://www.packyapi.ai/v1",
        model: "claude-3-7-sonnet",
        provider_label: "Third-Party",
    },
    OpenCodePreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://deepseek.com",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        provider_label: "Third-Party",
    },
    OpenCodePreset {
        id: "kimi",
        name: "Kimi (Moonshot)",
        website_url: "https://moonshot.cn",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "https://api.moonshot.cn/v1",
        model: "kimi-k2.6",
        provider_label: "Third-Party",
    },
    OpenCodePreset {
        id: "minimax",
        name: "MiniMax",
        website_url: "https://minimax.chat",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "https://api.minimax.chat/v1",
        model: "MiniMax-M2.7",
        provider_label: "Third-Party",
    },
    OpenCodePreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "https://openrouter.ai/api/v1",
        model: "anthropic/claude-3.7-sonnet",
        provider_label: "Third-Party",
    },
    OpenCodePreset {
        id: crate::WORKBUDDY_CODEBUDDY_PRESET_ID,
        name: "WorkBuddy (CodeBuddy)",
        website_url: crate::WORKBUDDY_WEBSITE_URL,
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: crate::WORKBUDDY_UPSTREAM_BASE,
        model: crate::WORKBUDDY_DEFAULT_CODEBUDDY_MODEL,
        provider_label: "CodeBuddy",
    },
    OpenCodePreset {
        id: "custom",
        name: "自定义模板",
        website_url: "",
        kind: OpenCodeKind::ThirdParty,
        npm: "@ai-sdk/openai-compatible",
        base_url: "",
        model: "",
        provider_label: "Custom",
    },
];

pub fn official_opencode_settings() -> OpenCodeSettings {
    OpenCodeSettings {
        kind: OpenCodeKind::Official,
        npm: DEFAULT_OPENCODE_NPM.to_string(),
        options: json!({}),
        models: json!({}),
        request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
        model_mappings: Vec::new(),
    }
}

pub fn official_opencode_provider() -> Provider {
    Provider {
        id: OFFICIAL_OPENCODE_ID.to_string(),
        app: AppKind::OpenCode,
        name: "OpenCode 官方".to_string(),
        website_url: Some("https://opencode.ai".to_string()),
        settings: ProviderSettings::OpenCode(official_opencode_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_opencode_form(form: OpenCodeForm) -> Result<OpenCodeSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }

    match form.kind {
        OpenCodeKind::Official => Ok(official_opencode_settings()),
        OpenCodeKind::ThirdParty => {
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

            let mut options_map = Map::new();
            options_map.insert("baseURL".to_string(), json!(base_url));
            options_map.insert("apiKey".to_string(), json!(api_key));

            let mut models_map = Map::new();
            let mut main_model_obj = Map::new();
            main_model_obj.insert("name".to_string(), json!(model));
            models_map.insert(model.to_string(), Value::Object(main_model_obj));

            for mapping in &form.model_mappings {
                if !mapping.model_id.trim().is_empty() {
                    let mut m = Map::new();
                    m.insert(
                        "name".to_string(),
                        json!(if mapping.display_name.trim().is_empty() {
                            &mapping.model_id
                        } else {
                            &mapping.display_name
                        }),
                    );
                    if let (Some(c), Some(o)) = (mapping.context_limit, mapping.output_limit) {
                        m.insert("limit".to_string(), json!({ "context": c, "output": o }));
                    }
                    models_map.insert(mapping.model_id.clone(), Value::Object(m));
                }
            }

            let npm = if form.npm.trim().is_empty() {
                DEFAULT_OPENCODE_NPM.to_string()
            } else {
                form.npm.trim().to_string()
            };

            Ok(OpenCodeSettings {
                kind: OpenCodeKind::ThirdParty,
                npm,
                options: Value::Object(options_map),
                models: Value::Object(models_map),
                request_protocol: RequestProtocol::parse(&form.request_protocol)
                    .as_str()
                    .into(),
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn extract_opencode_options(options: &Value) -> (String, String) {
    let api_key = options
        .get("apiKey")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let base_url = options
        .get("baseURL")
        .or_else(|| options.get("baseUrl"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    (api_key, base_url)
}

pub fn extract_opencode_api_key(options: &Value) -> Option<String> {
    options
        .get("apiKey")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub fn extract_opencode_base_url(options: &Value) -> Option<String> {
    options
        .get("baseURL")
        .or_else(|| options.get("baseUrl"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub fn extract_opencode_model(models: &Value) -> Option<String> {
    if let Some(obj) = models.as_object() {
        if let Some(first_key) = obj.keys().next() {
            return Some(first_key.clone());
        }
    }
    None
}

pub fn generate_opencode_provider_json(settings: &OpenCodeSettings, provider_name: &str) -> Value {
    let mut obj = Map::new();
    obj.insert("npm".to_string(), json!(settings.npm));
    obj.insert("name".to_string(), json!(provider_name));
    obj.insert("options".to_string(), settings.options.clone());
    obj.insert("models".to_string(), settings.models.clone());
    Value::Object(obj)
}
