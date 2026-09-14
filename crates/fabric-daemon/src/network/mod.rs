//! Network utilities for Phenotype Fabric.
//!
//! Provides UPnP port forwarding, Tailscale mesh networking, and STUN NAT
//! traversal for establishing peer-to-peer connections.

pub mod stun;
pub mod tailscale;
pub mod upnp;

use serde::{Deserialize, Serialize};

/// Errors that can occur in network operations.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// UPnP discovery failed.
    #[error("UPnP discovery failed: {message}")]
    UpnpDiscovery { message: String },

    /// UPnP SOAP request failed.
    #[error("UPnP SOAP {action} failed: {message}")]
    UpnpSoap { action: String, message: String },

    /// UDP socket bind failure.
    #[error("UDP bind failed: {source}")]
    UdpBind {
        #[source]
        source: std::io::Error,
    },

    /// UDP send failure.
    #[error("UDP send failed: {source}")]
    UdpSend {
        #[source]
        source: std::io::Error,
    },

    /// UDP receive failure.
    #[error("UDP recv failed: {source}")]
    UdpRecv {
        #[source]
        source: std::io::Error,
    },

    /// SSDP discovery timed out.
    #[error("SSDP discovery timed out")]
    DiscoveryTimeout,

    /// STUN server DNS resolution failed.
    #[error("STUN DNS resolution failed for {server}: {source}")]
    StunDns {
        server: String,
        #[source]
        source: std::io::Error,
    },

    /// STUN transaction timed out.
    #[error("STUN timeout for server {server}")]
    StunTimeout { server: String },

    /// STUN response could not be parsed.
    #[error("STUN parse error: {message}")]
    StunParse { message: String },

    /// STUN TCP connection failed.
    #[error("STUN TCP connection to {server} failed: {source}")]
    StunTcp {
        server: String,
        #[source]
        source: std::io::Error,
    },

    /// No STUN servers responded.
    #[error("No STUN servers responded (tried {servers:?})")]
    StunNoServers { servers: Vec<String> },

    /// Tailscale CLI error.
    #[error("Tailscale CLI error: {message}")]
    TailscaleCli { message: String },
}

/// Configuration for the network module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Whether UPnP port forwarding is enabled.
    pub upnp_enabled: bool,
    /// STUN servers to query (empty = use defaults).
    pub stun_servers: Vec<String>,
    /// Path to the Tailscale CLI binary.
    pub tailscale_path: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            upnp_enabled: true,
            stun_servers: Vec::new(),
            tailscale_path: "tailscale".into(),
        }
    }
}
