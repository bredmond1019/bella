//! Synchronous event loop: draw → read event → dispatch → repeat.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::App;
use crate::history::HistoryEntry;
use crate::ui;

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
    Quit,
    None,
}

/// Pure key→action mapper (unit-testable without a live terminal).
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
            // If search is active, cancel it first; otherwise clear link focus.
            if app.search.is_some() {
                app.cancel_search();
            } else {
                app.clear_focus();
            }
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
    }
}

/// Synchronous event loop.  Draws, then blocks on the next terminal event.
pub fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut app: App,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            ui::draw_reader(f, f.area(), &mut app);
        })?;

        if app.should_quit {
            break;
        }

        match event::read()? {
            Event::Key(key) => {
                // In search input mode, character keys feed into the query instead of
                // the normal key bindings.
                let in_search_input = app.search.as_ref().map(|s| s.input_mode).unwrap_or(false);
                let action = if in_search_input {
                    map_search_key(key)
                } else {
                    map_key(key, app.viewport_height)
                };
                apply(action, &mut app);
            }
            Event::Resize(width, height) => {
                app.set_viewport_height(height.saturating_sub(1));
                app.render(width);
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

    use crate::app::App;

    use super::{Action, map_key};

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
        App::new(src, PathBuf::from("test.md"), 80, 11)
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
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchChar('i'), &mut app);
        assert_eq!(app.search.as_ref().unwrap().query, "hi");
    }

    #[test]
    fn apply_search_cancel_clears_search() {
        let src = "hello world".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 11);
        super::apply(Action::SearchStart, &mut app);
        super::apply(Action::SearchChar('h'), &mut app);
        super::apply(Action::SearchCancel, &mut app);
        assert!(app.search.is_none(), "SearchCancel must clear search state");
    }

    #[test]
    fn apply_search_commit_leaves_input_mode() {
        let src = "hello world\n\nanother line".to_string();
        let mut app = App::new(src, std::path::PathBuf::from("test.md"), 80, 25);
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
}
