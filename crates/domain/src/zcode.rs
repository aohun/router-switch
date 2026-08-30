use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::protocol::RequestProtocol;
use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_ZCODE_ID: &str = "zcode-official";
pub const DEFAULT_ZCODE_MODEL: &str = "GLM-5.3";
pub const DEFAULT_ZCODE_PROVIDER_KIND: &str = "anthropic";

fn default_zcode_provider_kind() -> String {
    DEFAULT_ZCODE_PROVIDER_KIND.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZCodeKind {
    Official,
    ThirdParty,
}

impl ZCodeKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZCodeModelMapping {
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<u64>,
    pub output_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZCodeSettings {
    pub kind: ZCodeKind,
    #[serde(default = "default_zcode_provider_kind")]
    pub provider_kind: String,
    pub options: Value,
    pub models: Value,
    #[serde(default)]
    pub model_mappings: Vec<ZCodeModelMapping>,
}

impl ZCodeSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> ZCodeForm {
        let (api_key, base_url) = extract_zcode_options(&self.options);
        let model =
            extract_zcode_model(&self.models).unwrap_or_else(|| DEFAULT_ZCODE_MODEL.to_string());
        let (modality_text, modality_image) = extract_zcode_modalities(&self.models);

        ZCodeForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            provider_kind: if self.provider_kind.is_empty() {
                DEFAULT_ZCODE_PROVIDER_KIND.to_string()
            } else {
                self.provider_kind.clone()
            },
            api_key,
            base_url,
            model,
            modality_text,
            request_protocol: match self.provider_kind.as_str() {
                "openai" | "openai-chat" => RequestProtocol::OpenAiChat.as_str().to_string(),
                "openai-responses" | "responses" => {
                    RequestProtocol::OpenAiResponses.as_str().to_string()
                }
                _ => RequestProtocol::Anthropic.as_str().to_string(),
            },
            modality_image,
            model_mappings: self.model_mappings.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZCodeForm {
    pub name: String,
    pub website_url: String,
    pub kind: ZCodeKind,
    pub provider_kind: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub request_protocol: String,
    pub modality_text: bool,
    pub modality_image: bool,
    pub model_mappings: Vec<ZCodeModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZCodePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: ZCodeKind,
    pub provider_kind: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub modality_text: bool,
    pub modality_image: bool,
    pub provider_label: &'static str,
}

pub const ZCODE_PRESETS: &[ZCodePreset] = &[
    ZCodePreset {
        id: "official",
        name: "ZCode 官方",
        website_url: "https://zcode.z.ai",
        kind: ZCodeKind::Official,
        provider_kind: "anthropic",
        base_url: "",
        model: "GLM-5.3",
        modality_text: true,
        modality_image: true,
        provider_label: "Official",
    },
    ZCodePreset {
        id: "bigmodel",
        name: "智谱 BigModel",
        website_url: "https://open.bigmodel.cn",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "anthropic",
        base_url: "https://open.bigmodel.cn/api/anthropic",
        model: "GLM-5.3",
        modality_text: true,
        modality_image: true,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "anthropic",
        base_url: "https://www.packyapi.ai/v1",
        model: "claude-3-7-sonnet",
        modality_text: true,
        modality_image: true,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "cchost",
        name: "CCHost",
        website_url: "https://cchost.ai",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "anthropic",
        base_url: "https://cchost.ai",
        model: "gemini-3.7-flash-high",
        modality_text: true,
        modality_image: true,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://deepseek.com",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "openai-compatible",
        base_url: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        modality_text: true,
        modality_image: false,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "kimi",
        name: "Kimi (Moonshot)",
        website_url: "https://moonshot.cn",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "openai-compatible",
        base_url: "https://api.moonshot.cn/v1",
        model: "kimi-k2.6",
        modality_text: true,
        modality_image: true,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "openai-compatible",
        base_url: "https://openrouter.ai/api/v1",
        model: "anthropic/claude-3.7-sonnet",
        modality_text: true,
        modality_image: true,
        provider_label: "Third-Party",
    },
    ZCodePreset {
        id: "custom",
        name: "自定义模板",
        website_url: "",
        kind: ZCodeKind::ThirdParty,
        provider_kind: "anthropic",
        base_url: "",
        model: "",
        modality_text: true,
        modality_image: true,
        provider_label: "Custom",
    },
];

pub fn official_zcode_settings() -> ZCodeSettings {
    ZCodeSettings {
        kind: ZCodeKind::Official,
        provider_kind: DEFAULT_ZCODE_PROVIDER_KIND.to_string(),
        options: json!({}),
        models: json!({}),
        model_mappings: Vec::new(),
    }
}

pub fn official_zcode_provider() -> Provider {
    Provider {
        id: OFFICIAL_ZCODE_ID.to_string(),
        app: AppKind::ZCode,
        name: "ZCode 官方".to_string(),
        website_url: Some("https://zcode.z.ai".to_string()),
        settings: ProviderSettings::ZCode(official_zcode_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_zcode_form(form: ZCodeForm) -> Result<ZCodeSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }

    match form.kind {
        ZCodeKind::Official => Ok(official_zcode_settings()),
        ZCodeKind::ThirdParty => {
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
            options_map.insert("apiKeyRequired".to_string(), json!(true));

            let mut input_modalities = Vec::new();
            if form.modality_text {
                input_modalities.push("text");
            }
            if form.modality_image {
                input_modalities.push("image");
            }
            if input_modalities.is_empty() {
                input_modalities.push("text");
            }

            let mut models_map = Map::new();
            let mut main_model_obj = Map::new();
            let mut limit_map = Map::new();
            limit_map.insert("context".to_string(), json!(200000));
            limit_map.insert("output".to_string(), json!(128000));
            main_model_obj.insert("limit".to_string(), Value::Object(limit_map));
            main_model_obj.insert(
                "modalities".to_string(),
                json!({
                    "input": input_modalities,
                    "output": ["text"]
                }),
            );
            main_model_obj.insert(
                "zcode".to_string(),
                json!({
                    "modalitiesConfigured": true,
                    "modified": true
                }),
            );
            models_map.insert(model.to_string(), Value::Object(main_model_obj));

            for mapping in &form.model_mappings {
                if !mapping.model_id.trim().is_empty() {
                    let mut m = Map::new();
                    let mut limit = Map::new();
                    limit.insert(
                        "context".to_string(),
                        json!(mapping.context_limit.unwrap_or(200000)),
                    );
                    if let Some(out) = mapping.output_limit {
                        limit.insert("output".to_string(), json!(out));
                    }
                    m.insert("limit".to_string(), Value::Object(limit));
                    let mut map_inputs = Vec::new();
                    if form.modality_text {
                        map_inputs.push("text");
                    }
                    if form.modality_image {
                        map_inputs.push("image");
                    }
                    if map_inputs.is_empty() {
                        map_inputs.push("text");
                    }
                    m.insert(
                        "modalities".to_string(),
                        json!({
                            "input": map_inputs,
                            "output": ["text"]
                        }),
                    );
                    m.insert(
                        "zcode".to_string(),
                        json!({
                            "modalitiesConfigured": true,
                            "modified": true
                        }),
                    );
                    models_map.insert(mapping.model_id.clone(), Value::Object(m));
                }
            }

            let provider_kind = if !form.request_protocol.trim().is_empty() {
                match RequestProtocol::parse(&form.request_protocol) {
                    RequestProtocol::Anthropic => "anthropic".to_string(),
                    RequestProtocol::OpenAiResponses => "openai-compatible".to_string(),
                    RequestProtocol::OpenAiChat => "openai-compatible".to_string(),
                }
            } else if form.provider_kind.trim().is_empty() {
                DEFAULT_ZCODE_PROVIDER_KIND.to_string()
            } else {
                form.provider_kind.trim().to_string()
            };

            Ok(ZCodeSettings {
                kind: ZCodeKind::ThirdParty,
                provider_kind,
                options: Value::Object(options_map),
                models: Value::Object(models_map),
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn extract_zcode_options(options: &Value) -> (String, String) {
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

pub fn extract_zcode_api_key(options: &Value) -> Option<String> {
    options
        .get("apiKey")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub fn extract_zcode_base_url(options: &Value) -> Option<String> {
    options
        .get("baseURL")
        .or_else(|| options.get("baseUrl"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub fn extract_zcode_model(models: &Value) -> Option<String> {
    if let Some(obj) = models.as_object() {
        if let Some(first_key) = obj.keys().next() {
            return Some(first_key.clone());
        }
    }
    None
}

pub fn extract_zcode_modalities(models: &Value) -> (bool, bool) {
    if let Some(obj) = models.as_object() {
        for (_, model_val) in obj {
            if let Some(modalities) = model_val.get("modalities") {
                if let Some(input_arr) = modalities.get("input").and_then(|v| v.as_array()) {
                    let mut has_text = false;
                    let mut has_image = false;
                    for item in input_arr {
                        if let Some(s) = item.as_str() {
                            if s == "text" {
                                has_text = true;
                            } else if s == "image" {
                                has_image = true;
                            }
                        }
                    }
                    return (has_text, has_image);
                }
            }
        }
    }
    (true, true)
}

pub fn generate_zcode_provider_json(settings: &ZCodeSettings, provider_name: &str) -> Value {
    let mut obj = Map::new();
    obj.insert("name".to_string(), json!(provider_name));
    obj.insert("kind".to_string(), json!(settings.provider_kind));
    obj.insert("options".to_string(), settings.options.clone());
    obj.insert("source".to_string(), json!("custom"));
    obj.insert("models".to_string(), settings.models.clone());
    Value::Object(obj)
}
