use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    model::{PromptSpec, ProviderMessage},
    prompting::Mode,
    proto::agent::v1 as pb,
    sessions::{CursorSessionHandle, CursorSessionRegistry},
    GatewayError, Result,
};

pub struct PreparedRun {
    pub model_id: String,
    pub conversation_id: String,
    pub prompt: PromptSpec,
    pub history: Vec<ProviderMessage>,
    pub extra_params: Value,
}

pub async fn prepare(
    handle: &CursorSessionHandle,
    registry: &CursorSessionRegistry,
    request: pb::AgentRunRequest,
) -> Result<PreparedRun> {
    let model_id = request
        .requested_model
        .as_ref()
        .map(|m| m.model_id.as_str())
        .filter(|m| !m.is_empty())
        .or_else(|| {
            request
                .model_details
                .as_ref()
                .map(|m| m.model_id.as_str())
                .filter(|m| !m.is_empty())
        })
        .map(str::to_string)
        .or_else(|| {
            registry
                .settings()
                .read()
                .as_ref()
                .map(|s| s.model.clone())
                .filter(|m| !m.is_empty())
        })
        .unwrap_or_else(|| "default".into());

    let conversation_id = request
        .conversation_id
        .clone()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| handle.request_id().to_string());

    let mode = Mode::Agent;
    let blob_text = resolve_blobs(handle, &request).await?;
    let history_text = project_history(&request);
    let user_text = extract_user_text(&request);
    let combined_user = combine_user_context(&user_text, &history_text, &blob_text);

    if combined_user.trim().is_empty() {
        return Err(GatewayError::Protocol(
            "AgentRunRequest produced an empty user prompt (no text, history, or blobs)".into(),
        ));
    }

    let mut runtime_values = BTreeMap::new();
    runtime_values.insert("TIMESTAMP", chrono::Utc::now().to_rfc3339());
    runtime_values.insert("USER_QUERY", combined_user.clone());
    runtime_values.insert("OPEN_FILES", String::new());
    runtime_values.insert("SELECTED_CONTEXT", String::new());
    runtime_values.insert("ACTION_CONTEXT", String::new());
    runtime_values.insert("DEBUG_SERVER_ENDPOINT", String::new());
    runtime_values.insert("DEBUG_LOG_PATH", String::new());
    runtime_values.insert("DEBUG_SESSION_ID", String::new());

    let runtime = registry
        .compiler()
        .runtime_message(mode, &runtime_values)
        .unwrap_or_else(|_| String::new());
    let mut prompt =
        registry
            .compiler()
            .prompt_spec(mode, &model_id, Some(&model_id), &[], false, false)?;
    if !runtime.is_empty() {
        prompt.instructions = format!("{}\n\n{}", prompt.instructions, runtime);
    }
    if let Some(custom) = request
        .custom_system_prompt
        .as_ref()
        .filter(|s| !s.is_empty())
    {
        prompt.instructions = format!("{}\n\n{custom}", prompt.instructions);
    }

    let history = vec![ProviderMessage::user(combined_user)];

    let extra_params = registry
        .settings()
        .read()
        .as_ref()
        .map(|s| s.options.clone())
        .unwrap_or(Value::Null);

    Ok(PreparedRun {
        model_id,
        conversation_id,
        prompt,
        history,
        extra_params,
    })
}

fn extract_user_text(request: &pb::AgentRunRequest) -> String {
    let Some(action) = request.action.as_ref().and_then(|a| a.action.as_ref()) else {
        return String::new();
    };
    match action {
        pb::conversation_action::Action::UserMessageAction(action) => action
            .user_message
            .as_ref()
            .map(|m| m.text.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn project_history(request: &pb::AgentRunRequest) -> String {
    let mut parts = Vec::new();
    if let Some(state) = &request.conversation_state {
        for root in &state.root_prompt_messages_json {
            if let Ok(text) = std::str::from_utf8(root) {
                if !text.trim().is_empty() {
                    parts.push(text.to_string());
                }
            }
        }
    }
    if let Some(action) = request.action.as_ref().and_then(|a| a.action.as_ref()) {
        if let pb::conversation_action::Action::UserMessageAction(action) = action {
            for prepended in &action.prepend_user_messages {
                if !prepended.text.trim().is_empty() {
                    parts.push(prepended.text.clone());
                }
            }
            if let Some(history) = &action.conversation_history {
                for message in &history.messages {
                    if let Some(content) = history_message_text(message) {
                        parts.push(content);
                    }
                }
            }
        }
    }
    parts.join("\n\n")
}

fn history_message_text(message: &pb::ConversationHistoryMessage) -> Option<String> {
    use pb::conversation_history_message::Message;
    match message.message.as_ref()? {
        Message::User(user) => {
            let text = user
                .content
                .iter()
                .filter_map(|part| match part.content.as_ref()? {
                    pb::conversation_history_user_content::Content::Text(text) => {
                        Some(text.text.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then(|| format!("User: {text}"))
        }
        Message::Assistant(assistant) => {
            let text = assistant
                .content
                .iter()
                .filter_map(|part| match part.content.as_ref()? {
                    pb::conversation_history_assistant_content::Content::Text(text) => {
                        Some(text.text.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then(|| format!("Assistant: {text}"))
        }
        Message::Tool(tool) => {
            let text = tool
                .content
                .iter()
                .filter_map(|part| match part.content.as_ref()? {
                    pb::conversation_history_tool_result_content::Content::Text(text) => {
                        Some(text.text.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then(|| format!("Tool: {text}"))
        }
    }
}

fn combine_user_context(user_text: &str, history_text: &str, blob_text: &str) -> String {
    let mut parts = Vec::new();
    if !history_text.trim().is_empty() {
        parts.push(history_text.trim().to_string());
    }
    if !blob_text.trim().is_empty() {
        parts.push(format!("Additional context:\n{}", blob_text.trim()));
    }
    if !user_text.trim().is_empty() {
        parts.push(user_text.trim().to_string());
    }
    parts.join("\n\n")
}

async fn resolve_blobs(
    handle: &CursorSessionHandle,
    request: &pb::AgentRunRequest,
) -> Result<String> {
    let mut texts = Vec::new();
    let mut kv_id = 1u32;
    for blob in &request.pre_fetched_blobs {
        if !blob.value.is_empty() {
            if let Ok(text) = std::str::from_utf8(&blob.value) {
                texts.push(text.to_string());
            } else {
                texts.push(format!("[binary blob {} bytes]", blob.value.len()));
            }
            continue;
        }
        if blob.id.is_empty() {
            continue;
        }
        let _ = handle.emit(&pb::AgentServerMessage {
            ttft_breakdown: None,
            message: Some(pb::agent_server_message::Message::KvServerMessage(
                pb::KvServerMessage {
                    id: kv_id,
                    span_context: None,
                    message: Some(pb::kv_server_message::Message::GetBlobArgs(
                        pb::GetBlobArgs {
                            blob_id: blob.id.clone(),
                        },
                    )),
                },
            )),
        });
        kv_id += 1;
    }
    Ok(texts.join("\n\n"))
}

/// Test helper: project prompt materials the same way prepare does.
pub fn project_history_for_tests(request: &pb::AgentRunRequest) -> String {
    let user_text = extract_user_text(request);
    let history_text = project_history(request);
    let blob_text = request
        .pre_fetched_blobs
        .iter()
        .filter_map(|blob| {
            if blob.value.is_empty() {
                None
            } else {
                std::str::from_utf8(&blob.value)
                    .ok()
                    .map(str::to_string)
                    .or_else(|| Some(format!("[binary blob {} bytes]", blob.value.len())))
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    combine_user_context(&user_text, &history_text, &blob_text)
}
