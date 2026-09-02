//! Long-horizon planner for sequence/batch intents.
//!
//! When a caller has a *sequence* of intents (e.g. a multi-step build, an ML
//! pipeline, a data transformation chain), the planner can reason about
//! them as a unit: re-using cached capabilities, ordering for locality,
//! and emitting a single [`MultiRoutePlan`].
//!
//! The planner is a thin layer on top of [`compile`](crate::compile::compile).

use crate::compile::{compile, CompileError};
use crate::model::{Intent, RoutePlan, Topology};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlanError {
    #[error("compile error: {0}")]
    Compile(#[from] CompileError),

    #[error("empty sequence (no intents to plan)")]
    EmptySequence,
}

/// A multi-step route plan: ordered plans for a sequence of intents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRoutePlan {
    pub sequence_name: String,
    pub plans: Vec<RoutePlan>,
    /// Sum of estimated latencies (end-to-end).
    pub total_estimated_latency_us: Option<f64>,
    /// Topology epoch at which this was planned.
    pub planned_at_epoch: crate::model::TopologyEpoch,
}

/// Plan a sequence of intents against a topology.
///
/// This is a simple, sequential planner: each intent is compiled in order
/// against the same topology snapshot. A future implementation could
/// reason about intermediate state (caching, dependency-aware reordering).
pub fn plan_sequence(
    topology: &Topology,
    sequence_name: &str,
    intents: &[Intent],
) -> Result<MultiRoutePlan, PlanError> {
    if intents.is_empty() {
        return Err(PlanError::EmptySequence);
    }

    let mut plans = Vec::with_capacity(intents.len());
    let mut total_latency = 0.0;

    for intent in intents {
        let plan = compile(topology, intent)?;
        if let Some(lat) = plan.estimated_latency_us {
            total_latency += lat;
        }
        plans.push(plan);
    }

    Ok(MultiRoutePlan {
        sequence_name: sequence_name.to_string(),
        plans,
        total_estimated_latency_us: if total_latency > 0.0 {
            Some(total_latency)
        } else {
            None
        },
        planned_at_epoch: topology.epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{IntentBuilder, TopologyBuilder};
    use crate::model::Intent;
    use fabric_capability::locality::LocalityTier;

    fn make_topology() -> Topology {
        TopologyBuilder::new()
            .add_with_tag("gpu-0", LocalityTier::L1, "rt-island")
            .add_simple_node("cpu-0", LocalityTier::L2)
            .connect("gpu-0", "cpu-0", LocalityTier::L1)
            .build()
    }

    fn make_intents() -> Vec<Intent> {
        vec![
            IntentBuilder::new().name("step-1").require_tag("rt-island").build(),
            IntentBuilder::new().name("step-2").require_tag("rt-island").build(),
            IntentBuilder::new().name("step-3").require_tag("rt-island").build(),
        ]
    }

    #[test]
    fn test_plan_sequence_success() {
        let topo = make_topology();
        let intents = make_intents();
        let multi = plan_sequence(&topo, "build", &intents).expect("should plan");

        assert_eq!(multi.sequence_name, "build");
        assert_eq!(multi.plans.len(), 3);
    }

    #[test]
    fn test_plan_sequence_empty() {
        let topo = make_topology();
        let err = plan_sequence(&topo, "empty", &[]).expect_err("should fail");
        assert!(matches!(err, PlanError::EmptySequence));
    }

    #[test]
    fn test_plan_sequence_one_intent() {
        let topo = make_topology();
        let intents = vec![IntentBuilder::new().name("solo").require_tag("rt-island").build()];
        let multi = plan_sequence(&topo, "solo", &intents).expect("should plan");
        assert_eq!(multi.plans.len(), 1);
    }

    #[test]
    fn test_plan_sequence_compile_failure_propagates() {
        let topo = make_topology();
        // Intent that no node can satisfy
        let intents = vec![IntentBuilder::new()
            .name("unsatisfiable")
            .require_tag("nonexistent-tag")
            .build()];

        let err = plan_sequence(&topo, "fail", &intents).expect_err("should fail");
        assert!(matches!(err, PlanError::Compile(_)));
    }
}
