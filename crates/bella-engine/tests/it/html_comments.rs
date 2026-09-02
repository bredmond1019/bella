//! Regression tests locking in that bella-engine's markdown render never emits HTML-comment
//! lines (`<!-- ... -->`) as visible text, so sentinel fences used in status/spec docs never
//! leak into `bastion`'s TUI (see CLAUDE.md rule 6 / D3 cross-repo contract, planning/BE.4.A).

use bella_engine::links::TableExpansions;
use bella_engine::{Rendered, Theme, render_with_edit};

/// Concatenate every rendered line's span content into a single string per line,
/// then join all lines with newlines for easy substring inspection.
fn visible_text(rendered: &Rendered) -> String {
    rendered
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn html_comments_never_appear_in_visible_text() {
    let src = "\
# Heading with comment ## Heading <!-- SENTINEL_HEADING -->

Standalone block comment below.

<!-- SENTINEL_STANDALONE -->

Text before an inline-adjacent comment.
<!-- SENTINEL_INLINE_ADJACENT -->
Text after the inline-adjacent comment.

Multiline comment follows.
<!--
SENTINEL_MULTILINE
spans several lines
-->
Text after multiline comment.

Trailing comment on the same line as text <!-- SENTINEL_TRAILING -->.

## Heading directly adjacent <!-- SENTINEL_HEADING_ADJACENT -->
";

    let theme = Theme::dark();
    let rendered = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    assert!(
        !rendered.lines.is_empty(),
        "rendered.lines must be non-empty"
    );

    let text = visible_text(&rendered);

    assert!(
        !text.contains("<!--"),
        "visible text must never contain the HTML-comment open sentinel `<!--`; got:\n{text}"
    );
    assert!(
        !text.contains("-->"),
        "visible text must never contain the HTML-comment close sentinel `-->`; got:\n{text}"
    );

    for sentinel in [
        "SENTINEL_HEADING",
        "SENTINEL_STANDALONE",
        "SENTINEL_INLINE_ADJACENT",
        "SENTINEL_MULTILINE",
        "SENTINEL_TRAILING",
        "SENTINEL_HEADING_ADJACENT",
    ] {
        assert!(
            !text.contains(sentinel),
            "visible text must never contain comment-body sentinel `{sentinel}`; got:\n{text}"
        );
    }
}

#[test]
fn html_comment_inside_fenced_code_block_is_shown_verbatim() {
    let src = "\
Some text before a code block.

```html
<!-- SENTINEL_IN_CODE_BLOCK -->
```

Some text after the code block.
";

    let theme = Theme::dark();
    let rendered = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    let text = visible_text(&rendered);

    assert!(
        text.contains("<!--"),
        "an HTML comment inside a fenced code block must still be shown verbatim (open marker); got:\n{text}"
    );
    assert!(
        text.contains("SENTINEL_IN_CODE_BLOCK"),
        "an HTML comment inside a fenced code block must still be shown verbatim (body); got:\n{text}"
    );
}
