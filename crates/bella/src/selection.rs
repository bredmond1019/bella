//! Selection model: anchor/cursor positions + text extraction + clipboard copy.
//!
//! This module is pure and event-loop-independent — it holds no terminal or
//! application state, making it straightforward to unit-test.

// Items in this module are consumed by later tasks in Block D (Tasks 2–5).
// Suppress dead-code warnings until those tasks wire the module in.
#![allow(dead_code)]

use ratatui::text::Line;

/// A text selection in rendered-line space.
///
/// Both `anchor` and `cursor` are `(row, col)` positions where `row` is the
/// index into `App::lines` and `col` is a **char-column** count (not a byte
/// offset), mirroring the column arithmetic in `ui::apply_span_highlight`.
///
/// Either endpoint may be the drag origin or the current pointer position —
/// call [`Selection::normalized`] to obtain them in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Fixed endpoint set when the drag begins (mouse-down position).
    pub anchor: (usize, usize),
    /// Moving endpoint updated while the mouse is dragged (current pointer position).
    pub cursor: (usize, usize),
}

impl Selection {
    /// Create a new, zero-width selection anchored at `pos`.
    pub fn new(pos: (usize, usize)) -> Self {
        Self {
            anchor: pos,
            cursor: pos,
        }
    }

    /// Returns `(start, end)` ordered top-to-bottom / left-to-right.
    ///
    /// When the user drags upward or to the left the raw `anchor`/`cursor`
    /// values are in reversed order.  `normalized()` always returns the
    /// earlier position as `start` and the later one as `end` so callers
    /// never need to handle reversed ranges.
    pub fn normalized(&self) -> ((usize, usize), (usize, usize)) {
        let (ar, ac) = self.anchor;
        let (cr, cc) = self.cursor;
        if ar < cr || (ar == cr && ac <= cc) {
            ((ar, ac), (cr, cc))
        } else {
            ((cr, cc), (ar, ac))
        }
    }

    /// True when the selection spans zero characters (anchor equals cursor).
    pub fn is_empty(&self) -> bool {
        self.anchor == self.cursor
    }
}

/// Extract the selected text from `lines`.
///
/// Columns are clamped to the actual char-count of each row so an out-of-bounds
/// column never panics.  Multi-row selections are assembled as:
///
/// - **head**: chars from `start_col` to the end of `start_row`
/// - **middle** (zero or more): the complete text of each interior row
/// - **tail**: chars from the start of `end_row` up to `end_col`
///
/// The three sections are joined with `\n`.
pub fn extract_text(lines: &[Line<'_>], sel: &Selection) -> String {
    let ((start_row, start_col), (end_row, end_col)) = sel.normalized();

    if start_row == end_row {
        // Single-row selection.
        let text = row_text(lines, start_row);
        let chars: Vec<char> = text.chars().collect();
        let from = start_col.min(chars.len());
        let to = end_col.min(chars.len());
        chars[from..to].iter().collect()
    } else {
        let mut parts: Vec<String> = Vec::new();

        // Head: from start_col to the end of start_row.
        let head_text = row_text(lines, start_row);
        let head_chars: Vec<char> = head_text.chars().collect();
        let head_from = start_col.min(head_chars.len());
        parts.push(head_chars[head_from..].iter().collect());

        // Middle rows (complete).
        for row in (start_row + 1)..end_row {
            parts.push(row_text(lines, row));
        }

        // Tail: from the start of end_row up to end_col.
        let tail_text = row_text(lines, end_row);
        let tail_chars: Vec<char> = tail_text.chars().collect();
        let tail_to = end_col.min(tail_chars.len());
        parts.push(tail_chars[..tail_to].iter().collect());

        parts.join("\n")
    }
}

/// Copy `text` to the system clipboard via [`arboard`].
///
/// Returns `Ok(())` on success.  If no clipboard backend is available (for
/// example in a headless CI environment) the error is returned as a `String`
/// rather than panicking — callers should surface it through
/// `App::status_message`.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("clipboard write failed: {e}"))
}

/// Collect all span contents from `lines[row]` into a plain `String`.
///
/// Returns an empty string when `row` is out of range (never panics).
fn row_text(lines: &[Line<'_>], row: usize) -> String {
    lines
        .get(row)
        .map(|line| {
            line.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use ratatui::text::{Line, Span};

    use super::{Selection, extract_text};

    /// Build a `Vec<Line<'static>>` from a slice of plain-text strings.
    fn make_lines(rows: &[&str]) -> Vec<Line<'static>> {
        rows.iter()
            .map(|&s| Line::from(vec![Span::raw(s.to_owned())]))
            .collect()
    }

    // --- normalized() ---

    #[test]
    fn normalized_forward_drag_unchanged() {
        let sel = Selection {
            anchor: (1, 2),
            cursor: (3, 4),
        };
        assert_eq!(sel.normalized(), ((1, 2), (3, 4)));
    }

    #[test]
    fn normalized_reversed_drag_swapped() {
        // Drag started lower in the document and moved upward.
        let sel = Selection {
            anchor: (3, 4),
            cursor: (1, 2),
        };
        assert_eq!(sel.normalized(), ((1, 2), (3, 4)));
    }

    #[test]
    fn normalized_same_row_right_to_left() {
        // Drag on the same line from col 5 back to col 2.
        let sel = Selection {
            anchor: (0, 5),
            cursor: (0, 2),
        };
        assert_eq!(sel.normalized(), ((0, 2), (0, 5)));
    }

    #[test]
    fn normalized_same_position_returns_equal_endpoints() {
        let sel = Selection {
            anchor: (2, 3),
            cursor: (2, 3),
        };
        let (s, e) = sel.normalized();
        assert_eq!(s, e);
    }

    // --- extract_text: single-row ---

    #[test]
    fn single_row_extraction() {
        let lines = make_lines(&["hello world"]);
        let sel = Selection {
            anchor: (0, 6),
            cursor: (0, 11),
        };
        assert_eq!(extract_text(&lines, &sel), "world");
    }

    #[test]
    fn single_row_extraction_from_start() {
        let lines = make_lines(&["hello world"]);
        let sel = Selection {
            anchor: (0, 0),
            cursor: (0, 5),
        };
        assert_eq!(extract_text(&lines, &sel), "hello");
    }

    #[test]
    fn single_row_col_end_clamped_to_line_length() {
        // col_end beyond the line length must clamp without panicking.
        let lines = make_lines(&["hi"]);
        let sel = Selection {
            anchor: (0, 0),
            cursor: (0, 100),
        };
        assert_eq!(extract_text(&lines, &sel), "hi");
    }

    #[test]
    fn empty_selection_returns_empty_string() {
        let lines = make_lines(&["hello"]);
        let sel = Selection {
            anchor: (0, 3),
            cursor: (0, 3),
        };
        assert_eq!(extract_text(&lines, &sel), "");
    }

    // --- extract_text: multi-row ---

    #[test]
    fn multi_row_extraction_head_mid_tail() {
        let lines = make_lines(&["abcdef", "ghijkl", "mnopqr"]);
        // anchor at row 0 col 3, cursor at row 2 col 3
        let sel = Selection {
            anchor: (0, 3),
            cursor: (2, 3),
        };
        // head: row 0 from col 3 → "def"
        // mid:  row 1 complete  → "ghijkl"
        // tail: row 2 up to col 3 → "mno"
        assert_eq!(extract_text(&lines, &sel), "def\nghijkl\nmno");
    }

    #[test]
    fn multi_row_extraction_adjacent_rows_no_middle() {
        let lines = make_lines(&["first line", "second line"]);
        let sel = Selection {
            anchor: (0, 6),
            cursor: (1, 6),
        };
        // head: "line" (row 0, col 6..10)
        // tail: "second" (row 1, col 0..6)
        assert_eq!(extract_text(&lines, &sel), "line\nsecond");
    }

    #[test]
    fn reversed_drag_normalizes_and_extracts_correctly() {
        // User dragged from row 1 back up to row 0 (anchor > cursor in doc order).
        let lines = make_lines(&["first line", "second line"]);
        let sel = Selection {
            anchor: (1, 6), // drag started here
            cursor: (0, 6), // dragged up to here
        };
        // normalized: start=(0,6), end=(1,6)
        // head: "line" (row 0, col 6..end)
        // tail: "second" (row 1, col 0..6)
        assert_eq!(extract_text(&lines, &sel), "line\nsecond");
    }

    #[test]
    fn out_of_range_row_returns_empty_string() {
        let lines = make_lines(&["hello"]);
        let sel = Selection {
            anchor: (5, 0),
            cursor: (5, 3),
        };
        // Row 5 does not exist — should return empty without panicking.
        assert_eq!(extract_text(&lines, &sel), "");
    }

    // --- copy_to_clipboard ---

    #[test]
    fn copy_to_clipboard_does_not_panic() {
        // In a headless CI environment the clipboard may not be available.
        // This test only asserts the call does not panic — an Err result is
        // acceptable.  The caller will surface errors via status_message.
        let result = super::copy_to_clipboard("test clipboard content");
        // Discard the result; neither Ok nor Err is a test failure here.
        let _ = result;
    }
}
