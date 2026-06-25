---
type: Attribution
title: bella-engine Attribution
description: Records the MIT derivation of bella-engine from zemse/hackmd.
---

# Attribution — bella-engine

`bella-engine` is a derivative work based on the TUI rendering subgraph of
[zemse/hackmd](https://github.com/zemse/hackmd), commit `7650cdc`, licensed MIT.

The following source files were ported and adapted from `zemse/hackmd @ 7650cdc`
(`src/tui/`): `markdown.rs`, `links.rs`, `syntax.rs`, `theme.rs`, `palette.rs`,
`md_config.rs`. The geometry pure-function module (`geometry.rs`) was extracted
from `src/tui/events.rs` in the same upstream, refactored to remove `App`
dependencies.

Changes from upstream:
- Import paths flattened: `crate::tui::X` → `crate::X`
- `app`/`events`/`ui`/`cloud` modules not included (engine-only boundary)
- `geometry.rs`: `body_pos` and `select_word_at` lifted as pure standalone
  functions; `View::Cloud` scroll path dropped; clipboard/status/dict
  side-effects deferred to Block D
- macOS dictionary lookup (`dict.rs`) dropped entirely from Bella

Upstream license: MIT — see `LICENSE` for full text.
