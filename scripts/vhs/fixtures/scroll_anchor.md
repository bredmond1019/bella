# Scroll Anchor Fixture

Three headed blocks with paragraphs long enough to wrap onto several
display lines once the terminal narrows, but fit on one or two at a wide
one — the same shape as the seams spike's measured 11-vs-15-display-line
shift for an 80-vs-30 resize that motivates BE.7.D (see the block record's
"why", and `SCROLL_ANCHOR_FIXTURE` in `crates/bella/src/app.rs`, which this
mirrors). Used by the `*_scroll_anchor_*` scenes in `scripts/vhs/scenes.toml`
to give the reference screenshot set a visual regression surface for scroll
anchoring, alongside the Rust unit/integration tests that exercise the
mechanism itself against the real render worker.

## Heading One

This paragraph is deliberately long so that it wraps onto several display
lines once the terminal narrows, which is exactly the resize behaviour this
block anchors against.

## Heading Two

A second paragraph, also long enough to reflow across multiple rows at a
narrow width, sits between two headings so the fixture has more than one
block to scroll through.

## Heading Three

A third paragraph closes out the document and gives the reader something
real to land on once the viewport has scrolled past the earlier blocks.
