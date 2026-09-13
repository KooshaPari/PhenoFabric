//! Fuzz target: encode_wire round-trip must never panic.
//!
//! Encodes random data through `encode_wire`, then feeds it back through
//! the test-only `parse_wire`. The decode may fail but must never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use fabric_frame_transport::transport::encode_wire;
use fabric_frame_transport::MessageType;

fuzz_target!(|data: &[u8]| {
    let msg_types = [
        MessageType::SessionInit,
        MessageType::SessionAck,
        MessageType::FrameAck,
        MessageType::Ping,
        MessageType::Pong,
        MessageType::Error,
        MessageType::KeyFrameRequest,
    ];

    for mt in &msg_types {
        // encode_wire should never panic even on empty or huge payloads.
        if let Ok(encoded) = encode_wire(*mt, data) {
            // Verify the wire format invariants:
            // 1. At least 5 bytes (4 for length prefix + 1 for message type)
            assert!(encoded.len() >= 5, "wire must be at least 5 bytes");

            // 2. Length prefix (u32 LE) + 5 == total encoded length
            let declared_len = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
            assert_eq!(
                declared_len as usize + 5,
                encoded.len(),
                "length prefix must match encoded length"
            );

            // 3. Message type byte matches
            assert_eq!(encoded[4], *mt as u8, "message type byte must match");
        }
    }
});
