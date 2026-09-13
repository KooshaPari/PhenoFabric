//! Fuzz target: parse_message must never panic on arbitrary input.
//!
//! We feed random bytes as payload + random message type. The function should
//! return `Err(...)` on malformed input, never panic.

#![no_main]

use bytes::Bytes;
use libfuzzer_sys::fuzz_target;
use fabric_frame_transport::transport::parse_message;
use fabric_frame_transport::MessageType;

fuzz_target!(|data: &[u8]| {
    // Pick a message type from the valid range + one invalid value.
    let msg_types = [
        MessageType::SessionInit,
        MessageType::SessionAck,
        MessageType::FrameData,
        MessageType::FrameAck,
        MessageType::Ping,
        MessageType::Pong,
        MessageType::Error,
        MessageType::KeyFrameRequest,
    ];

    for mt in &msg_types {
        let payload = Bytes::copy_from_slice(data);
        // Must never panic — Result::Err is acceptable.
        let _ = parse_message(*mt, payload.clone());
    }

    // Also try with an invalid message type byte by using a high value
    // that doesn't map to any variant. We can't call parse_message with an
    // invalid MessageType since it's an enum, but we can test the JSON
    // parsing path by feeding garbage into valid message types.
});
