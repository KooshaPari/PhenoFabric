//! Topology graph utilities.
//!
//! In R0, the topology graph is implicit: each `CapabilityDescriptor` knows
//! about itself, and the `topology` section contains edges to adjacent
//! nodes (peers discovered via mDNS, a static config, or a peer-list file).
//!
//! In R1+ (PF-WP-020), the topology becomes a fully synthesized graph with
//! a path planner.

use crate::descriptor::{CapabilityDescriptor, LinkMetrics, TopologyEdge, TopologyCapabilities};
use crate::error::Result;
use crate::locality::LocalityTier;
use uuid::Uuid;

/// Builds a `TopologyCapabilities` with a single self-loop edge.
///
/// Useful for testing and for the very first probe where there are no
/// known peers yet.
pub fn self_loop(node_id: Uuid, locality: LocalityTier) -> TopologyCapabilities {
    TopologyCapabilities {
        edges: vec![TopologyEdge {
            target_node_id: node_id,
            locality_tier: locality,
            link_metrics: LinkMetrics {
                rtt_us: crate::descriptor::Percentiles {
                    p50: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                    max: 0.0,
                },
                jitter_us: None,
                loss_rate: 0.0,
                bandwidth_mbps: crate::descriptor::Percentiles {
                    p50: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                    max: 0.0,
                },
                copy_paths: vec![],
            },
        }],
    }
}

/// Attaches topology to a descriptor and returns the updated descriptor.
pub fn attach_topology(
    mut descriptor: CapabilityDescriptor,
    topology: TopologyCapabilities,
) -> Result<CapabilityDescriptor> {
    descriptor.capabilities.topology = Some(topology);
    descriptor.topology_hash = descriptor.topology_hash();
    Ok(descriptor)
}
