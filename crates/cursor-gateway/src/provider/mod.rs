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
