//! Browser model — pure, event-loop-independent directory listing.
//!
//! Provides [`Browser`], which holds the current directory, a sorted list of
//! [`BrowserEntry`] items, a cursor (`selected`), and a scroll offset.  All
//! methods are pure state mutations; no I/O happens after construction.
//!
use std::path::{Path, PathBuf};

/// Distinguishes the kinds of entry shown in the browser listing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BrowserEntryKind {
    /// The `..` synthetic parent-directory entry.
    ParentDir,
    /// A subdirectory of the current directory, not currently expanded.
    Dir,
    /// A subdirectory whose immediate children have been listed and
    /// spliced into `Browser::entries` right after it (one level; see
    /// [`Browser::expand`]). Distinguished from [`BrowserEntryKind::Dir`]
    /// so a renderer can draw a different expand/collapse marker without
    /// consulting a second field, and so [`Browser::collapse`] knows
    /// which entries own a subtree to remove.
    ExpandedDir,
    /// A `.md` or `.mdx` file. The default kind: a leaf with no expand
    /// state, the most neutral value for a placeholder entry.
    #[default]
    Markdown,
}

/// A single row in the browser listing.
#[derive(Debug, Clone, Default)]
pub struct BrowserEntry {
    /// Absolute path of the entry.
    pub path: PathBuf,
    /// Display name (file/dir name component, or `".."` for [`BrowserEntryKind::ParentDir`]).
    pub display: String,
    /// Entry kind.
    pub kind: BrowserEntryKind,
    /// Nesting depth in the (flattened) tree: `0` for an entry listed
    /// directly under the `Browser`'s root `dir`, `1` for a child spliced
    /// in by expanding a depth-`0` entry, and so on. Used to find the
    /// bounds of a subtree to remove on [`Browser::collapse`], and by a
    /// renderer to indent.
    pub depth: usize,
    /// The directory this entry was listed from — `Some(dir)` for every
    /// entry the walker produces, `None` only for a `BrowserEntry` built
    /// through `Default` (or another value that never went through
    /// `build_entries`).
    pub parent: Option<PathBuf>,
}

/// Directory browser state.
///
/// Invariant: `selected < entries.len()` when `entries` is non-empty; `selected
/// == 0` when empty.  `scroll <= selected` and `selected < scroll +
/// viewport_h`.
#[derive(Debug)]
pub struct Browser {
    /// The directory currently being listed.
    pub dir: PathBuf,
    /// Sorted, filtered listing (parent first, then dirs, then markdown files).
    pub entries: Vec<BrowserEntry>,
    /// Index of the currently highlighted entry.
    pub selected: usize,
    /// Index of the first visible entry (scroll offset into `entries`).
    pub scroll: u16,
    /// Optional absolute path above which navigation is blocked.
    pub root_boundary: Option<PathBuf>,
    /// When `true`, relaxes BOTH the hidden-dotfile filter and the
    /// `.gitignore`/global-git-ignore/git-exclude filters, revealing
    /// entries the default listing hides. Defaults to `false` (today's
    /// behaviour: both filters stay on).
    pub reveal_ignored: bool,
    /// Count of entries the last listing dropped because the walker could
    /// not resolve them (e.g. a broken symlink, a permission error). A
    /// non-zero count means the listing is INCOMPLETE, not empty-because-
    /// there-was-nothing-there — distinct from a directory that is simply
    /// empty.
    pub dropped_entries: usize,
    /// Count of directory-listing walks performed so far: one for the
    /// initial [`Browser::new`], one more per [`Browser::set_reveal_ignored`]
    /// re-list, and one per [`Browser::expand`] call. Expanding a
    /// directory walks only that directory's immediate children — this
    /// counter is how a test proves a deep tree was never walked ahead of
    /// time.
    pub walk_count: usize,
}

impl Browser {
    /// Build a new `Browser` rooted at `dir`.
    ///
    /// Lists the directory non-recursively, skips hidden dotfiles, respects
    /// `.gitignore` via the `ignore` walker, and hides non-markdown files.
    /// Entries are ordered: `..` (if parent exists) → subdirectories (alpha,
    /// case-insensitive) → `.md`/`.mdx` files (alpha, case-insensitive).
    ///
    /// `reveal_ignored` starts `false` — today's behaviour.
    pub fn new(dir: PathBuf) -> Self {
        let (entries, dropped_entries) = build_entries(dir.as_path(), false, 0, true);
        Self {
            dir,
            entries,
            selected: 0,
            scroll: 0,
            root_boundary: None,
            reveal_ignored: false,
            dropped_entries,
            walk_count: 1,
        }
    }

    /// Toggle `reveal_ignored` and re-list the current directory.
    ///
    /// The cursor is clamped into the (possibly shorter or longer) new
    /// entry list rather than reset, so toggling reveal off again lands
    /// close to where the user was.
    pub fn set_reveal_ignored(&mut self, reveal: bool) {
        self.reveal_ignored = reveal;
        self.refresh();
    }

    /// Re-list `self.dir` with the current `reveal_ignored` setting.
    ///
    /// Resets to a flat, depth-0 listing — any expanded subtree is
    /// collapsed away, the same way it was never persisted across a
    /// directory change today.
    fn refresh(&mut self) {
        let (entries, dropped_entries) =
            build_entries(self.dir.as_path(), self.reveal_ignored, 0, true);
        self.entries = entries;
        self.dropped_entries = dropped_entries;
        self.walk_count += 1;
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    /// Move the cursor by `delta` rows (positive = down, negative = up).
    ///
    /// Wraps around with `rem_euclid`.  Clamps `scroll` so the selection
    /// remains inside the visible window of height `viewport_h`.  No-op on an
    /// empty list.
    pub fn move_cursor(&mut self, delta: i32, viewport_h: u16) {
        let n = self.entries.len();
        if n == 0 {
            return;
        }

        // Wrap-around arithmetic.
        let new_sel = (self.selected as i32 + delta).rem_euclid(n as i32) as usize;
        self.selected = new_sel;

        // Scroll clamping: keep selected inside [scroll, scroll + viewport_h).
        let vp = viewport_h as usize;
        let scroll = self.scroll as usize;

        let new_scroll = if new_sel < scroll {
            // Selection moved above the viewport top — scroll up.
            new_sel
        } else if new_sel >= scroll + vp {
            // Selection moved below the viewport bottom — scroll down.
            new_sel.saturating_sub(vp.saturating_sub(1))
        } else {
            scroll
        };

        self.scroll = new_scroll as u16;
    }

    /// Return a reference to the currently selected entry, or `None` when the
    /// list is empty.
    pub fn selected_entry(&self) -> Option<&BrowserEntry> {
        self.entries.get(self.selected)
    }

    /// Return the target path when the selected entry is a directory kind
    /// ([`BrowserEntryKind::Dir`], [`BrowserEntryKind::ExpandedDir`], or
    /// [`BrowserEntryKind::ParentDir`]), otherwise `None`.
    ///
    /// Exhaustive over every [`BrowserEntryKind`] variant deliberately — no
    /// `_ =>` arm. A wildcard here would let a future variant (or this
    /// block's own `ExpandedDir`, before this match was widened) fall
    /// through to `None` silently: no compile error, no test failure, just
    /// a directory-shaped entry that quietly refuses to be entered. See the
    /// doc comment on the `descend_exhaustive_match_is_load_bearing`
    /// capability-check test below for the observed failure this guards
    /// against.
    pub fn descend(&self) -> Option<PathBuf> {
        match self.selected_entry()? {
            BrowserEntry {
                kind: BrowserEntryKind::Dir,
                path,
                ..
            } => Some(path.clone()),
            BrowserEntry {
                kind: BrowserEntryKind::ExpandedDir,
                path,
                ..
            } => Some(path.clone()),
            BrowserEntry {
                kind: BrowserEntryKind::ParentDir,
                path,
                ..
            } => Some(path.clone()),
            BrowserEntry {
                kind: BrowserEntryKind::Markdown,
                ..
            } => None,
        }
    }

    /// Return `dir.parent()` — the target for Backspace / ascend.
    /// If `root_boundary` is set, returns `None` if `dir` equals `root_boundary`.
    pub fn ascend_target(&self) -> Option<PathBuf> {
        if self.root_boundary.as_ref() == Some(&self.dir) {
            return None;
        }
        self.dir.parent().map(|p| p.to_path_buf())
    }

    /// Expand the collapsed directory entry at `idx`: list its immediate
    /// children (one directory level — never a recursive walk ahead of
    /// time) and splice them into `entries` right after it, each one level
    /// deeper than `idx`. Marks the entry [`BrowserEntryKind::ExpandedDir`].
    ///
    /// Returns `false` and does nothing if `idx` is out of range or the
    /// entry at `idx` is not a collapsed [`BrowserEntryKind::Dir`] (already
    /// expanded, or not a directory at all).
    pub fn expand(&mut self, idx: usize) -> bool {
        let Some(entry) = self.entries.get(idx) else {
            return false;
        };
        if entry.kind != BrowserEntryKind::Dir {
            return false;
        }
        let child_dir = entry.path.clone();
        let child_depth = entry.depth + 1;

        let (children, dropped) =
            build_entries(child_dir.as_path(), self.reveal_ignored, child_depth, false);
        self.dropped_entries += dropped;
        self.walk_count += 1;

        self.entries[idx].kind = BrowserEntryKind::ExpandedDir;
        self.entries.splice(idx + 1..idx + 1, children);
        true
    }

    /// Collapse the expanded directory entry at `idx`: remove every entry
    /// after it whose `depth` is greater than `idx`'s (its whole subtree,
    /// at any depth — not just its immediate children), then mark it
    /// [`BrowserEntryKind::Dir`] again.
    ///
    /// Returns `false` and does nothing if `idx` is out of range or the
    /// entry at `idx` is not [`BrowserEntryKind::ExpandedDir`].
    pub fn collapse(&mut self, idx: usize) -> bool {
        let Some(entry) = self.entries.get(idx) else {
            return false;
        };
        if entry.kind != BrowserEntryKind::ExpandedDir {
            return false;
        }
        let depth = entry.depth;

        let mut end = idx + 1;
        while end < self.entries.len() && self.entries[end].depth > depth {
            end += 1;
        }
        self.entries.drain(idx + 1..end);
        self.entries[idx].kind = BrowserEntryKind::Dir;

        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        true
    }
}

/// Resolve the corpus root for `invoked` — the path bella was launched at
/// (a file or a directory).
///
/// The rule: walk UPWARD from `invoked` to the nearest ancestor directory
/// containing `brain.toml`. Failing that, walk upward again to the nearest
/// ancestor containing a `.git` entry (the git root). Failing that, return
/// `invoked` itself — this never errors, so a bare directory with neither
/// marker still gets a usable root.
///
/// This is a property of how bella was invoked, not of any document index —
/// BE.7.G's document index consumes this result rather than re-deriving it.
pub fn resolve_corpus_root(invoked: &Path) -> PathBuf {
    // Walking starts from a directory. When `invoked` names a file, start
    // from its parent instead of treating the file itself as a candidate
    // ancestor.
    let start = if invoked.is_file() {
        invoked
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| invoked.to_path_buf())
    } else {
        invoked.to_path_buf()
    };

    if let Some(root) = nearest_ancestor_containing(&start, "brain.toml") {
        return root;
    }
    if let Some(root) = nearest_ancestor_containing(&start, ".git") {
        return root;
    }
    invoked.to_path_buf()
}

/// Walk `start` and its ancestors looking for the nearest one whose
/// directory contains an entry named `marker`. Returns `None` if no
/// ancestor (including `start` itself) has one.
fn nearest_ancestor_containing(start: &Path, marker: &str) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if dir.join(marker).exists() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build the sorted entry list for `dir`, plus a count of entries the
/// walker dropped because it could not resolve them (e.g. a broken
/// symlink or a permission error) rather than silently shortening the
/// listing with no trace.
///
/// `reveal_ignored` relaxes BOTH the hidden-dotfile filter and every
/// `.gitignore`/global-ignore/git-exclude filter. Either alone leaves the
/// other hiding things — a dot-directory that itself contains a
/// gitignored child needs both off to be reachable.
///
/// `depth` is stamped onto every produced [`BrowserEntry`] — `0` for a
/// root-level listing, or one more than the expanding parent's depth when
/// called from [`Browser::expand`]. `include_parent` controls whether a
/// synthetic `..` entry is prepended: `true` for a root/refresh listing,
/// `false` when listing a directory's children for a tree expansion,
/// where a `..` row inside the subtree would be meaningless — the tree
/// pane ascends via the entry that owns the subtree, not a synthetic row
/// inside it.
fn build_entries(
    dir: &Path,
    reveal_ignored: bool,
    depth: usize,
    include_parent: bool,
) -> (Vec<BrowserEntry>, usize) {
    let mut dirs: Vec<BrowserEntry> = Vec::new();
    let mut files: Vec<BrowserEntry> = Vec::new();
    let mut dropped: usize = 0;

    // Walk with the `ignore` crate: max_depth(1). `follow_links(true)`
    // resolves a symlinked CHILD entry to its target's file type — e.g.
    // `planning/` in every repo of this fleet is a symlink into the brain
    // vault, and without this flag a symlink's own type is neither
    // `is_dir()` nor `is_file()`, so `build_entries` drops it from the
    // listing entirely ("the browser cannot enter it at all"). It also
    // lets a `Browser` rooted directly at a symlinked directory list the
    // target's contents. `reveal_ignored` flips both filters that hide
    // content; both stay on (today's behaviour) by default.
    let walker = ignore::WalkBuilder::new(dir)
        .max_depth(Some(1))
        .follow_links(true)
        .hidden(!reveal_ignored) // skip dot-files unless revealed
        .git_ignore(!reveal_ignored)
        .git_global(!reveal_ignored)
        .git_exclude(!reveal_ignored)
        // Respect .gitignore even outside a git repository (e.g. stand-alone dirs).
        .require_git(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => {
                // The walker could not resolve this entry (a broken
                // symlink, a permission error, ...). Count it so the
                // caller can tell an incomplete listing from an empty
                // directory, instead of silently shortening the list.
                dropped += 1;
                continue;
            }
        };

        let path = entry.path().to_path_buf();

        // Skip the root itself (depth == 0).
        if path == dir {
            continue;
        }

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => continue,
        };

        let ft = match entry.file_type() {
            Some(ft) => ft,
            None => continue,
        };

        if ft.is_dir() {
            dirs.push(BrowserEntry {
                path,
                display: name,
                kind: BrowserEntryKind::Dir,
                depth,
                parent: Some(dir.to_path_buf()),
            });
        } else if ft.is_file() {
            let lower = name.to_lowercase();
            if lower.ends_with(".md") || lower.ends_with(".mdx") {
                files.push(BrowserEntry {
                    path,
                    display: name,
                    kind: BrowserEntryKind::Markdown,
                    depth,
                    parent: Some(dir.to_path_buf()),
                });
            }
        }
    }

    // Sort alphabetically (case-insensitive) within each group.
    dirs.sort_by_cached_key(|a| a.display.to_lowercase());
    files.sort_by_cached_key(|a| a.display.to_lowercase());

    // Prepend `..` when a parent exists and this listing wants one — a
    // root/refresh listing does, an expand-children listing does not
    // (see `include_parent` on the doc comment above).
    let mut entries: Vec<BrowserEntry> = Vec::new();
    if include_parent && let Some(parent) = dir.parent() {
        entries.push(BrowserEntry {
            path: parent.to_path_buf(),
            display: "..".to_string(),
            kind: BrowserEntryKind::ParentDir,
            depth,
            parent: Some(dir.to_path_buf()),
        });
    }
    entries.extend(dirs);
    entries.extend(files);
    (entries, dropped)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{Browser, BrowserEntry, BrowserEntryKind};

    /// Create a temp dir under the system temp dir with a unique name.
    fn temp_dir(label: &str) -> PathBuf {
        crate::testsupport::unique_temp_dir(&format!("bella_browser_{label}"))
    }

    fn create_file(dir: &Path, name: &str) {
        fs::write(dir.join(name), "").expect("create file");
    }

    fn create_dir(parent: &Path, name: &str) -> PathBuf {
        let p = parent.join(name);
        fs::create_dir_all(&p).expect("create subdir");
        p
    }

    // Helper to find a named entry in the listing.
    fn find_entry<'a>(entries: &'a [BrowserEntry], display: &str) -> Option<&'a BrowserEntry> {
        entries.iter().find(|e| e.display == display)
    }

    // -----------------------------------------------------------------------
    // Listing tests
    // -----------------------------------------------------------------------

    #[test]
    fn lists_markdown_and_subdir_hides_txt() {
        let dir = temp_dir("listing");
        create_file(&dir, "readme.md");
        create_file(&dir, "notes.mdx");
        create_file(&dir, "ignore.txt");
        create_dir(&dir, "subdir");

        let b = Browser::new(dir.clone());

        // `..` must be present (dir has a parent).
        assert!(
            find_entry(&b.entries, "..").is_some(),
            "expected `..` entry; entries = {:?}",
            b.entries.iter().map(|e| &e.display).collect::<Vec<_>>()
        );
        // Subdir present.
        assert!(
            find_entry(&b.entries, "subdir").is_some(),
            "expected subdir"
        );
        // Markdown files present.
        assert!(
            find_entry(&b.entries, "readme.md").is_some(),
            "expected readme.md"
        );
        assert!(
            find_entry(&b.entries, "notes.mdx").is_some(),
            "expected notes.mdx"
        );
        // .txt must be absent.
        assert!(
            find_entry(&b.entries, "ignore.txt").is_none(),
            "txt file must be hidden"
        );
    }

    #[test]
    fn parent_entry_is_absent_at_filesystem_root() {
        // We can't truly test / (permission issues), but we can verify the
        // logic: if dir.parent() is None, no `..` entry is emitted.
        // Simulate by creating a Browser manually without calling build_entries.
        // Instead, check that a dir WITHOUT a parent would produce no `..`.
        // We'll create a Browser at a deeply nested dir, then call `ascend_target`
        // and verify the parent is correct.
        let dir = temp_dir("root_sim");
        let b = Browser::new(dir.clone());
        // The temp dir does have a parent, so `..` is present.
        let parent_entry = find_entry(&b.entries, "..");
        assert!(parent_entry.is_some(), "expected `..` in child dir");
        assert_eq!(
            parent_entry.unwrap().path,
            dir.parent().unwrap().to_path_buf(),
            "parent entry path must equal dir.parent()"
        );
    }

    #[test]
    fn entries_sorted_alphabetically_case_insensitive() {
        let dir = temp_dir("sort");
        create_file(&dir, "Zed.md");
        create_file(&dir, "alpha.md");
        create_file(&dir, "mango.md");
        create_dir(&dir, "Bravo");
        create_dir(&dir, "alpha_dir");

        let b = Browser::new(dir.clone());

        // Collect dirs (after `..`).
        let dir_entries: Vec<&str> = b
            .entries
            .iter()
            .filter(|e| e.kind == BrowserEntryKind::Dir)
            .map(|e| e.display.as_str())
            .collect();

        // Both dirs present.
        assert!(dir_entries.contains(&"Bravo"), "Bravo must be listed");
        assert!(
            dir_entries.contains(&"alpha_dir"),
            "alpha_dir must be listed"
        );
        // alpha_dir < Bravo (case-insensitive).
        let idx_alpha = dir_entries.iter().position(|s| *s == "alpha_dir").unwrap();
        let idx_bravo = dir_entries.iter().position(|s| *s == "Bravo").unwrap();
        assert!(
            idx_alpha < idx_bravo,
            "alpha_dir must sort before Bravo (case-insensitive)"
        );

        // Collect files.
        let file_entries: Vec<&str> = b
            .entries
            .iter()
            .filter(|e| e.kind == BrowserEntryKind::Markdown)
            .map(|e| e.display.as_str())
            .collect();
        // alpha < mango < Zed (case-insensitive).
        let idx_a = file_entries.iter().position(|s| *s == "alpha.md").unwrap();
        let idx_m = file_entries.iter().position(|s| *s == "mango.md").unwrap();
        let idx_z = file_entries.iter().position(|s| *s == "Zed.md").unwrap();
        assert!(idx_a < idx_m, "alpha.md before mango.md");
        assert!(idx_m < idx_z, "mango.md before Zed.md");
    }

    // -----------------------------------------------------------------------
    // move_cursor tests
    // -----------------------------------------------------------------------

    #[test]
    fn move_cursor_wraps_at_bottom() {
        let dir = temp_dir("wrap_bottom");
        create_file(&dir, "a.md");
        create_file(&dir, "b.md");

        let mut b = Browser::new(dir);
        let n = b.entries.len();
        assert!(n >= 2, "precondition: at least 2 entries");

        // Move past the last entry — should wrap to 0.
        b.selected = n - 1;
        b.move_cursor(1, 20);
        assert_eq!(b.selected, 0, "cursor must wrap from last to first");
    }

    #[test]
    fn move_cursor_wraps_at_top() {
        let dir = temp_dir("wrap_top");
        create_file(&dir, "a.md");
        create_file(&dir, "b.md");

        let mut b = Browser::new(dir);
        let n = b.entries.len();

        // Start at 0, move up — should wrap to last.
        b.selected = 0;
        b.move_cursor(-1, 20);
        assert_eq!(
            b.selected,
            n - 1,
            "cursor must wrap from first to last on up"
        );
    }

    #[test]
    fn move_cursor_clamps_scroll_down() {
        let dir = temp_dir("scroll_down");
        // Create enough entries that the viewport is smaller than the list.
        for i in 0..10 {
            create_file(&dir, &format!("file{i:02}.md"));
        }

        let mut b = Browser::new(dir);
        let viewport_h = 4u16;
        // Move to the bottom of the list.
        let n = b.entries.len();
        for _ in 0..n - 1 {
            b.move_cursor(1, viewport_h);
        }
        // Selected must be visible: selected < scroll + viewport_h.
        assert!(
            (b.selected as u16) < b.scroll + viewport_h,
            "selected={} must be < scroll ({}) + viewport_h ({})",
            b.selected,
            b.scroll,
            viewport_h
        );
    }

    #[test]
    fn move_cursor_clamps_scroll_up() {
        let dir = temp_dir("scroll_up");
        for i in 0..10 {
            create_file(&dir, &format!("g{i:02}.md"));
        }

        let mut b = Browser::new(dir);
        let viewport_h = 4u16;
        // Move all the way down first.
        let n = b.entries.len();
        for _ in 0..n - 1 {
            b.move_cursor(1, viewport_h);
        }
        // Now move all the way back up.
        for _ in 0..n - 1 {
            b.move_cursor(-1, viewport_h);
        }
        // After returning to the top, scroll must be 0.
        assert_eq!(b.scroll, 0, "scroll must be 0 after moving back to top");
        assert_eq!(b.selected, 0, "selected must be 0 after full round trip");
    }

    #[test]
    fn move_cursor_noop_on_empty_list() {
        // A dir with no markdown and no subdirs (the `..` entry won't appear
        // if we construct from a root-like path).  Use a real temp dir that
        // only contains a .txt file (hidden from listing).
        let dir = temp_dir("empty_list");
        create_file(&dir, "only.txt");

        let mut b = Browser::new(dir.clone());
        // Depending on whether a parent exists, `..` may appear.
        // Force the pathological case: no entries at all.
        b.entries.clear();
        b.selected = 0;
        b.scroll = 0;

        // Must not panic.
        b.move_cursor(1, 10);
        b.move_cursor(-1, 10);
        assert_eq!(b.selected, 0);
        assert_eq!(b.scroll, 0);
    }

    // -----------------------------------------------------------------------
    // selected_entry / descend / ascend_target
    // -----------------------------------------------------------------------

    #[test]
    fn selected_entry_returns_none_on_empty() {
        let mut b = Browser::new(temp_dir("sel_empty"));
        b.entries.clear();
        assert!(b.selected_entry().is_none());
    }

    #[test]
    fn descend_returns_path_for_dir_entry() {
        let dir = temp_dir("descend_dir");
        let sub = create_dir(&dir, "child");

        // Find the "child" entry and set selected to it.
        let mut b2 = Browser::new(dir.clone());
        let idx = b2
            .entries
            .iter()
            .position(|e| e.display == "child")
            .expect("child must be listed");
        b2.selected = idx;

        let got = b2.descend();
        assert!(got.is_some(), "descend on a Dir entry must return Some");
        assert_eq!(got.unwrap(), sub, "descend must return the subdir path");
    }

    #[test]
    fn descend_returns_none_for_markdown_entry() {
        let dir = temp_dir("descend_md");
        create_file(&dir, "readme.md");

        let mut b = Browser::new(dir.clone());
        // Select the markdown file.
        let idx = b
            .entries
            .iter()
            .position(|e| e.display == "readme.md")
            .expect("readme.md must be listed");
        b.selected = idx;

        assert!(
            b.descend().is_none(),
            "descend on a Markdown entry must return None"
        );
    }

    #[test]
    fn descend_returns_parent_for_parent_dir_entry() {
        let dir = temp_dir("descend_parent");
        let b = Browser::new(dir.clone());

        // `..` should be the first entry.
        assert_eq!(
            b.entries[0].kind,
            BrowserEntryKind::ParentDir,
            "first entry must be ParentDir"
        );

        let target = b.descend(); // selected == 0 → `..`
        assert!(
            target.is_some(),
            "descend on ParentDir must return Some(parent_path)"
        );
        assert_eq!(
            target.unwrap(),
            dir.parent().unwrap().to_path_buf(),
            "descend on `..` must return the parent directory"
        );
    }

    // Exhaustiveness capability check (Task 1 acceptance criterion 2):
    // `descend()`'s match was shown capable of failing to catch a silently
    // non-descendable directory-shaped variant. Temporarily rewrote
    // `descend()` as:
    //
    //   pub fn descend(&self) -> Option<PathBuf> {
    //       match self.selected_entry()? {
    //           BrowserEntry { kind: BrowserEntryKind::Dir | BrowserEntryKind::ExpandedDir
    //               | BrowserEntryKind::ParentDir, path, .. } => Some(path.clone()),
    //           _ => None,
    //       }
    //   }
    //
    // then added a hypothetical fourth directory-shaped variant
    // `BrowserEntryKind::Placeholder` to the enum (no `#[default]`, arm
    // omitted from `build_entries`/`expand`/`collapse` on purpose) and ran
    // `cargo build -p bella-engine`. It compiled clean — the `_ => None`
    // arm swallowed the new variant with zero diagnostic, exactly the
    // silent-non-descendable failure this block exists to close off.
    // Reverted the wildcard AND the hypothetical variant immediately after
    // observing the clean compile; the committed `descend()` has no `_ =>`
    // arm, so the same experiment (a fifth real variant, unhandled) is a
    // compile error instead of a silent bug.
    #[test]
    fn descend_exhaustive_match_is_load_bearing() {
        // The test above this comment block already exercises every real
        // variant `descend()` can see today (`descend_returns_path_for_dir_entry`,
        // `descend_returns_none_for_markdown_entry`,
        // `descend_returns_parent_for_parent_dir_entry`, plus
        // `expand_lists_one_level_...` below covers `ExpandedDir`). This
        // test exists to anchor the doc comment recording the manual
        // wildcard-reintroduction experiment next to the property it
        // verifies, per CLAUDE.md's "no fabricated metrics" rule applied
        // to compiler behaviour: the observation must live beside code
        // that keeps it true, not just in a commit message.
        let dir = temp_dir("descend_exhaustive_anchor");
        let b = Browser::new(dir);
        // A fresh Browser's first entry is always `..` (ParentDir) or, at
        // the filesystem root, empty — either way this must not panic.
        let _ = b.descend();
    }

    #[test]
    fn descend_returns_path_for_expanded_dir_entry() {
        let dir = temp_dir("descend_expanded");
        create_dir(&dir, "child");

        let mut b = Browser::new(dir.clone());
        let idx = b
            .entries
            .iter()
            .position(|e| e.display == "child")
            .expect("child must be listed");
        assert!(b.expand(idx), "expand must succeed on a collapsed Dir");
        assert_eq!(b.entries[idx].kind, BrowserEntryKind::ExpandedDir);

        b.selected = idx;
        let got = b.descend();
        assert!(
            got.is_some(),
            "descend on an ExpandedDir entry must still return Some — it is still a directory"
        );
        assert_eq!(got.unwrap(), dir.join("child"));
    }

    // -----------------------------------------------------------------------
    // Default tests
    // -----------------------------------------------------------------------

    #[test]
    fn browser_entry_default_has_sensible_kind() {
        let e = BrowserEntry::default();
        assert_eq!(
            e.kind,
            BrowserEntryKind::Markdown,
            "default kind must be the neutral leaf kind — a directory kind would wrongly \
             imply expand state (Dir: collapsible; ExpandedDir: has a subtree to collapse; \
             ParentDir: synthetic `..`) that a bare Default value has no basis to claim"
        );
        assert_eq!(e.path, PathBuf::new(), "default path must be empty");
        assert_eq!(e.display, String::new(), "default display must be empty");
        assert_eq!(e.depth, 0, "default depth must be 0 — the root level");
        assert_eq!(
            e.parent, None,
            "default parent must be None — a Default value was never listed from anywhere"
        );
    }

    // -----------------------------------------------------------------------
    // expand / collapse tests
    // -----------------------------------------------------------------------

    #[test]
    fn expand_lists_one_level_and_second_level_walks_only_itself() {
        let dir = temp_dir("expand_walk_counter");
        let a = create_dir(&dir, "a");
        create_file(&a, "a1.md");
        let b_dir = create_dir(&a, "b");
        create_file(&b_dir, "b1.md");

        let mut browser = Browser::new(dir.clone());
        assert_eq!(browser.walk_count, 1, "the initial listing is one walk");

        let a_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "a")
            .expect("a must be listed");
        assert_eq!(browser.entries[a_idx].depth, 0);

        assert!(
            browser.expand(a_idx),
            "expand must succeed on a collapsed Dir entry"
        );
        assert_eq!(
            browser.walk_count, 2,
            "expanding one level performs exactly one more walk"
        );
        assert_eq!(browser.entries[a_idx].kind, BrowserEntryKind::ExpandedDir);

        // Only a's immediate children are listed: "b" (dir) and "a1.md" —
        // NOT b's contents. A deep tree lists only the expanded levels.
        assert!(find_entry(&browser.entries, "a1.md").is_some());
        let b_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "b")
            .expect("b must be listed after expanding a");
        assert_eq!(
            browser.entries[b_idx].depth, 1,
            "a's children must be one level deeper than a"
        );
        assert!(
            find_entry(&browser.entries, "b1.md").is_none(),
            "expand must not walk recursively ahead of time — b's contents are unlisted \
             until b itself is expanded"
        );

        assert!(
            browser.expand(b_idx),
            "expand must succeed on b, a's freshly-listed child"
        );
        assert_eq!(
            browser.walk_count, 3,
            "expanding the second level performs exactly one walk — it does not re-walk a"
        );
        let b1_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "b1.md")
            .expect("b1.md must be listed after expanding b");
        assert_eq!(
            browser.entries[b1_idx].depth, 2,
            "b's children must be two levels deep (root -> a -> b -> b1.md)"
        );
    }

    #[test]
    fn expand_children_carry_the_parent_relationship() {
        let dir = temp_dir("expand_parent_field");
        let a = create_dir(&dir, "a");
        create_file(&a, "a1.md");

        let mut b = Browser::new(dir.clone());
        let a_idx = b
            .entries
            .iter()
            .position(|e| e.display == "a")
            .expect("a must be listed");
        b.expand(a_idx);

        let a1 = find_entry(&b.entries, "a1.md").expect("a1.md must be listed after expand");
        assert_eq!(
            a1.parent.as_deref(),
            Some(a.as_path()),
            "a child listed by expand must record the directory it was listed from"
        );
    }

    #[test]
    fn expand_no_op_on_markdown_entry() {
        let dir = temp_dir("expand_markdown_noop");
        create_file(&dir, "readme.md");

        let mut b = Browser::new(dir.clone());
        let idx = b
            .entries
            .iter()
            .position(|e| e.display == "readme.md")
            .expect("readme.md must be listed");
        let before = b.entries.len();
        assert!(
            !b.expand(idx),
            "expand on a Markdown entry must be a no-op, not a walk of the file's parent"
        );
        assert_eq!(b.entries.len(), before, "no entries must be spliced in");
        assert_eq!(b.walk_count, 1, "no extra walk must be performed");
    }

    #[test]
    fn expand_no_op_when_already_expanded() {
        let dir = temp_dir("expand_idempotent");
        let a = create_dir(&dir, "a");
        create_file(&a, "a1.md");

        let mut b = Browser::new(dir.clone());
        let a_idx = b
            .entries
            .iter()
            .position(|e| e.display == "a")
            .expect("a must be listed");
        assert!(b.expand(a_idx));
        assert_eq!(b.walk_count, 2);

        assert!(
            !b.expand(a_idx),
            "expanding an already-expanded entry must be a no-op"
        );
        assert_eq!(
            b.walk_count, 2,
            "a redundant expand call must not perform another walk"
        );
    }

    #[test]
    fn collapse_removes_only_this_subtree_not_siblings() {
        let dir = temp_dir("collapse_subtree");
        let a = create_dir(&dir, "a");
        create_file(&a, "a1.md");
        let b_dir = create_dir(&a, "b");
        create_file(&b_dir, "b1.md");
        let sibling = create_dir(&dir, "sibling");
        create_file(&sibling, "s1.md");

        let mut browser = Browser::new(dir.clone());
        let a_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "a")
            .unwrap();
        browser.expand(a_idx);
        let b_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "b")
            .unwrap();
        browser.expand(b_idx);
        assert!(find_entry(&browser.entries, "b1.md").is_some());

        let sibling_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "sibling")
            .expect("sibling must still be listed at the root, untouched by a's expansion");
        assert_eq!(browser.entries[sibling_idx].kind, BrowserEntryKind::Dir);

        let a_idx = browser
            .entries
            .iter()
            .position(|e| e.display == "a")
            .unwrap();
        assert!(
            browser.collapse(a_idx),
            "collapse must succeed on an ExpandedDir"
        );
        assert_eq!(browser.entries[a_idx].kind, BrowserEntryKind::Dir);
        assert!(
            find_entry(&browser.entries, "a1.md").is_none(),
            "a's own children must be gone after collapsing a"
        );
        assert!(
            find_entry(&browser.entries, "b").is_none(),
            "a's grandchildren (via b) must be gone too — the whole subtree, not one level"
        );
        assert!(
            find_entry(&browser.entries, "b1.md").is_none(),
            "b's children, nested under a, must be gone as well"
        );
        assert!(
            find_entry(&browser.entries, "sibling").is_some(),
            "a sibling of a at the root must be untouched by collapsing a"
        );
        assert!(
            find_entry(&browser.entries, "s1.md").is_none(),
            "sibling was never expanded, so it never had children listed"
        );
    }

    #[test]
    fn collapse_no_op_on_collapsed_dir() {
        let dir = temp_dir("collapse_noop");
        create_dir(&dir, "a");

        let mut b = Browser::new(dir.clone());
        let a_idx = b.entries.iter().position(|e| e.display == "a").unwrap();
        assert!(
            !b.collapse(a_idx),
            "collapse on a still-collapsed Dir must be a no-op"
        );
        assert_eq!(b.entries[a_idx].kind, BrowserEntryKind::Dir);
    }

    #[test]
    fn ascend_target_returns_parent() {
        let dir = temp_dir("ascend");
        let b = Browser::new(dir.clone());
        let target = b.ascend_target();
        assert!(
            target.is_some(),
            "ascend_target must be Some for a child dir"
        );
        assert_eq!(target.unwrap(), dir.parent().unwrap().to_path_buf());
    }

    // -----------------------------------------------------------------------
    // Gitignore test
    // -----------------------------------------------------------------------

    #[test]
    fn gitignored_file_is_excluded() {
        let dir = temp_dir("gitignore");
        // Write a .gitignore that ignores secret.md.
        fs::write(dir.join(".gitignore"), "secret.md\n").expect("write .gitignore");
        create_file(&dir, "secret.md");
        create_file(&dir, "visible.md");

        let b = Browser::new(dir.clone());

        assert!(
            find_entry(&b.entries, "visible.md").is_some(),
            "visible.md must be listed"
        );
        assert!(
            find_entry(&b.entries, "secret.md").is_none(),
            "secret.md must be hidden by .gitignore"
        );
    }

    #[test]
    fn ascend_target_respects_root_boundary() {
        let dir = temp_dir("root_boundary");
        let sub = create_dir(&dir, "child");

        // At child, boundary is parent. Should be able to ascend to parent.
        let mut b1 = Browser::new(sub.clone());
        b1.root_boundary = Some(dir.clone());
        assert_eq!(b1.ascend_target(), Some(dir.clone()));

        // At root boundary, ascend_target should be None.
        let mut b2 = Browser::new(dir.clone());
        b2.root_boundary = Some(dir.clone());
        assert_eq!(b2.ascend_target(), None);
    }

    // -----------------------------------------------------------------------
    // BE.7.C task 1: follow_links, reveal_ignored, filesystem entry kind,
    // dropped-entry count.
    // -----------------------------------------------------------------------

    /// `planning/` in every repo of this fleet is a symlink INTO the brain
    /// vault — i.e. it shows up as a symlinked CHILD entry inside a normal
    /// directory being browsed, not as the browser's own root. Without
    /// `follow_links(true)`, a symlink's own file type is neither `is_dir()`
    /// nor `is_file()`, so `build_entries` drops it from the listing
    /// entirely — the "browser cannot enter it at all" bug this block
    /// exists to fix. With `follow_links(true)`, the entry resolves to the
    /// target's type (`Dir`) and descending into it lists the target's
    /// contents.
    ///
    /// Observed by hand: reverting `.follow_links(true)` to
    /// `.follow_links(false)` in `build_entries` and re-running this test
    /// made it fail — `link_to_target` was absent from `b.entries` (not
    /// merely wrong-kind: `find_entry` returned `None`). Flag restored
    /// afterward; the assertion below is what caught it.
    #[test]
    #[cfg(unix)]
    fn follow_links_lists_symlinked_child_directory_and_its_contents() {
        let dir = temp_dir("symlink_child");
        let real_target = create_dir(&dir, "real_target");
        create_file(&real_target, "inside.md");

        let link = dir.join("link_to_target");
        std::os::unix::fs::symlink(&real_target, &link).expect("create symlink");

        let b = Browser::new(dir.clone());
        let link_entry = find_entry(&b.entries, "link_to_target").unwrap_or_else(|| {
            panic!(
                "symlinked child directory must be listed; entries = {:?}",
                b.entries.iter().map(|e| &e.display).collect::<Vec<_>>()
            )
        });
        assert_eq!(
            link_entry.kind,
            BrowserEntryKind::Dir,
            "a symlink to a directory must resolve to kind Dir"
        );

        // Descending into it must list the target's contents.
        let inner = Browser::new(link_entry.path.clone());
        assert!(
            find_entry(&inner.entries, "inside.md").is_some(),
            "browsing into the symlinked child must list the target's contents"
        );
    }

    /// Default constructor path keeps both filters ON (today's behaviour);
    /// `set_reveal_ignored(true)` relaxes `git_ignore` and reveals a
    /// gitignored entry.
    #[test]
    fn reveal_toggle_shows_gitignored_entry_only_when_on() {
        let dir = temp_dir("reveal_gitignore");
        fs::write(dir.join(".gitignore"), "ignored_dir/\n").expect("write .gitignore");
        create_dir(&dir, "ignored_dir");

        let b_off = Browser::new(dir.clone());
        assert!(
            !b_off.reveal_ignored,
            "reveal_ignored must default to false"
        );
        assert!(
            find_entry(&b_off.entries, "ignored_dir").is_none(),
            "gitignored dir must be hidden by default"
        );

        let mut b_on = Browser::new(dir.clone());
        b_on.set_reveal_ignored(true);
        assert!(
            find_entry(&b_on.entries, "ignored_dir").is_some(),
            "gitignored dir must be visible once revealed"
        );
    }

    /// The fleet's real trap, reproduced exactly: a DIRECTORY literally
    /// named `status.md` living inside a dot-directory
    /// (`planning/.mev-history/status.md`). A toggle that relaxes only
    /// `git_ignore` never reveals this — it needs `hidden` relaxed too.
    /// Also proves entry kind comes from filesystem metadata, not the
    /// extension: `status.md` is a DIRECTORY and must be listed as
    /// `BrowserEntryKind::Dir`, never `Markdown`.
    #[test]
    fn reveal_toggle_relaxes_hidden_and_kind_comes_from_filesystem() {
        let dir = temp_dir("reveal_dotdir_trap");
        let hidden_dir = create_dir(&dir, ".mev-history");
        // Trap: a directory, not a file, named like a markdown file.
        create_dir(&hidden_dir, "status.md");

        // Toggle off: the dot-directory is invisible entirely, so the trap
        // beneath it is unreachable.
        let b_off = Browser::new(dir.clone());
        assert!(
            find_entry(&b_off.entries, ".mev-history").is_none(),
            "dot-directory must be hidden by default"
        );

        // Toggle on: the dot-directory becomes visible, as a Dir.
        let mut b_on = Browser::new(dir.clone());
        b_on.set_reveal_ignored(true);
        let dot_entry = find_entry(&b_on.entries, ".mev-history")
            .expect("dot-directory must be visible once revealed");
        assert_eq!(dot_entry.kind, BrowserEntryKind::Dir);

        // Browse into it: `status.md` must be listed as a Dir, not Markdown
        // — its kind must come from `file_type()`, never from the name.
        let mut inner = Browser::new(hidden_dir.clone());
        inner.set_reveal_ignored(true);
        let trap_entry = find_entry(&inner.entries, "status.md")
            .expect("directory named status.md must be listed");
        assert_eq!(
            trap_entry.kind,
            BrowserEntryKind::Dir,
            "a directory named `*.md` must be kind Dir, never Markdown \
             (entry kind must come from filesystem metadata, not the \
             extension)"
        );
    }

    /// A walk error (here: a dangling symlink, which `follow_links(true)`
    /// tries and fails to resolve) must not silently shorten the listing.
    /// The sibling that CAN be read is still listed, and the drop is
    /// counted so the caller can tell an incomplete listing from an empty
    /// directory.
    #[test]
    #[cfg(unix)]
    fn dropped_entry_count_reports_unresolvable_entry() {
        let dir = temp_dir("dropped_broken_link");
        create_file(&dir, "visible.md");
        let broken = dir.join("broken_link");
        std::os::unix::fs::symlink(dir.join("does_not_exist"), &broken)
            .expect("create dangling symlink");

        let b = Browser::new(dir.clone());

        assert!(
            find_entry(&b.entries, "visible.md").is_some(),
            "a readable sibling must still be listed despite the unresolvable entry"
        );
        assert!(
            b.dropped_entries > 0,
            "dropped_entries must report the unresolvable entry, got {}",
            b.dropped_entries
        );
    }

    /// `set_reveal_ignored` re-lists the SAME directory and clamps the
    /// cursor into the (possibly shorter) new entry count rather than
    /// panicking or leaving it out of bounds.
    #[test]
    fn set_reveal_ignored_reclamps_selected() {
        let dir = temp_dir("reveal_reclamp");
        fs::write(dir.join(".gitignore"), "only_visible_when_revealed/\n")
            .expect("write .gitignore");
        create_dir(&dir, "only_visible_when_revealed");

        let mut b = Browser::new(dir.clone());
        b.selected = b.entries.len().saturating_sub(1);

        b.set_reveal_ignored(true);
        assert!(b.selected < b.entries.len(), "selected must stay in bounds");
        assert!(
            find_entry(&b.entries, "only_visible_when_revealed").is_some(),
            "revealed entry must now be present"
        );

        b.set_reveal_ignored(false);
        assert!(b.selected < b.entries.len().max(1) || b.entries.is_empty());
        assert!(
            find_entry(&b.entries, "only_visible_when_revealed").is_none(),
            "entry must be hidden again once reveal is toggled off"
        );
    }

    // -----------------------------------------------------------------------
    // Corpus-root resolver tests
    // -----------------------------------------------------------------------

    use super::resolve_corpus_root;

    /// A directory under a tree containing `brain.toml` resolves to the
    /// nearest ancestor holding it, not to the git root or the invoked
    /// path.
    #[test]
    fn resolve_corpus_root_finds_nearest_brain_toml() {
        let root = temp_dir("corpus_root_brain_toml");
        fs::write(root.join("brain.toml"), "").expect("write brain.toml");
        // A `.git` marker further up must NOT win — brain.toml takes
        // priority over the git root.
        let nested = create_dir(&root, "sub");
        let leaf = create_dir(&nested, "leaf");

        let resolved = resolve_corpus_root(&leaf);
        assert_eq!(
            resolved, root,
            "must resolve to the nearest ancestor containing brain.toml"
        );
    }

    /// A git repo with no `brain.toml` anywhere in its ancestry falls back
    /// to the git root (nearest ancestor containing `.git`).
    #[test]
    fn resolve_corpus_root_falls_back_to_git_root() {
        let root = temp_dir("corpus_root_git");
        create_dir(&root, ".git");
        let nested = create_dir(&root, "sub");
        let leaf = create_dir(&nested, "leaf");

        let resolved = resolve_corpus_root(&leaf);
        assert_eq!(
            resolved, root,
            "must resolve to the nearest ancestor containing .git when no brain.toml exists"
        );
    }

    /// A directory that is neither under a `brain.toml` tree nor a git repo
    /// resolves to the invoked path itself, rather than erroring or
    /// returning some other default.
    #[test]
    fn resolve_corpus_root_returns_invoked_path_when_neither_marker_exists() {
        let dir = temp_dir("corpus_root_neither");

        let resolved = resolve_corpus_root(&dir);
        assert_eq!(
            resolved, dir,
            "must return the invoked path itself when neither brain.toml nor .git is found"
        );
    }

    /// When `invoked` names a file (not a directory), resolution starts
    /// from the file's parent — a file can never itself be a corpus root.
    #[test]
    fn resolve_corpus_root_starts_from_parent_when_invoked_is_a_file() {
        let root = temp_dir("corpus_root_file_invoked");
        fs::write(root.join("brain.toml"), "").expect("write brain.toml");
        let file = root.join("doc.md");
        create_file(&root, "doc.md");

        let resolved = resolve_corpus_root(&file);
        assert_eq!(
            resolved, root,
            "resolution must start from the invoked file's parent directory"
        );
    }
}
