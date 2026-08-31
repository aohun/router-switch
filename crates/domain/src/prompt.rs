use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::AppKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

/// 返回各 AI 应用的目标提示词配置文件名
pub fn prompt_filename(app: AppKind) -> &'static str {
    match app {
        AppKind::Claude => "CLAUDE.md",
        AppKind::Codex
        | AppKind::Grok
        | AppKind::OpenCode
        | AppKind::Pi
        | AppKind::Cursor
        | AppKind::ZCode
        | AppKind::WorkBuddy => "AGENTS.md",
    }
}

/// 返回指定应用所使用的提示词目标文件系统路径
pub fn prompt_file_path(app: AppKind) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = match app {
        AppKind::Claude => home.join(".claude").join("CLAUDE.md"),
        AppKind::Codex => home.join(".codex").join("AGENTS.md"),
        AppKind::Grok => home.join(".grok").join("AGENTS.md"),
        AppKind::OpenCode => home.join(".config").join("opencode").join("AGENTS.md"),
        AppKind::Pi => home.join(".pi").join("agent").join("AGENTS.md"),
        AppKind::Cursor => home.join(".cursor").join("AGENTS.md"),
        AppKind::ZCode => home.join(".zcode").join("AGENTS.md"),
        AppKind::WorkBuddy => home.join(".workbuddy").join("AGENTS.md"),
    };
    Some(path)
}

/// 返回用于 UI 展示的友好路径（以 `~` 开头）
pub fn prompt_display_path(app: AppKind) -> String {
    if let Some(path) = prompt_file_path(app) {
        let s = path.to_string_lossy().to_string();
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy().to_string();
            if s.starts_with(&home_str) {
                return format!("~{}", &s[home_str.len()..]);
            }
        }
        s
    } else {
        prompt_filename(app).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_filenames() {
        assert_eq!(prompt_filename(AppKind::Claude), "CLAUDE.md");
        assert_eq!(prompt_filename(AppKind::Codex), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::Grok), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::OpenCode), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::Pi), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::Cursor), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::ZCode), "AGENTS.md");
        assert_eq!(prompt_filename(AppKind::WorkBuddy), "AGENTS.md");
    }

    #[test]
    fn test_prompt_file_path() {
        let path = prompt_file_path(AppKind::Claude).unwrap();
        assert!(path.ends_with(".claude/CLAUDE.md"));

        let path = prompt_file_path(AppKind::Codex).unwrap();
        assert!(path.ends_with(".codex/AGENTS.md"));
    }
}
