//! OAuth 凭据的原生落盘与状态读取。
//!
//! - Codex: 登录成功写入原生 `~/.codex/auth.json`(与 Codex CLI 同构),
//!   状态直接读该文件——官方 `codex login` 写入的内容同样能被识别为已认证。
//! - xAI (Grok): 令牌保存在应用数据库(kv), 无 Grok CLI 原生 OAuth 落盘格式。
//! - WorkBuddy (CodeBuddy): 令牌保存在应用数据库(kv), 到期前 5 分钟刷新。

use serde::{Deserialize, Serialize};

use crate::oauth::AuthTokens;
use crate::workbuddy_auth::WorkBuddyStoredCredentials;
use crate::SessionError;
use adapters_codex::CodexPaths;
use store::Store;

const XAI_KV_KEY: &str = "oauth.xai.tokens";
const WORKBUDDY_KV_KEY: &str = "oauth.workbuddy.tokens";

/// 认证状态(UI 展示用)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    /// 登录账号(邮箱/用户名), 未知为 None
    pub account: Option<String>,
    /// 最近刷新时间(RFC3339 或秒级时间戳字符串)
    pub last_refresh: Option<String>,
}

impl NativeAuthStatus {
    fn unauthenticated(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            authenticated: false,
            account: None,
            last_refresh: None,
        }
    }
}

/// 读取指定提供方的认证状态
pub fn read_status(
    provider: &str,
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<NativeAuthStatus, SessionError> {
    match provider {
        crate::oauth::CODEX_PROVIDER => read_codex_status(store, codex_paths),
        crate::oauth::XAI_PROVIDER => read_xai_status(store),
        crate::oauth::WORKBUDDY_PROVIDER => read_workbuddy_status(store),
        other => Err(SessionError::Message(format!(
            "不支持的认证提供方: {other}"
        ))),
    }
}

/// 授权完成后持久化凭据
pub fn persist_tokens(
    provider: &str,
    tokens: &AuthTokens,
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<(), SessionError> {
    match provider {
        crate::oauth::CODEX_PROVIDER => persist_codex(tokens, store, codex_paths),
        crate::oauth::XAI_PROVIDER => persist_xai(tokens, store),
        crate::oauth::WORKBUDDY_PROVIDER => persist_workbuddy(tokens, store),
        other => Err(SessionError::Message(format!(
            "不支持的认证提供方: {other}"
        ))),
    }
}

/// 清除本地凭据(Codex 连原生 auth.json 一起移除)
pub fn clear_credentials(
    provider: &str,
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<(), SessionError> {
    match provider {
        crate::oauth::CODEX_PROVIDER => {
            store.kv_delete("oauth.codex.info")?;
            if codex_paths.auth.exists() {
                std::fs::remove_file(&codex_paths.auth)
                    .map_err(|e| SessionError::Message(format!("移除 auth.json 失败: {e}")))?;
            }
            Ok(())
        }
        crate::oauth::XAI_PROVIDER => {
            store.kv_delete(XAI_KV_KEY)?;
            Ok(())
        }
        crate::oauth::WORKBUDDY_PROVIDER => {
            store.kv_delete(WORKBUDDY_KV_KEY)?;
            Ok(())
        }
        other => Err(SessionError::Message(format!(
            "不支持的认证提供方: {other}"
        ))),
    }
}

// ==================== Codex ====================

fn persist_codex(
    tokens: &AuthTokens,
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<(), SessionError> {
    let last_refresh = chrono::Utc::now().to_rfc3339();
    let auth_json = serde_json::json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": tokens.id_token,
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "account_id": tokens.account_id,
        },
        "last_refresh": last_refresh,
    });
    if let Some(parent) = codex_paths.auth.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SessionError::Message(format!("创建目录失败: {e}")))?;
    }
    std::fs::write(
        &codex_paths.auth,
        serde_json::to_string_pretty(&auth_json)
            .map_err(|e| SessionError::Message(format!("auth.json 序列化失败: {e}")))?,
    )
    .map_err(|e| SessionError::Message(format!("写入 auth.json 失败: {e}")))?;

    let info = serde_json::json!({
        "email": tokens.email,
        "account_id": tokens.account_id,
        "authenticated_at": last_refresh,
    });
    store.kv_set("oauth.codex.info", &info.to_string())?;
    Ok(())
}

fn read_codex_status(
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<NativeAuthStatus, SessionError> {
    let Ok(text) = std::fs::read_to_string(&codex_paths.auth) else {
        return Ok(NativeAuthStatus::unauthenticated(
            crate::oauth::CODEX_PROVIDER,
        ));
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| SessionError::Message(format!("auth.json 解析失败: {e}")))?;

    let tokens = value.get("tokens");
    let authenticated = tokens
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|token| !token.trim().is_empty());
    if !authenticated {
        return Ok(NativeAuthStatus::unauthenticated(
            crate::oauth::CODEX_PROVIDER,
        ));
    }

    let id_token = tokens
        .and_then(|tokens| tokens.get("id_token"))
        .and_then(serde_json::Value::as_str);
    let claims = id_token.and_then(crate::oauth::jwt_claims);
    let account = claims
        .as_ref()
        .and_then(|claims| claims.get("email"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| store_fallback_email(store));
    let last_refresh = value
        .get("last_refresh")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(NativeAuthStatus {
        provider: crate::oauth::CODEX_PROVIDER.to_string(),
        authenticated: true,
        account,
        last_refresh,
    })
}

fn store_fallback_email(store: &Store) -> Option<String> {
    // auth.json 缺 id_token 时, 回退到登录时记录的账号信息
    let json = store.kv_get("oauth.codex.info").ok()??;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    value
        .get("email")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

// ==================== xAI ====================

#[derive(Debug, Serialize, Deserialize)]
struct XaiStoredCredentials {
    email: Option<String>,
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    authenticated_at: String,
}

fn persist_xai(tokens: &AuthTokens, store: &Store) -> Result<(), SessionError> {
    let credentials = XaiStoredCredentials {
        email: tokens.email.clone(),
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        authenticated_at: chrono::Utc::now().to_rfc3339(),
    };
    store.kv_set(
        XAI_KV_KEY,
        &serde_json::to_string(&credentials)
            .map_err(|e| SessionError::Message(format!("凭据序列化失败: {e}")))?,
    )?;
    Ok(())
}

fn read_xai_status(store: &Store) -> Result<NativeAuthStatus, SessionError> {
    let Some(json) = store.kv_get(XAI_KV_KEY)? else {
        return Ok(NativeAuthStatus::unauthenticated(
            crate::oauth::XAI_PROVIDER,
        ));
    };
    let credentials: XaiStoredCredentials = serde_json::from_str(&json)
        .map_err(|e| SessionError::Message(format!("xAI 凭据损坏: {e}")))?;
    Ok(NativeAuthStatus {
        provider: crate::oauth::XAI_PROVIDER.to_string(),
        authenticated: !credentials.access_token.trim().is_empty(),
        account: credentials.email,
        last_refresh: Some(credentials.authenticated_at),
    })
}

// ==================== WorkBuddy / CodeBuddy ====================

fn persist_workbuddy(tokens: &AuthTokens, store: &Store) -> Result<(), SessionError> {
    let credentials = WorkBuddyStoredCredentials::from_tokens(tokens);
    store.kv_set(
        WORKBUDDY_KV_KEY,
        &serde_json::to_string(&credentials)
            .map_err(|e| SessionError::Message(format!("WorkBuddy 凭据序列化失败: {e}")))?,
    )?;
    Ok(())
}

fn read_workbuddy_status(store: &Store) -> Result<NativeAuthStatus, SessionError> {
    match load_workbuddy_credentials(store)? {
        Some(credentials) if !credentials.access_token.trim().is_empty() => Ok(NativeAuthStatus {
            provider: crate::oauth::WORKBUDDY_PROVIDER.to_string(),
            authenticated: true,
            account: (!credentials.nickname.trim().is_empty()).then_some(credentials.nickname),
            last_refresh: (!credentials.authenticated_at.trim().is_empty())
                .then_some(credentials.authenticated_at),
        }),
        _ => Ok(NativeAuthStatus::unauthenticated(
            crate::oauth::WORKBUDDY_PROVIDER,
        )),
    }
}

pub fn load_workbuddy_credentials(
    store: &Store,
) -> Result<Option<WorkBuddyStoredCredentials>, SessionError> {
    let Some(json) = store.kv_get(WORKBUDDY_KV_KEY)? else {
        return Ok(None);
    };
    let credentials: WorkBuddyStoredCredentials = serde_json::from_str(&json)
        .map_err(|e| SessionError::Message(format!("WorkBuddy 凭据损坏: {e}")))?;
    Ok(Some(credentials))
}

fn save_workbuddy_credentials(
    store: &Store,
    credentials: &WorkBuddyStoredCredentials,
) -> Result<(), SessionError> {
    store.kv_set(
        WORKBUDDY_KV_KEY,
        &serde_json::to_string(credentials)
            .map_err(|e| SessionError::Message(format!("WorkBuddy 凭据序列化失败: {e}")))?,
    )?;
    Ok(())
}

/// 若凭据将在 5 分钟内过期则刷新, 返回最新 access token 与请求头。
pub fn ensure_workbuddy_access_token(
    store: &Store,
) -> Result<Option<WorkBuddyStoredCredentials>, SessionError> {
    let Some(credentials) = load_workbuddy_credentials(store)? else {
        return Ok(None);
    };
    let now = chrono::Utc::now().timestamp();
    if !credentials.needs_refresh(now) {
        return Ok(Some(credentials));
    }
    let rt = crate::tokio_runtime();
    let refreshed = rt
        .block_on(crate::workbuddy_auth::workbuddy_refresh(&credentials))
        .map_err(SessionError::Message)?;
    save_workbuddy_credentials(store, &refreshed)?;
    Ok(Some(refreshed))
}
