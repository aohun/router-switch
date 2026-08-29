use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub type Result<T, E = GatewayError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("config: {0}")]
    Config(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("provider: {0}")]
    Provider(String),
    #[error("upstream: {0}")]
    Upstream(String),
    #[error("store: {0}")]
    Store(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("cancelled")]
    Cancelled,
    #[error("decode: {0}")]
    Decode(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Encode(#[from] prost::EncodeError),
    #[error(transparent)]
    ProstDecode(#[from] prost::DecodeError),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = match &self {
            GatewayError::Protocol(_) | GatewayError::Decode(_) | GatewayError::Hex(_) => {
                StatusCode::BAD_REQUEST
            }
            GatewayError::Upstream(_) | GatewayError::Http(_) => StatusCode::BAD_GATEWAY,
            GatewayError::Provider(_) => StatusCode::INTERNAL_SERVER_ERROR,
            GatewayError::RunNotFound(_) => StatusCode::NOT_FOUND,
            GatewayError::Cancelled => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
