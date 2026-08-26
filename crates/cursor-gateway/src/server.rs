use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{DefaultBodyLimit, Extension},
    routing::{get, post},
    Router,
};
use domain::CursorSettings;
use parking_lot::RwLock;
use tokio::net::TcpListener;
use tower_http::decompression::RequestDecompressionLayer;

use crate::{
    account, analytics,
    bidi_append::{self, AppState},
    chat, model_catalog,
    proxy::{self, CursorProxy},
    run_sse,
    session::CursorSessionRegistry,
    Result,
};

pub fn build_router(
    registry: CursorSessionRegistry,
    settings: Arc<RwLock<Option<CursorSettings>>>,
    proxy: CursorProxy,
) -> Router {
    let app_state = AppState {
        registry,
        settings: settings.clone(),
    };

    Router::new()
        .route(
            "/agent.v1.AgentService/RunSSE",
            post(run_sse::run_sse_handler),
        )
        .route(
            "/aiserver.v1.BidiService/BidiAppend",
            post(bidi_append::bidi_append_handler),
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
        .with_state(app_state)
}

pub async fn start_backend_server(
    registry: CursorSessionRegistry,
    settings: Arc<RwLock<Option<CursorSettings>>>,
    requested_port: u16,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let proxy = CursorProxy::default_upstream();
    let app = build_router(registry, settings, proxy);

    let requested = SocketAddr::from(([127, 0, 0, 1], requested_port));
    let listener = match TcpListener::bind(requested).await {
        Ok(l) => l,
        Err(e) if requested_port != 0 => {
            tracing::warn!(%requested, %e, "backend port busy; using random port");
            TcpListener::bind("127.0.0.1:0").await?
        }
        Err(e) => return Err(e.into()),
    };

    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            tracing::error!(%err, "Cursor Axum backend server error");
        }
    });

    Ok((addr, handle))
}
