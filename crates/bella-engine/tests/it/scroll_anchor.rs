//! Integration tests for the display-row <-> source-line mapping
//! (`display_row_to_source_line` / `source_line_to_display_row`) that
//! BE.7.D's scroll anchoring is built on.

use bella_engine::links::TableExpansions;
use bella_engine::{
    Theme, display_row_to_source_line, render_with_edit, source_line_to_display_row,
};

/// A document with several distinct blocks so each direction has more than
/// one block to resolve into.
const DOC: &str = "\
# Heading One

First paragraph, short.

## Heading Two

Second paragraph body text that is intentionally long enough to wrap across \
more than one display row once the terminal is narrow, which is exactly the \
case this mapping has to handle without losing track of which source line \
the wrapped row belongs to.

Third paragraph, also short.
";

#[test]
fn display_row_to_source_line_resolves_each_block_start() {
    let theme = Theme::dark();
    let rendered = render_with_edit(DOC, None, 80, &theme, None, &TableExpansions::new());

    for block in &rendered.blocks {
        let line = display_row_to_source_line(&rendered, DOC, block.display_start)
            .expect("non-empty document must resolve");
        let expected = DOC[..block.source_range.start].matches('\n').count();
        assert_eq!(
            line, expected,
            "row {} (block source {:?}) resolved to line {line}, expected {expected}",
            block.display_start, block.source_range
        );
    }
}

#[test]
fn source_line_to_display_row_resolves_each_block_start() {
    let theme = Theme::dark();
    let rendered = render_with_edit(DOC, None, 80, &theme, None, &TableExpansions::new());

    for block in &rendered.blocks {
        let line = DOC[..block.source_range.start].matches('\n').count();
        let row = source_line_to_display_row(&rendered, DOC, line)
            .expect("non-empty document must resolve");
        assert_eq!(
            row, block.display_start,
            "line {line} (block source {:?}) resolved to row {row}, expected {}",
            block.source_range, block.display_start
        );
    }
}

#[test]
fn round_trip_is_stable_at_block_boundaries() {
    let theme = Theme::dark();
    let rendered = render_with_edit(DOC, None, 80, &theme, None, &TableExpansions::new());

    for block in &rendered.blocks {
        let line = display_row_to_source_line(&rendered, DOC, block.display_start).unwrap();
        let row = source_line_to_display_row(&rendered, DOC, line).unwrap();
        // Round-tripping a block's own start row must land back within the
        // block's own display span (accepted resolution: nearest line,
        // +/- one display row — not exact sub-line precision).
        assert!(
            row >= block.display_start && row < block.display_end,
            "round trip for block {:?} left row {row} outside display span {}..{}",
            block.source_range,
            block.display_start,
            block.display_end
        );
    }
}

#[test]
fn a_wrapped_source_line_spanning_several_display_rows_maps_within_its_own_block() {
    // Force a hard wrap: a single long paragraph rendered narrow enough
    // that it occupies multiple display rows, all belonging to the same
    // one-block, one-source-line paragraph.
    let src = "This is one long paragraph line that will definitely wrap across several display rows once rendered at a narrow terminal width, proving the mapping stays correct when one source line becomes many display rows.\n";
    let theme = Theme::dark();
    let rendered = render_with_edit(src, None, 20, &theme, None, &TableExpansions::new());

    assert!(
        rendered.lines.len() > 1,
        "fixture must actually wrap across multiple display rows at width 20, got {} lines",
        rendered.lines.len()
    );
    let block = rendered.blocks.first().expect("one paragraph block");
    assert_eq!(block.display_start, 0);
    assert!(
        block.display_end > 1,
        "the single paragraph block must itself span multiple display rows"
    );

    // Every display row within the block resolves to source line 0 — the
    // whole paragraph is one source line — for both directions.
    for row in block.display_start..block.display_end {
        let line = display_row_to_source_line(&rendered, src, row).unwrap();
        assert_eq!(
            line, 0,
            "row {row} should resolve to the paragraph's only source line"
        );
    }
    let row = source_line_to_display_row(&rendered, src, 0).unwrap();
    assert!(
        row >= block.display_start && row < block.display_end,
        "source line 0 should resolve into the wrapped block's display span {}..{}, got {row}",
        block.display_start,
        block.display_end
    );
}

#[test]
fn empty_document_returns_none_both_directions() {
    let theme = Theme::dark();
    let rendered = render_with_edit("", None, 80, &theme, None, &TableExpansions::new());
    assert!(rendered.blocks.is_empty());
    assert_eq!(display_row_to_source_line(&rendered, "", 0), None);
    assert_eq!(source_line_to_display_row(&rendered, "", 0), None);
}

#[test]
fn out_of_range_row_and_line_clamp_to_document_ends() {
    let theme = Theme::dark();
    let rendered = render_with_edit(DOC, None, 80, &theme, None, &TableExpansions::new());

    // A row far past the last line clamps to the last block's line.
    let last_block = rendered.blocks.last().unwrap();
    let far_row = rendered.lines.len() + 50;
    let line = display_row_to_source_line(&rendered, DOC, far_row).unwrap();
    let last_line = DOC[..last_block.source_range.end.min(DOC.len())]
        .matches('\n')
        .count();
    assert!(
        line <= last_line + 1,
        "row far past document end should clamp near the last source line"
    );

    // A source line far past the last line clamps to a row inside the last
    // block's display span.
    let far_line = last_line + 1000;
    let row = source_line_to_display_row(&rendered, DOC, far_line).unwrap();
    assert!(
        row >= last_block.display_start && row < last_block.display_end,
        "line far past document end should clamp into the last block's display span"
    );
}
