//! Schema registry for wire protocol messages.
//!
//! Embeds the JSON Schema as a const and provides simple validation
//! using field presence and type checks without external dependencies.

/// The complete JSON Schema for the wire protocol (draft-2020-12).
pub const WIRE_PROTOCOL_SCHEMA: &str = include_str!("../../../docs/spec/wire-protocol-schema.json");

/// All known request message types in the wire protocol.
pub const REQUEST_MESSAGE_TYPES: &[&str] = &[
    "heartbeat",
    "health_check",
    "topology_request",
    "routes_request",
    "capabilities_request",
    "probe_request",
    "compile_request",
    "webrtc_offer",
    "webrtc_answer",
    "webrtc_ice",
    "save_config",
    "network_status_request",
    "network_stun_request",
    "network_tailscale_request",
    "surface_lease_request",
    "auth_start",
    "auth_complete",
    "auth_email",
    "auth_status",
    "auth_verify",
    "auth_logout",
    // --- Frame plane (fabric-frame-transport message types) ---
    "session_init",
    "frame_data",
    "frame_ack",
    // Session teardown. `session_close` is the orderly end of a session;
    // `session_cancel` is an abort mid-transfer. Both name a server-assigned
    // `session_id` from a prior `session_ack` and both remove the session.
    "session_close",
    "session_cancel",
    // Session resume. A client that lost its TCP connection re-presents the
    // same server-assigned `session_id` to continue the same session. This is
    // the same proof `session_close`/`session_cancel` already require -- no
    // token, nonce, or handshake is invented for it.
    "session_resume",
    // Explicit ownership transition. The session_id is bearer-grade, so this
    // is the canonical way to hand the input / output / focus seats to a new
    // peer. The new owner ids are required; the focus seat defaults to the
    // new input owner.
    "session_transfer",
];

/// Required fields per message type (fields that MUST be present and non-null).
const REQUIRED_FIELDS: &[(&str, &[&str])] = &[
    ("heartbeat", &[]),
    ("health_check", &[]),
    ("topology_request", &[]),
    ("routes_request", &[]),
    ("capabilities_request", &[]),
    ("probe_request", &[]),
    ("compile_request", &["source", "destination"]),
    ("webrtc_offer", &["target", "sdp"]),
    ("webrtc_answer", &["target", "sdp"]),
    ("webrtc_ice", &["from", "candidate"]),
    // Network introspection requests carry no parameters: the daemon probes
    // the host itself and returns whatever it observes.
    ("network_status_request", &[]),
    ("network_stun_request", &[]),
    ("network_tailscale_request", &[]),
    (
        "surface_lease_request",
        &["name", "protocol", "locality_floor"],
    ),
    ("auth_start", &[]),
    ("auth_complete", &["code"]),
    ("auth_email", &["email"]),
    ("auth_status", &[]),
    ("auth_verify", &["email", "code"]),
    ("auth_logout", &[]),
    // Frame-plane fields mirror `fabric_frame_transport` exactly:
    // `SessionInit` (lib.rs:108-116), `FrameHeader` (lib.rs:131-141),
    // and `FrameAck` (lib.rs:195-199). `frame_data` requires the sequence
    // number because it is what the session state keys frames on.
    //
    // `sender_id` is the ownership field on the frame header: it MUST name the
    // client_id currently registered as the session's `input_owner`. A frame
    // from anyone else is rejected with `input_owner_mismatch` (see
    // `frame_handlers::handle_frame_data`). The same is required on
    // `frame_ack` since acks are produced by the same logical sender.
    ("session_init", &["client_id"]),
    ("frame_data", &["seq", "sender_id"]),
    ("frame_ack", &["seq", "sender_id"]),
    // Teardown names the session it tears down, so `session_id` is mandatory:
    // a close without one could not identify its target.
    ("session_close", &["session_id"]),
    ("session_cancel", &["session_id"]),
    // Resume names the session it continues, for the same reason.
    ("session_resume", &["session_id"]),
    // An explicit ownership transition: session_id names the session,
    // `input_owner` and `output_owner` name the new owners. `focus_owner` is
    // optional and falls back to the new input owner at handler time.
    (
        "session_transfer",
        &["session_id", "input_owner", "output_owner"],
    ),
];

/// Expected field types (field_name, expected_json_type).
/// Type is checked as serde_json::Value variant name.
const FIELD_TYPES: &[(&str, &str, &str)] = &[
    ("source", "string", "compile_request"),
    ("destination", "string", "compile_request"),
    ("intent_name", "string", "compile_request"),
    ("target", "string", "webrtc_offer"),
    ("sdp", "string", "webrtc_offer"),
    ("from", "string", "webrtc_offer"),
    ("target", "string", "webrtc_answer"),
    ("sdp", "string", "webrtc_answer"),
    ("target", "string", "webrtc_ice"),
    ("from", "string", "webrtc_ice"),
    ("candidate", "string", "webrtc_ice"),
    ("config", "object", "save_config"),
    ("overrides", "object", "save_config"),
    // surface_lease_request fields. `requires_rt` is a bool; the CLI always
    // sends it, but it is not required so a minimal request still validates.
    ("name", "string", "surface_lease_request"),
    ("protocol", "string", "surface_lease_request"),
    ("locality_floor", "string", "surface_lease_request"),
    ("requires_rt", "boolean", "surface_lease_request"),
    ("code", "string", "auth_complete"),
    ("code_verifier", "string", "auth_complete"),
    ("email", "string", "auth_email"),
    ("email", "string", "auth_verify"),
    ("code", "string", "auth_verify"),
    // Ownership fields on the frame plane. `sender_id` is a string on
    // `frame_data` and `frame_ack`; `input_owner`, `output_owner`, and
    // `focus_owner` are strings on `session_transfer`. None of these are
    // required by the schema to be non-empty -- emptiness is a semantic check
    // the handler performs against the live session state, where it can be
    // reported as a `degenerate_transfer` rather than a generic type error.
    ("sender_id", "string", "frame_data"),
    ("sender_id", "string", "frame_ack"),
    ("input_owner", "string", "session_transfer"),
    ("output_owner", "string", "session_transfer"),
    ("focus_owner", "string", "session_transfer"),
];

/// Error type for schema validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub message: String,
    pub code: String,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for SchemaError {}

/// Normalize a message type for case-insensitive + underscore-insensitive matching.
///
/// Converts to lowercase and strips underscores so that "HealthCheck",
/// "health_check", and "healthcheck" all match the same canonical type.
fn normalize_type(s: &str) -> String {
    s.to_ascii_lowercase().replace('_', "")
}

/// Get the expected required fields for a message type (case and underscore insensitive).
pub fn required_fields(msg_type: &str) -> &'static [&'static str] {
    let norm = normalize_type(msg_type);
    REQUIRED_FIELDS
        .iter()
        .find(|(t, _)| normalize_type(t) == norm)
        .map(|(_, fields)| *fields)
        .unwrap_or(&[])
}

/// Check if a message type is recognized (case and underscore insensitive).
pub fn is_known_type(msg_type: &str) -> bool {
    let norm = normalize_type(msg_type);
    REQUEST_MESSAGE_TYPES
        .iter()
        .any(|t| normalize_type(t) == norm)
}

/// Validate a JSON string against the embedded schema using simple
/// field-presence and type checks (no external JSON Schema library).
///
/// This performs the following checks:
/// 1. Valid JSON parse
/// 2. Has a "type" field that is a string
/// 3. Message type is recognized
/// 4. All required fields are present
/// 5. Known fields have correct types
pub fn validate_against_schema(json: &str, _schema_name: &str) -> Result<(), SchemaError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| SchemaError {
        message: format!("invalid JSON: {e}"),
        code: "INVALID_JSON".into(),
    })?;

    // Must be an object.
    let obj = value.as_object().ok_or_else(|| SchemaError {
        message: "message must be a JSON object".into(),
        code: "NOT_OBJECT".into(),
    })?;

    // Must have a "type" field.
    let msg_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SchemaError {
            message: "missing required field: type".into(),
            code: "MISSING_TYPE".into(),
        })?;

    // Must be a known type.
    if !is_known_type(msg_type) {
        return Err(SchemaError {
            message: format!("unknown message type: {msg_type}"),
            code: "UNKNOWN_TYPE".into(),
        });
    }

    // Check required fields.
    let required = required_fields(msg_type);
    for field in required {
        if !obj.contains_key(*field) {
            return Err(SchemaError {
                message: format!("missing required field '{field}' for message type '{msg_type}'"),
                code: "MISSING_FIELD".into(),
            });
        }
        // Must not be null.
        if obj.get(*field) == Some(&serde_json::Value::Null) {
            return Err(SchemaError {
                message: format!("field '{field}' must not be null for message type '{msg_type}'"),
                code: "NULL_FIELD".into(),
            });
        }
    }

    // Check field types for this message type.
    for (field, expected_type, for_type) in FIELD_TYPES {
        if normalize_type(for_type) == normalize_type(msg_type) {
            if let Some(val) = obj.get(*field) {
                if !value_matches_type(val, expected_type) {
                    return Err(SchemaError {
                        message: format!(
                            "field '{field}' has wrong type: expected {expected_type}, got {}",
                            value_type_name(val)
                        ),
                        code: "WRONG_TYPE".into(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if a JSON value matches an expected type name.
fn value_matches_type(val: &serde_json::Value, expected: &str) -> bool {
    match expected {
        "string" => val.is_string(),
        "integer" => val.is_i64(),
        "number" => val.is_number(),
        "boolean" => val.is_boolean(),
        "array" => val.is_array(),
        "object" => val.is_object(),
        _ => true,
    }
}

/// Get a human-readable type name for a JSON value.
fn value_type_name(val: &serde_json::Value) -> &'static str {
    match val {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Null => "null",
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
