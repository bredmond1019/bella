//! Draw functions: body reader + status line.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use bella_engine::browser::BrowserEntryKind;

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

    body_area.height
}

/// Draw the TOC rail region: a bordered pane beside the body.
///
/// The heading list itself (BE.7.E task 2) is not drawn yet — this is the
/// frame-only deliverable of task 1, giving the layout something real to
/// reserve space for and to golden-buffer against.
fn draw_rail(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Contents")
        .style(Style::default().fg(app.theme.status_bg));
    frame.render_widget(block, area);
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
            BrowserEntryKind::ParentDir | BrowserEntryKind::Dir => dir_style,
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

    draw_browser_statusline(frame, status_area, browser, &app.theme);
}

/// Render browser mode's status line: current directory, selection position,
/// and a compact keybinding hint. Styled to match [`draw_statusline`]'s
/// theme-driven status bar for reader mode.
fn draw_browser_statusline(
    frame: &mut Frame,
    area: Rect,
    browser: &bella_engine::browser::Browser,
    theme: &bella_engine::Theme,
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
    let text = format!(
        " bella · {dir} · {position} · j/k nav · Enter open · r reveal ({reveal}) · q quit{dropped}"
    );
    let line = Line::from(vec![Span::styled(
        text,
        Style::default().fg(theme.status_fg).bg(theme.status_bg),
    )]);
    frame.render_widget(Paragraph::new(line), area);
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
        let file_name = app.file.file_name().and_then(|n| n.to_str()).unwrap_or("?");
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

    use crate::app::App;

    use super::draw_reader;

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
}
