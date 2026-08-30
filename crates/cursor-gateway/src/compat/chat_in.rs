//! OpenAI Chat Completions inbound dialect: what Grok CLI / OpenCode / Pi send us.

use serde_json::{json, Map, Value};

use crate::model::{FinishReason, PromptSpec, ProviderMessage, ToolCall, ToolDefinition, Usage};
use crate::provider::{ModelEvent, ProviderStream};
use crate::Result;

use super::{sse_data, take_extra, CollectedResponse, DialectRenderer, ParsedInbound};

pub(crate) fn parse(body: &Value) -> Result<ParsedInbound> {
    let model = super::str_field(body, "model");
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);

    let mut instructions = String::new();
    let mut history: Vec<ProviderMessage> = Vec::new();
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        for message in messages {
            let role = super::str_field(message, "role");
            match role.as_str() {
                "system" | "developer" => {
                    let text = flatten_content(message.get("content").unwrap_or(&Value::Null));
                    if !text.is_empty() {
                        if !instructions.is_empty() {
                            instructions.push_str("\n\n");
                        }
                        instructions.push_str(&text);
                    }
                }
                "user" => {
                    let text = flatten_content(message.get("content").unwrap_or(&Value::Null));
                    history.push(ProviderMessage::user(text));
                }
                "assistant" => {
                    let text = message
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let mut tool_calls = Vec::new();
                    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                        for call in calls {
                            let function = call.get("function").unwrap_or(&Value::Null);
                            let call_id = super::str_field(call, "id");
                            let arguments_text = super::str_field(function, "arguments");
                            let arguments: Value = serde_json::from_str(&arguments_text)
                                .unwrap_or_else(|_| Value::String(arguments_text.clone()));
                            tool_calls.push(ToolCall {
                                index: tool_calls.len(),
                                model_call_id: call_id.clone(),
                                call_id,
                                name: super::str_field(function, "name"),
                                arguments_text,
                                arguments,
                            });
                        }
                    }
                    history.push(ProviderMessage::assistant(text, tool_calls));
                }
                "tool" => {
                    let call_id = super::str_field(message, "tool_call_id");
                    let text = flatten_content(message.get("content").unwrap_or(&Value::Null));
                    history.push(ProviderMessage::tool(call_id, text));
                }
                _ => {}
            }
        }
    }

    let tools = body
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    let function = tool.get("function").unwrap_or(tool);
                    let name = super::str_field(function, "name");
                    if name.is_empty() {
                        return None;
                    }
                    Some(ToolDefinition {
                        name,
                        description: super::str_field(function, "description"),
                        parameters: function
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let extra = take_extra(
        body,
        &[
            "max_tokens",
            "max_completion_tokens",
            "temperature",
            "top_p",
        ],
    );
    Ok(ParsedInbound {
        model,
        prompt: PromptSpec {
            instructions,
            tools,
        },
        history,
        stream,
        extra,
    })
}

fn flatten_content(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

fn finish_reason(finish: FinishReason) -> &'static str {
    match finish {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ToolUse => "tool_calls",
    }
}

pub(crate) struct ChatRenderer {
    model: String,
    id: String,
    created: i64,
    usage: Option<Usage>,
    finish: FinishReason,
    finished: bool,
}

impl ChatRenderer {
    pub(crate) fn new(model: String) -> Self {
        Self {
            model,
            id: format!("chatcmpl-{}", super::new_id()),
            created: chrono::Utc::now().timestamp(),
            usage: None,
            finish: FinishReason::Stop,
            finished: false,
        }
    }

    fn chunk(&self, delta: Value, finish: Option<&str>) -> String {
        sse_data(&json!({
            "id": self.id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
        }))
    }

    fn final_frames(&mut self) -> Vec<String> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        let finish = finish_reason(self.finish);
        let mut frames = vec![self.chunk(json!({}), Some(finish))];
        if let Some(usage) = &self.usage {
            frames.push(sse_data(&json!({
                "id": self.id,
                "object": "chat.completion.chunk",
                "created": self.created,
                "model": self.model,
                "choices": [],
                "usage": {
                    "prompt_tokens": usage.input_tokens.unwrap_or(0),
                    "completion_tokens": usage.output_tokens.unwrap_or(0),
                    "total_tokens": usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0),
                },
            })));
        }
        frames.push("data: [DONE]\n\n".to_string());
        frames
    }
}

impl DialectRenderer for ChatRenderer {
    fn start_frames(&mut self) -> Vec<String> {
        vec![self.chunk(json!({"role": "assistant", "content": ""}), None)]
    }

    fn event_frames(&mut self, event: &ModelEvent) -> Vec<String> {
        match event {
            ModelEvent::TextDelta(text) => vec![self.chunk(json!({"content": text}), None)],
            ModelEvent::ThinkingDelta(text) => {
                vec![self.chunk(json!({"reasoning_content": text}), None)]
            }
            ModelEvent::ToolCallStart {
                index,
                call_id,
                name,
            } => vec![self.chunk(
                json!({"tool_calls": [{
                    "index": index,
                    "id": call_id,
                    "type": "function",
                    "function": {"name": name, "arguments": ""},
                }]}),
                None,
            )],
            ModelEvent::ToolCallArgumentsDelta { index, delta } => vec![self.chunk(
                json!({"tool_calls": [{
                    "index": index,
                    "function": {"arguments": delta},
                }]}),
                None,
            )],
            ModelEvent::ToolCallEnd { .. } => Vec::new(),
            ModelEvent::Usage(usage) => {
                self.usage = Some(usage.clone());
                Vec::new()
            }
            ModelEvent::Done(finish) => {
                self.finish = *finish;
                Vec::new()
            }
        }
    }

    fn finish_frames(&mut self) -> Vec<String> {
        self.final_frames()
    }

    fn error_frames(&mut self, message: &str) -> Vec<String> {
        self.finished = true;
        vec![
            sse_data(&json!({"error": {"message": message, "type": "api_error"}})),
            "data: [DONE]\n\n".to_string(),
        ]
    }
}

pub(crate) fn stream_body(model: String, events: ProviderStream) -> axum::body::Body {
    super::stream_body(ChatRenderer::new(model), events)
}

pub(crate) fn complete_message(model: &str, collected: &CollectedResponse) -> Value {
    let mut message = Map::new();
    message.insert("role".into(), json!("assistant"));
    if !collected.thinking.is_empty() {
        message.insert("reasoning_content".into(), json!(collected.thinking));
    }
    message.insert("content".into(), json!(collected.text));
    if !collected.tools.is_empty() {
        let calls: Vec<Value> = collected
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "id": tool.call_id,
                    "type": "function",
                    "function": {"name": tool.name, "arguments": tool.arguments},
                })
            })
            .collect();
        message.insert("tool_calls".into(), json!(calls));
    }
    let input_tokens = collected.usage.input_tokens.unwrap_or(0);
    let output_tokens = collected.usage.output_tokens.unwrap_or(0);
    json!({
        "id": format!("chatcmpl-{}", super::new_id()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason(collected.finish),
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens,
        },
    })
}
