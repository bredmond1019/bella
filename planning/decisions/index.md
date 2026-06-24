---
type: Index
title: Bella Decisions Registry
description: Index of atomic, append-only architectural decision records for Bella.
---

# Decisions Registry

Architectural decision records (ADRs) for Bella. Each decision is **one atomic
file**, append-only — never edit a settled decision; supersede it with a new one and link back.

## Decisions

- [D1: Initial OKF Scaffold](./D1-initial-okf.md) — Project initialized on the standard OKF
  documentation structure.
- [D2: Two-crate split — attributed engine + original app shell](./D2-engine-app-crate-split.md) —
  Cargo workspace: `bella-engine` (vendored/attributed render engine, MIT-derived from
  zemse/hackmd) + `bella` (original binary). Reuse the engine, rewrite the plumbing.

<!-- Add a row per decision as they are made. Record new ones with /log-decision-style atomic
     files (D2, D3, …). -->
