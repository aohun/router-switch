//! Pure domain crate for AI provider management across Codex, Claude Code, Grok Build, OpenCode, Pi, Cursor, ZCode, and WorkBuddy.
//! No filesystem or SQLite dependencies here.

mod access_token;
mod api_provider;
mod app_kind;
mod claude;
mod clipboard;
mod codex;
mod connectivity;
mod cursor;
mod deeplink;
mod env_checker;
mod error;
mod grok;
mod opencode;
mod pi;
mod prompt;
mod protocol;
mod provider;
mod routing_settings;
mod updater;
mod usage_script;
mod workbuddy;
mod zcode;

pub use access_token::{
    normalize_access_token_name, token_hint, AccessTokenSource, AccessTokenSummary,
    AccessTokenUsage, CreatedAccessToken, NewAccessTokenRecord, ACCESS_TOKEN_ID_PREFIX,
    ACCESS_TOKEN_LIMIT, ACCESS_TOKEN_PREFIX, DEFAULT_ACCESS_TOKEN_NAME,
};
pub use api_provider::{
    api_protocol_descriptor, fetch_http_connection_models, merge_discovered_api_models,
    merge_visible_api_provider_order, resolve_upstream_candidates, supports_local_conversion,
    ApiAuthScheme, ApiCapability, ApiProtocolDescriptor, ApiProvider, ApiProviderKind,
    ApiProviderKindGroup, HttpConnection, SubscriptionConnection, SubscriptionStatus,
    API_PROTOCOL_CATALOG, CONVERTIBLE_PROTOCOL_IDS,
};
pub use app_kind::AppKind;
pub use claude::{
    backfill_claude_settings, extract_claude_api_key, extract_claude_base_url,
    extract_claude_model, extract_claude_provider_name, generate_claude_env,
    official_claude_desktop_provider, official_claude_provider, official_claude_settings,
    parse_claude_form, ClaudeForm, ClaudeKind, ClaudeModelMapping, ClaudePreset, ClaudeSettings,
    CLAUDE_DESKTOP_MODE_DIRECT, CLAUDE_DESKTOP_MODE_MAPPING, CLAUDE_DESKTOP_ONE_M_WINDOW,
    CLAUDE_DESKTOP_ROUTES, CLAUDE_PRESETS, DEFAULT_CLAUDE_MODEL, OFFICIAL_CLAUDE_DESKTOP_ID,
    OFFICIAL_CLAUDE_ID,
};
pub use clipboard::{parse_clipboard_provider_info, ClipboardProviderInfo};
pub use codex::{
    backfill_codex_settings, extract_codex_api_key, extract_codex_base_url, extract_codex_model,
    extract_codex_provider_name, fetch_models_from_api, generate_catalog_json,
    generate_third_party_auth, generate_third_party_config,
    generate_third_party_config_with_catalog, has_login_material, official_codex_provider,
    official_codex_settings, parse_codex_form, CodexForm, CodexKind, CodexModelMapping,
    CodexPreset, CodexSettings, DEFAULT_CODEX_MODEL, OFFICIAL_CODEX_ID, REASONING_LEVELS,
    RESPONSES_PRESETS,
};
pub use connectivity::{
    check_reachability, check_reachability_with_retry, extract_provider_probe_target,
    test_provider_connectivity, ConnectivityCheckConfig, ConnectivityCheckResult, HealthStatus,
};
pub use cursor::{
    backfill_cursor_settings, extract_cursor_api_key, extract_cursor_base_url,
    extract_cursor_model, extract_cursor_provider_type, official_cursor_provider,
    official_cursor_settings, parse_cursor_form, CursorForm, CursorKind, CursorModelMapping,
    CursorPreset, CursorSettings, CURSOR_PRESETS, DEFAULT_CURSOR_MODEL,
    DEFAULT_CURSOR_PROVIDER_TYPE, OFFICIAL_CURSOR_ID,
};
pub use deeplink::{parse_deeplink_url, DeepLinkImportRequest};
pub use env_checker::{
    build_tool_search_paths, compare_semver, extract_version, fetch_remote_latest_version,
    infer_install_source, inspect_all_tools, inspect_tool_environment, is_version_outdated,
    parse_semver, resolve_path_default, ToolEnvironmentStatus, ToolInstallation,
};
pub use error::DomainError;
pub use grok::{
    backfill_grok_settings, extract_grok_api_key, extract_grok_base_url, extract_grok_model,
    extract_grok_provider_name, generate_grok_config_toml, official_grok_provider,
    official_grok_settings, parse_grok_form, GrokForm, GrokKind, GrokModelMapping, GrokPreset,
    GrokSettings, DEFAULT_GROK_MODEL, GROK_PRESETS, OFFICIAL_GROK_ID,
};
pub use opencode::{
    extract_opencode_api_key, extract_opencode_base_url, extract_opencode_model,
    extract_opencode_options, generate_opencode_provider_json, official_opencode_provider,
    official_opencode_settings, parse_opencode_form, OpenCodeForm, OpenCodeKind,
    OpenCodeModelMapping, OpenCodePreset, OpenCodeSettings, DEFAULT_OPENCODE_MODEL,
    DEFAULT_OPENCODE_NPM, OFFICIAL_OPENCODE_ID, OPENCODE_PRESETS,
};
pub use pi::{
    generate_pi_models_json, generate_pi_settings_json, official_pi_provider, official_pi_settings,
    parse_pi_form, PiForm, PiKind, PiModelMapping, PiPreset, PiSettings, DEFAULT_PI_API_TYPE,
    DEFAULT_PI_MODEL, OFFICIAL_PI_ID, PI_PRESETS,
};
pub use prompt::{prompt_display_path, prompt_file_path, prompt_filename, Prompt};
pub use protocol::{
    normalize_thinking_effort, RequestProtocol, DEFAULT_THINKING_EFFORT, THINKING_EFFORTS,
};
pub use provider::{new_provider_id, Provider, ProviderSettings};
pub use routing_settings::{
    default_failure_policy, ensure_builtin_redirects, is_builtin_model_redirect,
    model_redirect_issues, validate_routing_settings, ChannelStickiness, FailoverStrategy,
    FailureAction, FailurePolicy, ModelRedirect, ModelRedirectIssue, RoutingSettings,
    BUILTIN_CODEX_AUTO_REVIEW_DEFAULT_TO, BUILTIN_CODEX_AUTO_REVIEW_FROM, MAX_MODEL_REDIRECTS,
    MAX_REDIRECT_MODEL_LEN,
};
pub use updater::{
    apply_downloaded_update, check_app_update, download_release_asset, parse_release_notes,
    sample_app_release, AppRelease, ApplyUpdateOutcome,
};
pub use usage_script::{
    failed_usage_result, parse_usage_result, preset_template, template_display_name, UsageDataItem,
    UsageQueryResult, UsageScriptConfig, TEMPLATE_CUSTOM, TEMPLATE_GENERAL, TEMPLATE_NEW_API,
};
pub use workbuddy::{
    is_workbuddy_upstream, official_workbuddy_provider, official_workbuddy_settings,
    parse_workbuddy_form, workbuddy_codex_mappings, workbuddy_model_context_length,
    workbuddy_model_ids, WorkBuddyForm, WorkBuddyKind, WorkBuddyModelItem, WorkBuddyPreset,
    WorkBuddyReasoningConfig, WorkBuddySettings, DEFAULT_WORKBUDDY_MODEL, DEFAULT_WORKBUDDY_VENDOR,
    OFFICIAL_WORKBUDDY_ID, WORKBUDDY_CODEBUDDY_PRESET_ID, WORKBUDDY_DEFAULT_CODEBUDDY_MODEL,
    WORKBUDDY_MODEL_IDS, WORKBUDDY_PRESETS, WORKBUDDY_UPSTREAM_BASE, WORKBUDDY_WEBSITE_URL,
};
pub use zcode::{
    extract_zcode_api_key, extract_zcode_base_url, extract_zcode_modalities, extract_zcode_model,
    extract_zcode_options, generate_zcode_provider_json, official_zcode_provider,
    official_zcode_settings, parse_zcode_form, ZCodeForm, ZCodeKind, ZCodeModelMapping,
    ZCodePreset, ZCodeSettings, DEFAULT_ZCODE_MODEL, DEFAULT_ZCODE_PROVIDER_KIND,
    OFFICIAL_ZCODE_ID, ZCODE_PRESETS,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderForm {
    Codex(CodexForm),
    Claude(ClaudeForm),
    Grok(GrokForm),
    OpenCode(OpenCodeForm),
    Pi(PiForm),
    Cursor(CursorForm),
    ZCode(ZCodeForm),
    WorkBuddy(WorkBuddyForm),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn third_party_config_is_responses_custom() {
        let toml =
            generate_third_party_config("PackyCode", "https://www.packyapi.ai/v1", "gpt-5.6-sol");
        assert!(toml.contains("model_provider = \"custom\""));
        assert!(toml.contains("wire_api = \"responses\""));
        assert!(toml.contains("requires_openai_auth = true"));
        assert!(toml.contains("base_url = \"https://www.packyapi.ai/v1\""));
        assert!(toml.contains("model = \"gpt-5.6-sol\""));
        assert_eq!(
            extract_codex_base_url(&toml).as_deref(),
            Some("https://www.packyapi.ai/v1")
        );
        assert_eq!(extract_codex_model(&toml).as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(
            extract_codex_provider_name(&toml).as_deref(),
            Some("PackyCode")
        );
    }

    #[test]
    fn quotes_are_escaped_in_toml_strings() {
        let toml = generate_third_party_config(r#"Acme "Labs""#, "https://example.com/v1", "m");
        assert!(toml.contains(r#"name = "Acme \"Labs\"""#));
    }

    #[test]
    fn login_material_detects_oauth_and_api_key() {
        assert!(!has_login_material(&json!({})));
        assert!(!has_login_material(&json!({"OPENAI_API_KEY": ""})));
        assert!(has_login_material(&json!({"OPENAI_API_KEY": "sk-test"})));
        assert!(has_login_material(
            &json!({"tokens": {"access_token": "chatgpt-oauth"}})
        ));
    }

    #[test]
    fn parse_form_requires_third_party_fields() {
        let err = parse_codex_form(CodexForm {
            name: "x".into(),
            website_url: String::new(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        })
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn official_form_does_not_need_key() {
        let settings = parse_codex_form(CodexForm {
            name: "OpenAI Official".into(),
            website_url: "https://chatgpt.com/codex".into(),
            kind: CodexKind::Official,
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        })
        .unwrap();
        assert_eq!(settings.kind, CodexKind::Official);
        assert!(!has_login_material(&settings.auth));
        assert!(settings.config_toml.trim().is_empty());
    }

    #[test]
    fn claude_env_generation_works() {
        let form = ClaudeForm {
            name: "Anthropic Third Party".into(),
            website_url: "https://example.com".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-ant-test".into(),
            base_url: "https://api.example.com".into(),
            model: "claude-3-7-sonnet-20250219".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
            desktop_mode: None,
        };
        let settings = parse_claude_form(form).unwrap();
        assert_eq!(settings.kind, ClaudeKind::ThirdParty);
        let env_obj = settings
            .env
            .get("env")
            .and_then(|v| v.as_object())
            .or_else(|| settings.env.as_object())
            .unwrap();
        assert_eq!(
            env_obj.get("ANTHROPIC_BASE_URL").unwrap(),
            "https://api.example.com"
        );
        assert_eq!(env_obj.get("ANTHROPIC_AUTH_TOKEN").unwrap(), "sk-ant-test");
        assert_eq!(
            env_obj.get("ANTHROPIC_MODEL").unwrap(),
            "claude-3-7-sonnet-20250219"
        );
    }

    #[test]
    fn grok_config_generation_works() {
        let form = GrokForm {
            name: "Packy Grok".into(),
            website_url: "https://packy.ai".into(),
            kind: GrokKind::ThirdParty,
            api_key: "xai-test-key".into(),
            base_url: "https://api.packy.ai/v1".into(),
            model: "grok-4.5".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let settings = parse_grok_form(form).unwrap();
        assert_eq!(settings.kind, GrokKind::ThirdParty);
        assert!(settings
            .config_toml
            .contains("base_url = \"https://api.packy.ai/v1\""));
        assert!(settings.config_toml.contains("api_key = \"xai-test-key\""));
        assert!(settings.config_toml.contains("model = \"grok-4.5\""));
    }

    #[test]
    fn opencode_config_generation_works() {
        let form = OpenCodeForm {
            name: "DeepSeek Provider".into(),
            website_url: "https://deepseek.com".into(),
            kind: OpenCodeKind::ThirdParty,
            npm: "@ai-sdk/openai-compatible".into(),
            api_key: "sk-ds-123".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            request_protocol: String::new(),
            model_mappings: vec![OpenCodeModelMapping {
                model_id: "deepseek-reasoner".into(),
                display_name: "DeepSeek R1".into(),
                context_limit: Some(64000),
                output_limit: Some(8192),
            }],
        };
        let settings = parse_opencode_form(form).unwrap();
        assert_eq!(settings.kind, OpenCodeKind::ThirdParty);
        let val = generate_opencode_provider_json(&settings, "DeepSeek Provider");
        assert_eq!(val["npm"], "@ai-sdk/openai-compatible");
        assert_eq!(val["options"]["baseURL"], "https://api.deepseek.com/v1");
        assert_eq!(val["options"]["apiKey"], "sk-ds-123");
        assert_eq!(val["models"]["deepseek-chat"]["name"], "deepseek-chat");
        assert_eq!(val["models"]["deepseek-reasoner"]["name"], "DeepSeek R1");
        assert_eq!(
            extract_opencode_api_key(&settings.options).as_deref(),
            Some("sk-ds-123")
        );
        assert_eq!(
            extract_opencode_base_url(&settings.options).as_deref(),
            Some("https://api.deepseek.com/v1")
        );
    }

    #[test]
    fn pi_config_generation_works() {
        let form = PiForm {
            name: "PackyCode Pi".into(),
            website_url: "https://packyapi.ai".into(),
            kind: PiKind::ThirdParty,
            api_type: "openai-completions".into(),
            api_key: "sk-pk-123".into(),
            base_url: "https://www.packyapi.ai/v1".into(),
            model: "gpt-4o".into(),
            request_protocol: String::new(),
            model_mappings: vec![PiModelMapping {
                model_id: "claude-3-7-sonnet".into(),
                display_name: "Claude Sonnet".into(),
                context_window: Some(200000),
            }],
        };
        let settings = parse_pi_form(form).unwrap();
        assert_eq!(settings.kind, PiKind::ThirdParty);
        let models_json = generate_pi_models_json(&settings, "packycode");
        assert_eq!(
            models_json["providers"]["packycode"]["baseUrl"],
            "https://www.packyapi.ai/v1"
        );
        assert_eq!(models_json["providers"]["packycode"]["apiKey"], "sk-pk-123");
        let settings_json = generate_pi_settings_json("packycode", "gpt-4o");
        assert_eq!(settings_json["defaultProvider"], "packycode");
        assert_eq!(settings_json["defaultModel"], "gpt-4o");
    }

    #[test]
    fn zcode_config_generation_works() {
        let form = ZCodeForm {
            name: "cchost".into(),
            website_url: "https://cchost.ai".into(),
            kind: ZCodeKind::ThirdParty,
            provider_kind: "anthropic".into(),
            api_key: "sk-zcode-mock-key-12345".into(),
            base_url: "https://cchost.ai".into(),
            model: "gemini-3.7-flash-high".into(),
            request_protocol: String::new(),
            modality_text: true,
            modality_image: true,
            model_mappings: vec![],
        };
        let settings = parse_zcode_form(form).unwrap();
        assert_eq!(settings.kind, ZCodeKind::ThirdParty);
        let val = generate_zcode_provider_json(&settings, "cchost");
        assert_eq!(val["name"], "cchost");
        assert_eq!(val["kind"], "anthropic");
        assert_eq!(val["source"], "custom");
        assert_eq!(val["options"]["baseURL"], "https://cchost.ai");
        assert_eq!(val["options"]["apiKey"], "sk-zcode-mock-key-12345");
        assert_eq!(val["options"]["apiKeyRequired"], true);
        assert_eq!(
            val["models"]["gemini-3.7-flash-high"]["limit"]["context"],
            200000
        );
        assert_eq!(
            val["models"]["gemini-3.7-flash-high"]["limit"]["output"],
            128000
        );
        assert_eq!(
            val["models"]["gemini-3.7-flash-high"]["zcode"]["modified"],
            true
        );
        assert_eq!(
            extract_zcode_api_key(&settings.options).as_deref(),
            Some("sk-zcode-mock-key-12345")
        );
        assert_eq!(
            extract_zcode_base_url(&settings.options).as_deref(),
            Some("https://cchost.ai")
        );
        assert_eq!(
            val["models"]["gemini-3.7-flash-high"]["modalities"]["input"],
            json!(["text", "image"])
        );
        let (has_text, has_image) = extract_zcode_modalities(&settings.models);
        assert!(has_text);
        assert!(has_image);

        // Text only
        let form_text_only = ZCodeForm {
            name: "deepseek".into(),
            website_url: "https://deepseek.com".into(),
            kind: ZCodeKind::ThirdParty,
            provider_kind: "openai-compatible".into(),
            api_key: "sk-ds-123".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            request_protocol: String::new(),
            modality_text: true,
            modality_image: false,
            model_mappings: vec![],
        };
        let settings_text_only = parse_zcode_form(form_text_only).unwrap();
        let val_text_only = generate_zcode_provider_json(&settings_text_only, "deepseek");
        assert_eq!(
            val_text_only["models"]["deepseek-chat"]["modalities"]["input"],
            json!(["text"])
        );
        let (t_only, i_only) = extract_zcode_modalities(&settings_text_only.models);
        assert!(t_only);
        assert!(!i_only);
    }

    #[test]
    fn workbuddy_form_and_item_conversion_works() {
        let form = WorkBuddyForm {
            name: "Gemini Flash".into(),
            website_url: "https://cchost.ai".into(),
            kind: WorkBuddyKind::ThirdParty,
            model_id: "gemini-3.7-flash-high".into(),
            vendor: "Custom".into(),
            base_url: "https://cchost.ai/v1".into(),
            api_key: "sk-mock-key-12345".into(),
            supports_tool_call: true,
            supports_images: true,
            supports_reasoning: true,
            reasoning_only: false,
            can_disable_reasoning: true,
            use_custom_protocol: false,
            max_input_tokens: Some(262144),
            max_output_tokens: Some(65536),
            reasoning_effort: "high".into(),
            supported_reasoning_efforts: vec!["medium".into(), "high".into()],
        };
        let settings = parse_workbuddy_form(form).unwrap();
        assert_eq!(settings.kind, WorkBuddyKind::ThirdParty);
        assert_eq!(settings.model_id, "gemini-3.7-flash-high");
        assert_eq!(settings.vendor, "Custom");
        assert_eq!(settings.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(settings.supported_reasoning_efforts, vec!["medium", "high"]);
        assert!(!settings.reasoning_only);
        assert!(settings.can_disable_reasoning);

        let item = settings.to_model_item("Gemini Flash");
        assert_eq!(item.id, "gemini-3.7-flash-high");
        assert_eq!(item.name, "Gemini Flash");
        assert_eq!(item.vendor, "Custom");
        assert_eq!(item.url, "https://cchost.ai/v1");
        assert_eq!(item.api_key, "sk-mock-key-12345");
        assert!(item.supports_tool_call);
        assert!(item.supports_images);
        assert!(item.supports_reasoning);
        assert!(!item.reasoning_only);
        assert!(item.can_disable_reasoning);
        assert!(!item.use_custom_protocol);
        assert_eq!(item.max_input_tokens, Some(262144));
        assert_eq!(item.max_output_tokens, Some(65536));
        assert_eq!(
            item.reasoning.as_ref().unwrap().default_effort.as_deref(),
            Some("high")
        );
        assert_eq!(
            item.reasoning
                .as_ref()
                .unwrap()
                .supported_efforts
                .as_deref(),
            Some(&["medium".to_string(), "high".to_string()][..])
        );

        let (from_item_settings, from_name) = WorkBuddySettings::from_model_item(&item);
        assert_eq!(from_name, "Gemini Flash");
        assert_eq!(from_item_settings.model_id, "gemini-3.7-flash-high");
        assert_eq!(from_item_settings.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(
            from_item_settings.supported_reasoning_efforts,
            vec!["medium", "high"]
        );
    }

    #[test]
    fn workbuddy_upstream_detects_codebuddy_hosts() {
        assert!(is_workbuddy_upstream("https://copilot.tencent.com/v2"));
        assert!(is_workbuddy_upstream("https://www.codebuddy.cn"));
        assert!(!is_workbuddy_upstream("https://api.example.com/v1"));
        assert!(!is_workbuddy_upstream(""));
    }

    #[test]
    fn workbuddy_preset_is_registered_for_routable_apps() {
        assert!(CLAUDE_PRESETS.iter().any(
            |p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID && p.base_url == WORKBUDDY_UPSTREAM_BASE
        ));
        assert!(RESPONSES_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID));
        assert!(GROK_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID));
        assert!(OPENCODE_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID));
        assert!(PI_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID));
        assert!(ZCODE_PRESETS.iter().any(
            |p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID && p.provider_kind == "openai-compatible"
        ));
        assert!(CURSOR_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID && p.provider_type == "openai-chat"));
        assert!(WORKBUDDY_PRESETS
            .iter()
            .any(|p| p.id == WORKBUDDY_CODEBUDDY_PRESET_ID));
    }

    #[test]
    fn fetch_models_returns_workbuddy_catalog_without_network() {
        let models = fetch_models_from_api(WORKBUDDY_UPSTREAM_BASE, "").unwrap();
        assert!(models.contains(&WORKBUDDY_DEFAULT_CODEBUDDY_MODEL.to_string()));
        assert_eq!(models.len(), WORKBUDDY_MODEL_IDS.len());
    }

    #[test]
    fn workbuddy_codex_form_auto_fills_catalog_for_hy4() {
        let settings = parse_codex_form(CodexForm {
            name: "WorkBuddy".into(),
            website_url: WORKBUDDY_WEBSITE_URL.into(),
            kind: CodexKind::ResponsesThirdParty,
            api_key: "sk-mock-key-12345".into(),
            base_url: WORKBUDDY_UPSTREAM_BASE.into(),
            model: "hy4-preview".into(),
            request_protocol: "openai-chat".into(),
            model_mappings: Vec::new(),
        })
        .unwrap();
        assert!(settings
            .model_mappings
            .iter()
            .any(|m| m.model == "hy4-preview"));
        assert!(settings
            .config_toml
            .contains("model_catalog_json = \"router-switch-model-catalog.json\""));
        let catalog = generate_catalog_json(&settings.model_mappings).unwrap();
        assert!(catalog.contains("\"slug\": \"hy4-preview\""));
        assert!(catalog.contains("base_instructions"));
    }
}
