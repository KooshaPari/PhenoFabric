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
//!
//! # Disconnect and resume
//!
//! Attachment state, the resume outcome types, and the disconnect/resume
//! operations live in [`super::frame_session_lifecycle`], which documents the
//! disconnect policy and the trust model a resume operates under. They are
//! re-exported here so callers keep importing from one module.

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

pub use super::frame_session_lifecycle::{
    FrameSessionDisconnect, ResumeOutcome, ResumeRejection, SessionState,
};

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
/// Bounded on purpose: a diagnostic aid, not a ledger. 64 entries is far more
/// than a client needs to learn the fate of the session it holds, and a ring
/// keeps the memory cost flat however much churns through the daemon.
const TEARDOWN_HISTORY: usize = 64;

/// Maximum decoded payload frames retained per session in
/// [`FrameSession::received_payloads`].
///
/// The sink is what proves the receiver reconstructed the bytes the sender
/// transmitted: the integration test in
/// `crates/fabric-integration-tests/tests/frame_payload_decode_wire.rs`
/// drives the real daemon wire server and drains this vector to assert the
/// exact bytes round-tripped through the live handler. The cap is the same
/// order of magnitude as the rest of the session's per-message bookkeeping
/// (the ack map, the rtt map) and is a deliberately fixed ceiling: a
/// long-running stream is supposed to be reaped by the drain at
/// observation time, not grown without bound. When the cap is reached the
/// oldest entry is dropped (FIFO) so the most recent frames -- the ones
/// most likely to be the ones a client cares about -- stay available.
pub const MAX_RECEIVED_PAYLOADS_PER_SESSION: usize = 1024;

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
    /// Decoded payload bytes from each accepted `frame_data`, in arrival order.
    ///
    /// The frame plane only ever inspects the bytes -- it does not consume
    /// them, because the daemon's job is to record what arrived -- but a
    /// future change (YUV conversion, codec demuxing, keyframe indexing)
    /// has a place to attach the actual payload without re-parsing the
    /// wire envelope. Bounded on the daemon side by
    /// [`MAX_RECEIVED_PAYLOADS_PER_SESSION`] so a long-running session
    /// cannot grow this buffer without bound.
    pub received_payloads: Vec<Vec<u8>>,
    /// Clock reading (ms, same domain as [`FrameSessionState::expire_idle_sessions`])
    /// of the most recent activity on this session.
    ///
    /// Seeded when the session is created and refreshed by every accepted
    /// `frame_data` / `frame_ack`, so the idle sweep measures silence on the
    /// wire rather than session age.
    pub last_activity_ms: u64,
    /// Whether a client is currently attached. Flipped by
    /// `FrameSessionDisconnect`; see [`super::frame_session_lifecycle`].
    pub state: SessionState,
    /// The peer id currently allowed to send input frames to this session.
    ///
    /// Initialized from the `client_id` in `session_init` and updated only by
    /// an explicit `session_transfer` (or, for the test-only `insert_session`,
    /// by the caller). A `frame_data` whose `sender_id` differs is rejected
    /// with `input_owner_mismatch` so a peer that is not the current input
    /// owner cannot drive the stream.
    pub input_owner: String,
    /// The peer id currently entitled to receive the daemon's output for this
    /// session. The line-delimited JSON transport sends responses on the
    /// socket that asked, so the output owner is recorded and reported back
    /// rather than enforced at packet-routing time -- the report is what lets
    /// a downstream consumer verify the daemon's framing decision.
    pub output_owner: String,
    /// The peer id the daemon currently considers the focus holder. Updated
    /// on every accepted `frame_data` to that frame's `sender_id`, so a
    /// silent peer cannot keep focus once another peer takes over input.
    pub focus_owner: String,
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
        // Initial ownership seats go to the session's opener: input is who
        // the daemon will accept frames from, output is who the daemon will
        // report the session's counters back to, and focus tracks the most
        // recent sender (which is the opener, by definition, at t=0).
        let owner = init.client_id.clone();
        Self {
            session_id,
            client_id: owner.clone(),
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
            received_payloads: Vec::new(),
            last_activity_ms: now_ms,
            state: SessionState::Active,
            input_owner: owner.clone(),
            output_owner: owner.clone(),
            focus_owner: owner,
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

    /// Append a decoded payload to the session's sink, evicting the oldest
    /// entry when the cap is reached.
    ///
    /// Called only on a successful decode in
    /// [`crate::wire::frame_handlers::handle_frame_data`]: a refused
    /// decode never reaches the sink, so the sink is the truth about what
    /// the receiver actually reconstructed. FIFO eviction means the
    /// sink always holds the most recent `MAX_RECEIVED_PAYLOADS_PER_SESSION`
    /// frames, not the oldest ones.
    pub fn push_received_payload(&mut self, bytes: Vec<u8>) {
        if self.received_payloads.len() >= MAX_RECEIVED_PAYLOADS_PER_SESSION {
            self.received_payloads.remove(0);
        }
        self.received_payloads.push(bytes);
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

/// A snapshot of a session's three ownership seats.
///
/// Returned by [`FrameSessionState::current_owners`] and by the `Ok` arm of
/// [`FrameSessionState::transfer_ownership`] so a handler can build a
/// `session_transfer_ack` (or a `session_ack` carrying the initial seats)
/// without re-locking the session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOwners {
    /// Peer id allowed to drive input for this session.
    pub input_owner: String,
    /// Peer id entitled to receive this session's output.
    pub output_owner: String,
    /// Peer id the daemon currently considers the focus holder.
    pub focus_owner: String,
}

/// Why an explicit ownership transition was refused.
///
/// A refusal is a control-plane outcome, not a state change: the session
/// keeps its previous owners and the caller can recover. The variant is
/// emitted on the wire as `error` and the message text names what was
/// wrong so the caller can fix the request rather than retry blindly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferRejection {
    /// A required field was missing or malformed upstream of the state
    /// layer. Callers should not normally see this; the schema layer rejects
    /// it first.
    MissingField,
    /// The new owner id was an empty string. An empty owner is a degenerate
    /// transition because the daemon has no peer to attribute frames to, so
    /// it is refused rather than silently recorded.
    DegenerateTransfer,
    /// The session id names a session this daemon never issued (or one
    /// whose teardown has aged out of the bounded history). Reported as
    /// `unknown_session` on the wire for the same reason `frame_data` and
    /// `session_resume` use that name.
    UnknownSession,
    /// The session id names a session this daemon tore down (close, cancel,
    /// or expiry). The session is final; a transfer against a dead id is
    /// not a transfer.
    SessionGone(TeardownReason),
}

impl TransferRejection {
    /// Stable snake_case name used on the wire.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingField => "missing_field",
            Self::DegenerateTransfer => "degenerate_transfer",
            Self::UnknownSession => "unknown_session",
            Self::SessionGone(reason) => match reason {
                TeardownReason::Closed => "session_gone_closed",
                TeardownReason::Cancelled => "session_gone_cancelled",
                TeardownReason::Expired => "session_gone_expired",
            },
        }
    }
}

/// Outcome of an ownership transition that actually took effect.
///
/// Carries the new owners so the response is one read; the session is
/// already updated when this returns, so the caller does not need to
/// re-read it to build the ack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferOutcome {
    /// The session whose ownership changed.
    pub session_id: u64,
    /// The new ownership seats.
    pub owners: SessionOwners,
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
    ///
    /// Passing through here also **re-attaches** a
    /// [`SessionState::Disconnected`] session: a client that reconnects and
    /// resumes by simply sending `frame_data` has demonstrated the session is
    /// live. An explicit `session_resume` makes that transition observable; this
    /// is the safety net for a client that skips it.
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
        session.state = SessionState::Active;
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

    /// Mutable access to a session by id, for
    /// [`super::frame_session_lifecycle::FrameSessionDisconnect`] which owns the
    /// disconnect/resume transitions. Same struct, so no wider trust boundary:
    /// the coordinator's mutex is what guards it.
    pub(super) fn get_mut(&mut self, session_id: u64) -> Option<&mut FrameSession> {
        self.sessions.get_mut(&session_id)
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

    /// Read the current ownership seats of `session_id`.
    ///
    /// Returns `None` for an id the daemon never issued, or one whose
    /// teardown has aged out. The owner ids are cloned out so the caller can
    /// drop the state lock before serializing a response.
    pub fn current_owners(&self, session_id: u64) -> Option<SessionOwners> {
        let session = self.sessions.get(&session_id)?;
        Some(SessionOwners {
            input_owner: session.input_owner.clone(),
            output_owner: session.output_owner.clone(),
            focus_owner: session.focus_owner.clone(),
        })
    }

    /// Drain the decoded payloads that have been accumulated for `session_id`.
    ///
    /// Returns the bytes the receiver reconstructed, in arrival order, and
    /// leaves the session with an empty sink. Returns `None` only when
    /// `session_id` names no live session, in which case the sink is
    /// unreachable. An empty `Vec` from a live session is a normal result:
    /// a session that opened, was configured, and has not yet seen a
    /// `frame_data` accepted by [`super::frame_handlers::handle_frame_data`]
    /// has an empty sink.
    ///
    /// The reachability test
    /// `crates/fabric-integration-tests/tests/frame_payload_decode_wire.rs`
    /// drains this vector after driving the real daemon wire server with a
    /// base64-encoded payload. The drain returns the exact bytes the sender
    /// transmitted -- that round-trip is the proof the decoder is wired
    /// into the live handler and not dead code.
    #[allow(dead_code)] // exercised by `frame_payload_decode_wire.rs`; not by daemon-internal tests
    pub fn drain_received_payloads(&mut self, session_id: u64) -> Option<Vec<Vec<u8>>> {
        let session = self.sessions.get_mut(&session_id)?;
        Some(std::mem::take(&mut session.received_payloads))
    }

    /// Read a snapshot of the decoded payloads that have been accumulated
    /// for `session_id`, without removing them.
    ///
    /// Companion to [`Self::drain_received_payloads`] for callers that
    /// only need to observe (not consume) what was decoded. Returns `None`
    /// only when `session_id` names no live session; an empty `Vec` is
    /// otherwise a normal result.
    #[allow(dead_code)] // companion to `drain_received_payloads`; used by future observers (e.g. keyframe indexer)
    pub fn received_payloads(&self, session_id: u64) -> Option<Vec<Vec<u8>>> {
        self.sessions
            .get(&session_id)
            .map(|s| s.received_payloads.clone())
    }

    /// Explicit ownership transition: hand the input / output / focus seats
    /// to new peer ids.
    ///
    /// This is the only state-layer path that mutates ownership after
    /// `session_init`. The session_id is bearer-grade, so any caller that
    /// knows the id can ask for a transfer; the daemon enforces the
    /// transition's shape (non-empty owners, focus defaults to the new
    /// input owner) and updates the seats atomically. Frames that were
    /// already in flight when the transfer completed observe the new owners
    /// on their next handler pass, because every check goes through the
    /// session's stored seats at read time.
    ///
    /// An empty `input_owner` or `output_owner` is refused as a
    /// [`TransferRejection::DegenerateTransfer`] because a session with no
    /// attributed input or output is a daemon state that no caller can
    /// reason about; the explicit refusal is what lets a bug report point
    /// at the offending request rather than at a later, less informative
    /// symptom.
    ///
    /// An id that names no live session is refused as either
    /// [`TransferRejection::UnknownSession`] (the daemon never issued it)
    /// or [`TransferRejection::SessionGone`] (the daemon issued it and has
    /// since torn it down). The two paths stay distinct for the same
    /// reason `frame_data` keeps them distinct: a client whose session
    /// expired deserves to be told so, not told its id was never valid.
    pub fn transfer_ownership(
        &mut self,
        session_id: u64,
        input_owner: &str,
        output_owner: &str,
        focus_owner: Option<&str>,
    ) -> Result<TransferOutcome, TransferRejection> {
        if input_owner.is_empty() || output_owner.is_empty() {
            return Err(TransferRejection::DegenerateTransfer);
        }
        // Focus falls back to the new input owner so the daemon never has a
        // session whose focus seat points at a peer the input gate would
        // refuse: input is the seat the daemon actually checks, and focus
        // tracks "the most recent legitimate input source".
        let focus = focus_owner.unwrap_or(input_owner);
        if focus.is_empty() {
            return Err(TransferRejection::DegenerateTransfer);
        }
        // Distinguish "this daemon never issued that id" from "this daemon
        // issued it and has since torn it down", using the bounded teardown
        // history the rest of the frame plane already consults. A
        // transfer against a dead id is not a transfer: the session is
        // gone, not pending a new owner.
        if !self.sessions.contains_key(&session_id) {
            return Err(match self.teardown_reason(session_id) {
                Some(reason) => TransferRejection::SessionGone(reason),
                None => TransferRejection::UnknownSession,
            });
        }
        let session = self
            .sessions
            .get_mut(&session_id)
            .expect("contains_key check above guarantees the entry exists");
        session.input_owner = input_owner.to_string();
        session.output_owner = output_owner.to_string();
        session.focus_owner = focus.to_string();
        Ok(TransferOutcome {
            session_id,
            owners: SessionOwners {
                input_owner: session.input_owner.clone(),
                output_owner: session.output_owner.clone(),
                focus_owner: session.focus_owner.clone(),
            },
        })
    }
}

#[cfg(test)]
#[path = "frame_session_state_tests.rs"]
mod tests;

/// Disconnect and resume live in their own test module purely for file size:
/// the tests belong to the same `FrameSessionState`, and splitting them kept
/// both files under the project's 500-line ceiling.
#[cfg(test)]
#[path = "frame_disconnect_state_tests.rs"]
mod disconnect_tests;
