//! Draw functions: body reader + status line.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::messages::{Message, MessageLog, Severity};
use bella_engine::FrontmatterValue;
use bella_engine::browser::BrowserEntryKind;

/// Text shown in the Metadata section when the document has no frontmatter
/// at all (or a fence with zero parsed entries) — BE.7.F task 2's empty
/// state. Never a panic, never a blank pane: this is always the sole row
/// whenever [`App::frontmatter`] has nothing to show.
const METADATA_EMPTY_STATE: &str = "(no frontmatter)";

/// Render one [`FrontmatterValue`] as a single display string.
///
/// `List` values (`keywords`, `related`, `layer` are all lists in this
/// corpus) are joined with `", "` rather than rendered one item per rail
/// row — a rail row IS the frontmatter *key*, not a nested list, and this
/// pane has no sub-indentation model, so folding the list onto its key's
/// one row is the shape that fits without inventing one. `Raw` values are
/// shown verbatim; whatever shape the parser couldn't specifically
/// understand is still the value the document actually has.
fn format_frontmatter_value(value: &FrontmatterValue) -> String {
    match value {
        FrontmatterValue::Scalar(s) => s.clone(),
        FrontmatterValue::List(items) => items.join(", "),
        FrontmatterValue::Raw(s) => s.clone(),
    }
}

/// Truncate `text` to at most `max_width` display columns, appending an
/// ellipsis when it had to cut. TRUNCATES, NEVER WRAPS (BE.7.F task 2's
/// contract for the Metadata pane) — the rail's region width must never
/// change and no line may overflow into the body.
///
/// Cuts on CHARACTER boundaries via [`UnicodeWidthChar`], never on a byte
/// index — this corpus's frontmatter is full of em dashes and accented
/// text, and a byte-slice cut (`text.as_bytes()[..n]` / `&text[..n]` at an
/// arbitrary byte offset) panics the instant `n` lands inside a multi-byte
/// character's encoding. Width, not byte or char count, is what must fit
/// the column budget — a wide glyph and an ASCII letter don't cost the
/// same screen column.
fn truncate_to_width(text: &str, max_width: usize) -> String {
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    // Reserve 1 column for the ellipsis itself.
    let budget = max_width - 1;
    let mut taken_bytes = 0usize;
    let mut taken_width = 0usize;
    for (byte_idx, ch) in text.char_indices() {
        let cw = ch.width().unwrap_or(0);
        if taken_width + cw > budget {
            break;
        }
        taken_width += cw;
        taken_bytes = byte_idx + ch.len_utf8();
    }
    format!("{}…", &text[..taken_bytes])
}

/// Fixed column width of the TOC rail when it is drawn.
const RAIL_WIDTH: u16 = 24;

/// The narrowest a body region may ever be drawn at. Below
/// `RAIL_WIDTH + MIN_BODY_WIDTH` total content width, the rail auto-collapses
/// rather than squeezing the body under this floor — see [`rail_should_show`].
const MIN_BODY_WIDTH: u16 = 20;

/// Whether the rail should actually be drawn this frame, given the user's
/// toggle preference (`rail_open`) and the available content width.
///
/// This is the minimum-body-width policy: even with the rail toggled on,
/// a `content_width` too narrow to fit both `RAIL_WIDTH` and
/// `MIN_BODY_WIDTH` auto-collapses the rail so the body is never squeezed
/// below its usable floor. A zero-width body is unreachable through this
/// path — when the rail is hidden the body takes the full `content_width`,
/// which is only zero if the terminal itself is.
fn rail_should_show(rail_open: bool, content_width: u16) -> bool {
    rail_open && content_width >= RAIL_WIDTH + MIN_BODY_WIDTH
}

/// Draw the full viewer: an optional TOC rail beside the body (scrolled
/// markdown), plus a 1-row status line.
///
/// `ui.rs` is the SOLE writer of [`App::width`] (BE.7.E) — it is derived
/// here from the BODY region's width, which is only known once this layout
/// has been computed (with a rail open, body width != terminal width). No
/// other call site may assign `app.width`; `events.rs`'s resize handling
/// intentionally leaves it untouched and relies on the next draw to pick up
/// any width change through this function instead.
///
/// Returns the body area height so the caller can push it back into `App`
/// with [`App::set_viewport_height`].
pub fn draw_reader(frame: &mut Frame, area: Rect, app: &mut App) -> u16 {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // content row (rail + body)
            Constraint::Length(1), // status line
        ])
        .split(area);

    let content_area = outer[0];
    let status_area = outer[1];

    let rail_visible = rail_should_show(app.rail_open, content_area.width);
    app.rail_visible = rail_visible;
    if !rail_visible {
        // Auto-collapse (or the rail simply being off) must not leave
        // keyboard focus on an invisible target.
        app.rail_focused = false;
    } else {
        // Clamp against the FOCUSED section's own length (BE.7.F) — never
        // `headings.len()` unconditionally, since the Metadata section has
        // a different (possibly zero) row count. A section with zero rows
        // lands selection at 0 rather than underflowing on `len() - 1`.
        let len = app.rail_section_len(app.rail_section);
        app.rail_selected = if len == 0 {
            0
        } else {
            app.rail_selected.min(len - 1)
        };
    }

    let (rail_area, body_area) = if rail_visible {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(RAIL_WIDTH), Constraint::Min(0)])
            .split(content_area);
        (chunks[0], chunks[1])
    } else {
        (Rect::default(), content_area)
    };

    // Feed the real body height back into the app so max_scroll stays accurate.
    app.set_viewport_height(body_area.height);
    // Store the body/rail areas for mouse coordinate conversion in event handlers.
    app.body_area = body_area;
    app.rail_area = rail_area;

    // The single write site for `App.width`: re-render whenever the body
    // region's width (not the terminal's) has changed since the last render.
    if body_area.width != app.width {
        app.width = body_area.width;
        app.render(body_area.width);
    }

    draw_body(frame, body_area, app);
    if rail_visible {
        draw_rail(frame, rail_area, app);
    }
    draw_statusline(frame, status_area, app);

    // Diagnostics overlay (BE.7.K task 2): drawn last, over whatever the
    // rest of this function just rendered, so dismissing it (setting
    // `diagnostics_open = false` and redrawing) reproduces the exact
    // pre-overlay buffer — nothing above this point reads `diagnostics_open`.
    if app.diagnostics_open {
        draw_diagnostics_overlay(frame, area, &app.message_log);
    }

    body_area.height
}

/// Floor on a rail section's on-screen height (BE.7.F): 2 rows, exactly
/// enough for a bordered `Block`'s top+bottom border with zero content
/// rows between them. Never asked to go below this except when the whole
/// rail area itself is shorter than it.
const MIN_SECTION_HEIGHT: u16 = 2;

/// Split a rail area of `total_height` rows between the Contents and
/// Metadata sections. **Policy**: Metadata is content-driven — its wanted
/// height is its row count plus 2 border rows, floored at
/// [`MIN_SECTION_HEIGHT`] so it always has a frame to draw into (including
/// the empty-state line BE.7.F task 2 adds when there is no frontmatter at
/// all) — and Contents/TOC takes whatever height is left over. When the
/// rail is too short to give both sections a useful frame, Metadata is the
/// one that degrades: below `2 * MIN_SECTION_HEIGHT` total rows, Contents
/// (the pane every prior version of the rail already had) gets everything
/// and Metadata is not drawn this frame. Returns `(contents_height,
/// metadata_height)`; neither exceeds `total_height` and their sum never
/// does either.
fn rail_section_heights(total_height: u16, metadata_rows: usize) -> (u16, u16) {
    if total_height == 0 {
        return (0, 0);
    }
    if total_height <= MIN_SECTION_HEIGHT * 2 {
        return (total_height, 0);
    }
    let wanted_metadata = (metadata_rows as u16).saturating_add(2);
    let max_metadata = total_height - MIN_SECTION_HEIGHT;
    let metadata_height = wanted_metadata.clamp(MIN_SECTION_HEIGHT, max_metadata);
    let contents_height = total_height - metadata_height;
    (contents_height, metadata_height)
}

/// Split a rail area of `total_height` rows between Contents, Metadata,
/// and Tree (BE.7.H task 2) — the three-section extension of
/// [`rail_section_heights`], reusing its exact policy per section
/// (content-driven height, floored at [`MIN_SECTION_HEIGHT`]) rather than
/// inventing a second one. Tree is carved out first (Metadata's existing
/// degrade-to-Contents-only branch, reused via [`rail_section_heights`],
/// covers the two-section case unchanged), then Metadata, then Contents
/// gets whatever remains — the same "the pane every prior version of the
/// rail already had gets the leftover space" policy `rail_section_heights`
/// already documents. Below three sections' worth of floor
/// (`MIN_SECTION_HEIGHT * 3`), Tree drops out entirely and this defers to
/// the existing two-way split, so the pre-BE.7.H degrade behaviour is
/// unchanged rather than re-derived. Returns `(contents_height,
/// metadata_height, tree_height)`; their sum never exceeds `total_height`.
fn rail_section_heights3(
    total_height: u16,
    metadata_rows: usize,
    tree_rows: usize,
) -> (u16, u16, u16) {
    if total_height == 0 {
        return (0, 0, 0);
    }
    if total_height <= MIN_SECTION_HEIGHT * 3 {
        let (contents_height, metadata_height) = rail_section_heights(total_height, metadata_rows);
        return (contents_height, metadata_height, 0);
    }
    let wanted_tree = (tree_rows as u16).saturating_add(2);
    let max_tree = total_height - MIN_SECTION_HEIGHT * 2;
    let tree_height = wanted_tree.clamp(MIN_SECTION_HEIGHT, max_tree);

    let remaining = total_height - tree_height;
    let (contents_height, metadata_height) = rail_section_heights(remaining, metadata_rows);
    (contents_height, metadata_height, tree_height)
}

/// Draw the rail: a stack of titled sections (BE.7.F) sharing the rail's
/// vertical space per [`rail_section_heights`]. Writes
/// [`App::rail_contents_area`] / [`App::rail_metadata_area`] so mouse
/// clicks can be routed to the section that owns the row
/// (`events::map_mouse`).
fn draw_rail(frame: &mut Frame, area: Rect, app: &mut App) {
    // BE.7.G task 3's "built lazily on first `related:` use": the rail is
    // the one place that actually needs a resolution, and this runs once
    // per drawn frame (never inside `load_file`) — `ensure_doc_index` is
    // itself idempotent past the first call, so this only ever spawns the
    // background build once per session, the moment a document with a
    // non-empty `related:` list is actually about to be shown.
    if app.frontmatter_has_related_entries() {
        app.ensure_doc_index();
    }
    let metadata_rows = app.rail_section_len(crate::app::RailSection::Metadata);
    let tree_rows = app.rail_section_len(crate::app::RailSection::Tree);
    let (contents_height, metadata_height, tree_height) =
        rail_section_heights3(area.height, metadata_rows, tree_rows);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(contents_height),
            Constraint::Length(metadata_height),
            Constraint::Length(tree_height),
        ])
        .split(area);
    let contents_area = chunks[0];
    let metadata_area = if metadata_height > 0 {
        chunks[1]
    } else {
        Rect::default()
    };
    let tree_area = if tree_height > 0 {
        chunks[2]
    } else {
        Rect::default()
    };

    if contents_height > 0 {
        draw_rail_contents(frame, contents_area, app);
        app.rail_contents_area = contents_area;
    } else {
        app.rail_contents_area = Rect::default();
    }
    if metadata_height > 0 {
        draw_rail_metadata(frame, metadata_area, app);
        app.rail_metadata_area = metadata_area;
    } else {
        app.rail_metadata_area = Rect::default();
    }
    if tree_height > 0 {
        draw_rail_tree(frame, tree_area, app);
        app.rail_tree_area = tree_area;
    } else {
        app.rail_tree_area = Rect::default();
    }
}

/// Draw the Contents (table-of-contents) section: a bordered pane listing
/// `app.headings`, one per row (indented by level, deepest levels clipped
/// by the fixed [`RAIL_WIDTH`] the same way any long line is). The row
/// under keyboard focus ([`App::rail_selected`], only meaningful while
/// [`App::rail_focused`] AND this section is the focused one) is
/// highlighted so [`App::activate_rail_selection`] has a visible target —
/// see BE.7.E task 2's keyboard-parity requirement.
fn draw_rail_contents(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Contents")
        .style(Style::default().fg(app.theme.status_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section_focused = app.rail_focused && app.rail_section == crate::app::RailSection::Contents;
    let lines: Vec<Line> = app
        .headings
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            let indent = "  ".repeat((h.level.saturating_sub(1)) as usize);
            let text = format!("{indent}{}", h.text);
            let selected = section_focused && idx == app.rail_selected;
            let style = if selected {
                Style::default()
                    .fg(app.theme.status_bg)
                    .bg(app.theme.status_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render one [`MetadataRow`] as `(display text, base style)`, before any
/// selection highlight is applied.
///
/// A [`MetadataRow::Plain`] row renders exactly as BE.7.F's original
/// `key: value` line. A [`MetadataRow::Related`] row (BE.7.G task 3) gets
/// a distinct leading marker AND a distinct color PER STATE — two
/// independent signals, so the states stay distinguishable even in a
/// theme or terminal where one of the two doesn't render (a colorless
/// capture, a color-blind reader): resolved (`→`, green), unresolved
/// (`✗`, yellow), ambiguous (`≠`, magenta), still building (`…`, cyan),
/// and a failed index build (`!`, red).
fn metadata_row_display(row: &crate::app::MetadataRow) -> (String, Style) {
    use crate::app::{MetadataRow, RelatedRowState};
    match row {
        MetadataRow::Plain(key, value) => (
            format!("{key}: {}", format_frontmatter_value(value)),
            Style::default(),
        ),
        MetadataRow::Related { doc_id, state } => match state {
            RelatedRowState::Resolved(_) => {
                (format!("→ {doc_id}"), Style::default().fg(Color::Green))
            }
            RelatedRowState::Unresolved => (
                format!("✗ {doc_id} (unresolved)"),
                Style::default().fg(Color::Yellow),
            ),
            RelatedRowState::Ambiguous(_) => (
                format!("≠ {doc_id} (ambiguous)"),
                Style::default().fg(Color::Magenta),
            ),
            RelatedRowState::Building => (
                format!("… {doc_id} (building)"),
                Style::default().fg(Color::Cyan),
            ),
            RelatedRowState::Failed(_) => (
                format!("! {doc_id} (index failed)"),
                Style::default().fg(Color::Red),
            ),
        },
    }
}

/// Draw the Metadata section: the current document's parsed frontmatter,
/// one row per entry, in SOURCE order (never re-sorted — see
/// `bella_engine::frontmatter`'s module doc, which names this pane as the
/// reason the type is a `Vec` and not a map) — EXCEPT `related:`, which
/// [`App::metadata_rows`] expands to one row per item so each `doc_id`
/// reference is individually clickable/activatable (BE.7.G task 3). A
/// document with no frontmatter (or a fence with zero entries) renders
/// [`METADATA_EMPTY_STATE`] instead of an empty pane, so the pane is never
/// indistinguishable from a broken one. The row under keyboard focus
/// (mirrors [`draw_rail_contents`]'s highlight) is only ever a real row's
/// index — [`App::rail_section_len`] floors at `1` so the empty state has
/// a row to occupy, but `rail_selected == 0` on an empty section
/// highlights the empty-state line itself, which is harmless (there is
/// nothing to activate onto either way — see [`App::rail_click`]).
fn draw_rail_metadata(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Metadata")
        .style(Style::default().fg(app.theme.status_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section_focused = app.rail_focused && app.rail_section == crate::app::RailSection::Metadata;
    let inner_width = inner.width as usize;

    let rows = app.metadata_rows();

    let lines: Vec<Line> = if rows.is_empty() {
        let style = if section_focused && app.rail_selected == 0 {
            Style::default()
                .fg(app.theme.status_bg)
                .bg(app.theme.status_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        vec![Line::from(Span::styled(
            truncate_to_width(METADATA_EMPTY_STATE, inner_width),
            style,
        ))]
    } else {
        rows.iter()
            .enumerate()
            .map(|(idx, row)| {
                let (text, base_style) = metadata_row_display(row);
                let truncated = truncate_to_width(&text, inner_width);
                let selected = section_focused && idx == app.rail_selected;
                let style = if selected {
                    Style::default()
                        .fg(app.theme.status_bg)
                        .bg(app.theme.status_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    base_style
                };
                Line::from(Span::styled(truncated, style))
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Draw the Tree section (BE.7.H task 2): the corpus tree
/// ([`App::tree`]), expandable in place beside the document being read.
/// Each row is indented by `entry.depth * 2` spaces and carries a marker
/// distinguishing its [`BrowserEntryKind`] — `▸ ` for a collapsed `Dir`,
/// `▾ ` for an `ExpandedDir`, no marker for `Markdown`/`ParentDir` — the
/// same visual language [`draw_browser`] already uses for the full-screen
/// listing, reused here rather than invented fresh. The row under
/// keyboard focus mirrors [`draw_rail_contents`]'s highlight.
/// [`App::tree`] is always `Some` (both constructors build one), but this
/// still handles `None` defensively rather than panic, the same caution
/// [`draw_browser`] takes over [`App::browser`].
fn draw_rail_tree(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Tree")
        .style(Style::default().fg(app.theme.status_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section_focused = app.rail_focused && app.rail_section == crate::app::RailSection::Tree;
    let inner_width = inner.width as usize;
    let dir_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let lines: Vec<Line> = app
        .tree
        .as_ref()
        .map(|tree| tree.entries.as_slice())
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let indent = "  ".repeat(entry.depth);
            let marker = match entry.kind {
                BrowserEntryKind::Dir => "▸ ",
                BrowserEntryKind::ExpandedDir => "▾ ",
                BrowserEntryKind::ParentDir | BrowserEntryKind::Markdown => "  ",
            };
            let text = format!("{indent}{marker}{}", entry.display);
            let truncated = truncate_to_width(&text, inner_width);
            let base_style = match entry.kind {
                BrowserEntryKind::ParentDir
                | BrowserEntryKind::Dir
                | BrowserEntryKind::ExpandedDir => dir_style,
                BrowserEntryKind::Markdown => Style::default(),
            };
            let selected = section_focused && idx == app.rail_selected;
            let style = if selected {
                Style::default()
                    .fg(app.theme.status_bg)
                    .bg(app.theme.status_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                base_style
            };
            Line::from(Span::styled(truncated, style))
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Draw the directory browser: a bordered full-screen pane titled with the
/// current directory path.
///
/// Each entry is rendered as a single row:
/// - `▶ ` prefix on the selected row, `  ` otherwise.
/// - [`BrowserEntryKind::Dir`] and [`BrowserEntryKind::ParentDir`] entries are
///   styled bold cyan; [`BrowserEntryKind::Markdown`] entries are plain.
///
/// The inner listing [`Rect`] is stored on [`App::browser_area`] after each
/// draw so that Task 4's mouse handlers can map click coordinates to rows.
///
/// Reserves a 1-row status line at the bottom (mirroring [`draw_reader`]'s
/// body+status-line split) so browser mode always shows the current
/// directory and selection position, instead of leaving the space below a
/// short entry list blank.
pub fn draw_browser(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.browser.is_none() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // bordered entry list
            Constraint::Length(1), // status line
        ])
        .split(area);
    let list_area = chunks[0];
    let status_area = chunks[1];

    let browser = match &app.browser {
        Some(b) => b,
        None => return,
    };

    // Bordered pane titled with the current directory.
    let title = browser.dir.to_string_lossy().into_owned();
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(list_area);

    // Store the inner area for Task 4 mouse hit-testing.
    app.browser_area = inner;

    // Render the border first.
    frame.render_widget(block, list_area);

    // Borrow browser again now that app.browser_area has been set.
    let browser = match &app.browser {
        Some(b) => b,
        None => return,
    };

    let scroll = browser.scroll as usize;
    let visible_height = inner.height as usize;
    let selected = browser.selected;

    let dir_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let file_style = Style::default();

    for (row, entry_idx) in (scroll..).take(visible_height).enumerate() {
        let entry = match browser.entries.get(entry_idx) {
            Some(e) => e,
            None => break,
        };

        let is_selected = entry_idx == selected;
        let prefix = if is_selected { "▶ " } else { "  " };

        let style = match entry.kind {
            // `ExpandedDir` (BE.7.H task 1's widening) is still a
            // directory-styled row here — the expand/collapse marker and
            // indentation the tree pane needs are BE.7.H task 2's own
            // deliverable, not this block's; this arm only keeps the match
            // exhaustive so the enum widening compiles.
            BrowserEntryKind::ParentDir | BrowserEntryKind::Dir | BrowserEntryKind::ExpandedDir => {
                dir_style
            }
            BrowserEntryKind::Markdown => file_style,
        };

        let line = Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(entry.display.clone(), style),
        ]);

        let row_area = Rect {
            x: inner.x,
            y: inner.y + row as u16,
            width: inner.width,
            height: 1,
        };

        frame.render_widget(Paragraph::new(line), row_area);
    }

    draw_browser_statusline(
        frame,
        status_area,
        browser,
        &app.theme,
        app.message_log.latest(),
    );

    // Diagnostics overlay (BE.7.K task 2): see the matching comment in
    // `draw_reader` — drawn last, over the fully-rendered browser frame, so
    // it opens identically from either focus and dismissal is a no-op on
    // the underlying buffer.
    if app.diagnostics_open {
        draw_diagnostics_overlay(frame, area, &app.message_log);
    }
}

/// Render browser mode's status line: current directory, selection position,
/// a compact keybinding hint, and (BE.7.K task 2) the latest retained
/// diagnostic message, if any.
///
/// Before this task, browser focus was a silent mode for the diagnostic
/// channel: this function took only `frame`/`area`/`browser`/`theme` and had
/// no way to show a message even when one existed (`draw_statusline`, the
/// reader-mode equivalent, never runs while the browser is on screen). The
/// `latest` parameter closes that gap. Styled to match [`draw_statusline`]'s
/// theme-driven status bar for reader mode.
fn draw_browser_statusline(
    frame: &mut Frame,
    area: Rect,
    browser: &bella_engine::browser::Browser,
    theme: &bella_engine::Theme,
    latest: Option<&Message>,
) {
    let dir = browser.dir.to_string_lossy();
    let total = browser.entries.len();
    let position = if total == 0 {
        "0/0".to_string()
    } else {
        format!("{}/{}", browser.selected + 1, total)
    };
    let reveal = if browser.reveal_ignored { "on" } else { "off" };
    let dropped = if browser.dropped_entries > 0 {
        format!(" · {} entries dropped", browser.dropped_entries)
    } else {
        String::new()
    };
    let message = latest.map(|m| format!(" · {}", m.text)).unwrap_or_default();
    let text = format!(
        " bella · {dir} · {position} · j/k nav · Enter open · r reveal ({reveal}) · q quit{dropped}{message}"
    );
    let line = Line::from(vec![Span::styled(
        text,
        Style::default().fg(theme.status_fg).bg(theme.status_bg),
    )]);
    frame.render_widget(Paragraph::new(line), area);
}

/// Render the diagnostics overlay: every retained message, newest first, in
/// a bordered box centered over whatever `draw_reader`/`draw_browser` just
/// rendered (BE.7.K task 2). This is the only place [`MessageLog`] is read
/// for display — the ring itself is written by task 1 (retention) and task
/// 3 (routing bella's existing silent diagnostic paths into it).
fn draw_diagnostics_overlay(frame: &mut Frame, area: Rect, log: &MessageLog) {
    let overlay_area = centered_rect(80, 60, area);
    // Clear the popup region first — without this, the underlying frame's
    // glyphs and colors show through wherever the overlay's own paragraph
    // has no styled span to overwrite them.
    frame.render_widget(Clear, overlay_area);

    let block = Block::default().borders(Borders::ALL).title(" Messages ");
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let lines: Vec<Line> = if log.is_empty() {
        vec![Line::from("(no messages)")]
    } else {
        log.iter_newest_first()
            .map(|m| {
                let (tag, style) = match m.severity {
                    Severity::Info => ("INFO", Style::default()),
                    Severity::Warning => (
                        "WARN",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Severity::Error => (
                        "ERROR",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                };
                Line::from(vec![
                    Span::styled(format!("[{tag}] "), style),
                    Span::raw(m.text.clone()),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Compute a centered `Rect` occupying `percent_x`/`percent_y` of `area`.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    let start = app.scroll as usize;
    let end = (start + area.height as usize).min(app.lines.len());
    let mut visible: Vec<Line<'static>> = app.lines[start..end].to_vec();

    // Overlay search-match highlights (applied before the link-focus highlight so
    // the focused-link style wins if they overlap).
    if let Some(s) = &app.search
        && !s.input_mode
        && !s.query.is_empty()
    {
        let current_line = s.matches.get(s.current).copied();
        let match_style = Style::default().fg(Color::Black).bg(Color::Yellow);
        let current_style = Style::default().fg(Color::Black).bg(Color::Cyan);
        for &doc_line in &s.matches {
            if doc_line >= start && doc_line < end {
                let row = doc_line - start;
                let style = if Some(doc_line) == current_line {
                    current_style
                } else {
                    match_style
                };
                if let Some((col_start, col_end)) = find_query_col(&visible[row], &s.query) {
                    visible[row] =
                        apply_span_highlight(visible[row].clone(), col_start, col_end, style);
                }
            }
        }
    }

    // Overlay hovered-link highlight (applied before focused-link so keyboard
    // focus wins on overlap).
    if let Some(hovered_idx) = app.hovered_link
        && let Some(span) = app.link_map.links.get(hovered_idx)
    {
        let doc_line = span.line;
        if doc_line >= start && doc_line < end {
            let row = doc_line - start;
            let hover_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED);
            visible[row] = apply_span_highlight(
                visible[row].clone(),
                span.col_start,
                span.col_end,
                hover_style,
            );
        }
    }

    // Overlay focused-link highlight if a link is focused and on a visible row.
    if let Some(focused_idx) = app.focused_link
        && let Some(span) = app.link_map.links.get(focused_idx)
    {
        let doc_line = span.line;
        if doc_line >= start && doc_line < end {
            let row = doc_line - start;
            let highlight = Style::default().add_modifier(Modifier::REVERSED);
            visible[row] = apply_span_highlight(
                visible[row].clone(),
                span.col_start,
                span.col_end,
                highlight,
            );
        }
    }

    // Overlay toggled-checkbox glyphs. For each visually-toggled checkbox, swap
    // the rendered `[ ]`/`[x]` glyph to its opposite state.
    for &cb_idx in &app.toggled_checkboxes {
        if let Some(cb) = app.checkbox_map.items.get(cb_idx) {
            let doc_line = cb.line;
            if doc_line >= start && doc_line < end {
                let row = doc_line - start;
                // Determine the replacement glyph (flip the original checked state).
                let replacement = if cb.checked { "[ ]" } else { "[x]" };
                visible[row] =
                    replace_glyph_at(visible[row].clone(), cb.col_start, cb.col_end, replacement);
            }
        }
    }

    // Overlay active selection highlight (applied last so it appears on top of
    // other overlays).  The LightBlue background is visually distinct from the
    // Yellow/Cyan search styles, Cyan-underline hover, and REVERSED focus styles.
    if let Some(sel) = &app.selection
        && !sel.is_empty()
    {
        let ((start_row, start_col), (end_row, end_col)) = sel.normalized();
        let selection_style = Style::default().fg(Color::Black).bg(Color::LightBlue);

        for doc_line in start_row..=end_row {
            if doc_line >= start && doc_line < end {
                let row = doc_line - start;
                // Full char-count of this visible row (for whole-line coverage).
                let line_len: usize = visible[row]
                    .spans
                    .iter()
                    .map(|s| s.content.chars().count())
                    .sum();
                let (col_s, col_e) = if doc_line == start_row && doc_line == end_row {
                    // Single-row selection.
                    (start_col.min(line_len), end_col.min(line_len))
                } else if doc_line == start_row {
                    // Head: from start_col to the end of the line.
                    (start_col.min(line_len), line_len)
                } else if doc_line == end_row {
                    // Tail: from the start of the line to end_col.
                    (0, end_col.min(line_len))
                } else {
                    // Middle: highlight the entire line.
                    (0, line_len)
                };
                if col_s < col_e {
                    visible[row] =
                        apply_span_highlight(visible[row].clone(), col_s, col_e, selection_style);
                }
            }
        }
    }

    let paragraph = Paragraph::new(visible).block(Block::default());
    frame.render_widget(paragraph, area);
}

/// Find the first case-insensitive occurrence of `query` in a rendered `Line`.
///
/// Returns `(col_start, col_end)` as character-column positions, or `None` if
/// the query is empty or not found.
fn find_query_col(line: &Line, query: &str) -> Option<(usize, usize)> {
    if query.is_empty() {
        return None;
    }
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let byte_pos = text_lower.find(&query_lower)?;
    // Slice text_lower (not text) — byte_pos is a valid boundary in text_lower,
    // but lowercasing can change UTF-8 byte lengths, making it invalid in text.
    let col_start = text_lower[..byte_pos].chars().count();
    let col_end = col_start + query_lower.chars().count();
    Some((col_start, col_end))
}

/// Apply `highlight` style to columns `[col_start, col_end)` within `line`,
/// splitting existing spans at the boundaries so the rest of the line is unchanged.
fn apply_span_highlight(
    line: Line<'static>,
    col_start: usize,
    col_end: usize,
    highlight: Style,
) -> Line<'static> {
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut col = 0usize;

    for span in line.spans {
        let text = span.content.as_ref();
        let char_count = text.chars().count();
        let span_start = col;
        let span_end = col + char_count;
        col = span_end;

        if span_end <= col_start || span_start >= col_end {
            // No overlap — pass through unchanged.
            result.push(span);
        } else {
            let chars: Vec<char> = text.chars().collect();

            // Characters before the highlight window.
            if span_start < col_start {
                let pre: String = chars[..col_start - span_start].iter().collect();
                result.push(Span::styled(pre, span.style));
            }

            // Characters inside the highlight window.
            let hl_local_start = col_start.saturating_sub(span_start);
            let hl_local_end = (col_end - span_start).min(char_count);
            let hl: String = chars[hl_local_start..hl_local_end].iter().collect();
            result.push(Span::styled(hl, highlight));

            // Characters after the highlight window.
            if span_end > col_end {
                let post: String = chars[col_end - span_start..].iter().collect();
                result.push(Span::styled(post, span.style));
            }
        }
    }

    Line::from(result)
}

/// Replace the characters in `[col_start, col_end)` of `line` with `replacement`.
///
/// The replacement text is inserted with the style of the span that contained
/// `col_start`, preserving surrounding text unchanged. Used for checkbox glyph
/// toggling where we swap `[ ]` ↔ `[x]`.
fn replace_glyph_at(
    line: Line<'static>,
    col_start: usize,
    col_end: usize,
    replacement: &'static str,
) -> Line<'static> {
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut col = 0usize;
    let mut replaced = false;

    for span in line.spans {
        let text = span.content.as_ref();
        let char_count = text.chars().count();
        let span_start = col;
        let span_end = col + char_count;
        col = span_end;

        if span_end <= col_start || span_start >= col_end {
            // No overlap — pass through unchanged.
            result.push(span);
        } else {
            let chars: Vec<char> = text.chars().collect();

            // Characters before the replacement window.
            if span_start < col_start {
                let pre: String = chars[..col_start - span_start].iter().collect();
                result.push(Span::styled(pre, span.style));
            }

            // Insert the replacement only once (the first overlapping span).
            if !replaced {
                result.push(Span::styled(replacement, span.style));
                replaced = true;
            }

            // Characters after the replacement window.
            if span_end > col_end {
                let post: String = chars[col_end - span_start..].iter().collect();
                result.push(Span::styled(post, span.style));
            }
            // Characters inside the window from subsequent spans are dropped
            // (replaced by the single replacement span above).
        }
    }

    Line::from(result)
}

fn draw_statusline(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(s) = &app.search {
        // Search mode: show the prompt with the live query and (after commit) match count.
        if s.input_mode {
            format!("/{}_", s.query)
        } else if s.matches.is_empty() {
            format!("/{} [no matches]", s.query)
        } else {
            format!("/{} [{}/{}]", s.query, s.current + 1, s.matches.len())
        }
    } else if let Some(msg) = &app.status_message {
        // Show the non-fatal status message (e.g. file-not-found) instead of
        // the normal scroll position.  It stays visible until the next action
        // that clears or replaces it.
        format!(" bella · {msg}")
    } else {
        let file_name = app
            .file()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let total = app.lines.len();
        let current = (app.scroll as usize + app.viewport_height as usize).min(total);
        format!(
            " bella · {file_name} · {current}/{total}  j/k scroll · / search · [ ] history · q quit"
        )
    };
    let line = Line::from(vec![Span::styled(
        text,
        Style::default()
            .fg(app.theme.status_fg)
            .bg(app.theme.status_bg),
    )]);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::{Terminal, backend::TestBackend};

    use crate::app::{App, RailSection};

    use super::{RAIL_WIDTH, draw_reader};

    /// Build an app from a multi-line markdown document.
    fn make_app(src: &str, width: u16, height: u16) -> App {
        let mut app = App::new(src.to_owned(), PathBuf::from("test.md"), width, height);
        app.block_until_ready();
        app
    }

    #[test]
    fn draw_renders_heading_in_body() {
        let src = "# Hello\n\nSome paragraph text here.";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_app(src, width, height);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        // The engine may place headings on any body row; check all visible rows
        // (rows 0..height-1; row height-1 is the status line).
        let body_rows: Vec<String> = (0..height - 1)
            .map(|y| {
                (0..width)
                    .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect();
        let found = body_rows.iter().any(|row| row.contains("Hello"));
        assert!(
            found,
            "a body row should contain heading text 'Hello'; rows: {body_rows:#?}"
        );
    }

    // --- Task 3 tests: focused-link highlight ---

    #[test]
    fn focused_link_row_differs_from_unfocused() {
        // Doc containing a link on the first rendered line.
        let src = "[click me](other.md)\n\nSome other text.";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // Draw without focus.
        let mut app_unfocused = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_unfocused);
            })
            .unwrap();
        let buf_unfocused = terminal.backend().buffer().clone();

        // Draw with focus on the first link.
        let mut app_focused = make_app(src, width, height);
        assert!(
            !app_focused.link_map.links.is_empty(),
            "precondition: link exists"
        );
        app_focused.focused_link = Some(0);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_focused);
            })
            .unwrap();
        let buf_focused = terminal.backend().buffer().clone();

        // At minimum one cell in the link's row must differ (the focused style
        // applies REVERSED, so either fg/bg or modifier is flipped).
        let link_line = app_focused.link_map.links[0].line;
        // The link is on a rendered doc line; find the matching body row.
        // (body rows start at terminal row 0 when scroll==0)
        let body_row = link_line as u16;
        let col_start = app_focused.link_map.links[0].col_start as u16;
        let col_end = app_focused.link_map.links[0].col_end as u16;

        let any_diff = (col_start..col_end).any(|x| {
            buf_unfocused
                .cell((x, body_row))
                .zip(buf_focused.cell((x, body_row)))
                .map(|(u, f)| u.style() != f.style())
                .unwrap_or(false)
        });
        assert!(
            any_diff,
            "at least one cell in the focused link span must have a different style \
             compared to the unfocused render"
        );
    }

    #[test]
    fn scroll_offset_shifts_rendered_output() {
        // Build a document with enough distinct headings to exceed the viewport.
        let lines: Vec<String> = (1..=30).map(|i| format!("# Section {i}")).collect();
        let src = lines.join("\n\n");
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_app(&src, width, height);

        // Draw at scroll = 0, capture first body row.
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();
        let buf_before = terminal.backend().buffer().clone();
        let row0_before: String = (0..width)
            .map(|x| buf_before.cell((x, 0)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        // Scroll down and redraw.
        app.scroll_down(3);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();
        let buf_after = terminal.backend().buffer().clone();
        let row0_after: String = (0..width)
            .map(|x| buf_after.cell((x, 0)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert_ne!(
            row0_before, row0_after,
            "first body row should change after scrolling"
        );
    }

    #[test]
    fn draw_reader_status_line_shows_keybinding_hint() {
        let src = "# Hello World\n\nSome text.";
        let width: u16 = 120;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_app(src, width, height);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let status_row: String = (0..width)
            .map(|x| buf.cell((x, height - 1)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert!(
            status_row.contains("q quit"),
            "reader status line must show a keybinding hint (e.g. 'q quit'); got:\n{status_row:?}"
        );
    }

    #[test]
    fn draw_reader_status_line_uses_theme_colors() {
        // Regression for the theme-wiring fix: draw_statusline used to hardcode
        // Color::Black/White regardless of App.theme. Assert the rendered status
        // cell's style actually matches app.theme, not a fixed pair of colors.
        let src = "# Hello World\n\nSome text.";
        let width: u16 = 40;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_app(src, width, height);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let style = buf.cell((0, height - 1)).unwrap().style();
        assert_eq!(
            style.fg,
            Some(app.theme.status_fg),
            "status line fg must come from app.theme.status_fg"
        );
        assert_eq!(
            style.bg,
            Some(app.theme.status_bg),
            "status line bg must come from app.theme.status_bg"
        );
    }

    // --- Task 5 tests: search prompt and highlighting ---

    #[test]
    fn search_prompt_shows_query_in_status_row() {
        let src = "# Hello World\n\nSome text.";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_app(src, width, height);
        // Enter search input mode with a query.
        app.start_search();
        app.push_search_char('h');
        app.push_search_char('e');
        app.push_search_char('l');

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        // The status row is the last row (height - 1).
        let status_row = height - 1;
        let buf = terminal.backend().buffer().clone();
        let row_text: String = (0..width)
            .map(|x| buf.cell((x, status_row)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert!(
            row_text.contains("hel"),
            "status row must show the search query 'hel'; got: {row_text:?}"
        );
        // The prompt should start with `/`.
        assert!(
            row_text.trim_start().starts_with('/'),
            "status row must start with '/' in search mode; got: {row_text:?}"
        );
    }

    #[test]
    fn search_match_highlight_differs_from_unhighlighted() {
        let src = "hello world\n\nanother line without the word";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // Draw without search.
        let mut app_plain = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_plain);
            })
            .unwrap();
        let buf_plain = terminal.backend().buffer().clone();

        // Draw with search committed on "hello".
        let mut app_search = make_app(src, width, height);
        app_search.start_search();
        app_search.push_search_char('h');
        app_search.push_search_char('e');
        app_search.push_search_char('l');
        app_search.push_search_char('l');
        app_search.push_search_char('o');
        app_search.commit_search();

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_search);
            })
            .unwrap();
        let buf_search = terminal.backend().buffer().clone();

        // Row 0 has "hello world" — some cells in that row must differ.
        let any_diff = (0..width).any(|x| {
            buf_plain
                .cell((x, 0))
                .zip(buf_search.cell((x, 0)))
                .map(|(p, s)| p.style() != s.style())
                .unwrap_or(false)
        });
        assert!(
            any_diff,
            "at least one cell in the matched row must have a different style when search is active"
        );
    }

    // --- Task 3 (Block D) tests: hover highlight and toggled checkbox rendering ---

    #[test]
    fn hovered_link_row_differs_from_unhovered() {
        let src = "[click me](other.md)\n\nSome other text.";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // Draw without hover.
        let mut app_plain = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_plain);
            })
            .unwrap();
        let buf_plain = terminal.backend().buffer().clone();

        // Draw with hover on the first link.
        let mut app_hovered = make_app(src, width, height);
        assert!(
            !app_hovered.link_map.links.is_empty(),
            "precondition: link exists"
        );
        app_hovered.hovered_link = Some(0);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_hovered);
            })
            .unwrap();
        let buf_hovered = terminal.backend().buffer().clone();

        let link = &app_hovered.link_map.links[0];
        let body_row = link.line as u16;
        let col_start = link.col_start as u16;
        let col_end = link.col_end as u16;

        let any_diff = (col_start..col_end).any(|x| {
            buf_plain
                .cell((x, body_row))
                .zip(buf_hovered.cell((x, body_row)))
                .map(|(p, h)| p.style() != h.style())
                .unwrap_or(false)
        });
        assert!(
            any_diff,
            "at least one cell in the hovered link span must have a different style"
        );
    }

    #[test]
    fn toggled_checkbox_row_differs_from_untoggled() {
        // Doc with a task-list checkbox.
        let src = "- [ ] First task\n\nSome other text.";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // Draw without toggle.
        let mut app_plain = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_plain);
            })
            .unwrap();
        let buf_plain = terminal.backend().buffer().clone();

        // Only proceed if the engine rendered a checkbox.
        if app_plain.checkbox_map.items.is_empty() {
            // Engine did not produce a checkbox — skip rather than panic.
            return;
        }

        // Draw with the first checkbox toggled.
        let mut app_toggled = make_app(src, width, height);
        app_toggled.toggled_checkboxes.insert(0);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_toggled);
            })
            .unwrap();
        let buf_toggled = terminal.backend().buffer().clone();

        let cb = &app_plain.checkbox_map.items[0];
        let body_row = cb.line as u16;

        // The checkbox row must produce different cell content when toggled.
        let any_diff = (0..width).any(|x| {
            buf_plain
                .cell((x, body_row))
                .zip(buf_toggled.cell((x, body_row)))
                .map(|(p, t)| p.symbol() != t.symbol())
                .unwrap_or(false)
        });
        assert!(
            any_diff,
            "checkbox row must differ (different glyph symbol) after toggling"
        );
    }

    // --- Task 4 (Block D) tests: selection highlight ---

    #[test]
    fn selected_cells_have_different_style_from_unselected() {
        // A plain-text document whose first rendered line contains known text.
        let src = "hello world\n\nanother line";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // Draw without selection.
        let mut app_plain = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_plain);
            })
            .unwrap();
        let buf_plain = terminal.backend().buffer().clone();

        // Draw with a selection covering columns 0..5 on row 0 ("hello").
        let mut app_selected = make_app(src, width, height);
        use crate::selection::Selection;
        app_selected.selection = Some(Selection {
            anchor: (0, 0),
            cursor: (0, 5),
        });
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_selected);
            })
            .unwrap();
        let buf_selected = terminal.backend().buffer().clone();

        // At least one cell in columns 0..5 of body row 0 must have a different style.
        let any_diff = (0_u16..5_u16).any(|x| {
            buf_plain
                .cell((x, 0))
                .zip(buf_selected.cell((x, 0)))
                .map(|(p, s)| p.style() != s.style())
                .unwrap_or(false)
        });
        assert!(
            any_diff,
            "selected cells must have a different style compared to unselected cells"
        );
    }

    #[test]
    fn empty_selection_does_not_change_render() {
        // A zero-length selection (anchor == cursor) must not change the rendered output.
        let src = "hello world\n\nanother line";
        let width: u16 = 80;
        let height: u16 = 10;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app_plain = make_app(src, width, height);
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_plain);
            })
            .unwrap();
        let buf_plain = terminal.backend().buffer().clone();

        // Zero-length selection.
        let mut app_empty_sel = make_app(src, width, height);
        use crate::selection::Selection;
        app_empty_sel.selection = Some(Selection {
            anchor: (0, 3),
            cursor: (0, 3),
        });
        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app_empty_sel);
            })
            .unwrap();
        let buf_empty = terminal.backend().buffer().clone();

        // Body rows must be identical to the plain render.
        for y in 0..height - 1 {
            for x in 0..width {
                let plain_style = buf_plain.cell((x, y)).map(|c| c.style());
                let empty_style = buf_empty.cell((x, y)).map(|c| c.style());
                assert_eq!(
                    plain_style, empty_style,
                    "empty selection must not change cell style at ({x}, {y})"
                );
            }
        }
    }

    // --- Task 3 (Block E) tests: draw_browser ---

    use super::draw_browser;
    use bella_engine::browser::{BrowserEntry, BrowserEntryKind};

    /// Build an `App` in browser mode for a given dir.
    fn make_browser_app(dir: std::path::PathBuf, width: u16, height: u16) -> App {
        App::new_browser(dir, width, height)
    }

    /// Insert a synthetic entry into the browser (bypasses filesystem).
    fn push_entry(app: &mut App, display: &str, kind: BrowserEntryKind) {
        if let Some(b) = app.browser.as_mut() {
            b.entries.push(BrowserEntry {
                path: std::path::PathBuf::from(display),
                display: display.to_string(),
                kind,
                ..Default::default()
            });
        }
    }

    #[test]
    fn draw_browser_shows_entry_display_names() {
        let width: u16 = 80;
        let height: u16 = 20;

        // Use a real temp dir so App::new_browser succeeds.
        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_names");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir.clone(), width, height);
        // Clear auto-populated entries and add controlled ones.
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
        }
        push_entry(&mut app, "docs", BrowserEntryKind::Dir);
        push_entry(&mut app, "README.md", BrowserEntryKind::Markdown);

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let mut all_text = String::new();
        for y in 0..height {
            for x in 0..width {
                all_text.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
            }
        }

        assert!(
            all_text.contains("docs"),
            "draw_browser must render Dir entry 'docs'; got:\n{all_text}"
        );
        assert!(
            all_text.contains("README.md"),
            "draw_browser must render Markdown entry 'README.md'; got:\n{all_text}"
        );
    }

    #[test]
    fn selected_row_has_different_prefix_than_unselected() {
        let width: u16 = 80;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_prefix");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir.clone(), width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
        }
        push_entry(&mut app, "alpha.md", BrowserEntryKind::Markdown);
        push_entry(&mut app, "beta.md", BrowserEntryKind::Markdown);

        // Select the first entry (index 0).
        if let Some(b) = app.browser.as_mut() {
            b.selected = 0;
        }

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();

        // Collect the content of the first two body rows (inside the border: y=1, y=2).
        let row1: String = (0..width)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();
        let row2: String = (0..width)
            .map(|x| buf.cell((x, 2)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        // Row 1 (selected) must contain the selection prefix character.
        // Row 2 (unselected) must not contain it (or differ from row 1 in prefix area).
        assert!(
            row1 != row2,
            "selected row (row 1) must differ from unselected row (row 2);\
             \n  row1={row1:?}\n  row2={row2:?}"
        );
    }

    #[test]
    fn draw_browser_shows_status_line() {
        // Wide enough that a real (possibly long) tmp-dir absolute path plus
        // the position/hint text is never truncated — this test asserts on
        // content, not on truncation behavior.
        let width: u16 = 200;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_status_line");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir.clone(), width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
        }
        push_entry(&mut app, "alpha.md", BrowserEntryKind::Markdown);
        push_entry(&mut app, "beta.md", BrowserEntryKind::Markdown);
        push_entry(&mut app, "gamma.md", BrowserEntryKind::Markdown);
        if let Some(b) = app.browser.as_mut() {
            b.selected = 1;
        }

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();

        // Bottom row (y = height - 1) must be the status line.
        let bottom: String = (0..width)
            .map(|x| buf.cell((x, height - 1)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert!(
            bottom.contains(dir.to_string_lossy().as_ref()),
            "browser status line must show the current directory; got:\n{bottom:?}"
        );
        assert!(
            bottom.contains("2/3"),
            "browser status line must show the selected/total position (2/3); got:\n{bottom:?}"
        );

        // The border must now stop one row above the bottom (status row is
        // outside the bordered box) — the bottom-left corner glyph moves up.
        let old_bottom_left = buf.cell((0, height - 1)).map(|c| c.symbol());
        assert_ne!(
            old_bottom_left,
            Some("└"),
            "bordered box must shrink to make room for the status line, \
             not draw its border through the status row"
        );
    }

    #[test]
    fn draw_browser_status_line_uses_theme_colors() {
        // Regression for the theme-wiring fix: draw_browser_statusline used to
        // hardcode Color::Black/White regardless of App.theme. Assert the
        // rendered status cell's style actually matches app.theme.
        let width: u16 = 80;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_status_line_theme");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir.clone(), width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
        }
        push_entry(&mut app, "alpha.md", BrowserEntryKind::Markdown);

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let style = buf.cell((0, height - 1)).unwrap().style();
        assert_eq!(
            style.fg,
            Some(app.theme.status_fg),
            "browser status line fg must come from app.theme.status_fg"
        );
        assert_eq!(
            style.bg,
            Some(app.theme.status_bg),
            "browser status line bg must come from app.theme.status_bg"
        );
    }

    #[test]
    fn draw_browser_status_line_shows_reveal_hint_and_dropped_count() {
        // The status line must (a) document the reveal key/state, and (b)
        // surface a non-zero dropped-entry count so an incomplete listing is
        // visible to the operator rather than silent.
        let width: u16 = 200;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_status_line_reveal");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir, width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
            b.dropped_entries = 3;
        }

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let bottom: String = (0..width)
            .map(|x| buf.cell((x, height - 1)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert!(
            bottom.contains("r reveal"),
            "browser status line must document the reveal key; got:\n{bottom:?}"
        );
        assert!(
            bottom.contains("off"),
            "browser status line must show reveal is off by default; got:\n{bottom:?}"
        );
        assert!(
            bottom.contains("3 entries dropped"),
            "browser status line must surface a non-zero dropped-entry count; got:\n{bottom:?}"
        );
    }

    #[test]
    fn draw_browser_status_line_hides_dropped_count_when_zero() {
        let width: u16 = 200;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_status_line_no_drop");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir, width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
            b.dropped_entries = 0;
        }

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let bottom: String = (0..width)
            .map(|x| buf.cell((x, height - 1)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();

        assert!(
            !bottom.contains("dropped"),
            "browser status line must not mention drops when the count is zero; got:\n{bottom:?}"
        );
    }

    #[test]
    fn dir_row_style_differs_from_markdown_row_style() {
        // Render a browser with a Dir entry at row 0 and a Markdown entry at row 1
        // (both unselected — index 2 is selected).  Because Dir uses bold+cyan and
        // Markdown uses the default style, their cells must differ in style.
        let width: u16 = 80;
        let height: u16 = 20;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_browser_style");

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = make_browser_app(dir.clone(), width, height);
        if let Some(b) = app.browser.as_mut() {
            b.entries.clear();
        }
        // Entry 0: Dir (unselected)
        push_entry(&mut app, "subdir", BrowserEntryKind::Dir);
        // Entry 1: Markdown (unselected)
        push_entry(&mut app, "notes.md", BrowserEntryKind::Markdown);
        // Entry 2: Markdown (selected) — a third entry keeps 0 and 1 unselected.
        push_entry(&mut app, "other.md", BrowserEntryKind::Markdown);
        if let Some(b) = app.browser.as_mut() {
            b.selected = 2; // neither Dir nor notes.md is selected
        }

        terminal
            .draw(|f| {
                draw_browser(f, f.area(), &mut app);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        // Dir entry is inside the border at y=1; Markdown at y=2.
        // At least one cell in the Dir row must have a style different from the
        // corresponding cell in the Markdown row (bold+cyan vs. default).
        let any_style_diff = (2..width).any(|x| {
            // Skip prefix columns (0..2) to avoid comparing identical spaces.
            buf.cell((x, 1))
                .zip(buf.cell((x, 2)))
                .map(|(d, m)| d.style() != m.style())
                .unwrap_or(false)
        });
        assert!(
            any_style_diff,
            "Dir entry row (y=1) must have a different style than Markdown row (y=2) \
             in the text columns; the Dir should be bold+cyan"
        );
    }

    // --- BE.7.F task 2: `truncate_to_width` unit tests ---

    #[test]
    fn truncate_to_width_returns_unchanged_when_it_fits() {
        assert_eq!(super::truncate_to_width("short", 20), "short");
        assert_eq!(super::truncate_to_width("exact", 5), "exact");
    }

    #[test]
    fn truncate_to_width_appends_ellipsis_when_it_overflows() {
        let out = super::truncate_to_width("a much longer string than fits", 10);
        assert!(out.ends_with('…'), "expected an ellipsis, got {out:?}");
        assert!(
            unicode_width::UnicodeWidthStr::width(out.as_str()) <= 10,
            "truncated output must not exceed the width budget: {out:?}"
        );
    }

    #[test]
    fn truncate_to_width_handles_a_width_narrower_than_the_shortest_content() {
        // max_width smaller than even one character plus the ellipsis.
        let out = super::truncate_to_width("hello", 1);
        assert_eq!(
            out, "…",
            "budget of 1 leaves room only for the ellipsis itself"
        );
    }

    #[test]
    fn truncate_to_width_zero_budget_is_empty_never_a_panic() {
        assert_eq!(super::truncate_to_width("hello", 0), "");
    }

    #[test]
    fn truncate_to_width_cuts_on_character_boundaries_never_a_byte_index() {
        // A run of 2-byte-wide `é` characters (width 1 each): any byte-slice
        // truncation at an arbitrary offset within this string is highly
        // likely to land mid-character and panic. `char_indices`-based
        // truncation must not, regardless of which budget is chosen.
        let text: String = std::iter::repeat_n('é', 40).collect();
        for width in 0..45 {
            let out = super::truncate_to_width(&text, width);
            assert!(
                unicode_width::UnicodeWidthStr::width(out.as_str()) <= width,
                "truncated output at width {width} exceeded budget: {out:?}"
            );
        }
    }

    #[test]
    fn truncate_to_width_em_dash_straddling_the_cut_does_not_panic() {
        // ASCII prefix, then a long run of em dashes (3 bytes, width 1
        // each in UTF-8) — a naive `&text[..budget]` byte slice at almost
        // any budget in range lands inside one of the dashes' multi-byte
        // encoding. `truncate_to_width` must not panic at any width here.
        let text = format!("Title {}", "—".repeat(30));
        for width in 0..40 {
            let out = super::truncate_to_width(&text, width);
            assert!(
                unicode_width::UnicodeWidthStr::width(out.as_str()) <= width,
                "truncated output at width {width} exceeded budget: {out:?}"
            );
        }
    }

    #[test]
    fn format_frontmatter_value_renders_all_three_arms() {
        use bella_engine::FrontmatterValue;

        assert_eq!(
            super::format_frontmatter_value(&FrontmatterValue::Scalar("Plan".to_string())),
            "Plan"
        );
        assert_eq!(
            super::format_frontmatter_value(&FrontmatterValue::List(vec![
                "alpha".to_string(),
                "beta".to_string(),
            ])),
            "alpha, beta"
        );
        assert_eq!(
            super::format_frontmatter_value(&FrontmatterValue::Raw("folded text".to_string())),
            "folded text"
        );
    }

    // --- BE.7.F task 2: metadata pane fixtures through `draw_reader` ---

    /// Fixture with frontmatter whose SOURCE order differs from
    /// alphabetical order (`type`, `keywords`, `description` — alphabetical
    /// would be `description`, `keywords`, `type`), and which exercises all
    /// three `FrontmatterValue` arms: `type` is a `Scalar`, `keywords` is a
    /// `List`, and `description`'s folded `>-` block becomes a `Raw`.
    const METADATA_FULL_FIXTURE: &str = "---\ntype: Plan\nkeywords: [gamma, alpha]\ndescription: >-\n  wraps across\n  lines\n---\n\n# Heading\n\nBody text.\n";

    /// Fixture with a single frontmatter key.
    const METADATA_SINGLE_KEY_FIXTURE: &str = "---\ntype: Note\n---\n\n# Heading\n";

    /// Fixture with no frontmatter at all.
    const METADATA_NONE_FIXTURE: &str = "# Heading\n\nJust a paragraph, no frontmatter fence.";

    fn body_rows(
        buf: &ratatui::buffer::Buffer,
        x_start: u16,
        width: u16,
        height: u16,
    ) -> Vec<String> {
        (0..height)
            .map(|y| {
                (x_start..x_start + width)
                    .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn metadata_pane_renders_full_fixture_keys_in_source_order_all_three_arms() {
        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app(METADATA_FULL_FIXTURE, width, height);
        app.rail_open = true;

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        assert!(app.frontmatter.is_some(), "fixture has a frontmatter fence");
        let entries = &app.frontmatter.as_ref().unwrap().entries;
        assert_eq!(
            entries.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["type", "keywords", "description"],
            "source order must be preserved, not re-sorted alphabetically"
        );

        let buf = terminal.backend().buffer().clone();
        // Rail spans x = 0..RAIL_WIDTH.
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let full = rows.join("\n");

        let type_row = rows
            .iter()
            .position(|r| r.contains("Plan"))
            .expect("Scalar arm (type) must render");
        let keywords_row = rows
            .iter()
            .position(|r| r.contains("gamma") && r.contains("alpha"))
            .expect("List arm (keywords) must render, joined");
        let description_row = rows
            .iter()
            // The rail is narrow enough (RAIL_WIDTH=24) that the full
            // folded text truncates before "across" — just check the
            // Raw value's visible prefix rendered at all.
            .position(|r| r.contains("wraps"))
            .expect("Raw arm (description) must render");

        assert!(
            type_row < keywords_row && keywords_row < description_row,
            "keys must render in SOURCE order (type, keywords, description); rows:\n{full}"
        );
    }

    #[test]
    fn metadata_pane_renders_single_key_fixture() {
        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app(METADATA_SINGLE_KEY_FIXTURE, width, height);
        app.rail_open = true;

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        assert!(
            rows.iter().any(|r| r.contains("Note")),
            "single-key fixture's value must render; rows:\n{}",
            rows.join("\n")
        );
    }

    #[test]
    fn metadata_pane_renders_empty_state_for_document_with_no_frontmatter() {
        // The gate this exists to prove capable of failing: an
        // implementation that `unwraps()` `app.frontmatter` instead of
        // handling `None` panics on this exact fixture. Observed against
        // such an implementation during development (a bare
        // `app.frontmatter.as_ref().unwrap().entries` in place of the
        // `.map(...).unwrap_or(&[])` above) — it panicked with "called
        // `Option::unwrap()` on a `None` value" instead of drawing the
        // empty state; reverted before committing.
        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app(METADATA_NONE_FIXTURE, width, height);
        app.rail_open = true;

        assert!(
            app.frontmatter.is_none(),
            "fixture has no frontmatter fence"
        );

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        assert!(
            rows.iter().any(|r| r.contains("no frontmatter")),
            "a document with no frontmatter must render an explicit empty state, \
             never a panic and never a blank rail; rows:\n{}",
            rows.join("\n")
        );
    }

    #[test]
    fn metadata_pane_truncates_long_value_with_multibyte_char_no_overflow() {
        let width: u16 = 120;
        let height: u16 = 40;
        let src =
            "---\ntitle: Café Café Café Café Café Café Café Café Café Café\n---\n\n# Heading\n";
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app(src, width, height);
        app.rail_open = true;

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let title_row = rows
            .iter()
            .find(|r| r.contains("title:"))
            .expect("truncated title row must still render");
        assert!(
            title_row.contains('…'),
            "an overlong value must be truncated with an ellipsis; row: {title_row:?}"
        );
        // The rail's region width (RAIL_WIDTH columns) must be unchanged
        // and no content may have overflowed into the body: the body's
        // first column (x = RAIL_WIDTH) must not carry the truncated tail.
        let body_start_rows = body_rows(&buf, RAIL_WIDTH, 20, height);
        assert!(
            !body_start_rows.iter().any(|r| r.contains("Café")),
            "truncated rail content must never overflow into the body region"
        );
    }

    // --- BE.7.G task 3: `related:` rows, three display states ---

    /// Task 3 AC: "The three display states are visually distinct in the
    /// rendered buffer, asserted against a golden buffer rather than by
    /// eye." Builds a fixture doc with `related: [resolved-id,
    /// unresolved-id, ambiguous-id]` and a real `DocIndex` (built once,
    /// synchronously, over a temp corpus) that resolves the first to one
    /// path, leaves the second unclaimed, and gives the third two
    /// claimants — then asserts each rendered row carries its OWN distinct
    /// marker glyph (never another state's), per
    /// [`super::metadata_row_display`].
    #[test]
    fn metadata_pane_related_rows_render_resolved_unresolved_and_ambiguous_distinctly() {
        let dir = crate::testsupport::unique_temp_dir("bella_ui_related_states");
        std::fs::write(dir.join("target.md"), "---\ndoc_id: resolved-id\n---\n").unwrap();
        std::fs::write(dir.join("dup-a.md"), "---\ndoc_id: ambiguous-id\n---\n").unwrap();
        std::fs::write(dir.join("dup-b.md"), "---\ndoc_id: ambiguous-id\n---\n").unwrap();

        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let src = "---\nrelated: [resolved-id, unresolved-id, ambiguous-id]\n---\n\n# Heading\n";
        let mut app = make_app(src, width, height);
        app.rail_open = true;
        app.doc_index_state = crate::app::DocIndexState::Ready(bella_engine::build_doc_index(&dir));

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let full = rows.join("\n");

        let resolved_row = rows
            .iter()
            .find(|r| r.contains("resolved-id"))
            .unwrap_or_else(|| panic!("resolved-id row must render; rows:\n{full}"));
        let unresolved_row = rows
            .iter()
            .find(|r| r.contains("unresolved-id"))
            .unwrap_or_else(|| panic!("unresolved-id row must render; rows:\n{full}"));
        let ambiguous_row = rows
            .iter()
            .find(|r| r.contains("ambiguous-id"))
            .unwrap_or_else(|| panic!("ambiguous-id row must render; rows:\n{full}"));

        assert!(
            resolved_row.contains('→'),
            "resolved row must carry the resolved marker, not another state's: {resolved_row:?}"
        );
        assert!(
            !resolved_row.contains('✗') && !resolved_row.contains('≠'),
            "resolved row must not carry another state's marker: {resolved_row:?}"
        );
        assert!(
            unresolved_row.contains('✗'),
            "unresolved row must carry the unresolved marker: {unresolved_row:?}"
        );
        assert!(
            !unresolved_row.contains('→') && !unresolved_row.contains('≠'),
            "unresolved row must not carry another state's marker: {unresolved_row:?}"
        );
        assert!(
            ambiguous_row.contains('≠'),
            "ambiguous row must carry the ambiguous marker: {ambiguous_row:?}"
        );
        assert!(
            !ambiguous_row.contains('→') && !ambiguous_row.contains('✗'),
            "ambiguous row must not carry another state's marker: {ambiguous_row:?}"
        );
        assert_ne!(
            resolved_row, unresolved_row,
            "resolved and unresolved rows must render differently"
        );
        assert_ne!(
            unresolved_row, ambiguous_row,
            "unresolved and ambiguous rows must render differently"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A related row while the index build is in flight (`Building`, the
    /// default until `draw_rail` triggers it) must render its own marker
    /// too, never blank and never one of the resolved/unresolved/ambiguous
    /// markers.
    #[test]
    fn metadata_pane_related_row_renders_building_marker_before_the_index_lands() {
        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let src = "---\nrelated: [some-doc]\n---\n\n# Heading\n";
        let mut app = make_app(src, width, height);
        app.rail_open = true;
        // `doc_index_state` starts `NotBuilt`; `draw_rail` triggers the
        // build this very frame but the background worker cannot have
        // landed a result before `terminal.draw` returns — so the row
        // painted THIS frame must be `Building`, not a guess at the
        // eventual outcome.

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let full = rows.join("\n");
        let row = rows
            .iter()
            .find(|r| r.contains("some-doc"))
            .unwrap_or_else(|| panic!("some-doc row must render; rows:\n{full}"));
        assert!(
            row.contains('…'),
            "an unresolved-yet index must render the Building marker: {row:?}"
        );
    }

    #[test]
    fn metadata_pane_related_row_renders_failed_marker_when_the_index_build_failed() {
        let width: u16 = 120;
        let height: u16 = 40;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let src = "---\nrelated: [some-doc]\n---\n\n# Heading\n";
        let mut app = make_app(src, width, height);
        app.rail_open = true;
        app.doc_index_state = crate::app::DocIndexState::Failed("unreadable root".to_string());

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let full = rows.join("\n");
        let row = rows
            .iter()
            .find(|r| r.contains("some-doc"))
            .unwrap_or_else(|| panic!("some-doc row must render; rows:\n{full}"));
        assert!(
            row.contains('!'),
            "a failed index build must render the Failed marker: {row:?}"
        );
    }

    #[test]
    fn metadata_pane_clears_and_rail_section_resets_on_load_file() {
        let width: u16 = 120;
        let height: u16 = 40;
        let mut app = make_app(METADATA_FULL_FIXTURE, width, height);
        assert!(app.frontmatter.is_some());

        app.rail_section = RailSection::Metadata;
        assert_eq!(app.rail_section, RailSection::Metadata);

        let dir = crate::testsupport::unique_temp_dir("bella_ui_metadata_load_file_reset");
        let path = dir.join("no_fm.md");
        std::fs::write(&path, METADATA_NONE_FIXTURE).unwrap();

        app.load_file(path).unwrap();
        app.block_until_ready();

        assert!(
            app.frontmatter.is_none(),
            "switching to a document with no frontmatter must clear the old metadata"
        );
        assert_eq!(
            app.rail_section,
            RailSection::Contents,
            "rail section focus must reset to Contents on load_file"
        );
        assert_eq!(app.rail_selected, 0);
    }

    // --- BE.7.H task 2 tests: rail_section_heights3 + the Tree section ---

    #[test]
    fn rail_section_heights3_gives_each_content_driven_section_its_wanted_height() {
        // 3 Contents rows implied by leftover space, 2 metadata rows,
        // 1 tree row — plenty of room (30 total) for every section to get
        // its wanted height rather than degrade.
        let (contents, metadata, tree) = super::rail_section_heights3(30, 2, 1);
        assert_eq!(metadata, 4, "2 rows + 2 border rows");
        assert_eq!(tree, 3, "1 row + 2 border rows");
        assert_eq!(contents, 30 - metadata - tree, "Contents takes the rest");
    }

    #[test]
    fn rail_section_heights3_below_three_floors_degrades_to_the_two_way_split() {
        // Below `MIN_SECTION_HEIGHT * 3` (6), Tree drops out entirely and
        // this must match `rail_section_heights`'s own two-way answer —
        // the pre-BE.7.H degrade behaviour is unchanged, not re-derived.
        let (contents2, metadata2) = super::rail_section_heights(5, 2);
        let (contents3, metadata3, tree3) = super::rail_section_heights3(5, 2, 9);
        assert_eq!(contents3, contents2);
        assert_eq!(metadata3, metadata2);
        assert_eq!(tree3, 0);
    }

    #[test]
    fn rail_section_heights3_zero_total_is_all_zero_not_a_panic() {
        assert_eq!(super::rail_section_heights3(0, 5, 5), (0, 0, 0));
    }

    #[test]
    fn rail_section_heights3_sum_never_exceeds_total() {
        for total in 0u16..40 {
            for metadata_rows in [0usize, 1, 3, 50] {
                for tree_rows in [0usize, 1, 3, 50] {
                    let (c, m, t) = super::rail_section_heights3(total, metadata_rows, tree_rows);
                    assert!(
                        c + m + t <= total,
                        "sum ({c}+{m}+{t}) must never exceed total ({total}) at \
                         metadata_rows={metadata_rows}, tree_rows={tree_rows}"
                    );
                }
            }
        }
    }

    #[test]
    fn draw_rail_tree_renders_indentation_and_expansion_markers() {
        let width: u16 = 100;
        let height: u16 = 30;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app("# Hi", width, height);
        app.rail_open = true;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_tree_render");
        std::fs::create_dir_all(dir.join("child")).unwrap();
        std::fs::write(dir.join("top.md"), "# top").unwrap();
        let mut tree = bella_engine::browser::Browser::new(dir.clone());
        let child_idx = tree
            .entries
            .iter()
            .position(|e| e.display == "child")
            .expect("child dir must be listed");
        assert!(tree.expand(child_idx), "precondition: child must expand");
        app.tree = Some(tree);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let full = rows.join("\n");

        assert!(
            rows.iter().any(|r| r.contains('▾') && r.contains("child")),
            "the expanded child dir must render the expanded-dir marker; rows:\n{full}"
        );
        assert!(
            rows.iter().any(|r| r.contains("top.md")),
            "an unexpanded sibling markdown file must still render; rows:\n{full}"
        );
    }

    #[test]
    fn draw_rail_tree_highlights_the_focused_selected_row() {
        let width: u16 = 100;
        let height: u16 = 30;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app("# Hi", width, height);
        app.rail_open = true;
        app.rail_focused = true;
        app.rail_section = RailSection::Tree;

        let dir = crate::testsupport::unique_temp_dir("bella_ui_tree_highlight");
        std::fs::write(dir.join("only.md"), "# only").unwrap();
        let tree = bella_engine::browser::Browser::new(dir);
        // Index of "only.md" specifically — NOT assumed to be 0: a fresh
        // temp dir's parent normally exists, so entry 0 is the synthetic
        // `..` row, not the file this test cares about.
        app.rail_selected = tree
            .entries
            .iter()
            .position(|e| e.display == "only.md")
            .expect("only.md must be listed");
        app.tree = Some(tree);

        terminal
            .draw(|f| {
                draw_reader(f, f.area(), &mut app);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let rows = body_rows(&buf, 0, RAIL_WIDTH, height);
        let row = rows
            .iter()
            .find(|r| r.contains("only.md"))
            .unwrap_or_else(|| panic!("only.md row must render; rows:\n{}", rows.join("\n")));
        let col = row.find("only.md").unwrap() as u16;
        // Find the actual cell in the buffer (row text alone doesn't carry
        // style) to assert the reverse-video highlight `draw_rail_tree`
        // applies to the focused selection, mirroring
        // `draw_rail_contents`/`draw_rail_metadata`'s own highlight tests.
        let y = rows.iter().position(|r| r.contains("only.md")).unwrap() as u16;
        let cell = buf.cell((col, y)).expect("cell must exist");
        assert_eq!(
            cell.bg, app.theme.status_fg,
            "the focused selected tree row must use the same reverse-video \
             highlight as Contents/Metadata"
        );
    }
}
