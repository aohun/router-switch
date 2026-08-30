use async_stream::try_stream;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::model::{FinishReason, ModelInvocation, Role, Usage};
use crate::GatewayError;

use super::{merge_extra_params, resolve_provider_url, ModelEvent, Provider, ProviderStream};

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

fn anthropic_messages(invocation: &ModelInvocation) -> (String, Vec<Value>) {
    let mut system = invocation.prompt.instructions.clone();
    let mut messages = Vec::new();
    for message in &invocation.history {
        match message.role {
            Role::System => {
                if !system.is_empty() {
                    system.push_str("\n\n");
                }
                system.push_str(&message.content);
            }
            Role::User => messages.push(json!({
                "role": "user",
                "content": message.content,
            })),
            Role::Assistant => {
                let mut content = Vec::new();
                if !message.content.is_empty() {
                    content.push(json!({"type": "text", "text": message.content}));
                }
                for call in &message.tool_calls {
                    content.push(json!({
                        "type": "tool_use",
                        "id": call.call_id,
                        "name": call.name,
                        "input": call.arguments,
                    }));
                }
                messages.push(json!({
                    "role": "assistant",
                    "content": content,
                }));
            }
            Role::Tool => messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": message.tool_call_id.clone().unwrap_or_default(),
                    "content": message.content,
                }],
            })),
        }
    }
    (system, messages)
}

impl Provider for AnthropicProvider {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let model = if !invocation.model.is_empty() {
            invocation.model.clone()
        } else {
            self.default_model.clone()
        };

        Box::pin(try_stream! {
            let url = resolve_provider_url(&base_url, "anthropic")?;
            let (system, messages) = anthropic_messages(&invocation);
            let mut body = json!({
                "model": model,
                "stream": true,
                "max_tokens": 8192,
                "messages": messages,
                "tools": invocation.prompt.tools.iter().map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters,
                })).collect::<Vec<_>>(),
            });
            if !system.is_empty() {
                body["system"] = Value::String(system);
            }
            merge_extra_params(&mut body, &invocation.extra_params)?;

            let request = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body);

            let response = tokio::select! {
                _ = cancellation.cancelled() => return,
                response = request.send() => response,
            };
            let response = response.map_err(|err| GatewayError::Provider(format!("Anthropic request failed: {err}")))?;
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                Err(GatewayError::Provider(format!("Anthropic error {status}: {error_text}")))?;
                return;
            }
            let mut event_stream = response.bytes_stream().eventsource();
            let mut tool_index = 0usize;
            let mut current_tool_index = None;
            let mut finish = FinishReason::Stop;
            loop {
                let event = tokio::select! {
                    _ = cancellation.cancelled() => return,
                    event = event_stream.next() => event,
                };
                let Some(event) = event else { break };
                let event = event.map_err(|err| GatewayError::Provider(format!("Anthropic stream error: {err}")))?;
                let Ok(val) = serde_json::from_str::<Value>(&event.data) else { continue };
                let event_type = val.get("type").and_then(Value::as_str).unwrap_or_default();
                match event_type {
                    "content_block_start" => {
                        let block = val.get("content_block").unwrap_or(&Value::Null);
                        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                            let call_id = block.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
                            let name = block.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                            current_tool_index = Some(tool_index);
                            yield ModelEvent::ToolCallStart {
                                index: tool_index,
                                call_id,
                                name,
                            };
                            tool_index += 1;
                            finish = FinishReason::ToolUse;
                        }
                    }
                    "content_block_delta" => {
                        let delta = val.get("delta").unwrap_or(&Value::Null);
                        match delta.get("type").and_then(Value::as_str) {
                            Some("text_delta") => {
                                if let Some(text) = delta.get("text").and_then(Value::as_str) {
                                    if !text.is_empty() {
                                        yield ModelEvent::TextDelta(text.to_string());
                                    }
                                }
                            }
                            Some("thinking_delta") | Some("reasoning_delta") => {
                                if let Some(text) = delta
                                    .get("thinking")
                                    .or_else(|| delta.get("text"))
                                    .and_then(Value::as_str)
                                {
                                    if !text.is_empty() {
                                        yield ModelEvent::ThinkingDelta(text.to_string());
                                    }
                                }
                            }
                            Some("input_json_delta") => {
                                if let (Some(index), Some(partial)) = (
                                    current_tool_index,
                                    delta.get("partial_json").and_then(Value::as_str),
                                ) {
                                    yield ModelEvent::ToolCallArgumentsDelta {
                                        index,
                                        delta: partial.to_string(),
                                    };
                                }
                            }
                            _ => {}
                        }
                    }
                    "content_block_stop" => {
                        if let Some(index) = current_tool_index.take() {
                            yield ModelEvent::ToolCallEnd { index };
                        }
                    }
                    "message_delta" => {
                        if let Some(usage) = val.get("usage") {
                            yield ModelEvent::Usage(Usage {
                                input_tokens: usage.get("input_tokens").and_then(Value::as_u64),
                                output_tokens: usage.get("output_tokens").and_then(Value::as_u64),
                            });
                        }
                        if val
                            .pointer("/delta/stop_reason")
                            .and_then(Value::as_str)
                            == Some("tool_use")
                        {
                            finish = FinishReason::ToolUse;
                        }
                    }
                    "message_stop" => break,
                    _ => {}
                }
            }
            yield ModelEvent::Done(finish);
        })
    }
}
