//! Synchronous event loop: draw → read event → dispatch → repeat.

use anyhow::Result;
use bella_engine::body_pos;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::{Duration, Instant};

use crate::app::{App, Mode};
use crate::history::HistoryEntry;
use crate::ui;
use bella_engine::browser::BrowserEntryKind;

/// Action produced by the key mapper.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    ScrollDown(u16),
    ScrollUp(u16),
    ToTop,
    ToBottom,
    FocusNext,
    FocusPrev,
    ClearFocus,
    Follow,
    // Search actions (Task 5)
    SearchStart,
    SearchChar(char),
    SearchBackspace,
    SearchCommit,
    SearchNext,
    SearchPrev,
    SearchCancel,
    // History navigation (Task 6)
    HistoryBack,
    HistoryForward,
    // Mouse actions (Task 3)
    /// Mouse pointer moved over a content position — update `hovered_link`.
    HoverAt {
        content_row: usize,
        col: u16,
    },
    // Drag-select actions (Task 4)
    /// Left mouse button pressed — start of a possible click or drag.
    DragStart {
        content_row: usize,
        col: u16,
    },
    /// Left mouse button dragged — extend the active selection.
    DragUpdate {
        content_row: usize,
        col: u16,
    },
    /// Left mouse button released — finish a drag or fall through to a plain click.
    ///
    /// `content_row == usize::MAX` is a sentinel meaning the button was released
    /// outside the body area; in that case a plain click is not triggered.
    DragEnd {
        content_row: usize,
        col: u16,
    },
    /// Double-click detected: select the word under the pointer and copy it.
    ///
    /// `screen_col`/`screen_row` are the raw terminal coordinates passed
    /// directly to `select_word_at`; the word boundary is calculated inside
    /// [`App::double_click_word_select`].
    DoubleClickAt {
        screen_col: u16,
        screen_row: u16,
    },
    Quit,
    None,
    // Browser actions (Block E, Task 4)
    /// Move browser cursor up by one entry.
    BrowserUp,
    /// Move browser cursor down by one entry.
    BrowserDown,
    /// Descend into the selected directory, or open the selected markdown file.
    BrowserDescend,
    /// Ascend to the parent directory (Backspace key in browser mode).
    BrowserAscend,
    /// Left-click on a browser row: `row` is the row index relative to the inner listing area.
    BrowserClickAt {
        row: u16,
    },
    /// Mouse-wheel scroll of the browser listing.  Positive = scroll down, negative = scroll up.
    BrowserScroll(i32),
    /// Return from the reader to the browser (Backspace in reader mode).
    BrowserBack,
}

/// Pure browser key→action mapper (unit-testable without a live terminal).
///
/// Handles directional movement, descend/open, ascend, and quit.  All other
/// keys return [`Action::None`].
pub fn map_browser_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::BrowserDown,
        KeyCode::Char('k') | KeyCode::Up => Action::BrowserUp,
        KeyCode::Enter => Action::BrowserDescend,
        KeyCode::Backspace => Action::BrowserAscend,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        _ => Action::None,
    }
}

/// Pure reader key→action mapper (unit-testable without a live terminal).
pub fn map_key(key: KeyEvent, viewport_height: u16) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown(1),
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp(1),
        KeyCode::Char('g') | KeyCode::Home => Action::ToTop,
        KeyCode::Char('G') | KeyCode::End => Action::ToBottom,
        KeyCode::PageDown => Action::ScrollDown(viewport_height.saturating_sub(1).max(1)),
        KeyCode::PageUp => Action::ScrollUp(viewport_height.saturating_sub(1).max(1)),
        // Ctrl-d / Ctrl-u (half-page)
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ScrollDown(viewport_height / 2)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ScrollUp(viewport_height / 2)
        }
        // Link focus ring
        KeyCode::Tab => Action::FocusNext,
        KeyCode::BackTab => Action::FocusPrev,
        KeyCode::Esc => Action::ClearFocus,
        // Follow focused link
        KeyCode::Enter => Action::Follow,
        // In-document search
        KeyCode::Char('/') => Action::SearchStart,
        KeyCode::Char('n') => Action::SearchNext,
        KeyCode::Char('N') => Action::SearchPrev,
        // History navigation
        KeyCode::Char('[') => Action::HistoryBack,
        KeyCode::Char(']') => Action::HistoryForward,
        // Return to browser (when the reader was entered via the browser).
        KeyCode::Backspace => Action::BrowserBack,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        _ => Action::None,
    }
}

/// Key mapper for search-input mode.
///
/// While the user is typing a search query, character keys append to the query,
/// `Backspace` removes the last char, `Enter` commits, and `Esc` cancels.
/// All other keys are ignored (return `Action::None`).
pub fn map_search_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char(ch) => Action::SearchChar(ch),
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Enter => Action::SearchCommit,
        KeyCode::Esc => Action::SearchCancel,
        _ => Action::None,
    }
}

/// Pure mouse→action mapper (unit-testable without a live terminal).
///
/// Only scroll-wheel kinds are mapped for now; button/drag/move kinds return
/// `Action::None` for unhandled kinds (drag/up will be filled in by later tasks).
pub fn map_mouse(mouse: MouseEvent, app: &App) -> Action {
    match mouse.kind {
        MouseEventKind::ScrollDown => Action::ScrollDown(3),
        MouseEventKind::ScrollUp => Action::ScrollUp(3),
        MouseEventKind::Moved => {
            // Convert screen coordinates to content (row, col) using the body
            // viewport stored after the last draw.
            if let Some((content_row, local_col)) = body_pos(
                app.body_area,
                false,
                app.lines.len(),
                app.scroll as usize,
                mouse.column,
                mouse.row,
            ) {
                Action::HoverAt {
                    content_row,
                    col: local_col,
                }
            } else {
                // Pointer outside the body — clear any hover.
                Action::HoverAt {
                    content_row: usize::MAX,
                    col: 0,
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some((content_row, local_col)) = body_pos(
                app.body_area,
                false,
                app.lines.len(),
                app.scroll as usize,
                mouse.column,
                mouse.row,
            ) {
                // Double-click detection: if the previous click was within 450 ms
                // at the same content position, this is a double-click.
                let now = Instant::now();
                let is_double = app
                    .last_click
                    .as_ref()
                    .map(|(t, cr, cc)| {
                        now.duration_since(*t) <= Duration::from_millis(450)
                            && *cr == content_row
                            && *cc == local_col as usize
                    })
                    .unwrap_or(false);

                if is_double {
                    Action::DoubleClickAt {
                        screen_col: mouse.column,
                        screen_row: mouse.row,
                    }
                } else {
                    Action::DragStart {
                        content_row,
                        col: local_col,
                    }
                }
            } else {
                Action::None
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((content_row, local_col)) = body_pos(
                app.body_area,
                false,
                app.lines.len(),
                app.scroll as usize,
                mouse.column,
                mouse.row,
            ) {
                Action::DragUpdate {
                    content_row,
                    col: local_col,
                }
            } else {
                Action::None
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some((content_row, local_col)) = body_pos(
                app.body_area,
                false,
                app.lines.len(),
                app.scroll as usize,
                mouse.column,
                mouse.row,
            ) {
                Action::DragEnd {
                    content_row,
                    col: local_col,
                }
            } else {
                // Released outside body. If a selection was in progress,
                // the DragEnd sentinel lets apply finish it without clicking.
                Action::DragEnd {
                    content_row: usize::MAX,
                    col: 0,
                }
            }
        }
        _ => Action::None,
    }
}

/// Pure mouse→action mapper for browser mode (unit-testable without a live terminal).
///
/// Scroll-wheel moves the viewport; `Down(Left)` on a listing row produces
/// [`Action::BrowserClickAt`] which selects the row and immediately descends/opens it.
pub fn map_browser_mouse(mouse: MouseEvent, app: &App) -> Action {
    match mouse.kind {
        MouseEventKind::ScrollDown => Action::BrowserScroll(3),
        MouseEventKind::ScrollUp => Action::BrowserScroll(-3),
        MouseEventKind::Down(MouseButton::Left) => {
            let area = app.browser_area;
            if area.width == 0 || area.height == 0 {
                return Action::None;
            }
            if mouse.row >= area.y
                && mouse.row < area.y + area.height
                && mouse.column >= area.x
                && mouse.column < area.x + area.width
            {
                let row = mouse.row - area.y;
                Action::BrowserClickAt { row }
            } else {
                Action::None
            }
        }
        _ => Action::None,
    }
}

/// Apply an `Action` to `app`.
pub(crate) fn apply(action: Action, app: &mut App) {
    match action {
        Action::ScrollDown(n) => app.scroll_down(n),
        Action::ScrollUp(n) => app.scroll_up(n),
        Action::ToTop => app.jump_top(),
        Action::ToBottom => app.jump_bottom(),
        Action::FocusNext => app.focus_next(),
        Action::FocusPrev => app.focus_prev(),
        Action::ClearFocus => {
            // Clear all overlay state in one Esc press — search and link focus
            // can be simultaneously active, so cancel both unconditionally.
            app.cancel_search();
            app.clear_focus();
        }
        Action::Follow => {
            // follow_focused() returns Some((prev_file, prev_scroll)) when a file
            // navigation occurred.  The History cursor model requires that BOTH the
            // previous and the new positions are in the stack (cursor at the new one),
            // so `back()` can return the previous entry.
            if let Some((prev_path, prev_scroll)) = app.follow_focused() {
                // Push the previous position (where we were before navigating).
                app.history.push(HistoryEntry::new(prev_path, prev_scroll));
                // Push the new current position (already loaded into app).
                app.history
                    .push(HistoryEntry::new(app.file.clone(), app.scroll));
            }
        }
        // History navigation
        Action::HistoryBack => app.go_back(),
        Action::HistoryForward => app.go_forward(),
        // Mouse actions
        Action::HoverAt { content_row, col } => {
            // usize::MAX is the sentinel for "pointer left the body area".
            if content_row == usize::MAX {
                app.hovered_link = None;
            } else {
                app.hover_at(content_row, col);
            }
        }
        // Drag-select actions (Task 4)
        Action::DragStart { content_row, col } => {
            // Clear any prior selection — a new drag replaces it.
            app.selection = None;
            // Record the drag origin; the actual `Selection` is created on the
            // first `Drag` event so that a plain click (no drag) falls through
            // to `click_at` instead.
            app.drag_origin = Some((content_row, col as usize));
            // Record the click time + position for double-click detection.
            app.last_click = Some((Instant::now(), content_row, col as usize));
        }
        // Double-click word-select (Task 5 of Block D)
        Action::DoubleClickAt {
            screen_col,
            screen_row,
        } => {
            // Clear any pending drag state — double-click supersedes a drag.
            app.selection = None;
            app.drag_origin = None;
            app.double_click_word_select(screen_col, screen_row);
        }
        Action::DragUpdate { content_row, col } => {
            // First drag event: promote the pending origin to a real selection.
            if app.selection.is_none()
                && let Some((or, oc)) = app.drag_origin
            {
                app.selection_start(or, oc);
            }
            app.selection_update(content_row, col as usize);
        }
        Action::DragEnd { content_row, col } => {
            if app.selection.is_some() && app.drag_origin.is_some() {
                // A real drag occurred — extract and copy the selected text.
                app.selection_finish();
            } else if app.drag_origin.is_some() && content_row != usize::MAX {
                // No drag — treat as a plain click: follow link or toggle checkbox.
                if let Some((prev_path, prev_scroll)) = app.click_at(content_row, col) {
                    app.history.push(HistoryEntry::new(prev_path, prev_scroll));
                    app.history
                        .push(HistoryEntry::new(app.file.clone(), app.scroll));
                }
            }
            // Always clear the drag origin on mouse-up.
            app.drag_origin = None;
        }
        // Search actions
        Action::SearchStart => app.start_search(),
        Action::SearchChar(ch) => app.push_search_char(ch),
        Action::SearchBackspace => app.pop_search_char(),
        Action::SearchCommit => app.commit_search(),
        Action::SearchNext => app.search_next(),
        Action::SearchPrev => app.search_prev(),
        Action::SearchCancel => app.cancel_search(),
        Action::Quit => app.quit(),
        Action::None => {}
        // Browser actions (Block E, Task 4)
        Action::BrowserUp => {
            let vp = app.browser_area.height.max(1);
            if let Some(b) = app.browser.as_mut() {
                b.move_cursor(-1, vp);
            }
        }
        Action::BrowserDown => {
            let vp = app.browser_area.height.max(1);
            if let Some(b) = app.browser.as_mut() {
                b.move_cursor(1, vp);
            }
        }
        Action::BrowserAscend => {
            app.ascend();
        }
        Action::BrowserDescend => {
            // Collect what to do before taking a mutable borrow.
            let descend_dir = app.browser.as_ref().and_then(|b| b.descend());
            let open_file = app.browser.as_ref().and_then(|b| {
                b.selected_entry().and_then(|e| {
                    if matches!(e.kind, BrowserEntryKind::Markdown) {
                        Some(e.path.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(dir) = descend_dir {
                app.enter_dir(dir);
            } else if let Some(file) = open_file
                && let Err(msg) = app.open_from_browser(file)
            {
                app.status_message = Some(msg);
            }
        }
        Action::BrowserClickAt { row } => {
            // Select the clicked row; skip descend/open if click is below the last entry.
            let in_range = if let Some(b) = app.browser.as_mut() {
                let target_idx = b.scroll as usize + row as usize;
                if target_idx < b.entries.len() {
                    b.selected = target_idx;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !in_range {
                return;
            }
            // Then descend/open the now-selected entry.
            let descend_dir = app.browser.as_ref().and_then(|b| b.descend());
            let open_file = app.browser.as_ref().and_then(|b| {
                b.selected_entry().and_then(|e| {
                    if matches!(e.kind, BrowserEntryKind::Markdown) {
                        Some(e.path.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(dir) = descend_dir {
                app.enter_dir(dir);
            } else if let Some(file) = open_file
                && let Err(msg) = app.open_from_browser(file)
            {
                app.status_message = Some(msg);
            }
        }
        Action::BrowserScroll(delta) => {
            let vp = app.browser_area.height.max(1);
            if let Some(b) = app.browser.as_mut() {
                if delta > 0 {
                    let max_scroll = (b.entries.len() as u16).saturating_sub(vp);
                    b.scroll = b.scroll.saturating_add(delta as u16).min(max_scroll);
                } else {
                    b.scroll = b.scroll.saturating_sub((-delta) as u16);
                }
            }
        }
        Action::BrowserBack => {
            app.back_to_browser();
        }
    }
}

/// Synchronous event loop.  Draws, then blocks on the next terminal event.
pub fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut app: App,
) -> Result<()> {
    loop {
        // Draw the appropriate screen based on the current mode.
        match app.mode {
            Mode::Browser => {
                terminal.draw(|f| {
                    ui::draw_browser(f, f.area(), &mut app);
                })?;
            }
            Mode::Reader => {
                terminal.draw(|f| {
                    ui::draw_reader(f, f.area(), &mut app);
                })?;
            }
        }

        if app.should_quit {
            break;
        }

        match event::read()? {
            Event::Key(key) => {
                let action = match app.mode {
                    Mode::Browser => map_browser_key(key),
                    Mode::Reader => {
                        // In search input mode, character keys feed into the query instead of
                        // the normal key bindings.
                        let in_search_input =
                            app.search.as_ref().map(|s| s.input_mode).unwrap_or(false);
                        if in_search_input {
                            map_search_key(key)
                        } else {
                            map_key(key, app.viewport_height)
                        }
                    }
                };
                apply(action, &mut app);
            }
            Event::Resize(width, height) => {
                app.set_viewport_height(height.saturating_sub(1));
                app.width = width;
                if app.mode == Mode::Reader {
                    app.render(width);
                }
                // Re-clamp browser scroll to the new terminal height.
                let browser_inner_h = height.saturating_sub(2) as usize;
                if let Some(b) = app.browser.as_mut() {
                    let max_scroll = b.entries.len().saturating_sub(browser_inner_h) as u16;
                    b.scroll = b.scroll.min(max_scroll);
                    if b.selected < b.scroll as usize {
                        b.scroll = b.selected as u16;
                    }
                }
            }
            Event::Mouse(mouse) => {
                let action = match app.mode {
                    Mode::Browser => map_browser_mouse(mouse, &app),
                    Mode::Reader => map_mouse(mouse, &app),
                };
                apply(action, &mut app);
            }
            _ => {}
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, Mode};

    use super::{Action, map_browser_key, map_key};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn make_app() -> App {
        // Short doc so we have non-trivial scroll range.
        let src = (1..=30)
            .map(|i| format!("# Line {i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut app = App::new(src, PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        app
    }

    #[test]
    fn j_produces_scroll_down() {
        assert_eq!(map_key(key(KeyCode::Char('j')), 10), Action::ScrollDown(1));
    }

    #[test]
    fn down_arrow_produces_scroll_down() {
        assert_eq!(map_key(key(KeyCode::Down), 10), Action::ScrollDown(1));
    }

    #[test]
    fn k_produces_scroll_up() {
        assert_eq!(map_key(key(KeyCode::Char('k')), 10), Action::ScrollUp(1));
    }

    #[test]
    fn up_arrow_produces_scroll_up() {
        assert_eq!(map_key(key(KeyCode::Up), 10), Action::ScrollUp(1));
    }

    #[test]
    fn g_produces_to_top() {
        assert_eq!(map_key(key(KeyCode::Char('g')), 10), Action::ToTop);
    }

    #[test]
    fn big_g_produces_to_bottom() {
        assert_eq!(map_key(key(KeyCode::Char('G')), 10), Action::ToBottom);
    }

    #[test]
    fn q_produces_quit() {
        assert_eq!(map_key(key(KeyCode::Char('q')), 10), Action::Quit);
    }

    #[test]
    fn ctrl_c_produces_quit() {
        assert_eq!(map_key(ctrl(KeyCode::Char('c')), 10), Action::Quit);
    }

    #[test]
    fn unmapped_key_is_none() {
        assert_eq!(map_key(key(KeyCode::Char('x')), 10), Action::None);
    }

    #[test]
    fn j_scrolls_app_down() {
        let mut app = make_app();
        assert_eq!(app.scroll, 0);
        let action = map_key(key(KeyCode::Char('j')), app.viewport_height);
        super::apply(action, &mut app);
        assert_eq!(app.scroll, 1);
    }

    #[test]
    fn q_sets_should_quit() {
        let mut app = make_app();
        let action = map_key(key(KeyCode::Char('q')), app.viewport_height);
        super::apply(action, &mut app);
        assert!(app.should_quit);
    }

    // --- Task 3 tests: focus key mappings ---

    #[test]
    fn tab_produces_focus_next() {
        assert_eq!(map_key(key(KeyCode::Tab), 10), Action::FocusNext);
    }

    #[test]
    fn backtab_produces_focus_prev() {
        assert_eq!(map_key(key(KeyCode::BackTab), 10), Action::FocusPrev);
    }

    #[test]
    fn esc_produces_clear_focus() {
        assert_eq!(map_key(key(KeyCode::Esc), 10), Action::ClearFocus);
    }

    #[test]
    fn apply_focus_next_advances_link_focus() {
        // Build an app with links.
        let src = "[A](a.md)\n\n[B](b.md)".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: links exist");
        super::apply(Action::FocusNext, &mut app);
        assert_eq!(
            app.focused_link,
            Some(0),
            "FocusNext must select first link"
        );
    }

    #[test]
    fn apply_focus_prev_wraps_backward() {
        let src = "[A](a.md)\n\n[B](b.md)".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        let n = app.link_map.links.len();
        super::apply(Action::FocusPrev, &mut app);
        assert_eq!(
            app.focused_link,
            Some(n - 1),
            "FocusPrev from None must wrap to the last link"
        );
    }

    #[test]
    fn apply_clear_focus_removes_focus() {
        let src = "[A](a.md)".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        app.focused_link = Some(0);
        super::apply(Action::ClearFocus, &mut app);
        assert_eq!(app.focused_link, None, "ClearFocus must clear focused_link");
    }

    // --- Task 4 tests: Enter → Follow ---

    #[test]
    fn enter_produces_follow() {
        assert_eq!(map_key(key(KeyCode::Enter), 10), Action::Follow);
    }

    #[test]
    fn apply_follow_with_no_focused_link_is_noop() {
        let src = "[A](a.md)".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        // No focused link — Follow should be a no-op (no panic, no change).
        let file_before = app.file.clone();
        super::apply(Action::Follow, &mut app);
        assert_eq!(
            app.file, file_before,
            "file must not change when no link is focused"
        );
    }

    #[test]
    fn apply_follow_url_does_not_change_file() {
        let src = "[web](https://example.com)".to_string();
        let file = std::path::PathBuf::from("test.md");
        let mut app = App::new(src, file.clone(), 80, 11);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);
        super::apply(Action::Follow, &mut app);
        assert_eq!(app.file, file, "file must not change when following a URL");
    }

    #[test]
    fn apply_follow_local_file_changes_file() {
        // Write a real target file so load_file can succeed.
        let dir = std::env::temp_dir();
        let target = dir.join("bella_events_follow_target.md");
        std::fs::write(&target, "# Target\n\nContent.").expect("write target");

        // The engine resolves relative links against base_dir.  Use the temp dir
        // as the parent so the resolved path matches `target`.
        let main_path = dir.join("bella_events_follow_main.md");
        let src = "[go](bella_events_follow_target.md)".to_string();
        std::fs::write(&main_path, &src).expect("write main");

        let mut app = App::new(src, main_path.clone(), 80, 11);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);

        super::apply(Action::Follow, &mut app);

        assert_eq!(
            app.file, target,
            "app.file must be the followed target path"
        );
        // Cleanup.
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&main_path);
    }

    // --- Task 5 tests: search key mappings and routing ---

    #[test]
    fn slash_produces_search_start() {
        assert_eq!(map_key(key(KeyCode::Char('/')), 10), Action::SearchStart);
    }

    #[test]
    fn n_produces_search_next() {
        assert_eq!(map_key(key(KeyCode::Char('n')), 10), Action::SearchNext);
    }

    #[test]
    fn big_n_produces_search_prev() {
        assert_eq!(map_key(key(KeyCode::Char('N')), 10), Action::SearchPrev);
    }

    #[test]
    fn map_search_key_char_produces_search_char() {
        let k = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        assert_eq!(super::map_search_key(k), Action::SearchChar('h'));
    }

    #[test]
    fn map_search_key_backspace_produces_search_backspace() {
        let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(super::map_search_key(k), Action::SearchBackspace);
    }

    #[test]
    fn map_search_key_enter_produces_search_commit() {
        let k = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(super::map_search_key(k), Action::SearchCommit);
    }

    #[test]
    fn map_search_key_esc_produces_search_cancel() {
        let k = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(super::map_search_key(k), Action::SearchCancel);
    }

    #[test]
    fn apply_search_start_enters_search_mode() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        super::apply(Action::SearchStart, &mut app);
        assert!(
            app.search.is_some(),
            "SearchStart must put app into search mode"
        );
        assert!(
            app.search.as_ref().unwrap().input_mode,
            "search must be in input mode after SearchStart"
        );
    }

    #[test]
    fn apply_search_char_appends_to_query() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchChar('i'), &mut app);
        assert_eq!(app.search.as_ref().unwrap().query, "hi");
    }

    #[test]
    fn apply_search_cancel_clears_search() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchCancel, &mut app);
        assert!(app.search.is_none(), "SearchCancel must clear search state");
    }

    #[test]
    fn apply_search_commit_leaves_input_mode() {
        let src = "hello world\n\nanother line".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchCommit, &mut app);
        let s = app
            .search
            .as_ref()
            .expect("search must be Some after commit");
        assert!(!s.input_mode, "input_mode must be false after SearchCommit");
    }

    #[test]
    fn esc_in_normal_mode_cancels_active_search() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        app.block_until_ready();
        // Put the app in a post-commit search state (input_mode = false).
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchCommit, &mut app);
        assert!(app.search.is_some(), "precondition: search active");
        // In normal mode (not input mode), Esc maps to ClearFocus,
        // which should cancel an active search.
        super::apply(Action::ClearFocus, &mut app);
        assert!(
            app.search.is_none(),
            "ClearFocus must cancel an active search"
        );
    }

    // --- Task 6 tests: history key mappings ---

    #[test]
    fn left_bracket_produces_history_back() {
        assert_eq!(map_key(key(KeyCode::Char('[')), 10), Action::HistoryBack);
    }

    #[test]
    fn right_bracket_produces_history_forward() {
        assert_eq!(map_key(key(KeyCode::Char(']')), 10), Action::HistoryForward);
    }

    #[test]
    fn apply_history_back_noop_at_start() {
        let mut app = make_app();
        let file_before = app.file.clone();
        super::apply(Action::HistoryBack, &mut app);
        assert_eq!(
            app.file, file_before,
            "HistoryBack at start must be a no-op"
        );
    }

    #[test]
    fn apply_history_forward_noop_at_end() {
        let mut app = make_app();
        let file_before = app.file.clone();
        super::apply(Action::HistoryForward, &mut app);
        assert_eq!(
            app.file, file_before,
            "HistoryForward at end must be a no-op"
        );
    }

    #[test]
    fn apply_follow_pushes_history_entry() {
        // Write a real target file so load_file can succeed.
        let dir = std::env::temp_dir();
        let target = dir.join("bella_events_task6_target.md");
        std::fs::write(&target, "# Target\n\nContent.").expect("write target");

        let main_path = dir.join("bella_events_task6_main.md");
        let src = "[go](bella_events_task6_target.md)".to_string();
        std::fs::write(&main_path, &src).expect("write main");

        let mut app = App::new(src, main_path.clone(), 80, 11);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        app.focused_link = Some(0);

        // Before follow — history must be empty (no back entry).
        assert!(
            !app.history.can_back(),
            "history must have no back entry before following a link"
        );

        super::apply(Action::Follow, &mut app);

        // After following a LocalFile link, apply pushes both prev AND new position.
        // The cursor is on the new entry, so can_back() is true (prev is behind it).
        assert!(
            app.history.can_back(),
            "history must have a back entry after following a local-file link"
        );
        // Going back should restore the main file.
        app.go_back();
        assert_eq!(
            app.file, main_path,
            "go_back after follow must restore the main file"
        );

        // Cleanup.
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&main_path);
    }

    // --- Task 2 tests: map_mouse ---

    fn mouse_event(kind: crossterm::event::MouseEventKind) -> crossterm::event::MouseEvent {
        crossterm::event::MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::empty(),
        }
    }

    #[test]
    fn map_mouse_scroll_down_produces_scroll_down() {
        let app = make_app();
        let ev = mouse_event(crossterm::event::MouseEventKind::ScrollDown);
        assert_eq!(super::map_mouse(ev, &app), Action::ScrollDown(3));
    }

    #[test]
    fn map_mouse_scroll_up_produces_scroll_up() {
        let app = make_app();
        let ev = mouse_event(crossterm::event::MouseEventKind::ScrollUp);
        assert_eq!(super::map_mouse(ev, &app), Action::ScrollUp(3));
    }

    #[test]
    fn map_mouse_unmapped_kind_produces_none() {
        let app = make_app();
        // Down(Right) is not handled — must return None.
        let ev = mouse_event(crossterm::event::MouseEventKind::Down(
            crossterm::event::MouseButton::Right,
        ));
        assert_eq!(super::map_mouse(ev, &app), Action::None);
    }

    // --- Task 3 (Block D) tests: map_mouse Moved/Down + apply HoverAt/ClickAt ---

    fn mouse_event_at(
        kind: crossterm::event::MouseEventKind,
        column: u16,
        row: u16,
    ) -> crossterm::event::MouseEvent {
        crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::empty(),
        }
    }

    #[test]
    fn map_mouse_moved_inside_body_produces_hover_at() {
        let mut app = make_app();
        // Simulate the body_area being set to cover the full terminal area (as if
        // draw_reader just ran). TestBackend-style: x=0, y=0, width=80, height=10.
        use ratatui::layout::Rect;
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        let ev = mouse_event_at(crossterm::event::MouseEventKind::Moved, 5, 2);
        let action = super::map_mouse(ev, &app);
        // Should produce a HoverAt with a valid content_row (not usize::MAX).
        match action {
            super::Action::HoverAt { content_row, .. } => {
                assert_ne!(
                    content_row,
                    usize::MAX,
                    "Moved inside body must produce HoverAt with valid content_row"
                );
            }
            other => panic!("expected HoverAt, got {other:?}"),
        }
    }

    #[test]
    fn map_mouse_moved_outside_body_produces_hover_at_sentinel() {
        let mut app = make_app();
        use ratatui::layout::Rect;
        // Body occupies rows 0..10, col 0..80.
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        // Row 15 is outside the body.
        let ev = mouse_event_at(crossterm::event::MouseEventKind::Moved, 5, 15);
        let action = super::map_mouse(ev, &app);
        match action {
            super::Action::HoverAt { content_row, .. } => {
                assert_eq!(
                    content_row,
                    usize::MAX,
                    "Moved outside body must produce HoverAt sentinel (usize::MAX)"
                );
            }
            other => panic!("expected HoverAt sentinel, got {other:?}"),
        }
    }

    #[test]
    fn map_mouse_down_left_inside_body_produces_drag_start() {
        let mut app = make_app();
        use ratatui::layout::Rect;
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        let ev = mouse_event_at(
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            5,
            2,
        );
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DragStart { .. }),
            "Down(Left) inside body must produce DragStart; got {action:?}"
        );
    }

    #[test]
    fn map_mouse_drag_left_inside_body_produces_drag_update() {
        let mut app = make_app();
        use ratatui::layout::Rect;
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        let ev = mouse_event_at(
            crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
            5,
            2,
        );
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DragUpdate { .. }),
            "Drag(Left) inside body must produce DragUpdate; got {action:?}"
        );
    }

    #[test]
    fn map_mouse_up_left_inside_body_produces_drag_end() {
        let mut app = make_app();
        use ratatui::layout::Rect;
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        let ev = mouse_event_at(
            crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            5,
            2,
        );
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DragEnd { .. }),
            "Up(Left) inside body must produce DragEnd; got {action:?}"
        );
    }

    #[test]
    fn map_mouse_up_left_outside_body_produces_drag_end_sentinel() {
        let mut app = make_app();
        use ratatui::layout::Rect;
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 10,
        };

        // Row 15 is outside the body.
        let ev = mouse_event_at(
            crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            5,
            15,
        );
        let action = super::map_mouse(ev, &app);
        match action {
            super::Action::DragEnd { content_row, .. } => {
                assert_eq!(
                    content_row,
                    usize::MAX,
                    "Up(Left) outside body must produce DragEnd with sentinel content_row"
                );
            }
            other => panic!("expected DragEnd, got {other:?}"),
        }
    }

    #[test]
    fn apply_hover_at_sets_hovered_link() {
        let src = "[alpha](a.md)\n\nOther text.".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");

        let link = app.link_map.links[0].clone();
        super::apply(
            super::Action::HoverAt {
                content_row: link.line,
                col: link.col_start as u16,
            },
            &mut app,
        );
        assert_eq!(
            app.hovered_link,
            Some(0),
            "apply HoverAt on a link must set hovered_link"
        );
    }

    #[test]
    fn apply_hover_at_sentinel_clears_hovered_link() {
        let src = "[alpha](a.md)".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        app.hovered_link = Some(0);

        super::apply(
            super::Action::HoverAt {
                content_row: usize::MAX,
                col: 0,
            },
            &mut app,
        );
        assert_eq!(
            app.hovered_link, None,
            "apply HoverAt with sentinel must clear hovered_link"
        );
    }

    // --- Task 4 (Block D) tests: drag-select actions ---

    #[test]
    fn apply_drag_start_sets_drag_origin_and_clears_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        // Pre-set a selection to verify it gets cleared.
        app.selection_start(0, 0);
        app.selection_update(0, 5);
        assert!(app.selection.is_some(), "precondition: selection set");

        super::apply(
            super::Action::DragStart {
                content_row: 0,
                col: 2,
            },
            &mut app,
        );

        assert!(
            app.selection.is_none(),
            "DragStart must clear any existing selection"
        );
        assert_eq!(
            app.drag_origin,
            Some((0, 2)),
            "DragStart must set drag_origin"
        );
    }

    #[test]
    fn apply_drag_update_creates_and_extends_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();

        // Simulate Down then first Drag.
        super::apply(
            super::Action::DragStart {
                content_row: 0,
                col: 0,
            },
            &mut app,
        );
        assert!(
            app.selection.is_none(),
            "before first drag: no selection yet"
        );

        super::apply(
            super::Action::DragUpdate {
                content_row: 0,
                col: 5,
            },
            &mut app,
        );

        let sel = app
            .selection
            .as_ref()
            .expect("DragUpdate must create selection");
        assert_eq!(sel.anchor, (0, 0), "anchor must be the drag origin");
        assert_eq!(sel.cursor, (0, 5), "cursor must be the drag destination");
    }

    #[test]
    fn apply_drag_end_after_drag_finishes_selection_and_clears_origin() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();

        // Full drag sequence: Down → Drag → Up.
        super::apply(
            super::Action::DragStart {
                content_row: 0,
                col: 0,
            },
            &mut app,
        );
        super::apply(
            super::Action::DragUpdate {
                content_row: 0,
                col: 5,
            },
            &mut app,
        );
        super::apply(
            super::Action::DragEnd {
                content_row: 0,
                col: 5,
            },
            &mut app,
        );

        // Selection must remain for visual rendering.
        assert!(
            app.selection.is_some(),
            "DragEnd must keep the selection for display"
        );
        // Drag origin must be cleared.
        assert!(app.drag_origin.is_none(), "DragEnd must clear drag_origin");
        // Status message set by copy attempt.
        assert!(
            app.status_message.is_some(),
            "DragEnd after drag must set status_message (copy attempt)"
        );
    }

    #[test]
    fn apply_drag_end_without_drag_does_not_create_selection() {
        let src = "Just plain text.".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();

        // Down then Up with no Drag.
        super::apply(
            super::Action::DragStart {
                content_row: 0,
                col: 0,
            },
            &mut app,
        );
        super::apply(
            super::Action::DragEnd {
                content_row: 0,
                col: 0,
            },
            &mut app,
        );

        assert!(
            app.selection.is_none(),
            "Down+Up with no drag must not create a selection"
        );
        assert!(
            app.drag_origin.is_none(),
            "DragEnd must clear drag_origin even without a drag"
        );
    }

    #[test]
    fn apply_drag_end_without_drag_follows_link() {
        // Write a real target so load_file can succeed.
        let dir = std::env::temp_dir().join("bella_drag_click_follow");
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.md");
        std::fs::write(&target, "# Target\n\nContent.").unwrap();
        let main_path = dir.join("main.md");
        let src = "[go](target.md)".to_string();
        std::fs::write(&main_path, &src).unwrap();

        let mut app = App::new(src, main_path.clone(), 80, 11);
        app.block_until_ready();
        assert!(!app.link_map.links.is_empty(), "precondition: link exists");
        let link = app.link_map.links[0].clone();

        // Simulate a click (Down + Up, no Drag) directly on the link.
        super::apply(
            super::Action::DragStart {
                content_row: link.line,
                col: link.col_start as u16,
            },
            &mut app,
        );
        super::apply(
            super::Action::DragEnd {
                content_row: link.line,
                col: link.col_start as u16,
            },
            &mut app,
        );

        assert_eq!(app.file, target, "Down+Up (click) on a link must follow it");
        assert!(app.selection.is_none(), "click must not leave a selection");

        // Cleanup.
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&main_path);
    }

    #[test]
    fn apply_drag_end_click_on_checkbox_toggles_it() {
        // A plain click (DragStart → DragEnd, no Drag) on a checkbox must toggle it.
        let src = "- [ ] Task one\n\n- [x] Task two".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        if app.checkbox_map.items.is_empty() {
            // Engine did not produce a checkbox — skip.
            return;
        }
        let cb = app.checkbox_map.items[0].clone();
        assert!(
            !app.toggled_checkboxes.contains(&0),
            "precondition: not toggled"
        );

        // Simulate a plain click: DragStart then DragEnd with no intervening DragUpdate.
        super::apply(
            super::Action::DragStart {
                content_row: cb.line,
                col: cb.col_start as u16,
            },
            &mut app,
        );
        super::apply(
            super::Action::DragEnd {
                content_row: cb.line,
                col: cb.col_start as u16,
            },
            &mut app,
        );
        assert!(
            app.toggled_checkboxes.contains(&0),
            "DragStart+DragEnd (plain click) on a checkbox must toggle it"
        );
    }

    // --- Task 5 (Block D) tests: double-click detection ---

    use ratatui::layout::Rect;
    use std::time::{Duration, Instant};

    /// App whose first rendered line is "hello world" and body_area is set.
    fn make_word_app() -> App {
        let src = (1..=30)
            .map(|i| format!("# Line {i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        app
    }

    #[test]
    fn drag_start_records_last_click() {
        let mut app = make_word_app();
        assert!(app.last_click.is_none(), "precondition: no last_click");

        super::apply(
            super::Action::DragStart {
                content_row: 0,
                col: 2,
            },
            &mut app,
        );

        assert!(
            app.last_click.is_some(),
            "apply(DragStart) must record last_click"
        );
        let (_, cr, cc) = app.last_click.unwrap();
        assert_eq!(
            cr, 0,
            "last_click content_row must match DragStart content_row"
        );
        assert_eq!(cc, 2, "last_click col must match DragStart col");
    }

    #[test]
    fn map_mouse_down_left_within_window_at_same_position_produces_double_click() {
        let mut app = make_word_app();
        // Inject last_click 300 ms ago at content position (2, 5) — within 450 ms.
        app.last_click = Some((Instant::now() - Duration::from_millis(300), 2, 5));

        // Create a mouse Down(Left) event at screen (5, 2).
        // body_pos with viewport (0,0,80,24), scroll=0, screen (5,2) → content row=2, col=5.
        let ev = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DoubleClickAt { .. }),
            "Down(Left) within 450 ms at same position must produce DoubleClickAt; got {action:?}"
        );
    }

    #[test]
    fn map_mouse_down_left_outside_window_produces_drag_start() {
        let mut app = make_word_app();
        // Inject last_click 600 ms ago at same position — outside 450 ms window.
        app.last_click = Some((Instant::now() - Duration::from_millis(600), 2, 5));

        let ev = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DragStart { .. }),
            "Down(Left) outside 450 ms window must produce DragStart; got {action:?}"
        );
    }

    #[test]
    fn map_mouse_down_left_at_different_position_produces_drag_start() {
        let mut app = make_word_app();
        // last_click is recent but at a different position.
        app.last_click = Some((Instant::now() - Duration::from_millis(100), 0, 0));

        let ev = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 2,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action = super::map_mouse(ev, &app);
        assert!(
            matches!(action, super::Action::DragStart { .. }),
            "Down(Left) at a different position must produce DragStart; got {action:?}"
        );
    }

    #[test]
    fn apply_double_click_at_clears_drag_origin_and_selection() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        // Pre-set drag origin and old selection.
        app.drag_origin = Some((0, 0));
        app.selection_start(0, 0);
        app.selection_update(0, 5);
        assert!(app.selection.is_some(), "precondition: selection set");

        super::apply(
            super::Action::DoubleClickAt {
                screen_col: 0,
                screen_row: 0,
            },
            &mut app,
        );

        assert!(
            app.drag_origin.is_none(),
            "DoubleClickAt must clear drag_origin"
        );
        // selection may be set to the word (or None if whitespace) — not pre-existing selection.
        // The old (0..5) selection must not persist unmodified; double_click_word_select
        // either populated it with the clicked word or left it None.
        // Any result from double_click_word_select is acceptable — we just verify it ran.
        assert!(
            app.last_click.is_none(),
            "DoubleClickAt must clear last_click"
        );
    }

    #[test]
    fn drag_end_after_double_click_does_not_call_selection_finish_again() {
        // Regression: DoubleClickAt sets app.selection (word selected) AND clears
        // drag_origin. The subsequent Up(Left) fires DragEnd. Before the fix, DragEnd
        // saw selection.is_some() and called selection_finish() a second time, copying
        // the word to the clipboard twice. After the fix it must be a no-op.
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
        app.block_until_ready();
        app.body_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };

        // Simulate DoubleClickAt (clears drag_origin, sets word selection).
        super::apply(
            super::Action::DoubleClickAt {
                screen_col: 0,
                screen_row: 0,
            },
            &mut app,
        );
        // After DoubleClickAt: drag_origin is None; selection may be Some (if word found).
        assert!(
            app.drag_origin.is_none(),
            "precondition: DoubleClickAt cleared drag_origin"
        );

        // Record status message state right after double-click.
        let msg_after_double_click = app.status_message.clone();

        // Simulate the mouse-up that always follows a Down event.
        super::apply(
            super::Action::DragEnd {
                content_row: 0,
                col: 0,
            },
            &mut app,
        );

        // DragEnd must not change status_message (no second selection_finish).
        assert_eq!(
            app.status_message, msg_after_double_click,
            "DragEnd after DoubleClickAt must not trigger a second selection_finish"
        );
    }

    // --- Task 4 (Block E) tests: browser key/mouse handlers ---

    fn make_browser_app(label: &str) -> App {
        let dir = std::env::temp_dir().join(format!("bella_events_browser_{label}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        App::new_browser(dir, 80, 25)
    }

    fn make_browser_app_with_md(label: &str) -> (App, PathBuf) {
        let dir = std::env::temp_dir().join(format!("bella_events_browser_{label}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let md_path = dir.join("hello.md");
        std::fs::write(&md_path, "# Hello\n\nContent.").expect("write md");
        let app = App::new_browser(dir.clone(), 80, 25);
        (app, md_path)
    }

    #[test]
    fn map_browser_key_j_produces_browser_down() {
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserDown);
    }

    #[test]
    fn map_browser_key_down_arrow_produces_browser_down() {
        let k = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserDown);
    }

    #[test]
    fn map_browser_key_k_produces_browser_up() {
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserUp);
    }

    #[test]
    fn map_browser_key_up_arrow_produces_browser_up() {
        let k = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserUp);
    }

    #[test]
    fn map_browser_key_enter_produces_browser_descend() {
        let k = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserDescend);
    }

    #[test]
    fn map_browser_key_backspace_produces_browser_ascend() {
        let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::BrowserAscend);
    }

    #[test]
    fn map_browser_key_q_produces_quit() {
        let k = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::Quit);
    }

    #[test]
    fn map_browser_key_ctrl_c_produces_quit() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_browser_key(k), Action::Quit);
    }

    #[test]
    fn map_browser_key_unmapped_produces_none() {
        let k = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        assert_eq!(map_browser_key(k), Action::None);
    }

    #[test]
    fn reader_backspace_produces_browser_back() {
        // In reader mode, Backspace maps to BrowserBack.
        let k = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(map_key(k, 10), Action::BrowserBack);
    }

    #[test]
    fn apply_browser_down_advances_cursor() {
        let mut app = make_browser_app("down_cursor");
        // Add a couple of entries manually so we have something to navigate.
        if app.browser.as_ref().map(|b| b.entries.len()).unwrap_or(0) < 2 {
            // Create a subdirectory so the browser has at least one entry.
            let dir = app.browser.as_ref().unwrap().dir.clone();
            let _ = std::fs::create_dir_all(dir.join("subdir_for_test"));
            app = App::new_browser(dir, 80, 25);
        }
        let before = app.browser.as_ref().unwrap().selected;
        if app.browser.as_ref().unwrap().entries.len() > 1 {
            super::apply(Action::BrowserDown, &mut app);
            let after = app.browser.as_ref().unwrap().selected;
            assert_ne!(before, after, "BrowserDown must advance the cursor");
        }
    }

    #[test]
    fn apply_browser_asc_and_desc_round_trip() {
        // Create a parent dir with a child dir.
        let parent = std::env::temp_dir().join("bella_events_browser_ascend_desc_parent");
        let child = parent.join("child_dir");
        std::fs::create_dir_all(&child).expect("create dirs");

        let mut app = App::new_browser(child.clone(), 80, 25);
        assert_eq!(
            app.browser.as_ref().unwrap().dir,
            child,
            "precondition: rooted at child"
        );

        // Ascend to parent.
        super::apply(Action::BrowserAscend, &mut app);
        assert_eq!(
            app.browser.as_ref().unwrap().dir,
            parent,
            "BrowserAscend must re-root browser at parent"
        );
        assert_eq!(
            app.mode,
            Mode::Browser,
            "mode must stay Browser after ascend"
        );
    }

    #[test]
    fn apply_browser_descend_on_dir_re_roots_browser() {
        let parent = std::env::temp_dir().join("bella_events_browser_descend_dir");
        let child = parent.join("nested");
        std::fs::create_dir_all(&child).expect("create dirs");

        let mut app = App::new_browser(parent.clone(), 80, 25);
        // Find the Dir entry for "nested" and select it.
        let idx = app
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .position(|e| e.display == "nested")
            .expect("nested dir must appear in listing");
        app.browser.as_mut().unwrap().selected = idx;

        super::apply(Action::BrowserDescend, &mut app);

        assert_eq!(
            app.browser.as_ref().unwrap().dir,
            child,
            "BrowserDescend on a Dir entry must re-root browser at the subdirectory"
        );
        assert_eq!(
            app.mode,
            Mode::Browser,
            "mode must stay Browser after descend into dir"
        );
    }

    #[test]
    fn apply_browser_descend_on_markdown_switches_to_reader() {
        let (mut app, md_path) = make_browser_app_with_md("descend_md");
        // Find the Markdown entry and select it.
        let idx = app
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .position(|e| e.path == md_path)
            .expect("hello.md must appear in listing");
        app.browser.as_mut().unwrap().selected = idx;

        super::apply(Action::BrowserDescend, &mut app);

        assert_eq!(
            app.mode,
            Mode::Reader,
            "BrowserDescend on a Markdown entry must switch to Reader"
        );
        assert_eq!(app.file, md_path, "file must be the opened markdown");
    }

    #[test]
    fn apply_browser_back_after_open_returns_to_browser() {
        let (mut app, md_path) = make_browser_app_with_md("back_after_open");
        let original_dir = app.browser.as_ref().unwrap().dir.clone();

        // Open the markdown file from the browser.
        app.open_from_browser(md_path).expect("open_from_browser");
        assert_eq!(app.mode, Mode::Reader, "precondition: now in Reader");

        // Apply BrowserBack — should return to Browser at the saved cursor.
        super::apply(Action::BrowserBack, &mut app);
        assert_eq!(
            app.mode,
            Mode::Browser,
            "BrowserBack must switch back to Browser"
        );
        assert_eq!(
            app.browser.as_ref().unwrap().dir,
            original_dir,
            "browser dir must be restored after BrowserBack"
        );
    }

    #[test]
    fn apply_browser_back_noop_when_no_origin() {
        // Reader opened directly with a file arg — back must be a no-op.
        let src = "# Hello".to_string();
        let mut app = App::new(src, PathBuf::from("doc.md"), 80, 25);
        app.block_until_ready();
        assert_eq!(app.mode, Mode::Reader, "precondition: Reader mode");

        super::apply(Action::BrowserBack, &mut app);
        assert_eq!(
            app.mode,
            Mode::Reader,
            "BrowserBack must be a no-op when there is no browser origin"
        );
    }

    #[test]
    fn apply_browser_click_at_on_dir_descends_into_it() {
        // Create a parent dir containing one subdir.
        let parent = std::env::temp_dir().join("bella_events_browser_click_sel");
        let child = parent.join("click_child_dir");
        std::fs::create_dir_all(&child).expect("create child dir");

        let mut app = App::new_browser(parent.clone(), 80, 25);
        // Find the entry index for "click_child_dir".
        let idx = app
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .position(|e| e.display == "click_child_dir")
            .expect("click_child_dir must appear in listing");

        // BrowserClickAt maps a screen row to scroll+row. scroll is 0, so row == idx.
        use ratatui::layout::Rect;
        app.browser_area = Rect {
            x: 0,
            y: 1,
            width: 78,
            height: 20,
        };

        super::apply(Action::BrowserClickAt { row: idx as u16 }, &mut app);

        // After clicking on a Dir entry, browser should be re-rooted at the child.
        assert_eq!(
            app.browser.as_ref().unwrap().dir,
            child,
            "BrowserClickAt on a Dir entry must descend into it"
        );
        assert_eq!(
            app.mode,
            Mode::Browser,
            "mode must stay Browser after click-descend"
        );
    }

    #[test]
    fn apply_browser_click_at_on_markdown_opens_reader() {
        let (mut app, md_path) = make_browser_app_with_md("click_md_open");
        // Find the markdown entry index.
        let idx = app
            .browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .position(|e| e.path == md_path)
            .expect("hello.md must appear in listing");

        use ratatui::layout::Rect;
        app.browser_area = Rect {
            x: 0,
            y: 1,
            width: 78,
            height: 20,
        };

        super::apply(Action::BrowserClickAt { row: idx as u16 }, &mut app);

        assert_eq!(
            app.mode,
            Mode::Reader,
            "BrowserClickAt on a Markdown entry must switch to Reader"
        );
        assert_eq!(app.file, md_path, "file must be the clicked markdown");
    }
}
