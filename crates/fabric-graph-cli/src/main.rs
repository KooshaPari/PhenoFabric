//! `fabric-graph-cli` — thin CLI binary that exposes `fabric_graph::failover::replan()`
//! over a JSON-over-stdin/stdout protocol.
//!
//! Per spec 023 (R2 first wedge, Q1-C decision from `releases/2026-09-08-R1.md`):
//!
//! - `replan` subcommand reads a `ReplanRequest` from `--request <file>` or stdin
//!   and prints a `ReplanResponse` (or `ReplanErrorResponse`) to stdout.
//! - Errors emit `ReplanErrorResponse` on stdout (so callers can parse it
//!   uniformly) and use the numeric exit code from `ReplanErrorCode::exit_code()`.
//! - Exit code 0 = success (Replaced or NoReplacement). Exit code > 0 = error.
//!
//! Invocation:
//!   fabric-graph-cli replan [--request <file>]
//!   echo '{...}' | fabric-graph-cli replan
//!
//! See `specs/023-fabric-graph-cli-replan/spec.md` for the wire contract.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use fabric_graph_cli::protocol::{
    replan as protocol_replan, ReplanErrorCode, ReplanErrorResponse, ReplanRequest, ReplanResponse,
};

/// Hand-rolled subcommand dispatcher — no `clap` dependency to keep the
/// binary tree tiny and the build deterministic. Matches spec 023 §2.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("replan") => run_replan(&args[2..]),
        Some("--help") | Some("-h") => {
            print_usage(&args[0]);
            ExitCode::SUCCESS
        }
        // No subcommand or unknown subcommand → usage error (exit 1).
        None => {
            eprintln!("error: missing subcommand");
            print_usage(&args[0]);
            ExitCode::from(1)
        }
        Some(other) => {
            eprintln!("error: unknown subcommand: {other}");
            print_usage(&args[0]);
            ExitCode::from(1)
        }
    }
}

fn print_usage(prog: &str) {
    println!("Usage:");  // stdout per spec 023 §5
    println!("  {prog} replan [--request <file>]");
    println!("  {prog} --help");
    println!();
    println!("Subcommands:");
    println!("  replan    Read a ReplanRequest from --request <file> or stdin,");
    println!("           invoke fabric_graph::failover::replan(), print the");
    println!("           ReplanResponse (or ReplanErrorResponse) to stdout.");
    println!();
    println!("Exit codes:");
    println!("  0   Replaced or NoReplacement");
    println!("  1   Internal error");
    println!("  10  EmptyIntent");
    println!("  11  AllCandidatesFailed");
    println!("  20  InvalidRequest");
    println!("  21  MissingRequestFile");
    println!("  22  StdinReadFailed");
}

fn run_replan(args: &[String]) -> ExitCode {
    // Parse `--request <file>` (optional). If absent, read from stdin.
    let request_path: Option<PathBuf> = match parse_request_flag(args) {
        Ok(p) => p,
        Err(msg) => {
            emit_error(ReplanErrorCode::InvalidRequest, msg);
            return ExitCode::from(ReplanErrorCode::InvalidRequest.exit_code() as u8);
        }
    };

    // Read JSON bytes.
    let json_bytes = match read_request(request_path.as_ref()) {
        Ok(b) => b,
        Err(code) => {
            emit_error(code, code_exit_msg(code));
            return ExitCode::from(code.exit_code() as u8);
        }
    };

    // Deserialize the request.
    let req: ReplanRequest = match serde_json::from_slice(&json_bytes) {
        Ok(r) => r,
        Err(e) => {
            emit_error(ReplanErrorCode::InvalidRequest, format!("invalid JSON: {e}"));
            return ExitCode::from(ReplanErrorCode::InvalidRequest.exit_code() as u8);
        }
    };

    // Invoke the protocol-layer replan.
    match protocol_replan(&req) {
        Ok(resp) => {
            match serde_json::to_string(&resp) {
                Ok(s) => {
                    println!("{s}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    emit_error(
                        ReplanErrorCode::Internal,
                        format!("failed to serialize response: {e}"),
                    );
                    ExitCode::from(ReplanErrorCode::Internal.exit_code() as u8)
                }
            }
        }
        Err(e) => {
            let resp = e.to_response();
            // Print the structured error to stdout (callers parse stdout uniformly).
            if let Ok(s) = serde_json::to_string(&resp) {
                println!("{s}");
            }
            emit_error(resp.code, resp.message);
            ExitCode::from(resp.code.exit_code() as u8)
        }
    }
}

/// Parse `--request <file>` from args. Returns `Ok(None)` if not present
/// (caller should read stdin). Returns `Err(msg)` for usage errors.
fn parse_request_flag(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "--request" {
            let path = iter
                .next()
                .ok_or_else(|| String::from("--request requires a file path argument"))?;
            return Ok(Some(PathBuf::from(path)));
        }
        if let Some(rest) = a.strip_prefix("--request=") {
            return Ok(Some(PathBuf::from(rest)));
        }
        // Unknown flag — surface as usage error so callers don't silently ignore it.
        if a.starts_with("--") || a.starts_with('-') {
            return Err(format!("unknown flag: {a}"));
        }
    }
    Ok(None)
}

/// Read the request JSON from `--request <file>` or stdin.
fn read_request(path: Option<&PathBuf>) -> Result<Vec<u8>, ReplanErrorCode> {
    match path {
        Some(p) => std::fs::read(p).map_err(|_| ReplanErrorCode::MissingRequestFile),
        None => {
            let mut buf = Vec::new();
            io::stdin()
                .read_to_end(&mut buf)
                .map_err(|_| ReplanErrorCode::StdinReadFailed)?;
            Ok(buf)
        }
    }
}

/// Emit a structured error to stdout AND a human-readable line to stderr.
/// Callers should parse stdout for the structured error; stderr is for humans.
fn emit_error(code: ReplanErrorCode, message: String) {
    let resp = ReplanErrorResponse { code, message: message.clone() };
    if let Ok(s) = serde_json::to_string(&resp) {
        println!("{s}");
    } else {
        // Fallback: at least surface the message to stderr.
        eprintln!("[{code:?}] {message}");
    }
    eprintln!("[{code:?}] {message}");
}

fn code_exit_msg(code: ReplanErrorCode) -> String {
    match code {
        ReplanErrorCode::EmptyIntent => String::from("intent has no requirements — nothing to replan onto"),
        ReplanErrorCode::AllCandidatesFailed => String::from("all candidate nodes are blacklisted"),
        ReplanErrorCode::InvalidRequest => String::from("invalid request JSON"),
        ReplanErrorCode::MissingRequestFile => String::from("could not read --request file"),
        ReplanErrorCode::StdinReadFailed => String::from("could not read stdin to EOF"),
        ReplanErrorCode::Internal => String::from("internal error"),
    }
}
