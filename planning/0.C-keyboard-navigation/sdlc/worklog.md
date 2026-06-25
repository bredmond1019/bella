# Worklog — 0.C-keyboard-navigation

## Task 1 — PASSED (1 attempt)
What: App now retains link_map + headings from each render, threads real base_dir from file.parent(), and carries focused_link/SearchState scaffolding fields defaulted to None
Decisions: Added #[allow(dead_code)] to SearchState struct and impl block — it is intentional scaffolding for Tasks 3-5 and would otherwise fail clippy -D warnings; Extracted render_metadata() helper that returns (lines, link_map, headings) triple to keep App::new and App::render symmetric
Validated: gating checks (fast tripwire)

## Task 2 — PASSED (1 attempt)
What: Added self-contained back/forward history stack in history.rs with browser semantics (push-after-back truncates forward tail), 10 unit tests, registered as pub mod history in main.rs
Decisions: Made mod history pub so Task 6 (App integration) can use the HistoryEntry type from tests via the bella:: path shown in the doc-comment example
Validated: gating checks (fast tripwire)

## Task 3 — PASSED (1 attempt)
What: Tab/Shift-Tab/Esc link focus ring with REVERSED highlight overlay in the body renderer
Decisions: Used `if let ... && let ...` chained pattern (let-chains) in draw_body to satisfy clippy collapsible_if lint; apply_span_highlight splits existing ratatui Span vec at col_start/col_end boundaries so only the link text gets the REVERSED modifier while surrounding text keeps its original style; apply() made pub(crate) so test modules in events.rs can call it directly without re-exporting
Validated: gating checks (fast tripwire)

## Task 4 — PASSED (1 attempt)
What: Task 4: link follow (Enter) — App::load_file + follow_focused dispatch on LocalFile/Url/Anchor/FileAnchor, Action::Follow mapped to Enter, status_message shown in status line for non-fatal file errors.
Decisions: Added `width: u16` field to App so load_file can re-render at the current terminal width without requiring the caller to pass width through.; follow_focused returns Option<(PathBuf, u16)> (prev file+scroll) so Task 6 can record history without having to intercept load_file — the return value is the clean hook the spec requested.; Anchor scroll clamped to max_scroll; test asserts expected = anchor_line.min(max_scroll) rather than the raw anchor line, which is the correct behavior when the anchor is on or near the last line.; open::that result is intentionally ignored (let _ = ...) — no browser in CI is non-fatal.
Validated: gating checks (fast tripwire)
