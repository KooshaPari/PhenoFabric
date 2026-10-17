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

use std::collections::HashMap;

use fabric_frame_transport::{Codec, SessionInit};

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
}

impl FrameSession {
    /// Build a session from a validated `session_init` line.
    ///
    /// The client names a `preferred_codec` and the daemon does not transcode,
    /// so the session records the client's preference verbatim. Recording the
    /// client's own value keeps `session_ack` a truthful description of what
    /// this daemon will actually carry; a real encoder negotiation is a
    /// separate increment.
    fn from_init(session_id: u64, init: &SessionInit) -> Self {
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

/// All live streaming sessions, keyed by daemon-assigned session id.
#[derive(Debug, Default)]
pub struct FrameSessionState {
    sessions: HashMap<u64, FrameSession>,
    next_session_id: u64,
}

impl FrameSessionState {
    /// Create an empty session table whose first allocated id is 1.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_session_id: 1,
        }
    }

    /// Allocate a session id and insert a new session.
    pub fn insert_session(&mut self, init: &SessionInit) -> FrameSession {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        let session = FrameSession::from_init(session_id, init);
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

    /// Look up a session by id.
    pub fn get(&self, session_id: u64) -> Option<&FrameSession> {
        self.sessions.get(&session_id)
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
