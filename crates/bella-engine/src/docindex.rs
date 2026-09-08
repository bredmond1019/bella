//! Scoped `doc_id` → path index (BE.7.G task 1).
//!
//! Maps a document's `doc_id` frontmatter value — falling back to the
//! filename stem when frontmatter omits it or the file has none, which is
//! the OKF default — to its path, built by walking ONE corpus root (see
//! [`crate::browser::resolve_corpus_root`]).
//!
//! Deliberately scoped to one root, matching okf-core's `scope:doc_id`
//! model: doc_ids are unique within a single repo's corpus but frequently
//! duplicated across repos — measured across the fleet, 1,232 of 2,036
//! doc_ids are duplicated, while bella's own 31, bastion's 124, and HQ
//! docs' 116 are each internally unique. A fleet-wide index is explicitly
//! out of scope for this block.
//!
//! This module is pure and synchronous: [`build_index`] is a plain
//! function over a root path — no threads, no `App`, no laziness. BE.7.G
//! task 2 wraps this in a lazy, off-thread build owned by `App`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::frontmatter::{self, FrontmatterValue};

/// The outcome of resolving one `doc_id` against a [`DocIndex`].
///
/// THREE outcomes, not two — `Ambiguous` is a real, distinct state rather
/// than a coin flip on the first hit that claimed a `doc_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one indexed file claims this `doc_id`.
    Resolved(PathBuf),
    /// No indexed file claims this `doc_id`.
    Unresolved,
    /// More than one indexed file claims this `doc_id`. Carries every path
    /// that claimed it (in walk order) so a caller can name all of them
    /// rather than silently picking one.
    Ambiguous(Vec<PathBuf>),
}

/// A `doc_id` → path(s) index, scoped to one corpus root.
///
/// Built once by [`build_index`]; resolution never re-walks the
/// filesystem.
#[derive(Clone, Debug, Default)]
pub struct DocIndex {
    entries: HashMap<String, Vec<PathBuf>>,
    /// Markdown files visited while building this index — part of the
    /// build-budget measurement (BE.7.G task 2 records this against the
    /// real corpus root; kept here since the walk itself happens here).
    files_visited: usize,
}

impl DocIndex {
    /// Resolve `doc_id` against this index.
    pub fn resolve(&self, doc_id: &str) -> Resolution {
        match self.entries.get(doc_id) {
            None => Resolution::Unresolved,
            Some(paths) if paths.len() == 1 => Resolution::Resolved(paths[0].clone()),
            Some(paths) => Resolution::Ambiguous(paths.clone()),
        }
    }

    /// Number of markdown files visited while building this index — the
    /// raw input to the build-budget measurement.
    pub fn files_visited(&self) -> usize {
        self.files_visited
    }

    /// Number of distinct `doc_id`s indexed (Resolved + Ambiguous keys).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the index holds no `doc_id`s at all — an empty or
    /// unreadable root, or a root with no markdown files.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Build a [`DocIndex`] by walking `root` for markdown files (`.md`),
/// reading each one's frontmatter and keying on its `doc_id` entry
/// (falling back to the filename stem when frontmatter omits it or the
/// file has none).
///
/// Follows symlinked directories, mirroring `browser::build_entries`'s
/// `follow_links(true)` — three quarters of this repo's own OKF documents
/// live behind `planning/ -> ../_planning/bella`, and a walker that does
/// not follow it sees a quarter of the corpus and reports the rest
/// `Unresolved`, which looks exactly like a correct empty answer.
///
/// Never errors: an unreadable, empty, or nonexistent root yields an empty
/// index, and every subsequent [`DocIndex::resolve`] call reports
/// `Resolution::Unresolved`. Hidden files and anything `.gitignore`d are
/// still walked (unlike `browser::build_entries`) — a doc_id target should
/// resolve regardless of whether the browser pane happens to be hiding its
/// containing directory.
pub fn build_index(root: &Path) -> DocIndex {
    let mut entries: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut files_visited = 0usize;

    let walker = ignore::WalkBuilder::new(root)
        .follow_links(true)
        .hidden(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            // A broken symlink or a permission error: skip it rather than
            // abort the whole build over one bad entry.
            Err(_) => continue,
        };

        let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        files_visited += 1;

        let doc_id = doc_id_for(path, &source);
        if doc_id.is_empty() {
            continue;
        }
        entries.entry(doc_id).or_default().push(path.to_path_buf());
    }

    DocIndex {
        entries,
        files_visited,
    }
}

/// The `doc_id` a document claims: its frontmatter `doc_id:` entry if
/// present and non-empty, otherwise the filename stem (the OKF default).
fn doc_id_for(path: &Path, source: &str) -> String {
    if let Some(fm) = frontmatter::parse(source) {
        for (key, value) in &fm.entries {
            if key == "doc_id"
                && let FrontmatterValue::Scalar(s) = value
                && !s.is_empty()
            {
                return s.clone();
            }
        }
    }
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn write_doc(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, contents).expect("write fixture doc");
        path
    }

    #[test]
    fn resolves_a_doc_id_to_its_path() {
        let root = crate::testsupport::unique_temp_dir("docindex-resolve");
        let target = write_doc(
            &root,
            "target.md",
            "---\ndoc_id: my-doc\ntitle: T\ndescription: d\n---\nbody\n",
        );

        let index = build_index(&root);

        assert_eq!(index.resolve("my-doc"), Resolution::Resolved(target));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn falls_back_to_filename_stem_when_doc_id_is_absent() {
        let root = crate::testsupport::unique_temp_dir("docindex-stem");
        let target = write_doc(
            &root,
            "no-front-matter-id.md",
            "---\ntitle: T\ndescription: d\n---\nbody\n",
        );

        let index = build_index(&root);

        assert_eq!(
            index.resolve("no-front-matter-id"),
            Resolution::Resolved(target)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn falls_back_to_filename_stem_when_the_file_has_no_frontmatter_at_all() {
        let root = crate::testsupport::unique_temp_dir("docindex-no-fm");
        let target = write_doc(&root, "plain-file.md", "just a body, no fence\n");

        let index = build_index(&root);

        assert_eq!(index.resolve("plain-file"), Resolution::Resolved(target));
        let _ = fs::remove_dir_all(&root);
    }

    /// Task 1 AC: "The gate was shown capable of failing: point the index
    /// at an empty root and every reference reports Unresolved."
    ///
    /// Observed by hand: an empty directory (no markdown files at all)
    /// produces a zero-entry index, and `resolve()` reports `Unresolved`
    /// for any doc_id rather than panicking, hanging, or returning a
    /// stale/default hit. Matches the observation-recording pattern used
    /// at crates/bella/src/app.rs:3515.
    #[test]
    fn empty_root_reports_every_reference_unresolved() {
        let root = crate::testsupport::unique_temp_dir("docindex-empty-root");

        let index = build_index(&root);

        assert!(index.is_empty(), "empty root must yield an empty index");
        assert_eq!(index.resolve("anything"), Resolution::Unresolved);
        assert_eq!(index.resolve("my-doc"), Resolution::Unresolved);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unresolved_when_no_file_claims_the_doc_id() {
        let root = crate::testsupport::unique_temp_dir("docindex-unresolved");
        write_doc(&root, "a.md", "---\ndoc_id: a-doc\n---\n");

        let index = build_index(&root);

        assert_eq!(index.resolve("nonexistent-doc"), Resolution::Unresolved);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn two_files_claiming_the_same_doc_id_are_ambiguous() {
        let root = crate::testsupport::unique_temp_dir("docindex-ambiguous");
        let a = write_doc(&root, "a.md", "---\ndoc_id: dup\n---\n");
        let b = write_doc(&root, "b.md", "---\ndoc_id: dup\n---\n");

        let index = build_index(&root);

        match index.resolve("dup") {
            Resolution::Ambiguous(paths) => {
                assert_eq!(paths.len(), 2, "expected exactly two claimants");
                assert!(paths.contains(&a), "ambiguous result must name {a:?}");
                assert!(paths.contains(&b), "ambiguous result must name {b:?}");
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// Task 1 AC: "The symlink fixture is a genuine control: it asserts a
    /// document reachable ONLY through the symlink is found." The target
    /// document lives solely under `real/`, reachable in the fixture tree
    /// only via `root/linked -> real`, mirroring bella's own
    /// `planning/ -> ../_planning/bella` layout.
    #[test]
    fn crosses_a_symlinked_subdirectory() {
        let root = crate::testsupport::unique_temp_dir("docindex-symlink-root");
        let real = crate::testsupport::unique_temp_dir("docindex-symlink-target");
        write_doc(
            &real,
            "behind-symlink.md",
            "---\ndoc_id: symlinked-doc\n---\n",
        );
        let link = root.join("linked");
        symlink(&real, &link).expect("create symlink fixture");

        let index = build_index(&root);

        assert_eq!(
            index.resolve("symlinked-doc"),
            Resolution::Resolved(link.join("behind-symlink.md")),
            "a document reachable only through a symlinked directory must resolve"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&real);
    }

    #[test]
    fn build_budget_counts_only_markdown_files_visited() {
        let root = crate::testsupport::unique_temp_dir("docindex-budget");
        write_doc(&root, "a.md", "---\ndoc_id: a\n---\n");
        write_doc(&root, "b.md", "---\ndoc_id: b\n---\n");
        write_doc(&root, "not-markdown.txt", "ignored, not .md");

        let index = build_index(&root);

        assert_eq!(index.files_visited(), 2);
        let _ = fs::remove_dir_all(&root);
    }
}
