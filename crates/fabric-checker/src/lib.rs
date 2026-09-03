// fabric-checker — public API surface
//
// PF-WP-011: cross-check a Fabric `CapabilityDescriptor` (live machine probe)
// against an `odin.nvms` v0.2 `BoundManifest` (declared application) and emit
// a structured `Decision` with one or more `CheckOutcome` reasons.
//
// Two consumers:
//   * Rust services that link the crate (e.g. fabric-workspace in the future)
//   * The Go reference adapter at `cmd/checker/main.go` — that Go file re-derives
//     the same decision semantics from the JSON output for cross-language
//     parity testing.

pub mod decision;
pub mod checks;
pub mod checker;

pub use decision::{CheckOutcome, Decision, ReasonCode, Severity};
pub use checks::{CheckContext, CheckFn, CheckRegistry};
pub use checker::{Checker, CheckerConfig};
