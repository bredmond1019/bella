//! Restricted OKF frontmatter reader.
//!
//! This is deliberately NOT a general YAML parser and does not depend on a
//! YAML crate (`yaml-rust` in particular is rejected — unmaintained, carries
//! RUSTSEC-2024-0320). It recognizes exactly four value shapes:
//!
//! 1. bare scalar        — `type: Plan`
//! 2. quoted scalar       — `title: "a: b"`
//! 3. inline array        — `related: [a, b]`
//! 4. block list           — `- item` lines indented under a key
//!
//! Anything outside those four shapes is retained as [`FrontmatterValue::Raw`]
//! rather than erroring — this module never fails to parse a document; it
//! only ever fails to *understand* part of one.
//!
//! Keys come back in source order (`Vec<(String, FrontmatterValue)>`, not a
//! `HashMap`) because the metadata pane (BE.7.F) renders them in the order
//! the author wrote them.

// Not yet wired into markdown.rs or re-exported from lib.rs (that's BE.7.A
// task 2), so nothing in the crate calls these public items yet. Silence
// the resulting dead_code lint rather than mark real API `pub(crate)` —
// task 2 makes this module's `pub`s part of the crate's public surface.
#![allow(dead_code)]

use std::ops::Range;

/// One frontmatter value, in whichever of the four shapes it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrontmatterValue {
    /// A bare or quoted scalar, with surrounding quotes (if any) stripped.
    Scalar(String),
    /// An inline array (`[a, b]`) or a block list (`- item` lines).
    List(Vec<String>),
    /// A value shape this reader doesn't specifically understand, kept
    /// verbatim rather than dropped or treated as an error.
    Raw(String),
}

/// Frontmatter keys in source order, alongside their parsed values.
pub type FrontmatterEntries = Vec<(String, FrontmatterValue)>;

/// A parsed frontmatter block: its entries plus the byte range (in the
/// original source) that the whole `---`-fenced block occupies, including
/// both fence lines and the trailing newline after the closing fence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontmatter {
    pub entries: FrontmatterEntries,
    pub byte_range: Range<usize>,
}

/// Detect a `---`-fenced frontmatter block at the start of `source`.
///
/// Returns the byte range of the whole block — from the start of the
/// opening `---` line through the end of the closing `---` line's trailing
/// newline (if the source has one) — or `None` if:
/// - the source is empty,
/// - the first line isn't exactly `---` (ignoring a trailing `\r`), or
/// - no closing `---` line is found anywhere after it.
///
/// A `---` that appears later in the document body, after a closing fence
/// has already been found, does not affect detection — only the *first*
/// `---` line after the opening fence closes the block.
pub fn detect_fence(source: &str) -> Option<Range<usize>> {
    if source.is_empty() {
        return None;
    }

    let mut lines = LineOffsets::new(source);
    let first = lines.next()?;
    if !is_fence_line(&source[first.clone()]) {
        return None;
    }

    for line in lines {
        if is_fence_line(&source[line.clone()]) {
            // Extend the range to include the line's own trailing newline,
            // if it has one, so the block includes the blank line the
            // fence sits on.
            let end = line_end_with_newline(source, line.end);
            return Some(first.start..end);
        }
    }

    None
}

/// True if the byte range `line` (excluding its terminator) is a fence
/// marker: exactly `---`, allowing a trailing `\r` from CRLF line endings.
fn is_fence_line(line: &str) -> bool {
    line.trim_end_matches('\r') == "---"
}

/// Given the byte offset just past a line's content (before its `\n`, if
/// any), return the offset just past that `\n` too, so callers can include
/// the newline in a range. If there's no `\n` at `pos` (end of file), `pos`
/// is returned unchanged.
fn line_end_with_newline(source: &str, pos: usize) -> usize {
    if source.as_bytes().get(pos) == Some(&b'\n') {
        pos + 1
    } else {
        pos
    }
}

/// Iterator over the byte ranges of each line in `source` (excluding the
/// `\n` terminator, if any).
struct LineOffsets<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> LineOffsets<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }
}

impl Iterator for LineOffsets<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Range<usize>> {
        if self.pos > self.source.len() {
            return None;
        }
        if self.pos == self.source.len() {
            // Only yield a final empty line if the string doesn't already
            // end in a newline that was consumed by the previous line.
            return None;
        }
        let start = self.pos;
        let rest = &self.source[start..];
        let end = match rest.find('\n') {
            Some(idx) => {
                self.pos = start + idx + 1;
                start + idx
            }
            None => {
                self.pos = self.source.len() + 1; // sentinel: stop next call
                self.source.len()
            }
        };
        Some(start..end)
    }
}

/// Parse the frontmatter block at the start of `source`, if one exists.
///
/// Returns `None` when [`detect_fence`] finds no fenced block. Never
/// returns `Err` — an unrecognized value shape is retained as
/// [`FrontmatterValue::Raw`] rather than failing the whole parse.
pub fn parse(source: &str) -> Option<Frontmatter> {
    let byte_range = detect_fence(source)?;
    let block = &source[byte_range.clone()];
    let entries = parse_entries(block);
    Some(Frontmatter {
        entries,
        byte_range,
    })
}

/// Parse the key/value entries out of a fenced block's raw text (fence
/// lines included — they're skipped by shape, not by position, so this is
/// robust to how the caller sliced the block).
fn parse_entries(block: &str) -> FrontmatterEntries {
    let mut entries: FrontmatterEntries = Vec::new();
    // Index into `entries` of the key currently eligible to receive
    // `- item` list continuation lines or raw continuation text.
    let mut pending: Option<usize> = None;

    for raw_line in block.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();

        if is_fence_line(trimmed) {
            // Opening or closing fence marker — not content.
            continue;
        }

        if trimmed.is_empty() {
            // A blank line doesn't close a pending block list or raw
            // continuation; okf-core rejects a blank line mid-block and
            // this corpus contains one, so it must not error here.
            continue;
        }

        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| if trimmed == "-" { Some("") } else { None })
        {
            if let Some(idx) = pending {
                match &mut entries[idx].1 {
                    FrontmatterValue::List(items) => items.push(item.trim().to_string()),
                    other => {
                        // A list item under a key that wasn't recognized as
                        // starting a list (e.g. it had an inline raw value
                        // already) — convert it to a list rather than drop
                        // the item.
                        *other = FrontmatterValue::List(vec![item.trim().to_string()]);
                    }
                }
                continue;
            }
            // A list item with no pending key — nothing to attach it to.
            // Not one of the four shapes; ignore rather than error.
            continue;
        }

        if let Some((key, value)) = split_key_value(trimmed) {
            let key = key.trim().to_string();
            let value_trimmed = value.trim();

            if value_trimmed.is_empty() {
                // Either a block-list header (`related:` followed by
                // `- item` lines) or a block-scalar header (e.g.
                // `description: >-` folded text). Start as an empty list;
                // if raw continuation lines follow instead of `- item`
                // lines, they get folded into Raw below.
                entries.push((key, FrontmatterValue::List(Vec::new())));
                pending = Some(entries.len() - 1);
                continue;
            }

            let parsed = parse_value(value_trimmed);
            entries.push((key, parsed));
            pending = Some(entries.len() - 1);
            continue;
        }

        // No `key:` and no `- item` — a continuation line (e.g. folded
        // block-scalar text, or a `>-`/`|` indicator's body). Fold it into
        // the pending entry as raw text rather than erroring.
        if let Some(idx) = pending {
            match &mut entries[idx].1 {
                FrontmatterValue::Raw(text) => {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
                FrontmatterValue::List(items) if items.is_empty() => {
                    entries[idx].1 = FrontmatterValue::Raw(trimmed.to_string());
                }
                FrontmatterValue::Scalar(existing) => {
                    let mut text = existing.clone();
                    text.push(' ');
                    text.push_str(trimmed);
                    entries[idx].1 = FrontmatterValue::Raw(text);
                }
                _ => {}
            }
        }
    }

    entries
}

/// Split a trimmed, non-empty line on its first `:` into `(key, value)`,
/// where `value` is everything after the colon (not yet trimmed). Returns
/// `None` if there's no `:` at all — such a line isn't `key: value` shaped.
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    let key = &line[..idx];
    // Guard against a colon inside a quoted scalar's key position, e.g. a
    // line that's actually a raw value with no real key — a real OKF key is
    // a bare identifier-ish token with no leading quote.
    if key.trim().is_empty() {
        return None;
    }
    let value = &line[idx + 1..];
    Some((key, value))
}

/// Classify a non-empty, already-trimmed value string into one of the four
/// shapes (bare scalar / quoted scalar / inline array), or `Raw` if it
/// doesn't fit any of them.
fn parse_value(value: &str) -> FrontmatterValue {
    if value.len() >= 2 && value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        let items = if inner.trim().is_empty() {
            Vec::new()
        } else {
            inner
                .split(',')
                .map(|item| unquote(item.trim()).to_string())
                .collect()
        };
        return FrontmatterValue::List(items);
    }

    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return FrontmatterValue::Scalar(unquote(value).to_string());
    }

    // Anything starting with a YAML indicator character this reader
    // doesn't specifically support (flow mappings, anchors, aliases,
    // explicit block-scalar markers with no key context, tags) is kept
    // verbatim as Raw rather than misparsed as a bare scalar.
    let first = value.chars().next();
    if matches!(
        first,
        Some('{') | Some('&') | Some('*') | Some('!') | Some('|') | Some('>')
    ) {
        return FrontmatterValue::Raw(value.to_string());
    }

    FrontmatterValue::Scalar(value.to_string())
}

/// Strip one layer of matching double quotes and unescape `\"` inside, if
/// `value` is quoted; otherwise return it unchanged.
fn unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        let inner = &value[1..value.len() - 1];
        inner.replace("\\\"", "\"")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- fence detector -----------------------------------------------

    #[test]
    fn fence_present() {
        let src = "---\ntype: Plan\n---\n# Hello\n";
        let range = detect_fence(src).expect("fence should be detected");
        assert_eq!(&src[range], "---\ntype: Plan\n---\n");
    }

    #[test]
    fn fence_absent() {
        let src = "# Hello\n\nNo frontmatter here.\n";
        assert_eq!(detect_fence(src), None);
    }

    #[test]
    fn fence_unclosed() {
        let src = "---\ntype: Plan\n# Hello\n";
        assert_eq!(detect_fence(src), None);
    }

    #[test]
    fn fence_empty_file() {
        assert_eq!(detect_fence(""), None);
    }

    #[test]
    fn fence_mid_document_dashes_do_not_confuse_closing() {
        // The *first* `---` after the opener closes the block; a later
        // `---` in the body (e.g. a markdown horizontal rule) must not be
        // mistaken for part of the fence detection and must not prevent
        // detection.
        let src = "---\ntype: Plan\n---\nBody text.\n\n---\n\nMore body.\n";
        let range = detect_fence(src).expect("fence should be detected");
        assert_eq!(&src[range], "---\ntype: Plan\n---\n");
    }

    #[test]
    fn fence_requires_opener_on_line_one() {
        let src = "Some text.\n---\ntype: Plan\n---\n";
        assert_eq!(detect_fence(src), None);
    }

    #[test]
    fn fence_no_trailing_newline_at_eof() {
        let src = "---\ntype: Plan\n---";
        let range = detect_fence(src).expect("fence should be detected");
        assert_eq!(range.end, src.len());
        assert_eq!(&src[range], "---\ntype: Plan\n---");
    }

    // --- the four supported shapes --------------------------------------

    #[test]
    fn bare_scalar_shape() {
        let src = "---\ntype: Plan\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![(
                "type".to_string(),
                FrontmatterValue::Scalar("Plan".to_string())
            )]
        );
    }

    #[test]
    fn quoted_scalar_shape() {
        let src = "---\ntitle: \"a: b\"\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![(
                "title".to_string(),
                FrontmatterValue::Scalar("a: b".to_string())
            )]
        );
    }

    #[test]
    fn inline_array_shape() {
        let src = "---\nrelated: [a, b, c]\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![(
                "related".to_string(),
                FrontmatterValue::List(vec!["a".to_string(), "b".to_string(), "c".to_string()])
            )]
        );
    }

    #[test]
    fn inline_array_empty() {
        let src = "---\nrelated: []\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![("related".to_string(), FrontmatterValue::List(vec![]))]
        );
    }

    #[test]
    fn block_list_shape() {
        let src = "---\nkeywords:\n  - alpha\n  - beta\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![(
                "keywords".to_string(),
                FrontmatterValue::List(vec!["alpha".to_string(), "beta".to_string()])
            )]
        );
    }

    #[test]
    fn all_four_shapes_together_and_in_source_order() {
        let src = concat!(
            "---\n",
            "type: Plan\n",
            "title: \"a: b\"\n",
            "related: [x, y]\n",
            "keywords:\n",
            "  - one\n",
            "  - two\n",
            "---\n",
            "# Hello\n",
        );
        let fm = parse(src).expect("frontmatter should parse");
        assert_eq!(
            fm.entries,
            vec![
                (
                    "type".to_string(),
                    FrontmatterValue::Scalar("Plan".to_string())
                ),
                (
                    "title".to_string(),
                    FrontmatterValue::Scalar("a: b".to_string())
                ),
                (
                    "related".to_string(),
                    FrontmatterValue::List(vec!["x".to_string(), "y".to_string()])
                ),
                (
                    "keywords".to_string(),
                    FrontmatterValue::List(vec!["one".to_string(), "two".to_string()])
                ),
            ]
        );
    }

    // --- unsupported shape: retained as raw, never an error -------------

    #[test]
    fn unsupported_flow_mapping_is_retained_as_raw_not_an_error() {
        let src = "---\nweird: {a: b}\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should still parse as a whole");
        assert_eq!(
            fm.entries,
            vec![(
                "weird".to_string(),
                FrontmatterValue::Raw("{a: b}".to_string())
            )]
        );
    }

    #[test]
    fn unsupported_anchor_is_retained_as_raw_not_an_error() {
        let src = "---\nnode: &anchor value\n---\n# Hello\n";
        let fm = parse(src).expect("frontmatter should still parse as a whole");
        assert_eq!(
            fm.entries,
            vec![(
                "node".to_string(),
                FrontmatterValue::Raw("&anchor value".to_string())
            )]
        );
    }

    // --- no crate/YAML dependency escape hatch ---------------------------
    // (enforced by task 1's acceptance criteria at the Cargo.toml level,
    // not by a runtime test — nothing to assert here.)
}
