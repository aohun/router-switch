pub mod anthropic;
pub mod openai_chat;
pub mod openai_responses;

use std::pin::Pin;
use std::sync::Arc;

use futures_util::Stream;
use tokio_util::sync::CancellationToken;

use crate::model::{FinishReason, ModelInvocation, Usage};
use crate::Result;

pub use anthropic::AnthropicProvider;
pub use openai_chat::OpenAiChatProvider;
pub use openai_responses::OpenAiResponsesProvider;

#[derive(Clone, Debug, PartialEq)]
pub enum ModelEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart {
        index: usize,
        call_id: String,
        name: String,
    },
    ToolCallArgumentsDelta {
        index: usize,
        delta: String,
    },
    ToolCallEnd {
        index: usize,
    },
    Usage(Usage),
    Done(FinishReason),
}

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<ModelEvent>> + Send>>;

pub trait Provider: Send + Sync {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream;
}

pub type SharedProvider = Arc<dyn Provider>;

pub fn create_provider(
    provider_type: &str,
    base_url: String,
    api_key: String,
    model: String,
) -> SharedProvider {
    match provider_type.trim().to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => Arc::new(AnthropicProvider::new(base_url, api_key, model)),
        "openai-responses" | "responses" => {
            Arc::new(OpenAiResponsesProvider::new(base_url, api_key, model))
        }
        _ => Arc::new(OpenAiChatProvider::new(base_url, api_key, model)),
    }
}

/// Build the upstream POST URL from a user-facing base URL.
///
/// Naive `base + /chat/completions` 405s on two common forms:
/// - host only (`https://api.example.com`) — gateways only allow POST on `/v1/chat/completions`
/// - already a full endpoint (`.../v1/chat/completions`) — a second suffix is Method Not Allowed
pub(crate) fn resolve_provider_url(base_url: &str, provider_type: &str) -> Result<String> {
    let base = base_url.trim();
    if base.is_empty() {
        return Err(crate::GatewayError::Config(
            "provider base URL is empty".into(),
        ));
    }
    let kind = provider_type.trim().to_ascii_lowercase();
    let leaf = match kind.as_str() {
        "anthropic" | "claude" => "messages",
        "openai-responses" | "responses" => "responses",
        _ => "chat/completions",
    };
    let default_path = match kind.as_str() {
        "anthropic" | "claude" => "/v1/messages",
        "openai-responses" | "responses" => "/v1/responses",
        _ => "/v1/chat/completions",
    };

    let url = base.trim_end_matches('/');
    let resolved = if url.ends_with(leaf) {
        url.to_string()
    } else if is_api_version_root(url) {
        format!("{url}/{leaf}")
    } else {
        format!("{url}{default_path}")
    };
    Ok(collapse_duplicate_v1(&resolved))
}

fn is_api_version_root(url: &str) -> bool {
    let last = url.rsplit('/').next().unwrap_or("");
    let Some(rest) = last.strip_prefix('v') else {
        return false;
    };
    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn collapse_duplicate_v1(url: &str) -> String {
    let mut normalized = url.to_string();
    while normalized.contains("/v1/v1") {
        normalized = normalized.replace("/v1/v1", "/v1");
    }
    normalized
}

pub(crate) fn merge_extra_params(
    body: &mut serde_json::Value,
    extra: &serde_json::Value,
) -> Result<()> {
    let Some(extra) = extra.as_object() else {
        if extra.is_null() {
            return Ok(());
        }
        return Err(crate::GatewayError::Config(
            "model extra params must be an object".into(),
        ));
    };
    let body = body.as_object_mut().ok_or_else(|| {
        crate::GatewayError::Provider("provider request body must be an object".into())
    })?;
    for (name, value) in extra {
        if matches!(
            name.as_str(),
            "model" | "stream" | "messages" | "input" | "tools" | "system" | "instructions"
        ) {
            return Err(crate::GatewayError::Config(format!(
                "model extra params cannot replace {name}"
            )));
        }
        body.insert(name.clone(), value.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_provider_url;

    #[test]
    fn chat_from_v1_root() {
        assert_eq!(
            resolve_provider_url("https://api.example.com/v1", "openai-chat").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            resolve_provider_url("https://api.example.com/v1/", "openai-chat").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn chat_from_bare_host_inserts_v1() {
        assert_eq!(
            resolve_provider_url("https://api.example.com", "openai-chat").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn chat_does_not_double_append_full_endpoint() {
        assert_eq!(
            resolve_provider_url("https://api.example.com/v1/chat/completions", "openai-chat")
                .unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn chat_collapses_duplicate_v1() {
        assert_eq!(
            resolve_provider_url("https://api.example.com/v1/v1", "openai-chat").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn openrouter_style_api_v1() {
        assert_eq!(
            resolve_provider_url("https://openrouter.ai/api/v1", "openai-chat").unwrap(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[test]
    fn responses_and_anthropic_paths() {
        assert_eq!(
            resolve_provider_url("https://api.example.com", "openai-responses").unwrap(),
            "https://api.example.com/v1/responses"
        );
        assert_eq!(
            resolve_provider_url("https://api.anthropic.com/v1", "anthropic").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            resolve_provider_url("https://api.anthropic.com/v1/messages", "anthropic").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
    }
}
