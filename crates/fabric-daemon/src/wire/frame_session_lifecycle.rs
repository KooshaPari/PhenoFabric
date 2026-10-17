//! Lifecycle types for frame sessions: attachment state and resume outcomes.
//!
//! Split out of `frame_session_state.rs` purely for file size -- these types
//! belong to that module's `FrameSessionState`, but keeping them here brings
//! both files under the project's 500-line ceiling.
//!
//! # The disconnect policy this module encodes
//!
//! **A dropped TCP connection does not destroy a frame session.** Session state
//! lives on the `Coordinator`, not on a socket, so the wire server can drop a
//! connection at any instant (client crash, killed process, severed link, NAT
//! rebind) without the session ceasing to exist. Rather than treating that
//! instant as an implicit teardown, the daemon marks the session
//! [`SessionState::Disconnected`] -- dormant, still fully accounted, still
//! resumable -- and lets the ordinary idle sweep reclaim it.
//!
//! This is the simpler of the two defensible policies, and it is the one
//! chosen, for three concrete reasons:
//!
//! 1. **Tearing down on drop would throw away recoverable work.** The session
//!    state is small and already durable. A client that loses its socket for two
//!    seconds during a network blip should not have to renegotiate a session and
//!    restart its sequence from zero.
//! 2. **Expiry already bounds the leak, and it is the *correct* bound.** The
//!    bounded-reclamation argument for
//!    `FrameSessionState::DEFAULT_IDLE_TIMEOUT_MS` applies verbatim: a vanished
//!    client cannot pin a session entry for the lifetime of the daemon. A
//!    disconnected session is swept by exactly the same
//!    `FrameSessionState::expire_idle_sessions` pass as an active one -- there
//!    is no separate reaper to get wrong, and no path by which a dormant
//!    session outlives the timeout.
//! 3. **The alternative needs a teardown hook the wire server does not have.**
//!    Destroying on drop would mean teaching `handle_connection` which sessions
//!    a given socket owned and removing them when `reader.lines()` ends. That is
//!    ownership state the protocol never modelled -- a session is deliberately
//!    not bound to a connection, which is exactly what let its state survive
//!    reconnects in the first place -- so introducing it would be more
//!    machinery for a *worse* outcome.
//!
//! The cost of this policy is honest and worth stating: a session abandoned by
//! its client keeps a slot for up to the idle timeout. That is bounded by the
//! maximum number of concurrent sessions times a small fixed record, so the
//! worst case is bounded memory, not unbounded growth.
//!
//! # What "proof" a resume is given, and what it is not
//!
//! `FrameSessionState::resume_session` requires exactly one thing: the
//! server-assigned `session_id`. There is no token, nonce, cookie, or
//! signature, because the existing protocol has none to give.
//!
//! That is a deliberate match to the trust level the protocol already operates
//! at, not an escalation. `frame_data`, `frame_ack`, `session_close`, and
//! `session_cancel` are all accepted today based on possession of a
//! daemon-issued, monotonically-allocated, never-reused `session_id`. Resume
//! asks for no proof those messages do not already require. Introducing real
//! authentication here would mean inventing a handshake the transport does not
//! have, which is out of scope and a much larger change.
//!
//! **What resume therefore does NOT provide:** it is bearer-token-grade, not
//! authenticated. Any peer that learns a live `session_id` can resume that
//! session, exactly as it could already stream frames into, close, or cancel
//! it. The one property resume does *not* weaken is that the id is never
//! reissued, so a stale or guessed id cannot land on an unrelated session.

// `main.rs` declares its own `mod wire;` rather than depending on this crate's
// library target, so the `fabric-daemon` *binary* compiles this module on its
// own -- without the resume handler and without `#[cfg(test)]`. A trait method
// that only the library's callers use is therefore dead as far as the binary is
// concerned, which would fail `cargo clippy --workspace --all-targets -- -D
// warnings`.
//
// Scoped to `not(test)` so the binary target is silenced while the library
// target and the test target still get real dead-code analysis.
#![cfg_attr(not(test), allow(dead_code))]

use super::frame_session_state::{SessionCounters, TeardownReason};

/// Whether a live session is currently attached to a client.
///
/// The distinction exists because a session outlives any single TCP
/// connection. See the module docs for why a drop moves a session to
/// [`SessionState::Disconnected`] rather than destroying it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// The session is attached and carrying frames.
    Active,
    /// The client's connection dropped. The session is dormant: fully
    /// accounted, still resumable, and still subject to the idle sweep.
    ///
    /// This is a *marker*, not a gate. A `frame_data` naming a `Disconnected`
    /// session is handled exactly as before -- it touches the session and its
    /// accounting continues, which also clears the marker. An explicit
    /// `session_resume` exists to make the re-attachment *explicit and
    /// observable* to both ends, not to be a precondition for sending frames.
    Disconnected,
}

impl SessionState {
    /// Stable snake_case name used on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disconnected => "disconnected",
        }
    }
}

/// Why a `session_resume` was refused.
///
/// Every variant is a hard failure that leaves the session table exactly as it
/// was: a refused resume never creates a session, never resurrects a torn-down
/// one, and never mutates an existing one's accounting. That is what makes
/// "no half-open state" checkable rather than merely intended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeRejection {
    /// No session with this id was ever issued here, or its tombstone has aged
    /// out of the bounded history.
    Unknown,
    /// The session existed and this daemon issued it, but it was closed,
    /// cancelled, or expired. Resuming it is refused: teardown is final.
    Gone(TeardownReason),
}

/// A successful resume: the session is live again and its accounting continues.
#[derive(Debug, Clone)]
pub struct ResumeOutcome {
    /// Id of the session that was resumed.
    pub session_id: u64,
    /// The session's `client_id`, so the client can confirm it resumed *its own*
    /// session rather than merely *a* session with that id.
    pub client_id: String,
    /// Whether the session had been marked [`SessionState::Disconnected`]
    /// before this resume. A resume of an already-`Active` session succeeds and
    /// reports `was_disconnected: false`; it is idempotent, not an error.
    pub was_disconnected: bool,
    /// The session's counters *at the moment of resume*, carried forward from
    /// before the disconnect. This is the field that proves sequencing did not
    /// restart: `last_seq` is the highest sequence the daemon already accepted,
    /// so the client's next frame must be above it.
    pub counters: SessionCounters,
    /// The session's state after the resume.
    pub state: SessionState,
}

// ---------------------------------------------------------------------------
// Operations on the session table
// ---------------------------------------------------------------------------
//
// These are an extension trait rather than inherent methods on
// `FrameSessionState` so that everything to do with attachment and re-attachment
// reads in one place. The types stay the same and the call sites are unchanged:
// `state.resume_session(...)` resolves here through the `use` in
// `frame_session_state.rs`, which re-exports this module's items.

/// Disconnect and resume operations on the frame-session table.
///
/// Implemented on [`super::frame_session_state::FrameSessionState`].
pub trait FrameSessionDisconnect {
    /// Mark a session dormant because its client's connection dropped.
    ///
    /// This is the whole of the disconnect policy: the session stays in the map
    /// with all of its counters, becomes [`SessionState::Disconnected`], and is
    /// reclaimed later by the ordinary idle sweep. Nothing is destroyed here,
    /// because nothing is destroyed on a drop.
    ///
    /// Deliberately does **not** refresh `last_activity_ms`. A disconnect is the
    /// opposite of activity, and the idle clock should keep running so the
    /// session's remaining lifetime is measured from its last real frame rather
    /// than being silently extended by the drop itself.
    ///
    /// Returns `false` when no live session has that id. Already-disconnected is
    /// not an error: the operation is idempotent and reports `true`, because
    /// "the session is dormant" is the postcondition either way.
    fn mark_disconnected(&mut self, session_id: u64) -> bool;

    /// Mark every session named in `session_ids` dormant.
    ///
    /// Used when a connection drops and the daemon wants to record which
    /// sessions that connection was carrying without tracking that ownership in
    /// the protocol. Ids with no live session are skipped: a disconnect cannot
    /// resurrect a torn-down session, and it must not fail because one of the
    /// ids has since expired.
    ///
    /// Returns the ids actually marked, ascending, so a caller can log exactly
    /// what went dormant.
    fn mark_disconnected_many(&mut self, session_ids: &[u64]) -> Vec<u64>;

    /// Re-attach a dormant session and report the accounting it carries.
    ///
    /// Succeeds only for a session that is **live in the table**: one this
    /// daemon closed, cancelled, or expired is refused with
    /// [`ResumeRejection::Gone`], and an id it never issued with
    /// [`ResumeRejection::Unknown`].
    ///
    /// Sequencing carries forward verbatim -- nothing here clears `last_seq`,
    /// `frame_count`, `frames_received`, or `acks` -- and a refused resume
    /// mutates nothing. Resuming an already-`Active` session is idempotent
    /// (`was_disconnected: false`) so a client that lost its response can
    /// retry. The resume refreshes `last_activity_ms`: it is proof of life.
    fn resume_session(
        &mut self,
        session_id: u64,
        now_ms: u64,
    ) -> Result<ResumeOutcome, ResumeRejection>;

    /// The ids of every session currently marked [`SessionState::Disconnected`],
    /// ascending.
    ///
    /// Used by the wire server's connection-drop path to report what went
    /// dormant, and by tests to assert that a dormant session is still present
    /// in the table (i.e. that a disconnect did not destroy it).
    fn disconnected_session_ids(&self) -> Vec<u64>;
}

impl FrameSessionDisconnect for super::frame_session_state::FrameSessionState {
    fn mark_disconnected(&mut self, session_id: u64) -> bool {
        use super::frame_session_state::FrameSessionState as S;
        match S::get_mut(self, session_id) {
            Some(session) => {
                session.state = SessionState::Disconnected;
                true
            }
            None => false,
        }
    }

    fn mark_disconnected_many(&mut self, session_ids: &[u64]) -> Vec<u64> {
        let mut marked: Vec<u64> = session_ids
            .iter()
            .copied()
            .filter(|id| self.mark_disconnected(*id))
            .collect();
        marked.sort_unstable();
        marked.dedup();
        marked
    }

    fn resume_session(
        &mut self,
        session_id: u64,
        now_ms: u64,
    ) -> Result<ResumeOutcome, ResumeRejection> {
        use super::frame_session_state::FrameSessionState as S;
        let Some(session) = S::get_mut(self, session_id) else {
            // Check the tombstone before giving up, so a session this daemon
            // really did tear down reports *why* rather than claiming it never
            // existed.
            return Err(match S::teardown_reason(self, session_id) {
                Some(reason) => ResumeRejection::Gone(reason),
                None => ResumeRejection::Unknown,
            });
        };

        let was_disconnected = session.state == SessionState::Disconnected;
        let counters = session.counters();
        let client_id = session.client_id.clone();
        session.state = SessionState::Active;
        session.last_activity_ms = now_ms;

        Ok(ResumeOutcome {
            session_id,
            client_id,
            was_disconnected,
            counters,
            state: SessionState::Active,
        })
    }

    fn disconnected_session_ids(&self) -> Vec<u64> {
        use super::frame_session_state::FrameSessionState as S;
        S::known_session_ids(self)
            .into_iter()
            .filter(|id| {
                matches!(
                    S::get(self, *id).map(|s| s.state),
                    Some(SessionState::Disconnected)
                )
            })
            .collect()
    }
}
