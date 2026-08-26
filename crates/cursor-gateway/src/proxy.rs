use std::sync::Arc;

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Extension,
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode},
};
use reqwest::Client;

use crate::{harness::proxy::UPSTREAM_URL_HEADER, GatewayError, Result};

pub const DEFAULT_UPSTREAM: &str = "https://api2.cursor.sh";

#[derive(Clone)]
pub struct CursorProxy {
    client: Client,
    upstream_url: Arc<String>,
}

pub struct BufferedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl BufferedResponse {
    pub fn into_response(self) -> Response<Body> {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }

    pub fn with_body(self, body: Bytes) -> Response<Body> {
        let mut response = Response::new(Body::from(body));
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }
}

impl CursorProxy {
    pub fn default_upstream() -> Self {
        Self::for_upstream(DEFAULT_UPSTREAM)
    }

    pub fn for_upstream(url: &str) -> Self {
        let client = Client::builder()
            .brotli(true)
            .deflate(true)
            .gzip(true)
            .build()
            .unwrap_or_default();
        Self {
            client,
            upstream_url: Arc::new(url.trim_end_matches('/').to_string()),
        }
    }

    pub fn target_url(&self, request: &Request<Body>) -> String {
        if let Some(header) = request.headers().get(UPSTREAM_URL_HEADER) {
            if let Ok(url) = header.to_str() {
                return url.to_string();
            }
        }
        let path = request
            .uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/");
        format!("{}{}", self.upstream_url, path)
    }
}

pub async fn forward(
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let url = proxy.target_url(&request);
    let method = request.method().clone();
    let headers = request.headers().clone();
    let (_parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| GatewayError::Protocol(format!("read request body: {e}")))?;

    let mut req = proxy.client.request(method, &url);
    for (name, value) in &headers {
        if name != "host" && name != UPSTREAM_URL_HEADER {
            req = req.header(name.as_str(), value.as_bytes());
        }
    }
    req = req.body(body_bytes);

    let res = req
        .send()
        .await
        .map_err(|e| GatewayError::Upstream(format!("forward request failed: {e}")))?;

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response_headers = HeaderMap::new();
    for (k, v) in res.headers() {
        if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
            response_headers.insert(k.clone(), val);
        }
    }

    let stream = res.bytes_stream();
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    *response.headers_mut() = response_headers;
    Ok(response)
}

pub async fn forward_buffered(
    proxy: &CursorProxy,
    request: Request<Body>,
) -> Result<BufferedResponse> {
    let url = proxy.target_url(&request);
    let method = request.method().clone();
    let headers = request.headers().clone();
    let (_parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| GatewayError::Protocol(format!("read request body: {e}")))?;

    let mut req = proxy.client.request(method, &url);
    for (name, value) in &headers {
        if name != "host" && name != UPSTREAM_URL_HEADER {
            req = req.header(name.as_str(), value.as_bytes());
        }
    }
    req = req.body(body_bytes);

    let res = req
        .send()
        .await
        .map_err(|e| GatewayError::Upstream(format!("forward request failed: {e}")))?;

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response_headers = HeaderMap::new();
    for (k, v) in res.headers() {
        if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
            response_headers.insert(k.clone(), val);
        }
    }
    let body = res
        .bytes()
        .await
        .map_err(|e| GatewayError::Upstream(format!("read upstream body: {e}")))?;

    Ok(BufferedResponse {
        status,
        headers: response_headers,
        body,
    })
}
