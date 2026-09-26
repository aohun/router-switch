//! Wake-style multi-agent session catalog.
//!
//! Builds an index from `tokens-core` file scans (+ legacy Codex/Claude meta),
//! persists rows via [`store::Store`], and loads transcripts for the 3-pane UI.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use store::{SessionIndexRow, SessionUserData, Store};
use tokens_core::{scan_all_clients, ClientId};

use crate::sessions::{self, SessionMessage, SessionMeta};

/// UI-facing session row (index + user flags).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedSession {
    pub key: String,
    pub agent_id: String,
    pub session_id: String,
    pub title: String,
    pub project_name: Option<String>,
    pub source_path: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub message_count: i64,
    pub tokens: i64,
    pub favorite: bool,
    pub pinned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionScope {
    #[default]
    All,
    Starred,
    Agent,
    Project,
}

#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub scope: SessionScope,
    pub agent_id: Option<String>,
    pub project_name: Option<String>,
    pub query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionSortKey {
    #[default]
    Updated,
    Created,
    Messages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionSort {
    pub key: SessionSortKey,
    /// `false` = descending (Wake default).
    pub ascending: bool,
}

/// Agents shown in the Wake-style sidebar (order matches common Wake list).
pub const SIDEBAR_AGENTS: &[&str] = &[
    "claude",
    "codex",
    "grok",
    "cursor",
    "opencode",
    "pi",
    "kimi",
    "qoder",
    "openclaw",
    "workbuddy",
    "zcode",
];

/// Hidden from the sessions sidebar (still scannable for Insights if needed).
const HIDDEN_SIDEBAR_AGENTS: &[&str] = &["gemini", "antigravity", "kiro"];

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn mtime_ms(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

fn session_id_from_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("session");

    // Kimi Code: …/session_<uuid>/agents/main/wire.jsonl → session_<uuid>
    if file_name == "wire.jsonl" {
        for ancestor in path.ancestors().skip(1).take(5) {
            if let Some(name) = ancestor.file_name().and_then(|s| s.to_str()) {
                if let Some(id) = name.strip_prefix("session_") {
                    if !id.is_empty() {
                        return id.to_string();
                    }
                }
            }
        }
    }

    // Sidecar-style session files live under `<session-id>/<fixed-name>`
    // (Grok: updates.jsonl, Junie: events.jsonl). Stem would collide all sessions.
    if matches!(
        file_name,
        "updates.jsonl" | "events.jsonl" | "session.jsonl" | "session.json" | "wire.jsonl"
    ) {
        if let Some(id) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
        {
            if !id.is_empty() && id != "sessions" && id != "main" && id != "agents" {
                return id.to_string();
            }
        }
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session")
        .to_string()
}

fn count_jsonl_lines(path: &Path) -> i64 {
    let Ok(file) = File::open(path) else {
        return 0;
    };
    BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .count() as i64
}

fn percent_decode_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = (bytes[i + 1] as char).to_digit(16);
            let l = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn project_name_from_cwd(cwd: &str) -> Option<String> {
    let cwd = cwd.trim().trim_end_matches('/');
    if cwd.is_empty() {
        return None;
    }
    Path::new(cwd)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Grok summary.json sidecar (title / cwd / timestamps / message count).
struct GrokSummaryMeta {
    title: Option<String>,
    project_name: Option<String>,
    created_at: Option<i64>,
    updated_at: Option<i64>,
    message_count: Option<i64>,
}

fn parse_iso_ms(s: &str) -> Option<i64> {
    // Accept RFC3339 / ISO-8601 timestamps from summary.json.
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn read_grok_summary(updates_path: &Path) -> GrokSummaryMeta {
    let mut meta = GrokSummaryMeta {
        title: None,
        project_name: None,
        created_at: None,
        updated_at: None,
        message_count: None,
    };
    let summary_path = updates_path.with_file_name("summary.json");
    let Ok(raw) = std::fs::read_to_string(&summary_path) else {
        return meta;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return meta;
    };
    for k in ["generated_title", "session_summary"] {
        if let Some(t) = v.get(k).and_then(|x| x.as_str()) {
            let t = t.trim();
            if !t.is_empty() {
                meta.title = Some(t.chars().take(120).collect());
                break;
            }
        }
    }
    if let Some(cwd) = v
        .pointer("/info/cwd")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        meta.project_name = project_name_from_cwd(cwd);
    }
    if let Some(t) = v.get("created_at").and_then(|x| x.as_str()) {
        meta.created_at = parse_iso_ms(t);
    }
    for k in ["updated_at", "last_active_at"] {
        if let Some(t) = v.get(k).and_then(|x| x.as_str()) {
            if let Some(ms) = parse_iso_ms(t) {
                meta.updated_at = Some(ms);
                break;
            }
        }
    }
    meta.message_count = v
        .get("num_chat_messages")
        .or_else(|| v.get("num_messages"))
        .and_then(|x| x.as_i64());
    meta
}

fn project_from_path(path: &Path, agent: &str) -> Option<String> {
    // Claude: ~/.claude/projects/<slug>/uuid.jsonl
    if agent == "claude" {
        return path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(|s| s.replace('-', "/"));
    }
    // Grok: ~/.grok/sessions/<urlencoded-cwd>/<uuid>/updates.jsonl
    if agent == "grok" {
        if let Some(from_summary) = read_grok_summary(path).project_name {
            return Some(from_summary);
        }
        return path
            .parent() // uuid dir
            .and_then(|p| p.parent()) // urlencoded cwd
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(percent_decode_component)
            .and_then(|cwd| project_name_from_cwd(&cwd).or(Some(cwd)));
    }
    // Kimi: …/wd_<name>_<hash>/session_<uuid>/agents/main/wire.jsonl
    if agent == "kimi" {
        for ancestor in path.ancestors().skip(1).take(6) {
            if let Some(name) = ancestor.file_name().and_then(|s| s.to_str()) {
                if let Some(rest) = name.strip_prefix("wd_") {
                    let label = rest.rsplit_once('_').map(|(n, _)| n).unwrap_or(rest);
                    if !label.is_empty() {
                        return Some(label.to_string());
                    }
                }
            }
        }
    }
    // OpenClaw: ~/.openclaw/agents/<agentId>/sessions/<uuid>.jsonl
    if agent == "openclaw" {
        // Prefer cwd from session header; fall back to agent id (not "sessions").
        if let Some(cwd) = peek_openclaw_cwd(path) {
            if let Some(name) = project_name_from_cwd(&cwd) {
                return Some(name);
            }
        }
        return path
            .parent() // sessions/
            .and_then(|p| p.parent()) // <agentId>/
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty() && *s != "sessions")
            .map(|s| s.to_string());
    }
    // WorkBuddy / CodeBuddy: ~/.workbuddy/projects/<slug>/<uuid>.jsonl
    if agent == "workbuddy" {
        return path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(clean_project_slug);
    }
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .filter(|s| !matches!(*s, "sessions" | "main" | "agents" | "message" | "storage"))
        .map(|s| s.to_string())
}

fn clean_project_slug(slug: &str) -> String {
    // Users-wayne-WorkBuddy-2026-09-03-08-26-57 → WorkBuddy / last meaningful segment
    let parts: Vec<&str> = slug.split('-').collect();
    if parts.len() >= 2 {
        // Drop leading Users / home segments when present.
        let start = parts
            .iter()
            .position(|p| {
                matches!(
                    p.to_ascii_lowercase().as_str(),
                    "workbuddy" | "desktop" | "documents" | "git" | "projects"
                )
            })
            .unwrap_or(0);
        let slice = &parts[start..];
        // Prefer a short human label: first 1–2 non-numeric tokens.
        let mut out = Vec::new();
        for p in slice.iter().take(3) {
            if p.chars().all(|c| c.is_ascii_digit()) {
                break;
            }
            out.push(*p);
        }
        if !out.is_empty() {
            return out.join("-");
        }
    }
    slug.chars().take(40).collect()
}

fn peek_openclaw_cwd(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(5).flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) == Some("session") {
            if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str()) {
                if !cwd.trim().is_empty() {
                    return Some(cwd.to_string());
                }
            }
        }
    }
    None
}

fn clean_title_candidate(raw: &str) -> Option<String> {
    let mut t = raw.replace('\r', " ").replace('\n', " ");
    // Cursor agent transcripts wrap the real prompt in <user_query>…</user_query>.
    if let Some(start) = t.to_ascii_lowercase().find("<user_query>") {
        let after = start + "<user_query>".len();
        if let Some(rel_end) = t[after..].to_ascii_lowercase().find("</user_query>") {
            t = t[after..after + rel_end].to_string();
        }
    }
    // Strip OpenClaw / gateway "Sender (untrusted metadata): ```json … ```" prefix.
    if let Some(idx) = t.find("```") {
        if t[..idx].contains("Sender") || t[..idx].contains("untrusted metadata") {
            if let Some(end) = t[idx + 3..].find("```") {
                t = t[idx + 3 + end + 3..].to_string();
            }
        }
    }
    // Drop leading markdown fences / timestamp prefixes like "[Fri 2026-03-13 …]"
    let mut t = t.trim().to_string();
    if t.starts_with('[') {
        if let Some(end) = t.find(']') {
            let head = &t[1..end];
            // Only strip if it looks like a date/time stamp, not a JSON array.
            if head.contains("202") || head.contains(':') || head.contains("GMT") {
                t = t[end + 1..].trim().to_string();
            }
        }
    }
    let t = t
        .trim_start_matches(|c: char| c == '`' || c == '#' || c == '*' || c == '-' || c == '>')
        .trim();
    if t.is_empty() {
        return None;
    }
    // Reject raw JSON objects / arrays.
    if (t.starts_with('{') && t.contains('}')) || (t.starts_with('[') && t.contains('{')) {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("<system")
        || lower.starts_with("<user_info")
        || lower.starts_with("<system-reminder")
        || lower.starts_with("<image_local_path>")
        || lower.starts_with("<manually_attached_skills>")
        || lower.starts_with("<timestamp>")
        || lower.contains("<system-reminder")
        || lower.contains("<user_info")
        || lower.contains("data-role=\"user-context\"")
        || t == "(No content)"
        || t == "/compact"
        || t == "New Session"
        || t.starts_with("New session -")
    {
        return None;
    }
    Some(t.chars().take(80).collect())
}

fn peek_title(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(80).flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        // WorkBuddy / CodeBuddy title events
        match v.get("type").and_then(|t| t.as_str()) {
            Some("custom-title") => {
                if let Some(t) = v
                    .get("customTitle")
                    .or_else(|| v.get("title"))
                    .and_then(|x| x.as_str())
                {
                    if let Some(c) = clean_title_candidate(t) {
                        return Some(c);
                    }
                }
            }
            Some("ai-title") => {
                if let Some(t) = v.get("aiTitle").and_then(|x| x.as_str()) {
                    if let Some(c) = clean_title_candidate(t) {
                        return Some(c);
                    }
                }
            }
            Some("topic") => {
                if let Some(t) = v.get("topic").and_then(|x| x.as_str()) {
                    if let Some(c) = clean_title_candidate(t) {
                        return Some(c);
                    }
                }
            }
            _ => {}
        }
        // OpenClaw / Pi: nested message.role + message.content[].text
        // WorkBuddy: flat role + content[].text / input_text
        if v.get("type").and_then(|t| t.as_str()) == Some("message") {
            let nested_role = v
                .pointer("/message/role")
                .and_then(|r| r.as_str())
                .unwrap_or("");
            if nested_role == "user" {
                if let Some(arr) = v.pointer("/message/content").and_then(|c| c.as_array()) {
                    for item in arr {
                        if let Some(t) = item.get("text").and_then(|x| x.as_str()) {
                            if let Some(c) = clean_title_candidate(t) {
                                return Some(c);
                            }
                        }
                    }
                }
            }
            let flat_role = v.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if flat_role == "user" {
                if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
                    for item in arr {
                        if let Some(t) = item
                            .get("text")
                            .or_else(|| item.get("input_text"))
                            .and_then(|x| x.as_str())
                        {
                            if let Some(c) = clean_title_candidate(t) {
                                return Some(c);
                            }
                        }
                    }
                }
            }
            continue;
        }
        // Common shapes across Codex / Claude / OpenCode-ish JSONL
        let candidates = [
            v.pointer("/message/content"),
            v.pointer("/content"),
            v.pointer("/text"),
            v.pointer("/title"),
            v.pointer("/session/title"),
            v.pointer("/input"),
        ];
        for c in candidates.into_iter().flatten() {
            if let Some(s) = c.as_str() {
                if let Some(t) = clean_title_candidate(s) {
                    return Some(t);
                }
            }
            if let Some(arr) = c.as_array() {
                for item in arr {
                    if let Some(t) = item
                        .get("text")
                        .or_else(|| item.get("input_text"))
                        .and_then(|x| x.as_str())
                    {
                        if let Some(c) = clean_title_candidate(t) {
                            return Some(c);
                        }
                    }
                }
            }
        }
        if v.get("type").and_then(|t| t.as_str()) == Some("user")
            || v.get("role").and_then(|t| t.as_str()) == Some("user")
        {
            if let Some(s) = v.get("text").and_then(|x| x.as_str()) {
                if let Some(t) = clean_title_candidate(s) {
                    return Some(t);
                }
            }
        }
    }
    None
}

fn read_kimi_state_title(wire_path: &Path) -> Option<String> {
    let session_dir = wire_path.ancestors().nth(3)?;
    let raw = std::fs::read_to_string(session_dir.join("state.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let t = v.get("title").and_then(|x| x.as_str())?;
    clean_title_candidate(t)
}

fn read_workbuddy_title(path: &Path) -> Option<String> {
    // Prefer dedicated title events (last-wins for ai-title/topic; custom wins).
    let file = File::open(path).ok()?;
    let mut custom = None;
    let mut ai = None;
    let mut topic = None;
    // ai-title often arrives a few dozen events after the first user turn.
    for line in BufReader::new(file).lines().take(800).flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("custom-title") => {
                if let Some(t) = v
                    .get("customTitle")
                    .or_else(|| v.get("title"))
                    .and_then(|x| x.as_str())
                {
                    if let Some(c) = clean_title_candidate(t) {
                        custom = Some(c);
                    }
                }
            }
            Some("ai-title") => {
                if let Some(t) = v.get("aiTitle").and_then(|x| x.as_str()) {
                    if let Some(c) = clean_title_candidate(t) {
                        ai = Some(c);
                    }
                }
            }
            Some("topic") => {
                if let Some(t) = v.get("topic").and_then(|x| x.as_str()) {
                    if let Some(c) = clean_title_candidate(t) {
                        topic = Some(c);
                    }
                }
            }
            _ => {}
        }
        // custom-title wins immediately; keep scanning for it otherwise.
        if custom.is_some() {
            break;
        }
    }
    custom.or(ai).or(topic).or_else(|| peek_title(path))
}

fn should_skip_index_path(agent: &str, path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    // OpenCode sessions come from SQLite, not per-message JSON files.
    if agent == "opencode" {
        return true;
    }
    // Cursor usage CSV is Insights-only; real sessions live under agent-transcripts.
    if agent == "cursor" {
        return true;
    }
    // ZCode sessions come from cli/db/db.sqlite (Wake), not legacy projects JSONL.
    if agent == "zcode" {
        return true;
    }
    // Single DB file is not a session row.
    if agent == "workbuddy"
        && (name == "workbuddy.db" || name.ends_with(".db") || name.ends_with(".ndjson"))
    {
        return true;
    }
    // WorkBuddy rollback / meta sidecars should never become sessions.
    if name.ends_with(".file-rollback.ndjson") || name.ends_with(".meta.json") {
        return true;
    }
    // Kimi: only main agent wire files (skip subagent wires).
    if agent == "kimi" && name == "wire.jsonl" {
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if parent != "main" {
            return true;
        }
    }
    false
}

fn resolve_title(agent: &str, path: &Path, session_id: &str) -> String {
    let title = match agent {
        "grok" => read_grok_summary(path)
            .title
            .or_else(|| peek_title(path)),
        "kimi" => read_kimi_state_title(path).or_else(|| peek_title(path)),
        "workbuddy" => read_workbuddy_title(path),
        "openclaw" => peek_title(path),
        _ => peek_title(path),
    };
    title
        .and_then(|t| clean_title_candidate(&t))
        .unwrap_or_else(|| session_id.to_string())
}

fn open_sqlite_ro(path: &Path) -> Option<rusqlite::Connection> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
        | rusqlite::OpenFlags::SQLITE_OPEN_URI;
    let uri = format!("file:{}?immutable=1", path.display());
    rusqlite::Connection::open_with_flags(&uri, flags)
        .or_else(|_| {
            rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
        })
        .ok()
}

/// Enumerate OpenCode sessions from `opencode*.db` (Wake-compatible).
fn list_opencode_sessions(db_path: &Path) -> Vec<SessionIndexRow> {
    let Some(conn) = open_sqlite_ro(db_path) else {
        return Vec::new();
    };
    let queries = [
        "SELECT id, COALESCE(title, ''), COALESCE(directory, ''), time_created, time_updated FROM session",
        "SELECT id, COALESCE(title, ''), COALESCE(directory, ''), time_created, time_updated FROM session_v2",
    ];
    let mut out = Vec::new();
    for sql in queries {
        let Ok(mut stmt) = conn.prepare(sql) else {
            continue;
        };
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let directory: String = row.get(2)?;
            let created: i64 = row.get(3).unwrap_or(0);
            let updated: i64 = row.get(4).unwrap_or(created);
            Ok((id, title, directory, created, updated))
        });
        let Ok(rows) = rows else {
            continue;
        };
        for row in rows.flatten() {
            let (id, title, directory, created, updated) = row;
            if id.is_empty() {
                continue;
            }
            let title = clean_title_candidate(&title).unwrap_or_else(|| id.clone());
            let project_name = project_name_from_cwd(&directory);
            out.push(SessionIndexRow {
                session_key: format!("opencode:{id}"),
                agent_id: "opencode".into(),
                session_id: id,
                title: Some(title),
                project_name,
                source_path: Some(db_path.to_string_lossy().to_string()),
                created_at: (created > 0).then_some(created),
                updated_at: (updated > 0).then_some(updated),
                message_count: 0,
                tokens: 0,
            });
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// Cursor CLI agent transcripts: `~/.cursor/projects/<slug>/agent-transcripts/<uuid>/<uuid>.jsonl`
fn list_cursor_sessions(home: &Path) -> Vec<SessionIndexRow> {
    let root = home.join(".cursor").join("projects");
    let mut out = Vec::new();
    let Ok(projects) = std::fs::read_dir(&root) else {
        return out;
    };
    for proj in projects.flatten() {
        let proj_path = proj.path();
        if !proj_path.is_dir() {
            continue;
        }
        let slug = proj
            .file_name()
            .to_str()
            .unwrap_or_default()
            .to_string();
        let transcripts = proj_path.join("agent-transcripts");
        let Ok(sessions) = std::fs::read_dir(&transcripts) else {
            continue;
        };
        for sess in sessions.flatten() {
            let sess_dir = sess.path();
            if !sess_dir.is_dir() {
                continue;
            }
            let id = sess
                .file_name()
                .to_str()
                .unwrap_or_default()
                .to_string();
            if id.is_empty() || id == "subagents" {
                continue;
            }
            let path = sess_dir.join(format!("{id}.jsonl"));
            if !path.is_file() {
                continue;
            }
            let updated = mtime_ms(&path);
            let title = resolve_title("cursor", &path, &id);
            out.push(SessionIndexRow {
                session_key: format!("cursor:{id}"),
                agent_id: "cursor".into(),
                session_id: id,
                title: Some(title),
                project_name: Some(clean_project_slug(&slug)),
                source_path: Some(path.to_string_lossy().to_string()),
                created_at: updated,
                updated_at: updated,
                message_count: count_jsonl_lines(&path),
                tokens: 0,
            });
        }
    }
    out
}

/// Qoder CLI: `~/.qoder/projects/<project-key>/<session-id>.jsonl` (+ optional `transcript/`).
fn list_qoder_sessions(home: &Path) -> Vec<SessionIndexRow> {
    let root = home.join(".qoder").join("projects");
    let mut out = Vec::new();
    let Ok(projects) = std::fs::read_dir(&root) else {
        return out;
    };
    for proj in projects.flatten() {
        let proj_path = proj.path();
        if !proj_path.is_dir() {
            continue;
        }
        let slug = proj
            .file_name()
            .to_str()
            .unwrap_or_default()
            .to_string();
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&proj_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name();
                let name = name.to_str().unwrap_or_default();
                if name == "subagents" {
                    continue;
                }
                if p.is_file() && name.ends_with(".jsonl") {
                    candidates.push(p);
                } else if p.is_dir() && name == "transcript" {
                    if let Ok(inner) = std::fs::read_dir(&p) {
                        for f in inner.flatten() {
                            let fp = f.path();
                            if fp.is_file()
                                && f.file_name()
                                    .to_str()
                                    .map(|s| s.ends_with(".jsonl"))
                                    .unwrap_or(false)
                            {
                                candidates.push(fp);
                            }
                        }
                    }
                }
            }
        }
        for path in candidates {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("session")
                .to_string();
            let updated = mtime_ms(&path);
            let title = resolve_title("qoder", &path, &id);
            out.push(SessionIndexRow {
                session_key: format!("qoder:{id}"),
                agent_id: "qoder".into(),
                session_id: id,
                title: Some(title),
                project_name: Some(clean_project_slug(&slug)),
                source_path: Some(path.to_string_lossy().to_string()),
                created_at: updated,
                updated_at: updated,
                message_count: count_jsonl_lines(&path),
                tokens: 0,
            });
        }
    }
    out
}

/// ZCode desktop/CLI sessions from `~/.zcode/cli/db/db.sqlite` (Wake-compatible).
fn list_zcode_sessions(db_path: &Path) -> Vec<SessionIndexRow> {
    let Some(conn) = open_sqlite_ro(db_path) else {
        return Vec::new();
    };
    // Prefer Wake whitelist when task_type exists; else parent_id IS NULL.
    let queries = [
        "SELECT id, COALESCE(title, ''), COALESCE(directory, ''), time_created, time_updated \
         FROM session WHERE task_type IN ('interactive', 'fork', 'selection_side_chat')",
        "SELECT id, COALESCE(title, ''), COALESCE(directory, ''), time_created, time_updated \
         FROM session WHERE parent_id IS NULL",
        "SELECT id, COALESCE(title, ''), COALESCE(directory, ''), time_created, time_updated FROM session",
    ];
    let mut out = Vec::new();
    for sql in queries {
        let Ok(mut stmt) = conn.prepare(sql) else {
            continue;
        };
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let directory: String = row.get(2)?;
            let created: i64 = row.get(3).unwrap_or(0);
            let updated: i64 = row.get(4).unwrap_or(created);
            Ok((id, title, directory, created, updated))
        });
        let Ok(rows) = rows else {
            continue;
        };
        for row in rows.flatten() {
            let (id, title, directory, created, updated) = row;
            if id.is_empty() {
                continue;
            }
            let title = clean_title_candidate(&title).unwrap_or_else(|| id.clone());
            out.push(SessionIndexRow {
                session_key: format!("zcode:{id}"),
                agent_id: "zcode".into(),
                session_id: id,
                title: Some(title),
                project_name: project_name_from_cwd(&directory),
                source_path: Some(db_path.to_string_lossy().to_string()),
                created_at: (created > 0).then_some(created),
                updated_at: (updated > 0).then_some(updated),
                message_count: 0,
                tokens: 0,
            });
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

fn normalize_agent_id(client: &str) -> String {
    let c = client.to_lowercase();
    if c.contains("claude") {
        "claude".into()
    } else if c.contains("codex") {
        "codex".into()
    } else if c.contains("grok") {
        "grok".into()
    } else if c.contains("cursor") {
        "cursor".into()
    } else if c.contains("opencode") {
        "opencode".into()
    } else if c == "pi" || c.starts_with("pi-") {
        "pi".into()
    } else if c.contains("kimi") {
        "kimi".into()
    } else if c.contains("gemini") {
        "gemini".into()
    } else if c.contains("antigravity") {
        "antigravity".into()
    } else if c.contains("qoder") {
        "qoder".into()
    } else if c.contains("openclaw") {
        "openclaw".into()
    } else if c.contains("workbuddy") || c.contains("codebuddy") {
        "workbuddy".into()
    } else if c.contains("zcode") {
        "zcode".into()
    } else {
        c
    }
}

fn brand_id_for_agent(agent: &str) -> &'static str {
    match agent {
        "claude" => "claude",
        "codex" => "codex",
        "grok" => "grok",
        "cursor" => "cursor",
        "opencode" => "opencode",
        "pi" => "pi",
        "kimi" => "kimi",
        "gemini" => "gemini",
        "antigravity" => "antigravity",
        "qoder" => "qoder",
        "openclaw" => "openclaw",
        "workbuddy" => "workbuddy",
        "zcode" => "zcode",
        _ => "codex",
    }
}

pub fn display_agent_name(agent: &str) -> String {
    match agent {
        "claude" => "Claude Code".into(),
        "codex" => "Codex".into(),
        "grok" => "Grok Build".into(),
        "cursor" => "Cursor".into(),
        "opencode" => "OpenCode".into(),
        "pi" => "Pi".into(),
        "kimi" => "Kimi Code".into(),
        "gemini" => "Gemini CLI".into(),
        "antigravity" => "Antigravity CLI".into(),
        "qoder" => "Qoder CLI".into(),
        "openclaw" => "OpenClaw".into(),
        "workbuddy" => "WorkBuddy".into(),
        "zcode" => "ZCode".into(),
        other => other.to_string(),
    }
}

pub fn agent_brand_id(agent: &str) -> &'static str {
    brand_id_for_agent(agent)
}

/// Scan disk and rebuild the persisted session index. Merges user_data flags.
pub fn refresh_and_load(store: &Store) -> Result<Vec<IndexedSession>, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法解析用户主目录".to_string())?;
    let home_str = home.to_string_lossy().to_string();

    let clients: Vec<String> = ClientId::ALL
        .iter()
        .map(|c| c.as_str().to_string())
        .collect();
    let scan = scan_all_clients(&home_str, &clients);

    let mut by_key: HashMap<String, SessionIndexRow> = HashMap::new();

    for client in ClientId::ALL {
        let files = &scan.files[client as usize];
        let agent = normalize_agent_id(client.as_str());
        for path in files {
            if !path.is_file() {
                continue;
            }
            if should_skip_index_path(&agent, path) {
                continue;
            }
            let session_id = session_id_from_path(path);
            let key = format!("{agent}:{session_id}");
            let updated = mtime_ms(path);
            let (title, project_name, created_at, updated_at, message_count) =
                if agent == "grok" {
                    let side = read_grok_summary(path);
                    (
                        side.title
                            .or_else(|| peek_title(path))
                            .and_then(|t| clean_title_candidate(&t))
                            .unwrap_or_else(|| session_id.clone()),
                        side.project_name.or_else(|| project_from_path(path, &agent)),
                        side.created_at.or(updated),
                        side.updated_at.or(updated),
                        side.message_count.unwrap_or_else(|| count_jsonl_lines(path)),
                    )
                } else if agent == "kimi" {
                    let state_title = read_kimi_state_title(path);
                    (
                        state_title
                            .or_else(|| peek_title(path))
                            .and_then(|t| clean_title_candidate(&t))
                            .unwrap_or_else(|| session_id.clone()),
                        project_from_path(path, &agent),
                        updated,
                        updated,
                        count_jsonl_lines(path),
                    )
                } else {
                    (
                        resolve_title(&agent, path, &session_id),
                        project_from_path(path, &agent),
                        updated,
                        updated,
                        count_jsonl_lines(path),
                    )
                };
            let row = SessionIndexRow {
                session_key: key.clone(),
                agent_id: agent.clone(),
                session_id,
                title: Some(title),
                project_name,
                source_path: Some(path.to_string_lossy().to_string()),
                created_at,
                updated_at,
                message_count,
                tokens: 0,
            };
            by_key
                .entry(key)
                .and_modify(|existing| {
                    if updated_at.unwrap_or(0) > existing.updated_at.unwrap_or(0) {
                        *existing = row.clone();
                    }
                })
                .or_insert(row);
        }
    }

    // OpenCode 1.2+: sessions live in SQLite, not per-message JSON files.
    for db_path in &scan.opencode_dbs {
        for row in list_opencode_sessions(db_path) {
            let key = row.session_key.clone();
            by_key
                .entry(key)
                .and_modify(|existing| {
                    if row.updated_at.unwrap_or(0) > existing.updated_at.unwrap_or(0) {
                        *existing = row.clone();
                    }
                })
                .or_insert(row);
        }
    }

    // Cursor CLI agent transcripts (not usage CSV).
    for row in list_cursor_sessions(&home) {
        let key = row.session_key.clone();
        by_key.entry(key).or_insert(row);
    }

    // Qoder CLI — not a tokens-core ClientId; walk ~/.qoder/projects directly.
    for row in list_qoder_sessions(&home) {
        let key = row.session_key.clone();
        by_key.entry(key).or_insert(row);
    }

    // ZCode: ~/.zcode/cli/db/db.sqlite (Wake).
    if let Some(db) = &scan.zcode_db {
        for row in list_zcode_sessions(db) {
            let key = row.session_key.clone();
            by_key.entry(key).or_insert(row);
        }
    } else {
        let fallback = home.join(".zcode").join("cli").join("db").join("db.sqlite");
        if fallback.is_file() {
            for row in list_zcode_sessions(&fallback) {
                let key = row.session_key.clone();
                by_key.entry(key).or_insert(row);
            }
        }
    }

    // Enrich with legacy Codex/Claude scanner (titles / resume metadata).
    let codex_roots = vec![
        home.join(".codex").join("sessions"),
        home.join(".codex").join("archived_sessions"),
    ];
    let claude_root = home.join(".claude").join("projects");
    for meta in sessions::scan_sessions(&codex_roots, &claude_root) {
        let agent = normalize_agent_id(&meta.provider_id);
        let key = format!("{agent}:{}", meta.session_id);
        let row = SessionIndexRow {
            session_key: key.clone(),
            agent_id: agent,
            session_id: meta.session_id,
            title: meta
                .title
                .or(meta.summary)
                .filter(|s| !s.trim().is_empty()),
            project_name: meta.project_dir,
            source_path: meta.source_path,
            created_at: meta.created_at,
            updated_at: meta.last_active_at.or(meta.created_at),
            message_count: 0,
            tokens: 0,
        };
        by_key
            .entry(key)
            .and_modify(|existing| {
                if existing.title.as_ref().map(|t| t.len()).unwrap_or(0)
                    < row.title.as_ref().map(|t| t.len()).unwrap_or(0)
                {
                    existing.title = row.title.clone();
                }
                if existing.source_path.is_none() {
                    existing.source_path = row.source_path.clone();
                }
                if row.updated_at.unwrap_or(0) > existing.updated_at.unwrap_or(0) {
                    existing.updated_at = row.updated_at;
                }
                if existing.project_name.is_none() {
                    existing.project_name = row.project_name.clone();
                }
            })
            .or_insert(row);
    }

    let mut rows: Vec<SessionIndexRow> = by_key.into_values().collect();
    rows.sort_by(|a, b| {
        b.updated_at
            .unwrap_or(0)
            .cmp(&a.updated_at.unwrap_or(0))
            .then_with(|| a.session_key.cmp(&b.session_key))
    });

    store
        .replace_session_index(&rows)
        .map_err(|e| e.to_string())?;

    load_indexed(store)
}

pub fn load_indexed(store: &Store) -> Result<Vec<IndexedSession>, String> {
    let rows = store.list_session_index().map_err(|e| e.to_string())?;
    let flags: HashMap<String, SessionUserData> = store
        .list_session_user_data()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    Ok(rows
        .into_iter()
        .map(|r| {
            let ud = flags.get(&r.session_key).cloned().unwrap_or_default();
            IndexedSession {
                key: r.session_key,
                agent_id: r.agent_id,
                session_id: r.session_id,
                title: r
                    .title
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| "Untitled".into()),
                project_name: r.project_name,
                source_path: r.source_path,
                created_at: r.created_at,
                updated_at: r.updated_at,
                message_count: r.message_count,
                tokens: r.tokens,
                favorite: ud.favorite,
                pinned: ud.pinned,
            }
        })
        .collect())
}

pub fn filter_sessions(
    sessions: &[IndexedSession],
    filter: &SessionFilter,
    sort: SessionSort,
) -> Vec<IndexedSession> {
    let q = filter.query.trim().to_lowercase();
    let mut out: Vec<IndexedSession> = sessions
        .iter()
        .filter(|s| match filter.scope {
            SessionScope::All => true,
            SessionScope::Starred => s.favorite,
            SessionScope::Agent => filter
                .agent_id
                .as_deref()
                .map(|a| s.agent_id == a)
                .unwrap_or(true),
            SessionScope::Project => filter
                .project_name
                .as_deref()
                .map(|p| s.project_name.as_deref() == Some(p))
                .unwrap_or(true),
        })
        .filter(|s| {
            if q.is_empty() {
                return true;
            }
            s.title.to_lowercase().contains(&q)
                || s.session_id.to_lowercase().contains(&q)
                || s.project_name
                    .as_deref()
                    .map(|p| p.to_lowercase().contains(&q))
                    .unwrap_or(false)
                || s.agent_id.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        // Pinned float to top (Wake behavior).
        b.pinned.cmp(&a.pinned).then_with(|| {
            let ord = match sort.key {
                SessionSortKey::Updated => a
                    .updated_at
                    .unwrap_or(0)
                    .cmp(&b.updated_at.unwrap_or(0)),
                SessionSortKey::Created => a
                    .created_at
                    .unwrap_or(0)
                    .cmp(&b.created_at.unwrap_or(0)),
                SessionSortKey::Messages => a.message_count.cmp(&b.message_count),
            };
            if sort.ascending {
                ord
            } else {
                ord.reverse()
            }
        })
    });
    out
}

pub fn agent_counts(sessions: &[IndexedSession]) -> Vec<(String, i64)> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for s in sessions {
        *map.entry(s.agent_id.clone()).or_default() += 1;
    }
    let mut ordered: Vec<(String, i64)> = SIDEBAR_AGENTS
        .iter()
        .filter_map(|id| map.get(*id).map(|n| (id.to_string(), *n)))
        .collect();
    for (id, n) in map {
        if !SIDEBAR_AGENTS.contains(&id.as_str())
            && !HIDDEN_SIDEBAR_AGENTS.contains(&id.as_str())
            && n > 0
        {
            ordered.push((id, n));
        }
    }
    ordered
}

pub fn project_counts(sessions: &[IndexedSession]) -> Vec<(String, i64)> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for s in sessions {
        if let Some(p) = &s.project_name {
            if !p.is_empty() {
                *map.entry(p.clone()).or_default() += 1;
            }
        }
    }
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

pub fn set_favorite(store: &Store, key: &str, favorite: bool) -> Result<(), String> {
    store
        .set_session_favorite(key, favorite)
        .map_err(|e| e.to_string())
}

pub fn set_pinned(store: &Store, key: &str, pinned: bool) -> Result<(), String> {
    store
        .set_session_pinned(key, pinned)
        .map_err(|e| e.to_string())
}

/// Load transcript messages for a session (Codex/Claude via dedicated parsers;
/// other agents via generic JSONL role/content extraction).
pub fn load_transcript(session: &IndexedSession) -> Result<Vec<SessionMessage>, String> {
    let Some(path) = session.source_path.as_deref() else {
        return Err("会话缺少源文件路径".into());
    };
    let path_buf = PathBuf::from(path);
    if !path_buf.is_file() {
        return Err(format!("会话文件不存在: {path}"));
    }

    match session.agent_id.as_str() {
        "codex" | "claude" => {
            let home = dirs::home_dir().ok_or_else(|| "无法解析用户主目录".to_string())?;
            let codex_roots = vec![
                home.join(".codex").join("sessions"),
                home.join(".codex").join("archived_sessions"),
            ];
            let claude_root = home.join(".claude").join("projects");
            let provider = if session.agent_id == "claude" {
                "claude"
            } else {
                "codex"
            };
            sessions::load_messages(provider, path, &codex_roots, &claude_root)
        }
        "grok" => load_grok_transcript(&path_buf),
        "opencode" => load_opencode_transcript(session),
        "zcode" => load_zcode_transcript(session),
        "kimi" => load_kimi_transcript(&path_buf),
        _ => load_generic_jsonl(&path_buf),
    }
}

fn load_zcode_transcript(session: &IndexedSession) -> Result<Vec<SessionMessage>, String> {
    let Some(db_path) = session.source_path.as_deref() else {
        return Err("ZCode 会话缺少数据库路径".into());
    };
    let Some(conn) = open_sqlite_ro(Path::new(db_path)) else {
        return Err(format!("无法打开 ZCode 数据库: {db_path}"));
    };
    let sid = &session.session_id;
    let mut out = Vec::new();
    let Ok(mut stmt) = conn.prepare(
        "SELECT m.data, p.data
         FROM part p
         JOIN message m ON m.id = p.message_id
         WHERE p.session_id = ?1
         ORDER BY p.sequence
         LIMIT 800",
    ) else {
        return Ok(out);
    };
    let rows = stmt.query_map([sid], |row| {
        let msg: String = row.get(0)?;
        let part: String = row.get(1)?;
        Ok((msg, part))
    });
    let Ok(rows) = rows else {
        return Ok(out);
    };
    for (msg_data, part_data) in rows.flatten() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&msg_data) else {
            continue;
        };
        let Ok(part) = serde_json::from_str::<serde_json::Value>(&part_data) else {
            continue;
        };
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "user" && role != "assistant" {
            continue;
        }
        let ty = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if ty != "text" {
            continue;
        }
        let Some(text) = part.get("text").and_then(|t| t.as_str()) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        out.push(SessionMessage {
            role: role.into(),
            content: text.to_string(),
            ts: None,
        });
        if out.len() >= 500 {
            break;
        }
    }
    Ok(out)
}

fn load_opencode_transcript(session: &IndexedSession) -> Result<Vec<SessionMessage>, String> {
    let Some(db_path) = session.source_path.as_deref() else {
        return Err("OpenCode 会话缺少数据库路径".into());
    };
    let Some(conn) = open_sqlite_ro(Path::new(db_path)) else {
        return Err(format!("无法打开 OpenCode 数据库: {db_path}"));
    };
    let sid = &session.session_id;
    // Prefer v2 session_message; fall back to message.data JSON blobs.
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT type, data FROM session_message WHERE session_id = ?1 ORDER BY seq LIMIT 500",
    ) {
        let rows = stmt.query_map([sid], |row| {
            let ty: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((ty, data))
        });
        if let Ok(rows) = rows {
            for (ty, data) in rows.flatten() {
                let role = match ty.as_str() {
                    "user" => "user",
                    "assistant" => "assistant",
                    _ => continue,
                };
                let content = serde_json::from_str::<serde_json::Value>(&data)
                    .ok()
                    .map(|v| extract_text(&v))
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| clean_title_candidate(&data));
                if let Some(content) = content {
                    out.push(SessionMessage {
                        role: role.into(),
                        content,
                        ts: None,
                    });
                }
            }
        }
    }
    if !out.is_empty() {
        return Ok(out);
    }
    if let Ok(mut stmt) =
        conn.prepare("SELECT data FROM message WHERE session_id = ?1 ORDER BY id LIMIT 500")
    {
        let rows = stmt.query_map([sid], |row| row.get::<_, String>(0));
        if let Ok(rows) = rows {
            for data in rows.flatten() {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) else {
                    continue;
                };
                let role = v
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string();
                if role != "user" && role != "assistant" {
                    continue;
                }
                let content = extract_text(&v);
                if content.trim().is_empty() {
                    continue;
                }
                out.push(SessionMessage {
                    role,
                    content,
                    ts: None,
                });
            }
        }
    }
    Ok(out)
}

fn load_kimi_transcript(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("打开会话失败: {e}"))?;
    let mut out = Vec::new();
    for line in BufReader::new(file).lines().flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("turn.prompt") | Some("turn.steer") => {
                let content = extract_text(v.get("input").unwrap_or(&serde_json::Value::Null));
                if !content.trim().is_empty() {
                    out.push(SessionMessage {
                        role: "user".into(),
                        content,
                        ts: None,
                    });
                }
            }
            Some("context.append_message") => {
                let Some(msg) = v.get("message") else {
                    continue;
                };
                if msg.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                    continue;
                }
                let content = extract_text(msg.get("content").unwrap_or(&serde_json::Value::Null));
                if !content.trim().is_empty() {
                    out.push(SessionMessage {
                        role: "assistant".into(),
                        content,
                        ts: None,
                    });
                }
            }
            _ => {}
        }
        if out.len() >= 500 {
            break;
        }
    }
    Ok(out)
}

/// Parse Grok ACP `updates.jsonl` into chat bubbles (Wake-compatible).
fn load_grok_transcript(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("打开会话失败: {e}"))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    let mut cur_role: Option<&'static str> = None;
    let mut cur_text = String::new();

    let flush = |role: &mut Option<&'static str>,
                 text: &mut String,
                 out: &mut Vec<SessionMessage>| {
        if let Some(r) = role.take() {
            let content = std::mem::take(text);
            if !content.trim().is_empty() {
                out.push(SessionMessage {
                    role: r.to_string(),
                    content,
                    ts: None,
                });
            } else {
                text.clear();
            }
        }
    };

    for line in reader.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(update) = v.pointer("/params/update") else {
            continue;
        };
        let Some(kind) = update.get("sessionUpdate").and_then(|x| x.as_str()) else {
            continue;
        };
        let chunk = update
            .pointer("/content/text")
            .or_else(|| update.get("text"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        match kind {
            "user_message_chunk" => {
                if cur_role != Some("user") {
                    flush(&mut cur_role, &mut cur_text, &mut out);
                    cur_role = Some("user");
                }
                cur_text.push_str(chunk);
            }
            "agent_message_chunk" => {
                if cur_role != Some("assistant") {
                    flush(&mut cur_role, &mut cur_text, &mut out);
                    cur_role = Some("assistant");
                }
                cur_text.push_str(chunk);
            }
            "agent_thought_chunk" | "tool_call" | "tool_call_update" => {
                // Keep assistant turn open; thoughts/tools omitted from simple bubbles.
                if cur_role != Some("assistant") {
                    flush(&mut cur_role, &mut cur_text, &mut out);
                    cur_role = Some("assistant");
                }
            }
            _ => {}
        }
        if out.len() >= 500 {
            break;
        }
    }
    flush(&mut cur_role, &mut cur_text, &mut out);
    Ok(out)
}

fn load_generic_jsonl(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("打开会话失败: {e}"))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines().flatten() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let role = v
            .get("role")
            .or_else(|| v.pointer("/message/role"))
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| {
                match v.get("type").and_then(|t| t.as_str()) {
                    Some("user") | Some("human") => "user",
                    Some("assistant") | Some("ai") => "assistant",
                    Some("system") => "system",
                    _ => "",
                }
            });
        if role.is_empty() {
            continue;
        }
        let content = extract_text(&v);
        if content.trim().is_empty() {
            continue;
        }
        out.push(SessionMessage {
            role: role.to_string(),
            content,
            ts: None,
        });
        if out.len() >= 500 {
            break;
        }
    }
    Ok(out)
}

fn extract_text(v: &serde_json::Value) -> String {
    if let Some(s) = v.get("content").and_then(|c| c.as_str()) {
        return s.to_string();
    }
    if let Some(s) = v.pointer("/message/content").and_then(|c| c.as_str()) {
        return s.to_string();
    }
    if let Some(arr) = v
        .get("content")
        .or_else(|| v.pointer("/message/content"))
        .and_then(|c| c.as_array())
    {
        let mut parts = Vec::new();
        for item in arr {
            if let Some(t) = item.get("text").and_then(|x| x.as_str()) {
                parts.push(t);
            } else if let Some(t) = item.as_str() {
                parts.push(t);
            }
        }
        return parts.join("\n");
    }
    if let Some(s) = v.get("text").and_then(|x| x.as_str()) {
        return s.to_string();
    }
    String::new()
}

pub fn export_markdown(session: &IndexedSession, messages: &[SessionMessage]) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", session.title));
    md.push_str(&format!(
        "- Agent: {}\n- Session: `{}`\n",
        display_agent_name(&session.agent_id),
        session.session_id
    ));
    if let Some(p) = &session.project_name {
        md.push_str(&format!("- Project: `{p}`\n"));
    }
    if let Some(path) = &session.source_path {
        md.push_str(&format!("- Path: `{path}`\n"));
    }
    md.push('\n');
    for m in messages {
        md.push_str(&format!("## {}\n\n{}\n\n", m.role, m.content));
    }
    let _ = now_ms();
    md
}

/// Convert indexed session into legacy [`SessionMeta`] for resume helpers.
pub fn to_session_meta(s: &IndexedSession) -> SessionMeta {
    SessionMeta {
        provider_id: if s.agent_id == "claude" {
            "claude".into()
        } else if s.agent_id == "codex" {
            "codex".into()
        } else {
            s.agent_id.clone()
        },
        session_id: s.session_id.clone(),
        title: Some(s.title.clone()),
        summary: None,
        project_dir: s.project_name.clone(),
        created_at: s.created_at,
        last_active_at: s.updated_at,
        source_path: s.source_path.clone(),
        resume_command: match s.agent_id.as_str() {
            "claude" => Some(format!("claude --resume {}", s.session_id)),
            "codex" => Some(format!("codex resume {}", s.session_id)),
            _ => None,
        },
    }
}

/// Wake-style "Open In" targets (macOS-focused; others fall back to copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenTarget {
    Terminal,
    ITerm,
    Ghostty,
    ClaudeDesktop,
}

impl OpenTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::ITerm => "iTerm",
            Self::Ghostty => "Ghostty",
            Self::ClaudeDesktop => "Claude Desktop",
        }
    }

    /// Stable short id (icon cache filename / last-used prefs).
    pub fn id(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::ITerm => "iterm",
            Self::Ghostty => "ghostty",
            Self::ClaudeDesktop => "claude-desktop",
        }
    }

    /// Embedded brand fallback when .app icon extract is unavailable.
    pub fn brand_icon(self) -> Option<&'static str> {
        match self {
            Self::ClaudeDesktop => Some("brands/claude-code.png"),
            _ => None,
        }
    }

    pub fn resolved_app_path(self) -> Option<std::path::PathBuf> {
        let candidates: &[&str] = match self {
            Self::Terminal => &["/System/Applications/Utilities/Terminal.app"],
            Self::ITerm => &["/Applications/iTerm.app"],
            Self::Ghostty => &["/Applications/Ghostty.app"],
            Self::ClaudeDesktop => &["/Applications/Claude.app"],
        };
        let home_apps = dirs::home_dir().unwrap_or_default().join("Applications");
        for c in candidates {
            let sys = std::path::PathBuf::from(c);
            if sys.is_dir() {
                return Some(sys);
            }
            if let Some(name) = std::path::Path::new(c).file_name() {
                let alt = home_apps.join(name);
                if alt.is_dir() {
                    return Some(alt);
                }
            }
        }
        None
    }

    pub fn is_installed(self) -> bool {
        self.resolved_app_path().is_some()
    }
}

/// Extract macOS .app icons into `cache_dir/{id}.png` (Wake-compatible pipeline).
pub fn ensure_open_target_icons(
    cache_dir: &std::path::Path,
) -> std::collections::HashMap<String, std::path::PathBuf> {
    use std::collections::HashMap;
    use std::process::Command;

    let _ = std::fs::create_dir_all(cache_dir);
    let mut out = HashMap::new();
    let mut jobs: Vec<(OpenTarget, std::path::PathBuf)> = Vec::new();
    for t in [
        OpenTarget::Terminal,
        OpenTarget::ITerm,
        OpenTarget::Ghostty,
        OpenTarget::ClaudeDesktop,
    ] {
        if !t.is_installed() {
            continue;
        }
        let png = cache_dir.join(format!("{}.png", t.id()));
        if png.is_file() {
            out.insert(t.id().to_string(), png);
        } else {
            jobs.push((t, png));
        }
    }
    if jobs.is_empty() {
        return out;
    }

    #[cfg(target_os = "macos")]
    {
        let mut script =
            String::from("ObjC.import('AppKit');\nconst ws = $.NSWorkspace.sharedWorkspace;\n");
        for (t, png) in &jobs {
            let Some(app) = t.resolved_app_path() else {
                continue;
            };
            script.push_str(&format!(
                "{{ const i = ws.iconForFile('{app}'); const rep = $.NSBitmapImageRep.imageRepWithData(i.TIFFRepresentation); \
                 const png = rep.representationUsingTypeProperties(4, $.NSDictionary.dictionary); \
                 png.writeToFileAtomically('{out}', true); }}\n",
                app = app.display(),
                out = png.display(),
            ));
        }
        let ok = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", &script])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            for (t, png) in &jobs {
                if png.is_file() {
                    let _ = Command::new("sips").args(["-Z", "64"]).arg(png).output();
                    out.insert(t.id().to_string(), png.clone());
                }
            }
        }
    }
    out
}

/// Resume shell command + optional cwd for Open In.
pub fn resume_shell_command(session: &IndexedSession) -> Option<(String, Option<String>)> {
    let id = &session.session_id;
    if id.is_empty() {
        return None;
    }
    let cwd = session
        .project_name
        .as_ref()
        .filter(|p| p.starts_with('/'))
        .cloned();
    let cmd = match session.agent_id.as_str() {
        "claude" => format!("claude --resume {id}"),
        "codex" => format!("codex resume {id}"),
        "opencode" => format!("opencode --session {id}"),
        "grok" => format!("grok --resume {id}"),
        "cursor" => format!("cursor-agent --resume {id}"),
        "pi" => format!("pi --session {id}"),
        "kimi" => format!("kimi --session {id}"),
        "qoder" => format!("qoder --resume {id}"),
        _ => return None,
    };
    Some((cmd, cwd))
}

pub fn open_targets_for(agent_id: &str) -> Vec<OpenTarget> {
    // Order matches Wake: Terminal → desktop hosts → other terminals.
    let mut out = Vec::new();
    for t in [
        OpenTarget::Terminal,
        OpenTarget::ClaudeDesktop,
        OpenTarget::ITerm,
        OpenTarget::Ghostty,
    ] {
        if !t.is_installed() {
            continue;
        }
        if t == OpenTarget::ClaudeDesktop && agent_id != "claude" {
            continue;
        }
        out.push(t);
    }
    out
}

/// Launch resume in the chosen app. Returns the shell command string for the toast.
pub fn open_session_in(
    session: &IndexedSession,
    target: OpenTarget,
) -> Result<String, String> {
    if target == OpenTarget::ClaudeDesktop {
        if session.agent_id != "claude" {
            return Err("Claude Desktop 仅支持 Claude Code 会话".into());
        }
        let url = format!("claude://resume?session={}", session.session_id);
        let status = std::process::Command::new("open")
            .arg(&url)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("打开 Claude Desktop 失败".into());
        }
        return Ok(url);
    }

    let (cmd, cwd) = resume_shell_command(session)
        .ok_or_else(|| format!("暂不支持打开 {}", display_agent_name(&session.agent_id)))?;
    let full = match &cwd {
        Some(dir) => format!("cd {} && {}", sh_quote(dir), cmd),
        None => cmd.clone(),
    };

    match target {
        OpenTarget::Terminal => launch_terminal(&full)?,
        OpenTarget::ITerm => launch_iterm(&full)?,
        OpenTarget::Ghostty => launch_ghostty(&full)?,
        OpenTarget::ClaudeDesktop => unreachable!(),
    }
    Ok(full)
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn applescript_quote(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}

fn launch_terminal(command: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let esc = applescript_quote(command);
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"Terminal\" to activate",
                "-e",
                &format!("tell application \"Terminal\" to do script \"{esc}\""),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = command;
        Err("当前平台请手动在终端运行恢复命令".into())
    }
}

fn launch_iterm(command: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let esc = applescript_quote(command);
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"iTerm\" to activate",
                "-e",
                "tell application \"iTerm\" to create window with default profile",
                "-e",
                &format!(
                    "tell current session of current window of application \"iTerm\" to write text \"{esc}\""
                ),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = command;
        Err("iTerm 仅支持 macOS".into())
    }
}

fn launch_ghostty(command: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .args([
                "-na",
                "Ghostty",
                "--args",
                "-e",
                "/bin/zsh",
                "-lic",
                &format!("{command}; exec /bin/zsh -il"),
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("打开 Ghostty 失败".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = command;
        Err("Ghostty 仅支持 macOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn grok_session_id_uses_parent_uuid_not_updates_stem() {
        let path = PathBuf::from(
            "/Users/wayne/.grok/sessions/%2Ftmp/01a0be35-e908-76e0-958b-1f1ab9051fce/updates.jsonl",
        );
        assert_eq!(
            session_id_from_path(&path),
            "01a0be35-e908-76e0-958b-1f1ab9051fce"
        );
    }

    #[test]
    fn kimi_wire_session_id_uses_session_dir() {
        let path = PathBuf::from(
            "/Users/wayne/.kimi-code/sessions/wd_wayne_abc/session_eb5f6b31-fd01-4812-8fba-2637b351c8cf/agents/main/wire.jsonl",
        );
        assert_eq!(
            session_id_from_path(&path),
            "eb5f6b31-fd01-4812-8fba-2637b351c8cf"
        );
    }

    #[test]
    fn clean_title_strips_openclaw_sender_metadata() {
        let raw = "Sender (untrusted metadata):\n```json\n{\n  \"label\": \"openclaw-control-ui\"\n}\n```\n\n[Fri 2026-03-13] 你是谁";
        let cleaned = clean_title_candidate(raw).unwrap();
        assert!(cleaned.contains("你是谁"), "{cleaned}");
        assert!(!cleaned.contains("label"), "{cleaned}");
    }

    #[test]
    fn clean_title_extracts_cursor_user_query() {
        let raw = "<timestamp>Sat</timestamp>\n<user_query>\n修复标题省略号\n</user_query>";
        let cleaned = clean_title_candidate(raw).unwrap();
        assert!(cleaned.contains("修复标题省略号"), "{cleaned}");
    }

    #[test]
    fn clean_title_rejects_workbuddy_system_reminder() {
        let raw = "<system-reminder data-role=\"user-context\">\nOS Version: darwin\n</system-reminder>\nhello";
        assert!(clean_title_candidate(raw).is_none());
    }

    #[test]
    fn openclaw_project_uses_agent_not_sessions_dir() {
        let path = PathBuf::from(
            "/tmp/does-not-exist/.openclaw/agents/main/sessions/1e53e968-a893-4c58-8525-c7450479cce1.jsonl",
        );
        // No readable cwd → fall back to agent id "main", never "sessions".
        assert_eq!(
            project_from_path(&path, "openclaw").as_deref(),
            Some("main")
        );
    }
}
