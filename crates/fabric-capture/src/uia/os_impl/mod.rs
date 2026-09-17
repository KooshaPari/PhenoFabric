pub(crate) mod capture;
pub(crate) mod extract;
pub(crate) mod tabs;

// Re-export everything needed by the public API.
pub(crate) use capture::{capture_via_uia, enumerate_and_capture};
pub(crate) use extract::diag;
pub(crate) use tabs::capture_all_tabs_via_uia;
