mod prepare;
mod tools;

use std::collections::BTreeMap;

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::{
    interaction,
    lifecycle,
    model::{FinishReason, ModelInvocation, ProviderMessage, ToolCall, Usage},
    provider::ModelEvent,
    proto::agent::v1 as pb,
    sessions::{CursorSessionHandle, CursorSessionRegistry},
    GatewayError, Result,
};

use prepare::PreparedRun;
use tools::{dispatch_tool, PendingTool, ToolOutcome};

pub struct AgentRun;

impl AgentRun {
    pub async fn start(
        handle: CursorSessionHandle,
        registry: CursorSessionRegistry,
        request: pb::AgentRunRequest,
        mut client_messages: mpsc::UnboundedReceiver<pb::AgentClientMessage>,
    ) -> Result<()> {
        let prepared = prepare::prepare(&handle, &registry, request).await?;
        run_loop(handle, registry, prepared, &mut client_messages).await
    }
}

async fn run_loop(
    handle: CursorSessionHandle,
    registry: CursorSessionRegistry,
    mut prepared: PreparedRun,
    client_messages: &mut mpsc::UnboundedReceiver<pb::AgentClientMessage>,
) -> Result<()> {
    let provider = registry.resolve_provider(&prepared.model_id)?;
    let cancellation = handle.cancellation();
    let mut usage = Usage::default();
    let mut next_interaction_id = 1u32;
    let mut next_exec_id = 1u32;

    loop {
        if cancellation.is_cancelled() {
            return Err(GatewayError::Cancelled);
        }

        let invocation = ModelInvocation {
            model: prepared.model_id.clone(),
            prompt: prepared.prompt.clone(),
            history: prepared.history.clone(),
            extra_params: prepared.extra_params.clone(),
        };

        let mut stream = provider.stream(invocation, cancellation.clone());
        let mut assistant_text = String::new();
        let mut tool_calls = BTreeMap::<usize, ToolCall>::new();
        let mut finish = FinishReason::Stop;

        while let Some(item) = stream.next().await {
            if cancellation.is_cancelled() {
                return Err(GatewayError::Cancelled);
            }
            match item? {
                ModelEvent::TextDelta(text) => {
                    assistant_text.push_str(&text);
                    handle.emit(&interaction::text_delta(text))?;
                }
                ModelEvent::ThinkingDelta(text) => {
                    handle.emit(&interaction::thinking_delta(text))?;
                }
                ModelEvent::ToolCallStart {
                    index,
                    call_id,
                    name,
                } => {
                    let call = ToolCall {
                        index,
                        call_id: call_id.clone(),
                        model_call_id: call_id.clone(),
                        name,
                        arguments_text: String::new(),
                        arguments: serde_json::Value::Null,
                    };
                    handle.emit(&interaction::partial_tool_call(&call, "")?)?;
                    tool_calls.insert(index, call);
                }
                ModelEvent::ToolCallArgumentsDelta { index, delta } => {
                    if let Some(call) = tool_calls.get_mut(&index) {
                        call.arguments_text.push_str(&delta);
                        handle.emit(&interaction::partial_tool_call(call, &delta)?)?;
                    }
                }
                ModelEvent::ToolCallEnd { index } => {
                    if let Some(call) = tool_calls.get_mut(&index) {
                        call.arguments = serde_json::from_str(&call.arguments_text)
                            .unwrap_or(serde_json::Value::Null);
                        handle.emit(&interaction::tool_started(call)?)?;
                    }
                }
                ModelEvent::Usage(next) => {
                    usage = next;
                }
                ModelEvent::Done(reason) => {
                    finish = reason;
                }
            }
        }

        if tool_calls.is_empty() || finish != FinishReason::ToolUse {
            handle.emit(&interaction::turn_ended(Some(usage.clone())))?;
            lifecycle::finish_success(&handle);
            return Ok(());
        }

        let mut ordered_calls = tool_calls.into_values().collect::<Vec<_>>();
        ordered_calls.sort_by_key(|call| call.index);
        prepared.history.push(ProviderMessage::assistant(
            assistant_text,
            ordered_calls.clone(),
        ));

        for call in &ordered_calls {
            let pending = dispatch_tool(
                &handle,
                call,
                &mut next_exec_id,
                &mut next_interaction_id,
                &prepared.conversation_id,
            )
            .await?;
            let outcome = match pending {
                PendingTool::Immediate(outcome) => outcome,
                PendingTool::AwaitClient { kind } => {
                    wait_for_tool_result(
                        &handle,
                        client_messages,
                        call,
                        kind,
                        &cancellation,
                    )
                    .await?
                }
            };
            let ToolOutcome {
                content,
                is_error,
                completed_tool_call,
            } = outcome;
            if let Some(tool_call) = completed_tool_call {
                handle.emit(&interaction::tool_completed(call, tool_call))?;
            }
            prepared
                .history
                .push(ProviderMessage::tool(call.call_id.clone(), content));
            let _ = is_error;
        }
    }
}

#[derive(Debug)]
pub(crate) enum AwaitKind {
    Exec { id: u32 },
    Interaction { id: u32 },
}

async fn wait_for_tool_result(
    handle: &CursorSessionHandle,
    client_messages: &mut mpsc::UnboundedReceiver<pb::AgentClientMessage>,
    call: &ToolCall,
    kind: AwaitKind,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<ToolOutcome> {
    loop {
        let message = tokio::select! {
            _ = cancellation.cancelled() => return Err(GatewayError::Cancelled),
            message = client_messages.recv() => message,
        };
        let Some(message) = message else {
            return Err(GatewayError::Protocol(
                "client message channel closed while waiting for tool result".into(),
            ));
        };
        match (&kind, message.message) {
            (
                AwaitKind::Exec { id },
                Some(pb::agent_client_message::Message::ExecClientMessage(exec)),
            ) if exec.id == *id => {
                return tools::complete_exec(call, &exec);
            }
            (
                AwaitKind::Interaction { id },
                Some(pb::agent_client_message::Message::InteractionResponse(response)),
            ) if response.id == *id => {
                return tools::complete_interaction(handle, call, &response).await;
            }
            (_, Some(pb::agent_client_message::Message::ClientHeartbeat(_))) => {
                handle.emit(&interaction::heartbeat())?;
            }
            _ => {}
        }
    }
}

pub use prepare::project_history_for_tests;
