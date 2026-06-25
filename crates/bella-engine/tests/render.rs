//! Integration test for the bella-engine public surface.
//! Calls `render_with_edit` and verifies heading + code-block styling on the output.

use bella_engine::links::TableExpansions;
use bella_engine::{Theme, render_with_edit};
use ratatui::style::Modifier;

#[test]
fn render_heading_and_code_block() {
    let src = "# hi\n\n```rs\nfn main(){}\n```";
    let theme = Theme::dark();
    let rendered = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    // Must produce output lines.
    assert!(
        !rendered.lines.is_empty(),
        "rendered.lines must be non-empty"
    );

    // Find the heading line: should contain "hi" and carry BOLD modifier
    // (heading_modifier = Modifier::BOLD in dark theme).
    let heading_line = rendered.lines.iter().find(|l| {
        l.spans
            .iter()
            .any(|s| s.content.contains("hi") && s.style.add_modifier.contains(Modifier::BOLD))
    });
    assert!(
        heading_line.is_some(),
        "expected a heading line with BOLD modifier containing 'hi'; lines: {:#?}",
        rendered
            .lines
            .iter()
            .map(|l| l
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>())
            .collect::<Vec<_>>()
    );

    // Find a code-block line: should contain "main" and carry a non-default fg
    // (syntect applies foreground color to every token in the code fence).
    let code_line = rendered.lines.iter().find(|l| {
        l.spans
            .iter()
            .any(|s| s.content.contains("main") && s.style.fg.is_some())
    });
    assert!(
        code_line.is_some(),
        "expected a code-block line with styled 'main' token; lines: {:#?}",
        rendered
            .lines
            .iter()
            .map(|l| l
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>())
            .collect::<Vec<_>>()
    );
}
