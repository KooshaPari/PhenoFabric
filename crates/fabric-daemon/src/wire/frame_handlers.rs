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

use super::frame_session_state::FrameSessionState;
use crate::coordinator::Coordinator;

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

/// Build a `frame_error` response with an explicit reason.
fn frame_error(error: &str, message: String) -> String {
    respond(serde_json::json!({
        "type": "frame_error",
        "error": error,
        "message": message,
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

    let session = coordinator.frame_sessions_mut().insert_session(&init);

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
        return unknown_session_error(&state, session_id);
    }

    let counters = state.update_session(session_id, |session| {
        session.last_seq = Some(seq);
        session.frame_count += 1;
        session.frames_received += payload_len as u64;
    });

    let Some(counters) = counters else {
        return unknown_session_error(&state, session_id);
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
    }))
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

    let mut state = coordinator.frame_sessions_mut();
    if !state.contains(session_id) {
        return unknown_session_error(&state, session_id);
    }

    let mut acked = false;
    let counters = state.update_session(session_id, |session| {
        // `acked` must mean "the daemon had already received a frame with this
        // sequence number", not merely "this ack is now in the map". The
        // session's `last_seq` is the highest sequence received, so an ack at
        // or below it is genuinely acknowledged.
        acked = session.last_seq.is_some_and(|last| ack.seq <= last);
        session.acks.insert(ack.seq, ack.rtt_us);
    });

    let Some(counters) = counters else {
        return unknown_session_error(&state, session_id);
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
    }))
}

#[cfg(test)]
#[path = "frame_handlers_tests.rs"]
mod tests;
