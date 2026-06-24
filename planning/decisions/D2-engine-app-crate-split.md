---
type: Decision
title: "D2: Two-crate split — attributed engine + original app shell"
description: Bella is a Cargo workspace with a vendored/attributed render engine crate and an original binary crate; reuse the engine, rewrite the plumbing.
---

# D2: Two-crate split — attributed engine + original app shell

**Date:** 2026-06-24
**Status:** Accepted
**Supersedes:** —

## Context

Bella is a local-only, mouse-driven terminal markdown viewer. Two reference repos exist locally:
`../potential-projects/hackmd/` (feature-complete TUI but cloud-coupled — a 17k-line **MIT fork
of `github.com/zemse/hackmd`**) and `../potential-projects/md-tui/` (clean viewer, no mouse, no
editor). Two goals constrain the build: (1) it must be **portfolio-grade and defensibly Brandon's**,
not a wholesale fork of someone else's code; (2) it ships **standalone first**, absorbing into
bastion later.

Verified facts driving the decision:
- The reusable render engine is a **closed, cloud-free subgraph**: `markdown.rs → {links.rs
  (std-only), syntax.rs → palette.rs, theme.rs → palette.rs}`. `markdown.rs` has **zero**
  references to `App`/`Reader`/`events`/`ui`; `render_with_edit()` takes a local `EditCtx`, not
  app state. → ~2,900 lines extract cleanly into their own crate.
- The cloud coupling is concentrated in the plumbing we'd rewrite anyway: `app.rs` (213 cloud
  refs), `events.rs` (54), `ui.rs` (22).
- Versions align with bastion (ratatui 0.30, crossterm 0.29, edition 2024) — a future absorption
  is clean, not a port.

## Decision

Build Bella as a **two-crate Cargo workspace**:

1. **`bella-engine`** — the render/layout/hit-test/geometry core, **ported** from hackmd's
   cloud-free subgraph (the clean trio + `theme`/`palette`/`md_config` + a new pure `geometry.rs`
   lifting `body_pos`/`select_word_at`). Vendored and **attributed** as MIT-derived from
   `zemse/hackmd @ 7650cdc`. Progressively rewritten over time — but never blocking the app.
2. **`bella`** — the binary (`main`/`app`/`events`/`ui`/`config`), **written fresh** using
   hackmd as reference. All cloud code is dropped by simply not writing it; no `CloudNote`/`Cloud`
   enum surgery is needed.

The crate boundary **is** the legal + narrative boundary: everything in `bella-engine` is
attributed-derivative; everything in `bella` is original work.

Scope: **v0.1 = viewer + mouse, no editor.** The engine's `row_source`/`EditCtx`/`BlockInfo`
edit-sync machinery is kept **dormant** (not stripped) so Phase 3 editing needs no byte-range
rederivation. Parser = pulldown-cmark; highlighting = syntect (both inherited via the engine).

## Alternatives considered

- **Strip hackmd wholesale into bastion (RECON Option A).** Fastest to a working tool, but the
  result is mostly someone else's 17k lines, triples bastion's size, and fails the "defensibly
  mine" goal.
- **md-tui base + transplant mouse/editor (Option B).** md-tui's `Word` has no source byte
  positions; retrofitting `row_source` is a larger job than it looks. Rejected.
- **Greenfield rewrite (Option C).** Throws away the one genuinely-hard, genuinely-reusable
  asset (the layout/geometry engine). Rejected as wasteful.

## Consequences

- **Attribution is mandatory:** `bella-engine` carries `LICENSE` (MIT + zemse/hackmd copyright)
  and `ATTRIBUTION.md`; each ported file keeps a 2-line source header. The `bella` crate is
  Brandon's copyright.
- **Honest public narrative:** "I built a mouse-driven terminal markdown viewer; its text-layout
  core started as an attributed MIT engine I've been replacing."
- The "own the engine" goal becomes incremental and non-blocking — rewrite `bella-engine`
  internals without touching the app surface.
- **Upstream snapshots are vendored for reference.** Frozen, gitignored copies of `hackmd`
  (@ `7650cdc`) and `md-tui` (@ `7988c55`) live in `reference/` (excluded from the Cargo
  workspace, never compiled) so porting and "how did upstream do X" questions don't depend on
  the `../potential-projects/` staging area. md-tui is a secondary reference only (fuzzy file
  finder, config schema, `<details>` folding) — not an engine port source. See
  `reference/README.md`.
