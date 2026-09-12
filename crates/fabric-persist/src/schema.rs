//! Schema definitions and migrations for the persistence layer.

use rusqlite::Connection;

use crate::error::PersistError;

/// Current schema version. Increment when adding migrations.
const CURRENT_VERSION: i32 = 1;

/// Run all pending migrations. Called once at startup.
pub fn run_migrations(conn: &Connection) -> Result<(), PersistError> {
    // Create version tracking table if it doesn't exist
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL,
            description TEXT
        );",
    )?;

    let current: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current < 1 {
        migrate_v1(conn)?;
    }

    Ok(())
}

fn migrate_v1(conn: &Connection) -> Result<(), PersistError> {
    conn.execute_batch(
        "
        -- Topology nodes
        CREATE TABLE IF NOT EXISTS topology_nodes (
            id TEXT PRIMARY KEY,
            label TEXT,
            locality_tier INTEGER NOT NULL,
            capabilities TEXT NOT NULL DEFAULT '[]',
            tags TEXT NOT NULL DEFAULT '[]',
            last_seen TEXT NOT NULL
        );

        -- Topology edges
        CREATE TABLE IF NOT EXISTS topology_edges (
            id TEXT PRIMARY KEY,
            from_node TEXT NOT NULL REFERENCES topology_nodes(id),
            to_node TEXT NOT NULL REFERENCES topology_nodes(id),
            locality_tier INTEGER NOT NULL,
            metrics TEXT,
            up INTEGER NOT NULL DEFAULT 1
        );

        -- Topology metadata (epoch, name, etc.)
        CREATE TABLE IF NOT EXISTS topology_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Surface leases
        CREATE TABLE IF NOT EXISTS leases (
            handle TEXT PRIMARY KEY,
            spec TEXT NOT NULL,
            current_binding TEXT,
            history TEXT NOT NULL DEFAULT '[]',
            state TEXT NOT NULL,
            exit_reason TEXT,
            created_at TEXT NOT NULL,
            terminated_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_leases_state ON leases(state);

        -- Route plans
        CREATE TABLE IF NOT EXISTS route_plans (
            id TEXT PRIMARY KEY,
            intent_id TEXT NOT NULL,
            topology_epoch INTEGER NOT NULL,
            steps TEXT NOT NULL,
            estimated_latency_us REAL,
            score TEXT,
            compiled_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            tags TEXT,
            state TEXT NOT NULL DEFAULT 'active',
            replaced_by TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_route_plans_intent ON route_plans(intent_id);
        CREATE INDEX IF NOT EXISTS idx_route_plans_state ON route_plans(state);

        -- Evidence log (append-only)
        CREATE TABLE IF NOT EXISTS evidence_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            event_id TEXT NOT NULL UNIQUE,
            producer_id TEXT,
            principal_id TEXT,
            correlation_id TEXT,
            topology_epoch INTEGER,
            payload TEXT NOT NULL,
            signature TEXT,
            observed_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_evidence_type ON evidence_log(event_type);
        CREATE INDEX IF NOT EXISTS idx_evidence_observed ON evidence_log(observed_at);

        -- Audit log (append-only)
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            actor TEXT NOT NULL,
            resource_type TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            details TEXT,
            result TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action);
        CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);

        -- Record migration
        INSERT INTO schema_version (version, applied_at, description)
        VALUES (1, datetime('now'), 'Initial schema: topology, leases, routes, evidence, audit');
        ",
    )?;

    Ok(())
}

/// Helper to check schema version at runtime.
pub fn schema_version(conn: &Connection) -> Result<i32, PersistError> {
    let v = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Persist;

    #[test]
    fn migration_runs_cleanly() {
        let persist = Persist::open_memory().unwrap();
        let version = persist
            .with_conn(schema_version)
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn idempotent_migration() {
        let persist = Persist::open_memory().unwrap();
        // Run again — should not fail.
        persist
            .with_conn(|conn| run_migrations(conn))
            .unwrap();
        let version = persist.with_conn(schema_version).unwrap();
        assert_eq!(version, 1);
    }
}
