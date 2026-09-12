//! Route plan CRUD operations.

use chrono::{DateTime, Utc};
use fabric_graph::model::{IntentId, RoutePlan, RoutePlanId, RouteStep, ScoreBreakdown, TopologyEpoch};
use rusqlite::params;

use crate::error::PersistError;
use crate::Persist;

impl Persist {
    /// Save a route plan.
    pub fn save_route_plan(&self, plan: &RoutePlan) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let steps_json = serde_json::to_string(&plan.steps)?;
            let score_json = serde_json::to_string(&plan.score)?;
            let tags_json = serde_json::to_string(&plan.tags)?;

            conn.execute(
                "INSERT OR REPLACE INTO route_plans
                 (id, intent_id, topology_epoch, steps, estimated_latency_us, score,
                  compiled_at, expires_at, tags, state, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', ?7)",
                params![
                    plan.id.0.to_string(),
                    plan.intent_id.0.to_string(),
                    plan.topology_epoch.0 as i64,
                    steps_json,
                    plan.estimated_latency_us,
                    score_json,
                    plan.compiled_at.to_rfc3339(),
                    plan.expires_at.to_rfc3339(),
                    tags_json,
                ],
            )?;
            Ok(())
        })
    }

    /// Load all active route plans.
    pub fn load_active_plans(&self) -> Result<Vec<RoutePlan>, PersistError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, intent_id, topology_epoch, steps, estimated_latency_us,
                        score, compiled_at, expires_at, tags
                 FROM route_plans WHERE state = 'active'",
            )?;
            let rows = stmt.query_map([], |row| {
                let id_str: String = row.get(0)?;
                let intent_id_str: String = row.get(1)?;
                let epoch_i64: i64 = row.get(2)?;
                let steps_json: String = row.get(3)?;
                let latency: Option<f64> = row.get(4)?;
                let score_json: Option<String> = row.get(5)?;
                let compiled_at: String = row.get(6)?;
                let expires_at: String = row.get(7)?;
                let tags_json: String = row.get(8)?;

                let steps: Vec<RouteStep> =
                    serde_json::from_str(&steps_json).unwrap_or_default();
                let score: Option<ScoreBreakdown> =
                    score_json.and_then(|s| serde_json::from_str(&s).ok());
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(RoutePlan {
                    id: RoutePlanId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
                    intent_id: IntentId(uuid::Uuid::parse_str(&intent_id_str).unwrap_or_default()),
                    topology_epoch: TopologyEpoch(epoch_i64 as u64),
                    steps,
                    estimated_latency_us: latency,
                    score,
                    compiled_at: DateTime::parse_from_rfc3339(&compiled_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    tags,
                })
            })?;

            let mut plans = Vec::new();
            for row in rows {
                plans.push(row?);
            }
            Ok(plans)
        })
    }

    /// Load a route plan by ID.
    pub fn load_plan(&self, id: &RoutePlanId) -> Result<Option<RoutePlan>, PersistError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, intent_id, topology_epoch, steps, estimated_latency_us,
                        score, compiled_at, expires_at, tags
                 FROM route_plans WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map(params![id.0.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let intent_id_str: String = row.get(1)?;
                let epoch_i64: i64 = row.get(2)?;
                let steps_json: String = row.get(3)?;
                let latency: Option<f64> = row.get(4)?;
                let score_json: Option<String> = row.get(5)?;
                let compiled_at: String = row.get(6)?;
                let expires_at: String = row.get(7)?;
                let tags_json: String = row.get(8)?;

                let steps: Vec<RouteStep> =
                    serde_json::from_str(&steps_json).unwrap_or_default();
                let score: Option<ScoreBreakdown> =
                    score_json.and_then(|s| serde_json::from_str(&s).ok());
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(RoutePlan {
                    id: RoutePlanId(uuid::Uuid::parse_str(&id_str).unwrap_or_default()),
                    intent_id: IntentId(uuid::Uuid::parse_str(&intent_id_str).unwrap_or_default()),
                    topology_epoch: TopologyEpoch(epoch_i64 as u64),
                    steps,
                    estimated_latency_us: latency,
                    score,
                    compiled_at: DateTime::parse_from_rfc3339(&compiled_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    tags,
                })
            })?;
            match rows.next() {
                Some(r) => Ok(Some(r?)),
                None => Ok(None),
            }
        })
    }

    /// Expire a route plan.
    pub fn expire_plan(&self, id: &RoutePlanId) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let updated = conn.execute(
                "UPDATE route_plans SET state = 'expired' WHERE id = ?1 AND state = 'active'",
                params![id.0.to_string()],
            )?;
            if updated == 0 {
                return Err(PersistError::NotFound(format!("route plan {}", id.0)));
            }
            Ok(())
        })
    }

    /// Replace an old route plan with a new one (transactional).
    pub fn replace_plan(
        &self,
        old_id: &RoutePlanId,
        new_plan: &RoutePlan,
    ) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;

            // Expire old plan.
            tx.execute(
                "UPDATE route_plans SET state = 'replaced', replaced_by = ?1 WHERE id = ?2",
                params![new_plan.id.0.to_string(), old_id.0.to_string()],
            )?;

            // Save new plan.
            let steps_json = serde_json::to_string(&new_plan.steps)?;
            let score_json = serde_json::to_string(&new_plan.score)?;
            let tags_json = serde_json::to_string(&new_plan.tags)?;

            tx.execute(
                "INSERT OR REPLACE INTO route_plans
                 (id, intent_id, topology_epoch, steps, estimated_latency_us, score,
                  compiled_at, expires_at, tags, state, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', ?7)",
                params![
                    new_plan.id.0.to_string(),
                    new_plan.intent_id.0.to_string(),
                    new_plan.topology_epoch.0 as i64,
                    steps_json,
                    new_plan.estimated_latency_us,
                    score_json,
                    new_plan.compiled_at.to_rfc3339(),
                    new_plan.expires_at.to_rfc3339(),
                    tags_json,
                ],
            )?;

            tx.commit()?;
            Ok(())
        })
    }

    /// Count active route plans.
    pub fn count_active_plans(&self) -> Result<i64, PersistError> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM route_plans WHERE state = 'active'",
                [],
                |row| row.get(0),
            )?;
            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Persist;
    use fabric_graph::model::{NodeId, RouteStep};
    use uuid::Uuid;

    fn test_plan() -> RoutePlan {
        RoutePlan {
            id: RoutePlanId(Uuid::now_v7()),
            intent_id: IntentId(Uuid::now_v7()),
            topology_epoch: TopologyEpoch(1),
            steps: vec![RouteStep {
                node: NodeId("test-node".into()),
                capability_id: None,
                via_edge: None,
                action: "execute".into(),
            }],
            estimated_latency_us: Some(500.0),
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec!["test".into()],
        }
    }

    #[test]
    fn save_and_load_plan_roundtrip() {
        let persist = Persist::open_memory().unwrap();
        let plan = test_plan();
        let id = plan.id.clone();

        persist.save_route_plan(&plan).unwrap();
        let loaded = persist.load_plan(&id).unwrap().unwrap();

        assert_eq!(loaded.id, id);
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.estimated_latency_us, Some(500.0));
        assert_eq!(loaded.tags, vec!["test".to_string()]);
    }

    #[test]
    fn load_nonexistent_plan_returns_none() {
        let persist = Persist::open_memory().unwrap();
        let id = RoutePlanId(Uuid::now_v7());
        assert!(persist.load_plan(&id).unwrap().is_none());
    }

    #[test]
    fn expire_plan() {
        let persist = Persist::open_memory().unwrap();
        let plan = test_plan();
        let id = plan.id.clone();
        persist.save_route_plan(&plan).unwrap();

        persist.expire_plan(&id).unwrap();
        // load_plan still returns the plan (it's expired but exists).
        assert!(persist.load_plan(&id).unwrap().is_some());
        assert_eq!(persist.count_active_plans().unwrap(), 0);
    }

    #[test]
    fn replace_plan() {
        let persist = Persist::open_memory().unwrap();
        let old_plan = test_plan();
        let old_id = old_plan.id.clone();
        persist.save_route_plan(&old_plan).unwrap();
        assert_eq!(persist.count_active_plans().unwrap(), 1);

        let mut new_plan = test_plan();
        new_plan.estimated_latency_us = Some(200.0);
        persist.replace_plan(&old_id, &new_plan).unwrap();

        assert_eq!(persist.count_active_plans().unwrap(), 1);
        let loaded = persist.load_plan(&new_plan.id).unwrap().unwrap();
        assert_eq!(loaded.estimated_latency_us, Some(200.0));
    }

    #[test]
    fn load_active_plans() {
        let persist = Persist::open_memory().unwrap();
        let plan = test_plan();
        persist.save_route_plan(&plan).unwrap();

        let plans = persist.load_active_plans().unwrap();
        assert_eq!(plans.len(), 1);
    }
}
