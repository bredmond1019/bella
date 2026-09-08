---
type: Reference
title: Bella Related Demo
description: A fixture whose related list exercises all three doc_id resolution outcomes the rail's Metadata section can show (BE.7.G task 3).
status: active
keywords: [related, doc_id, rail, resolution]
related: [fx-res, fx-mis, fx-amb]
---

# Bella Related Demo

A single document whose `related:` frontmatter list carries three
`doc_id`s that resolve to three different outcomes against the real,
whole-fleet doc_id index BE.7.G builds:

- `fx-res` — claimed by exactly one document
  (`related_resolved_target.md`, this same directory) — Resolved.
- `fx-mis` — claimed by no document anywhere in
  the corpus — Unresolved.
- `fx-amb` — claimed by two documents
  (`related_ambiguous_a.md` and `related_ambiguous_b.md`, this same
  directory) — Ambiguous.

## Body

This body exists only so the document isn't a bare frontmatter block.
