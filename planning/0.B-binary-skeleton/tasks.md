---
type: TaskSpec
title: Task Spec — Phase 0, Block B (Binary skeleton renders a file)
description: Decomposed task spec for the bella binary crate that renders a file with scroll, no mouse.
---

# Task Spec — Phase 0, Block B (Binary skeleton renders a file)

**Status:** Not started · **Last run:** never

## Goal
`bella file.md` reads the file, renders it through `bella_engine::render_with_edit`, and draws it to the terminal with scroll (`j/k`, `g/G`) and clean `q` exit — no mouse, no nav.

## Context Pointers
- **Plan:** `planning/master-plan.md` → *Phase 0 → Block B — Binary skeleton renders a file (no mouse)* (the only authoritative section for this spec).
- **Engine surface (frozen, from Block A):** `crates/bella-engine/src/lib.rs` re-exports `render_with_edit`, `Rendered`, `Theme`. Signature (confirmed in `crates/bella-engine/tests/render.rs`):
  `render_with_edit(src: &str, edit: Option<EditCtx>, width: u16, theme: &Theme, ..., &TableExpansions::new()) -> Rendered`, where `Rendered.lines: Vec<ratatui::text::Line<'static>>` and `Theme::dark()`/`Theme::light()` exist. Pass `None` for the edit ctx — edit-sync stays **dormant** (do not activate `EditCtx`/`row_source`).
- **Stack/deps:** workspace pins `ratatui = "0.30"`, `crossterm = "0.29"`, `edition = "2024"` (see `crates/bella-engine/Cargo.toml`). The `bella` crate must match.
- **Standing rules (`CLAUDE.md`):** every task ships tests; OKF frontmatter on markdown; all four gated checks (`planning/harness.json`) stay green.
- **Out of scope (hard boundary, from the block):** no mouse (Block D); no link-following / search / history (Block C); no config or themes (Block F); no directory mode (Block E) — a file path arg is **required**.

## Step-by-Step Tasks

### 1. Crate scaffold + terminal lifecycle (`crates/bella/Cargo.toml`, `crates/bella/src/main.rs`)
- Create `crates/bella/Cargo.toml`: name `bella`, edition `2024`, a `[[bin]]` (or default bin) named `bella`. Deps: `bella-engine = { path = "../bella-engine" }`, `ratatui = "0.30"`, `crossterm = "0.29"`, `clap = { version = "4", features = ["derive"] }`, `anyhow = "1"`. The root workspace `Cargo.toml` already globs `members = ["crates/*"]`, so no workspace edit is needed — confirm the new crate is picked up by `cargo build`.
- `main.rs`: clap `derive` CLI with a single **required** positional `file: PathBuf`. Read the file to a `String` (clear error via `anyhow` if missing/unreadable).
- Terminal lifecycle: enable raw mode + enter alternate screen, build a ratatui `Terminal` over a crossterm backend on stdout; on exit (normal **or** panic/error) restore — disable raw mode, leave alternate screen, show cursor. Use a guard/`Drop` or a panic hook so a render error never leaves the terminal corrupted. **Do not enable mouse capture** (Block D).
- Declare the crate modules upfront so later tasks fill them: `mod app; mod events; mod ui;`. Wire `main` to build the `App` (Task 2) and hand off to the event loop (Task 4); keep `main.rs` itself thin.
- Test: a unit test that constructs the clap command and asserts the `file` arg parses (and that a missing positional is rejected) — terminal I/O itself is not unit-tested.

### 2. Reader / App state + scroll model (`crates/bella/src/app.rs`)
- Define `App` holding: source `String`, the rendered output (call `render_with_edit(&src, None, width, &Theme::dark(), …)`), a `scroll: u16` offset (top display row), the last-known viewport height, the file path (for the statusline), and a `should_quit: bool`.
- Scroll methods, all clamped to `[0, max_scroll]` where `max_scroll = lines.len().saturating_sub(viewport_height)`:
  `scroll_down(n)`, `scroll_up(n)`, `to_top()` (`g`), `to_bottom()` (`G`). `quit()` sets `should_quit`.
- A `render(width)` (or equivalent) that (re)builds `Rendered` for a given content width so a resize re-renders. Keep width derivation simple (use the body area width).
- Tests: unit tests for clamping — scroll past top stays at 0, scroll past bottom stays at `max_scroll`, `to_top`/`to_bottom` land exactly, and `max_scroll` is 0 when content fits the viewport.

### 3. Reader + statusline draw path (`crates/bella/src/ui.rs`)
- `draw_reader(frame, area, app)`: split `area` into a body region and a 1-row statusline (ratatui `Layout`). Render the visible slice of `app.lines` starting at `app.scroll` into the body — clone the `Line` slice into a `Paragraph` (or render line-by-line). Code-block syntax highlighting and heading styling come through unchanged from the engine `Line` styles (do not restyle).
- `draw_statusline(frame, area, app)`: show the file name and scroll position (e.g. `bella · README.md · 12/240`). Keep it a single styled line.
- After computing the body area, push its height back into `App` (so Task 2's clamp uses the real viewport) — or expose the body height to the caller; pick one and keep it consistent with Task 2's `viewport_height`.
- Test: a unit test using ratatui's `TestBackend`/`Buffer` — render a small known document and assert the buffer's first body cell matches the expected first visible line's text, then assert that after `scroll_down` the first body row advances. This proves the scroll offset drives the draw.

### 4. Sync key event loop (`crates/bella/src/events.rs`)
- A synchronous loop: `terminal.draw(|f| ui::draw_reader(f, f.area(), &app))` then `crossterm::event::read()`; map `KeyEvent`s to `App` methods:
  `j` / `Down` → `scroll_down(1)`; `k` / `Up` → `scroll_up(1)`; `g` (or `Home`) → `to_top()`; `G` (or `End`) → `to_bottom()`; `q` (and `Ctrl-C`) → `quit()`. Optionally `Ctrl-d`/`Ctrl-u` or `PageDown`/`PageUp` for half/full-page — viewport-sized scroll is fine but not required. Ignore unmapped keys. Handle `Event::Resize` by re-rendering at the new width.
- Loop exits when `app.should_quit`. No mouse arm (Block D). No async runtime (sync only, per D2).
- Factor the key→action mapping into a pure function (e.g. `fn map_key(key: KeyEvent) -> Option<Action>` or a method that takes a `KeyCode` and mutates a passed `&mut App`) so it is unit-testable without a live terminal.
- Test: unit tests over the pure key-mapping — `j` produces a scroll-down effect, `q` sets quit, an unmapped key is a no-op — asserted against `App` state (no terminal needed).

### 5. Validate
- Run the Validation Commands listed below and confirm all pass.
- Manual smoke (not gated, note result in `## Notes`): `cargo run -p bella -- README.md` renders with highlighted code blocks, `j/k`/`g/G` scroll smoothly, and `q` exits restoring the terminal cleanly.

## Acceptance Criteria
- `cargo run -p bella -- <some.md>` displays the file with syntax-highlighted code blocks and styled headings (styling inherited from the engine `Line`s).
- `j`/`k` scroll one line and `g`/`G` jump to top/bottom, all clamped (no scrolling past either end).
- `q` exits cleanly and the terminal is fully restored (raw mode off, main screen, cursor shown) — also on an error/panic path.
- A file path arg is **required**; running with no arg fails with a clear usage error (directory mode is Block E).
- No mouse capture is enabled and no link/search/history/config code is present (later blocks).
- Unit tests cover scroll clamping, key→action mapping, and a `TestBackend` draw assertion tying scroll offset to rendered output.
- All four gated checks pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.

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
