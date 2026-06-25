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

## Task 5 — PASSED (1 attempt)
What: Implement in-document search (/, n, N, Esc) with query input mode, case-insensitive match highlighting, match cycling with viewport scrolling, and search prompt in the status row
Decisions: Esc in normal mode cancels active search via the existing ClearFocus action (checked in apply), so no new key binding is needed; map_search_key is a separate pure function for search input mode; run_loop dispatches via it when app.search.input_mode is true, keeping map_key unaware of app state; commit_search on a blank query clears search entirely rather than showing zero results; Current match highlighted in Cyan, other matches in Yellow to make the active match visually distinct; Used let-chain (&&) in draw_body to collapse nested if per clippy::collapsible_if requirement
Validated: gating checks (fast tripwire)

## Task 6 — PASSED (1 attempt)
What: Task 6: wired back/forward history navigation into App and the event loop — `history: History` field added, `go_back`/`go_forward` methods, `[`/`]` key bindings, and history push on link follow.
Decisions: History cursor model requires pushing BOTH the previous and new positions on follow (not just the previous): the cursor must sit on the new entry so history.back() returns the prior one.; go_back/go_forward load the file via load_file() then override scroll to the saved value (load_file resets scroll to 0).; Tests that verify scroll restoration require a document long enough that max_scroll >= the target scroll value.
Validated: gating checks (fast tripwire)
