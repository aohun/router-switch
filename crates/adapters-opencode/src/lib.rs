//! Read/write ~/.config/opencode live files (opencode.json). Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{generate_opencode_provider_json, OpenCodeKind, OpenCodeSettings};
use serde_json::{json, Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenCodeAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 OpenCode 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("opencode.json 不是合法 JSON: {0}")]
    ConfigJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodePaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub config_official_bak: PathBuf,
}

impl OpenCodePaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            config: home.join("opencode.json"),
            config_official_bak: home.join("opencode.json.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveOpenCode {
    pub config: Value,
}

pub fn resolve_opencode_paths(
    override_home: Option<&Path>,
) -> Result<OpenCodePaths, OpenCodeAdapterError> {
    if let Some(home) = override_home {
        return Ok(OpenCodePaths::from_home(home));
    }
    if let Ok(home) = std::env::var("OPENCODE_DIR") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(OpenCodePaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(OpenCodeAdapterError::HomeDir)?;
    Ok(OpenCodePaths::from_home(
        home.join(".config").join("opencode"),
    ))
}

pub fn read_live(paths: &OpenCodePaths) -> Result<LiveOpenCode, OpenCodeAdapterError> {
    let config = match fs::read_to_string(&paths.config) {
        Ok(text) if text.trim().is_empty() => json!({
            "$schema": "https://opencode.ai/config.json"
        }),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => json!({
            "$schema": "https://opencode.ai/config.json"
        }),
        Err(err) => return Err(err.into()),
    };
    Ok(LiveOpenCode { config })
}

/// Backup official opencode.json if current config doesn't have custom providers or is clean.
pub fn backup_official_if_needed(paths: &OpenCodePaths) -> Result<(), OpenCodeAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official = match live.config.get("provider") {
        Some(Value::Object(providers)) => providers.is_empty(),
        _ => true,
    };

    if is_official && paths.config.exists() {
        let _ = fs::copy(&paths.config, &paths.config_official_bak);
    }
    Ok(())
}

/// Restore official configuration if backup exists, or clear provider object.
pub fn restore_official(paths: &OpenCodePaths) -> Result<(), OpenCodeAdapterError> {
    if paths.config_official_bak.exists() {
        let _ = fs::copy(&paths.config_official_bak, &paths.config);
        return Ok(());
    }

    if paths.config.exists() {
        let mut live = read_live(paths)?;
        if let Some(map) = live.config.as_object_mut() {
            map.remove("provider");
        }
        atomic_write_json(&paths.config, &live.config)?;
    }
    Ok(())
}

/// Write live opencode.json for provider.
pub fn write_live_for_provider(
    paths: &OpenCodePaths,
    provider_key: &str,
    provider_name: &str,
    settings: &OpenCodeSettings,
) -> Result<(), OpenCodeAdapterError> {
    match settings.kind {
        OpenCodeKind::Official => {
            restore_official(paths)?;
        }
        OpenCodeKind::ThirdParty => {
            backup_official_if_needed(paths)?;
            fs::create_dir_all(&paths.home)?;

            let mut live = read_live(paths)?;
            if !live.config.is_object() {
                live.config = json!({
                    "$schema": "https://opencode.ai/config.json"
                });
            }

            let provider_val = generate_opencode_provider_json(settings, provider_name);
            let mut providers_map = Map::new();
            providers_map.insert(provider_key.to_string(), provider_val);

            if let Some(map) = live.config.as_object_mut() {
                map.insert("provider".to_string(), Value::Object(providers_map));
            }

            atomic_write_json(&paths.config, &live.config)?;
        }
    }
    Ok(())
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), OpenCodeAdapterError> {
    let formatted = serde_json::to_string_pretty(value)? + "\n";
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, formatted.as_bytes())?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{parse_opencode_form, OpenCodeForm, OpenCodeModelMapping};
    use tempfile::TempDir;

    #[test]
    fn opencode_third_party_and_restore_cycle() {
        let temp = TempDir::new().unwrap();
        let paths = OpenCodePaths::from_home(temp.path());

        // Create official file first
        let official_json = json!({
            "$schema": "https://opencode.ai/config.json",
            "plugin": ["some-plugin"]
        });
        atomic_write_json(&paths.config, &official_json).unwrap();

        // Third-party write
        let form = OpenCodeForm {
            name: "DeepSeek".into(),
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
        write_live_for_provider(&paths, "deepseek", "DeepSeek", &settings).unwrap();

        assert!(paths.config_official_bak.exists());
        let live = read_live(&paths).unwrap();
        assert_eq!(
            live.config["provider"]["deepseek"]["options"]["baseURL"],
            "https://api.deepseek.com/v1"
        );
        assert_eq!(live.config["plugin"][0], "some-plugin");

        // Restore official
        restore_official(&paths).unwrap();
        let restored = read_live(&paths).unwrap();
        assert!(restored.config.get("provider").is_none());
        assert_eq!(restored.config["plugin"][0], "some-plugin");
    }
}
