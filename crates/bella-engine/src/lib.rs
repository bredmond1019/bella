// bella-engine — render/layout engine, attributed MIT derivative of zemse/hackmd @ 7650cdc.
// Public surface for the bella app crate and integration tests.

pub mod browser;
pub mod geometry;
pub mod links;
pub mod markdown;
pub mod md_config;
pub mod palette;
pub mod syntax;
pub mod theme;

// Re-export the stable public surface consumed by the bella app crate and
// later blocks. Names match the Engine surface line in master-plan.md.
pub use geometry::{body_pos, select_word_at};
pub use links::{CheckboxMap, LinkMap, LinkTarget, TableMap};
pub use markdown::{Rendered, render_with_edit};
pub use theme::Theme;
