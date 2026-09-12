//! SQLite persistence layer for Phenotype Fabric coordinator state (PF-WP-080, R3).
//!
//! Provides durable storage for topology, leases, route plans, evidence, and
//! audit logs. The in-memory graph is the source of truth during operation;
//! SQLite is the durability layer.
//!
//! ## Design
//!
//! - WAL mode for concurrent reads during writes
//! - Async writes during operation, synchronous flush on shutdown
//! - Append-only schema migrations (no down migrations)
//! - Startup recovery loads all active state into memory

mod error;
mod schema;
mod topology;
mod leases;
mod routes;
mod evidence;
mod recovery;

pub use error::PersistError;
pub use recovery::RecoveredState;

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// The main persistence handle. Thread-safe via internal Mutex.
///
/// # Example
///
/// ```no_run
/// use fabric_persist::Persist;
/// use fabric_graph::Topology;
///
/// let persist = Persist::open("state.db")?;
/// let topology = Topology::new();
/// persist.save_topology(&topology)?;
/// let recovered = persist.recover_state()?;
/// # Ok::<(), fabric_persist::PersistError>(())
/// ```
pub struct Persist {
    conn: Mutex<Connection>,
}

impl Persist {
    /// Open or create a SQLite database at the given path.
    /// Enables WAL mode and runs migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistError> {
        let conn = Connection::open(path.as_ref())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let persist = Self {
            conn: Mutex::new(conn),
        };
        persist.run_migrations()?;
        Ok(persist)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self, PersistError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let persist = Self {
            conn: Mutex::new(conn),
        };
        persist.run_migrations()?;
        Ok(persist)
    }

    /// Run all pending schema migrations.
    fn run_migrations(&self) -> Result<(), PersistError> {
        let conn = self.conn.lock().map_err(|e| PersistError::Lock(e.to_string()))?;
        schema::run_migrations(&conn)
    }

    /// Execute a closure with exclusive access to the connection.
    /// Used internally for operations that need transactional guarantees.
    fn with_conn<F, R>(&self, f: F) -> Result<R, PersistError>
    where
        F: FnOnce(&Connection) -> Result<R, PersistError>,
    {
        let conn = self.conn.lock().map_err(|e| PersistError::Lock(e.to_string()))?;
        f(&conn)
    }
}
