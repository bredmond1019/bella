# CLAUDE.md — Bella

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

## Workflow engine telemetry

**After invoking `Workflow({name: 'sdlc-task'|'sdlc-flow', ...})`, load the `stamp-workflow-run-id`
skill.** The engine script can't read its own Workflow run id back — the Workflow script API has no
`runId` global and no filesystem access — so joining a run's `sdlc-task-state.json`/
`sdlc-flow-state.json` to the exact Claude Code session transcript for cost telemetry relies on the
*invoking* agent patching the id in after the call returns. Skip this and `workflow_run_id` simply
stays `null` — a normal, expected state, never a defect to chase.

## Before you start

- **Strategic context:** `planning/context.md` (read first) → `planning/status.md` (current state)
- **Symlink warning:** the `planning/` directory is actually a local symlink pointing to the company brain repo's `_planning/` vault (e.g. `core/_planning/bella/`). The brain repo is responsible for tracking all planning files under Git. Do not track `planning/` in this project's public Git repository (it is gitignored).
- **Symlink traps:** `rg`/`grep`/`find` are symlink-blind by default — a search that must include `planning/` content needs `-L`/`--follow`. `git mv` fails through the symlink face ("source directory is empty") — move planning files via the real vault path (`.../_planning/<slug>/...`), never via `planning/...`. Planning changes are committed in the brain repo (`agentic-portfolio`) with an explicit pathspec, never in this repo.
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
8. **Never `git push` this repo directly from inside it.** `bastion` path-depends on
   `bella-engine` in this repo, and every Rust repo's CI clones its sibling path-deps at their
   unpinned default branch — pushing out of order breaks a sibling's CI on code that was
   actually fine (the 2026-08-18 mev/bastion outage is the canonical example of this failure
   mode). Route every push through the company-brain's `agentic-portfolio/scripts/git_push.sh
   --all`, which pushes the whole fleet in dependency order and skips a repo flagged
   `ci-blocked` (a Cargo dependency is red on GitHub with nothing queued to fix it). Branching,
   committing, and opening/reviewing/merging PRs to `main` locally are all fine from inside this
   repo — only the final `git push` of `main` to `origin` must go through that script.

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
> **`sccache` is deliberately NOT wired in.** It was removed fleet-wide after measuring zero
> cache hits: sccache refuses to cache incremental compilations and cargo passes
> `-C incremental=...` for the dev/test profile, so every rustc call fell through to plain rustc
> plus a useless wrapper hop. See the comment at the top of `.cargo/config.toml`, and
> `[profile.dev]` in the workspace root `Cargo.toml` for the link-time setting that did help.
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

<!-- BEGIN:response-style -->
## Response Style

You are read by an operator scanning several concurrent agent sessions. Long prose is the failure
mode, not thoroughness.

1. **First line = the outcome** — what happened, and whether it needs them.
2. **Then the specifics** — bullets, one line each, max ~6. Facts, not narration.
3. **Last line = the ask**, if there is one. One question, answerable in a word.

**Ceiling: 10 lines for a normal turn, 20 for an end-of-run report.** Only depth the operator
explicitly asked for may exceed it.

Durable detail goes to disk — the commands already require that. **Link the path; do not restate
the file.** Lead with failures, blocks, and anything that did not match the ask, in plain words with
the real error text. Cut reasoning narration, unasked-for next steps, and self-assessment.

Full rationale, the complete cut-list, and worked before/after examples: the
**`report-to-the-operator`** skill.
<!-- END:response-style -->

<!-- BEGIN:session-continuity -->
## Stopping, continuing, and handing off

**Run to completion. Never stop, clear, or hand off because context is getting large.** There is no
token band, no percentage, and no "the next block would be cleaner in a fresh session." A chain runs
every block it was given; a lane that stops after one block and waits to be relaunched by hand
defeats the entire point of the run and puts the operator back in the loop after every block. If
context genuinely runs out, the harness summarizes and you keep going — that is its job, not yours.

There is exactly **one** reason to end a session early, and it is about correctness, not cost:
**something the running session depends on changed underneath it** — an engine, command file,
installed binary (`mev`, `bastion`), hook or `settings.json` edited this session, or a `CLAUDE.md`
you already read. The running session is a launch-time snapshot (base-template standing rule 10), so
it keeps producing pre-change results, which read as an unreliable agent rather than a stale
snapshot. **Name the trigger, finish the unit of work in flight, and say plainly that a fresh
session is needed.** Do not present it as a context-budget decision, and do not go looking for the
trigger as an excuse to stop.

Whenever you do hand off, write the entry point first — `status.md`, `handoff.md`, a spec's
`tasks.json`, or an orchestration-run `notes.md` — so the next agent starts from an artifact instead
of from your memory.
<!-- END:session-continuity -->
