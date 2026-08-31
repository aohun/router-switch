//! Skills 中心库(hub): 托管副本、仓库发现、skills.sh 搜索、备份与更新。
//!
//! 约定与 cc-switch 对齐: 通过「发现技能 / ZIP / 导入已有」落地的 Skill 先写入
//! 中心目录 `~/.router-switch/skills/<dir_name>`, 再同步拷贝到各应用的 skills
//! 目录; 覆盖或移除已有副本前先备份到 `~/.router-switch/skills-backup/`。
//! 来源(仓库坐标)记录在 `sources.json`, 供「检查更新」做内容哈希比对。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::parse_skill_metadata;

/// 备份保留数量(与 cc-switch 一致, 超出后淘汰最旧的)
pub const BACKUP_RETAIN_COUNT: usize = 20;
/// 归档条目数与解压后总字节上限(压缩炸弹防护)
pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub const MAX_ARCHIVE_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// 下载阶段对响应体自身的上限(Content-Length 不可信)
pub const MAX_DOWNLOAD_BYTES: u64 = 128 * 1024 * 1024;

// ===== 数据模型 =====

/// 技能仓库配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRepo {
    pub owner: String,
    pub name: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_branch() -> String {
    "main".into()
}

fn default_true() -> bool {
    true
}

impl SkillRepo {
    pub fn key(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// 已安装技能的来源(仓库坐标), 用于检查更新
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSource {
    pub owner: String,
    pub name: String,
    pub branch: String,
}

/// 仓库中可发现的技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverableSkill {
    /// 唯一标识: "owner/name:directory"
    pub key: String,
    pub name: String,
    pub description: String,
    /// 仓库内的技能目录路径(相对仓库根, `/` 分隔)
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    /// skills.sh 安装量(仓库来源时为空)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme_url: Option<String>,
}

/// 检查更新结果: 有新版本的技能
#[derive(Debug, Clone, Serialize)]
pub struct SkillUpdateInfo {
    pub dir_name: String,
    pub name: String,
    pub repo: String,
}

/// 备份条目元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBackupEntry {
    pub backup_id: String,
    /// Skill 目录名
    pub dir_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// 创建时间(unix 毫秒)
    pub created_at: i64,
    /// 备份来源操作
    pub origin: String,
    /// 被备份副本的原路径
    pub source_path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ReposFile {
    #[serde(default)]
    repos: Vec<SkillRepo>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SourcesFile {
    #[serde(default)]
    sources: BTreeMap<String, SkillSource>,
}

/// 与 cc-switch 相同的默认发现仓库
pub fn default_repos() -> Vec<SkillRepo> {
    vec![
        repo("anthropics", "skills", "main"),
        repo("ComposioHQ", "awesome-claude-skills", "master"),
        repo("cexll", "myclaude", "master"),
        repo("JimLiu", "baoyu-skills", "main"),
    ]
}

fn repo(owner: &str, name: &str, branch: &str) -> SkillRepo {
    SkillRepo {
        owner: owner.into(),
        name: name.into(),
        branch: branch.into(),
        enabled: true,
    }
}

// ===== 元数据读写 =====

pub fn load_repos(hub: &Path) -> Vec<SkillRepo> {
    read_json(&repos_path(hub))
        .map(|f: ReposFile| f.repos)
        .unwrap_or_else(default_repos)
}

pub fn save_repos(hub: &Path, repos: &[SkillRepo]) -> Result<(), String> {
    write_json(
        &repos_path(hub),
        &ReposFile {
            repos: repos.to_vec(),
        },
    )
}

pub fn load_sources(hub: &Path) -> BTreeMap<String, SkillSource> {
    read_json(&sources_path(hub))
        .map(|f: SourcesFile| f.sources)
        .unwrap_or_default()
}

pub fn save_sources(hub: &Path, sources: &BTreeMap<String, SkillSource>) -> Result<(), String> {
    write_json(
        &sources_path(hub),
        &SourcesFile {
            sources: sources.clone(),
        },
    )
}

pub fn record_source(hub: &Path, dir_name: &str, source: SkillSource) -> Result<(), String> {
    let mut sources = load_sources(hub);
    sources.insert(dir_name.to_string(), source);
    save_sources(hub, &sources)
}

fn repos_path(hub: &Path) -> PathBuf {
    hub.join("repos.json")
}

fn sources_path(hub: &Path) -> PathBuf {
    hub.join("sources.json")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| format!("写入失败: {e}"))
}

// ===== 备份 =====

/// 把一个 Skill 目录备份到备份目录, 返回条目。
///
/// 布局: `<backup_dir>/<backup_id>/backup.json` + `<backup_dir>/<backup_id>/payload/…`,
/// 元数据与技能内容分离, 恢复时不会把 backup.json 带进技能目录。
pub fn create_backup(
    skill_dir: &Path,
    backup_dir: &Path,
    origin: &str,
) -> Result<SkillBackupEntry, String> {
    let Some(dir_name) = skill_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
    else {
        return Err("无效的 Skill 目录".into());
    };
    let (name, description) = parse_skill_metadata(skill_dir);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let backup_id = format!("{now_ms}-{dir_name}");
    let target = backup_dir.join(&backup_id);
    super::copy_dir_recursive(skill_dir, &target.join("payload"))
        .map_err(|e| format!("创建备份失败: {e}"))?;
    let entry = SkillBackupEntry {
        backup_id,
        dir_name,
        name,
        description,
        created_at: now_ms,
        origin: origin.into(),
        source_path: skill_dir.to_string_lossy().to_string(),
    };
    write_json(&target.join("backup.json"), &entry)?;
    prune_backups(backup_dir);
    Ok(entry)
}

/// 列出全部备份(新→旧)
pub fn list_backups(backup_dir: &Path) -> Vec<SkillBackupEntry> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(backup_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(meta) = read_json::<SkillBackupEntry>(&path.join("backup.json")) {
            out.push(meta);
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

/// 超出保留数量时删除最旧的备份
fn prune_backups(backup_dir: &Path) {
    let backups = list_backups(backup_dir);
    for entry in backups.into_iter().skip(BACKUP_RETAIN_COUNT) {
        let _ = fs::remove_dir_all(backup_dir.join(&entry.backup_id));
    }
}

pub fn delete_backup(backup_dir: &Path, backup_id: &str) -> Result<(), String> {
    validate_id(backup_id)?;
    let path = backup_dir.join(backup_id);
    if !path.is_dir() {
        return Err("备份不存在".into());
    }
    fs::remove_dir_all(&path).map_err(|e| format!("删除备份失败: {e}"))
}

/// 恢复备份: 回写中心库, 并同步回原应用目录(原位置已有内容时先备份)
pub fn restore_backup(
    entry: &SkillBackupEntry,
    backup_dir: &Path,
    hub: &Path,
) -> Result<(), String> {
    validate_id(&entry.backup_id)?;
    let payload = backup_dir.join(&entry.backup_id).join("payload");
    if !payload.is_dir() {
        return Err("备份不存在".into());
    }
    let hub_target = hub.join(&entry.dir_name);
    if hub_target.exists() {
        create_backup(&hub_target, backup_dir, "restore-overwrite")?;
        fs::remove_dir_all(&hub_target).map_err(|e| format!("清空中心副本失败: {e}"))?;
    }
    fs::create_dir_all(hub).map_err(|e| format!("创建目录失败: {e}"))?;
    super::copy_dir_recursive(&payload, &hub_target)
        .map_err(|e| format!("恢复到中心库失败: {e}"))?;

    // 原应用目录若已无该副本, 一并恢复(source_path 是备份时记录的绝对路径)
    let app_dir = Path::new(&entry.source_path)
        .parent()
        .map(Path::to_path_buf);
    if let Some(app_dir) = app_dir {
        if app_dir.is_dir() {
            let app_target = app_dir.join(&entry.dir_name);
            if !app_target.exists() {
                super::copy_dir_recursive(&payload, &app_target)
                    .map_err(|e| format!("恢复到应用目录失败: {e}"))?;
            }
        }
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        || id.contains("..")
    {
        return Err("非法的备份标识".into());
    }
    Ok(())
}

// ===== 中心库同步 =====

/// 把中心库副本同步到某应用目录; 应用侧已有不同内容时先备份再覆盖
pub fn sync_to_app(
    hub: &Path,
    app_dir: &Path,
    dir_name: &str,
    backup_dir: &Path,
) -> Result<(), String> {
    let source = hub.join(dir_name);
    if !source.is_dir() {
        return Err(format!("中心库中不存在 Skill: {dir_name}"));
    }
    let target = app_dir.join(dir_name);
    if target.exists() {
        if dir_hash(&target).ok().as_deref() == dir_hash(&source).ok().as_deref() {
            return Ok(());
        }
        create_backup(&target, backup_dir, "sync-overwrite")?;
        fs::remove_dir_all(&target).map_err(|e| format!("清理旧副本失败: {e}"))?;
    }
    fs::create_dir_all(app_dir).map_err(|e| format!("创建目录失败: {e}"))?;
    super::copy_dir_recursive(&source, &target).map_err(|e| format!("同步 Skill 失败: {e}"))
}

/// 把应用目录里的 Skill 收编进中心库(导入已有)
pub fn import_to_hub(skill_dir: &Path, hub: &Path, backup_dir: &Path) -> Result<String, String> {
    let Some(dir_name) = skill_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
    else {
        return Err("无效的 Skill 目录".into());
    };
    let target = hub.join(&dir_name);
    if target.exists() {
        if dir_hash(&target).ok().as_deref() == dir_hash(skill_dir).ok().as_deref() {
            return Ok(dir_name);
        }
        create_backup(&target, backup_dir, "import-overwrite")?;
        fs::remove_dir_all(&target).map_err(|e| format!("清理中心旧副本失败: {e}"))?;
    }
    fs::create_dir_all(hub).map_err(|e| format!("创建目录失败: {e}"))?;
    super::copy_dir_recursive(skill_dir, &target).map_err(|e| format!("导入失败: {e}"))?;
    Ok(dir_name)
}

/// 未纳管的应用侧 Skill: 目录名在中心库中没有副本
pub fn scan_unmanaged(
    app_dirs: &[(String, PathBuf)],
    hub: &Path,
) -> Vec<(String, PathBuf, String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    let managed: Vec<String> = managed_dir_names(hub);
    for (app, dir) in app_dirs {
        for (dir_name, name, description) in super::scan_app_dir(dir) {
            if managed.contains(&dir_name) {
                continue;
            }
            out.push((
                app.clone(),
                dir.join(&dir_name),
                dir_name,
                name,
                description,
            ));
        }
    }
    out
}

/// 中心库中托管的 Skill 目录名(排除元数据文件)
pub fn managed_dir_names(hub: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(hub) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            if let Some(name) = path.file_name() {
                out.push(name.to_string_lossy().to_string());
            }
        }
    }
    out.sort();
    out
}

/// 目录内容哈希(相对路径 + 文件内容, 排序后逐一喂入)
pub fn dir_hash(dir: &Path) -> Result<String, String> {
    let mut files = BTreeMap::new();
    collect_files(dir, dir, &mut files)?;
    let mut hasher = Sha256::new();
    for (rel, abs) in files {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        let content = fs::read(&abs).map_err(|e| format!("读取失败 {}: {e}", abs.display()))?;
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(content);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(
    dir: &Path,
    base: &Path,
    out: &mut BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, base, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, path);
        }
    }
    Ok(())
}

// ===== 归档解压(共用预算防护) =====

struct Budget {
    entries: usize,
    total_bytes: u64,
}

impl Budget {
    fn charge_entry(&mut self) -> Result<(), String> {
        self.entries += 1;
        if self.entries > MAX_ARCHIVE_ENTRIES {
            return Err("归档条目数超出上限".into());
        }
        Ok(())
    }

    fn charge_bytes(&mut self, n: u64) -> Result<(), String> {
        self.total_bytes = self.total_bytes.saturating_add(n);
        if self.total_bytes > MAX_ARCHIVE_TOTAL_BYTES {
            return Err("归档解压后体积超出上限".into());
        }
        Ok(())
    }
}

/// 把 zip 归档解压到 dest(带条目数/体积/路径逃逸防护; 跳过 symlink)
pub fn extract_zip_to_dir<R: Read + std::io::Seek>(reader: R, dest: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("读取 ZIP 失败: {e}"))?;
    let mut budget = Budget {
        entries: 0,
        total_bytes: 0,
    };
    fs::create_dir_all(dest).map_err(|e| format!("创建目录失败: {e}"))?;
    let dest = dest
        .canonicalize()
        .map_err(|e| format!("解析目录失败: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取条目失败: {e}"))?;
        budget.charge_entry()?;
        if file.is_dir() {
            continue;
        }
        // symlink 与未知类型一律跳过, 防止逃逸与链接攻击
        if !file.is_file() {
            continue;
        }
        let Some(name) = file.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let target = dest.join(&name);
        let Some(parent) = target.parent() else {
            continue;
        };
        if !parent.starts_with(&dest) {
            continue;
        }
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        budget.charge_bytes(file.size())?;
        let mut out = fs::File::create(&target).map_err(|e| format!("写入失败: {e}"))?;
        std::io::copy(&mut file, &mut out).map_err(|e| format!("解压失败: {e}"))?;
        budget.charge_bytes(0)?; // 已计 file.size, 此处仅占位保持结构清晰
    }
    Ok(())
}

// ===== GitHub 仓库下载与发现 =====

/// 共享 HTTP client(连接超时 10s, 不设总超时: 下载体积由字节上限把关)
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .user_agent("router-switch-skills")
        .build()
        .expect("failed to build http client")
}

/// 校验 GitHub 仓库坐标, 防注入
pub fn validate_repo_ref(owner: &str, name: &str, branch: &str) -> Result<(), String> {
    let valid_owner = !owner.is_empty()
        && owner.len() <= 39
        && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let valid_name = !name.is_empty()
        && name.len() <= 100
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    let valid_branch = !branch.is_empty()
        && branch.len() <= 100
        && branch
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c))
        && !branch.starts_with('.');
    if !valid_owner || !valid_name || !valid_branch {
        return Err(format!("非法的仓库地址: {owner}/{name}"));
    }
    Ok(())
}

fn assert_github_archive_url(
    url: &str,
    owner: &str,
    name: &str,
    branch: &str,
) -> Result<(), String> {
    let prefix = format!("https://github.com/{owner}/{name}/archive/refs/heads/{branch}");
    if !url.starts_with(&prefix) {
        return Err("归档 URL 校验失败".into());
    }
    Ok(())
}

/// 下载仓库归档(zip)并解压到 dest, 依次尝试指定分支/main/master, 返回实际分支
pub async fn download_repo(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    branch: &str,
    dest: &Path,
) -> Result<String, String> {
    validate_repo_ref(owner, name, branch)?;
    let mut branches = Vec::new();
    if !branch.is_empty() && !branch.eq_ignore_ascii_case("HEAD") {
        branches.push(branch.to_string());
    }
    for candidate in ["main", "master"] {
        if !branches.iter().any(|b| b == candidate) {
            branches.push(candidate.into());
        }
    }
    let mut last_error = String::from("所有分支下载失败");
    for candidate in &branches {
        let url = format!("https://github.com/{owner}/{name}/archive/refs/heads/{candidate}.zip");
        if assert_github_archive_url(&url, owner, name, candidate).is_err() {
            continue;
        }
        match download_and_extract(client, &url, dest).await {
            Ok(()) => return Ok(candidate.clone()),
            Err(err) => {
                let _ = fs::remove_dir_all(dest);
                let _ = fs::create_dir_all(dest);
                last_error = err;
            }
        }
    }
    Err(last_error)
}

async fn download_and_extract(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), String> {
    // 不设总超时: 大归档在慢网络下需要较久, 体积上限由 MAX_DOWNLOAD_BYTES 把关
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status().as_u16()));
    }
    let mut body: Vec<u8> = Vec::new();
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("下载失败: {e}"))?
    {
        if body.len().saturating_add(chunk.len()) as u64 > MAX_DOWNLOAD_BYTES {
            return Err("归档体积超出下载上限".into());
        }
        body.extend_from_slice(&chunk);
    }
    extract_zip_to_dir(std::io::Cursor::new(body), dest)
}

/// 递归扫描解压后的仓库目录, 收集包含 SKILL.md 的技能
pub fn scan_repo_skills(
    root: &Path,
    owner: &str,
    name: &str,
    branch: &str,
) -> Vec<DiscoverableSkill> {
    let mut out = Vec::new();
    scan_repo_dir(root, root, owner, name, branch, &mut out);
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn scan_repo_dir(
    current: &Path,
    base: &Path,
    owner: &str,
    name: &str,
    branch: &str,
    out: &mut Vec<DiscoverableSkill>,
) {
    if current.join("SKILL.md").is_file() {
        let directory = current
            .strip_prefix(base)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.to_string());
        let directory = if directory.is_empty() {
            name.to_string()
        } else {
            directory
        };
        let (skill_name, description) = parse_skill_metadata(current);
        // 归档根目录形如 repo-main/, 计算相对路径时已剥掉
        out.push(DiscoverableSkill {
            key: format!("{owner}/{name}:{directory}"),
            name: skill_name.unwrap_or_else(|| {
                directory
                    .rsplit('/')
                    .next()
                    .unwrap_or(&directory)
                    .to_string()
            }),
            description: description.unwrap_or_default(),
            directory,
            repo_owner: owner.into(),
            repo_name: name.into(),
            repo_branch: branch.into(),
            installs: None,
            readme_url: Some(format!("https://github.com/{owner}/{name}")),
        });
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_repo_dir(&path, base, owner, name, branch, out);
        }
    }
}

/// 并发上限语义保持简单: 顺序抓取各仓库, 单仓库失败仅告警
pub async fn discover_available(
    client: &reqwest::Client,
    repos: &[SkillRepo],
) -> Vec<DiscoverableSkill> {
    let mut skills = Vec::new();
    for repo in repos.iter().filter(|r| r.enabled) {
        let Ok(temp) = tempfile::tempdir() else {
            continue;
        };
        match download_repo(client, &repo.owner, &repo.name, &repo.branch, temp.path()).await {
            Ok(resolved) => {
                skills.extend(scan_repo_skills(
                    temp.path(),
                    &repo.owner,
                    &repo.name,
                    &resolved,
                ));
            }
            Err(err) => {
                tracing::warn!("获取仓库 {} 技能失败: {err}", repo.key());
            }
        }
    }
    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    skills
}

// ===== skills.sh =====

#[derive(Debug, Deserialize)]
struct SkillsShApiResponse {
    skills: Vec<SkillsShApiSkill>,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct SkillsShApiSkill {
    id: String,
    #[serde(rename = "skillId")]
    skill_id: String,
    name: String,
    installs: u64,
    source: String,
}

#[derive(Debug, Serialize)]
pub struct SkillsShSearchResult {
    pub skills: Vec<DiscoverableSkill>,
    pub total_count: usize,
    pub query: String,
}

/// 搜索 skills.sh 索引
pub async fn search_skills_sh(
    client: &reqwest::Client,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, String> {
    let url = reqwest::Url::parse_with_params(
        "https://skills.sh/api/search",
        &[
            ("q", query),
            ("limit", &limit.to_string()),
            ("offset", &offset.to_string()),
        ],
    )
    .map_err(|e| e.to_string())?;
    let resp: SkillsShApiResponse = client
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("搜索失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("搜索失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("解析搜索结果失败: {e}"))?;
    let skills = resp
        .skills
        .into_iter()
        .filter_map(|s| {
            let (owner, name) = s.source.split_once('/')?;
            if validate_repo_ref(owner, name, "main").is_err() {
                return None;
            }
            Some(DiscoverableSkill {
                key: s.id,
                name: s.name,
                description: String::new(),
                directory: s.skill_id,
                repo_owner: owner.to_string(),
                repo_name: name.to_string(),
                repo_branch: "main".into(),
                installs: Some(s.installs),
                readme_url: Some(format!("https://github.com/{owner}/{name}")),
            })
        })
        .collect();
    Ok(SkillsShSearchResult {
        skills,
        total_count: resp.count,
        query: query.to_string(),
    })
}

// ===== 安装与更新 =====

/// 从下载好的仓库目录中解析技能源目录:
/// 先精确匹配 directory, 否则按最后一段目录名在 3 层深度内查找
fn resolve_skill_source_dir(repo_root: &Path, directory: &str) -> Option<PathBuf> {
    let direct = repo_root.join(directory);
    if direct.join("SKILL.md").is_file() {
        return Some(direct);
    }
    let target = directory.rsplit('/').next()?.to_string();
    find_dir_by_name(repo_root, &target, 0)
}

fn find_dir_by_name(dir: &Path, target: &str, depth: usize) -> Option<PathBuf> {
    if depth > 3 {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name()?.to_string_lossy() == target && path.join("SKILL.md").is_file() {
            return Some(path);
        }
        if let Some(found) = find_dir_by_name(&path, target, depth + 1) {
            return Some(found);
        }
    }
    None
}

/// 安装一个已发现的技能: 下载仓库 → 写入中心库 → 同步到全部应用目录
pub async fn install_discovered(
    client: &reqwest::Client,
    skill: &DiscoverableSkill,
    hub: &Path,
    app_dirs: &[(String, PathBuf)],
    backup_dir: &Path,
) -> Result<usize, String> {
    let temp = tempfile::tempdir().map_err(|e| format!("创建临时目录失败: {e}"))?;
    let resolved = download_repo(
        client,
        &skill.repo_owner,
        &skill.repo_name,
        &skill.repo_branch,
        temp.path(),
    )
    .await?;
    let source = resolve_skill_source_dir(temp.path(), &skill.directory)
        .ok_or_else(|| format!("仓库中未找到技能目录: {}", skill.directory))?;
    let dir_name = import_to_hub(&source, hub, backup_dir)?;
    record_source(
        hub,
        &dir_name,
        SkillSource {
            owner: skill.repo_owner.clone(),
            name: skill.repo_name.clone(),
            branch: resolved,
        },
    )?;
    let mut synced = 0usize;
    for (_, app_dir) in app_dirs {
        if sync_to_app(hub, app_dir, &dir_name, backup_dir).is_ok() {
            synced += 1;
        }
    }
    Ok(synced)
}

/// 检查全部受管技能的更新(按仓库分组下载, 内容哈希比对)
pub async fn check_updates(
    client: &reqwest::Client,
    hub: &Path,
) -> Result<Vec<SkillUpdateInfo>, String> {
    let sources = load_sources(hub);
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let mut groups: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
    for (dir_name, source) in &sources {
        groups
            .entry((
                source.owner.clone(),
                source.name.clone(),
                source.branch.clone(),
            ))
            .or_default()
            .push(dir_name.clone());
    }

    let mut updates = Vec::new();
    for ((owner, name, branch), dir_names) in &groups {
        let Ok(temp) = tempfile::tempdir() else {
            continue;
        };
        let resolved = match download_repo(client, owner, name, branch, temp.path()).await {
            Ok(resolved) => resolved,
            Err(err) => {
                tracing::warn!("检查更新下载 {owner}/{name} 失败: {err}");
                continue;
            }
        };
        let remote_skills = scan_repo_skills(temp.path(), owner, name, &resolved);
        for dir_name in dir_names {
            let Some(remote) = remote_skills
                .iter()
                .find(|s| s.directory.rsplit('/').next().eq(&Some(dir_name.as_str())))
            else {
                continue;
            };
            let Some(remote_dir) = resolve_skill_source_dir(temp.path(), &remote.directory) else {
                continue;
            };
            let Ok(remote_hash) = dir_hash(&remote_dir) else {
                continue;
            };
            let local_hash = local_managed_hash(hub, dir_name);
            if local_hash.as_deref() != Some(remote_hash.as_str()) {
                updates.push(SkillUpdateInfo {
                    dir_name: dir_name.clone(),
                    name: remote.name.clone(),
                    repo: format!("{owner}/{name}"),
                });
            }
        }
    }
    Ok(updates)
}

fn local_managed_hash(hub: &Path, dir_name: &str) -> Option<String> {
    let hub_copy = hub.join(dir_name);
    if hub_copy.is_dir() {
        return dir_hash(&hub_copy).ok();
    }
    None
}

/// 更新单个受管技能: 重新下载并替换中心库副本, 再同步到已有该副本的应用
pub async fn update_skill(
    client: &reqwest::Client,
    dir_name: &str,
    hub: &Path,
    app_dirs: &[(String, PathBuf)],
    backup_dir: &Path,
) -> Result<usize, String> {
    let source = load_sources(hub)
        .get(dir_name)
        .cloned()
        .ok_or_else(|| format!("Skill 无仓库来源: {dir_name}"))?;
    let temp = tempfile::tempdir().map_err(|e| format!("创建临时目录失败: {e}"))?;
    let resolved = download_repo(
        client,
        &source.owner,
        &source.name,
        &source.branch,
        temp.path(),
    )
    .await?;
    let remote_skills = scan_repo_skills(temp.path(), &source.owner, &source.name, &resolved);
    let remote = remote_skills
        .iter()
        .find(|s| s.directory.rsplit('/').next().eq(&Some(dir_name)))
        .ok_or_else(|| format!("仓库中未找到技能: {dir_name}"))?;
    let remote_dir = resolve_skill_source_dir(temp.path(), &remote.directory)
        .ok_or_else(|| format!("仓库中未找到技能目录: {dir_name}"))?;

    let hub_target = hub.join(dir_name);
    if hub_target.exists() {
        create_backup(&hub_target, backup_dir, "update")?;
        fs::remove_dir_all(&hub_target).map_err(|e| format!("清理中心旧副本失败: {e}"))?;
    }
    fs::create_dir_all(hub).map_err(|e| format!("创建目录失败: {e}"))?;
    super::copy_dir_recursive(&remote_dir, &hub_target)
        .map_err(|e| format!("更新中心副本失败: {e}"))?;
    record_source(
        hub,
        dir_name,
        SkillSource {
            owner: source.owner,
            name: source.name,
            branch: resolved,
        },
    )?;

    let mut synced = 0usize;
    for (_, app_dir) in app_dirs {
        if app_dir.join(dir_name).is_dir()
            && sync_to_app(hub, app_dir, dir_name, backup_dir).is_ok()
        {
            synced += 1;
        }
    }
    Ok(synced)
}

/// 从本地 ZIP 文件安装技能: 解压到临时目录 → 逐个发现 SKILL.md → 入库并同步
pub fn install_from_zip_bytes(
    bytes: Vec<u8>,
    hub: &Path,
    app_dirs: &[(String, PathBuf)],
    backup_dir: &Path,
) -> Result<Vec<String>, String> {
    let temp = tempfile::tempdir().map_err(|e| format!("创建临时目录失败: {e}"))?;
    extract_zip_to_dir(std::io::Cursor::new(bytes), temp.path())?;
    let skills = scan_repo_skills(temp.path(), "local", "zip", "main");
    if skills.is_empty() {
        return Err("ZIP 中未发现包含 SKILL.md 的技能目录".into());
    }
    let mut installed = Vec::new();
    for skill in &skills {
        let Some(source) = resolve_skill_source_dir(temp.path(), &skill.directory) else {
            continue;
        };
        let dir_name = import_to_hub(&source, hub, backup_dir)?;
        for (_, app_dir) in app_dirs {
            let _ = sync_to_app(hub, app_dir, &dir_name, backup_dir);
        }
        installed.push(dir_name);
    }
    Ok(installed)
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
    fn repos_default_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(load_repos(temp.path()).len(), default_repos().len());
        let mut repos = default_repos();
        repos.push(repo("foo", "bar", "main"));
        save_repos(temp.path(), &repos).unwrap();
        assert_eq!(load_repos(temp.path()).len(), repos.len());
    }

    #[test]
    fn backup_roundtrip_and_prune() {
        let temp = tempfile::tempdir().unwrap();
        let hub = temp.path().join("skills");
        let backup_dir = temp.path().join("backup");
        let skill = write_skill(temp.path(), "demo", "Demo", "desc");

        let entry = create_backup(&skill, &backup_dir, "test").unwrap();
        assert!(backup_dir
            .join(&entry.backup_id)
            .join("payload")
            .join("SKILL.md")
            .exists());
        assert_eq!(list_backups(&backup_dir).len(), 1);

        // 恢复: 原目录删除后回写
        fs::remove_dir_all(&skill).unwrap();
        restore_backup(&entry, &backup_dir, &hub).unwrap();
        assert!(skill.join("SKILL.md").exists());
        assert!(hub.join("demo").join("SKILL.md").exists());

        delete_backup(&backup_dir, &entry.backup_id).unwrap();
        assert!(list_backups(&backup_dir).is_empty());

        // 保留数量: 只留最新 BACKUP_RETAIN_COUNT 份
        for i in 0..(BACKUP_RETAIN_COUNT + 3) {
            fs::write(skill.join("SKILL.md"), format!("# v{i}\n")).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
            create_backup(&skill, &backup_dir, "test").unwrap();
        }
        assert_eq!(list_backups(&backup_dir).len(), BACKUP_RETAIN_COUNT);

        // 非法 id 拒绝路径穿越
        assert!(delete_backup(&backup_dir, "..").is_err());
        assert!(delete_backup(&backup_dir, "a/b").is_err());
    }

    #[test]
    fn import_and_unmanaged_flow() {
        let temp = tempfile::tempdir().unwrap();
        let hub = temp.path().join("skills");
        let backup_dir = temp.path().join("backup");
        let app_dir = temp.path().join("app-skills");
        fs::create_dir_all(&app_dir).unwrap();
        let skill = write_skill(&app_dir, "local-only", "Local", "本地技能");
        write_skill(&app_dir, "already-hub", "Hub", "已在中心库");
        let hub_copy = import_to_hub(&app_dir.join("already-hub"), &hub, &backup_dir).unwrap();
        assert_eq!(hub_copy, "already-hub");

        let unmanaged = scan_unmanaged(
            &[(super::super::APP_CLAUDE.to_string(), app_dir.clone())],
            &hub,
        );
        assert_eq!(unmanaged.len(), 1, "unmanaged: {unmanaged:?}");
        assert_eq!(unmanaged[0].2, "local-only");

        import_to_hub(&skill, &hub, &backup_dir).unwrap();
        assert!(scan_unmanaged(&[(super::super::APP_CLAUDE.into(), app_dir)], &hub).is_empty());
    }

    #[test]
    fn dir_hash_detects_changes() {
        let temp = tempfile::tempdir().unwrap();
        let dir = write_skill(temp.path(), "demo", "Demo", "desc");
        let h1 = dir_hash(&dir).unwrap();
        assert_eq!(dir_hash(&dir).unwrap(), h1);
        fs::write(dir.join("extra.txt"), "x").unwrap();
        assert_ne!(dir_hash(&dir).unwrap(), h1);
    }

    #[test]
    fn zip_extract_budget_and_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.add_directory("skills/demo/", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.start_file(
            "skills/demo/SKILL.md",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut zip, b"---\nname: ZipSkill\n---\n").unwrap();
        // 路径逃逸条目
        zip.start_file("../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let dest = temp.path().join("out");
        extract_zip_to_dir(std::io::Cursor::new(bytes), &dest).unwrap();
        assert!(dest.join("skills/demo/SKILL.md").exists());
        assert!(!temp.path().join("evil.txt").exists());
    }

    #[test]
    fn scan_repo_skills_finds_nested() {
        let temp = tempfile::tempdir().unwrap();
        let alpha_dir = temp.path().join("skills").join("alpha");
        fs::create_dir_all(&alpha_dir).unwrap();
        fs::write(
            alpha_dir.join("SKILL.md"),
            "---\nname: Alpha\ndescription: A\n---\n",
        )
        .unwrap();
        write_skill(temp.path(), "root-skill", "Root", "R");
        let skills = scan_repo_skills(temp.path(), "owner", "repo", "main");
        let dirs: Vec<String> = skills.iter().map(|s| s.directory.clone()).collect();
        assert!(dirs.contains(&"skills/alpha".to_string()), "dirs: {dirs:?}");
        assert!(dirs.contains(&"root-skill".to_string()));
        let alpha = skills
            .iter()
            .find(|s| s.directory == "skills/alpha")
            .unwrap();
        assert_eq!(alpha.name, "Alpha");
        assert_eq!(alpha.key, "owner/repo:skills/alpha");
    }

    #[test]
    fn repo_ref_validation_rejects_traversal() {
        assert!(validate_repo_ref("anthropics", "skills", "main").is_ok());
        assert!(validate_repo_ref("../etc", "skills", "main").is_err());
        assert!(validate_repo_ref("owner", "a/b", "main").is_err());
        assert!(validate_repo_ref("owner", "skills", "../main").is_err());
        assert!(validate_repo_ref("has.dot", "skills", "main").is_err());
    }

    #[test]
    fn zip_install_flow_installs_to_hub_and_apps() {
        let temp = tempfile::tempdir().unwrap();
        let hub = temp.path().join("skills");
        let backup_dir = temp.path().join("backup");
        let app_dir = temp.path().join("app-skills");

        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file(
            "packed-skill/SKILL.md",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut zip, b"---\nname: Packed\n---\n").unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let installed = install_from_zip_bytes(
            bytes,
            &hub,
            &[(super::super::APP_CODEX.to_string(), app_dir.clone())],
            &backup_dir,
        )
        .unwrap();
        assert_eq!(installed, vec!["packed-skill"]);
        assert!(hub.join("packed-skill/SKILL.md").exists());
        assert!(app_dir.join("packed-skill/SKILL.md").exists());
    }
}
