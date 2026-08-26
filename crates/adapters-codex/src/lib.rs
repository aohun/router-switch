//! Read/write ~/.codex live files. Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{generate_catalog_json, has_login_material, CodexKind, CodexSettings};
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

/// Backup current official config / auth files if they look like official configs.
pub fn backup_official_if_needed(paths: &CodexPaths) -> Result<(), CodexAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official = !live.config_toml.contains("model_provider = \"custom\"")
        && !live.config_toml.contains("wire_api = \"responses\"");

    if is_official {
        if paths.auth.exists() {
            let _ = fs::copy(&paths.auth, &paths.auth_official_bak);
        }
        if paths.config.exists() {
            let _ = fs::copy(&paths.config, &paths.config_official_bak);
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
    fs::create_dir_all(&paths.home)?;

    match settings.kind {
        CodexKind::Official => {
            if paths.catalog.exists() {
                let _ = fs::remove_file(&paths.catalog);
            }
            if paths.auth_official_bak.exists() && !has_login_material(&settings.auth) {
                let _ = fs::copy(&paths.auth_official_bak, &paths.auth);
            } else if has_login_material(&settings.auth) {
                write_json_atomic(&paths.auth, &settings.auth)?;
            }
            if paths.config_official_bak.exists() && settings.config_toml.trim().is_empty() {
                let _ = fs::copy(&paths.config_official_bak, &paths.config);
            } else {
                write_text_atomic(&paths.config, &settings.config_toml)?;
            }
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

            write_codex_live_atomic(paths, &settings.auth, &settings.config_toml)
        }
    }
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
}
