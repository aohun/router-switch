//! OpenAI Responses inbound dialect: what Codex sends us.

use serde_json::{json, Value};

use crate::model::{PromptSpec, ProviderMessage, ToolCall, ToolDefinition};
use crate::provider::{ModelEvent, ProviderStream};
use crate::Result;

use super::{sse_event, take_extra, CollectedResponse, DialectRenderer, ParsedInbound};

pub(crate) fn parse(body: &Value) -> Result<ParsedInbound> {
    let model = super::str_field(body, "model");
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);

    let mut instructions = super::str_field(body, "instructions");
    let mut history: Vec<ProviderMessage> = Vec::new();
    match body.get("input") {
        Some(Value::String(text)) => history.push(ProviderMessage::user(text.clone())),
        Some(Value::Array(items)) => {
            for item in items {
                match super::str_field(item, "type").as_str() {
                    "function_call" => {
                        let arguments_text = super::str_field(item, "arguments");
                        let arguments: Value = serde_json::from_str(&arguments_text)
                            .unwrap_or_else(|_| Value::String(arguments_text.clone()));
                        let call_id = super::str_field(item, "call_id");
                        history.push(ProviderMessage::assistant(
                            String::new(),
                            vec![ToolCall {
                                index: 0,
                                model_call_id: call_id.clone(),
                                call_id,
                                name: super::str_field(item, "name"),
                                arguments_text: arguments.to_string(),
                                arguments,
                            }],
                        ));
                    }
                    "function_call_output" => {
                        let call_id = super::str_field(item, "call_id");
                        let output = match item.get("output") {
                            Some(Value::String(text)) => text.clone(),
                            Some(other) => other.to_string(),
                            None => String::new(),
                        };
                        history.push(ProviderMessage::tool(call_id, output));
                    }
                    "reasoning" | "item_reference" => {}
                    _ => {
                        let text = flatten_content(item.get("content").unwrap_or(&Value::Null));
                        match super::str_field(item, "role").as_str() {
                            "assistant" => {
                                history.push(ProviderMessage::assistant(text, Vec::new()))
                            }
                            "system" | "developer" => {
                                if !text.is_empty() {
                                    if !instructions.is_empty() {
                                        instructions.push_str("\n\n");
                                    }
                                    instructions.push_str(&text);
                                }
                            }
                            _ => history.push(ProviderMessage::user(text)),
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let tools = body
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    if super::str_field(tool, "type") != "function" {
                        return None;
                    }
                    let name = super::str_field(tool, "name");
                    if name.is_empty() {
                        return None;
                    }
                    Some(ToolDefinition {
                        name,
                        description: super::str_field(tool, "description"),
                        parameters: tool
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let extra = take_extra(body, &["max_output_tokens", "temperature", "top_p"]);
    Ok(ParsedInbound {
        model,
        prompt: PromptSpec {
            instructions,
            tools,
        },
        history,
        stream,
        extra,
        hosted_web_search: false,
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

/// Current open output item while streaming: assistant text, reasoning, or a
/// function call. Indices follow the Responses `output_index` field.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenItem {
    None,
    Message,
    Reasoning,
    Tool,
}

pub(crate) struct ResponsesRenderer {
    model: String,
    response_id: String,
    message_id: String,
    reasoning_id: String,
    seq: u32,
    output_index: usize,
    open_item: OpenItem,
    output_items: Vec<Value>,
    text_accum: String,
    tools: Vec<Value>,
    active_tool: Option<(String, String, String)>, // item_id, call_id, arguments
    input_tokens: u64,
    output_tokens: u64,
    finished: bool,
}

impl ResponsesRenderer {
    pub(crate) fn new(model: String) -> Self {
        let id = super::new_id();
        Self {
            model,
            response_id: format!("resp_{id}"),
            message_id: format!("msg_{id}"),
            reasoning_id: format!("rs_{id}"),
            seq: 0,
            output_index: 0,
            open_item: OpenItem::None,
            output_items: Vec::new(),
            text_accum: String::new(),
            tools: Vec::new(),
            active_tool: None,
            input_tokens: 0,
            output_tokens: 0,
            finished: false,
        }
    }

    fn event(&mut self, event_type: &str, mut payload: Value) -> String {
        if let Some(object) = payload.as_object_mut() {
            object.insert("type".into(), json!(event_type));
            object.insert("sequence_number".into(), json!(self.seq));
        }
        self.seq += 1;
        sse_event(event_type, &payload)
    }

    fn close_open_item(&mut self) -> Vec<String> {
        match self.open_item {
            OpenItem::None => Vec::new(),
            OpenItem::Message => {
                self.open_item = OpenItem::None;
                let item = json!({
                    "type": "message",
                    "id": self.message_id,
                    "role": "assistant",
                    "status": "completed",
                    "content": [{"type": "output_text", "text": std::mem::take(&mut self.text_accum)}],
                });
                let index = self.output_index;
                self.output_index += 1;
                self.output_items.push(item.clone());
                vec![self.event(
                    "response.output_item.done",
                    json!({"output_index": index, "item": item}),
                )]
            }
            OpenItem::Reasoning => {
                self.open_item = OpenItem::None;
                let item = json!({"type": "reasoning", "id": self.reasoning_id, "summary": []});
                let index = self.output_index;
                self.output_index += 1;
                self.output_items.push(item.clone());
                vec![self.event(
                    "response.output_item.done",
                    json!({"output_index": index, "item": item}),
                )]
            }
            OpenItem::Tool => {
                self.open_item = OpenItem::None;
                let Some((item_id, call_id, arguments)) = self.active_tool.take() else {
                    return Vec::new();
                };
                let name = self
                    .tools
                    .last()
                    .and_then(|item| item.get("name").and_then(Value::as_str))
                    .unwrap_or_default()
                    .to_string();
                let item = json!({
                    "type": "function_call",
                    "id": item_id,
                    "call_id": call_id,
                    "name": name,
                    "arguments": arguments,
                    "status": "completed",
                });
                let index = self.output_index;
                self.output_index += 1;
                self.output_items.push(item.clone());
                vec![
                    self.event(
                        "response.function_call_arguments.done",
                        json!({"item_id": item_id, "output_index": index, "arguments": arguments}),
                    ),
                    self.event(
                        "response.output_item.done",
                        json!({"output_index": index, "item": item}),
                    ),
                ]
            }
        }
    }

    fn usage_payload(&self) -> Value {
        json!({
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "total_tokens": self.input_tokens + self.output_tokens,
        })
    }

    pub(crate) fn complete_response(model: &str, collected: &CollectedResponse) -> Value {
        let mut output = Vec::new();
        if !collected.thinking.is_empty() {
            output.push(json!({
                "type": "reasoning",
                "id": format!("rs_{}", super::new_id()),
                "summary": [{"type": "summary_text", "text": collected.thinking}],
            }));
        }
        if !collected.text.is_empty() {
            output.push(json!({
                "type": "message",
                "id": format!("msg_{}", super::new_id()),
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": collected.text}],
            }));
        }
        for tool in &collected.tools {
            output.push(json!({
                "type": "function_call",
                "id": format!("fc_{}", super::new_id()),
                "call_id": tool.call_id,
                "name": tool.name,
                "arguments": tool.arguments,
                "status": "completed",
            }));
        }
        let input_tokens = collected.usage.input_tokens.unwrap_or(0);
        let output_tokens = collected.usage.output_tokens.unwrap_or(0);
        json!({
            "id": format!("resp_{}", super::new_id()),
            "object": "response",
            "created_at": chrono::Utc::now().timestamp(),
            "status": "completed",
            "model": model,
            "output": output,
            "parallel_tool_calls": true,
            "usage": {
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "total_tokens": input_tokens + output_tokens,
            },
        })
    }
}

impl DialectRenderer for ResponsesRenderer {
    fn start_frames(&mut self) -> Vec<String> {
        let response = json!({
            "id": self.response_id,
            "object": "response",
            "created_at": chrono::Utc::now().timestamp(),
            "status": "in_progress",
            "model": self.model,
            "output": [],
            "usage": Value::Null,
        });
        vec![self.event("response.created", json!({"response": response}))]
    }

    fn event_frames(&mut self, event: &ModelEvent) -> Vec<String> {
        match event {
            ModelEvent::TextDelta(text) => {
                let mut frames = Vec::new();
                if self.open_item != OpenItem::Message {
                    frames.extend(self.close_open_item());
                    self.open_item = OpenItem::Message;
                    let item = json!({
                        "type": "message",
                        "id": self.message_id,
                        "role": "assistant",
                        "status": "in_progress",
                        "content": [],
                    });
                    frames.push(self.event(
                        "response.output_item.added",
                        json!({"output_index": self.output_index, "item": item}),
                    ));
                }
                self.text_accum.push_str(text);
                frames.push(self.event(
                    "response.output_text.delta",
                    json!({
                        "item_id": self.message_id,
                        "output_index": self.output_index,
                        "content_index": 0,
                        "delta": text,
                    }),
                ));
                frames
            }
            ModelEvent::ThinkingDelta(text) => {
                let mut frames = Vec::new();
                if self.open_item != OpenItem::Reasoning {
                    frames.extend(self.close_open_item());
                    self.open_item = OpenItem::Reasoning;
                    let item = json!({"type": "reasoning", "id": self.reasoning_id, "summary": []});
                    frames.push(self.event(
                        "response.output_item.added",
                        json!({"output_index": self.output_index, "item": item}),
                    ));
                }
                frames.push(self.event(
                    "response.reasoning_summary_text.delta",
                    json!({
                        "item_id": self.reasoning_id,
                        "output_index": self.output_index,
                        "summary_index": 0,
                        "delta": text,
                    }),
                ));
                frames
            }
            ModelEvent::ToolCallStart { call_id, name, .. } => {
                let mut frames = self.close_open_item();
                self.open_item = OpenItem::Tool;
                let item_id = format!("fc_{}", super::new_id());
                let item = json!({
                    "type": "function_call",
                    "id": item_id,
                    "call_id": call_id,
                    "name": name,
                    "arguments": "",
                    "status": "in_progress",
                });
                frames.push(self.event(
                    "response.output_item.added",
                    json!({"output_index": self.output_index, "item": item}),
                ));
                self.tools.push(json!({"name": name}));
                self.active_tool = Some((item_id, call_id.clone(), String::new()));
                frames
            }
            ModelEvent::ToolCallArgumentsDelta { delta, .. } => {
                let Some((item_id, _, arguments)) = self.active_tool.as_mut() else {
                    return Vec::new();
                };
                arguments.push_str(delta);
                let item_id = item_id.clone();
                vec![self.event(
                    "response.function_call_arguments.delta",
                    json!({
                        "item_id": item_id,
                        "output_index": self.output_index,
                        "delta": delta,
                    }),
                )]
            }
            ModelEvent::ToolCallEnd { .. } => Vec::new(),
            ModelEvent::Usage(usage) => {
                if let Some(value) = usage.input_tokens {
                    self.input_tokens = value;
                }
                if let Some(value) = usage.output_tokens {
                    self.output_tokens = value;
                }
                Vec::new()
            }
            ModelEvent::Done(_) => Vec::new(),
        }
    }

    fn finish_frames(&mut self) -> Vec<String> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        let mut frames = self.close_open_item();
        let response = json!({
            "id": self.response_id,
            "object": "response",
            "created_at": chrono::Utc::now().timestamp(),
            "status": "completed",
            "model": self.model,
            "output": self.output_items,
            "usage": self.usage_payload(),
        });
        frames.push(self.event("response.completed", json!({"response": response})));
        frames
    }

    fn error_frames(&mut self, message: &str) -> Vec<String> {
        self.finished = true;
        vec![self.event(
            "error",
            json!({"code": "api_error", "message": message, "param": Value::Null}),
        )]
    }
}

pub(crate) fn stream_body(model: String, events: ProviderStream) -> axum::body::Body {
    super::stream_body(ResponsesRenderer::new(model), events)
}

pub(crate) fn complete_message(model: &str, collected: &CollectedResponse) -> Value {
    ResponsesRenderer::complete_response(model, collected)
}
