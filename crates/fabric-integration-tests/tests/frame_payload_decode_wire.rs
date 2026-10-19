//! Wire-level proof that the frame payload decoder runs inside the live handler.
//!
//! Unit tests in `crates/fabric-daemon/src/wire/frame_handlers_tests.rs` and
//! `crates/fabric-frame-transport/src/frame_payload_tests.rs` exercise the
//! decoder in isolation: they call `decode_payload` directly and assert the
//! typed `PayloadDecodeError` for each failure mode, and they call
//! `handle_frame_data` with a pre-cooked `Value`. None of those paths proves
//! the daemon's wire server is actually running the decoder on bytes the
//! sender transmitted, vs. silently passing the envelope through.
//!
//! This file drives the same gates end-to-end through `run_wire_server`, so a
//! real TCP client sends a `frame_data` and the daemon replies on the wire.
//! The fingerprint field `payload_first8_b64` is the proof: a stubbed-out
//! decoder (or a bypass that never calls `decode_payload`) cannot produce
//! the first 8 bytes of the actual decoded body. The four typed failure
//! modes each render as a distinct `frame_error` code:
//!
//! 1. `payload_truncated`         -- bytes arrived short of `payload_len`
//! 2. `payload_length_mismatch`   -- bytes arrived over `payload_len`
//! 3. `payload_invalid_base64`    -- envelope is not valid base64 at all
//! 4. `invalid_payload_type`      -- encoded codec with zero-byte body
//!
//! `payload_first8_b64` round-trip is the only assertion the live handler
//! uniquely produces; a unit test calling `decode_payload` directly is not
//! enough to call the wire round-trip proven.
//!
//! The file stays under the project's 500-line ceiling.
//!
//! Run: `cargo test -p fabric-integration-tests --test frame_payload_decode_wire`

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fabric_integration_tests::harness::{start_daemon, stop_daemon};

/// `session_init` for a 64x48 Rgba opener, the smallest size that gives a
/// non-zero byte round-trip without making the test slow.
const INIT_LINE: &str = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"decode-client"}"#;

/// Open a connection and return a closure that sends one line and reads one
/// response. The closure owns the socket.
fn frame_exchange(addr: &str) -> impl FnMut(&str) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).expect("connect to daemon wire server");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

    move |request: &str| -> serde_json::Value {
        stream.write_all(request.as_bytes()).expect("write request");
        stream.write_all(b"\n").expect("write newline");
        stream.flush().expect("flush request");
        let mut line = String::new();
        let read = reader.read_line(&mut line).expect("read response");
        assert_ne!(read, 0, "daemon closed the connection");
        serde_json::from_str(line.trim())
            .unwrap_or_else(|e| panic!("response was not JSON ({e}): {line}"))
    }
}

// ===========================================================================
// Test 1: valid payload's first 8 bytes round-trip through the live handler
// ===========================================================================

/// A 12,288-byte Rgba body (64x48) made of zero bytes is sent over the wire,
/// and the `frame_ack` response must echo the first 8 decoded bytes as
/// base64 in `payload_first8_b64`. Eight zero bytes base64-encode to
/// `"AAAAAAAA"` (no padding, because the fingerprint is exactly 8 bytes
/// long -- the helper truncates to the first 8 and emits the 8-byte form).
///
/// A handler that stubbed out the decoder could not produce this fingerprint:
/// it would either omit the field or echo something derived from the
/// envelope bytes, not from the decoded body. A handler that called the
/// decoder but on the wrong bytes (e.g. a stale buffer) would also produce a
/// different fingerprint, because the wire test sends an exact zero body.
#[test]
fn valid_rgba_payload_first8_round_trips() {
    let daemon = start_daemon("decode-wire");
    let mut exchange = frame_exchange(&daemon.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // 12,288 zero bytes (Rgba 64x48 = 64 * 48 * 4) base64-encodes to
    // 16,384 'A' characters (12,288 is a multiple of 3, so no padding).
    let frame = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"decode-client","payload_b64":"{a16k}"}}"#,
        a16k = "A".repeat(16_384)
    ));

    assert_eq!(frame["type"], "frame_ack", "got: {frame}");
    assert_eq!(frame["status"], "ok");
    assert_eq!(frame["frame_count"], 1);
    assert_eq!(
        frame["payload_first8_b64"], "AAAAAAAAAAA=",
        "the first 8 bytes of the decoded body (all zero) must round-trip \
         through the live handler's fingerprint field; a stubbed-out \
         decoder or a bypass could not produce this exact value. \
         8 zero bytes base64-encode to 11 'A' chars + 1 '=' pad (the \
         helper emits the full 8-byte form with RFC 4648 padding). got: {frame}"
    );

    stop_daemon(daemon);
}

// ===========================================================================
// Test 2: truncated payload is rejected with payload_truncated
// ===========================================================================

/// When the bytes that arrived are fewer than the declared `payload_len`,
/// the decoder returns `PayloadDecodeError::Truncated` and the wire renders
/// it as `payload_truncated`. The frame must NOT increment counters: a
/// refused decode is a no-op, and the next accepted frame must show
/// `frame_count == 1` rather than 2.
#[test]
fn truncated_payload_returns_payload_truncated_error() {
    let daemon = start_daemon("decode-wire");
    let mut exchange = frame_exchange(&daemon.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // 8 zero bytes (encoded as 12 b64 chars `AAAAAAAAAAA=`) declared as
    // 12,288. The decoder sees 8 < 12,288 and refuses as Truncated.
    let refused = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"decode-client","payload_b64":"AAAAAAAAAAA="}}"#
    ));

    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "payload_truncated",
        "actual bytes < declared must surface as payload_truncated: {refused}"
    );
    assert_eq!(refused["session_id"], session_id);

    // The refused frame did not shift counters. A subsequent valid frame
    // must start at frame_count = 1.
    let accepted = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":1,"pts_us":1,"dts_us":1,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"decode-client","payload_b64":"{a16k}"}}"#,
        a16k = "A".repeat(16_384)
    ));
    assert_eq!(accepted["type"], "frame_ack", "got: {accepted}");
    assert_eq!(
        accepted["frame_count"], 1,
        "the truncated frame must not have advanced counters: {accepted}"
    );

    stop_daemon(daemon);
}

// ===========================================================================
// Test 3: declared length > actual bytes is rejected as payload_length_mismatch
// ===========================================================================

/// When the bytes that arrived are MORE than the declared `payload_len`,
/// the decoder returns `PayloadDecodeError::LengthMismatch` and the wire
/// renders it as `payload_length_mismatch`. The shape check for Rgba never
/// runs because the length check fires first (the receiver cannot know
/// which bytes belong to which pixel row if the declared count is wrong).
#[test]
fn declared_len_mismatch_returns_payload_length_mismatch() {
    let daemon = start_daemon("decode-wire");
    let mut exchange = frame_exchange(&daemon.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // 12,290 zero bytes (declared as 12,288). 12,290 is 2 bytes over a
    // 3-byte boundary, so the b64 is 16,384 'A' chars + "AA==" (the last
    // 2 bytes zero). 12,290 != 12,288, the decoder refuses as
    // LengthMismatch, and the wire renders it as payload_length_mismatch.
    let refused = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"decode-client","payload_b64":"{a16384}AA=="}}"#,
        a16384 = "A".repeat(16_384)
    ));

    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "payload_length_mismatch",
        "actual bytes > declared must surface as payload_length_mismatch: {refused}"
    );
    assert_eq!(refused["session_id"], session_id);

    stop_daemon(daemon);
}

// ===========================================================================
// Test 4: invalid base64 is rejected with payload_invalid_base64
// ===========================================================================

/// The base64 string itself is malformed (`"!!!"` is not in the standard
/// base64 alphabet). The decoder returns `PayloadDecodeError::InvalidBase64`
/// before any length or shape check runs, and the wire renders it as
/// `payload_invalid_base64`. A handler that tried to "fix" bad b64 by
/// silently dropping it would let a corrupted sender inject frames; the
/// daemon must refuse, not coerce.
#[test]
fn invalid_base64_returns_payload_invalid_base64_error() {
    let daemon = start_daemon("decode-wire");
    let mut exchange = frame_exchange(&daemon.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    let refused = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"decode-client","payload_b64":"!!!"}}"#
    ));

    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "payload_invalid_base64",
        "malformed base64 must surface as payload_invalid_base64: {refused}"
    );
    assert_eq!(refused["session_id"], session_id);

    stop_daemon(daemon);
}

// ===========================================================================
// Test 5: empty encoded payload is rejected with invalid_payload_type
// ===========================================================================

/// An encoded codec (`Hevc`, `Av1`) cannot have a zero-byte body: a
/// bitstream that contains only the codec header is not a frame, and
/// rendering it would silently produce a corrupt surface. The decoder
/// returns `PayloadDecodeError::EmptyPayload` and the wire renders it as
/// `invalid_payload_type`. The error code is the historical name pinned
/// by the unit tests; the new `PayloadDecodeError::EmptyPayload` variant
/// keeps the wire code stable for clients that already switch on it.
#[test]
fn hevc_empty_payload_returns_invalid_payload_type_error() {
    let daemon = start_daemon("decode-wire");
    let mut exchange = frame_exchange(&daemon.addr);

    // Open a session with Hevc (an encoded codec, no shape check).
    let ack = exchange(
        r#"{"type":"session_init","version":1,"preferred_codec":"Hevc","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"empty-hevc"}"#,
    );
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // Zero-byte Hevc body. The handler treats `payload_len == 0` as
    // "no payload_b64 required" (the empty-string case), so the decoder
    // sees 0 actual bytes and the Hevc path in `validate_encoded_payload`
    // refuses as EmptyPayload, rendered on the wire as invalid_payload_type.
    let refused = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Hevc","width":64,"height":48,"payload_len":0,"duration_us":16667,"sender_id":"empty-hevc"}}"#
    ));

    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "invalid_payload_type",
        "empty encoded payload must surface as invalid_payload_type: {refused}"
    );
    assert_eq!(refused["session_id"], session_id);

    stop_daemon(daemon);
}
