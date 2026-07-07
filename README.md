---
type: Index
title: Bella
description: A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.
doc_id: bella-readme
layer: [console]
project: bella
status: active
keywords: [terminal markdown viewer, TUI, ratatui, mouse support, local-only, Rust]
related: [bella-docs-index, bella-planning-index]
---

# Bella

> Part of the **Bastion** ecosystem — see the [bastion-os](https://github.com/bredmond1019/bastion-os) front door for the full architecture.

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

## Prerequisites

- Rust toolchain (stable, edition 2024) — install via [rustup](https://rustup.rs/)

## Setup

```bash
# 1. Clone the repo
git clone https://github.com/bredmond1019/bella && cd bella

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
│   └── bella/                ← TUI binary (clap CLI, ratatui draw loop, events, app state, browser, selection)
├── planning/                 ← context, status, master-plan, harness.json, decisions/, specs/
└── reference/                ← upstream zemse/hackmd source (excluded from workspace)
```

## Keybindings

### Keyboard — Reader mode

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
| `Backspace` | Return to file browser |
| `q` / `Ctrl-C` | Quit |

### Keyboard — Browser mode

| Key | Action |
|---|---|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Enter` | Open file or descend into directory |
| `Backspace` | Ascend to parent directory |
| `q` / `Ctrl-C` | Quit |

### Mouse — Reader mode

| Gesture | Action |
|---|---|
| Scroll wheel | Scroll up / down (3 lines per tick) |
| Hover | Highlight link under cursor |
| Click link | Follow link (local file or browser URL) |
| Click checkbox | Toggle checkbox visual state |
| Click + drag | Select text; releases copy selection to system clipboard (arboard) |
| Double-click | Select word under cursor (450 ms window); copies to system clipboard |

### Mouse — Browser mode

| Gesture | Action |
|---|---|
| Scroll wheel | Scroll the entry list up / down |
| Click entry | Select and immediately open the file or descend into the directory |

## Documentation

### Technical docs

| Doc | Contents |
|---|---|
| [docs/architecture.md](docs/architecture.md) | Two-crate design, render pipeline, event loop, coordinate system, Mode model |
| [docs/modules.md](docs/modules.md) | Per-module reference: purpose, key types, public functions |
| [docs/development.md](docs/development.md) | Prerequisites, build/test/lint steps, adding keybindings, SDLC pipeline |
| [docs/features.md](docs/features.md) | All keybindings and mouse gestures with internal descriptions |

### Planning docs

| Doc | Contents |
|---|---|
| [planning/context.md](planning/context.md) | Orientation + governing principles |
| [planning/master-plan.md](planning/master-plan.md) | Strategy + phase specifications |
| [planning/status.md](planning/status.md) | Current progress |
| [planning/harness.json](planning/harness.json) | SDLC validation/UI-test config (see `harness.examples.md`) |
| [planning/decisions/index.md](planning/decisions/index.md) | Architectural decision records (D1–…) |

## Roadmap / Known limitations

Markdown parsing already runs off the TUI event loop on a worker thread, so large files stay responsive. Planned work:

- **Editor mode:** Reactivate edit-sync and add full mouse support for editing.
- **Config & theming:** Live reload, user configuration, and themes.
- **Console absorption:** Bella is planned to fold into the unified `bastion` Console binary (`bastion bella`) rather than remain a standalone app.

---

*Initialized 2026-06-24 from `base-template` (commit `45bda73d575ceba2ae0216f67a10a5334de3f5b4`).*
