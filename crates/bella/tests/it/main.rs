//! The single integration-test binary for `bella` — every file under `tests/it/` is a
//! module of THIS binary, not a binary of its own.
//!
//! Why: cargo builds one test binary per `tests/*.rs` file, and each one statically links
//! the whole crate plus its dependency graph. One binary per crate means one link instead
//! of N. Same pattern as `core/mev`'s `tests/it/` and `core/engine-rs`'s
//! `crates/engine-core/tests/it/`.
//!
//! Test ISOLATION is unaffected under `cargo nextest run` (this repo's mandated runner —
//! CLAUDE.md standing rule 7): it executes every test in its own process regardless of how
//! many binaries the tests are packed into. Plain `cargo test` runs them multi-threaded in
//! one process instead, which is why no test here may mutate global process state (e.g.
//! `env::set_current_dir`, `env::set_var`) — audited clear as of BE.7.M task 4.
//!
//! Adding an integration test: create `tests/it/<name>.rs` and add one `mod <name>;` line
//! below. Do NOT add a new `tests/*.rs` file at this level — that silently reintroduces a
//! second binary (guarded by `scripts/check_test_layout.sh`).

mod golden_draw;
mod layout;
mod render_async;
