//! 会话管理: 扫描/解析/删除/预览各 CLI 工具的本地会话(移植自 cc-switch)。
//!
//! V1 支持 Codex(~/.codex/sessions + archived_sessions 的 rollout JSONL)与
//! Claude Code(~/.claude/projects/**/*.jsonl)。所有路径由调用方注入, 便于测试。

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;

pub const TITLE_MAX_CHARS: usize = 80;
const VSCODE_CONTEXT_PREFIX: &str = "# Context from my IDE setup:";
const CODEX_REQUEST_MARKER: &str = "my request for codex";

/// 会话元信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    pub provider_id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_command: Option<String>,
}

/// 会话内一条消息
#[derive(Debug, Clone, Serialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

/// 批量删除请求
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteSessionRequest {
    pub provider_id: String,
    pub session_id: String,
    pub source_path: String,
}

/// 批量删除结果
#[derive(Debug, Clone, Serialize)]
pub struct DeleteSessionOutcome {
    pub provider_id: String,
    pub session_id: String,
    pub source_path: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 扫描全部受支持 provider 的会话, 按最近活跃降序
pub fn scan_sessions(codex_roots: &[PathBuf], claude_root: &Path) -> Vec<SessionMeta> {
    let thread_titles = load_codex_thread_titles(&codex_roots.first().and_then(|root| {
        root.parent()
            .map(|parent| parent.join("session_index.jsonl"))
    }));

    let mut sessions = Vec::new();
    sessions.extend(scan_codex(codex_roots, &thread_titles));
    sessions.extend(scan_claude(claude_root));
    sessions.sort_by(|a, b| {
        let a_ts = a.last_active_at.or(a.created_at).unwrap_or(0);
        let b_ts = b.last_active_at.or(b.created_at).unwrap_or(0);
        b_ts.cmp(&a_ts)
    });
    sessions
}

/// 加载会话消息预览
pub fn load_messages(
    provider_id: &str,
    source_path: &str,
    codex_roots: &[PathBuf],
    claude_root: &Path,
) -> Result<Vec<SessionMessage>, String> {
    let path = validated_source_path(provider_id, source_path, codex_roots, claude_root)?;
    match provider_id {
        "codex" => codex_load_messages(&path),
        "claude" => claude_load_messages(&path),
        other => Err(format!("Unsupported provider: {other}")),
    }
}

/// 删除单个会话(source_path 必须位于对应 provider 根目录下)
pub fn delete_session(
    provider_id: &str,
    session_id: &str,
    source_path: &str,
    codex_roots: &[PathBuf],
    claude_root: &Path,
) -> Result<bool, String> {
    let path = validated_source_path(provider_id, source_path, codex_roots, claude_root)?;
    match provider_id {
        "codex" => codex_delete(&path, session_id),
        "claude" => claude_delete(&path, session_id),
        other => Err(format!("Unsupported provider: {other}")),
    }
}

/// 批量删除
pub fn delete_sessions(
    requests: &[DeleteSessionRequest],
    codex_roots: &[PathBuf],
    claude_root: &Path,
) -> Vec<DeleteSessionOutcome> {
    requests
        .iter()
        .map(|request| {
            match delete_session(
                &request.provider_id,
                &request.session_id,
                &request.source_path,
                codex_roots,
                claude_root,
            ) {
                Ok(true) => DeleteSessionOutcome {
                    provider_id: request.provider_id.clone(),
                    session_id: request.session_id.clone(),
                    source_path: request.source_path.clone(),
                    success: true,
                    error: None,
                },
                Ok(false) => DeleteSessionOutcome {
                    provider_id: request.provider_id.clone(),
                    session_id: request.session_id.clone(),
                    source_path: request.source_path.clone(),
                    success: false,
                    error: Some("Session was not deleted".to_string()),
                },
                Err(error) => DeleteSessionOutcome {
                    provider_id: request.provider_id.clone(),
                    session_id: request.session_id.clone(),
                    source_path: request.source_path.clone(),
                    success: false,
                    error: Some(error),
                },
            }
        })
        .collect()
}

/// 校验 source_path 必须位于对应 provider 的根目录内(防越权删除)
fn validated_source_path(
    provider_id: &str,
    source_path: &str,
    codex_roots: &[PathBuf],
    claude_root: &Path,
) -> Result<PathBuf, String> {
    let source = Path::new(source_path);
    let validated_source = canonicalize_existing(source, "session source")?;
    let roots: Vec<PathBuf> = match provider_id {
        "codex" => codex_roots.to_vec(),
        "claude" => vec![claude_root.to_path_buf()],
        other => return Err(format!("Unsupported provider: {other}")),
    };
    let roots_display = roots
        .first()
        .map(|root| root.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());

    let mut saw_existing_root = false;
    for root in roots {
        if !root.exists() {
            continue;
        }
        saw_existing_root = true;
        let validated_root = canonicalize_existing(&root, "session root")?;
        if validated_source.starts_with(&validated_root) {
            return Ok(validated_source);
        }
    }

    if !saw_existing_root {
        return Err(format!(
            "Session root not found for provider {provider_id}: {roots_display}"
        ));
    }
    Err(format!(
        "Session source path is outside provider roots: {source_path}"
    ))
}

fn canonicalize_existing(path: &Path, label: &str) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err(format!("{label} not found: {}", path.display()));
    }
    path.canonicalize()
        .map_err(|e| format!("Failed to resolve {label} {}: {e}", path.display()))
}

// ==================== 通用工具 ====================

/// 读文件头 head_n 行 + 尾 tail_n 行(小文件整读, 大文件 seek 到尾部 16KB)
fn read_head_tail_lines(
    path: &Path,
    head_n: usize,
    tail_n: usize,
) -> std::io::Result<(Vec<String>, Vec<String>)> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();

    if file_len < 16_384 {
        let reader = BufReader::new(file);
        let all: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let head = all.iter().take(head_n).cloned().collect();
        let skip = all.len().saturating_sub(tail_n);
        let tail = all.into_iter().skip(skip).collect();
        return Ok((head, tail));
    }

    let reader = BufReader::new(file);
    let head: Vec<String> = reader.lines().take(head_n).map_while(Result::ok).collect();

    let seek_pos = file_len.saturating_sub(16_384);
    let mut file2 = File::open(path)?;
    file2.seek(SeekFrom::Start(seek_pos))?;
    let tail_reader = BufReader::new(file2);
    let all_tail: Vec<String> = tail_reader.lines().map_while(Result::ok).collect();
    let skip_first = if seek_pos > 0 { 1 } else { 0 };
    let usable: Vec<String> = all_tail.into_iter().skip(skip_first).collect();
    let skip = usable.len().saturating_sub(tail_n);
    let tail = usable.into_iter().skip(skip).collect();

    Ok((head, tail))
}

/// 时间戳: 毫秒整数 / 秒整数 / RFC3339 字符串 → 毫秒
fn parse_timestamp_to_ms(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    if let Some(n) = value.as_f64() {
        let n = n as i64;
        return Some(if n > 1_000_000_000_000 { n } else { n * 1000 });
    }
    let raw = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// 从 content(string | 数组 | 对象) 提取可读文本
fn extract_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(extract_text_from_item)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => map
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn extract_text_from_item(item: &Value) -> Option<String> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
    if matches!(item_type, "tool_use" | "toolCall") {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Some(format!("[Tool: {name}]"));
    }
    if item_type == "tool_result" {
        if let Some(content) = item.get("content") {
            let text = extract_text(content);
            if !text.is_empty() {
                return Some(text);
            }
        }
        return None;
    }
    for key in ["text", "input_text", "output_text"] {
        if let Some(text) = item.get(key).and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
    }
    if let Some(content) = item.get("content") {
        let text = extract_text(content);
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

fn truncate_summary(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut result = trimmed.chars().take(max_chars).collect::<String>();
    result.push_str("...");
    result
}

fn path_basename(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.trim_end_matches(['/', '\\']);
    normalized
        .split(['/', '\\'])
        .next_back()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn collect_jsonl_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

/// 从文件名推断会话 UUID(8-4-4-4-12 hex)
fn infer_session_id_from_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let bytes = name.as_bytes();
    let is_hex = |c: u8| c.is_ascii_hexdigit();
    let mut index = 0usize;
    while index + 36 <= bytes.len() {
        let segment = &bytes[index..index + 36];
        let dashes = [8usize, 13, 18, 23];
        if dashes.iter().all(|&dash| segment[dash] == b'-')
            && segment
                .iter()
                .enumerate()
                .all(|(i, &c)| dashes.contains(&i) || is_hex(c))
        {
            return Some(String::from_utf8_lossy(segment).to_string());
        }
        index += 1;
    }
    None
}

// ==================== Codex ====================

#[derive(Deserialize)]
struct SessionIndexEntry {
    id: String,
    thread_name: String,
}

fn load_codex_thread_titles(
    index_path: &Option<PathBuf>,
) -> std::collections::HashMap<String, String> {
    let mut titles = std::collections::HashMap::new();
    let Some(index_path) = index_path else {
        return titles;
    };
    if !index_path.exists() {
        return titles;
    }
    let file = match File::open(index_path) {
        Ok(file) => file,
        Err(_) => return titles,
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<SessionIndexEntry>(line.trim()) else {
            continue;
        };
        let id = entry.id.trim();
        let title = entry.thread_name.trim();
        if !id.is_empty() && !title.is_empty() {
            titles.insert(id.to_string(), title.to_string());
        }
    }
    titles
}

fn scan_codex(
    roots: &[PathBuf],
    thread_titles: &std::collections::HashMap<String, String>,
) -> Vec<SessionMeta> {
    let mut files = Vec::new();
    for root in roots {
        collect_jsonl_files(root, &mut files);
    }
    files
        .iter()
        .filter_map(|path| codex_parse_session(path, thread_titles))
        .collect()
}

fn codex_load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open session file: {e}"))?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");

        let (role, content) = match payload_type {
            "message" => {
                let role = payload
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let content = payload.get("content").map(extract_text).unwrap_or_default();
                (role, content)
            }
            "function_call" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                ("assistant".to_string(), format!("[Tool: {name}]"))
            }
            "function_call_output" => {
                let output = payload
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                ("tool".to_string(), output)
            }
            _ => continue,
        };

        if content.trim().is_empty() {
            continue;
        }
        let ts = value.get("timestamp").and_then(parse_timestamp_to_ms);
        messages.push(SessionMessage { role, content, ts });
    }

    Ok(messages)
}

fn codex_delete(path: &Path, session_id: &str) -> Result<bool, String> {
    let meta = codex_parse_session(path, &std::collections::HashMap::new())
        .ok_or_else(|| format!("Failed to parse Codex session metadata: {}", path.display()))?;
    if meta.session_id != session_id {
        return Err(format!(
            "Codex session ID mismatch: expected {session_id}, found {}",
            meta.session_id
        ));
    }
    std::fs::remove_file(path).map_err(|e| {
        format!(
            "Failed to delete Codex session file {}: {e}",
            path.display()
        )
    })?;
    Ok(true)
}

fn codex_parse_session(
    path: &Path,
    thread_titles: &std::collections::HashMap<String, String>,
) -> Option<SessionMeta> {
    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;

    let mut session_id: Option<String> = None;
    let mut project_dir: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut first_user_message: Option<String> = None;

    for line in &head {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if value.get("type").and_then(Value::as_str) == Some("session_meta") {
            if let Some(payload) = value.get("payload") {
                // 子代理会话不展示
                if payload
                    .get("source")
                    .and_then(Value::as_object)
                    .is_some_and(|source| source.contains_key("subagent"))
                {
                    return None;
                }
                if session_id.is_none() {
                    session_id = payload
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                if project_dir.is_none() {
                    project_dir = payload
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                if let Some(ts) = payload.get("timestamp").and_then(parse_timestamp_to_ms) {
                    created_at.get_or_insert(ts);
                }
            }
        }
        if first_user_message.is_none()
            && value.get("type").and_then(Value::as_str) == Some("response_item")
        {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message")
                    && payload.get("role").and_then(Value::as_str) == Some("user")
                {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    if let Some(title) = codex_title_candidate(&text) {
                        first_user_message = Some(title);
                    }
                }
            }
        }
        if session_id.is_some()
            && project_dir.is_some()
            && created_at.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    let mut last_active_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    for line in tail.iter().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if last_active_at.is_none() {
            last_active_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if summary.is_none() && value.get("type").and_then(Value::as_str) == Some("response_item") {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message") {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    if !text.trim().is_empty() {
                        summary = Some(text);
                    }
                }
            }
        }
        if last_active_at.is_some() && summary.is_some() {
            break;
        }
    }

    let session_id = session_id.or_else(|| infer_session_id_from_filename(path))?;
    let title = thread_titles
        .get(&session_id)
        .map(|t| truncate_summary(t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            project_dir
                .as_deref()
                .and_then(path_basename)
                .map(|v| v.to_string())
        });
    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        provider_id: "codex".to_string(),
        session_id: session_id.clone(),
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
        resume_command: Some(format!("codex resume {session_id}")),
    })
}

fn codex_title_candidate(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("# AGENTS.md")
        || trimmed.starts_with("<environment_context>")
    {
        return None;
    }
    if trimmed.starts_with(VSCODE_CONTEXT_PREFIX) {
        return codex_extract_ide_request(trimmed);
    }
    Some(trimmed.to_string())
}

/// 从 VS Code IDE 注入的上下文里提取「My request for Codex:」真实请求
fn codex_extract_ide_request(text: &str) -> Option<String> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut prompt: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(inline_prompt) = codex_request_heading_payload(line) else {
            continue;
        };
        if !inline_prompt.is_empty() {
            prompt = Some(inline_prompt.to_string());
            continue;
        }
        let following_prompt = lines[index + 1..].join("\n").trim().to_string();
        prompt = (!following_prompt.is_empty()).then_some(following_prompt);
    }
    prompt
}

fn codex_request_heading_payload(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let heading = trimmed.trim_start_matches('#').trim_start();
    let lowered = heading.to_ascii_lowercase();
    if !lowered.starts_with(CODEX_REQUEST_MARKER) {
        return None;
    }
    let suffix = heading[CODEX_REQUEST_MARKER.len()..].trim_start();
    if suffix.is_empty() {
        return Some("");
    }
    let Some(separator) = suffix.chars().next() else {
        return Some("");
    };
    if !matches!(separator, ':' | '：' | '-' | '—') {
        return None;
    }
    Some(
        suffix
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '：' | '-' | '—'))
            .trim(),
    )
}

// ==================== Claude Code ====================

fn scan_claude(root: &Path) -> Vec<SessionMeta> {
    let mut files = Vec::new();
    collect_jsonl_files(root, &mut files);
    files
        .iter()
        .filter_map(|path| claude_parse_session(path))
        .collect()
}

fn claude_load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open session file: {e}"))?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };

        let mut role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        // Claude 把 tool_result 包在 user 消息里; 重分类为 tool
        if role == "user" {
            if let Some(Value::Array(items)) = message.get("content") {
                let all_tool_results = !items.is_empty()
                    && items.iter().all(|item| {
                        item.get("type").and_then(Value::as_str) == Some("tool_result")
                    });
                if all_tool_results {
                    role = "tool".to_string();
                }
            }
        }

        let content = message.get("content").map(extract_text).unwrap_or_default();
        if content.trim().is_empty() {
            continue;
        }
        let ts = value.get("timestamp").and_then(parse_timestamp_to_ms);
        messages.push(SessionMessage { role, content, ts });
    }

    Ok(messages)
}

fn claude_delete(path: &Path, session_id: &str) -> Result<bool, String> {
    let meta = claude_parse_session(path).ok_or_else(|| {
        format!(
            "Failed to parse Claude session metadata: {}",
            path.display()
        )
    })?;
    if meta.session_id != session_id {
        return Err(format!(
            "Claude session ID mismatch: expected {session_id}, found {}",
            meta.session_id
        ));
    }

    // 同名 sidecar 目录一并删除
    if let Some(stem) = path.file_stem() {
        let sibling = path.parent().unwrap_or_else(|| Path::new("")).join(stem);
        if sibling.is_dir() {
            std::fs::remove_dir_all(&sibling).map_err(|e| {
                format!(
                    "Failed to delete Claude session sidecar {}: {e}",
                    sibling.display()
                )
            })?;
        }
    }

    std::fs::remove_file(path).map_err(|e| {
        format!(
            "Failed to delete Claude session file {}: {e}",
            path.display()
        )
    })?;
    Ok(true)
}

fn claude_parse_session(path: &Path) -> Option<SessionMeta> {
    // 子代理会话不展示
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("agent-"))
    {
        return None;
    }

    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;

    let mut session_id: Option<String> = None;
    let mut project_dir: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut first_user_message: Option<String> = None;

    for line in &head {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if session_id.is_none() {
            session_id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if project_dir.is_none() {
            project_dir = value.get("cwd").and_then(Value::as_str).map(str::to_string);
        }
        if created_at.is_none() {
            created_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if first_user_message.is_none() {
            let is_user = value.get("type").and_then(Value::as_str) == Some("user")
                || value
                    .get("message")
                    .and_then(|m| m.get("role"))
                    .and_then(Value::as_str)
                    == Some("user");
            if is_user {
                if let Some(message) = value.get("message") {
                    let text = message.get("content").map(extract_text).unwrap_or_default();
                    let trimmed = text.trim();
                    if !trimmed.is_empty()
                        && !trimmed.contains("<local-command-caveat>")
                        && !trimmed.starts_with("<command-name>")
                    {
                        first_user_message = Some(trimmed.to_string());
                    }
                }
            }
        }
        if session_id.is_some()
            && project_dir.is_some()
            && created_at.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    let mut last_active_at: Option<i64> = None;
    let mut summary: Option<String> = None;
    let mut custom_title: Option<String> = None;
    for line in tail.iter().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if last_active_at.is_none() {
            last_active_at = value.get("timestamp").and_then(parse_timestamp_to_ms);
        }
        if custom_title.is_none()
            && value.get("type").and_then(Value::as_str) == Some("custom-title")
        {
            custom_title = value
                .get("customTitle")
                .and_then(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        }
        if summary.is_none() {
            if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            if let Some(message) = value.get("message") {
                let text = message.get("content").map(extract_text).unwrap_or_default();
                if !text.trim().is_empty() {
                    summary = Some(text);
                }
            }
        }
        if last_active_at.is_some() && summary.is_some() && custom_title.is_some() {
            break;
        }
    }

    let session_id = session_id.or_else(|| infer_session_id_from_filename(path))?;
    let title = custom_title
        .map(|t| truncate_summary(&t, TITLE_MAX_CHARS))
        .or_else(|| first_user_message.map(|t| truncate_summary(&t, TITLE_MAX_CHARS)))
        .or_else(|| {
            project_dir
                .as_deref()
                .and_then(path_basename)
                .map(|v| v.to_string())
        });
    let summary = summary.map(|text| truncate_summary(&text, 160));

    Some(SessionMeta {
        provider_id: "claude".to_string(),
        session_id: session_id.clone(),
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: Some(path.to_string_lossy().to_string()),
        resume_command: Some(format!("claude --resume {session_id}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_codex_session(path: &Path, session_id: &str, message: &str) {
        std::fs::write(
            path,
            format!(
                "{{\"timestamp\":\"2026-03-06T21:50:12Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{session_id}\",\"cwd\":\"/tmp/project\"}}}}\n\
                 {{\"timestamp\":\"2026-03-06T21:50:13Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":\"{message}\"}}}}\n"
            ),
        )
        .expect("write session");
    }

    #[test]
    fn codex_scan_includes_active_and_archived() {
        let temp = tempfile::tempdir().unwrap();
        let active = temp.path().join("sessions");
        let archived = temp.path().join("archived_sessions");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&archived).unwrap();
        write_codex_session(&active.join("active.jsonl"), "active-id", "Active session");
        write_codex_session(
            &archived.join("archived.jsonl"),
            "archived-id",
            "Archived session",
        );

        let sessions = scan_codex(&[active, archived], &Default::default());
        let ids: Vec<String> = sessions.into_iter().map(|s| s.session_id).collect();
        assert!(ids.contains(&"active-id".to_string()));
        assert!(ids.contains(&"archived-id".to_string()));
    }

    #[test]
    fn codex_title_prefers_thread_index_then_user_message() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"timestamp\":\"2026-03-06T21:50:12Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"test-id\",\"cwd\":\"/tmp/project\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:13Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"How do I deploy?\"}}\n"
            ),
        )
        .unwrap();
        assert_eq!(
            codex_parse_session(&path, &Default::default())
                .unwrap()
                .title
                .as_deref(),
            Some("How do I deploy?")
        );

        let mut titles = std::collections::HashMap::new();
        titles.insert("test-id".to_string(), "Renamed thread".to_string());
        assert_eq!(
            codex_parse_session(&path, &titles)
                .unwrap()
                .title
                .as_deref(),
            Some("Renamed thread")
        );
    }

    #[test]
    fn codex_title_skips_agents_md_and_env_context() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"timestamp\":\"2026-03-06T21:50:12Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"test-id\",\"cwd\":\"/tmp/project\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:13Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"# AGENTS.md instructions for /tmp/project\\n<INSTRUCTIONS>x</INSTRUCTIONS>\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:14Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"Fix the login bug\"}}\n"
            ),
        )
        .unwrap();
        assert_eq!(
            codex_parse_session(&path, &Default::default())
                .unwrap()
                .title
                .as_deref(),
            Some("Fix the login bug")
        );
    }

    #[test]
    fn codex_ide_context_title_extraction() {
        assert_eq!(
            codex_title_candidate(
                "# Context from my IDE setup:\n\n## My request for Codex:\nFix the session title preview"
            )
            .as_deref(),
            Some("Fix the session title preview")
        );
        assert_eq!(
            codex_title_candidate(
                "# Context from my IDE setup:\n\n## My request for Codex: Fix inline"
            )
            .as_deref(),
            Some("Fix inline")
        );
    }

    #[test]
    fn codex_subagent_sessions_are_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"timestamp\":\"2026-04-28T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"sub-id\",\"cwd\":\"/tmp\",\"source\":{\"subagent\":{\"depth\":1}}}}\n",
                "{\"timestamp\":\"2026-04-28T10:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"Inspect\"}}\n"
            ),
        )
        .unwrap();
        assert!(codex_parse_session(&path, &Default::default()).is_none());
    }

    #[test]
    fn codex_load_messages_includes_tool_flow() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"timestamp\":\"2026-03-06T21:50:12Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"test-id\",\"cwd\":\"/tmp\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:13Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"list files\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:14Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"shell\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:15Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"output\":\"file1.txt\"}}\n",
                "{\"timestamp\":\"2026-03-06T21:50:16Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Done.\"}]}}\n"
            ),
        )
        .unwrap();
        let messages = codex_load_messages(&path).unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, "user");
        assert!(messages[1].content.contains("[Tool: shell]"));
        assert_eq!(messages[2].role, "tool");
        assert_eq!(messages[3].content, "Done.");
    }

    #[test]
    fn claude_parse_and_messages() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"sessionId\":\"claude-1\",\"cwd\":\"/tmp/proj\",\"timestamp\":\"2026-03-06T21:50:12Z\",\"message\":{\"role\":\"user\",\"content\":\"Fix bug\"}}\n",
                "{\"type\":\"assistant\",\"timestamp\":\"2026-03-06T21:50:14Z\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"Fixed.\"}]}}\n"
            ),
        )
        .unwrap();
        let meta = claude_parse_session(&path).unwrap();
        assert_eq!(meta.session_id, "claude-1");
        assert_eq!(
            meta.resume_command.as_deref(),
            Some("claude --resume claude-1")
        );
        let messages = claude_load_messages(&path).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Fix bug");
    }

    #[test]
    fn delete_rejects_path_outside_roots() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = outside.path().join("session.jsonl");
        std::fs::write(&source, "{}").unwrap();
        let err = delete_session(
            "codex",
            "s1",
            &source.to_string_lossy(),
            &[root.path().to_path_buf()],
            Path::new("/nonexistent-claude"),
        )
        .unwrap_err();
        assert!(err.contains("outside provider roots"), "{err}");
    }

    #[test]
    fn uuid_inference_from_filename() {
        let path =
            Path::new("rollout-2026-03-06T21-50-12-019cc369-bd7c-7891-b371-7b20b4fe0b18.jsonl");
        assert_eq!(
            infer_session_id_from_filename(path).as_deref(),
            Some("019cc369-bd7c-7891-b371-7b20b4fe0b18")
        );
    }

    #[test]
    fn timestamp_parsing() {
        assert_eq!(
            parse_timestamp_to_ms(&serde_json::json!(1_771_061_953_033_i64)),
            Some(1_771_061_953_033)
        );
        assert_eq!(
            parse_timestamp_to_ms(&serde_json::json!(1_771_061_953_i64)),
            Some(1_771_061_953_000)
        );
        assert_eq!(
            parse_timestamp_to_ms(&serde_json::json!("1970-01-01T00:00:01Z")),
            Some(1_000)
        );
    }
}
