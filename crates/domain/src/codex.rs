use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::provider::ProviderSettings;
use crate::{AppKind, DomainError, Provider};

pub const OFFICIAL_CODEX_ID: &str = "codex-official";
pub const DEFAULT_CODEX_MODEL: &str = "gpt-5.6-sol";
const THIRD_PARTY_PROVIDER_ID: &str = "custom";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexKind {
    Official,
    ResponsesThirdParty,
}

impl CodexKind {
    pub fn is_official(self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexModelMapping {
    pub display_name: String,
    pub model: String,
    pub context_window: Option<u64>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodexSettings {
    pub kind: CodexKind,
    pub auth: Value,
    pub config_toml: String,
    #[serde(default)]
    pub model_mappings: Vec<CodexModelMapping>,
}

impl CodexSettings {
    pub fn form_snapshot(&self, name: &str, website_url: Option<&str>) -> CodexForm {
        CodexForm {
            name: name.to_string(),
            website_url: website_url.unwrap_or("").to_string(),
            kind: self.kind,
            api_key: extract_codex_api_key(&self.auth).unwrap_or_default(),
            base_url: extract_codex_base_url(&self.config_toml).unwrap_or_default(),
            model: extract_codex_model(&self.config_toml)
                .unwrap_or_else(|| DEFAULT_CODEX_MODEL.to_string()),
            model_mappings: self.model_mappings.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexForm {
    pub name: String,
    pub website_url: String,
    pub kind: CodexKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub model_mappings: Vec<CodexModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodexPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub website_url: &'static str,
    pub kind: CodexKind,
    pub base_url: &'static str,
    pub model: &'static str,
    pub provider_label: &'static str,
}

pub const RESPONSES_PRESETS: &[CodexPreset] = &[
    CodexPreset {
        id: "official",
        name: "OpenAI Official",
        website_url: "https://chatgpt.com/codex",
        kind: CodexKind::Official,
        base_url: "",
        model: "",
        provider_label: "openai",
    },
    CodexPreset {
        id: "kimi",
        name: "Kimi (Moonshot)",
        website_url: "https://platform.kimi.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.moonshot.cn/v1",
        model: "kimi-k2.7-code",
        provider_label: "kimi",
    },
    CodexPreset {
        id: "kimi-coding",
        name: "Kimi For Coding",
        website_url: "https://www.kimi.com/code",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.kimi.com/coding/v1",
        model: "kimi-for-coding",
        provider_label: "kimi",
    },
    CodexPreset {
        id: "deepseek",
        name: "DeepSeek",
        website_url: "https://platform.deepseek.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.deepseek.com",
        model: "deepseek-v4-flash",
        provider_label: "deepseek",
    },
    CodexPreset {
        id: "siliconflow",
        name: "SiliconFlow (硅基流动)",
        website_url: "https://siliconflow.cn",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.siliconflow.cn/v1",
        model: "Pro/MiniMaxAI/MiniMax-M2.7",
        provider_label: "siliconflow",
    },
    CodexPreset {
        id: "zhipu",
        name: "Zhipu GLM (智谱)",
        website_url: "https://open.bigmodel.cn",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4",
        model: "glm-5.2",
        provider_label: "zhipu",
    },
    CodexPreset {
        id: "bailian",
        name: "Bailian DashScope (阿里百炼)",
        website_url: "https://bailian.console.aliyun.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        model: "qwen3-coder-plus",
        provider_label: "bailian",
    },
    CodexPreset {
        id: "volcengine",
        name: "火山方舟 / BytePlus",
        website_url: "https://ark.cn-beijing.volces.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://ark.cn-beijing.volces.com/api/coding/v3",
        model: "ark-code-latest",
        provider_label: "volcengine",
    },
    CodexPreset {
        id: "hunyuan",
        name: "Tencent Hunyuan (腾讯混元)",
        website_url: "https://cloud.tencent.com/product/tokenhub",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://tokenhub.tencentmaas.com/v1",
        model: "hy3",
        provider_label: "hunyuan",
    },
    CodexPreset {
        id: "minimax",
        name: "MiniMax",
        website_url: "https://platform.minimaxi.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.minimaxi.com/v1",
        model: "MiniMax-M3",
        provider_label: "minimax",
    },
    CodexPreset {
        id: "stepfun",
        name: "StepFun (阶跃星辰)",
        website_url: "https://platform.stepfun.com",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.stepfun.com/step_plan/v1",
        model: "step-3.7-flash",
        provider_label: "stepfun",
    },
    CodexPreset {
        id: "grok",
        name: "xAI (Grok)",
        website_url: "https://x.ai/api",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.x.ai/v1",
        model: "grok-4.5",
        provider_label: "xai",
    },
    CodexPreset {
        id: "packycode",
        name: "PackyCode",
        website_url: "https://www.packyapi.ai",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://www.packyapi.ai/v1",
        model: DEFAULT_CODEX_MODEL,
        provider_label: "packycode",
    },
    CodexPreset {
        id: "opencode",
        name: "OpenCode Go",
        website_url: "https://opencode.ai/go",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://opencode.ai/zen/go/v1",
        model: "glm-5.2",
        provider_label: "opencode",
    },
    CodexPreset {
        id: "openrouter",
        name: "OpenRouter",
        website_url: "https://openrouter.ai",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://openrouter.ai/api/v1",
        model: "openai/gpt-5.6-sol",
        provider_label: "openrouter",
    },
    CodexPreset {
        id: "custom",
        name: "Custom Responses (自定义模板)",
        website_url: "",
        kind: CodexKind::ResponsesThirdParty,
        base_url: "https://api.example.com/v1",
        model: DEFAULT_CODEX_MODEL,
        provider_label: "custom",
    },
];

pub fn official_codex_settings() -> CodexSettings {
    CodexSettings {
        kind: CodexKind::Official,
        auth: json!({}),
        config_toml: String::new(),
        model_mappings: Vec::new(),
    }
}

pub fn official_codex_provider() -> Provider {
    Provider {
        id: OFFICIAL_CODEX_ID.to_string(),
        app: AppKind::Codex,
        name: "OpenAI Official".to_string(),
        website_url: Some("https://chatgpt.com/codex".to_string()),
        settings: ProviderSettings::Codex(official_codex_settings()),
        created_at: 0,
        sort_index: 0,
    }
}

pub fn generate_third_party_auth(api_key: &str) -> Value {
    json!({ "OPENAI_API_KEY": api_key })
}

pub fn generate_catalog_json(mappings: &[CodexModelMapping]) -> Option<String> {
    if mappings.is_empty() {
        return None;
    }
    let models: Vec<Value> = mappings
        .iter()
        .map(|m| {
            let context = m.context_window.unwrap_or(128_000);
            let display_name = if m.display_name.trim().is_empty() {
                &m.model
            } else {
                &m.display_name
            };
            let mut obj = json!({
                "slug": m.model,
                "display_name": display_name,
                "description": display_name,
                "context_window": context,
                "max_context_window": context,
                "supports_reasoning_summaries": true,
            });
            if let Some(effort) = &m.reasoning_effort {
                let effort_trimmed = effort.trim();
                if !effort_trimmed.is_empty() && effort_trimmed != "未设置" {
                    obj["reasoning_effort"] = json!(effort_trimmed);
                }
            }
            obj
        })
        .collect();

    serde_json::to_string_pretty(&json!({ "models": models })).ok()
}

pub fn generate_third_party_config(provider_name: &str, base_url: &str, model: &str) -> String {
    generate_third_party_config_with_catalog(provider_name, base_url, model, false)
}

pub fn generate_third_party_config_with_catalog(
    provider_name: &str,
    base_url: &str,
    model: &str,
    has_catalog: bool,
) -> String {
    let catalog_line = if has_catalog {
        "model_catalog_json = \"router-switch-model-catalog.json\"\n"
    } else {
        ""
    };
    format!(
        "model_provider = {provider_id}\n\
         model = {model}\n\
         model_reasoning_effort = \"high\"\n\
         disable_response_storage = true\n\
         {catalog_line}\
         \n\
         [model_providers.{table}]\n\
         name = {name}\n\
         base_url = {base_url}\n\
         wire_api = \"responses\"\n\
         requires_openai_auth = true\n",
        provider_id = toml_string(THIRD_PARTY_PROVIDER_ID),
        model = toml_string(model),
        catalog_line = catalog_line,
        table = THIRD_PARTY_PROVIDER_ID,
        name = toml_string(provider_name),
        base_url = toml_string(base_url),
    )
}

pub fn fetch_models_from_api(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let trimmed_base = base_url.trim().trim_end_matches('/');
    if trimmed_base.is_empty() {
        return Err("API 端点为空".into());
    }

    let candidate_urls = if trimmed_base.ends_with("/v1") {
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
        let mut req = ureq::get(&url).timeout(std::time::Duration::from_secs(8));
        let trimmed_key = api_key.trim();
        if !trimmed_key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {trimmed_key}"));
        }
        match req.call() {
            Ok(resp) => {
                if let Ok(json_val) = resp.into_json::<Value>() {
                    let mut models = Vec::new();
                    if let Some(arr) = json_val.get("data").and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                                models.push(id.to_string());
                            }
                        }
                    } else if let Some(arr) = json_val.get("models").and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(id) = item
                                .get("id")
                                .and_then(|v| v.as_str())
                                .or_else(|| item.as_str())
                            {
                                models.push(id.to_string());
                            }
                        }
                    } else if let Some(arr) = json_val.as_array() {
                        for item in arr {
                            if let Some(id) = item
                                .get("id")
                                .and_then(|v| v.as_str())
                                .or_else(|| item.as_str())
                            {
                                models.push(id.to_string());
                            }
                        }
                    }
                    if !models.is_empty() {
                        models.sort();
                        models.dedup();
                        return Ok(models);
                    }
                }
            }
            Err(e) => {
                last_error = e.to_string();
            }
        }
    }

    if last_error.is_empty() {
        Err("服务商未返回可用模型列表或响应格式不兼容".into())
    } else {
        Err(format!("请求模型列表失败: {last_error}"))
    }
}

pub fn parse_codex_form(form: CodexForm) -> Result<CodexSettings, DomainError> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(DomainError::validation("名称不能为空"));
    }

    match form.kind {
        CodexKind::Official => Ok(official_codex_settings()),
        CodexKind::ResponsesThirdParty => {
            let api_key = form.api_key.trim();
            let base_url = form.base_url.trim();
            let model = form.model.trim();
            if api_key.is_empty() {
                return Err(DomainError::validation("第三方服务商需要 API Key"));
            }
            if base_url.is_empty() {
                return Err(DomainError::validation("第三方服务商需要 API 端点"));
            }
            if model.is_empty() {
                return Err(DomainError::validation("第三方服务商需要模型名"));
            }
            let has_catalog = !form.model_mappings.is_empty();
            Ok(CodexSettings {
                kind: CodexKind::ResponsesThirdParty,
                auth: generate_third_party_auth(api_key),
                config_toml: generate_third_party_config_with_catalog(
                    name,
                    base_url,
                    model,
                    has_catalog,
                ),
                model_mappings: form.model_mappings,
            })
        }
    }
}

pub fn has_login_material(auth: &Value) -> bool {
    if extract_codex_api_key(auth).is_some() {
        return true;
    }
    let tokens = auth.get("tokens");
    non_empty_str(tokens.and_then(|t| t.get("access_token"))).is_some()
        || non_empty_str(tokens.and_then(|t| t.get("refresh_token"))).is_some()
}

pub fn extract_codex_api_key(auth: &Value) -> Option<String> {
    non_empty_str(auth.get("OPENAI_API_KEY")).map(str::to_string)
}

pub fn extract_codex_base_url(config_toml: &str) -> Option<String> {
    let value = parse_toml(config_toml)?;
    let providers = value.get("model_providers")?.as_table()?;
    if let Some(url) = providers
        .get(THIRD_PARTY_PROVIDER_ID)
        .and_then(|entry| entry.get("base_url"))
        .and_then(toml::Value::as_str)
    {
        return Some(url.to_string());
    }
    providers.values().find_map(|entry| {
        entry
            .get("base_url")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    })
}

pub fn extract_codex_model(config_toml: &str) -> Option<String> {
    parse_toml(config_toml)?
        .get("model")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

pub fn extract_codex_provider_name(config_toml: &str) -> Option<String> {
    let value = parse_toml(config_toml)?;
    let providers = value.get("model_providers")?.as_table()?;
    if let Some(name) = providers
        .get(THIRD_PARTY_PROVIDER_ID)
        .and_then(|entry| entry.get("name"))
        .and_then(toml::Value::as_str)
    {
        return Some(name.to_string());
    }
    providers.values().find_map(|entry| {
        entry
            .get("name")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    })
}

pub fn backfill_codex_settings(
    stored: &CodexSettings,
    live_auth: &Value,
    live_config_toml: &str,
) -> CodexSettings {
    let mut next = stored.clone();
    if has_login_material(live_auth) {
        next.auth = live_auth.clone();
    }
    if !live_config_toml.trim().is_empty() {
        next.config_toml = live_config_toml.to_string();
    }
    next
}

fn parse_toml(config_toml: &str) -> Option<toml::Value> {
    toml::from_str(config_toml).ok()
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

fn non_empty_str(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
