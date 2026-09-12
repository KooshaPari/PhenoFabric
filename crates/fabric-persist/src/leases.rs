//! Lease persistence — save/load/expire surface leases to/from SQLite.

use chrono::{DateTime, Utc};
use fabric_graph::surface::{LeaseExitReason, LeaseState, SurfaceLease};
use fabric_graph::{LocalityTier, SurfaceHandle, SurfaceSpec};
use rusqlite::params;

use crate::error::PersistError;
use crate::Persist;

impl Persist {
    /// Save a lease (insert or update by handle).
    pub fn save_lease(&self, lease: &SurfaceLease) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let handle_str = lease.handle.0.to_string();
            let spec_json = serde_json::to_string(&lease.spec)?;
            let current_json = serde_json::to_string(&lease.current)?;
            let history_json = serde_json::to_string(&lease.history)?;
            let state = format!("{:?}", lease.state);
            let exit_reason_json = serde_json::to_string(&lease.exit_reason)?;

            conn.execute(
                "INSERT OR REPLACE INTO leases
                 (handle, spec, current_binding, history, state, exit_reason,
                  created_at, terminated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    handle_str,
                    spec_json,
                    current_json,
                    history_json,
                    state,
                    exit_reason_json,
                    lease.created_at.to_rfc3339(),
                    lease.terminated_at.map(|t| t.to_rfc3339()),
                ],
            )?;
            Ok(())
        })
    }

    /// Load a lease by handle.
    pub fn load_lease(&self, handle: &SurfaceHandle) -> Result<Option<SurfaceLease>, PersistError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT handle, spec, current_binding, history, state, exit_reason,
                        created_at, terminated_at
                 FROM leases WHERE handle = ?1",
            )?;
            let mut rows = stmt.query_map(params![handle.0.to_string()], |row| {
                row_to_lease(row)
            })?;
            match rows.next() {
                Some(r) => Ok(Some(r?)),
                None => Ok(None),
            }
        })
    }

    /// Load all leases in a given state.
    pub fn load_leases_by_state(
        &self,
        state: LeaseState,
    ) -> Result<Vec<SurfaceLease>, PersistError> {
        let state_str = format!("{:?}", state);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT handle, spec, current_binding, history, state, exit_reason,
                        created_at, terminated_at
                 FROM leases WHERE state = ?1",
            )?;
            let rows = stmt.query_map(params![state_str], |row| row_to_lease(row))?;
            let mut leases = Vec::new();
            for row in rows {
                leases.push(row?);
            }
            Ok(leases)
        })
    }

    /// Load all active leases (Pending or Active).
    pub fn load_active_leases(&self) -> Result<Vec<SurfaceLease>, PersistError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT handle, spec, current_binding, history, state, exit_reason,
                        created_at, terminated_at
                 FROM leases WHERE state IN ('Pending', 'Active')",
            )?;
            let rows = stmt.query_map([], |row| row_to_lease(row))?;
            let mut leases = Vec::new();
            for row in rows {
                leases.push(row?);
            }
            Ok(leases)
        })
    }

    /// Transition a lease to a new state.
    pub fn set_lease_state(
        &self,
        handle: &SurfaceHandle,
        new_state: LeaseState,
        exit_reason: Option<&LeaseExitReason>,
    ) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let state_str = format!("{:?}", new_state);
            let exit_json = serde_json::to_string(&exit_reason)?;
            let now = Utc::now().to_rfc3339();

            let terminated_at = if matches!(
                new_state,
                LeaseState::Completed
                    | LeaseState::Failed
                    | LeaseState::Revoked
                    | LeaseState::Expired
            ) {
                Some(now.as_str())
            } else {
                None
            };

            let updated = conn.execute(
                "UPDATE leases SET state = ?1, exit_reason = ?2, terminated_at = COALESCE(?3, terminated_at)
                 WHERE handle = ?4",
                params![state_str, exit_json, terminated_at, handle.0.to_string()],
            )?;
            if updated == 0 {
                return Err(PersistError::NotFound(format!(
                    "lease handle {}",
                    handle.0
                )));
            }
            Ok(())
        })
    }

    /// Delete a lease by handle.
    pub fn delete_lease(&self, handle: &SurfaceHandle) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM leases WHERE handle = ?1",
                params![handle.0.to_string()],
            )?;
            Ok(())
        })
    }

    /// Count leases in each state.
    pub fn count_leases_by_state(
        &self,
    ) -> Result<Vec<(String, i64)>, PersistError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT state, COUNT(*) FROM leases GROUP BY state ORDER BY state",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            let mut counts = Vec::new();
            for row in rows {
                counts.push(row?);
            }
            Ok(counts)
        })
    }
}

/// Parse a LeaseState from its Debug string representation.
fn parse_lease_state(s: &str) -> LeaseState {
    match s {
        "Pending" => LeaseState::Pending,
        "Active" => LeaseState::Active,
        "Completed" => LeaseState::Completed,
        "Failed" => LeaseState::Failed,
        "Revoked" => LeaseState::Revoked,
        "Expired" => LeaseState::Expired,
        _ => LeaseState::Pending,
    }
}

/// Convert a database row to a SurfaceLease.
fn row_to_lease(row: &rusqlite::Row<'_>) -> Result<SurfaceLease, rusqlite::Error> {
    let handle_str: String = row.get(0)?;
    let spec_json: String = row.get(1)?;
    let current_json: String = row.get(2)?;
    let history_json: String = row.get(3)?;
    let state_str: String = row.get(4)?;
    let exit_reason_json: String = row.get(5)?;
    let created_at_str: String = row.get(6)?;
    let terminated_at_str: Option<String> = row.get(7)?;

    let handle = SurfaceHandle(
        uuid::Uuid::parse_str(&handle_str).unwrap_or_default(),
    );
    let spec: SurfaceSpec = serde_json::from_str(&spec_json).unwrap_or_else(|_| {
        // Fallback: minimal valid spec.
        SurfaceSpec {
            name: "unknown".into(),
            protocol: fabric_graph::surface::SurfaceProtocol::Posix,
            capture: None,
            locality_floor: LocalityTier::L7Wan,
            refresh_hz: None,
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: fabric_graph::model::TrustLevel::default(),
            expires_at: None,
        }
    });
    let current: Option<fabric_graph::surface::RouteBinding> =
        serde_json::from_str(&current_json).ok().flatten();
    let history: Vec<fabric_graph::surface::RouteBinding> =
        serde_json::from_str(&history_json).unwrap_or_default();
    let state = parse_lease_state(&state_str);
    let exit_reason: Option<LeaseExitReason> =
        serde_json::from_str(&exit_reason_json).ok().flatten();
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let terminated_at = terminated_at_str
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    Ok(SurfaceLease {
        handle,
        spec,
        current,
        history,
        state,
        exit_reason,
        created_at,
        terminated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Persist;
    use fabric_graph::surface::{LeaseState, SurfaceLease, SurfaceSpec};
    use fabric_graph::{LocalityTier, SurfaceHandle};
    use fabric_graph::surface::{CaptureDirection, SurfaceProtocol};
    use fabric_graph::model::TrustLevel;

    fn test_lease() -> SurfaceLease {
        SurfaceLease {
            handle: SurfaceHandle::new(),
            spec: SurfaceSpec {
                name: "test-surface".into(),
                protocol: SurfaceProtocol::WebRtc,
                capture: Some(CaptureDirection::Bidirectional),
                locality_floor: LocalityTier::L6Lan,
                refresh_hz: Some(60),
                audio_sample_rate_hz: None,
                requires_rt_island: false,
                strict_epoch_binding: false,
                min_host_trust: TrustLevel::default(),
                expires_at: None,
            },
            current: None,
            history: vec![],
            state: LeaseState::Pending,
            exit_reason: None,
            created_at: Utc::now(),
            terminated_at: None,
        }
    }

    #[test]
    fn save_and_load_lease_roundtrip() {
        let persist = Persist::open_memory().unwrap();
        let lease = test_lease();
        let handle = lease.handle;

        persist.save_lease(&lease).unwrap();
        let loaded = persist.load_lease(&handle).unwrap().unwrap();

        assert_eq!(loaded.handle, handle);
        assert_eq!(loaded.state, LeaseState::Pending);
        assert!(loaded.current.is_none());
    }

    #[test]
    fn load_nonexistent_lease_returns_none() {
        let persist = Persist::open_memory().unwrap();
        let handle = SurfaceHandle::new();
        assert!(persist.load_lease(&handle).unwrap().is_none());
    }

    #[test]
    fn set_lease_state_active() {
        let persist = Persist::open_memory().unwrap();
        let lease = test_lease();
        let handle = lease.handle;
        persist.save_lease(&lease).unwrap();

        persist
            .set_lease_state(&handle, LeaseState::Active, None)
            .unwrap();
        let loaded = persist.load_lease(&handle).unwrap().unwrap();
        assert_eq!(loaded.state, LeaseState::Active);
    }

    #[test]
    fn set_lease_state_expired_sets_terminated_at() {
        let persist = Persist::open_memory().unwrap();
        let lease = test_lease();
        let handle = lease.handle;
        persist.save_lease(&lease).unwrap();

        persist
            .set_lease_state(&handle, LeaseState::Expired, None)
            .unwrap();
        let loaded = persist.load_lease(&handle).unwrap().unwrap();
        assert_eq!(loaded.state, LeaseState::Expired);
        assert!(loaded.terminated_at.is_some());
    }

    #[test]
    fn delete_lease() {
        let persist = Persist::open_memory().unwrap();
        let lease = test_lease();
        let handle = lease.handle;
        persist.save_lease(&lease).unwrap();

        persist.delete_lease(&handle).unwrap();
        assert!(persist.load_lease(&handle).unwrap().is_none());
    }

    #[test]
    fn load_leases_by_state() {
        let persist = Persist::open_memory().unwrap();
        let mut lease1 = test_lease();
        lease1.state = LeaseState::Pending;
        persist.save_lease(&lease1).unwrap();

        let mut lease2 = test_lease();
        lease2.state = LeaseState::Active;
        persist.save_lease(&lease2).unwrap();

        let pending = persist
            .load_leases_by_state(LeaseState::Pending)
            .unwrap();
        assert_eq!(pending.len(), 1);

        let active = persist
            .load_leases_by_state(LeaseState::Active)
            .unwrap();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn count_leases_by_state() {
        let persist = Persist::open_memory().unwrap();
        let lease = test_lease();
        persist.save_lease(&lease).unwrap();

        let counts = persist.count_leases_by_state().unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].0, "Pending");
        assert_eq!(counts[0].1, 1);
    }
}
