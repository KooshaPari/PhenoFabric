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
pub(super) fn normalize_type(s: &str) -> String {
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
        "surfaceleaserequest" => {
            validate_string_field(obj, "name", msg_type)?;
            validate_string_field(obj, "protocol", msg_type)?;
            validate_string_field(obj, "locality_floor", msg_type)?;
            validate_optional_bool(obj, "requires_rt", msg_type)?;
        }
        // --- Frame plane ---
        // Implemented in `crate::frame_validation` so this file stays under the
        // 500-line ceiling. Same contract, same error codes.
        _ => return crate::frame_validation::validate_frame_fields(obj, msg_type),
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

/// Validate that an optional field, if present, is a boolean.
fn validate_optional_bool(
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
pub(super) fn type_name(val: &Value) -> &'static str {
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
#[path = "validation_tests.rs"]
mod tests;
