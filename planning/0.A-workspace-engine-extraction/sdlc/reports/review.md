---
type: ReviewReport
title: "Review Report — 0.A-workspace-engine-extraction"
description: Review verdict for Phase 0 Block A — workspace scaffold and bella-engine extraction.
---

# Review Report — 0.A-workspace-engine-extraction

**Date:** 2026-06-24
**Spec:** planning/0.A-workspace-engine-extraction/tasks.md
**Scope:** Full spec
**Verdict:** PASS

## Acceptance Criteria Check

| Criterion | Status | Evidence |
|---|---|---|
| `cargo build -p bella-engine` succeeds; root workspace excludes `reference/` | MET | `Cargo.toml` has `members = ["crates/*"]` and `exclude = ["reference"]`; fresh build exit 0 |
| Public surface exports `render_with_edit`, `Rendered`, `LinkMap`/`CheckboxMap`/`TableMap`, `LinkTarget`, `Theme`, `body_pos`, `select_word_at` | MET | `lib.rs:14-17` — all exports present |
| Unit test calls `render_with_edit` on heading+code-block input and asserts non-empty `Rendered.lines` with heading + code-block styling | MET | `tests/render.rs` — `render_heading_and_code_block` passes |
| `body_pos` and `select_word_at` are pure standalone functions matching Task 3 signatures, each with passing unit tests; no clipboard/status/dictionary side-effects | MET | `geometry.rs:26,68` — signatures match; 10 passing unit tests; no App, no I/O; doc comment confirms side-effects deferred |
| Edit-sync types (`row_source`, `EditCtx`, `BlockInfo`, corresponding `Rendered` fields) present but unused — preserved dormant | MET | `markdown.rs:42,54,73,80` — all fields present; no `bella` app crate reads them |
| `crates/bella-engine/LICENSE` and `ATTRIBUTION.md` exist; each ported source file carries 2-line attribution header to `zemse/hackmd @ 7650cdc` | MET | LICENSE and ATTRIBUTION.md present; all ported files carry `// Derived from zemse/hackmd @ 7650cdc (MIT)` header |
| No `bella` (app) crate is created; no cloud code written | MET | `crates/` contains only `bella-engine`; no cloud modules present |
| `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` all clean | MET | All four commands exit 0 in fresh run |

## Fresh Test Results

```
$ cargo fmt --check
(no output — clean)
EXIT: 0

$ cargo clippy --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
EXIT: 0

$ cargo test
running 37 tests
test geometry::tests::body_pos_accounts_for_scroll ... ok
test geometry::tests::body_pos_col_before_viewport_returns_none ... ok
test geometry::tests::body_pos_in_body_returns_coord ... ok
test geometry::tests::body_pos_gutter_returns_none ... ok
test geometry::tests::body_pos_offset_viewport ... ok
test geometry::tests::body_pos_out_of_bounds_row_returns_none ... ok
test geometry::tests::select_word_at_outside_viewport_returns_none ... ok
test geometry::tests::select_word_at_whitespace_returns_none ... ok
test geometry::tests::select_word_at_finds_word ... ok
test geometry::tests::select_word_at_finds_second_word ... ok
test geometry::tests::word_span_hash_anchor_kept ... ok
test geometry::tests::word_span_keeps_internal_punctuation ... ok
test geometry::tests::word_span_past_end_returns_none ... ok
test geometry::tests::word_span_simple ... ok
test geometry::tests::word_span_strips_leading_trailing_punctuation ... ok
test geometry::tests::word_span_whitespace_returns_none ... ok
test markdown::tests::checkbox_lookup_by_line_col ... ok
test markdown::tests::multiple_checkboxes_indexed_in_order ... ok
test markdown::tests::expanding_column_reclaims_natural_width ... ok
test markdown::tests::expanding_whole_table_shows_all_content ... ok
test markdown::tests::expanding_cell_wraps_it_across_multiple_lines ... ok
test markdown::tests::link_span_split_across_wrapped_lines ... ok
test markdown::tests::records_heading_anchors ... ok
test markdown::tests::renders_paragraph_and_link ... ok
test markdown::tests::records_document_outline ... ok
test markdown::tests::renders_checked_task_marker ... ok
test markdown::tests::renders_unchecked_task_marker ... ok
test markdown::tests::table_hit_test_classifies_clicks ... ok
test markdown::tests::renders_aligned_table ... ok
test markdown::tests::table_truncates_overflowing_cell_by_default ... ok
test markdown::tests::wrap_ranges_are_contiguous ... ok
test markdown::tests::wraps_long_paragraph ... ok
test palette::tests::mid_gray_uses_grayscale_ramp ... ok
test palette::tests::pure_black_maps_to_cube_black ... ok
test palette::tests::pure_white_maps_to_cube_white ... ok
test palette::tests::saturated_red_picks_cube_red ... ok
test markdown::tests::wrap_never_exceeds_width ... ok
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Running tests/render.rs
running 1 test
test render_heading_and_code_block ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
EXIT: 0

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.07s
EXIT: 0
```

## Verdict: PASS

All eight acceptance criteria are fully met and all four gating validation checks pass with exit 0. The Cargo workspace is correctly structured with `reference/` excluded. The `bella-engine` crate exports the complete required public surface. The render subgraph (6 ported files) and the new `geometry.rs` pure-functions module are present with correct attribution headers on every ported file. Edit-sync types are preserved dormant. The `geometry.rs` functions carry no App dependency, no I/O, and no side-effects. Tests cover 37 unit tests plus 1 integration test, all passing. No `bella` app crate was created prematurely.

## Issues Found

None.

## Next Steps

Proceed to Block B — the `bella` app crate (TUI entrypoint, viewport, event loop).
