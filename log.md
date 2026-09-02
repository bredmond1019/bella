---
type: Log
title: Bella Development Log
description: Chronological log of work completed for Bella.
timestamp: "2026-09-01T19:15:00Z"
---

# Log — Bella

*Append-only working log. One dated entry per session. Newest entries at the top.*

---

## [run: 2026-09-02]

Closed `BE.7.D` (scroll anchoring across re-render) via `/sdlc-flow`; all 5 tasks passed, review verdict PASS. Task 1 added additive `display_row_to_source_line`/`source_line_to_display_row` lookups to `bella-engine`'s `markdown.rs`, built on `Rendered::blocks` with block-granular linear interpolation, tested in both directions including a source line spanning several display rows after wrapping. Task 2 replaced `app.rs`'s clamp-only handling of `render()`'s `scroll` field with a real anchor resolve/restore: a new `pending_scroll_anchor` field and `blocks_as_rendered()` helper let a resize re-resolve the source-line anchor after the real (unstubbed) render worker delivers, including the case where a second resize fires before the first render lands — the async race this block exists to fix, not a synchronous-only fix. Task 3 rewrote `history.rs`'s `HistoryEntry` to store a `usize` source-line anchor instead of a raw `u16` display index (back/forward now restore via the same resize-survival path), added a `RESIZE:<cols>x<rows>` pseudo-key to `capture_scenes.sh` for VHS scenes that must exercise a real terminal resize, and — after an interim bail on an incomplete vault commit — re-captured two VHS reference screenshots that had gone blank under concurrent-lane CPU contention and widened marginal post-keystroke settle times in both reference tapes to reduce recurrence. Task 4 confirmed bella-engine's two new public functions are additive-only and ran the real cross-repo bastion gate (`cargo build`/`cargo test`, 2713 passed). Task 5 confirmed the full authoritative validation suite (fmt, clippy, `cargo test` via nextest-policy override, release build, test-layout, three consecutive `check_scenes.sh` runs, VHS freshness) green with no further changes needed. `BE.7.E` (horizontal frame + body-width single writer + TOC rail) is now the next layout block in sequence.

```
324434b docs: update docs for BE.7.D
c4c434b docs: note the VHS settle-timing fix in scenes.toml (BE.7.D-task3)
6a95dbc fix: fix pass 1 for BE.7.D-task3
1d83b56 feat: implement BE.7.D-task3
ee46931 feat: implement BE.7.D-task2
a79b304 feat: implement BE.7.D-task1
```

Next: `/sdlc-flow BE.7.E` (horizontal frame + body-width single writer + TOC rail).

---

## [run: 2026-09-02]

Closed `BE.7.C` (walker: symlinks, hidden/ignored reveal, corpus-root rule) via `/sdlc-flow`; all 5 tasks passed, review verdict PASS. Task 1 set `WalkBuilder::follow_links(true)` on `Browser::build_entries` (fixing the long-standing symlinked-child-entry drop that hid `planning/` in every repo of this fleet) and added a `reveal_ignored` field/setter relaxing both `hidden(true)` and `git_ignore(true)` together — plus `build_entries` now returns a dropped-entry count instead of silently swallowing walk errors. Task 2 added `resolve_corpus_root` (invoked path → nearest `brain.toml` ancestor → git root → invoked path) and stored it as `App.corpus_root` at startup. Task 3 bound reveal-toggle to `r` in browser mode, surfaced reveal state and the dropped-entry count in the browser status line, and refreshed the scene/VHS baselines for the new browser behaviour (14 reference PNGs touched, most re-saved pixel-identical with a PNG tEXt freshness stamp; one genuine new capture). Task 4 ran the real cross-repo bastion gate (`cargo build`/`cargo test`, 2713+4+1+7 tests green) confirming all five `Browser::new` call sites keep default filtering unchanged. Task 5 confirmed the full authoritative validation suite (fmt, clippy, `cargo test` via nextest-policy override, release build, test-layout, scenes, VHS freshness) green with a clean tree. One deviation from the spec as written: task 3 required editing `crates/bella/src/ui.rs` (not in task 3's declared `files[]`) because browser mode never renders through `App::status_message`, so surfacing reveal state and the dropped-entry count visibly required touching the draw function directly. `BE.7.D` (scroll anchoring across re-render) is now the next layout block in sequence.

```
7b2f205 docs: update docs for BE.7.C
c498c46 fix: lengthen two VHS scene settle times against measured blanking
1387815 feat: implement BE.7.C-task3
0b4ce99 feat: implement BE.7.C-task2
dcb96ea feat: implement BE.7.C-task1
```

Next: `/sdlc-flow BE.7.D` (scroll anchoring across re-render).

---

## [run: 2026-09-02]

Closed `BE.7.L` (visual regression harness) via `/sdlc-flow` resume; all 6 tasks passed, review verdict PASS. Picking up from the prior bail after task 3 (stale VHS reference PNGs), task 4 captured verbatim evidence that both `check_scenes.sh` and `check_vhs_fresh.sh` actually go red on a real drawn-output regression, a known-bad capture, and a simulated stale reference set, then confirmed the tree stays clean. Task 5 added tape/manifest parity checking to `scripts/check_scenes.sh` (demonstrated failing, then reverted), pinning headers to both reference tapes, a three-tier rewrite of `planning/artifacts/screenshots/README.md`, and the two new scene commands to `CLAUDE.md`'s Build/test/run block. Task 6 ran the full authoritative validation suite — fmt, clippy, `cargo test` (via `NEXTEST_POLICY_OVERRIDE`), release build, `check_test_layout.sh`, `check_scenes.sh`, `check_vhs_fresh.sh` — all 8 checks green with no further code changes needed. Notable decision from the earlier task 3 resolution: 6 of 11 regenerated reference PNGs came back byte-identical to their prior blobs, so each was stamped with a PNG tEXt chunk recording the source commit it was verified against, forcing a real (pixel-identical) blob change that lets the git-commit-time freshness gate re-fire honestly. `BE.7.C` (walker: symlinks, hidden/ignored reveal, corpus-root rule) is now the next layout block in sequence.

```
845e5e7 docs: update docs for BE.7.L
779a0af feat: implement BE.7.L-task5
ad2e781 fix: bump VHS reference tape settle time to fix corrupt browser captures
```

Next: `/sdlc-flow BE.7.C` (walker: symlinks, hidden/ignored reveal, corpus-root rule).

---

## [run: 2026-09-02]

`BE.7.L` (visual regression harness) BAILED via `/sdlc-flow` after task 3 of 6 — task 1 added `scripts/vhs/scenes.toml` (11-scene manifest) and `scripts/capture_scenes.sh`, driving the real release `bella` binary through plain tmux (no `bastion` dependency) and writing capture-pane text to `tests/scenes/`. Task 2 added `scripts/check_scenes.sh` (re-capture + diff against the committed baselines, with a distinct hard-error path for blank/near-empty captures) and registered `scenes` as a gating `validation.checks[]` entry, committing the 11 text baselines. Task 3 built `scripts/check_vhs_fresh.sh` — sanity (byte floor + non-blank companion text scene) and git-commit-time freshness on the reference PNGs, resolving each side's commit time in its own repo since `planning/` is a symlink into a separate vault — and registered it as the `vhs-fresh` check; it currently and correctly reports 10 of 11 reference PNGs stale, captured against commit 273c486 before `BE.7.A`'s `b6b5c71` touched `crates/bella-engine/src/markdown.rs` (frontmatter stripping). The gate's own logic verified sound (sanity passes all 11 PNGs, mtime-immune, bastion-absence positive control passes); what remains is regenerating the reference PNG set, which is out of this task's declared scope (files: `check_vhs_fresh.sh`, `harness.json`) and reproducibly failed in this sandbox — two `vhs` capture attempts produced corrupted/near-empty PNGs for several scenes, most likely from resource contention with other concurrently-running agent lanes on this machine; both attempts were fully reverted rather than committed. Tasks 4-6 (reference-tape sync, docs, and final validation) did not run. Next: regenerate the VHS reference PNG set in a quieter environment or its own dedicated block, then resume `BE.7.L` from task 4.

```
be60716 feat: implement BE.7.L-task3
1b94108 feat: implement BE.7.L-task2
03fc43b feat: implement BE.7.L-task1
```

---

## [run: 2026-09-02]

Closed `BE.7.B` (browser resize fix + structural golden buffer) via `/sdlc-flow`; all 4 tasks passed, review verdict PASS. Task 1 added `crates/bella/tests/it/golden_draw.rs` (plus its `mod` line in `tests/it/main.rs`) — TestBackend structural assertions over `draw_reader`/`draw_browser` at three terminal sizes, pinning region widths, x-offsets, pane boundaries and the status-row's position, deliberately never cell contents, so the buffer stays a regression gate rather than a source of cosmetic churn. Task 2 fixed the live off-by-one: `events.rs`'s `Event::Resize` arm now reads the existing single source `app.browser_area.height` instead of recomputing `height.saturating_sub(2)`, extracted as a testable `reclamp_browser_scroll_on_resize` so both the crate's own unit tests and the external `tests/it` binary can exercise it; the negative control (temporarily reverting to the old computation) was run manually and confirmed the golden-draw resize test fails without the fix, then fully restored before committing. Task 3 re-captured `planning/artifacts/screenshots/narrow_reader_demo_table.png` via a scoped, disposable VHS tape — it now shows bella's actual reader render (110546 bytes) instead of a bare shell prompt (was 20326 bytes); committed from the HQ vault repo since the file lives under the `planning/` symlink. Task 4 ran the full validation suite (fmt, clippy, `cargo test` via the designated full-suite override, release build, `check_test_layout.sh`) clean with no further code changes. `BE.7.L` (visual regression harness) is now the next layout block in sequence.

Next: `/sdlc-flow BE.7.L` (visual regression harness — scripted terminal scenes + VHS freshness gate).

```
050c038 docs: update docs for BE.7.B
1a99f40 fix: resize off-by-one browser clamp + failing golden test
e0f0084 feat: implement BE.7.B-task1
```

---

## [run: 2026-09-01]

Resumed and closed `BE.7.A` (frontmatter strip + restricted OKF parse) via `/sdlc-flow`; all 5 tasks passed, review verdict PASS. The prior fmt bail (task 5's `cargo fmt --check` failing on `crates/bella-engine/tests/it/frontmatter.rs`, unformatted since task 3) was cleared by a scoped, task-independent commit (`74d6950`) that ran `cargo fmt` on only that file — whitespace-only, no assertion or test-name changes. Task 5 then re-ran clean: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (via `NEXTEST_POLICY_OVERRIDE=1`, this task's designated full-suite validation), `cargo build --release`, and `scripts/check_test_layout.sh` all passed with zero further code changes. Docs updated (`docs/modules.md`, `docs/capabilities.md`, `docs/development.md`) to reflect the frontmatter module. Net result: `bella-engine` now strips OKF frontmatter as a pre-pass before `pulldown-cmark`, carries a parsed `Frontmatter` on `Rendered`, and corrects `blocks[].source_range`/`row_source[]`/`EditCtx.cursor` back into original-file byte space — clearing the live rendering defect (bogus setext-H2 from the YAML close fence) on every OKF document in this fleet. Cross-repo evidence (bastion `cargo build`/`test` green, real `tui_capture.sh` capture) was recorded in task 4. `BE.7.B` is now the only remaining startable layout block.

Next: `/sdlc-flow BE.7.B` (browser resize fix + structural golden buffer).

```
0919bb5 docs: update docs for BE.7.A
74d6950 style: cargo fmt crates/bella-engine/tests/it/frontmatter.rs
26f5a99 chore: wrap up BE.7.A
55ff6ba feat: implement BE.7.A-task3
```

---

## [run: 2026-09-01]

Ran `BE.7.A` (frontmatter strip + restricted OKF parse) via `/sdlc-flow`; tasks 1-4 passed, task 5 (Validate) BAILED. Task 1 added `crates/bella-engine/src/frontmatter.rs`, a restricted four-shape hand-rolled OKF reader (bare scalar, quoted scalar, inline array, block list; anything else retained as Raw), 15 unit tests. Task 2 wired the strip into `render_with_edit` as a pre-pass before pulldown-cmark, carried the parsed `Frontmatter` on `Rendered`, and translated `blocks[].source_range`/`row_source[]`/`EditCtx.cursor` back into original-file byte space via a single delta — verified with a live negative control (deleting the correction fails the byte-offset fixtures). Task 3 added integration fixtures in `crates/bella-engine/tests/it/frontmatter.rs` proving `headings[0]` is the real H1 (not the setext-H2 misparse), the two okf-core-rejected shapes this corpus contains (blank line, `description: >-`), and line-index agreement across a stripped document. Task 4 recorded the un-gateable cross-repo evidence: bastion `cargo build`/`cargo test` both green against the new reader, and a real `tui_capture.sh` run confirms the H1 renders as the first body row. Task 5 (`cargo fmt --check`) then failed on `crates/bella-engine/tests/it/frontmatter.rs`, which task 3 had committed unformatted — confirmed by re-running `cargo fmt --check -p bella-engine` directly against HEAD `55ff6ba`, a clean tree with no task-5 changes: exit 1, same 4 diff hunks at lines 68/106/152/216. Task 5's `files[]` is empty, so the only fix (running `cargo fmt` on that file) is out of scope for this task per standing rule 3a; the agent confirmed the fix resolves every remaining gate (clippy/nextest/test/build/check_test_layout.sh) but reverted it via `git reset --hard HEAD~1` to avoid committing outside task 5's declared scope, leaving the tree exactly as task 3 left it. This needs an operator or re-plan decision on which task owns the `cargo fmt` fix before `BE.7.A` can resume.

Next: resolve the fmt-ownership question (either widen task 5's scope to include the fmt fix, or add a task-3.5-style scoped fix), then resume `/sdlc-flow BE.7.A` from task 5.

```
55ff6ba feat: implement BE.7.A-task3
b6b5c71 feat: implement BE.7.A-task2
ca61d7f feat: implement BE.7.A-task1
```

---

## [run: 2026-09-01]

Implemented `BE.7.M` (test layout + collision-proof fixtures) end to end via `/sdlc-flow`, 7/7 tasks passed, review verdict PASS. Consolidated `bella-engine`'s and `bella`'s integration tests into one `tests/it/main.rs` binary per crate (mirroring `mev`/`engine-rs`'s pattern); added `#[cfg(test)]`-only `unique_temp_dir` helpers to both crates and replaced all 19 fixed-name `std::env::temp_dir()` fixture sites (found via measurement to be 19 across 4 files, not the block record's original 8/12 estimate — `app.rs` was missing from the file list entirely) with the collision-proof helper, removing every `remove_dir_all`-before-use pattern; added `scripts/check_test_layout.sh` plus its fixture suite as a gating `harness.json` check to keep the one-binary-per-crate layout from silently regressing; documented the layout rule, the helper, and measured relink timings in `CLAUDE.md`. Notable decisions: the pre-fix concurrency collision was not reproduced live in either baseline or post-fix runs (nextest's per-process isolation and non-deterministic scheduling), so the safety argument rests on removing the structural hazard rather than an observed-then-fixed repro — stated plainly rather than rounded off; relink-timing savings at this block's 2-3 file scale measured as noise (~0.2s), consistent with but not itself demonstrating the larger-scale win the block record claims for ~10 added files; `testsupport` modules confirmed absent from the release binary's symbol table via `nm`/`strings`, not just `cfg(test)` alone. Full validation suite (fmt, clippy -D warnings, the authoritative full-suite gate, release build, the new layout check) all green — 256 tests total. `BE.7.A` and `BE.7.B` are now startable.

Next: `BE.7.A` (frontmatter strip + restricted OKF parse) and `BE.7.B` (browser resize fix + structural golden buffer), runnable concurrently.

```
42aaf38 docs: update docs for BE.7.M
3126fa6 feat: implement BE.7.M-task5
9da850d feat: implement BE.7.M-task4
b3c56c3 feat: implement BE.7.M-task3
858b310 feat: implement BE.7.M-task2
```

## [2026-09-01]

### Ported bastiel's cool-aurora theme, wired the status bar, built visual-QA tooling
- **What:** Built `scripts/tui_capture.sh` (tmux-driven text capture) and wired up VHS
  (`scripts/vhs/*.tape`, pixel-level PNG/GIF capture) since no tooling existed to visually verify
  a ratatui TUI. Wrote a calibrated `polish-standard.md`, fixed its actionable findings as two
  tickets (browser status bar + viewport, in-app keybinding hint — both closed). Ported bastiel's
  cool-aurora palette onto `Theme::dark()`, and found + fixed that the status bar never read the
  theme at all (`Color::Black`/`Color::White` hardcoded, `status_fg`/`status_bg` were dead
  fields) — added `App.theme` and regression tests. Captured a reference screenshot set at
  `planning/artifacts/screenshots/`. Found and recorded a real markdown rendering bug (a list
  item whose only child is a singleton nested sublist collapses onto one line) — not fixed,
  carried in `state.json` and `CLAUDE.md`. Closed out with the full gating suite, a coverage fill,
  and a doc-health sweep (`docs/features.md`, `architecture.md`, `modules.md`, `development.md`
  no longer say "Catppuccin" for the live theme).
- **Why:** Operator wanted bella's visual identity aligned with `business/bastiel` (the practice's
  flagship web app) and asked for a durable way to visually verify the TUI going forward, since
  reviewing a terminal app currently required either a human at a real terminal or trusting
  automated tests that only assert cell content, not appearance.
- **Refs:** `planning/polish-standard/polish-standard.md`, `planning/artifacts/screenshots/README.md`,
  bella commits `4946698`..`2b4e088`.

---

## [2026-08-29]

### Planned the modeless mouse-driven editor (BE.6) — 3 blocks, lane, HQ roadmap
- **What:** Authored the `modeless-editor` initiative: `BE.6.A` (edit-mode toggle + cursor,
  read-only, plus the missing engine tests), `BE.6.B` (mutation + atomic save), `BE.6.C`
  (selection, clipboard, undo/redo). Registered all three in `state.json` under a new Phase 6,
  wrote the lane record, and moved it into HQ's `planning/roadmaps/modeless-editor/`.
- **Why:** The operator wants to edit markdown in place with the mouse and arrow keys — not vim
  (modal controls they dislike) and not VS Code (heavy; now used only to read markdown). The
  existing `BE.3.H` spec is a port of hackmd's **modal vim-style** editor (`:w`/`:wq`/`:q!`), so it
  was superseded rather than salvaged and stays `wontfix`. Re-specced modeless, this is *cheaper*
  than the vim port — the whole command-line and motion layer drops out.
- **The finding the cut is built around:** `bella-engine`'s edit path (`EditCtx`, `row_source`,
  `cursor_xy`, `substitute_inline_at_cursor`, `make_raw_block`) is fully written with **zero tests
  and zero production call sites** — the edit branch has never executed, and it shapes `Rendered`,
  a cross-repo contract with bastion. `BE.6.A` proves it before anything is spent on top, and an
  operator gate (`OP.editor-go-no-go`) sits between block 1 and 2 so the chain stops for a
  fund/don't-fund call on real evidence.
- **Two corrections found while doing it:** `/plan --lane` writes the lane record to the repo's
  `planning/<slug>/`, but `/begin-orchestration` resolves a roadmap slug against **`BRAIN_ROOT`** —
  so it would not have resolved; moved to HQ. And an HQ doc citing a bella `doc_id` needs the
  `bella:` prefix (caught as `E_GRAPH_DANGLING_RELATED`).
- **Refs:** `planning/modeless-editor/plan.md`, `planning/blocks/BE.6.*.json`,
  `agentic-portfolio/planning/roadmaps/modeless-editor/`

### Docs cleanup pass — capability catalogue + source-contradicted claims corrected
- **What:** Added `docs/capabilities.md` (every capability and how to invoke it, derived from the
  clap definition, the `Action` enum and the `pulldown-cmark` handlers — not from doc titles).
  Rewrote `docs/index.md` to one-line task-grouped rows. Added Quickstarts (1 -> 3 of 5) and
  mermaid diagrams (0 -> 2), plus plain-English openers on architecture/development/features/
  modules. Commit `7118dbb`.
- **Why:** A `write-repo-doc` pass. Five defect classes, and the docs asserted features the binary
  does not have: `theme::resolve`, `detect_terminal_theme`, `Theme::light`,
  `Theme::mission_control` and `md_config::load` have **zero call sites** — every render site calls
  `Theme::dark()`, so bella is dark-only and `config.toml` does nothing. README, `features.md` and
  `modules.md` all described `COLORFGBG` auto-detection and a working config file. Also fixed:
  `browser.rs` documented in the wrong crate; a config path stated as `~/.config/md/` when
  `dirs::config_dir()` makes it platform-dependent; `docs/index.md` advertising `sdlc-run` and
  `sdlc-block`, which have no engine or command file here; and a link into the gitignored
  `planning/` vault that 404s on GitHub.
- **Refs:** `docs/capabilities.md` section "Not wired up"

---

## [2026-08-27]

### Build/security cleanup — no feature work
- **What:** Removed the dead `rustc-wrapper = "sccache"` from `.cargo/config.toml` (0 cache hits
  measured); added workspace-root `[profile.dev]` (`line-tables-only` + unpacked split-debuginfo).
  `cargo clean` reclaimed 1.9GiB (`target/` 1.8G -> 1.1G). Then applied four RustSec fixes via plain
  `cargo update -p` (no `Cargo.toml` edit): `crossbeam-epoch` (`RUSTSEC-2026-0204`), `lru`
  (`RUSTSEC-2026-0253`), `anyhow` (`RUSTSEC-2026-0190`), and `quick-xml` 0.39.4->0.41.0 (the real
  finding — `RUSTSEC-2026-0195`/`0194`, 7.5/high DoS on crafted XML/plist input) by bumping `plist`
  1.9.0->1.10.0, staying inside `syntect`'s existing `plist = "^1.3"` range. `cargo audit` is now
  clean except two unmaintained warnings (`bincode`, `yaml-rust`, both via `syntect`, no patched
  version exists upstream) — tracked in HQ's `docs/rust-dependency-audit.md`.
- **Why:** Same fleet-wide build-speed/security pass run across every `core/*` Rust repo this
  session; `bella` pulls in `bella-engine`'s `syntect` dependency, which is what surfaced the
  high-severity `quick-xml` finding.
- **Refs:** commits below, HQ's `docs/rust-dependency-audit.md` and `docs/infrastructure.md`'s
  "Rust build artifacts" section.

```
736cb58 perf(build): remove dead sccache wrapper, add profile.dev link-time fix
d6d3aac security(deps): bump crossbeam-epoch, lru, anyhow, plist for RustSec fixes
```

## [2026-07-07]

### Shipped ticket-async-markdown-render — background render worker, non-blocking event loop
- **What:** Ran `/sdlc-task` on the ticket `ticket-async-markdown-render` (`planning/ticket-async-markdown-render/tasks.md`) — moved bella's synchronous markdown parse/render off the event loop onto a background `std::thread` (new `crates/bella/src/render_worker.rs`), added a `Loading`/`Ready` `RenderState` to `App`, and switched `run_loop` to a non-blocking 50ms poll loop instead of blocking on `event::read()`. This was queued as a portfolio-release high-value item (item A2) to unblock the TUI so large files no longer freeze it. All 5 tasks passed (commits `13865db`, `bb503b8`, `affca17`, `fbae58a`, `66ce188`). Then ran a full `/close-out`: gating suite green (fmt/clippy/test/build --release + emoji gate), coverage scan found no blocking gaps, low-level code review found no issues, and docs were patched (`docs/development.md`, `docs/modules.md`) to cover the new `render_worker.rs` module and `App` fields; `docs/architecture.md`'s Render Pipeline/Event Loop sections were flagged NEEDS_REVIEW (architecture-level, not surgically fixed).
- **Why:** Shipped ticket-async-markdown-render (background render worker, non-blocking event loop) via `/sdlc-task`; closed out with full gating suite, coverage check, code review, and doc patch — the TUI blocked on synchronous render for large files, which was queued as high-value portfolio-release item A2.
- **Refs:** `planning/ticket-async-markdown-render/tasks.md`, `crates/bella/src/render_worker.rs`, `crates/bella/tests/render_async.rs`

---

## [2026-07-04]

### BE.4.A — Hide HTML comments in bella-engine render
- **What:** `bella-engine`'s markdown render pipeline now explicitly drops raw HTML and HTML comments (`<!-- ... -->`) — `Event::Html`/`Event::InlineHtml` produce no output, so sentinel comments in status/spec docs never surface as literal rendered text. Shipped with a dedicated regression test file `crates/bella-engine/tests/html_comments.rs` (2 tests). Delivered via the `/sdlc-task` lean engine (implement → fast-test → commit, in-place on `main`; both tasks passed 2/2), then `/close-out`: full gating suite (cargo fmt/clippy/test/build) all green, 52+ tests pass, and `docs/modules.md` patched (corrected `markdown.rs` line count 2556→2561; noted the HTML/comment drop behavior).
- **Why:** Load-bearing for the `bella-engine` ↔ `bastion` cross-repo contract (CLAUDE.md rule 6 / D3) — sentinel HTML comments embedded in docs must not leak into the visible render in bastion's TUI.
- **Refs:** `planning/BE.4.A/tasks.md`, `crates/bella-engine/tests/html_comments.rs`

---

## 2026-06-25 — Merged PR #3; added docs/ (architecture, modules, development, features)

Merged Block E PR #3 after resolving rebase conflict and browser cursor/scroll edge cases. Added comprehensive `docs/` directory: `architecture.md` (two-crate workspace, load-bearing render/layout isolation, v0.1 scope + Phase 3 dormant edit machinery), `modules.md` (15 core modules from geometry/palette/links/syntax through app/events/ui/browser), `development.md` (build/test/run commands, Rust edition 2024, clippy gates), `features.md` (port scope from hackmd derived source + original app shell). All changes committed and pushed. Working tree clean. All 237 tests passing; next: Phase 2 Block F (config + themes + live reload).

```
Working tree clean — all changes committed and pushed
```

---

## 2026-06-25 — Phase 1 Block E complete: file browser (5 tasks, 237 tests, PASS)

Completed `/sdlc-flow` for Block 1.E — file browser (directory navigator). All 5 tasks implemented and passed on first attempt: (1) `browser.rs` with `BrowserEntryKind`/`BrowserEntry`/`Browser` model, gitignore-aware listing via the `ignore` crate (`max_depth(1)`, hidden dotfiles skipped), cursor wrap+scroll-clamp, and 14 unit tests; (2) `Mode` enum and browser state in `App` (`new_browser`, `open_from_browser`, `back_to_browser`, `enter_dir`, `ascend`), CLI dispatch changed to `Option<PathBuf>` (no-arg/dir/file paths); (3) `draw_browser` in `ui.rs` with bordered pane, dir title, `▶ ` selection prefix, bold-cyan Dir/ParentDir vs plain Markdown styling, and `browser_area` stored for mouse hit-testing; (4) browser key/mouse handlers (`map_browser_key`, `map_browser_mouse`, new `Action` variants), `apply` wired to App methods, `run_loop` mode-aware dispatch, and all `#[allow(dead_code)]` guards removed; (5) full validation — all four gating checks (fmt, clippy, 237 tests, release build) pass. PASS review verdict on all acceptance criteria. Notable decisions: `BrowserClickAt` single-click selects and immediately descends/opens; mouse scroll moves the viewport offset directly; Backspace in reader mode maps to `back_to_browser`; `require_git(false)` on the `ignore` WalkBuilder ensures `.gitignore` is honoured outside git repos. Next: Phase 2 Block F — Config + themes + live reload.

```
d4ba5cf chore: flow state — docs
8ef57cc docs: update docs for 1.E-file-browser
b04f520 chore: flow state — task 5 passed
4c1e5d9 chore: flow state — task 4 passed
9982256 feat: implement 1.E-file-browser-task4
68aab2b chore: flow state — task 3 passed
cddf877 feat: implement 1.E-file-browser-task3
7159872 chore: flow state — task 2 passed
```

---

## 2026-06-25 — Code review: Block D PR #2 fix and close-out

Code review of PR #2 (Block D mouse support) identified a subtle bug in the double-click handler: after a successful double-click word-select, `selection_finish()` was being called twice — once on the second `Down` of the double-click sequence and again on the final `Up`. This caused the selection to be extracted and cleared prematurely. Fixed by guarding the DragEnd branch in `events.rs:307` behind `drag_origin.is_some()`, ensuring `selection_finish()` only runs when a real drag was initiated. Added regression test to catch this scenario. Block D now has 196 tests passing across all crates, all four gating checks (cargo fmt, clippy, test, build --release) exit 0. PR #2 merged; Block D shipped as v0.1.

```diff
planning/handoff.md | 76 ++++++++++++++++++++++++++++++++---------------------
 1 file changed, 46 insertions(+), 30 deletions(-)
```

---

## 2026-06-25 — Phase 1 Block D complete: mouse support (6 tasks, 195 tests, PASS)

Completed `/sdlc-flow` for Block 0.D — mouse support, the v0.1 deliverable. All 6 tasks implemented and passed on first attempt: (1) `selection.rs` module with `Selection` type, `extract_text`, and `copy_to_clipboard` via `arboard`; (2) mouse capture enabled/disabled in terminal setup, teardown, and panic hook, plus scroll-wheel → `ScrollUp`/`ScrollDown` dispatch; (3) click-to-follow links, hover highlight, and checkbox visual toggle using `body_pos` coordinate conversion and a stored `body_area` Rect in App; (4) drag-select with `DragStart`/`DragUpdate`/`DragEnd` action pipeline — selections are highlighted in LightBlue and released to the system clipboard; (5) double-click word-select within a 450 ms window using `bella_engine::select_word_at`, with deterministic timestamp injection for tests; (6) validation gate confirming all four checks (fmt, clippy, 195 tests, release build) pass clean. PASS review verdict on all acceptance criteria. Notable decisions: ClickAt action variant added in Task 3 was removed in Task 4 in favour of a unified DragStart+DragEnd plain-click path; scroll wheel maps to 3 lines per tick. Next: Phase 1 Block E — File browser (directory navigator).

```
3db5208 chore: flow state — docs
e3da5e0 docs: update docs for 0.D-mouse-support
56dd851 chore: flow state — task 6 passed
5224309 chore: flow state — task 5 passed
e17c9fe feat: implement 0.D-mouse-support-task5
e11c201 chore: flow state — task 4 passed
46c0bb2 feat: implement 0.D-mouse-support-task4
bc9bfd3 chore: flow state — task 3 passed
```

---

## 2026-06-25 — Fix: suppress browser-tab side effect in tests

Added a `#[cfg(not(test))]` guard to `open::that(url)` in `crates/bella/src/app.rs` to prevent unintended browser tabs from opening during test runs. This was a hygiene fix found during Block D setup — the link-follow feature from Block C was triggering browser launches in unit tests. Guard is scoped tightly to the side-effect call only, leaving production behavior unchanged.

```diff
crates/bella/src/app.rs | 4 ++++
 1 file changed, 4 insertions(+)
```

---

## 2026-06-25 — Phase 0 Block C complete: keyboard navigation (7 tasks, 136 tests, PASS)

Completed `/sdlc-flow` for Block 0.C — keyboard navigation. All 7 tasks implemented and passed on first attempt: (1) retained link/heading metadata + real base_dir in App, (2) back/forward history stack, (3) link focus ring + Tab/Shift-Tab highlight, (4) link follow via Enter (URLs, local files, anchors), (5) in-document search with `/`, `n`/`N` cycling, (6) history navigation wiring via `[`/`]`, (7) validation. PASS review verdict on all acceptance criteria. Test suite grew to 136 tests total (21 + 37 engine + 78 new); all four gating checks (cargo fmt, clippy, test, build --release) exit 0. PR #1 opened to main. Close-out also patched `CLAUDE.md` (added keybindings section + directory map), and `README.md` (keybindings → Link focus with Tab/Shift-Tab, Enter to follow, `/` search, `[`/`]` history). Block C is now fully shipped; next focus is Phase 0 Block D — Mouse support (scroll/hover/click).

```diff
 CLAUDE.md                                          |    5 +-
 Cargo.lock                                         |   37 +
 README.md                                          |   21 +-
 crates/bella/Cargo.toml                            |    1 +
 crates/bella/src/app.rs                            | 1185 +++++++++++++++++++-
 crates/bella/src/events.rs                         |  384 ++++++-
 crates/bella/src/history.rs                        |  303 +++++
 crates/bella/src/main.rs                           |    1 +
 crates/bella/src/ui.rs                             |  285 ++++-
 log.md                                             |   16 +
 .../sdlc/sdlc-flow-state.json                      |  152 +++
 planning/0.C-keyboard-navigation/sdlc/worklog.md   |   45 +
 planning/0.C-keyboard-navigation/tasks.md          |    6 +-
 planning/status.md                                 |    6 +-
 14 files changed, 2424 insertions(+), 23 deletions(-)
```

---


## 2026-06-25 — Close-out: Block 0.B gating checks + doc patch

Completed `/close-out` for Block 0.B. All four gating checks passed (cargo fmt, cargo clippy, cargo test, cargo build --release) and emoji gate cleared. Coverage verified: 59 tests total (21 bella + 37 engine + 1 integration), all inline #[cfg(test)] blocks across all 4 source files in the bella binary crate (main.rs, app.rs, ui.rs, events.rs). Updated README.md via `/update-docs --patch` to add the `crates/bella/` entry to the directory map. Wrote `planning/handoff.md` marking Phase 0 Block C (keyboard navigation: link focus/follow, `/` search, history) as the next focus. Block B is now fully shipped.

```diff
README.md           |  3 +-
 planning/handoff.md | 96 ++++++++++++++++++++++++++++++++------------------------
 2 files changed, 55 insertions(+), 44 deletions(-)
```

## 2026-06-25 — Phase 0 Block B complete: bella binary skeleton renders a file

Created the `bella` binary crate (4 source modules: `main.rs`, `app.rs`, `ui.rs`, `events.rs`). The clap CLI accepts a required file path; terminal lifecycle uses raw mode, alternate screen, and a panic hook that restores the terminal before re-raising. The `App` struct holds rendered lines and a clamped scroll offset; `draw_reader` splits the frame into a body + 1-row statusline and pushes the body height back to `App` on every draw. The pure `map_key` function maps `j/k`, `g/G`, arrows, PageDown/PageUp, Ctrl-d/u, and `q/Ctrl-C` to actions with no terminal dependency. All 7 acceptance criteria were met on the first review attempt (PASS). 21 new tests pass (scroll clamping, key mapping, TestBackend draw assertions); total suite is 59 tests across both crates. All four gating checks exit 0. Next: Phase 0 Block C — keyboard navigation (link focus/follow, `/` search, history).

```
6eac051 docs: update docs for 0.B-binary-skeleton
e6aa18e feat: implement 0.B-binary-skeleton
765951e chore: add spec for 0.B-binary-skeleton
```

## 2026-06-24 — Close-out: Block 0.A gating checks + doc patch

Completed `/close-out` for Block 0.A. All four gating checks passed (cargo fmt, cargo clippy, cargo test, cargo build --release) and emoji gate cleared. Coverage verified: 38 tests (37 unit + 1 integration). Updated planning docs via `/update-docs --patch` to add `planning/decisions/index.md` to the root README. Wrote `planning/handoff.md` marking Block B as the next focus. Block A is now fully shipped; current focus is Phase 0 Block B (binary skeleton renders a file without mouse).

```diff
README.md | 1 +
 1 file changed, 1 insertion(+)
```

## 2026-06-24 — Phase 0 Block A complete: workspace + bella-engine extraction

Created the Cargo workspace and ported the render/layout subgraph from `zemse/hackmd @ 7650cdc` (MIT) into the new `bella-engine` crate. Six modules were ported (`markdown.rs`, `links.rs`, `syntax.rs`, `theme.rs`, `palette.rs`, `md_config.rs`) with all App/cloud dependencies removed and edit-sync types preserved dormant. A new `geometry.rs` lifted `body_pos` and `select_word_at` as pure functions with explicit parameters replacing the upstream `&App` reads. All eight acceptance criteria passed on the first review attempt (PASS verdict). 38 tests pass (37 unit + 1 integration); all four gating checks (fmt, clippy, test, build --release) exit 0. Next: Phase 0 Block B — `bella` binary skeleton that renders a file without mouse support.

```
8ee949b docs: update docs for 0.A-workspace-engine-extraction
184005a feat: implement 0.A-workspace-engine-extraction
d87e9eb chore: sharpen Task 3 (geometry lift) in 0.A spec — exact pure signatures, deferred side-effects
cd8de1f chore: add spec for 0.A-workspace-engine-extraction
10feb3b docs(planning): Bella master plan — phases 0–3, blocks A–J
```

## 2026-06-24 — Harness pull from base-template (b8ebbf7)

Pulled the current `base-template` harness (commit `b8ebbf71c20445de65195037aa24bfe00bbf080b`) into
`.claude/`. Brought all SDLC commands current and added **`/generate-master-plan`** plus the
**block-definition planning seam** (D34): `/generate-tasks --from <path>` to decompose a standalone
block file, `/plan` as a single standalone block definition, and the hardened block skeleton
(What/Why/Files/Interfaces/Out-of-scope/Acceptance). Also the **plan-quality floor** (D35) — planning
commands clarify-or-abort rather than fabricate a load-bearing element. The `/sdlc-flow` engine was
already present from scaffold. Engines `node --check` clean; command/engine files byte-identical to
base. `planning/harness.json` untouched. Removed the scaffold `.claude/settings.json` (Python
`pre/post_tool_use.py` hooks) — Bella is Rust with no `.claude/hooks/`, so those hooks would error;
deletion is correct. Provenance re-stamped in `planning/.template-version`.

## 2026-06-24

Project initialized from `base-template` (commit `45bda73d575ceba2ae0216f67a10a5334de3f5b4`) via `/new-project`.
Planning infrastructure scaffolded: `planning/context.md`, `planning/status.md`,
`planning/master-plan.md`, `planning/index.md`, `planning/harness.json`, `planning/decisions/`,
and the root `CLAUDE.md` / `README.md`. Concept folders (`planning/<concept>/`) are created on
demand by the SDLC pipeline. Curated SDLC harness (`.claude/`) in place.

Next step: run `/generate-tasks` for the first Phase 0 block to begin the pipeline.

```diff
(no code changes — planning files only)
```
