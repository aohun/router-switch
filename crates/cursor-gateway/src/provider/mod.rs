pub mod anthropic;
pub mod openai_chat;
pub mod openai_responses;

use std::pin::Pin;

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::Result;

pub use anthropic::AnthropicProvider;
pub use openai_chat::OpenAiChatProvider;
pub use openai_responses::OpenAiResponsesProvider;

#[derive(Clone, Debug, PartialEq)]
pub enum ModelEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart {
        call_id: String,
        name: String,
    },
    ToolCallArgumentsDelta {
        call_id: String,
        delta: String,
    },
    ToolCallEnd {
        call_id: String,
    },
    Usage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    },
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Default)]
pub struct ModelInvocation {
    pub prompt: String,
    pub model: String,
    pub messages: Vec<ProviderMessage>,
}

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<ModelEvent>> + Send>>;

pub trait Provider: Send + Sync {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream;
}

pub fn create_provider(
    provider_type: &str,
    base_url: String,
    api_key: String,
    model: String,
) -> Box<dyn Provider> {
    match provider_type.trim().to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => Box::new(AnthropicProvider::new(base_url, api_key, model)),
        "openai-responses" | "responses" => {
            Box::new(OpenAiResponsesProvider::new(base_url, api_key, model))
        }
        _ => Box::new(OpenAiChatProvider::new(base_url, api_key, model)),
    }
}
