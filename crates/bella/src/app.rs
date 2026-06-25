//! App state: holds the source, rendered output, scroll position, and navigation metadata.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bella_engine::{
    CheckboxMap, LinkMap, Theme,
    links::{LinkTarget, TableExpansions},
    markdown::{HeadingInfo, render_with_edit},
};
use ratatui::layout::Rect;
use ratatui::text::Line;

use crate::history::{History, HistoryEntry};
use crate::selection::{self, Selection};

/// Search state while a `/`-search is active or has matches.
#[derive(Debug, Clone)]
pub struct SearchState {
    /// The search query as typed so far.
    pub query: String,
    /// Rendered-line indices that match the query (in document order).
    pub matches: Vec<usize>,
    /// Index into `matches` for the current match.
    pub current: usize,
    /// True while the user is typing the query (search input mode active).
    pub input_mode: bool,
}

impl SearchState {
    /// Create a new, empty search state in input mode.
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current: 0,
            input_mode: true,
        }
    }
}

/// Central application state passed through the event loop.
pub struct App {
    /// Raw markdown source.
    pub src: String,
    /// Rendered display lines (produced by bella-engine).
    pub lines: Vec<Line<'static>>,
    /// Link metadata extracted from the last render.
    pub link_map: LinkMap,
    /// Checkbox metadata extracted from the last render.
    pub checkbox_map: CheckboxMap,
    /// Heading metadata extracted from the last render.
    pub headings: Vec<HeadingInfo>,
    /// Current top display row (scroll offset).
    pub scroll: u16,
    /// Height of the content viewport (body area, excluding status line).
    pub viewport_height: u16,
    /// Terminal width — stored so `load_file` can re-render at the same width.
    pub width: u16,
    /// File path shown in the status line.
    pub file: PathBuf,
    /// Exit flag — set to `true` to terminate the event loop.
    pub should_quit: bool,
    /// Index of the currently focused link (into `link_map.links`), if any.
    pub focused_link: Option<usize>,
    /// Index of the link currently under the mouse pointer (into `link_map.links`), if any.
    pub hovered_link: Option<usize>,
    /// Visual-only set of toggled checkbox indices (into `checkbox_map.items`).
    /// No source mutation — edit-sync stays dormant.
    pub toggled_checkboxes: HashSet<usize>,
    /// Active search state, if any.
    pub search: Option<SearchState>,
    /// Non-fatal status message to display in the status line (e.g. file-not-found).
    /// Cleared on the next successful action that overwrites it.
    pub status_message: Option<String>,
    /// Back/forward navigation history stack (Task 6).
    pub history: History,
    /// Body viewport rectangle — updated after each draw so mouse handlers can
    /// call `body_pos` with accurate geometry.
    pub body_area: Rect,
    /// Pending drag origin: the content position where `Down(Left)` was recorded.
    /// Cleared on `Up(Left)`.  Used to distinguish a plain click from a true drag.
    pub drag_origin: Option<(usize, usize)>,
    /// Active or completed text selection (drag-select or double-click).
    /// `None` when no selection is in progress or completed.
    pub selection: Option<Selection>,
}

impl App {
    /// Construct a new `App` by rendering `src` at `width`.
    ///
    /// `term_height` is the full terminal height; the status line takes 1 row,
    /// so `viewport_height = term_height.saturating_sub(1)`.
    pub fn new(src: String, file: PathBuf, width: u16, term_height: u16) -> Self {
        let viewport_height = term_height.saturating_sub(1);
        let base_dir = file.parent().map(Path::to_path_buf);
        let (lines, link_map, checkbox_map, headings) =
            render_metadata(&src, width, base_dir.as_deref());
        Self {
            src,
            lines,
            link_map,
            checkbox_map,
            headings,
            scroll: 0,
            viewport_height,
            width,
            file,
            should_quit: false,
            focused_link: None,
            hovered_link: None,
            toggled_checkboxes: HashSet::new(),
            search: None,
            status_message: None,
            history: History::new(),
            body_area: Rect::default(),
            drag_origin: None,
            selection: None,
        }
    }

    /// Re-render at a new `width` (called on terminal resize).
    pub fn render(&mut self, width: u16) {
        self.width = width;
        let base_dir = self.file.parent().map(Path::to_path_buf);
        let (lines, link_map, checkbox_map, headings) =
            render_metadata(&self.src, width, base_dir.as_deref());
        self.lines = lines;
        self.link_map = link_map;
        self.checkbox_map = checkbox_map;
        self.headings = headings;
        // Re-clamp scroll in case the new render is shorter.
        self.scroll = self.scroll.min(self.max_scroll());
        // Reset navigation state — line indices change on resize.
        self.focused_link = None;
        self.hovered_link = None;
        self.toggled_checkboxes.clear();
        self.search = None;
        self.drag_origin = None;
        self.selection = None;
    }

    /// Update the viewport height (called after each draw when the body area
    /// is known).  Re-clamps scroll if the viewport shrank.
    pub fn set_viewport_height(&mut self, h: u16) {
        self.viewport_height = h;
        self.scroll = self.scroll.min(self.max_scroll());
    }

    // --- scroll helpers ---

    /// The maximum valid scroll offset so the last line is always reachable.
    pub fn max_scroll(&self) -> u16 {
        (self.lines.len() as u16).saturating_sub(self.viewport_height)
    }

    /// Scroll down by `n` lines (clamped).
    pub fn scroll_down(&mut self, n: u16) {
        self.scroll = self.scroll.saturating_add(n).min(self.max_scroll());
    }

    /// Scroll up by `n` lines (clamped).
    pub fn scroll_up(&mut self, n: u16) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    /// Jump to the top of the document.
    pub fn jump_top(&mut self) {
        self.scroll = 0;
    }

    /// Jump to the bottom of the document.
    pub fn jump_bottom(&mut self) {
        self.scroll = self.max_scroll();
    }

    /// Request application exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Scroll the viewport so that `line` is visible.
    fn scroll_to_line(&mut self, line: u16) {
        let viewport_end = self.scroll.saturating_add(self.viewport_height);
        if line < self.scroll {
            self.scroll = line;
        } else if line >= viewport_end {
            self.scroll = line.saturating_sub(self.viewport_height.saturating_sub(1));
        }
        self.scroll = self.scroll.min(self.max_scroll());
    }

    /// Scroll to `matches[current]`, if search is active and matches is non-empty.
    fn scroll_to_current_match(&mut self) {
        if let Some(line) = self
            .search
            .as_ref()
            .and_then(|s| s.matches.get(s.current))
            .copied()
        {
            self.scroll_to_line(line as u16);
        }
    }

    // --- link focus helpers ---

    /// Advance focus to the next link (wrapping).  No-op when there are no links.
    pub fn focus_next(&mut self) {
        let n = self.link_map.links.len();
        if n == 0 {
            return;
        }
        self.focused_link = Some(match self.focused_link {
            None => 0,
            Some(i) => (i + 1) % n,
        });
        self.scroll_to_focused_link();
    }

    /// Retreat focus to the previous link (wrapping).  No-op when there are no links.
    pub fn focus_prev(&mut self) {
        let n = self.link_map.links.len();
        if n == 0 {
            return;
        }
        self.focused_link = Some(match self.focused_link {
            None => n.saturating_sub(1),
            Some(0) => n - 1,
            Some(i) => i - 1,
        });
        self.scroll_to_focused_link();
    }

    /// Clear link focus (e.g. on `Esc`).
    pub fn clear_focus(&mut self) {
        self.focused_link = None;
    }

    /// If a link is focused and its display line is outside the viewport, scroll
    /// so that line becomes visible.
    pub fn scroll_to_focused_link(&mut self) {
        let Some(idx) = self.focused_link else {
            return;
        };
        let Some(span) = self.link_map.links.get(idx) else {
            return;
        };
        let line = span.line as u16;
        let viewport_start = self.scroll;
        let viewport_end = self.scroll.saturating_add(self.viewport_height);
        if line < viewport_start {
            self.scroll = line;
        } else if line >= viewport_end {
            self.scroll = line.saturating_sub(self.viewport_height.saturating_sub(1));
        }
        // Re-clamp in case of rounding edge.
        self.scroll = self.scroll.min(self.max_scroll());
    }

    // --- link follow (Task 4) ---

    /// Load a new file into the reader.
    ///
    /// Re-renders at the current `width`, resets scroll and navigation state,
    /// and updates `self.file`. Returns a user-facing error string if the file
    /// cannot be read (caller surfaces it via `status_message`).
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), String> {
        let src = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        self.src = src;
        self.file = path;
        let base_dir = self.file.parent().map(Path::to_path_buf);
        let (lines, link_map, checkbox_map, headings) =
            render_metadata(&self.src, self.width, base_dir.as_deref());
        self.lines = lines;
        self.link_map = link_map;
        self.checkbox_map = checkbox_map;
        self.headings = headings;
        self.scroll = 0;
        self.focused_link = None;
        self.hovered_link = None;
        self.toggled_checkboxes.clear();
        self.search = None;
        self.status_message = None;
        self.drag_origin = None;
        self.selection = None;
        Ok(())
    }

    /// Follow the currently-focused link.
    ///
    /// Dispatches on the target type:
    /// - `LocalFile` — load the file via [`Self::load_file`] and reset the reader.
    /// - `Url` — open in the system browser; reader state unchanged.
    /// - `Anchor` — scroll to the heading's line within the current document.
    /// - `FileAnchor` — load the file then scroll to the anchor.
    ///
    /// Returns `Some((prev_file, prev_scroll))` when the open file changed so
    /// that Task 6 can record the previous location in the history stack.
    /// Returns `None` for URL opens (no file change), anchor-only scrolls, and
    /// when no link is focused or file loading fails.
    pub fn follow_focused(&mut self) -> Option<(PathBuf, u16)> {
        let idx = self.focused_link?;
        let span = self.link_map.links.get(idx)?.clone();
        match span.target {
            LinkTarget::LocalFile(path) => {
                let prev = (self.file.clone(), self.scroll);
                if let Err(msg) = self.load_file(path) {
                    self.status_message = Some(msg);
                    return None;
                }
                Some(prev)
            }
            LinkTarget::Url(url) => {
                // Open in the system browser; ignore errors (no browser available is non-fatal).
                let _ = open::that(url);
                None
            }
            LinkTarget::Anchor(slug) => {
                if let Some(&line) = self.link_map.anchors.get(&slug) {
                    self.scroll = (line as u16).min(self.max_scroll());
                }
                None
            }
            LinkTarget::FileAnchor(path, slug) => {
                let prev = (self.file.clone(), self.scroll);
                if let Err(msg) = self.load_file(path) {
                    self.status_message = Some(msg);
                    return None;
                }
                // Scroll to the anchor in the newly-loaded document.
                if let Some(&line) = self.link_map.anchors.get(&slug) {
                    self.scroll = (line as u16).min(self.max_scroll());
                }
                Some(prev)
            }
        }
    }

    // --- history navigation (Task 6) ---

    /// Go back one step in the navigation history.
    ///
    /// Restores the previous file and its recorded scroll position.
    /// No-op when there is no back history.  Does NOT push a new entry —
    /// the history cursor moves without adding to the stack.
    pub fn go_back(&mut self) {
        // Peek first so the cursor only moves if the file loads successfully.
        // If we moved the cursor unconditionally (via history.back()) and
        // load_file then failed, the cursor would be permanently out of sync
        // with the displayed file.
        let Some(HistoryEntry { path, scroll }) = self.history.peek_back().cloned() else {
            return;
        };
        if let Err(msg) = self.load_file(path) {
            self.status_message = Some(msg);
        } else {
            self.history.back();
            self.scroll = scroll.min(self.max_scroll());
        }
    }

    /// Go forward one step in the navigation history.
    ///
    /// Restores the next file and its recorded scroll position.
    /// No-op when there is no forward history.  Does NOT push a new entry.
    pub fn go_forward(&mut self) {
        let Some(HistoryEntry { path, scroll }) = self.history.peek_forward().cloned() else {
            return;
        };
        if let Err(msg) = self.load_file(path) {
            self.status_message = Some(msg);
        } else {
            self.history.forward();
            self.scroll = scroll.min(self.max_scroll());
        }
    }

    // --- mouse hover/click (Task 3) ---

    /// Update `hovered_link` based on the pointer's content position.
    ///
    /// `content_row` and `col` are in rendered-line space (as returned by
    /// `body_pos`). Clears the hover when the pointer is not over any link.
    pub fn hover_at(&mut self, content_row: usize, col: u16) {
        self.hovered_link = self.link_map.at(content_row, col as usize);
    }

    /// Handle a left-click at the given content position.
    ///
    /// - If the click lands on a link, follows it (exactly as the keyboard
    ///   `Enter` path does), recording history.
    /// - If the click lands on a checkbox, toggles its visual state in
    ///   `toggled_checkboxes` (visual-only — no source mutation).
    /// - Otherwise, clears any hover state.
    ///
    /// Returns `Some((prev_file, prev_scroll))` when a file navigation occurred
    /// (passed up to the event loop for history recording), otherwise `None`.
    pub fn click_at(&mut self, content_row: usize, col: u16) -> Option<(PathBuf, u16)> {
        // Check link hit first.
        if let Some(idx) = self.link_map.at(content_row, col as usize) {
            // Mirror the keyboard Follow path: set focused_link then follow.
            self.focused_link = Some(idx);
            return self.follow_focused();
        }
        // Check checkbox hit.
        if let Some(idx) = self.checkbox_map.at(content_row, col as usize) {
            if self.toggled_checkboxes.contains(&idx) {
                self.toggled_checkboxes.remove(&idx);
            } else {
                self.toggled_checkboxes.insert(idx);
            }
        }
        None
    }

    // --- drag-select (Task 4) ---

    /// Start a new drag selection anchored at `(row, col)`.
    ///
    /// Replaces any existing selection.
    pub fn selection_start(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection::new((row, col)));
    }

    /// Update the moving end of the active selection to `(row, col)`.
    ///
    /// No-op when no selection is in progress.
    pub fn selection_update(&mut self, row: usize, col: usize) {
        if let Some(sel) = &mut self.selection {
            sel.cursor = (row, col);
        }
    }

    /// Finish the drag selection: extract the selected text, copy to clipboard,
    /// and record a status message.  The selection is **kept** for visual rendering.
    ///
    /// No-op when there is no selection or it is zero-length.
    pub fn selection_finish(&mut self) {
        let Some(sel) = &self.selection else { return };
        if sel.is_empty() {
            return;
        }
        let text = selection::extract_text(&self.lines, sel);
        match selection::copy_to_clipboard(&text) {
            Ok(()) => {
                let count = text.chars().count();
                self.status_message = Some(format!("Copied {count} chars"));
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
        // Keep self.selection alive for the visual highlight.
    }

    // --- in-document search (Task 5) ---

    /// Enter search-input mode (triggered by `/`).
    pub fn start_search(&mut self) {
        self.search = Some(SearchState::new());
    }

    /// Append a character to the live search query.
    pub fn push_search_char(&mut self, ch: char) {
        if let Some(s) = &mut self.search {
            s.query.push(ch);
        }
    }

    /// Remove the last character from the search query (backspace).
    pub fn pop_search_char(&mut self) {
        if let Some(s) = &mut self.search {
            s.query.pop();
        }
    }

    /// Commit the search: compute match list, leave input mode, scroll to first match.
    ///
    /// A blank query clears the search entirely.
    pub fn commit_search(&mut self) {
        let query = match &self.search {
            Some(s) => s.query.clone(),
            None => return,
        };
        if query.is_empty() {
            self.search = None;
            return;
        }
        let query_lower = query.to_lowercase();
        let matches: Vec<usize> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                text.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();
        if let Some(s) = &mut self.search {
            s.matches = matches;
            s.current = 0;
            s.input_mode = false;
        }
        self.scroll_to_current_match();
    }

    /// Advance to the next match (wrapping), scrolling it into view.
    pub fn search_next(&mut self) {
        if let Some(s) = &mut self.search {
            if s.matches.is_empty() {
                return;
            }
            s.current = (s.current + 1) % s.matches.len();
        } else {
            return;
        }
        self.scroll_to_current_match();
    }

    /// Retreat to the previous match (wrapping), scrolling it into view.
    pub fn search_prev(&mut self) {
        if let Some(s) = &mut self.search {
            if s.matches.is_empty() {
                return;
            }
            let len = s.matches.len();
            s.current = if s.current == 0 {
                len - 1
            } else {
                s.current - 1
            };
        } else {
            return;
        }
        self.scroll_to_current_match();
    }

    /// Cancel the search and clear all search state.
    pub fn cancel_search(&mut self) {
        self.search = None;
    }
}

/// Render `src` and return `(lines, link_map, checkbox_map, headings)`.
fn render_metadata(
    src: &str,
    width: u16,
    base_dir: Option<&Path>,
) -> (Vec<Line<'static>>, LinkMap, CheckboxMap, Vec<HeadingInfo>) {
    let theme = Theme::dark();
    let rendered = render_with_edit(src, base_dir, width, &theme, None, &TableExpansions::new());
    (
        rendered.lines,
        rendered.link_map,
        rendered.checkbox_map,
        rendered.headings,
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::path::PathBuf;

    use super::App;

    fn make_app(line_count: usize, viewport: u16) -> App {
        // Build a doc with `line_count` lines by using that many headings.
        let src = (1..=line_count)
            .map(|i| format!("# Line {i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut app = App::new(src, PathBuf::from("test.md"), 80, viewport + 1);
        // Force exact viewport height (the +1 above accounts for status row).
        app.viewport_height = viewport;
        app
    }

    #[test]
    fn scroll_down_clamps_at_max() {
        let mut app = make_app(20, 5);
        let max = app.max_scroll();
        app.scroll_down(9999);
        assert_eq!(app.scroll, max, "scroll must not exceed max_scroll");
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut app = make_app(20, 5);
        // First go to bottom.
        app.jump_bottom();
        app.scroll_up(9999);
        assert_eq!(app.scroll, 0, "scroll must not go below 0");
    }

    #[test]
    fn to_top_lands_at_zero() {
        let mut app = make_app(20, 5);
        app.jump_bottom();
        app.jump_top();
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn to_bottom_lands_at_max() {
        let mut app = make_app(20, 5);
        let max = app.max_scroll();
        app.jump_bottom();
        assert_eq!(app.scroll, max);
    }

    #[test]
    fn max_scroll_is_zero_when_content_fits() {
        // viewport_height >= number of rendered lines → max_scroll == 0
        let mut app = make_app(3, 50);
        app.viewport_height = 50;
        assert_eq!(
            app.max_scroll(),
            0,
            "max_scroll must be 0 when content fits the viewport"
        );
    }

    // --- Task 1 tests: link/heading metadata + base_dir + nav scaffolding ---

    #[test]
    fn link_map_populated_for_doc_with_links() {
        // A doc with a relative link and an external URL must produce a non-empty link_map.
        let src = "See [local](other.md) and [web](https://example.com).".to_string();
        let file = PathBuf::from("/some/dir/readme.md");
        let app = App::new(src, file, 80, 25);
        assert!(
            !app.link_map.links.is_empty(),
            "link_map must be populated for a doc containing links"
        );
        assert_eq!(
            app.link_map.links.len(),
            2,
            "expected exactly 2 links (local + URL)"
        );
    }

    #[test]
    fn base_dir_equals_file_parent() {
        // Verify that the render received a real base_dir: a relative LocalFile link should
        // be present in the link_map (engine threads base_dir into link resolution).
        let src = "[local](other.md)".to_string();
        let file = PathBuf::from("/projects/docs/index.md");
        let app = App::new(src, file.clone(), 80, 25);
        // The app stores the file path correctly.
        assert_eq!(app.file, file, "app.file must match the provided path");
        // And the render did not discard link metadata.
        assert!(!app.link_map.links.is_empty(), "link_map must not be empty");
    }

    #[test]
    fn navigation_state_defaults_to_none() {
        let src = "[link](other.md)".to_string();
        let app = App::new(src, PathBuf::from("test.md"), 80, 25);
        assert!(
            app.focused_link.is_none(),
            "focused_link must default to None"
        );
        assert!(app.search.is_none(), "search must default to None");
    }

    #[test]
    fn render_resets_nav_state() {
        let src = "[link](other.md)".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        // Manually set navigation state.
        app.focused_link = Some(0);
        // Re-render (e.g. on resize) must reset nav state.
        app.render(80);
        assert!(
            app.focused_link.is_none(),
            "render() must reset focused_link"
        );
        assert!(app.search.is_none(), "render() must reset search");
    }

    #[test]
    fn headings_populated_for_doc_with_headings() {
        let src = "# Heading One\n\nSome text.\n\n## Heading Two\n".to_string();
        let app = App::new(src, PathBuf::from("doc.md"), 80, 25);
        assert!(
            !app.headings.is_empty(),
            "headings must be populated for a doc with headings"
        );
        assert_eq!(app.headings.len(), 2, "expected exactly 2 headings");
        assert_eq!(app.headings[0].text, "Heading One");
        assert_eq!(app.headings[1].text, "Heading Two");
    }

    #[test]
    fn link_map_empty_for_plain_doc() {
        let src = "Just plain text, no links here.".to_string();
        let app = App::new(src, PathBuf::from("plain.md"), 80, 25);
        assert!(
            app.link_map.links.is_empty(),
            "link_map must be empty for a doc with no links"
        );
    }

    // --- Task 3 tests: link focus ring ---

    fn make_link_app() -> App {
        // Doc with two links on separate lines so we can test cycling.
        let src = "[Alpha](a.md)\n\n[Beta](b.md)".to_string();
        App::new(src, PathBuf::from("test.md"), 80, 25)
    }

    #[test]
    fn focus_next_from_none_selects_first_link() {
        let mut app = make_link_app();
        assert!(!app.link_map.links.is_empty(), "precondition: links exist");
        app.focus_next();
        assert_eq!(
            app.focused_link,
            Some(0),
            "focus_next from None must select index 0"
        );
    }

    #[test]
    fn focus_next_wraps_at_end() {
        let mut app = make_link_app();
        let n = app.link_map.links.len();
        // Advance past the last link.
        for _ in 0..n {
            app.focus_next();
        }
        // One more wrap should land back at 0.
        app.focus_next();
        assert_eq!(
            app.focused_link,
            Some(0),
            "focus_next must wrap back to index 0 after the last link"
        );
    }

    #[test]
    fn focus_prev_wraps_backward() {
        let mut app = make_link_app();
        let n = app.link_map.links.len();
        // focus_prev from None should select the last link.
        app.focus_prev();
        assert_eq!(
            app.focused_link,
            Some(n - 1),
            "focus_prev from None must wrap to the last link"
        );
    }

    #[test]
    fn focus_prev_from_first_wraps_to_last() {
        let mut app = make_link_app();
        let n = app.link_map.links.len();
        app.focused_link = Some(0);
        app.focus_prev();
        assert_eq!(
            app.focused_link,
            Some(n - 1),
            "focus_prev from index 0 must wrap to the last link"
        );
    }

    #[test]
    fn focus_next_noop_without_links() {
        let src = "Just plain text.".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.focus_next();
        assert_eq!(
            app.focused_link, None,
            "focus_next must be a no-op when there are no links"
        );
    }

    #[test]
    fn focus_prev_noop_without_links() {
        let src = "Just plain text.".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.focus_prev();
        assert_eq!(
            app.focused_link, None,
            "focus_prev must be a no-op when there are no links"
        );
    }

    #[test]
    fn clear_focus_clears_focused_link() {
        let mut app = make_link_app();
        app.focus_next();
        assert!(app.focused_link.is_some(), "precondition: focus set");
        app.clear_focus();
        assert_eq!(
            app.focused_link, None,
            "clear_focus must set focused_link to None"
        );
    }

    #[test]
    fn scroll_to_focused_link_brings_off_screen_link_into_view() {
        // Create a doc with a link far below the initial viewport.
        let mut lines = (1..=30).map(|i| format!("line {i}")).collect::<Vec<_>>();
        lines.push("[deep link](deep.md)".to_string());
        let src = lines.join("\n\n");
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 11);
        app.viewport_height = 10;

        // Focus the last (deep) link — it's well below the viewport.
        let n = app.link_map.links.len();
        assert!(n > 0, "precondition: link exists");
        app.focused_link = Some(n - 1);
        let link_line = app.link_map.links[n - 1].line as u16;

        app.scroll_to_focused_link();

        let viewport_start = app.scroll;
        let viewport_end = app.scroll + app.viewport_height;
        assert!(
            link_line >= viewport_start && link_line < viewport_end,
            "focused link (line {link_line}) must be visible after scroll_to_focused_link \
             (scroll={}, height={})",
            app.scroll,
            app.viewport_height
        );
    }

    #[test]
    fn app_with_temp_file_uses_correct_base_dir() {
        // Create a real temp file so base_dir is a real directory.
        let dir = std::env::temp_dir();
        let file_path = dir.join("bella_test_base_dir.md");
        let src = "[rel](sibling.md)".to_string();
        {
            let mut f = std::fs::File::create(&file_path).expect("create temp file");
            f.write_all(src.as_bytes()).expect("write temp file");
        }
        let app = App::new(src, file_path.clone(), 80, 25);
        // The engine received the parent dir as base_dir — confirm the link was picked up.
        assert!(!app.link_map.links.is_empty(), "link must be found");
        // Clean up.
        let _ = std::fs::remove_file(&file_path);
    }

    // --- Task 4 tests: link follow ---

    /// Write a temp file and return its path.  The file is created relative to a
    /// unique temp directory so tests don't collide with each other.
    fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write temp file");
        path
    }

    #[test]
    fn follow_local_file_link_swaps_app_file_and_rerenders() {
        let dir = tempdir_for_test("follow_local");
        // Write the target file.
        let target_content = "# Target File\n\nThis is the target.";
        write_temp_file(&dir, "target.md", target_content);

        // Create main file with a link to target.md.
        let main_path = write_temp_file(&dir, "main.md", "[go](target.md)");
        let src = std::fs::read_to_string(&main_path).unwrap();
        let mut app = App::new(src, main_path.clone(), 80, 25);

        // Precondition: links present and first link is a LocalFile.
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);

        let original_file = app.file.clone();
        let result = app.follow_focused();

        // Navigation happened — result contains previous location.
        assert!(
            result.is_some(),
            "follow_focused must return previous location"
        );
        let (prev_file, _prev_scroll) = result.unwrap();
        assert_eq!(
            prev_file, original_file,
            "returned prev_file must be the original"
        );

        // App now shows the target file.
        assert_eq!(
            app.file,
            dir.join("target.md"),
            "app.file must be updated to the followed path"
        );
        // Lines reflect the new file's content.
        let all_text: String = app
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            all_text.contains("Target File"),
            "re-rendered lines must contain the target file's heading; got: {all_text:?}"
        );
        // Scroll reset.
        assert_eq!(app.scroll, 0, "scroll must be reset after following a link");
    }

    #[test]
    fn follow_anchor_scrolls_to_heading_line() {
        // Build a document with a heading far below the viewport and an anchor link.
        // Use a small viewport so the anchor is well below the initial scroll position.
        let mut lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
        lines.push("[jump](#deep-anchor)".to_string());
        lines.push(String::new());
        lines.push("## Deep Anchor".to_string());
        let src = lines.join("\n\n");

        let mut app = App::new(src, PathBuf::from("doc.md"), 80, 11);
        app.viewport_height = 10;

        // Precondition: an anchor link exists.
        let anchor_idx = app
            .link_map
            .links
            .iter()
            .position(|s| matches!(&s.target, bella_engine::links::LinkTarget::Anchor(_)))
            .expect("anchor link must exist");
        app.focused_link = Some(anchor_idx);

        // The anchor "deep-anchor" must resolve to some line.
        let anchor_line = *app
            .link_map
            .anchors
            .get("deep-anchor")
            .expect("anchor must be in link_map.anchors");

        // The expected scroll is the anchor line clamped to max_scroll (scroll cannot exceed
        // max_scroll even if the anchor is on the very last line).
        let max = app.max_scroll();
        let expected = (anchor_line as u16).min(max);

        app.follow_focused();

        assert_eq!(
            app.scroll, expected,
            "scroll must land on the anchor's line ({anchor_line}), clamped to max_scroll ({max}); got {}",
            app.scroll
        );
        // Verify scroll moved — it must be greater than the initial scroll of 0.
        assert!(app.scroll > 0, "scroll must have moved toward the anchor");
    }

    #[test]
    fn follow_url_leaves_file_and_lines_unchanged() {
        // Doc with a URL link.
        let src = "[web](https://example.com)".to_string();
        let file = PathBuf::from("readme.md");
        let mut app = App::new(src.clone(), file.clone(), 80, 25);

        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);

        let lines_before = app.lines.clone();
        let result = app.follow_focused();

        // URL follow returns None (no file navigation).
        assert!(
            result.is_none(),
            "follow_focused on a URL must return None (no file navigation)"
        );
        // File and lines are unchanged.
        assert_eq!(app.file, file, "app.file must not change for a URL link");
        assert_eq!(
            app.lines.len(),
            lines_before.len(),
            "lines must not change for a URL link"
        );
    }

    #[test]
    fn follow_missing_file_sets_status_message_and_returns_none() {
        let src = "[missing](nonexistent_file_that_does_not_exist.md)".to_string();
        let mut app = App::new(src, PathBuf::from("doc.md"), 80, 25);

        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);

        let result = app.follow_focused();

        assert!(
            result.is_none(),
            "follow_focused on a missing file must return None"
        );
        assert!(
            app.status_message.is_some(),
            "status_message must be set on a file-not-found error"
        );
    }

    #[test]
    fn follow_noop_when_no_link_focused() {
        let src = "[link](other.md)".to_string();
        let file = PathBuf::from("doc.md");
        let mut app = App::new(src, file.clone(), 80, 25);
        // No focused link.
        assert!(app.focused_link.is_none());

        let result = app.follow_focused();
        assert!(
            result.is_none(),
            "follow_focused with no focus must return None"
        );
        assert_eq!(app.file, file, "file must not change");
    }

    #[test]
    fn load_file_updates_src_and_resets_state() {
        let dir = tempdir_for_test("load_file");
        let target_content = "# Loaded\n\nSome content.";
        let target_path = write_temp_file(&dir, "loaded.md", target_content);

        let mut app = App::new("Original".to_string(), PathBuf::from("orig.md"), 80, 25);
        app.scroll = 5;
        app.focused_link = Some(0);

        app.load_file(target_path.clone())
            .expect("load_file must succeed");

        assert_eq!(app.file, target_path, "file must be updated");
        assert_eq!(app.scroll, 0, "scroll must be reset");
        assert!(app.focused_link.is_none(), "focused_link must be reset");
        // Lines reflect the loaded content.
        let all_text: String = app
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            all_text.contains("Loaded"),
            "lines must reflect loaded content"
        );
    }

    /// Create a uniquely-named temp directory to avoid inter-test collisions.
    fn tempdir_for_test(label: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("bella_task4_{label}"));
        std::fs::create_dir_all(&base).expect("create temp dir");
        base
    }

    // --- Task 5 tests: in-document search ---

    /// Build an App from a source string with a generous viewport (no scroll needed).
    fn make_search_app(src: &str) -> App {
        App::new(src.to_owned(), PathBuf::from("test.md"), 80, 50)
    }

    #[test]
    fn start_search_enters_input_mode() {
        let mut app = make_search_app("hello world");
        assert!(app.search.is_none(), "precondition: no search");
        app.start_search();
        let s = app
            .search
            .as_ref()
            .expect("search must be Some after start");
        assert!(s.input_mode, "search must be in input mode");
        assert!(s.query.is_empty(), "query must be empty on start");
    }

    #[test]
    fn push_and_pop_search_char_edits_query() {
        let mut app = make_search_app("hello world");
        app.start_search();
        app.push_search_char('h');
        app.push_search_char('i');
        assert_eq!(
            app.search.as_ref().unwrap().query,
            "hi",
            "push_search_char must append characters"
        );
        app.pop_search_char();
        assert_eq!(
            app.search.as_ref().unwrap().query,
            "h",
            "pop_search_char must remove the last character"
        );
    }

    #[test]
    fn commit_search_finds_matching_lines_in_document_order() {
        // Two headings that match "sec", one that does not.
        let src = "# Section One\n\nOther text here.\n\n# Section Two\n\n# Unrelated";
        let mut app = make_search_app(src);
        app.start_search();
        app.push_search_char('s');
        app.push_search_char('e');
        app.push_search_char('c');
        app.commit_search();

        let s = app
            .search
            .as_ref()
            .expect("search must be Some after commit");
        assert!(
            !s.input_mode,
            "input_mode must be false after commit_search"
        );
        assert!(
            s.matches.len() >= 2,
            "expected >= 2 matching lines; got {}",
            s.matches.len()
        );
        // Matches must be in ascending (document) order.
        let sorted = {
            let mut v = s.matches.clone();
            v.sort_unstable();
            v
        };
        assert_eq!(s.matches, sorted, "matches must be in document order");
    }

    #[test]
    fn commit_search_case_insensitive() {
        let src = "Hello World\n\nAnother line.";
        let mut app = make_search_app(src);
        app.start_search();
        // Query in upper case — should still match "Hello World".
        for ch in "HELLO".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();
        let s = app.search.as_ref().unwrap();
        assert!(
            !s.matches.is_empty(),
            "case-insensitive search must find 'Hello World' with query 'HELLO'"
        );
    }

    #[test]
    fn commit_search_no_matches_yields_empty_list() {
        let src = "Nothing relevant here.";
        let mut app = make_search_app(src);
        app.start_search();
        for ch in "zzz_no_match_zzz".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();
        let s = app
            .search
            .as_ref()
            .expect("search must be Some (not blank)");
        assert!(
            s.matches.is_empty(),
            "non-matching query must produce an empty match list"
        );
    }

    #[test]
    fn search_next_wraps_and_updates_current() {
        let src = "# Section A\n\n# Section B\n\n# Section C";
        let mut app = make_search_app(src);
        app.start_search();
        for ch in "section".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();

        let n = app.search.as_ref().unwrap().matches.len();
        assert!(n >= 2, "precondition: at least 2 matches");

        // current should start at 0.
        assert_eq!(app.search.as_ref().unwrap().current, 0);

        app.search_next();
        assert_eq!(app.search.as_ref().unwrap().current, 1);

        // Wrap from last back to 0.
        for _ in 0..n - 1 {
            app.search_next();
        }
        assert_eq!(
            app.search.as_ref().unwrap().current,
            0,
            "search_next must wrap back to 0 after the last match"
        );
    }

    #[test]
    fn search_prev_wraps_backward() {
        let src = "# Section A\n\n# Section B\n\n# Section C";
        let mut app = make_search_app(src);
        app.start_search();
        for ch in "section".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();

        let n = app.search.as_ref().unwrap().matches.len();
        assert!(n >= 2, "precondition: at least 2 matches");

        // current = 0; going back must wrap to the last match.
        app.search_prev();
        assert_eq!(
            app.search.as_ref().unwrap().current,
            n - 1,
            "search_prev from 0 must wrap to the last match"
        );
    }

    #[test]
    fn search_next_noop_on_no_matches() {
        let src = "Nothing here.";
        let mut app = make_search_app(src);
        app.start_search();
        for ch in "zzz_no_match_zzz".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();
        // This must not panic or change anything.
        app.search_next();
        let s = app.search.as_ref().unwrap();
        assert!(s.matches.is_empty());
        assert_eq!(s.current, 0);
    }

    #[test]
    fn cancel_search_clears_state() {
        let mut app = make_search_app("hello");
        app.start_search();
        app.push_search_char('h');
        app.cancel_search();
        assert!(
            app.search.is_none(),
            "cancel_search must clear the search state"
        );
    }

    #[test]
    fn commit_blank_query_clears_search() {
        let mut app = make_search_app("hello");
        app.start_search();
        // No characters pushed — query is empty.
        app.commit_search();
        assert!(
            app.search.is_none(),
            "committing a blank query must clear the search state"
        );
    }

    #[test]
    fn search_next_scrolls_match_into_view() {
        // Build a large document so matches are off-screen.
        let mut lines: Vec<String> = (1..=30).map(|i| format!("line {i}")).collect();
        lines.push("# Section Target".to_string());
        let src = lines.join("\n\n");
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 11);
        app.viewport_height = 10;

        app.start_search();
        for ch in "target".chars() {
            app.push_search_char(ch);
        }
        app.commit_search();

        let s = app.search.as_ref().expect("search must be Some");
        assert!(!s.matches.is_empty(), "precondition: match found");
        let match_line = s.matches[s.current] as u16;

        // The match line must be visible in the viewport.
        let vp_start = app.scroll;
        let vp_end = app.scroll + app.viewport_height;
        assert!(
            match_line >= vp_start && match_line < vp_end,
            "commit_search must scroll the first match into view: \
             line={match_line}, scroll={vp_start}, vp_end={vp_end}"
        );
    }

    // --- Task 6 tests: history navigation wiring ---

    /// Helper: create two real temp files (main and target) and return their paths.
    fn make_two_temp_files(label: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("bella_task6_{label}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let main_path = dir.join("main.md");
        let target_path = dir.join("target.md");
        std::fs::write(&main_path, "[go](target.md)").expect("write main");
        std::fs::write(&target_path, "# Target\n\nContent.").expect("write target");
        (main_path, target_path)
    }

    #[test]
    fn history_field_defaults_empty() {
        let app = App::new("hello".to_string(), PathBuf::from("doc.md"), 80, 25);
        assert!(
            !app.history.can_back(),
            "history must be empty on construction"
        );
        assert!(!app.history.can_forward());
    }

    #[test]
    fn go_back_after_follow_restores_file_and_scroll() {
        // Write files with enough content so max_scroll > 0 and scroll=3 is valid.
        let dir = std::env::temp_dir().join("bella_task6_go_back_scroll");
        std::fs::create_dir_all(&dir).expect("create dir");
        let main_content: String = (1..=30)
            .map(|i| format!("line {i}\n\n"))
            .collect::<Vec<_>>()
            .join("");
        let main_path = dir.join("main.md");
        std::fs::write(&main_path, format!("{main_content}\n[go](target.md)")).expect("write main");
        let target_path = dir.join("target.md");
        std::fs::write(&target_path, "# Target\n\nContent.").expect("write target");

        let src = std::fs::read_to_string(&main_path).unwrap();
        let mut app = App::new(src, main_path.clone(), 80, 11);
        app.viewport_height = 10;

        // Verify scroll=3 is within range.
        assert!(
            app.max_scroll() >= 3,
            "precondition: max_scroll must be >= 3 for this test to be meaningful"
        );

        // Set a non-zero scroll offset so we can verify it is restored.
        app.scroll = 3;
        let prev_scroll = app.scroll;
        let prev_file = app.file.clone();

        // Simulate following a link using the History cursor model:
        // push BOTH the previous position AND the new position so the cursor
        // sits on the new entry and `back()` returns the previous one.
        use crate::history::HistoryEntry;
        app.history
            .push(HistoryEntry::new(prev_file.clone(), prev_scroll));
        app.load_file(target_path.clone()).expect("load target");
        app.history
            .push(HistoryEntry::new(app.file.clone(), app.scroll));

        // Sanity: we are now on the target.
        assert_ne!(app.file, prev_file, "precondition: file must have changed");

        // Now go back.
        app.go_back();

        assert_eq!(
            app.file, prev_file,
            "go_back must restore the previous file"
        );
        assert_eq!(
            app.scroll, prev_scroll,
            "go_back must restore the previous scroll offset"
        );
    }

    #[test]
    fn go_forward_after_go_back_returns_to_followed_file() {
        let (main_path, target_path) = make_two_temp_files("go_forward");
        let src = std::fs::read_to_string(&main_path).unwrap();
        let mut app = App::new(src, main_path.clone(), 80, 25);

        // Simulate a follow: push prev (main) and new (target) to the history.
        use crate::history::HistoryEntry;
        app.history.push(HistoryEntry::new(main_path.clone(), 0));
        app.load_file(target_path.clone()).expect("load target");
        app.history.push(HistoryEntry::new(target_path.clone(), 0));

        // go_back → back to main.
        app.go_back();
        assert_eq!(app.file, main_path, "after go_back: file must be main.md");

        // go_forward → back to target.
        app.go_forward();
        assert_eq!(
            app.file, target_path,
            "go_forward must return to the followed file"
        );
    }

    #[test]
    fn go_back_at_start_is_noop() {
        let mut app = App::new("hello".to_string(), PathBuf::from("doc.md"), 80, 25);
        let file_before = app.file.clone();
        // No history — go_back must be a no-op.
        app.go_back();
        assert_eq!(
            app.file, file_before,
            "go_back at start must not change file"
        );
        assert!(app.status_message.is_none(), "no error message expected");
    }

    #[test]
    fn go_forward_at_end_is_noop() {
        let mut app = App::new("hello".to_string(), PathBuf::from("doc.md"), 80, 25);
        let file_before = app.file.clone();
        // No forward history.
        app.go_forward();
        assert_eq!(
            app.file, file_before,
            "go_forward at end must not change file"
        );
        assert!(app.status_message.is_none(), "no error message expected");
    }

    #[test]
    fn follow_then_new_follow_after_go_back_truncates_forward() {
        // Verify that the App-level delegation to History truncates forward entries
        // when a new entry is pushed after going back.
        let (main_path, target_path) = make_two_temp_files("truncate_fwd");
        let src = std::fs::read_to_string(&main_path).unwrap();
        let mut app = App::new(src, main_path.clone(), 80, 25);

        use crate::history::HistoryEntry;

        // First follow: main → target.
        // Push prev (main) then new (target) so history cursor lands on target.
        app.history.push(HistoryEntry::new(main_path.clone(), 0));
        app.load_file(target_path.clone()).expect("load target");
        app.history.push(HistoryEntry::new(target_path.clone(), 0));
        // history = [main@0, target@0], cursor = 1

        // Go back to main (cursor moves to 0).
        app.go_back();
        assert_eq!(app.file, main_path, "precondition: back to main");

        // Now follow a new link from main — this should truncate the forward
        // history (target@0 at index 1).  Push prev (main@scroll_now) at cursor+1
        // (truncating target@0), then push new target.
        let prev_file = app.file.clone();
        let prev_scroll = app.scroll;
        app.history.push(HistoryEntry::new(prev_file, prev_scroll));
        app.load_file(target_path.clone()).expect("reload target");
        app.history
            .push(HistoryEntry::new(app.file.clone(), app.scroll));
        // After truncation and two pushes: [main@0, main@0, target@0], cursor=2
        // (or similar — the key invariant is no entries after cursor).

        // Forward history must have been truncated — can_forward should be false.
        assert!(
            !app.history.can_forward(),
            "forward history must be truncated after a new follow from a mid-stack position"
        );
    }

    // --- Task 3 (Block D) tests: hover_at, click_at, checkbox toggle ---

    /// Build an App from a doc that contains a task-list checkbox.
    fn make_checkbox_app() -> App {
        // The engine renders `- [ ] Item` as a task-list item with a checkbox glyph.
        let src = "- [ ] First task\n- [x] Second task".to_string();
        App::new(src, PathBuf::from("test.md"), 80, 25)
    }

    #[test]
    fn hover_at_sets_hovered_link_when_over_link() {
        // Doc with a link so hover can hit it.
        let src = "[alpha](a.md)\n\nOther text.".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");

        // The link should be on line 0, starting at col 0.
        let link = &app.link_map.links[0];
        let link_line = link.line;
        let link_col = link.col_start;

        // Before hover — no hovered_link.
        assert_eq!(app.hovered_link, None, "hovered_link must start as None");

        app.hover_at(link_line, link_col as u16);
        assert_eq!(
            app.hovered_link,
            Some(0),
            "hover_at on the link must set hovered_link to Some(0)"
        );
    }

    #[test]
    fn hover_at_clears_hovered_link_when_off_link() {
        let src = "[alpha](a.md)\n\nOther text.".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.hovered_link = Some(0);

        // Hover on a row/col that has no link.
        app.hover_at(100, 100);
        assert_eq!(
            app.hovered_link, None,
            "hover_at off any link must clear hovered_link"
        );
    }

    #[test]
    fn click_at_on_checkbox_toggles_membership() {
        let mut app = make_checkbox_app();
        assert!(
            !app.checkbox_map.items.is_empty(),
            "precondition: checkbox exists"
        );

        let cb = app.checkbox_map.items[0].clone();
        // Initially not in toggled set.
        assert!(
            !app.toggled_checkboxes.contains(&0),
            "precondition: checkbox 0 not toggled"
        );

        // First click — should add to toggled set.
        app.click_at(cb.line, cb.col_start as u16);
        assert!(
            app.toggled_checkboxes.contains(&0),
            "click_at on a checkbox must add it to toggled_checkboxes"
        );

        // Second click — should remove from toggled set.
        app.click_at(cb.line, cb.col_start as u16);
        assert!(
            !app.toggled_checkboxes.contains(&0),
            "second click_at on a checkbox must remove it from toggled_checkboxes"
        );
    }

    #[test]
    fn click_at_on_local_file_link_follows_file() {
        let dir = tempdir_for_test("click_at_follow");
        let target_content = "# Clicked Target\n\nContent.";
        write_temp_file(&dir, "target.md", target_content);
        let main_path = write_temp_file(&dir, "main.md", "[go](target.md)");
        let src = std::fs::read_to_string(&main_path).unwrap();

        let mut app = App::new(src, main_path.clone(), 80, 25);
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");

        let link = &app.link_map.links[0];
        let link_line = link.line;
        let link_col = link.col_start;

        let result = app.click_at(link_line, link_col as u16);

        assert!(
            result.is_some(),
            "click_at on a local file link must return Some((prev_path, prev_scroll))"
        );
        assert_eq!(
            app.file,
            dir.join("target.md"),
            "app.file must be updated to the followed file"
        );
    }

    #[test]
    fn click_at_on_empty_area_is_noop() {
        let src = "Just plain text, no links.".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        let file_before = app.file.clone();

        let result = app.click_at(0, 0);
        assert!(result.is_none(), "click_at on empty area must return None");
        assert_eq!(app.file, file_before, "file must not change");
    }

    // --- Task 4 (Block D) tests: drag-select methods ---

    #[test]
    fn selection_start_creates_selection_at_position() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        assert!(app.selection.is_none(), "precondition: no selection");

        app.selection_start(0, 3);
        let sel = app.selection.as_ref().expect("selection must be Some");
        assert_eq!(sel.anchor, (0, 3));
        assert_eq!(sel.cursor, (0, 3));
        assert!(sel.is_empty(), "zero-length selection must be empty");
    }

    #[test]
    fn selection_update_moves_cursor() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.selection_start(0, 3);
        app.selection_update(0, 8);

        let sel = app.selection.as_ref().unwrap();
        assert_eq!(sel.anchor, (0, 3));
        assert_eq!(sel.cursor, (0, 8));
        assert!(!sel.is_empty(), "non-zero selection must not be empty");
    }

    #[test]
    fn selection_update_noop_when_no_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        // No selection — must not panic.
        app.selection_update(0, 5);
        assert!(app.selection.is_none());
    }

    #[test]
    fn selection_finish_sets_status_on_nonempty_selection() {
        // Use a rendered document whose first line contains known text.
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        // Select characters 0..5 on line 0 ("hello").
        app.selection_start(0, 0);
        app.selection_update(0, 5);

        app.selection_finish();

        // Whether clipboard succeeded or failed, status_message must be set.
        assert!(
            app.status_message.is_some(),
            "selection_finish must set status_message"
        );
        // Selection must be kept for visual rendering.
        assert!(
            app.selection.is_some(),
            "selection_finish must keep the selection"
        );
    }

    #[test]
    fn selection_finish_noop_on_empty_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.selection_start(0, 3);
        // Don't update — selection is zero-length.
        app.selection_finish();
        // No status message set (nothing to copy).
        assert!(
            app.status_message.is_none(),
            "selection_finish on empty selection must not set status_message"
        );
    }

    #[test]
    fn selection_finish_noop_when_no_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        // Must not panic when there is no selection.
        app.selection_finish();
        assert!(app.status_message.is_none());
    }

    #[test]
    fn selection_cleared_on_render() {
        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.selection_start(0, 0);
        app.selection_update(0, 5);
        assert!(app.selection.is_some(), "precondition: selection set");

        app.render(80);
        assert!(app.selection.is_none(), "render must clear the selection");
    }

    #[test]
    fn selection_cleared_on_load_file() {
        let dir = tempdir_for_test("selection_clear_load");
        let target_path = write_temp_file(&dir, "target.md", "# New file");

        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.selection_start(0, 0);
        app.selection_update(0, 5);
        assert!(app.selection.is_some(), "precondition: selection set");

        app.load_file(target_path).expect("load_file must succeed");
        assert!(
            app.selection.is_none(),
            "load_file must clear the selection"
        );
    }

    #[test]
    fn drag_origin_cleared_on_load_file() {
        let dir = tempdir_for_test("drag_origin_clear_load");
        let target_path = write_temp_file(&dir, "target.md", "# New file");

        let src = "hello world".to_string();
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 25);
        app.drag_origin = Some((0, 5));

        app.load_file(target_path).expect("load_file must succeed");
        assert!(
            app.drag_origin.is_none(),
            "load_file must clear drag_origin"
        );
    }

    #[test]
    fn checkbox_toggled_state_cleared_on_load_file() {
        let dir = tempdir_for_test("checkbox_load_clear");
        let target_path = write_temp_file(&dir, "target.md", "# New file");

        let mut app = make_checkbox_app();
        app.toggled_checkboxes.insert(0);
        assert!(!app.toggled_checkboxes.is_empty(), "precondition: toggled");

        app.load_file(target_path).expect("load_file must succeed");
        assert!(
            app.toggled_checkboxes.is_empty(),
            "toggled_checkboxes must be cleared on load_file"
        );
    }
}
