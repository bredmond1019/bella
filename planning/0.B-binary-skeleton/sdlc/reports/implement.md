---
type: ImplementReport
title: Implementation Report — 0.B-binary-skeleton
description: Report for the bella binary crate skeleton (reader, scroll, key events, statusline).
---

# Implementation Report — 0.B-binary-skeleton

**Date:** 2026-06-25
**Plan:** planning/0.B-binary-skeleton/tasks.md
**Scope:** Full spec

## What Was Built or Changed
- Created `crates/bella/Cargo.toml` — new binary crate with deps: bella-engine, ratatui, crossterm, clap, anyhow
- Created `crates/bella/src/main.rs` — clap CLI (required `file` positional), terminal lifecycle (raw mode, alternate screen, panic hook), thin `run()` dispatcher, unit tests for arg parsing
- Created `crates/bella/src/app.rs` — `App` struct holding source, rendered lines, scroll offset, viewport height, file path, quit flag; scroll methods (`scroll_down`, `scroll_up`, `jump_top`, `jump_bottom`) clamped to `[0, max_scroll]`; `render(width)` re-renders on resize; unit tests for all clamping cases
- Created `crates/bella/src/ui.rs` — `draw_reader(frame, area, app)` splits area into body + 1-row statusline, renders visible line slice into `Paragraph`, shows `bella · filename · current/total`; unit tests using `TestBackend` verify heading text appears and scroll offset shifts rendered output
- Created `crates/bella/src/events.rs` — pure `map_key(KeyEvent) -> Action` mapper (j/k/arrows, g/G/Home/End, PageDown/PageUp, Ctrl-d/u, q/Ctrl-C); synchronous `run_loop` draws then blocks on `event::read()`; resize re-renders at new width; unit tests over key mapping and App state mutations
- Modified `Cargo.lock` — 12 new packages locked (clap 4.6.1, anyhow 1.0.102, and transitive deps)

## Files Created or Modified
| File | Action |
|---|---|
| `crates/bella/Cargo.toml` | created |
| `crates/bella/src/main.rs` | created |
| `crates/bella/src/app.rs` | created |
| `crates/bella/src/ui.rs` | created |
| `crates/bella/src/events.rs` | created |
| `Cargo.lock` | modified (new deps locked) |

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
cargo fmt --check        → (no output, exit 0)
cargo clippy ...         → Finished `dev` profile — 0 warnings, 0 errors
cargo test               → 21 passed; 0 failed (bella) + 37 passed (bella-engine) + 1 passed (integration)
cargo build --release    → Finished `release` profile
```
Status: PASSED

## Decisions and Trade-offs
- Renamed `to_top`/`to_bottom` → `jump_top`/`jump_bottom` to satisfy `clippy::wrong_self_convention` (methods mutating `self` should not use `to_*` naming convention).
- `draw_reader` returns the body height and pushes it back into `App::set_viewport_height` on every draw, ensuring the clamp stays accurate even after a resize.
- Panic hook takes ownership of the previous hook and calls it after restoring the terminal, so the default backtrace still prints.
- `TestBackend` draw test searches all body rows rather than only row 0, because the engine may prepend blank decorative lines before heading text.
- No mouse capture enabled; no async runtime introduced; no `EditCtx` activated — all per the hard scope boundary in the spec.

## Follow-up Work
- Block C: link-following, search, navigation history
- Block D: mouse support
- Block E: directory mode
- Block F: theme/config system

## git diff --stat
```
 Cargo.lock | 119 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 119 insertions(+)
```
(New files are untracked — they appear in `git status` but not in `git diff --stat` until staged.)
