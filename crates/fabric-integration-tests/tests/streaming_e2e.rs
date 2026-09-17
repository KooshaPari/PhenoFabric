//! E2E streaming pipeline tests: wire protocol, frame encoding, WebRTC compatibility.
//!
//! Exercises the transport layer end-to-end:
//! - Wire protocol roundtrip for RGBA frames
//! - Frame encoding fidelity across sizes and codecs
//! - WebRTC data channel wire format compatibility
//! - Wire keepalive roundtrip
//! - Concurrent frame encoding stress

use bytes::BytesMut;
use fabric_frame_transport::transport::{encode_wire, parse_message};
use fabric_frame_transport::{
    Codec, FrameAck, FrameHeader, FrameMessage, KeyFrameRequest, MessageType, Ping, Pong,
    SessionAck, SessionInit, TransportError, PROTOCOL_VERSION,
};

// Import shared helpers from the crate lib.
use fabric_integration_tests::{decode_frame_wire, encode_frame_wire, make_rgba_payload};

// ===========================================================================
// Test 1: Wire protocol roundtrip — RGBA pixel data
// ===========================================================================

#[test]
fn wire_protocol_rgba_frame_roundtrip() {
    let width = 640u32;
    let height = 480u32;
    let pixel_data = make_rgba_payload(width, height);

    let header = FrameHeader {
        seq: 1,
        pts_us: 33_333,
        dts_us: 33_000,
        is_keyframe: true,
        codec: Codec::Rgba,
        width,
        height,
        payload_len: pixel_data.len() as u32,
        duration_us: 16_667,
    };

    let wire = encode_frame_wire(&header, &pixel_data);
    assert!(wire.len() > 5);

    // Verify wire structure: [4 bytes len LE] [1 byte type] [body]
    let declared_len = u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]);
    assert_eq!(wire[4], MessageType::FrameData as u8);
    assert_eq!(
        declared_len as usize,
        wire.len() - 5,
        "wire = [4 len] [1 type] [payload.len() bytes]"
    );

    let (decoded_header, decoded_payload) = decode_frame_wire(&wire);

    assert_eq!(decoded_header.seq, 1);
    assert_eq!(decoded_header.pts_us, 33_333);
    assert_eq!(decoded_header.dts_us, 33_000);
    assert!(decoded_header.is_keyframe);
    assert_eq!(decoded_header.codec, Codec::Rgba);
    assert_eq!(decoded_header.width, 640);
    assert_eq!(decoded_header.height, 480);
    assert_eq!(decoded_header.payload_len, pixel_data.len() as u32);
    assert_eq!(decoded_header.duration_us, 16_667);
    assert_eq!(&decoded_payload[..], &pixel_data[..]);
}

// ===========================================================================
// Test 2: Frame encoding fidelity — various sizes
// ===========================================================================

#[test]
fn frame_encoding_fidelity_various_sizes() {
    let cases = vec![
        ("1x1", 1u32, 1u32),
        ("100x100", 100u32, 100u32),
        ("1920x1080_sim", 1920u32, 1080u32),
    ];

    for (label, width, height) in cases {
        let pixel_data = make_rgba_payload(width, height);
        let header = FrameHeader {
            seq: 42,
            pts_us: 1_000_000,
            dts_us: 990_000,
            is_keyframe: false,
            codec: Codec::Rgba,
            width,
            height,
            payload_len: pixel_data.len() as u32,
            duration_us: 33_333,
        };

        let wire = encode_frame_wire(&header, &pixel_data);
        let (decoded_header, decoded_payload) = decode_frame_wire(&wire);

        assert_eq!(decoded_header.width, width, "{label}: width mismatch");
        assert_eq!(decoded_header.height, height, "{label}: height mismatch");
        assert_eq!(
            decoded_header.payload_len,
            pixel_data.len() as u32,
            "{label}: payload_len mismatch"
        );
        assert_eq!(
            &decoded_payload[..],
            &pixel_data[..],
            "{label}: pixel data mismatch"
        );
    }
}

#[test]
fn frame_header_field_fidelity_across_codecs() {
    let codecs = [Codec::Hevc, Codec::Av1, Codec::Nv12, Codec::Rgba];
    let keyframes = [true, false];

    for codec in &codecs {
        for &is_kf in &keyframes {
            let header = FrameHeader {
                seq: 999,
                pts_us: 12_345_678,
                dts_us: 12_340_000,
                is_keyframe: is_kf,
                codec: *codec,
                width: 3840,
                height: 2160,
                payload_len: 1024,
                duration_us: 16_667,
            };

            let mut buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
            header.encode(&mut buf);
            let mut bytes = buf.freeze();
            let decoded = FrameHeader::decode(&mut bytes).unwrap();

            assert_eq!(decoded.seq, header.seq, "seq for {codec:?}");
            assert_eq!(decoded.pts_us, header.pts_us, "pts for {codec:?}");
            assert_eq!(
                decoded.is_keyframe, header.is_keyframe,
                "keyframe for {codec:?}"
            );
            assert_eq!(decoded.codec, header.codec, "codec");
            assert_eq!(decoded.width, header.width, "width");
            assert_eq!(decoded.payload_len, header.payload_len, "payload_len");
        }
    }
}

// ===========================================================================
// Test 3: Wire protocol keepalive roundtrip
// ===========================================================================

#[test]
fn wire_protocol_keepalive_roundtrip() {
    let ping = Ping {
        timestamp_us: 1_000_000_000,
        nonce: 42,
    };
    let wire = encode_wire(MessageType::Ping, &serde_json::to_vec(&ping).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    match parse_message(MessageType::Ping, payload).unwrap() {
        FrameMessage::Ping(parsed) => assert_eq!(parsed.nonce, 42),
        _ => panic!("expected Ping"),
    }

    let pong = Pong {
        timestamp_us: 1_000_010_000,
        nonce: 42,
    };
    let wire = encode_wire(MessageType::Pong, &serde_json::to_vec(&pong).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    match parse_message(MessageType::Pong, payload).unwrap() {
        FrameMessage::Pong(parsed) => assert_eq!(parsed.nonce, 42),
        _ => panic!("expected Pong"),
    }
}

// ===========================================================================
// Test 4: WebRTC data channel wire format compatibility
// ===========================================================================

/// Verify all JSON message types survive encode_wire / parse_message roundtrip,
/// matching the pattern used by fabric_web::webrtc_channel.
#[test]
fn webrtc_wire_format_json_message_compatibility() {
    // SessionInit
    let init = SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "webrtc-client".to_string(),
    };
    let wire = encode_wire(
        MessageType::SessionInit,
        &serde_json::to_vec(&init).unwrap(),
    )
    .unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::SessionInit(p) = parse_message(MessageType::SessionInit, payload).unwrap()
    {
        assert_eq!(p.client_id, "webrtc-client");
        assert_eq!(p.width, 1920);
    } else {
        panic!("expected SessionInit");
    }

    // SessionAck
    let ack = SessionAck {
        codec: Codec::Rgba,
        width: 1920,
        height: 1080,
        fps: 60,
        session_id: 42,
        server_time_ms: 1_000_000,
    };
    let wire = encode_wire(MessageType::SessionAck, &serde_json::to_vec(&ack).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::SessionAck(p) = parse_message(MessageType::SessionAck, payload).unwrap() {
        assert_eq!(p.session_id, 42);
    } else {
        panic!("expected SessionAck");
    }

    // FrameAck
    let fa = FrameAck {
        seq: 100,
        rtt_us: 5000,
        recv_pts_us: 3_000_000,
    };
    let wire = encode_wire(MessageType::FrameAck, &serde_json::to_vec(&fa).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::FrameAck(p) = parse_message(MessageType::FrameAck, payload).unwrap() {
        assert_eq!(p.seq, 100);
    } else {
        panic!("expected FrameAck");
    }

    // Ping
    let ping = Ping {
        timestamp_us: 12_345,
        nonce: 77,
    };
    let wire = encode_wire(MessageType::Ping, &serde_json::to_vec(&ping).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::Ping(p) = parse_message(MessageType::Ping, payload).unwrap() {
        assert_eq!(p.nonce, 77);
    } else {
        panic!("expected Ping");
    }

    // Pong
    let pong = Pong {
        timestamp_us: 12_345,
        nonce: 77,
    };
    let wire = encode_wire(MessageType::Pong, &serde_json::to_vec(&pong).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::Pong(p) = parse_message(MessageType::Pong, payload).unwrap() {
        assert_eq!(p.nonce, 77);
    } else {
        panic!("expected Pong");
    }

    // Error
    let err = TransportError {
        code: 503,
        message: "unavailable".into(),
        fatal: false,
    };
    let wire = encode_wire(MessageType::Error, &serde_json::to_vec(&err).unwrap()).unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::Error(p) = parse_message(MessageType::Error, payload).unwrap() {
        assert_eq!(p.code, 503);
    } else {
        panic!("expected Error");
    }

    // KeyFrameRequest
    let kf = KeyFrameRequest { reason: 3 };
    let wire = encode_wire(
        MessageType::KeyFrameRequest,
        &serde_json::to_vec(&kf).unwrap(),
    )
    .unwrap();
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    if let FrameMessage::KeyFrameRequest(p) =
        parse_message(MessageType::KeyFrameRequest, payload).unwrap()
    {
        assert_eq!(p.reason, 3);
    } else {
        panic!("expected KeyFrameRequest");
    }
}

/// Verify FrameData binary wire format matches the WebRtcChannel pattern:
/// encode header + payload into body, wrap in wire, parse back.
#[test]
fn webrtc_wire_format_frame_data_binary_compatibility() {
    let pixel_data = make_rgba_payload(320, 240);
    let header = FrameHeader {
        seq: 7,
        pts_us: 8,
        dts_us: 9,
        is_keyframe: true,
        codec: Codec::Hevc,
        width: 320,
        height: 240,
        payload_len: pixel_data.len() as u32,
        duration_us: 33_333,
    };

    // Simulate WebRtcChannel::send_frame_data
    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + pixel_data.len());
    header.encode(&mut body);
    body.extend_from_slice(&pixel_data);
    let wire = encode_wire(MessageType::FrameData, &body).unwrap();

    // Simulate WebRtcChannel::on_message parsing
    let total_len = u32::from_le_bytes([wire[0], wire[1], wire[2], wire[3]]);
    // wire = [4 len] [1 type] [payload.len() bytes], so payload_len = wire.len() - 5
    assert_eq!(total_len as usize, wire.len() - 5);
    let msg_type = MessageType::from_u8(wire[4]).unwrap();
    assert_eq!(msg_type, MessageType::FrameData);

    // payload = bytes after the 4-byte length prefix and 1-byte message type
    let payload = bytes::Bytes::copy_from_slice(&wire[5..]);
    match parse_message(msg_type, payload).unwrap() {
        FrameMessage::FrameData {
            header: h,
            payload: p,
        } => {
            assert_eq!(h.seq, 7);
            assert_eq!(h.width, 320);
            assert!(h.is_keyframe);
            assert_eq!(h.codec, Codec::Hevc);
            assert_eq!(&p[..], &pixel_data[..]);
        }
        _ => panic!("expected FrameData"),
    }
}

// ===========================================================================
// Test 5: Concurrent frame encoding stress
// ===========================================================================

#[tokio::test]
async fn concurrent_frame_encode_decode_stress() {
    let mut handles = Vec::new();
    for task_id in 0..10 {
        handles.push(tokio::spawn(async move {
            let pixel_data = make_rgba_payload(32, 32);
            for seq in 0..50 {
                let header = FrameHeader {
                    seq: (task_id * 1000 + seq),
                    pts_us: seq * 33_333,
                    dts_us: seq * 33_000,
                    is_keyframe: seq == 0,
                    codec: Codec::Rgba,
                    width: 32,
                    height: 32,
                    payload_len: pixel_data.len() as u32,
                    duration_us: 16_667,
                };
                let wire = encode_frame_wire(&header, &pixel_data);
                let (dh, dp) = decode_frame_wire(&wire);
                assert_eq!(dh.width, 32);
                assert_eq!(&dp[..], &pixel_data[..]);
            }
        }));
    }
    for h in handles {
        h.await.expect("task should not panic");
    }
}
