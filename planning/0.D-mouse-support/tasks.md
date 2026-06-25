---
type: TaskSpec
title: "Task Spec — Phase 0, Block D (Mouse)"
description: Decomposed task spec for Bella Block D — mouse support (scroll/hover/click/checkbox/drag-select/double-click), the v0.1 deliverable.
---

# Task Spec — Phase 0, Block D (Mouse)

**Status:** Done · **Last run:** 2026-06-25 (PASS, 6/6 tasks)

## Goal
Enable full mouse support — scroll, hover, click-to-follow, checkbox visual toggle, drag-select → clipboard copy, and double-click word-select — making mouse-driven reading work in a real terminal (the v0.1 release).

## Context Pointers
- **Plan:** `planning/master-plan.md` → *Phase 1 → Block D — Mouse* (the only block section that applies). Block D is tagged **= v0.1**.
- **Engine surface consumed** (all exported from Block A, `crates/bella-engine/src/lib.rs`): `body_pos`, `select_word_at` (`geometry.rs`), `LinkMap::at`, `CheckboxMap::at`, `LinkTarget` (`links.rs`). `Rendered` already carries `.checkbox_map` (`markdown.rs:35`) — currently discarded by `App::render_metadata` and must be captured.
- **App crate files:** `crates/bella/src/{main.rs, events.rs, app.rs, ui.rs}` (all from Blocks B/C) plus a new `selection.rs`.
- **Existing patterns to mirror:** `events.rs` already has a pure `map_key`/`map_search_key` → `Action` → `apply(action, &mut app)` pipeline and a synchronous `run_loop` with `Event::Key`/`Event::Resize` arms; the mouse arm slots in beside them. `app.rs` already exposes `scroll_up`/`scroll_down`, `follow_focused`, `focused_link`, and `link_map`. `ui.rs` already overlays focused-link and search highlights via `apply_span_highlight` — selection/checkbox styling reuses that helper.
- **CLAUDE.md standing rules:** every task ships tests (rule 1); OKF frontmatter on markdown (rule 2); engine is the only attributed upstream (rule 5). Keep edit-sync types dormant — do **not** wake `row_source`/`EditCtx`/`BlockInfo`.

## Step-by-Step Tasks

### 1. Selection model + clipboard copy (`selection.rs`)
- **New** `crates/bella/src/selection.rs`: a pure, event-loop-independent selection model.
  - A `Selection` type holding an `anchor` and `cursor` content position, each `(row: usize, col: usize)` in rendered-line space.
  - `normalized()` → returns `(start, end)` ordered top-to-bottom / left-to-right so a drag works in any direction.
  - `extract_text(lines: &[Line], selection)` → the selected substring: full span on single-row selections, and head/middle/tail joined with `\n` across multi-row selections (column-clamped per row using char counts, mirroring `ui::apply_span_highlight`'s char-column math).
  - `copy_to_clipboard(text: &str) -> Result<(), String>` via `arboard::Clipboard` — errors are returned (non-fatal; the caller surfaces them through `status_message`), never panicked.
- **Modify** `crates/bella/Cargo.toml`: add the `arboard` dependency (the one new dep this block introduces).
- Register the module in `crates/bella/src/main.rs` (`mod selection;`) — the only line this task adds to `main.rs`.
- Unit tests: single-row extraction, multi-row extraction (head/mid/tail), reversed-drag normalization, empty selection → empty string. Guard the clipboard test so it tolerates a headless CI environment (assert the call returns without panicking; skip/allow `Err` when no clipboard backend exists).

### 2. Enable mouse capture + scroll-wheel
- **Modify** `crates/bella/src/main.rs`: enable mouse capture in terminal setup (`EnableMouseCapture`) and disable it on teardown **and** inside the panic hook (`DisableMouseCapture`), alongside the existing `EnterAlternateScreen`/raw-mode handling, so the terminal is always restored.
- **Modify** `crates/bella/src/events.rs`: add an `Event::Mouse(mouse)` arm to `run_loop`, and a pure `map_mouse(mouse: MouseEvent, app) -> Action` dispatcher beside `map_key`. Map `MouseEventKind::ScrollUp`/`ScrollDown` to the existing `Action::ScrollUp`/`ScrollDown` (a few lines per wheel tick). Leave button/drag/move kinds returning `Action::None` for now (later tasks fill them in).
- **Modify** `crates/bella/src/app.rs`: no new state required for scroll — reuse `scroll_up`/`scroll_down`. (This task only needs the wheel path.)
- Unit tests (in `events.rs`): `map_mouse` ScrollDown → `Action::ScrollDown`, ScrollUp → `Action::ScrollUp`, and an unmapped kind → `Action::None`.

### 3. Click-to-follow links + hover highlight + checkbox visual toggle
- **Modify** `crates/bella/src/app.rs`:
  - Capture the checkbox map: change `render_metadata` to also return `CheckboxMap` and store it on `App` (a new `checkbox_map` field), wiring it through all three render call sites (`App::new`, `render`, `load_file`).
  - Add a `hovered_link: Option<usize>` field and a `toggled_checkboxes: HashSet<usize>` field (visual-only — index into `checkbox_map.items`; **no source mutation**, edit-sync stays dormant).
  - Add `hover_at(content_row, col)` → sets `hovered_link` via `link_map.at`, and `click_at(content_row, col)` → if a link hit, follow it (reuse `follow_focused` logic by setting `focused_link` then following, recording history exactly as the keyboard `Follow` path does); else if a checkbox hit (`checkbox_map.at`), toggle membership in `toggled_checkboxes`.
- **Modify** `crates/bella/src/events.rs`: in `map_mouse`, handle `MouseEventKind::Moved` (hover) and `MouseEventKind::Down(Left)` (click) by calling `body_pos(...)` to convert screen `(col,row)` → content `(row, local_col)`, then routing to `app.hover_at` / `app.click_at`. Use the body viewport `Rect` the draw path already computes; `line_numbers` is `false` for now.
- **Modify** `crates/bella/src/ui.rs`: render the hover highlight (reuse the focused-link overlay path for `hovered_link`) and render toggled checkboxes by restyling/replacing the `[ ]`↔`[x]` glyph for indices in `toggled_checkboxes` (visual only), via the existing `apply_span_highlight`-style span rewrite.
- Unit tests: `click_at` on a link line follows the file (mirror the `follow_*` tests in `app.rs`); `click_at` on a checkbox toggles `toggled_checkboxes` membership and a second click clears it; `hover_at` sets/clears `hovered_link`; a draw test asserts a toggled checkbox row differs from the untoggled render.

### 4. Drag-select → highlight + clipboard copy
- **Modify** `crates/bella/src/app.rs`: add a `selection: Option<Selection>` field; methods `selection_start(row, col)` (set anchor+cursor), `selection_update(row, col)` (move cursor), and `selection_finish()` → extract text via `selection::extract_text(&self.lines, sel)`, copy via `selection::copy_to_clipboard`, set a `status_message` on copy success/failure, and keep the selection for highlight. A click that starts a new drag clears any prior selection.
- **Modify** `crates/bella/src/events.rs`: in `map_mouse`/the mouse handler, route `Down(Left)` → start drag (distinguish from a plain click: a click with no subsequent drag still follows links — start a pending selection on down, promote to a real selection on the first `Drag`, and on `Up` with no drag fall through to `click_at`), `Drag(Left)` → `selection_update`, `Up(Left)` → `selection_finish`. Convert coordinates with `body_pos` as in Task 3.
- **Modify** `crates/bella/src/ui.rs`: overlay the active selection range with a distinct highlight style (reuse `apply_span_highlight`; selection style must be visually distinct from search/hover/focus styles).
- Unit tests: a down→drag→up sequence builds a `Selection`, `selection_finish` extracts the expected text and attempts the clipboard copy; a down→up with no drag does **not** create a selection (so click-to-follow still wins); a draw test asserts selected rows are styled differently from unselected rows.

### 5. Double-click word-select (450 ms window)
- **Modify** `crates/bella/src/app.rs`: add `last_click: Option<(Instant, usize, usize)>` (time + content row/col). On a left-down, if the previous click was within **450 ms** and at the same position, treat it as a double-click: call `bella_engine::select_word_at(...)` to get the word + its display columns, set `selection` to span that word, and copy it via `selection::copy_to_clipboard`. Otherwise record the click for next time.
- **Modify** `crates/bella/src/events.rs`: feed `Down(Left)` events through the double-click detector before the single-click/drag routing from Tasks 3–4 (double-click takes precedence; a non-double down proceeds as before).
- Unit tests: two `app`-level click calls within the window at the same cell select a word and populate `selection` (inject the timestamps so the test is deterministic — do not sleep); two clicks outside the window do **not**; a double-click on whitespace selects nothing (matches `select_word_at` returning `None`).

### 6. Validate
- Run the Validation Commands listed below and confirm all pass.
- Confirm in a real terminal (`cargo run -p bella -- <file>`) that: scroll wheel scrolls; hovering a link highlights it; clicking a link follows it; clicking a checkbox toggles its glyph; drag-select copies to the system clipboard; double-click selects and copies a word.

## Acceptance Criteria
- Mouse capture is enabled on startup and **always** disabled on normal exit and on panic (terminal left usable).
- Scroll wheel up/down scrolls the document (clamped like `j/k`).
- Hovering the pointer over a link highlights that link; moving off clears it.
- Left-click on a link follows it (local file loads in the reader / URL opens the browser) and records history identically to the keyboard `Enter` path; left-click on a task-list checkbox toggles its rendered `[ ]`↔`[x]` glyph with **no** change to the source buffer.
- Click-drag selects a range that is visibly highlighted and, on release, lands the selected text in the system clipboard.
- Double-click within 450 ms on a word selects that word and copies it; double-click on whitespace selects nothing.
- A plain click (down→up, no drag) still follows links — drag and click do not conflict.
- `selection.rs` is independently unit-tested; no edit-sync types (`row_source`/`EditCtx`/`BlockInfo`) are awakened.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo build --release` all pass.

## Validation Commands
```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Notes
<!-- filled in as work happens -->

## Amendment Log
<!-- Append-only. Pipeline stages append one dated line here when they deviate from the spec. -->
2026-06-25 [task 4] ClickAt Action variant (added in Task 3 for click routing) was removed and replaced with a unified DragStart+DragEnd path for plain clicks; the Task 3 test that used ClickAt was updated to use DragStart+DragEnd instead. Spec did not name the Action variants explicitly, but this refactors the click-routing design adopted one task earlier.
