//! `fabric-graph-cli` — thin binary that exposes `fabric_graph::failover::replan()`
//! over a JSON-over-stdin / stdout protocol.
//!
//! This is the Q1-C deliverable from `releases/2026-09-08-R1.md`: a Rust binary
//! that calls `failover::replan()` and prints JSON, designed to be invoked by
//! `cmd/checker` (Go) via `os/exec` so the Go side can consume failover results
//! without cgo or a duplicate algorithm.
//!
//! # Layout
//!
//! - [`protocol`] — request/response/error types and the [`replan`] pure function.
//! - `main` (in `main.rs`) — CLI arg parsing + JSON I/O + exit code mapping.
//!
//! See `specs/023-fabric-graph-cli-replan` for the wire contract.

pub mod protocol;
