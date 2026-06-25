---
type: TaskSpec
title: "Phase 0, Block A — Workspace + engine extraction"
description: Task spec to create the Bella Cargo workspace and port the cloud-free render engine subgraph into the attributed bella-engine crate.
---

# Task Spec — Phase 0, Block A

**Status:** Done · **Last run:** 2026-06-24

## Goal
Get the hard, reusable render/layout IP compiling in isolation behind a clean public `bella-engine` surface — before any app code exists.

## Context Pointers
- **Plan:** `planning/master-plan.md` → *Phase 0 — Block A — Workspace + engine extraction* (the only block section in scope).
- **Decision:** `planning/decisions/D2-engine-app-crate-split.md` — the two-crate split is the legal + narrative boundary; engine is attributed-derivative, app is original work.
- **Standing rules (`CLAUDE.md`):** every block ships with tests (rule 1); OKF frontmatter on every `.md` (rule 2); attribution is mandatory — `bella-engine` is MIT-derived from `zemse/hackmd @ 7650cdc`, the only authoritative upstream (rule 5).
- **Port source:** `reference/hackmd/src/tui/` (frozen, gitignored, excluded from the workspace). The reusable subgraph is `markdown.rs → {links.rs (std-only), syntax.rs → palette.rs, theme.rs → palette.rs, md_config.rs}`; `markdown.rs` has zero `App`/`events`/`ui` coupling.
- **Grounded signatures (from the reference):**
  - `markdown::render_with_edit(source: &str, base_dir: Option<&Path>, width: u16, theme: &Theme, edit: Option<EditCtx>, tables: &TableExpansions) -> Rendered` (markdown.rs:89).
  - `Rendered` fields include `lines`, `link_map`, `checkbox_map`, `table_map`, `blocks`, `row_source`, `cursor_xy` — keep the edit-sync fields (`blocks`/`row_source`/`cursor_xy`/`BlockInfo`/`EditCtx`) **dormant, not stripped** (markdown.rs:22).
  - `body_pos(app: &App, col, row) -> Option<(usize, u16)>` (events.rs:1986) and `select_word_at(app: &mut App, col, row)` (events.rs:2068) — both take `&App` upstream; lifting them means refactoring the App-dependent reads into explicit parameters so they become pure functions in `geometry.rs`.
- **Validation:** `planning/harness.json` → `validation.checks[]` (fmt / clippy / test / build, all gating).

## Step-by-Step Tasks

### 1. Workspace + engine-crate scaffold
- Create root `Cargo.toml` as a `[workspace]` with `members = ["crates/*"]` and **`exclude = ["reference"]`** (the gitignored `reference/` tree carries its own `Cargo.toml`s and must never be pulled into the workspace).
- Create `crates/bella-engine/Cargo.toml`: edition 2024, deps aligned with bastion/D2 — `ratatui` 0.30, `crossterm` 0.29, `pulldown-cmark`, `syntect`, and `unicode-width = "0.2"` (non-optional — `geometry::word_span_at_col` needs it; upstream has it behind a feature, Bella does not); keep the `images` feature on by default (no engine surgery in v0.1). Confirm exact dep versions against `reference/hackmd/Cargo.toml` while porting.
- Create the legal files: `crates/bella-engine/LICENSE` (MIT + `zemse/hackmd` copyright) and `crates/bella-engine/ATTRIBUTION.md` (records derivation from `zemse/hackmd @ 7650cdc`, per D2).
- Create a stub `crates/bella-engine/src/lib.rs` (empty/minimal) so the crate compiles before the port lands.
- **Files (owned):** `Cargo.toml` (new, root), `crates/bella-engine/Cargo.toml` (new), `crates/bella-engine/LICENSE` (new), `crates/bella-engine/ATTRIBUTION.md` (new), `crates/bella-engine/src/lib.rs` (new stub).

### 2. Port the render/layout module subgraph
- Copy `markdown.rs`, `links.rs`, `syntax.rs`, `theme.rs`, `palette.rs`, `md_config.rs` from `reference/hackmd/src/tui/` into `crates/bella-engine/src/`.
- Flatten import paths: `crate::tui::X → crate::X`. Resolve any other module references against the ported subgraph only — do **not** drag in `app`/`events`/`ui`/`cloud` modules.
- Keep all edit-sync machinery **dormant**: do not delete `row_source`/`EditCtx`/`BlockInfo` or the related `Rendered` fields (Phase 3 needs them).
- Give each ported file a 2-line source header attributing `zemse/hackmd @ 7650cdc` (MIT), per D2's per-file attribution requirement.
- **Files (owned):** `crates/bella-engine/src/{markdown.rs, links.rs, syntax.rs, theme.rs, palette.rs, md_config.rs}` (all new/ported).
- **Depends on:** Task 1 (crate must exist).

### 3. Lift `geometry.rs` as pure functions
- Create `crates/bella-engine/src/geometry.rs` lifting `body_pos` (`reference/hackmd/src/tui/events.rs:1986`) and the word-resolution core of `select_word_at` (`events.rs:2068`) into **standalone pure functions** — no `App`, no I/O, no threads. Also bring across the two pure helpers `select_word_at` depends on: `word_span_at_col` (`events.rs:2455`) and `point_in` (`events.rs:2505`).
- **`body_pos`** — exact target signature (collapse the upstream `View::Reader/Browser/Cloud` scroll match to a single `scroll` param; Bella has no Cloud):
  ```rust
  pub fn body_pos(viewport: Rect, line_numbers: bool, line_count: usize,
                  scroll: usize, col: u16, row: u16) -> Option<(usize, u16)>
  //                                          → (content_row_index, local_col)
  ```
  `line_count` replaces the upstream `rendered.lines.len()` read used only to size the line-number gutter (`format!("{line_count}").len() + 1` when `line_numbers`).
- **`select_word_at`** — lift **only the pure resolution**; the upstream clipboard write, `app.status` update, and the macOS `dict` lookup/popover are **NOT ported** (see Out of scope). Exact target signature:
  ```rust
  pub fn select_word_at(viewport: Rect, line_numbers: bool, scroll: usize,
                        lines: &[Line<'static>], col: u16, row: u16)
      -> Option<(String, usize, usize)>   // → (word, word_start_col, word_width)
  ```
  Body = upstream's geometry (the `point_in` guard, gutter width, `line_idx = scroll + (row - viewport.y)`, build the line's text from its spans), then delegate to `word_span_at_col(&text, local_col)` and return its result. Drop everything after the `copy_to_clipboard`/`app.status`/`dict` lines.
- `word_span_at_col(line: &str, target_col: usize) -> Option<(String, usize, usize)>` and `point_in(rect: Rect, col: u16, row: u16) -> bool` port verbatim — both are already pure.
- Add unit tests in `geometry.rs` for each: `body_pos` (in-body coord → expected `(row, col)`; out-of-bounds / gutter → `None`), `select_word_at` (click on a known word → that word + span; click on whitespace → `None`), and `word_span_at_col` (punctuation-trimming, internal `_-/.#` kept whole).
- **Files (owned):** `crates/bella-engine/src/geometry.rs` (new).
- **Depends on:** Task 2 (uses the ported `Rendered`/`Line` geometry; `unicode-width` from Task 1).

### 4. Public surface in `lib.rs`
- Replace the Task-1 stub with the real public contract: `pub mod` the ported modules + `geometry`, and `pub use` the exports every later `bella`-crate block consumes — `render_with_edit`, `Rendered`, `LinkMap`, `CheckboxMap`, `TableMap`, `LinkTarget`, `Theme`, `body_pos`, `select_word_at`.
- This is the stable public surface; keep names matching the master-plan *Engine surface* line.
- **Files (owned):** `crates/bella-engine/src/lib.rs` (modify the stub from Task 1).
- **Depends on:** Tasks 2 and 3 (re-exports must resolve).

### 5. Engine render unit test
- Add `crates/bella-engine/tests/render.rs`: call `render_with_edit("# hi\n\n```rs\nfn main(){}\n```", None, <width>, &theme, None, &Default::default())` and assert `Rendered.lines` is non-empty and carries heading styling on the `# hi` line and code-block styling on the fenced block.
- **Files (owned):** `crates/bella-engine/tests/render.rs` (new).
- **Depends on:** Task 4 (consumes the public surface).

### 6. Validate
- Run the Validation Commands listed below and confirm all pass.

## Acceptance Criteria
- `cargo build -p bella-engine` succeeds; the root workspace excludes `reference/`.
- The public surface exports `render_with_edit`, `Rendered`, `LinkMap`/`CheckboxMap`/`TableMap`, `LinkTarget`, `Theme`, `body_pos`, `select_word_at`.
- A unit test calls `render_with_edit` on `"# hi\n\n```rs\nfn main(){}\n```"` and asserts non-empty `Rendered.lines` with heading + code-block styling.
- `body_pos` and `select_word_at` are callable as pure standalone functions (no `App`, no I/O), matching the signatures in Task 3, each with its own passing unit test; `select_word_at` returns `(word, col, width)` and performs no clipboard/status/dictionary side-effects (those are deferred to Block D; the macOS `dict` lookup is dropped).
- Edit-sync types (`row_source`, `EditCtx`, `BlockInfo`, and the corresponding `Rendered` fields) are present but unused — preserved dormant, not stripped.
- `crates/bella-engine/LICENSE` + `ATTRIBUTION.md` exist and each ported source file carries a 2-line attribution header to `zemse/hackmd @ 7650cdc`.
- No `bella` (app) crate is created (that is Block B); no cloud code is written.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` all clean.

## Validation Commands
```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Out of Scope
- The `bella` (app) crate — Block B.
- `select_word_at`'s clipboard write + status update — deferred to **Block D (mouse)**, which owns `arboard` and the app status line.
- The macOS dictionary-lookup popover (`dict.rs`, `spawn_lookup`, `LookupState`) — **dropped from Bella entirely**; not ported. `select_word_at`'s `(word, col, width)` return preserves the option to re-add it later.
- Any cloud code (never written); `app`/`events`/`ui`/`cloud` modules are not pulled into the engine.

## Notes
<filled in as work happens>

## Amendment Log
<!-- Append-only. Pipeline stages append one dated line here when they deviate from the spec. -->
_No amendments yet._
