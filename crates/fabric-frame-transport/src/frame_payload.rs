//! Frame payload decoding on the receive path.
//!
//! The `frame_data` wire message carries a header (seq, codec, dimensions,
//! declared `payload_len`) and the body bytes the sender intended for that
//! frame, base64-encoded into the JSON envelope under the key `payload_b64`.
//! Until now the wire server only accounted the frame's declared length; the
//! bytes were never decoded against the codec's shape, so a receiver could
//! not reconstruct the data the sender transmitted. A truncated frame, a
//! frame whose declared length did not match its actual bytes, and a frame
//! whose shape did not match its declared codec all passed the existing
//! gate as long as `payload_len` added up.
//!
//! # What this module owns
//!
//! 1. The codec-specific shape rules: the bytes a `Rgba` frame must carry
//!    (four per pixel, exactly `width * height * 4` bytes), the bytes an
//!    `Nv12` frame must carry (one and a half per pixel, exactly
//!    `width * height * 3 / 2` bytes), and the lower bounds on the encoded
//!    codecs (`Hevc`, `Av1` -- must be non-empty, must not exceed
//!    `MAX_FRAME_SIZE`).
//! 2. The base64 decode of the wire envelope into the raw payload bytes.
//! 3. The reconciliation between the declared `payload_len` and the actual
//!    decoded byte count: a frame whose declared length disagrees with its
//!    actual bytes is not silently accepted on either side of the
//!    disagreement.
//! 4. A typed error enum so the handler can render a distinct wire error
//!    code for each failure mode rather than collapsing them into a single
//!    "decode failed" string.
//!
//! # What this module does not own
//!
//! The handler keeps the ownership gate (`sender_id` against the session's
//! recorded `input_owner`) and the connection-id sequencing. The decoder
//! runs *after* the ownership gate, so a peer that is not the current input
//! owner never gets its payload bytes inspected: a foreign sender is
//! rejected by `input_owner_mismatch` before this module is reached. The
//! state-layer offset (`frames_received` and `last_seq`) is mutated only on
//! a successful decode, so a refused decode leaves the session's accounting
//! byte-identical to what it was before the refused frame.
//!
//! # Why base64 in JSON
//!
//! The daemon's wire server is line-delimited JSON, so the payload bytes
//! must ride inside a JSON value. Base64 is the standard way to carry
//! arbitrary bytes in JSON without escaping concerns; the alternative
//! (length-prefixed binary trailing the JSON line) would change the wire
//! protocol's framing for every message type. Base64 keeps the change
//! scoped to `frame_data` and keeps the rest of the wire untouched.
//!
//! # Reachability proof
//!
//! The decoder is exported and called from
//! `fabric-daemon::wire::frame_handlers::handle_frame_data`. The integration
//! test `crates/fabric-integration-tests/tests/frame_payload_decode_wire.rs`
//! drives the real daemon wire server and asserts the exact bytes round-trip
//! through the live handler. That test fails if the decoder is removed
//! from the handler, because the live handler without the decoder could not
//! reconstruct the bytes the sender transmitted.

use crate::Codec;

/// Bytes the receiver can hand to a downstream consumer.
///
/// The receiver can use `bytes` as the exact byte sequence the sender
/// transmitted for this frame, in order. The daemon validates the shape
/// and the declared length, then accounts the frame. The bytes are
/// returned so a future change (YUV conversion, codec demuxing, keyframe
/// detection) has a place to attach the actual payload without re-parsing
/// the wire envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPayload {
    /// The raw payload bytes the sender transmitted, after base64 decode.
    ///
    /// `bytes.len()` is guaranteed to equal the `payload_len` declared in
    /// the frame header by the time this struct is constructed: any
    /// disagreement has already been rejected as a `PayloadDecodeError`.
    pub bytes: Vec<u8>,
}

/// Failure modes for payload decoding.
///
/// Each variant names the specific reason the decoder refused the frame,
/// so a client can distinguish "your bytes were cut off" from "your bytes
/// were the right count but the wrong shape" from "your codec is fine but
/// you sent an empty encoded frame". The wire renders each variant as a
/// distinct `frame_error.code` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadDecodeError {
    /// The base64-decoded payload was shorter than the declared
    /// `payload_len`. The sender either truncated the bytes (a network
    /// condition) or declared a length it did not actually send. Either
    /// way the frame cannot be reconstructed: the bytes that should have
    /// been at the tail are not present, and silently zero-filling them
    /// would let a sender ship a frame that the receiver pretends to have.
    Truncated {
        /// What the header said the payload length was.
        declared: u32,
        /// What the base64 decode actually produced.
        actual: usize,
    },
    /// The declared `payload_len` differs from the byte count the codec
    /// requires for the declared `width` and `height`. A `Rgba` frame that
    /// does not contain exactly `width * height * 4` bytes cannot be
    /// reconstructed, because the receiver has no way to know which bytes
    /// belong to which pixel row.
    LengthMismatch {
        codec: Codec,
        declared: u32,
        expected: u32,
    },
    /// The codec requires a non-empty encoded body (`Hevc`, `Av1`) and the
    /// sender sent zero bytes. An empty encoded frame is not a valid
    /// bitstream: the codec header alone is not enough to render a frame.
    EmptyPayload { codec: Codec },
    /// The base64 string itself was not valid base64. A sender that
    /// mangles the envelope is not a sender whose bytes can be trusted at
    /// all, so this is reported rather than silently truncated.
    InvalidBase64 { reason: String },
}

impl PayloadDecodeError {
    /// Stable wire code for this failure mode.
    ///
    /// Pinned by the unit tests so a client can switch on the string
    /// without a version bump.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Truncated { .. } => "payload_truncated",
            Self::LengthMismatch { .. } => "payload_length_mismatch",
            Self::EmptyPayload { .. } => "invalid_payload_type",
            Self::InvalidBase64 { .. } => "payload_invalid_base64",
        }
    }

    /// Human-readable message for the wire response.
    pub fn message(&self) -> String {
        match self {
            Self::Truncated { declared, actual } => format!(
                "frame payload was truncated: declared {declared} bytes, \
                 received {actual} bytes after base64 decode"
            ),
            Self::LengthMismatch {
                codec,
                declared,
                expected,
            } => format!(
                "{codec:?} frame at declared width/height requires exactly \
                 {expected} payload bytes, but the sender declared {declared}"
            ),
            Self::EmptyPayload { codec } => format!(
                "{codec:?} frame must carry a non-empty encoded bitstream; \
                 the sender declared an empty payload"
            ),
            Self::InvalidBase64 { reason } => {
                format!("frame payload was not valid base64: {reason}")
            }
        }
    }
}

/// Compute the byte count a frame of `width` x `height` must carry for
/// `codec`.
///
/// Returns `None` for the encoded codecs (`Hevc`, `Av1`), whose payload
/// size is bounded by `MAX_FRAME_SIZE` but not fixed by the dimensions.
/// The decoder uses the `None` case to skip the length check and apply
/// the encoded-codec rules instead.
fn expected_payload_bytes(codec: Codec, width: u32, height: u32) -> Option<u32> {
    let pixels = (width as u64).checked_mul(height as u64)?;
    match codec {
        Codec::Rgba => u32::try_from(pixels.checked_mul(4)?).ok(),
        Codec::Nv12 => u32::try_from(pixels.checked_mul(3)?.checked_div(2)?).ok(),
        Codec::Hevc | Codec::Av1 => None,
    }
}

/// Validate an encoded-codec (`Hevc` / `Av1`) payload.
///
/// An encoded frame must be non-empty (no codec produces a zero-byte
/// bitstream that renders anything) and must not exceed the transport's
/// `MAX_FRAME_SIZE` ceiling. The transport-level ceiling is enforced here
/// rather than in the handler so the same rule applies to every entry
/// point that decodes a payload.
fn validate_encoded_payload(codec: Codec, bytes: &[u8]) -> Result<(), PayloadDecodeError> {
    if bytes.is_empty() {
        return Err(PayloadDecodeError::EmptyPayload { codec });
    }
    if bytes.len() > crate::MAX_FRAME_SIZE {
        // The handler has its own `frame_too_large` check that fires before
        // the decoder, so reaching here is a code bug, not a wire condition.
        // Surface it as a length mismatch anyway: a client that sees this has
        // a sender that lied about `payload_len` (the declared value is
        // what brought the bytes through the `frame_too_large` gate).
        return Err(PayloadDecodeError::LengthMismatch {
            codec,
            declared: bytes.len() as u32,
            expected: crate::MAX_FRAME_SIZE as u32,
        });
    }
    Ok(())
}

/// Decode a frame's payload from the wire envelope.
///
/// `payload_b64` is the base64-encoded bytes the sender put in the
/// `frame_data` JSON. `declared_len` is the `payload_len` the sender
/// declared in the same header. The function base64-decodes, reconciles
/// the declared and actual byte counts, and applies the codec-specific
/// shape check. A successful return is the exact bytes the sender
/// transmitted, in order, in `DecodedPayload::bytes`.
///
/// Refusal is exhaustive: any of the failure modes in
/// [`PayloadDecodeError`] means the frame was not decoded and the caller
/// must not commit any state based on it.
pub fn decode_payload(
    codec: Codec,
    width: u32,
    height: u32,
    declared_len: u32,
    payload_b64: &str,
) -> Result<DecodedPayload, PayloadDecodeError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    let bytes = STANDARD
        .decode(payload_b64)
        .map_err(|e| PayloadDecodeError::InvalidBase64 {
            reason: e.to_string(),
        })?;

    // Truncation: the bytes that arrived were fewer than the header said
    // would arrive. Distinct from `LengthMismatch` because the declared
    // length is plausible for the codec; the truncation is in the
    // transport (network cut, sender crash mid-write).
    if bytes.len() < declared_len as usize {
        return Err(PayloadDecodeError::Truncated {
            declared: declared_len,
            actual: bytes.len(),
        });
    }

    // The declared length is a u32; the base64-decode length is a usize
    // (the platform's address-space size). On a 64-bit host the only way
    // the decoded length exceeds the declared length is if the sender
    // sent more bytes than it said it did. The decoder refuses on either
    // side of the disagreement, because the receiver cannot reconstruct
    // the frame in order if the lengths do not match.
    if bytes.len() > declared_len as usize {
        return Err(PayloadDecodeError::LengthMismatch {
            codec,
            declared: declared_len,
            expected: bytes.len() as u32,
        });
    }

    // Codec-specific shape check. The decoded byte count has already been
    // reconciled with `declared_len`; this block checks the
    // dimensions-derived expectation.
    match expected_payload_bytes(codec, width, height) {
        Some(expected) if expected != declared_len => Err(PayloadDecodeError::LengthMismatch {
            codec,
            declared: declared_len,
            expected,
        }),
        Some(_) => Ok(DecodedPayload { bytes }),
        None => {
            validate_encoded_payload(codec, &bytes)?;
            Ok(DecodedPayload { bytes })
        }
    }
}

#[cfg(test)]
#[path = "frame_payload_tests.rs"]
mod tests;
