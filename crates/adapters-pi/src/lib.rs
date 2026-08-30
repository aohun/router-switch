//! Read/write ~/.pi/agent live files (models.json and settings.json). Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{generate_pi_models_json, PiKind, PiSettings};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PiAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 Pi 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("Pi 配置文件不是合法 JSON: {0}")]
    ConfigJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiPaths {
    pub home: PathBuf,
    pub models: PathBuf,
    pub models_official_bak: PathBuf,
    pub settings: PathBuf,
    pub settings_official_bak: PathBuf,
}

impl PiPaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            models: home.join("models.json"),
            models_official_bak: home.join("models.json.official.bak"),
            settings: home.join("settings.json"),
            settings_official_bak: home.join("settings.json.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivePi {
    pub models: Value,
    pub settings: Value,
}

pub fn resolve_pi_paths(override_home: Option<&Path>) -> Result<PiPaths, PiAdapterError> {
    if let Some(home) = override_home {
        return Ok(PiPaths::from_home(home));
    }
    if let Ok(home) = std::env::var("PI_CODING_AGENT_DIR") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PiPaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(PiAdapterError::HomeDir)?;
    Ok(PiPaths::from_home(home.join(".pi").join("agent")))
}

pub fn read_live(paths: &PiPaths) -> Result<LivePi, PiAdapterError> {
    let models = match fs::read_to_string(&paths.models) {
        Ok(text) if text.trim().is_empty() => json!({}),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => json!({}),
        Err(err) => return Err(err.into()),
    };

    let settings = match fs::read_to_string(&paths.settings) {
        Ok(text) if text.trim().is_empty() => json!({}),
        Ok(text) => serde_json::from_str(&text)?,
        Err(err) if err.kind() == ErrorKind::NotFound => json!({}),
        Err(err) => return Err(err.into()),
    };

    Ok(LivePi { models, settings })
}

/// Backup official configs if currently in official state or empty.
pub fn backup_official_if_needed(paths: &PiPaths) -> Result<(), PiAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;

    let is_official = match live.models.get("providers") {
        Some(Value::Object(providers)) => providers.is_empty(),
        _ => true,
    };

    if is_official {
        if paths.models.exists() {
            let _ = fs::copy(&paths.models, &paths.models_official_bak);
        }
        if paths.settings.exists() {
            let _ = fs::copy(&paths.settings, &paths.settings_official_bak);
        }
    }
    Ok(())
}

/// Restore official configuration if backup exists, or clear custom provider definitions.
pub fn restore_official(paths: &PiPaths) -> Result<(), PiAdapterError> {
    let mut restored = false;
    if paths.models_official_bak.exists() {
        let _ = fs::copy(&paths.models_official_bak, &paths.models);
        restored = true;
    }
    if paths.settings_official_bak.exists() {
        let _ = fs::copy(&paths.settings_official_bak, &paths.settings);
        restored = true;
    }

    if !restored {
        if paths.models.exists() {
            let mut live = read_live(paths)?;
            if let Some(map) = live.models.as_object_mut() {
                map.remove("providers");
            }
            atomic_write_json(&paths.models, &live.models)?;
        }
        if paths.settings.exists() {
            let mut live = read_live(paths)?;
            if let Some(map) = live.settings.as_object_mut() {
                map.remove("defaultProvider");
                map.remove("defaultModel");
            }
            atomic_write_json(&paths.settings, &live.settings)?;
        }
    }
    Ok(())
}

/// Write live models.json and settings.json for provider.
pub fn write_live_for_provider(
    paths: &PiPaths,
    provider_key: &str,
    settings: &PiSettings,
) -> Result<(), PiAdapterError> {
    match settings.kind {
        PiKind::Official => {
            restore_official(paths)?;
        }
        PiKind::ThirdParty => {
            backup_official_if_needed(paths)?;
            fs::create_dir_all(&paths.home)?;

            let models_doc = generate_pi_models_json(settings, provider_key);
            atomic_write_json(&paths.models, &models_doc)?;

            let mut live = read_live(paths)?;
            if !live.settings.is_object() {
                live.settings = json!({});
            }
            if let Some(map) = live.settings.as_object_mut() {
                map.insert("defaultProvider".to_string(), json!(provider_key));
                map.insert("defaultModel".to_string(), json!(settings.model));
            }
            atomic_write_json(&paths.settings, &live.settings)?;
        }
    }
    Ok(())
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), PiAdapterError> {
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
    use domain::{parse_pi_form, PiForm, PiModelMapping};
    use tempfile::TempDir;

    #[test]
    fn pi_third_party_and_restore_cycle() {
        let temp = TempDir::new().unwrap();
        let paths = PiPaths::from_home(temp.path());

        // Create official files first
        let official_settings = json!({
            "theme": "dark",
            "packages": ["npm:pi-web-access"]
        });
        atomic_write_json(&paths.settings, &official_settings).unwrap();

        // Third-party write
        let form = PiForm {
            name: "S2A Grok".into(),
            website_url: "https://s2a.ii.sb".into(),
            kind: PiKind::ThirdParty,
            api_type: "openai-completions".into(),
            api_key: "sk-s2a-key".into(),
            base_url: "https://s2a.ii.sb/v1".into(),
            model: "grok-4.6".into(),
            request_protocol: String::new(),
            model_mappings: vec![PiModelMapping {
                model_id: "claude-3-7-sonnet".into(),
                display_name: "Claude Sonnet".into(),
                context_window: Some(200000),
            }],
        };
        let settings = parse_pi_form(form).unwrap();
        write_live_for_provider(&paths, "s2a", &settings).unwrap();

        assert!(paths.settings_official_bak.exists());
        let live = read_live(&paths).unwrap();
        assert_eq!(live.settings["defaultProvider"], "s2a");
        assert_eq!(live.settings["defaultModel"], "grok-4.6");
        assert_eq!(live.settings["theme"], "dark");
        assert_eq!(
            live.models["providers"]["s2a"]["baseUrl"],
            "https://s2a.ii.sb/v1"
        );

        // Restore official
        restore_official(&paths).unwrap();
        let restored = read_live(&paths).unwrap();
        assert_eq!(restored.settings["theme"], "dark");
        assert!(restored.settings.get("defaultProvider").is_none());
    }
}
