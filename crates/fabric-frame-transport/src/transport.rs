//! Async TCP transport for the Fabric frame wire protocol.
use anyhow::{bail, Context, Result};
#[cfg(test)]
use bytes::Buf;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    FrameAck, FrameHeader, FrameMessage, KeyFrameRequest, MessageType, Ping, Pong, SessionAck,
    SessionInit, TransportError, MAX_FRAME_SIZE,
};

const LENGTH_PREFIX_SIZE: usize = 4;
const MESSAGE_TYPE_SIZE: usize = 1;
const MAX_WIRE_PAYLOAD_SIZE: usize = MAX_FRAME_SIZE + FrameHeader::SERIALIZED_SIZE;

/// Sends length-prefixed Fabric frame protocol messages over a TCP stream.
#[derive(Debug)]
pub struct FrameSender {
    stream: TcpStream,
}

impl FrameSender {
    /// Creates a sender over `stream`.
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn send_session_init(&mut self, init: &SessionInit) -> Result<()> {
        self.send_json(MessageType::SessionInit, init).await
    }

    pub async fn send_session_ack(&mut self, ack: &SessionAck) -> Result<()> {
        self.send_json(MessageType::SessionAck, ack).await
    }

    pub async fn send_frame_data(&mut self, header: &FrameHeader, payload: &[u8]) -> Result<()> {
        if payload.len() != header.payload_len as usize {
            bail!(
                "frame payload length {} does not match header payload_len {}",
                payload.len(),
                header.payload_len
            );
        }

        let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
        header.encode(&mut body);
        body.extend_from_slice(payload);
        self.send_wire(MessageType::FrameData, &body).await
    }

    pub async fn send_frame_ack(&mut self, ack: &FrameAck) -> Result<()> {
        self.send_json(MessageType::FrameAck, ack).await
    }

    pub async fn send_ping(&mut self, ping: &Ping) -> Result<()> {
        self.send_json(MessageType::Ping, ping).await
    }

    pub async fn send_pong(&mut self, pong: &Pong) -> Result<()> {
        self.send_json(MessageType::Pong, pong).await
    }

    pub async fn send_error(&mut self, error: &TransportError) -> Result<()> {
        self.send_json(MessageType::Error, error).await
    }

    pub async fn send_keyframe_request(&mut self, request: &KeyFrameRequest) -> Result<()> {
        self.send_json(MessageType::KeyFrameRequest, request).await
    }

    async fn send_json<T: serde::Serialize>(
        &mut self,
        message_type: MessageType,
        value: &T,
    ) -> Result<()> {
        let payload = serde_json::to_vec(value).context("serialize transport message as JSON")?;
        self.send_wire(message_type, &payload).await
    }

    async fn send_wire(&mut self, message_type: MessageType, payload: &[u8]) -> Result<()> {
        let wire = encode_wire(message_type, payload)?;
        self.stream
            .write_all(&wire)
            .await
            .context("write framed transport message")?;
        self.stream
            .flush()
            .await
            .context("flush framed transport message")
    }
}

/// Receives length-prefixed Fabric frame protocol messages from a TCP stream.
#[derive(Debug)]
pub struct FrameReceiver {
    stream: TcpStream,
}

impl FrameReceiver {
    /// Creates a receiver over `stream`.
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    /// Reads and parses exactly one framed protocol message.
    pub async fn recv(&mut self) -> Result<FrameMessage> {
        let payload_len = self
            .stream
            .read_u32_le()
            .await
            .context("read transport message length")? as usize;
        if payload_len > MAX_WIRE_PAYLOAD_SIZE {
            bail!("transport message payload exceeds maximum size: {payload_len} bytes");
        }

        let message_type = MessageType::from_u8(
            self.stream
                .read_u8()
                .await
                .context("read transport message type")?,
        )
        .context("unknown transport message type")?;
        let mut payload = vec![0; payload_len];
        self.stream
            .read_exact(&mut payload)
            .await
            .context("read transport message payload")?;
        parse_message(message_type, Bytes::from(payload))
    }
}

fn encode_wire(message_type: MessageType, payload: &[u8]) -> Result<BytesMut> {
    let payload_len =
        u32::try_from(payload.len()).context("transport payload exceeds u32 length")?;
    let mut wire = BytesMut::with_capacity(LENGTH_PREFIX_SIZE + MESSAGE_TYPE_SIZE + payload.len());
    wire.put_u32_le(payload_len);
    wire.put_u8(message_type as u8);
    wire.extend_from_slice(payload);
    Ok(wire)
}

#[cfg(test)]
fn parse_wire(mut wire: Bytes) -> Result<FrameMessage> {
    if wire.remaining() < LENGTH_PREFIX_SIZE + MESSAGE_TYPE_SIZE {
        bail!("transport frame too short for length prefix and message type");
    }
    let payload_len = wire.get_u32_le() as usize;
    let message_type =
        MessageType::from_u8(wire.get_u8()).context("unknown transport message type")?;
    if wire.remaining() != payload_len {
        bail!(
            "transport payload length mismatch: declared {payload_len}, actual {}",
            wire.remaining()
        );
    }
    parse_message(message_type, wire)
}

fn parse_message(message_type: MessageType, mut payload: Bytes) -> Result<FrameMessage> {
    match message_type {
        MessageType::SessionInit => Ok(FrameMessage::SessionInit(parse_json(&payload)?)),
        MessageType::SessionAck => Ok(FrameMessage::SessionAck(parse_json(&payload)?)),
        MessageType::FrameData => {
            let header = FrameHeader::decode(&mut payload)?;
            if payload.len() != header.payload_len as usize {
                bail!(
                    "frame payload length {} does not match header payload_len {}",
                    payload.len(),
                    header.payload_len
                );
            }
            Ok(FrameMessage::FrameData { header, payload })
        }
        MessageType::FrameAck => Ok(FrameMessage::FrameAck(parse_json(&payload)?)),
        MessageType::Ping => Ok(FrameMessage::Ping(parse_json(&payload)?)),
        MessageType::Pong => Ok(FrameMessage::Pong(parse_json(&payload)?)),
        MessageType::Error => Ok(FrameMessage::Error(parse_json(&payload)?)),
        MessageType::KeyFrameRequest => Ok(FrameMessage::KeyFrameRequest(parse_json(&payload)?)),
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T> {
    serde_json::from_slice(payload).context("deserialize transport message JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Codec, PROTOCOL_VERSION};

    fn init() -> SessionInit {
        SessionInit {
            version: PROTOCOL_VERSION,
            preferred_codec: Codec::Av1,
            width: 1280,
            height: 720,
            target_fps: 60,
            max_latency_ms: 33,
            client_id: "test-client".into(),
        }
    }

    fn json<T: serde::Serialize>(message_type: MessageType, value: &T) -> FrameMessage {
        let payload = serde_json::to_vec(value).unwrap();
        parse_wire(encode_wire(message_type, &payload).unwrap().freeze()).unwrap()
    }

    #[test]
    fn length_prefix_is_little_endian_payload_length() {
        let wire = encode_wire(MessageType::Ping, b"abc").unwrap();
        assert_eq!(&wire[..4], &[3, 0, 0, 0]);
        assert_eq!(wire[4], MessageType::Ping as u8);
    }

    #[test]
    fn session_init_frame_roundtrip() {
        match json(MessageType::SessionInit, &init()) {
            FrameMessage::SessionInit(value) => assert_eq!(value.client_id, "test-client"),
            _ => panic!("expected SessionInit"),
        }
    }

    #[test]
    fn session_ack_frame_roundtrip() {
        let ack = SessionAck {
            codec: Codec::Rgba,
            width: 10,
            height: 20,
            fps: 30,
            session_id: 40,
            server_time_ms: 50,
        };
        match json(MessageType::SessionAck, &ack) {
            FrameMessage::SessionAck(value) => assert_eq!(value.session_id, 40),
            _ => panic!("expected SessionAck"),
        }
    }

    #[test]
    fn frame_data_binary_roundtrip() {
        let raw = b"encoded-frame";
        let frame_header = FrameHeader {
            seq: 7,
            pts_us: 8,
            dts_us: 9,
            is_keyframe: true,
            codec: Codec::Hevc,
            width: 1920,
            height: 1080,
            payload_len: raw.len() as u32,
            duration_us: 16_667,
        };
        let mut payload = BytesMut::new();
        frame_header.encode(&mut payload);
        payload.extend_from_slice(raw);
        match parse_wire(
            encode_wire(MessageType::FrameData, &payload)
                .unwrap()
                .freeze(),
        )
        .unwrap()
        {
            FrameMessage::FrameData { header, payload } => {
                assert_eq!(header.seq, 7);
                assert_eq!(&payload[..], raw);
            }
            _ => panic!("expected FrameData"),
        }
    }

    #[test]
    fn frame_ack_frame_roundtrip() {
        let ack = FrameAck {
            seq: 1,
            rtt_us: 2,
            recv_pts_us: 3,
        };
        match json(MessageType::FrameAck, &ack) {
            FrameMessage::FrameAck(value) => assert_eq!(value.rtt_us, 2),
            _ => panic!("expected FrameAck"),
        }
    }

    #[test]
    fn ping_and_pong_frames_roundtrip() {
        assert!(matches!(
            json(
                MessageType::Ping,
                &Ping {
                    timestamp_us: 1,
                    nonce: 2
                }
            ),
            FrameMessage::Ping(_)
        ));
        assert!(matches!(
            json(
                MessageType::Pong,
                &Pong {
                    timestamp_us: 3,
                    nonce: 4
                }
            ),
            FrameMessage::Pong(_)
        ));
    }

    #[test]
    fn error_frame_roundtrip() {
        let error = TransportError {
            code: 9,
            message: "bad frame".into(),
            fatal: true,
        };
        match json(MessageType::Error, &error) {
            FrameMessage::Error(value) => assert!(value.fatal),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn keyframe_request_frame_roundtrip() {
        match json(
            MessageType::KeyFrameRequest,
            &KeyFrameRequest { reason: 12 },
        ) {
            FrameMessage::KeyFrameRequest(value) => assert_eq!(value.reason, 12),
            _ => panic!("expected KeyFrameRequest"),
        }
    }

    #[test]
    fn short_read_returns_error() {
        assert!(parse_wire(Bytes::from_static(&[1, 0, 0])).is_err());
        assert!(parse_wire(Bytes::from_static(&[
            5,
            0,
            0,
            0,
            MessageType::Ping as u8,
            b'{',
            b'}'
        ]))
        .is_err());
    }
}
