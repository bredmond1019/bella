---
type: Reference
title: Bella Metadata Demo
description: A fixture whose frontmatter exercises the metadata rail pane — one Scalar, one List, and one Raw value, plus a value long enough to force truncation in the rail.
status: active
keywords: [metadata, rail, frontmatter, truncation]
summary: >-
  A folded block-scalar value, which the restricted frontmatter reader
  cannot classify as a scalar, list, or inline array and so keeps verbatim
  as Raw text for the metadata pane to render as-is.
---

# Bella Metadata Demo

A single document whose OKF frontmatter exercises the metadata rail pane
(BE.7.F): a bare scalar (`type`), an inline list (`keywords`), and a folded
block scalar (`summary`) that the restricted frontmatter reader keeps as
`Raw` text. `description` is deliberately long enough to force the rail's
truncation-with-ellipsis path at the default rail width.

## Body

This body exists only so the document isn't a bare frontmatter block —
the metadata pane renders from parsed frontmatter, not from the body, but a
realistic fixture has both.
