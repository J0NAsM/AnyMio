//! Versioned, length-delimited relay signalling protocol.
//! The maximum wire size is checked before allocating message storage.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Message {
    Hello {
        protocol_version: u16,
        app_version: String,
    },
    Challenge {
        nonce: Vec<u8>,
    },
    Register {
        device_id: u32,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    RegisterOk {
        device_id: u32,
    },
    Lookup {
        device_id: u32,
    },
    LookupResult {
        device_id: u32,
        online: bool,
    },
    ConnectRequest {
        request_id: Uuid,
        from_device_id: u32,
        target_device_id: u32,
    },
    ConnectAccept {
        request_id: Uuid,
    },
    ConnectReject {
        request_id: Uuid,
        reason: RejectReason,
    },
    SessionStart {
        session_id: Uuid,
    },
    SessionEnd {
        session_id: Uuid,
        reason: String,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RejectReason {
    Expired,
    Rejected,
    TargetOffline,
    RateLimited,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidMessage,
    IncompatibleProtocol,
    NotRegistered,
    TargetOffline,
    AuthenticationFailed,
    RateLimited,
    Internal,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("network I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("message exceeds protocol limit")]
    TooLarge,
    #[error("message is invalid JSON")]
    InvalidJson,
    #[error("unsupported protocol version")]
    IncompatibleVersion,
}

pub async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &Message,
) -> Result<(), ProtocolError> {
    let encoded = serde_json::to_vec(message).map_err(|_| ProtocolError::InvalidJson)?;
    if encoded.len() > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    writer.write_u32(encoded.len() as u32).await?;
    writer.write_all(&encoded).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Message, ProtocolError> {
    let len = reader.read_u32().await? as usize;
    if len > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let mut data = vec![0_u8; len];
    reader.read_exact(&mut data).await?;
    serde_json::from_slice(&data).map_err(|_| ProtocolError::InvalidJson)
}

pub fn require_compatible(version: u16) -> Result<(), ProtocolError> {
    if version == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ProtocolError::IncompatibleVersion)
    }
}

pub fn registration_payload(nonce: &[u8], device_id: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(24 + nonce.len());
    payload.extend_from_slice(b"JREMOTE/2 REGISTER");
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(&device_id.to_be_bytes());
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;
    #[tokio::test]
    async fn round_trip() {
        let (mut a, mut b) = duplex(1024);
        let wanted = Message::Ping { nonce: 22 };
        write_message(&mut a, &wanted).await.unwrap();
        assert_eq!(read_message(&mut b).await.unwrap(), wanted);
    }
    #[tokio::test]
    async fn rejects_oversized_prefix_without_allocating() {
        let (mut a, mut b) = duplex(64);
        a.write_u32((MAX_MESSAGE_BYTES + 1) as u32).await.unwrap();
        assert!(matches!(
            read_message(&mut b).await,
            Err(ProtocolError::TooLarge)
        ));
    }
}
