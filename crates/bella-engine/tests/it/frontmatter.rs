//! Integration fixtures for BE.7.A task 3 — the live heading defect this
//! block exists to fix, the two frontmatter shapes okf-core rejects that
//! this corpus contains, and line-index agreement across `lines`,
//! `row_source`, `HeadingInfo.line` and the `LinkMap` on a stripped
//! document.
//!
//! Task 1 and task 2 already cover the parser's four shapes and the
//! byte-offset correctness in unit tests local to `frontmatter.rs` and
//! `markdown.rs`. This file drives the same machinery end to end through
//! `render_with_edit`, the public entry point, over fixed fixtures whose
//! expected values are written out literally — never computed by the code
//! under test.

use bella_engine::links::TableExpansions;
use bella_engine::{Theme, render_with_edit};

/// The live defect this block exists to fix: `pulldown-cmark` reads the
/// frontmatter's closing `---` as a setext H2 underline and injects a
/// bogus level-2 heading whose text is the whole YAML block. `headings[0]`
/// must be the document's real H1, and no heading's text may contain YAML.
#[test]
fn headings_zero_is_the_real_h1_not_the_yaml_block() {
    let src = "---\ntype: Plan\ntitle: Hello\n---\n# Real Heading\n\nbody text\n";
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    let first = r.headings.first().expect("at least one heading");
    assert_eq!(first.level, 1, "the first heading must be the H1");
    assert_eq!(first.text, "Real Heading");

    for h in &r.headings {
        assert!(
            !h.text.contains("type: Plan") && !h.text.contains("title: Hello"),
            "no heading may carry raw frontmatter YAML as its text; got {:?}",
            h.text
        );
    }
}

/// NEGATIVE CONTROL for the fixture above: this test is the one that
/// fails if the frontmatter pre-pass in `render_with_edit` is reverted,
/// because `pulldown-cmark` then sees the `---` fences directly and
/// misreads the closing fence as a setext H2 underline.
///
/// Executed, not assumed: the pre-pass in `render_with_edit` (the three
/// lines computing `frontmatter`/`delta`/`body` from `crate::frontmatter::
/// parse`) was temporarily replaced with `frontmatter = None; delta = 0;
/// body = source` and this exact test run against it. Output observed
/// this session:
///   assertion `left != right` failed: regression: the setext-H2 misparse is back
///     left: 2
///    right: 2
/// (headings_zero_is_the_real_h1_not_the_yaml_block failed the same way:
/// `left: 2, right: 1`.) The pre-pass was then restored and the full
/// `frontmatter` test group re-run green (23/23) before this file was
/// committed. Recorded here so a future regression that reintroduces the
/// bug is caught by this same test, not merely believed fixed.
#[test]
fn setext_h2_regression_is_caught_by_the_same_fixture() {
    let src = "---\ntype: Plan\ntitle: Hello\n---\n# Real Heading\n\nbody text\n";
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    // Without the strip, pulldown-cmark parses the frontmatter block as a
    // setext H2: `type: Plan\ntitle: Hello` becomes the heading text (the
    // `---` fence line is read as the underline), so headings[0].level
    // would be 2 and its text would contain "type: Plan". Both must be
    // false with the strip in place — this is what "observed, not
    // assumed" means for this criterion.
    let first = r.headings.first().expect("at least one heading");
    assert_ne!(
        first.level, 2,
        "regression: the setext-H2 misparse is back"
    );
    assert!(
        !first.text.contains("type: Plan"),
        "regression: the YAML block leaked into a heading's text"
    );
}

/// End-to-end through render for all four supported shapes at once: the
/// document renders, the frontmatter is carried on `Rendered` with the
/// expected entries in source order, and none of the YAML text leaks into
/// the rendered body.
#[test]
fn all_four_shapes_end_to_end_through_render() {
    use bella_engine::FrontmatterValue;

    let src = concat!(
        "---\n",
        "type: Plan\n",
        "title: \"a: b\"\n",
        "related: [x, y]\n",
        "keywords:\n",
        "  - one\n",
        "  - two\n",
        "---\n",
        "# Hello\n",
        "\n",
        "body text\n",
    );
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    let fm = r.frontmatter.as_ref().expect("frontmatter should parse");
    assert_eq!(
        fm.entries,
        vec![
            ("type".to_string(), FrontmatterValue::Scalar("Plan".to_string())),
            (
                "title".to_string(),
                FrontmatterValue::Scalar("a: b".to_string())
            ),
            (
                "related".to_string(),
                FrontmatterValue::List(vec!["x".to_string(), "y".to_string()])
            ),
            (
                "keywords".to_string(),
                FrontmatterValue::List(vec!["one".to_string(), "two".to_string()])
            ),
        ]
    );

    let rendered_text: String = r
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(!rendered_text.contains("type: Plan"));
    assert!(!rendered_text.contains("related:"));
}

/// The two shapes okf-core rejects and this corpus contains, per the block
/// record: a blank line inside the frontmatter block, and a
/// `description: >-` folded block scalar. Both must parse without
/// erroring — this is the whole reason bella has its own reader instead of
/// adopting okf-core (Fork 2).
#[test]
fn blank_line_inside_frontmatter_block_does_not_error() {
    use bella_engine::FrontmatterValue;

    let src = "---\ntype: Plan\n\ntitle: Hello\n---\n# Hello\n\nbody\n";
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    let fm = r
        .frontmatter
        .as_ref()
        .expect("frontmatter should parse despite the blank line");
    assert_eq!(
        fm.entries,
        vec![
            ("type".to_string(), FrontmatterValue::Scalar("Plan".to_string())),
            (
                "title".to_string(),
                FrontmatterValue::Scalar("Hello".to_string())
            ),
        ]
    );
    let first = r.headings.first().expect("at least one heading");
    assert_eq!(first.level, 1);
    assert_eq!(first.text, "Hello");
}

#[test]
fn description_folded_block_scalar_does_not_error() {
    use bella_engine::FrontmatterValue;

    let src = concat!(
        "---\n",
        "type: Plan\n",
        "description: >-\n",
        "  This is a folded\n",
        "  block scalar description.\n",
        "---\n",
        "# Hello\n",
        "\n",
        "body\n",
    );
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    let fm = r
        .frontmatter
        .as_ref()
        .expect("frontmatter should parse despite the >- block scalar");
    let description = fm
        .entries
        .iter()
        .find(|(k, _)| k == "description")
        .map(|(_, v)| v.clone())
        .expect("description key should be present");
    match description {
        FrontmatterValue::Raw(text) => {
            assert!(text.contains("This is a folded"));
            assert!(text.contains("block scalar description."));
        }
        other => panic!("expected description to be Raw, got {other:?}"),
    }

    let first = r.headings.first().expect("at least one heading");
    assert_eq!(first.level, 1);
    assert_eq!(first.text, "Hello");
}

/// Line-index agreement: `lines`, `row_source`, `HeadingInfo.line` and the
/// `LinkMap` must all agree on a stripped document that carries both a
/// link and a heading below the frontmatter. `LinkMap` and `HeadingInfo`
/// both index into `Rendered::lines` in *display*-line space, and that
/// space starts fresh at 0 for the stripped body regardless of how many
/// lines the frontmatter block occupied — proof the strip doesn't leave a
/// phantom offset in the display-line indices even though the *byte*
/// offsets (`row_source`, `blocks[].source_range`) are deliberately kept
/// in original-file space.
#[test]
fn line_indices_agree_across_lines_row_source_headings_and_link_map() {
    let src = "---\ntype: Plan\ntitle: Hello\n---\n# Real Heading\n\n[a link](https://example.com)\n";
    let theme = Theme::dark();
    let r = render_with_edit(src, None, 80, &theme, None, &TableExpansions::new());

    // The heading must land on a real row within `lines`, and that row's
    // rendered content must actually contain the heading text.
    let heading = r.headings.first().expect("expected the H1 heading");
    assert!(
        heading.line < r.lines.len(),
        "heading.line {} must index into lines (len {})",
        heading.line,
        r.lines.len()
    );
    let heading_row_text: String = r.lines[heading.line]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        heading_row_text.contains("Real Heading"),
        "line {} must render the heading text; got {:?}",
        heading.line,
        heading_row_text
    );

    // The link must be recorded in the LinkMap on a row within `lines`,
    // and that row must contain the link text.
    let link = r.link_map.links.first().expect("expected one link");
    assert!(
        link.line < r.lines.len(),
        "link.line {} must index into lines (len {})",
        link.line,
        r.lines.len()
    );
    let link_row_text: String = r.lines[link.line]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        link_row_text.contains("a link"),
        "line {} must render the link text; got {:?}",
        link.line,
        link_row_text
    );

    // The heading must come before the link in display order — the
    // frontmatter strip must not have scrambled ordering between the two
    // index systems (display-line for headings/links, source-byte for
    // row_source/blocks).
    assert!(
        heading.line < link.line,
        "heading (line {}) must render before the link (line {})",
        heading.line,
        link.line
    );

    // `row_source` is populated only for rows in a raw-substituted block
    // (edit mode); outside edit mode every entry is `None`, but its length
    // must still track `lines` 1:1 so index-based lookups never go out of
    // bounds.
    assert_eq!(
        r.row_source.len(),
        r.lines.len(),
        "row_source must have one entry per display line"
    );
}
