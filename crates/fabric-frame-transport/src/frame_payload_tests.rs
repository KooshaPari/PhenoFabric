//! Unit tests for the frame payload decoder.
//!
//! These tests cover the pure decode function in isolation -- the wire
//! integration test (`crates/fabric-integration-tests/tests/frame_payload_decode_wire.rs`)
//! drives the same decoder through the real daemon over a real TCP socket.
//! The split keeps the failure modes (truncation, length mismatch, invalid
//! payload type, malformed envelope) testable without standing up a daemon.

use super::*;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

fn b64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn rgba_frame(width: u32, height: u32) -> Vec<u8> {
    vec![0u8; (width * height * 4) as usize]
}

fn nv12_frame(width: u32, height: u32) -> Vec<u8> {
    vec![0u8; (width * height * 3 / 2) as usize]
}

// ===========================================================================
// Success paths
// ===========================================================================

/// A `Rgba` frame whose declared `payload_len` equals its actual bytes
/// and equals `width * height * 4` decodes cleanly and the returned
/// bytes match what the sender transmitted.
#[test]
fn decode_rgba_frame_round_trip() {
    let width = 64u32;
    let height = 48u32;
    let raw = rgba_frame(width, height);
    // Stamp a recognizable pattern at index 0 and at the tail so a
    // round-trip that returns the same bytes is provable.
    let mut raw = raw;
    raw[0] = 0xAB;
    let last = raw.len() - 1;
    raw[last] = 0xCD;
    let declared = raw.len() as u32;

    let decoded = decode_payload(Codec::Rgba, width, height, declared, &b64(&raw)).expect("decode");
    assert_eq!(decoded.bytes, raw);
    assert_eq!(decoded.bytes.len(), declared as usize);
    assert_eq!(decoded.bytes[0], 0xAB);
    assert_eq!(decoded.bytes[decoded.bytes.len() - 1], 0xCD);
}

/// A `Nv12` frame whose declared `payload_len` equals its actual bytes
/// and equals `width * height * 3 / 2` decodes cleanly.
#[test]
fn decode_nv12_frame_round_trip() {
    let width = 64u32;
    let height = 48u32;
    let raw = nv12_frame(width, height);
    let declared = raw.len() as u32;

    let decoded = decode_payload(Codec::Nv12, width, height, declared, &b64(&raw)).expect("decode");
    assert_eq!(decoded.bytes.len(), declared as usize);
}

/// An `Av1` frame with a non-empty payload decodes cleanly, regardless
/// of dimensions (encoded codecs do not have a fixed pixel-byte mapping).
#[test]
fn decode_av1_non_empty_payload() {
    let raw = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let declared = raw.len() as u32;
    let decoded = decode_payload(Codec::Av1, 1920, 1080, declared, &b64(&raw)).expect("decode");
    assert_eq!(decoded.bytes, raw);
}

/// An `Hevc` frame with a non-empty payload decodes cleanly.
#[test]
fn decode_hevc_non_empty_payload() {
    let raw = vec![0xAB; 1024];
    let declared = raw.len() as u32;
    let decoded = decode_payload(Codec::Hevc, 1920, 1080, declared, &b64(&raw)).expect("decode");
    assert_eq!(decoded.bytes.len(), declared as usize);
}

// ===========================================================================
// Failure mode 1: truncated payload
// ===========================================================================

/// The base64-decoded payload is shorter than the declared `payload_len`.
/// This is the "truncated" failure mode: the sender declared a length it
/// did not actually send (or the transport cut the bytes). The decoder
/// refuses with `payload_truncated` and the bytes are NOT silently
/// zero-filled.
#[test]
fn truncated_payload_is_rejected() {
    let width = 64u32;
    let height = 48u32;
    let full = rgba_frame(width, height);
    let declared = full.len() as u32;
    // Send only the first 3/4 of the bytes.
    let truncated = &full[..full.len() * 3 / 4];

    let err = decode_payload(Codec::Rgba, width, height, declared, &b64(truncated))
        .expect_err("must refuse truncated frame");
    assert_eq!(err.code(), "payload_truncated");
    match &err {
        PayloadDecodeError::Truncated { declared, actual } => {
            assert_eq!(*declared, full.len() as u32);
            assert_eq!(*actual, truncated.len());
        }
        other => panic!("expected Truncated, got {other:?}"),
    }
    // The human-readable message names both numbers, so a client can
    // diagnose without re-parsing the wire.
    let msg = err.message();
    assert!(msg.contains(&full.len().to_string()));
    assert!(msg.contains(&truncated.len().to_string()));
}

// ===========================================================================
// Failure mode 2: declared length disagrees with actual length
// ===========================================================================

/// The sender's `payload_len` is `width * height * 4 - 1` (one byte
/// short) for a `Rgba` frame. The base64 decode succeeds and produces
/// `width * height * 4 - 1` bytes, but the codec requires
/// `width * height * 4` exactly. The decoder refuses with
/// `payload_length_mismatch` -- silently zero-filling or truncating the
/// last byte would let a sender ship a frame the receiver pretends to
/// have reconstructed.
#[test]
fn rgba_length_mismatch_is_rejected() {
    let width = 64u32;
    let height = 48u32;
    let raw = rgba_frame(width, height);
    let expected = raw.len() as u32;
    let declared = expected - 1; // Sender claims one byte short.

    let err = decode_payload(Codec::Rgba, width, height, declared, &b64(&raw))
        .expect_err("must refuse length mismatch");
    assert_eq!(err.code(), "payload_length_mismatch");
    match &err {
        PayloadDecodeError::LengthMismatch {
            codec,
            declared,
            expected: reported_expected,
        } => {
            assert_eq!(*codec, Codec::Rgba);
            assert_eq!(*declared, expected - 1);
            assert_eq!(*reported_expected, expected);
        }
        other => panic!("expected LengthMismatch, got {other:?}"),
    }
}

/// `Nv12` frames require exactly `width * height * 3 / 2` bytes. A
/// declared length that does not match the codec-derived expectation is
/// refused with `payload_length_mismatch`, not silently coerced. The
/// sender declared one byte fewer than the codec requires, so the bytes
/// the sender actually sent exceed the declared length (the "too many
/// bytes" side of the length disagreement) and the decoder refuses
/// before the codec-shape check has a chance to apply.
#[test]
fn nv12_length_mismatch_is_rejected() {
    let width = 64u32;
    let height = 48u32;
    let raw = nv12_frame(width, height);
    let expected = raw.len() as u32;
    let declared = expected - 1; // Sender claims one byte fewer than required.

    let err = decode_payload(Codec::Nv12, width, height, declared, &b64(&raw))
        .expect_err("must refuse length mismatch");
    assert_eq!(err.code(), "payload_length_mismatch");
    match &err {
        PayloadDecodeError::LengthMismatch { codec, .. } => assert_eq!(*codec, Codec::Nv12),
        other => panic!("expected LengthMismatch, got {other:?}"),
    }
}

/// A `Rgba` frame whose declared `payload_len` is `width * height * 4 +
/// 1` is refused: the base64 decode succeeds, the declared length is
/// shorter than the bytes that arrived, so the decoder refuses on the
/// "too many bytes" side of the length disagreement.
#[test]
fn rgba_payload_exceeds_declared_is_rejected() {
    let width = 64u32;
    let height = 48u32;
    let raw = rgba_frame(width, height);
    let declared = (raw.len() as u32) - 1; // Sender claims one byte fewer than they sent.

    let err = decode_payload(Codec::Rgba, width, height, declared, &b64(&raw))
        .expect_err("must refuse over-length");
    // Over-length is the "you sent more than you declared" side of the
    // same length disagreement, and lands as a `LengthMismatch` because
    // the codec-derived expected is `width * height * 4` and the declared
    // is one byte short of that.
    assert_eq!(err.code(), "payload_length_mismatch");
}

// ===========================================================================
// Failure mode 3: invalid payload type
// ===========================================================================

/// An `Av1` frame with an empty payload is refused with
/// `invalid_payload_type`. An empty encoded bitstream is not a valid
/// frame for any encoder, so the decoder rejects it as a payload-type
/// problem rather than a length problem.
#[test]
fn av1_empty_payload_is_invalid_payload_type() {
    let raw: &[u8] = &[];
    let declared: u32 = 0;
    let err = decode_payload(Codec::Av1, 1920, 1080, declared, &b64(raw))
        .expect_err("must refuse empty encoded");
    assert_eq!(err.code(), "invalid_payload_type");
    match &err {
        PayloadDecodeError::EmptyPayload { codec } => assert_eq!(*codec, Codec::Av1),
        other => panic!("expected EmptyPayload, got {other:?}"),
    }
}

/// An `Hevc` frame with an empty payload is also `invalid_payload_type`.
#[test]
fn hevc_empty_payload_is_invalid_payload_type() {
    let raw: &[u8] = &[];
    let declared: u32 = 0;
    let err = decode_payload(Codec::Hevc, 1920, 1080, declared, &b64(raw))
        .expect_err("must refuse empty encoded");
    assert_eq!(err.code(), "invalid_payload_type");
    match &err {
        PayloadDecodeError::EmptyPayload { codec } => assert_eq!(*codec, Codec::Hevc),
        other => panic!("expected EmptyPayload, got {other:?}"),
    }
}

/// The base64 string is not valid base64 at all. The decoder refuses
/// with `payload_invalid_base64` so a sender that mangles the envelope
/// is reported rather than silently truncated.
#[test]
fn invalid_base64_is_rejected() {
    let err = decode_payload(Codec::Rgba, 64, 48, 12288, "not!base64!!!")
        .expect_err("must refuse non-base64");
    assert_eq!(err.code(), "payload_invalid_base64");
    assert!(matches!(err, PayloadDecodeError::InvalidBase64 { .. }));
}

// ===========================================================================
// Decoded byte count always equals declared
// ===========================================================================

/// On success, `DecodedPayload::bytes.len()` always equals the declared
/// `payload_len`. This is the round-trip property: the receiver
/// reconstructs exactly the bytes the sender declared, no more, no less.
#[test]
fn success_bytes_len_equals_declared() {
    let cases: Vec<(u32, u32, Codec, Vec<u8>)> = vec![
        (64, 48, Codec::Rgba, rgba_frame(64, 48)),
        (32, 32, Codec::Nv12, nv12_frame(32, 32)),
        (16, 16, Codec::Av1, vec![0u8; 100]),
        (16, 16, Codec::Hevc, vec![0u8; 100]),
    ];

    for (w, h, codec, raw) in cases {
        let declared = raw.len() as u32;
        let decoded = decode_payload(codec, w, h, declared, &b64(&raw))
            .unwrap_or_else(|e| panic!("decode for {codec:?} failed: {e:?}"));
        assert_eq!(
            decoded.bytes.len(),
            declared as usize,
            "{codec:?} bytes.len() != declared"
        );
        assert_eq!(decoded.bytes, raw, "{codec:?} bytes differ from raw");
    }
}

// ===========================================================================
// Stable error codes (wire contract)
// ===========================================================================

/// The wire error codes are stable. Pinned here so a client can switch
/// on the string without a version bump, and a refactor that renames a
/// code is caught immediately rather than leaking into the integration
/// tests.
#[test]
fn error_codes_are_stable() {
    assert_eq!(
        PayloadDecodeError::Truncated {
            declared: 100,
            actual: 50
        }
        .code(),
        "payload_truncated"
    );
    assert_eq!(
        PayloadDecodeError::LengthMismatch {
            codec: Codec::Rgba,
            declared: 100,
            expected: 128
        }
        .code(),
        "payload_length_mismatch"
    );
    assert_eq!(
        PayloadDecodeError::EmptyPayload { codec: Codec::Av1 }.code(),
        "invalid_payload_type"
    );
    assert_eq!(
        PayloadDecodeError::InvalidBase64 {
            reason: "x".to_string()
        }
        .code(),
        "payload_invalid_base64"
    );
}
