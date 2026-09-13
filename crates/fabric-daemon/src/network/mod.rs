//! Network utilities for Phenotype Fabric.
//!
//! Provides UPnP port forwarding, Tailscale mesh networking, and STUN NAT
//! traversal for establishing peer-to-peer connections.

pub mod stun;
pub mod tailscale;
pub mod upnp;
