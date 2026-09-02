//! JSON Schema validation for capability descriptors.
//!
//! Uses the `jsonschema` crate for runtime validation. A fuller schema
//! check is done in CI via `check_json_schemas.py`.

use crate::error::{Error, Result};
use jsonschema::validator_for;
use serde::Serialize;

/// Validates a capability descriptor against a minimal canonical JSON schema.
///
/// A fuller schema check is done in CI via `check_json_schemas.py`.
pub fn validate_descriptor<T: Serialize>(value: &T) -> Result<()> {
    let minimal_schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["node_id", "epoch", "schema_version", "probed_at",
                     "probe_version", "topology_hash", "capabilities", "signatures"],
        "properties": {
            "node_id": { "type": "string" },
            "epoch": { "type": "number", "minimum": 0 },
            "schema_version": { "type": "string" },
            "probed_at": { "type": "string" },
            "probe_version": { "type": "string" },
            "topology_hash": { "type": "string" },
            "capabilities": { "type": "object" },
            "signatures": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["key_id", "alg", "sig", "signed_at"]
                }
            }
        }
    });

    let compiled = validator_for(&minimal_schema)
        .map_err(|e| Error::Schema(e.to_string()))?;

    let value_json = serde_json::to_value(value)
        .map_err(|e| Error::Serde(e.to_string()))?;

    if let Err(e) = compiled.validate(&value_json) {
        // jsonschema 0.27 returns a single ValidationError
        let msg = format!("{}: {}", e.instance_path, e);
        return Err(Error::Schema(msg));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{Capabilities, CapabilityDescriptor};
    use uuid::Uuid;

    #[test]
    fn validate_valid_descriptor() {
        let d = CapabilityDescriptor {
            node_id: Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".into(),
            probed_at: chrono::Utc::now(),
            probe_version: "0.1.0".into(),
            topology_hash: "test".into(),
            capabilities: Capabilities::default(),
            signatures: vec![],
        };
        assert!(validate_descriptor(&d).is_ok());
    }

    #[test]
    fn validate_rejects_missing_fields() {
        let bad = serde_json::json!({});
        assert!(validate_descriptor(&bad).is_err());
    }
}
