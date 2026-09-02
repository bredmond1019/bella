//! bella library crate: exposes the TUI's internal modules so integration
//! tests (`crates/bella/tests/`) can drive them directly — notably the
//! background render worker (`render_worker`), which has no other seam for
//! a hermetic, non-blocking test.
//!
//! The `bella` binary (`src/main.rs`) consumes this crate rather than
//! declaring its own module tree, so there is exactly one copy of each
//! module compiled for both the bin and any integration tests.

pub mod app;
pub mod events;
pub mod history;
pub mod render_worker;
pub mod selection;
pub mod ui;

// Test-only fixture helper (unique_temp_dir). Deliberately NOT part of the
// pub mod list above — that list exists so integration tests can drive the
// modules; testsupport must not be reachable from the shipped binary.
#[cfg(test)]
mod testsupport;
