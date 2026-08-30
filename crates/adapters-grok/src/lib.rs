//! Read/write ~/.grok live files (config.toml). Official vs third-party policy lives here.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::{GrokKind, GrokSettings};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrokAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 Grok 配置失败: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokPaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub config_official_bak: PathBuf,
}

impl GrokPaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            config: home.join("config.toml"),
            config_official_bak: home.join("config.toml.official.bak"),
            home,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveGrok {
    pub config_toml: String,
}

pub fn resolve_grok_paths(override_home: Option<&Path>) -> Result<GrokPaths, GrokAdapterError> {
    if let Some(home) = override_home {
        return Ok(GrokPaths::from_home(home));
    }
    if let Ok(home) = std::env::var("GROK_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(GrokPaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(GrokAdapterError::HomeDir)?;
    Ok(GrokPaths::from_home(home.join(".grok")))
}

pub fn read_live(paths: &GrokPaths) -> Result<LiveGrok, GrokAdapterError> {
    let config_toml = match fs::read_to_string(&paths.config) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };
    Ok(LiveGrok { config_toml })
}

/// Backup official config.toml if current config doesn't specify third-party backend.
pub fn backup_official_if_needed(paths: &GrokPaths) -> Result<(), GrokAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let live = read_live(paths)?;
    let is_official =
        !live.config_toml.contains("api_backend") && !live.config_toml.contains("base_url");

    if is_official && paths.config.exists() {
        let _ = fs::copy(&paths.config, &paths.config_official_bak);
    }
    Ok(())
}

/// Restore official configuration if backup exists, or clear models table.
pub fn restore_official(paths: &GrokPaths) -> Result<(), GrokAdapterError> {
    if paths.config_official_bak.exists() {
        let _ = fs::copy(&paths.config_official_bak, &paths.config);
        return Ok(());
    }

    if paths.config.exists() {
        let live = read_live(paths)?;
        if live.config_toml.contains("api_backend") || live.config_toml.contains("base_url") {
            write_text_atomic(&paths.config, "")?;
        }
    }
    Ok(())
}

pub fn write_live_for_provider(
    paths: &GrokPaths,
    settings: &GrokSettings,
) -> Result<(), GrokAdapterError> {
    fs::create_dir_all(&paths.home)?;

    match settings.kind {
        GrokKind::Official => restore_official(paths),
        GrokKind::ThirdParty => {
            let _ = backup_official_if_needed(paths);
            write_text_atomic(&paths.config, &settings.config_toml)
        }
    }
}

fn write_text_atomic(path: &Path, contents: &str) -> Result<(), GrokAdapterError> {
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
    use domain::{official_grok_settings, parse_grok_form, GrokForm};

    fn temp_paths() -> (tempfile::TempDir, GrokPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = GrokPaths::from_home(dir.path().join("grok"));
        (dir, paths)
    }

    #[test]
    fn grok_third_party_and_restore_cycle() {
        let (_dir, paths) = temp_paths();
        fs::create_dir_all(&paths.home).unwrap();
        fs::write(&paths.config, "# Official Grok config\ntheme = \"dark\"\n").unwrap();

        let form = GrokForm {
            name: "Packy Grok".into(),
            website_url: "".into(),
            kind: GrokKind::ThirdParty,
            api_key: "xai-12345".into(),
            base_url: "https://api.packy.ai/v1".into(),
            model: "grok-4.5".into(),
            request_protocol: String::new(),
            model_mappings: Vec::new(),
        };
        let settings = parse_grok_form(form).unwrap();

        // Switch to third party -> should backup official
        write_live_for_provider(&paths, &settings).unwrap();
        assert!(paths.config_official_bak.exists());

        let live = read_live(&paths).unwrap();
        assert!(live
            .config_toml
            .contains("base_url = \"https://api.packy.ai/v1\""));
        assert!(live.config_toml.contains("api_key = \"xai-12345\""));

        // Switch back to official -> should restore official backup
        write_live_for_provider(&paths, &official_grok_settings()).unwrap();
        let live_off = read_live(&paths).unwrap();
        assert_eq!(
            live_off.config_toml,
            "# Official Grok config\ntheme = \"dark\"\n"
        );
    }
}
