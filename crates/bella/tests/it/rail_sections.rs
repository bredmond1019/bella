//! BE.7.F task 1: the rail section model (Contents + Metadata) drawn
//! through the real `draw_reader` path.
//!
//! `layout.rs` already pins the rail's outer geometry (rail column width,
//! x-offset, auto-collapse). This file is the section-stack gap: it must be
//! shown incapable of panicking when BOTH sections have zero rows — a
//! document with no headings and no frontmatter, the case a `len() - 1`
//! underflow would hit first.

use std::path::PathBuf;

use bella::app::{App, RailSection};
use bella::ui::draw_reader;
use ratatui::{Terminal, backend::TestBackend};

fn make_app_no_headings(width: u16, height: u16) -> App {
    // No `#` headings anywhere in the source, so `app.headings` is empty
    // whether the background render has landed yet or not (`block_until_ready`
    // is `pub(crate)` and unreachable from this integration-test binary —
    // geometry/panic behaviour does not depend on the render completing,
    // same as `golden_draw.rs`'s `make_reader_app`). Metadata is also
    // always empty in this task, so both rail sections have zero rows.
    let src = "Just a paragraph, no headings at all.".to_string();
    App::new(src, PathBuf::from("no_headings.md"), width, height)
}

/// Both sections empty, rail open and keyboard-focused on Contents: must
/// draw without panicking and must leave `rail_selected` at 0 rather than
/// underflowing.
#[test]
fn draw_with_both_sections_empty_does_not_panic_focus_on_contents() {
    let (width, height): (u16, u16) = (120, 40);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app_no_headings(width, height);
    assert!(app.headings.is_empty(), "precondition: no headings");
    app.rail_open = true;
    app.rail_focused = true;
    app.rail_section = RailSection::Contents;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert_eq!(
        app.rail_selected, 0,
        "an empty focused section must clamp selection to 0, never underflow"
    );
}

/// Same as above but focused on Metadata — the section this task adds and
/// which is always empty until task 2 parses real frontmatter.
#[test]
fn draw_with_both_sections_empty_does_not_panic_focus_on_metadata() {
    let (width, height): (u16, u16) = (120, 40);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app_no_headings(width, height);
    app.rail_open = true;
    app.rail_focused = true;
    app.rail_section = RailSection::Metadata;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();

    assert_eq!(
        app.rail_selected, 0,
        "an empty focused section must clamp selection to 0, never underflow"
    );
}

/// A short terminal drives the rail area down to a handful of rows —
/// exercises `rail_section_heights`' degrade-to-Contents-only branch
/// without panicking.
#[test]
fn draw_with_very_short_rail_does_not_panic() {
    let (width, height): (u16, u16) = (60, 3);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app_no_headings(width, height);
    app.rail_open = true;
    app.rail_focused = true;

    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut app);
        })
        .unwrap();
}

/// Switching focus between sections mid-session and re-drawing repeatedly
/// must stay panic-free — the case closest to real interactive use.
#[test]
fn cycling_focus_and_redrawing_repeatedly_does_not_panic() {
    let (width, height): (u16, u16) = (100, 30);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app_no_headings(width, height);
    app.rail_open = true;
    app.rail_focused = true;

    for _ in 0..4 {
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();
        app.cycle_rail_section();
    }
}
