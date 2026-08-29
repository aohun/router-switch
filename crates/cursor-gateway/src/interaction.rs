use crate::{
    model::{ToolCall, Usage},
    proto::agent::v1 as pb,
    GatewayError, Result,
};

pub fn server_interaction(message: pb::interaction_update::Message) -> pb::AgentServerMessage {
    pb::AgentServerMessage {
        ttft_breakdown: None,
        message: Some(pb::agent_server_message::Message::InteractionUpdate(
            pb::InteractionUpdate {
                message: Some(message),
            },
        )),
    }
}

pub fn heartbeat() -> pb::AgentServerMessage {
    server_interaction(pb::interaction_update::Message::Heartbeat(
        pb::HeartbeatUpdate {},
    ))
}

pub fn text_delta(text: String) -> pb::AgentServerMessage {
    server_interaction(pb::interaction_update::Message::TextDelta(
        pb::TextDeltaUpdate {
            text,
            is_server_notice: false,
        },
    ))
}

pub fn thinking_delta(text: String) -> pb::AgentServerMessage {
    server_interaction(pb::interaction_update::Message::ThinkingDelta(
        pb::ThinkingDeltaUpdate {
            text,
            thinking_style: Some(pb::ThinkingStyle::Default as i32),
        },
    ))
}

pub fn turn_ended(usage: Option<Usage>) -> pb::AgentServerMessage {
    server_interaction(pb::interaction_update::Message::TurnEnded(
        pb::TurnEndedUpdate {
            input_tokens: usage
                .as_ref()
                .and_then(|u| u.input_tokens.map(|v| v as i64)),
            output_tokens: usage
                .as_ref()
                .and_then(|u| u.output_tokens.map(|v| v as i64)),
            cache_read_tokens: None,
            cache_write_tokens: None,
            reasoning_tokens: None,
        },
    ))
}

pub fn normalized(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn tool_placeholder(name: &str, call_id: &str) -> Result<pb::ToolCall> {
    use pb::tool_call::Tool;
    let tool = match normalized(name).as_str() {
        "shell" => Tool::ShellToolCall(pb::ShellToolCall::default()),
        "delete" => Tool::DeleteToolCall(pb::DeleteToolCall::default()),
        "glob" => Tool::GlobToolCall(pb::GlobToolCall::default()),
        "grep" => Tool::GrepToolCall(pb::GrepToolCall::default()),
        "read" => Tool::ReadToolCall(pb::ReadToolCall::default()),
        "todowrite" => Tool::UpdateTodosToolCall(pb::UpdateTodosToolCall::default()),
        "strreplace" | "editnotebook" | "write" => Tool::EditToolCall(pb::EditToolCall::default()),
        "readlints" => Tool::ReadLintsToolCall(pb::ReadLintsToolCall::default()),
        "callmcptool" | "semblesearch" | "semblefindrelated" | "semanticsearch" => {
            Tool::McpToolCall(pb::McpToolCall::default())
        }
        "createplan" => Tool::CreatePlanToolCall(pb::CreatePlanToolCall::default()),
        "websearch" => Tool::WebSearchToolCall(pb::WebSearchToolCall::default()),
        "task" => Tool::TaskToolCall(pb::TaskToolCall::default()),
        "fetchmcpresource" => Tool::ReadMcpResourceToolCall(pb::ReadMcpResourceToolCall::default()),
        "askquestion" => Tool::AskQuestionToolCall(pb::AskQuestionToolCall::default()),
        "webfetch" => Tool::WebFetchToolCall(pb::WebFetchToolCall::default()),
        "switchmode" => Tool::SwitchModeToolCall(pb::SwitchModeToolCall::default()),
        "generateimage" => Tool::GenerateImageToolCall(pb::GenerateImageToolCall::default()),
        "updatecurrentstep" => {
            Tool::CommunicateUpdateToolCall(pb::CommunicateUpdateToolCall::default())
        }
        "getmcptools" => Tool::GetMcpToolsToolCall(pb::GetMcpToolsToolCall::default()),
        _ => return Err(GatewayError::Protocol(format!("unsupported tool: {name}"))),
    };
    Ok(pb::ToolCall {
        hook_additional_contexts: Vec::new(),
        tool_call_id: Some(call_id.into()),
        started_at_ms: None,
        completed_at_ms: None,
        tool: Some(tool),
    })
}

pub fn tool_started(call: &ToolCall) -> Result<pb::AgentServerMessage> {
    Ok(server_interaction(
        pb::interaction_update::Message::ToolCallStarted(pb::ToolCallStartedUpdate {
            call_id: call.call_id.clone(),
            tool_call: Some(tool_placeholder(&call.name, &call.call_id)?),
            model_call_id: call.model_call_id.clone(),
        }),
    ))
}

pub fn tool_completed(call: &ToolCall, tool_call: pb::ToolCall) -> pb::AgentServerMessage {
    server_interaction(pb::interaction_update::Message::ToolCallCompleted(
        pb::ToolCallCompletedUpdate {
            call_id: call.call_id.clone(),
            tool_call: Some(tool_call),
            model_call_id: call.model_call_id.clone(),
        },
    ))
}

pub fn partial_tool_call(call: &ToolCall, delta: &str) -> Result<pb::AgentServerMessage> {
    Ok(server_interaction(
        pb::interaction_update::Message::PartialToolCall(pb::PartialToolCallUpdate {
            call_id: call.call_id.clone(),
            tool_call: Some(tool_placeholder(&call.name, &call.call_id)?),
            args_text_delta: delta.into(),
            model_call_id: call.model_call_id.clone(),
        }),
    ))
}

pub fn web_search_query(id: u32, call: &ToolCall) -> Result<pb::AgentServerMessage> {
    let search_term = call
        .arguments
        .get("search_term")
        .or_else(|| call.arguments.get("query"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| GatewayError::Protocol("WebSearch is missing search_term".into()))?
        .to_string();
    Ok(pb::AgentServerMessage {
        ttft_breakdown: None,
        message: Some(pb::agent_server_message::Message::InteractionQuery(
            pb::InteractionQuery {
                id,
                query: Some(pb::interaction_query::Query::WebSearchRequestQuery(
                    pb::WebSearchRequestQuery {
                        args: Some(pb::WebSearchArgs {
                            search_term,
                            tool_call_id: call.call_id.clone(),
                        }),
                    },
                )),
            },
        )),
    })
}

pub fn web_fetch_query(id: u32, call: &ToolCall) -> Result<pb::AgentServerMessage> {
    let url = call
        .arguments
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| GatewayError::Protocol("WebFetch is missing url".into()))?
        .to_string();
    Ok(pb::AgentServerMessage {
        ttft_breakdown: None,
        message: Some(pb::agent_server_message::Message::InteractionQuery(
            pb::InteractionQuery {
                id,
                query: Some(pb::interaction_query::Query::WebFetchRequestQuery(
                    pb::WebFetchRequestQuery {
                        args: Some(pb::WebFetchArgs {
                            url,
                            tool_call_id: call.call_id.clone(),
                        }),
                        skip_approval: false,
                        smart_mode_approval: None,
                    },
                )),
            },
        )),
    })
}
