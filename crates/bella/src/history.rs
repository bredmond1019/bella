//! Back/forward navigation history stack.
//!
//! `History` is a self-contained, app-independent data structure — no engine
//! or UI dependencies — so it can be unit-tested in isolation and is safe to
//! construct in parallel with other modules.
//!
//! Semantics follow the standard browser model: pushing a new entry while
//! sitting anywhere other than the top of the stack truncates the forward tail
//! before appending.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single visited location: the file that was open and a **source-line
/// anchor** into it (BE.7.D) — not a raw display-line scroll index.
///
/// A raw display-line index means nothing once the line count changes: the
/// same document re-wraps to a different number of display rows at every
/// terminal width, so an index stored at width 80 can point at a wholly
/// different part of the file when restored after a resize. The source line
/// is stable across width — it names a place in the *file*, not a row on
/// screen — so back/forward restores the reader to the same place they were
/// reading, not the same row number. Resolving an anchor back to a display
/// row (for the *current* layout) is the app crate's job, via
/// `bella_engine::markdown::source_line_to_display_row`; this type stays
/// engine- and UI-independent and just carries the number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub path: PathBuf,
    /// Source-line anchor (0-based), not a display-line scroll index.
    pub anchor: usize,
}

impl HistoryEntry {
    pub fn new(path: PathBuf, anchor: usize) -> Self {
        Self { path, anchor }
    }
}

/// A back/forward navigation stack.
///
/// Internally the stack is a `Vec<HistoryEntry>` combined with a cursor that
/// points at the "current" entry (the one visible right now).  The entries
/// *before* the cursor are the back history; the entries *after* it are the
/// forward history.
///
/// ```
/// use std::path::PathBuf;
/// use bella::history::{History, HistoryEntry};
///
/// let mut h = History::new();
/// h.push(HistoryEntry::new(PathBuf::from("a.md"), 0));
/// h.push(HistoryEntry::new(PathBuf::from("b.md"), 5));
///
/// assert!(h.can_back());
/// let prev = h.back().unwrap();
/// assert_eq!(prev.path, PathBuf::from("a.md"));
/// assert_eq!(prev.anchor, 0);
/// ```
#[derive(Debug, Default)]
pub struct History {
    /// All visited entries in chronological order.
    entries: Vec<HistoryEntry>,
    /// Index of the current entry within `entries`.  `None` means no entries
    /// have been pushed yet.
    cursor: Option<usize>,
}

impl History {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new entry.
    ///
    /// If the cursor is not at the end of the list (i.e. the user went back
    /// then navigated somewhere new) all forward entries are discarded first.
    pub fn push(&mut self, entry: HistoryEntry) {
        match self.cursor {
            None => {
                self.entries.push(entry);
                self.cursor = Some(0);
            }
            Some(idx) => {
                // Truncate any forward history.
                self.entries.truncate(idx + 1);
                self.entries.push(entry);
                self.cursor = Some(self.entries.len() - 1);
            }
        }
    }

    /// Move the cursor one step back and return a reference to that entry, or
    /// `None` if already at the beginning.
    pub fn back(&mut self) -> Option<&HistoryEntry> {
        let idx = self.cursor?;
        if idx == 0 {
            return None;
        }
        self.cursor = Some(idx - 1);
        Some(&self.entries[idx - 1])
    }

    /// Move the cursor one step forward and return a reference to that entry,
    /// or `None` if already at the end.
    pub fn forward(&mut self) -> Option<&HistoryEntry> {
        let idx = self.cursor?;
        let next = idx + 1;
        if next >= self.entries.len() {
            return None;
        }
        self.cursor = Some(next);
        Some(&self.entries[next])
    }

    /// Return `true` if `back()` would yield an entry.
    pub fn can_back(&self) -> bool {
        matches!(self.cursor, Some(idx) if idx > 0)
    }

    /// Return `true` if `forward()` would yield an entry.
    pub fn can_forward(&self) -> bool {
        match self.cursor {
            Some(idx) => idx + 1 < self.entries.len(),
            None => false,
        }
    }

    /// Return a reference to the current entry without changing the cursor.
    pub fn current(&self) -> Option<&HistoryEntry> {
        self.cursor.map(|idx| &self.entries[idx])
    }

    /// Peek at the previous entry without moving the cursor.
    pub fn peek_back(&self) -> Option<&HistoryEntry> {
        let idx = self.cursor?;
        if idx == 0 {
            return None;
        }
        Some(&self.entries[idx - 1])
    }

    /// Peek at the next entry without moving the cursor.
    pub fn peek_forward(&self) -> Option<&HistoryEntry> {
        let idx = self.cursor?;
        let next = idx + 1;
        if next >= self.entries.len() {
            return None;
        }
        Some(&self.entries[next])
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{History, HistoryEntry};

    fn entry(name: &str, anchor: usize) -> HistoryEntry {
        HistoryEntry::new(PathBuf::from(name), anchor)
    }

    // --- basic empty state ---

    #[test]
    fn empty_history_cannot_navigate() {
        let mut h = History::new();
        assert!(!h.can_back());
        assert!(!h.can_forward());
        assert!(h.back().is_none());
        assert!(h.forward().is_none());
        assert!(h.current().is_none());
    }

    // --- single entry ---

    #[test]
    fn single_entry_no_navigation() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        assert!(!h.can_back());
        assert!(!h.can_forward());
        assert!(h.back().is_none());
        assert!(h.forward().is_none());
    }

    // --- push / back / forward round-trip ---

    #[test]
    fn push_back_forward_round_trip() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        h.push(entry("b.md", 5));
        h.push(entry("c.md", 10));

        // At "c.md" — can go back twice, not forward.
        assert!(h.can_back());
        assert!(!h.can_forward());

        // back → "b.md"
        let prev = h.back().unwrap();
        assert_eq!(prev.path, PathBuf::from("b.md"));
        assert_eq!(prev.anchor, 5);
        assert!(h.can_back());
        assert!(h.can_forward());

        // back → "a.md"
        let prev = h.back().unwrap();
        assert_eq!(prev.path, PathBuf::from("a.md"));
        assert_eq!(prev.anchor, 0);
        assert!(!h.can_back());
        assert!(h.can_forward());

        // forward → "b.md"
        let next = h.forward().unwrap();
        assert_eq!(next.path, PathBuf::from("b.md"));
        assert_eq!(next.anchor, 5);

        // forward → "c.md"
        let next = h.forward().unwrap();
        assert_eq!(next.path, PathBuf::from("c.md"));
        assert_eq!(next.anchor, 10);

        // No more forward.
        assert!(h.forward().is_none());
    }

    // --- back() at the beginning returns None ---

    #[test]
    fn back_at_start_returns_none() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        assert!(h.back().is_none());
    }

    // --- forward() at the end returns None ---

    #[test]
    fn forward_at_end_returns_none() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        h.push(entry("b.md", 3));
        assert!(h.forward().is_none());
    }

    // --- pushing after back() truncates forward history ---

    #[test]
    fn push_after_back_truncates_forward() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        h.push(entry("b.md", 1));
        h.push(entry("c.md", 2));

        // Go back to "b.md".
        h.back(); // cursor at b.md

        // Navigate somewhere new — forward history (c.md) should be truncated.
        h.push(entry("d.md", 7));

        // d.md is the current entry — no forward.
        assert!(!h.can_forward());
        assert!(h.forward().is_none());

        // Going back lands on b.md, not c.md.
        let prev = h.back().unwrap();
        assert_eq!(prev.path, PathBuf::from("b.md"));

        // Then a.md.
        let prev = h.back().unwrap();
        assert_eq!(prev.path, PathBuf::from("a.md"));

        // c.md is gone.
        assert!(!h.can_back());
    }

    // --- anchor is preserved per entry ---

    #[test]
    fn anchor_is_preserved() {
        let mut h = History::new();
        h.push(entry("first.md", 42));
        h.push(entry("second.md", 0));

        let prev = h.back().unwrap();
        assert_eq!(prev.anchor, 42);

        let next = h.forward().unwrap();
        assert_eq!(next.anchor, 0);
    }

    // --- current() returns the entry at the cursor without moving it ---

    #[test]
    fn current_reflects_cursor() {
        let mut h = History::new();
        h.push(entry("a.md", 0));
        h.push(entry("b.md", 5));

        assert_eq!(h.current().unwrap().path, PathBuf::from("b.md"));

        h.back();
        assert_eq!(h.current().unwrap().path, PathBuf::from("a.md"));
    }

    // --- can_back / can_forward consistency ---

    #[test]
    fn can_flags_consistent_with_navigate_results() {
        let mut h = History::new();
        h.push(entry("x.md", 0));
        h.push(entry("y.md", 0));

        // Forward history is empty at the tip.
        assert_eq!(h.can_forward(), h.forward().is_some());

        // Reset cursor to tip.
        let mut h = History::new();
        h.push(entry("x.md", 0));
        h.push(entry("y.md", 0));
        h.back();
        assert_eq!(h.can_back(), h.back().is_some());
    }
}
