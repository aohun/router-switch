use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::GatewayError;

use super::{ModelEvent, ModelInvocation, Provider, ProviderStream};

pub struct AnthropicProvider {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_key: String, default_model: String) -> Self {
        Self {
            base_url,
            api_key,
            default_model,
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }
}

fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}{}",
        base.trim_end_matches('/'),
        if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        }
    )
}

impl Provider for AnthropicProvider {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream {
        let client = self.client.clone();
        let url = join_url(&self.base_url, "/v1/messages");
        let api_key = self.api_key.clone();
        let model = if !invocation.model.is_empty() {
            invocation.model
        } else {
            self.default_model.clone()
        };

        let messages = if !invocation.messages.is_empty() {
            invocation
                .messages
                .into_iter()
                .map(|m| json!({"role": m.role, "content": m.content}))
                .collect::<Vec<_>>()
        } else {
            vec![json!({"role": "user", "content": invocation.prompt})]
        };

        let body = json!({
            "model": model,
            "max_tokens": 8192,
            "stream": true,
            "messages": messages
        });

        let stream = async_stream::stream! {
            let request = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body);

            let response = match request.send().await {
                Ok(res) => res,
                Err(err) => {
                    yield Err(GatewayError::Provider(format!("Anthropic request failed: {err}")));
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                yield Err(GatewayError::Provider(format!(
                    "Anthropic error {status}: {error_text}"
                )));
                return;
            }

            let mut event_stream = response.bytes_stream().eventsource();
            while let Some(event_res) = event_stream.next().await {
                if cancellation.is_cancelled() {
                    return;
                }
                match event_res {
                    Ok(event) => {
                        let event_type = event.event.as_str();
                        let data = event.data.trim();
                        if event_type == "message_stop" {
                            yield Ok(ModelEvent::Done);
                            break;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            if event_type == "content_block_delta" {
                                if let Some(delta) = val.get("delta") {
                                    if let Some(text) = delta.get("text").and_then(Value::as_str) {
                                        if !text.is_empty() {
                                            yield Ok(ModelEvent::TextDelta(text.to_string()));
                                        }
                                    } else if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                                        if !thinking.is_empty() {
                                            yield Ok(ModelEvent::ThinkingDelta(thinking.to_string()));
                                        }
                                    }
                                }
                            } else if event_type == "message_delta" {
                                if let Some(usage) = val.get("usage") {
                                    let output = usage.get("output_tokens").and_then(Value::as_u64);
                                    yield Ok(ModelEvent::Usage {
                                        input_tokens: None,
                                        output_tokens: output,
                                    });
                                }
                            }
                        }
                    }
                    Err(err) => {
                        yield Err(GatewayError::Provider(format!("Anthropic stream error: {err}")));
                        return;
                    }
                }
            }
        };

        Box::pin(stream)
    }
}
