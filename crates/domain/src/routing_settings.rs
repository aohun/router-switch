//! Gateway routing settings (AstrLink RoutingSettingsPanel alignment).
//! Persisted in app.db `kv.routing_settings`; runtime apply is a follow-up.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_MODEL_REDIRECTS: usize = 200;
pub const MAX_REDIRECT_MODEL_LEN: usize = 256;
pub const BUILTIN_CODEX_AUTO_REVIEW_FROM: &str = "codex-auto-review";
pub const BUILTIN_CODEX_AUTO_REVIEW_DEFAULT_TO: &str = "gpt-5.6-luna";
pub const ASTRLINK_AUTO_MODEL_ID: &str = "astrlink/auto";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureAction {
    Stop,
    Retry,
    Failover,
    RetryAndFailover,
}

impl FailureAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Retry => "retry",
            Self::Failover => "failover",
            Self::RetryAndFailover => "retry_and_failover",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Stop,
            Self::Retry,
            Self::Failover,
            Self::RetryAndFailover,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverStrategy {
    RetryFirst,
    FailoverFirst,
    FailoverOnly,
}

impl FailoverStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RetryFirst => "retry_first",
            Self::FailoverFirst => "failover_first",
            Self::FailoverOnly => "failover_only",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::RetryFirst, Self::FailoverFirst, Self::FailoverOnly]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailurePolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u32,
    pub max_delay_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_start_timeout_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_signature_recovery: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_reasoning_recovery: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_function_output_recovery: Option<bool>,
    pub network_error: FailureAction,
    pub response_timeout: FailureAction,
    #[serde(default)]
    pub http_status: BTreeMap<String, FailureAction>,
}

impl Default for FailurePolicy {
    fn default() -> Self {
        default_failure_policy()
    }
}

pub fn default_failure_policy() -> FailurePolicy {
    let mut http_status = BTreeMap::new();
    for code in ["408", "429", "500", "502", "503", "504", "529"] {
        http_status.insert(code.to_string(), FailureAction::RetryAndFailover);
    }
    http_status.insert("401".into(), FailureAction::Failover);
    http_status.insert("403".into(), FailureAction::Failover);
    FailurePolicy {
        max_retries: 1,
        initial_delay_ms: 500,
        max_delay_ms: 5000,
        response_start_timeout_seconds: None,
        thinking_signature_recovery: None,
        openai_reasoning_recovery: None,
        openai_function_output_recovery: None,
        network_error: FailureAction::RetryAndFailover,
        response_timeout: FailureAction::RetryAndFailover,
        http_status,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelStickiness {
    pub enabled: bool,
    pub ttl_seconds: u32,
}

impl Default for ChannelStickiness {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRedirect {
    pub from: String,
    pub to: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingSettings {
    #[serde(default = "default_true")]
    pub codex_identity_enforcement: bool,
    #[serde(default = "default_true")]
    pub claude_identity_enforcement: bool,
    #[serde(default = "default_true")]
    pub grok_identity_enforcement: bool,
    #[serde(default)]
    pub model_redirects: Vec<ModelRedirect>,
    #[serde(default)]
    pub channel_stickiness: ChannelStickiness,
    #[serde(default)]
    pub default_failure_policy: FailurePolicy,
    #[serde(default = "default_true")]
    pub allow_unmatched_failover: bool,
    #[serde(default)]
    pub strategy: FailoverStrategy,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_attempts() -> u32 {
    6
}

impl Default for FailoverStrategy {
    fn default() -> Self {
        Self::FailoverOnly
    }
}

impl Default for RoutingSettings {
    fn default() -> Self {
        Self {
            codex_identity_enforcement: true,
            claude_identity_enforcement: true,
            grok_identity_enforcement: true,
            model_redirects: Vec::new(),
            channel_stickiness: ChannelStickiness::default(),
            default_failure_policy: default_failure_policy(),
            allow_unmatched_failover: true,
            strategy: FailoverStrategy::FailoverOnly,
            max_attempts: 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRedirectIssue {
    EmptyFrom,
    EmptyTo,
    TooLong,
    ControlCharacter,
    Whitespace,
    SameModel,
    AutoTarget,
    DuplicateFrom,
    ChainedTarget,
}

impl ModelRedirectIssue {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyFrom => "empty_from",
            Self::EmptyTo => "empty_to",
            Self::TooLong => "too_long",
            Self::ControlCharacter => "control_character",
            Self::Whitespace => "whitespace",
            Self::SameModel => "same_model",
            Self::AutoTarget => "auto_target",
            Self::DuplicateFrom => "duplicate_from",
            Self::ChainedTarget => "chained_target",
        }
    }
}

fn has_control_character(value: &str) -> bool {
    value.chars().any(|c| {
        let code = c as u32;
        code < 32 && code != 9
    })
}

fn has_edge_whitespace(value: &str) -> bool {
    value.chars().next().is_some_and(|c| c.is_whitespace())
        || value.chars().next_back().is_some_and(|c| c.is_whitespace())
}

/// First issue per row (AstrLink `modelRedirectIssues`).
pub fn model_redirect_issues(redirects: &[ModelRedirect]) -> Vec<Option<ModelRedirectIssue>> {
    let mut sources: BTreeMap<&str, usize> = BTreeMap::new();
    for r in redirects {
        *sources.entry(r.from.as_str()).or_default() += 1;
    }
    redirects
        .iter()
        .map(|r| {
            if r.from.is_empty() {
                return Some(ModelRedirectIssue::EmptyFrom);
            }
            if r.to.is_empty() {
                return Some(ModelRedirectIssue::EmptyTo);
            }
            if r.from.chars().count() > MAX_REDIRECT_MODEL_LEN
                || r.to.chars().count() > MAX_REDIRECT_MODEL_LEN
            {
                return Some(ModelRedirectIssue::TooLong);
            }
            if has_control_character(&r.from) || has_control_character(&r.to) {
                return Some(ModelRedirectIssue::ControlCharacter);
            }
            if has_edge_whitespace(&r.from) || has_edge_whitespace(&r.to) {
                return Some(ModelRedirectIssue::Whitespace);
            }
            if r.from == r.to {
                return Some(ModelRedirectIssue::SameModel);
            }
            if r.to == ASTRLINK_AUTO_MODEL_ID {
                return Some(ModelRedirectIssue::AutoTarget);
            }
            if sources.get(r.from.as_str()).copied().unwrap_or(0) > 1 {
                return Some(ModelRedirectIssue::DuplicateFrom);
            }
            if sources.contains_key(r.to.as_str()) {
                return Some(ModelRedirectIssue::ChainedTarget);
            }
            None
        })
        .collect()
}

pub fn validate_failure_policy(policy: &FailurePolicy) -> Result<(), String> {
    if policy.max_retries > 5 {
        return Err("max_retries must be between 0 and 5".into());
    }
    if policy.initial_delay_ms > 60_000 {
        return Err("initial_delay_ms must be between 0 and 60000".into());
    }
    if policy.max_delay_ms < policy.initial_delay_ms || policy.max_delay_ms > 60_000 {
        return Err("max_delay_ms must be between initial_delay_ms and 60000".into());
    }
    if let Some(n) = policy.response_start_timeout_seconds {
        if n > 86_400 {
            return Err("response_start_timeout_seconds must be between 0 and 86400".into());
        }
    }
    for (code, _) in &policy.http_status {
        let n: u32 = code
            .parse()
            .map_err(|_| format!("invalid http_status rule {code:?}"))?;
        if !(400..=599).contains(&n) || code != &n.to_string() {
            return Err(format!("invalid http_status rule {code:?}"));
        }
    }
    Ok(())
}

pub fn validate_channel_stickiness(s: &ChannelStickiness) -> Result<(), String> {
    if !(60..=86_400).contains(&s.ttl_seconds) {
        return Err("ttl_seconds must be between 60 and 86400".into());
    }
    Ok(())
}

pub fn validate_routing_settings(settings: &RoutingSettings) -> Result<(), String> {
    if settings.model_redirects.len() > MAX_MODEL_REDIRECTS {
        return Err(format!(
            "model_redirects exceeds limit ({MAX_MODEL_REDIRECTS})"
        ));
    }
    if model_redirect_issues(&settings.model_redirects)
        .iter()
        .any(|i| i.is_some())
    {
        return Err("invalid model_redirects".into());
    }
    validate_channel_stickiness(&settings.channel_stickiness)?;
    validate_failure_policy(&settings.default_failure_policy)?;
    if !(1..=20).contains(&settings.max_attempts) {
        return Err("max_attempts must be between 1 and 20".into());
    }
    Ok(())
}

/// Ensure builtin redirect row exists in the editable list (disabled by default).
pub fn ensure_builtin_redirects(settings: &mut RoutingSettings) {
    if !settings
        .model_redirects
        .iter()
        .any(|r| r.from == BUILTIN_CODEX_AUTO_REVIEW_FROM)
    {
        settings.model_redirects.insert(
            0,
            ModelRedirect {
                from: BUILTIN_CODEX_AUTO_REVIEW_FROM.into(),
                to: BUILTIN_CODEX_AUTO_REVIEW_DEFAULT_TO.into(),
                enabled: false,
            },
        );
    }
}

/// The locked built-in row is only the first `codex-auto-review` entry.
/// A duplicate with the same `from` stays editable so the user can fix the conflict.
/// Apply the first enabled exact `from` match (AstrLink model redirects).
pub fn apply_model_redirect(redirects: &[ModelRedirect], model: &str) -> String {
    let model = model.trim();
    if model.is_empty() || model == ASTRLINK_AUTO_MODEL_ID {
        return String::new();
    }
    for redirect in redirects {
        if redirect.enabled && redirect.from == model {
            return redirect.to.clone();
        }
    }
    model.to_string()
}

pub fn is_builtin_model_redirect(redirects: &[ModelRedirect], index: usize) -> bool {
    redirects
        .iter()
        .position(|r| r.from == BUILTIN_CODEX_AUTO_REVIEW_FROM)
        == Some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_astrlink_shape() {
        let s = RoutingSettings::default();
        assert!(s.allow_unmatched_failover);
        assert_eq!(s.strategy, FailoverStrategy::FailoverOnly);
        assert_eq!(s.max_attempts, 6);
        assert_eq!(s.channel_stickiness.ttl_seconds, 3600);
        assert_eq!(s.default_failure_policy.max_retries, 1);
        assert!(s.codex_identity_enforcement);
        assert!(validate_routing_settings(&s).is_ok());
    }

    #[test]
    fn rejects_chained_redirects() {
        let s = RoutingSettings {
            model_redirects: vec![
                ModelRedirect {
                    from: "a".into(),
                    to: "b".into(),
                    enabled: true,
                },
                ModelRedirect {
                    from: "b".into(),
                    to: "c".into(),
                    enabled: true,
                },
            ],
            ..Default::default()
        };
        assert!(validate_routing_settings(&s).is_err());
        let issues = model_redirect_issues(&s.model_redirects);
        // a→b is flagged because `b` is also a source (would form a chain).
        assert_eq!(issues[0], Some(ModelRedirectIssue::ChainedTarget));
    }

    #[test]
    fn ensure_builtin_inserts_once() {
        let mut s = RoutingSettings::default();
        ensure_builtin_redirects(&mut s);
        ensure_builtin_redirects(&mut s);
        assert_eq!(
            s.model_redirects
                .iter()
                .filter(|r| r.from == BUILTIN_CODEX_AUTO_REVIEW_FROM)
                .count(),
            1
        );
        assert!(!s.model_redirects[0].enabled);
    }

    #[test]
    fn only_first_codex_auto_review_is_builtin() {
        let redirects = vec![
            ModelRedirect {
                from: BUILTIN_CODEX_AUTO_REVIEW_FROM.into(),
                to: "gpt-a".into(),
                enabled: false,
            },
            ModelRedirect {
                from: BUILTIN_CODEX_AUTO_REVIEW_FROM.into(),
                to: "gpt-b".into(),
                enabled: true,
            },
        ];
        assert!(is_builtin_model_redirect(&redirects, 0));
        assert!(!is_builtin_model_redirect(&redirects, 1));
    }

    #[test]
    fn roundtrip_json() {
        let s = RoutingSettings::default();
        let raw = serde_json::to_string(&s).unwrap();
        let back: RoutingSettings = serde_json::from_str(&raw).unwrap();
        assert_eq!(s, back);
    }
}
