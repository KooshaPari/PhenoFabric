//! Decision taxonomy for spec 019 (PF-WP-015 surface plane).
//!
//! Mirrors the Go-native checker taxonomy (`cmd/checker/decision.go`)
//! in a Rust-native enum so surface-level call sites can branch on
//! `Decision::Admit | AdmitWithNotes | Reject` without an extra layer
//! of string parsing.

/// Surface admission decision (3-valued).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Decision {
    /// Surface can be exposed unconditionally.
    Admit,
    /// Surface can be exposed, but notes must accompany the binding.
    AdmitWithNotes,
    /// Surface cannot be exposed.
    Reject,
}

impl Decision {
    /// Stable lower-case identifier used in JSON manifests + telemetry.
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Admit => "admit",
            Decision::AdmitWithNotes => "admit_with_notes",
            Decision::Reject => "reject",
        }
    }
}

/// Severity of a single finding/reason inside a Decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// Hard-stop. Presence of a Block finding forces Decision::Reject.
    Block,
    /// Soft-warning. Aggregated to Decision::AdmitWithNotes if present.
    Warn,
    /// Informational. Aggregated to Decision::Admit (no action).
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Block => "block",
            Severity::Warn => "warn",
            Severity::Info => "info",
        }
    }
}

/// Reduce a list of severities to a single Decision.
///
/// Rules:
/// - Any `Block` → `Reject`
/// - Any `Warn` (no `Block`) → `AdmitWithNotes`
/// - Only `Info` (or empty) → `Admit`
pub fn reduce(severities: impl IntoIterator<Item = Severity>) -> Decision {
    let mut saw_warn = false;
    for sev in severities {
        match sev {
            Severity::Block => return Decision::Reject,
            Severity::Warn => saw_warn = true,
            Severity::Info => {}
        }
    }
    if saw_warn {
        Decision::AdmitWithNotes
    } else {
        Decision::Admit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_iterable_is_admit() {
        assert_eq!(reduce([]), Decision::Admit);
        assert_eq!(reduce([Severity::Info, Severity::Info]), Decision::Admit);
    }

    #[test]
    fn warn_lone_is_admit_with_notes() {
        assert_eq!(reduce([Severity::Warn]), Decision::AdmitWithNotes);
    }

    #[test]
    fn warn_among_info_is_admit_with_notes() {
        assert_eq!(
            reduce([Severity::Info, Severity::Warn, Severity::Info]),
            Decision::AdmitWithNotes
        );
    }

    #[test]
    fn block_forces_reject_even_with_other_severities() {
        assert_eq!(
            reduce([Severity::Info, Severity::Block, Severity::Warn]),
            Decision::Reject
        );
    }

    #[test]
    fn as_str_round_trip() {
        for d in [Decision::Admit, Decision::AdmitWithNotes, Decision::Reject] {
            let s = d.as_str();
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
        for s in [Severity::Block, Severity::Warn, Severity::Info] {
            assert!(s.as_str().chars().all(|c| c.is_ascii_lowercase()));
        }
    }
}
