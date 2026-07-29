# CLAUDE.md — Bella

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

## Before you start

- **Strategic context:** `planning/context.md` (read first) → `planning/status.md` (current state)
- **Symlink warning:** the `planning/` directory is actually a local symlink pointing to the company brain repo's `_planning/` vault (e.g. `core/_planning/bella/`). The brain repo is responsible for tracking all planning files under Git. Do not track `planning/` in this project's public Git repository (it is gitignored).
- **Plan:** `planning/master-plan.md` — the phase/block sequence
- **Pipeline config:** `planning/harness.json` — the validation commands + UI-test config the
  SDLC engines run (see `planning/harness.examples.md` for ready-made stack profiles)
- **Decisions log:** `planning/decisions/` (start at `planning/decisions/index.md`) — check
  before relitigating any settled choice

## Standing rules

1. **Every new function, module, or behaviour change ships with tests.** No exceptions — this applies to ad-hoc fixes and one-off changes just as much as formal blocks/tasks. If you add or change code, add or update the tests that cover it.
2. **OKF frontmatter is required on every new `.md` file** under `docs/` and `planning/`.
   Open each file with a YAML block containing the three required fields and, where known, all
   six optional fields:
   - **Required:** `type` (Decision · Index · Plan · Architecture · Reference · Guide · Log ·
     ProjectStatus · LocalContext · Handoff · …), `title`, `description`
   - **Optional:** `doc_id` (kebab-case, defaults to filename stem), `layer` (closed list:
     `brain` · `engine` · `factory` · `console` · `surface` · `infra` · `business` · `content` ·
     `meta`), `project` (`bella` for this repo; omit only for genuinely cross-cutting docs),
     `status` (`active` · `draft` · `deprecated` · `superseded` · `archived`), `keywords`
     (3–7 concrete topic terms), `related` (list of `doc_id` values from real in-repo cross-refs)
   - **Retained only on Log / ProjectStatus docs:** `timestamp`
   - Canonical guide: `docs/okf-frontmatter.md` in the company-brain repo; governing decision: D27.
   - **Adding a file to a directory requires updating that directory's `index.md`** (add a row or
     entry for the new file). If the update changes the scope of a parent directory's `index.md`,
     update that too — propagate up the chain as needed.
3. **Sequence, not calendar** — work the order in `master-plan.md`; pick up where you left off.
4. **Decisions are append-only** — never edit a settled decision; supersede it with a new
   atomic file in `planning/decisions/` and link back.
5. **Verified identity / handles:** Brandon Redmond — sole maintainer. No public handles/URLs
   yet (local-only personal tool). The render/layout engine is derived from
   `github.com/zemse/hackmd` (MIT) — treat that as the only authoritative upstream; flag any
   other handle or profile link as unverified before publishing it.
6. **`bella-engine`'s public surface is a cross-repo contract with `bastion`.** `bastion` depends on
   `bella-engine` as a Cargo path dependency (see `planning/decisions/D3-bella-engine-shared-with-bastion.md`)
   with no version pin and no cross-repo CI. Before merging any change to `bella-engine/src/lib.rs`'s
   re-exports, or to public items in `browser.rs`/`theme.rs`/`markdown.rs`, run `cargo build && cargo
   test` in `core/bastion` and fix or coordinate any break. `bella` (the standalone binary crate)
   stays independent — it is meant to also ship as its own open-source project, separate from
   bastion's ops framing (D3).
7. **Use `cargo nextest run`, never plain `cargo test`, for any test run you invoke yourself
   during a task** (scoped: `cargo nextest run -p <crate> <module::path>`; full fast pass:
   `cargo nextest run --lib --bins --workspace`). The one exception is the task explicitly designated to
   own full-suite validation for a spec — that task runs the real `cargo test` / `cargo build
   --release` gates, per `planning/harness.json`'s `command` (not `fastCommand`). See "Build /
   test / run" below for the full rationale.

## Known bugs

None known at initialization.

## Build / test / run

```bash
# Rust Cargo workspace (crates/bella-engine + crates/bella).
cargo build                                  # build all crates (debug)
cargo build --release                        # release build
cargo nextest run --lib --bins --workspace          # fast — use this, not plain `cargo test`
cargo test                                   # full suite (authoritative)
cargo clippy --all-targets -- -D warnings    # lint gate
cargo fmt --check                            # format gate
cargo run -p bella -- <file|dir>             # run the viewer
```

> **Stack note:** the SDLC harness/skills default to npm/Next assumptions. This is a Rust
> project — `planning/harness.json` is already set to the `rust` profile (fmt/clippy/test/build).

> **Always prefer `cargo nextest run --lib --bins --workspace` over plain `cargo test` in this repo.**
> This is wired as the `fastCommand` on the `test` check in `planning/harness.json`, which the
> SDLC engines use for per-task (`testDepth: "fast"`) runs — reach for it manually too whenever
> iterating outside the harness. Requires `cargo-nextest` on PATH (`brew install cargo-nextest`);
> `cargo test` remains the authoritative full-suite gate.
>
> **Scope even narrower while mid-task**: `cargo nextest run -p <crate> <module::path>` for just
> the touched crate/module. Only the task(s) explicitly owning full-suite validation for a spec
> should run the full `cargo test` / `cargo build --release` gates.
>
> **`sccache` is wired in via `.cargo/config.toml`** (`rustc-wrapper = "sccache"`) — caches
> compiled object code across builds so repeated compiles within an SDLC spec reuse work instead
> of recompiling from scratch. Requires `sccache` on PATH (`brew install sccache`).
>
> The SDLC pipeline reads its validation suite from `planning/harness.json` (not from this
> block). Keep the `<test>`/`<build>` commands here in sync with that file's
> `validation.checks[]` so humans and the pipeline run the same thing.

## Directory map

```
bella/
├── .claude/        ← Claude Code commands + SDLC workflow engines
├── crates/
│   ├── bella-engine/   ← render/layout library (palette, markdown, geometry, syntax, links)
│   └── bella/          ← TUI binary (clap CLI, ratatui draw loop, events, app state, history)
├── planning/       ← context, status, master-plan, harness.json, decisions/, <concept>/
└── reference/      ← upstream zemse/hackmd source (excluded from workspace)
```

## What NOT to touch

<!-- Reference-only code, generated files, migration history, etc. List them as they appear. -->

---

## SDLC pipeline

This project carries the curated SDLC harness. Run `/prime` to orient, then drive structured
work through `/generate-tasks → /implement → /test → /review-task → /document → /log-work`.
See `.claude/commands/README.md` for the full pipeline reference.

> **Stack note:** the SDLC engines carry no stack defaults. Point them at this project's stack
> by filling `planning/harness.json` (validation commands + optional UI-test config). Copy a
> ready-made profile from `planning/harness.examples.md` (Rust / Python / Next.js). Do **not**
> edit the `workflows/*.js` engines for stack reasons — that's what `harness.json` is for.
