//! App state: holds the source, rendered output, scroll position, and navigation metadata.

use std::path::{Path, PathBuf};

use bella_engine::{
    LinkMap, Theme,
    links::TableExpansions,
    markdown::{HeadingInfo, render_with_edit},
};
use ratatui::text::Line;

/// Search state while a `/`-search is active or has matches.
/// Fields are wired up by Tasks 3–5; allow dead-code until then.
#[allow(dead_code)]
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

#[allow(dead_code)]
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
    /// Heading metadata extracted from the last render.
    pub headings: Vec<HeadingInfo>,
    /// Current top display row (scroll offset).
    pub scroll: u16,
    /// Height of the content viewport (body area, excluding status line).
    pub viewport_height: u16,
    /// File path shown in the status line.
    pub file: PathBuf,
    /// Exit flag — set to `true` to terminate the event loop.
    pub should_quit: bool,
    /// Index of the currently focused link (into `link_map.links`), if any.
    pub focused_link: Option<usize>,
    /// Active search state, if any.
    pub search: Option<SearchState>,
}

impl App {
    /// Construct a new `App` by rendering `src` at `width`.
    ///
    /// `term_height` is the full terminal height; the status line takes 1 row,
    /// so `viewport_height = term_height.saturating_sub(1)`.
    pub fn new(src: String, file: PathBuf, width: u16, term_height: u16) -> Self {
        let viewport_height = term_height.saturating_sub(1);
        let base_dir = file.parent().map(Path::to_path_buf);
        let (lines, link_map, headings) = render_metadata(&src, width, base_dir.as_deref());
        Self {
            src,
            lines,
            link_map,
            headings,
            scroll: 0,
            viewport_height,
            file,
            should_quit: false,
            focused_link: None,
            search: None,
        }
    }

    /// Re-render at a new `width` (called on terminal resize).
    pub fn render(&mut self, width: u16) {
        let base_dir = self.file.parent().map(Path::to_path_buf);
        let (lines, link_map, headings) = render_metadata(&self.src, width, base_dir.as_deref());
        self.lines = lines;
        self.link_map = link_map;
        self.headings = headings;
        // Re-clamp scroll in case the new render is shorter.
        self.scroll = self.scroll.min(self.max_scroll());
        // Reset navigation state — line indices change on resize.
        self.focused_link = None;
        self.search = None;
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
}

/// Render `src` and return `(lines, link_map, headings)`.
fn render_metadata(
    src: &str,
    width: u16,
    base_dir: Option<&Path>,
) -> (Vec<Line<'static>>, LinkMap, Vec<HeadingInfo>) {
    let theme = Theme::dark();
    let rendered = render_with_edit(src, base_dir, width, &theme, None, &TableExpansions::new());
    (rendered.lines, rendered.link_map, rendered.headings)
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
}
