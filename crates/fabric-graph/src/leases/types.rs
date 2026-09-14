//! Rebind outcome type for lease failover integration.

use serde::{Deserialize, Serialize};

use crate::model::RoutePlanId;
use crate::surface::LeaseExitReason;

/// The result of [`rebind_or_fail`](super::rebind_or_fail): did the lease
/// silently re-bind, or did it fail (caller must drop the `SurfaceHandle`)?
///
/// `Rebound` is the "everything worked" case: the handle is unchanged,
/// the prior binding has been rotated into `lease.history`, and a new
/// `RoutePlan` is now bound.
///
/// `Failed` is the "we tried, no replacement exists" case: the lease is
/// now in `LeaseState::Failed` with `exit_reason` populated; the caller
/// MUST drop the handle and re-admit if desired.
///
/// Both variants are `Serialize` so they can flow into the per-surface
/// event log that ADR-0030 \u00a76 reserves for a future wedge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebindOutcome {
    /// The lease was silently re-bound; prior binding moved to history.
    Rebound { new_plan_id: RoutePlanId },
    /// No replacement was available; the lease is now `Failed` and the
    /// caller MUST drop the handle.
    Failed { reason: LeaseExitReason },
}
