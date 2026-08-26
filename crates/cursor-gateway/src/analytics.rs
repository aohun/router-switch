use axum::{
    body::Body,
    http::{header, Response, StatusCode},
};
use serde_json::json;

use crate::Result;

pub async fn bootstrap_statsig() -> Result<Response<Body>> {
    let body = json!({
        "feature_gates": {
            "cursor_composer": { "value": true },
            "cursor_agent": { "value": true },
            "cursor_agent_v2": { "value": true },
            "cursor_tab": { "value": true },
            "cursor_cpp": { "value": true },
            "cursor_cmux": { "value": true },
            "cursor_shadow_workspace": { "value": true }
        },
        "dynamic_configs": {},
        "layer_configs": {},
        "sdkParams": {},
        "has_updates": true,
        "time": chrono::Utc::now().timestamp_millis()
    });

    let bytes = serde_json::to_vec(&body)?;
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    Ok(response)
}

pub async fn first_window_statsig() -> Result<Response<Body>> {
    bootstrap_statsig().await
}
