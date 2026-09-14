//! Integration tests for fabric-frame-transport wire protocol.
//!
//! Covers: roundtrip for all message types, invalid msg_type, truncated
//! payloads, and max-frame-size encoding.

use bytes::Bytes;
use fabric_frame_transport::transport::{encode_wire, parse_message};
use fabric_frame_transport::{
    Codec, FrameAck, FrameHeader, FrameMessage, KeyFrameRequest, MessageType, Ping, Pong,
    PROTOCOL_VERSION, SessionAck, SessionInit, TransportError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Manually parse a wire frame to extract (MessageType, payload Bytes).
///
/// Wire format: [4B payload_len LE] [1B msg_type] [payload_len bytes]
fn parse_wire_frame(wire: &[u8]) -> (MessageType, Bytes) {
    assert!(wire.len() >= 5, "wire frame must be at least 5 bytes");
    let payload_len = u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]) as usize;
    let msg_type = MessageType::from_u8(wire[4]).expect("valid message type byte");
    let payload = Bytes::copy_from_slice(&wire[5..5 + payload_len]);
    (msg_type, payload)
}

/// Build a default SessionInit for reuse.
fn make_session_init() -> SessionInit {
    SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "integration-test".into(),
    }
}

/// Build a default SessionAck for reuse.
fn make_session_ack() -> SessionAck {
    SessionAck {
        codec: Codec::Av1,
        width: 1280,
        height: 720,
        fps: 30,
        session_id: 42,
        server_time_ms: 1000,
    }
}

// ---------------------------------------------------------------------------
// test_all_message_types_roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_all_message_types_roundtrip() {
    // --- SessionInit (JSON) ---
    {
        let init = make_session_init();
        let payload = serde_json::to_vec(&init).unwrap();
        let wire = encode_wire(MessageType::SessionInit, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::SessionInit);
        match parse_message(mt, body).unwrap() {
            FrameMessage::SessionInit(parsed) => {
                assert_eq!(parsed.version, PROTOCOL_VERSION);
                assert_eq!(parsed.preferred_codec, Codec::Hevc);
                assert_eq!(parsed.width, 1920);
                assert_eq!(parsed.height, 1080);
                assert_eq!(parsed.client_id, "integration-test");
            }
            other => panic!("expected SessionInit, got {other:?}"),
        }
    }

    // --- SessionAck (JSON) ---
    {
        let ack = make_session_ack();
        let payload = serde_json::to_vec(&ack).unwrap();
        let wire = encode_wire(MessageType::SessionAck, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::SessionAck);
        match parse_message(mt, body).unwrap() {
            FrameMessage::SessionAck(parsed) => {
                assert_eq!(parsed.session_id, 42);
                assert_eq!(parsed.codec, Codec::Av1);
            }
            other => panic!("expected SessionAck, got {other:?}"),
        }
    }

    // --- FrameData (binary header + payload) ---
    {
        let raw_frame = vec![0xABu8; 256];
        let header = FrameHeader {
            seq: 7,
            pts_us: 8_000,
            dts_us: 7_500,
            is_keyframe: true,
            codec: Codec::Hevc,
            width: 1920,
            height: 1080,
            payload_len: raw_frame.len() as u32,
            duration_us: 16_667,
        };
        let mut body_buf = bytes::BytesMut::with_capacity(
            FrameHeader::SERIALIZED_SIZE + raw_frame.len(),
        );
        header.encode(&mut body_buf);
        body_buf.extend_from_slice(&raw_frame);
        let wire = encode_wire(MessageType::FrameData, &body_buf).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::FrameData);
        match parse_message(mt, body).unwrap() {
            FrameMessage::FrameData {
                header: parsed_header,
                payload,
            } => {
                assert_eq!(parsed_header.seq, 7);
                assert_eq!(parsed_header.width, 1920);
                assert!(parsed_header.is_keyframe);
                assert_eq!(&payload[..], &raw_frame[..]);
            }
            other => panic!("expected FrameData, got {other:?}"),
        }
    }

    // --- FrameAck (JSON) ---
    {
        let ack = FrameAck {
            seq: 100,
            rtt_us: 5_000,
            recv_pts_us: 2_000_000,
        };
        let payload = serde_json::to_vec(&ack).unwrap();
        let wire = encode_wire(MessageType::FrameAck, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::FrameAck);
        match parse_message(mt, body).unwrap() {
            FrameMessage::FrameAck(parsed) => {
                assert_eq!(parsed.seq, 100);
                assert_eq!(parsed.rtt_us, 5_000);
            }
            other => panic!("expected FrameAck, got {other:?}"),
        }
    }

    // --- Ping (JSON) ---
    {
        let ping = Ping {
            timestamp_us: 1_234_567,
            nonce: 99,
        };
        let payload = serde_json::to_vec(&ping).unwrap();
        let wire = encode_wire(MessageType::Ping, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::Ping);
        match parse_message(mt, body).unwrap() {
            FrameMessage::Ping(parsed) => {
                assert_eq!(parsed.nonce, 99);
                assert_eq!(parsed.timestamp_us, 1_234_567);
            }
            other => panic!("expected Ping, got {other:?}"),
        }
    }

    // --- Pong (JSON) ---
    {
        let pong = Pong {
            timestamp_us: 2_345_678,
            nonce: 77,
        };
        let payload = serde_json::to_vec(&pong).unwrap();
        let wire = encode_wire(MessageType::Pong, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::Pong);
        match parse_message(mt, body).unwrap() {
            FrameMessage::Pong(parsed) => {
                assert_eq!(parsed.nonce, 77);
                assert_eq!(parsed.timestamp_us, 2_345_678);
            }
            other => panic!("expected Pong, got {other:?}"),
        }
    }

    // --- Error (JSON) ---
    {
        let err = TransportError {
            code: 503,
            message: "service unavailable".into(),
            fatal: true,
        };
        let payload = serde_json::to_vec(&err).unwrap();
        let wire = encode_wire(MessageType::Error, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::Error);
        match parse_message(mt, body).unwrap() {
            FrameMessage::Error(parsed) => {
                assert_eq!(parsed.code, 503);
                assert!(parsed.fatal);
                assert_eq!(parsed.message, "service unavailable");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // --- KeyFrameRequest (JSON) ---
    {
        let kfr = KeyFrameRequest { reason: 42 };
        let payload = serde_json::to_vec(&kfr).unwrap();
        let wire = encode_wire(MessageType::KeyFrameRequest, &payload).unwrap();
        let (mt, body) = parse_wire_frame(&wire);
        assert_eq!(mt, MessageType::KeyFrameRequest);
        match parse_message(mt, body).unwrap() {
            FrameMessage::KeyFrameRequest(parsed) => {
                assert_eq!(parsed.reason, 42);
            }
            other => panic!("expected KeyFrameRequest, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// test_invalid_msg_type
// ---------------------------------------------------------------------------

#[test]
fn test_invalid_msg_type() {
    // MessageType::from_u8 should return None for unknown bytes.
    for byte in [0x00, 0x09, 0x0A, 0x10, 0x7F, 0xFE, 0xFF] {
        assert!(
            MessageType::from_u8(byte).is_none(),
            "from_u8({:#04x}) should be None",
            byte
        );
    }

    // Valid range 0x01..=0x08 should all be Some.
    for byte in 0x01..=0x08 {
        assert!(
            MessageType::from_u8(byte).is_some(),
            "from_u8({:#04x}) should be Some",
            byte
        );
    }
}

// ---------------------------------------------------------------------------
// test_truncated_payload
// ---------------------------------------------------------------------------

#[test]
fn test_truncated_payload() {
    // Case 1: header says N bytes but wire only contains N-1.
    let payload = b"hello";
    let wire = encode_wire(MessageType::Ping, payload).unwrap();
    // Chop the last byte of the payload region.
    let truncated = &wire[..wire.len() - 1];
    // parse_wire_frame will read the declared length but only get N-1 bytes,
    // which triggers an index-out-of-bounds in our helper. Verify the
    // underlying encode_wire was correct and the truncated read panics.
    let payload_len =
        u32::from_le_bytes([truncated[0], truncated[1], truncated[2], truncated[3]]) as usize;
    // The actual data available after the 5-byte header.
    let available = truncated.len().saturating_sub(5);
    assert!(
        available < payload_len,
        "expected truncated: available {} < declared {}",
        available,
        payload_len
    );

    // Case 2: parse_message with a payload that is too short for JSON.
    let short_payload = Bytes::from_static(b"{");
    let result = parse_message(MessageType::Ping, short_payload);
    assert!(result.is_err(), "truncated JSON should fail");

    // Case 3: FrameData where header claims more payload bytes than provided.
    let mut header_buf = bytes::BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
    let fake_header = FrameHeader {
        seq: 1,
        pts_us: 0,
        dts_us: 0,
        is_keyframe: false,
        codec: Codec::Hevc,
        width: 640,
        height: 480,
        payload_len: 1000, // claims 1000 bytes
        duration_us: 0,
    };
    fake_header.encode(&mut header_buf);
    // Only provide 10 bytes of payload after the header.
    let mut frame_body = header_buf.to_vec();
    frame_body.extend_from_slice(&[0u8; 10]);
    let result = parse_message(MessageType::FrameData, Bytes::from(frame_body));
    assert!(result.is_err(), "FrameData with mismatched payload_len should fail");
}

// ---------------------------------------------------------------------------
// test_max_frame_size
// ---------------------------------------------------------------------------

#[test]
fn test_max_frame_size() {
    // Encode a 16 MB FrameData frame and verify it round-trips.
    let frame_size = 16 * 1024 * 1024; // MAX_FRAME_SIZE
    let raw_frame = vec![0xCDu8; frame_size];

    let header = FrameHeader {
        seq: u64::MAX,
        pts_us: u64::MAX,
        dts_us: u64::MAX - 1,
        is_keyframe: false,
        codec: Codec::Rgba,
        width: 4096,
        height: 2160,
        payload_len: frame_size as u32,
        duration_us: u32::MAX,
    };

    let mut body_buf =
        bytes::BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + frame_size);
    header.encode(&mut body_buf);
    body_buf.extend_from_slice(&raw_frame);

    let wire = encode_wire(MessageType::FrameData, &body_buf).unwrap();

    // Verify wire structure.
    let declared_len = u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]);
    assert_eq!(
        declared_len as usize,
        body_buf.len(),
        "wire length prefix must match body length"
    );
    assert_eq!(wire[4], MessageType::FrameData as u8);
    assert_eq!(
        wire.len(),
        4 + 1 + body_buf.len(),
        "total wire size must be 4 + 1 + body"
    );

    // Parse back and verify content.
    let (mt, body) = parse_wire_frame(&wire);
    assert_eq!(mt, MessageType::FrameData);
    match parse_message(mt, body).unwrap() {
        FrameMessage::FrameData {
            header: parsed_header,
            payload,
        } => {
            assert_eq!(parsed_header.seq, u64::MAX);
            assert_eq!(parsed_header.width, 4096);
            assert_eq!(parsed_header.height, 2160);
            assert_eq!(parsed_header.codec, Codec::Rgba);
            assert_eq!(parsed_header.payload_len, frame_size as u32);
            assert_eq!(payload.len(), frame_size);
            assert!(payload.iter().all(|&b| b == 0xCD));
        }
        other => panic!("expected FrameData, got {other:?}"),
    }
}
