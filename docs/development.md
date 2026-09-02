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

### Visual regression testing

bella draws to a terminal, so ordinary tests can assert what is *in* a cell but not whether the
screen actually looks right. Three independent tiers cover that, and **none replaces another** —
they answer different questions and fail in different ways.

| Tier | What it captures | Gated? | Regenerate with |
|---|---|---|---|
| **1. Buffer assertions** | Geometry and content, in-process via ratatui's `TestBackend`. Never runs `main.rs` | yes, as part of `test` | ordinary `cargo` test runs |
| **2. Text scenes** | What the **real release binary** prints, driven through tmux and diffed against committed baselines in `tests/scenes/` | yes, as `scenes` | `bash scripts/capture_scenes.sh` |
| **3. VHS reference PNGs** | Colour, glyphs and font rendering, as images under a sanity + freshness gate — **not** pixel-diffed | yes, as `vhs-fresh` | `vhs scripts/vhs/reference-wide.tape` (and `-narrow`, `-collapse`) |

Tier 1 can pass while the real binary shows nothing, because it never starts one. Tier 2 catches
that. Tier 3 is the only tier that sees colour, but images are non-deterministic across font and
antialiasing changes, so it is checked for *plausibility and freshness* rather than diffed.

```bash
bash scripts/check_scenes.sh      # tier 2 — re-capture and diff against tests/scenes/
bash scripts/check_vhs_fresh.sh   # tier 3 — sanity + freshness of the reference PNGs
```

All three tiers read one manifest, `scripts/vhs/scenes.toml`, so a scene declared once is
consumed by both the text captures and the tapes.

**Both gated checks are `perTask: false`** in `planning/harness.json` — they run once per block at
the end review or terminal reconcile, not after every task. `scenes` drives 19 tmux sessions and
takes ~1m45s, and `vhs-fresh` compares git commit times, so it is red from the first task that
touches a render source until the re-capture lands. Neither per-task verdict was meaningful; the
block-level one is.

#### Four rules, each learned by breaking it

1. **A green gate is not evidence a capture is good.** `check_vhs_fresh.sh` passed two corrupt
   references — a 20032-byte and a 25135-byte PNG, both showing a bare shell prompt. A blank
   frame's size depends on how much shell text is on screen, so **no byte threshold separates a
   blank from a sparse real frame.** After re-capturing, open the images and look at them.
2. **Every `Screenshot` in a tape must be preceded by `Wait+Screen@30s /<pattern>/`**, never a bare
   `Sleep`. A fixed sleep races the app's startup; a readiness match makes the failure impossible
   rather than detectable. **The pattern must be ASCII** — waiting on `/bella ·/` (the middle dot
   bella actually prints) silently produces no screenshot and exits 0.
3. **A scene's `target` must be a committed fixture or a stable in-repo path**, never a shared or
   generated directory. Use `scripts/vhs/fixtures/`. A baseline that captures a directory outside
   this repo's control drifts whenever anything else writes there.
4. **Any block that changes what appears on screen owes a scene.** Add it to `scenes.toml` with a
   per-scene `min_bytes`, commit the baseline, and review the diff against the previous set rather
   than accepting it blind.

### Manual visual inspection

For ad-hoc looks that are not part of any gate:

| Tool | What it does | Use it for |
|---|---|---|
| `scripts/tui_capture.sh <file\|dir> [key ...]` | Drives bella in a detached tmux session and dumps the rendered screen as text | Quick structural checks — is a section present, did navigation land where expected |
| `vhs scripts/vhs/*.tape` (needs `brew install vhs`) | Scripts a real pty session and renders it to PNG/GIF | Eyeballing theme colours, alignment, wrapping |

`tui_capture.sh` depends on `bastion` for session lifecycle and is therefore an **interactive tool
only** — no gated check may depend on it, so that bella stays testable outside its monorepo.

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
