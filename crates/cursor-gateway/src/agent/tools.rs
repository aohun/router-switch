use serde_json::Value;

use crate::{
    agent::AwaitKind,
    interaction::{self, normalized},
    model::{ToolCall, ToolResult},
    proto::agent::v1 as pb,
    sessions::CursorSessionHandle,
    web::{WebFetch, WebSearch},
    GatewayError, Result,
};

pub struct ToolOutcome {
    pub content: String,
    pub is_error: bool,
    pub completed_tool_call: Option<pb::ToolCall>,
}

pub enum PendingTool {
    Immediate(ToolOutcome),
    AwaitClient { kind: AwaitKind },
}

pub async fn dispatch_tool(
    handle: &CursorSessionHandle,
    call: &ToolCall,
    next_exec_id: &mut u32,
    next_interaction_id: &mut u32,
    conversation_id: &str,
) -> Result<PendingTool> {
    match normalized(&call.name).as_str() {
        "websearch" | "askquestion" | "switchmode" | "createplan" | "generateimage" => {
            let id = *next_interaction_id;
            *next_interaction_id += 1;
            let message = interaction_server_message(id, call)?;
            handle.emit(&message)?;
            Ok(PendingTool::AwaitClient {
                kind: AwaitKind::Interaction { id },
            })
        }
        "webfetch" => {
            let id = *next_interaction_id;
            *next_interaction_id += 1;
            handle.emit(&interaction::web_fetch_query(id, call)?)?;
            Ok(PendingTool::AwaitClient {
                kind: AwaitKind::Interaction { id },
            })
        }
        "semblesearch" | "semblefindrelated" | "semanticsearch" => {
            // Degraded fallback: no semble/ONNX. Prefer a short tool error so the model can Grep.
            Ok(PendingTool::Immediate(ToolOutcome {
                content: "SemanticSearch is unavailable in Router Switch; use Grep or Glob instead."
                    .into(),
                is_error: true,
                completed_tool_call: Some(interaction::tool_placeholder(&call.name, &call.call_id)?),
            }))
        }
        "todowrite" | "updatecurrentstep" | "awaitshell" => Ok(PendingTool::Immediate(
            ToolOutcome {
                content: "ok".into(),
                is_error: false,
                completed_tool_call: Some(interaction::tool_placeholder(
                    &call.name,
                    &call.call_id,
                )?),
            },
        )),
        "shell" | "read" | "delete" | "grep" | "glob" | "readlints" | "write" | "strreplace"
        | "editnotebook" | "task" | "callmcptool" | "fetchmcpresource" | "getmcptools" => {
            let id = *next_exec_id;
            *next_exec_id += 1;
            let message = exec_server_message(id, call, conversation_id)?;
            handle.emit(&message)?;
            Ok(PendingTool::AwaitClient {
                kind: AwaitKind::Exec { id },
            })
        }
        _ => {
            // Unknown tools still go to the client when possible via generic shell-less error.
            Ok(PendingTool::Immediate(ToolOutcome {
                content: format!("unsupported tool in local Agent kernel: {}", call.name),
                is_error: true,
                completed_tool_call: interaction::tool_placeholder(&call.name, &call.call_id).ok(),
            }))
        }
    }
}

fn interaction_server_message(id: u32, call: &ToolCall) -> Result<pb::AgentServerMessage> {
    match normalized(&call.name).as_str() {
        "websearch" => interaction::web_search_query(id, call),
        "askquestion" => Ok(pb::AgentServerMessage {
            ttft_breakdown: None,
            message: Some(pb::agent_server_message::Message::InteractionQuery(
                pb::InteractionQuery {
                    id,
                    query: Some(pb::interaction_query::Query::AskQuestionInteractionQuery(
                        pb::AskQuestionInteractionQuery {
                            tool_call_id: call.call_id.clone(),
                            args: Some(pb::AskQuestionArgs {
                                title: call
                                    .arguments
                                    .get("title")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                ..Default::default()
                            }),
                        },
                    )),
                },
            )),
        }),
        "switchmode" => Ok(pb::AgentServerMessage {
            ttft_breakdown: None,
            message: Some(pb::agent_server_message::Message::InteractionQuery(
                pb::InteractionQuery {
                    id,
                    query: Some(pb::interaction_query::Query::SwitchModeRequestQuery(
                        pb::SwitchModeRequestQuery {
                            args: Some(pb::SwitchModeArgs {
                                target_mode_id: call
                                    .arguments
                                    .get("target_mode_id")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                explanation: call
                                    .arguments
                                    .get("explanation")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                tool_call_id: call.call_id.clone(),
                            }),
                        },
                    )),
                },
            )),
        }),
        "generateimage" => Ok(pb::AgentServerMessage {
            ttft_breakdown: None,
            message: Some(pb::agent_server_message::Message::InteractionQuery(
                pb::InteractionQuery {
                    id,
                    query: Some(pb::interaction_query::Query::GenerateImageRequestQuery(
                        pb::GenerateImageRequestQuery {
                            tool_call_id: call.call_id.clone(),
                            args: Some(pb::GenerateImageArgs {
                                description: call
                                    .arguments
                                    .get("description")
                                    .or_else(|| call.arguments.get("prompt"))
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                ..Default::default()
                            }),
                        },
                    )),
                },
            )),
        }),
        _ => Ok(pb::AgentServerMessage {
            ttft_breakdown: None,
            message: Some(pb::agent_server_message::Message::InteractionQuery(
                pb::InteractionQuery {
                    id,
                    query: Some(pb::interaction_query::Query::CreatePlanRequestQuery(
                        pb::CreatePlanRequestQuery {
                            tool_call_id: call.call_id.clone(),
                            args: Some(pb::CreatePlanArgs {
                                plan: call
                                    .arguments
                                    .get("plan")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                name: call
                                    .arguments
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                overview: call
                                    .arguments
                                    .get("overview")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .into(),
                                ..Default::default()
                            }),
                        },
                    )),
                },
            )),
        }),
    }
}

fn exec_server_message(
    id: u32,
    call: &ToolCall,
    conversation_id: &str,
) -> Result<pb::AgentServerMessage> {
    use pb::exec_server_message::Message;
    let string = |name: &str| -> Result<String> {
        call.arguments
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| GatewayError::Protocol(format!("{} is missing {name}", call.name)))
    };
    let optional_string = |name: &str| {
        call.arguments
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let message = match normalized(&call.name).as_str() {
        "shell" => Message::ShellStreamArgs(pb::ShellArgs {
            command: string("command")?,
            working_directory: optional_string("working_directory").unwrap_or_default(),
            timeout: call
                .arguments
                .get("timeout")
                .and_then(Value::as_i64)
                .unwrap_or(30_000) as i32,
            tool_call_id: call.call_id.clone(),
            timeout_behavior: pb::TimeoutBehavior::Background as i32,
            hard_timeout: Some(86_400_000),
            description: optional_string("description"),
            close_stdin: true,
            conversation_id: Some(conversation_id.to_string()),
            ..Default::default()
        }),
        "read" => Message::ReadArgs(pb::ReadArgs {
            path: string("path")?,
            tool_call_id: call.call_id.clone(),
            offset: call
                .arguments
                .get("offset")
                .and_then(Value::as_i64)
                .map(|v| v as i32),
            limit: call
                .arguments
                .get("limit")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            encoding_hint: optional_string("encoding_hint"),
        }),
        "delete" => Message::DeleteArgs(pb::DeleteArgs {
            path: string("path")?,
            tool_call_id: call.call_id.clone(),
        }),
        "grep" => Message::GrepArgs(pb::GrepArgs {
            pattern: string("pattern")?,
            path: optional_string("path"),
            glob: optional_string("glob"),
            output_mode: optional_string("output_mode"),
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        "glob" => Message::GrepArgs(pb::GrepArgs {
            pattern: String::new(),
            path: optional_string("target_directory"),
            glob: optional_string("glob_pattern"),
            output_mode: Some("files_with_matches".into()),
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        "readlints" => Message::DiagnosticsArgs(pb::DiagnosticsArgs {
            path: call
                .arguments
                .get("paths")
                .and_then(Value::as_array)
                .and_then(|paths| paths.first())
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            tool_call_id: call.call_id.clone(),
        }),
        "write" => Message::WriteArgs(pb::WriteArgs {
            path: string("path")?,
            file_text: string("contents")
                .or_else(|_| string("file_text"))
                .unwrap_or_default(),
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        "strreplace" => Message::WriteArgs(pb::WriteArgs {
            path: string("path")?,
            file_text: format!(
                "old={}\nnew={}",
                optional_string("old_string").unwrap_or_default(),
                optional_string("new_string").unwrap_or_default()
            ),
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        "task" => Message::SubagentArgs(pb::SubagentArgs {
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        "callmcptool" | "getmcptools" | "fetchmcpresource" => Message::McpArgs(pb::McpArgs {
            name: call.name.clone(),
            tool_call_id: call.call_id.clone(),
            tool_name: optional_string("toolName").unwrap_or_default(),
            provider_identifier: optional_string("server").unwrap_or_default(),
            ..Default::default()
        }),
        "editnotebook" => Message::WriteArgs(pb::WriteArgs {
            path: optional_string("target_notebook").unwrap_or_default(),
            file_text: call.arguments_text.clone(),
            tool_call_id: call.call_id.clone(),
            ..Default::default()
        }),
        other => {
            return Err(GatewayError::Protocol(format!(
                "no exec mapping for tool {other}"
            )))
        }
    };
    Ok(pb::AgentServerMessage {
        ttft_breakdown: None,
        message: Some(pb::agent_server_message::Message::ExecServerMessage(
            pb::ExecServerMessage {
                id,
                exec_id: call.call_id.clone(),
                span_context: None,
                accept_hook_additional_contexts: None,
                message: Some(message),
            },
        )),
    })
}

pub fn complete_exec(call: &ToolCall, exec: &pb::ExecClientMessage) -> Result<ToolOutcome> {
    let content = exec_result_text(exec);
    let is_error = content.to_ascii_lowercase().contains("error");
    let mut tool_call = interaction::tool_placeholder(&call.name, &call.call_id)?;
    tool_call.completed_at_ms = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    );
    let _ = ToolResult {
        call_id: call.call_id.clone(),
        content: content.clone(),
        is_error,
    };
    Ok(ToolOutcome {
        content,
        is_error,
        completed_tool_call: Some(tool_call),
    })
}

fn exec_result_text(exec: &pb::ExecClientMessage) -> String {
    use pb::exec_client_message::Message;
    match exec.message.as_ref() {
        Some(Message::ShellResult(result)) => format!("{result:?}"),
        Some(Message::ReadResult(result)) => format!("{result:?}"),
        Some(Message::GrepResult(result)) => format!("{result:?}"),
        Some(Message::WriteResult(result)) => format!("{result:?}"),
        Some(Message::DeleteResult(result)) => format!("{result:?}"),
        Some(Message::DiagnosticsResult(result)) => format!("{result:?}"),
        Some(Message::McpResult(result)) => format!("{result:?}"),
        Some(Message::SubagentResult(result)) => format!("{result:?}"),
        Some(other) => format!("{other:?}"),
        None => "empty exec result".into(),
    }
}

pub async fn complete_interaction(
    _handle: &CursorSessionHandle,
    call: &ToolCall,
    response: &pb::InteractionResponse,
) -> Result<ToolOutcome> {
    match normalized(&call.name).as_str() {
        "websearch" => {
            if is_web_search_approved(response) {
                let query = call
                    .arguments
                    .get("search_term")
                    .or_else(|| call.arguments.get("query"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let content = WebSearch::new().search(query).await.unwrap_or_else(|err| {
                    format!("WebSearch failed: {err}")
                });
                return Ok(ToolOutcome {
                    content,
                    is_error: false,
                    completed_tool_call: Some(interaction::tool_placeholder(
                        &call.name,
                        &call.call_id,
                    )?),
                });
            }
            Ok(ToolOutcome {
                content: "WebSearch rejected by client".into(),
                is_error: true,
                completed_tool_call: Some(interaction::tool_placeholder(
                    &call.name,
                    &call.call_id,
                )?),
            })
        }
        "webfetch" => {
            if is_web_fetch_approved(response) {
                let url = call
                    .arguments
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let content = WebFetch::new().fetch(url).await.unwrap_or_else(|err| {
                    format!("WebFetch failed: {err}")
                });
                return Ok(ToolOutcome {
                    content,
                    is_error: false,
                    completed_tool_call: Some(interaction::tool_placeholder(
                        &call.name,
                        &call.call_id,
                    )?),
                });
            }
            Ok(ToolOutcome {
                content: "WebFetch rejected by client".into(),
                is_error: true,
                completed_tool_call: Some(interaction::tool_placeholder(
                    &call.name,
                    &call.call_id,
                )?),
            })
        }
        _ => Ok(ToolOutcome {
            content: format!("interaction completed for {}", call.name),
            is_error: false,
            completed_tool_call: Some(interaction::tool_placeholder(&call.name, &call.call_id)?),
        }),
    }
}

fn is_web_search_approved(response: &pb::InteractionResponse) -> bool {
    matches!(
        response.result.as_ref(),
        Some(pb::interaction_response::Result::WebSearchRequestResponse(
            pb::WebSearchRequestResponse {
                result: Some(pb::web_search_request_response::Result::Approved(_)),
            }
        ))
    )
}

fn is_web_fetch_approved(response: &pb::InteractionResponse) -> bool {
    matches!(
        response.result.as_ref(),
        Some(pb::interaction_response::Result::WebFetchRequestResponse(
            pb::WebFetchRequestResponse {
                result: Some(pb::web_fetch_request_response::Result::Approved(_)),
            }
        ))
    )
}
