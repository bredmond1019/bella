---
type: TaskSpec
title: "Task Spec — Phase 0, Block C: Keyboard navigation"
description: Decomposed task spec for Bella Block C — link focus/follow, in-document search, and back/forward history.
---

# Task Spec — Phase 0, Block C: Keyboard navigation

**Status:** Not started · **Last run:** never

## Goal
Make the reader navigable: link focus (`Tab`/`Shift-Tab`) + follow (`Enter`), `/` in-document search with `n/N` cycling, and a back/forward history stack.

## Context Pointers
- **Plan section:** `planning/master-plan.md` → *Phase 1 → Block C — Keyboard navigation* (the
  only block section that governs this spec). Note: the table lists Block C under Phase 1, but
  the directory slug carries the `0.C` prefix from the working sequence — that is cosmetic; the
  block contract is the Block C section.
- **Engine surface (already exported — no new engine code):**
  - `bella_engine::links::{LinkMap, LinkSpan, LinkTarget}` (`crates/bella-engine/src/links.rs`).
    `LinkSpan` carries `line`/`col_start`/`col_end`/`target`; `LinkTarget` is
    `Url | LocalFile(PathBuf) | Anchor(String) | FileAnchor(PathBuf, String)`.
  - `bella_engine::markdown::Rendered` (`crates/bella-engine/src/markdown.rs`) — fields
    `lines`, `link_map`, `headings` (`Vec<HeadingInfo{level,text,line}>`); `link_map.anchors`
    maps slug → line index for anchor follow.
  - `bella_engine::links::resolve(dest, base_dir)` already maps a raw destination string to a
    `LinkTarget` — but the engine produces `LinkSpan.target` during render, so the app consumes
    `link_map.links` directly; `base_dir` matters because relative `LocalFile` paths are resolved
    against the open file's parent directory.
- **Repo files (current state — read before editing):**
  - `crates/bella/src/app.rs` — `App` currently keeps only `lines`/`scroll`/`viewport_height`/
    `file`/`should_quit`, **discards `link_map`/`headings`**, and calls `render_with_edit(src,
    None, width, &theme, None, &TableExpansions::new())` with `base_dir = None`. Block C must
    retain the link/heading metadata and pass a real `base_dir`.
  - `crates/bella/src/events.rs` — pure `map_key(key, viewport_height) -> Action` + `apply` +
    `run_loop`. New keys extend this.
  - `crates/bella/src/ui.rs` — `draw_reader` → `draw_body` + `draw_statusline`. New highlights
    and the search prompt extend this.
- **Standing rules (`CLAUDE.md`):** every task ships tests; OKF frontmatter on markdown; all logic
  stays in the `bella` crate calling `bella-engine` API only (D2 boundary); edit-sync types stay
  dormant (do not touch `row_source`/`EditCtx`/`BlockInfo`).
- **Out of scope (hard boundary from the block):** mouse link-following and selection (Block D);
  cross-file / project-wide search; fuzzy file open (Block E). No `notify`/watch (Block F).

## Step-by-Step Tasks

### 1. Retain link/heading metadata + real base_dir in `App`
- **Owns:** `crates/bella/src/app.rs`.
- Change `render_lines` (or add a sibling) so `App` keeps the full `Rendered` metadata it needs
  rather than discarding it: store `link_map: LinkMap` and `headings: Vec<HeadingInfo>` (clone or
  move out of `Rendered`) alongside the existing `lines`. Keep `lines` as the render output.
- Pass a real `base_dir` to `render_with_edit`: derive it from `self.file.parent()` so relative
  `LinkTarget::LocalFile` paths resolve correctly. Thread this through `App::new` and `render`.
- Add the navigation-state scaffolding fields the later tasks wire into, defaulted to inert:
  `focused_link: Option<usize>` (index into `link_map.links`), a search-state holder
  (e.g. `search: Option<SearchState>` with query + match line list + current index — define the
  struct here), and leave history wiring to Task 6 (do not add the stack field here; Task 6 owns it).
- Re-clamp / reset `focused_link` and `search` on `render()` (line indices change on resize).
- Unit tests: a doc containing a relative link and a URL yields a non-empty `link_map.links` on
  the `App`; `base_dir` equals the file's parent; `focused_link`/`search` default to `None`.

### 2. Back/forward history stack (`history.rs`)
- **Owns:** `crates/bella/src/history.rs` (new file) + register `mod history;` in
  `crates/bella/src/main.rs` (append-only `mod` line).
- Implement a self-contained, app-independent stack: a `History` struct holding visited entries
  (each an owned `{ path: PathBuf, scroll: u16 }`) with `push(entry)`, `back() -> Option<&Entry>`,
  `forward() -> Option<&Entry>`, and `can_back`/`can_forward`. Pushing a new entry after going
  back truncates the forward tail (standard browser semantics).
- No `App`/engine dependency — pure data structure so it is unit-testable in isolation and a
  parallel-safe file.
- Unit tests: push/back/forward round-trips; pushing after `back()` truncates forward history;
  `back()`/`forward()` at the ends return `None`; scroll position is preserved per entry.

### 3. Link focus ring + highlight
- **Owns (shared, serialized after Task 1):** `crates/bella/src/events.rs`,
  `crates/bella/src/app.rs`, `crates/bella/src/ui.rs`.
- `app.rs`: add `focus_next()` / `focus_prev()` that cycle `focused_link` over
  `link_map.links` indices (wrapping), no-op when there are no links; add a helper to scroll the
  focused link's `line` into the viewport so an off-screen focused link becomes visible.
- `events.rs`: extend `Action` + `map_key` with `FocusNext` (`Tab`) / `FocusPrev` (`Shift-Tab`,
  i.e. `KeyCode::BackTab`) and `ClearFocus` (`Esc`); route them through `apply`.
- `ui.rs`: in `draw_body`, when a link is focused and on a visible row, render its span
  (`col_start..col_end` on `line`) with a distinct highlight style (e.g. reversed / bracketed)
  so the focused link is visually obvious.
- Unit tests: `focus_next` from `None` selects the first link and wraps at the end; `focus_prev`
  wraps backward; `map_key(Tab)` → `FocusNext`, `BackTab` → `FocusPrev`, `Esc` → `ClearFocus`;
  a `TestBackend` draw asserts the focused span row differs from the unfocused render.

### 4. Link follow (`Enter`)
- **Owns (shared, serialized after Task 3):** `crates/bella/src/events.rs`,
  `crates/bella/src/app.rs`, `crates/bella/src/Cargo.toml` (add the `open` dependency).
- `app.rs`: add `follow_focused()` that reads the focused `LinkSpan.target` and dispatches:
  - `LinkTarget::LocalFile(path)` → load that file into the reader (read source, re-render at the
    current width, reset scroll, update `self.file`/`base_dir`). Push the *previous* location onto
    history (the wiring to the `History` field lands in Task 6; here expose a hook/return value or
    a `load_file(path)` method Task 6 calls — keep `follow_focused` returning enough for Task 6 to
    record history).
  - `LinkTarget::Url(url)` → open in the system browser via the `open` crate (`open::that(url)`);
    do not change the reader.
  - `LinkTarget::Anchor(slug)` → scroll to `link_map.anchors[slug]` line within the current doc.
  - `LinkTarget::FileAnchor(path, slug)` → load the file (as `LocalFile`) then scroll to the anchor.
  - Missing/unreadable local files: surface a non-fatal status message, do not crash.
- `events.rs`: map `Enter` (`KeyCode::Enter`) → a `Follow` action routed to `follow_focused()`.
- Add `open` to `crates/bella/Cargo.toml` dependencies.
- Unit tests: `follow_focused` on a `LocalFile` target swaps `App::file` and re-renders (assert
  new `lines` reflect the target file's content via a temp file); on an `Anchor` target the scroll
  lands on the anchor's line; a `Url` target leaves `file`/`lines` unchanged. (Do not invoke a real
  browser in tests — guard the `open::that` call behind the target match so the `Url` test asserts
  state is untouched without launching anything.)

### 5. In-document search (`/`, `n`, `N`)
- **Owns (shared, serialized after Task 4):** `crates/bella/src/events.rs`,
  `crates/bella/src/app.rs`, `crates/bella/src/ui.rs`.
- `app.rs`: implement search over rendered text — `start_search()` enters search-input mode,
  `push_search_char`/`pop_search_char` edit the query, `commit_search()` computes the list of
  matching display-line indices (case-insensitive substring over each `Line`'s concatenated
  text), `search_next()`/`search_prev()` advance the current-match index (wrapping) and scroll the
  matched line into view. `cancel_search()` (`Esc`) clears the input and match state.
- `events.rs`: a search-input sub-mode — while active, character keys append to the query, Enter
  commits, `Esc` cancels; outside it, `/` starts search and `n`/`N` cycle matches. Keep `map_key`
  pure where practical (input-mode routing may live in `apply`/`run_loop`, but cover the routing
  with tests).
- `ui.rs`: render a search prompt on the status row (or a dedicated bottom row) showing the live
  query while searching; in `draw_body`, highlight matched substrings on visible lines, with the
  current match styled distinctly from other matches.
- Unit tests: a query matching ≥2 lines populates the match list in document order;
  `search_next`/`search_prev` wrap and update scroll; a non-matching query yields an empty match
  list and a non-crashing state; `Esc` clears search state; a `TestBackend` draw shows the query
  text in the prompt row.

### 6. History navigation wiring (`[` / `]`)
- **Owns (shared, serialized after Tasks 2 + 4):** `crates/bella/src/events.rs`,
  `crates/bella/src/app.rs`.
- `app.rs`: add the `history: History` field (the struct from Task 2) to `App`; on every file
  load (Task 4's `load_file`/`follow_focused` path) push the prior `{file, scroll}` entry. Add
  `go_back()` / `go_forward()` that pop from the stack, load the recorded file, and restore its
  recorded scroll position. Going back/forward must not itself push a new history entry.
- `events.rs`: map `[` → `HistoryBack` and `]` → `HistoryForward` (also accept `Alt-Left`/
  `Alt-Right` if cleanly expressible) routed to `go_back`/`go_forward`; no-op at the ends.
- Unit tests: follow a `LocalFile` link then `go_back()` restores the original file *and* its
  scroll offset; `go_forward()` returns to the followed file; back/forward at the stack ends are
  no-ops; following a new link after `go_back()` truncates forward history (delegated to Task 2's
  `History`, asserted here through `App`).

### 7. Validate
- Run the Validation Commands listed below and confirm all pass.
- Confirm the full Block C acceptance criteria below hold against the integrated tree.

## Acceptance Criteria
- In a multi-file doc set, `Tab`/`Shift-Tab` cycles the rendered links with a visible highlight on
  the focused link; `Esc` clears focus.
- `Enter` on a focused relative link opens that file in the reader (scroll reset, status line shows
  the new file); `Enter` on a URL launches the system browser without changing the reader; `Enter`
  on an anchor scrolls to the heading.
- `/term` opens a search prompt showing the live query; committing jumps to the first match and
  `n`/`N` cycle matches with the viewport scrolling to each; `Esc` dismisses search.
- `[` / `]` (back / forward) restore the prior file **and** its scroll position; back/forward are
  no-ops at the stack ends; following a new link after going back truncates forward history.
- `crates/bella/src/history.rs` exists as a self-contained, unit-tested stack.
- `App` retains `link_map` + `headings` and renders with a `base_dir` derived from the open file's
  parent directory.
- All Validation Commands pass; no mouse, cross-file search, or fuzzy open was added (those are
  Blocks D/E).

## Validation Commands
```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Notes
<filled in as work happens>

## Amendment Log
<!-- Append-only. Pipeline stages append one dated line here when they deviate from the spec. -->
_No amendments yet._
