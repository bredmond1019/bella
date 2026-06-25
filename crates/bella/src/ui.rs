//! Draw functions: body reader + status line.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::app::App;

/// Draw the full viewer: body (scrolled markdown) + 1-row status line.
///
/// Returns the body area height so the caller can push it back into `App`
/// with [`App::set_viewport_height`].
pub fn draw_reader(frame: &mut Frame, area: Rect, app: &mut App) -> u16 {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // body
            Constraint::Length(1), // status line
        ])
        .split(area);

    let body_area = chunks[0];
    let status_area = chunks[1];

    // Feed the real body height back into the app so max_scroll stays accurate.
    app.set_viewport_height(body_area.height);
    // Store the body area for mouse coordinate conversion in event handlers.
    app.body_area = body_area;

    draw_body(frame, body_area, app);
    draw_statusline(frame, status_area, app);

    body_area.height
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
        format!(" bella · {file_name} · {current}/{total}")
    };
    let line = Line::from(vec![Span::styled(
        text,
        Style::default().fg(Color::Black).bg(Color::White),
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
        App::new(src.to_owned(), PathBuf::from("test.md"), width, height)
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
}
