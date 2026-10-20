//! Shared support code for the `c_link_smoke` cross-language link test.
//!
//! The test itself lives in `tests/c_link_smoke.rs`. This module tree holds
//! the pieces it depends on, split by concern so each file stays small and
//! each concern is independently testable:
//!
//! * [`ansi`] — removal of ANSI SGR escape sequences from compiler output.
//! * [`flags`] — discovery and filtering of the native system libraries a
//!   Rust `staticlib` needs.
//! * [`marker`] — recognition of the `native-static-libs:` marker in rustc
//!   output.
//! * [`toolchain`] — locating a C compiler and building the staticlib.
//! * [`c_program`] — the C source that the test compiles and links.
//!
//! # Why the split
//!
//! The test used to be one 748-line file that mixed the C source, the
//! toolchain plumbing, the output parsing, and the tests for that parsing.
//! That is over the project's 500-line hard limit, and it made the parsing
//! rules hard to reach without also reaching the C source.

pub mod ansi;
pub mod c_program;
pub mod flags;
pub mod marker;
pub mod toolchain;
