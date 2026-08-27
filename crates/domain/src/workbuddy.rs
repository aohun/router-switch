use serde::{Deserialize, Serialize};

use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_WORKBUDDY_ID: &str = "workbuddy-official";
pub const DEFAULT_WORKBUDDY_MODEL: &str = "gemini-2.5-pro";
pub const DEFAULT_WORKBUDDY_VENDOR: &str = "Custom";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkBuddyKind {
    Official,
    ThirdParty,
}

impl WorkBuddyKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_efforts: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "std::ops::Not::not",
        alias = "onlyThinking",
        alias = "only_thinking",
        alias = "reasoning_only"
    )]
    pub reasoning_only: bool,
    #[serde(
        default = "default_true",
        alias = "allowCloseThinking",
        alias = "allow_close_thinking",
        alias = "allowDisableReasoning",
        alias = "allow_disable_reasoning",
        alias = "can_disable_reasoning"
    )]
    pub can_disable_reasoning: bool,
}

/// A model entry in ~/.workbuddy/models.json
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyModelItem {
    pub id: String,
    pub name: String,
    #[serde(default = "default_vendor")]
    pub vendor: String,
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_true")]
    pub supports_tool_call: bool,
    #[serde(default = "default_true")]
    pub supports_images: bool,
    #[serde(default)]
    pub supports_reasoning: bool,
    #[serde(
        default,
        skip_serializing_if = "std::ops::Not::not",
        alias = "reasoning_only",
        alias = "onlyThinking",
        alias = "only_thinking"
    )]
    pub reasoning_only: bool,
    #[serde(
        default = "default_true",
        alias = "can_disable_reasoning",
        alias = "allowCloseThinking",
        alias = "allow_close_thinking",
        alias = "allowDisableReasoning",
        alias = "allow_disable_reasoning"
    )]
    pub can_disable_reasoning: bool,
    #[serde(default)]
    pub use_custom_protocol: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<WorkBuddyReasoningConfig>,
}

fn default_vendor() -> String {
    DEFAULT_WORKBUDDY_VENDOR.to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkBuddySettings {
    pub kind: WorkBuddyKind,
    pub model_id: String,
    #[serde(default = "default_vendor")]
    pub vendor: String,
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_true")]
    pub supports_tool_call: bool,
    #[serde(default = "default_true")]
    pub supports_images: bool,
    #[serde(default)]
    pub supports_reasoning: bool,
    #[serde(default)]
    pub reasoning_only: bool,
    #[serde(default = "default_true")]
    pub can_disable_reasoning: bool,
    #[serde(default)]
    pub use_custom_protocol: bool,
    #[serde(default)]
    pub max_input_tokens: Option<u64>,
    #[serde(default)]
    pub max_output_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub supported_reasoning_efforts: Vec<String>,
}

impl WorkBuddySettings {
    pub fn to_model_item(&self, name: &str) -> WorkBuddyModelItem {
        let reasoning = if self.supports_reasoning {
            let effort = self
                .reasoning_effort
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "medium".to_string());
            let supported = if self.supported_reasoning_efforts.is_empty() {
                vec!["medium".to_string()]
            } else {
                self.supported_reasoning_efforts.clone()
            };
            Some(WorkBuddyReasoningConfig {
                default_effort: Some(effort),
                supported_efforts: Some(supported),
                reasoning_only: self.reasoning_only,
                can_disable_reasoning: self.can_disable_reasoning,
            })
        } else {
            None
        };

        WorkBuddyModelItem {
            id: self.model_id.clone(),
            name: if name.trim().is_empty() {
                self.model_id.clone()
            } else {
                name.to_string()
            },
            vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
            url: self.url.clone(),
            api_key: self.api_key.clone(),
            supports_tool_call: self.supports_tool_call,
            supports_images: self.supports_images,
            supports_reasoning: self.supports_reasoning,
            reasoning_only: self.reasoning_only,
            can_disable_reasoning: self.can_disable_reasoning,
            use_custom_protocol: self.use_custom_protocol,
            max_input_tokens: self.max_input_tokens,
            max_output_tokens: self.max_output_tokens,
            reasoning,
        }
    }

    pub fn from_model_item(item: &WorkBuddyModelItem) -> (Self, String) {
        let name = item.name.clone();
        let reasoning_effort = item
            .reasoning
            .as_ref()
            .and_then(|r| r.default_effort.clone());
        let supported_reasoning_efforts = item
            .reasoning
            .as_ref()
            .and_then(|r| r.supported_efforts.clone())
            .unwrap_or_else(|| {
                if item.supports_reasoning {
                    vec!["medium".to_string()]
                } else {
                    Vec::new()
                }
            });
        let reasoning_only = item.reasoning_only
            || item
                .reasoning
                .as_ref()
                .map(|r| r.reasoning_only)
                .unwrap_or(false);
        let can_disable_reasoning = item
            .reasoning
            .as_ref()
            .map(|r| r.can_disable_reasoning)
            .unwrap_or(item.can_disable_reasoning);

        (
            WorkBuddySettings {
                kind: WorkBuddyKind::ThirdParty,
                model_id: item.id.clone(),
                vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
                url: item.url.clone(),
                api_key: item.api_key.clone(),
                supports_tool_call: item.supports_tool_call,
                supports_images: item.supports_images,
                supports_reasoning: item.supports_reasoning,
                reasoning_only,
                can_disable_reasoning,
                use_custom_protocol: item.use_custom_protocol,
                max_input_tokens: item.max_input_tokens,
                max_output_tokens: item.max_output_tokens,
                reasoning_effort,
                supported_reasoning_efforts,
            },
            name,
        )
    }

    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> WorkBuddyForm {
        WorkBuddyForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            model_id: self.model_id.clone(),
            vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
            base_url: self.url.clone(),
            api_key: self.api_key.clone(),
            supports_tool_call: self.supports_tool_call,
            supports_images: self.supports_images,
            supports_reasoning: self.supports_reasoning,
            reasoning_only: self.reasoning_only,
            can_disable_reasoning: self.can_disable_reasoning,
            use_custom_protocol: self.use_custom_protocol,
            max_input_tokens: self.max_input_tokens,
            max_output_tokens: self.max_output_tokens,
            reasoning_effort: self
                .reasoning_effort
                .clone()
                .unwrap_or_else(|| "medium".to_string()),
            supported_reasoning_efforts: self.supported_reasoning_efforts.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBuddyForm {
    pub name: String,
    pub website_url: String,
    pub kind: WorkBuddyKind,
    pub model_id: String,
    pub vendor: String,
    pub base_url: String,
    pub api_key: String,
    pub supports_tool_call: bool,
    pub supports_images: bool,
    pub supports_reasoning: bool,
    pub reasoning_only: bool,
    pub can_disable_reasoning: bool,
    pub use_custom_protocol: bool,
    pub max_input_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub reasoning_effort: String,
    pub supported_reasoning_efforts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkBuddyPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: WorkBuddyKind,
    pub model_id: &'static str,
    pub vendor: &'static str,
    pub base_url: &'static str,
    pub supports_tool_call: bool,
    pub supports_images: bool,
    pub supports_reasoning: bool,
    pub reasoning_only: bool,
    pub can_disable_reasoning: bool,
    pub use_custom_protocol: bool,
    pub max_input_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub reasoning_effort: &'static str,
    pub provider_label: &'static str,
}

pub const WORKBUDDY_PRESETS: &[WorkBuddyPreset] = &[
    WorkBuddyPreset {
        id: "official",
        name: "WorkBuddy 官方",
        website_url: "https://workbuddy.cn",
        kind: WorkBuddyKind::Official,
        model_id: "default",
        vendor: "Custom",
        base_url: "",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: None,
        max_output_tokens: None,
        reasoning_effort: "medium",
        provider_label: "Official",
    },
    WorkBuddyPreset {
        id: "cchost-gemini",
        name: "CCHost (Gemini 3.7)",
        website_url: "https://cchost.ai",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "gemini-3.7-flash-high",
        vendor: "Custom",
        base_url: "https://cchost.ai/v1",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(262144),
        max_output_tokens: Some(65536),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "cchost-gpt",
        name: "CCHost (GPT-5.6 Reasoning)",
        website_url: "https://cchost.ai",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "gpt-5.6",
        vendor: "Custom",
        base_url: "https://cchost.ai/v1",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: true,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(262144),
        max_output_tokens: Some(65536),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "openai-gpt4o",
        name: "OpenAI (GPT-4o)",
        website_url: "https://platform.openai.com",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "gpt-4o",
        vendor: "Custom",
        base_url: "https://api.openai.com/v1",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(128000),
        max_output_tokens: Some(16384),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "deepseek-chat",
        name: "DeepSeek (V3)",
        website_url: "https://platform.deepseek.com",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "deepseek-chat",
        vendor: "Custom",
        base_url: "https://api.deepseek.com/v1",
        supports_tool_call: true,
        supports_images: false,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(64000),
        max_output_tokens: Some(8192),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "deepseek-reasoner",
        name: "DeepSeek (R1 深度思考)",
        website_url: "https://platform.deepseek.com",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "deepseek-reasoner",
        vendor: "Custom",
        base_url: "https://api.deepseek.com/v1",
        supports_tool_call: true,
        supports_images: false,
        supports_reasoning: true,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(64000),
        max_output_tokens: Some(8192),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "siliconflow",
        name: "SiliconFlow 硅基流动",
        website_url: "https://siliconflow.cn",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "deepseek-ai/DeepSeek-V3",
        vendor: "Custom",
        base_url: "https://api.siliconflow.cn/v1",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: Some(128000),
        max_output_tokens: Some(8192),
        reasoning_effort: "medium",
        provider_label: "Third-Party",
    },
    WorkBuddyPreset {
        id: "custom",
        name: "自定义模型",
        website_url: "",
        kind: WorkBuddyKind::ThirdParty,
        model_id: "",
        vendor: "Custom",
        base_url: "",
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: None,
        max_output_tokens: None,
        reasoning_effort: "medium",
        provider_label: "Custom",
    },
];

pub fn official_workbuddy_settings() -> WorkBuddySettings {
    WorkBuddySettings {
        kind: WorkBuddyKind::Official,
        model_id: "default".into(),
        vendor: DEFAULT_WORKBUDDY_VENDOR.into(),
        url: "".into(),
        api_key: "".into(),
        supports_tool_call: true,
        supports_images: true,
        supports_reasoning: false,
        reasoning_only: false,
        can_disable_reasoning: true,
        use_custom_protocol: false,
        max_input_tokens: None,
        max_output_tokens: None,
        reasoning_effort: None,
        supported_reasoning_efforts: Vec::new(),
    }
}

pub fn official_workbuddy_provider() -> Provider {
    Provider {
        id: OFFICIAL_WORKBUDDY_ID.to_string(),
        app: AppKind::WorkBuddy,
        name: "WorkBuddy 官方".to_string(),
        website_url: Some("https://workbuddy.cn".to_string()),
        settings: ProviderSettings::WorkBuddy(official_workbuddy_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn parse_workbuddy_form(form: WorkBuddyForm) -> Result<WorkBuddySettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("模型名称不能为空".into()));
    }

    match form.kind {
        WorkBuddyKind::Official => Ok(official_workbuddy_settings()),
        WorkBuddyKind::ThirdParty => {
            let model_id = form.model_id.trim();
            if model_id.is_empty() {
                return Err(DomainError::Validation("模型 ID 不能为空".into()));
            }
            let base_url = form.base_url.trim();
            if base_url.is_empty() {
                return Err(DomainError::Validation("接口地址 (URL) 不能为空".into()));
            }
            let api_key = form.api_key.trim();
            if api_key.is_empty() {
                return Err(DomainError::Validation("API Key 不能为空".into()));
            }

            let reasoning_effort = if form.supports_reasoning {
                let effort = form.reasoning_effort.trim();
                Some(if effort.is_empty() {
                    "medium".to_string()
                } else {
                    effort.to_string()
                })
            } else {
                None
            };

            let supported_reasoning_efforts = if form.supports_reasoning {
                if form.supported_reasoning_efforts.is_empty() {
                    vec!["medium".to_string()]
                } else {
                    form.supported_reasoning_efforts
                }
            } else {
                Vec::new()
            };

            Ok(WorkBuddySettings {
                kind: WorkBuddyKind::ThirdParty,
                model_id: model_id.to_string(),
                vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
                url: base_url.to_string(),
                api_key: api_key.to_string(),
                supports_tool_call: form.supports_tool_call,
                supports_images: form.supports_images,
                supports_reasoning: form.supports_reasoning,
                reasoning_only: form.reasoning_only,
                can_disable_reasoning: form.can_disable_reasoning,
                use_custom_protocol: form.use_custom_protocol,
                max_input_tokens: form.max_input_tokens,
                max_output_tokens: form.max_output_tokens,
                reasoning_effort,
                supported_reasoning_efforts,
            })
        }
    }
}
