//! 提示词（Prompt）管理业务服务
//!
//! 提供提示词的 CRUD、单一激活、智能回填（Smart Backfill）、自动备份及跨应用 Live 文件同步。

use std::fs;
use std::path::Path;

use domain::{prompt_file_path, AppKind, Prompt};
use store::Store;

use crate::SessionError;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn write_text_file(path: &Path, content: &str) -> Result<(), SessionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            SessionError::Message(format!("无法创建目录 {}: {e}", parent.display()))
        })?;
    }
    fs::write(path, content)
        .map_err(|e| SessionError::Message(format!("无法写入提示词文件 {}: {e}", path.display())))
}

pub struct PromptService;

impl PromptService {
    /// 获取指定应用的所有提示词
    pub fn get_prompts(store: &Store, app: AppKind) -> Result<Vec<Prompt>, SessionError> {
        Ok(store.list_prompts(app)?)
    }

    /// 保存或更新提示词
    pub fn save_prompt(
        store: &Store,
        app: AppKind,
        id: &str,
        mut prompt: Prompt,
    ) -> Result<(), SessionError> {
        prompt.id = id.to_string();
        let is_enabled = prompt.enabled;
        store.upsert_prompt(app, &prompt)?;

        if let Some(target_path) = prompt_file_path(app) {
            if is_enabled {
                write_text_file(&target_path, &prompt.content)?;
            } else {
                let prompts = store.list_prompts(app)?;
                let any_enabled = prompts.iter().any(|p| p.enabled);
                if !any_enabled && target_path.exists() {
                    write_text_file(&target_path, "")?;
                }
            }
        }

        Ok(())
    }

    /// 删除指定提示词（若当前处于启用状态则禁止删除）
    pub fn delete_prompt(store: &Store, app: AppKind, id: &str) -> Result<(), SessionError> {
        let prompts = store.list_prompts(app)?;
        if let Some(target) = prompts.iter().find(|p| p.id == id) {
            if target.enabled {
                return Err(SessionError::Message(
                    "无法删除已启用的提示词，请先停用后再删除".to_string(),
                ));
            }
        }
        store.delete_prompt(app, id)?;
        Ok(())
    }

    /// 启用指定提示词（带智能回填与自动备份保护）
    pub fn enable_prompt(store: &Store, app: AppKind, id: &str) -> Result<(), SessionError> {
        let target_path = prompt_file_path(app)
            .ok_or_else(|| SessionError::Message("无法确定提示词文件路径".to_string()))?;

        // 1. 智能回填与自动备份
        if target_path.exists() {
            if let Ok(live_content) = fs::read_to_string(&target_path) {
                if !live_content.trim().is_empty() {
                    let mut prompts = store.list_prompts(app)?;

                    if let Some(enabled_prompt) = prompts.iter_mut().find(|p| p.enabled) {
                        // 若当前已有激活项，且磁盘内容发生改动，回填到该激活项
                        if enabled_prompt.content.trim() != live_content.trim() {
                            enabled_prompt.content = live_content.clone();
                            enabled_prompt.updated_at = Some(now_secs());
                            store.upsert_prompt(app, enabled_prompt)?;
                        }
                    } else {
                        // 若当前没有激活项，且磁盘内容未在任何已有预设中出现，自动备份
                        let already_saved = prompts
                            .iter()
                            .any(|p| p.content.trim() == live_content.trim());
                        if !already_saved {
                            let ts = now_secs();
                            let backup_id = format!("backup-{ts}");
                            let backup_prompt = Prompt {
                                id: backup_id,
                                name: format!(
                                    "原始提示词 {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M")
                                ),
                                content: live_content,
                                description: Some("自动备份的原始提示词".to_string()),
                                enabled: false,
                                created_at: Some(ts),
                                updated_at: Some(ts),
                            };
                            store.upsert_prompt(app, &backup_prompt)?;
                        }
                    }
                }
            }
        }

        // 2. 禁用其他所有预设
        let mut prompts = store.list_prompts(app)?;
        for p in &mut prompts {
            if p.id != id && p.enabled {
                p.enabled = false;
                store.upsert_prompt(app, p)?;
            }
        }

        // 3. 启用目标提示词并写入 Live 文件
        if let Some(target) = prompts.iter_mut().find(|p| p.id == id) {
            target.enabled = true;
            target.updated_at = Some(now_secs());
            write_text_file(&target_path, &target.content)?;
            store.upsert_prompt(app, target)?;
        } else {
            return Err(SessionError::Message(format!("提示词 {id} 不存在")));
        }

        Ok(())
    }

    /// 停用指定提示词
    pub fn disable_prompt(store: &Store, app: AppKind, id: &str) -> Result<(), SessionError> {
        store.set_prompt_enabled(app, id, false)?;

        let prompts = store.list_prompts(app)?;
        let any_enabled = prompts.iter().any(|p| p.enabled);
        if !any_enabled {
            if let Some(target_path) = prompt_file_path(app) {
                if target_path.exists() {
                    write_text_file(&target_path, "")?;
                }
            }
        }
        Ok(())
    }

    /// 从现有磁盘配置文件抓取内容新建提示词预设
    pub fn import_from_file(store: &Store, app: AppKind) -> Result<Prompt, SessionError> {
        let target_path = prompt_file_path(app)
            .ok_or_else(|| SessionError::Message("无法确定提示词文件路径".to_string()))?;
        if !target_path.exists() {
            return Err(SessionError::Message("提示词配置文件不存在".to_string()));
        }

        let content = fs::read_to_string(&target_path)
            .map_err(|e| SessionError::Message(format!("读取提示词文件失败: {e}")))?;

        if content.trim().is_empty() {
            return Err(SessionError::Message("提示词配置文件为空".to_string()));
        }

        let ts = now_secs();
        let id = format!("imported-{ts}");
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "导入的提示词 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("从现有配置文件导入".to_string()),
            enabled: false,
            created_at: Some(ts),
            updated_at: Some(ts),
        };

        store.upsert_prompt(app, &prompt)?;
        Ok(prompt)
    }

    /// 获取当前磁盘 Live 文件的内容
    pub fn get_current_file_content(app: AppKind) -> Result<Option<String>, SessionError> {
        let target_path = match prompt_file_path(app) {
            Some(p) => p,
            None => return Ok(None),
        };
        if !target_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&target_path)
            .map_err(|e| SessionError::Message(format!("读取提示词文件失败: {e}")))?;
        Ok(Some(content))
    }

    /// 将数据库中的启用项全量投影到 Live 文件
    pub fn sync_to_live(store: &Store, app: AppKind) -> Result<(), SessionError> {
        let target_path = match prompt_file_path(app) {
            Some(p) => p,
            None => return Ok(()),
        };
        let prompts = store.list_prompts(app)?;
        if let Some(enabled_prompt) = prompts.iter().find(|p| p.enabled) {
            write_text_file(&target_path, &enabled_prompt.content)?;
        }
        Ok(())
    }

    /// 全量同步所有应用的提示词到磁盘
    pub fn sync_all_to_live(store: &Store) -> Result<(), SessionError> {
        for app in &[
            AppKind::Codex,
            AppKind::Claude,
            AppKind::Grok,
            AppKind::OpenCode,
            AppKind::Pi,
            AppKind::Cursor,
            AppKind::ZCode,
            AppKind::WorkBuddy,
        ] {
            let _ = Self::sync_to_live(store, *app);
        }
        Ok(())
    }

    /// 首次启动时自动导入现有文件（若数据库中为空）
    pub fn import_from_file_on_first_launch(
        store: &Store,
        app: AppKind,
    ) -> Result<usize, SessionError> {
        let existing = store.list_prompts(app)?;
        if !existing.is_empty() {
            return Ok(0);
        }

        let target_path = match prompt_file_path(app) {
            Some(p) => p,
            None => return Ok(0),
        };
        if !target_path.exists() {
            return Ok(0);
        }

        let content = match fs::read_to_string(&target_path) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        if content.trim().is_empty() {
            return Ok(0);
        }

        let ts = now_secs();
        let prompt = Prompt {
            id: format!("auto-imported-{ts}"),
            name: format!(
                "自动导入提示词 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("首次启动自动导入现有配置".to_string()),
            enabled: true,
            created_at: Some(ts),
            updated_at: Some(ts),
        };

        store.upsert_prompt(app, &prompt)?;
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path().join("app.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn test_prompt_service_crud_and_single_active() {
        let (_dir, store) = temp_store();
        let p1 = Prompt {
            id: "p1".into(),
            name: "Prompt 1".into(),
            content: "Content 1".into(),
            description: None,
            enabled: false,
            created_at: Some(10),
            updated_at: Some(10),
        };
        let p2 = Prompt {
            id: "p2".into(),
            name: "Prompt 2".into(),
            content: "Content 2".into(),
            description: None,
            enabled: false,
            created_at: Some(20),
            updated_at: Some(20),
        };

        PromptService::save_prompt(&store, AppKind::Codex, "p1", p1).unwrap();
        PromptService::save_prompt(&store, AppKind::Codex, "p2", p2).unwrap();

        let list = PromptService::get_prompts(&store, AppKind::Codex).unwrap();
        assert_eq!(list.len(), 2);

        // Enable p1
        PromptService::enable_prompt(&store, AppKind::Codex, "p1").unwrap();
        let list = PromptService::get_prompts(&store, AppKind::Codex).unwrap();
        assert!(list.iter().find(|p| p.id == "p1").unwrap().enabled);
        assert!(!list.iter().find(|p| p.id == "p2").unwrap().enabled);

        // Cannot delete enabled prompt
        assert!(PromptService::delete_prompt(&store, AppKind::Codex, "p1").is_err());

        // Enable p2, p1 should become disabled
        PromptService::enable_prompt(&store, AppKind::Codex, "p2").unwrap();
        let list = PromptService::get_prompts(&store, AppKind::Codex).unwrap();
        assert!(!list.iter().find(|p| p.id == "p1").unwrap().enabled);
        assert!(list.iter().find(|p| p.id == "p2").unwrap().enabled);

        // Now p1 can be deleted
        assert!(PromptService::delete_prompt(&store, AppKind::Codex, "p1").is_ok());
        let list = PromptService::get_prompts(&store, AppKind::Codex).unwrap();
        assert_eq!(list.len(), 1);
    }
}
