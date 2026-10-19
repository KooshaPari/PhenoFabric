//! Frame-plane wire handlers.
//!
//! Routes the `fabric-frame-transport` message types (`session_init`,
//! `frame_data`, `frame_ack`) that arrive on the daemon's line-delimited JSON
//! wire server.
//!
//! # Why these are JSON, not length-prefixed binary
//!
//! `fabric_frame_transport::transport` encodes messages as
//! `[4B len][1B type][payload]` over a raw `TcpStream`
//! (`transport.rs:141-149`), which is what `FrameSender`/`FrameReceiver` use.
//! The daemon wire server instead reads newline-delimited JSON
//! (`wire/mod.rs:157`, `for line in reader.lines()`). Those are two distinct
//! transports, and this module is the adapter between them.
//!
//! Nothing here re-implements the transport framing. Each handler
//! deserializes the JSON line into the **real** transport structs
//! (`SessionInit`, `FrameHeader`, `FrameAck`), works with those typed values,
//! and serializes the response back with the real `SessionAck`. Where a value
//! has no JSON representation in a `session_init`/`frame_data` line, the
//! handler reports that explicitly instead of inventing a field.
//!
//! # Session state
//!
//! A frame arriving for an unknown session is an error, never a silent drop.
//! Sessions live in [`frame_session_state::FrameSessionState`], owned by the
//! coordinator, so state survives across connections.

use fabric_frame_transport::{Codec, FrameAck, FrameHeader, SessionAck, SessionInit};
use serde_json::Value;

use super::frame_session_state::{FrameSessionState, TeardownOutcome};
use crate::coordinator::Coordinator;

/// Idle timeout, in milliseconds, applied by [`handle_session_expiry_sweep`].
///
/// Mirrors [`super::frame_session_state::DEFAULT_IDLE_TIMEOUT_MS`] rather than
/// redefining it: one documented bound, one place it can change.
const IDLE_TIMEOUT_MS: u64 = super::frame_session_state::DEFAULT_IDLE_TIMEOUT_MS;

// ---------------------------------------------------------------------------
// Response builders
// ---------------------------------------------------------------------------

/// Serialize a response value to a wire line.
fn respond(value: Value) -> String {
    value.to_string()
}

/// Build the error response for a frame message that references a session
/// which was never established (or has since been dropped).
///
/// This is deliberately a distinct, named error: a frame for an unknown
/// session is a client bug or a race against session teardown, and hiding it
/// behind a generic error would make the stream undebuggable.
fn unknown_session_error(state: &FrameSessionState, session_id: u64) -> String {
    let known = state.known_session_ids();
    respond(serde_json::json!({
        "type": "frame_error",
        "error": "unknown_session",
        "message": format!(
            "no frame session with id {session_id}; {} live session(s): {}",
            state.len(),
            if state.is_empty() {
                "none".to_string()
            } else {
                known.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
            }
        ),
        "session_id": session_id,
        "active_sessions": known,
    }))
}

/// Build the right "this session cannot carry frames" error for `session_id`.
///
/// Two cases, deliberately distinct:
///
/// * The daemon issued this id and has since torn it down (close, cancel, or
///   idle expiry) -- `session_gone` with the reason, because the client did
///   nothing wrong and the daemon owes it an explanation.
/// * The daemon never issued this id -- `unknown_session`, because the client
///   is talking about a session that does not exist here.
///
/// Anything that does not resolve here falls back to `unknown_session`, which
/// is the safe reading: a stale id whose tombstone has aged out of the bounded
/// history is indistinguishable from one that was never valid.
fn session_lookup_error(state: &FrameSessionState, session_id: u64) -> String {
    match state.teardown_reason(session_id) {
        Some(reason) => session_gone_error(reason.as_str(), session_id),
        None => unknown_session_error(state, session_id),
    }
}

/// Build a `frame_error` response with an explicit reason.
fn frame_error(error: &str, message: String) -> String {
    respond(serde_json::json!({
        "type": "frame_error",
        "error": error,
        "message": message,
    }))
}

/// Build the rejection for a frame whose declared sender is not the session's
/// current input owner.
///
/// The error names the owner the daemon has on file (not the offender's id)
/// because the offender's id is the *untrusted* input: the daemon does not
/// assume the caller is who they say they are, so the diagnostic points at
/// the truth the daemon can verify.
fn input_owner_mismatch(session_id: u64, current_owner: &str, declared_sender: &str) -> String {
    respond(serde_json::json!({
        "type": "frame_error",
        "error": "input_owner_mismatch",
        "session_id": session_id,
        "current_input_owner": current_owner,
        "declared_sender_id": declared_sender,
        "message": format!(
            "frame declared sender_id '{declared_sender}' but session {session_id} is \
             owned by '{current_owner}'; only the current input owner can drive frames"
        ),
    }))
}

/// Build the `frame_error` for a `frame_data` / `frame_ack` that names a
/// session which is no longer live.
///
/// `reason` distinguishes "the client closed it", "the client cancelled it",
/// "the idle sweep took it", and "this daemon never had that id". A client
/// whose stream died sees `session_gone` with the real cause rather than a
/// generic unknown-session error that makes an expiry look like a client bug.
fn session_gone_error(reason: &str, session_id: u64) -> String {
    respond(serde_json::json!({
        "type": "frame_error",
        "error": "session_gone",
        "reason": reason,
        "message": format!(
            "session {session_id} is no longer live ({reason}); \
             open a new session with session_init"
        ),
        "session_id": session_id,
    }))
}

// ---------------------------------------------------------------------------
// Field extraction
// ---------------------------------------------------------------------------

/// Read an optional `u64` field, defaulting to 0 when absent.
///
/// Type correctness is already enforced upstream by
/// `fabric_frame_transport::validation`, so a present value is always a
/// non-negative integer that fits.
fn opt_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

/// Read an optional `u32` field, defaulting to 0 when absent.
fn opt_u32(value: &Value, field: &str) -> u32 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0) as u32
}

/// Read an optional `bool` field, defaulting to false when absent.
fn opt_bool(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

/// Parse a codec name into the real [`Codec`] enum.
///
/// The transport serializes `Codec` via serde, so its JSON representation is
/// the variant name (`"Hevc"`, `"Av1"`, `"Nv12"`, `"Rgba"`). An unrecognized
/// name is an error rather than a silent fallback, because quietly choosing a
/// different codec than the client asked for would misreport what the daemon
/// did with the frame.
fn parse_codec(value: &Value) -> Result<Codec, String> {
    let name = value
        .as_str()
        .ok_or_else(|| "codec must be a string".to_string())?;
    match name {
        "Hevc" | "HEVC" | "hevc" => Ok(Codec::Hevc),
        "Av1" | "AV1" | "av1" => Ok(Codec::Av1),
        "Nv12" | "NV12" | "nv12" => Ok(Codec::Nv12),
        "Rgba" | "RGBA" | "rgba" => Ok(Codec::Rgba),
        other => Err(format!("unknown codec '{other}'")),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Handle a `session_init` message.
///
/// Deserializes the line into the real [`SessionInit`], registers a session,
/// and replies with a serialized real [`SessionAck`].
///
/// # Errors
///
/// Returns a `frame_error` response when the line cannot be deserialized into
/// `SessionInit` — the handler reports the reason rather than defaulting a
/// half-built session.
pub fn handle_session_init(value: &Value, coordinator: &Coordinator) -> String {
    let init: SessionInit = match serde_json::from_value(value.clone()) {
        Ok(init) => init,
        Err(e) => {
            return frame_error(
                "invalid_session_init",
                format!("session_init does not match the transport SessionInit shape: {e}"),
            )
        }
    };

    if init.width == 0 || init.height == 0 {
        return frame_error(
            "invalid_session_init",
            format!(
                "session_init dimensions must be non-zero, got {}x{}",
                init.width, init.height
            ),
        );
    }

    if init.target_fps == 0 {
        return frame_error(
            "invalid_session_init",
            "session_init target_fps must be non-zero".to_string(),
        );
    }

    // Cap check before insertion, so the (N+1)th session is rejected rather
    // than admitted and then failed. `session_limit_reached` is distinct from
    // `invalid_session_init`: nothing about this request is malformed, the
    // daemon simply has no room, and a client that backs off should be able to
    // tell that apart from a bug in its own request.
    let now_ms = coordinator.now_ms();
    {
        let state = coordinator.frame_sessions_mut();
        if !state.has_session_capacity() {
            return frame_error(
                "session_limit_reached",
                format!(
                    "this daemon already holds the maximum of {} concurrent frame sessions; \
                     close or cancel an existing session before opening a new one",
                    state.session_capacity()
                ),
            );
        }
    }

    let session = coordinator
        .frame_sessions_mut()
        .insert_session_at(&init, now_ms);

    // Reply with the real SessionAck type, not an ad-hoc JSON object, so the
    // response is guaranteed to match what a transport client expects.
    let ack = SessionAck {
        codec: session.codec,
        width: session.width,
        height: session.height,
        fps: session.fps,
        session_id: session.session_id,
        server_time_ms: coordinator.now_ms(),
    };

    // Echo the initial ownership seats on the session_ack so the client
    // does not have to send a follow-up request to learn them. The seats
    // are the same string -- the opener's client_id -- for all three fields
    // at t=0; the daemon only diverges them on an explicit session_transfer.
    let input_owner = session.input_owner.clone();
    let output_owner = session.output_owner.clone();
    let focus_owner = session.focus_owner.clone();

    respond(serde_json::json!({
        "type": "session_ack",
        "codec": ack.codec,
        "width": ack.width,
        "height": ack.height,
        "fps": ack.fps,
        "session_id": ack.session_id,
        "server_time_ms": ack.server_time_ms,
        // Echoed back so the client can see exactly what the daemon recorded
        // for this session. These are the `SessionInit` values, not invented
        // ones: `max_latency_ms` and `version` are the client's own.
        "client_id": session.client_id,
        "max_latency_ms": session.max_latency_ms,
        "version": session.version,
        // Initial ownership seats. A subsequent session_transfer can change
        // any of these; the response to a transfer is `session_transfer_ack`.
        "input_owner": input_owner,
        "output_owner": output_owner,
        "focus_owner": focus_owner,
    }))
}

/// Handle a `frame_data` message.
///
/// Reconstructs the real [`FrameHeader`] from the line and accounts the frame
/// against its session.
///
/// # Errors
///
/// Returns `unknown_session` when `session_id` names no live session. The
/// frame is not counted anywhere and the error names the sessions that do
/// exist, so the failure is diagnosable from the wire alone.
pub fn handle_frame_data(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match value.get("session_id").and_then(Value::as_u64) {
        Some(id) => id,
        None => {
            return frame_error(
                "missing_session_id",
                "frame_data must carry the session_id from its session_ack".to_string(),
            )
        }
    };

    // Reconstruct the real transport header. Fields not present on the JSON
    // line keep the transport's documented shape rather than being invented:
    // `seq` is required by the schema, and the codec must be named explicitly
    // because guessing it would misreport the frame.
    let seq = opt_u64(value, "seq");
    let codec = match value.get("codec").map(parse_codec) {
        Some(Ok(codec)) => codec,
        Some(Err(msg)) => return frame_error("invalid_codec", msg),
        None => return frame_error("missing_codec", "frame_data must name a codec".to_string()),
    };

    // `sender_id` is required by the schema, so reaching here with a missing
    // field is a protocol bug rather than a normal path. The schema layer
    // produces `validation_error`; this branch is the safety net for any
    // future call that bypasses the schema and reaches the handler directly.
    let sender_id = match value.get("sender_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return frame_error(
                "missing_sender_id",
                "frame_data must declare sender_id naming the current input owner".to_string(),
            )
        }
    };

    let payload_len = opt_u32(value, "payload_len");
    let header = FrameHeader {
        seq,
        pts_us: opt_u64(value, "pts_us"),
        dts_us: opt_u64(value, "dts_us"),
        is_keyframe: opt_bool(value, "is_keyframe"),
        codec,
        width: opt_u32(value, "width"),
        height: opt_u32(value, "height"),
        payload_len,
        duration_us: opt_u32(value, "duration_us"),
    };

    // Enforce the transport's own size ceiling (`MAX_FRAME_SIZE`, lib.rs:38)
    // rather than inventing a different limit.
    if header.payload_len as usize > fabric_frame_transport::MAX_FRAME_SIZE {
        return frame_error(
            "frame_too_large",
            format!(
                "frame payload_len {} exceeds MAX_FRAME_SIZE {}",
                header.payload_len,
                fabric_frame_transport::MAX_FRAME_SIZE
            ),
        );
    }

    let mut state = coordinator.frame_sessions_mut();
    if !state.contains(session_id) {
        return session_lookup_error(&state, session_id);
    }

    // Ownership gate: the declared sender must match the session's current
    // input owner. A mismatch is refused before the frame is counted, so a
    // non-owner's stream cannot pollute the session's frame accounting or
    // move the focus seat. The match is exact -- whitespace and case count
    // -- so a typo in `sender_id` is reported rather than silently coerced
    // into acceptance.
    let current_input_owner = state
        .get(session_id)
        .map(|s| s.input_owner.clone())
        .unwrap_or_default();
    if sender_id != current_input_owner {
        return input_owner_mismatch(session_id, &current_input_owner, &sender_id);
    }

    // Decode the body bytes. Runs *after* the ownership gate so a foreign
    // sender's bytes are never inspected, and *before* the session is
    // touched, so a refused decode leaves the session's accounting
    // byte-identical to what it was before the refused frame. The four
    // failure modes (truncated, length mismatch, invalid payload type,
    // invalid base64) each render as a distinct wire `error` code so a
    // client can distinguish "your bytes were cut off" from "your bytes
    // were the right count but the wrong shape" from "your codec is fine
    // but you sent an empty encoded frame".
    let payload_b64 = match value.get("payload_b64").and_then(Value::as_str) {
        Some(b64) => b64.to_string(),
        None if payload_len == 0 => String::new(),
        None => {
            return frame_error(
                "missing_payload_b64",
                "frame_data with payload_len > 0 must carry payload_b64 (base64)".to_string(),
            );
        }
    };
    let decoded = match fabric_frame_transport::frame_payload::decode_payload(
        codec,
        header.width,
        header.height,
        payload_len,
        &payload_b64,
    ) {
        Ok(decoded) => decoded,
        Err(err) => {
            // Include `session_id` in the decoder refusal so a client that
            // sees the error knows which session the daemon was working
            // against. The generic `frame_error` helper omits it because
            // some pre-decode error paths (missing `session_id` itself,
            // unknown codec) have no session to name; the decoder runs
            // *after* the session lookup, so a session is always in scope.
            return respond(serde_json::json!({
                "type": "frame_error",
                "error": err.code(),
                "session_id": session_id,
                "message": err.message(),
            }));
        }
    };

    // Verification fingerprint: the base64 of the first 8 bytes of the
    // decoded body. An empty body produces an empty string. The integration
    // test `frame_payload_decode_wire.rs` round-trips this field to
    // prove the live handler is running the decoder on the bytes the
    // sender transmitted, not silently passing the envelope through. A
    // stubbed-out decoder (or a bypass that skips the decode step) would
    // either omit the field or echo a different prefix, and the test
    // would fail.
    //
    // The fingerprint must be captured *before* the closure that hands
    // the bytes to `push_received_payload`, because that closure moves
    // `decoded.bytes`. A borrowed `&[u8]` slice of the first 8 bytes is
    // enough for the helper.
    let preview_bytes: [u8; 8] = {
        let src = decoded.bytes.get(..8).unwrap_or(&decoded.bytes);
        let mut buf = [0u8; 8];
        buf[..src.len()].copy_from_slice(src);
        buf
    };
    let payload_first8_b64: String =
        base64_encode_first8(&preview_bytes[..decoded.bytes.len().min(8)]);

    let counters = state.touch_session(session_id, coordinator.now_ms(), |session| {
        session.last_seq = Some(seq);
        session.frame_count += 1;
        session.frames_received += payload_len as u64;
        session.focus_owner = sender_id.clone();
        // The sink is the truth about what the receiver reconstructed.
        // Pushing only on a successful decode means a refused frame
        // does not pollute the integration test's view of the wire.
        session.push_received_payload(decoded.bytes);
    });

    let Some(counters) = counters else {
        return session_lookup_error(&state, session_id);
    };

    respond(serde_json::json!({
        "type": "frame_ack",
        "status": "ok",
        "session_id": session_id,
        "seq": seq,
        "last_seq": counters.last_seq,
        "frame_count": counters.frame_count,
        "frames_received": counters.frames_received,
        "payload_len": header.payload_len,
        "is_keyframe": header.is_keyframe,
        // Echo the ownership seats so a downstream consumer can verify the
        // daemon's framing decision on every accepted frame, not just at
        // session start.
        "input_owner": current_input_owner,
        "focus_owner": sender_id,
        // The verification fingerprint described above. `""` for an empty
        // body, 12 base64 characters (8 bytes + padding) otherwise. The
        // field is additive and ignored by clients that only care about
        // the standard `frame_ack` shape.
        "payload_first8_b64": payload_first8_b64,
    }))
}

/// Base64-encode the first 8 bytes (or fewer) of a decoded body.
///
/// `fabric-daemon` is not supposed to grow a new top-level dependency on
/// `base64` for one 12-character field, so this hand-rolls the standard
/// alphabet for the eight-byte-or-fewer case the wire exposes. The
/// alphabet is the same one the decoder uses, so an integration test
/// that hand-rolls the same encoding gets an exact byte-for-byte match.
///
/// `bytes.len()` is guaranteed to be `<= 8` by the caller. The function
/// is private to `handle_frame_data` and is not part of the public API.
fn base64_encode_first8(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    debug_assert!(bytes.len() <= 8, "first8 helper expects <= 8 bytes");
    if bytes.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(12);
    // 8 bytes -> 6 full triples (18 chars) + 1 trailing pair (3 chars) + 1
    // padding char = 12 chars, matching base64 of an 8-byte input.
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        out.push(ALPHABET[((b1 & 0x0f) << 2 | (b2 >> 6)) as usize] as char);
        out.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i];
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[((b0 & 0x03) << 4) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        out.push(ALPHABET[((b1 & 0x0f) << 2) as usize] as char);
        out.push('=');
    }
    out
}

/// Handle a `frame_ack` message.
///
/// Records the client's acknowledgment and its reported RTT against the
/// session, then echoes the session's current counters.
///
/// # Errors
///
/// Returns `unknown_session` when `session_id` names no live session. A client
/// acknowledging a frame the daemon never received is exactly the kind of
/// event that must be visible rather than absorbed.
pub fn handle_frame_ack(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match value.get("session_id").and_then(Value::as_u64) {
        Some(id) => id,
        None => {
            return frame_error(
                "missing_session_id",
                "frame_ack must carry the session_id from its session_ack".to_string(),
            )
        }
    };

    // Rebuild the real transport ack. `seq` and `rtt_us` are what the session
    // state records; `recv_pts_us` is carried by the transport type even
    // though the daemon does not currently act on it.
    let ack = FrameAck {
        seq: opt_u64(value, "seq"),
        rtt_us: opt_u64(value, "rtt_us"),
        recv_pts_us: opt_u64(value, "recv_pts_us"),
    };

    // `sender_id` is the ownership gate for frame_ack as for frame_data: the
    // ack is produced by the same logical sender (the input owner) and a
    // foreign ack is refused before it can move counters.
    let sender_id = match value.get("sender_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return frame_error(
                "missing_sender_id",
                "frame_ack must declare sender_id naming the current input owner".to_string(),
            )
        }
    };

    let mut state = coordinator.frame_sessions_mut();
    if !state.contains(session_id) {
        return session_lookup_error(&state, session_id);
    }

    let current_input_owner = state
        .get(session_id)
        .map(|s| s.input_owner.clone())
        .unwrap_or_default();
    if sender_id != current_input_owner {
        return input_owner_mismatch(session_id, &current_input_owner, &sender_id);
    }

    let mut acked = false;
    let counters = state.touch_session(session_id, coordinator.now_ms(), |session| {
        // `acked` must mean "the daemon had already received a frame with this
        // sequence number", not merely "this ack is now in the map". The
        // session's `last_seq` is the highest sequence received, so an ack at
        // or below it is genuinely acknowledged.
        acked = session.last_seq.is_some_and(|last| ack.seq <= last);
        session.acks.insert(ack.seq, ack.rtt_us);
    });

    let Some(counters) = counters else {
        return session_lookup_error(&state, session_id);
    };

    // Reported back so the client can see what the daemon recorded for it.
    let recorded_rtt_us = state
        .get(session_id)
        .and_then(|s| s.acks.get(&ack.seq))
        .copied();

    respond(serde_json::json!({
        "type": "frame_ack_response",
        "status": "ok",
        "session_id": session_id,
        "seq": ack.seq,
        "acked": acked,
        "rtt_us": ack.rtt_us,
        "recv_pts_us": ack.recv_pts_us,
        "frame_count": counters.frame_count,
        "frames_received": counters.frames_received,
        "ack_count": counters.ack_count,
        "last_seq": counters.last_seq,
        "recorded_rtt_us": recorded_rtt_us,
        // The input owner is the only legitimate sender of an ack for this
        // session; echoing it back lets the caller verify its own
        // identity assumption against the daemon's state.
        "input_owner": current_input_owner,
    }))
}

// ---------------------------------------------------------------------------
// Session teardown
// ---------------------------------------------------------------------------

/// Read the `session_id` a teardown message names, or report the missing field.
///
/// Shared by close, cancel, and the sweep so all three reject a malformed
/// teardown request the same way rather than each inventing its own error.
fn teardown_session_id(value: &Value, message_type: &str) -> Result<u64, String> {
    match value.get("session_id").and_then(Value::as_u64) {
        Some(id) => Ok(id),
        None => Err(frame_error(
            "missing_session_id",
            format!("{message_type} must carry the session_id from its session_ack"),
        )),
    }
}

/// Render the response to a successful close or cancel.
///
/// One builder for both: the two paths have identical effects (the session
/// leaves the map), so their responses differ only in the `reason` string the
/// state recorded. Sharing the builder keeps the two responses impossible to
/// drift apart field by field.
fn teardown_response(outcome: &TeardownOutcome) -> String {
    respond(serde_json::json!({
        "type": "session_close_ack",
        "status": "ok",
        "session_id": outcome.session_id,
        "reason": outcome.reason.as_str(),
        "client_id": outcome.client_id,
        "frame_count": outcome.counters.frame_count,
        "frames_received": outcome.counters.frames_received,
        "ack_count": outcome.counters.ack_count,
        "last_seq": outcome.counters.last_seq,
    }))
}

/// Handle a `session_close` message.
///
/// Removes the session from the coordinator's session table. After this
/// returns, a `frame_data` naming that id is a `session_gone` error, not a
/// silent accept and not a panic.
///
/// # Errors
///
/// Returns the missing-field error when `session_id` is absent, and
/// `session_gone` / `unknown_session` when no live session has that id. A
/// double-close is therefore an error rather than a second success, which is
/// what lets a client tell "I closed it" from "someone already tore it down".
pub fn handle_session_close(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match teardown_session_id(value, "session_close") {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let outcome = coordinator.frame_sessions_mut().close_session(session_id);

    match outcome {
        Some(outcome) => teardown_response(&outcome),
        None => session_lookup_error(&coordinator.frame_sessions_mut(), session_id),
    }
}

/// Handle a `session_cancel` message.
///
/// Cancels a session mid-transfer. The semantics required of cancellation are
/// met by the same removal that close performs: the session is not left in an
/// active state (it is gone from the map), any partial state is released (the
/// entry, its counters, and its recorded RTTs are dropped), and a subsequent
/// `frame_data` for that id fails cleanly with `session_gone`.
///
/// Cancellation is a distinct message from close for the client's benefit --
/// it records intent ("I aborted") instead of success ("I finished") -- not
/// because the daemon treats them differently.
///
/// # Errors
///
/// Returns the missing-field error when `session_id` is absent, and
/// `session_gone` / `unknown_session` when no live session has that id.
pub fn handle_session_cancel(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match teardown_session_id(value, "session_cancel") {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let outcome = coordinator.frame_sessions_mut().cancel_session(session_id);

    match outcome {
        Some(outcome) => teardown_response(&outcome),
        None => session_lookup_error(&coordinator.frame_sessions_mut(), session_id),
    }
}

/// Run the idle-expiry sweep and return the number of sessions removed.
///
/// Called by the wire server on its accept-loop tick (see `wire/mod.rs`), which
/// is the only place that runs on a wall-clock cadence independent of client
/// traffic. A daemon that receives no frame traffic at all still sweeps,
/// because the sweep does not depend on a message arriving to trigger it.
///
/// The timeout is [`IDLE_TIMEOUT_MS`]; see
/// [`super::frame_session_state::DEFAULT_IDLE_TIMEOUT_MS`] for why that bound.
pub fn sweep_expired_frame_sessions(coordinator: &Coordinator) -> Vec<u64> {
    let now_ms = coordinator.now_ms();
    coordinator
        .frame_sessions_mut()
        .expire_idle_sessions(now_ms, IDLE_TIMEOUT_MS)
}

#[cfg(test)]
#[path = "frame_handlers_tests.rs"]
mod tests;
