//! ChatGPT (Codex)、xAI (Grok) 设备码登录，以及 CodeBuddy 扫码登录。
//!
//! 流程: `auth_start` 获取设备码/登录 URL → 用户在浏览器完成授权 → `auth_poll`
//! 轮询换 token → 会话层把凭据写入原生配置/本地存储。

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CODEX_PROVIDER: &str = "codex";
pub const XAI_PROVIDER: &str = "xai";
pub const WORKBUDDY_PROVIDER: &str = "workbuddy";

const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_DEVICE_USERCODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const CODEX_DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
pub const CODEX_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";

const XAI_ISSUER: &str = "https://auth.x.ai";
const XAI_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
const XAI_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";

const OAUTH_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const CODEX_DEVICE_DEFAULT_EXPIRES_IN: u64 = 900;
const XAI_DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
const XAI_MAX_POLL_INTERVAL_SECS: u64 = 60;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap_or_default()
}

/// 设备码启动结果
#[derive(Debug, Clone, Serialize)]
pub struct DeviceCodeStart {
    pub provider: &'static str,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval_secs: u64,
    /// xAI 专用: 轮询使用的 token 端点(由 discovery 解析)
    pub token_endpoint: Option<String>,
}

/// 轮询状态
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DevicePollStatus {
    /// 用户尚未完成授权, 继续轮询
    Pending,
    /// 授权成功, 凭据如下
    Complete(AuthTokens),
    /// 设备码已过期
    Expired,
    /// 用户拒绝授权
    AccessDenied,
    /// 其他失败
    Failed { message: String },
}

/// 授权得到的令牌
#[derive(Debug, Clone, Serialize, Default)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub account_id: Option<String>,
    pub email: Option<String>,
    /// CodeBuddy: access token 到期 unix 秒
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// CodeBuddy: 登录域
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// CodeBuddy: 企业 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexDeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CodexPollSuccess {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct XaiDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    #[serde(default = "default_xai_expires")]
    expires_in: u64,
    #[serde(default = "default_xai_interval")]
    interval: u64,
}

fn default_xai_expires() -> u64 {
    900
}

fn default_xai_interval() -> u64 {
    XAI_DEFAULT_POLL_INTERVAL_SECS
}

/// 解析 xAI discovery 文档中的设备授权/令牌端点
async fn xai_endpoints() -> Result<(String, String), String> {
    #[derive(Deserialize)]
    struct Discovery {
        device_authorization_endpoint: String,
        token_endpoint: String,
    }
    let discovery: Discovery = http_client()
        .get(format!("{XAI_ISSUER}/.well-known/openid-configuration"))
        .timeout(OAUTH_HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("xAI discovery 请求失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("xAI discovery 解析失败: {e}"))?;
    Ok((
        discovery.device_authorization_endpoint,
        discovery.token_endpoint,
    ))
}

/// 启动 Codex (ChatGPT) 设备码流程
pub async fn codex_start_device_flow() -> Result<DeviceCodeStart, String> {
    let response = http_client()
        .post(CODEX_DEVICE_USERCODE_URL)
        .timeout(OAUTH_HTTP_TIMEOUT)
        .header("Content-Type", "application/json")
        .header("User-Agent", "router-switch-codex-oauth")
        .json(&serde_json::json!({ "client_id": CODEX_CLIENT_ID }))
        .send()
        .await
        .map_err(|e| format!("Device Code 请求失败: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Device Code 请求失败: {status} - {text}"));
    }

    let device: CodexDeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| format!("Device Code 解析失败: {e}"))?;
    let interval_secs = parse_interval(device.interval).unwrap_or(5);

    Ok(DeviceCodeStart {
        provider: CODEX_PROVIDER,
        device_code: device.device_auth_id,
        user_code: device.user_code,
        verification_uri: CODEX_VERIFICATION_URL.to_string(),
        expires_in: device.expires_in.unwrap_or(CODEX_DEVICE_DEFAULT_EXPIRES_IN),
        interval_secs,
        token_endpoint: None,
    })
}

/// 轮询 Codex 设备码状态
pub async fn codex_poll_device(
    device_code: &str,
    user_code: &str,
) -> Result<DevicePollStatus, String> {
    let poll_response = http_client()
        .post(CODEX_DEVICE_TOKEN_URL)
        .timeout(OAUTH_HTTP_TIMEOUT)
        .header("Content-Type", "application/json")
        .header("User-Agent", "router-switch-codex-oauth")
        .json(&serde_json::json!({
            "device_auth_id": device_code,
            "user_code": user_code,
        }))
        .send()
        .await
        .map_err(|e| format!("轮询请求失败: {e}"))?;

    let status = poll_response.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
        return Ok(DevicePollStatus::Pending);
    }
    if status == reqwest::StatusCode::GONE {
        return Ok(DevicePollStatus::Expired);
    }
    if !status.is_success() {
        let text = poll_response.text().await.unwrap_or_default();
        return Ok(DevicePollStatus::Failed {
            message: format!("{status} - {text}"),
        });
    }

    let success: CodexPollSuccess = poll_response
        .json()
        .await
        .map_err(|e| format!("轮询响应解析失败: {e}"))?;

    // 授权码换 token
    let token_response = http_client()
        .post(CODEX_OAUTH_TOKEN_URL)
        .timeout(OAUTH_HTTP_TIMEOUT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "router-switch-codex-oauth")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", success.authorization_code.as_str()),
            ("redirect_uri", CODEX_DEVICE_REDIRECT_URI),
            ("client_id", CODEX_CLIENT_ID),
            ("code_verifier", success.code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Token 交换失败: {e}"))?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let text = token_response.text().await.unwrap_or_default();
        return Ok(DevicePollStatus::Failed {
            message: format!("Token 交换失败: {status} - {text}"),
        });
    }

    let tokens: OAuthTokenResponse = token_response
        .json()
        .await
        .map_err(|e| format!("Token 响应解析失败: {e}"))?;
    let claims = tokens.id_token.as_deref().and_then(jwt_claims);
    let account_id = claims
        .as_ref()
        .and_then(|claims| claims.pointer("/https://api.openai.com/auth/chatgpt_account_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let email = claims
        .as_ref()
        .and_then(|claims| claims.get("email"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(DevicePollStatus::Complete(AuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        id_token: tokens.id_token,
        account_id,
        email,
        ..AuthTokens::default()
    }))
}

/// 启动 xAI (Grok) 设备码流程
pub async fn xai_start_device_flow() -> Result<DeviceCodeStart, String> {
    let (device_authorization_endpoint, token_endpoint) = xai_endpoints().await?;
    let response = http_client()
        .post(&device_authorization_endpoint)
        .timeout(OAUTH_HTTP_TIMEOUT)
        .header("User-Agent", "router-switch-xai-oauth")
        .form(&[("client_id", XAI_CLIENT_ID), ("scope", XAI_SCOPE)])
        .send()
        .await
        .map_err(|e| format!("Device Code 请求失败: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Device Code 请求失败: {status} - {text}"));
    }

    let device: XaiDeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| format!("Device Code 解析失败: {e}"))?;

    Ok(DeviceCodeStart {
        provider: XAI_PROVIDER,
        device_code: device.device_code,
        user_code: device.user_code,
        verification_uri: device
            .verification_uri_complete
            .unwrap_or(device.verification_uri),
        expires_in: device.expires_in,
        interval_secs: device.interval.clamp(1, XAI_MAX_POLL_INTERVAL_SECS),
        token_endpoint: Some(token_endpoint),
    })
}

/// 轮询 xAI 设备码状态
pub async fn xai_poll_device(
    device_code: &str,
    token_endpoint: &str,
) -> Result<DevicePollStatus, String> {
    let response = http_client()
        .post(token_endpoint)
        .timeout(OAUTH_HTTP_TIMEOUT)
        .header("User-Agent", "router-switch-xai-oauth")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", XAI_CLIENT_ID),
            ("device_code", device_code),
        ])
        .send()
        .await
        .map_err(|e| format!("轮询请求失败: {e}"))?;

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("轮询响应解析失败: {e}"))?;

    if let Some(error_code) = value.get("error").and_then(serde_json::Value::as_str) {
        return Ok(match error_code {
            "authorization_pending" | "slow_down" => DevicePollStatus::Pending,
            "access_denied" => DevicePollStatus::AccessDenied,
            "expired_token" => DevicePollStatus::Expired,
            other => DevicePollStatus::Failed {
                message: format!("OAuth 错误: {other}"),
            },
        });
    }

    let tokens: OAuthTokenResponse =
        serde_json::from_value(value).map_err(|e| format!("Token 响应解析失败: {e}"))?;
    let claims = tokens.id_token.as_deref().and_then(jwt_claims);
    let email = claims
        .as_ref()
        .and_then(|claims| claims.get("email"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(DevicePollStatus::Complete(AuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        id_token: tokens.id_token,
        account_id: None,
        email,
        ..AuthTokens::default()
    }))
}

/// 解析 JWT payload 的 claims(不验签, 仅读取展示字段)
pub(crate) fn jwt_claims(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim())
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn parse_interval(value: Option<serde_json::Value>) -> Option<u64> {
    match value? {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_claims_extracts_email() {
        // header.payload.signature, payload = {"email":"a@b.c"}
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJlbWFpbCI6ImFAYi5jIn0.sig";
        let claims = jwt_claims(token).expect("claims");
        assert_eq!(claims.get("email").and_then(|v| v.as_str()), Some("a@b.c"));
    }

    #[test]
    fn jwt_claims_rejects_garbage() {
        assert!(jwt_claims("not-a-jwt").is_none());
    }

    #[test]
    fn parse_interval_handles_number_and_string() {
        assert_eq!(parse_interval(Some(serde_json::json!(5))), Some(5));
        assert_eq!(parse_interval(Some(serde_json::json!("7"))), Some(7));
        assert_eq!(parse_interval(None), None);
    }
}
