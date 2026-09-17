//! Runtime validation for wire protocol messages.
//!
//! Parses JSON, checks required fields, validates types, and returns
//! structured errors for invalid messages.

use serde_json::Value;

/// Structured validation error for wire protocol messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
    pub code: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// The result of validating a wire protocol message.
pub type ValidationResult = Result<ValidatedMessage, ValidationError>;

/// A validated wire protocol message with its parsed type and value.
#[derive(Debug, Clone)]
pub struct ValidatedMessage {
    pub msg_type: String,
    pub value: Value,
}

/// Validate a JSON string as a wire protocol message.
///
/// Checks:
/// 1. Valid JSON
/// 2. Is a JSON object
/// 3. Has a `type` field (string)
/// 4. Message type is recognized
/// 5. Required fields are present and non-null
/// 6. Known fields have correct types
pub fn validate_message(json: &str) -> ValidationResult {
    // Parse JSON.
    let value: Value = serde_json::from_str(json).map_err(|e| ValidationError {
        message: format!("invalid JSON: {e}"),
        code: "INVALID_JSON".into(),
    })?;

    // Must be an object.
    let obj = value.as_object().ok_or_else(|| ValidationError {
        message: "message must be a JSON object".into(),
        code: "NOT_OBJECT".into(),
    })?;

    // Must have a "type" field.
    let msg_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError {
            message: "missing required field: type".into(),
            code: "MISSING_TYPE".into(),
        })?;

    // Must be a known type.
    if !crate::schema::is_known_type(msg_type) {
        return Err(ValidationError {
            message: format!("unknown message type: {msg_type}"),
            code: "UNKNOWN_TYPE".into(),
        });
    }

    // Check required fields.
    let required = crate::schema::required_fields(msg_type);
    for field in required {
        match obj.get(*field) {
            None => {
                return Err(ValidationError {
                    message: format!(
                        "missing required field '{field}' for message type '{msg_type}'"
                    ),
                    code: "MISSING_FIELD".into(),
                });
            }
            Some(Value::Null) => {
                return Err(ValidationError {
                    message: format!(
                        "field '{field}' must not be null for message type '{msg_type}'"
                    ),
                    code: "NULL_FIELD".into(),
                });
            }
            _ => {}
        }
    }

    // Validate field types for known fields.
    validate_field_types(obj, msg_type)?;

    Ok(ValidatedMessage {
        msg_type: msg_type.to_string(),
        value,
    })
}

/// Normalize a message type for matching: lowercase + strip underscores.
fn normalize_type(s: &str) -> String {
    s.to_ascii_lowercase().replace('_', "")
}

/// Validate field types for a specific message type (case and underscore insensitive).
fn validate_field_types(
    obj: &serde_json::Map<String, Value>,
    msg_type: &str,
) -> Result<(), ValidationError> {
    match normalize_type(msg_type).as_str() {
        "compilerequest" => {
            validate_string_field(obj, "source", msg_type)?;
            validate_string_field(obj, "destination", msg_type)?;
            validate_optional_string(obj, "intent_name", msg_type)?;
        }
        "webrtcoffer" | "webrtcanswer" => {
            validate_string_field(obj, "target", msg_type)?;
            validate_string_field(obj, "sdp", msg_type)?;
            validate_optional_string(obj, "from", msg_type)?;
        }
        "webrtcice" => {
            validate_string_field(obj, "from", msg_type)?;
            validate_string_field(obj, "candidate", msg_type)?;
            validate_optional_string(obj, "target", msg_type)?;
        }
        "saveconfig" => {
            // config and overrides are optional objects; validate types if present.
            validate_optional_object(obj, "config", msg_type)?;
            validate_optional_object(obj, "overrides", msg_type)?;
        }
        _ => {}
    }
    Ok(())
}

/// Validate that a field is present and is a string.
fn validate_string_field(
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

/// Validate that an optional field, if present, is a string.
fn validate_optional_string(
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

/// Validate that an optional field, if present, is an object.
fn validate_optional_object(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    msg_type: &str,
) -> Result<(), ValidationError> {
    if let Some(val) = obj.get(field) {
        if !val.is_object() {
            return Err(ValidationError {
                message: format!(
                    "field '{field}' must be an object in '{msg_type}', got {}",
                    type_name(val)
                ),
                code: "WRONG_TYPE".into(),
            });
        }
    }
    Ok(())
}

/// Get a human-readable type name for a JSON value.
fn type_name(val: &Value) -> &'static str {
    match val {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => "null",
    }
}

/// Convert a ValidationError into a JSON error response string.
pub fn validation_error_response(err: &ValidationError) -> String {
    format!(
        r#"{{"type":"validation_error","error":"{}","message":"{}"}}"#,
        err.code, err.message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_heartbeat() {
        let result = validate_message(r#"{"type":"heartbeat"}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().msg_type, "heartbeat");
    }

    #[test]
    fn valid_health_check() {
        let result = validate_message(r#"{"type":"health_check"}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().msg_type, "health_check");
    }

    #[test]
    fn valid_topology_request() {
        let result = validate_message(r#"{"type":"topology_request"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_routes_request() {
        let result = validate_message(r#"{"type":"routes_request"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_capabilities_request() {
        let result = validate_message(r#"{"type":"capabilities_request"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_probe_request() {
        let result = validate_message(r#"{"type":"probe_request"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_compile_request() {
        let result =
            validate_message(r#"{"type":"compile_request","source":"a","destination":"b"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_compile_request_with_optional_fields() {
        let result = validate_message(
            r#"{"type":"compile_request","source":"a","destination":"b","intent_name":"my-intent"}"#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn valid_webrtc_offer() {
        let result = validate_message(r#"{"type":"webrtc_offer","target":"n1","sdp":"v=0..."}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_webrtc_answer() {
        let result = validate_message(r#"{"type":"webrtc_answer","target":"n1","sdp":"v=0..."}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn valid_webrtc_ice() {
        let result =
            validate_message(r#"{"type":"webrtc_ice","from":"browser","candidate":"candidate:1"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_json() {
        let result = validate_message("not json at all");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_JSON");
    }

    #[test]
    fn not_object() {
        let result = validate_message(r#""just a string""#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_OBJECT");
    }

    #[test]
    fn missing_type_field() {
        let result = validate_message(r#"{"foo":"bar"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
    }

    #[test]
    fn type_not_string() {
        let result = validate_message(r#"{"type":123}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
    }

    #[test]
    fn unknown_message_type() {
        let result = validate_message(r#"{"type":"unknown_thing"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "UNKNOWN_TYPE");
    }

    #[test]
    fn compile_request_missing_source() {
        let result = validate_message(r#"{"type":"compile_request","destination":"b"}"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "MISSING_FIELD");
        assert!(err.message.contains("source"));
    }

    #[test]
    fn compile_request_missing_destination() {
        let result = validate_message(r#"{"type":"compile_request","source":"a"}"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "MISSING_FIELD");
        assert!(err.message.contains("destination"));
    }

    #[test]
    fn compile_request_wrong_type_source() {
        let result =
            validate_message(r#"{"type":"compile_request","source":123,"destination":"b"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
    }

    #[test]
    fn compile_request_wrong_type_destination() {
        let result =
            validate_message(r#"{"type":"compile_request","source":"a","destination":true}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
    }

    #[test]
    fn compile_request_null_source() {
        let result =
            validate_message(r#"{"type":"compile_request","source":null,"destination":"b"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NULL_FIELD");
    }

    #[test]
    fn webrtc_offer_missing_target() {
        let result = validate_message(r#"{"type":"webrtc_offer","sdp":"v=0..."}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn webrtc_offer_missing_sdp() {
        let result = validate_message(r#"{"type":"webrtc_offer","target":"n1"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn webrtc_offer_wrong_type_target() {
        let result = validate_message(r#"{"type":"webrtc_offer","target":42,"sdp":"v=0..."}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
    }

    #[test]
    fn webrtc_ice_missing_from() {
        let result = validate_message(r#"{"type":"webrtc_ice","candidate":"c1"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn webrtc_ice_missing_candidate() {
        let result = validate_message(r#"{"type":"webrtc_ice","from":"browser"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn validation_error_response_format() {
        let err = ValidationError {
            message: "test error".into(),
            code: "TEST_CODE".into(),
        };
        let resp = validation_error_response(&err);
        assert!(resp.contains("TEST_CODE"));
        assert!(resp.contains("test error"));
        assert!(resp.contains("validation_error"));
    }

    #[test]
    fn case_insensitive_types_still_work() {
        // PascalCase types should also be valid.
        assert!(validate_message(r#"{"type":"Heartbeat"}"#).is_ok());
        assert!(validate_message(r#"{"type":"HealthCheck"}"#).is_ok());
        assert!(validate_message(r#"{"type":"TopologyRequest"}"#).is_ok());
        assert!(validate_message(r#"{"type":"RoutesRequest"}"#).is_ok());
        assert!(validate_message(r#"{"type":"CapabilitiesRequest"}"#).is_ok());
        assert!(validate_message(r#"{"type":"ProbeRequest"}"#).is_ok());
    }

    #[test]
    fn valid_save_config() {
        let result = validate_message(
            r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:5555"}}}"#,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().msg_type, "save_config");
    }

    #[test]
    fn valid_save_config_empty() {
        // save_config has no required fields.
        let result = validate_message(r#"{"type":"save_config"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn save_config_wrong_type_overrides() {
        let result = validate_message(r#"{"type":"save_config","overrides":"not-an-object"}"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
    }
}
