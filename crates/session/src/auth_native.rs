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
    persist_codex_with_expires(tokens, store, codex_paths, tokens.expires_at)
}

fn persist_codex_with_expires(
    tokens: &AuthTokens,
    store: &Store,
    codex_paths: &CodexPaths,
    expires_at: Option<i64>,
) -> Result<(), SessionError> {
    let last_refresh = chrono::Utc::now().to_rfc3339();
    let expires_at = expires_at.or(tokens.expires_at).or_else(|| {
        crate::oauth::jwt_claims(&tokens.access_token)
            .as_ref()
            .and_then(|c| c.get("exp"))
            .and_then(|v| v.as_i64())
    });
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
        "expires_at": expires_at,
    });
    if let Some(parent) = codex_paths.auth.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SessionError::Message(format!("创建目录失败: {e}")))?;
    }
    // Write via temp + rename so concurrent readers never see a torn auth.json.
    let tmp = codex_paths.auth.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_string_pretty(&auth_json)
            .map_err(|e| SessionError::Message(format!("auth.json 序列化失败: {e}")))?,
    )
    .map_err(|e| SessionError::Message(format!("写入 auth.json 失败: {e}")))?;
    std::fs::rename(&tmp, &codex_paths.auth)
        .map_err(|e| SessionError::Message(format!("更新 auth.json 失败: {e}")))?;

    // Keep official.bak in sync. Switching providers / preserve_official_auth
    // restores auth.json from this backup — a stale bak would clobber a fresh
    // OAuth login and leave a burned one-time refresh_token behind.
    let _ = std::fs::copy(&codex_paths.auth, &codex_paths.auth_official_bak);

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

/// Read Codex ChatGPT tokens from `~/.codex/auth.json` for usage/models APIs.
pub fn read_codex_tokens(
    codex_paths: &CodexPaths,
) -> Result<Option<crate::codex_subscription::CodexStoredTokens>, SessionError> {
    let Ok(text) = std::fs::read_to_string(&codex_paths.auth) else {
        return Ok(None);
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| SessionError::Message(format!("auth.json 解析失败: {e}")))?;
    let tokens = value.get("tokens");
    let access_token = tokens
        .and_then(|t| t.get("access_token"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let Some(access_token) = access_token else {
        return Ok(None);
    };
    let refresh_token = tokens
        .and_then(|t| t.get("refresh_token"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let account_id = tokens
        .and_then(|t| t.get("account_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let id_token = tokens
        .and_then(|t| t.get("id_token"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let last_refresh = value
        .get("last_refresh")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Ok(Some(crate::codex_subscription::CodexStoredTokens {
        access_token,
        refresh_token,
        account_id,
        id_token,
        last_refresh,
    }))
}

/// AstrLink `DefaultRefreshSkew`: refresh when fewer than 5 minutes remain.
const CODEX_REFRESH_SKEW_SECS: i64 = 5 * 60;

fn codex_refresh_mutex() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Whether the access token should be refreshed (JWT `exp` + 5m skew, AstrLink-style).
fn access_token_needs_refresh(access_token: &str, last_refresh: Option<&str>) -> bool {
    let now = chrono::Utc::now().timestamp();
    if let Some(exp) = crate::oauth::jwt_claims(access_token)
        .as_ref()
        .and_then(|c| c.get("exp"))
        .and_then(|v| v.as_i64())
    {
        return now + CODEX_REFRESH_SKEW_SECS >= exp;
    }
    // No JWT exp — fall back to last_refresh age (assume ~60m access lifetime).
    let Some(raw) = last_refresh.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return true;
    };
    let age = chrono::Utc::now().signed_duration_since(ts.with_timezone(&chrono::Utc));
    age.num_seconds() >= 55 * 60
}

fn stored_to_auth_merge(
    refreshed: crate::oauth::AuthTokens,
    previous: &crate::codex_subscription::CodexStoredTokens,
    store: &Store,
) -> crate::oauth::AuthTokens {
    let mut refreshed = refreshed;
    if refreshed.account_id.is_none() {
        refreshed.account_id = previous.account_id.clone();
    }
    if refreshed.id_token.is_none() {
        refreshed.id_token = previous.id_token.clone();
    }
    if refreshed.email.is_none() {
        refreshed.email = store_fallback_email(store);
    }
    if refreshed.refresh_token.is_none() {
        refreshed.refresh_token = previous.refresh_token.clone();
    }
    refreshed
}

/// Async ensure + refresh (safe to call on the session tokio runtime).
///
/// OpenAI rotates refresh tokens — concurrent refresh must be single-flight and
/// the new refresh_token must be persisted before another caller runs (AstrLink TokenSource).
pub async fn ensure_codex_tokens_async(
    store: &Store,
    codex_paths: &CodexPaths,
    force_refresh: bool,
) -> Result<crate::codex_subscription::CodexStoredTokens, String> {
    let Some(current) = read_codex_tokens(codex_paths).map_err(|e| e.to_string())? else {
        return Err("Codex 未登录".into());
    };
    if !force_refresh
        && !access_token_needs_refresh(&current.access_token, current.last_refresh.as_deref())
    {
        return Ok(current);
    }

    // Serialize refresh so two callers cannot burn the same one-time refresh_token.
    let _guard = codex_refresh_mutex().lock().await;

    // Re-read under lock — another waiter may have already refreshed + persisted.
    let Some(current) = read_codex_tokens(codex_paths).map_err(|e| e.to_string())? else {
        return Err("Codex 未登录".into());
    };
    if !force_refresh
        && !access_token_needs_refresh(&current.access_token, current.last_refresh.as_deref())
    {
        return Ok(current);
    }

    let Some(refresh) = current.refresh_token.clone() else {
        return Err("Codex 登录已过期，请重新授权".into());
    };

    let refreshed = match crate::oauth::refresh_codex_access_token(&refresh).await {
        Ok(tokens) => tokens,
        Err(e) if e.contains("refresh_token_invalid") => {
            // Codex CLI / another client may have rotated auth.json — re-read once.
            drop(_guard);
            let Some(retry) = read_codex_tokens(codex_paths).map_err(|e| e.to_string())? else {
                return Err("Codex 登录已过期，请重新授权".into());
            };
            if retry.refresh_token.as_deref() != Some(refresh.as_str())
                && !access_token_needs_refresh(&retry.access_token, retry.last_refresh.as_deref())
            {
                return Ok(retry);
            }
            return Err("Codex 登录已过期，请重新授权".into());
        }
        Err(e) => return Err(e),
    };
    let merged = stored_to_auth_merge(refreshed, &current, store);
    persist_codex_with_expires(&merged, store, codex_paths, merged.expires_at)
        .map_err(|e| e.to_string())?;
    read_codex_tokens(codex_paths)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "刷新后无法读取 Codex 凭据".into())
}

/// Ensure Codex access token is usable (sync wrapper).
#[allow(dead_code)]
pub fn ensure_codex_tokens(
    store: &Store,
    codex_paths: &CodexPaths,
    force_refresh: bool,
) -> Result<crate::codex_subscription::CodexStoredTokens, SessionError> {
    let rt = crate::tokio_runtime();
    rt.block_on(ensure_codex_tokens_async(store, codex_paths, force_refresh))
        .map_err(SessionError::Message)
}

/// Fetch usage with automatic access-token refresh (AstrLink TokenSource behavior).
pub async fn fetch_codex_usage_with_refresh(
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<crate::codex_subscription::CodexSubscriptionUsage, String> {
    let tokens = ensure_codex_tokens_async(store, codex_paths, false).await?;
    match crate::codex_subscription::fetch_codex_usage(&tokens).await {
        Ok(usage) => Ok(usage),
        Err(err) if crate::codex_subscription::is_token_auth_error(&err) => {
            let refreshed = ensure_codex_tokens_async(store, codex_paths, true).await?;
            crate::codex_subscription::fetch_codex_usage(&refreshed).await
        }
        Err(err) => Err(err),
    }
}

pub async fn fetch_codex_models_with_refresh(
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<Vec<String>, String> {
    let tokens = ensure_codex_tokens_async(store, codex_paths, false).await?;
    match crate::codex_subscription::fetch_codex_models(&tokens).await {
        Ok(models) => Ok(models),
        Err(err) if crate::codex_subscription::is_token_auth_error(&err) => {
            let refreshed = ensure_codex_tokens_async(store, codex_paths, true).await?;
            crate::codex_subscription::fetch_codex_models(&refreshed).await
        }
        Err(err) => Err(err),
    }
}

/// Ensure once, then fetch models + usage (avoids parallel refresh races after login).
pub async fn fetch_codex_models_and_usage(
    store: &Store,
    codex_paths: &CodexPaths,
) -> Result<
    (
        Vec<String>,
        crate::codex_subscription::CodexSubscriptionUsage,
    ),
    String,
> {
    let tokens = ensure_codex_tokens_async(store, codex_paths, false).await?;
    let models = match crate::codex_subscription::fetch_codex_models(&tokens).await {
        Ok(m) => m,
        Err(err) if crate::codex_subscription::is_token_auth_error(&err) => {
            let refreshed = ensure_codex_tokens_async(store, codex_paths, true).await?;
            crate::codex_subscription::fetch_codex_models(&refreshed).await?
        }
        Err(err) => return Err(err),
    };
    // Re-read tokens after possible refresh during models fetch.
    let tokens = ensure_codex_tokens_async(store, codex_paths, false).await?;
    let usage = match crate::codex_subscription::fetch_codex_usage(&tokens).await {
        Ok(u) => u,
        Err(err) if crate::codex_subscription::is_token_auth_error(&err) => {
            let refreshed = ensure_codex_tokens_async(store, codex_paths, true).await?;
            crate::codex_subscription::fetch_codex_usage(&refreshed).await?
        }
        Err(err) => return Err(err),
    };
    Ok((models, usage))
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn fake_jwt_with_exp(exp: i64) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!(r#"{{"exp":{exp}}}"#).as_bytes());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn access_token_needs_refresh_respects_jwt_skew() {
        let far = chrono::Utc::now().timestamp() + 3600;
        assert!(!access_token_needs_refresh(&fake_jwt_with_exp(far), None));

        let soon = chrono::Utc::now().timestamp() + 60; // within 5m skew
        assert!(access_token_needs_refresh(&fake_jwt_with_exp(soon), None));

        let past = chrono::Utc::now().timestamp() - 10;
        assert!(access_token_needs_refresh(&fake_jwt_with_exp(past), None));
    }
}
