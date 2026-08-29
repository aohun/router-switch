use prost::Message;

use crate::{
    actor::CursorCommand,
    interaction,
    proto::{agent::v1 as agent, aiserver::v1 as ai},
    sessions::{CursorParent, CursorSessionRegistry},
    GatewayError, Result,
};

pub struct DecodedAppend {
    pub request_id: String,
    pub seqno: i64,
    pub message: agent::AgentClientMessage,
}

impl DecodedAppend {
    pub fn model_id(&self) -> Option<&str> {
        let agent::agent_client_message::Message::RunRequest(request) =
            self.message.message.as_ref()?
        else {
            return None;
        };
        request
            .requested_model
            .as_ref()
            .map(|model| model.model_id.as_str())
            .filter(|model| !model.is_empty())
            .or_else(|| {
                request
                    .model_details
                    .as_ref()
                    .map(|model| model.model_id.as_str())
                    .filter(|model| !model.is_empty())
            })
    }

    pub fn conversation_id(&self) -> Option<&str> {
        let agent::agent_client_message::Message::RunRequest(request) =
            self.message.message.as_ref()?
        else {
            return None;
        };
        request.conversation_id.as_deref()
    }
}

pub fn decode(request: &ai::BidiAppendRequest) -> Result<DecodedAppend> {
    let request_id = request
        .request_id
        .as_ref()
        .map(|id| id.request_id.as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| GatewayError::Protocol("BidiAppend request_id is required".into()))?;
    if !request.data_binary.is_empty() {
        return Err(GatewayError::Protocol(
            "BidiAppend data_binary is not part of the captured protocol".into(),
        ));
    }
    if request.data.is_empty() {
        return Err(GatewayError::Protocol(
            "BidiAppend contains no AgentClientMessage".into(),
        ));
    }
    let payload = hex::decode(&request.data)?;
    Ok(DecodedAppend {
        request_id: request_id.into(),
        seqno: request.append_seqno,
        message: agent::AgentClientMessage::decode(payload.as_slice())?,
    })
}

pub async fn append(
    registry: &CursorSessionRegistry,
    request: DecodedAppend,
    parent: Option<CursorParent>,
) -> Result<ai::BidiAppendResponse> {
    let handle = registry.get_or_create(&request.request_id).await?;
    if let Some(parent) = parent {
        handle.set_parent(parent)?;
    }
    if matches!(
        request.message.message.as_ref(),
        Some(agent::agent_client_message::Message::ClientHeartbeat(_))
    ) {
        handle.emit(&interaction::heartbeat())?;
    }
    handle
        .command(CursorCommand::Append {
            seqno: request.seqno,
            message: Box::new(request.message),
        })
        .await?;
    Ok(ai::BidiAppendResponse {})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(run: agent::AgentRunRequest) -> ai::BidiAppendRequest {
        let message = agent::AgentClientMessage {
            message: Some(agent::agent_client_message::Message::RunRequest(run)),
        };
        ai::BidiAppendRequest {
            data: hex::encode(message.encode_to_vec()),
            request_id: Some(ai::BidiRequestId {
                request_id: "request".into(),
            }),
            append_seqno: 1,
            data_binary: Vec::new(),
        }
    }

    #[test]
    fn route_model_uses_requested_model_id() {
        let decoded = decode(&encoded(agent::AgentRunRequest {
            requested_model: Some(agent::RequestedModel {
                model_id: "33ceed20".into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(decoded.model_id(), Some("33ceed20"));
    }

    #[test]
    fn decodes_exec_client_message_instead_of_dropping() {
        let message = agent::AgentClientMessage {
            message: Some(agent::agent_client_message::Message::ExecClientMessage(
                agent::ExecClientMessage {
                    id: 7,
                    exec_id: "call-1".into(),
                    message: Some(agent::exec_client_message::Message::ReadResult(
                        agent::ReadResult::default(),
                    )),
                    ..Default::default()
                },
            )),
        };
        let request = ai::BidiAppendRequest {
            data: hex::encode(message.encode_to_vec()),
            request_id: Some(ai::BidiRequestId {
                request_id: "req-exec".into(),
            }),
            append_seqno: 2,
            data_binary: Vec::new(),
        };
        let decoded = decode(&request).unwrap();
        match decoded.message.message {
            Some(agent::agent_client_message::Message::ExecClientMessage(exec)) => {
                assert_eq!(exec.id, 7);
                assert_eq!(exec.exec_id, "call-1");
            }
            other => panic!("expected exec_client_message, got {other:?}"),
        }
    }
}
