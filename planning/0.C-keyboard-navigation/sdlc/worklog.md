# Worklog — 0.C-keyboard-navigation

## Task 1 — PASSED (1 attempt)
What: App now retains link_map + headings from each render, threads real base_dir from file.parent(), and carries focused_link/SearchState scaffolding fields defaulted to None
Decisions: Added #[allow(dead_code)] to SearchState struct and impl block — it is intentional scaffolding for Tasks 3-5 and would otherwise fail clippy -D warnings; Extracted render_metadata() helper that returns (lines, link_map, headings) triple to keep App::new and App::render symmetric
Validated: gating checks (fast tripwire)

## Task 2 — PASSED (1 attempt)
What: Added self-contained back/forward history stack in history.rs with browser semantics (push-after-back truncates forward tail), 10 unit tests, registered as pub mod history in main.rs
Decisions: Made mod history pub so Task 6 (App integration) can use the HistoryEntry type from tests via the bella:: path shown in the doc-comment example
Validated: gating checks (fast tripwire)
