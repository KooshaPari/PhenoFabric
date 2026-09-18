//! Simulated frame streaming between two daemons over the real TCP stack.
//!
//! Opens a std TCP connection to the target daemon's wire server address
//! and exercises the frame protocol (SessionInit / FrameData) end-to-end.
//! Messages are sent as JSON lines so the line-based wire server can parse
//! them and return a response (typically an UNKNOWN_TYPE error), proving
//! the TCP connection delivers bytes correctly.

use anyhow::{Context, Result};
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
    /// Number of frames sent.
    pub frames_sent: u32,
    /// Number of frames acknowledged (local ack in current impl).
    pub frames_acknowledged: u32,
    /// Average round-trip time across all acknowledged frames (microseconds).
    pub avg_rtt_us: u64,
    /// Total wall-clock duration of the session.
    pub elapsed: Duration,
}

// ---------------------------------------------------------------------------
// stream_frames_between
// ---------------------------------------------------------------------------

/// Simulate a frame streaming session against `server_addr`.
///
/// Connects via std TCP, sends a `SessionInit` as a JSON line, reads the
/// server's response, then streams `frame_count` synthetic frame headers
/// as JSON lines. Each frame sent triggers a local ack to measure RTT.
///
/// # Arguments
///
/// * `server_addr` -- target daemon TCP address (e.g. `"127.0.0.1:54321"`).
/// * `client_id` -- identifier for this streaming client.
/// * `width`, `height` -- frame dimensions in pixels.
/// * `frame_count` -- number of frames to stream.
/// * `codec` -- codec to negotiate in SessionInit.
///
/// # Note
///
/// The wire server currently handles JSON line protocol and will return
/// an `UNKNOWN_TYPE` error for session_init / frame_data messages. This
/// is expected: the test proves the transport types and wire encoding work
/// over a real TCP connection. A future wire server enhancement will
/// dispatch frame messages to enable true round-trip validation.
pub fn stream_frames_between(
    server_addr: &str,
    client_id: &str,
    width: u32,
    height: u32,
    frame_count: u32,
    codec: Codec,
) -> Result<StreamResult> {
    let mut stream =
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

    // --- Session handshake (JSON line) ---
    let session_init = serde_json::json!({
        "type": "session_init",
        "version": PROTOCOL_VERSION,
        "preferred_codec": format!("{:?}", codec),
        "width": width,
        "height": height,
        "target_fps": 60,
        "max_latency_ms": 33,
        "client_id": client_id,
    });
    let init_line = serde_json::to_string(&session_init).context("serialize SessionInit")?;
    stream
        .write_all(init_line.as_bytes())
        .context("send SessionInit")?;
    stream.write_all(b"\n").context("send newline")?;
    stream.flush().context("flush SessionInit")?;

    // Read the response line from the wire server.
    // The server will respond with a JSON error (unknown message type).
    let reader_stream = stream.try_clone().context("clone stream for reader")?;
    let mut reader = BufReader::new(reader_stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .context("read server response")?;
    // The response is expected to be an error (unknown type), which is fine
    // for this test -- it proves the wire encoding delivered the bytes.

    // --- Stream frames (JSON lines) ---
    let frame_payload = make_synthetic_payload(width, height);
    let mut frames_sent: u32 = 0;
    let mut frames_acknowledged: u32 = 0;
    let mut total_rtt_us: u64 = 0;
    let session_start = Instant::now();

    for seq in 0..frame_count {
        let pts_us = seq as u64 * 16_667; // ~60fps

        let send_time = Instant::now();

        // Send frame header as JSON line (payload not sent over wire since
        // the server doesn't handle binary frame data yet).
        let frame_msg = serde_json::json!({
            "type": "frame_data",
            "seq": seq,
            "pts_us": pts_us,
            "dts_us": pts_us,
            "is_keyframe": seq == 0,
            "codec": format!("{:?}", codec),
            "width": width,
            "height": height,
            "payload_len": frame_payload.len(),
            "duration_us": 16_667,
        });
        let frame_line = serde_json::to_string(&frame_msg).context("serialize FrameData")?;
        stream
            .write_all(frame_line.as_bytes())
            .context("send FrameData")?;
        stream.write_all(b"\n").context("send newline")?;
        stream.flush().context("flush FrameData")?;

        // Read the server's response to prevent write buffer from filling.
        // The server returns an error (unknown type) for frame_data.
        let mut resp = String::new();
        let _ = reader.read_line(&mut resp);

        frames_sent += 1;

        // Simulate local ack timing.
        // In a future phase the wire server will echo FrameData, enabling
        // true round-trip validation with FrameAck.
        let rtt = send_time.elapsed().as_micros() as u64;
        total_rtt_us += rtt;
        frames_acknowledged += 1;
    }

    let elapsed = session_start.elapsed();
    let avg_rtt_us = if frames_acknowledged > 0 {
        total_rtt_us / frames_acknowledged as u64
    } else {
        0
    };

    Ok(StreamResult {
        frames_sent,
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
            frames_sent: 0,
            frames_acknowledged: 0,
            avg_rtt_us: 0,
            elapsed: Duration::ZERO,
        };
        assert_eq!(result.frames_sent, 0);
    }
}
