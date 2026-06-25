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
