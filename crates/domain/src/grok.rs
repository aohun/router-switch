use serde::{Deserialize, Serialize};

use crate::protocol::RequestProtocol;
use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_GROK_ID: &str = "grok-official";
pub const DEFAULT_GROK_MODEL: &str = "grok-4.5";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrokKind {
    Official,
    ThirdParty,
}

impl GrokKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrokModelMapping {
    pub display_name: String,
    pub model: String,
    pub context_window: Option<u64>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrokSettings {
    pub kind: GrokKind,
    pub config_toml: String,
    #[serde(default)]
    pub request_protocol: String,
    #[serde(default)]
    pub model_mappings: Vec<GrokModelMapping>,
}

impl GrokSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> GrokForm {
        GrokForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            api_key: extract_grok_api_key(&self.config_toml).unwrap_or_default(),
            base_url: extract_grok_base_url(&self.config_toml).unwrap_or_default(),
            model: extract_grok_model(&self.config_toml)
                .unwrap_or_else(|| DEFAULT_GROK_MODEL.to_string()),
            request_protocol: if self.request_protocol.is_empty() {
                RequestProtocol::default_for_app(AppKind::Grok)
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
pub struct GrokForm {
    pub name: String,
    pub website_url: String,
    pub kind: GrokKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub request_protocol: String,
    pub model_mappings: Vec<GrokModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrokPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: GrokKind,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_label: &'static str,
}

pub const GROK_PRESETS: &[GrokPreset] = &[
    GrokPreset {
        id: "official",
        name: "xAI Official",
        website_url: "https://x.ai",
        kind: GrokKind::Official,
        base_url: "https://api.x.ai/v1",
        model: "grok-4.5",
        provider_label: "Official",
    },
    GrokPreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: GrokKind::ThirdParty,
        base_url: "https://www.packyapi.ai/v1",
        model: "grok-4.5",
        provider_label: "Third-Party",
    },
    GrokPreset {
        id: "zetaapi",
        name: "ZetaAPI",
        website_url: "https://zetaapi.com",
        kind: GrokKind::ThirdParty,
        base_url: "https://api.zetaapi.com/v1",
        model: "grok-4.5",
        provider_label: "Third-Party",
    },
    GrokPreset {
        id: "custom",
        name: "自定义服务商",
        website_url: "",
        kind: GrokKind::ThirdParty,
        base_url: "",
        model: "grok-4.5",
        provider_label: "Custom",
    },
];

pub fn official_grok_settings() -> GrokSettings {
    GrokSettings {
        kind: GrokKind::Official,
        config_toml: String::new(),
        request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
        model_mappings: Vec::new(),
    }
}

pub fn official_grok_provider() -> Provider {
    Provider {
        id: OFFICIAL_GROK_ID.into(),
        app: AppKind::Grok,
        name: "xAI Official".into(),
        website_url: Some("https://x.ai".into()),
        settings: ProviderSettings::Grok(official_grok_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_grok_form(form: GrokForm) -> Result<GrokSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("服务商名称不能为空".into()));
    }
    match form.kind {
        GrokKind::Official => Ok(GrokSettings {
            kind: GrokKind::Official,
            config_toml: String::new(),
            request_protocol: RequestProtocol::OpenAiChat.as_str().into(),
            model_mappings: Vec::new(),
        }),
        GrokKind::ThirdParty => {
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
                DEFAULT_GROK_MODEL
            } else {
                model
            };
            let config_toml = generate_grok_config_toml(name, api_key, base_url, model);
            Ok(GrokSettings {
                kind: GrokKind::ThirdParty,
                config_toml,
                request_protocol: RequestProtocol::parse(&form.request_protocol)
                    .as_str()
                    .into(),
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn generate_grok_config_toml(name: &str, api_key: &str, base_url: &str, model: &str) -> String {
    let escaped_name = name.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_key = api_key.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_url = base_url.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_model = model.replace('\\', "\\\\").replace('"', "\\\"");

    format!(
        r#"[models]
default = "{escaped_model}"

[model."{escaped_model}"]
model = "{escaped_model}"
base_url = "{escaped_url}"
name = "{escaped_name}"
api_key = "{escaped_key}"
api_backend = "responses"
context_window = 500000
"#
    )
}

pub fn extract_grok_api_key(toml: &str) -> Option<String> {
    extract_toml_value(toml, "api_key")
}

pub fn extract_grok_base_url(toml: &str) -> Option<String> {
    extract_toml_value(toml, "base_url")
}

pub fn extract_grok_model(toml: &str) -> Option<String> {
    extract_toml_value(toml, "default").or_else(|| extract_toml_value(toml, "model"))
}

pub fn extract_grok_provider_name(toml: &str) -> Option<String> {
    extract_toml_value(toml, "name")
}

pub fn backfill_grok_settings(stored: &GrokSettings, live_toml: &str) -> GrokSettings {
    if stored.kind.is_official() {
        return stored.clone();
    }
    let live_key = extract_grok_api_key(live_toml);
    let live_base_url = extract_grok_base_url(live_toml);
    let live_model = extract_grok_model(live_toml);

    if live_key.is_none() && live_base_url.is_none() && live_model.is_none() {
        return stored.clone();
    }

    let key = live_key
        .or_else(|| extract_grok_api_key(&stored.config_toml))
        .unwrap_or_default();
    let base_url = live_base_url
        .or_else(|| extract_grok_base_url(&stored.config_toml))
        .unwrap_or_default();
    let model = live_model
        .or_else(|| extract_grok_model(&stored.config_toml))
        .unwrap_or_else(|| DEFAULT_GROK_MODEL.to_string());
    let name = extract_grok_provider_name(live_toml)
        .or_else(|| extract_grok_provider_name(&stored.config_toml))
        .unwrap_or_else(|| "Grok Provider".to_string());

    let config_toml = generate_grok_config_toml(&name, &key, &base_url, &model);
    GrokSettings {
        kind: GrokKind::ThirdParty,
        config_toml,
        request_protocol: stored.request_protocol.clone(),
        model_mappings: stored.model_mappings.clone(),
    }
}

fn extract_toml_value(toml: &str, key: &str) -> Option<String> {
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let prefix = format!("{key} =");
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val = rest.trim();
            if let Some(stripped) = val.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                return Some(stripped.replace("\\\"", "\"").replace("\\\\", "\\"));
            }
            if let Some(stripped) = val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                return Some(stripped.to_string());
            }
            return Some(val.to_string());
        }
    }
    None
}
