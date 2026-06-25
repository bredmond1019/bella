---
type: Handoff
created: 2026-06-25
---

# Handoff — Block C shipped; Block D (mouse support) is next

> **For the next agent:** Read this immediately after `/prime`. Delete this file once consumed.

## What we're doing and why

Building Bella — a local-only terminal markdown viewer. Phase 0 is the foundation sequence
(Blocks A → B → C → D). Block C just shipped: full keyboard navigation is live — Tab/Shift-Tab
link focus ring, Enter to follow links (local `.md` files or browser URLs), `/`+n/N search
with match highlight, `[`/`]` history back/forward, and a `history.rs` module. 136 tests pass
across all crates, all four gating checks clean. A PR is open at
https://github.com/bredmond1019/bella/pull/1. Block D (mouse support) is the next block and
completes Phase 1 / v0.1.

## Completed this session

- **Block C shipped** via `/sdlc-flow 0.C-keyboard-navigation` (PASS on first review):
  - `crates/bella/src/history.rs` — new `NavigationHistory` struct with push/back/forward,
    scroll-position preservation, 9 unit tests
  - `crates/bella/src/app.rs` — full `App` rewrite: `focus_next/prev/clear_focus`,
    `scroll_to_focused_link`, `follow_focused` (local file swap or `open` URL), `go_back/forward`,
    `start/push/pop/commit/cancel_search`, `search_next/prev`, `load_file`; 43 unit tests
  - `crates/bella/src/events.rs` — extended `map_key` for Tab/Shift-Tab/Enter/`[`/`]`/`/`/n/N/Esc;
    `handle_event` drives all new `App` methods; 38 unit tests
  - `crates/bella/src/ui.rs` — search prompt in status row, focused-link highlight render, 5 tests
  - `crates/bella/src/main.rs` — wired search key loop into event dispatch
  - Total: 136 tests (98 bella + 37 engine + 1 integration); all four gating checks exit 0
  - Tasks 1–7 all passed on first implement attempt — no fix loops triggered
- **Close-out** (`/close-out`):
  - `CLAUDE.md` directory map updated to list `crates/bella-engine/` and `crates/bella/`
  - `README.md` "Keybindings" section added; stale "Block B and later" comment removed
  - Docs committed: `7489886`
- **PR opened**: https://github.com/bredmond1019/bella/pull/1
- **`planning/status.md`** updated: Block C → Done; focus → Block D

## Remaining work

- **Merge PR #1** (review agent handoff — merge when review passes)
- **Block D — Mouse support** ← next after merge
  - Scroll with wheel
  - Hover highlight over links
  - Click to follow link
  - Drag-select text
  - Double-click word select
  - This block completes Phase 1 / v0.1
- Blocks E → J follow in order per `planning/master-plan.md`

## Open questions / choices

- `planning/0.A-workspace-engine-extraction/sdlc/sdlc-state.json` and
  `planning/0.B-binary-skeleton/sdlc/sdlc-state.json` are untracked pipeline breadcrumbs —
  safe to `.gitignore` or leave as-is; not blocking.
- The `sdlc-flow` workflow script had a stale `recordFilesRead()` call that crashed the first
  run. It was patched inline in the session script. The base-template source should be fixed —
  worth flagging to the harness maintainer.

## Context the next agent needs

- Block D must go in `bella` crate only (per D2 — `bella-engine` is attributed-derivative;
  all new original logic goes in `bella`).
- Mouse events in Crossterm/Ratatui: enable via `crossterm::event::EnableMouseCapture` in the
  terminal init block (`crates/bella/src/main.rs`). The event loop already handles
  `Event::Key`; add a `Event::Mouse` arm.
- `LinkMap` (in `crates/bella-engine/src/links.rs`) has the link spans needed for click
  hit-testing. `geometry.rs` `body_pos` converts a terminal coordinate to a document
  coordinate — this is the bridge for click-to-follow.
- `geometry.rs` `select_word_at` already exists for double-click word selection.
- Edit-sync types (`EditCtx`, `BlockInfo`, `row_source`) remain dormant — do NOT activate
  until Block H.
- Upstream reference implementation lives at `reference/hackmd/` (excluded from workspace).

## First command after `/prime`

`/generate-tasks 0.D-mouse`
