//! Draw functions: body reader + status line.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
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
    let visible: Vec<Line<'static>> = app.lines[start..end].to_vec();
    let paragraph = Paragraph::new(visible).block(Block::default());
    frame.render_widget(paragraph, area);
}

fn draw_statusline(frame: &mut Frame, area: Rect, app: &App) {
    let file_name = app.file.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let total = app.lines.len();
    let current = (app.scroll as usize + app.viewport_height as usize).min(total);
    let text = format!(" bella · {file_name} · {current}/{total}");
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
