use axum::{
    body::{Body, Bytes},
    extract::Extension,
    http::{header, Request, Response, StatusCode},
};
use prost::Message;
use serde_json::{json, Map, Value};

use crate::{
    proto::account::{
        Empty, GetCurrentPeriodUsageResponse, GetEmailResponse, GetMeResponse,
        GetUsageLimitStatusAndActiveGrantsResponse, GetUserProfileResponse, PlanUsage,
        SpendLimitUsage, UsageLimitPolicyStatus,
    },
    proxy::{self, CursorProxy},
    Result,
};

pub const LOCAL_AUTH_ID: &str = "local_ultra";
pub const LOCAL_EMAIL: &str = "cursor@ai.com";
pub const LOCAL_ULTRA_PLAN_INCLUDED_CENTS: i32 = 20_000;

pub async fn get_email(
    Extension(upstream): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    forward_or(upstream, request, || {
        proto(GetEmailResponse {
            email: LOCAL_EMAIL.into(),
            sign_up_type: 3,
        })
    })
    .await
}

pub async fn get_me(
    Extension(upstream): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    forward_or(upstream, request, || {
        proto(GetMeResponse {
            auth_id: LOCAL_AUTH_ID.into(),
            user_id: 1,
            email: Some(LOCAL_EMAIL.into()),
            first_name: Some("Cursor".into()),
            last_name: Some("Local".into()),
            created_at: Some(chrono::Utc::now().to_rfc3339()),
            is_enterprise_user: Some(false),
            email_domain_type: Some("personal".into()),
            country: Some("US".into()),
        })
    })
    .await
}

pub async fn get_teams(
    Extension(upstream): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    forward_or(upstream, request, || proto(Empty {})).await
}

pub async fn get_user_profile(
    Extension(upstream): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    forward_or(upstream, request, || {
        proto(GetUserProfileResponse {
            public_visibility_allowed: Some(true),
            max_visibility: Some("PUBLIC".into()),
        })
    })
    .await
}

pub async fn current_period_usage() -> Result<Response<Body>> {
    let now = chrono::Utc::now();
    proto(GetCurrentPeriodUsageResponse {
        billing_cycle_start: (now - chrono::Duration::days(30)).timestamp_millis(),
        billing_cycle_end: (now + chrono::Duration::days(10 * 365)).timestamp_millis(),
        plan_usage: Some(PlanUsage {
            total_spend: 0,
            included_spend: LOCAL_ULTRA_PLAN_INCLUDED_CENTS,
            remaining: LOCAL_ULTRA_PLAN_INCLUDED_CENTS,
            limit: LOCAL_ULTRA_PLAN_INCLUDED_CENTS,
            remaining_bonus: Some(false),
            bonus_tooltip: Some("Ultra local account active.".into()),
            auto_spend: Some(0),
            api_spend: Some(0),
            auto_percent_used: Some(0.0),
            api_percent_used: Some(0.0),
            total_percent_used: Some(0.0),
        }),
        spend_limit_usage: Some(SpendLimitUsage {
            limit_type: "user".into(),
        }),
        display_threshold: Some(99_999_999),
        enabled: true,
        display_message: "Ultra plan active".into(),
        auto_model_selected_display_message: Some("Ultra plan active".into()),
        named_model_selected_display_message: Some("Ultra plan active".into()),
    })
}

pub async fn usage_limit_status() -> Result<Response<Body>> {
    proto(GetUsageLimitStatusAndActiveGrantsResponse {
        usage_limit_policy_status: Some(UsageLimitPolicyStatus {
            is_in_slow_pool: false,
            features: Default::default(),
            can_configure_spend_limit: true,
            has_pending_request: false,
            allowed_model_ids: Vec::new(),
            allowed_model_tags: Vec::new(),
        }),
    })
}

pub async fn stripe_profile(
    Extension(upstream): Extension<CursorProxy>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    match proxy::forward_buffered(&upstream, request).await {
        Ok(response) if response.status.is_success() => {
            let mut profile = serde_json::from_slice::<Map<String, Value>>(&response.body)?;
            ultra(&mut profile);
            Ok(response.with_body(Bytes::from(serde_json::to_vec(&profile)?)))
        }
        Ok(response) => {
            tracing::warn!(
                status = %response.status,
                "Cursor account upstream rejected profile; using local Ultra identity"
            );
            json_response(ultra_profile())
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "Cursor account upstream unavailable; using local Ultra identity"
            );
            json_response(ultra_profile())
        }
    }
}

pub async fn auth_session() -> Result<Response<Body>> {
    json_response(json!({
        "user": {
            "id": "cursor-local-user",
            "email": LOCAL_EMAIL,
            "name": "Cursor User",
            "image": null
        },
        "expires": "2099-12-31T23:59:59.999Z",
        "accessToken": "cursor-mock-access-token"
    }))
}

pub async fn auth_user() -> Result<Response<Body>> {
    json_response(json!({
        "id": "cursor-local-user",
        "email": LOCAL_EMAIL,
        "name": "Cursor User",
        "membershipType": "ultra",
        "subscriptionStatus": "active"
    }))
}

pub async fn auth_poll() -> Result<Response<Body>> {
    json_response(json!({
        "status": "success",
        "token": "cursor-mock-token"
    }))
}

pub async fn auth_logout() -> Result<Response<Body>> {
    json_response(json!({"success": true}))
}

async fn forward_or(
    upstream: CursorProxy,
    request: Request<Body>,
    fallback: impl FnOnce() -> Result<Response<Body>>,
) -> Result<Response<Body>> {
    match proxy::forward_buffered(&upstream, request).await {
        Ok(response) if response.status.is_success() => Ok(response.into_response()),
        Ok(response) => {
            tracing::warn!(
                status = %response.status,
                "Cursor identity upstream rejected request; using local identity"
            );
            fallback()
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "Cursor identity upstream unavailable; using local identity"
            );
            fallback()
        }
    }
}

fn proto(message: impl Message) -> Result<Response<Body>> {
    let body = message.encode_to_vec();
    let length = body.len();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/proto"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
    Ok(response)
}

fn json_response(value: Value) -> Result<Response<Body>> {
    let bytes = serde_json::to_vec(&value)?;
    let length = bytes.len();
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
    Ok(response)
}

fn ultra(profile: &mut Map<String, Value>) {
    profile.insert("membershipType".into(), Value::String("ultra".into()));
    profile.insert(
        "individualMembershipType".into(),
        Value::String("ultra".into()),
    );
    profile.insert("subscriptionStatus".into(), Value::String("active".into()));
}

fn ultra_profile() -> Value {
    json!({
        "membershipType": "ultra",
        "individualMembershipType": "ultra",
        "subscriptionStatus": "active",
        "lastPaymentFailed": false,
        "pendingCancellationDate": null,
        "daysRemainingOnTrial": 0,
        "paymentId": LOCAL_AUTH_ID,
        "isTeamMember": false
    })
}
