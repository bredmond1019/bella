---
type: Index
title: Bella
description: A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.
---

# Bella

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

## Prerequisites

- Rust toolchain (stable, edition 2024) — install via [rustup](https://rustup.rs/)

## Setup

```bash
# 1. Clone the repo
git clone <repo-url> bella && cd bella

# 2. Build all crates
cargo build
```

## Running locally

```bash
# Release build
cargo build --release

# Run the viewer
cargo run -p bella -- <file|dir>
```

## Tests

```bash
cargo test
```

## Lint / format

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Directory map

```
bella/
├── .claude/                  ← Claude Code commands + SDLC workflow engines
├── crates/
│   ├── bella-engine/         ← render/layout library (palette, syntax, theme, links, markdown, geometry)
│   └── bella/                ← TUI binary (clap CLI, ratatui draw loop, events, app state, selection)
├── planning/                 ← context, status, master-plan, harness.json, decisions/, specs/
└── reference/                ← upstream zemse/hackmd source (excluded from workspace)
```

## Keybindings

### Keyboard

| Key | Action |
|---|---|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Tab` | Focus next link |
| `Shift-Tab` | Focus previous link |
| `Enter` | Follow focused link (local file or browser URL) |
| `[` | History back |
| `]` | History forward |
| `/` | Start search |
| `n` | Next search match |
| `N` | Previous search match |
| `Esc` | Clear focus / cancel search |
| `q` / `Ctrl-C` | Quit |

### Mouse

| Gesture | Action |
|---|---|
| Scroll wheel | Scroll up / down (3 lines per tick) |
| Hover | Highlight link under cursor |
| Click link | Follow link (local file or browser URL) |
| Click checkbox | Toggle checkbox visual state |
| Click + drag | Select text; releases copy selection to system clipboard (arboard) |
| Double-click | Select word under cursor (450 ms window); copies to system clipboard |

## Documentation

| Doc | Contents |
|---|---|
| [planning/context.md](planning/context.md) | Orientation + governing principles |
| [planning/master-plan.md](planning/master-plan.md) | Strategy + phase specifications |
| [planning/status.md](planning/status.md) | Current progress |
| [planning/harness.json](planning/harness.json) | SDLC validation/UI-test config (see `harness.examples.md`) |
| [planning/decisions/index.md](planning/decisions/index.md) | Architectural decision records (D1–…) |

---

*Initialized 2026-06-24 from `base-template` (commit `45bda73d575ceba2ae0216f67a10a5334de3f5b4`).*
