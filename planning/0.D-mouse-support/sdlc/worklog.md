# Worklog — 0.D-mouse-support

## Task 1 — PASSED (1 attempt)
What: Add selection.rs with Selection type, extract_text, copy_to_clipboard (arboard), and 12 unit tests; register mod in main.rs; add arboard = "3" to Cargo.toml
Decisions: Added #![allow(dead_code)] at module level in selection.rs because the items will be consumed by Tasks 2-5 and the dead_code lint fires at -D warnings without it
Validated: gating checks (fast tripwire)

## Task 2 — PASSED (1 attempt)
What: Enable mouse capture in terminal setup/teardown/panic-hook and add map_mouse dispatcher (scroll wheel → ScrollDown/ScrollUp, all other kinds → None) with Event::Mouse arm in run_loop, plus 3 unit tests.
Decisions: map_mouse takes &App (not &mut App) to keep it a pure mapper matching the map_key pattern; later tasks will need &mut App or direct app calls so the signature can evolve; ScrollDown/ScrollUp maps to 3 lines per tick (a common TUI convention for smooth scrolling) rather than 1
Validated: gating checks (fast tripwire)
