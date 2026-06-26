---
type: Index
title: Docs Index
description: Navigation index for Bella's docs/ directory.
doc_id: bella-docs-index
layer: [console]
project: bella
status: active
keywords: [bella docs, navigation index, architecture, modules, development, features]
related: [architecture, modules, development, features, bella-workflows-index]
---

# Docs — Bella

| Doc | Contents |
|---|---|
| [architecture.md](architecture.md) | Two-crate design, render pipeline, event loop, coordinate system, Mode model |
| [modules.md](modules.md) | Per-module reference: purpose, key types, public functions |
| [development.md](development.md) | Prerequisites, build/test/lint steps, adding keybindings, SDLC pipeline |
| [features.md](features.md) | All keybindings and mouse gestures with internal descriptions |

## SDLC workflows (docs/workflows/)

| Doc | Contents |
|---|---|
| [workflows/index.md](workflows/index.md) | Engine ladder overview + committed-state model |
| [workflows/sdlc-run.md](workflows/sdlc-run.md) | `sdlc-run` — full spec, in-place on main |
| [workflows/sdlc-task.md](workflows/sdlc-task.md) | `sdlc-task` — lean single-unit implement→test→fix→commit |
| [workflows/sdlc-flow.md](workflows/sdlc-flow.md) | `sdlc-flow` — shared worktree, per-task loop, one PR |
| [workflows/sdlc-block.md](workflows/sdlc-block.md) | `sdlc-block` — block-level roadmap orchestrator, branch train |
| [workflows/commands.md](workflows/commands.md) | Ad-hoc planning + utility commands reference |
