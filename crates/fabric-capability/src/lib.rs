//! `fabric-capability` — Fabric capability discovery, descriptor signing, and topology probing.
//!
//! # Overview
//!
//! This crate provides:
//!
//! - **Capability descriptor types** ([`CapabilityDescriptor`]) — a signed, versioned,
//!   self-describing inventory of a node's hardware and software capabilities.
//! - **Probe trait** ([`Probe`]) — a pluggable interface for discovering capabilities
//!   on a given platform.
//! - **Signing and verification** ([`sign`], [`verify`], [`SigningKey`]) — Ed25519 signing
//!   of descriptor deltas so that a malicious peer cannot inject false capabilities.
//! - **Schema validation** ([`validate_descriptor`] ) — validates a descriptor against
//!   the canonical JSON schema.
//!
//! # Locality tiers
//!
//! Fabric categorizes resources by their locality to the current process:
//!
//! | Tier | Meaning |
//! |:----:|:--------|
//! | L0 | Same NUMA node, same process |
//! | L1 | Same NUMA node, different process |
//! | L2 | Cross-NUMA, same host, shared memory |
//! | L3 | Cross-NUMA, same host, PCIe P2P |
//! | L4 | Same host, RDMA |
//! | L5 | Loopback (127.0.0.1 / ::1) |
//! | L6 | LAN (same subnet) |
//! | L7 | WAN (different subnet) |
//! | L8 | Out-of-band (IPMI, KVM-over-IP, WoL) |
//!
//! # Example
//!
//! ```ignore
//! use fabric_capability::{CapabilityDescriptor, SigningKey, sign};
//!
//! let descriptor = CapabilityDescriptor::probe()?;
//! let key = SigningKey::generate();
//! let signed = sign(&descriptor, &key)?;
//! assert!(verify(&signed).is_ok());
//! ```

pub mod descriptor;
pub mod error;
pub mod locality;
pub mod probe;
pub mod schema;
pub mod signing;
pub mod topology;
pub mod trust_root;

pub use descriptor::CapabilityDescriptor;
pub use error::Error;
pub use locality::LocalityTier;
pub use probe::Probe;
pub use schema::validate_descriptor;
pub use signing::{sign, verify, SigningKey, VerificationKey};
pub use trust_root::{
    Authority, ChainVerification, RevocationEntry, RevocationList, RevocationReason, TrustError,
    TrustStore, MAX_CHAIN_DEPTH,
};
