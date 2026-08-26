use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use domain::CursorSettings;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    connect::{encode_end_stream, encode_message},
    proto::agent::v1 as agent_pb,
    provider::{create_provider, ModelEvent, ModelInvocation},
    Result,
};

#[derive(Clone)]
pub struct SessionHandle {
    pub request_id: String,
    sender: mpsc::UnboundedSender<Bytes>,
    cancellation: CancellationToken,
}

impl SessionHandle {
    pub fn new(request_id: String) -> (Self, mpsc::UnboundedReceiver<Bytes>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let cancellation = CancellationToken::new();
        (
            Self {
                request_id,
                sender,
                cancellation,
            },
            receiver,
        )
    }

    pub fn emit_message(&self, msg: &agent_pb::AgentServerMessage) -> Result<()> {
        let framed = encode_message(msg)?;
        let _ = self.sender.send(framed);
        Ok(())
    }

    pub fn emit_text_delta(&self, text: String) -> Result<()> {
        let msg = agent_pb::AgentServerMessage {
            interaction_update: Some(agent_pb::InteractionUpdate {
                text_delta: Some(agent_pb::TextDeltaUpdate {
                    text,
                    is_server_notice: false,
                }),
                ..Default::default()
            }),
        };
        self.emit_message(&msg)
    }

    pub fn emit_thinking_delta(&self, text: String) -> Result<()> {
        let msg = agent_pb::AgentServerMessage {
            interaction_update: Some(agent_pb::InteractionUpdate {
                thinking_delta: Some(agent_pb::ThinkingDeltaUpdate { text }),
                ..Default::default()
            }),
        };
        self.emit_message(&msg)
    }

    pub fn emit_turn_ended(
        &self,
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    ) -> Result<()> {
        let msg = agent_pb::AgentServerMessage {
            interaction_update: Some(agent_pb::InteractionUpdate {
                turn_ended: Some(agent_pb::TurnEndedUpdate {
                    input_tokens: input_tokens.map(|v| v as i64),
                    output_tokens: output_tokens.map(|v| v as i64),
                }),
                ..Default::default()
            }),
        };
        self.emit_message(&msg)
    }

    pub fn emit_end_stream(&self) {
        let end_frame = encode_end_stream();
        let _ = self.sender.send(end_frame);
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

pub struct SessionEntry {
    pub handle: SessionHandle,
    pub receiver: Option<mpsc::UnboundedReceiver<Bytes>>,
    pub prompt: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct CursorSessionRegistry {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<SessionEntry>>>>>,
}

impl CursorSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create(&self, request_id: &str) -> SessionHandle {
        let mut guard = self.sessions.write();
        if let Some(entry) = guard.get(request_id) {
            return entry.read().handle.clone();
        }
        let (handle, receiver) = SessionHandle::new(request_id.to_string());
        guard.insert(
            request_id.to_string(),
            Arc::new(RwLock::new(SessionEntry {
                handle: handle.clone(),
                receiver: Some(receiver),
                prompt: None,
                model_id: None,
            })),
        );
        handle
    }

    pub fn take_receiver(&self, request_id: &str) -> Option<mpsc::UnboundedReceiver<Bytes>> {
        let guard = self.sessions.read();
        let entry = guard.get(request_id)?;
        let mut entry_guard = entry.write();
        entry_guard.receiver.take()
    }

    pub fn start_agent_run(
        &self,
        request_id: &str,
        prompt: String,
        model_id: Option<String>,
        settings: Option<CursorSettings>,
    ) {
        let handle = self.get_or_create(request_id);
        let s = settings.unwrap_or_else(|| domain::official_cursor_settings());

        let target_model = model_id
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| s.model.clone());

        let provider = create_provider(
            &s.provider_type,
            s.base_url.clone(),
            s.api_key.clone(),
            target_model.clone(),
        );

        let cancellation = handle.cancellation();
        tokio::spawn(async move {
            use futures_util::StreamExt;

            let invocation = ModelInvocation {
                prompt,
                model: target_model,
                messages: Vec::new(),
            };

            let mut stream = provider.stream(invocation, cancellation.clone());
            let mut in_tokens = None;
            let mut out_tokens = None;

            while let Some(item) = stream.next().await {
                if cancellation.is_cancelled() {
                    break;
                }
                match item {
                    Ok(ModelEvent::TextDelta(text)) => {
                        let _ = handle.emit_text_delta(text);
                    }
                    Ok(ModelEvent::ThinkingDelta(thinking)) => {
                        let _ = handle.emit_thinking_delta(thinking);
                    }
                    Ok(ModelEvent::Usage {
                        input_tokens,
                        output_tokens,
                    }) => {
                        in_tokens = input_tokens;
                        out_tokens = output_tokens;
                    }
                    Ok(ModelEvent::Done) => {
                        break;
                    }
                    Err(err) => {
                        tracing::error!(%err, "provider stream error during agent run");
                        let _ = handle.emit_text_delta(format!("\n\n[Error: {err}]"));
                        break;
                    }
                    _ => {}
                }
            }

            let _ = handle.emit_turn_ended(in_tokens, out_tokens);
            handle.emit_end_stream();
        });
    }
}
