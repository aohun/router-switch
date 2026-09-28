use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use std::collections::HashMap;
use url::Url;

use crate::{
    AppKind, ClaudeForm, ClaudeKind, CodexForm, CodexKind, CursorForm, CursorKind, DomainError,
    GrokForm, GrokKind, OpenCodeForm, OpenCodeKind, PiForm, PiKind, ProviderForm, WorkBuddyForm,
    WorkBuddyKind, ZCodeForm, ZCodeKind, DEFAULT_CLAUDE_MODEL, DEFAULT_CODEX_MODEL,
    DEFAULT_CURSOR_MODEL, DEFAULT_GROK_MODEL, DEFAULT_OPENCODE_MODEL, DEFAULT_OPENCODE_NPM,
    DEFAULT_PI_API_TYPE, DEFAULT_PI_MODEL, DEFAULT_WORKBUDDY_MODEL, DEFAULT_WORKBUDDY_VENDOR,
    DEFAULT_ZCODE_MODEL, DEFAULT_ZCODE_PROVIDER_KIND,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeepLinkImportRequest {
    pub version: String,
    pub resource: String,
    pub app: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub homepage: Option<String>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub notes: Option<String>,
    pub config: Option<String>,
    pub config_format: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
    pub npm: Option<String>,
    pub api_type: Option<String>,
}

pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkImportRequest, DomainError> {
    let trimmed = url_str.trim();
    let parsed_url = Url::parse(trimmed)
        .map_err(|e| DomainError::Validation(format!("无效的深链接 URL: {e}")))?;

    let scheme = parsed_url.scheme().to_ascii_lowercase();
    if scheme != "router-switch" && scheme != "ccswitch" && scheme != "routerswitch" {
        return Err(DomainError::Validation(format!(
            "不支持的深链接协议 scheme: '{scheme}', 仅支持 router-switch:// 或 ccswitch://"
        )));
    }

    let host = parsed_url.host_str().unwrap_or("v1");
    let path = parsed_url.path().trim_start_matches('/');

    let pairs: HashMap<String, String> = parsed_url.query_pairs().into_owned().collect();

    let mut resource = pairs
        .get("resource")
        .cloned()
        .unwrap_or_else(|| {
            if path == "provider" || host == "provider" {
                "provider".to_string()
            } else {
                "provider".to_string()
            }
        })
        .to_ascii_lowercase();

    if resource.is_empty() {
        resource = "provider".to_string();
    }

    if resource != "provider" {
        return Err(DomainError::Validation(format!(
            "当前仅支持 provider 资源类型导入, 收到: '{resource}'"
        )));
    }

    let app = pairs.get("app").or_else(|| pairs.get("targetApp")).cloned();

    let name = pairs
        .get("name")
        .or_else(|| pairs.get("providerName"))
        .cloned();

    let endpoint = pairs
        .get("endpoint")
        .or_else(|| pairs.get("baseUrl"))
        .or_else(|| pairs.get("base_url"))
        .or_else(|| pairs.get("url"))
        .cloned();

    let api_key = pairs
        .get("apiKey")
        .or_else(|| pairs.get("api_key"))
        .or_else(|| pairs.get("key"))
        .or_else(|| pairs.get("token"))
        .cloned();

    let homepage = pairs
        .get("homepage")
        .or_else(|| pairs.get("website"))
        .or_else(|| pairs.get("websiteUrl"))
        .or_else(|| pairs.get("website_url"))
        .cloned();

    let model = pairs
        .get("model")
        .or_else(|| pairs.get("defaultModel"))
        .cloned();

    let notes = pairs.get("notes").cloned();
    let config = pairs.get("config").cloned();
    let config_format = pairs
        .get("configFormat")
        .or_else(|| pairs.get("config_format"))
        .cloned();

    let enabled = pairs
        .get("enabled")
        .and_then(|v| match v.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        });

    let haiku_model = pairs
        .get("haikuModel")
        .or_else(|| pairs.get("haiku_model"))
        .cloned();
    let sonnet_model = pairs
        .get("sonnetModel")
        .or_else(|| pairs.get("sonnet_model"))
        .cloned();
    let opus_model = pairs
        .get("opusModel")
        .or_else(|| pairs.get("opus_model"))
        .cloned();

    let npm = pairs.get("npm").cloned();
    let api_type = pairs
        .get("apiType")
        .or_else(|| pairs.get("api_type"))
        .cloned();

    Ok(DeepLinkImportRequest {
        version: host.to_string(),
        resource,
        app,
        name,
        enabled,
        homepage,
        endpoint,
        api_key,
        model,
        notes,
        config,
        config_format,
        haiku_model,
        sonnet_model,
        opus_model,
        npm,
        api_type,
    })
}

fn decode_base64(raw: &str) -> Option<String> {
    let cleaned = raw.trim();
    if let Ok(bytes) = STANDARD.decode(cleaned) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    if let Ok(bytes) = STANDARD_NO_PAD.decode(cleaned) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    if let Ok(bytes) = URL_SAFE.decode(cleaned) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    if let Ok(bytes) = URL_SAFE_NO_PAD.decode(cleaned) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    None
}

impl DeepLinkImportRequest {
    pub fn resolve_app_kind(&self) -> Result<AppKind, DomainError> {
        let app_str = self
            .app
            .as_deref()
            .unwrap_or("claude")
            .trim()
            .to_ascii_lowercase();

        match app_str.as_str() {
            "claude" | "claudecode" | "claude_code" | "anthropic" => Ok(AppKind::Claude),
            "codex" | "openai" | "chatgpt" => Ok(AppKind::Codex),
            "grok" | "grokbuild" | "grok_build" | "xai" => Ok(AppKind::Grok),
            "opencode" | "open_code" => Ok(AppKind::OpenCode),
            "pi" | "pi_switch" => Ok(AppKind::Pi),
            "cursor" | "cursor_ide" | "cursorbyok" => Ok(AppKind::Cursor),
            "zcode" | "zai" | "bigmodel" => Ok(AppKind::ZCode),
            "workbuddy" | "work_buddy" | "codebuddy" | "code_buddy" => Ok(AppKind::WorkBuddy),
            other => Err(DomainError::Validation(format!(
                "不支持的应用类型: '{other}', 可用应用: claude, codex, grok, opencode, pi, cursor, zcode, workbuddy"
            ))),
        }
    }

    pub fn to_provider_form(&self) -> Result<(AppKind, ProviderForm, bool), DomainError> {
        let app_kind = self.resolve_app_kind()?;
        let is_enabled = self.enabled.unwrap_or(false);

        // 1. Extract values from config (Base64 JSON or TOML) if provided
        let mut cfg_api_key: Option<String> = None;
        let mut cfg_base_url: Option<String> = None;
        let mut cfg_model: Option<String> = None;
        let mut cfg_haiku: Option<String> = None;
        let mut cfg_sonnet: Option<String> = None;
        let mut cfg_opus: Option<String> = None;

        if let Some(config_b64) = &self.config {
            if let Some(decoded) = decode_base64(config_b64) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                    // Extract Claude env
                    let env_obj = json
                        .get("env")
                        .and_then(|v| v.as_object())
                        .or_else(|| json.as_object());

                    if let Some(env) = env_obj {
                        if let Some(k) = env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) {
                            cfg_api_key = Some(k.to_string());
                        } else if let Some(k) = env.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                            cfg_api_key = Some(k.to_string());
                        } else if let Some(k) = env.get("GEMINI_API_KEY").and_then(|v| v.as_str()) {
                            cfg_api_key = Some(k.to_string());
                        } else if let Some(k) = env.get("apiKey").and_then(|v| v.as_str()) {
                            cfg_api_key = Some(k.to_string());
                        }

                        if let Some(u) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                            cfg_base_url = Some(u.to_string());
                        } else if let Some(u) = env.get("OPENAI_BASE_URL").and_then(|v| v.as_str())
                        {
                            cfg_base_url = Some(u.to_string());
                        } else if let Some(u) = env.get("GEMINI_BASE_URL").and_then(|v| v.as_str())
                        {
                            cfg_base_url = Some(u.to_string());
                        } else if let Some(u) = env
                            .get("baseUrl")
                            .or_else(|| env.get("baseURL"))
                            .and_then(|v| v.as_str())
                        {
                            cfg_base_url = Some(u.to_string());
                        }

                        if let Some(m) = env.get("ANTHROPIC_MODEL").and_then(|v| v.as_str()) {
                            cfg_model = Some(m.to_string());
                        } else if let Some(m) = env.get("OPENAI_MODEL").and_then(|v| v.as_str()) {
                            cfg_model = Some(m.to_string());
                        } else if let Some(m) = env.get("GEMINI_MODEL").and_then(|v| v.as_str()) {
                            cfg_model = Some(m.to_string());
                        } else if let Some(m) = env.get("model").and_then(|v| v.as_str()) {
                            cfg_model = Some(m.to_string());
                        }

                        if let Some(m) = env
                            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                            .and_then(|v| v.as_str())
                        {
                            cfg_haiku = Some(m.to_string());
                        }
                        if let Some(m) = env
                            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                            .and_then(|v| v.as_str())
                        {
                            cfg_sonnet = Some(m.to_string());
                        }
                        if let Some(m) = env
                            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
                            .and_then(|v| v.as_str())
                        {
                            cfg_opus = Some(m.to_string());
                        }
                    }

                    // Extract Codex auth / config
                    if let Some(auth) = json.get("auth").and_then(|v| v.as_object()) {
                        if let Some(k) = auth.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
                            cfg_api_key = Some(k.to_string());
                        }
                    }
                    if let Some(cfg_toml) = json.get("config").and_then(|v| v.as_str()) {
                        if let Ok(parsed_toml) = toml::from_str::<toml::Value>(cfg_toml) {
                            if let Some(u) = parsed_toml
                                .get("model_providers")
                                .and_then(|v| v.get("openai").or_else(|| v.get("custom")))
                                .and_then(|v| v.get("base_url"))
                                .and_then(|v| v.as_str())
                            {
                                cfg_base_url = Some(u.to_string());
                            }
                            if let Some(m) = parsed_toml
                                .get("general")
                                .or_else(|| Some(&parsed_toml))
                                .and_then(|v| v.get("model"))
                                .and_then(|v| v.as_str())
                            {
                                cfg_model = Some(m.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 2. URL parameters override config values (Precedence rule)
        let primary_endpoint = self
            .endpoint
            .as_deref()
            .and_then(|ep| ep.split(',').next())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or(cfg_base_url)
            .unwrap_or_default();

        let api_key = self
            .api_key
            .clone()
            .filter(|s| !s.is_empty())
            .or(cfg_api_key)
            .unwrap_or_default();

        let model = self.model.clone().filter(|s| !s.is_empty()).or(cfg_model);

        let name = self
            .name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{} 导入配置", app_kind.display_name()));

        let website_url = self
            .homepage
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                if !primary_endpoint.is_empty() {
                    Url::parse(&primary_endpoint).ok().map(|u| {
                        format!(
                            "{}://{}",
                            u.scheme(),
                            u.host_str().unwrap_or("api.anthropic.com")
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let form = match app_kind {
            AppKind::Claude | AppKind::ClaudeDesktop => {
                let sonnet = self
                    .sonnet_model
                    .clone()
                    .or(cfg_sonnet)
                    .or(self.opus_model.clone())
                    .or(cfg_opus)
                    .or(self.haiku_model.clone())
                    .or(cfg_haiku)
                    .or(model.clone())
                    .unwrap_or_else(|| DEFAULT_CLAUDE_MODEL.to_string());

                ProviderForm::Claude(ClaudeForm {
                    name,
                    website_url,
                    kind: ClaudeKind::ThirdParty,
                    api_key,
                    base_url: primary_endpoint,
                    model: sonnet,
                    request_protocol: String::new(),
                    model_mappings: Vec::new(),
                    desktop_mode: None,
                })
            }
            AppKind::Codex => {
                let m = model.unwrap_or_else(|| DEFAULT_CODEX_MODEL.to_string());
                ProviderForm::Codex(CodexForm {
                    name,
                    website_url,
                    kind: CodexKind::ResponsesThirdParty,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    request_protocol: String::new(),
                    model_mappings: Vec::new(),
                })
            }
            AppKind::Grok => {
                let m = model.unwrap_or_else(|| DEFAULT_GROK_MODEL.to_string());
                ProviderForm::Grok(GrokForm {
                    name,
                    website_url,
                    kind: GrokKind::ThirdParty,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    request_protocol: String::new(),
                    model_mappings: Vec::new(),
                })
            }
            AppKind::OpenCode => {
                let m = model.unwrap_or_else(|| DEFAULT_OPENCODE_MODEL.to_string());
                let npm = self
                    .npm
                    .clone()
                    .unwrap_or_else(|| DEFAULT_OPENCODE_NPM.to_string());
                ProviderForm::OpenCode(OpenCodeForm {
                    name,
                    website_url,
                    kind: OpenCodeKind::ThirdParty,
                    npm,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    request_protocol: String::new(),
                    model_mappings: Vec::new(),
                })
            }
            AppKind::Pi => {
                let m = model.unwrap_or_else(|| DEFAULT_PI_MODEL.to_string());
                let api_type = self
                    .api_type
                    .clone()
                    .unwrap_or_else(|| DEFAULT_PI_API_TYPE.to_string());
                ProviderForm::Pi(PiForm {
                    name,
                    website_url,
                    kind: PiKind::ThirdParty,
                    api_type,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    request_protocol: String::new(),
                    model_mappings: Vec::new(),
                })
            }
            AppKind::Cursor => {
                let m = model.unwrap_or_else(|| DEFAULT_CURSOR_MODEL.to_string());
                let provider_type = self
                    .api_type
                    .clone()
                    .unwrap_or_else(|| "openai-chat".to_string());
                ProviderForm::Cursor(CursorForm {
                    name,
                    website_url,
                    kind: CursorKind::ThirdParty,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    provider_type,
                    default_reasoning_effort: "high".into(),
                    model_mappings: Vec::new(),
                })
            }
            AppKind::ZCode => {
                let m = model.unwrap_or_else(|| DEFAULT_ZCODE_MODEL.to_string());
                let provider_kind = self
                    .api_type
                    .clone()
                    .unwrap_or_else(|| DEFAULT_ZCODE_PROVIDER_KIND.to_string());
                ProviderForm::ZCode(ZCodeForm {
                    name,
                    website_url,
                    kind: ZCodeKind::ThirdParty,
                    provider_kind,
                    api_key,
                    base_url: primary_endpoint,
                    model: m,
                    request_protocol: String::new(),
                    modality_text: true,
                    modality_image: true,
                    model_mappings: Vec::new(),
                })
            }
            AppKind::WorkBuddy => {
                let m = model.unwrap_or_else(|| DEFAULT_WORKBUDDY_MODEL.to_string());
                ProviderForm::WorkBuddy(WorkBuddyForm {
                    name,
                    website_url,
                    kind: WorkBuddyKind::ThirdParty,
                    model_id: m,
                    vendor: DEFAULT_WORKBUDDY_VENDOR.to_string(),
                    base_url: primary_endpoint,
                    api_key,
                    supports_tool_call: true,
                    supports_images: true,
                    supports_reasoning: false,
                    reasoning_only: false,
                    can_disable_reasoning: true,
                    use_custom_protocol: false,
                    max_input_tokens: Some(262144),
                    max_output_tokens: Some(65536),
                    reasoning_effort: "medium".to_string(),
                    supported_reasoning_efforts: vec!["medium".to_string()],
                })
            }
        };

        Ok((app_kind, form, is_enabled))
    }
}

/// Build `ccswitch://v1/import?...` for opening CC Switch (AstrLink `cc_switch::import_url`).
///
/// Does not echo `api_key` in error messages.
pub fn build_cc_switch_import_url(
    app: &str,
    name: &str,
    inference_url: &str,
    api_key: &str,
    model: Option<&str>,
    haiku_model: Option<&str>,
    sonnet_model: Option<&str>,
    opus_model: Option<&str>,
) -> Result<String, String> {
    let app = app.trim().to_ascii_lowercase();
    let app = match app.as_str() {
        "claude" | "codex" | "gemini" | "opencode" | "openclaw" => app,
        _ => return Err("unsupported client".into()),
    };

    let mut endpoint = Url::parse(inference_url.trim()).map_err(|_| "invalid inference URL")?;
    if endpoint.scheme() != "http"
        || endpoint.host_str() != Some("127.0.0.1")
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err("invalid local inference URL".into());
    }
    // Accept bare origin or `/` / `/v1`; normalize path before client-specific rewrite.
    let path = endpoint.path();
    if path != "/" && path != "" && path != "/v1" {
        return Err("invalid local inference URL".into());
    }
    endpoint.set_path("/");
    endpoint.set_query(None);
    endpoint.set_fragment(None);

    let name = name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err("invalid provider name".into());
    }
    if api_key.is_empty() {
        return Err("access token is unavailable".into());
    }

    let mut model_params: Vec<(&str, String)> = Vec::new();
    let push_model = |key: &'static str, value: Option<&str>, out: &mut Vec<(&str, String)>| {
        let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(());
        };
        if value.chars().count() > 256 || value.chars().any(char::is_control) {
            return Err(format!("invalid {key}"));
        }
        out.push((key, value.to_string()));
        Ok(())
    };
    push_model("model", model, &mut model_params)?;
    if app == "claude" {
        push_model("haikuModel", haiku_model, &mut model_params)?;
        push_model("sonnetModel", sonnet_model, &mut model_params)?;
        push_model("opusModel", opus_model, &mut model_params)?;
    }
    if app != "claude" && model_params.is_empty() {
        return Err("model is required for this client".into());
    }

    if matches!(app.as_str(), "codex" | "opencode" | "openclaw") {
        endpoint.set_path("/v1");
    } else {
        endpoint.set_path("/");
    }

    let mut url = Url::parse("ccswitch://v1/import").expect("static CC Switch URL");
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("resource", "provider")
            .append_pair("app", &app)
            .append_pair("name", name)
            .append_pair("endpoint", endpoint.as_str().trim_end_matches('/'))
            .append_pair("apiKey", api_key)
            .append_pair("enabled", "false");
        for (key, value) in &model_params {
            params.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cc_switch_import_url_like_astrlink() {
        let url = build_cc_switch_import_url(
            "codex",
            "Router Switch · 默认令牌",
            "http://127.0.0.1:8787",
            "rsw_test+token",
            Some("gpt-5.6"),
            None,
            None,
            None,
        )
        .unwrap();
        assert!(url.starts_with("ccswitch://v1/import?"));
        let parsed = Url::parse(&url).unwrap();
        let params: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(params["app"], "codex");
        assert_eq!(params["endpoint"], "http://127.0.0.1:8787/v1");
        assert_eq!(params["model"], "gpt-5.6");
        assert_eq!(params["enabled"], "false");
        assert_eq!(params["apiKey"], "rsw_test+token");
    }

    #[test]
    fn cc_switch_non_claude_requires_model() {
        assert!(build_cc_switch_import_url(
            "codex",
            "name",
            "http://127.0.0.1:8787",
            "key",
            None,
            None,
            None,
            None,
        )
        .is_err());
    }

    #[test]
    fn test_parse_router_switch_claude_url() {
        let url = "router-switch://v1/import?resource=provider&app=claude&name=Claude%20Proxy&endpoint=https%3A%2F%2Fapi.anthropic.com%2Fv1&apiKey=sk-ant-test-123&model=claude-3-7-sonnet&enabled=true";
        let req = parse_deeplink_url(url).unwrap();
        assert_eq!(req.resource, "provider");
        assert_eq!(req.app.as_deref(), Some("claude"));
        assert_eq!(req.name.as_deref(), Some("Claude Proxy"));
        assert_eq!(
            req.endpoint.as_deref(),
            Some("https://api.anthropic.com/v1")
        );
        assert_eq!(req.api_key.as_deref(), Some("sk-ant-test-123"));
        assert_eq!(req.model.as_deref(), Some("claude-3-7-sonnet"));
        assert_eq!(req.enabled, Some(true));

        let (app, form, enabled) = req.to_provider_form().unwrap();
        assert_eq!(app, AppKind::Claude);
        assert!(enabled);
        if let ProviderForm::Claude(f) = form {
            assert_eq!(f.name, "Claude Proxy");
            assert_eq!(f.base_url, "https://api.anthropic.com/v1");
            assert_eq!(f.api_key, "sk-ant-test-123");
            assert_eq!(f.model, "claude-3-7-sonnet");
        } else {
            panic!("Expected Claude form");
        }
    }

    #[test]
    fn test_parse_ccswitch_compatibility() {
        let url = "ccswitch://v1/import?resource=provider&app=codex&name=OpenAI%20Relay&endpoint=https%3A%2F%2Fapi.openai.com%2Fv1&apiKey=sk-proj-test&model=gpt-5.1";
        let req = parse_deeplink_url(url).unwrap();
        assert_eq!(req.app.as_deref(), Some("codex"));
        let (app, form, enabled) = req.to_provider_form().unwrap();
        assert_eq!(app, AppKind::Codex);
        assert!(!enabled);
        if let ProviderForm::Codex(f) = form {
            assert_eq!(f.name, "OpenAI Relay");
            assert_eq!(f.base_url, "https://api.openai.com/v1");
            assert_eq!(f.api_key, "sk-proj-test");
            assert_eq!(f.model, "gpt-5.1");
        } else {
            panic!("Expected Codex form");
        }
    }

    #[test]
    fn test_parse_zcode_url() {
        let url = "router-switch://v1/import?resource=provider&app=zcode&name=CCHost%20ZCode&endpoint=https%3A%2F%2Fcchost.ai&apiKey=sk-cchost-123&model=gemini-3.7-flash-high&enabled=true";
        let req = parse_deeplink_url(url).unwrap();
        assert_eq!(req.app.as_deref(), Some("zcode"));
        let (app, form, enabled) = req.to_provider_form().unwrap();
        assert_eq!(app, AppKind::ZCode);
        assert!(enabled);
        if let ProviderForm::ZCode(f) = form {
            assert_eq!(f.name, "CCHost ZCode");
            assert_eq!(f.base_url, "https://cchost.ai");
            assert_eq!(f.api_key, "sk-cchost-123");
            assert_eq!(f.model, "gemini-3.7-flash-high");
        } else {
            panic!("Expected ZCode form");
        }
    }

    #[test]
    fn test_parse_workbuddy_url() {
        let url = "router-switch://v1/import?resource=provider&app=workbuddy&name=CCHost%20Gemini&endpoint=https%3A%2F%2Fcchost.ai%2Fv1&apiKey=sk-wb-mock-123&model=gemini-3.7-flash-high&enabled=true";
        let req = parse_deeplink_url(url).unwrap();
        assert_eq!(req.app.as_deref(), Some("workbuddy"));
        let (app, form, _) = req.to_provider_form().unwrap();
        assert_eq!(app, AppKind::WorkBuddy);
        if let ProviderForm::WorkBuddy(f) = form {
            assert_eq!(f.name, "CCHost Gemini");
            assert_eq!(f.base_url, "https://cchost.ai/v1");
            assert_eq!(f.api_key, "sk-wb-mock-123");
            assert_eq!(f.model_id, "gemini-3.7-flash-high");
        } else {
            panic!("Expected WorkBuddy form");
        }
    }

    #[test]
    fn test_parse_base64_config_with_url_override() {
        let config_json = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-from-config","ANTHROPIC_BASE_URL":"https://config.example.com","ANTHROPIC_MODEL":"claude-haiku"}}"#;
        let b64 = STANDARD.encode(config_json);
        let url = format!("router-switch://v1/import?resource=provider&app=claude&name=Config%20Import&apiKey=sk-override-key&config={}", b64);
        let req = parse_deeplink_url(&url).unwrap();
        let (app, form, _) = req.to_provider_form().unwrap();
        assert_eq!(app, AppKind::Claude);
        if let ProviderForm::Claude(f) = form {
            assert_eq!(f.api_key, "sk-override-key"); // URL parameter overrides config
            assert_eq!(f.base_url, "https://config.example.com"); // extracted from config
            assert_eq!(f.model, "claude-haiku"); // extracted from config
        } else {
            panic!("Expected Claude form");
        }
    }
}
