use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Extension, State},
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::{
    connect::{decode_unary, encode_end_stream, encode_message},
    handlers::AppState,
    model::{ModelInvocation, PromptSpec, ProviderMessage},
    proto::aiserver::v1 as ai_pb,
    provider::{create_provider, ModelEvent},
    proxy::{self, CursorProxy},
    GatewayError, Result,
};

pub async fn stream_chat_handler(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let s = state
        .registry
        .settings()
        .read()
        .clone()
        .unwrap_or_else(|| domain::official_cursor_settings());

    // Official 服务商: do not adapt as OpenAI — proxy upstream.
    if s.kind.is_official() {
        return proxy::forward(Extension(proxy), request).await;
    }

    let (_parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| GatewayError::Protocol(format!("read chat body: {e}")))?;

    let chat_req: ai_pb::StreamChatRequest = decode_unary(&body_bytes).unwrap_or_default();
    let prompt = chat_req.prompt.unwrap_or_default();
    let requested_model = chat_req.model_id;

    let target_model = requested_model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| s.model.clone());

    let provider = create_provider(
        &s.provider_type,
        s.base_url.clone(),
        s.api_key.clone(),
        target_model.clone(),
    );

    let cancellation = CancellationToken::new();
    let invocation = ModelInvocation {
        model: target_model.clone(),
        prompt: PromptSpec {
            instructions: String::new(),
            tools: Vec::new(),
        },
        history: vec![ProviderMessage::user(prompt)],
        extra_params: s.options.clone(),
    };

    let mut provider_stream = provider.stream(invocation, cancellation.clone());

    let stream = async_stream::stream! {
        while let Some(item) = provider_stream.next().await {
            match item {
                Ok(ModelEvent::TextDelta(text)) => {
                    let msg = ai_pb::StreamChatResponse {
                        text,
                        model_name: Some(target_model.clone()),
                    };
                    if let Ok(framed) = encode_message(&msg) {
                        yield Ok::<Bytes, Infallible>(framed);
                    }
                }
                Ok(ModelEvent::Done(_)) => {
                    break;
                }
                Err(err) => {
                    let msg = ai_pb::StreamChatResponse {
                        text: format!("\n\n[Error: {err}]"),
                        model_name: Some(target_model.clone()),
                    };
                    if let Ok(framed) = encode_message(&msg) {
                        yield Ok::<Bytes, Infallible>(framed);
                    }
                    break;
                }
                _ => {}
            }
        }
        let end = encode_end_stream();
        yield Ok::<Bytes, Infallible>(end);
    };

    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/connect+proto"),
    );
    response
        .headers_mut()
        .insert("connect-protocol-version", HeaderValue::from_static("1"));
    Ok(response)
}
