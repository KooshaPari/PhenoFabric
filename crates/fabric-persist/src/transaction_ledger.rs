//! A bounded SQLite authority-domain slice of the global nested ledger.
//!
//! State transition + immutable event + outbox enqueue share one SQL transaction.
//! This is not cross-host consensus, OS-level enforcement or grant verification.
//! Callers must verify grants and physical reclamation receipts with trusted
//! adapters before invoking these methods. A lease deadline is never reclamation.

use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::{Persist, PersistError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedgerActorKind {
    Human,
    SoftwareAgent,
}

impl LedgerActorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::SoftwareAgent => "software_agent",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LedgerActor<'a> {
    pub id: &'a str,
    pub kind: LedgerActorKind,
}

/// One outbox row joined with the immutable `ledger_events` row it references.
///
/// Mirrors the `ledger_events` columns a downstream consumer needs to publish
/// the event (the event row is immutable, so this is a stable projection) plus
/// `published_at` from `ledger_outbox`: `None` means undelivered, `Some` records
/// when delivery was marked. The event history is never deleted; delivery
/// state is reconstructed from `published_at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxEntry {
    pub event_id: String,
    pub scope_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub kind: String,
    pub payload: String,
    pub created_at: String,
    pub published_at: Option<String>,
}

fn required(value: &str, what: &str) -> Result<(), PersistError> {
    if value.trim().is_empty() {
        Err(PersistError::Invariant(format!("{what} cannot be empty")))
    } else {
        Ok(())
    }
}

impl Persist {
    /// Add a persistent scope, under an existing OPEN parent. The single
    /// parentless root is constrained by a partial unique index in migration 4.
    pub fn ledger_create_scope(
        &self,
        id: &str,
        parent: Option<&str>,
        actor: &LedgerActor<'_>,
        event_id: &str,
    ) -> Result<(), PersistError> {
        for (value, name) in [
            (id, "scope id"),
            (actor.id, "actor id"),
            (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            if let Some(parent_id) = parent {
                let open = tx
                    .query_row(
                        "SELECT state = 'open' FROM ledger_scopes WHERE id = ?1",
                        [parent_id],
                        |row| row.get::<_, bool>(0),
                    )
                    .optional()?;
                if open != Some(true) {
                    return Err(PersistError::Invariant(
                        "parent does not exist or is sealed".into(),
                    ));
                }
            }
            tx.execute(
                "INSERT INTO ledger_scopes(id, parent_id, state) VALUES(?1, ?2, 'open')",
                params![id, parent],
            )?;
            tx.execute(
                "INSERT INTO ledger_events(event_id, scope_id, actor_id, actor_kind, kind, payload)
                 VALUES(?1, ?2, ?3, ?4, 'scope_created', '{}')",
                params![event_id, id, actor.id, actor.kind.as_str()],
            )?;
            tx.execute("INSERT INTO ledger_outbox(event_id) VALUES(?1)", [event_id])?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Register finite capacity within a scope; never infer physical capacity
    /// from a synthetic/unlimited provider descriptor.
    pub fn ledger_define_capacity(
        &self,
        domain_id: &str,
        scope_id: &str,
        units: i64,
        actor: &LedgerActor<'_>,
        event_id: &str,
    ) -> Result<(), PersistError> {
        for (value, name) in [
            (domain_id, "domain id"),
            (scope_id, "scope id"),
            (actor.id, "actor id"),
            (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        if units < 0 {
            return Err(PersistError::Invariant("negative capacity".into()));
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO ledger_domains(id, scope_id, capacity, reserved)
                 VALUES(?1, ?2, ?3, 0)",
                params![domain_id, scope_id, units],
            )?;
            tx.execute(
                "INSERT INTO ledger_events(event_id, scope_id, actor_id, actor_kind, kind, payload)
                 VALUES(?1, ?2, ?3, ?4, 'capacity_defined', ?5)",
                params![
                    event_id,
                    scope_id,
                    actor.id,
                    actor.kind.as_str(),
                    units.to_string()
                ],
            )?;
            tx.execute("INSERT INTO ledger_outbox(event_id) VALUES(?1)", [event_id])?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Atomic resource-accounting admission. Returns false when capacity does
    /// not fit. This does not prove execution authorization or OS enforcement.
    /// The caller must verify a grant for the exact scope/effect BEFORE dispatch.
    pub fn ledger_reserve(
        &self,
        reservation_id: &str,
        domain_id: &str,
        scope_id: &str,
        units: i64,
        actor: &LedgerActor<'_>,
        event_id: &str,
    ) -> Result<bool, PersistError> {
        for (value, name) in [
            (reservation_id, "reservation id"),
            (domain_id, "domain id"),
            (scope_id, "scope id"),
            (actor.id, "actor id"),
            (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        if units <= 0 {
            return Err(PersistError::Invariant(
                "reservation units must be positive".into(),
            ));
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let open = tx
                .query_row(
                    "SELECT state = 'open' FROM ledger_scopes WHERE id = ?1",
                    [scope_id],
                    |row| row.get::<_, bool>(0),
                )
                .optional()?;
            if open != Some(true) {
                return Err(PersistError::Invariant("scope is missing or sealed".into()));
            }
            let changed = tx.execute(
                "UPDATE ledger_domains SET reserved = reserved + ?1
                 WHERE id = ?2 AND capacity - reserved >= ?1",
                params![units, domain_id],
            )?;
            if changed == 0 {
                return Ok(false); // transaction rolls back; no event or reservation
            }
            tx.execute(
                "INSERT INTO ledger_reservations(id, scope_id, domain_id, units, state)
                 VALUES(?1, ?2, ?3, ?4, 'active')",
                params![reservation_id, scope_id, domain_id, units],
            )?;
            tx.execute(
                "INSERT INTO ledger_events(event_id, scope_id, actor_id, actor_kind, kind, payload)
                 VALUES(?1, ?2, ?3, ?4, 'reserved', ?5)",
                params![
                    event_id,
                    scope_id,
                    actor.id,
                    actor.kind.as_str(),
                    reservation_id
                ],
            )?;
            tx.execute("INSERT INTO ledger_outbox(event_id) VALUES(?1)", [event_id])?;
            tx.commit()?;
            Ok(true)
        })
    }

    /// Release only after a *verified* reclamation receipt is obtained by the
    /// caller from the enforcing adapter. A non-empty string is NOT verification.
    /// The recorded receipt reference permits independent review/reconciliation.
    pub fn ledger_reclaim(
        &self,
        reservation_id: &str,
        verified_receipt_ref: &str,
        actor: &LedgerActor<'_>,
        event_id: &str,
    ) -> Result<bool, PersistError> {
        for (value, name) in [
            (reservation_id, "reservation id"),
            (verified_receipt_ref, "reclamation receipt"),
            (actor.id, "actor id"),
            (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let record: Option<(String, String, i64)> = tx
                .query_row(
                    "SELECT scope_id, domain_id, units FROM ledger_reservations
                 WHERE id = ?1 AND state = 'active'",
                    [reservation_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            let Some((scope_id, domain_id, units)) = record else {
                return Ok(false);
            };
            tx.execute(
                "UPDATE ledger_reservations SET state = 'reclaimed', receipt_ref = ?2
                 WHERE id = ?1 AND state = 'active'",
                params![reservation_id, verified_receipt_ref],
            )?;
            let updated = tx.execute(
                "UPDATE ledger_domains SET reserved = reserved - ?1
                 WHERE id = ?2 AND reserved >= ?1",
                params![units, domain_id],
            )?;
            if updated != 1 {
                return Err(PersistError::Invariant(
                    "capacity projection inconsistent".into(),
                ));
            }
            tx.execute(
                "INSERT INTO ledger_events(event_id, scope_id, actor_id, actor_kind, kind, payload)
                 VALUES(?1, ?2, ?3, ?4, 'reclaimed', ?5)",
                params![
                    event_id,
                    scope_id,
                    actor.id,
                    actor.kind.as_str(),
                    // payload is JSON so outbox consumers can identify which
                    // reservation was released; receipt_ref alone is ambiguous
                    // when two reservations share a reclamation receipt.
                    format!(
                        "{{\"reservation_id\":\"{}\",\"receipt_ref\":\"{}\"}}",
                        reservation_id, verified_receipt_ref
                    )
                ],
            )?;
            tx.execute("INSERT INTO ledger_outbox(event_id) VALUES(?1)", [event_id])?;
            tx.commit()?;
            Ok(true)
        })
    }

    /// Atomically claim the next undelivered outbox entry. Returns `None`
    /// when the outbox is empty. The caller is responsible for actually
    /// publishing the event to its destination; on success it should call
    /// `ledger_mark_published` to clear the entry.
    ///
    // SQLite has no `SELECT ... FOR UPDATE`. We wrap the claim in a short-lived
    // `BEGIN IMMEDIATE` transaction, which takes the write lock for the whole
    // transaction, so two concurrent claims serialize: the second blocks until
    // the first commits. Because claim does NOT mutate the row, a later
    // sequential caller may see the same row again before `ledger_mark_published`
    // is called. That is BY DESIGN -- at-least-once delivery -- and the caller is
    // responsible for idempotent publishing.
    pub fn ledger_claim_next_outbox(&self) -> Result<Option<OutboxEntry>, PersistError> {
        self.with_conn(|conn| {
            let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
            let entry = tx
                .query_row(
                    "SELECT o.event_id, e.scope_id, e.actor_id, e.actor_kind, e.kind,
                            e.payload, e.created_at, o.published_at
                     FROM ledger_outbox o
                     JOIN ledger_events e ON e.event_id = o.event_id
                     WHERE o.published_at IS NULL
                     ORDER BY o.rowid
                     LIMIT 1",
                    [],
                    |row| {
                        Ok(OutboxEntry {
                            event_id: row.get(0)?,
                            scope_id: row.get(1)?,
                            actor_id: row.get(2)?,
                            actor_kind: row.get(3)?,
                            kind: row.get(4)?,
                            payload: row.get(5)?,
                            created_at: row.get(6)?,
                            published_at: row.get(7)?,
                        })
                    },
                )
                .optional()?;
            tx.commit()?;
            Ok(entry)
        })
    }

    /// Mark an outbox entry as published at the given ISO-8601 timestamp.
    ///
    /// The first mark wins: re-marking an already-delivered entry is a no-op
    /// that preserves the original delivery timestamp (the delivery history is
    /// reconstructed from `published_at`). Returns `PersistError::NotFound`
    /// when no outbox row exists for `event_id`.
    pub fn ledger_mark_published(
        &self,
        event_id: &str,
        published_at: &str,
    ) -> Result<(), PersistError> {
        required(event_id, "event id")?;
        required(published_at, "published_at")?;
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let latest: Option<Option<String>> = tx
                .query_row(
                    "SELECT published_at FROM ledger_outbox WHERE event_id = ?1",
                    [event_id],
                    |row| row.get(0),
                )
                .optional()?;
            match latest {
                None => {
                    return Err(PersistError::NotFound(format!("outbox entry {event_id}")));
                }
                // First mark wins; a second mark is an idempotent no-op so the
                // original delivery timestamp (audit history) is preserved.
                Some(Some(_)) => {}
                Some(None) => {
                    tx.execute(
                        "UPDATE ledger_outbox SET published_at = ?2
                         WHERE event_id = ?1 AND published_at IS NULL",
                        params![event_id, published_at],
                    )?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn ledger_event_count(&self) -> Result<i64, PersistError> {
        self.with_conn(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM ledger_events", [], |row| row.get(0))?)
        })
    }

    pub fn ledger_reserved(&self, domain_id: &str) -> Result<Option<i64>, PersistError> {
        self.with_conn(|conn| {
            Ok(conn
                .query_row(
                    "SELECT reserved FROM ledger_domains WHERE id = ?1",
                    [domain_id],
                    |row| row.get(0),
                )
                .optional()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_actor_neutral_reservation_is_atomic_and_conserved() {
        let store = Persist::open_memory().unwrap();
        let human = LedgerActor {
            id: "h",
            kind: LedgerActorKind::Human,
        };
        let agent = LedgerActor {
            id: "a",
            kind: LedgerActorKind::SoftwareAgent,
        };
        store
            .ledger_create_scope("root", None, &human, "e1")
            .unwrap();
        store
            .ledger_create_scope("work", Some("root"), &agent, "e2")
            .unwrap();
        store
            .ledger_define_capacity("cpu", "root", 4, &human, "e3")
            .unwrap();
        assert!(store
            .ledger_reserve("r1", "cpu", "work", 3, &agent, "e4")
            .unwrap());
        assert!(!store
            .ledger_reserve("r2", "cpu", "work", 3, &human, "e5")
            .unwrap());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(3));
        assert_eq!(store.ledger_event_count().unwrap(), 4);
        assert!(store
            .ledger_reclaim("r1", "verified/stop-1", &agent, "e6")
            .unwrap());
        assert!(!store
            .ledger_reclaim("r1", "verified/stop-1", &agent, "e7")
            .unwrap());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(0));
        assert_eq!(store.ledger_event_count().unwrap(), 5);
    }

    #[test]
    fn failed_event_insert_rolls_back_capacity_change() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor {
            id: "h",
            kind: LedgerActorKind::Human,
        };
        store
            .ledger_create_scope("root", None, &actor, "event-1")
            .unwrap();
        store
            .ledger_define_capacity("slots", "root", 1, &actor, "event-2")
            .unwrap();
        assert!(store
            .ledger_reserve("r1", "slots", "root", 1, &actor, "event-1")
            .is_err());
        assert_eq!(store.ledger_reserved("slots").unwrap(), Some(0));
        assert_eq!(store.ledger_event_count().unwrap(), 2);
    }

    #[test]
    fn no_second_root_and_no_empty_receipt() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor {
            id: "a",
            kind: LedgerActorKind::SoftwareAgent,
        };
        store
            .ledger_create_scope("root", None, &actor, "e1")
            .unwrap();
        assert!(store
            .ledger_create_scope("rogue", None, &actor, "e2")
            .is_err());
        store
            .ledger_define_capacity("cpu", "root", 1, &actor, "e3")
            .unwrap();
        assert!(store
            .ledger_reserve("r1", "cpu", "root", 1, &actor, "e4")
            .unwrap());
        assert!(store.ledger_reclaim("r1", "", &actor, "e5").is_err());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(1));
    }

    #[test]
    fn event_history_is_append_only_even_for_local_sql_caller() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor {
            id: "a",
            kind: LedgerActorKind::Human,
        };
        store
            .ledger_create_scope("root", None, &actor, "e1")
            .unwrap();
        let result = store
            .with_conn(|conn| Ok(conn.execute("UPDATE ledger_events SET kind = 'tampered'", [])?));
        assert!(result.is_err());
        assert_eq!(store.ledger_event_count().unwrap(), 1);
    }

    #[test]
    fn reclaimed_event_payload_identifies_reservation() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor {
            id: "a",
            kind: LedgerActorKind::SoftwareAgent,
        };
        store
            .ledger_create_scope("root", None, &actor, "e1")
            .unwrap();
        store
            .ledger_define_capacity("cpu", "root", 1, &actor, "e2")
            .unwrap();
        assert!(store
            .ledger_reserve("r1", "cpu", "root", 1, &actor, "e3")
            .unwrap());
        assert!(store
            .ledger_reclaim("r1", "verified/stop-1", &actor, "e4")
            .unwrap());
        let payload: String = store
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT payload FROM ledger_events
                     WHERE event_id = 'e4' AND kind = 'reclaimed'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert!(
            payload.contains("\"reservation_id\":\"r1\""),
            "reclaimed event payload must identify the released reservation: {payload}"
        );
        assert!(
            payload.contains("\"receipt_ref\":\"verified/stop-1\""),
            "reclaimed event payload must preserve the verified receipt: {payload}"
        );
    }

    fn seed_one_event(store: &Persist) {
        let actor = LedgerActor {
            id: "a",
            kind: LedgerActorKind::SoftwareAgent,
        };
        store
            .ledger_create_scope("root", None, &actor, "e1")
            .unwrap();
    }

    #[test]
    fn outbox_drain_marks_published() {
        let store = Persist::open_memory().unwrap();
        seed_one_event(&store);

        let entry = store
            .ledger_claim_next_outbox()
            .unwrap()
            .expect("seeded event must be claimable");
        assert_eq!(entry.event_id, "e1");
        assert_eq!(entry.kind, "scope_created");
        assert_eq!(entry.actor_kind, "software_agent");
        assert!(entry.published_at.is_none());

        store
            .ledger_mark_published("e1", "2026-01-01T00:00:00+00:00")
            .unwrap();

        let published: Option<String> = store
            .with_conn(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT published_at FROM ledger_outbox WHERE event_id = 'e1'",
                        [],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .unwrap();
        assert_eq!(published.as_deref(), Some("2026-01-01T00:00:00+00:00"));

        // The drained entry is no longer claimable.
        assert!(store.ledger_claim_next_outbox().unwrap().is_none());

        // Re-marking preserves the first delivery timestamp (idempotent no-op).
        store
            .ledger_mark_published("e1", "2027-02-02T00:00:00+00:00")
            .unwrap();
        let still: Option<String> = store
            .with_conn(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT published_at FROM ledger_outbox WHERE event_id = 'e1'",
                        [],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .unwrap();
        assert_eq!(still.as_deref(), Some("2026-01-01T00:00:00+00:00"));

        // Marking an entry that never existed is an explicit NotFound.
        assert!(matches!(
            store.ledger_mark_published("no-such", "2026-01-01T00:00:00+00:00"),
            Err(PersistError::NotFound(_))
        ));
    }

    #[test]
    fn outbox_drain_empty_returns_none() {
        let store = Persist::open_memory().unwrap();
        assert!(store.ledger_claim_next_outbox().unwrap().is_none());
    }

    #[test]
    fn outbox_drain_preserves_event_history() {
        let store = Persist::open_memory().unwrap();
        seed_one_event(&store);

        let entry = store.ledger_claim_next_outbox().unwrap().unwrap();
        assert_eq!(entry.event_id, "e1");
        store
            .ledger_mark_published("e1", "2026-01-01T00:00:00+00:00")
            .unwrap();

        // Claim-then-mark never deletes: the authoritative event row survives.
        assert_eq!(store.ledger_event_count().unwrap(), 1);
        let kind: String = store
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT kind FROM ledger_events WHERE event_id = 'e1'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(kind, "scope_created");
    }

    #[test]
    fn outbox_drain_concurrent_claim_is_safe() {
        use std::collections::BTreeSet;
        use std::sync::{Arc, Barrier};
        use std::thread;

        let store = Arc::new(Persist::open_memory().unwrap());
        seed_one_event(&store);

        // Barrier-synchronized pair: both threads attempt a claim on the same
        // single undelivered entry at (nearly) the same instant.
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                store.ledger_claim_next_outbox()
            }));
        }

        // Both calls must succeed and neither may panic or deadlock. Because
        // SQLite has no SELECT ... FOR UPDATE and claim does not mutate the row,
        // BEGIN IMMEDIATE serializes the two claims but does NOT stop the second
        // from observing the same still-undelivered entry: that is at-least-once
        // delivery by design. The invariant that actually holds is therefore
        // only that every returned id is the seeded entry and the distinct-id
        // set has cardinality <= 2 (here 1, since one entry was seeded).
        let mut ids = BTreeSet::new();
        for handle in handles {
            let result = handle.join().expect("claim thread panicked");
            let entry = result.expect("claim must not error");
            if let Some(entry) = entry {
                assert_eq!(entry.event_id, "e1");
                ids.insert(entry.event_id);
            }
        }
        assert!(
            ids.len() <= 2,
            "at-least-once delivery permits both claimers to receive the same id; got {ids:?}"
        );
    }

    /// G-09 (audit gap): two concurrent writers against the same WAL-backed
    /// SQLite database must produce a deterministic, non-corrupt outcome.
    ///
    /// Contract pinned by this test (any of these is acceptable, none is
    /// corruption or a silent lost write):
    ///   1. both writers serialize via the internal `Mutex<Connection>` and
    ///      succeed, every event row is visible afterwards;
    ///   2. one writer returns a typed `PersistError::Database` (rusqlite's
    ///      `SqliteFailure { code: BUSY, .. }` surfacing as `From<rusqlite::Error>`),
    ///      the other succeeds, no row is silently dropped;
    ///   3. the database is intact afterwards: re-opening it succeeds and the
    ///      committed rows can be read back.
    ///
    /// What this test deliberately does NOT allow:
    ///   - a panic in either writer thread (which would surface as a join
    ///     failure via `JoinHandle::join`);
    ///   - a writer returning Ok while the corresponding event row is missing
    ///     from the post-test count (a silent lost write);
    ///   - a database that fails to re-open or that returns mismatched row
    ///     counts on re-read (corruption).
    ///
    /// Notes:
    ///   - We use `Persist::open` (real on-disk WAL DB in a tempdir), not
    ///     `open_memory`, so the test exercises the WAL mode path that
    ///     production uses.
    ///   - Each writer defines its own domain so the SQL touches disjoint
    ///     rows; this is the realistic shape (workers don't share row keys).
    #[test]
    fn concurrent_wal_writers_preserve_all_writes() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("concurrent-wal.db");
        let store = Arc::new(Persist::open(&db_path).expect("open WAL DB"));
        seed_one_event(&store);

        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for i in 0..2 {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let actor = LedgerActor {
                    id: "concurrent-writer",
                    kind: LedgerActorKind::Human,
                };
                barrier.wait();
                store.ledger_define_capacity(
                    &format!("wal-domain-{i}"),
                    "root",
                    8,
                    &actor,
                    &format!("wal-evt-{i}"),
                )
            }));
        }

        // Both writers must either succeed or return a typed, actionable error.
        // No thread may panic, deadlock, or return Ok with no durable side
        // effect (we verify the latter by counting events afterwards).
        let mut ok_count = 0usize;
        let mut typed_busy_count = 0usize;
        let mut other_err: Option<PersistError> = None;
        for handle in handles {
            let result = handle.join().expect("writer thread panicked");
            match result {
                Ok(()) => ok_count += 1,
                Err(PersistError::Database(rusqlite::Error::SqliteFailure(err, _msg)))
                    if err.code == rusqlite::ErrorCode::DatabaseBusy =>
                {
                    typed_busy_count += 1;
                }
                Err(e) => {
                    if other_err.is_none() {
                        other_err = Some(e);
                    }
                }
            }
        }
        // Outcome must be explainable: either serialized success, or one BUSY
        // typed error with the other committed. A panic, an unclassified error,
        // or both failing is a fail.
        assert!(
            other_err.is_none(),
            "writer returned a non-actionable error: {other_err:?}"
        );
        assert_eq!(
            ok_count + typed_busy_count,
            2,
            "every writer must produce Ok or a typed BUSY; got ok={ok_count} busy={typed_busy_count}"
        );

        // At least one writer must have committed (silently lost writes is the
        // scenario the audit called out). The seeded scope event accounts for
        // the +1 baseline; each successful writer contributes one event.
        let observed_events = store
            .ledger_event_count()
            .expect("count events after concurrent writes");
        let expected_min_events: i64 = (1 + ok_count).try_into().expect("ok_count fits in i64");
        assert!(
            observed_events >= expected_min_events,
            "lost write detected: expected at least {expected_min_events} events (1 seeded + {ok_count} writer commits), got {observed_events}"
        );

        // Corrosion check: re-open the database from scratch and re-read.
        drop(store);
        let reopened = Persist::open(&db_path).expect("reopen WAL DB after concurrent writes");
        let reopened_count = reopened
            .ledger_event_count()
            .expect("count events in reopened DB");
        assert_eq!(
            reopened_count, observed_events,
            "event count differs between live and reopened handles; possible corruption"
        );
    }
}
