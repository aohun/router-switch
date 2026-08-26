use axum::{
    body::{to_bytes, Body, Bytes},
    extract::State,
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use std::convert::Infallible;
use tokio::sync::mpsc;

use crate::{
    bidi_append::AppState,
    connect::{decode_unary, END_STREAM_FLAG},
    proto::agent::v1 as agent_pb,
    GatewayError, Result,
};

pub async fn run_sse_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let (_parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| GatewayError::Protocol(format!("read RunSSE body: {e}")))?;

    let req_id_msg: agent_pb::BidiRequestId = decode_unary(&body_bytes)?;
    let request_id = req_id_msg.request_id;

    let receiver = state
        .registry
        .take_receiver(&request_id)
        .unwrap_or_else(|| {
            let (_handle, rx) = mpsc::unbounded_channel();
            rx
        });

    let body_stream = create_body_stream(receiver);
    let mut response = Response::new(Body::from_stream(body_stream));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert("connect-protocol-version", HeaderValue::from_static("1"));
    Ok(response)
}

fn create_body_stream(
    mut receiver: mpsc::UnboundedReceiver<Bytes>,
) -> impl tokio_stream::Stream<Item = std::result::Result<Bytes, Infallible>> {
    async_stream::stream! {
        while let Some(chunk) = receiver.recv().await {
            let is_terminal = chunk.first().is_some_and(|f| f & END_STREAM_FLAG != 0);
            yield Ok::<Bytes, Infallible>(chunk);
            if is_terminal {
                return;
            }
        }
    }
}
