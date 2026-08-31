//! Skills 管理: 扫描各 CLI 的 skills 目录并支持跨应用安装/移除。
//!
//! 约定与 cc-switch 一致: 每个 Skill 是包含 `SKILL.md` 的目录, 位于各应用的
//! skills 目录下(Claude `~/.claude/skills`, Codex `~/.codex/skills`,
//! Grok Build `~/.grok/skills`, OpenCode `~/.config/opencode/skills`,
//! Pi `~/.pi/agent/skills`)。安装 = 从任一已有副本递归拷贝到目标应用;
//! 移除 = 删除该应用下的副本(仅当其它应用仍有副本时)。
//!
//! 中心库(hub)与仓库发现/备份/更新见 [`hub`]。

pub mod hub;

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_CLAUDE: &str = "claude";
pub const APP_CODEX: &str = "codex";
pub const APP_GROK: &str = "grok";
pub const APP_OPENCODE: &str = "opencode";
pub const APP_PI: &str = "pi";

/// 某应用下的一个已安装副本
#[derive(Debug, Clone, Serialize)]
pub struct SkillInstall {
    pub app: &'static str,
    pub path: PathBuf,
}

/// 聚合后的 Skill(按目录名跨应用归并)
#[derive(Debug, Clone, Serialize)]
pub struct SkillEntry {
    pub dir_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub installs: Vec<SkillInstall>,
}

/// 受支持应用的 skills 目录
pub fn app_skills_dir(app: &str, home: &Path) -> PathBuf {
    match app {
        APP_CLAUDE => home.join(".claude").join("skills"),
        APP_CODEX => home.join(".codex").join("skills"),
        APP_GROK => home.join(".grok").join("skills"),
        APP_OPENCODE => home.join(".config").join("opencode").join("skills"),
        APP_PI => home.join(".pi").join("agent").join("skills"),
        _ => home.join(&format!(".{app}")).join("skills"),
    }
}

/// 扫描全部应用目录, 按 Skill 目录名归并
pub fn scan_skills(app_dirs: &[(String, PathBuf)]) -> Vec<SkillEntry> {
    let mut entries: Vec<SkillEntry> = Vec::new();
    for (app, dir) in app_dirs {
        for (dir_name, name, description) in scan_app_dir(dir) {
            if let Some(entry) = entries.iter_mut().find(|e| e.dir_name == dir_name) {
                entry.installs.push(SkillInstall {
                    app: app_from_str(app),
                    path: dir.join(&dir_name),
                });
                if entry.name.is_none() {
                    entry.name = name;
                }
                if entry.description.is_none() {
                    entry.description = description;
                }
            } else {
                entries.push(SkillEntry {
                    dir_name: dir_name.clone(),
                    name,
                    description,
                    installs: vec![SkillInstall {
                        app: app_from_str(app),
                        path: dir.join(dir_name),
                    }],
                });
            }
        }
    }
    entries.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    entries
}

fn app_from_str(app: &str) -> &'static str {
    match app {
        APP_CLAUDE => APP_CLAUDE,
        APP_CODEX => APP_CODEX,
        APP_GROK => APP_GROK,
        APP_OPENCODE => APP_OPENCODE,
        APP_PI => APP_PI,
        _ => "other",
    }
}

/// 扫描单个应用的 skills 目录: (dir_name, frontmatter name, description)
pub(crate) fn scan_app_dir(dir: &Path) -> Vec<(String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("SKILL.md").exists() {
            continue;
        }
        let Some(dir_name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        let (name, description) = parse_skill_metadata(&path);
        out.push((dir_name, name, description));
    }
    out
}

/// 解析 SKILL.md frontmatter 中的 name / description(容错: 单行键值)
pub(crate) fn parse_skill_metadata(skill_dir: &Path) -> (Option<String>, Option<String>) {
    let Ok(content) = fs::read_to_string(skill_dir.join("SKILL.md")) else {
        return (None, None);
    };
    let mut name = None;
    let mut description = None;
    let mut in_frontmatter = false;
    let mut lines = content.lines();
    // frontmatter 以 --- 开始
    for line in lines.by_ref() {
        if line.trim() == "---" {
            in_frontmatter = true;
            break;
        }
        if !line.trim().is_empty() {
            // 无 frontmatter: 首个非空行可能是标题
            if let Some(title) = line.trim().strip_prefix("# ") {
                name = Some(title.trim().to_string());
            }
            break;
        }
    }
    if !in_frontmatter {
        return (name, description);
    }
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("name:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() && name.is_none() {
                name = Some(value.to_string());
            }
        } else if let Some(value) = trimmed.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() && description.is_none() {
                description = Some(value.to_string());
            }
        }
    }
    (name, description)
}

/// 把一个 Skill 目录递归拷贝到目标应用
pub fn install_skill(source: &Path, target_app_dir: &Path) -> Result<(), String> {
    let Some(dir_name) = source.file_name() else {
        return Err("无效的 Skill 源目录".into());
    };
    let target = target_app_dir.join(dir_name);
    if target.exists() {
        return Err(format!("目标已存在: {}", target.display()));
    }
    fs::create_dir_all(target_app_dir).map_err(|e| format!("创建目录失败: {e}"))?;
    copy_dir_recursive(source, &target).map_err(|e| format!("拷贝 Skill 失败: {e}"))?;
    Ok(())
}

/// 移除某应用下的 Skill 副本
pub fn remove_skill(path: &Path) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|e| format!("移除 Skill 失败: {e}"))
}

pub(crate) fn copy_dir_recursive(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let entry_target = target.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &entry_target)?;
        } else {
            fs::copy(&entry_path, &entry_target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(root: &Path, dir_name: &str, name: &str, description: &str) -> PathBuf {
        let dir = root.join(dir_name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
        )
        .unwrap();
        dir
    }

    #[test]
    fn scan_aggregates_across_apps() {
        let temp = tempfile::tempdir().unwrap();
        let claude_root = temp.path().join("claude-skills");
        fs::create_dir_all(&claude_root).unwrap();
        let claude_dir = write_skill(&claude_root, "apifox-branch", "apifox-branch", "分支协作");
        let codex_root = temp.path().join("codex");
        fs::create_dir_all(&codex_root).unwrap();
        let codex_dir = write_skill(
            &codex_root,
            "apifox-branch",
            "apifox-branch",
            "分支协作 Codex",
        );

        let app_dirs = vec![
            (APP_CLAUDE.to_string(), claude_root),
            (APP_CODEX.to_string(), codex_root),
        ];

        let entries = scan_skills(&app_dirs);
        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].dir_name, "apifox-branch");
        assert_eq!(entries[0].name.as_deref(), Some("apifox-branch"));
        assert_eq!(
            entries[0].installs.len(),
            2,
            "installs: {:?}",
            entries[0].installs
        );
        assert_eq!(entries[0].installs[0].path, claude_dir);
        assert_eq!(entries[0].installs[1].path, codex_dir);
    }

    #[test]
    fn parse_metadata_from_frontmatter_and_fallback_title() {
        let temp = tempfile::tempdir().unwrap();
        let with_fm = write_skill(temp.path(), "with-fm", "My Skill", "Does things");
        assert_eq!(
            parse_skill_metadata(&with_fm),
            (Some("My Skill".into()), Some("Does things".into()))
        );

        let no_fm = temp.path().join("no-fm");
        fs::create_dir_all(&no_fm).unwrap();
        fs::write(no_fm.join("SKILL.md"), "# Fallback Title\n\nbody\n").unwrap();
        assert_eq!(
            parse_skill_metadata(&no_fm).0.as_deref(),
            Some("Fallback Title")
        );
    }

    #[test]
    fn install_then_remove_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let source = write_skill(temp.path(), "demo-skill", "Demo", "Demo skill");
        let target_root = temp.path().join("target-app");
        install_skill(&source, &target_root).unwrap();
        let installed = target_root.join("demo-skill");
        assert!(installed.join("SKILL.md").exists());

        // 重复安装应报错
        assert!(install_skill(&source, &target_root).is_err());

        remove_skill(&installed).unwrap();
        assert!(!installed.exists());
    }

    #[test]
    fn scan_ignores_dirs_without_skill_md() {
        let temp = tempfile::tempdir().unwrap();
        let plain = temp.path().join("not-a-skill");
        fs::create_dir_all(&plain).unwrap();
        fs::write(plain.join("README.md"), "x").unwrap();
        let entries = scan_skills(&[(APP_CLAUDE.to_string(), temp.path().to_path_buf())]);
        assert!(entries.is_empty());
    }
}
