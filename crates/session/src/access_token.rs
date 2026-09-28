//! Local inference access token manager (AstrLink accesstoken alignment).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use domain::{
    normalize_access_token_name, token_hint, AccessTokenSource, AccessTokenSummary,
    AccessTokenUsage, CreatedAccessToken, NewAccessTokenRecord, ACCESS_TOKEN_ID_PREFIX,
    ACCESS_TOKEN_PREFIX, DEFAULT_ACCESS_TOKEN_NAME,
};
use sha2::{Digest, Sha256};
use store::Store;

use crate::SessionError;

const TOKEN_RANDOM_BYTES: usize = 32;
const ID_RANDOM_BYTES: usize = 16;

pub fn ensure_default_access_tokens(store: &Store) -> Result<(), SessionError> {
    let candidate =
        new_access_token_record(DEFAULT_ACCESS_TOKEN_NAME, AccessTokenSource::SystemDefault)?;
    match store.ensure_default_access_token(&candidate)? {
        Some(_) => tracing::info!("created bootstrap access token"),
        None => {}
    }
    Ok(())
}

pub fn list_access_tokens(store: &Store) -> Result<Vec<AccessTokenSummary>, SessionError> {
    Ok(store.list_access_tokens()?)
}

pub fn create_access_token(store: &Store, name: &str) -> Result<CreatedAccessToken, SessionError> {
    let (canonical_name, _) = normalize_access_token_name(name).map_err(SessionError::Message)?;
    let candidate = new_access_token_record(&canonical_name, AccessTokenSource::User)?;
    let token = store.create_access_token(&candidate)?;
    Ok(CreatedAccessToken {
        token,
        access_token: candidate.value,
    })
}

pub fn reveal_access_token(store: &Store, id: &str) -> Result<String, SessionError> {
    Ok(store.reveal_access_token(id)?)
}

pub fn delete_access_token(store: &Store, id: &str) -> Result<(), SessionError> {
    store.delete_access_token(id)?;
    Ok(())
}

pub fn list_access_token_usage(
    store: &Store,
    token_ids: &[String],
) -> Result<Vec<AccessTokenUsage>, SessionError> {
    Ok(store.list_access_token_usage(token_ids)?)
}

fn new_access_token_record(
    name: &str,
    source: AccessTokenSource,
) -> Result<NewAccessTokenRecord, SessionError> {
    let (canonical_name, name_key) =
        normalize_access_token_name(name).map_err(SessionError::Message)?;
    let id_bytes = random_bytes(ID_RANDOM_BYTES);
    let secret_bytes = random_bytes(TOKEN_RANDOM_BYTES);
    let id = format!(
        "{ACCESS_TOKEN_ID_PREFIX}{}",
        id_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let raw = format!(
        "{ACCESS_TOKEN_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(secret_bytes)
    );
    let hash = Sha256::digest(raw.as_bytes());
    Ok(NewAccessTokenRecord {
        id,
        name: canonical_name,
        name_key,
        hash: hash.into(),
        hint: token_hint(&raw),
        source,
        value: raw,
        created_at: now_created_at(),
    })
}

/// Authenticate a raw bearer token; returns only the stable non-secret id.
pub fn authenticate_access_token(store: &Store, raw: &str) -> Result<String, SessionError> {
    if !valid_raw_token(raw) {
        return Err(SessionError::Message("无效的访问令牌".into()));
    }
    let hash = Sha256::digest(raw.as_bytes());
    match store.find_access_token_by_hash(hash.as_slice())? {
        Some(summary) => Ok(summary.id),
        None => Err(SessionError::Message("无效的访问令牌".into())),
    }
}

fn valid_raw_token(value: &str) -> bool {
    let Some(encoded) = value.strip_prefix(ACCESS_TOKEN_PREFIX) else {
        return false;
    };
    match URL_SAFE_NO_PAD.decode(encoded) {
        Ok(bytes) => bytes.len() == TOKEN_RANDOM_BYTES,
        Err(_) => false,
    }
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        out.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    out.truncate(len);
    out
}

pub fn format_created_at(value: &str) -> String {
    if let Ok(secs) = value.parse::<i64>() {
        if let Some(dt) = chrono::DateTime::<Utc>::from_timestamp(secs, 0) {
            return dt.format("%Y/%m/%d %H:%M").to_string();
        }
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return dt.format("%Y/%m/%d %H:%M").to_string();
    }
    value.to_string()
}

pub fn now_created_at() -> String {
    Utc::now().to_rfc3339()
}
