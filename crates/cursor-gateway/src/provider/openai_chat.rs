use std::collections::BTreeMap;

use async_stream::try_stream;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::model::{FinishReason, ModelInvocation, Role, Usage};
use crate::{GatewayError, Result};

use super::{merge_extra_params, resolve_provider_url, ModelEvent, Provider, ProviderStream};

#[derive(Default)]
struct ChatToolState {
    call_id: String,
    name: String,
    arguments: String,
    started: bool,
}

pub struct OpenAiChatProvider {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    extra_headers: Vec<(String, String)>,
    client: reqwest::Client,
}

impl OpenAiChatProvider {
    pub fn new(
        base_url: String,
        api_key: String,
        default_model: String,
        extra_headers: Vec<(String, String)>,
    ) -> Self {
        Self {
            base_url,
            api_key,
            default_model,
            extra_headers,
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }
}

fn is_codebuddy_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("copilot.tencent.com") || lower.contains("codebuddy.cn")
}

fn sanitize_codebuddy_chat_body(body: &mut Value, model: &str) {
    if let Some(object) = body.as_object_mut() {
        object.remove("stream_options");
        if object
            .get("tools")
            .and_then(Value::as_array)
            .is_some_and(|tools| tools.is_empty())
        {
            object.remove("tools");
        }
        if let Some(max) = object.get("max_tokens").and_then(Value::as_u64) {
            if max > 8192 {
                object.insert("max_tokens".into(), json!(8192u64));
            }
        }
        if let Some(max) = object.get("max_completion_tokens").and_then(Value::as_u64) {
            if max > 8192 {
                object.insert("max_completion_tokens".into(), json!(8192u64));
            }
        }
        match object.get("tool_choice") {
            Some(Value::Object(choice))
                if choice
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t == "function") =>
            {
                object.insert("tool_choice".into(), json!("required"));
            }
            Some(Value::Object(_)) => {
                object.insert("tool_choice".into(), json!("auto"));
            }
            _ => {}
        }
        if model.starts_with("hy3") {
            object.insert("reasoning_effort".into(), json!("high"));
        }
    }
}

fn format_upstream_error(raw: &str) -> String {
    codebuddy_envelope_error(raw).unwrap_or_else(|| raw.chars().take(800).collect())
}

fn codebuddy_envelope_error(raw: &str) -> Option<String> {
    let val: Value = serde_json::from_str(raw.trim()).ok()?;
    codebuddy_value_error(&val)
}

fn codebuddy_value_error(val: &Value) -> Option<String> {
    let code = val.get("code").and_then(Value::as_i64)?;
    if code == 0 {
        return None;
    }
    if val.get("choices").is_some() {
        return None;
    }
    let msg = val
        .get("msg")
        .or_else(|| val.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("upstream error");
    Some(format!("CodeBuddy error {code}: {msg}"))
}

fn openai_messages(invocation: &ModelInvocation) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    if !invocation.prompt.instructions.is_empty() {
        out.push(json!({
            "role": "system",
            "content": invocation.prompt.instructions,
        }));
    }
    for message in &invocation.history {
        match message.role {
            Role::System => out.push(json!({
                "role": "system",
                "content": message.content,
            })),
            Role::User => out.push(json!({
                "role": "user",
                "content": message.content,
            })),
            Role::Assistant => {
                let mut item = json!({
                    "role": "assistant",
                    "content": if message.content.is_empty() { Value::Null } else { Value::String(message.content.clone()) },
                });
                if !message.tool_calls.is_empty() {
                    item["tool_calls"] = Value::Array(
                        message
                            .tool_calls
                            .iter()
                            .map(|call| {
                                json!({
                                    "id": call.call_id,
                                    "type": "function",
                                    "function": {
                                        "name": call.name,
                                        "arguments": if call.arguments_text.is_empty() {
                                            call.arguments.to_string()
                                        } else {
                                            call.arguments_text.clone()
                                        },
                                    }
                                })
                            })
                            .collect(),
                    );
                }
                out.push(item);
            }
            Role::Tool => out.push(json!({
                "role": "tool",
                "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
                "content": message.content,
            })),
        }
    }
    Ok(out)
}

impl Provider for OpenAiChatProvider {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let extra_headers = self.extra_headers.clone();
        let codebuddy = is_codebuddy_url(&base_url);
        let model = if !invocation.model.is_empty() {
            invocation.model.clone()
        } else {
            self.default_model.clone()
        };

        Box::pin(try_stream! {
            let url = resolve_provider_url(&base_url, "openai-chat")?;
            let messages = openai_messages(&invocation)?;
            let tools: Vec<Value> = invocation.prompt.tools.iter().map(|tool| json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            })).collect();
            let mut body = json!({
                "model": model,
                "stream": true,
                "messages": messages,
            });
            if !codebuddy {
                body["stream_options"] = json!({"include_usage": true});
            }
            if !tools.is_empty() {
                body["tools"] = Value::Array(tools);
            }
            merge_extra_params(&mut body, &invocation.extra_params)?;
            if codebuddy {
                sanitize_codebuddy_chat_body(&mut body, &model);
            }

            let mut request = client
                .post(&url)
                .header("authorization", format!("Bearer {api_key}"))
                .header("content-type", "application/json");
            if codebuddy {
                request = request
                    .header("User-Agent", "CLI/2.63.2 CodeBuddy/2.63.2")
                    .header("Origin", "https://www.codebuddy.cn")
                    .header("Referer", "https://www.codebuddy.cn/")
                    .header("X-Requested-With", "XMLHttpRequest")
                    .header("X-Product", "SaaS")
                    .header("Accept", "text/event-stream, application/json, text/plain, */*");
                let has = |name: &str| extra_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
                if !has("X-User-Id") {
                    request = request.header("X-No-User-Id", "1");
                }
                if !has("X-Enterprise-Id") {
                    request = request.header("X-No-Enterprise-Id", "1");
                }
                if !has("X-Domain") {
                    request = request.header("X-No-Department-Info", "1");
                }
            }
            for (name, value) in &extra_headers {
                request = request.header(name.as_str(), value.as_str());
            }
            let request = request.json(&body);

            let response = tokio::select! {
                _ = cancellation.cancelled() => return,
                response = request.send() => response,
            };
            let response = response.map_err(|err| GatewayError::Provider(format!("OpenAI request failed: {err}")))?;
            let status = response.status();
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                Err(GatewayError::Provider(format!(
                    "OpenAI error {status}: {}",
                    format_upstream_error(&error_text)
                )))?;
                return;
            }
            if codebuddy
                && content_type.contains("application/json")
                && !content_type.contains("event-stream")
            {
                let error_text = response.text().await.unwrap_or_default();
                Err(GatewayError::Provider(
                    codebuddy_envelope_error(&error_text)
                        .unwrap_or_else(|| format_upstream_error(&error_text)),
                ))?;
                return;
            }
            let mut event_stream = response.bytes_stream().eventsource();
            let mut tools = BTreeMap::<usize, ChatToolState>::new();
            let mut finish = None;
            loop {
                let event = tokio::select! {
                    _ = cancellation.cancelled() => return,
                    event = event_stream.next() => event,
                };
                let Some(event) = event else { break };
                let event = event.map_err(|err| GatewayError::Provider(format!("SSE stream error: {err}")))?;
                let data = event.data.trim();
                if data == "[DONE]" {
                    break;
                }
                let Ok(val) = serde_json::from_str::<Value>(data) else { continue };
                if let Some(message) = codebuddy_value_error(&val) {
                    Err(GatewayError::Provider(message))?;
                    return;
                }
                if let Some(usage) = val.get("usage").filter(|value| !value.is_null()) {
                    yield ModelEvent::Usage(Usage {
                        input_tokens: usage.get("prompt_tokens").and_then(Value::as_u64),
                        output_tokens: usage.get("completion_tokens").and_then(Value::as_u64),
                    });
                }
                let Some(choice) = val.get("choices").and_then(Value::as_array).and_then(|v| v.first()) else {
                    continue;
                };
                let delta = choice.get("delta").unwrap_or(&Value::Null);
                if let Some(content) = delta.get("content").and_then(Value::as_str).filter(|t| !t.is_empty()) {
                    yield ModelEvent::TextDelta(content.to_string());
                }
                if let Some(reasoning) = delta
                    .get("reasoning_content")
                    .or_else(|| delta.get("reasoning"))
                    .and_then(Value::as_str)
                    .filter(|t| !t.is_empty())
                {
                    yield ModelEvent::ThinkingDelta(reasoning.to_string());
                }
                if let Some(tool_deltas) = delta.get("tool_calls").and_then(Value::as_array) {
                    for (position, tool) in tool_deltas.iter().enumerate() {
                        let index = tool
                            .get("index")
                            .and_then(Value::as_u64)
                            .map_or(position, |index| index as usize);
                        let state = tools.entry(index).or_default();
                        if let Some(id) = tool.get("id").and_then(Value::as_str) {
                            state.call_id = id.to_string();
                        }
                        let function = tool.get("function").unwrap_or(&Value::Null);
                        if let Some(name) = function.get("name").and_then(Value::as_str) {
                            if !name.is_empty() {
                                state.name = name.to_string();
                            }
                        }
                        if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
                            state.arguments.push_str(arguments);
                            if !state.started && !state.name.is_empty() {
                                if state.call_id.is_empty() {
                                    state.call_id = format!("call-{index}");
                                }
                                state.started = true;
                                yield ModelEvent::ToolCallStart {
                                    index,
                                    call_id: state.call_id.clone(),
                                    name: state.name.clone(),
                                };
                            }
                            if state.started {
                                yield ModelEvent::ToolCallArgumentsDelta {
                                    index,
                                    delta: arguments.to_string(),
                                };
                            }
                        } else if !state.started && !state.name.is_empty() {
                            if state.call_id.is_empty() {
                                state.call_id = format!("call-{index}");
                            }
                            state.started = true;
                            yield ModelEvent::ToolCallStart {
                                index,
                                call_id: state.call_id.clone(),
                                name: state.name.clone(),
                            };
                        }
                    }
                }
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish = Some(match reason {
                        "tool_calls" | "function_call" => FinishReason::ToolUse,
                        "length" => FinishReason::Length,
                        _ => FinishReason::Stop,
                    });
                }
            }
            for (index, tool) in &mut tools {
                if !tool.started {
                    if tool.name.is_empty() {
                        Err(GatewayError::Provider("OpenAI Chat tool call is missing name".into()))?;
                    }
                    if tool.call_id.is_empty() {
                        tool.call_id = format!("call-{index}");
                    }
                    tool.started = true;
                    yield ModelEvent::ToolCallStart {
                        index: *index,
                        call_id: tool.call_id.clone(),
                        name: tool.name.clone(),
                    };
                    if !tool.arguments.is_empty() {
                        yield ModelEvent::ToolCallArgumentsDelta {
                            index: *index,
                            delta: tool.arguments.clone(),
                        };
                    }
                }
                yield ModelEvent::ToolCallEnd { index: *index };
            }
            let finish = finish.unwrap_or(if tools.is_empty() {
                FinishReason::Stop
            } else {
                FinishReason::ToolUse
            });
            yield ModelEvent::Done(finish);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_codebuddy_unsupported_fields() {
        let mut body = json!({
            "model": "hy3-preview",
            "stream": true,
            "stream_options": {"include_usage": true},
            "tools": [],
            "max_tokens": 128000,
            "tool_choice": {"type": "function", "function": {"name": "shell"}},
        });
        sanitize_codebuddy_chat_body(&mut body, "hy3-preview");
        assert!(body.get("stream_options").is_none());
        assert!(body.get("tools").is_none());
        assert_eq!(body["max_tokens"], 8192);
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn codebuddy_envelope_error_ignores_success() {
        assert!(codebuddy_envelope_error(r#"{"code":0,"msg":"ok","data":{}}"#).is_none());
        assert_eq!(
            codebuddy_envelope_error(r#"{"code":11101,"msg":"stream required"}"#).as_deref(),
            Some("CodeBuddy error 11101: stream required")
        );
    }
}
