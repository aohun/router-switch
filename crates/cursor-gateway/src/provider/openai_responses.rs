use async_stream::try_stream;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::model::{FinishReason, ModelInvocation, Role, Usage};
use crate::GatewayError;

use super::{merge_extra_params, resolve_provider_url, ModelEvent, Provider, ProviderStream};

pub struct OpenAiResponsesProvider {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    client: reqwest::Client,
}

impl OpenAiResponsesProvider {
    pub fn new(base_url: String, api_key: String, default_model: String) -> Self {
        Self {
            base_url,
            api_key,
            default_model,
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }
}

fn responses_input(invocation: &ModelInvocation) -> Vec<Value> {
    let mut out = Vec::new();
    if !invocation.prompt.instructions.is_empty() {
        out.push(json!({
            "role": "system",
            "content": [{"type": "input_text", "text": invocation.prompt.instructions}],
        }));
    }
    for message in &invocation.history {
        match message.role {
            Role::System => out.push(json!({
                "role": "system",
                "content": [{"type": "input_text", "text": message.content}],
            })),
            Role::User => out.push(json!({
                "role": "user",
                "content": [{"type": "input_text", "text": message.content}],
            })),
            Role::Assistant => {
                let mut content = Vec::new();
                if !message.content.is_empty() {
                    content.push(json!({"type": "output_text", "text": message.content}));
                }
                for call in &message.tool_calls {
                    content.push(json!({
                        "type": "function_call",
                        "call_id": call.call_id,
                        "name": call.name,
                        "arguments": if call.arguments_text.is_empty() {
                            call.arguments.to_string()
                        } else {
                            call.arguments_text.clone()
                        },
                    }));
                }
                out.push(json!({
                    "role": "assistant",
                    "content": content,
                }));
            }
            Role::Tool => out.push(json!({
                "type": "function_call_output",
                "call_id": message.tool_call_id.clone().unwrap_or_default(),
                "output": message.content,
            })),
        }
    }
    out
}

impl Provider for OpenAiResponsesProvider {
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
            let url = resolve_provider_url(&base_url, "openai-responses")?;
            let mut extra_params = invocation.extra_params.clone();
            // Anthropic 托管 WebSearch → Responses 内建 web_search 工具:
            // 搜索在上游执行, 结果与 URL 引用随响应返回
            let hosted_web_search = extra_params
                .as_object_mut()
                .and_then(|obj| obj.remove("hosted_web_search"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let mut body = json!({
                "model": model,
                "stream": true,
                "input": responses_input(&invocation),
                "tools": invocation.prompt.tools.iter().map(|tool| json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                })).collect::<Vec<_>>(),
            });
            if hosted_web_search {
                body["tools"].as_array_mut().expect("tools array").insert(
                    0,
                    json!({ "type": "web_search" }),
                );
            }
            merge_extra_params(&mut body, &extra_params)?;

            let request = client
                .post(&url)
                .header("authorization", format!("Bearer {api_key}"))
                .header("content-type", "application/json")
                .json(&body);

            let response = tokio::select! {
                _ = cancellation.cancelled() => return,
                response = request.send() => response,
            };
            let response = response.map_err(|err| GatewayError::Provider(format!("OpenAI Responses request failed: {err}")))?;
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                Err(GatewayError::Provider(format!("Responses error {status}: {error_text}")))?;
                return;
            }
            let mut event_stream = response.bytes_stream().eventsource();
            let mut tool_index = 0usize;
            // 是否有尚未收尾的 function_call(arguments.done 与 output_item.done
            // 可能先后到达, 只在首次收到时结束该调用)
            let mut tool_call_open = false;
            let mut finish = FinishReason::Stop;
            loop {
                let event = tokio::select! {
                    _ = cancellation.cancelled() => return,
                    event = event_stream.next() => event,
                };
                let Some(event) = event else { break };
                let event = event.map_err(|err| GatewayError::Provider(format!("Responses stream error: {err}")))?;
                let data = event.data.trim();
                if data == "[DONE]" {
                    break;
                }
                let Ok(val) = serde_json::from_str::<Value>(data) else { continue };
                let event_type = val.get("type").and_then(Value::as_str).unwrap_or_default();
                match event_type {
                    "response.output_text.delta" => {
                        if let Some(text) = val.get("delta").and_then(Value::as_str) {
                            if !text.is_empty() {
                                yield ModelEvent::TextDelta(text.to_string());
                            }
                        }
                    }
                    "response.reasoning.delta" | "response.reasoning_summary_text.delta" => {
                        if let Some(text) = val.get("delta").and_then(Value::as_str) {
                            if !text.is_empty() {
                                yield ModelEvent::ThinkingDelta(text.to_string());
                            }
                        }
                    }
                    "response.output_item.added" => {
                        let item = val.get("item").unwrap_or(&Value::Null);
                        if item.get("type").and_then(Value::as_str) == Some("web_search_call") {
                            // 托管搜索在上游执行: 把搜索动作转成可见文本,
                            // 结果与 URL 引用包含在其后的模型输出里
                            let query = item
                                .pointer("/action/query")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            if !query.is_empty() {
                                yield ModelEvent::TextDelta(format!(
                                    "\n[Web Search: {query}]\n"
                                ));
                            }
                            continue;
                        }
                        if item.get("type").and_then(Value::as_str) == Some("function_call") {
                            let call_id = item
                                .get("call_id")
                                .or_else(|| item.get("id"))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let name = item.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                            yield ModelEvent::ToolCallStart {
                                index: tool_index,
                                call_id,
                                name,
                            };
                            tool_call_open = true;
                            finish = FinishReason::ToolUse;
                        }
                    }
                    "response.function_call_arguments.delta" => {
                        if let Some(delta) = val.get("delta").and_then(Value::as_str) {
                            yield ModelEvent::ToolCallArgumentsDelta {
                                index: tool_index,
                                delta: delta.to_string(),
                            };
                        }
                    }
                    "response.function_call_arguments.done" | "response.output_item.done" => {
                        let item = val.get("item").unwrap_or(&Value::Null);
                        let is_function_call =
                            item.get("type").and_then(Value::as_str) == Some("function_call")
                                || event_type == "response.function_call_arguments.done";
                        if is_function_call && tool_call_open {
                            yield ModelEvent::ToolCallEnd { index: tool_index };
                            tool_index += 1;
                            tool_call_open = false;
                        }
                    }
                    "response.completed" => {
                        if let Some(usage) = val.pointer("/response/usage") {
                            yield ModelEvent::Usage(Usage {
                                input_tokens: usage.get("input_tokens").and_then(Value::as_u64),
                                output_tokens: usage.get("output_tokens").and_then(Value::as_u64),
                            });
                        }
                        break;
                    }
                    _ => {}
                }
            }
            yield ModelEvent::Done(finish);
        })
    }
}
