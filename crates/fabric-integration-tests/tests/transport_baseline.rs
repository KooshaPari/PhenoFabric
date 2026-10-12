//! Empirical baseline for the existing TCP frame-transport path.
//!
//! This is a *characterization* harness, not a feature test. It stands up the
//! smallest stub TCP listener that speaks the daemon's line-delimited JSON wire
//! protocol (one JSON ack line per request, `TCP_NODELAY` set the way
//! `run_wire_server` sets it) and drives the real
//! [`stream_frames_between`] harness against it at 1280x720 for N = 100, 1000,
//! and 10000 frames.
//!
//! It deliberately does NOT reimplement the streamer; it only supplies the
//! server end the streamer expects, so the client request/response loop, JSON
//! encoding, and TCP timing all run through the real code paths.
//!
//! Output: one `BASELINE {json}` line per (run, N). The JSON carries
//! `frames_sent`, `frames_acknowledged`, `avg_rtt_us`, `elapsed_ms`, and
//! `throughput_fps`. External tooling aggregates the min/median/max.
//!
//! Run:
//! `BASELINE_RUN=<i> cargo test -p fabric-integration-tests \
//!     --test transport_baseline -- --test-threads=1 --nocapture`

use fabric_frame_transport::Codec;
use fabric_integration_tests::harness::frame_streamer::stream_frames_between;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};

/// Sizes required by the baseline task.
const SIZES: [u32; 3] = [100, 1000, 10000];
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

/// A running stub wire server plus its bound address.
struct StubServer {
    addr: String,
    _handle: JoinHandle<()>,
}

/// Start a stub listener on `127.0.0.1:0` that serves connections sequentially
/// until the test process exits.
fn start_stub_server() -> StubServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub listener");
    let addr = listener.local_addr().expect("stub local_addr").to_string();
    let handle = thread::spawn(move || {
        for connection in listener.incoming() {
            match connection {
                Ok(stream) => serve_connection(stream),
                Err(_) => break,
            }
        }
    });
    StubServer {
        addr,
        _handle: handle,
    }
}

/// Serve one client connection: read JSON lines, reply one JSON ack line each.
///
/// Mirrors the daemon wire server in the two ways this baseline depends on:
/// line-delimited JSON request/response, and `TCP_NODELAY` on the accepted
/// socket. The reply *content* is an ack rather than the daemon's current
/// `unknown_message` error: the streamer only requires a line per request to
/// keep its read/write loop synchronized, and a valid ack keeps the exchange a
/// well-formed request/response stream.
fn serve_connection(stream: TcpStream) {
    // Match run_wire_server: request/response framing, small writes.
    let _ = stream.set_nodelay(true);

    let mut writer = stream;
    let mut reader = BufReader::new(writer.try_clone().expect("clone stream"));

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // client closed the connection
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let framed = format!("{}\n", ack_for(trimmed));
                if writer.write_all(framed.as_bytes()).is_err() {
                    break;
                }
                if writer.flush().is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Build the JSON ack line for an incoming wire message.
fn ack_for(message: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => return r#"{"type":"error","error":"invalid_json"}"#.to_string(),
    };
    match parsed.get("type").and_then(|v| v.as_str()) {
        Some("session_init") => r#"{"type":"session_ack","status":"ok"}"#.to_string(),
        Some("frame_data") => {
            let seq = parsed
                .get("seq")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            format!(r#"{{"type":"frame_ack","seq":{seq},"status":"ok"}}"#)
        }
        Some(other) => {
            format!(r#"{{"type":"error","error":"unknown_message","type":"{other}"}}"#)
        }
        None => r#"{"type":"error","error":"missing_type"}"#.to_string(),
    }
}

#[test]
fn transport_baseline_three_sizes() {
    let stub = start_stub_server();
    let run = std::env::var("BASELINE_RUN").unwrap_or_else(|_| "0".to_string());

    for n in SIZES {
        let result =
            stream_frames_between(&stub.addr, "baseline-client", WIDTH, HEIGHT, n, Codec::Hevc)
                .expect("stream_frames_between");

        let elapsed_s = result.elapsed.as_secs_f64();
        let throughput_fps = if elapsed_s > 0.0 {
            f64::from(result.frames_sent) / elapsed_s
        } else {
            0.0
        };

        let row = serde_json::json!({
            "run": run.as_str(),
            "n": n,
            "width": WIDTH,
            "height": HEIGHT,
            "frames_sent": result.frames_sent,
            "frames_acknowledged": result.frames_acknowledged,
            "avg_rtt_us": result.avg_rtt_us,
            "elapsed_ms": elapsed_s * 1000.0,
            "throughput_fps": throughput_fps,
        });

        assert_eq!(result.frames_sent, n, "all frames sent for n={n}");
        assert_eq!(
            result.frames_acknowledged, n,
            "all frames acknowledged for n={n}"
        );

        println!("BASELINE {row}");
    }
}
