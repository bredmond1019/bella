---
type: Index
title: Docs Index
description: Navigation index for Bella's docs/ directory, grouped by what you are trying to do.
doc_id: bella-docs-index
layer: [console]
project: bella
status: active
keywords: [bella docs, navigation index, capabilities, architecture, development]
related: [capabilities, architecture, modules, development, features, bella-workflows-index]
---

# Docs — Bella

Start with [`capabilities.md`](capabilities.md) if you want to *use* bella, and
[`architecture.md`](architecture.md) if you want to *change* it.

> **A note on `planning/` paths.** These docs sometimes cite files under `planning/` — decision
> records like `planning/decisions/D3-bella-engine-shared-with-bastion.md`, or the validation
> config `planning/harness.json`. **That directory is not part of this repository.** It is a
> symlink into a private planning vault and is gitignored, so a clone of bella will not contain
> it. Those citations are provenance for maintainers, not links you can follow. Everything you
> need to build, test and change bella lives in `docs/`, `scripts/` and `crates/`.

## Using bella

| Doc | One line |
|---|---|
| [capabilities.md](capabilities.md) | Every capability and how to invoke it — derived from source |
| [features.md](features.md) | The same gestures, each traced to its `Action` and `App` method |

## Changing bella

| Doc | One line |
|---|---|
| [architecture.md](architecture.md) | Crate split, render pipeline, async render worker, event loop, modes |
| [modules.md](modules.md) | Per-module purpose, key types, and public functions |
| [development.md](development.md) | Build, test, lint, the three-tier visual regression harness, and how to add a keybinding |

## Running the SDLC harness

The automated pipelines that drive a spec from `tasks.md` to merged code.

| Doc | One line |
|---|---|
| [workflows/index.md](workflows/index.md) | Which engine to reach for, and the shared committed-state model |
| [workflows/sdlc-task.md](workflows/sdlc-task.md) | One small unit: implement → test → fix → commit |
| [workflows/sdlc-flow.md](workflows/sdlc-flow.md) | A full spec on one branch, ending in a PR |
| [workflows/commands.md](workflows/commands.md) | The ad-hoc planning and utility slash commands |

> **Only two engines are installed in this repo** — `sdlc-task` and `sdlc-flow`. The `sdlc-run` and
> `sdlc-block` engines described in the ladder exist in `base-template` but have no `.js` engine and
> no command file here, so they cannot be invoked from bella.
