---
type: ImplementReport
title: "Implementation Report — 0.A-workspace-engine-extraction"
description: Full implementation of the workspace scaffold and bella-engine render/layout crate.
---

# Implementation Report — 0.A-workspace-engine-extraction

**Date:** 2026-06-24
**Plan:** planning/0.A-workspace-engine-extraction/tasks.md
**Scope:** Full spec

## What Was Built or Changed

- `Cargo.toml` (root) — new Cargo workspace with `members = ["crates/*"]` and `exclude = ["reference"]`
- `crates/bella-engine/Cargo.toml` — engine crate manifest; edition 2024; deps: ratatui 0.30, crossterm 0.29, pulldown-cmark 0.13, syntect 5 (default-fancy), unicode-width 0.2, serde 1, dirs 6, toml 1
- `crates/bella-engine/LICENSE` — MIT, dual copyright zemse/hackmd + Brandon Redmond
- `crates/bella-engine/ATTRIBUTION.md` — records derivation from `zemse/hackmd @ 7650cdc`, lists ported files and changes
- `crates/bella-engine/src/palette.rs` — ported from upstream; color-depth detection + xterm-256 downgrade
- `crates/bella-engine/src/syntax.rs` — ported; syntect-based code highlighting
- `crates/bella-engine/src/md_config.rs` — ported; `Config` struct + `load()` from `~/.config/md/config.toml`
- `crates/bella-engine/src/theme.rs` — ported; `Theme` struct with `dark()` / `light()` constructors
- `crates/bella-engine/src/links.rs` — ported; `LinkTarget`, `LinkMap`, `CheckboxMap`, `TableMap`, `TableExpansions`, `slugify`, `resolve`
- `crates/bella-engine/src/markdown.rs` — ported; `render_with_edit`, `Rendered`, `BlockInfo`, `EditCtx`, full layout engine
- `crates/bella-engine/src/geometry.rs` — new; pure functions `body_pos`, `select_word_at`, `word_span_at_col`, `point_in` lifted from upstream `events.rs` with `App` dependencies replaced by explicit parameters
- `crates/bella-engine/src/lib.rs` — public surface: `pub mod` all modules; `pub use` exports per spec
- `crates/bella-engine/tests/render.rs` — integration test calling `render_with_edit` on `"# hi\n\n```rs\nfn main(){}\n```"` and asserting heading BOLD styling + code-block fg styling

All ported files carry a 2-line source header attributing `zemse/hackmd @ 7650cdc` (MIT). File-level `#![allow(...)]` directives suppress upstream-style lints (collapsible_if, too_many_arguments, extend_with_drain, redundant_pattern_matching, redundant_closure, manual_strip, double_ended_iterator_last) without touching the ported logic.

## Files Created or Modified

| File | Action |
|---|---|
| `Cargo.toml` | created |
| `crates/bella-engine/Cargo.toml` | created |
| `crates/bella-engine/LICENSE` | created |
| `crates/bella-engine/ATTRIBUTION.md` | created |
| `crates/bella-engine/src/lib.rs` | created |
| `crates/bella-engine/src/palette.rs` | created |
| `crates/bella-engine/src/syntax.rs` | created |
| `crates/bella-engine/src/md_config.rs` | created |
| `crates/bella-engine/src/theme.rs` | created |
| `crates/bella-engine/src/links.rs` | created |
| `crates/bella-engine/src/markdown.rs` | created |
| `crates/bella-engine/src/geometry.rs` | created |
| `crates/bella-engine/tests/render.rs` | created |

## Validation Output

**Commands run:**
```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

**Results:**
```
$ cargo fmt --check
(no output — clean)

$ cargo clippy --all-targets -- -D warnings
    Checking bella-engine v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.38s

$ cargo test
running 37 tests
test geometry::tests::body_pos_col_before_viewport_returns_none ... ok
test geometry::tests::body_pos_in_body_returns_coord ... ok
test geometry::tests::body_pos_offset_viewport ... ok
test geometry::tests::body_pos_accounts_for_scroll ... ok
test geometry::tests::body_pos_gutter_returns_none ... ok
test geometry::tests::body_pos_out_of_bounds_row_returns_none ... ok
test geometry::tests::select_word_at_outside_viewport_returns_none ... ok
test geometry::tests::select_word_at_whitespace_returns_none ... ok
test geometry::tests::word_span_hash_anchor_kept ... ok
test geometry::tests::select_word_at_finds_second_word ... ok
test geometry::tests::word_span_keeps_internal_punctuation ... ok
test geometry::tests::select_word_at_finds_word ... ok
test geometry::tests::word_span_past_end_returns_none ... ok
test geometry::tests::word_span_simple ... ok
test geometry::tests::word_span_strips_leading_trailing_punctuation ... ok
test geometry::tests::word_span_whitespace_returns_none ... ok
test markdown::tests::link_span_split_across_wrapped_lines ... ok
test markdown::tests::records_document_outline ... ok
test markdown::tests::checkbox_lookup_by_line_col ... ok
test markdown::tests::records_heading_anchors ... ok
test markdown::tests::multiple_checkboxes_indexed_in_order ... ok
test markdown::tests::renders_checked_task_marker ... ok
test markdown::tests::renders_aligned_table ... ok
test markdown::tests::expanding_column_reclaims_natural_width ... ok
test markdown::tests::renders_paragraph_and_link ... ok
test markdown::tests::expanding_cell_wraps_it_across_multiple_lines ... ok
test markdown::tests::expanding_whole_table_shows_all_content ... ok
test markdown::tests::renders_unchecked_task_marker ... ok
test markdown::tests::table_hit_test_classifies_clicks ... ok
test markdown::tests::table_truncates_overflowing_cell_by_default ... ok
test markdown::tests::wrap_ranges_are_contiguous ... ok
test markdown::tests::wraps_long_paragraph ... ok
test palette::tests::mid_gray_uses_grayscale_ramp ... ok
test palette::tests::pure_black_maps_to_cube_black ... ok
test palette::tests::pure_white_maps_to_cube_white ... ok
test palette::tests::saturated_red_picks_cube_red ... ok
test markdown::tests::wrap_never_exceeds_width ... ok
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

Running tests/render.rs
running 1 test
test render_heading_and_code_block ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

$ cargo build --release
   Compiling bella-engine v0.1.0 (...)
    Finished `release` profile [optimized] target(s) in 11.30s
```

Status: PASSED

## Decisions and Trade-offs

- **`#![allow]` on ported files:** The upstream code predates the clippy lints now enabled (`collapsible_if`, `too_many_arguments`, etc.). Rather than refactoring the ported logic (which would diverge from upstream and complicate future merges), file-level `#![allow]` directives were added. The new `geometry.rs` code is written clean with no allows needed.
- **`md_config` deps included:** `dirs` and `toml` are needed by `md_config.rs` for the `load()` function. The `theme::resolve` function uses `Config`. Both crates are small and commonly available; keeping them avoids forking the theme API.
- **`ratatui-image` not included:** The engine does not directly depend on `ratatui-image` — the `Rendered::images` field is `Vec<ImageRef>` where `ImageRef` just wraps a `PathBuf`. Actual image rendering lives in `ui.rs` (not ported). The spec note "keep the `images` feature on" means the `images` field in `Rendered` is preserved (it is), not that `ratatui-image` must be a dep.
- **`select_word_at` no side-effects:** Clipboard write, status update, and macOS `dict` lookup all omitted per spec; `select_word_at` returns `Option<(String, usize, usize)>` only.

## Follow-up Work

- Block B: the `bella` (app) crate — TUI entrypoint, viewport, event loop.
- Block D (mouse): clipboard write + status update + `arboard` integration for `select_word_at`.

## git diff --stat

```
 Cargo.lock                                       |  215 +++
 Cargo.toml                                       |    4 +
 crates/bella-engine/ATTRIBUTION.md              |   27 +
 crates/bella-engine/Cargo.toml                  |   16 +
 crates/bella-engine/LICENSE                     |   21 +
 crates/bella-engine/src/geometry.rs             |  307 ++++
 crates/bella-engine/src/lib.rs                  |   17 +
 crates/bella-engine/src/links.rs                |  278 +++
 crates/bella-engine/src/markdown.rs             | 2557 +++++++++++++++++
 crates/bella-engine/src/md_config.rs            |   21 +
 crates/bella-engine/src/palette.rs              |  152 ++
 crates/bella-engine/src/syntax.rs               |   74 +
 crates/bella-engine/src/theme.rs                |  149 ++
 crates/bella-engine/tests/render.rs             |   61 +
```
