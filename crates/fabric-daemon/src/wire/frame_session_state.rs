//! Frame-streaming session state for the daemon wire server.
//!
//! Holds the negotiated parameters and frame-accounting counters for every
//! live streaming session. Kept in its own module so the handler functions in
//! [`crate::wire::frame_handlers`] stay focused on message handling rather
//! than state bookkeeping.
//!
//! # Where session state lives
//!
//! The `Coordinator` owns one [`FrameSessionState`], behind its own mutex,
//! because frame traffic is high-frequency and must not contend with the
//! topology/lease state that the rest of the daemon holds.
//!
//! # Why ids are server-assigned
//!
//! `session_init` carries a `client_id` (`fabric_frame_transport::SessionInit`,
//! `lib.rs:108-116`) but no session id: the transport's `SessionAck` returns
//! `session_id` (`lib.rs:120-127`) and the client supplies it thereafter. So
//! the daemon allocates ids from 1 upward and the client learns its id from
//! the `session_ack` response.
//!
//! # Session lifecycle
//!
//! A session leaves the map on exactly one of four paths, and every one of
//! them removes the entry rather than flagging it:
//!
//! | Path | Entry point | Who initiates |
//! |---|---|---|
//! | Explicit close | [`FrameSessionState::close_session`] | the client, via `session_close` |
//! | Cancellation | [`FrameSessionState::cancel_session`] | the client, via `session_cancel` |
//! | Idle expiry | [`FrameSessionState::expire_idle_sessions`] | the daemon's sweep, via a clock |
//! | Never | — | a session with no close, cancel, or sweep lives until daemon shutdown |
//!
//! Removal is the whole of teardown. There is no per-session resource outside
//! this map, so a removed entry has released everything it held: counters,
//! recorded RTTs, and the id's slot. That is what makes "cancellation
//! releases state" a checkable property rather than a claim.
//!
//! A removed id is never reused: `next_session_id` only advances. So the two
//! ways a `frame_data` can name a dead session stay distinguishable. The state
//! keeps a bounded [`TeardownReason`] history
//! ([`FrameSessionState::teardown_reason`]) so a client whose session expired
//! gets `session_gone` with `reason: "expired"`, while a client that names an
//! id this daemon never issued gets `unknown_session`. Collapsing both into one
//! error would make a daemon-side expiry look like a client bug.

// `main.rs` declares its own `mod wire;` rather than depending on this crate's
// library target, so the `fabric-daemon` *binary* compiles this module on its
// own -- without `frame_handlers` (which is what calls the session API) and
// without `#[cfg(test)]`. Anything reachable only from the library's handlers
// or from the unit tests is therefore dead as far as the binary is concerned,
// which would fail `cargo clippy --workspace --all-targets -- -D warnings`.
//
// Scoped to `not(test)` so the binary target is silenced while the library
// target and the test target still get real dead-code analysis.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{HashMap, VecDeque};

use fabric_frame_transport::{Codec, SessionInit};

/// Default idle timeout before a session is swept, in milliseconds.
///
/// 120 s is chosen against the wire server's own request timeout (5 s in the
/// test harness, `request_timeout_ms` in production config): a session is
/// re-touched by any `frame_data` or `frame_ack`, and at any plausible frame
/// rate a live session is touched many times per second. A 120 s window is
/// therefore roughly two orders of magnitude above the gap between touches for
/// even a 1 fps session, which absorbs GC pauses, a client-side stall, and a
/// slow client without letting a dead session pin its slot for long.
///
/// The bound is deliberately finite so that a client that vanishes without
/// closing (power loss, dropped link, process kill) cannot leak a session
/// entry for the lifetime of the daemon.
pub const DEFAULT_IDLE_TIMEOUT_MS: u64 = 120_000;

/// Maximum concurrent frame sessions on one daemon.
///
/// 256 is a deliberate ceiling rather than an estimate. Each entry is a small
/// fixed-size record (a negotiated parameter set plus a `u64`-keyed ack map),
/// so 256 sessions is a few hundred kilobytes at worst -- cheap enough that
/// the cap never fires in normal use, and small enough that a client looping
/// on `session_init` cannot grow the map without bound. The daemon's own
/// connection cap (`ServerConfig::max_connections`, 16 in the test harness)
/// already bounds concurrent sockets; this bounds the *session* table, which
/// outlives any single socket because a client can reconnect and keep using an
/// earlier session id.
pub const MAX_CONCURRENT_SESSIONS: usize = 256;

/// How many recently-removed session ids to remember, and why each went.
///
/// Bounded on purpose: this is a diagnostic aid, not a ledger. 64 entries is
/// far more than a client needs to learn the fate of the session it is holding
/// (one round trip after teardown), and a ring keeps the memory cost flat no
/// matter how many sessions churn through the daemon over its lifetime.
const TEARDOWN_HISTORY: usize = 64;

/// Negotiated parameters for one streaming session, as agreed at `session_init`.
///
/// Field names and types come from `fabric_frame_transport::SessionInit`
/// (`lib.rs:108-116`). They are recorded here rather than re-derived so the
/// daemon can report what a session actually negotiated.
#[derive(Debug, Clone)]
pub struct FrameSession {
    /// Server-assigned session id, echoed in every response as `session_id`.
    pub session_id: u64,
    /// The `client_id` the caller sent in `session_init`.
    pub client_id: String,
    /// Codec in use for this session.
    pub codec: Codec,
    /// Negotiated width in pixels.
    pub width: u32,
    /// Negotiated height in pixels.
    pub height: u32,
    /// Negotiated frame rate target.
    pub fps: u32,
    /// Maximum tolerated latency in milliseconds, as requested by the client.
    pub max_latency_ms: u32,
    /// The protocol version the client declared.
    pub version: u32,
    /// Highest frame sequence number accepted on this session so far.
    pub last_seq: Option<u64>,
    /// Total frame bytes accepted on this session.
    pub frames_received: u64,
    /// Total frames accepted on this session.
    pub frame_count: u64,
    /// Sequence numbers acknowledged by the client, mapped to reported RTT.
    pub acks: HashMap<u64, u64>,
    /// Clock reading (ms, same domain as [`FrameSessionState::expire_idle_sessions`])
    /// of the most recent activity on this session.
    ///
    /// Seeded when the session is created and refreshed by every accepted
    /// `frame_data` / `frame_ack`, so the idle sweep measures silence on the
    /// wire rather than session age.
    pub last_activity_ms: u64,
}

impl FrameSession {
    /// Build a session from a validated `session_init` line.
    ///
    /// The client names a `preferred_codec` and the daemon does not transcode,
    /// so the session records the client's preference verbatim. Recording the
    /// client's own value keeps `session_ack` a truthful description of what
    /// this daemon will actually carry; a real encoder negotiation is a
    /// separate increment.
    fn from_init(session_id: u64, init: &SessionInit, now_ms: u64) -> Self {
        Self {
            session_id,
            client_id: init.client_id.clone(),
            codec: init.preferred_codec,
            width: init.width,
            height: init.height,
            fps: init.target_fps,
            max_latency_ms: init.max_latency_ms,
            version: init.version,
            last_seq: None,
            frames_received: 0,
            frame_count: 0,
            acks: HashMap::new(),
            last_activity_ms: now_ms,
        }
    }

    /// Snapshot the counters a handler needs.
    pub fn counters(&self) -> SessionCounters {
        SessionCounters {
            frame_count: self.frame_count,
            frames_received: self.frames_received,
            ack_count: self.acks.len() as u64,
            last_seq: self.last_seq,
        }
    }
}

/// Frame-accounting counters, snapshotted out from under the lock.
///
/// Copied by value so a handler can release the session lock before it
/// serializes a response.
#[derive(Debug, Clone, Copy)]
pub struct SessionCounters {
    /// Frames accepted on this session.
    pub frame_count: u64,
    /// Frame bytes accepted on this session.
    pub frames_received: u64,
    /// Sequence numbers the client has acknowledged.
    pub ack_count: u64,
    /// Highest sequence number accepted so far.
    pub last_seq: Option<u64>,
}

/// Why a session left the table.
///
/// Distinguishing the paths matters to the client-facing error: a session that
/// was explicitly closed and one that expired idle are both "gone", but a
/// client debugging a dropped stream benefits from knowing which happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeardownReason {
    /// The client sent `session_close`.
    Closed,
    /// The client sent `session_cancel` mid-transfer.
    Cancelled,
    /// The idle sweep found no activity within the timeout.
    Expired,
}

impl TeardownReason {
    /// Stable snake_case name used on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }
}

/// Outcome of a teardown attempt against a live session.
///
/// Carries the session's final counters so a `session_close` / `session_cancel`
/// response can report what the session had accumulated before it went away.
/// After teardown those counters exist nowhere else, so this is the only place
/// they can be read. Not `Copy` because it owns the session's `client_id`.
#[derive(Debug, Clone)]
pub struct TeardownOutcome {
    /// Id of the session that was removed.
    pub session_id: u64,
    /// Why it was removed.
    pub reason: TeardownReason,
    /// The session's `client_id`.
    pub client_id: String,
    /// Its final counters.
    pub counters: SessionCounters,
}

/// All live streaming sessions, keyed by daemon-assigned session id.
#[derive(Debug, Default)]
pub struct FrameSessionState {
    sessions: HashMap<u64, FrameSession>,
    next_session_id: u64,
    /// Bounded, newest-last history of removed ids and their teardown reason.
    tombstones: VecDeque<(u64, TeardownReason)>,
}

impl FrameSessionState {
    /// Create an empty session table whose first allocated id is 1.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            tombstones: VecDeque::new(),
        }
    }

    /// Allocate a session id and insert a new session.
    pub fn insert_session(&mut self, init: &SessionInit) -> FrameSession {
        self.insert_session_at(init, 0)
    }

    /// Allocate a session id and insert a new session stamped with `now_ms`.
    ///
    /// The clock is a parameter rather than a call to `SystemTime` so that a
    /// test can place a session at a known point in time and make idle expiry
    /// deterministic instead of sleep-dependent. Production callers pass the
    /// daemon's own clock (`Coordinator::now_ms`).
    pub fn insert_session_at(&mut self, init: &SessionInit, now_ms: u64) -> FrameSession {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        let session = FrameSession::from_init(session_id, init, now_ms);
        self.sessions.insert(session_id, session.clone());
        session
    }

    /// Apply a mutation to a session and return its post-mutation counters.
    ///
    /// Returns `None` when no session has that id, which the handlers turn
    /// into an explicit `unknown_session` error. Counters come back by value
    /// so the caller can drop the lock before building a response.
    pub fn update_session<F>(&mut self, session_id: u64, f: F) -> Option<SessionCounters>
    where
        F: FnOnce(&mut FrameSession),
    {
        let session = self.sessions.get_mut(&session_id)?;
        f(session);
        Some(session.counters())
    }

    /// Apply a mutation that also refreshes the session's idle clock.
    ///
    /// Every message that proves the session is still alive goes through here
    /// rather than [`Self::update_session`], so a live session cannot be swept
    /// for being idle while it is actively streaming. `update_session` stays
    /// available for tests and for bookkeeping-only callers that should not
    /// count as activity.
    pub fn touch_session<F>(
        &mut self,
        session_id: u64,
        now_ms: u64,
        f: F,
    ) -> Option<SessionCounters>
    where
        F: FnOnce(&mut FrameSession),
    {
        let session = self.sessions.get_mut(&session_id)?;
        f(session);
        session.last_activity_ms = now_ms;
        Some(session.counters())
    }

    /// Remove a session on an explicit `session_close`.
    ///
    /// Returns `None` when no session has that id, which the handler turns
    /// into `unknown_session` -- the same error a client gets for a session
    /// that never existed, so a double-close is reported rather than
    /// silently succeeding.
    pub fn close_session(&mut self, session_id: u64) -> Option<TeardownOutcome> {
        self.remove_session(session_id, TeardownReason::Closed)
    }

    /// Remove a session on an explicit `session_cancel`.
    ///
    /// Identical in effect to [`Self::close_session`] -- the session leaves
    /// the map and everything it held is dropped -- but carries its own
    /// [`TeardownReason`] so the client can tell "I asked to stop" from "I
    /// finished normally".
    pub fn cancel_session(&mut self, session_id: u64) -> Option<TeardownOutcome> {
        self.remove_session(session_id, TeardownReason::Cancelled)
    }

    /// Remove every session with no activity for at least `idle_timeout_ms`.
    ///
    /// `now_ms` and each session's `last_activity_ms` must come from the same
    /// clock. The comparison saturates so a clock that steps backwards cannot
    /// make a session look infinitely idle and evict the whole table.
    ///
    /// Returns the ids removed, ascending, so a caller can log exactly what
    /// the sweep took. Expiry is deliberately quiet on the wire: the client
    /// that owned an expired session finds out on its next `frame_data`, which
    /// is a `frame_error` naming the reason.
    pub fn expire_idle_sessions(&mut self, now_ms: u64, idle_timeout_ms: u64) -> Vec<u64> {
        let mut expired: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                now_ms.saturating_sub(session.last_activity_ms) >= idle_timeout_ms
            })
            .map(|(id, _)| *id)
            .collect();
        expired.sort_unstable();

        for id in &expired {
            if self.sessions.remove(id).is_some() {
                self.remember_teardown(*id, TeardownReason::Expired);
            }
        }

        expired
    }

    /// Why a session id was removed, if it was removed recently enough to
    /// still be remembered.
    ///
    /// Returns `None` for an id that was never issued here, or for one whose
    /// tombstone has aged out of the bounded history.
    pub fn teardown_reason(&self, session_id: u64) -> Option<TeardownReason> {
        self.tombstones
            .iter()
            .rev()
            .find(|(id, _)| *id == session_id)
            .map(|(_, reason)| *reason)
    }

    /// Append to the bounded teardown history, dropping the oldest entry when
    /// full.
    fn remember_teardown(&mut self, session_id: u64, reason: TeardownReason) {
        if self.tombstones.len() >= TEARDOWN_HISTORY {
            self.tombstones.pop_front();
        }
        self.tombstones.push_back((session_id, reason));
    }

    /// Shared removal path for close and cancel.
    fn remove_session(
        &mut self,
        session_id: u64,
        reason: TeardownReason,
    ) -> Option<TeardownOutcome> {
        let session = self.sessions.remove(&session_id)?;
        self.remember_teardown(session_id, reason);
        // Counters are read before `client_id` moves out, so the response can
        // report both.
        let counters = session.counters();
        Some(TeardownOutcome {
            session_id,
            reason,
            client_id: session.client_id,
            counters,
        })
    }

    /// Look up a session by id.
    pub fn get(&self, session_id: u64) -> Option<&FrameSession> {
        self.sessions.get(&session_id)
    }

    /// Mutable access to a session, for tests that need to place a session at
    /// a known point in time.
    ///
    /// Test-only on purpose. Production code reaches session fields through
    /// [`Self::touch_session`], which is the only path that also refreshes the
    /// idle clock; a general mutable accessor would be a way to mutate a
    /// session without the daemon noticing it is active.
    #[cfg(test)]
    pub(crate) fn get_mut_for_test(&mut self, session_id: u64) -> Option<&mut FrameSession> {
        self.sessions.get_mut(&session_id)
    }

    /// Number of live sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether no sessions are live.
    ///
    /// Pairs with [`Self::len`] (required by `clippy::len_without_is_empty`)
    /// and used by the `unknown_session` error to distinguish "this daemon has
    /// no sessions at all" from "it has sessions, just not that one".
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Whether `session_id` names a live session.
    pub fn contains(&self, session_id: u64) -> bool {
        self.sessions.contains_key(&session_id)
    }

    /// Whether a new session may be opened.
    ///
    /// `false` once [`MAX_CONCURRENT_SESSIONS`] sessions are live. Checked
    /// before insertion so the cap rejects the (N+1)th session rather than
    /// inserting it and then failing.
    pub fn has_session_capacity(&self) -> bool {
        self.sessions.len() < MAX_CONCURRENT_SESSIONS
    }

    /// The configured cap on concurrent sessions.
    pub fn session_capacity(&self) -> usize {
        MAX_CONCURRENT_SESSIONS
    }

    /// The ids of every live session, ascending.
    ///
    /// Used to make an `unknown_session` error actionable: the client can see
    /// which sessions do exist instead of guessing whether it is behind or
    /// ahead of the server.
    pub fn known_session_ids(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.sessions.keys().copied().collect();
        ids.sort_unstable();
        ids
    }
}

#[cfg(test)]
#[path = "frame_session_state_tests.rs"]
mod tests;
