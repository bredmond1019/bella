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
