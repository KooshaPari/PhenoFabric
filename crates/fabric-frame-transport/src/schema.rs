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
    "auth_start",
    "auth_complete",
    "auth_email",
    "auth_status",
    "auth_verify",
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
    ("auth_start", &[]),
    ("auth_complete", &["code"]),
    ("auth_email", &["email"]),
    ("auth_status", &[]),
    ("auth_verify", &["email", "code"]),
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
    ("code", "string", "auth_complete"),
    ("code_verifier", "string", "auth_complete"),
    ("email", "string", "auth_email"),
    ("email", "string", "auth_verify"),
    ("code", "string", "auth_verify"),
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
mod tests {
    use super::*;

    #[test]
    fn schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(WIRE_PROTOCOL_SCHEMA)
            .expect("WIRE_PROTOCOL_SCHEMA must be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn validate_empty_object_fails() {
        let result = validate_against_schema(r#"{}"#, "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
    }

    #[test]
    fn validate_non_object_fails() {
        let result = validate_against_schema(r#""hello""#, "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_OBJECT");
    }

    #[test]
    fn validate_invalid_json_fails() {
        let result = validate_against_schema("not json", "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_JSON");
    }

    #[test]
    fn validate_unknown_type_fails() {
        let result = validate_against_schema(r#"{"type":"foo"}"#, "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "UNKNOWN_TYPE");
    }

    #[test]
    fn validate_heartbeat_passes() {
        let result = validate_against_schema(r#"{"type":"heartbeat"}"#, "wire");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_health_check_passes() {
        let result = validate_against_schema(r#"{"type":"health_check"}"#, "wire");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_compile_request_missing_source_fails() {
        let result =
            validate_against_schema(r#"{"type":"compile_request","destination":"b"}"#, "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn validate_compile_request_missing_destination_fails() {
        let result = validate_against_schema(r#"{"type":"compile_request","source":"a"}"#, "wire");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn validate_compile_request_passes() {
        let result = validate_against_schema(
            r#"{"type":"compile_request","source":"a","destination":"b"}"#,
            "wire",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_compile_request_wrong_type_fails() {
        let result = validate_against_schema(
            r#"{"type":"compile_request","source":123,"destination":"b"}"#,
            "wire",
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
    }

    #[test]
    fn validate_webrtc_offer_passes() {
        let result = validate_against_schema(
            r#"{"type":"webrtc_offer","target":"n1","sdp":"v=0..."}"#,
            "wire",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_webrtc_offer_missing_target_fails() {
        let result = validate_against_schema(r#"{"type":"webrtc_offer","sdp":"v=0..."}"#, "wire");
        assert!(result.is_err());
    }

    #[test]
    fn validate_compile_request_null_source_fails() {
        let result = validate_against_schema(
            r#"{"type":"compile_request","source":null,"destination":"b"}"#,
            "wire",
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NULL_FIELD");
    }

    #[test]
    fn validate_save_config_passes() {
        let result = validate_against_schema(
            r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:5555"}}}"#,
            "wire",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_save_config_empty_passes() {
        // save_config has no required fields.
        let result = validate_against_schema(r#"{"type":"save_config"}"#, "wire");
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod auth_types_tests {
    use super::*;

    #[test]
    fn auth_start_is_known_and_requires_nothing() {
        for t in ["auth_start", "AuthStart"] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn auth_complete_requires_code() {
        assert!(is_known_type("auth_complete"));
        assert_eq!(required_fields("auth_complete"), &["code"]);
    }

    #[test]
    fn auth_email_requires_email() {
        assert!(is_known_type("auth_email"));
        assert_eq!(required_fields("auth_email"), &["email"]);
    }

    #[test]
    fn auth_status_is_known_and_requires_nothing() {
        for t in ["auth_status", "AuthStatus"] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn auth_verify_requires_email_and_code() {
        assert!(is_known_type("auth_verify"));
        assert_eq!(required_fields("auth_verify"), &["email", "code"]);
    }

    #[test]
    fn unknown_still_rejected() {
        assert!(!is_known_type("auth_destroy"));
        assert!(!is_known_type("definitely_not_a_type"));
    }
}
