use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

use crate::{GatewayError, Result};

pub const DEFAULT_EMAIL: &str = "cursor@ai.com";
pub const SIGN_UP_TYPE: &str = "Google";
pub const SUBJECT: &str = "cursor-local-user";
pub const MEMBERSHIP_TYPE: &str = "ultra";
pub const SUBSCRIPTION_STATUS: &str = "active";

pub fn state_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| GatewayError::Config("cannot resolve user home directory".into()))?;
    match std::env::consts::OS {
        "macos" => {
            Ok(home.join("Library/Application Support/Cursor/User/globalStorage/state.vscdb"))
        }
        "windows" => Ok(std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"))
            .join("Cursor/User/globalStorage/state.vscdb")),
        "linux" => Ok(std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("Cursor/User/globalStorage/state.vscdb")),
        platform => Err(GatewayError::Config(format!(
            "Cursor account injection is unsupported on {platform}"
        ))),
    }
}

pub fn ensure_local_ultra_account() -> Result<()> {
    ensure_local_ultra_account_at(&state_db_path()?, None)
}

pub fn ensure_local_ultra_account_at(path: &Path, email_override: Option<&str>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
        [],
    )?;

    let access_token: Option<String> = conn
        .query_row(
            "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
            params!["cursorAuth/accessToken"],
            |row| row.get(0),
        )
        .optional()?;

    let membership: Option<String> = conn
        .query_row(
            "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
            params!["cursorAuth/stripeMembershipType"],
            |row| row.get(0),
        )
        .optional()?;

    let subscription: Option<String> = conn
        .query_row(
            "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
            params!["cursorAuth/stripeSubscriptionStatus"],
            |row| row.get(0),
        )
        .optional()?;

    // If token is missing, or membership is not ultra, or subscription is not active -> write local ultra account
    let needs_injection = access_token.as_ref().map_or(true, |s| s.trim().is_empty())
        || membership.as_deref() != Some(MEMBERSHIP_TYPE)
        || subscription.as_deref() != Some(SUBSCRIPTION_STATUS);

    if needs_injection {
        force_inject_ultra_at(path, email_override)?;
    }
    Ok(())
}

pub fn inject_if_missing() -> Result<()> {
    ensure_local_ultra_account()
}

pub fn inject_if_missing_at(path: &Path) -> Result<()> {
    ensure_local_ultra_account_at(path, None)
}

pub fn force_inject_ultra() -> Result<()> {
    force_inject_ultra_at(&state_db_path()?, None)
}

pub fn force_inject_ultra_at(path: &Path, email_override: Option<&str>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
        [],
    )?;

    let email = email_override.unwrap_or(DEFAULT_EMAIL);
    let token = local_token(email)?;
    let values = [
        ("cursorAuth/accessToken", token.as_str()),
        ("cursorAuth/refreshToken", token.as_str()),
        ("cursorAuth/cachedEmail", email),
        ("cursorAuth/cachedSignUpType", SIGN_UP_TYPE),
        ("cursorAuth/stripeMembershipType", MEMBERSHIP_TYPE),
        ("cursorAuth/stripeSubscriptionStatus", SUBSCRIPTION_STATUS),
    ];

    let tx = conn.unchecked_transaction()?;
    for (key, value) in values {
        tx.execute(
            "INSERT OR REPLACE INTO ItemTable(key, value) VALUES(?, ?)",
            params![key, value],
        )?;
    }
    tx.commit()?;
    tracing::info!(
        email,
        subject = SUBJECT,
        "injected local Cursor Ultra account into state.vscdb"
    );
    Ok(())
}

fn local_token(email: &str) -> Result<String> {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({
        "sub": SUBJECT,
        "email": email,
        "type": "session",
        "iss": "cursor-client",
        "scope": "openid profile email",
        "exp": 4070908800_u64
    }))?);
    Ok(format!("{header}.{payload}.{SUBJECT}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_the_local_account_and_ensures_ultra_status() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.vscdb");

        ensure_local_ultra_account_at(&path, None).unwrap();

        let conn = Connection::open(&path).unwrap();
        let token: String = conn
            .query_row(
                "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
                params!["cursorAuth/accessToken"],
                |row| row.get(0),
            )
            .unwrap();
        let email: String = conn
            .query_row(
                "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
                params!["cursorAuth/cachedEmail"],
                |row| row.get(0),
            )
            .unwrap();
        let membership: String = conn
            .query_row(
                "SELECT CAST(value AS TEXT) FROM ItemTable WHERE key = ?",
                params!["cursorAuth/stripeMembershipType"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(email, DEFAULT_EMAIL);
        assert_eq!(membership, MEMBERSHIP_TYPE);
        assert!(!token.is_empty());
    }
}
