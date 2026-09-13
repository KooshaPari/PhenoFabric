//! Versioned schema migrations for the persistence layer.
//!
//! Migrations are defined as const structs with UP/DOWN SQL. Each migration
//! is applied exactly once, tracked in the `migrations` table. The system
//! supports rollback to a target version.

use rusqlite::Connection;
use tracing::info;

use crate::error::PersistError;

// ---------------------------------------------------------------------------
// Migration trait
// ---------------------------------------------------------------------------

/// A single versioned migration.
trait Migration {
    fn version(&self) -> i32;
    fn name(&self) -> &str;
    fn up(&self) -> &str;
    fn down(&self) -> &str;
}

// ---------------------------------------------------------------------------
// V1 – initial schema
// ---------------------------------------------------------------------------

struct V1InitialSchema;

impl Migration for V1InitialSchema {
    fn version(&self) -> i32 {
        1
    }
    fn name(&self) -> &str {
        "initial_schema"
    }
    fn up(&self) -> &str {
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
        "
    }
    fn down(&self) -> &str {
        "
        DROP TABLE IF EXISTS audit_log;
        DROP TABLE IF EXISTS evidence_log;
        DROP TABLE IF EXISTS route_plans;
        DROP TABLE IF EXISTS leases;
        DROP TABLE IF EXISTS topology_meta;
        DROP TABLE IF EXISTS topology_edges;
        DROP TABLE IF EXISTS topology_nodes;
        "
    }
}

// ---------------------------------------------------------------------------
// V2 – query indexes
// ---------------------------------------------------------------------------

struct V2QueryIndexes;

impl Migration for V2QueryIndexes {
    fn version(&self) -> i32 {
        2
    }
    fn name(&self) -> &str {
        "query_indexes"
    }
    fn up(&self) -> &str {
        "
        CREATE INDEX IF NOT EXISTS idx_leases_state ON leases(state);
        CREATE INDEX IF NOT EXISTS idx_route_plans_intent ON route_plans(intent_id);
        CREATE INDEX IF NOT EXISTS idx_route_plans_state ON route_plans(state);
        CREATE INDEX IF NOT EXISTS idx_route_plans_epoch ON route_plans(topology_epoch);
        CREATE INDEX IF NOT EXISTS idx_evidence_type ON evidence_log(event_type);
        CREATE INDEX IF NOT EXISTS idx_evidence_observed ON evidence_log(observed_at);
        CREATE INDEX IF NOT EXISTS idx_evidence_producer ON evidence_log(producer_id);
        CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action);
        CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);
        CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor);
        CREATE INDEX IF NOT EXISTS idx_topology_edges_from ON topology_edges(from_node);
        CREATE INDEX IF NOT EXISTS idx_topology_edges_to ON topology_edges(to_node);
        "
    }
    fn down(&self) -> &str {
        "
        DROP INDEX IF EXISTS idx_topology_edges_to;
        DROP INDEX IF EXISTS idx_topology_edges_from;
        DROP INDEX IF EXISTS idx_audit_actor;
        DROP INDEX IF EXISTS idx_audit_resource;
        DROP INDEX IF EXISTS idx_audit_action;
        DROP INDEX IF EXISTS idx_evidence_producer;
        DROP INDEX IF NOT EXISTS idx_evidence_observed;
        DROP INDEX IF EXISTS idx_evidence_type;
        DROP INDEX IF EXISTS idx_route_plans_epoch;
        DROP INDEX IF EXISTS idx_route_plans_state;
        DROP INDEX IF EXISTS idx_route_plans_intent;
        DROP INDEX IF EXISTS idx_leases_state;
        "
    }
}

// ---------------------------------------------------------------------------
// V3 – metadata JSON columns
// ---------------------------------------------------------------------------

struct V3MetadataColumns;

impl Migration for V3MetadataColumns {
    fn version(&self) -> i32 {
        3
    }
    fn name(&self) -> &str {
        "metadata_json_columns"
    }
    fn up(&self) -> &str {
        "
        ALTER TABLE topology_nodes ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        ALTER TABLE topology_edges ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        ALTER TABLE leases ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        ALTER TABLE route_plans ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        ALTER TABLE evidence_log ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        ALTER TABLE audit_log ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
        "
    }
    fn down(&self) -> &str {
        "
        ALTER TABLE audit_log DROP COLUMN metadata;
        ALTER TABLE evidence_log DROP COLUMN metadata;
        ALTER TABLE route_plans DROP COLUMN metadata;
        ALTER TABLE leases DROP COLUMN metadata;
        ALTER TABLE topology_edges DROP COLUMN metadata;
        ALTER TABLE topology_nodes DROP COLUMN metadata;
        "
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

fn all_migrations() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(V1InitialSchema),
        Box::new(V2QueryIndexes),
        Box::new(V3MetadataColumns),
    ]
}

/// Return the latest migration version number.
pub fn latest_version() -> i32 {
    all_migrations()
        .iter()
        .map(|m| m.version())
        .max()
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Core migration functions
// ---------------------------------------------------------------------------

/// Ensure the migrations tracking table exists.
fn ensure_migrations_table(conn: &Connection) -> Result<(), PersistError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

/// Return the current applied schema version (0 if none).
pub fn current_version(conn: &Connection) -> Result<i32, PersistError> {
    ensure_migrations_table(conn)?;
    let v: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(v)
}

/// Run all pending migrations from the current version up to the latest.
///
/// This is idempotent: calling it on an already-migrated database is a no-op.
pub fn migrate(conn: &Connection) -> Result<(), PersistError> {
    ensure_migrations_table(conn)?;
    let current = current_version(conn)?;
    let migrations = all_migrations();

    for m in &migrations {
        if m.version() > current {
            info!(
                version = m.version(),
                name = m.name(),
                "applying migration"
            );
            conn.execute_batch("BEGIN;")?;
            conn.execute_batch(m.up())?;
            conn.execute_batch(&format!(
                "INSERT INTO migrations (version, name, applied_at) VALUES ({}, '{}', datetime('now'));",
                m.version(),
                m.name(),
            ))?;
            conn.execute_batch("COMMIT;")?;
        }
    }

    let final_version = current_version(conn)?;
    info!(version = final_version, "migration complete");
    Ok(())
}

/// Rollback migrations down to `to_version`.
///
/// All migrations with version > `to_version` are rolled back in reverse order.
pub fn rollback(conn: &Connection, to_version: i32) -> Result<(), PersistError> {
    ensure_migrations_table(conn)?;
    let current = current_version(conn)?;
    let migrations = all_migrations();

    // Collect migrations to rollback in reverse order
    let mut to_rollback: Vec<&dyn Migration> = migrations
        .iter()
        .map(|m| m.as_ref())
        .filter(|m| m.version() > to_version && m.version() <= current)
        .collect::<Vec<_>>();
    to_rollback.reverse();

    for m in &to_rollback {
        info!(
            version = m.version(),
            name = m.name(),
            "rolling back migration"
        );
        conn.execute_batch("BEGIN;")?;
        conn.execute_batch(m.down())?;
        conn.execute_batch(&format!(
            "DELETE FROM migrations WHERE version = {};",
            m.version(),
        ))?;
        conn.execute_batch("COMMIT;")?;
    }

    let final_version = current_version(conn)?;
    info!(version = final_version, "rollback complete");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    #[test]
    fn fresh_db_applies_all_migrations() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();
        let v = current_version(&conn).unwrap();
        assert_eq!(v, latest_version(), "should apply all migrations");
    }

    #[test]
    fn idempotent_migration() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();
        let v1 = current_version(&conn).unwrap();
        // Run again - should be a no-op
        migrate(&conn).unwrap();
        let v2 = current_version(&conn).unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn incremental_migration_on_existing_db() {
        let conn = fresh_conn();
        // Apply only V1 manually
        let m = V1InitialSchema;
        conn.execute_batch(m.up()).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );
            INSERT INTO migrations (version, name, applied_at) VALUES (1, '{}', datetime('now'));",
            m.name(),
        ))
        .unwrap();
        assert_eq!(current_version(&conn).unwrap(), 1);

        // Now run full migrate - should apply V2 and V3 only
        migrate(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn rollback_to_zero_removes_all() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), latest_version());

        rollback(&conn, 0).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 0);
    }

    #[test]
    fn rollback_to_version_1() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();

        rollback(&conn, 1).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 1);
    }

    #[test]
    fn rollback_and_re_migrate() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();

        rollback(&conn, 1).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 1);

        migrate(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn migration_records_name_and_timestamp() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM migrations WHERE name IS NOT NULL AND applied_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, latest_version());
    }

    #[test]
    fn v1_schema_tables_exist_after_migration() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();

        let tables = [
            "topology_nodes",
            "topology_edges",
            "topology_meta",
            "leases",
            "route_plans",
            "evidence_log",
            "audit_log",
        ];
        for table in &tables {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "table {table} should exist after migration");
        }
    }

    #[test]
    fn v3_metadata_columns_exist() {
        let conn = fresh_conn();
        migrate(&conn).unwrap();

        // Verify metadata column exists on topology_nodes
        let has_col: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('topology_nodes') WHERE name='metadata'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_col, "topology_nodes.metadata column should exist");
    }
}
