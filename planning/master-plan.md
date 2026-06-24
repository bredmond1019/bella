---
type: Plan
title: Bella Master Plan
description: Strategic roadmap and phase specifications for Bella.
---

# Bella — Master Plan

*Living document. Created 2026-06-24.*

## The Goal, Stated Plainly

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

Brandon works exclusively in the terminal and wants the HackMD reading UX (click links,
drag-select, checkbox toggle, eventually split-edit) without HackMD's cloud API — something
between a bare viewer (md-tui) and VS Code. **Bella** is that tool: a Rust TUI that renders
markdown beautifully and is driven by the mouse, running entirely offline.

"Ready" for **v0.1** (end of Phase 1) means: open a multi-file markdown set, navigate links by
mouse *and* keyboard, drag-select a passage and have it land in the system clipboard — no cloud,
no API key, no editor. The editor is a deliberate Phase 3 extension.

This is portfolio-grade tooling that must be **defensibly Brandon's**. The hard layout/geometry
engine is isolated in an attributed crate derived from MIT-licensed `github.com/zemse/hackmd`;
the application shell is original work. The crate boundary is the ownership story made structural.

## The Destination

A standalone binary `bella <file|dir>` for daily terminal markdown reading — and, in the
**bastion family**, the eventual `bastion bella` subcommand (markdown viewing is directly
related to inspecting agentic workflow outputs). Dependency versions already align with bastion
(ratatui 0.30, crossterm 0.29, edition 2024) so absorption is a clean Phase 3 step, not a port.

## Architecture / Design Overview

Two-crate Cargo workspace. The crate boundary is the legal + narrative boundary.

```
bella/                          # standalone repo (separate git)
├── Cargo.toml                  # [workspace] members
└── crates/
    ├── bella-engine/           # VENDORED + ATTRIBUTED (derived from zemse/hackmd @ 7650cdc, MIT)
    │   └── src/                #   markdown.rs · links.rs · syntax.rs · theme.rs · palette.rs
    │                           #   · md_config.rs · geometry.rs (NEW) · lib.rs
    │                           #   Render/layout/hit-test/geometry. Progressively rewritten.
    └── bella/                  # THE BINARY — original work
        └── src/                #   main.rs · app.rs · events.rs · ui.rs · config.rs
                                #   No cloud code (never written). No EditState until Phase 3.
```

**Load-bearing decisions:**
- **Reuse the cloud-free engine subgraph, rewrite the plumbing.** `markdown.rs → {links (std),
  syntax → palette, theme → palette}` has zero app coupling (verified) — it ports cleanly into
  `bella-engine` with only import-path flattening (`crate::tui::X → crate::X`). The cloud rot
  lives in `app.rs`/`events.rs`/`ui.rs`, which we write fresh — so the 213 cloud refs are
  dropped by simply not writing them, not by untangling.
- **v0.1 = viewer + mouse, no editor.** The engine's `row_source`/`EditCtx`/`BlockInfo`
  edit-sync machinery is preserved **dormant** so Phase 3 editing needs no byte-range rederivation.
- **Parser pulldown-cmark, highlighting syntect** — both inherited via the engine; pure Rust,
  clean build, and required if the editor ever ships.
- **No async runtime.** The viewer is fully synchronous (mirrors bastion's sync `sessions/`).

See `decisions/D2-engine-app-crate-split.md` for the full rationale + attribution terms.

---

## Phase 0 — Foundation

### Block A — Workspace + engine extraction
- **What:** Create the Cargo workspace and the two crates. The root `Cargo.toml` must use
  `members = ["crates/*"]` and **`exclude = ["reference"]`** — the gitignored `reference/`
  tree holds frozen upstream snapshots (`hackmd`, `md-tui`) with their own `Cargo.toml`s and
  must never be pulled into the workspace. Port the 7 engine files from
  `reference/hackmd/src/tui/` into `crates/bella-engine/src/` (`markdown.rs`,
  `links.rs`, `syntax.rs`, `theme.rs`, `palette.rs`, `md_config.rs`), flattening imports
  `crate::tui::X → crate::X`. Add `geometry.rs` lifting `body_pos()` (hackmd events.rs:1986) and
  `select_word_at()` (events.rs:2068) as pure functions. Write `lib.rs` re-exporting
  `render_with_edit`, `Rendered`, `LinkMap`/`CheckboxMap`/`TableMap`, `LinkTarget`, `Theme`,
  `body_pos`, `select_word_at`. Add `LICENSE` (MIT + zemse/hackmd copyright) and `ATTRIBUTION.md`.
- **Why:** Get the hard, reusable IP compiling in isolation behind a clean public surface before
  any app code exists.
- **Build notes:** Keep `images` feature on by default (no engine surgery in v0.1). Keep all
  edit-sync types dormant — do not delete `row_source`/`EditCtx`.
- **Acceptance criteria:** `cargo build -p bella-engine` + a unit test that calls
  `render_with_edit("# hi\n\n```rs\nfn main(){}\n```", …)` and asserts non-empty `Rendered.lines`
  with heading + code-block styling. `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` clean.

### Block B — Binary skeleton renders a file (no mouse)
- **What:** `bella file.md` (clap entry) → read raw → `bella_engine::render_with_edit` → static
  ratatui draw with scroll (`j/k`, `g/G`, `q`). Minimal `App`/`Reader` in `app.rs`; sync event
  loop in `events.rs`; `draw_reader` + `draw_statusline` in `ui.rs`.
- **Why:** Prove the engine drives a real terminal render before adding interaction.
- **Acceptance criteria:** A real `.md` displays with syntax-highlighted code blocks and scrolls
  smoothly; `cargo build`/`clippy`/`fmt`/`test` green.

---

## Phase 1 — Core viewer (the v0.1 deliverable)

### Block C — Keyboard navigation
- **What:** Link focus (`Tab`) + follow (`Enter`: relative-link resolution + `open` for URLs),
  `/` in-document search + `n/N`, history back/forward stack.
- **Acceptance criteria:** Navigate a multi-file doc set by keyboard; links resolve correctly.

### Block D — Mouse  **(= v0.1)**
- **What:** `crossterm::EnableMouseCapture`; scroll/hover/click → `click_at` (link follow,
  checkbox visual toggle), drag-select → `arboard` clipboard copy, double-click word-select
  (450ms window). Uses engine `body_pos`/`select_word_at`.
- **Acceptance criteria:** Click-to-follow, drag-to-copy, and double-click-word all work in a
  real terminal. **This block is the v0.1 release.**

### Block E — File browser (directory navigator)
- **What:** `bella` with no arg (or a dir arg) → a **hackmd-style directory navigator** (port
  hackmd's `Browser`, app.rs:758 / `draw_browser` ui.rs:1086). Full-screen pane showing the
  current folder's entries: subdirectories + `.md`/`.mdx` files (`walkdir`/`ignore`,
  gitignore-aware), with a `..` parent entry. `Enter` descends into a folder or opens a file;
  `..`/Backspace ascends; `j/k` move the cursor; mouse click/scroll work (reuse Block D
  geometry). Opening a file enters the reader; back returns to the browser at the same cursor.
- **Why:** Daily-driver ergonomics — explore an unfamiliar tree without leaving the terminal.
- **Build notes:** Directory-navigator model only (chosen 2026-06-24). md-tui's flat
  fuzzy-find-all-files index is **out of scope for v0.1** (kept as reference; revisit as a `/`
  overlay in Phase 2 if wanted). A `/`-driven *in-current-listing* filter is optional.
- **Acceptance criteria:** browse → descend/ascend → open → back round-trips, cursor preserved;
  non-markdown files are hidden, directories are shown.

---

## Phase 2 — Depth / Hardening

### Block F — Config + themes + live reload
- **What:** TOML config at `~/.config/bella/config.toml` (theme, width, line_numbers); port the
  `poll_external_change` file-watch pattern (hackmd app.rs:2275) for on-disk-change reload.

### Block G — Images decision + packaging
- **What:** Validate `ratatui-image` across terminals; decide keep / feature-off default.
  `cargo install` path; README with screenshot and brand framing.

---

## Phase 3+ — Differentiating Build

### Block H — Editor (un-dormant edit-sync)
- **What:** Port `EditState` + `draw_edit_split` against the preserved `row_source` machinery;
  vim-ish keys, `:w` save, undo/redo.

### Block I — mev live validation (opt-in)
- **What:** `markdown-engine-validator` as a library dep; opt-in diagnostics panel on save,
  scoped to content trees only — never force the portfolio schema on every `.md`.

### Block J — Absorb into bastion
- **What:** `bella-engine` becomes a bastion dependency; add `src/bella/` module + `bastion bella`
  subcommand reusing bastion's config/event-loop conventions.

---

## Quick Reference Sequence Table

| Phase | Block | What | Why | Role in destination |
|---|---|---|---|---|
| 0 | A | Workspace + engine extraction | Hard IP compiling behind a clean surface | Foundation everything drives |
| 0 | B | Binary renders a file (no mouse) | Engine drives a real render | First visible output |
| 1 | C | Keyboard navigation | Links + search + history | Usable reader |
| 1 | D | Mouse (drag-select, click, dbl-click) | The differentiating UX | **v0.1 release** |
| 1 | E | File browser | Open without a path arg | Daily-driver ergonomics |
| 2 | F | Config + themes + live reload | Personalization + freshness | Polish |
| 2 | G | Images + packaging | Distribution | Shippable artifact |
| 3+ | H | Editor (split-edit) | The "between md-tui and VS Code" promise | Differentiator |
| 3+ | I | mev live validation | Write+validate in one tool | Content workflow |
| 3+ | J | Absorb into bastion | One terminal command center | Bastion family |

---

*Sequenced by dependency and competence, not calendar. When life gets in the way, pick up
where you left off.*
