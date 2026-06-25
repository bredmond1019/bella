//! Synchronous event loop: draw → read event → dispatch → repeat.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::App;
use crate::ui;

/// Action produced by the key mapper.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    ScrollDown(u16),
    ScrollUp(u16),
    ToTop,
    ToBottom,
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
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        _ => Action::None,
    }
}

/// Apply an `Action` to `app`.
fn apply(action: Action, app: &mut App) {
    match action {
        Action::ScrollDown(n) => app.scroll_down(n),
        Action::ScrollUp(n) => app.scroll_up(n),
        Action::ToTop => app.jump_top(),
        Action::ToBottom => app.jump_bottom(),
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
                let action = map_key(key, app.viewport_height);
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
}
