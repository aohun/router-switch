//! Standard-protocol routing endpoints (the "compat" gateway).
//!
//! Cursor always talks to this gateway, but the other AI tools (Claude Code,
//! Codex, Grok CLI, ...) are switched by writing their live configs to point
//! straight at the provider. When such a tool is switched onto a provider
//! whose request protocol differs from the tool's native dialect, the session
//! layer registers a [`CompatTarget`] here and rewrites the live config to
//! point at this listener instead; the endpoints below then translate
//! inbound-protocol ⇄ [`ModelInvocation`] ⇄ upstream-protocol, the same job
//! cc-switch's "需开启路由" proxy does.

mod anthropic_in;
mod chat_in;
mod responses_in;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, Request, Response, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use domain::{AppKind, RequestProtocol};
use futures_util::StreamExt;
use parking_lot::RwLock;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::model::{FinishReason, ModelInvocation, Usage};
use crate::provider::{create_provider, ModelEvent, ProviderStream};
use crate::Result;

/// Preferred localhost port for the compat listener; a random port is used
/// when it is busy (the rewritten live config always carries the actual one).
pub const COMPAT_DEFAULT_PORT: u16 = 8787;

/// One routed provider: where to really send traffic upstream and in which
/// protocol. Registered by the session layer, keyed by [`CompatTarget::app`].
#[derive(Clone, Debug)]
pub struct CompatTarget {
    pub app: AppKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub protocol: RequestProtocol,
    /// inbound model id → upstream model id; unmatched ids pass through.
    pub model_mappings: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct CompatRegistry {
    targets: Arc<RwLock<Vec<CompatTarget>>>,
}

impl CompatRegistry {
    pub fn set_target(&self, target: CompatTarget) {
        let mut targets = self.targets.write();
        targets.retain(|existing| existing.app != target.app);
        targets.push(target);
    }

    pub fn remove_target(&self, app: AppKind) {
        self.targets.write().retain(|existing| existing.app != app);
    }

    pub fn target_for_app(&self, app: AppKind) -> Option<CompatTarget> {
        self.targets.read().iter().find(|t| t.app == app).cloned()
    }

    pub fn routed_apps(&self) -> Vec<AppKind> {
        self.targets.read().iter().map(|t| t.app).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.targets.read().is_empty()
    }

    pub fn models(&self) -> Vec<String> {
        let mut models: Vec<String> = Vec::new();
        for target in self.targets.read().iter() {
            let mut push = |model: &str| {
                if !model.is_empty() && !models.iter().any(|m| m == model) {
                    models.push(model.to_string());
                }
            };
            push(&target.model);
            for (from, to) in &target.model_mappings {
                push(from);
                push(to);
            }
        }
        models
    }

    /// Pick the target a request belongs to. The live config hands each tool
    /// its own provider API key, so an exact key match wins; otherwise fall
    /// back to the sole target (two simultaneous routed apps sharing an empty
    /// key cannot be told apart and resolve to the first one).
    fn resolve(&self, presented_key: Option<&str>) -> Option<CompatTarget> {
        let targets = self.targets.read();
        if let Some(key) = presented_key.map(str::trim).filter(|k| !k.is_empty()) {
            if let Some(target) = targets
                .iter()
                .find(|t| !t.api_key.trim().is_empty() && t.api_key.trim() == key)
            {
                return Some(target.clone());
            }
        }
        targets.first().cloned()
    }
}

/// Lazily-started localhost HTTP listener hosting the compat endpoints.
pub struct CompatGateway {
    registry: CompatRegistry,
    addr: Option<SocketAddr>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Default for CompatGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CompatGateway {
    fn drop(&mut self) {
        // Abort the listener so a dropped workspace doesn't leak the port.
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl CompatGateway {
    pub fn new() -> Self {
        Self {
            registry: CompatRegistry::default(),
            addr: None,
            task: None,
        }
    }

    pub fn registry(&self) -> &CompatRegistry {
        &self.registry
    }

    pub fn port(&self) -> Option<u16> {
        self.addr.map(|addr| addr.port())
    }

    pub fn is_running(&self) -> bool {
        self.addr.is_some() && self.task.as_ref().is_some_and(|task| !task.is_finished())
    }

    pub async fn start(&mut self, requested_port: u16) -> Result<u16> {
        if self.is_running() {
            return Ok(self.port().unwrap_or(COMPAT_DEFAULT_PORT));
        }

        let requested = SocketAddr::from(([127, 0, 0, 1], requested_port));
        let listener = match TcpListener::bind(requested).await {
            Ok(listener) => listener,
            Err(err) if requested_port != 0 => {
                tracing::warn!(%err, requested_port, "compat port busy; using random port");
                TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?
            }
            Err(err) => return Err(err.into()),
        };

        let addr = listener.local_addr()?;
        let app = build_compat_router(self.registry.clone());
        let task = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                tracing::error!(%err, "compat gateway server error");
            }
        });

        self.addr = Some(addr);
        self.task = Some(task);
        tracing::info!(port = addr.port(), "compat routing gateway started");
        Ok(addr.port())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.addr = None;
        Ok(())
    }
}

pub fn build_compat_router(registry: CompatRegistry) -> Router {
    Router::new()
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(responses))
        .route("/v1/models", get(list_models))
        // Tools build request URLs from their configured base URL, which does
        // not always carry a `/v1` prefix — accept the bare forms too.
        .route("/messages", post(anthropic_messages))
        .route("/chat/completions", post(chat_completions))
        .route("/responses", post(responses))
        .route("/models", get(list_models))
        // Codex 新客户端把搜索命令发到独立的 /alpha/search 协议(而非内嵌在
        // /responses 里); 不注册会在路由层直接 404。载荷按 Responses 方言处理。
        .route("/alpha/search", post(responses))
        .route("/v1/alpha/search", post(responses))
        .fallback(compat_fallback)
        .layer(DefaultBodyLimit::disable())
        .with_state(registry)
}

async fn compat_fallback(
    State(registry): State<CompatRegistry>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    // Some tools sit behind provider URLs with custom prefixes
    // (`https://host/api/v1`); dispatch on the endpoint suffix so the
    // preserved path still lands on the right dialect.
    let dialect = if request.uri().path().ends_with("/messages") {
        InboundDialect::Anthropic
    } else if request.uri().path().ends_with("/chat/completions") {
        InboundDialect::Chat
    } else if request.uri().path().ends_with("/responses") {
        InboundDialect::Responses
    } else {
        return Ok(json_response(
            StatusCode::NOT_FOUND,
            &json!({"error": {"message": "unknown compat endpoint"}}),
        ));
    };
    route_compat(registry, request, dialect).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InboundDialect {
    Anthropic,
    Chat,
    Responses,
}

async fn anthropic_messages(
    State(registry): State<CompatRegistry>,
    request: Request<axum::body::Body>,
) -> Result<Response<Body>> {
    route_compat(registry, request, InboundDialect::Anthropic).await
}

async fn chat_completions(
    State(registry): State<CompatRegistry>,
    request: Request<axum::body::Body>,
) -> Result<Response<Body>> {
    route_compat(registry, request, InboundDialect::Chat).await
}

async fn responses(
    State(registry): State<CompatRegistry>,
    request: Request<axum::body::Body>,
) -> Result<Response<Body>> {
    route_compat(registry, request, InboundDialect::Responses).await
}

async fn list_models(State(registry): State<CompatRegistry>) -> Response<Body> {
    let data: Vec<Value> = registry
        .models()
        .into_iter()
        .map(|id| json!({"id": id, "object": "model", "owned_by": "router-switch"}))
        .collect();
    json_response(StatusCode::OK, &json!({"object": "list", "data": data}))
}

async fn route_compat(
    registry: CompatRegistry,
    request: Request<Body>,
    dialect: InboundDialect,
) -> Result<Response<Body>> {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|err| crate::GatewayError::Protocol(format!("read body: {err}")))?;
    let presented_key = presented_api_key(&parts.headers);
    let Some(target) = registry.resolve(presented_key.as_deref()) else {
        return Err(crate::GatewayError::Config(
            "no routed provider registered on the compat gateway; switch to a provider whose \
             request protocol differs from the tool's native one first"
                .into(),
        ));
    };

    let payload: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };

    let mut parsed = match dialect {
        InboundDialect::Anthropic => anthropic_in::parse(&payload)?,
        InboundDialect::Chat => chat_in::parse(&payload)?,
        InboundDialect::Responses => responses_in::parse(&payload)?,
    };

    let requested_model = parsed.model.clone();
    let upstream_model = map_model(&target, &requested_model);
    parsed.extra = normalize_extra_for_upstream(parsed.extra, target.protocol);
    // Anthropic 托管 WebSearch → OpenAI Responses 内建 web_search 工具:
    // 搜索在上游执行, 结果与引用随后随响应文本返回。
    if parsed.hosted_web_search && target.protocol == RequestProtocol::OpenAiResponses {
        parsed.extra["hosted_web_search"] = Value::Bool(true);
    } else {
        parsed
            .extra
            .as_object_mut()
            .map(|obj| obj.remove("hosted_web_search"));
    }

    let invocation = ModelInvocation {
        model: upstream_model.clone(),
        prompt: parsed.prompt,
        history: parsed.history,
        extra_params: parsed.extra,
    };
    let provider = create_provider(
        target.protocol.as_str(),
        target.base_url.clone(),
        target.api_key.clone(),
        upstream_model,
    );
    let events = provider.stream(invocation, CancellationToken::new());
    let display_model = if requested_model.is_empty() {
        target.model.clone()
    } else {
        requested_model
    };

    if parsed.stream {
        let body = match dialect {
            InboundDialect::Anthropic => anthropic_in::stream_body(display_model, events),
            InboundDialect::Chat => chat_in::stream_body(display_model, events),
            InboundDialect::Responses => responses_in::stream_body(display_model, events),
        };
        let mut response = Response::new(body);
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/event-stream"),
        );
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        Ok(response)
    } else {
        let collected = collect_events(events).await?;
        let payload = match dialect {
            InboundDialect::Anthropic => anthropic_in::complete_message(&display_model, &collected),
            InboundDialect::Chat => chat_in::complete_message(&display_model, &collected),
            InboundDialect::Responses => responses_in::complete_message(&display_model, &collected),
        };
        Ok(json_response(StatusCode::OK, &payload))
    }
}

fn presented_api_key(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return Some(value.to_string());
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
}

fn map_model(target: &CompatTarget, requested: &str) -> String {
    let requested = requested.trim();
    if requested.is_empty() {
        return target.model.clone();
    }
    for (from, to) in &target.model_mappings {
        if from == requested {
            return to.clone();
        }
    }
    requested.to_string()
}

/// Unify the token-limit knob with the name the upstream dialect expects so a
/// cross-protocol hop never carries a foreign field (Anthropic requires
/// `max_tokens`; OpenAI Chat wants `max_tokens`, Responses `max_output_tokens`).
fn normalize_extra_for_upstream(mut extra: Value, upstream: RequestProtocol) -> Value {
    let mut limit = None;
    if let Some(object) = extra.as_object_mut() {
        for key in ["max_tokens", "max_completion_tokens", "max_output_tokens"] {
            if let Some(value) = object.remove(key).filter(|v| !v.is_null()) {
                limit = Some(value);
                break;
            }
        }
    }
    if let Some(limit) = limit {
        match upstream {
            RequestProtocol::OpenAiResponses => extra["max_output_tokens"] = limit,
            _ => extra["max_tokens"] = limit,
        }
    } else if upstream == RequestProtocol::Anthropic {
        extra["max_tokens"] = json!(8192u64);
    }
    extra
}

fn json_response(status: StatusCode, payload: &Value) -> Response<Body> {
    let mut response = Response::new(Body::from(payload.to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    response
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub(crate) fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Copy the caller's sampling knobs we are willing to forward upstream; every
/// other dialect-specific field is dropped rather than risk a 400 upstream.
pub(crate) fn take_extra(body: &Value, keys: &[&str]) -> Value {
    let mut extra = serde_json::Map::new();
    for key in keys {
        if let Some(value) = body.get(*key).filter(|v| !v.is_null()) {
            extra.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(extra)
}

pub(crate) fn sse_event(event: &str, data: &Value) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

pub(crate) fn sse_data(data: &Value) -> String {
    format!("data: {data}\n\n")
}

/// Outcome of a parse: everything the provider needs plus how to answer.
pub(crate) struct ParsedInbound {
    pub model: String,
    pub prompt: crate::model::PromptSpec,
    pub history: Vec<crate::model::ProviderMessage>,
    pub stream: bool,
    pub extra: Value,
    /// 请求携带 Anthropic 托管 WebSearch 工具(web_search_20250305 等)
    pub hosted_web_search: bool,
}

#[derive(Debug, Default)]
pub(crate) struct CollectedTool {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug)]
pub(crate) struct CollectedResponse {
    pub text: String,
    pub thinking: String,
    pub tools: Vec<CollectedTool>,
    pub usage: Usage,
    pub finish: FinishReason,
}

impl Default for CollectedResponse {
    fn default() -> Self {
        Self {
            text: String::new(),
            thinking: String::new(),
            tools: Vec::new(),
            usage: Usage::default(),
            finish: FinishReason::Stop,
        }
    }
}

pub(crate) async fn collect_events(mut events: ProviderStream) -> Result<CollectedResponse> {
    let mut collected = CollectedResponse::default();
    while let Some(item) = events.next().await {
        match item? {
            ModelEvent::TextDelta(text) => collected.text.push_str(&text),
            ModelEvent::ThinkingDelta(text) => collected.thinking.push_str(&text),
            ModelEvent::ToolCallStart {
                index,
                call_id,
                name,
            } => {
                while collected.tools.len() <= index {
                    collected.tools.push(CollectedTool::default());
                }
                collected.tools[index] = CollectedTool {
                    call_id,
                    name,
                    arguments: String::new(),
                };
            }
            ModelEvent::ToolCallArgumentsDelta { index, delta } => {
                if let Some(tool) = collected.tools.get_mut(index) {
                    tool.arguments.push_str(&delta);
                }
            }
            ModelEvent::ToolCallEnd { .. } => {}
            ModelEvent::Usage(usage) => collected.usage = usage,
            ModelEvent::Done(finish) => {
                collected.finish = finish;
                break;
            }
        }
    }
    Ok(collected)
}

/// A dialect's streaming renderer: pure functions from provider events to SSE
/// frames so the async pump below stays dialect-independent.
pub(crate) trait DialectRenderer: Send {
    fn start_frames(&mut self) -> Vec<String>;
    fn event_frames(&mut self, event: &ModelEvent) -> Vec<String>;
    /// Closing frames; no-op when [`ModelEvent::Done`] already closed things.
    fn finish_frames(&mut self) -> Vec<String>;
    fn error_frames(&mut self, message: &str) -> Vec<String>;
}

pub(crate) fn stream_body(
    mut renderer: impl DialectRenderer + 'static,
    mut events: ProviderStream,
) -> Body {
    let stream = async_stream::stream! {
        for frame in renderer.start_frames() {
            yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(frame));
        }
        while let Some(item) = events.next().await {
            match item {
                Ok(event) => {
                    let done = matches!(event, ModelEvent::Done(_));
                    for frame in renderer.event_frames(&event) {
                        yield Ok(Bytes::from(frame));
                    }
                    if done {
                        break;
                    }
                }
                Err(err) => {
                    for frame in renderer.error_frames(&err.to_string()) {
                        yield Ok(Bytes::from(frame));
                    }
                    break;
                }
            }
        }
        for frame in renderer.finish_frames() {
            yield Ok(Bytes::from(frame));
        }
    };
    Body::from_stream(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post as post_route;

    fn sse_line(value: Value) -> String {
        format!("data: {value}\n\n")
    }

    fn chat_upstream_body(text: &str, with_tool: bool) -> String {
        let mut body =
            String::from("data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"");
        body.push_str(text);
        body.push_str("\"}}]}\n\n");
        if with_tool {
            body.push_str(&sse_line(json!({
                "choices": [{"delta": {"tool_calls": [{
                    "index": 0, "id": "call-1", "type": "function",
                    "function": {"name": "read_file", "arguments": "{\"path\":"},
                }]}}],
            })));
            body.push_str(&sse_line(json!({
                "choices": [{"delta": {"tool_calls": [{
                    "index": 0, "function": {"arguments": "\"a.md\"}"},
                }]}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5},
            })));
            body.push_str(&sse_line(json!({
                "choices": [{"delta": {}, "finish_reason": "tool_calls"}],
            })));
        } else {
            body.push_str(&sse_line(json!({
                "choices": [{"delta": {}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 3, "completion_tokens": 2},
            })));
        }
        body.push_str("data: [DONE]\n\n");
        body
    }

    async fn spawn_chat_upstream(with_tool: bool) -> String {
        let app = Router::new().route(
            "/v1/chat/completions",
            post_route(move |headers: HeaderMap, body: String| async move {
                // Echo a marker so tests can assert the upstream request shape.
                let wants_tool = headers.get("x-expect-tool").is_some();
                let _ = body;
                axum::http::Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/event-stream")
                    .body(Body::from(chat_upstream_body(
                        "hi",
                        wants_tool || with_tool,
                    )))
                    .unwrap()
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}/v1")
    }

    fn routed_gateway(base_url: String) -> CompatGateway {
        let gateway = CompatGateway::new();
        gateway.registry().set_target(CompatTarget {
            app: AppKind::Claude,
            base_url,
            api_key: "sk-mock-key-12345".into(),
            model: "gpt-test".into(),
            protocol: RequestProtocol::OpenAiChat,
            model_mappings: vec![("claude-sonnet-4-5".into(), "gpt-test".into())],
        });
        gateway
    }

    fn anthropic_request(stream: bool) -> Value {
        json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1024,
            "stream": stream,
            "system": "be brief",
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "tool_result", "tool_use_id": "call-1", "content": "file body"},
                ]},
            ],
            "tools": [
                {"name": "read_file", "description": "read a file", "input_schema": {"type": "object"}},
            ],
        })
    }

    #[tokio::test]
    async fn anthropic_stream_routes_to_chat_upstream() {
        let upstream = spawn_chat_upstream(true).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages"))
            .header("x-api-key", "sk-mock-key-12345")
            .json(&anthropic_request(true))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let text = response.text().await.unwrap();
        assert!(text.contains("event: message_start"), "{text}");
        assert!(text.contains("\"text\":\"hi\""), "{text}");
        assert!(
            text.contains("\"type\": \"tool_use\"") || text.contains("\"type\":\"tool_use\""),
            "{text}"
        );
        assert!(text.contains("input_json_delta"), "{text}");
        assert!(text.contains("\"stop_reason\":\"tool_use\""), "{text}");
        assert!(text.contains("event: message_stop"), "{text}");
    }

    #[tokio::test]
    async fn anthropic_non_stream_returns_message_json() {
        let upstream = spawn_chat_upstream(true).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages"))
            .header("x-api-key", "sk-mock-key-12345")
            .json(&anthropic_request(false))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let payload: Value = response.json().await.unwrap();
        assert_eq!(payload["type"], "message");
        assert_eq!(payload["stop_reason"], "tool_use");
        let content = payload["content"].as_array().unwrap();
        assert!(content
            .iter()
            .any(|b| b["type"] == "text" && b["text"] == "hi"));
        let tool = content
            .iter()
            .find(|b| b["type"] == "tool_use")
            .expect("tool_use block");
        assert_eq!(tool["id"], "call-1");
        assert_eq!(tool["name"], "read_file");
        assert_eq!(tool["input"]["path"], "a.md");
        assert_eq!(payload["usage"]["input_tokens"], 10);
    }

    #[tokio::test]
    async fn chat_inbound_routes_through_gateway() {
        let upstream = spawn_chat_upstream(false).await;
        let mut gateway = CompatGateway::new();
        gateway.registry().set_target(CompatTarget {
            app: AppKind::Grok,
            base_url: upstream,
            api_key: "sk-grok-key".into(),
            model: "gpt-test".into(),
            protocol: RequestProtocol::OpenAiChat,
            model_mappings: Vec::new(),
        });
        let port = gateway.start(0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .bearer_auth("sk-grok-key")
            .json(&json!({
                "model": "grok-4.5",
                "stream": true,
                "messages": [
                    {"role": "system", "content": "be brief"},
                    {"role": "user", "content": "hello"},
                ],
            }))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let text = response.text().await.unwrap();
        assert!(text.contains("chat.completion.chunk"), "{text}");
        assert!(text.contains("\"content\":\"hi\""), "{text}");
        assert!(text.contains("\"finish_reason\":\"stop\""), "{text}");
        assert!(text.contains("[DONE]"), "{text}");
    }

    #[tokio::test]
    async fn responses_inbound_routes_to_chat_upstream() {
        let upstream = spawn_chat_upstream(false).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .bearer_auth("sk-mock-key-12345")
            .json(&json!({
                "model": "gpt-5.3",
                "stream": true,
                "instructions": "be brief",
                "input": [
                    {"type": "message", "role": "user",
                     "content": [{"type": "input_text", "text": "hello"}]},
                ],
                "tools": [{"type": "function", "name": "shell", "description": "run", "parameters": {"type": "object"}}],
            }))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let text = response.text().await.unwrap();
        assert!(text.contains("event: response.created"), "{text}");
        assert!(text.contains("response.output_item.added"), "{text}");
        assert!(text.contains("response.output_text.delta"), "{text}");
        assert!(text.contains("response.completed"), "{text}");
        assert!(text.contains("\"status\":\"completed\""), "{text}");
    }

    #[tokio::test]
    async fn missing_target_is_rejected() {
        let gateway = CompatGateway::new();
        let mut gateway = gateway;
        let port = gateway.start(0).await.unwrap();
        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages"))
            .json(&anthropic_request(false))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn models_lists_registered_targets() {
        let upstream = spawn_chat_upstream(false).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();
        let payload: Value = reqwest::get(format!("http://127.0.0.1:{port}/v1/models"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let ids: Vec<&str> = payload["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"claude-sonnet-4-5"));
        assert!(ids.contains(&"gpt-test"));
    }

    #[tokio::test]
    async fn bare_and_prefixed_endpoint_paths_are_accepted() {
        let upstream = spawn_chat_upstream(false).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();
        let client = reqwest::Client::new();

        // Codex posts to `<base>/responses` when its base URL has no /v1.
        let bare = client
            .post(format!("http://127.0.0.1:{port}/responses"))
            .bearer_auth("sk-mock-key-12345")
            .json(&json!({"model": "gpt-5.3", "stream": false, "input": "hello"}))
            .send()
            .await
            .unwrap();
        assert!(bare.status().is_success());
        assert_eq!(bare.json::<Value>().await.unwrap()["object"], "response");

        // A preserved custom prefix (e.g. `https://host/api/v1`) still lands.
        let prefixed = client
            .post(format!("http://127.0.0.1:{port}/api/v1/messages"))
            .header("x-api-key", "sk-mock-key-12345")
            .json(&anthropic_request(false))
            .send()
            .await
            .unwrap();
        assert!(prefixed.status().is_success());
        assert_eq!(prefixed.json::<Value>().await.unwrap()["type"], "message");

        let unknown = client
            .post(format!("http://127.0.0.1:{port}/nope"))
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(unknown.status(), reqwest::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn alpha_search_endpoint_is_served() {
        // Codex 新客户端把搜索发到 /alpha/search; 未注册时会在路由层 404
        let upstream = spawn_chat_upstream(false).await;
        let mut gateway = routed_gateway(upstream);
        let port = gateway.start(0).await.unwrap();
        for path in ["/alpha/search", "/v1/alpha/search"] {
            let response = reqwest::Client::new()
                .post(format!("http://127.0.0.1:{port}{path}"))
                .bearer_auth("sk-mock-key-12345")
                .json(&json!({"model": "gpt-5.3", "stream": false, "input": "search weather"}))
                .send()
                .await
                .unwrap();
            assert!(
                response.status().is_success(),
                "{path} -> {}",
                response.status()
            );
        }
    }

    #[test]
    fn anthropic_hosted_web_search_is_not_sent_as_function() {
        let body = json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1024,
            "stream": false,
            "messages": [{"role": "user", "content": "search"}],
            "tools": [
                {"type": "web_search_20250305", "name": "web_search", "max_uses": 8},
                {"type": "text_editor_20250124", "name": "str_replace_editor"},
                {"name": "read_file", "description": "read", "input_schema": {"type": "object"}},
            ],
        });
        let parsed = anthropic_in::parse(&body).unwrap();
        assert!(parsed.hosted_web_search);
        // 只有普通 function 工具进入 tools 列表
        assert_eq!(parsed.prompt.tools.len(), 1);
        assert_eq!(parsed.prompt.tools[0].name, "read_file");
    }

    #[test]
    fn anthropic_parse_maps_tool_flow() {
        let parsed = anthropic_in::parse(&anthropic_request(true)).unwrap();
        assert_eq!(parsed.model, "claude-sonnet-4-5");
        assert!(parsed.stream);
        assert_eq!(parsed.prompt.instructions, "be brief");
        assert_eq!(parsed.prompt.tools.len(), 1);
        assert_eq!(parsed.prompt.tools[0].name, "read_file");
        assert_eq!(parsed.history.len(), 2);
        assert!(matches!(parsed.history[0].role, crate::model::Role::User));
        assert!(matches!(parsed.history[1].role, crate::model::Role::Tool));
        assert_eq!(parsed.history[1].tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(parsed.history[1].content, "file body");
        assert_eq!(parsed.extra["max_tokens"], 1024);
    }

    #[test]
    fn responses_parse_maps_codex_input() {
        let parsed = responses_in::parse(&json!({
            "model": "gpt-5.3",
            "instructions": "be brief",
            "input": [
                {"type": "message", "role": "user",
                 "content": [{"type": "input_text", "text": "list files"}]},
                {"type": "function_call", "call_id": "call-9", "name": "shell",
                 "arguments": "{\"cmd\":\"ls\"}"},
                {"type": "function_call_output", "call_id": "call-9", "output": "a.md"},
            ],
            "tools": [{"type": "function", "name": "shell", "parameters": {"type": "object"}}],
            "max_output_tokens": 512,
        }))
        .unwrap();
        assert_eq!(parsed.prompt.instructions, "be brief");
        assert_eq!(parsed.prompt.tools.len(), 1);
        assert_eq!(parsed.history.len(), 3);
        assert!(matches!(
            parsed.history[1].role,
            crate::model::Role::Assistant
        ));
        assert_eq!(parsed.history[1].tool_calls[0].call_id, "call-9");
        assert!(matches!(parsed.history[2].role, crate::model::Role::Tool));
        assert_eq!(parsed.history[2].tool_call_id.as_deref(), Some("call-9"));
        assert_eq!(parsed.extra["max_output_tokens"], 512);
    }

    #[test]
    fn extra_params_are_normalized_per_upstream() {
        let inbound = json!({"max_completion_tokens": 777});
        let extra = normalize_extra_for_upstream(inbound, RequestProtocol::Anthropic);
        assert_eq!(extra["max_tokens"], 777);
        assert!(extra.get("max_completion_tokens").is_none());

        let extra = normalize_extra_for_upstream(
            json!({"max_tokens": 777}),
            RequestProtocol::OpenAiResponses,
        );
        assert_eq!(extra["max_output_tokens"], 777);

        let extra = normalize_extra_for_upstream(json!({}), RequestProtocol::Anthropic);
        assert_eq!(extra["max_tokens"], 8192);
    }

    #[test]
    fn model_mapping_and_registry_resolution() {
        let registry = CompatRegistry::default();
        registry.set_target(CompatTarget {
            app: AppKind::Claude,
            base_url: "https://api.example.com/v1".into(),
            api_key: "key-a".into(),
            model: "model-a".into(),
            protocol: RequestProtocol::OpenAiChat,
            model_mappings: vec![("alias".into(), "upstream-a".into())],
        });
        registry.set_target(CompatTarget {
            app: AppKind::Codex,
            base_url: "https://api.example.com/v1".into(),
            api_key: "key-b".into(),
            model: "model-b".into(),
            protocol: RequestProtocol::Anthropic,
            model_mappings: Vec::new(),
        });

        assert_eq!(
            registry.target_for_app(AppKind::Codex).unwrap().model,
            "model-b"
        );
        let target = registry.resolve(Some("key-a")).unwrap();
        assert_eq!(target.model, "model-a");
        assert_eq!(map_model(&target, "alias"), "upstream-a");
        assert_eq!(map_model(&target, "unknown"), "unknown");
        assert_eq!(map_model(&target, ""), "model-a");

        registry.remove_target(AppKind::Claude);
        assert!(registry.target_for_app(AppKind::Claude).is_none());
        assert_eq!(registry.resolve(Some("key-a")).unwrap().model, "model-b");
    }
}
