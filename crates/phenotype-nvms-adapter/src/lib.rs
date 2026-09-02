//! `phenotype-nvms-adapter` — bridge between `odin.nvms` v0.2 application
//! manifests and Phenotype Fabric `CapabilityDescriptor`s.
//!
//! ## What this crate is
//!
//! `odin.nvms` v0.2 (defined in `phenotype-manifest`) is an *application*
//! manifest: it says "I want a deployment with 2 CPU cores, 512 MiB of
//! memory, network ports 80/443, and OTel collector X." It is not a
//! *machine* descriptor — it does not describe the host that will run
//! the application.
//!
//! Phenotype Fabric's `CapabilityDescriptor` is the opposite: it says
//! "this host has CPU X, NUMA Y, GPU Z, display W." It is what the
//! route compiler reads to decide where to place something.
//!
//! This adapter bridges the two:
//!
//! 1. **Manifest → Required Capabilities**: Given an `odin.nvms` manifest,
//!    produce a Fabric `Capabilities` block that *describes the minimum
//!    machine capability needed to satisfy it*. Useful for the route
//!    compiler to filter candidate hosts.
//! 2. **Manifest + Descriptor → Bound Manifest**: Given a manifest
//!    AND a target host descriptor, produce a `BoundManifest` that says
//!    "this manifest is bound to this host, signed by the manifest
//!    author, witnessed by the host." This is the artifact the
//!    deployment runtime consumes.
//!
//! ## Out of scope (R0.5)
//!
//! - Actual placement / scheduling (PF-WP-020, R1)
//! - Pulling live capabilities from the host (use `capability-probe`
//!   for that; this crate just maps manifests)
//! - Reverse mapping (host → manifest). The host's descriptor is the
//!   ground truth; the manifest is a wish.

#![forbid(unsafe_code)]

mod bound;
mod required;

pub use bound::{BoundManifest, BoundManifestBuilder, BoundManifestError};
pub use required::{
    required_capabilities, RequiredCapabilities, RequiredCapabilitiesError,
};

/// Re-export the upstream `phenotype_manifest` types so downstream
/// consumers don't need to add a second dependency just to construct
/// a manifest.
pub use phenotype_manifest;
