//! Evidence and audit log operations.

use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json;

use crate::error::PersistError;
use crate::Persist;

/// A simple event envelope for evidence storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope {
    pub event_type: String,
    pub event_id: String,
    pub producer_id: Option<String>,
    pub principal_id: Option<String>,
    pub correlation_id: Option<String>,
    pub topology_epoch: Option<u64>,
    pub payload: serde_json::Value,
    pub signature: Option<String>,
    pub observed_at: DateTime<Utc>,
}

/// An audit log entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub actor: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: Option<serde_json::Value>,
    pub result: String,
    pub observed_at: DateTime<Utc>,
}

impl Persist {
    /// Append an event to the evidence log.
    pub fn append_evidence(&self, event: &EventEnvelope) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let payload_json = serde_json::to_string(&event.payload).unwrap_or_default();
            let now = Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO evidence_log
                 (event_type, event_id, producer_id, principal_id, correlation_id,
                  topology_epoch, payload, signature, observed_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.event_type,
                    event.event_id,
                    event.producer_id,
                    event.principal_id,
                    event.correlation_id,
                    event.topology_epoch.map(|e| e as i64),
                    payload_json,
                    event.signature,
                    event.observed_at.to_rfc3339(),
                    now,
                ],
            )?;
            Ok(())
        })
    }

    /// Query evidence events.
    pub fn query_evidence(
        &self,
        event_type: Option<&str>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<EventEnvelope>, PersistError> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT event_type, event_id, producer_id, principal_id, correlation_id,
                        topology_epoch, payload, signature, observed_at
                 FROM evidence_log WHERE 1=1",
            );
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(et) = event_type {
                sql.push_str(" AND event_type = ?1");
                params_vec.push(Box::new(et.to_string()));
            }
            if let Some(s) = since {
                let idx = params_vec.len() + 1;
                sql.push_str(&format!(" AND observed_at >= ?{}", idx));
                params_vec.push(Box::new(s.to_rfc3339()));
            }
            sql.push_str(&format!(" ORDER BY id DESC LIMIT {}", limit));

            let params_refs: Vec<&dyn rusqlite::types::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                let event_type: String = row.get(0)?;
                let event_id: String = row.get(1)?;
                let producer_id: Option<String> = row.get(2)?;
                let principal_id: Option<String> = row.get(3)?;
                let correlation_id: Option<String> = row.get(4)?;
                let epoch: Option<i64> = row.get(5)?;
                let payload_json: String = row.get(6)?;
                let signature: Option<String> = row.get(7)?;
                let observed_at: String = row.get(8)?;

                let payload: serde_json::Value =
                    serde_json::from_str(&payload_json).unwrap_or_default();
                let observed = DateTime::parse_from_rfc3339(&observed_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(EventEnvelope {
                    event_type,
                    event_id,
                    producer_id,
                    principal_id,
                    correlation_id,
                    topology_epoch: epoch.map(|e| e as u64),
                    payload,
                    signature,
                    observed_at: observed,
                })
            })?;

            let mut events = Vec::new();
            for row in rows {
                events.push(row?);
            }
            Ok(events)
        })
    }

    /// Append an entry to the audit log.
    pub fn append_audit(&self, entry: &AuditEntry) -> Result<(), PersistError> {
        self.with_conn(|conn| {
            let details_json = entry
                .details
                .as_ref()
                .and_then(|d| serde_json::to_string(d).ok())
                .unwrap_or_default();
            let now = Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO audit_log
                 (action, actor, resource_type, resource_id, details, result, observed_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.action,
                    entry.actor,
                    entry.resource_type,
                    entry.resource_id,
                    details_json,
                    entry.result,
                    entry.observed_at.to_rfc3339(),
                    now,
                ],
            )?;
            Ok(())
        })
    }

    /// Query audit log entries.
    pub fn query_audit(
        &self,
        action: Option<&str>,
        resource: Option<(&str, &str)>,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, PersistError> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT action, actor, resource_type, resource_id, details, result, observed_at
                 FROM audit_log WHERE 1=1",
            );
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(a) = action {
                let idx = params_vec.len() + 1;
                sql.push_str(&format!(" AND action = ?{}", idx));
                params_vec.push(Box::new(a.to_string()));
            }
            if let Some((rt, rid)) = resource {
                let idx1 = params_vec.len() + 1;
                let idx2 = params_vec.len() + 2;
                sql.push_str(&format!(" AND resource_type = ?{} AND resource_id = ?{}", idx1, idx2));
                params_vec.push(Box::new(rt.to_string()));
                params_vec.push(Box::new(rid.to_string()));
            }
            if let Some(s) = since {
                let idx = params_vec.len() + 1;
                sql.push_str(&format!(" AND observed_at >= ?{}", idx));
                params_vec.push(Box::new(s.to_rfc3339()));
            }
            sql.push_str(&format!(" ORDER BY id DESC LIMIT {}", limit));

            let params_refs: Vec<&dyn rusqlite::types::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                let action: String = row.get(0)?;
                let actor: String = row.get(1)?;
                let resource_type: String = row.get(2)?;
                let resource_id: String = row.get(3)?;
                let details_json: Option<String> = row.get(4)?;
                let result: String = row.get(5)?;
                let observed_at: String = row.get(6)?;

                let details: Option<serde_json::Value> = details_json
                    .and_then(|j| serde_json::from_str(&j).ok());
                let observed = DateTime::parse_from_rfc3339(&observed_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(AuditEntry {
                    action,
                    actor,
                    resource_type,
                    resource_id,
                    details,
                    result,
                    observed_at: observed,
                })
            })?;

            let mut entries = Vec::new();
            for row in rows {
                entries.push(row?);
            }
            Ok(entries)
        })
    }
}
