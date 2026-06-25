---
type: Handoff
created: 2026-06-25
---

# Handoff — Block B done; Block C (keyboard navigation) is next

> **For the next agent:** Read this immediately after `/prime`. Delete this file once consumed.

## What we're doing and why

Building Bella — a local-only terminal markdown viewer. Phase 0 is the foundation sequence
(Blocks A → B → C → D). Block A shipped `bella-engine` (pure render library, 37 unit +
1 integration tests). Block B shipped the `bella` binary — clap CLI, Crossterm raw mode,
Ratatui draw loop, `App` scroll model, pure key-mapping — 21 tests, all four gating checks
clean. Block C adds keyboard navigation: link focus cycling with visual highlight, `/` search
(jump to heading or text), and navigation history (back/forward). This is the next block in
`planning/master-plan.md`.

## Completed this session

- **Block B shipped** via `/sdlc-run 0.B-binary-skeleton --from implement` (PASS on first attempt):
  - `crates/bella/src/main.rs` — clap CLI (`bella <file>`), terminal lifecycle (raw mode,
    alternate screen, panic hook), sync event loop
  - `crates/bella/src/app.rs` — `App` struct: rendered lines, clamped scroll offset,
    `scroll_down/up/jump_top/jump_bottom` (note: `to_top/to_bottom` renamed → `jump_*` to
    satisfy `clippy::wrong_self_convention` on mutating methods)
  - `crates/bella/src/ui.rs` — `draw_reader`: body + 1-row statusline; pushes body height
    back to `App`; `TestBackend` draw assertions
  - `crates/bella/src/events.rs` — pure `map_key` → `Action` (j/k, g/G, arrows, PgDn/Up,
    Ctrl-d/u, q, Ctrl-C); `handle_event` drives `App` and sets `should_quit`
  - 21 bella tests + 37 engine tests + 1 integration = 59 total; all four gating checks exit 0
  - Commits: `e6aa18e` (implement), `6eac051` (docs), `1d75c76` (wrap-up)
- **`/close-out` run**: fmt/clippy/test/build + emoji gate all pass; `README.md` directory map
  patched to add `crates/bella/` entry (committed in this session)
- **`planning/status.md`** updated: Block B → Done; focus → Block C

## Remaining work

- **Block C — Keyboard navigation** ← next
  - Link focus ring: cycle through links in the rendered view with Tab/Shift-Tab; highlight
    the focused link visually (a colour or bracket style in the Ratatui render pass)
  - Link follow: `Enter` on a focused link → open in browser or descend into a local `.md`
    file; `Esc` dismisses focus
  - `/` search: open an inline search bar (bottom row); jump to first heading or text match;
    `n/N` to cycle matches; `Esc` to dismiss
  - History: `[` / `]` (or `Alt-Left/Right`) to navigate back/forward through visited files
  - No mouse — that is Block D
- Blocks D → J follow in order per `planning/master-plan.md`

## Open questions / choices

- `README.md` "Running locally" previously had a dead `cargo run -p bella` note marked
  "(Block B and later)". Block B is now shipped — that parenthetical should be dropped when
  README prose is next touched (minor; not blocking).
- `planning/0.A-workspace-engine-extraction/sdlc/sdlc-state.json` and
  `planning/0.B-binary-skeleton/sdlc/sdlc-state.json` are untracked pipeline breadcrumbs.
  They are safe to `.gitignore` or leave as-is; no action needed.

## Context the next agent needs

- `bella-engine` public surface lives in `crates/bella-engine/src/lib.rs`. The rendered
  output type is `Rendered` (contains `Vec<RenderedLine>`). Each `RenderedLine` carries span
  metadata that includes link ranges (`LinkMap` / `LinkTarget`) — this is the data structure
  Block C's focus ring will consume to locate tabbable links.
- `crates/bella-engine/src/links.rs` owns `LinkMap` and `LinkTarget` — read it to understand
  how link spans are tracked across wrapped lines.
- Upstream reference is at `reference/hackmd/` (excluded from Cargo workspace). For "how did
  hackmd do keyboard nav / link follow", start there.
- D2 (`planning/decisions/D2-engine-app-crate-split.md`) governs the crate boundary:
  `bella-engine` is attributed-derivative; `bella` is original. All Block C logic goes in
  `bella`, calling into `bella-engine` API only.
- Edit-sync types (`EditCtx`, `BlockInfo`, `row_source`) remain dormant — do NOT activate
  them until Block H.

## First command after `/prime`

`/generate-tasks 0.C-keyboard-navigation`
