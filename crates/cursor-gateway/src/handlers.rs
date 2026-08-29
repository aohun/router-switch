use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{DefaultBodyLimit, Extension, State},
    http::{header, HeaderMap, HeaderValue, Request, Response, StatusCode},
    routing::{get, post},
    Router,
};
use tower_http::decompression::RequestDecompressionLayer;

use crate::{
    account, analytics, bidi_append, chat, connect, model_catalog,
    proto::{agent::v1 as agent, aiserver::v1 as ai},
    proxy::{self, CursorProxy},
    run_sse,
    sessions::{CursorParent, CursorRoute, CursorSessionRegistry},
    Result,
};

#[derive(Clone)]
pub struct AppState {
    pub registry: CursorSessionRegistry,
}

pub fn build_router(registry: CursorSessionRegistry, proxy: CursorProxy) -> Router {
    let state = AppState {
        registry: registry.clone(),
    };
    Router::new()
        .route(
            "/__router-switch__/healthz",
            get(|| async { StatusCode::NO_CONTENT }),
        )
        .route("/agent.v1.AgentService/RunSSE", post(run_sse_handler))
        .route(
            "/aiserver.v1.BidiService/BidiAppend",
            post(bidi_append_handler),
        )
        .route(
            "/aiserver.v1.AiService/AvailableModels",
            post(model_catalog::available_models),
        )
        .route(
            "/agent.v1.AgentService/GetUsableModels",
            post(model_catalog::usable_models),
        )
        .route(
            "/aiserver.v1.AiService/GetUsableModels",
            post(model_catalog::usable_models),
        )
        .route(
            "/aiserver.v1.AuthService/GetEmail",
            post(account::get_email),
        )
        .route("/aiserver.v1.DashboardService/GetMe", post(account::get_me))
        .route(
            "/aiserver.v1.DashboardService/GetTeams",
            post(account::get_teams),
        )
        .route(
            "/aiserver.v1.DashboardService/GetUserProfile",
            post(account::get_user_profile),
        )
        .route(
            "/aiserver.v1.DashboardService/GetCurrentPeriodUsage",
            post(account::current_period_usage),
        )
        .route(
            "/aiserver.v1.DashboardService/GetUsageLimitStatusAndActiveGrants",
            post(account::usage_limit_status),
        )
        .route(
            "/aiserver.v1.AnalyticsService/BootstrapStatsig",
            post(analytics::bootstrap_statsig),
        )
        .route(
            "/aiserver.v1.AnalyticsService/GetFirstWindowStatsigDecision",
            post(analytics::first_window_statsig),
        )
        .route("/auth/full_stripe_profile", get(account::stripe_profile))
        .route("/auth/stripe_profile", get(account::stripe_profile))
        .route(
            "/auth/session",
            get(account::auth_session).post(account::auth_session),
        )
        .route(
            "/api/auth/session",
            get(account::auth_session).post(account::auth_session),
        )
        .route(
            "/auth/user",
            get(account::auth_user).post(account::auth_user),
        )
        .route(
            "/api/auth/user",
            get(account::auth_user).post(account::auth_user),
        )
        .route(
            "/auth/poll",
            get(account::auth_poll).post(account::auth_poll),
        )
        .route(
            "/auth/logout",
            get(account::auth_logout).post(account::auth_logout),
        )
        .route(
            "/aiserver.v1.AiService/StreamChat",
            post(chat::stream_chat_handler),
        )
        .route(
            "/aiserver.v1.ChatService/StreamChat",
            post(chat::stream_chat_handler),
        )
        .route(
            "/aiserver.v1.AiService/StreamComposer",
            post(chat::stream_chat_handler),
        )
        .route(
            "/aiserver.v1.AiService/StreamComposer2",
            post(chat::stream_chat_handler),
        )
        .route_layer(DefaultBodyLimit::disable())
        .route_layer(RequestDecompressionLayer::new())
        .fallback(proxy::forward)
        .method_not_allowed_fallback(proxy::forward)
        .layer(Extension(proxy))
        .with_state(state)
}

async fn run_sse_handler(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let (parts, body) = buffered(request).await?;
    let request: agent::BidiRequestId = connect::decode_unary(&body)?;
    let route = state.registry.wait_route(&request.request_id).await;
    match route {
        CursorRoute::Local => run_sse::stream(&state.registry, &request.request_id).await,
        CursorRoute::Upstream(generation) => {
            let response = proxy::forward(
                Extension(proxy),
                Request::from_parts(parts, Body::from(body)),
            )
            .await?;
            Ok(run_sse::upstream(state.registry, request.request_id, generation, response).await)
        }
    }
}

async fn bidi_append_handler(
    State(state): State<AppState>,
    Extension(proxy): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let (parts, body) = buffered(request).await?;
    let request: ai::BidiAppendRequest = connect::decode_unary(&body)?;
    let decoded = bidi_append::decode(&request)?;
    let first_model = decoded.model_id().map(str::to_owned);

    // Official 服务商 → upstream; ThirdParty → local Agent kernel.
    // Continuing appends follow the route already established for request_id.
    let local = if first_model.is_some() {
        state.registry.is_local_kind()
    } else if state.registry.local(&decoded.request_id).await.is_some() {
        true
    } else if state.registry.upstream(&decoded.request_id).await {
        false
    } else if state.registry.is_local_kind() {
        true
    } else {
        false
    };

    if !local {
        if first_model.is_some() || !state.registry.upstream(&decoded.request_id).await {
            state.registry.mark_upstream(&decoded.request_id).await;
        }
        return proxy::forward(
            Extension(proxy),
            Request::from_parts(parts, Body::from(body)),
        )
        .await;
    }

    let parent = parent_headers(&parts.headers)?;
    bidi_append::append(&state.registry, decoded, parent).await?;
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/proto"),
    );
    Ok(response)
}

fn parent_headers(headers: &HeaderMap) -> Result<Option<CursorParent>> {
    let request_id = headers
        .get("x-cursor-parent-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let tool_call_id = headers
        .get("x-cursor-parent-tool-call-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    match (request_id, tool_call_id) {
        (Some(request_id), Some(tool_call_id)) => Ok(Some(CursorParent {
            request_id,
            tool_call_id,
        })),
        (None, None) => Ok(None),
        _ => Err(crate::GatewayError::Protocol(
            "parent request id and tool call id must both be present".into(),
        )),
    }
}

async fn buffered(request: Request<Body>) -> Result<(axum::http::request::Parts, Bytes)> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| crate::GatewayError::Protocol(format!("read body: {e}")))?;
    Ok((parts, body))
}
