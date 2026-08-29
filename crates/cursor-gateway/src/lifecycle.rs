use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use prost::Message;

use crate::{
    connect::{
        encode_end_stream, encode_error_end_stream, ConnectCode, ConnectErrorDetail,
        ConnectStreamError,
    },
    proto::aiserver::v1 as ai,
    sessions::CursorSessionHandle,
    GatewayError, Result,
};

pub fn finish_success(handle: &CursorSessionHandle) {
    handle.emit_frame(encode_end_stream());
    handle.close_output();
}

pub fn fail(handle: &CursorSessionHandle, error: &GatewayError) -> Result<()> {
    let stream_error = match error {
        GatewayError::Provider(_) | GatewayError::Http(_) => provider_error(error),
        GatewayError::Protocol(message) => {
            plain_message(ConnectCode::InvalidArgument, message.clone())
        }
        GatewayError::Decode(_) | GatewayError::Json(_) | GatewayError::Hex(_) => {
            plain_error(ConnectCode::InvalidArgument, error)
        }
        GatewayError::RunNotFound(_) => plain_error(ConnectCode::NotFound, error),
        GatewayError::Cancelled => plain_error(ConnectCode::Canceled, error),
        _ => plain_error(ConnectCode::Internal, error),
    };
    handle.emit_frame(encode_error_end_stream(&stream_error)?);
    handle.close_output();
    Ok(())
}

fn plain_error(code: ConnectCode, error: &GatewayError) -> ConnectStreamError {
    plain_message(code, error.to_string())
}

fn plain_message(code: ConnectCode, message: String) -> ConnectStreamError {
    ConnectStreamError {
        code,
        message,
        details: Vec::new(),
    }
}

fn provider_error(error: &GatewayError) -> ConnectStreamError {
    let detail = ai::ErrorDetails {
        error: ai::error_details::Error::ProviderError as i32,
        details: Some(ai::CustomErrorDetails {
            title: "Provider Error".into(),
            detail: error.to_string(),
            allow_command_links_potentially_unsafe_please_only_use_for_handwritten_trusted_markdown:
                Some(true),
            is_retryable: Some(true),
            show_request_id: Some(true),
            should_show_immediate_error: Some(false),
        }),
        is_expected: Some(true),
    };
    ConnectStreamError {
        code: ConnectCode::Unavailable,
        message: error.to_string(),
        details: vec![ConnectErrorDetail {
            type_name: "aiserver.v1.ErrorDetails".into(),
            value: STANDARD_NO_PAD.encode(detail.encode_to_vec()),
        }],
    }
}
