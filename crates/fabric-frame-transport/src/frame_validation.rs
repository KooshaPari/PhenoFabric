//! Field-type validation for the frame-plane message types.
//!
//! Split out of `validation.rs` so both files stay under the project's
//! 500-line ceiling. Behavior is unchanged: `validation::validate_message`
//! dispatches here for `session_init`, `frame_data`, and `frame_ack`.
//!
//! The field names and widths come from `fabric_frame_transport`:
//! `SessionInit` (`lib.rs:108-116`), `FrameHeader` (`lib.rs:131-141`), and
//! `FrameAck` (`lib.rs:195-199`).

use serde_json::Value;

use crate::validation::{normalize_type, type_name, ValidationError};

/// Validate the fields of a frame-plane message.
///
/// Only the fields that the daemon's frame handlers actually read are
/// checked. A malformed value is reported as `WRONG_TYPE` here rather than
/// silently defaulting, because `serde_json::Value::as_u64` returns `None`
/// for a float, a negative number, or a string, and the handlers would
/// otherwise treat all three as `0`.
pub(super) fn validate_frame_fields(
    obj: &serde_json::Map<String, Value>,
    msg_type: &str,
) -> Result<(), ValidationError> {
    match normalize_type(msg_type).as_str() {
        "sessioninit" => {
            string(obj, "client_id", msg_type)?;
            u32(obj, "version", msg_type)?;
            u32(obj, "width", msg_type)?;
            u32(obj, "height", msg_type)?;
            u32(obj, "target_fps", msg_type)?;
            u32(obj, "max_latency_ms", msg_type)?;
            Ok(())
        }
        "framedata" => {
            u64(obj, "seq", msg_type)?;
            u64(obj, "pts_us", msg_type)?;
            u64(obj, "dts_us", msg_type)?;
            u32(obj, "width", msg_type)?;
            u32(obj, "height", msg_type)?;
            u32(obj, "payload_len", msg_type)?;
            u32(obj, "duration_us", msg_type)?;
            bool(obj, "is_keyframe", msg_type)?;
            Ok(())
        }
        "frameack" => {
            u64(obj, "seq", msg_type)?;
            u64(obj, "rtt_us", msg_type)?;
            u64(obj, "recv_pts_us", msg_type)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Require a present field to be a string.
fn string(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
) -> Result<(), ValidationError> {
    if let Some(val) = obj.get(field) {
        if !val.is_string() {
            return Err(ValidationError {
                message: format!(
                    "field '{field}' must be a string in '{msg_type}', got {}",
                    type_name(val)
                ),
                code: "WRONG_TYPE".into(),
            });
        }
    }
    Ok(())
}

/// Require a present field to be an optional boolean.
fn bool(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
) -> Result<(), ValidationError> {
    if let Some(val) = obj.get(field) {
        if !val.is_boolean() {
            return Err(ValidationError {
                message: format!(
                    "field '{field}' must be a boolean in '{msg_type}', got {}",
                    type_name(val)
                ),
                code: "WRONG_TYPE".into(),
            });
        }
    }
    Ok(())
}

/// Require a present field to be an unsigned integer that fits a `u32`.
fn u32(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
) -> Result<(), ValidationError> {
    unsigned(obj, field, msg_type, u32::MAX as u64)
}

/// Require a present field to be an unsigned integer that fits a `u64`.
fn u64(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
) -> Result<(), ValidationError> {
    unsigned(obj, field, msg_type, u64::MAX)
}

/// Shared check for optional non-negative integer fields.
fn unsigned(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
    max: u64,
) -> Result<(), ValidationError> {
    if let Some(val) = obj.get(field) {
        if val.as_u64().filter(|n| *n <= max).is_none() {
            return Err(ValidationError {
                message: format!(
                    "field '{field}' must be an unsigned integer <= {max} in '{msg_type}', got {}",
                    type_name(val)
                ),
                code: "WRONG_TYPE".into(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "frame_validation_tests.rs"]
mod tests;
