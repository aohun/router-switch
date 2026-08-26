use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use domain::CursorSettings;
use parking_lot::RwLock;
use prost::Message;

use crate::{
    connect::decode_unary,
    proto::{agent::v1 as agent_pb, aiserver::v1 as ai_pb},
    session::CursorSessionRegistry,
    GatewayError, Result,
};

#[derive(Clone)]
pub struct AppState {
    pub registry: CursorSessionRegistry,
    pub settings: Arc<RwLock<Option<CursorSettings>>>,
}

pub async fn bidi_append_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let (_parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| GatewayError::Protocol(format!("read bidi body: {e}")))?;

    let bidi_req: ai_pb::BidiAppendRequest = decode_unary(&body_bytes)?;
    let request_id = bidi_req
        .request_id
        .map(|r| r.request_id)
        .unwrap_or_default();

    let client_msg_bytes = if !bidi_req.data_binary.is_empty() {
        bidi_req.data_binary
    } else if !bidi_req.data.is_empty() {
        hex::decode(&bidi_req.data).unwrap_or_default()
    } else {
        Vec::new()
    };

    let client_msg: Option<agent_pb::AgentClientMessage> = if !client_msg_bytes.is_empty() {
        agent_pb::AgentClientMessage::decode(&client_msg_bytes[..]).ok()
    } else {
        None
    };

    let mut prompt = String::new();
    let mut model_id = None;

    if let Some(msg) = client_msg {
        if let Some(run_req) = msg.run_request {
            if let Some(action) = run_req.action {
                if let Some(user_action) = action.user_message_action {
                    if let Some(user_msg) = user_action.user_message {
                        prompt = user_msg.text;
                    }
                }
            }
            if prompt.is_empty() {
                if let Some(conv_state) = run_req.conversation_state {
                    if let Some(last) = conv_state.root_prompt_messages_json.last() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(last) {
                            if let Some(text) = val.get("text").and_then(|t| t.as_str()) {
                                prompt = text.to_string();
                            }
                        }
                    }
                }
            }
            if let Some(req_model) = run_req.requested_model {
                if !req_model.model_id.is_empty() {
                    model_id = Some(req_model.model_id);
                }
            } else if let Some(details) = run_req.model_details {
                if !details.model_id.is_empty() {
                    model_id = Some(details.model_id);
                }
            }
        }
    }

    if !request_id.is_empty() {
        let current_settings = state.settings.read().clone();
        state
            .registry
            .start_agent_run(&request_id, prompt, model_id, current_settings);
    }

    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/proto"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from_static("0"));
    Ok(response)
}
