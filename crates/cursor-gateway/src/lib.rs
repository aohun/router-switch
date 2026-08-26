pub mod account;
pub mod analytics;
pub mod bidi_append;
pub mod chat;
pub mod connect;
pub mod error;
pub mod harness;
pub mod model_catalog;
pub mod proto;
pub mod provider;
pub mod proxy;
pub mod run_sse;
pub mod runtime;
pub mod server;
pub mod session;

pub use error::{GatewayError, Result};
pub use harness::{
    clear_proxy_settings, ensure_local_ultra_account, ensure_local_ultra_account_at,
    force_inject_ultra, force_inject_ultra_at, inject_if_missing, inject_if_missing_at,
    settings_match, settings_path, state_db_path, write_proxy_settings, CaManager, CaState,
    LoadedCa, ProxyRuntime,
};
pub use runtime::CursorGatewayRuntime;
pub use server::{build_router, start_backend_server};
pub use session::CursorSessionRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::to_bytes,
        http::{header, Request},
    };
    use domain::{CursorKind, CursorModelMapping, CursorSettings};
    use parking_lot::RwLock;
    use prost::Message;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_account_endpoints_return_proto_and_json() {
        let registry = CursorSessionRegistry::new();
        let settings = Arc::new(RwLock::new(None));
        let proxy = proxy::CursorProxy::default_upstream();
        let router = build_router(registry, settings, proxy);

        // Unary Protobuf endpoints
        let res = router
            .clone()
            .oneshot(
                Request::post("/aiserver.v1.AuthService/GetEmail")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/proto"
        );

        let res = router
            .clone()
            .oneshot(
                Request::post("/aiserver.v1.DashboardService/GetMe")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/proto"
        );

        // JSON endpoints
        let res = router
            .clone()
            .oneshot(
                Request::get("/auth/session")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let res = router
            .clone()
            .oneshot(
                Request::post("/aiserver.v1.AnalyticsService/BootstrapStatsig")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_available_models_returns_configured_third_party_models() {
        let registry = CursorSessionRegistry::new();
        let custom_settings = CursorSettings {
            kind: CursorKind::ThirdParty,
            api_key: "sk-test".into(),
            base_url: "https://api.example.com/v1".into(),
            model: "custom-gpt-5".into(),
            provider_type: "openai-chat".into(),
            options: serde_json::json!({}),
            model_mappings: vec![CursorModelMapping {
                model: "custom-claude-4".into(),
                display_name: "Custom Claude 4".into(),
                context_window: Some(128000),
                reasoning_effort: None,
            }],
        };
        let settings = Arc::new(RwLock::new(Some(custom_settings)));
        let proxy = proxy::CursorProxy::default_upstream();
        let router = build_router(registry, settings, proxy);

        let res = router
            .clone()
            .oneshot(
                Request::post("/aiserver.v1.AiService/AvailableModels")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/proto"
        );

        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let decoded = proto::catalog::AvailableModelsAddition::decode(body_bytes).unwrap();
        assert!(decoded.model_names.contains(&"custom-gpt-5".to_string()));
        assert!(decoded.model_names.contains(&"custom-claude-4".to_string()));
        assert!(decoded.models.iter().any(|m| m.name == "custom-gpt-5"));
        assert!(decoded.models.iter().any(|m| m.name == "custom-claude-4"));
    }
}
