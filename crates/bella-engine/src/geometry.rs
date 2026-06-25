// Derived from zemse/hackmd @ 7650cdc (MIT) — src/tui/events.rs (body_pos:1986, select_word_at:2068, word_span_at_col:2455, point_in:2505)
// Ported to bella-engine: App dependencies removed; pure standalone functions only. Clipboard/status/dict side-effects deferred to Block D.

//! Pure geometry helpers — no `App`, no I/O, no threads.
//!
//! These functions are lifted from `events.rs` in the upstream and refactored
//! so all `App`-dependent reads are passed as explicit parameters. The
//! clipboard write, status update, and macOS dictionary lookup from
//! `select_word_at` are intentionally **not** ported (Block D owns those).

use ratatui::layout::Rect;
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

/// Convert a screen `(col, row)` to a body-local `(content_row_index, local_col)`.
///
/// Returns `None` if the point is outside the viewport or before the
/// line-number gutter.
///
/// - `viewport` — the terminal `Rect` occupied by the document body.
/// - `line_numbers` — whether the line-number gutter is visible.
/// - `line_count` — total rendered lines (used only to size the gutter when
///   `line_numbers` is `true`; equivalent to `rendered.lines.len()`).
/// - `scroll` — current vertical scroll offset (lines scrolled past the top).
/// - `col`, `row` — absolute screen coordinates of the click.
pub fn body_pos(
    viewport: Rect,
    line_numbers: bool,
    line_count: usize,
    scroll: usize,
    col: u16,
    row: u16,
) -> Option<(usize, u16)> {
    if row < viewport.y || row >= viewport.y + viewport.height {
        return None;
    }
    if col < viewport.x {
        return None;
    }
    let line_num_w = if line_numbers {
        (format!("{line_count}").len() + 1) as u16
    } else {
        0
    };
    if col < viewport.x + line_num_w {
        return None;
    }
    let local_row = (row - viewport.y) as usize;
    let local_col = col - viewport.x - line_num_w;
    Some((scroll + local_row, local_col))
}

/// Return the word under the click, along with its display start column and
/// display width.
///
/// Returns `None` when the click lands outside the viewport, on the gutter,
/// on whitespace, or on a line that does not exist in `lines`.
///
/// The clipboard write, status update, and macOS dictionary lookup from the
/// upstream `select_word_at` are **not** included here — those side-effects
/// are deferred to Block D (mouse), which owns `arboard` and the status line.
///
/// - `viewport` — the terminal `Rect` occupied by the document body.
/// - `line_numbers` — whether the line-number gutter is visible.
/// - `scroll` — current vertical scroll offset.
/// - `lines` — rendered lines slice (typically `Rendered::lines`).
/// - `col`, `row` — absolute screen coordinates of the click.
pub fn select_word_at(
    viewport: Rect,
    line_numbers: bool,
    scroll: usize,
    lines: &[Line<'static>],
    col: u16,
    row: u16,
) -> Option<(String, usize, usize)> {
    if !point_in(viewport, col, row) {
        return None;
    }
    let line_num_w = if line_numbers {
        (format!("{}", lines.len()).len() + 1) as u16
    } else {
        0
    };
    let inner_x = viewport.x + line_num_w;
    if col < inner_x {
        return None;
    }
    let local_col = (col - inner_x) as usize;
    let line_idx = scroll + (row - viewport.y) as usize;
    let line = lines.get(line_idx)?;
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    word_span_at_col(&text, local_col)
}

/// Walk left and right from `target_col` in `line` (using display widths) to
/// find the run of non-whitespace characters covering that column, and report
/// its display position: `(word, start_col, width)`.
///
/// Leading and trailing ASCII punctuation is stripped (except `_`, `-`, `/`,
/// `.`, `#`) so identifiers and paths like `src/foo_bar-baz.rs` stay whole.
pub fn word_span_at_col(line: &str, target_col: usize) -> Option<(String, usize, usize)> {
    use unicode_width::UnicodeWidthChar;
    let mut col = 0usize;
    let mut hit_byte: Option<usize> = None;
    for (i, ch) in line.char_indices() {
        let w = ch.width().unwrap_or(0);
        if target_col >= col && target_col < col + w.max(1) {
            hit_byte = Some(i);
            break;
        }
        col += w;
    }
    let hit = hit_byte?;
    if line[hit..].chars().next()?.is_whitespace() {
        return None;
    }
    // Expand to whitespace on both sides.
    let mut start = hit;
    while start > 0 {
        let prev = line[..start].chars().next_back()?;
        if prev.is_whitespace() {
            break;
        }
        start -= prev.len_utf8();
    }
    let mut end = hit;
    for ch in line[hit..].chars() {
        if ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    if end <= start {
        return None;
    }
    // Strip leading/trailing punctuation but keep internal characters (so
    // links/anchors like `src/foo_bar-baz.rs` stay whole).
    let raw = &line[start..end];
    let word = raw.trim_matches(|c: char| {
        c.is_ascii_punctuation() && !matches!(c, '_' | '-' | '/' | '.' | '#')
    });
    if word.is_empty() {
        return None;
    }
    // Byte offset of the trimmed word within the line → its display column.
    let word_start = start + (word.as_ptr() as usize - raw.as_ptr() as usize);
    let start_col = line[..word_start].width();
    Some((word.to_string(), start_col, word.width()))
}

/// Return `true` when the screen point `(col, row)` falls within `rect`.
pub fn point_in(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn make_lines(texts: &[&str]) -> Vec<Line<'static>> {
        texts
            .iter()
            .map(|t| Line::from(Span::styled(t.to_string(), Style::default())))
            .collect()
    }

    // -------------------------------------------------------------------------
    // body_pos tests
    // -------------------------------------------------------------------------

    #[test]
    fn body_pos_in_body_returns_coord() {
        // viewport at (0,0) 80x24, no line numbers, scroll=0
        // click at col=5, row=3 → content row 3, local_col 5
        let vp = rect(0, 0, 80, 24);
        assert_eq!(body_pos(vp, false, 0, 0, 5, 3), Some((3, 5)));
    }

    #[test]
    fn body_pos_accounts_for_scroll() {
        let vp = rect(0, 0, 80, 24);
        // scroll=10 → content_row = 10 + local_row
        assert_eq!(body_pos(vp, false, 0, 10, 0, 0), Some((10, 0)));
    }

    #[test]
    fn body_pos_out_of_bounds_row_returns_none() {
        let vp = rect(0, 0, 80, 24);
        assert_eq!(body_pos(vp, false, 0, 0, 0, 24), None); // row == height
        assert_eq!(body_pos(vp, false, 0, 0, 0, 25), None); // row > height
    }

    #[test]
    fn body_pos_col_before_viewport_returns_none() {
        let vp = rect(5, 0, 80, 24);
        assert_eq!(body_pos(vp, false, 0, 0, 4, 0), None); // col < x
    }

    #[test]
    fn body_pos_gutter_returns_none() {
        // viewport at (0,0), line_numbers=true, 100 lines → gutter = 4 cols ("100 ")
        let vp = rect(0, 0, 80, 24);
        // line_count=100 → format!("100").len()+1 = 4
        assert_eq!(body_pos(vp, true, 100, 0, 3, 0), None); // col inside gutter
        assert_eq!(body_pos(vp, true, 100, 0, 4, 0), Some((0, 0))); // first body col
    }

    #[test]
    fn body_pos_offset_viewport() {
        let vp = rect(2, 3, 60, 20);
        // click at (7, 5) → local_row = 5-3=2, local_col = 7-2=5
        assert_eq!(body_pos(vp, false, 0, 0, 7, 5), Some((2, 5)));
    }

    // -------------------------------------------------------------------------
    // select_word_at tests
    // -------------------------------------------------------------------------

    #[test]
    fn select_word_at_finds_word() {
        let vp = rect(0, 0, 80, 24);
        // line 0: "hello world" — click at col=2 → "hello"
        let lines = make_lines(&["hello world"]);
        let result = select_word_at(vp, false, 0, &lines, 2, 0);
        assert_eq!(result, Some(("hello".to_string(), 0, 5)));
    }

    #[test]
    fn select_word_at_finds_second_word() {
        let vp = rect(0, 0, 80, 24);
        let lines = make_lines(&["hello world"]);
        // click at col=8 → "world" (starts at col 6)
        let result = select_word_at(vp, false, 0, &lines, 8, 0);
        assert_eq!(result, Some(("world".to_string(), 6, 5)));
    }

    #[test]
    fn select_word_at_whitespace_returns_none() {
        let vp = rect(0, 0, 80, 24);
        let lines = make_lines(&["hello world"]);
        // col=5 is the space between "hello" and "world"
        assert_eq!(select_word_at(vp, false, 0, &lines, 5, 0), None);
    }

    #[test]
    fn select_word_at_outside_viewport_returns_none() {
        let vp = rect(0, 0, 80, 10);
        let lines = make_lines(&["hello world"]);
        assert_eq!(select_word_at(vp, false, 0, &lines, 0, 10), None);
    }

    // -------------------------------------------------------------------------
    // word_span_at_col tests
    // -------------------------------------------------------------------------

    #[test]
    fn word_span_simple() {
        assert_eq!(
            word_span_at_col("hello world", 2),
            Some(("hello".to_string(), 0, 5))
        );
    }

    #[test]
    fn word_span_strips_leading_trailing_punctuation() {
        // "(hello)" — parens stripped, word = "hello"
        let r = word_span_at_col("(hello)", 3);
        assert_eq!(r.map(|(w, _, _)| w), Some("hello".to_string()));
    }

    #[test]
    fn word_span_keeps_internal_punctuation() {
        // "src/foo_bar-baz.rs" — all kept
        let line = "see src/foo_bar-baz.rs here";
        let r = word_span_at_col(line, 6);
        assert_eq!(r.map(|(w, _, _)| w), Some("src/foo_bar-baz.rs".to_string()));
    }

    #[test]
    fn word_span_whitespace_returns_none() {
        assert_eq!(word_span_at_col("hello world", 5), None);
    }

    #[test]
    fn word_span_past_end_returns_none() {
        assert_eq!(word_span_at_col("hi", 100), None);
    }

    #[test]
    fn word_span_hash_anchor_kept() {
        // "#anchor" at start — hash is kept
        let r = word_span_at_col("#anchor-link here", 2);
        assert_eq!(r.map(|(w, _, _)| w), Some("#anchor-link".to_string()));
    }
}
