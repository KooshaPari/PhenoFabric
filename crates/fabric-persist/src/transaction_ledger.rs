//! A bounded SQLite authority-domain slice of the global nested ledger.
//!
//! State transition + immutable event + outbox enqueue share one SQL transaction.
//! This is not cross-host consensus, OS-level enforcement or grant verification.
//! Callers must verify grants and physical reclamation receipts with trusted
//! adapters before invoking these methods. A lease deadline is never reclamation.

use rusqlite::{params, OptionalExtension};

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
        for (value, name) in [(id, "scope id"), (actor.id, "actor id"), (event_id, "event id")] {
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
                    return Err(PersistError::Invariant("parent does not exist or is sealed".into()));
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
            (domain_id, "domain id"), (scope_id, "scope id"),
            (actor.id, "actor id"), (event_id, "event id"),
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
                params![event_id, scope_id, actor.id, actor.kind.as_str(), units.to_string()],
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
            (reservation_id, "reservation id"), (domain_id, "domain id"),
            (scope_id, "scope id"), (actor.id, "actor id"), (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        if units <= 0 {
            return Err(PersistError::Invariant("reservation units must be positive".into()));
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let open = tx.query_row(
                "SELECT state = 'open' FROM ledger_scopes WHERE id = ?1",
                [scope_id],
                |row| row.get::<_, bool>(0),
            ).optional()?;
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
                params![event_id, scope_id, actor.id, actor.kind.as_str(), reservation_id],
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
            (reservation_id, "reservation id"), (verified_receipt_ref, "reclamation receipt"),
            (actor.id, "actor id"), (event_id, "event id"),
        ] {
            required(value, name)?;
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let record: Option<(String, String, i64)> = tx.query_row(
                "SELECT scope_id, domain_id, units FROM ledger_reservations
                 WHERE id = ?1 AND state = 'active'",
                [reservation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?;
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
                return Err(PersistError::Invariant("capacity projection inconsistent".into()));
            }
            tx.execute(
                "INSERT INTO ledger_events(event_id, scope_id, actor_id, actor_kind, kind, payload)
                 VALUES(?1, ?2, ?3, ?4, 'reclaimed', ?5)",
                params![event_id, scope_id, actor.id, actor.kind.as_str(), verified_receipt_ref],
            )?;
            tx.execute("INSERT INTO ledger_outbox(event_id) VALUES(?1)", [event_id])?;
            tx.commit()?;
            Ok(true)
        })
    }

    pub fn ledger_event_count(&self) -> Result<i64, PersistError> {
        self.with_conn(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM ledger_events", [], |row| row.get(0))?)
        })
    }

    pub fn ledger_reserved(&self, domain_id: &str) -> Result<Option<i64>, PersistError> {
        self.with_conn(|conn| {
            Ok(conn.query_row(
                "SELECT reserved FROM ledger_domains WHERE id = ?1",
                [domain_id],
                |row| row.get(0),
            ).optional()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_actor_neutral_reservation_is_atomic_and_conserved() {
        let store = Persist::open_memory().unwrap();
        let human = LedgerActor { id: "h", kind: LedgerActorKind::Human };
        let agent = LedgerActor { id: "a", kind: LedgerActorKind::SoftwareAgent };
        store.ledger_create_scope("root", None, &human, "e1").unwrap();
        store.ledger_create_scope("work", Some("root"), &agent, "e2").unwrap();
        store.ledger_define_capacity("cpu", "root", 4, &human, "e3").unwrap();
        assert!(store.ledger_reserve("r1", "cpu", "work", 3, &agent, "e4").unwrap());
        assert!(!store.ledger_reserve("r2", "cpu", "work", 3, &human, "e5").unwrap());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(3));
        assert_eq!(store.ledger_event_count().unwrap(), 4);
        assert!(store.ledger_reclaim("r1", "verified/stop-1", &agent, "e6").unwrap());
        assert!(!store.ledger_reclaim("r1", "verified/stop-1", &agent, "e7").unwrap());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(0));
        assert_eq!(store.ledger_event_count().unwrap(), 5);
    }

    #[test]
    fn failed_event_insert_rolls_back_capacity_change() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor { id: "h", kind: LedgerActorKind::Human };
        store.ledger_create_scope("root", None, &actor, "event-1").unwrap();
        store.ledger_define_capacity("slots", "root", 1, &actor, "event-2").unwrap();
        assert!(store.ledger_reserve("r1", "slots", "root", 1, &actor, "event-1").is_err());
        assert_eq!(store.ledger_reserved("slots").unwrap(), Some(0));
        assert_eq!(store.ledger_event_count().unwrap(), 2);
    }

    #[test]
    fn no_second_root_and_no_empty_receipt() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor { id: "a", kind: LedgerActorKind::SoftwareAgent };
        store.ledger_create_scope("root", None, &actor, "e1").unwrap();
        assert!(store.ledger_create_scope("rogue", None, &actor, "e2").is_err());
        store.ledger_define_capacity("cpu", "root", 1, &actor, "e3").unwrap();
        assert!(store.ledger_reserve("r1", "cpu", "root", 1, &actor, "e4").unwrap());
        assert!(store.ledger_reclaim("r1", "", &actor, "e5").is_err());
        assert_eq!(store.ledger_reserved("cpu").unwrap(), Some(1));
    }

    #[test]
    fn event_history_is_append_only_even_for_local_sql_caller() {
        let store = Persist::open_memory().unwrap();
        let actor = LedgerActor { id: "a", kind: LedgerActorKind::Human };
        store.ledger_create_scope("root", None, &actor, "e1").unwrap();
        let result = store.with_conn(|conn| {
            Ok(conn.execute("UPDATE ledger_events SET kind = 'tampered'", [])?)
        });
        assert!(result.is_err());
        assert_eq!(store.ledger_event_count().unwrap(), 1);
    }
}
