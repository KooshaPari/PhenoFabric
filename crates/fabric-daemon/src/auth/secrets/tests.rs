//! Tests for the sibling `secrets` module. See `super` for the code.
use super::*;

#[test]
fn default_config_is_empty() {
    let config = InfisicalConfig::default();
    assert!(config.client_id.is_empty());
    assert!(config.client_secret.is_empty());
    assert!(config.project_id.is_empty());
    assert_eq!(config.base_url, DEFAULT_INFISICAL_BASE_URL);
}

#[test]
fn default_config_uses_cli_documented_us_cloud_api_host() {
    let config = InfisicalConfig::default();
    assert_eq!(config.base_url, "https://app.infisical.com");
}

#[test]
fn secret_value_serialization_roundtrip() {
    let sv = SecretValue {
        key: "DB_PASSWORD".into(),
        value: "s3cret".into(),
        environment: "prod".into(),
        path: Some("/database".into()),
    };

    let json = serde_json::to_string(&sv).unwrap();
    let deserialized: SecretValue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.key, "DB_PASSWORD");
    assert_eq!(deserialized.value, "s3cret");
    assert_eq!(deserialized.environment, "prod");
}

#[test]
fn secret_value_optional_path() {
    let json = r#"{"key":"K","value":"V","environment":"dev"}"#;
    let sv: SecretValue = serde_json::from_str(json).unwrap();
    assert!(sv.path.is_none());
}

/// Official docs (observed 2026-09-27,
/// https://infisical.com/docs/api-reference/endpoints/universal-auth/login)
/// return `accessToken`/`expiresIn`/`tokenType`; the snake_case fixture
/// shape no longer describes the live contract.
#[test]
fn token_response_matches_documented_camelcase_shape() {
    let docs_json =
        r#"{"accessToken":"tok","expiresIn":300,"accessTokenMaxTTL":600,"tokenType":"Bearer"}"#;
    let parsed: TokenResponse =
        serde_json::from_str(docs_json).expect("documented shape must parse");
    assert_eq!(parsed.access_token, "tok");
    assert_eq!(parsed.expires_in, 300);
    assert_eq!(parsed._token_type, "Bearer");
}
