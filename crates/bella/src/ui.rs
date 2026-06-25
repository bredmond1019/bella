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

    draw_body(frame, body_area, app);
    draw_statusline(frame, status_area, app);

    body_area.height
}

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    let start = app.scroll as usize;
    let end = (start + area.height as usize).min(app.lines.len());
    let mut visible: Vec<Line<'static>> = app.lines[start..end].to_vec();

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

    let paragraph = Paragraph::new(visible).block(Block::default());
    frame.render_widget(paragraph, area);
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

fn draw_statusline(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(msg) = &app.status_message {
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
}
