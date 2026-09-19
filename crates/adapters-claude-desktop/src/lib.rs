//! 读写 Claude Desktop 的 3P 推理网关配置。
//!
//! Claude Desktop(macOS)的配置位于:
//! - `~/Library/Application Support/Claude/claude_desktop_config.json`(常规)
//! - `~/Library/Application Support/Claude-3p/claude_desktop_config.json`(3P)
//! - `~/Library/Application Support/Claude-3p/configLibrary/<profile>.json`(网关档案)
//!
//! 第三方供应商: 两个 config 写入 `deploymentMode = "3p"`，档案写入
//! `inferenceGatewayBaseUrl / inferenceGatewayApiKey / inferenceModels`。
//! 官方订阅: `deploymentMode = "1p"`，删除档案，还原官方登录态。

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use domain::{extract_claude_api_key, extract_claude_base_url, ClaudeKind, ClaudeSettings};
use serde_json::{json, Value};
use thiserror::Error;

/// 与 cc-switch 对齐的固定档案 ID(Claude Desktop 模型菜单按档案路由)
pub const PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";
const CONFIG_FILE: &str = "claude_desktop_config.json";
const CONFIG_LIBRARY_DIR: &str = "configLibrary";

#[derive(Debug, Error)]
pub enum ClaudeDesktopError {
    #[error("无法定位 Claude Desktop 配置目录")]
    HomeDir,
    #[error("当前平台不支持 Claude Desktop")]
    UnsupportedPlatform,
    #[error("读写 Claude Desktop 配置失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("Claude Desktop 配置不是合法 JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeDesktopPaths {
    pub normal_config: PathBuf,
    pub threep_config: PathBuf,
    pub profile_path: PathBuf,
    pub meta_path: PathBuf,
}

impl ClaudeDesktopPaths {
    /// macOS 布局; `override_home` 供测试注入(等价于 Application Support)。
    pub fn from_app_support(app_support: impl Into<PathBuf>) -> Self {
        let app_support = app_support.into();
        let normal_dir = app_support.join("Claude");
        let threep_dir = app_support.join("Claude-3p");
        let config_library = threep_dir.join(CONFIG_LIBRARY_DIR);
        Self {
            normal_config: normal_dir.join(CONFIG_FILE),
            threep_config: threep_dir.join(CONFIG_FILE),
            profile_path: config_library.join(format!("{PROFILE_ID}.json")),
            meta_path: config_library.join("_meta.json"),
        }
    }

    pub fn resolve(override_home: Option<&Path>) -> Result<Self, ClaudeDesktopError> {
        if let Some(home) = override_home {
            return Ok(Self::from_app_support(home));
        }
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or(ClaudeDesktopError::HomeDir)?;
            return Ok(Self::from_app_support(
                home.join("Library").join("Application Support"),
            ));
        }
        #[cfg(target_os = "windows")]
        {
            let app_data = dirs::data_dir().ok_or(ClaudeDesktopError::HomeDir)?;
            return Ok(Self::from_app_support(app_data));
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = override_home;
            Err(ClaudeDesktopError::UnsupportedPlatform)
        }
    }
}

/// 解析 Claude Desktop 配置路径(供 session 层使用)
pub fn resolve_claude_desktop_paths(
    override_home: Option<&Path>,
) -> Result<ClaudeDesktopPaths, ClaudeDesktopError> {
    ClaudeDesktopPaths::resolve(override_home)
}

/// 第三方直连供应商要求 env 携带 ANTHROPIC_BASE_URL 与 ANTHROPIC_AUTH_TOKEN。
pub fn is_compatible_direct_settings(settings: &ClaudeSettings) -> bool {
    !extract_claude_base_url(&settings.env)
        .unwrap_or_default()
        .trim()
        .is_empty()
        && !extract_claude_api_key(&settings.env)
            .unwrap_or_default()
            .trim()
            .is_empty()
}

/// Claude Desktop 接入方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMode {
    /// 直连: 档案直接指向供应商端点，模型菜单为实际模型名
    Direct,
    /// 模型映射: 档案暴露 claude-* 安全路由 ID，经本地网关映射到实际模型
    Mapping,
}

pub fn desktop_mode(settings: &ClaudeSettings) -> DesktopMode {
    match settings.desktop_mode.as_deref() {
        Some(domain::CLAUDE_DESKTOP_MODE_MAPPING) => DesktopMode::Mapping,
        _ => DesktopMode::Direct,
    }
}

/// 档案菜单模型: name + labelOverride + supports1m
#[derive(Debug, Clone, PartialEq, Eq)]
struct InferenceModelSpec {
    name: String,
    label_override: Option<String>,
    supports_1m: bool,
}

/// 模型映射模式: 把四档角色投影为固定 claude-* 路由；空档沿用
/// Sonnet(或第一个已填档)的实际模型，确保子-agent 的 Haiku 可用。
fn mapping_model_specs(
    settings: &ClaudeSettings,
) -> Result<Vec<InferenceModelSpec>, ClaudeDesktopError> {
    let mappings = &settings.model_mappings;
    let has_any_model = mappings.iter().any(|m| !m.model.trim().is_empty());
    if !has_any_model {
        return Err(ClaudeDesktopError::Validation(
            "模型映射模式至少要为一个角色填写实际请求模型".into(),
        ));
    }
    Ok(domain::CLAUDE_DESKTOP_ROUTES
        .iter()
        .enumerate()
        .map(|(idx, (_role, route_id))| {
            let mapping = mappings.get(idx);
            let label_override = mapping
                .map(|m| m.display_name.trim().to_string())
                .filter(|label| !label.is_empty());
            let supports_1m = mapping
                .and_then(|m| m.context_window)
                .is_some_and(|window| window >= domain::CLAUDE_DESKTOP_ONE_M_WINDOW);
            InferenceModelSpec {
                name: route_id.to_string(),
                label_override,
                supports_1m,
            }
        })
        .collect())
}

fn inference_model_json(spec: &InferenceModelSpec) -> Value {
    if spec.supports_1m || spec.label_override.is_some() {
        let mut item = json!({ "name": spec.name });
        if let Some(label_override) = &spec.label_override {
            item["labelOverride"] = json!(label_override);
        }
        if spec.supports_1m {
            item["supports1m"] = json!(true);
        }
        item
    } else {
        Value::String(spec.name.clone())
    }
}

/// 把第三方 Claude 供应商投影为 Claude Desktop 3P 网关配置。
pub fn write_live(
    paths: &ClaudeDesktopPaths,
    settings: &ClaudeSettings,
) -> Result<(), ClaudeDesktopError> {
    match settings.kind {
        ClaudeKind::Official => restore_official(paths),
        ClaudeKind::ThirdParty => match desktop_mode(settings) {
            DesktopMode::Direct => apply_direct(paths, settings),
            DesktopMode::Mapping => apply_mapping(paths, settings),
        },
    }
}

fn apply_direct(
    paths: &ClaudeDesktopPaths,
    settings: &ClaudeSettings,
) -> Result<(), ClaudeDesktopError> {
    let base_url = extract_claude_base_url(&settings.env)
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
        .ok_or_else(|| {
            ClaudeDesktopError::Validation(
                "缺少 ANTHROPIC_BASE_URL，无法写入 Claude Desktop 直连配置".into(),
            )
        })?;
    let api_key = extract_claude_api_key(&settings.env)
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            ClaudeDesktopError::Validation(
                "缺少 ANTHROPIC_AUTH_TOKEN，无法写入 Claude Desktop 直连配置".into(),
            )
        })?;

    // 直连模式: 模型菜单为实际模型名(labelOverride = 显示名)。
    // 无映射时省略 inferenceModels，Desktop 展示默认菜单。
    let models: Vec<InferenceModelSpec> = settings
        .model_mappings
        .iter()
        .filter(|mapping| !mapping.model.trim().is_empty())
        .map(|mapping| InferenceModelSpec {
            name: mapping.model.trim().to_string(),
            label_override: {
                let label = mapping.display_name.trim();
                (!label.is_empty()).then(|| label.to_string())
            },
            supports_1m: false,
        })
        .collect();
    write_gateway_profile(paths, &base_url, &api_key, &models)
}

/// 模型映射模式: 档案暴露固定 claude-* 安全路由，请求经本地网关映射。
fn apply_mapping(
    paths: &ClaudeDesktopPaths,
    settings: &ClaudeSettings,
) -> Result<(), ClaudeDesktopError> {
    let base_url = required_base_url(&settings.env)?;
    let api_key = required_api_key(&settings.env)?;
    let models = mapping_model_specs(settings)?;
    write_gateway_profile(paths, &base_url, &api_key, &models)
}

fn required_base_url(env: &Value) -> Result<String, ClaudeDesktopError> {
    extract_claude_base_url(env)
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
        .ok_or_else(|| {
            ClaudeDesktopError::Validation(
                "缺少 ANTHROPIC_BASE_URL，无法写入 Claude Desktop 直连配置".into(),
            )
        })
}

fn required_api_key(env: &Value) -> Result<String, ClaudeDesktopError> {
    extract_claude_api_key(env)
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            ClaudeDesktopError::Validation(
                "缺少 ANTHROPIC_AUTH_TOKEN，无法写入 Claude Desktop 直连配置".into(),
            )
        })
}

fn write_gateway_profile(
    paths: &ClaudeDesktopPaths,
    base_url: &str,
    api_key: &str,
    models: &[InferenceModelSpec],
) -> Result<(), ClaudeDesktopError> {
    let mut profile = json!({
        "coworkEgressAllowedHosts": ["*"],
        "disableDeploymentModeChooser": true,
        "inferenceGatewayApiKey": api_key,
        "inferenceGatewayAuthScheme": "bearer",
        "inferenceGatewayBaseUrl": base_url,
        "inferenceProvider": "gateway",
    });
    if !models.is_empty() {
        profile["inferenceModels"] =
            Value::Array(models.iter().map(inference_model_json).collect());
    }

    write_deployment_mode(&paths.normal_config, "3p")?;
    write_deployment_mode(&paths.threep_config, "3p")?;
    write_json_file(&paths.profile_path, &profile)?;
    write_meta(paths, Some(PROFILE_ID))?;
    Ok(())
}

/// 还原官方登录态: deploymentMode 回 1p，移除网关档案。
pub fn restore_official(paths: &ClaudeDesktopPaths) -> Result<(), ClaudeDesktopError> {
    write_deployment_mode(&paths.normal_config, "1p")?;
    write_deployment_mode(&paths.threep_config, "1p")?;
    if paths.profile_path.exists() {
        fs::remove_file(&paths.profile_path)?;
    }
    write_meta(paths, None)?;
    Ok(())
}

fn read_json_or_empty(path: &Path) -> Result<Value, ClaudeDesktopError> {
    let value = match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => json!({}),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => json!({}),
        Err(err) => return Err(err.into()),
    };
    Ok(if value.is_object() { value } else { json!({}) })
}

fn write_deployment_mode(path: &Path, mode: &str) -> Result<(), ClaudeDesktopError> {
    let mut value = read_json_or_empty(path)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("deploymentMode".into(), json!(mode));
    }
    write_json_file(path, &value)
}

/// _meta.json 记录当前生效档案；为 None 时移除档案登记但保留文件结构。
fn write_meta(paths: &ClaudeDesktopPaths, applied: Option<&str>) -> Result<(), ClaudeDesktopError> {
    let mut meta = read_json_or_empty(&paths.meta_path)?;
    if let Some(obj) = meta.as_object_mut() {
        match applied {
            Some(id) => {
                obj.insert("appliedId".into(), json!(id));
            }
            None => {
                obj.remove("appliedId");
            }
        }
    }
    write_json_file(&paths.meta_path, &meta)
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), ClaudeDesktopError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = serde_json::to_string_pretty(value)?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    let tmp = path.with_extension("json.rs-tmp");
    fs::write(&tmp, body)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(err.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ClaudeModelMapping, RequestProtocol};
    use serde_json::json;

    fn temp_paths() -> (tempfile::TempDir, ClaudeDesktopPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = ClaudeDesktopPaths::from_app_support(dir.path());
        (dir, paths)
    }

    fn third_party() -> ClaudeSettings {
        ClaudeSettings {
            kind: ClaudeKind::ThirdParty,
            env: json!({
                "ANTHROPIC_BASE_URL": "https://api.example.com",
                "ANTHROPIC_AUTH_TOKEN": "sk-mock-key-12345",
            }),
            request_protocol: RequestProtocol::Anthropic.as_str().into(),
            model_mappings: vec![ClaudeModelMapping {
                display_name: "Sonnet".into(),
                model: "claude-sonnet-5".into(),
                context_window: None,
                reasoning_effort: None,
            }],
            desktop_mode: None,
        }
    }

    #[test]
    fn direct_provider_writes_3p_profile_and_deployment_mode() {
        let (_dir, paths) = temp_paths();
        write_live(&paths, &third_party()).unwrap();

        let normal: Value =
            serde_json::from_str(&fs::read_to_string(&paths.normal_config).unwrap()).unwrap();
        assert_eq!(normal["deploymentMode"], "3p");
        let threep: Value =
            serde_json::from_str(&fs::read_to_string(&paths.threep_config).unwrap()).unwrap();
        assert_eq!(threep["deploymentMode"], "3p");

        let profile: Value =
            serde_json::from_str(&fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
        assert_eq!(profile["inferenceProvider"], "gateway");
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://api.example.com"
        );
        assert_eq!(profile["inferenceGatewayApiKey"], "sk-mock-key-12345");
        assert_eq!(profile["inferenceGatewayAuthScheme"], "bearer");
        let models = profile["inferenceModels"].as_array().unwrap();
        assert_eq!(models[0]["name"], "claude-sonnet-5");
        assert_eq!(models[0]["labelOverride"], "Sonnet");

        let meta: Value =
            serde_json::from_str(&fs::read_to_string(&paths.meta_path).unwrap()).unwrap();
        assert_eq!(meta["appliedId"], PROFILE_ID);
    }

    #[test]
    fn direct_write_preserves_existing_config_keys() {
        let (_dir, paths) = temp_paths();
        fs::create_dir_all(paths.normal_config.parent().unwrap()).unwrap();
        fs::write(
            &paths.normal_config,
            r#"{"deploymentMode": "1p", "mcpServers": {"fs": {}}}"#,
        )
        .unwrap();
        write_live(&paths, &third_party()).unwrap();
        let normal: Value =
            serde_json::from_str(&fs::read_to_string(&paths.normal_config).unwrap()).unwrap();
        assert_eq!(normal["deploymentMode"], "3p");
        assert!(normal["mcpServers"].is_object());
    }

    #[test]
    fn official_restores_1p_and_removes_profile() {
        let (_dir, paths) = temp_paths();
        write_live(&paths, &third_party()).unwrap();
        write_live(
            &paths,
            &ClaudeSettings {
                kind: ClaudeKind::Official,
                env: json!({}),
                request_protocol: RequestProtocol::Anthropic.as_str().into(),
                model_mappings: Vec::new(),
                desktop_mode: None,
            },
        )
        .unwrap();

        let normal: Value =
            serde_json::from_str(&fs::read_to_string(&paths.normal_config).unwrap()).unwrap();
        assert_eq!(normal["deploymentMode"], "1p");
        assert!(!paths.profile_path.exists());
        let meta: Value =
            serde_json::from_str(&fs::read_to_string(&paths.meta_path).unwrap()).unwrap();
        assert!(meta.get("appliedId").is_none());
    }

    #[test]
    fn mapping_mode_writes_claude_safe_routes_with_1m_and_labels() {
        let (_dir, paths) = temp_paths();
        let mut settings = third_party();
        settings.desktop_mode = Some(domain::CLAUDE_DESKTOP_MODE_MAPPING.into());
        // 四档: Sonnet(1M) / Opus(DeepSeek) / Fable 空(沿用 Sonnet) / Haiku
        settings.model_mappings = vec![
            ClaudeModelMapping {
                display_name: "gemini-3.7-flash-high".into(),
                model: "gpt-5.4-mini".into(),
                context_window: Some(domain::CLAUDE_DESKTOP_ONE_M_WINDOW),
                reasoning_effort: None,
            },
            ClaudeModelMapping {
                display_name: "DeepSeek V4 Pro".into(),
                model: "deepseek-v4-pro".into(),
                context_window: None,
                reasoning_effort: None,
            },
            ClaudeModelMapping {
                display_name: String::new(),
                model: String::new(),
                context_window: None,
                reasoning_effort: None,
            },
            ClaudeModelMapping {
                display_name: "DeepSeek V4 Flash".into(),
                model: "deepseek-v4-flash".into(),
                context_window: None,
                reasoning_effort: None,
            },
        ];
        write_live(&paths, &settings).unwrap();

        let profile: Value =
            serde_json::from_str(&fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
        let models = profile["inferenceModels"].as_array().unwrap();
        assert_eq!(models.len(), 4);
        assert_eq!(models[0]["name"], "claude-sonnet-5");
        assert_eq!(models[0]["labelOverride"], "gemini-3.7-flash-high");
        assert_eq!(models[0]["supports1m"], true);
        assert_eq!(models[1]["name"], "claude-opus-5");
        // Fable 档留空 -> 无覆盖时为裸字符串形态(cc-switch 同款)
        assert_eq!(models[2], json!("claude-fable-5"));
        assert_eq!(models[3]["name"], "claude-haiku-4-5");
        // 直连字段不变
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://api.example.com"
        );

        // 全空映射应报校验错误
        settings.model_mappings = vec![
            ClaudeModelMapping {
                display_name: String::new(),
                model: String::new(),
                context_window: None,
                reasoning_effort: None,
            };
            4
        ];
        assert!(matches!(
            write_live(&paths, &settings),
            Err(ClaudeDesktopError::Validation(_))
        ));
    }

    #[test]
    fn direct_provider_requires_base_url_and_token() {
        let mut settings = third_party();
        settings.env = json!({"ANTHROPIC_BASE_URL": "https://api.example.com"});
        let (_dir, paths) = temp_paths();
        assert!(matches!(
            write_live(&paths, &settings),
            Err(ClaudeDesktopError::Validation(_))
        ));
        assert!(!is_compatible_direct_settings(&settings));

        settings.env = json!({"ANTHROPIC_AUTH_TOKEN": "sk-mock"});
        assert!(!is_compatible_direct_settings(&settings));
        assert!(is_compatible_direct_settings(&third_party()));
    }

    #[test]
    fn resolve_paths_does_not_fail_on_supported_platforms() {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let paths = ClaudeDesktopPaths::resolve(None);
            assert!(paths.is_ok());
        }
    }
}
