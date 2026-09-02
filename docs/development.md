---
type: Guide
title: Development Guide
description: Prerequisites, build steps, test layers, and the SDLC pipeline for Bella contributors.
doc_id: development
layer: [console]
project: bella
status: active
keywords: [build instructions, cargo, tests, development setup, SDLC pipeline, Rust]
related: [capabilities, bella-docs-index, harness-examples]
---

# Development Guide

**How to build, test and change bella.** If you only want to *use* it, you want
[`capabilities.md`](capabilities.md); this page is for someone editing the code.

## Quickstart

Run these in a shell, from the repo root:

```bash
cargo build                                       # 1. build (debug — fastest iteration)
cargo nextest run --lib --bins --workspace        # 2. fast test pass while iterating
cargo clippy --all-targets -- -D warnings         # 3. lint gate
cargo fmt --check                                 # 4. format gate
cargo run -p bella -- README.md                   # 5. see it working
```

Step 2 needs `cargo-nextest` (`brew install cargo-nextest`). `cargo test` remains the
authoritative full-suite gate — see [Testing](#testing) for when to use which.

## Prerequisites

- Rust stable toolchain — install via [rustup](https://rustup.rs/)
- `cargo` in your `PATH`
- A terminal with mouse support (most modern emulators: iTerm2, WezTerm, kitty, Alacritty, Ghostty)

No other runtime dependencies. All crates are pure Rust; `syntect` bundles its syntax definitions; `arboard` links to the system clipboard provider.

## Build

```bash
# Debug build (fastest iteration)
cargo build

# Release build (optimised — what you'd actually run daily)
cargo build --release

# Run directly from source
cargo run -p bella -- README.md
cargo run -p bella -- .          # open current dir in browser
cargo run -p bella               # same — no arg defaults to CWD
```

## Testing

```bash
# Full suite (authoritative)
cargo test

# Watch mode (requires cargo-watch)
cargo watch -x test

# Single crate
cargo test -p bella-engine
cargo test -p bella

# Single test by name
cargo test -p bella -- test_scroll_clamp
```

### Test Layers

| Layer | Where | What it covers |
|---|---|---|
| Engine unit tests | `crates/bella-engine/src/*.rs` (`#[cfg(test)]` blocks) | Word-wrap, link resolution, slug generation, geometry coordinate math, checkbox detection |
| Engine integration | `crates/bella-engine/tests/it/render.rs` | Full render pipeline: source → `Rendered`; checks line count, link extraction, checkbox spans |
| App unit tests | `crates/bella/src/*.rs` (`#[cfg(test)]` blocks) | Scroll clamping, key mapping, history push/back/forward, selection normalisation, double-click timing, browser cursor wrap, browser entry ordering |
| App draw tests | `crates/bella/src/ui.rs` (`#[cfg(test)]`) | `ratatui::backend::TestBackend` assertions on rendered cell content |
| App draw integration (golden) | `crates/bella/tests/it/golden_draw.rs` | `TestBackend` structural assertions (region widths, x-offsets, pane boundaries, status-row position) for `draw_reader`/`draw_browser` across multiple terminal sizes |
| Layout integration | `crates/bella/tests/it/layout.rs` | TOC rail geometry (rail-on, rail-off, the `RAIL_WIDTH + MIN_BODY_WIDTH` auto-collapse threshold) and the `App.width` single-writer invariant (BE.7.E) |

All mappers (`map_key`, `map_rail_key`, `map_browser_key`, `map_search_key`, `map_mouse`, `map_browser_mouse`) are pure functions with no terminal dependency — they are exercised directly in unit tests without any mocking.

Each crate's integration tests live in one binary, `crates/<crate>/tests/it/main.rs`, with one
`mod <name>;` line per test file — never a second top-level `crates/<crate>/tests/<name>.rs`,
which cargo would link as its own binary and slow every rebuild. `scripts/check_test_layout.sh`
gates this (`test-layout` in `planning/harness.json`); see the note in `CLAUDE.md` for the
measured rebuild-cost rationale.

Any test that needs a scratch directory on disk must call `crate::testsupport::unique_temp_dir`
(`crates/bella/src/testsupport.rs` and `crates/bella-engine/src/testsupport.rs`, both
`#[cfg(test)]`-only) rather than a fixed `std::env::temp_dir().join("bella_...")` path — a fixed
name collides when two runs (e.g. two `--worktree` lanes) share one `/tmp`.

### Visual QA (manual/agent review)

Automated tests assert cell *content*, not appearance — for an actual visual check (spacing,
color, layout), two scripts under `scripts/`:

| Tool | What it does | Use it for |
|---|---|---|
| `scripts/tui_capture.sh <file\|dir> [key ...]` | Drives bella in a detached tmux session (via `bastion new`/`capture`/`kill`) and dumps the rendered screen as text | Fast structural checks — is a section present, did navigation land where expected |
| `scripts/vhs/*.tape` (run with `vhs <tape>`, requires `brew install vhs`) | Scripts a real pty session and renders it to PNG/GIF | Pixel-level review — theme colors, alignment, wrapping. `reference-wide.tape`/`reference-narrow.tape` regenerate the baseline set in `planning/artifacts/screenshots/` (see that directory's `README.md`) |

Beyond these two manual-review tools, bella also gates on two **automated** regression checks —
run `bash scripts/check_scenes.sh` and `bash scripts/check_vhs_fresh.sh` to reproduce what the
harness's `scenes` and `vhs-fresh` checks run (`planning/harness.json`). Both are driven off one
shared scene manifest, `scripts/vhs/scenes.toml`, and both are documented in full — what each
tier catches, why VHS output is checked for freshness/sanity rather than pixel-diffed, and how to
regenerate baselines — in `planning/artifacts/screenshots/README.md`. `scripts/capture_scenes.sh`
regenerates the tier-2 text baselines under `tests/scenes/`.

## Lint / Format

```bash
# Lint gate (CI-equivalent)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check

# Auto-format
cargo fmt
```

Both must pass clean before any PR merges. The SDLC validation suite in `planning/harness.json` runs these as gating checks.

## Running the Viewer

```bash
# Open a file in reader mode
cargo run -p bella -- path/to/file.md

# Open a directory in browser mode
cargo run -p bella -- path/to/dir

# No arg: browser mode in CWD
cargo run -p bella

# Release binary (after cargo build --release)
./target/release/bella README.md
```

## Adding a New Keybinding

A keybinding touches six places. Work them in order — the mapper first, because its unit test is
the fastest way to confirm the key is even reaching you.

1. **`events.rs` mapper** — add a match arm to `map_key` (reader), `map_browser_key` (browser), or `map_search_key` (search). Return an `Action` variant.
2. **`events.rs` Action enum** — if you need a new action, add a variant to `Action`.
3. **`events.rs::apply`** — add a match arm to dispatch the action to an `App` method.
4. **`app.rs`** — add or extend the App method the action calls.
5. **Tests** — add a unit test in the mapper's `#[cfg(test)]` block asserting the key produces the expected `Action`.
6. **Docs** — add a row to the keybinding table in `README.md` (the user-facing list), to
   [`capabilities.md`](capabilities.md) (what it does, one line), and to
   [`features.md`](features.md) (the `Action` → `App` method chain).

## SDLC Pipeline

Structured block work follows: `/generate-tasks → /implement → /test → /review-task → /document → /log-work`.

The pipeline reads its validation commands from `planning/harness.json`. The current profile runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`, `scripts/check_test_layout.sh` (plus its own fixture suite, `scripts/tests/test_check_test_layout.sh`), `scripts/check_scenes.sh`, and `scripts/check_vhs_fresh.sh` (see "Visual QA" above) as gating checks. Do not edit the workflow engine scripts (`.claude/workflows/*.js`) for stack reasons — only `harness.json`.

To start a new block:

```
/generate-tasks <spec-slug>   # decompose the block spec into tasks.md
/sdlc-flow <spec-slug>        # run the full pipeline on a branch (add --worktree to isolate)
```

See `.claude/commands/README.md` for the full command reference.

## Project Layout Quick Reference

```
bella/
├── Cargo.toml                  ← workspace (members: bella-engine, bella)
├── crates/
│   ├── bella-engine/
│   │   ├── Cargo.toml
│   │   ├── ATTRIBUTION.md      ← required MIT attribution
│   │   └── src/
│   │       ├── lib.rs          ← public re-exports
│   │       ├── browser.rs      ← directory listing model
│   │       ├── frontmatter.rs  ← restricted OKF frontmatter reader
│   │       ├── geometry.rs     ← coordinate conversion
│   │       ├── links.rs        ← link/checkbox/table hit-testing
│   │       ├── markdown.rs     ← render pipeline
│   │       ├── md_config.rs    ← config.toml loader (written, but NOT wired — see capabilities.md)
│   │       ├── palette.rs      ← color-depth detection + RGB downgrade
│   │       ├── syntax.rs       ← syntect highlighting
│   │       └── theme.rs        ← color themes (bastiel cool-aurora default)
│   └── bella/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs         ← CLI + terminal lifecycle
│           ├── lib.rs          ← library re-exports (shared by bin + integration tests)
│           ├── app.rs          ← App state + all navigation logic
│           ├── events.rs       ← event loop + mappers + Action dispatcher
│           ├── history.rs      ← back/forward stack
│           ├── render_worker.rs ← background render thread (async markdown parse/render)
│           ├── selection.rs    ← text selection + clipboard
│           └── ui.rs           ← ratatui draw functions
├── planning/                   ← context, status, master-plan, specs, decisions
│                                 (a symlink into the private brain vault; gitignored here —
│                                  never link to it from a doc, the link 404s on GitHub)
└── reference/                  ← upstream zemse/hackmd source (read-only, not in workspace)
```
