// bella-engine — render/layout engine, attributed MIT derivative of zemse/hackmd @ 7650cdc.
// Public surface for the bella app crate and integration tests.

pub mod browser;
pub mod frontmatter;
pub mod geometry;
pub mod links;
pub mod markdown;
pub mod md_config;
pub mod palette;
pub mod syntax;
pub mod theme;

// Test-only fixture helper (unique_temp_dir). Deliberately not re-exported
// or added to the public surface above — this crate's public surface is a
// cross-repo contract with bastion (see CLAUDE.md), and testsupport must not
// be part of it.
#[cfg(test)]
mod testsupport;

// Re-export the stable public surface consumed by the bella app crate and
// later blocks. Names match the Engine surface line in master-plan.md.
pub use frontmatter::{Frontmatter, FrontmatterValue, parse as parse_frontmatter};
pub use geometry::{body_pos, select_word_at};
pub use links::{CheckboxMap, LinkMap, LinkTarget, TableMap};
pub use markdown::{
    Rendered, display_row_to_source_line, render_with_edit, source_line_to_display_row,
};
pub use theme::Theme;
