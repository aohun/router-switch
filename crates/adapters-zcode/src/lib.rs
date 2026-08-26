//! Read/write ~/.zcode/v2 live files (config.json). Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{generate_zcode_provider_json, ZCodeKind, ZCodeSettings};
use serde_json::{json, Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZCodeAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 ZCode 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("config.json 不是合法 JSON: {0}")]
    ConfigJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZCodePaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub config_official_bak: PathBuf,
}

impl ZCodePaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            config: home.join("config.json"),
            config_official_bak: home.join("config.json.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveZCode {
    pub config: Value,
}

pub fn resolve_zcode_paths(override_home: Option<&Path>) -> Result<ZCodePaths, ZCodeAdapterError> {
    if let Some(home) = override_home {
        return Ok(ZCodePaths::from_home(home));
    }
    if let Ok(home) = std::env::var("ZCODE_DIR") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(ZCodePaths::from_home(trimmed));
        }
    }
    if let Ok(home) = std::env::var("ZCODE_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(ZCodePaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(ZCodeAdapterError::HomeDir)?;
    Ok(ZCodePaths::from_home(home.join(".zcode").join("v2")))
}

pub fn read_live(paths: &ZCodePaths) -> Result<LiveZCode, ZCodeAdapterError> {
    let config = match fs::read_to_string(&paths.config) {
        Ok(text) if text.trim().is_empty() => json!({
            "provider": {}
        }),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => json!({
            "provider": {}
        }),
        Err(err) => return Err(err.into()),
    };
    Ok(LiveZCode { config })
}

/// Backup official config.json if current config doesn't have custom third-party providers.
pub fn backup_official_if_needed(paths: &ZCodePaths) -> Result<(), ZCodeAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official = match live.config.get("provider") {
        Some(Value::Object(providers)) => providers.keys().all(|k| k.starts_with("builtin:")),
        _ => true,
    };

    if is_official && paths.config.exists() {
        let _ = fs::copy(&paths.config, &paths.config_official_bak);
    }
    Ok(())
}

/// Restore official configuration if backup exists, or filter out non-builtin providers.
pub fn restore_official(paths: &ZCodePaths) -> Result<(), ZCodeAdapterError> {
    if paths.config_official_bak.exists() {
        let _ = fs::copy(&paths.config_official_bak, &paths.config);
        return Ok(());
    }

    if paths.config.exists() {
        let mut live = read_live(paths)?;
        if let Some(map) = live
            .config
            .get_mut("provider")
            .and_then(|v| v.as_object_mut())
        {
            map.retain(|k, _| k.starts_with("builtin:"));
        }
        atomic_write_json(&paths.config, &live.config)?;
    }
    Ok(())
}

/// Write live config.json for provider.
pub fn write_live_for_provider(
    paths: &ZCodePaths,
    provider_key: &str,
    provider_name: &str,
    settings: &ZCodeSettings,
) -> Result<(), ZCodeAdapterError> {
    match settings.kind {
        ZCodeKind::Official => {
            restore_official(paths)?;
        }
        ZCodeKind::ThirdParty => {
            backup_official_if_needed(paths)?;
            fs::create_dir_all(&paths.home)?;

            let mut live = read_live(paths)?;
            if !live.config.is_object() {
                live.config = json!({
                    "provider": {}
                });
            }

            if live
                .config
                .get("provider")
                .and_then(|v| v.as_object())
                .is_none()
            {
                if let Some(obj) = live.config.as_object_mut() {
                    obj.insert("provider".to_string(), Value::Object(Map::new()));
                }
            }

            let provider_val = generate_zcode_provider_json(settings, provider_name);

            if let Some(providers_map) = live
                .config
                .get_mut("provider")
                .and_then(|v| v.as_object_mut())
            {
                providers_map.insert(provider_key.to_string(), provider_val);
            }

            atomic_write_json(&paths.config, &live.config)?;
        }
    }
    Ok(())
}

/// List all custom (non-builtin) providers from config.json.
pub fn list_custom_providers_from_config(
    paths: &ZCodePaths,
) -> Result<Vec<(String, String, Option<String>, ZCodeSettings)>, ZCodeAdapterError> {
    if !paths.config.exists() {
        return Ok(Vec::new());
    }
    let live = read_live(paths)?;
    let mut result = Vec::new();
    if let Some(map) = live.config.get("provider").and_then(|v| v.as_object()) {
        for (key, val) in map {
            if key.starts_with("builtin:") {
                continue;
            }
            if let Some(obj) = val.as_object() {
                let name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(key)
                    .to_string();
                let provider_kind = obj
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("anthropic")
                    .to_string();
                let options = obj.get("options").cloned().unwrap_or_else(|| json!({}));
                let models = obj.get("models").cloned().unwrap_or_else(|| json!({}));
                let base_url = domain::extract_zcode_base_url(&options);
                let settings = ZCodeSettings {
                    kind: ZCodeKind::ThirdParty,
                    provider_kind,
                    options,
                    models,
                    model_mappings: Vec::new(),
                };
                result.push((key.clone(), name, base_url, settings));
            }
        }
    }
    Ok(result)
}

/// Delete a provider entry from live config.json.
pub fn delete_live_for_provider(
    paths: &ZCodePaths,
    provider_key: &str,
) -> Result<(), ZCodeAdapterError> {
    if !paths.config.exists() {
        return Ok(());
    }
    let mut live = read_live(paths)?;
    if let Some(providers_map) = live
        .config
        .get_mut("provider")
        .and_then(|v| v.as_object_mut())
    {
        if providers_map.remove(provider_key).is_some() {
            atomic_write_json(&paths.config, &live.config)?;
        }
    }
    Ok(())
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), ZCodeAdapterError> {
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
    use domain::{parse_zcode_form, ZCodeForm};
    use tempfile::TempDir;

    #[test]
    fn zcode_third_party_and_restore_cycle() {
        let temp = TempDir::new().unwrap();
        let paths = ZCodePaths::from_home(temp.path());

        // Create official file first with builtin providers
        let official_json = json!({
            "provider": {
                "builtin:bigmodel": {
                    "name": "Bigmodel - API Key",
                    "kind": "anthropic"
                }
            }
        });
        atomic_write_json(&paths.config, &official_json).unwrap();

        // Third-party write
        let form = ZCodeForm {
            name: "cchost".into(),
            website_url: "https://cchost.ai".into(),
            kind: ZCodeKind::ThirdParty,
            provider_kind: "anthropic".into(),
            api_key: "sk-zcode-mock-key-12345".into(),
            base_url: "https://cchost.ai".into(),
            model: "gemini-3.7-flash-high".into(),
            modality_text: true,
            modality_image: true,
            model_mappings: vec![],
        };
        let settings = parse_zcode_form(form).unwrap();
        write_live_for_provider(
            &paths,
            "afacd854-6fb2-487f-a404-956231be121",
            "cchost",
            &settings,
        )
        .unwrap();

        assert!(paths.config_official_bak.exists());
        let live = read_live(&paths).unwrap();
        assert_eq!(
            live.config["provider"]["afacd854-6fb2-487f-a404-956231be121"]["options"]["baseURL"],
            "https://cchost.ai"
        );
        assert_eq!(
            live.config["provider"]["builtin:bigmodel"]["name"],
            "Bigmodel - API Key"
        );

        // Delete third-party provider
        delete_live_for_provider(&paths, "afacd854-6fb2-487f-a404-956231be121").unwrap();
        let after_delete = read_live(&paths).unwrap();
        assert!(after_delete.config["provider"]
            .get("afacd854-6fb2-487f-a404-956231be121")
            .is_none());
        assert_eq!(
            after_delete.config["provider"]["builtin:bigmodel"]["name"],
            "Bigmodel - API Key"
        );

        // Restore official
        restore_official(&paths).unwrap();
        let restored = read_live(&paths).unwrap();
        assert_eq!(
            restored.config["provider"]["builtin:bigmodel"]["name"],
            "Bigmodel - API Key"
        );
        assert!(restored.config["provider"]
            .get("afacd854-6fb2-487f-a404-956231be121")
            .is_none());
    }
}
