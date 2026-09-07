//! CodeBuddy (WorkBuddy) 扫码登录与到期前刷新。
//!
//! 对齐 workbuddy-cliproxy:
//! 1. POST `/v2/plugin/auth/state?platform=CLI` 拿 `state` + `authUrl`
//! 2. 浏览器打开 `authUrl` 扫码
//! 3. GET `/v2/plugin/auth/token?state=` 轮询，code 11217 为进行中
//! 4. 同一 cookie jar 贯穿 state / poll
//! 5. POST `/v2/plugin/auth/token/refresh`，提前 5 分钟刷新

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::oauth::{AuthTokens, DeviceCodeStart, DevicePollStatus, WORKBUDDY_PROVIDER};

pub const WORKBUDDY_ORIGIN: &str = "https://www.codebuddy.cn";
pub const WORKBUDDY_CLIENT_UA: &str = "CLI/2.63.2 CodeBuddy/2.63.2";
const ENDPOINT_AUTH_STATE: &str = "https://copilot.tencent.com/v2/plugin/auth/state?platform=CLI";
const ENDPOINT_AUTH_TOKEN: &str = "https://copilot.tencent.com/v2/plugin/auth/token?state=";
const ENDPOINT_LOGIN_ACCOUNT: &str = "https://copilot.tencent.com/v2/plugin/login/account?state=";
const ENDPOINT_TOKEN_REFRESH: &str = "https://copilot.tencent.com/v2/plugin/auth/token/refresh";

const LOGIN_TTL: Duration = Duration::from_secs(600);
const POLL_INTERVAL_SECS: u64 = 2;
pub const AUTH_REFRESH_LEAD: Duration = Duration::from_secs(5 * 60);
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
/// 上游「登录进行中」
const LOGIN_PENDING_CODE: i64 = 11217;

struct LoginCtx {
    client: reqwest::Client,
    expires: Instant,
}

fn login_states() -> &'static Mutex<HashMap<String, LoginCtx>> {
    static STATES: OnceLock<Mutex<HashMap<String, LoginCtx>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn login_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .cookie_store(true)
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("创建登录 HTTP 客户端失败: {e}"))
}

fn shared_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .unwrap_or_default()
}

fn apply_common_headers(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Origin", WORKBUDDY_ORIGIN)
        .header("Referer", format!("{WORKBUDDY_ORIGIN}/"))
        .header("User-Agent", WORKBUDDY_CLIENT_UA)
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: Value,
}

#[derive(Debug, Deserialize)]
struct AuthStateData {
    #[serde(default)]
    state: String,
    #[serde(rename = "authUrl", default)]
    auth_url: String,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    #[serde(rename = "accessToken", default)]
    access_token: String,
    #[serde(rename = "refreshToken", default)]
    refresh_token: String,
    #[serde(rename = "expiresIn", default)]
    expires_in: i64,
    #[serde(default)]
    domain: String,
}

#[derive(Debug, Deserialize)]
struct AccountData {
    #[serde(default)]
    uid: String,
    #[serde(rename = "enterpriseId", default)]
    enterprise_id: String,
    #[serde(default)]
    nickname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyStoredCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub enterprise_id: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub authenticated_at: String,
}

impl WorkBuddyStoredCredentials {
    pub fn from_tokens(tokens: &AuthTokens) -> Self {
        Self {
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone().unwrap_or_default(),
            expires_at: tokens.expires_at.unwrap_or(0),
            domain: tokens.domain.clone().unwrap_or_default(),
            uid: tokens.account_id.clone().unwrap_or_default(),
            enterprise_id: tokens.enterprise_id.clone().unwrap_or_default(),
            nickname: tokens.email.clone().unwrap_or_default(),
            authenticated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn needs_refresh(&self, now_unix: i64) -> bool {
        if self.refresh_token.trim().is_empty() {
            return false;
        }
        if self.expires_at <= 0 {
            return true;
        }
        self.expires_at - AUTH_REFRESH_LEAD.as_secs() as i64 <= now_unix
    }

    pub fn extra_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        if !self.uid.trim().is_empty() {
            headers.push(("X-User-Id".into(), self.uid.clone()));
        }
        if !self.enterprise_id.trim().is_empty() {
            headers.push(("X-Enterprise-Id".into(), self.enterprise_id.clone()));
        }
        if !self.refresh_token.trim().is_empty() {
            headers.push(("X-Refresh-Token".into(), self.refresh_token.clone()));
        }
        if !self.domain.trim().is_empty() {
            headers.push(("X-Domain".into(), self.domain.clone()));
        }
        headers
    }
}

async fn parse_envelope(response: reqwest::Response) -> Result<(ApiEnvelope, u16), String> {
    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;
    if status >= 400 {
        return Err(format!("http_error: upstream {status}"));
    }
    let env: ApiEnvelope =
        serde_json::from_str(&text).map_err(|e| format!("解析 CodeBuddy 响应失败: {e}"))?;
    Ok((env, status))
}

/// 启动扫码登录: 申请 state 并打开 authUrl。
pub async fn workbuddy_start_login() -> Result<DeviceCodeStart, String> {
    let client = login_client()?;
    let request = apply_common_headers(client.post(ENDPOINT_AUTH_STATE)).body("{}");
    let response = request
        .send()
        .await
        .map_err(|e| format!("auth state 请求失败: {e}"))?;
    let (env, _) = parse_envelope(response).await?;
    if env.code != 0 {
        return Err(format!(
            "auth state 失败: code={} msg={}",
            env.code, env.msg
        ));
    }
    let data: AuthStateData =
        serde_json::from_value(env.data).map_err(|e| format!("auth state 解析失败: {e}"))?;
    if data.state.trim().is_empty() || data.auth_url.trim().is_empty() {
        return Err("auth state: missing state or authUrl".into());
    }
    login_states().lock().insert(
        data.state.clone(),
        LoginCtx {
            client,
            expires: Instant::now() + LOGIN_TTL,
        },
    );
    Ok(DeviceCodeStart {
        provider: WORKBUDDY_PROVIDER,
        device_code: data.state,
        user_code: String::new(),
        verification_uri: data.auth_url,
        expires_in: LOGIN_TTL.as_secs(),
        interval_secs: POLL_INTERVAL_SECS,
        token_endpoint: None,
    })
}

/// 轮询一次登录状态。cookie 必须与 start 使用同一客户端。
pub async fn workbuddy_poll_login(state: &str) -> Result<DevicePollStatus, String> {
    let state = state.trim();
    if state.is_empty() {
        return Err("poll: empty state".into());
    }
    let client = {
        let mut states = login_states().lock();
        let Some(ctx) = states.get(state) else {
            return Err("poll: unknown state (restart login)".into());
        };
        if Instant::now() >= ctx.expires {
            states.remove(state);
            return Ok(DevicePollStatus::Expired);
        }
        ctx.client.clone()
    };

    let token_url = format!("{ENDPOINT_AUTH_TOKEN}{state}");
    let request = apply_common_headers(client.get(&token_url));
    let response = match request.send().await {
        Ok(resp) => resp,
        Err(_) => return Ok(DevicePollStatus::Pending),
    };
    let (env, _) = match parse_envelope(response).await {
        Ok(parsed) => parsed,
        Err(_) => return Ok(DevicePollStatus::Pending),
    };
    if env.code == LOGIN_PENDING_CODE {
        return Ok(DevicePollStatus::Pending);
    }
    if env.code != 0 {
        return Ok(DevicePollStatus::Pending);
    }
    let tok: TokenData = match serde_json::from_value::<TokenData>(env.data) {
        Ok(tok) if !tok.access_token.trim().is_empty() => tok,
        _ => return Ok(DevicePollStatus::Pending),
    };

    let mut account = AccountData {
        uid: String::new(),
        enterprise_id: String::new(),
        nickname: String::new(),
    };
    let acct_url = format!("{ENDPOINT_LOGIN_ACCOUNT}{state}");
    let acct_req = apply_common_headers(client.get(&acct_url))
        .header("Authorization", format!("Bearer {}", tok.access_token));
    if let Ok(resp) = acct_req.send().await {
        if let Ok((env, _)) = parse_envelope(resp).await {
            if env.code == 0 {
                if let Ok(parsed) = serde_json::from_value::<AccountData>(env.data) {
                    account = parsed;
                }
            }
        }
    }

    login_states().lock().remove(state);
    let expires_in = if tok.expires_in > 0 {
        tok.expires_in
    } else {
        3600
    };
    let expires_at = chrono::Utc::now().timestamp() + expires_in;
    Ok(DevicePollStatus::Complete(AuthTokens {
        access_token: tok.access_token,
        refresh_token: (!tok.refresh_token.is_empty()).then_some(tok.refresh_token),
        id_token: None,
        account_id: (!account.uid.is_empty()).then_some(account.uid),
        email: (!account.nickname.is_empty()).then_some(account.nickname),
        expires_at: Some(expires_at),
        domain: (!tok.domain.is_empty()).then_some(tok.domain),
        enterprise_id: (!account.enterprise_id.is_empty()).then_some(account.enterprise_id),
    }))
}

/// 用 refresh token 换新的 access token。
pub async fn workbuddy_refresh(
    credentials: &WorkBuddyStoredCredentials,
) -> Result<WorkBuddyStoredCredentials, String> {
    if credentials.refresh_token.trim().is_empty() {
        return Err("refresh: missing refresh token".into());
    }
    let mut req = apply_common_headers(shared_client().post(ENDPOINT_TOKEN_REFRESH))
        .header("X-Refresh-Token", credentials.refresh_token.trim())
        .header("X-Auth-Refresh-Source", WORKBUDDY_PROVIDER);
    if !credentials.enterprise_id.trim().is_empty() {
        req = req.header("X-Enterprise-Id", credentials.enterprise_id.trim());
    }
    let response = req
        .send()
        .await
        .map_err(|e| format!("refresh 请求失败: {e}"))?;
    let (env, _) = parse_envelope(response).await?;
    if env.code != 0 {
        return Err(format!("refresh failed: code={} msg={}", env.code, env.msg));
    }
    let tok: TokenData =
        serde_json::from_value(env.data).map_err(|e| format!("refresh 响应解析失败: {e}"))?;
    if tok.access_token.trim().is_empty() {
        return Err("refresh_failed: no accessToken".into());
    }
    let expires_in = if tok.expires_in > 0 {
        tok.expires_in
    } else {
        3600
    };
    let mut refreshed = credentials.clone();
    refreshed.access_token = tok.access_token;
    if !tok.refresh_token.trim().is_empty() {
        refreshed.refresh_token = tok.refresh_token;
    }
    if !tok.domain.trim().is_empty() {
        refreshed.domain = tok.domain;
    }
    refreshed.expires_at = chrono::Utc::now().timestamp() + expires_in;
    refreshed.authenticated_at = chrono::Utc::now().to_rfc3339();
    Ok(refreshed)
}

pub fn workbuddy_default_headers() -> Vec<(String, String)> {
    vec![
        ("User-Agent".into(), WORKBUDDY_CLIENT_UA.into()),
        ("Origin".into(), WORKBUDDY_ORIGIN.into()),
        ("Referer".into(), format!("{WORKBUDDY_ORIGIN}/")),
        ("X-Requested-With".into(), "XMLHttpRequest".into()),
        ("X-Product".into(), "SaaS".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_refresh_respects_lead_time() {
        let creds = WorkBuddyStoredCredentials {
            access_token: "at".into(),
            refresh_token: "rt".into(),
            expires_at: 1_000,
            domain: String::new(),
            uid: String::new(),
            enterprise_id: String::new(),
            nickname: String::new(),
            authenticated_at: String::new(),
        };
        assert!(creds.needs_refresh(1_000 - AUTH_REFRESH_LEAD.as_secs() as i64));
        assert!(!creds.needs_refresh(1_000 - AUTH_REFRESH_LEAD.as_secs() as i64 - 1));
    }

    #[test]
    fn extra_headers_omit_empty_fields() {
        let creds = WorkBuddyStoredCredentials {
            access_token: "at".into(),
            refresh_token: "rt".into(),
            expires_at: 1,
            domain: String::new(),
            uid: "u1".into(),
            enterprise_id: String::new(),
            nickname: String::new(),
            authenticated_at: String::new(),
        };
        let headers = creds.extra_headers();
        assert!(headers.iter().any(|(k, v)| k == "X-User-Id" && v == "u1"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-Refresh-Token" && v == "rt"));
        assert!(!headers.iter().any(|(k, _)| k == "X-Enterprise-Id"));
    }
}
