//! Frame-session ownership transition over the daemon's line-delimited JSON wire.
//!
//! Handles the `session_transfer` request: a peer that knows the
//! server-assigned `session_id` hands the input / output / focus seats of the
//! session to new peer ids. After a successful transfer, frames that name a
//! `sender_id` other than the new `input_owner` are rejected by
//! [`super::frame_handlers`], so the previous owner is locked out atomically
//! rather than by some out-of-band session-close.
//!
//! # Why this is its own module
//!
//! `frame_handlers.rs` is the dispatcher for the frame plane. The transfer
//! path is a self-contained control-plane request with its own error
//! taxonomy (`degenerate_transfer`, `unknown_session`), and putting it in
//! its own module keeps both the dispatcher and the transfer handler readable
//! and under the project's 500-line ceiling.
//!
//! # The trust model
//!
//! Like resume, transfer is bearer-grade: a peer that knows the
//! `session_id` can request a transfer. This matches the existing protocol
//! because a peer that can transfer could also send `frame_data` as the
//! current input owner until a legitimate transfer happens. The transfer
//! path adds two things on top of the bearer model:
//!
//! 1. The new seats are validated *at the daemon*: empty owner ids are
//!    refused, the focus seat is constrained to track the new input owner,
//!    and a transfer against an unknown session is reported by the same
//!    `unknown_session` error `frame_data` uses. The state after a refused
//!    transfer is byte-identical to the state before it.
//! 2. The transfer is the only state-layer path that mutates ownership after
//!    `session_init`. There is no implicit "the most recent sender is the
//!    input owner" rule; ownership is recorded on the session, and it only
//!    changes when the protocol says so.

use serde_json::Value;

use super::frame_session_state::TransferRejection;
use crate::coordinator::Coordinator;

/// Handle a `session_transfer` message.
///
/// On success, the session's input / output / focus seats are replaced and
/// the response carries the new seats so the caller can verify the daemon
/// recorded what it asked for. On refusal, the session is unchanged and the
/// error names the reason.
///
/// # Errors
///
/// Refuses, with a specific and distinct error, when:
/// * `session_id` is absent (`missing_session_id`) -- a transfer that cannot
///   identify its target is not a transfer.
/// * `input_owner` or `output_owner` is empty (`degenerate_transfer`).
/// * the session id names a session this daemon never issued
///   (`unknown_session`).
/// * the session id names a session this daemon tore down (`session_gone`).
///
/// Every refusal path mutates nothing. The state after a refused transfer is
/// the state the session was already in.
pub fn handle_session_transfer(value: &Value, coordinator: &Coordinator) -> String {
    let session_id = match value.get("session_id").and_then(Value::as_u64) {
        Some(id) => id,
        None => {
            return frame_error(
                "missing_session_id",
                "session_transfer must carry the session_id of the session to transfer".to_string(),
            )
        }
    };

    // Empty-string handling: the schema layer guarantees these are present
    // and of string type, but it does not check for emptiness. An empty
    // `input_owner` is a degenerate transition because the daemon would have
    // no peer to attribute frames to; reporting it here (rather than
    // recording it and rejecting on the next frame) is the difference
    // between a clear refusal and a confused downstream consumer.
    let input_owner = match value.get("input_owner").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return transfer_rejection_error(
                &TransferRejection::DegenerateTransfer,
                session_id,
                "input_owner must be a non-empty string",
            )
        }
    };
    let output_owner = match value.get("output_owner").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return transfer_rejection_error(
                &TransferRejection::DegenerateTransfer,
                session_id,
                "output_owner must be a non-empty string",
            )
        }
    };
    // `focus_owner` is optional. If absent, the state layer falls back to the
    // new `input_owner`; if present, an empty string is a degenerate
    // transition for the same reason as the others.
    let focus_owner = match value.get("focus_owner") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => {
            return transfer_rejection_error(
                &TransferRejection::DegenerateTransfer,
                session_id,
                "focus_owner must be a non-empty string when present",
            )
        }
    };

    let outcome = coordinator.frame_sessions_mut().transfer_ownership(
        session_id,
        &input_owner,
        &output_owner,
        focus_owner.as_deref(),
    );

    match outcome {
        Ok(outcome) => transfer_ack(&outcome),
        Err(rejection) => transfer_rejection_error(&rejection, session_id, ""),
    }
}

/// Render a successful transfer.
///
/// The response carries the new ownership seats so a peer that asked for a
/// transfer can verify the daemon recorded exactly what it asked for, and a
/// downstream consumer can record the ownership transition for the same
/// reason it records the frame flow.
fn transfer_ack(outcome: &super::frame_session_state::TransferOutcome) -> String {
    let owners = &outcome.owners;
    serde_json::json!({
        "type": "session_transfer_ack",
        "status": "ok",
        "session_id": outcome.session_id,
        "input_owner": owners.input_owner,
        "output_owner": owners.output_owner,
        "focus_owner": owners.focus_owner,
    })
    .to_string()
}

/// Render the error for a refused transfer.
///
/// `degenerate_transfer`, `unknown_session`, and `session_gone` are kept
/// distinct because the recovery path is different: the first means "fix
/// your request and retry", the second means "the session id is not yours
/// to act on", and the third means "the session is final -- it was torn
/// down before your transfer landed".
fn transfer_rejection_error(
    rejection: &TransferRejection,
    session_id: u64,
    detail: &str,
) -> String {
    match rejection {
        TransferRejection::DegenerateTransfer => serde_json::json!({
            "type": "frame_error",
            "error": "degenerate_transfer",
            "session_id": session_id,
            "message": if detail.is_empty() {
                "ownership transfer would leave a session with an empty owner".to_string()
            } else {
                format!("ownership transfer refused: {detail}")
            },
        })
        .to_string(),
        TransferRejection::UnknownSession => serde_json::json!({
            "type": "frame_error",
            "error": "unknown_session",
            "session_id": session_id,
            "message": format!(
                "no frame session with id {session_id}; a session_transfer must \
                 name a session this daemon has issued"
            ),
        })
        .to_string(),
        TransferRejection::SessionGone(reason) => serde_json::json!({
            "type": "frame_error",
            "error": "session_gone",
            "reason": reason.as_str(),
            "session_id": session_id,
            "message": format!(
                "session {session_id} is no longer live ({}); ownership cannot be \
                 transferred on a torn-down session",
                reason.as_str()
            ),
        })
        .to_string(),
        TransferRejection::MissingField => serde_json::json!({
            "type": "frame_error",
            "error": "missing_session_id",
            "session_id": session_id,
            "message": "session_transfer must carry the session_id of the session to transfer".to_string(),
        })
        .to_string(),
    }
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
#[path = "frame_transfer_tests.rs"]
mod tests;
