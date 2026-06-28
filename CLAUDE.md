# CLAUDE.md — Bella

A beautiful terminal markdown viewer and editor with full mouse support — local-only, no cloud.

## Before you start

- **Strategic context:** `planning/context.md` (read first) → `planning/status.md` (current state)
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
6. <!-- Add project-specific standing rules here (prompt handling, registries, deployment
   boundaries, code style, etc.). -->

## Known bugs

None known at initialization.

## Build / test / run

```bash
# Rust Cargo workspace (crates/bella-engine + crates/bella).
cargo build                                  # build all crates (debug)
cargo build --release                        # release build
cargo test                                   # run the suite (authoritative)
cargo clippy --all-targets -- -D warnings    # lint gate
cargo fmt --check                            # format gate
cargo run -p bella -- <file|dir>             # run the viewer
```

> **Stack note:** the SDLC harness/skills default to npm/Next assumptions. This is a Rust
> project — `planning/harness.json` is already set to the `rust` profile (fmt/clippy/test/build).

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
