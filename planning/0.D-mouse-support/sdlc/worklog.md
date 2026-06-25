# Worklog — 0.D-mouse-support

## Task 1 — PASSED (1 attempt)
What: Add selection.rs with Selection type, extract_text, copy_to_clipboard (arboard), and 12 unit tests; register mod in main.rs; add arboard = "3" to Cargo.toml
Decisions: Added #![allow(dead_code)] at module level in selection.rs because the items will be consumed by Tasks 2-5 and the dead_code lint fires at -D warnings without it
Validated: gating checks (fast tripwire)

## Task 2 — PASSED (1 attempt)
What: Enable mouse capture in terminal setup/teardown/panic-hook and add map_mouse dispatcher (scroll wheel → ScrollDown/ScrollUp, all other kinds → None) with Event::Mouse arm in run_loop, plus 3 unit tests.
Decisions: map_mouse takes &App (not &mut App) to keep it a pure mapper matching the map_key pattern; later tasks will need &mut App or direct app calls so the signature can evolve; ScrollDown/ScrollUp maps to 3 lines per tick (a common TUI convention for smooth scrolling) rather than 1
Validated: gating checks (fast tripwire)

## Task 3 — PASSED (1 attempt)
What: Add click-to-follow links, hover highlight, and checkbox visual toggle with full coordinate conversion via body_pos
Decisions: Stored body_area Rect in App (set during draw_reader) so map_mouse can call body_pos without needing an extra parameter thread; Used usize::MAX as a sentinel in HoverAt to mean 'pointer left the body area' — avoids a separate ClearHover action variant; Added HoverAt/ClickAt Action variants (rather than handling mouse events inline in run_loop) to keep map_mouse unit-testable and consistent with the existing key→Action→apply pipeline
Validated: gating checks (fast tripwire)

## Task 4 — PASSED (1 attempt)
What: Implements drag-select: DragStart/DragUpdate/DragEnd actions route mouse events, selection model in App tracks anchor/cursor, selection_finish extracts text and copies to clipboard, ui.rs overlays LightBlue highlight on selected rows, plain click (Down+Up no drag) still follows links.
Decisions: Removed the now-dead ClickAt action variant from the Action enum rather than leaving it as dead code; the test that used it was updated to use DragStart+DragEnd (plain click) instead.; Selection is NOT created on DragStart — only on the first DragUpdate event. This cleanly separates plain clicks (no drag) from selections so click-to-follow links works without conflict.; Used usize::MAX as a sentinel in DragEnd content_row to mean 'released outside body area' so selection_finish still runs but click_at is not triggered.; Selection highlight (LightBlue bg) is applied last in the draw pipeline so it wins over search/hover/focus overlays.
Validated: gating checks (fast tripwire)

## Task 5 — PASSED (1 attempt)
What: Double-click word-select (450 ms window): App.last_click field, double_click_word_select method, DoubleClickAt Action, detection in map_mouse, apply routing — all with unit tests.
Decisions: Stored last_click as (Instant, content_row, col) using content coordinates for position comparison, and passed screen coordinates via DoubleClickAt action to select_word_at — this correctly handles scrolling (same screen spot after scroll is a different word).; last_click is reset to None inside double_click_word_select so a third click always starts a fresh single-click cycle without requiring special handling in apply.; Duration import lives only in events.rs top-level (used in map_mouse); in app.rs tests it is re-imported inside the test module only — no unused-import warnings.
Validated: gating checks (fast tripwire)

## Task 6 — PASSED (1 attempt)
What: Validation task: all four validation commands pass — fmt, clippy, 195 tests (157 bella + 37 bella-engine + 1 integration), release build
Decisions: Task 6 is purely a validation gate with no source changes — nothing to commit; the working tree was already clean from Tasks 1-5
Validated: gating checks (fast tripwire)

## Docs
Patched: /Users/brandon/Dev/agentic-portfolio/bella/trees/0.D-mouse-support-flow/README.md, /Users/brandon/Dev/agentic-portfolio/bella/trees/0.D-mouse-support-flow/planning/status.md

## Wrap-up — PASS
Next: Phase 1, Block E — File browser (directory navigator)

## PR
https://github.com/bredmond1019/bella/pull/2
