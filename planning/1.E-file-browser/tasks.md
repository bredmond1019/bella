---
type: TaskSpec
title: "Task Spec — Phase 1, Block E (File browser)"
description: Decomposed task spec for Bella Block E — a hackmd-style directory navigator (no-arg / dir-arg dispatch, descend/ascend, j/k + mouse, round-trip back to the reader).
---

# Task Spec — Phase 1, Block E (File browser)

**Status:** Done · **Last run:** 2026-06-25

## Goal
`bella` with no arg (or a dir arg) opens a hackmd-style directory navigator that lists subdirectories and `.md`/`.mdx` files, descends/ascends with `Enter`/`..`/Backspace, moves the cursor with `j/k` and the mouse, and round-trips into the reader and back with the cursor preserved.

## Context Pointers
- **Plan:** `planning/master-plan.md` → *Phase 1 → Block E — File browser (directory navigator)* (the only block section that applies). It names the files (new `browser.rs`; modified `main.rs`/`app.rs`/`events.rs`/`ui.rs`), the engine surface (none new — reuse Block D mouse geometry), the new deps (`walkdir`/`ignore`), and the out-of-scope boundary (no md-tui flat fuzzy index; an in-listing `/` filter is optional, not required).
- **Upstream to port (read, do not vendor):** `reference/hackmd/src/tui/app.rs:758` (`Browser`, `BrowserEntry`, `BrowserEntryKind { ParentDir, Dir, Markdown }`, `move_selection` wrap+scroll-clamp pattern at `app.rs:740`) and `reference/hackmd/src/tui/ui.rs:1086` (`draw_browser`: bordered pane, dir title, `▶ ` selection prefix, per-kind entry style, scrollbar). Drop hackmd's cloud/read-state/unread-badge machinery — none of it is in scope. The render/layout engine is the only attributed upstream (CLAUDE.md rule 5); `browser.rs` is original app code, not engine code.
- **Current reader architecture (all from Blocks B–D):**
  - `crates/bella/src/main.rs` — `Cli { file: PathBuf }` (a **required** positional today), terminal setup/teardown (mouse capture already enabled), panic hook, `run()` builds `App::new` and calls `events::run_loop`.
  - `crates/bella/src/app.rs` — `App` holds reader state (`lines`, `link_map`, `scroll`, `body_area`, selection/hover, history, …). No mode concept yet.
  - `crates/bella/src/events.rs` — `Action` enum + pure `map_key`/`map_search_key`/`map_mouse` → `apply(action, &mut app)` → `run_loop` (draws `ui::draw_reader`, blocks on `event::read()`, dispatches `Key`/`Resize`/`Mouse`). Reuse `bella_engine::body_pos` for screen→content mapping exactly as the reader mouse arm does.
  - `crates/bella/src/ui.rs` — `draw_reader` (body + 1-row status line; stores `app.body_area`), `draw_body`, `draw_statusline`, `apply_span_highlight`. Browser rendering slots in beside `draw_reader`.
- **CLAUDE.md standing rules:** every task ships tests (rule 1); OKF frontmatter on markdown files (rule 2); engine is the only attributed upstream (rule 5). Keep edit-sync types dormant — do **not** wake `row_source`/`EditCtx`/`BlockInfo`. Every task must leave `planning/harness.json`'s gated checks green (fmt/clippy/test/build).

## Step-by-Step Tasks

### 1. Browser model + entry listing (`browser.rs`)
- **New** `crates/bella/src/browser.rs` — a pure, event-loop-independent directory model:
  - `BrowserEntryKind` enum `{ ParentDir, Dir, Markdown }` and `BrowserEntry { path: PathBuf, display: String, kind: BrowserEntryKind }`.
  - `Browser { dir: PathBuf, entries: Vec<BrowserEntry>, selected: usize, scroll: u16 }`.
  - `Browser::new(dir: PathBuf) -> Self` — list the **current directory only** (non-recursive), gitignore-aware via the `ignore` walker (`max_depth(1)`), hidden dotfiles skipped. Build entries as: a leading `..` (`ParentDir`, target `dir.parent()`) **only when `dir.parent().is_some()`** (omit at a filesystem root); then subdirectories; then `.md`/`.mdx` files. Non-markdown files are **excluded**. Sort directories and files alphabetically (case-insensitive) within their groups. `selected = 0`, `scroll = 0`.
  - `move_cursor(&mut self, delta: i32, viewport_h: u16)` — wrap selection with `rem_euclid` over `entries.len()` and clamp `scroll` so the selection stays visible (port the `move_selection` math at hackmd `app.rs:740`); no-op on an empty list.
  - `selected_entry(&self) -> Option<&BrowserEntry>` and `descend(&self) -> Option<PathBuf>` (the target dir when the selected entry is a `Dir` or `ParentDir`, else `None`).
  - `ascend_target(&self) -> Option<PathBuf>` (= `dir.parent()`), for Backspace.
- **Modify** `crates/bella/Cargo.toml`: add the `ignore` dependency (gitignore-aware listing; pull in `walkdir` too only if used directly). This is the only block that adds these deps.
- **Modify** `crates/bella/src/main.rs`: add `mod browser;` (the **only** line this task adds to `main.rs`; dispatch wiring is Task 2).
- Unit tests (in `browser.rs`): a temp dir with a `.md`, a `.mdx`, a `.txt`, and a subdir lists exactly the markdown files + the subdir + a `..` (txt hidden); `..` is absent when `new` is called on a path with no parent representation under test (assert the leading `ParentDir` is present for a child dir and the parent target equals `dir.parent()`); `move_cursor` wraps at both ends and clamps scroll; `descend` returns the subdir path for a `Dir` entry and `None` for a `Markdown` entry; a gitignored file is excluded when a `.gitignore` is present.

### 2. App mode integration + CLI dispatch (`app.rs`, `main.rs`)
- **Modify** `crates/bella/src/app.rs`:
  - Add a `Mode` enum `{ Reader, Browser }` (or equivalent) and carry browser state — e.g. `browser: Option<Browser>` plus a `mode` field, or fold `Browser` into the mode variant. Keep the existing reader fields intact.
  - `App::new_browser(dir: PathBuf, width: u16, term_height: u16) -> Self` — construct in browser mode with a `Browser::new(dir)`; reader fields may be lazily initialized/empty until a file is opened.
  - `open_from_browser(&mut self, path: PathBuf) -> Result<(), String>` — record the originating browser (its `dir` + `selected` cursor) so a later "back" restores it, then `load_file(path)` and switch `mode` to `Reader`.
  - `back_to_browser(&mut self)` — if a browser origin was recorded, rebuild the `Browser` at the saved dir, restore the saved `selected`, and switch `mode` to `Browser` (no-op when the reader was opened directly from a file arg, i.e. no origin).
  - `enter_dir(&mut self, dir: PathBuf)` / `ascend(&mut self)` — replace the active `Browser` with one rooted at the new dir (preserving `mode == Browser`); ascend uses the current browser's `ascend_target`.
- **Modify** `crates/bella/src/main.rs`: change `Cli.file` to `Option<PathBuf>` and dispatch in `run()`:
  - no arg → `App::new_browser(current_dir, …)`;
  - a **directory** path → `App::new_browser(that_dir, …)`;
  - a **file** path → the existing reader path (`std::fs::read_to_string` + `App::new`).
  Update the existing `main.rs` CLI tests: `missing_positional_is_rejected` no longer holds (no arg is now valid) — replace it with a test that a missing arg parses to `None`, and keep `file_arg_parses` / `command_compiles`.
- Unit tests (in `app.rs`): `App::new_browser` starts in `Browser` mode with a populated `Browser`; `open_from_browser` switches to `Reader`, loads the file, and records the origin; `back_to_browser` returns to `Browser` at the saved dir+cursor; `back_to_browser` is a no-op when no origin was recorded; `enter_dir`/`ascend` re-root the browser.
- **Depends on:** Task 1 (consumes `Browser`).

### 3. Browser rendering (`draw_browser` in `ui.rs`)
- **Modify** `crates/bella/src/ui.rs`:
  - Add `draw_browser(frame, area, app)` — a bordered full-screen pane titled with the current `dir` (mirror hackmd `ui.rs:1086`): render each entry as a row with a `▶ ` prefix on the selected row (blank prefix otherwise), styling `Dir`/`ParentDir`/`Markdown` distinctly (e.g. bold/colored dirs vs. plain files), honoring `browser.scroll` as the list offset. A scrollbar is optional. Store the list's inner `Rect` on `App` (a `browser_area` field, set each draw) so Task 4's mouse handlers can map clicks to rows — analogous to how `draw_reader` stores `body_area`.
  - Keep `draw_reader` unchanged; both draw paths coexist.
- Unit tests (in `ui.rs`, `TestBackend`): rendering a browser with ≥2 entries shows the entry display names; the selected row differs from an unselected row (prefix/style); a `Dir` row renders distinctly from a `Markdown` row.
- **Depends on:** Task 2 (reads `App` browser state).

### 4. Browser key + mouse handlers + loop dispatch (`events.rs`)
- **Modify** `crates/bella/src/events.rs`:
  - Add browser `Action` variants (e.g. `BrowserUp`, `BrowserDown`, `BrowserDescend`, `BrowserAscend`, `BrowserClickAt { row, col }`, `BrowserScroll(i32)`) and a pure `map_browser_key(key) -> Action`: `j`/`Down` → down, `k`/`Up` → up, `Enter` → descend-or-open (descend into a `Dir`/`ParentDir`, else `open_from_browser` the selected `Markdown`), `Backspace` → ascend, `q`/`Ctrl-C` → quit. Add reader-mode `Backspace` (or `[`) → `back_to_browser` so opening a file and pressing back returns to the browser (Task 2 stored the origin).
  - Extend `map_mouse` (or add `map_browser_mouse`) for browser mode: scroll wheel scrolls the listing; `Down(Left)`/`Up(Left)` on a row selects it and a click on the selected entry (or a single click) descends/opens it. Reuse the stored `browser_area` and the same row arithmetic the reader uses with `body_pos` — clicks map to `scroll + (mouse.row - area.y)`.
  - `apply` handles the new actions by calling the Task 2 `App` methods (`move_cursor`, `enter_dir`/`open_from_browser`/`ascend`, `back_to_browser`).
  - `run_loop` dispatches by `app.mode`: draw `ui::draw_browser` vs `ui::draw_reader`, and route keys/mouse through the browser mappers when in `Browser` mode (mirror the existing `in_search_input` branch).
- Unit tests (in `events.rs`): `map_browser_key` mappings (`j`/`k`/`Enter`/`Backspace`/`q`); `apply(BrowserDown)` advances the cursor; `apply(BrowserDescend)` on a `Dir` re-roots the browser and on a `Markdown` switches to `Reader`; `apply(BrowserAscend)` re-roots to the parent; a reader-mode back action after `open_from_browser` returns to `Browser` at the saved cursor; a browser click selects the clicked row.
- **Depends on:** Task 2 (App methods) and Task 3 (`draw_browser` must exist for `run_loop` dispatch to compile).

### 5. Validate
- Run the Validation Commands listed below and confirm all pass.
- Confirm in a real terminal that the round-trip works end-to-end: `cargo run -p bella` (no arg) and `cargo run -p bella -- <dir>` open the browser; `j/k` and the mouse move the cursor; `Enter`/click descends into a folder and opens a `.md`/`.mdx`; `..`/Backspace ascends; opening a file then pressing back returns to the browser at the same cursor; non-markdown files are hidden and directories are shown.

## Acceptance Criteria
- `bella` with **no argument** opens the browser at the current directory; `bella <dir>` opens it at that directory; `bella <file>` still opens the reader (Block B–D behavior unchanged).
- The listing shows subdirectories and `.md`/`.mdx` files only — non-markdown files are hidden; a `..` parent entry is present except at a filesystem root; the listing is gitignore-aware.
- `j`/`k` (and `↑`/`↓`) move the cursor with wrap-around; the viewport scrolls to keep the selection visible; mouse scroll scrolls the listing.
- `Enter` (or click) on a directory or `..` descends/ascends into it and rebuilds the listing; `Enter`/click on a `.md`/`.mdx` opens it in the reader; `..`/Backspace ascends and Backspace matches `..`.
- After opening a file from the browser, a back action returns to the browser **at the same cursor position** (round-trip preserved); a file opened directly via a file arg has no browser to return to (back is a no-op).
- `browser.rs` is independently unit-tested; no edit-sync types (`row_source`/`EditCtx`/`BlockInfo`) are awakened; no cloud/read-state machinery is ported.
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
_No amendments yet._
