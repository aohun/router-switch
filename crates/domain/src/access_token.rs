//! Local inference access tokens (AstrLink accesstoken alignment).

use serde::{Deserialize, Serialize};

pub const ACCESS_TOKEN_LIMIT: usize = 100;
pub const DEFAULT_ACCESS_TOKEN_NAME: &str = "默认令牌";
pub const ACCESS_TOKEN_PREFIX: &str = "rsw_";
pub const ACCESS_TOKEN_ID_PREFIX: &str = "access_token_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessTokenSource {
    SystemDefault,
    User,
}

impl AccessTokenSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SystemDefault => "system_default",
            Self::User => "user",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "system_default" => Some(Self::SystemDefault),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenSummary {
    pub id: String,
    pub name: String,
    pub hint: String,
    pub source: AccessTokenSource,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedAccessToken {
    pub token: AccessTokenSummary,
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenUsage {
    pub token_id: String,
    pub today_tokens: i64,
    pub total_tokens: i64,
}

/// Metadata-only candidate passed to the store on insert.
#[derive(Debug, Clone)]
pub struct NewAccessTokenRecord {
    pub id: String,
    pub name: String,
    pub name_key: String,
    pub hash: [u8; 32],
    pub hint: String,
    pub source: AccessTokenSource,
    pub value: String,
    /// RFC3339 timestamp.
    pub created_at: String,
}

pub fn token_hint(raw: &str) -> String {
    const VISIBLE_SUFFIX: usize = 6;
    if raw.len() <= VISIBLE_SUFFIX {
        return raw.to_string();
    }
    format!(
        "{ACCESS_TOKEN_PREFIX}…{}",
        &raw[raw.len() - VISIBLE_SUFFIX..]
    )
}

pub fn normalize_access_token_name(value: &str) -> Result<(String, String), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("请输入令牌名称。".into());
    }
    let char_count = trimmed.chars().count();
    if char_count > 64 {
        return Err("令牌名称最多 64 个字符。".into());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("令牌名称包含无效字符。".into());
    }
    Ok((trimmed.to_string(), simple_fold_key(trimmed)))
}

fn simple_fold_key(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            // ASCII-only SimpleFold cycle (A↔a). Full Unicode fold can land later.
            let mut smallest = c;
            let mut folded = unicode_simple_fold(c);
            while folded != c {
                if folded < smallest {
                    smallest = folded;
                }
                folded = unicode_simple_fold(folded);
            }
            smallest
        })
        .collect()
}

fn unicode_simple_fold(c: char) -> char {
    match c {
        'A'..='Z' => ((c as u8) + 32) as char,
        'a'..='z' => ((c as u8) - 32) as char,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hint_masks_middle() {
        let raw = format!("{ACCESS_TOKEN_PREFIX}abcdefghijklmnopqrstuvwxyz123456");
        let hint = token_hint(&raw);
        assert!(hint.starts_with(ACCESS_TOKEN_PREFIX));
        assert!(hint.contains('…'));
    }

    #[test]
    fn normalize_rejects_empty_name() {
        assert!(normalize_access_token_name("  ").is_err());
    }

    #[test]
    fn normalize_folds_ascii_case_for_key() {
        let (_, key_a) = normalize_access_token_name("CI Agent").unwrap();
        let (_, key_b) = normalize_access_token_name("ci agent").unwrap();
        assert_eq!(key_a, key_b);
    }
}
