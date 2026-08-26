use bytes::{BufMut, Bytes, BytesMut};
use prost::Message;
use serde::Serialize;

use crate::{GatewayError, Result};

pub const END_STREAM_FLAG: u8 = 0x02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectCode {
    Canceled,
    InvalidArgument,
    NotFound,
    Unavailable,
    Internal,
}

impl ConnectCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::InvalidArgument => "invalid_argument",
            Self::NotFound => "not_found",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConnectErrorDetail {
    #[serde(rename = "type")]
    pub type_name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectStreamError {
    pub code: ConnectCode,
    pub message: String,
    pub details: Vec<ConnectErrorDetail>,
}

#[derive(Serialize)]
struct EndStreamResponse<'a> {
    error: WireError<'a>,
}

#[derive(Serialize)]
struct WireError<'a> {
    code: &'static str,
    #[serde(skip_serializing_if = "str::is_empty")]
    message: &'a str,
    #[serde(skip_serializing_if = "details_are_empty")]
    details: &'a [ConnectErrorDetail],
}

fn details_are_empty(details: &&[ConnectErrorDetail]) -> bool {
    details.is_empty()
}

pub fn encode_message<M: Message>(message: &M) -> Result<Bytes> {
    let len = message.encoded_len();
    let mut output = BytesMut::with_capacity(5 + len);
    output.put_u8(0);
    output.put_u32(len as u32);
    message.encode(&mut output)?;
    Ok(output.freeze())
}

pub fn encode_raw_message(payload: &[u8]) -> Bytes {
    let mut output = BytesMut::with_capacity(5 + payload.len());
    output.put_u8(0);
    output.put_u32(payload.len() as u32);
    output.extend_from_slice(payload);
    output.freeze()
}

pub fn encode_end_stream() -> Bytes {
    encode_end_stream_payload(b"{}")
}

pub fn encode_error_end_stream(error: &ConnectStreamError) -> Result<Bytes> {
    let payload = serde_json::to_vec(&EndStreamResponse {
        error: WireError {
            code: error.code.as_str(),
            message: &error.message,
            details: &error.details,
        },
    })?;
    Ok(encode_end_stream_payload(&payload))
}

fn encode_end_stream_payload(payload: &[u8]) -> Bytes {
    let mut output = BytesMut::with_capacity(5 + payload.len());
    output.put_u8(END_STREAM_FLAG);
    output.put_u32(payload.len() as u32);
    output.extend_from_slice(payload);
    output.freeze()
}

/// Helper to encode a complete Connect-RPC framed response buffer (data frame + end-stream frame).
pub fn encode_connect_proto_response(message: &impl Message) -> Result<Vec<u8>> {
    let data_bytes = message.encode_to_vec();
    Ok(encode_connect_raw_response(&data_bytes))
}

/// Helper to encode a raw protobuf slice into a complete Connect-RPC framed response buffer.
pub fn encode_connect_raw_response(payload: &[u8]) -> Vec<u8> {
    let data_len = payload.len();
    let mut buf = Vec::with_capacity(5 + data_len + 5 + 2);
    // Data frame (flag 0x00, 4-byte big-endian len, payload)
    buf.push(0x00);
    buf.extend_from_slice(&(data_len as u32).to_be_bytes());
    buf.extend_from_slice(payload);
    // End-stream frame (flag 0x02, 4-byte big-endian len 2, b"{}")
    buf.push(END_STREAM_FLAG);
    buf.extend_from_slice(&2u32.to_be_bytes());
    buf.extend_from_slice(b"{}");
    buf
}

pub fn decode_unary<M: Message + Default>(body: &[u8]) -> Result<M> {
    if body.len() >= 5 {
        let flags = body[0];
        let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
        if flags & END_STREAM_FLAG == 0 && length == body.len() - 5 {
            return Ok(M::decode(&body[5..])?);
        }
    }
    Ok(M::decode(body)?)
}

pub fn decode_frames(mut body: &[u8]) -> Result<Vec<(u8, Bytes)>> {
    let mut frames = Vec::new();
    while !body.is_empty() {
        if body.len() < 5 {
            return Err(GatewayError::Protocol("truncated Connect envelope".into()));
        }
        let flags = body[0];
        let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
        body = &body[5..];
        if body.len() < length {
            return Err(GatewayError::Protocol("truncated Connect payload".into()));
        }
        frames.push((flags, Bytes::copy_from_slice(&body[..length])));
        body = &body[length..];
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Message)]
    struct Dummy {
        #[prost(string, tag = "1")]
        text: String,
    }

    #[test]
    fn connect_unary_roundtrip() {
        let original = Dummy {
            text: "hello".into(),
        };
        let encoded = encode_message(&original).unwrap();
        let decoded: Dummy = decode_unary(&encoded).unwrap();
        assert_eq!(decoded.text, "hello");
    }

    #[test]
    fn connect_raw_response_frames() {
        let dummy = Dummy {
            text: "test".into(),
        };
        let framed = encode_connect_proto_response(&dummy).unwrap();
        let frames = decode_frames(&framed).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].0, 0); // Data frame
        assert_eq!(frames[1].0, END_STREAM_FLAG); // End stream
    }
}
