---
type: Handoff
created: 2026-06-24
---

# Handoff — Block A done; Block B (binary skeleton) is next

> **For the next agent:** Read this immediately after `/prime`. Delete this file once consumed.

## What we're doing and why

Building Bella — a local-only, mouse-driven terminal markdown viewer (see D2 for the
architectural split). Block 0.A (workspace + `bella-engine` extraction) shipped this session
with a PASS verdict on the first review attempt: 38 tests pass, all four gating checks
(fmt/clippy/test/release build) exit 0. The codebase is clean and ready for Block B —
the `bella` binary crate that wires `bella-engine` into a real Crossterm/Ratatui TUI and
renders a file from the CLI (`cargo run -p bella -- <file>`). No mouse support yet; that is
Block D.

## Completed this session

- **Block 0.A shipped** via `/sdlc-run 0.A-workspace-engine-extraction --from implement`:
  - Root `Cargo.toml` workspace with `members = ["crates/*"]`, `exclude = ["reference"]`
  - `crates/bella-engine`: 6 ported modules (`markdown`, `links`, `syntax`, `theme`,
    `palette`, `md_config`) + new pure `geometry.rs`; all App/cloud deps removed; edit-sync
    types (`row_source`, `EditCtx`, `BlockInfo`) preserved dormant
  - 38 tests (37 unit + 1 integration) pass; public surface: `render_with_edit`, `Rendered`,
    `LinkMap`, `CheckboxMap`, `TableMap`, `LinkTarget`, `Theme`, `body_pos`, `select_word_at`
  - Attribution: `LICENSE` + `ATTRIBUTION.md` + per-file headers (zemse/hackmd @ 7650cdc, MIT)
  - Commits: `184005a` (implement), `8ee949b` (docs), `a391dd1` (wrap-up)
- **`/close-out` run**: all 4 gating checks + emoji gate passed; doc patch added
  `planning/decisions/index.md` row to `README.md` (uncommitted — included in this session's commit)
- **`planning/status.md`** updated: Block A → Done; focus → Block B

## Remaining work

- **Block B — Binary skeleton renders a file (no mouse)** ← next
  - Create `crates/bella` binary crate in the workspace
  - Wire `bella-engine::render_with_edit` into Crossterm raw mode + Ratatui `Frame`
  - CLI: `bella <file>` → render + scroll (j/k or arrow keys); `q` to quit
  - No mouse, no keyboard nav beyond scroll and quit — that is Blocks C and D
- Blocks C → J follow in order per `planning/master-plan.md`

## Open questions / choices

- `cargo run -p bella` in `README.md`'s "Running locally" section is currently dead (the `bella`
  crate doesn't exist yet). The comment "(Block B and later)" is accurate. Verify once Block B
  ships and remove the parenthetical.
- `planning/0.A-workspace-engine-extraction/sdlc/sdlc-state.json` is untracked — it's an
  internal pipeline breadcrumb for `/sdlc-block` resume. Safe to `.gitignore` or leave; no
  action required.

## Context the next agent needs

- `bella-engine` public surface lives in `crates/bella-engine/src/lib.rs` (re-exports only;
  the crate is a library). The integration test at `crates/bella-engine/tests/render.rs`
  shows how to call `render_with_edit`.
- Upstream reference frozen at `reference/hackmd/` (excluded from workspace via
  `Cargo.toml:exclude`). For "how did hackmd do X" questions, read from there — don't
  depend on `../potential-projects/`.
- D2 (`planning/decisions/D2-engine-app-crate-split.md`) governs the crate boundary:
  everything in `bella-engine` is attributed-derivative; everything in `bella` is original.
- The edit-sync types (`EditCtx`, `BlockInfo`, `row_source` on `RenderedLine`) are dormant in
  `bella-engine` — do NOT activate them in Block B. They ship in Block H.

## First command after `/prime`

`/generate-tasks 0.B-binary-skeleton`
