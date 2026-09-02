//! Layout tests for BE.7.E: the horizontal frame (rail + body), the
//! minimum-body-width auto-collapse policy, and the single-writer invariant
//! on `App.width`.
//!
//! `crates/bella/tests/it/golden_draw.rs` already pins reader/browser
//! geometry with the rail off (the default) — this file is the rail-on gap:
//! it must be shown failing against a `draw_reader` that does not carve out
//! a rail region, and against an `events::handle_resize` that writes
//! `App.width` from terminal size instead of leaving that write to
//! `ui::draw_reader`'s body-derived value.

use std::path::PathBuf;

use bella::app::App;
use bella::events::handle_resize;
use bella::ui::draw_reader;
use ratatui::{Terminal, backend::TestBackend};

/// Mirrors `ui::draw_reader`'s private constants — kept in sync by the
/// assertions below rather than re-exported, since they are an
/// implementation detail of the layout, not part of the public contract.
const RAIL_WIDTH: u16 = 24;
const MIN_BODY_WIDTH: u16 = 20;

fn make_reader_app(width: u16, height: u16) -> App {
    let src = "# Hello\n\nSome paragraph text.".to_string();
    App::new(src, PathBuf::from("layout_test.md"), width, height)
}

/// Rail on, wide terminal: the rail occupies a fixed-width column on the
/// left, and the body takes the remainder — never the full terminal width.
#[test]
fn rail_on_splits_content_row_into_rail_and_body() {
    let (width, height): (u16, u16) = (120, 40);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_reader_app(width, height);
    app.rail_open = true;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert!(
        app.rail_visible,
        "a wide terminal must show the rail when open"
    );
    assert_eq!(app.rail_area.x, 0, "rail sits at the left edge");
    assert_eq!(
        app.rail_area.width, RAIL_WIDTH,
        "rail has a fixed column width"
    );
    assert_eq!(
        app.body_area.x, RAIL_WIDTH,
        "body starts immediately after the rail"
    );
    assert_eq!(
        app.body_area.width,
        width - RAIL_WIDTH,
        "body takes the remainder of the content row, not the full terminal width"
    );
}

/// Rail off (the default): the body occupies the same full-width region it
/// does today, matching `golden_draw.rs`'s `reader_body_region_spans_full_width_with_one_status_row`.
#[test]
fn rail_off_body_occupies_full_width_unchanged() {
    let (width, height): (u16, u16) = (120, 40);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_reader_app(width, height);
    assert!(!app.rail_open, "precondition: rail is closed by default");

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert!(!app.rail_visible);
    assert_eq!(app.body_area.x, 0);
    assert_eq!(
        app.body_area.width, width,
        "body spans the full terminal width"
    );
}

/// The core single-writer assertion: with the rail open, `App.width` must
/// end up equal to the BODY region's width, never the terminal's.
#[test]
fn app_width_is_body_width_not_terminal_width_when_rail_open() {
    let (width, height): (u16, u16) = (120, 40);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_reader_app(width, height);
    app.rail_open = true;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert_ne!(
        app.width, width,
        "App.width must not be left at the terminal width once a rail is open"
    );
    assert_eq!(
        app.width, app.body_area.width,
        "App.width must equal the body region's width"
    );
}

/// Below the minimum body width, the rail auto-collapses rather than
/// squeezing the body under its floor. A zero-width body is unreachable
/// through this path.
#[test]
fn rail_auto_collapses_below_minimum_body_width() {
    // Content width one column short of RAIL_WIDTH + MIN_BODY_WIDTH: too
    // narrow to fit a rail without squeezing the body under its floor.
    let width = RAIL_WIDTH + MIN_BODY_WIDTH - 1;
    let height: u16 = 24;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_reader_app(width, height);
    app.rail_open = true;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert!(
        !app.rail_visible,
        "the rail must auto-collapse when there is no room for both rail and \
         a usable body"
    );
    assert_eq!(
        app.body_area.width, width,
        "the body must take the full content width once the rail collapses"
    );
    assert!(
        app.body_area.width > 0,
        "a zero-width body must be unreachable"
    );
}

/// Exactly at the threshold, the rail is shown (the boundary is inclusive on
/// the wide side) and the body still meets its minimum width.
#[test]
fn rail_shown_exactly_at_minimum_threshold() {
    let width = RAIL_WIDTH + MIN_BODY_WIDTH;
    let height: u16 = 24;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_reader_app(width, height);
    app.rail_open = true;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert!(
        app.rail_visible,
        "at exactly the threshold the rail must show"
    );
    assert_eq!(app.body_area.width, MIN_BODY_WIDTH);
}

/// `events::handle_resize` — the extracted `Event::Resize` handler — must
/// never write `App.width`. This is the automated half of the single-writer
/// gate: `ui::draw_reader` above is the only place `App.width` may change.
///
/// The other half was verified by hand during implementation: temporarily
/// reinstating `app.width = width;` inside `handle_resize` was observed to
/// make this test fail (the sentinel below gets overwritten), then the line
/// was reverted and this test observed passing again.
#[test]
fn handle_resize_never_writes_app_width() {
    let mut app = make_reader_app(80, 24);
    app.width = 12345; // sentinel: no real width will ever equal this.

    handle_resize(&mut app, 999, 50);

    assert_eq!(
        app.width, 12345,
        "handle_resize must not write App.width — only ui::draw_reader may"
    );
    // The part handle_resize DOES own: viewport height from the new
    // terminal height, minus the 1-row status line.
    assert_eq!(app.viewport_height, 49);
}
