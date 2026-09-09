//! Wire protocol for `fabric-graph-cli replan`.
//!
//! Per spec 023 §3, the binary accepts a `ReplanRequest` (JSON) on stdin or via
//! `--request <file>` and prints a `ReplanResponse` (JSON) to stdout. Errors
//! are emitted on stderr and surface an exit code per spec 023 §3.3.
//!
//! The protocol is intentionally minimal: it wraps the existing `failover::replan`
//! signature and never invents new types beyond a thin request/response wrapper.

use serde::{Deserialize, Serialize};

use fabric_graph::failover::{FailoverError, FailoverOutcome};
use fabric_graph::model::{Intent, NodeId, RoutePlan, Topology};

/// The full request body for `replan`.
///
/// All four inputs are required. The protocol does not support partial requests:
/// the caller must supply the full topology + intent + plan + blacklist. This
/// matches the `fabric_graph::failover::replan` signature exactly.
///
/// `PartialEq`/`Eq` are intentionally NOT derived — `RoutePlan` is a plan
/// record with UUIDs and timestamps, not a value type (see
/// `crates/fabric-graph/src/model.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanRequest {
    /// The current live topology (after the caller has pruned failed nodes).
    pub topology: Topology,
    /// The placement intent (what we want to satisfy).
    pub intent: Intent,
    /// The plan we are replacing.
    pub old_plan: RoutePlan,
    /// Nodes we know to have failed (informational; topology must already be pruned).
    pub failed_nodes: Vec<NodeId>,
}

/// The successful response body.
///
/// `PartialEq`/`Eq` intentionally omitted for the same reason as [`ReplanRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReplanResponse {
    /// A replacement plan was successfully compiled.
    Replaced {
        /// The new plan.
        new_plan: RoutePlan,
    },
    /// No feasible replacement exists; the caller must release the lease.
    NoReplacement {
        /// The NodeIDs that failed (echoed back for caller convenience).
        failed_nodes: Vec<NodeId>,
    },
}

impl ReplanResponse {
    /// Convenience for tests/callers that prefer a tagged-enum shape.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            ReplanResponse::Replaced { .. } => "replaced",
            ReplanResponse::NoReplacement { .. } => "no_replacement",
        }
    }
}

/// The error response body, in addition to a non-zero process exit code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanErrorResponse {
    /// Stable error code (matches the spec 023 §3.3 table).
    pub code: ReplanErrorCode,
    /// Human-readable message (for log + stderr, never for UI).
    pub message: String,
}

/// Stable error codes per spec 023 §3.3.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplanErrorCode {
    /// `failover::replan` returned `Err(FailoverError::EmptyIntent)`.
    EmptyIntent,
    /// `failover::replan` returned `Err(FailoverError::AllCandidatesFailed)`.
    AllCandidatesFailed,
    /// The request JSON could not be parsed.
    InvalidRequest,
    /// The request file (if `--request <file>` used) could not be read.
    MissingRequestFile,
    /// The stdin stream could not be read to EOF.
    StdinReadFailed,
    /// Internal error (caller-side bug; surfaces as exit code 1).
    Internal,
}

impl ReplanErrorCode {
    /// Numeric exit code per spec 023 §3.3.
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            ReplanErrorCode::EmptyIntent => 10,
            ReplanErrorCode::AllCandidatesFailed => 11,
            ReplanErrorCode::InvalidRequest => 20,
            ReplanErrorCode::MissingRequestFile => 21,
            ReplanErrorCode::StdinReadFailed => 22,
            ReplanErrorCode::Internal => 1,
        }
    }
}

/// The pure (non-IO) replan function that the binary wraps.
///
/// This is the protocol-layer `replan`: it owns nothing more than the
/// conversion between `ReplanRequest` / `ReplanResponse` and the underlying
/// `failover::replan` signature. Exposed so that other Rust callers can embed
/// the same logic without going through the subprocess boundary.
pub fn replan(req: &ReplanRequest) -> Result<ReplanResponse, ReplanError> {
    let outcome = fabric_graph::failover::replan(
        &req.topology,
        &req.intent,
        &req.old_plan,
        &req.failed_nodes,
    )
    .map_err(replan_err_from)?;
    Ok(match outcome {
        FailoverOutcome::Replaced(new_plan) => ReplanResponse::Replaced { new_plan },
        FailoverOutcome::NoReplacement => ReplanResponse::NoReplacement {
            failed_nodes: req.failed_nodes.clone(),
        },
    })
}

/// Protocol-layer error type (distinct from the internal `FailoverError`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplanError {
    #[error("intent has no requirements — nothing to replan onto")]
    EmptyIntent,
    #[error("all candidate nodes are blacklisted")]
    AllCandidatesFailed,
}

fn replan_err_from(e: FailoverError) -> ReplanError {
    match e {
        FailoverError::EmptyIntent => ReplanError::EmptyIntent,
        FailoverError::AllCandidatesFailed => ReplanError::AllCandidatesFailed,
    }
}

impl ReplanError {
    /// Map to the wire-level [`ReplanErrorCode`].
    #[must_use]
    pub fn code(&self) -> ReplanErrorCode {
        match self {
            ReplanError::EmptyIntent => ReplanErrorCode::EmptyIntent,
            ReplanError::AllCandidatesFailed => ReplanErrorCode::AllCandidatesFailed,
        }
    }

    /// Map to the wire-level [`ReplanErrorResponse`] (a value-safe projection).
    #[must_use]
    pub fn to_response(&self) -> ReplanErrorResponse {
        ReplanErrorResponse {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fabric_graph::builder::{IntentBuilder, TopologyBuilder};
    use fabric_graph::LocalityTier;

    fn two_node_topology() -> (Topology, NodeId, NodeId) {
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let topo = TopologyBuilder::new()
            .with_name("test")
            .add(fabric_graph::Node::new(a.clone(), LocalityTier::L5Loopback))
            .add(fabric_graph::Node::new(b.clone(), LocalityTier::L5Loopback))
            .connect("a", "b", LocalityTier::L1SameNuma)
            .build();
        (topo, a, b)
    }

    #[test]
    fn replan_with_empty_blacklist_returns_old_plan() {
        let (topo, a, _) = two_node_topology();
        let intent = IntentBuilder::new()
            .name("test-intent")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        let original = fabric_graph::compile(&topo, &intent).unwrap();

        let req = ReplanRequest {
            topology: topo,
            intent,
            old_plan: original.clone(),
            failed_nodes: vec![],
        };
        let resp = replan(&req).unwrap();
        match resp {
            ReplanResponse::Replaced { new_plan } => {
                assert_eq!(new_plan.id, original.id);
            }
            other => panic!("expected Replaced, got {:?}", other.kind()),
        }
        let _ = a;
    }

    #[test]
    fn replan_after_node_pruning_produces_new_route() {
        let (topo, a, b) = two_node_topology();
        let intent = IntentBuilder::new()
            .name("test-intent")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        let original = fabric_graph::compile(&topo, &intent).unwrap();

        // Caller prunes `a` (simulating node failure) → topology with only `b`.
        let pruned_topo = TopologyBuilder::new()
            .with_name("pruned")
            .add(fabric_graph::Node::new(b.clone(), LocalityTier::L5Loopback))
            .build();

        let req = ReplanRequest {
            topology: pruned_topo,
            intent,
            old_plan: original,
            failed_nodes: vec![a.clone()],
        };
        let resp = replan(&req).unwrap();
        match resp {
            ReplanResponse::Replaced { new_plan } => {
                for step in &new_plan.steps {
                    assert_eq!(step.node, b, "step must use surviving node");
                }
            }
            other => panic!("expected Replaced, got {:?}", other.kind()),
        }
    }

    #[test]
    fn replan_with_no_survivors_returns_no_replacement() {
        let (topo, a, b) = two_node_topology();
        let intent = IntentBuilder::new()
            .name("test-intent")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        let original = fabric_graph::compile(&topo, &intent).unwrap();

        let empty_topo = TopologyBuilder::new().with_name("empty").build();
        let req = ReplanRequest {
            topology: empty_topo,
            intent,
            old_plan: original,
            failed_nodes: vec![a, b],
        };
        let resp = replan(&req).unwrap();
        match resp {
            ReplanResponse::NoReplacement { failed_nodes } => {
                assert_eq!(failed_nodes.len(), 2);
            }
            other => panic!("expected NoReplacement, got {:?}", other.kind()),
        }
    }

    #[test]
    fn replan_with_empty_intent_returns_empty_intent_error() {
        let (topo, _, _) = two_node_topology();
        let intent = IntentBuilder::new()
            .name("")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        let original = fabric_graph::compile(&topo, &intent).unwrap();

        let req = ReplanRequest {
            topology: topo,
            intent,
            old_plan: original,
            failed_nodes: vec![],
        };
        let err = replan(&req).unwrap_err();
        assert_eq!(err.code(), ReplanErrorCode::EmptyIntent);
    }

    #[test]
    fn replan_request_json_round_trip() {
        let (topo, _, _) = two_node_topology();
        let intent = IntentBuilder::new()
            .name("rt-test")
            .min_trust(fabric_graph::TrustLevel::Untrusted)
            .build();
        let original = fabric_graph::compile(&topo, &intent).unwrap();
        let req = ReplanRequest {
            topology: topo,
            intent,
            old_plan: original,
            failed_nodes: vec![NodeId::new("a")],
        };
        let s = serde_json::to_string(&req).unwrap();
        let back: ReplanRequest = serde_json::from_str(&s).unwrap();
        // RoutePlan doesn't derive PartialEq — assert field-by-field on the parts
        // we can compare directly. Topology exposes name via meta; Intent has it
        // as a direct field (not a method).
        assert_eq!(back.topology.meta.name, req.topology.meta.name);
        assert_eq!(back.intent.name, req.intent.name);
        assert_eq!(back.failed_nodes.len(), req.failed_nodes.len());
    }

    #[test]
    fn replan_response_serde_distinguishes_variants() {
        let r1 = ReplanResponse::NoReplacement {
            failed_nodes: vec![NodeId::new("a")],
        };
        let s = serde_json::to_string(&r1).unwrap();
        // The tag format forces a recognisable prefix.
        assert!(s.contains("\"no_replacement\""), "got: {s}");
    }

    #[test]
    fn error_codes_have_expected_exit_codes() {
        assert_eq!(ReplanErrorCode::EmptyIntent.exit_code(), 10);
        assert_eq!(ReplanErrorCode::AllCandidatesFailed.exit_code(), 11);
        assert_eq!(ReplanErrorCode::InvalidRequest.exit_code(), 20);
        assert_eq!(ReplanErrorCode::MissingRequestFile.exit_code(), 21);
        assert_eq!(ReplanErrorCode::StdinReadFailed.exit_code(), 22);
        assert_eq!(ReplanErrorCode::Internal.exit_code(), 1);
    }
}
