//! A bounded, timestamped, severity-tagged message log.
//!
//! This is the durable half of bella's diagnostic channel (BE.7.K). Bella's
//! only diagnostic output today is `App.status_message: Option<String>`,
//! which the next `load_file` silently clears — whatever it held is gone
//! before the operator can act on it. [`MessageLog`] retains a bounded
//! history of messages across that clear, so a later diagnostics overlay
//! (task 2) can show the operator what actually happened, not just the
//! latest fragment.
//!
//! Scope for this module: the data structure only. Routing bella's existing
//! diagnostic paths (the `ascend` no-op, the walk-error drop count, etc.)
//! into this log is task 3; the overlay that displays it is task 2.

use std::collections::VecDeque;
use std::time::Instant;

// TIMESTAMP SOURCE: `std::time::Instant`, not `std::time::SystemTime`.
//
// The overlay this log feeds only ever needs two things: an ordering
// (newest-first) and a relative age ("3s ago"). `Instant` is monotonic,
// immune to wall-clock adjustments, and already available with no new
// dependency — `Instant::elapsed()` gives the relative age directly.
// `SystemTime` would buy an absolute wall-clock string, but bella has no
// `chrono` (or any other formatting) dependency, and hand-rolling
// `SystemTime`-to-string formatting is not warranted for a debug overlay
// nobody persists (this block is explicitly in-memory only — see
// `out_of_scope` on BE.7.K). No new dependency was added.

/// Severity of a retained diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// One retained diagnostic message.
#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub severity: Severity,
    pub timestamp: Instant,
}

/// Default capacity for a [`MessageLog`] constructed via [`MessageLog::default`].
///
/// Bounded so a long session (or a noisy loop of failures) cannot grow the
/// log without limit; generous enough that the overlay has real history to
/// show rather than just the last couple of entries.
pub const DEFAULT_CAPACITY: usize = 200;

/// A bounded ring of retained [`Message`]s. Oldest entries drop first once
/// `capacity` is reached.
///
/// This is deliberately NOT cleared by `App::load_file` — that asymmetry
/// (the transient status line clears, the log does not) is the entire point
/// of BE.7.K.
#[derive(Debug, Clone)]
pub struct MessageLog {
    capacity: usize,
    entries: VecDeque<Message>,
}

impl MessageLog {
    /// Construct an empty log bounded to `capacity` entries.
    ///
    /// `capacity` of `0` is accepted and behaves as an always-empty log
    /// (every push is immediately evicted) rather than panicking — there is
    /// no invariant that depends on a non-zero bound.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity.min(64)),
        }
    }

    /// The configured maximum number of retained entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of entries currently retained.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no messages have been retained.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Push a new message onto the log, timestamped now. If the log is at
    /// capacity, the oldest entry is dropped first.
    pub fn push(&mut self, text: impl Into<String>, severity: Severity) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(Message {
            text: text.into(),
            severity,
            timestamp: Instant::now(),
        });
    }

    /// The most recently pushed message, if any.
    pub fn latest(&self) -> Option<&Message> {
        self.entries.back()
    }

    /// All retained messages, newest first — the order the diagnostics
    /// overlay (task 2) displays them in.
    pub fn iter_newest_first(&self) -> impl Iterator<Item = &Message> {
        self.entries.iter().rev()
    }
}

impl Default for MessageLog {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_latest_returns_it() {
        let mut log = MessageLog::new(10);
        assert!(log.is_empty());
        log.push("hello", Severity::Info);
        assert_eq!(log.len(), 1);
        let latest = log.latest().expect("latest must be Some after a push");
        assert_eq!(latest.text, "hello");
        assert_eq!(latest.severity, Severity::Info);
    }

    #[test]
    fn ring_bound_exercised_at_the_boundary() {
        let mut log = MessageLog::new(3);
        log.push("a", Severity::Info);
        log.push("b", Severity::Info);
        log.push("c", Severity::Info);
        // Exactly at capacity: nothing dropped yet.
        assert_eq!(log.len(), 3);
        let texts: Vec<&str> = log.iter_newest_first().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["c", "b", "a"]);

        // One past capacity: oldest ("a") must be the one dropped.
        log.push("d", Severity::Info);
        assert_eq!(log.len(), 3, "ring must stay bounded at capacity");
        let texts: Vec<&str> = log.iter_newest_first().map(|m| m.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["d", "c", "b"],
            "oldest entry must drop first, newest must be present"
        );
    }

    #[test]
    fn severity_is_carried_and_readable_for_all_three_levels() {
        let mut log = MessageLog::new(10);
        log.push("info msg", Severity::Info);
        log.push("warn msg", Severity::Warning);
        log.push("error msg", Severity::Error);

        let entries: Vec<&Message> = log.iter_newest_first().collect();
        assert_eq!(entries[0].severity, Severity::Error);
        assert_eq!(entries[0].text, "error msg");
        assert_eq!(entries[1].severity, Severity::Warning);
        assert_eq!(entries[1].text, "warn msg");
        assert_eq!(entries[2].severity, Severity::Info);
        assert_eq!(entries[2].text, "info msg");
    }

    #[test]
    fn zero_capacity_log_retains_nothing() {
        let mut log = MessageLog::new(0);
        log.push("dropped immediately", Severity::Info);
        assert!(log.is_empty());
        assert!(log.latest().is_none());
    }

    #[test]
    fn default_uses_default_capacity() {
        let log = MessageLog::default();
        assert_eq!(log.capacity(), DEFAULT_CAPACITY);
        assert!(log.is_empty());
    }
}
