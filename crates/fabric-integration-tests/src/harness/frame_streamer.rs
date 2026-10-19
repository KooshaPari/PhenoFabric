//! Frame streaming between a client and a real daemon over the real TCP stack.
//!
//! Opens a std TCP connection to the target daemon's wire server address and
//! drives the frame protocol (`session_init` / `frame_data` / `frame_ack`)
//! end-to-end as line-delimited JSON, which is the framing the daemon wire
//! server speaks (`fabric-daemon/src/wire/mod.rs`, `reader.lines()`).
//!
//! # What changed when the frame dispatcher landed
//!
//! Previously every frame message came back as an `UNKNOWN_TYPE` validation
//! error: the types were absent from `REQUEST_MESSAGE_TYPES`, so
//! `validate_message` rejected them before the dispatcher was consulted.
//! `fabric-daemon/src/wire/frame_handlers.rs` now handles them, so this
//! harness gets real responses: `session_ack`, `frame_ack`, and
//! `frame_ack_response`. The response types are asserted here rather than
//! discarded, because an ignored response is exactly what hid this gap.

use anyhow::{bail, Context, Result};
use fabric_frame_transport::{Codec, PROTOCOL_VERSION};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// StreamResult
// ---------------------------------------------------------------------------

/// Summary of a simulated streaming session.
#[derive(Debug)]
pub struct StreamResult {
    /// Server-assigned session id, learned from the `session_ack`.
    pub session_id: u64,
    /// Number of frames sent.
    pub frames_sent: u32,
    /// Number of frames the daemon acknowledged with `status: "ok"`.
    pub frames_acknowledged: u32,
    /// Average round-trip time across all acknowledged frames (microseconds).
    pub avg_rtt_us: u64,
    /// Total wall-clock duration of the session.
    pub elapsed: Duration,
}

// ---------------------------------------------------------------------------
// Wire helpers
// ---------------------------------------------------------------------------

/// Send one JSON line and read exactly one JSON response line.
fn exchange(
    writer: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    message: &serde_json::Value,
) -> Result<serde_json::Value> {
    let line = serde_json::to_string(message).context("serialize frame wire message")?;
    writer
        .write_all(line.as_bytes())
        .context("write frame wire message")?;
    writer.write_all(b"\n").context("write newline")?;
    writer.flush().context("flush frame wire message")?;

    let mut response = String::new();
    let read = reader
        .read_line(&mut response)
        .context("read frame wire response")?;
    if read == 0 {
        bail!("daemon closed the connection before responding to {line}");
    }
    serde_json::from_str(response.trim())
        .with_context(|| format!("daemon response was not JSON: {}", response.trim()))
}

/// The transport variant name for a `Codec`, as the wire represents it.
///
/// `Codec` derives `Serialize`/`Deserialize` (`fabric-frame-transport/src/lib.rs`),
/// so the wire form is the Rust variant name (`"Hevc"`, `"Av1"`, `"Nv12"`,
/// `"Rgba"`). Using the serde name rather than a display name keeps the JSON
/// the daemon reads identical to what the transport itself would emit.
fn codec_name(codec: Codec) -> &'static str {
    match codec {
        Codec::Hevc => "Hevc",
        Codec::Av1 => "Av1",
        Codec::Nv12 => "Nv12",
        Codec::Rgba => "Rgba",
    }
}

// ---------------------------------------------------------------------------
// FrameBody
// ---------------------------------------------------------------------------

/// Whether [`stream_frames_between`] embeds the frame body on the wire.
///
/// The wire server's decoder requires a body whose decoded length matches the
/// declared `payload_len`, so any peer running the production handler needs
/// [`FrameBody::Embedded`]. A stub peer that only counts headers takes
/// [`FrameBody::Omitted`]; embedding a full body there adds no coverage and
/// turns a framing-throughput measurement into a base64-cost measurement.
///
/// Base64 inflates the body by roughly 4/3, so a 1280x720 RGBA frame is
/// ~4.9 MB on the wire. A baseline that streams thousands of such frames in a
/// debug build runs for hours; that is why omission is explicit rather than
/// assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBody {
    /// Embed the synthetic body, base64-encoded, so the production decoder
    /// accepts the frame.
    Embedded,
    /// Send the header only. Valid **only** against a stub that ignores the
    /// body; the production handler rejects this as `payload_truncated`.
    Omitted,
}

// ---------------------------------------------------------------------------
// stream_frames_between
// ---------------------------------------------------------------------------

/// Stream `frame_count` frames against the daemon at `server_addr`.
///
/// Drives a real session: `session_init` opens a session and the daemon
/// returns the `session_id`; each `frame_data` carries that id and must come
/// back with `status: "ok"`; then each `frame_ack` reports the measured RTT.
///
/// # Arguments
///
/// * `server_addr` -- target daemon TCP address (e.g. `"127.0.0.1:54321"`).
/// * `client_id` -- identifier for this streaming client.
/// * `width`, `height` -- frame dimensions in pixels.
/// * `frame_count` -- number of frames to stream.
/// * `codec` -- codec to negotiate in SessionInit.
/// * `body` -- whether to embed the frame body on the wire; see [`FrameBody`].
///
/// # Errors
///
/// Returns an error if any message gets a `frame_error` response, or if the
/// daemon answers `session_init` without a usable `session_id`. A frame for an
/// unknown session is reported, never silently ignored.
pub fn stream_frames_between(
    server_addr: &str,
    client_id: &str,
    width: u32,
    height: u32,
    frame_count: u32,
    codec: Codec,
    body: FrameBody,
) -> Result<StreamResult> {
    let stream =
        TcpStream::connect(server_addr).with_context(|| format!("connect to {server_addr}"))?;
    // Disable Nagle, matching the daemon's wire server. Both directions must
    // opt out: with Nagle left on, a small framing write waits for an ACK that
    // the peer's delayed-ACK timer holds for ~40 ms, which dominates the RTT
    // this harness measures.
    stream
        .set_nodelay(true)
        .context("set TCP_NODELAY on frame stream")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("set read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("set write timeout")?;

    let reader_stream = stream.try_clone().context("clone stream for reader")?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    // --- Session handshake ---
    let session_init = serde_json::json!({
        "type": "session_init",
        "version": PROTOCOL_VERSION,
        "preferred_codec": codec_name(codec),
        "width": width,
        "height": height,
        "target_fps": 60,
        "max_latency_ms": 33,
        "client_id": client_id,
    });
    let ack = exchange(&mut writer, &mut reader, &session_init)?;
    if ack["type"] != "session_ack" {
        bail!("expected session_ack, got: {ack}");
    }
    let session_id = ack["session_id"]
        .as_u64()
        .with_context(|| format!("session_ack carried no session_id: {ack}"))?;

    // --- Stream frames ---
    // The synthetic payload is built once so `payload_len` below is a real
    // measurement of the bytes this frame stands for, not arithmetic.
    let frame_payload = make_synthetic_payload(width, height);
    let frame_payload_len = frame_payload.len();
    let mut frames_acknowledged: u32 = 0;
    let mut total_rtt_us: u64 = 0;
    let session_start = Instant::now();

    for seq in 0..frame_count {
        let pts_us = seq as u64 * 16_667; // ~60fps

        let send_time = Instant::now();

        // The frame header travels as JSON, mirroring
        // `fabric_frame_transport::FrameHeader` field for field. Whether the
        // body travels with it is the caller's explicit choice (`FrameBody`):
        // the production wire server decodes the body, while the baseline's
        // stub peer only counts headers.
        //
        // `sender_id` is the wire protocol's ownership gate: the daemon
        // refuses `frame_data` whose declared sender is not the session's
        // current input owner. Using the same `client_id` we sent in
        // `session_init` means the frame is accepted by the default owner;
        // ownership hand-off tests open a fresh session and then issue a
        // `session_transfer` before sending their frames under the new
        // owner.
        let mut frame_msg = serde_json::json!({
            "type": "frame_data",
            "session_id": session_id,
            "seq": seq,
            "pts_us": pts_us,
            "dts_us": pts_us,
            "is_keyframe": seq == 0,
            "codec": codec_name(codec),
            "width": width,
            "height": height,
            "payload_len": frame_payload_len,
            "duration_us": 16_667,
            "sender_id": client_id,
        });
        // Embedded only when the peer runs the production decoder, which
        // refuses a `frame_data` with non-zero `payload_len` and no body. The
        // body's decoded length must equal `payload_len` exactly; a stub peer
        // takes `Omitted` so it does not pay base64 cost per frame.
        if body == FrameBody::Embedded {
            frame_msg["payload_b64"] = serde_json::Value::String(base64_encode(&frame_payload));
        }
        let frame_ack = exchange(&mut writer, &mut reader, &frame_msg)?;
        if frame_ack["type"] != "frame_ack" {
            bail!("expected frame_ack for seq {seq}, got: {frame_ack}");
        }
        if frame_ack["status"] != "ok" {
            bail!("daemon did not accept frame {seq}: {frame_ack}");
        }

        let rtt = send_time.elapsed().as_micros() as u64;
        total_rtt_us += rtt;
        frames_acknowledged += 1;

        // --- Acknowledge the frame back ---
        //
        // The ack carries the same `sender_id` as the frame that
        // produced it: the input owner is the one entity that may emit
        // both directions of the per-frame exchange.
        let ack_msg = serde_json::json!({
            "type": "frame_ack",
            "session_id": session_id,
            "seq": seq,
            "rtt_us": rtt,
            "recv_pts_us": pts_us,
            "sender_id": client_id,
        });
        let ack_response = exchange(&mut writer, &mut reader, &ack_msg)?;
        if ack_response["type"] != "frame_ack_response" {
            bail!("expected frame_ack_response for seq {seq}, got: {ack_response}");
        }
    }

    let elapsed = session_start.elapsed();
    let avg_rtt_us = if frames_acknowledged > 0 {
        total_rtt_us / frames_acknowledged as u64
    } else {
        0
    };

    Ok(StreamResult {
        session_id,
        frames_sent: frame_count,
        frames_acknowledged,
        avg_rtt_us,
        elapsed,
    })
}

// ---------------------------------------------------------------------------
// Synthetic payload generator
// ---------------------------------------------------------------------------

/// Create a small synthetic RGBA payload for testing.
///
/// Uses a fixed pattern (gradient ramp) that is fast to generate and
/// deterministic for assertion purposes.
fn make_synthetic_payload(width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        data.push(((i * 4) & 0xFF) as u8);
        data.push(((i * 4 + 1) & 0xFF) as u8);
        data.push(((i * 4 + 2) & 0xFF) as u8);
        data.push(255u8);
    }
    data
}

/// Standard-alphabet base64 encoder.
///
/// Local copy of the RFC 4648 §4 encoder so the harness does not
/// pull in the `base64` crate for one call. Only used on the test
/// fixture body; the wire server's decoder is the production-grade
/// impl, tested separately in `fabric-frame-transport::frame_payload`.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    // Index-driven rather than `chunks_exact(3)`: newer clippy releases flag
    // `chunks_exact` with a constant size and suggest `as_chunks`, which is
    // still unstable. A plain index walk is stable on every toolchain and
    // makes the tail handling explicit.
    let mut i = 0;
    while i + 2 < input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        0 => {}
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => unreachable!(),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_payload_size() {
        let payload = make_synthetic_payload(1920, 1080);
        assert_eq!(payload.len(), 1920 * 1080 * 4);
    }

    #[test]
    fn stream_result_default_fields() {
        let result = StreamResult {
            session_id: 0,
            frames_sent: 0,
            frames_acknowledged: 0,
            avg_rtt_us: 0,
            elapsed: Duration::ZERO,
        };
        assert_eq!(result.frames_sent, 0);
    }

    #[test]
    fn base64_encode_matches_rfc4648_vectors() {
        // Section 10 test vectors, covering all three padding cases.
        // A wrong tail here would corrupt every `FrameBody::Embedded` frame,
        // so the encoder is pinned directly rather than only through the
        // decoder's acceptance of large synthetic bodies.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_encode_handles_a_full_frame_body() {
        // The real fixture path: 64x48 RGBA is 12,288 bytes, a multiple of
        // three, so it must encode with no padding at all.
        let body = make_synthetic_payload(64, 48);
        assert_eq!(body.len(), 12_288);
        let encoded = base64_encode(&body);
        assert_eq!(encoded.len(), 16_384);
        assert!(
            !encoded.contains('='),
            "a 3-byte-aligned body needs no padding"
        );
    }

    #[test]
    fn codec_names_match_the_transport_serde_representation() {
        // The wire carries the serde variant name, which is what the frame
        // handler's `parse_codec` accepts. Asserting this pins the contract:
        // if the enum were renamed, these tests would fail rather than the
        // daemon silently rejecting every frame.
        assert_eq!(codec_name(Codec::Hevc), "Hevc");
        assert_eq!(codec_name(Codec::Av1), "Av1");
        assert_eq!(codec_name(Codec::Nv12), "Nv12");
        assert_eq!(codec_name(Codec::Rgba), "Rgba");
    }
}
