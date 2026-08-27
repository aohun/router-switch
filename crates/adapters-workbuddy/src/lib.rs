//! Read/write ~/.workbuddy/models.json live files.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use domain::WorkBuddyModelItem;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkBuddyAdapterError {
    #[error("无法解析用户主目录")]
    HomeDir,
    #[error("读写 WorkBuddy 配置失败: {0}")]
    Io(#[from] io::Error),
    #[error("models.json 不是合法 JSON: {0}")]
    ModelsJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBuddyPaths {
    pub home: PathBuf,
    pub models_json: PathBuf,
}

impl WorkBuddyPaths {
    pub fn from_home(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            models_json: home.join("models.json"),
            home,
        }
    }
}

pub fn resolve_workbuddy_paths(
    override_home: Option<&Path>,
) -> Result<WorkBuddyPaths, WorkBuddyAdapterError> {
    if let Some(home) = override_home {
        return Ok(WorkBuddyPaths::from_home(home));
    }
    if let Ok(home) = std::env::var("WORKBUDDY_DIR") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(WorkBuddyPaths::from_home(trimmed));
        }
    }
    if let Ok(home) = std::env::var("WORKBUDDY_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(WorkBuddyPaths::from_home(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or(WorkBuddyAdapterError::HomeDir)?;
    Ok(WorkBuddyPaths::from_home(home.join(".workbuddy")))
}

pub fn read_live_models(
    paths: &WorkBuddyPaths,
) -> Result<Vec<WorkBuddyModelItem>, WorkBuddyAdapterError> {
    if !paths.models_json.exists() {
        return Ok(Vec::new());
    }
    let text = match fs::read_to_string(&paths.models_json) {
        Ok(t) => t,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let models: Vec<WorkBuddyModelItem> = serde_json::from_str(trimmed)?;
    Ok(models)
}

/// Write / upsert a model item into ~/.workbuddy/models.json
pub fn write_live_for_model(
    paths: &WorkBuddyPaths,
    original_id: Option<&str>,
    item: &WorkBuddyModelItem,
) -> Result<(), WorkBuddyAdapterError> {
    fs::create_dir_all(&paths.home)?;
    let mut models = read_live_models(paths).unwrap_or_default();

    // Check if we are updating an existing entry
    let mut updated = false;
    if let Some(orig) = original_id {
        if let Some(idx) = models.iter().position(|m| m.id == orig) {
            models[idx] = item.clone();
            updated = true;
        }
    }

    if !updated {
        // Look for exact ID match
        if let Some(idx) = models.iter().position(|m| m.id == item.id) {
            models[idx] = item.clone();
        } else {
            models.push(item.clone());
        }
    }

    atomic_write_models(&paths.models_json, &models)?;
    Ok(())
}

/// Delete a model by its ID from ~/.workbuddy/models.json
pub fn delete_live_for_model(
    paths: &WorkBuddyPaths,
    model_id: &str,
) -> Result<(), WorkBuddyAdapterError> {
    if !paths.models_json.exists() {
        return Ok(());
    }
    let mut models = read_live_models(paths).unwrap_or_default();
    let original_len = models.len();
    models.retain(|m| m.id != model_id);
    if models.len() != original_len {
        atomic_write_models(&paths.models_json, &models)?;
    }
    Ok(())
}

fn atomic_write_models(
    path: &Path,
    models: &[WorkBuddyModelItem],
) -> Result<(), WorkBuddyAdapterError> {
    let formatted = serde_json::to_string_pretty(models)? + "\n";
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
    use domain::WorkBuddyReasoningConfig;
    use tempfile::TempDir;

    #[test]
    fn workbuddy_model_crud_lifecycle() {
        let temp = TempDir::new().unwrap();
        let paths = WorkBuddyPaths::from_home(temp.path());

        // 1. Initial read should be empty
        let initial = read_live_models(&paths).unwrap();
        assert!(initial.is_empty());

        // 2. Add first model
        let item1 = WorkBuddyModelItem {
            id: "gemini-3.7-flash-high".into(),
            name: "Gemini 3.7".into(),
            vendor: "Custom".into(),
            url: "https://cchost.example/v1".into(),
            api_key: "sk-mock-key-12345".into(),
            supports_tool_call: true,
            supports_images: true,
            supports_reasoning: false,
            reasoning_only: false,
            can_disable_reasoning: true,
            use_custom_protocol: false,
            max_input_tokens: Some(262144),
            max_output_tokens: Some(65536),
            reasoning: None,
        };
        write_live_for_model(&paths, None, &item1).unwrap();

        let after_add1 = read_live_models(&paths).unwrap();
        assert_eq!(after_add1.len(), 1);
        assert_eq!(after_add1[0].id, "gemini-3.7-flash-high");
        assert_eq!(after_add1[0].name, "Gemini 3.7");

        // 3. Add second model with reasoning
        let item2 = WorkBuddyModelItem {
            id: "gpt-5.6".into(),
            name: "GPT 5.6".into(),
            vendor: "Custom".into(),
            url: "https://cchost.example/v1".into(),
            api_key: "sk-mock-key-12345".into(),
            supports_tool_call: true,
            supports_images: true,
            supports_reasoning: true,
            reasoning_only: false,
            can_disable_reasoning: true,
            use_custom_protocol: false,
            max_input_tokens: Some(262144),
            max_output_tokens: Some(65536),
            reasoning: Some(WorkBuddyReasoningConfig {
                default_effort: Some("high".into()),
                supported_efforts: Some(vec!["medium".into(), "high".into()]),
                reasoning_only: false,
                can_disable_reasoning: true,
            }),
        };
        write_live_for_model(&paths, None, &item2).unwrap();

        let after_add2 = read_live_models(&paths).unwrap();
        assert_eq!(after_add2.len(), 2);
        assert_eq!(after_add2[1].id, "gpt-5.6");
        assert!(after_add2[1].supports_reasoning);

        // 4. Update item 1 (e.g. rename or change URL)
        let mut item1_updated = item1.clone();
        item1_updated.name = "Gemini 3.7 Flash Updated".into();
        write_live_for_model(&paths, Some("gemini-3.7-flash-high"), &item1_updated).unwrap();

        let after_update = read_live_models(&paths).unwrap();
        assert_eq!(after_update.len(), 2);
        assert_eq!(after_update[0].name, "Gemini 3.7 Flash Updated");

        // 5. Delete item 2
        delete_live_for_model(&paths, "gpt-5.6").unwrap();
        let after_delete = read_live_models(&paths).unwrap();
        assert_eq!(after_delete.len(), 1);
        assert_eq!(after_delete[0].id, "gemini-3.7-flash-high");

        // 6. Delete remaining item
        delete_live_for_model(&paths, "gemini-3.7-flash-high").unwrap();
        let after_delete_all = read_live_models(&paths).unwrap();
        assert!(after_delete_all.is_empty());
    }
}
