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

## The Block Contract

`/generate-tasks` reads **only the target block's section** below — not this overview, not
sibling blocks. So every block section must be self-sufficient and hand the generator the
four things it is required to produce: concrete **files per task** (for disjoint, merge-safe
decomposition), **observable acceptance criteria**, correct **scope boundaries**, and the
**engine surface** it leans on. Every block A–G therefore uses the same skeleton:

- **What** — the scope, in implementation terms.
- **Why** — the motivation (keeps the generator from over- or under-scoping).
- **Files** — *new* vs *modified*, named by path and crate. This is load-bearing: tasks that
  share a file must be serialized (`dependsOn`) or append-only; tasks owning distinct files run
  in parallel. A block that doesn't name its files forces the generator to guess ownership.
- **Engine surface** — which `bella-engine` exports it consumes, and any *new* export it must add.
- **Out of scope** — explicit boundaries; what belongs to a later block.
- **Acceptance criteria** — each a true/false condition a reviewer can check against the diff.

Phase 3+ blocks (H/I/J) carry the full contract too — mapped while the editor/edit-sync context
was fresh — but they are **forward-looking**: H/J reach into sibling repos and I depends on a
not-yet-built project, so expect to refine their Files/Engine-surface lines when each becomes next.

---

## Phase 0 — Foundation

### Block A — Workspace + engine extraction
- **What:** Create the Cargo workspace and the two crates. The root `Cargo.toml` uses
  `members = ["crates/*"]` and **`exclude = ["reference"]`** — the gitignored `reference/` tree
  holds frozen upstream snapshots (`hackmd`, `md-tui`) with their own `Cargo.toml`s and must
  never be pulled into the workspace. Port the engine files from `reference/hackmd/src/tui/` into
  `crates/bella-engine/src/`, flattening imports `crate::tui::X → crate::X`. Add a new pure
  `geometry.rs` lifting `body_pos()` (hackmd events.rs:1986) and `select_word_at()`
  (events.rs:2068) as standalone functions.
- **Why:** Get the hard, reusable IP compiling in isolation behind a clean public surface before
  any app code exists — the foundation every later block drives.
- **Files:**
  - *New* root `Cargo.toml` (workspace), `crates/bella-engine/Cargo.toml`.
  - *New (ported)* `crates/bella-engine/src/{markdown.rs, links.rs, syntax.rs, theme.rs,
    palette.rs, md_config.rs}`.
  - *New (original)* `crates/bella-engine/src/geometry.rs`, `crates/bella-engine/src/lib.rs`.
  - *New (legal)* `crates/bella-engine/LICENSE` (MIT + zemse/hackmd copyright),
    `crates/bella-engine/ATTRIBUTION.md`. Each ported file keeps a 2-line source header.
- **Engine surface (exports `lib.rs` must add):** `render_with_edit`, `Rendered`,
  `LinkMap`/`CheckboxMap`/`TableMap`, `LinkTarget`, `Theme`, `body_pos`, `select_word_at`. This is
  the public contract every `bella`-crate block consumes — keep it stable.
- **Out of scope:** No `bella` (app) crate yet — Block B. No cloud code (never written). Keep all
  edit-sync types **dormant** — do not delete `row_source`/`EditCtx`/`BlockInfo` (Phase 3 needs
  them). Keep the `images` feature on by default (no engine surgery in v0.1).
- **Acceptance criteria:** `cargo build -p bella-engine` succeeds. A unit test calls
  `render_with_edit("# hi\n\n```rs\nfn main(){}\n```", …)` and asserts non-empty `Rendered.lines`
  with heading + code-block styling. `body_pos`/`select_word_at` are callable as pure functions
  with their own unit tests. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
  clean.

### Block B — Binary skeleton renders a file (no mouse)
- **What:** `bella file.md` (clap entry) → read raw → `bella_engine::render_with_edit` → static
  ratatui draw with scroll (`j/k`, `g/G`, `q`). Minimal `App`/`Reader`, a synchronous event loop,
  and a reader + statusline draw path.
- **Why:** Prove the engine drives a real terminal render before adding any interaction.
- **Files:**
  - *New* `crates/bella/Cargo.toml`, `crates/bella/src/main.rs` (clap entry, terminal
    setup/teardown), `crates/bella/src/app.rs` (`App`/`Reader` state, scroll offset),
    `crates/bella/src/events.rs` (sync key loop: `j/k`/`g/G`/`q`), `crates/bella/src/ui.rs`
    (`draw_reader`, `draw_statusline`).
- **Engine surface:** consumes `render_with_edit` + `Rendered` from Block A. Adds no engine exports.
- **Out of scope:** No mouse (Block D). No link-following, search, or history (Block C). No config
  or themes (Block F). No directory mode (Block E) — a path arg is required for now.
- **Acceptance criteria:** A real `.md` displays with syntax-highlighted code blocks and scrolls
  smoothly with `j/k`/`g/G`; `q` exits cleanly restoring the terminal; `cargo
  build`/`clippy`/`fmt`/`test` green.

---

## Phase 1 — Core viewer (the v0.1 deliverable)

### Block C — Keyboard navigation
- **What:** Link focus (`Tab`/`Shift-Tab` cycles the engine's `LinkMap` entries) + follow
  (`Enter`: relative-link → open that file in the reader; URL → `open` crate), `/` in-document
  search with `n/N` match cycling, and a back/forward history stack.
- **Why:** A reader is only usable once you can move between linked files and find text — this is
  the first block that makes Bella navigable, not just scrollable.
- **Files:**
  - *Modify* `crates/bella/src/events.rs` (key handlers: `Tab`/`Shift-Tab`/`Enter`/`/`/`n`/`N`),
    `crates/bella/src/app.rs` (focused-link index, search state, history stack wiring),
    `crates/bella/src/ui.rs` (focused-link highlight, search-match highlight, search prompt).
  - *New* `crates/bella/src/history.rs` (back/forward stack — its own file so it can be a parallel
    task with its own unit tests).
- **Engine surface:** consumes `LinkMap` + `LinkTarget` (exported by Block A). No new engine
  exports needed.
- **Out of scope:** mouse link-following and selection (Block D); cross-file / project-wide search;
  fuzzy file open (Block E).
- **Acceptance criteria:** In a multi-file doc set, `Tab` cycles visible links with a visible
  highlight; `Enter` on a relative link opens that file and on a URL launches the browser;
  `/term` + `n`/`N` cycles matches with the viewport scrolling to each; back/forward restores the
  prior file *and* scroll position. `cargo build`/`clippy`/`fmt`/`test` green.

### Block D — Mouse  **(= v0.1)**
- **What:** Enable `crossterm` mouse capture; scroll/hover/click → link follow + checkbox visual
  toggle (`click_at` hit-test), drag-select → `arboard` clipboard copy, double-click word-select
  (450ms window). Uses engine `body_pos`/`select_word_at` for coordinate→content mapping.
- **Why:** Mouse-driven reading is the differentiating UX — the whole reason Bella exists rather
  than md-tui. **This block is the v0.1 release.**
- **Files:**
  - *Modify* `crates/bella/src/events.rs` (mouse event arm: scroll/hover/down/drag/up,
    double-click timing), `crates/bella/src/app.rs` (selection range, hover state, last-click
    timestamp), `crates/bella/src/ui.rs` (selection highlight rendering), `crates/bella/src/main.rs`
    (enable/disable mouse capture in terminal setup/teardown).
  - *New* `crates/bella/src/selection.rs` (selection model + clipboard copy via `arboard` — its
    own file, unit-testable independent of the event loop).
- **Engine surface:** consumes `body_pos`, `select_word_at`, `LinkMap`, `CheckboxMap` (all from
  Block A). New dep: `arboard`.
- **Out of scope:** editor-mode selection and edit-sync (Phase 3); keyboard-driven copy; mouse in
  the file browser (handled by Block E reusing this block's geometry).
- **Acceptance criteria:** Click-to-follow, drag-to-copy (selection lands in the system
  clipboard), checkbox-click visual toggle, and double-click word-select all work in a real
  terminal; `cargo build`/`clippy`/`fmt`/`test` green.

### Block E — File browser (directory navigator)
- **What:** `bella` with no arg (or a dir arg) → a **hackmd-style directory navigator** (port
  hackmd's `Browser`, app.rs:758 / `draw_browser` ui.rs:1086). Full-screen pane showing the
  current folder's entries: subdirectories + `.md`/`.mdx` files (`walkdir`/`ignore`,
  gitignore-aware), with a `..` parent entry. `Enter` descends into a folder or opens a file;
  `..`/Backspace ascends; `j/k` move the cursor; mouse click/scroll work (reuse Block D geometry).
  Opening a file enters the reader; back returns to the browser at the same cursor.
- **Why:** Daily-driver ergonomics — explore an unfamiliar tree without leaving the terminal.
- **Files:**
  - *New* `crates/bella/src/browser.rs` (directory model, entry list, cursor, descend/ascend logic).
  - *Modify* `crates/bella/src/main.rs` (no-arg / dir-arg dispatch into browser mode),
    `crates/bella/src/app.rs` (a mode enum: `Reader` | `Browser`, with browser cursor state),
    `crates/bella/src/events.rs` (browser key + mouse handlers), `crates/bella/src/ui.rs`
    (`draw_browser`).
- **Engine surface:** none new — reuses Block D's mouse geometry for click/scroll. New deps:
  `walkdir`/`ignore`.
- **Out of scope:** md-tui's flat fuzzy-find-all-files index is **out of scope for v0.1** (kept as
  reference; revisit as a `/` overlay in Phase 2 if wanted). A `/`-driven *in-current-listing*
  filter is optional, not required.
- **Acceptance criteria:** browse → descend/ascend → open → back round-trips with the cursor
  preserved; non-markdown files are hidden, directories are shown; `..` ascends and Backspace
  matches it; `cargo build`/`clippy`/`fmt`/`test` green.

---

## Phase 2 — Depth / Hardening

### Block F — Config + themes + live reload
- **What:** TOML config at `~/.config/bella/config.toml` (`theme`, `width`, `line_numbers`); apply
  it at startup and select the engine `Theme` from it. Port the `poll_external_change` file-watch
  pattern (hackmd app.rs:2275) so an on-disk change to the open file re-renders it live.
- **Why:** Personalization plus freshness — read a file while a tool rewrites it and see updates
  without re-opening.
- **Files:**
  - *New* `crates/bella/src/config.rs` (serde structs, default path resolution, load + defaults).
  - *Modify* `crates/bella/src/main.rs` (load config before launch), `crates/bella/src/app.rs`
    (hold resolved config; apply width/line_numbers; reload-on-change state),
    `crates/bella/src/events.rs` (poll tick that detects the mtime change and re-renders).
- **Engine surface:** selects `Theme` (from Block A) by name; passes width/line-number options into
  the existing render path. No new engine exports unless a theme-by-name lookup must be added —
  if so, add it to `lib.rs` and note it here.
- **Out of scope:** per-file or per-directory config; a live theme-switch keybinding (optional
  follow-up); `notify`-based watching — stay with the ported poll pattern for parity with hackmd.
- **Acceptance criteria:** A missing config file falls back to documented defaults; editing
  `config.toml` values (theme/width/line_numbers) changes the next launch's render; editing the
  open file on disk triggers a re-render within the poll window; `cargo
  build`/`clippy`/`fmt`/`test` green.

### Block G — Images decision + packaging
- **What:** Validate `ratatui-image` across the terminals Brandon uses; decide keep vs.
  feature-off-by-default and record it as a decision. Establish the `cargo install` path and write
  the README (usage, screenshot, brand/attribution framing).
- **Why:** Turn the working tool into a shippable, installable artifact with an honest public story.
- **Files:**
  - *Modify* `crates/bella-engine/Cargo.toml` and/or `crates/bella/Cargo.toml` (the `images`
    feature default per the decision), `README.md` (install + screenshot + framing).
  - *New* `planning/decisions/D<next>-images-and-packaging.md` (the keep/feature-off decision +
    terminal support matrix).
- **Engine surface:** no API change — only the `images` feature flag's default may flip.
- **Out of scope:** new rendering capabilities; publishing to crates.io (local `cargo install`
  only for now, per the local-only constraint).
- **Acceptance criteria:** the terminal support matrix is documented and the keep/feature-off
  decision is recorded in `planning/decisions/`; `cargo install --path crates/bella` produces a
  working `bella` binary; the README has a screenshot and the attribution framing from D2; `cargo
  build`/`clippy`/`fmt`/`test` green.

---

## Phase 3+ — Differentiating Build

### Block H — Editor (un-dormant edit-sync)
- **What:** Wake the edit-sync machinery that has been kept **dormant since Block A** and build a
  split editor: enter with `e`/`i` → a raw buffer pane beside a live preview that re-renders the
  buffer through `render_with_edit` with an `EditCtx` cursor. Insert/newline/backspace/delete,
  undo/redo (Ctrl-Z / Ctrl-Y), a vim-style command line (`:w` save, `:wq`, `:q`/`:q!`,
  `:preview` full-screen), and mouse: click in the raw pane places the cursor at the right byte via
  `row_source`, drag-select + `y` copies. Port the hackmd **Split** editor; the legacy `InPlace`
  block-toggle mode comes across **dormant** (compiled, inactive) just as it is upstream.
- **Why:** The "between md-tui and VS Code" promise and the single biggest differentiator — and the
  payoff for paying the cost of keeping `row_source`/`EditCtx`/`BlockInfo` alive since Block A.
- **Files:**
  - *New* `crates/bella/src/edit.rs` — port `EditState`, `EditSelection`, `EditSnapshot`,
    `EditMode` (Split active / InPlace dormant), `UNDO_LIMIT` from hackmd `app.rs:454–532`. Cursor
    is a byte offset into the raw buffer; both selection ends sit on UTF-8 char boundaries.
  - *Modify* `crates/bella/src/app.rs` — `enter_edit`/`exit_edit`/`exit_edit_discard`,
    `edit_insert`/`edit_newline`/`edit_backspace`/`edit_delete`, `edit_undo`/redo, and save via
    `std::fs::write(path, &raw)` (port from hackmd `app.rs:2369` + `2643–2853`); the view/mode enum
    gains an editing flavor.
  - *Modify* `crates/bella/src/events.rs` — `handle_edit_key` (637), `handle_edit_command_key`
    (925), `exec_edit_command` (997), `complete_edit_command` (914), `in_split_edit` (1777), and
    `xy_to_source_offset` (2418, the click→byte mapper that reads `row_source`).
  - *Modify* `crates/bella/src/ui.rs` — `draw_edit_split` (584; side-by-side ≥100 cols, vertical
    stack below) and `draw_edit_command_line` (1636).
- **Engine surface:** consumes the **dormant exports preserved by Block A** — `render_with_edit(…,
  edit: Option<EditCtx>, …)`, `EditCtx`, `BlockInfo`, `Rendered.row_source`, `Rendered.cursor_xy`.
  No new engine code if Block A preserved them faithfully; this block is their first real caller.
  If any were trimmed during an engine rewrite, re-add them to `bella-engine`'s `lib.rs` here.
- **Out of scope:** cloud/collaborative sync (never written); the `InPlace` block-toggle mode
  (kept dormant, not activated); LSP/autocomplete; the mev diagnostics panel (Block I).
- **Acceptance criteria:** `e` enters split edit on the open file; typing and Enter/Backspace/Delete
  mutate the raw buffer and the preview re-renders live with the cursor visible; Ctrl-Z/Ctrl-Y undo
  and redo across ≥1 edit; `:w` writes to disk and clears `dirty`, `:q` refuses on a dirty buffer
  while `:q!` discards, `:wq` saves and exits; a click in the raw pane lands the cursor on the
  correct byte (verified against `row_source`); `:preview` shows the unsaved buffer full-screen and
  Esc returns. Unit tests cover the click→byte mapping, undo/redo, and the command parser. `cargo
  build`/`clippy`/`fmt`/`test` green.

### Block I — mev live validation (opt-in)
- **What:** Add the sibling **`markdown-engine-validator` (mev)** as a library dependency and wire
  an **opt-in** diagnostics path: on save (and optionally on an idle tick) validate the buffer and
  surface parse/link/schema diagnostics in a panel or gutter, each anchored to its display row.
  Strictly scoped to **configured content roots** — never force the portfolio schema on every `.md`.
- **Why:** Write + validate in one tool — closes the content workflow loop (e.g. learn-agentic-ai
  content) without leaving the terminal.
- **Files:**
  - *New* `crates/bella/src/validate.rs` — run mev over the buffer, map diagnostics → display rows
    (anchored via `Rendered` geometry), and gate on the opt-in config flag + content-root detection.
  - *Modify* `crates/bella/src/ui.rs` — diagnostics panel / gutter markers in editor and reader.
  - *Modify* `crates/bella/src/events.rs` — trigger validation on `:w`/save and on an idle tick;
    a keybinding to toggle the panel.
  - *Modify* `crates/bella/src/config.rs` — a `[validate]` section (`enabled`, content-root globs).
- **Engine surface:** none — mev is a separate crate dep, **not** part of `bella-engine`. Reuses
  `Rendered` row geometry to anchor diagnostics; adds no engine exports.
- **Out of scope:** authoring or enforcing the schema here (mev owns the rubric); validating `.md`
  outside configured content roots; auto-fix. **Cross-repo dependency:** mev is listed "not started"
  in the brain — this block cannot start until mev exposes a library surface; track that as a
  prerequisite, not work inside this block.
- **Acceptance criteria:** with validation enabled inside a configured content root, saving a file
  with a known violation shows a diagnostic anchored to the offending line; with validation disabled
  or outside content roots, no diagnostics appear and there is no added latency; the panel toggles;
  mev is integrated as a pinned path/git dep; `cargo build`/`clippy`/`fmt`/`test` green.

### Block J — Absorb into bastion
- **What:** Make `bella-engine` a **bastion** workspace dependency and add a `bastion bella
  <file|dir>` subcommand backed by a new `src/bella/` module that reuses bastion's config and
  event-loop conventions. The standalone `bella` crate stays independent; bastion consumes the
  **engine**, re-using (or re-homing) the reader/browser shell rather than the `bella` binary.
- **Why:** One terminal command center — viewing markdown (and agentic-workflow output) lives next
  to the rest of the ops stack. Clean rather than a port because versions already align (ratatui
  0.30, crossterm 0.29, edition 2024 — see D2).
- **Files** *(primarily in the **bastion** repo, not bella)*:
  - *New* `bastion/src/bella/` (mod + view/event glue adapting bella's reader/browser to bastion's
    loop).
  - *Modify* bastion `Cargo.toml` (add `bella-engine`, pinned path/git) and bastion's
    CLI/subcommand registry (register `bella`).
  - *Modify (in bella repo)* possibly `crates/bella-engine/Cargo.toml` / `lib.rs` if the reader or
    browser logic must be promoted out of the `bella` binary into a bastion-consumable surface.
- **Engine surface:** bastion consumes the full `bella-engine` public API (`render_with_edit`,
  `Rendered`, `LinkMap`, `body_pos`/`select_word_at`, `Theme`). The open design question this block
  resolves: share the reader/browser shell via the engine vs. copy it into bastion — record the
  choice as a decision.
- **Out of scope:** relocating the standalone `bella` binary into bastion (it stays separate); the
  editor (Block H) inside bastion unless explicitly wanted; any cloud feature. **Cross-repo:** most
  of this work lands and is logged in the **bastion** repo per the brain's update protocol.
- **Acceptance criteria:** `bastion bella <file>` renders a file and `bastion bella <dir>` opens the
  browser, reusing bastion's config + key conventions; bastion's existing suite stays green;
  `bella-engine` builds as a bastion dep with no version conflicts; the share-vs-copy decision is
  recorded in `planning/decisions/`. `cargo build`/`clippy`/`fmt`/`test` green in both repos.

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
