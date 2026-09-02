//! Structural golden buffer for `draw_reader` / `draw_browser`.
//!
//! Deliberately asserts GEOMETRY — region widths, x-offsets, pane boundaries,
//! and the status row's index — never cell contents or full-buffer text.
//! `crates/bella/src/ui.rs` already carries 17 TestBackend assertions and
//! they all assert text presence, which is exactly why a horizontal split
//! (BE.7.E) would pass most of them unchanged. This file is the narrow gap:
//! it must be shown failing against a layout change that the text-presence
//! suite cannot see. Pin structure only — a buffer that asserts every cell
//! churns on cosmetic changes and gets disabled, which loses the whole
//! deliverable. No snapshot/insta dependency; hand-written assertions only.
//!
//! The one assertion that must be shown failing against unfixed behaviour
//! (the resize off-by-one) belongs in BE.7.B task 2, alongside its fix — this
//! file only pins today's already-correct geometry.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bella::app::App;
use bella::ui::{draw_browser, draw_reader};
use bella_engine::browser::{BrowserEntry, BrowserEntryKind};
use ratatui::{Terminal, backend::TestBackend};

/// Three fixed terminal sizes exercised by every geometry assertion below:
/// a wide terminal, a standard 80-column terminal, and a short one.
const SIZES: [(u16, u16); 3] = [(120, 40), (80, 24), (80, 8)];

/// A collision-proof temp directory, created fresh under the system temp
/// dir. `crate::testsupport::unique_temp_dir` (used by `src/ui.rs`'s own
/// browser tests) is `mod testsupport` — crate-private and unreachable from
/// this integration-test binary — so this mirrors its pid+nanos+counter
/// scheme locally rather than risking a fixed-name collision across
/// concurrent runs sharing one `/tmp` (CLAUDE.md standing rule).
fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{seq}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("unique_temp_dir: failed to create fixture dir");
    dir
}

/// Build a reader-mode `App` over a small markdown document, without
/// blocking on the background render (`block_until_ready` is
/// `pub(crate)` and unreachable here) — geometry does not depend on the
/// render having completed, only on the terminal size passed in.
fn make_reader_app(width: u16, height: u16) -> App {
    let src = "# Hello\n\nSome paragraph text.".to_string();
    App::new(src, PathBuf::from("test.md"), width, height)
}

/// Build a browser-mode `App` rooted at a fresh temp dir, with the
/// auto-populated (real filesystem) entries cleared and two synthetic
/// entries pushed in their place — mirrors `src/ui.rs`'s own
/// `make_browser_app`/`push_entry` test helpers.
fn make_browser_app(width: u16, height: u16) -> App {
    let dir = unique_temp_dir("bella_golden_draw_browser");
    let mut app = App::new_browser(dir, width, height);
    if let Some(b) = app.browser.as_mut() {
        b.entries.clear();
        b.entries.push(BrowserEntry {
            path: PathBuf::from("docs"),
            display: "docs".to_string(),
            kind: BrowserEntryKind::Dir,
        });
        b.entries.push(BrowserEntry {
            path: PathBuf::from("README.md"),
            display: "README.md".to_string(),
            kind: BrowserEntryKind::Markdown,
        });
    }
    app
}

/// Reader mode, at every fixed size: the body region spans the full
/// terminal width starting at x = 0, and its height leaves exactly one row
/// for the status line at the bottom (`draw_reader`'s
/// `[Constraint::Min(0), Constraint::Length(1)]` split).
#[test]
fn reader_body_region_spans_full_width_with_one_status_row() {
    for (width, height) in SIZES {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_reader_app(width, height);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        // Body region: x-offset 0, full terminal width.
        assert_eq!(
            app.body_area.x, 0,
            "body region x-offset must be 0 at {width}x{height}"
        );
        assert_eq!(
            app.body_area.width, width,
            "body region width must span the full terminal width at {width}x{height}"
        );
        assert_eq!(
            app.body_area.y, 0,
            "body region must start at the top row at {width}x{height}"
        );

        // Pane boundary: the body region's height leaves exactly one row
        // for the status line — this IS the status row's index, expressed
        // structurally rather than as a bare number: status_row_y ==
        // body_area.height (the row immediately below the body region).
        let status_row_y = app.body_area.height;
        assert_eq!(
            status_row_y + 1,
            height,
            "status row must be the single row immediately below the body \
             region (status_row_y={status_row_y}, term_height={height})"
        );
    }
}

/// Browser mode, at every fixed size: the bordered inner listing area sits
/// inset by exactly one cell on each side of the bordered pane (`Borders::
/// ALL`), and the pane itself leaves exactly one row for the status line at
/// the bottom, mirroring `draw_reader`'s split.
#[test]
fn browser_pane_boundaries_and_status_row_at_every_size() {
    for (width, height) in SIZES {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_browser_app(width, height);

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let inner = app.browser_area;

        // Pane boundary: the border consumes exactly one cell on each side,
        // so the inner listing area starts at (1, 1).
        assert_eq!(
            inner.x, 1,
            "browser inner area must be inset 1 cell from the left border \
             at {width}x{height}"
        );
        assert_eq!(
            inner.y, 1,
            "browser inner area must be inset 1 cell from the top border \
             at {width}x{height}"
        );

        // Width: full terminal width minus the two border columns.
        assert_eq!(
            inner.width,
            width.saturating_sub(2),
            "browser inner width must be term_width - 2 (left+right border) \
             at {width}x{height}"
        );

        // Height / status row: the bordered pane's outer region reserves
        // exactly one row for the status line at the bottom (Constraint::
        // Length(1)), and the border itself consumes 2 more rows (top +
        // bottom) inside that. So inner.height + 2 (borders) + 1 (status)
        // must equal the full terminal height — this pins both the pane
        // boundary and the status row's position in one structural
        // relationship.
        let reconstructed_height = inner.height + 2 + 1;
        assert_eq!(
            reconstructed_height, height,
            "inner.height + border rows (2) + status row (1) must equal \
             the full terminal height at {width}x{height} \
             (inner.height={}, got total={reconstructed_height})",
            inner.height
        );
    }
}

/// The 80-column, standard-height case gets an explicit numeric pin in
/// addition to the size-swept relational assertions above — a concrete
/// worked example a reader can check by hand, at the size this app is most
/// commonly run at.
#[test]
fn reader_and_browser_geometry_pinned_at_80x24() {
    let (width, height): (u16, u16) = (80, 24);

    // Reader.
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut reader_app = make_reader_app(width, height);
    terminal
        .draw(|f| {
            draw_reader(f, f.area(), &mut reader_app);
        })
        .unwrap();
    assert_eq!(reader_app.body_area.x, 0);
    assert_eq!(reader_app.body_area.y, 0);
    assert_eq!(reader_app.body_area.width, 80);
    assert_eq!(reader_app.body_area.height, 23, "status line takes 1 row");

    // Browser.
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut browser_app = make_browser_app(width, height);
    terminal
        .draw(|f| {
            draw_browser(f, f.area(), &mut browser_app);
        })
        .unwrap();
    assert_eq!(browser_app.browser_area.x, 1);
    assert_eq!(browser_app.browser_area.y, 1);
    assert_eq!(browser_app.browser_area.width, 78, "term width - 2 borders");
    assert_eq!(
        browser_app.browser_area.height, 21,
        "term height (24) - 1 status row - 2 border rows"
    );
}
