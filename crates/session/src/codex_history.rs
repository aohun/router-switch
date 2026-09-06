//! Codex 官方会话历史归桶迁移(统一会话历史)。
//!
//! 只操作本机 `~/.codex` 历史数据：jsonl 会话文件中的 `session_meta`
//! 行与 state SQLite(`state_5.sqlite`) `threads` 表的 `model_provider`
//! 字段。迁移前自动备份到 `~/.router-switch/backups/` 账本目录，
//! 关闭开关时可按账本精确还原。

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{backup::Backup, params_from_iter, Connection};
use serde_json::Value;

/// Codex 内建默认 provider id: config.toml 没有 `model_provider` 时会话归入此桶。
const OFFICIAL_PROVIDER_ID: &str = "openai";
/// 共享 custom 供应商标识(与 adapters-codex 注入产物一致)。
const UNIFIED_PROVIDER_ID: &str = "custom";
/// Codex 每线程 state 数据库文件名(版本随 Codex 升级，需要跟进)。
const CODEX_STATE_DB_FILENAME: &str = "state_5.sqlite";
/// SQLite IN 列表分块上限。
const STATE_DB_ID_CHUNK: usize = 500;

/// 迁移/还原结果。`skipped_reason` 非空表示本次未执行任何改写。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HistoryOutcome {
    pub jsonl_files: usize,
    pub state_rows: usize,
    pub skipped_reason: Option<String>,
}

impl HistoryOutcome {
    pub(crate) fn skipped(reason: &str) -> Self {
        Self {
            jsonl_files: 0,
            state_rows: 0,
            skipped_reason: Some(reason.to_string()),
        }
    }

    pub fn is_skipped(&self) -> bool {
        self.skipped_reason.is_some()
    }
}

/// 解析 state SQLite 的候选路径: codex 目录内的库 + `sqlite_home`
/// (config.toml 顶层键或 `CODEX_SQLITE_HOME` 环境变量)指向的库。
pub fn state_db_paths(codex_home: &Path, config_text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if !paths.contains(&path) {
            paths.push(path);
        }
    };
    push(codex_home.join(CODEX_STATE_DB_FILENAME));
    let sqlite_home = sqlite_home_from_config(config_text).or_else(sqlite_home_from_env);
    if let Some(home) = sqlite_home {
        push(home.join(CODEX_STATE_DB_FILENAME));
    }
    paths
}

fn sqlite_home_from_config(config_text: &str) -> Option<PathBuf> {
    let config = toml::from_str::<toml::Value>(config_text).ok()?;
    let raw = config.get("sqlite_home")?.as_str()?.trim().to_string();
    resolve_user_path(&raw)
}

fn sqlite_home_from_env() -> Option<PathBuf> {
    let raw = std::env::var("CODEX_SQLITE_HOME").ok()?;
    resolve_user_path(raw.trim())
}

fn resolve_user_path(raw: &str) -> Option<PathBuf> {
    if raw.is_empty() {
        return None;
    }
    if raw == "~" {
        return dirs::home_dir();
    }
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        return dirs::home_dir().map(|home| home.join(rest));
    }
    Some(PathBuf::from(raw))
}

/// live config.toml 是否已路由到共享 custom 桶(会话分桶只看这个实态)。
pub fn config_routes_custom(config_text: &str) -> bool {
    toml::from_str::<toml::Value>(config_text)
        .ok()
        .and_then(|config| {
            config
                .get("model_provider")
                .and_then(|v| v.as_str())
                .map(|id| id.trim() == UNIFIED_PROVIDER_ID)
        })
        .unwrap_or(false)
}

fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>, depth: u8, max_depth: u8) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files, depth + 1, max_depth);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

fn collect_sqlite_files(dir: &Path, files: &mut Vec<PathBuf>, depth: u8, max_depth: u8) {
    if depth > max_depth || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sqlite_files(&path, files, depth + 1, max_depth);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("sqlite") {
            files.push(path);
        }
    }
}

/// session_meta 行改写: payload.model_provider 在来源集合内时改为目标 id。
/// 返回 None 表示该行无需改写。
fn rewrite_meta_line(line: &str, from: &HashSet<&str>, to: &str) -> Option<String> {
    if !line.contains("\"session_meta\"") || !line.contains("\"model_provider\"") {
        return None;
    }
    let mut value: Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = value.get_mut("payload")?.as_object_mut()?;
    let current = payload.get("model_provider")?.as_str()?;
    if !from.contains(current) {
        return None;
    }
    payload.insert("model_provider".to_string(), Value::String(to.to_string()));
    serde_json::to_string(&value).ok()
}

fn rewrite_jsonl_files(
    codex_home: &Path,
    rewrite_line: impl Fn(&str) -> Option<String>,
    backup_root: &Path,
) -> Result<usize, String> {
    let mut files = Vec::new();
    collect_jsonl_files(&codex_home.join("sessions"), &mut files, 0, 8);
    collect_jsonl_files(&codex_home.join("archived_sessions"), &mut files, 0, 4);

    let mut migrated = 0;
    for path in files {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取会话文件失败 {}: {e}", path.display()))?;
        let mut rewritten = String::with_capacity(content.len());
        let mut changed = false;
        for segment in content.split_inclusive('\n') {
            let (line, newline) = segment
                .strip_suffix('\n')
                .map(|line| (line, "\n"))
                .unwrap_or((segment, ""));
            match rewrite_line(line) {
                Some(next) => {
                    rewritten.push_str(&next);
                    changed = true;
                }
                None => rewritten.push_str(line),
            }
            rewritten.push_str(newline);
        }
        if !changed {
            continue;
        }
        backup_jsonl_file(&path, codex_home, backup_root)?;
        write_atomic(&path, rewritten.as_bytes())?;
        migrated += 1;
    }
    Ok(migrated)
}

fn backup_jsonl_file(path: &Path, codex_home: &Path, backup_root: &Path) -> Result<(), String> {
    let rel = path
        .strip_prefix(codex_home)
        .map_err(|_| format!("会话文件不在 Codex 目录内: {}", path.display()))?;
    let target = backup_root.join("jsonl").join(rel);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建备份目录失败: {e}"))?;
    }
    fs::copy(path, &target).map_err(|e| format!("备份会话文件失败: {e}"))?;
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("jsonl.rs-tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(format!("替换会话文件失败 {}: {err}", path.display()))
        }
    }
}

fn open_state_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("打开 Codex state DB 失败 {}: {e}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| format!("设置 state DB busy_timeout 失败: {e}"))?;
    Ok(conn)
}

fn threads_model_provider_usable(conn: &Connection) -> bool {
    table_exists(conn, "threads") && has_column(conn, "threads", "model_provider")
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let Ok(mut stmt) = conn.prepare(&format!("PRAGMA table_info({table})")) else {
        return false;
    };
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default();
    names.iter().any(|name| name == column)
}

fn backup_state_db(path: &Path, backup_root: &Path, conn: &Connection) -> Result<(), String> {
    let target_dir = backup_root.join("state");
    fs::create_dir_all(&target_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;
    let target = target_dir.join(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(CODEX_STATE_DB_FILENAME),
    );
    let _ = fs::remove_file(&target);
    let mut dst = Connection::open(&target).map_err(|e| format!("创建备份 DB 失败: {e}"))?;
    Backup::new(conn, &mut dst)
        .and_then(|backup| backup.run_to_completion(5, Duration::from_millis(250), None))
        .map_err(|e| format!("备份 state DB 失败: {e}"))?;
    Ok(())
}

fn placeholders(count: usize) -> String {
    std::iter::repeat("?".to_string())
        .take(count)
        .collect::<Vec<_>>()
        .join(", ")
}

/// 把官方桶(openai)的会话迁入共享 custom 桶。
/// `codex_home` / `config_text` 提供实态，`backup_root` 为账本代际目录。
pub fn migrate_official_history(
    codex_home: &Path,
    config_text: &str,
    backup_root: &Path,
) -> Result<HistoryOutcome, String> {
    // live 必须已实际路由到 custom 桶: 注入被拒绝(显式 model_provider /
    // 形态冲突 custom 表)时新会话仍落 openai 桶，迁移只会把历史搬进
    // live 看不见的桶里。
    if !config_routes_custom(config_text) {
        return Ok(HistoryOutcome::skipped("live_not_unified"));
    }

    let sources: HashSet<&str> = [OFFICIAL_PROVIDER_ID].into_iter().collect();
    fs::create_dir_all(backup_root).map_err(|e| format!("创建备份目录失败: {e}"))?;
    let jsonl_files = rewrite_jsonl_files(
        codex_home,
        |line| rewrite_meta_line(line, &sources, UNIFIED_PROVIDER_ID),
        backup_root,
    )?;
    let state_rows = migrate_state_dbs(codex_home, config_text, backup_root)?;

    // 记录备份代际来源目录，还原时只取属于当前 Codex 目录的账本。
    let meta = serde_json::json!({ "codexConfigDir": canonical_dir_string(codex_home) });
    fs::write(
        backup_root.join("meta.json"),
        serde_json::to_vec_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("写入备份账本失败: {e}"))?;

    Ok(HistoryOutcome {
        jsonl_files,
        state_rows,
        skipped_reason: None,
    })
}

fn migrate_state_dbs(
    codex_home: &Path,
    config_text: &str,
    backup_root: &Path,
) -> Result<usize, String> {
    let mut rows = 0;
    for db_path in state_db_paths(codex_home, config_text) {
        if !db_path.exists() {
            continue;
        }
        let conn = open_state_db(&db_path)?;
        if !threads_model_provider_usable(&conn) {
            continue;
        }
        let matching: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM threads WHERE model_provider = ?1",
                [OFFICIAL_PROVIDER_ID],
                |row| row.get(0),
            )
            .map_err(|e| format!("统计待迁移会话行失败: {e}"))?;
        if matching == 0 {
            continue;
        }
        backup_state_db(&db_path, backup_root, &conn)?;
        let changed = conn
            .execute(
                "UPDATE threads SET model_provider = ?1 WHERE model_provider = ?2",
                rusqlite::params![UNIFIED_PROVIDER_ID, OFFICIAL_PROVIDER_ID],
            )
            .map_err(|e| format!("迁移会话行失败: {e}"))?;
        rows += changed;
    }
    Ok(rows)
}

/// 是否存在可用于还原的迁移账本(属当前 Codex 目录)。
pub fn has_unify_backup(ledger_parent: &Path, codex_home: &Path) -> bool {
    let key = canonical_dir_string(codex_home);
    ledger_generations(ledger_parent)
        .iter()
        .any(|generation| generation_matches_dir(generation, &key))
}

fn canonical_dir_string(dir: &Path) -> String {
    fs::canonicalize(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn ledger_generations(ledger_parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(ledger_parent) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

/// 代际是否属于指定 Codex 目录；无 meta.json 时宽容接受(旧版本账本)。
fn generation_matches_dir(generation: &Path, codex_dir_key: &str) -> bool {
    let Ok(text) = fs::read_to_string(generation.join("meta.json")) else {
        return true;
    };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| {
            value
                .get("codexConfigDir")
                .and_then(Value::as_str)
                .map(|dir| dir == codex_dir_key)
        })
        .unwrap_or(true)
}

/// 从迁移账本精确还原官方会话: 只把账本中记录的会话 id / thread id
/// 从 custom 桶翻回 openai 桶，迁移后新增的 custom 会话不受影响。
pub fn restore_official_history(
    codex_home: &Path,
    config_text: &str,
    ledger_parent: &Path,
    restore_backup_root: &Path,
) -> Result<HistoryOutcome, String> {
    let (session_ids, thread_ids) = collect_official_ledger(ledger_parent, codex_home);
    if session_ids.is_empty() && thread_ids.is_empty() {
        return Ok(HistoryOutcome::skipped("no_backup_ledger"));
    }

    let official_ids: HashSet<&str> = session_ids.iter().map(String::as_str).collect();
    let jsonl_files = rewrite_jsonl_files(
        codex_home,
        |line| rewrite_meta_line_for_restore(line, &official_ids),
        restore_backup_root,
    )?;
    let state_rows = restore_state_dbs(codex_home, config_text, &thread_ids, restore_backup_root)?;

    if jsonl_files == 0 && state_rows == 0 {
        return Ok(HistoryOutcome::skipped("nothing_to_restore"));
    }
    Ok(HistoryOutcome {
        jsonl_files,
        state_rows,
        skipped_reason: None,
    })
}

/// 账本收集: jsonl 备份里 openai 桶的会话 id + state 备份里 openai 桶的
/// thread id。只采纳属于当前 Codex 目录的代际。
fn collect_official_ledger(ledger_parent: &Path, codex_home: &Path) -> (Vec<String>, Vec<String>) {
    let key = canonical_dir_string(codex_home);
    let mut session_ids = HashSet::new();
    let mut thread_ids = BTreeSet::new();
    for generation in ledger_generations(ledger_parent) {
        if !generation_matches_dir(&generation, &key) {
            continue;
        }
        let mut backup_files = Vec::new();
        collect_jsonl_files(&generation.join("jsonl"), &mut backup_files, 0, 10);
        for file in backup_files {
            collect_session_ids_from_backup(&file, &mut session_ids);
        }
        let mut backup_dbs = Vec::new();
        collect_sqlite_files(&generation.join("state"), &mut backup_dbs, 0, 4);
        for db in backup_dbs {
            collect_thread_ids_from_backup(&db, &mut thread_ids);
        }
    }
    (
        session_ids.into_iter().collect(),
        thread_ids.into_iter().collect(),
    )
}

fn collect_session_ids_from_backup(path: &Path, session_ids: &mut HashSet<String>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        if !line.contains("\"session_meta\"") || !line.contains("\"model_provider\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("model_provider").and_then(Value::as_str) != Some(OFFICIAL_PROVIDER_ID) {
            continue;
        }
        if let Some(id) = payload.get("id").and_then(Value::as_str) {
            session_ids.insert(id.to_string());
        }
    }
}

fn collect_thread_ids_from_backup(path: &Path, thread_ids: &mut BTreeSet<String>) {
    let Ok(conn) = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return;
    };
    if !threads_model_provider_usable(&conn) {
        return;
    }
    let Ok(mut stmt) = conn.prepare("SELECT id FROM threads WHERE model_provider = ?1") else {
        return;
    };
    let Ok(rows) = stmt.query_map([OFFICIAL_PROVIDER_ID], |row| row.get::<_, String>(0)) else {
        return;
    };
    for id in rows.flatten() {
        thread_ids.insert(id);
    }
}

fn rewrite_meta_line_for_restore(line: &str, official_ids: &HashSet<&str>) -> Option<String> {
    if !line.contains("\"session_meta\"") || !line.contains("\"model_provider\"") {
        return None;
    }
    let mut value: Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = value.get_mut("payload")?.as_object_mut()?;
    if payload.get("model_provider")?.as_str()? != UNIFIED_PROVIDER_ID {
        return None;
    }
    let session_id = payload.get("id")?.as_str()?;
    if !official_ids.contains(session_id) {
        return None;
    }
    payload.insert(
        "model_provider".to_string(),
        Value::String(OFFICIAL_PROVIDER_ID.to_string()),
    );
    serde_json::to_string(&value).ok()
}

fn restore_state_dbs(
    codex_home: &Path,
    config_text: &str,
    thread_ids: &[String],
    restore_backup_root: &Path,
) -> Result<usize, String> {
    if thread_ids.is_empty() {
        return Ok(0);
    }
    let mut rows = 0;
    for db_path in state_db_paths(codex_home, config_text) {
        if !db_path.exists() {
            continue;
        }
        let conn = open_state_db(&db_path)?;
        if !threads_model_provider_usable(&conn) {
            continue;
        }
        let mut matching: i64 = 0;
        for chunk in thread_ids.chunks(STATE_DB_ID_CHUNK) {
            let sql = format!(
                "SELECT COUNT(*) FROM threads WHERE model_provider = ? AND id IN ({})",
                placeholders(chunk.len())
            );
            let mut values: Vec<String> = vec![UNIFIED_PROVIDER_ID.to_string()];
            values.extend(chunk.iter().cloned());
            matching += conn
                .query_row(&sql, params_from_iter(values.iter()), |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|e| format!("统计待还原会话行失败: {e}"))?;
        }
        if matching == 0 {
            continue;
        }
        backup_state_db(&db_path, restore_backup_root, &conn)?;
        for chunk in thread_ids.chunks(STATE_DB_ID_CHUNK) {
            let sql = format!(
                "UPDATE threads SET model_provider = ? WHERE model_provider = ? AND id IN ({})",
                placeholders(chunk.len())
            );
            let mut values: Vec<String> = vec![
                OFFICIAL_PROVIDER_ID.to_string(),
                UNIFIED_PROVIDER_ID.to_string(),
            ];
            values.extend(chunk.iter().cloned());
            rows += conn
                .execute(&sql, params_from_iter(values.iter()))
                .map_err(|e| format!("还原会话行失败: {e}"))?;
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta_line(session_id: &str, provider: &str) -> String {
        json!({
            "type": "session_meta",
            "payload": {
                "id": session_id,
                "model_provider": provider,
            }
        })
        .to_string()
    }

    #[test]
    fn migrates_and_restores_official_sessions_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let codex_home = dir.path().join("codex");
        let session_dir = codex_home.join("sessions").join("2026").join("09");
        fs::create_dir_all(&session_dir).unwrap();

        // 官方会话 + 已是 custom 的会话(迁移后新增)
        let official_file = session_dir.join("rollout-a.jsonl");
        fs::write(
            &official_file,
            format!("{}\nother line\n", meta_line("session-a", "openai")),
        )
        .unwrap();
        let custom_file = session_dir.join("rollout-b.jsonl");
        fs::write(
            &custom_file,
            format!("{}\n", meta_line("session-b", "custom")),
        )
        .unwrap();

        // state DB: openai 线程 + custom 线程
        let db_path = codex_home.join(CODEX_STATE_DB_FILENAME);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT);
                 INSERT INTO threads VALUES ('thread-a', 'openai');
                 INSERT INTO threads VALUES ('thread-b', 'custom');",
            )
            .unwrap();
        }

        let backup_root = dir.path().join("backups").join("gen-1");
        let outcome =
            migrate_official_history(&codex_home, "model_provider = \"custom\"\n", &backup_root)
                .unwrap();
        assert_eq!(outcome.skipped_reason, None);
        assert_eq!(outcome.jsonl_files, 1);
        assert_eq!(outcome.state_rows, 1);

        // 官方会话已迁入 custom 桶，既有 custom 会话不动
        let migrated = fs::read_to_string(&official_file).unwrap();
        assert!(migrated.contains("\"model_provider\":\"custom\""));
        assert!(migrated.contains("other line"));
        // 备份账本保留原始内容
        let backup_file = backup_root
            .join("jsonl")
            .join("sessions")
            .join("2026")
            .join("09")
            .join("rollout-a.jsonl");
        assert!(backup_file.exists());
        assert!(fs::read_to_string(&backup_file)
            .unwrap()
            .contains("\"model_provider\":\"openai\""));

        // 还原: 只翻回账本中的官方会话(账本父目录下按代际归档)
        let restore_root = dir.path().join("backups").join("restore-gen");
        let restored = restore_official_history(
            &codex_home,
            "model_provider = \"custom\"\n",
            &dir.path().join("backups"),
            &restore_root,
        )
        .unwrap();
        assert_eq!(restored.skipped_reason, None);
        assert_eq!(restored.jsonl_files, 1);
        assert_eq!(restored.state_rows, 1);

        let back = fs::read_to_string(&official_file).unwrap();
        assert!(back.contains("\"model_provider\":\"openai\""));
        let untouched = fs::read_to_string(&custom_file).unwrap();
        assert!(untouched.contains("\"model_provider\":\"custom\""));
        {
            let conn = Connection::open(&db_path).unwrap();
            let a: String = conn
                .query_row(
                    "SELECT model_provider FROM threads WHERE id = 'thread-a'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(a, "openai");
            let b: String = conn
                .query_row(
                    "SELECT model_provider FROM threads WHERE id = 'thread-b'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(b, "custom");
        }
    }

    #[test]
    fn migration_skips_when_live_not_unified() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = migrate_official_history(
            dir.path(),
            "model_provider = \"openai\"\n",
            &dir.path().join("gen"),
        )
        .unwrap();
        assert_eq!(outcome.skipped_reason.as_deref(), Some("live_not_unified"));
    }

    #[test]
    fn restore_without_ledger_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = restore_official_history(
            dir.path(),
            "",
            &dir.path().join("empty-ledger"),
            &dir.path().join("restore"),
        )
        .unwrap();
        assert_eq!(outcome.skipped_reason.as_deref(), Some("no_backup_ledger"));
    }

    #[test]
    fn state_db_paths_include_sqlite_home_override() {
        let temp = tempfile::tempdir().unwrap();
        let config = format!("sqlite_home = '{}'\n", temp.path().display());
        let paths = state_db_paths(Path::new("/home/user/.codex"), &config);
        assert!(paths.contains(&PathBuf::from("/home/user/.codex").join(CODEX_STATE_DB_FILENAME)));
        assert!(paths.contains(&temp.path().join(CODEX_STATE_DB_FILENAME)));
    }

    #[test]
    fn has_unify_backup_respects_generation_dir_match() {
        let dir = tempfile::tempdir().unwrap();
        let codex_home = dir.path().join("codex");
        fs::create_dir_all(&codex_home).unwrap();
        let ledger = dir.path().join("ledger").join("gen-1");
        fs::create_dir_all(ledger.join("jsonl")).unwrap();
        assert!(has_unify_backup(&ledger.parent().unwrap(), &codex_home));

        // 不同 Codex 目录的代际不采纳
        fs::write(
            ledger.join("meta.json"),
            serde_json::to_vec_pretty(&json!({"codexConfigDir": "/other/codex"})).unwrap(),
        )
        .unwrap();
        assert!(!has_unify_backup(&ledger.parent().unwrap(), &codex_home));
    }
}
