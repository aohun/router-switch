use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
};
use bytes::Bytes;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    connect::END_STREAM_FLAG,
    sessions::CursorSessionRegistry,
    Result,
};

pub async fn stream(registry: &CursorSessionRegistry, request_id: &str) -> Result<Response<Body>> {
    let handle = registry.get_or_create(request_id).await?;
    let receiver = handle.subscribe();
    let body_stream = local_body_stream(receiver, handle.cancellation());
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

fn local_body_stream(
    mut receiver: mpsc::UnboundedReceiver<Bytes>,
    cancellation: CancellationToken,
) -> impl tokio_stream::Stream<Item = std::result::Result<Bytes, Infallible>> {
    async_stream::stream! {
        let mut guard = LocalRunGuard::new(cancellation);
        while let Some(chunk) = receiver.recv().await {
            let terminal = is_end_stream_frame(&chunk);
            if terminal {
                guard.complete();
            }
            yield Ok::<Bytes, Infallible>(chunk);
            if terminal {
                return;
            }
        }
        guard.complete();
    }
}

fn is_end_stream_frame(frame: &Bytes) -> bool {
    frame
        .first()
        .is_some_and(|flags| flags & END_STREAM_FLAG != 0)
}

struct LocalRunGuard {
    cancellation: CancellationToken,
    completed: bool,
}

impl LocalRunGuard {
    fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            completed: false,
        }
    }

    fn complete(&mut self) {
        self.completed = true;
    }
}

impl Drop for LocalRunGuard {
    fn drop(&mut self) {
        if !self.completed {
            self.cancellation.cancel();
        }
    }
}

pub async fn upstream(
    registry: CursorSessionRegistry,
    request_id: String,
    generation: u64,
    response: Response<Body>,
) -> Response<Body> {
    use futures_util::StreamExt;
    let (parts, body) = response.into_parts();
    let stream = async_stream::stream! {
        let _guard = UpstreamRunGuard {
            registry,
            request_id,
            generation,
        };
        let mut body = body.into_data_stream();
        while let Some(chunk) = body.next().await {
            yield chunk;
        }
    };
    Response::from_parts(parts, Body::from_stream(stream))
}

struct UpstreamRunGuard {
    registry: CursorSessionRegistry,
    request_id: String,
    generation: u64,
}

impl Drop for UpstreamRunGuard {
    fn drop(&mut self) {
        let registry = self.registry.clone();
        let request_id = self.request_id.clone();
        let generation = self.generation;
        tokio::spawn(async move {
            registry.finish_upstream(request_id, generation).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, proto::agent::v1 as pb};
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn local_stream_cancels_when_the_client_disconnects() {
        let (sender, receiver) = mpsc::unbounded_channel();
        let cancellation = CancellationToken::new();
        sender
            .send(connect::encode_message(&pb::AgentServerMessage::default()).unwrap())
            .unwrap();
        let mut stream = Box::pin(local_body_stream(receiver, cancellation.clone()));

        stream.next().await.unwrap().unwrap();

        drop(sender);
        drop(stream);
        assert!(cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn terminal_frame_does_not_cancel_a_completed_local_run() {
        let (sender, receiver) = mpsc::unbounded_channel();
        let cancellation = CancellationToken::new();
        sender.send(connect::encode_end_stream()).unwrap();
        let mut stream = Box::pin(local_body_stream(receiver, cancellation.clone()));

        let terminal = stream.next().await.unwrap().unwrap();
        assert!(is_end_stream_frame(&terminal));
        drop(stream);
        assert!(!cancellation.is_cancelled());
    }
}
