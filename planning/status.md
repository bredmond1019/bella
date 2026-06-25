---
type: ProjectStatus
title: Bella Status
description: Current state and progress tracker for Bella.
---

# STATUS — Current State & Progress

**Last updated:** 2026-06-25 — Block C complete; keyboard navigation live with 136 passing tests
**Current focus:** Phase 0, Block D — Mouse support

---

## How to Read / Update This File

- Status values: `Not started` · `In progress` · `Done` · `Blocked` · `Skipped`
- Keep `Current focus` and `Last updated` accurate; update as work happens.
- This file is **state only**. For what the work means, see `master-plan.md`.

---

## Progress Table

### Phase 0 — Foundation
| Block | What | Status | Notes |
|---|---|---|---|
| Block A | Workspace + engine extraction | Done | `bella-engine` crate: 6 ported modules + `geometry.rs`; 38 tests pass; public surface complete |
| Block B | Binary skeleton renders a file (no mouse) | Done | `bella` binary crate: clap CLI, ratatui draw loop, scroll engine; 21 tests pass |

### Phase 1 — Core viewer (v0.1)
| Block | What | Status | Notes |
|---|---|---|---|
| Block C | Keyboard navigation | Done | Tab/Shift-Tab link focus ring, Enter follow (local file or browser), `/`+n/N search with match highlight, `[`/`]` back/forward history; `history.rs` module; 136 tests pass |
| Block D | Mouse | Not started | Scroll/hover/click/drag-select/double-click — **= v0.1** |
| Block E | File browser (directory navigator) | Not started | Port hackmd `Browser`; descend/ascend, `.md/.mdx` + dirs; mouse + `j/k` |

### Phase 2 — Depth / Hardening
| Block | What | Status | Notes |
|---|---|---|---|
| Block F | Config + themes + live reload | Not started | TOML config; port `poll_external_change` |
| Block G | Images decision + packaging | Not started | `ratatui-image` validation; `cargo install`; README |

### Phase 3+ — Differentiating Build
| Block | What | Status | Notes |
|---|---|---|---|
| Block H | Editor (un-dormant edit-sync) | Not started | `EditState` + `draw_edit_split` against preserved `row_source` |
| Block I | mev live validation (opt-in) | Not started | `markdown-engine-validator` as library; content trees only |
| Block J | Absorb into bastion | Not started | `bastion bella` subcommand |

---

## Decisions & Deviations Log

*Record deviations from the plan and notable in-flight choices here. Promote durable ones to
`decisions/` via `/log-work`.*

---

## Quick Self-Check

- Is `Current focus` accurate?
- Any `In progress` rows that are actually `Done`?
- Anything `Blocked` that needs surfacing?

---

*State only. For what things mean, see master-plan.md. For orientation, see context.md.*
