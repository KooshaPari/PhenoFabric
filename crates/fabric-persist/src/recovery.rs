//! Startup recovery: load all active state from SQLite into memory.

use crate::error::PersistError;
use fabric_graph::model::{RoutePlan, Topology, TopologyEpoch};
use fabric_graph::surface::SurfaceLease;

use crate::Persist;

/// State recovered from the database at startup.
pub struct RecoveredState {
    pub topology: Topology,
    pub active_leases: Vec<SurfaceLease>,
    pub active_plans: Vec<RoutePlan>,
    pub last_evidence_id: i64,
    pub last_audit_id: i64,
}

impl Persist {
    /// Recover all active state from the database.
    /// Called once at daemon startup.
    pub fn recover_state(&self) -> Result<RecoveredState, PersistError> {
        let topology = self.load_topology()?.unwrap_or_default();
        let active_leases = self.load_active_leases()?;
        let active_plans = self.load_active_plans()?;

        let last_evidence_id: i64 = self.with_conn(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COALESCE(MAX(id), 0) FROM evidence_log",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0))
        })?;

        let last_audit_id: i64 = self.with_conn(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COALESCE(MAX(id), 0) FROM audit_log",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0))
        })?;

        Ok(RecoveredState {
            topology,
            active_leases,
            active_plans,
            last_evidence_id,
            last_audit_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recover_empty_database() {
        let persist = Persist::open_memory().unwrap();
        let state = persist.recover_state().unwrap();
        assert_eq!(state.topology.nodes.len(), 0);
        assert_eq!(state.active_leases.len(), 0);
        assert_eq!(state.active_plans.len(), 0);
        assert_eq!(state.last_evidence_id, 0);
        assert_eq!(state.last_audit_id, 0);
    }
}
