//! Read/write ~/.codex live files. Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{
    extract_codex_api_key, generate_catalog_json, has_login_material, CodexKind, CodexSettings,
};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodexAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 Codex 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("auth.json 不是合法 JSON: {0}")]
    AuthJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexPaths {
    pub home: PathBuf,
    pub auth: PathBuf,
    pub config: PathBuf,
    pub catalog: PathBuf,
    pub auth_official_bak: PathBuf,
    pub config_official_bak: PathBuf,
}

impl CodexPaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            auth: home.join("auth.json"),
            config: home.join("config.toml"),
            catalog: home.join("router-switch-model-catalog.json"),
            auth_official_bak: home.join("auth.json.official.bak"),
            config_official_bak: home.join("config.toml.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveCodex {
    pub auth: Value,
    pub config_toml: String,
}

/// 共享 custom 供应商标识: 统一会话历史开启时官方配置以此 id 运行，
/// 使官方与第三方会话落入同一个 resume 历史桶。
pub const UNIFIED_PROVIDER_ID: &str = "custom";

/// Codex live 写入策略选项(来自应用设置)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CodexWriteOptions {
    /// 非接管切换第三方供应商时保留 auth.json 的官方 ChatGPT 登录，
    /// 第三方密钥改经 config.toml 的 experimental_bearer_token 承载。
    pub preserve_official_auth: bool,
    /// 统一 Codex 会话历史: 官方配置注入共享 custom 供应商标识。
    pub unify_session_history: bool,
}

pub fn resolve_codex_paths(override_home: Option<&Path>) -> Result<CodexPaths, CodexAdapterError> {
    if let Some(home) = override_home {
        return Ok(CodexPaths::from_home(home));
    }
    if let Ok(home) = std::env::var("CODEX_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(CodexPaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(CodexAdapterError::HomeDir)?;
    Ok(CodexPaths::from_home(home.join(".codex")))
}

pub fn read_live(paths: &CodexPaths) -> Result<LiveCodex, CodexAdapterError> {
    let auth = match fs::read_to_string(&paths.auth) {
        Ok(text) if text.trim().is_empty() => serde_json::json!({}),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(err.into()),
    };
    let config_toml = match fs::read_to_string(&paths.config) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };
    Ok(LiveCodex { auth, config_toml })
}

/// live 配置是否为官方形态: 无显式 `model_provider`，或仅带本模块注入的
/// 统一会话共享 custom 路由(形态精确匹配、无 base_url/bearer 差异字段)。
/// 统一会话开启时官方 live 也带 `model_provider = "custom"`，不能据此误判
/// 为第三方而跳过官方备份。
fn looks_official_live_config(config_text: &str) -> bool {
    match toml::from_str::<toml::Value>(config_text) {
        Ok(config) => match config.get("model_provider") {
            None => true,
            Some(provider) if provider.as_str() == Some(UNIFIED_PROVIDER_ID) => config
                .get("model_providers")
                .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
                .and_then(|v| v.as_table())
                .is_some_and(|custom| custom.len() == 4 && custom_table_matches_unified(&config)),
            Some(_) => false,
        },
        // 解析失败退回旧的文本启发式
        Err(_) => {
            !config_text.contains("model_provider = \"custom\"")
                && !config_text.contains("wire_api = \"responses\"")
        }
    }
}

/// Backup current official config / auth files if they look like official configs.
pub fn backup_official_if_needed(paths: &CodexPaths) -> Result<(), CodexAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official = looks_official_live_config(&live.config_toml);

    if is_official {
        if paths.auth.exists() {
            let _ = fs::copy(&paths.auth, &paths.auth_official_bak);
        }
        if paths.config.exists() {
            // 备份剥离统一路由注入，还原得到纯净官方配置；
            // 统一会话仍开启时官方写入会重新注入。
            let clean = strip_unified_session_bucket(&live.config_toml);
            let _ = fs::write(&paths.config_official_bak, clean);
        }
    }
    Ok(())
}

/// Restore official configuration and auth if backups exist.
pub fn restore_official(paths: &CodexPaths) -> Result<(), CodexAdapterError> {
    if paths.auth_official_bak.exists() {
        let _ = fs::copy(&paths.auth_official_bak, &paths.auth);
    }
    if paths.config_official_bak.exists() {
        let _ = fs::copy(&paths.config_official_bak, &paths.config);
    } else if paths.config.exists() {
        let config_text = fs::read_to_string(&paths.config).unwrap_or_default();
        if config_text.contains("model_provider = \"custom\"") {
            let _ = fs::write(&paths.config, "");
        }
    }
    if paths.catalog.exists() {
        let _ = fs::remove_file(&paths.catalog);
    }
    Ok(())
}

/// Official without stored login material must restore or not clobber ChatGPT OAuth in auth.json.
/// Third-party always backs up official first, then writes both files atomically.
pub fn write_live_for_provider(
    paths: &CodexPaths,
    settings: &CodexSettings,
) -> Result<(), CodexAdapterError> {
    write_live_for_provider_with_options(paths, settings, CodexWriteOptions::default())
}

pub fn write_live_for_provider_with_options(
    paths: &CodexPaths,
    settings: &CodexSettings,
    options: CodexWriteOptions,
) -> Result<(), CodexAdapterError> {
    fs::create_dir_all(&paths.home)?;

    match settings.kind {
        CodexKind::Official => {
            if paths.catalog.exists() {
                let _ = fs::remove_file(&paths.catalog);
            }
            // 官方配置写入优先级: 卡片显式配置 > config.toml.official.bak
            // (上次离开官方时的实态) > live 现状(无凭据的官方卡跟随 live，
            // 不清空用户配置)。统一路由注入作用在最终落盘的配置文本上。
            let stored_config_empty = settings.config_toml.trim().is_empty();
            let base_config = if !stored_config_empty {
                settings.config_toml.clone()
            } else if paths.config_official_bak.exists() {
                fs::read_to_string(&paths.config_official_bak).unwrap_or_default()
            } else {
                read_live(paths)?.config_toml
            };
            let config_toml = if options.unify_session_history {
                inject_unified_session_bucket(&base_config)
            } else {
                base_config
            };
            if paths.auth_official_bak.exists() && !has_login_material(&settings.auth) {
                let _ = fs::copy(&paths.auth_official_bak, &paths.auth);
            } else if has_login_material(&settings.auth) {
                write_json_atomic(&paths.auth, &settings.auth)?;
            }
            write_text_atomic(&paths.config, &config_toml)?;
            Ok(())
        }
        CodexKind::ResponsesThirdParty => {
            // Check and backup official config before overwriting with third party
            let _ = backup_official_if_needed(paths);

            // Write or remove router-switch-model-catalog.json
            if let Some(catalog_json) = generate_catalog_json(&settings.model_mappings) {
                write_text_atomic(&paths.catalog, &catalog_json)?;
            } else if paths.catalog.exists() {
                let _ = fs::remove_file(&paths.catalog);
            }

            if options.preserve_official_auth {
                // 保留官方登录: auth.json 不写入第三方密钥，官方 ChatGPT
                // 登录从备份恢复(或保留 live 现有登录)；请求认证改由
                // config.toml 中 custom 表的 experimental_bearer_token 承载。
                if paths.auth_official_bak.exists() {
                    let _ = fs::copy(&paths.auth_official_bak, &paths.auth);
                } else {
                    let live_auth = read_live(paths)?.auth;
                    if !has_login_material(&live_auth) {
                        write_json_atomic(&paths.auth, &settings.auth)?;
                    }
                }
                let api_key = extract_codex_api_key(&settings.auth)
                    .or_else(|| {
                        let live = read_live(paths).ok()?;
                        extract_codex_api_key(&live.auth)
                    })
                    .unwrap_or_default();
                let config_toml = inject_third_party_bearer_token(&settings.config_toml, &api_key);
                write_text_atomic(&paths.config, &config_toml)
            } else {
                write_codex_live_atomic(paths, &settings.auth, &settings.config_toml)
            }
        }
    }
}

/// `model_providers.custom` 表与统一会话注入产物完全一致(4 个字段)。
fn custom_table_matches_unified(config: &toml::Value) -> bool {
    let Some(custom) = config
        .get("model_providers")
        .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
    else {
        return false;
    };
    custom.get("name").and_then(|v| v.as_str()) == Some("OpenAI")
        && custom.get("requires_openai_auth").and_then(|v| v.as_bool()) == Some(true)
        && custom.get("supports_websockets").and_then(|v| v.as_bool()) == Some(true)
        && custom.get("wire_api").and_then(|v| v.as_str()) == Some("responses")
}

/// 判断统一会话注入是否可以修改该配置。
/// 两种情况拒绝注入: 已有显式顶层 `model_provider`；或已有形态与注入产物
/// 不同的 `[model_providers.custom]` 表(可能带第三方 base_url/token，
/// 激活它会把 ChatGPT OAuth 流量路由到错误后端)。
fn unified_injection_allowed(config_text: &str) -> bool {
    match toml::from_str::<toml::Value>(config_text) {
        Ok(config) => {
            if config.get("model_provider").is_some() {
                return false;
            }
            match config
                .get("model_providers")
                .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
            {
                Some(_) => custom_table_matches_unified(&config),
                None => true,
            }
        }
        // 无法解析的配置保守起见不注入
        Err(_) => false,
    }
}

const UNIFIED_PROVIDER_TABLE: &str = "[model_providers.custom]\n\
     name = \"OpenAI\"\n\
     requires_openai_auth = true\n\
     supports_websockets = true\n\
     wire_api = \"responses\"\n";

/// 统一会话路由注入块的精确文本(追加在配置末尾)。
fn unified_injected_block() -> String {
    format!("\nmodel_provider = \"{UNIFIED_PROVIDER_ID}\"\n\n{UNIFIED_PROVIDER_TABLE}")
}

/// 统一 Codex 会话历史: 把官方配置改写为以共享 custom 供应商标识运行
/// (认证仍走 auth.json 的 ChatGPT 登录)。幂等；已注入时原样返回。
pub fn inject_unified_session_bucket(config_text: &str) -> String {
    if !unified_injection_allowed(config_text) {
        return config_text.to_string();
    }
    let mut out = config_text.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&unified_injected_block());
    out
}

/// `inject_unified_session_bucket` 的反向操作: 仅当 `model_provider` 与
/// `[model_providers.custom]` 表形态与注入产物完全一致时剥离，保证切换
/// 回填不会把统一路由带进数据库的存储配置。第三方模板与用户自定义的
/// custom 表(带 base_url 等差异字段)原样保留。
pub fn strip_unified_session_bucket(config_text: &str) -> String {
    let Ok(config) = toml::from_str::<toml::Value>(config_text) else {
        return config_text.to_string();
    };
    if config.get("model_provider").and_then(|v| v.as_str()) != Some(UNIFIED_PROVIDER_ID) {
        return config_text.to_string();
    }
    // 形态校验: custom 表必须恰好是注入的 4 个字段(无 base_url 等)
    let matches_unified = config
        .get("model_providers")
        .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
        .and_then(|v| v.as_table())
        .is_some_and(|custom| custom.len() == 4 && custom_table_matches_unified(&config));
    if !matches_unified {
        return config_text.to_string();
    }

    // 注入块固定追加在配置末尾，优先按精确后缀剥离；若被其他内容追加打断，
    // 退化为逐行剥离(仅移除 model_provider 行与注入的 4 个键及空表头)。
    let block = unified_injected_block();
    let trimmed_end = config_text.trim_end_matches('\n');
    if let Some(stripped) = trimmed_end.strip_suffix(block.trim_end_matches('\n')) {
        return ensure_trailing_newline(stripped.trim_end_matches('\n'));
    }

    let injected_keys = [
        "name = \"OpenAI\"",
        "requires_openai_auth = true",
        "supports_websockets = true",
        "wire_api = \"responses\"",
    ];
    let mut out_lines: Vec<&str> = Vec::new();
    let mut in_custom_table = false;
    let mut custom_table_kept_keys = false;
    for line in config_text.lines() {
        let trimmed = line.trim();
        if trimmed == "[model_providers.custom]" {
            in_custom_table = true;
            custom_table_kept_keys = false;
            continue;
        }
        if in_custom_table && trimmed.starts_with('[') && trimmed.ends_with(']') {
            // 下一张表: 表头按是否还有保留键决定去留
            if custom_table_kept_keys {
                out_lines.push("[model_providers.custom]");
            }
            in_custom_table = false;
        } else if in_custom_table {
            if trimmed.is_empty() || injected_keys.contains(&trimmed) {
                continue;
            }
            custom_table_kept_keys = true;
        }
        if trimmed == "model_provider = \"custom\"" {
            continue;
        }
        out_lines.push(line);
    }
    if in_custom_table && custom_table_kept_keys {
        out_lines.push("[model_providers.custom]");
    }
    ensure_trailing_newline(out_lines.join("\n").trim_end_matches('\n'))
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("{text}\n")
    }
}

/// 把第三方 API 密钥写进激活 custom 供应商表的 `experimental_bearer_token`，
/// 用于保留官方登录时的第三方请求认证。已有该键时原样返回。
pub fn inject_third_party_bearer_token(config_text: &str, api_key: &str) -> String {
    if api_key.trim().is_empty() {
        return config_text.to_string();
    }
    let Ok(config) = toml::from_str::<toml::Value>(config_text) else {
        return config_text.to_string();
    };
    let has_custom_table = config
        .get("model_providers")
        .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
        .is_some();
    if !has_custom_table {
        return config_text.to_string();
    }
    if config
        .get("model_providers")
        .and_then(|v| v.get(UNIFIED_PROVIDER_ID))
        .map(|custom| custom.get("experimental_bearer_token").is_some())
        .unwrap_or(false)
    {
        return config_text.to_string();
    }

    let mut out = String::with_capacity(config_text.len() + api_key.len() + 64);
    let mut inserted = false;
    for line in config_text.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted && line.trim() == "[model_providers.custom]" {
            out.push_str(&format!(
                "experimental_bearer_token = {}\n",
                serde_json::to_string(api_key).unwrap_or_else(|_| format!("\"{api_key}\""))
            ));
            inserted = true;
        }
    }
    if !inserted {
        return config_text.to_string();
    }
    out
}

pub fn write_codex_live_atomic(
    paths: &CodexPaths,
    auth: &Value,
    config_toml: &str,
) -> Result<(), CodexAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let previous_auth = read_optional(&paths.auth)?;
    write_json_atomic(&paths.auth, auth)?;
    if let Err(err) = write_text_atomic(&paths.config, config_toml) {
        restore_previous(&paths.auth, previous_auth.as_deref())?;
        return Err(err);
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), CodexAdapterError> {
    let mut body = serde_json::to_string_pretty(value)?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    write_text_atomic(path, &body)
}

fn write_text_atomic(path: &Path, contents: &str) -> Result<(), CodexAdapterError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    fs::write(&tmp, contents)?;
    replace_file(&tmp, path)?;
    Ok(())
}

fn replace_file(tmp: &Path, dest: &Path) -> io::Result<()> {
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(err) if dest.exists() => {
            fs::remove_file(dest)?;
            fs::rename(tmp, dest)
        }
        Err(err) => {
            let _ = fs::remove_file(tmp);
            Err(err)
        }
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| {
            let mut name = name.to_os_string();
            name.push(".tmp");
            name
        })
        .unwrap_or_else(|| "file.tmp".into());
    path.with_file_name(name)
}

fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn restore_previous(path: &Path, previous: Option<&[u8]>) -> io::Result<()> {
    match previous {
        Some(bytes) => {
            let tmp = tmp_path(path);
            fs::write(&tmp, bytes)?;
            replace_file(&tmp, path)
        }
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{generate_third_party_auth, generate_third_party_config, official_codex_settings};
    use serde_json::json;

    fn temp_paths() -> (tempfile::TempDir, CodexPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = CodexPaths::from_home(dir.path().join("codex"));
        (dir, paths)
    }

    fn third_party() -> CodexSettings {
        CodexSettings {
            kind: CodexKind::ResponsesThirdParty,
            auth: generate_third_party_auth("sk-live"),
            config_toml: generate_third_party_config(
                "PackyCode",
                "https://www.packyapi.ai/v1",
                "gpt-5.6-sol",
            ),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        }
    }

    #[test]
    fn third_party_writes_auth_and_config() {
        let (_dir, paths) = temp_paths();
        write_live_for_provider(&paths, &third_party()).unwrap();
        let live = read_live(&paths).unwrap();
        assert_eq!(live.auth["OPENAI_API_KEY"], "sk-live");
        assert!(live.config_toml.contains("wire_api = \"responses\""));
        assert!(live.config_toml.contains("https://www.packyapi.ai/v1"));
    }

    #[test]
    fn official_backup_and_restore_cycle() {
        let (_dir, paths) = temp_paths();
        fs::create_dir_all(&paths.home).unwrap();
        fs::write(
            &paths.auth,
            r#"{"tokens":{"access_token":"chatgpt-oauth"}}"#,
        )
        .unwrap();
        fs::write(&paths.config, "default_model = \"gpt-5.6\"\n").unwrap();

        // Switch to third party -> should backup official
        write_live_for_provider(&paths, &third_party()).unwrap();
        assert!(paths.auth_official_bak.exists());
        assert!(paths.config_official_bak.exists());
        let live_tp = read_live(&paths).unwrap();
        assert_eq!(live_tp.auth["OPENAI_API_KEY"], "sk-live");

        // Switch back to official -> should restore official backup
        write_live_for_provider(&paths, &official_codex_settings()).unwrap();
        let live_off = read_live(&paths).unwrap();
        assert_eq!(live_off.auth["tokens"]["access_token"], "chatgpt-oauth");
        assert_eq!(live_off.config_toml, "default_model = \"gpt-5.6\"\n");
    }

    #[test]
    fn official_with_stored_key_overwrites_auth() {
        let (_dir, paths) = temp_paths();
        let mut settings = official_codex_settings();
        settings.auth = json!({"OPENAI_API_KEY": "sk-official"});
        write_live_for_provider(&paths, &settings).unwrap();
        let live = read_live(&paths).unwrap();
        assert_eq!(live.auth["OPENAI_API_KEY"], "sk-official");
    }

    fn official_live_files(paths: &CodexPaths) {
        fs::create_dir_all(&paths.home).unwrap();
        fs::write(
            &paths.auth,
            r#"{"tokens":{"access_token":"chatgpt-oauth"}}"#,
        )
        .unwrap();
        fs::write(&paths.config, "default_model = \"gpt-5.6\"\n").unwrap();
    }

    #[test]
    fn unify_injects_shared_custom_bucket_for_official() {
        let (_dir, paths) = temp_paths();
        official_live_files(&paths);
        let mut official = official_codex_settings();
        official.config_toml = "default_model = \"gpt-5.6\"\n".into();
        let options = CodexWriteOptions {
            unify_session_history: true,
            ..Default::default()
        };
        write_live_for_provider_with_options(&paths, &official, options).unwrap();
        let config = fs::read_to_string(&paths.config).unwrap();
        assert!(config.contains("model_provider = \"custom\""));
        assert!(config.contains("[model_providers.custom]"));
        assert!(config.contains("requires_openai_auth = true"));
        assert!(config.contains("supports_websockets = true"));
        // 认证仍走官方登录
        let live = read_live(&paths).unwrap();
        assert_eq!(live.auth["tokens"]["access_token"], "chatgpt-oauth");

        // 幂等: 重复注入不产生重复路由
        let injected = inject_unified_session_bucket(&config);
        assert_eq!(inject_unified_session_bucket(&injected), injected);

        // 剥离后还原原文
        assert_eq!(
            strip_unified_session_bucket(&config),
            "default_model = \"gpt-5.6\"\n"
        );
    }

    #[test]
    fn unify_injection_skips_explicit_provider_or_conflicting_custom_table() {
        let explicit = "model_provider = \"openai\"\n".to_string();
        assert_eq!(inject_unified_session_bucket(&explicit), explicit);

        let conflict = "[model_providers.custom]\nname = \"PackyCode\"\nbase_url = \"https://api.example.com/v1\"\nwire_api = \"responses\"\n".to_string();
        assert_eq!(inject_unified_session_bucket(&conflict), conflict);
        // 第三方模板与用户自定义 custom 表不被剥离
        assert_eq!(strip_unified_session_bucket(&conflict), conflict);
    }

    #[test]
    fn preserve_official_auth_keeps_login_and_moves_key_to_bearer_token() {
        let (_dir, paths) = temp_paths();
        official_live_files(&paths);
        let options = CodexWriteOptions {
            preserve_official_auth: true,
            ..Default::default()
        };
        write_live_for_provider_with_options(&paths, &third_party(), options).unwrap();

        // auth.json 保留官方 ChatGPT 登录，而不是第三方密钥
        let live = read_live(&paths).unwrap();
        assert_eq!(live.auth["tokens"]["access_token"], "chatgpt-oauth");
        assert!(live.auth.get("OPENAI_API_KEY").is_none());
        // 第三方密钥经 config.toml bearer token 承载
        assert!(live
            .config_toml
            .contains("experimental_bearer_token = \"sk-live\""));
        // 官方备份仍然产出
        assert!(paths.auth_official_bak.exists());
    }

    #[test]
    fn bearer_token_injection_is_idempotent_and_requires_custom_table() {
        let config = third_party().config_toml;
        let injected = inject_third_party_bearer_token(&config, "sk-live");
        assert_eq!(
            inject_third_party_bearer_token(&injected, "sk-live"),
            injected
        );
        // 无 custom 表的配置不注入
        let plain = "model = \"gpt-5.6\"\n".to_string();
        assert_eq!(inject_third_party_bearer_token(&plain, "sk-live"), plain);
    }
}
