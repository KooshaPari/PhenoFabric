//! Frame-session resume over the daemon's line-delimited JSON wire.
//!
//! Handles the `session_resume` request: a client that lost its TCP connection
//! re-presents the server-assigned `session_id` from its original
//! `session_ack` and continues the *same* session.
//!
//! # Why this is its own module
//!
//! `frame_handlers.rs` was already at 530 lines before resume existed -- over
//! this project's 500-line hard ceiling. Adding the resume handler, its error
//! builders, and its documentation to that file would have pushed it further
//! past the limit rather than bringing it back under. The resume path is a
//! self-contained concern with its own error taxonomy, so it lives here and
//! `frame_handlers` gains a single dispatch line.
//!
//! # The trust model, stated plainly
//!
//! `session_resume` carries exactly one field: `session_id`. That is the same
//! proof the protocol already accepts for `frame_data`, `frame_ack`,
//! `session_close`, and `session_cancel` -- a daemon-issued, monotonically
//! allocated, never-reused id that the client learned from its own `session_ack`.
//!
//! **This is bearer-token-grade, not authenticated.** Resume does not invent a
//! token, a nonce, a handshake, or a credential, because the underlying
//! transport has none to give. It also does not weaken the existing model: a
//! peer that could resume a session could already stream frames into it and
//! tear it down. The property resume adds is that a *torn-down* id stays dead
//! forever -- a refused resume never resurrects a session and never creates a
//! partial one.
//!
//! A future authenticated resume would need a handshake this protocol does not
//! have, and belongs in its own change rather than being faked here.

use serde_json::Value;

use super::frame_session_state::{
    FrameSessionDisconnect, ResumeOutcome, ResumeRejection, TeardownReason,
};
use crate::coordinator::Coordinator;

/// Handle a `session_resume` message.
///
/// On success, reports the accounting the session was carrying at the moment of
/// the disconnect, so the client can resume its sequence where it left off
/// rather than restarting from zero.
///
/// # Errors
///
/// Refuses, with a specific and distinct error, when:
///
/// * `session_id` is absent (`missing_session_id`) -- a resume that cannot
///   identify its target is not a resume.
/// * the id names no session this daemon issued (`unknown_session`).
/// * the id names a session this daemon tore down (`session_gone` with the
///   reason: `closed`, `cancelled`, or `expired`). Teardown is final.
///
/// Every refusal path mutates nothing: no session is created, none is
/// resurrected, and no existing session's accounting is touched. A refused
/// resume therefore cannot leave half-open state behind.
pub fn handle_session_resume(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match value.get("session_id").and_then(Value::as_u64) {
        Some(id) => id,
        None => {
            return frame_error(
                "missing_session_id",
                "session_resume must carry the session_id from its session_ack".to_string(),
            )
        }
    };

    let now_ms = coordinator.now_ms();
    let outcome = coordinator
        .frame_sessions_mut()
        .resume_session(session_id, now_ms);

    match outcome {
        Ok(outcome) => resume_ack(&outcome),
        Err(rejection) => resume_rejection_error(&rejection, session_id),
    }
}

/// Render a successful resume.
///
/// The response is deliberately shaped like the `session_ack` it follows: it
/// echoes the negotiated parameters plus the session's current counters, so a
/// client that already parses `session_ack` can parse this too. The two fields
/// that make resume *useful* rather than merely permitted are `last_seq` (the
/// highest sequence the daemon already accepted -- the client's next frame must
/// be above it) and `was_disconnected` (whether the daemon had actually
/// recorded a drop, as opposed to the client resuming a session that was never
/// marked dormant).
fn resume_ack(outcome: &ResumeOutcome) -> String {
    let counters = &outcome.counters;
    serde_json::json!({
        "type": "session_resume_ack",
        "status": "ok",
        "session_id": outcome.session_id,
        "client_id": outcome.client_id,
        "state": outcome.state.as_str(),
        "was_disconnected": outcome.was_disconnected,
        "frame_count": counters.frame_count,
        "frames_received": counters.frames_received,
        "ack_count": counters.ack_count,
        // Carried forward, not reset. A client that treats this as its next
        // sequence number is reading the daemon's real accounting.
        "last_seq": counters.last_seq,
    })
    .to_string()
}

/// Render the error for a refused resume.
///
/// `unknown_session` and `session_gone` are kept distinct for the same reason
/// `frame_data` keeps them distinct: a client whose session expired deserves to
/// be told so, not told its id was never valid.
fn resume_rejection_error(rejection: &ResumeRejection, session_id: u64) -> String {
    match rejection {
        ResumeRejection::Unknown => serde_json::json!({
            "type": "frame_error",
            "error": "unknown_session",
            "message": format!(
                "no frame session with id {session_id}; open a new session with session_init"
            ),
            "session_id": session_id,
        })
        .to_string(),
        ResumeRejection::Gone(reason) => session_gone_error(reason, session_id),
    }
}

/// Build the `session_gone` error for a resume of a torn-down session.
///
/// The message says resume is refused specifically -- not merely that the id is
/// unknown -- because the client may well be retrying a resume it has every
/// reason to believe should work.
fn session_gone_error(reason: &TeardownReason, session_id: u64) -> String {
    serde_json::json!({
        "type": "frame_error",
        "error": "session_gone",
        "reason": reason.as_str(),
        "message": format!(
            "session {session_id} is no longer live ({}); it cannot be resumed -- \
             open a new session with session_init",
            reason.as_str()
        ),
        "session_id": session_id,
    })
    .to_string()
}

/// Build a `frame_error` response with an explicit reason and code.
fn frame_error(error: &str, message: String) -> String {
    serde_json::json!({
        "type": "frame_error",
        "error": error,
        "message": message,
    })
    .to_string()
}
#[cfg(test)]
#[path = "frame_resume_tests.rs"]
mod tests;
