//! App state: holds the source, rendered output, and scroll position.

use std::path::PathBuf;

use bella_engine::{Theme, links::TableExpansions, markdown::render_with_edit};
use ratatui::text::Line;

/// Central application state passed through the event loop.
pub struct App {
    /// Raw markdown source.
    pub src: String,
    /// Rendered display lines (produced by bella-engine).
    pub lines: Vec<Line<'static>>,
    /// Current top display row (scroll offset).
    pub scroll: u16,
    /// Height of the content viewport (body area, excluding status line).
    pub viewport_height: u16,
    /// File path shown in the status line.
    pub file: PathBuf,
    /// Exit flag — set to `true` to terminate the event loop.
    pub should_quit: bool,
}

impl App {
    /// Construct a new `App` by rendering `src` at `width`.
    ///
    /// `term_height` is the full terminal height; the status line takes 1 row,
    /// so `viewport_height = term_height.saturating_sub(1)`.
    pub fn new(src: String, file: PathBuf, width: u16, term_height: u16) -> Self {
        let viewport_height = term_height.saturating_sub(1);
        let lines = render_lines(&src, width);
        Self {
            src,
            lines,
            scroll: 0,
            viewport_height,
            file,
            should_quit: false,
        }
    }

    /// Re-render at a new `width` (called on terminal resize).
    pub fn render(&mut self, width: u16) {
        self.lines = render_lines(&self.src, width);
        // Re-clamp scroll in case the new render is shorter.
        self.scroll = self.scroll.min(self.max_scroll());
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
}

fn render_lines(src: &str, width: u16) -> Vec<Line<'static>> {
    let theme = Theme::dark();
    let rendered = render_with_edit(src, None, width, &theme, None, &TableExpansions::new());
    rendered.lines
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
}
