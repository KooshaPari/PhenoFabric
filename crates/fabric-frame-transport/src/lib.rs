//! fabric-frame-transport: Frame transport protocol for Parsec-style surface streaming.
//!
//! Provides the wire protocol for encoding, framing, and transmitting desktop
//! frames (HEVC/AV1/NV12/RGBA) over TCP between the daemon and GUI surfaces.
//!
//! # Wire Protocol
//!
//! All messages use length-prefixed binary framing:
//! ```text
//! [4 bytes: total_len (u32 LE)] [1 byte: msg_type] [payload: total_len bytes]
//! ```
//!
//! Message types:
//! - `0x01` SessionInit — client → server, negotiate codec/format
//! - `0x02` SessionAck — server → client, confirm session
//! - `0x03` FrameData  — server → client, encoded frame
//! - `0x04` FrameAck   — client → server, acknowledgment
//! - `0x05` Ping       — bidirectional, keepalive
//! - `0x06` Pong       — bidirectional, response to ping
//! - `0x07` Error      — bidirectional, error notification
//! - `0x08` KeyFrame   — server → client, request IDR/keyframe

pub mod stats;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};

// === Constants ===

/// Protocol version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum frame payload size (16 MB).
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Default frame rate target.
pub const DEFAULT_TARGET_FPS: u32 = 60;

/// Default maximum latency target.
pub const DEFAULT_MAX_LATENCY_MS: u32 = 33;

// === Message Types ===

/// Protocol message type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    SessionInit = 0x01,
    SessionAck = 0x02,
    FrameData = 0x03,
    FrameAck = 0x04,
    Ping = 0x05,
    Pong = 0x06,
    Error = 0x07,
    KeyFrameRequest = 0x08,
}

impl MessageType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::SessionInit),
            0x02 => Some(Self::SessionAck),
            0x03 => Some(Self::FrameData),
            0x04 => Some(Self::FrameAck),
            0x05 => Some(Self::Ping),
            0x06 => Some(Self::Pong),
            0x07 => Some(Self::Error),
            0x08 => Some(Self::KeyFrameRequest),
            _ => None,
        }
    }
}

// === Codec Types ===

/// Video codec used for frame encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    Hevc,
    Av1,
    Nv12,
    Rgba,
}

impl Codec {
    pub fn name(&self) -> &'static str {
        match self {
            Codec::Hevc => "HEVC",
            Codec::Av1 => "AV1",
            Codec::Nv12 => "NV12",
            Codec::Rgba => "RGBA",
        }
    }

    pub fn is_encoded(&self) -> bool {
        matches!(self, Codec::Hevc | Codec::Av1)
    }
}

// === Protocol Messages ===

/// Session initialization (client → server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInit {
    pub version: u32,
    pub preferred_codec: Codec,
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
    pub max_latency_ms: u32,
    pub client_id: String,
}

/// Session acknowledgment (server → client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAck {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub session_id: u64,
    pub server_time_ms: u64,
}

/// Encoded frame header (binary, not JSON).
#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub seq: u64,
    pub pts_us: u64,
    pub dts_us: u64,
    pub is_keyframe: bool,
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub payload_len: u32,
    pub duration_us: u32,
}

impl FrameHeader {
    /// Byte size of the serialized header (fixed).
    pub const SERIALIZED_SIZE: usize = 42;

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u64_le(self.seq);
        buf.put_u64_le(self.pts_us);
        buf.put_u64_le(self.dts_us);
        buf.put_u8(if self.is_keyframe { 1 } else { 0 });
        buf.put_u8(self.codec as u8);
        buf.put_u32_le(self.width);
        buf.put_u32_le(self.height);
        buf.put_u32_le(self.payload_len);
        buf.put_u32_le(self.duration_us);
    }

    pub fn decode(buf: &mut Bytes) -> anyhow::Result<Self> {
        if buf.remaining() < Self::SERIALIZED_SIZE {
            anyhow::bail!("Frame header too short: {} bytes", buf.remaining());
        }
        let seq = buf.get_u64_le();
        let pts_us = buf.get_u64_le();
        let dts_us = buf.get_u64_le();
        let is_keyframe = buf.get_u8() != 0;
        let codec_byte = buf.get_u8();
        let codec = match codec_byte {
            0 => Codec::Hevc,
            1 => Codec::Av1,
            2 => Codec::Nv12,
            3 => Codec::Rgba,
            _ => Codec::Hevc,
        };
        let width = buf.get_u32_le();
        let height = buf.get_u32_le();
        let payload_len = buf.get_u32_le();
        let duration_us = buf.get_u32_le();
        Ok(Self {
            seq,
            pts_us,
            dts_us,
            is_keyframe,
            codec,
            width,
            height,
            payload_len,
            duration_us,
        })
    }
}

/// Frame acknowledgment (client → server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameAck {
    pub seq: u64,
    pub rtt_us: u64,
    pub recv_pts_us: u64,
}

/// Ping keepalive message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    pub timestamp_us: u64,
    pub nonce: u32,
}

/// Pong response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    pub timestamp_us: u64,
    pub nonce: u32,
}

/// Error notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportError {
    pub code: u32,
    pub message: String,
    pub fatal: bool,
}

/// Keyframe request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFrameRequest {
    pub reason: u32,
}

/// All possible protocol messages.
#[derive(Debug, Clone)]
pub enum FrameMessage {
    SessionInit(SessionInit),
    SessionAck(SessionAck),
    FrameData {
        header: FrameHeader,
        payload: Bytes,
    },
    FrameAck(FrameAck),
    Ping(Ping),
    Pong(Pong),
    Error(TransportError),
    KeyFrameRequest(KeyFrameRequest),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_type_roundtrip() {
        for v in 0x01..=0x08 {
            assert!(
                MessageType::from_u8(v).is_some(),
                "MessageType::from_u8({:#x}) should exist",
                v
            );
        }
        assert!(MessageType::from_u8(0x00).is_none());
        assert!(MessageType::from_u8(0xFF).is_none());
    }

    #[test]
    fn codec_names() {
        assert_eq!(Codec::Hevc.name(), "HEVC");
        assert_eq!(Codec::Av1.name(), "AV1");
        assert_eq!(Codec::Nv12.name(), "NV12");
        assert_eq!(Codec::Rgba.name(), "RGBA");
    }

    #[test]
    fn codec_is_encoded() {
        assert!(Codec::Hevc.is_encoded());
        assert!(Codec::Av1.is_encoded());
        assert!(!Codec::Nv12.is_encoded());
        assert!(!Codec::Rgba.is_encoded());
    }

    #[test]
    fn frame_header_encode_decode_roundtrip() {
        let header = FrameHeader {
            seq: 42,
            pts_us: 1_000_000,
            dts_us: 990_000,
            is_keyframe: true,
            codec: Codec::Hevc,
            width: 1920,
            height: 1080,
            payload_len: 50_000,
            duration_us: 16_667,
        };

        let mut buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
        header.encode(&mut buf);
        assert_eq!(buf.len(), FrameHeader::SERIALIZED_SIZE);

        let mut bytes = buf.freeze();
        let decoded = FrameHeader::decode(&mut bytes).unwrap();

        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.pts_us, 1_000_000);
        assert!(decoded.is_keyframe);
        assert_eq!(decoded.codec, Codec::Hevc);
        assert_eq!(decoded.width, 1920);
        assert_eq!(decoded.payload_len, 50_000);
    }

    #[test]
    fn frame_header_short_buffer() {
        let mut buf = Bytes::from(vec![0u8; 10]);
        assert!(FrameHeader::decode(&mut buf).is_err());
    }

    #[test]
    fn session_init_serialization() {
        let init = SessionInit {
            version: PROTOCOL_VERSION,
            preferred_codec: Codec::Hevc,
            width: 1920,
            height: 1080,
            target_fps: 60,
            max_latency_ms: 33,
            client_id: "fabric-gui".to_string(),
        };
        let json = serde_json::to_string(&init).unwrap();
        let parsed: SessionInit = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, PROTOCOL_VERSION);
        assert_eq!(parsed.width, 1920);
    }

    #[test]
    fn frame_ack_serialization() {
        let ack = FrameAck {
            seq: 100,
            rtt_us: 5000,
            recv_pts_us: 2_000_000,
        };
        let json = serde_json::to_string(&ack).unwrap();
        let parsed: FrameAck = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, 100);
    }

    #[test]
    fn ping_pong_roundtrip() {
        let ping = Ping { timestamp_us: 12345, nonce: 42 };
        let json = serde_json::to_string(&ping).unwrap();
        let parsed: Ping = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nonce, 42);

        let pong = Pong { timestamp_us: 12345, nonce: 42 };
        let json = serde_json::to_string(&pong).unwrap();
        let parsed: Pong = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nonce, 42);
    }

    #[test]
    fn transport_error_serialization() {
        let err = TransportError {
            code: 400,
            message: "codec not supported".to_string(),
            fatal: false,
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: TransportError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, 400);
        assert!(!parsed.fatal);
    }
}
