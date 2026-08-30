//! Anthropic Messages inbound dialect: what Claude Code / ZCode send us.

use serde_json::{json, Value};

use crate::model::{FinishReason, PromptSpec, ProviderMessage, ToolCall, ToolDefinition};
use crate::provider::{ModelEvent, ProviderStream};
use crate::Result;

use super::{sse_event, take_extra, CollectedResponse, DialectRenderer, ParsedInbound};

pub(crate) fn parse(body: &Value) -> Result<ParsedInbound> {
    let model = super::str_field(body, "model");
    let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);

    let mut instructions = String::new();
    match body.get("system") {
        Some(Value::String(text)) => instructions = text.clone(),
        Some(Value::Array(blocks)) => {
            for block in blocks {
                if block.get("type").and_then(Value::as_str) == Some("text") {
                    append_text(&mut instructions, block);
                }
            }
        }
        _ => {}
    }

    let mut history: Vec<ProviderMessage> = Vec::new();
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        for message in messages {
            let is_user = message.get("role").and_then(Value::as_str) != Some("assistant");
            match message.get("content") {
                Some(Value::String(text)) => {
                    history.push(if is_user {
                        ProviderMessage::user(text.clone())
                    } else {
                        ProviderMessage::assistant(text.clone(), Vec::new())
                    });
                }
                Some(Value::Array(blocks)) => {
                    let mut text = String::new();
                    let mut tool_calls: Vec<ToolCall> = Vec::new();
                    for block in blocks {
                        match block.get("type").and_then(Value::as_str) {
                            Some("text") => append_text(&mut text, block),
                            Some("thinking") | Some("redacted_thinking") => {}
                            Some("tool_use") => {
                                let arguments = block.get("input").cloned().unwrap_or(json!({}));
                                let call_id = super::str_field(block, "id");
                                tool_calls.push(ToolCall {
                                    index: tool_calls.len(),
                                    model_call_id: call_id.clone(),
                                    call_id,
                                    name: super::str_field(block, "name"),
                                    arguments_text: arguments.to_string(),
                                    arguments,
                                });
                            }
                            Some("tool_result") => {
                                flush_pending(&mut history, is_user, &mut text, &mut tool_calls);
                                let call_id = super::str_field(block, "tool_use_id");
                                let content =
                                    flatten_content(block.get("content").unwrap_or(&Value::Null));
                                history.push(ProviderMessage::tool(call_id, content));
                            }
                            _ => {}
                        }
                    }
                    flush_pending(&mut history, is_user, &mut text, &mut tool_calls);
                }
                _ => {}
            }
        }
    }

    let tools = parse_tools(body.get("tools"));
    let hosted_web_search = body
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| is_hosted_tool(tool) && super::str_field(tool, "name") == "web_search")
        });
    let extra = take_extra(body, &["max_tokens", "temperature", "top_p", "top_k"]);
    Ok(ParsedInbound {
        model,
        prompt: PromptSpec {
            instructions,
            tools,
        },
        history,
        stream,
        extra,
        hosted_web_search,
    })
}

/// Anthropic 托管工具类型(web_search_20250305 / web_search / computer_* 等):
/// 它们不是 function 工具, 不能当作 function 发给上游
fn is_hosted_tool(tool: &Value) -> bool {
    tool.get("type")
        .and_then(Value::as_str)
        .is_some_and(|tool_type| {
            tool_type.starts_with("web_search")
                || tool_type.starts_with("computer_")
                || tool_type.starts_with("text_editor")
                || tool_type.starts_with("bash_")
        })
}

fn parse_tools(tools: Option<&Value>) -> Vec<ToolDefinition> {
    tools
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter(|tool| !is_hosted_tool(tool))
                .filter_map(|tool| {
                    let name = super::str_field(tool, "name");
                    if name.is_empty() {
                        return None;
                    }
                    Some(ToolDefinition {
                        name,
                        description: super::str_field(tool, "description"),
                        parameters: tool
                            .get("input_schema")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn append_text(target: &mut String, block: &Value) {
    let text = block
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push_str("\n\n");
    }
    target.push_str(text);
}

fn flatten_content(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

fn flush_pending(
    history: &mut Vec<ProviderMessage>,
    is_user: bool,
    text: &mut String,
    tool_calls: &mut Vec<ToolCall>,
) {
    if !tool_calls.is_empty() {
        history.push(ProviderMessage::assistant(
            std::mem::take(text),
            std::mem::take(tool_calls),
        ));
    } else if !text.is_empty() {
        let content = std::mem::take(text);
        history.push(if is_user {
            ProviderMessage::user(content)
        } else {
            ProviderMessage::assistant(content, Vec::new())
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenBlock {
    Text,
    Thinking,
    Tool,
}

pub(crate) struct AnthropicRenderer {
    model: String,
    message_id: String,
    next_block: usize,
    open_block: Option<OpenBlock>,
    input_tokens: u64,
    output_tokens: u64,
    last_finish: FinishReason,
    finished: bool,
}

impl AnthropicRenderer {
    pub(crate) fn new(model: String) -> Self {
        Self {
            model,
            message_id: format!("msg_{}", super::new_id()),
            next_block: 0,
            open_block: None,
            input_tokens: 0,
            output_tokens: 0,
            last_finish: FinishReason::Stop,
            finished: false,
        }
    }

    fn open_block_frames(&mut self, kind: OpenBlock) -> Vec<String> {
        let index = self.next_block;
        self.next_block += 1;
        self.open_block = Some(kind);
        let content_block = match kind {
            OpenBlock::Text => json!({"type": "text", "text": ""}),
            OpenBlock::Thinking => json!({"type": "thinking", "thinking": ""}),
            OpenBlock::Tool => return Vec::new(),
        };
        vec![sse_event(
            "content_block_start",
            &json!({
                "type": "content_block_start",
                "index": index,
                "content_block": content_block,
            }),
        )]
    }

    fn close_open_frames(&mut self) -> Vec<String> {
        if self.open_block.take().is_some() {
            let index = self.next_block - 1;
            vec![sse_event(
                "content_block_stop",
                &json!({"type": "content_block_stop", "index": index}),
            )]
        } else {
            Vec::new()
        }
    }

    fn delta_frames(&mut self, kind: OpenBlock, delta: Value) -> Vec<String> {
        let mut frames = Vec::new();
        if self.open_block != Some(kind) {
            frames.extend(self.close_open_frames());
            frames.extend(self.open_block_frames(kind));
        }
        let index = self.next_block - 1;
        frames.push(sse_event(
            "content_block_delta",
            &json!({
                "type": "content_block_delta",
                "index": index,
                "delta": delta,
            }),
        ));
        frames
    }
}

fn stop_reason(finish: FinishReason) -> &'static str {
    match finish {
        FinishReason::Stop => "end_turn",
        FinishReason::Length => "max_tokens",
        FinishReason::ToolUse => "tool_use",
    }
}

impl DialectRenderer for AnthropicRenderer {
    fn start_frames(&mut self) -> Vec<String> {
        vec![sse_event(
            "message_start",
            &json!({
                "type": "message_start",
                "message": {
                    "id": self.message_id,
                    "type": "message",
                    "role": "assistant",
                    "model": self.model,
                    "content": [],
                    "stop_reason": Value::Null,
                    "stop_sequence": Value::Null,
                    "usage": {"input_tokens": 0, "output_tokens": 0},
                },
            }),
        )]
    }

    fn event_frames(&mut self, event: &ModelEvent) -> Vec<String> {
        match event {
            ModelEvent::TextDelta(text) => {
                self.delta_frames(OpenBlock::Text, json!({"type": "text_delta", "text": text}))
            }
            ModelEvent::ThinkingDelta(text) => self.delta_frames(
                OpenBlock::Thinking,
                json!({"type": "thinking_delta", "thinking": text}),
            ),
            ModelEvent::ToolCallStart { call_id, name, .. } => {
                let mut frames = self.close_open_frames();
                let index = self.next_block;
                self.next_block += 1;
                self.open_block = Some(OpenBlock::Tool);
                frames.push(sse_event(
                    "content_block_start",
                    &json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {"type": "tool_use", "id": call_id, "name": name, "input": {}},
                    }),
                ));
                frames
            }
            ModelEvent::ToolCallArgumentsDelta { delta, .. } => {
                if self.open_block != Some(OpenBlock::Tool) {
                    return Vec::new();
                }
                let index = self.next_block - 1;
                vec![sse_event(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "input_json_delta", "partial_json": delta},
                    }),
                )]
            }
            ModelEvent::ToolCallEnd { .. } => self.close_open_frames(),
            ModelEvent::Usage(usage) => {
                if let Some(value) = usage.input_tokens {
                    self.input_tokens = value;
                }
                if let Some(value) = usage.output_tokens {
                    self.output_tokens = value;
                }
                Vec::new()
            }
            ModelEvent::Done(finish) => {
                self.last_finish = *finish;
                Vec::new()
            }
        }
    }

    fn finish_frames(&mut self) -> Vec<String> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        let mut frames = self.close_open_frames();
        frames.push(sse_event(
            "message_delta",
            &json!({
                "type": "message_delta",
                "delta": {"stop_reason": stop_reason(self.last_finish), "stop_sequence": Value::Null},
                "usage": {"input_tokens": self.input_tokens, "output_tokens": self.output_tokens},
            }),
        ));
        frames.push(sse_event("message_stop", &json!({"type": "message_stop"})));
        frames
    }

    fn error_frames(&mut self, message: &str) -> Vec<String> {
        self.finished = true;
        vec![sse_event(
            "error",
            &json!({"type": "error", "error": {"type": "api_error", "message": message}}),
        )]
    }
}

pub(crate) fn stream_body(model: String, events: ProviderStream) -> axum::body::Body {
    super::stream_body(AnthropicRenderer::new(model), events)
}

pub(crate) fn complete_message(model: &str, collected: &CollectedResponse) -> Value {
    let mut content = Vec::new();
    if !collected.thinking.is_empty() {
        content.push(json!({
            "type": "thinking",
            "thinking": collected.thinking,
            "signature": "",
        }));
    }
    if !collected.text.is_empty() {
        content.push(json!({"type": "text", "text": collected.text}));
    }
    for tool in &collected.tools {
        let input: Value = serde_json::from_str(&tool.arguments).unwrap_or(json!({}));
        content.push(json!({
            "type": "tool_use",
            "id": tool.call_id,
            "name": tool.name,
            "input": input,
        }));
    }
    json!({
        "id": format!("msg_{}", super::new_id()),
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": stop_reason(collected.finish),
        "stop_sequence": Value::Null,
        "usage": {
            "input_tokens": collected.usage.input_tokens.unwrap_or(0),
            "output_tokens": collected.usage.output_tokens.unwrap_or(0),
        },
    })
}
