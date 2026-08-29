use axum::{
    body::{Body, Bytes},
    extract::{Extension, State},
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use bytes::{BufMut, BytesMut};
use domain::CursorSettings;
use parking_lot::RwLock;
use prost::Message;
use std::sync::Arc;

use crate::{
    handlers::AppState,
    proto::{
        agent::v1 as agent_pb,
        catalog::{AvailableModel, AvailableModelsAddition, UsableModelsAddition},
    },
    proxy::{self, CursorProxy},
    GatewayError, Result,
};

pub async fn available_models(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let local_models = get_active_models(&state.registry.settings());
    let model_names: Vec<String> = local_models.iter().map(|(id, _)| id.clone()).collect();
    let models: Vec<AvailableModel> = local_models
        .iter()
        .map(|(id, display_name)| build_available_model(id, display_name))
        .collect();

    let local = AvailableModelsAddition {
        model_names,
        models,
    }
    .encode_to_vec();

    match proxy::forward_buffered(&proxy, request).await {
        Ok(upstream) => merge_response(upstream, local),
        Err(error) => {
            tracing::warn!(%error, "Cursor AvailableModels upstream unavailable; using local catalog");
            Ok(local_response(local))
        }
    }
}

pub async fn usable_models(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let local_models = get_active_models(&state.registry.settings());
    let models: Vec<agent_pb::ModelDetails> = local_models
        .iter()
        .map(|(id, display_name)| build_model_details(id, display_name))
        .collect();

    let local = UsableModelsAddition { models }.encode_to_vec();

    match proxy::forward_buffered(&proxy, request).await {
        Ok(upstream) => merge_response(upstream, local),
        Err(error) => {
            tracing::warn!(%error, "Cursor GetUsableModels upstream unavailable; using local catalog");
            Ok(local_response(local))
        }
    }
}

fn build_available_model(id: &str, display_name: &str) -> AvailableModel {
    let short_name = display_name
        .split('/')
        .last()
        .unwrap_or(display_name)
        .to_string();

    AvailableModel {
        name: id.to_string(),
        default_on: true,
        supports_agent: Some(true),
        degradation_status: None,
        tooltip_data: None,
        supports_thinking: Some(true),
        supports_images: Some(true),
        supports_max_mode: Some(true),
        client_display_name: Some(display_name.to_string()),
        server_model_name: Some(id.to_string()),
        supports_non_max_mode: Some(true),
        is_recommended_for_background_composer: Some(true),
        supports_plan_mode: Some(true),
        inputbox_short_model_name: Some(short_name),
        supports_sandboxing: Some(true),
        supports_cmd_k: Some(true),
        vendor_name: Some("Router Switch".to_string()),
    }
}

fn build_model_details(id: &str, display_name: &str) -> agent_pb::ModelDetails {
    let short_name = display_name
        .split('/')
        .last()
        .unwrap_or(display_name)
        .to_string();

    agent_pb::ModelDetails {
        model_id: id.to_string(),
        thinking_details: Some(agent_pb::ThinkingDetails {}),
        display_model_id: id.to_string(),
        display_name: display_name.to_string(),
        display_name_short: short_name,
        aliases: vec![id.to_string()],
        max_mode: Some(false),
        ..Default::default()
    }
}

fn get_active_models(settings: &Arc<RwLock<Option<CursorSettings>>>) -> Vec<(String, String)> {
    let mut models: Vec<(String, String)> = Vec::new();
    if let Some(s) = settings.read().as_ref() {
        if !s.model.is_empty() {
            let id = s.model.clone();
            models.push((id.clone(), id));
        }
        for mapping in &s.model_mappings {
            if !mapping.model.is_empty() {
                let id = mapping.model.clone();
                let display = if mapping.display_name.is_empty() {
                    id.clone()
                } else {
                    mapping.display_name.clone()
                };
                if !models.iter().any(|(existing, _)| existing == &id) {
                    models.push((id, display));
                }
            }
        }
    }
    let defaults = [
        ("claude-3.7-sonnet", "Claude 3.7 Sonnet"),
        ("gpt-4o", "GPT-4o"),
        ("deepseek-chat", "DeepSeek V3"),
    ];
    for (id, display) in defaults {
        if !models.iter().any(|(existing, _)| existing == id) {
            models.push((id.to_string(), display.to_string()));
        }
    }
    models
}

fn merge_response(upstream: proxy::BufferedResponse, extra: Vec<u8>) -> Result<Response<Body>> {
    if !upstream.status.is_success() {
        tracing::warn!(status = %upstream.status, "Cursor model catalog upstream rejected request; using local catalog");
        return Ok(local_response(extra));
    }
    let (framed, payload) = unary_payload(&upstream.body)?;
    let body = if framed {
        let mut merged = BytesMut::with_capacity(5 + payload.len() + extra.len());
        merged.put_u8(0);
        merged.put_u32((payload.len() + extra.len()) as u32);
        merged.extend_from_slice(payload);
        merged.extend_from_slice(&extra);
        merged.freeze()
    } else {
        let mut merged = BytesMut::with_capacity(payload.len() + extra.len());
        merged.extend_from_slice(payload);
        merged.extend_from_slice(&extra);
        merged.freeze()
    };
    Ok(upstream.with_body(body))
}

fn local_response(body: Vec<u8>) -> Response<Body> {
    let length = body.len();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/proto"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
    response
}

fn unary_payload(body: &Bytes) -> Result<(bool, &[u8])> {
    if body.len() < 5 {
        return Ok((false, body));
    }
    let flags = body[0];
    let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    if length != body.len() - 5 {
        return Ok((false, body));
    }
    if flags != 0 {
        return Err(GatewayError::Protocol(format!(
            "cannot merge compressed or terminal model catalog frame: flags={flags}"
        )));
    }
    Ok((true, &body[5..]))
}
