//! Read/write ~/.claude live files (settings.json). Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{ClaudeKind, ClaudeSettings};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClaudeAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 Claude 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("settings.json 不是合法 JSON: {0}")]
    SettingsJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePaths {
    pub home: PathBuf,
    pub settings: PathBuf,
    pub settings_official_bak: PathBuf,
}

impl ClaudePaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            settings: home.join("settings.json"),
            settings_official_bak: home.join("settings.json.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveClaude {
    pub settings: Value,
}

pub fn resolve_claude_paths(
    override_home: Option<&Path>,
) -> Result<ClaudePaths, ClaudeAdapterError> {
    if let Some(home) = override_home {
        return Ok(ClaudePaths::from_home(home));
    }
    if let Ok(home) = std::env::var("CLAUDE_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(ClaudePaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(ClaudeAdapterError::HomeDir)?;
    Ok(ClaudePaths::from_home(home.join(".claude")))
}

pub fn read_live(paths: &ClaudePaths) -> Result<LiveClaude, ClaudeAdapterError> {
    let settings = match fs::read_to_string(&paths.settings) {
        Ok(text) if text.trim().is_empty() => serde_json::json!({}),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(err.into()),
    };
    Ok(LiveClaude { settings })
}

/// Backup official settings.json if current config doesn't have custom ANTHROPIC_BASE_URL.
pub fn backup_official_if_needed(paths: &ClaudePaths) -> Result<(), ClaudeAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official = match live.settings.get("env") {
        Some(Value::Object(env_map)) => {
            !env_map.contains_key("ANTHROPIC_BASE_URL")
                && !env_map.contains_key("ANTHROPIC_AUTH_TOKEN")
        }
        _ => true,
    };

    if is_official && paths.settings.exists() {
        let _ = fs::copy(&paths.settings, &paths.settings_official_bak);
    }
    Ok(())
}

/// Restore official configuration if backup exists, or strip custom env fields.
pub fn restore_official(paths: &ClaudePaths) -> Result<(), ClaudeAdapterError> {
    if paths.settings_official_bak.exists() {
        let _ = fs::copy(&paths.settings_official_bak, &paths.settings);
        return Ok(());
    }

    if paths.settings.exists() {
        let mut live = read_live(paths)?;
        if let Some(Value::Object(ref mut env_map)) = live.settings.get_mut("env") {
            env_map.remove("ANTHROPIC_BASE_URL");
            env_map.remove("ANTHROPIC_AUTH_TOKEN");
            env_map.remove("ANTHROPIC_API_KEY");
            env_map.remove("ANTHROPIC_MODEL");
            env_map.remove("ANTHROPIC_DEFAULT_HAIKU_MODEL");
            env_map.remove("ANTHROPIC_DEFAULT_SONNET_MODEL");
            env_map.remove("ANTHROPIC_DEFAULT_OPUS_MODEL");
        }
        if let Some(Value::Object(env_map)) = live.settings.get("env") {
            if env_map.is_empty() {
                if let Value::Object(ref mut root) = live.settings {
                    root.remove("env");
                }
            }
        }
        write_json_atomic(&paths.settings, &live.settings)?;
    }
    Ok(())
}

pub fn write_live_for_provider(
    paths: &ClaudePaths,
    settings: &ClaudeSettings,
) -> Result<(), ClaudeAdapterError> {
    fs::create_dir_all(&paths.home)?;

    match settings.kind {
        ClaudeKind::Official => restore_official(paths),
        ClaudeKind::ThirdParty => {
            let _ = backup_official_if_needed(paths);

            // Merge new env keys into existing settings.json (preserving user's other settings)
            let mut live = read_live(paths)?;
            let new_env = settings
                .env
                .get("env")
                .and_then(|v| v.as_object())
                .or_else(|| settings.env.as_object());

            let mut env_obj = match live.settings.get("env") {
                Some(Value::Object(m)) => m.clone(),
                _ => serde_json::Map::new(),
            };

            // Remove existing Anthropic env keys to avoid residue
            env_obj.remove("ANTHROPIC_BASE_URL");
            env_obj.remove("ANTHROPIC_AUTH_TOKEN");
            env_obj.remove("ANTHROPIC_API_KEY");
            env_obj.remove("ANTHROPIC_MODEL");
            env_obj.remove("ANTHROPIC_DEFAULT_HAIKU_MODEL");
            env_obj.remove("ANTHROPIC_DEFAULT_SONNET_MODEL");
            env_obj.remove("ANTHROPIC_DEFAULT_OPUS_MODEL");

            if let Some(new_map) = new_env {
                for (k, v) in new_map {
                    env_obj.insert(k.clone(), v.clone());
                }
            }

            if let Value::Object(ref mut root) = live.settings {
                root.insert("env".to_string(), Value::Object(env_obj));
            } else {
                let mut root = serde_json::Map::new();
                root.insert("env".to_string(), Value::Object(env_obj));
                live.settings = Value::Object(root);
            }

            write_json_atomic(&paths.settings, &live.settings)
        }
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), ClaudeAdapterError> {
    let mut body = serde_json::to_string_pretty(value)?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    write_text_atomic(path, &body)
}

fn write_text_atomic(path: &Path, contents: &str) -> Result<(), ClaudeAdapterError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{official_claude_settings, parse_claude_form, ClaudeForm};

    fn temp_paths() -> (tempfile::TempDir, ClaudePaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = ClaudePaths::from_home(dir.path().join("claude"));
        (dir, paths)
    }

    #[test]
    fn claude_third_party_and_restore_cycle() {
        let (_dir, paths) = temp_paths();
        fs::create_dir_all(&paths.home).unwrap();
        fs::write(
            &paths.settings,
            r#"{"autoUpdaterStatus":"disabled","model":"claude-3-7-sonnet"}"#,
        )
        .unwrap();

        let form = ClaudeForm {
            name: "Packy Claude".into(),
            website_url: "".into(),
            kind: ClaudeKind::ThirdParty,
            api_key: "sk-ant-test".into(),
            base_url: "https://api.packy.ai".into(),
            model: "claude-3-7-sonnet-20250219".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let settings = parse_claude_form(form).unwrap();

        // Switch to third party -> should backup official
        write_live_for_provider(&paths, &settings).unwrap();
        assert!(paths.settings_official_bak.exists());

        let live = read_live(&paths).unwrap();
        assert_eq!(
            live.settings["env"]["ANTHROPIC_BASE_URL"],
            "https://api.packy.ai"
        );
        assert_eq!(live.settings["autoUpdaterStatus"], "disabled");

        // Switch back to official -> should restore official backup
        write_live_for_provider(&paths, &official_claude_settings()).unwrap();
        let live_off = read_live(&paths).unwrap();
        assert!(live_off.settings.get("env").is_none());
        assert_eq!(live_off.settings["autoUpdaterStatus"], "disabled");
    }
}
