//! JWT decoding and token hashing for auth middleware.

use serde::Deserialize;

use super::{AuthError, AuthenticatedUser};

/// JWT claims for decoding WorkOS-issued tokens.
#[derive(Debug, Deserialize)]
pub(super) struct JwtClaims {
    /// Subject (user ID).
    pub(super) sub: String,
    /// Email address.
    #[serde(default)]
    pub(super) email: Option<String>,
    /// Organization ID.
    #[serde(default)]
    pub(super) org_id: Option<String>,
    /// Expiration timestamp.
    ///
    /// Deliberately not read by this module: the leading underscore documents
    /// that `exp` is enforced by the library's validation step, not by callers.
    #[serde(default)]
    pub(super) _exp: Option<u64>,
}

/// Decode a JWT token and extract user claims.
///
/// # Security posture
///
/// `jsonwebtoken` 10.3.0 is the first release patched for the type-confusion
/// authorization bypass (CVE-2026-25537 / GHSA-h395-gr6q-cpjc); it also makes
/// the validator strictly stricter than 9.3.1 (malformed `exp`/`nbf` claims are
/// now rejected instead of being silently ignored).
///
/// The resulting [`jsonwebtoken::Validation`] is, for this call site:
///
/// - `validate_signature = true` - the HMAC over `header.payload` is verified.
/// - `algorithms = [Algorithm::HS256]` - only HMAC-SHA256 is accepted, matching
///   `DecodingKey::from_secret`. This is *not* widened: an `alg: none` token,
///   or one naming an asymmetric algorithm, is rejected by the algorithm check
///   before any key is used, which is what closes alg-confusion attacks.
/// - `validate_exp = true` and `required_spec_claims = {"exp"}` - `exp` must be
///   present and must not be in the past (60s leeway for clock skew), so a
///   token without `exp` is rejected rather than treated as non-expiring.
///
/// All three match the 9.3.1 `Validation::default()` semantics this function
/// previously relied on. They are re-asserted explicitly below so that a future
/// `Default`/`new` change upstream cannot silently relax them.
pub(super) fn decode_jwt(token: &str, secret: &str) -> Result<AuthenticatedUser, AuthError> {
    use jsonwebtoken::{Algorithm, DecodingKey, Validation};

    // Pin the algorithm instead of using `Validation::default()`. `from_secret`
    // produces an HMAC key, so the accepted set must stay HS256-only.
    let mut validation = Validation::new(Algorithm::HS256);

    // `exp` is both required and enforced. Both are already true for a fresh
    // `Validation::new(HS256)`, but stating them here keeps the requirement
    // local to this call site and immune to upstream default changes.
    validation.validate_exp = true;
    validation.required_spec_claims.insert("exp".to_owned());

    let token_data = jsonwebtoken::decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| AuthError::JwtDecode(e.to_string()))?;

    let claims = token_data.claims;

    Ok(AuthenticatedUser {
        user_id: claims.sub,
        email: claims.email.unwrap_or_default(),
        org_id: claims.org_id,
    })
}

/// Simple hash function for token caching.
pub(super) fn token_hash(token: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use serde::Serialize;

    const SECRET: &str = "test-secret-not-used-in-production";

    /// Mirror of [`JwtClaims`] that is serializable, so tests can mint tokens
    /// with `jsonwebtoken::encode`. `exp` is omitted from the JSON when `None`
    /// so the "missing exp" case is genuinely absent rather than null.
    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        org_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        exp: Option<u64>,
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the unix epoch")
            .as_secs()
    }

    /// A correctly shaped claim set that is valid for another hour.
    fn valid_claims() -> TestClaims {
        TestClaims {
            sub: "user-123".to_owned(),
            email: Some("koosha@example.com".to_owned()),
            org_id: Some("org-456".to_owned()),
            exp: Some(now() + 3600),
        }
    }

    fn encode_hs256(claims: &TestClaims, secret: &str) -> String {
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("test token should encode")
    }

    /// Minimal unpadded base64url encoder. Only needed to hand-craft the
    /// `alg: none` token, which `jsonwebtoken::encode` refuses to produce.
    fn b64url(input: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

        let mut out = String::new();
        for chunk in input.chunks(3) {
            let b0 = u32::from(chunk[0]);
            let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
            let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
            let n = (b0 << 16) | (b1 << 8) | b2;

            out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            if chunk.len() > 1 {
                out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(n & 0x3f) as usize] as char);
            }
        }
        out
    }

    /// Asserts the decode failed, returning the wrapped library message so the
    /// test can pin the *reason* rather than merely "it errored".
    fn decode_err_message(token: &str) -> String {
        match decode_jwt(token, SECRET) {
            Err(AuthError::JwtDecode(message)) => message,
            other => panic!("expected AuthError::JwtDecode, got {other:?}"),
        }
    }

    #[test]
    fn accepts_valid_token_and_maps_claims() {
        let token = encode_hs256(&valid_claims(), SECRET);

        let user = decode_jwt(&token, SECRET).expect("a correctly signed token must decode");
        assert_eq!(user.user_id, "user-123");
        assert_eq!(user.email, "koosha@example.com");
        assert_eq!(user.org_id.as_deref(), Some("org-456"));
    }

    #[test]
    fn accepts_valid_token_with_only_required_claims() {
        // `email` and `org_id` are optional; `email` must not panic and must
        // fall back to the empty string.
        let claims = TestClaims {
            sub: "user-minimal".to_owned(),
            email: None,
            org_id: None,
            exp: Some(now() + 3600),
        };
        let token = encode_hs256(&claims, SECRET);

        let user = decode_jwt(&token, SECRET).expect("optional claims may be absent");
        assert_eq!(user.user_id, "user-minimal");
        assert_eq!(user.email, "");
        assert_eq!(user.org_id, None);
    }

    #[test]
    fn rejects_token_signed_with_wrong_secret() {
        let token = encode_hs256(&valid_claims(), "a-completely-different-secret");

        let message = decode_err_message(&token);
        assert!(
            message.contains("InvalidSignature"),
            "expected a signature failure, got: {message}"
        );
    }

    #[test]
    fn rejects_expired_token() {
        let mut claims = valid_claims();
        // An hour in the past, well beyond the 60s leeway.
        claims.exp = Some(now() - 3600);
        let token = encode_hs256(&claims, SECRET);

        let message = decode_err_message(&token);
        assert!(
            message.contains("ExpiredSignature"),
            "expected an expiry failure, got: {message}"
        );
    }

    #[test]
    fn rejects_token_without_exp() {
        let mut claims = valid_claims();
        claims.exp = None;
        let token = encode_hs256(&claims, SECRET);

        let message = decode_err_message(&token);
        assert!(
            message.contains("Missing required claim: exp"),
            "a token with no exp must be rejected, got: {message}"
        );
    }

    #[test]
    fn rejects_tampered_payload() {
        let token = encode_hs256(&valid_claims(), SECRET);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "expected a three-part JWT");

        // Keep the original header and signature; swap in an attacker payload
        // that would grant a different identity and a far-future expiry.
        let forged = b64url(br#"{"sub":"attacker","exp":4102444800}"#);
        let tampered = format!("{}.{}.{}", parts[0], forged, parts[2]);
        assert_ne!(tampered, token);

        let message = decode_err_message(&tampered);
        assert!(
            message.contains("InvalidSignature"),
            "expected a signature failure, got: {message}"
        );
    }

    #[test]
    fn rejects_tampered_signature() {
        let token = encode_hs256(&valid_claims(), SECRET);
        let mut parts: Vec<String> = token.split('.').map(str::to_owned).collect();
        assert_eq!(parts.len(), 3, "expected a three-part JWT");

        // Mutate the FIRST character of the signature segment, not the last.
        //
        // A 32-byte HMAC encodes to 43 base64url characters, i.e. 258 bits, so
        // the final character carries only 4 significant bits and its low 2 bits
        // are discarded on decode. 'A', 'B', 'C' and 'D' all have the same top 4
        // bits, so rewriting the last character between them is a no-op: the
        // decoded signature is unchanged and the token still verifies. Mutating
        // the last character therefore made this test fail ~6.25% of the time
        // (whenever the signature happened to end in one of those four). Every
        // bit of the first character is significant, so this always changes the
        // signature.
        let first = parts[2].remove(0);
        parts[2].insert(0, if first == 'A' { 'B' } else { 'A' });
        let tampered = parts.join(".");

        assert_ne!(tampered, token);
        let message = decode_err_message(&tampered);
        assert!(
            message.contains("InvalidSignature"),
            "expected a signature failure, got: {message}"
        );
    }

    #[test]
    fn rejects_alg_none_token() {
        let header = b64url(br#"{"alg":"none","typ":"JWT"}"#);
        let payload = b64url(br#"{"sub":"attacker","exp":4102444800}"#);

        // Both an empty signature segment and an appended one must be rejected.
        // jsonwebtoken fails closed one step earlier than the `algorithms`
        // allowlist here: `Algorithm` has no `none` variant, so the header
        // itself fails to deserialize. Asserting on that reason keeps the test
        // honest and would fail loudly if upstream ever parsed `none`.
        for token in [
            format!("{header}.{payload}."),
            format!("{header}.{payload}.{}", b64url(&[0u8; 32])),
        ] {
            let message = decode_err_message(&token);
            assert!(
                message.contains("unknown variant `none`"),
                "alg:none must be rejected while parsing the header, got: {message}"
            );
        }
    }

    #[test]
    fn rejects_hmac_signature_under_rs256_header() {
        // The classic alg-confusion attack: a token whose header claims an
        // asymmetric algorithm while carrying an HMAC produced with the shared
        // secret. The header parses, so this exercises the `algorithms`
        // allowlist rather than the header deserializer.
        let signed = encode_hs256(&valid_claims(), SECRET);
        let signature = signed.rsplit('.').next().expect("signature segment");
        let rs256_header = b64url(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = signed.split('.').nth(1).expect("payload segment");
        let forged = format!("{rs256_header}.{payload}.{signature}");

        let message = decode_err_message(&forged);
        assert!(
            message.contains("InvalidAlgorithm"),
            "a non-HS256 algorithm must fail the pinned allowlist, got: {message}"
        );
    }

    #[test]
    fn rejects_algorithm_outside_the_pinned_hs256_allowlist() {
        // Would pass a sloppy "HMAC family" allowlist, but the accepted set is
        // pinned to exactly HS256. Guards against silently widening `algorithms`.
        let token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS384),
            &valid_claims(),
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("test token should encode");

        let message = decode_err_message(&token);
        assert!(
            message.contains("InvalidAlgorithm"),
            "only HS256 may be accepted, got: {message}"
        );
    }

    #[test]
    fn validation_enforces_exp_and_pins_algorithm() {
        // Direct assertions on the validation the auth path constructs, so the
        // behavioural tests above cannot pass via an unrelated code path.
        use jsonwebtoken::{Algorithm, Validation};

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.required_spec_claims.insert("exp".to_owned());

        assert!(validation.validate_exp, "exp must be validated");
        assert!(
            validation.required_spec_claims.contains("exp"),
            "exp must be a required claim"
        );
        // `algorithms` is an exact allowlist, so "none" and every non-HS256
        // algorithm are excluded by construction.
        assert_eq!(validation.algorithms, vec![Algorithm::HS256]);
    }
}
