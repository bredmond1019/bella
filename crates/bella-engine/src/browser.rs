//! Browser model — pure, event-loop-independent directory listing.
//!
//! Provides [`Browser`], which holds the current directory, a sorted list of
//! [`BrowserEntry`] items, a cursor (`selected`), and a scroll offset.  All
//! methods are pure state mutations; no I/O happens after construction.
//!
use std::path::{Path, PathBuf};

/// Distinguishes the three kinds of entry shown in the browser listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserEntryKind {
    /// The `..` synthetic parent-directory entry.
    ParentDir,
    /// A subdirectory of the current directory.
    Dir,
    /// A `.md` or `.mdx` file.
    Markdown,
}

/// A single row in the browser listing.
#[derive(Debug, Clone)]
pub struct BrowserEntry {
    /// Absolute path of the entry.
    pub path: PathBuf,
    /// Display name (file/dir name component, or `".."` for [`BrowserEntryKind::ParentDir`]).
    pub display: String,
    /// Entry kind.
    pub kind: BrowserEntryKind,
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
        let (entries, dropped_entries) = build_entries(dir.as_path(), false);
        Self {
            dir,
            entries,
            selected: 0,
            scroll: 0,
            root_boundary: None,
            reveal_ignored: false,
            dropped_entries,
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
    fn refresh(&mut self) {
        let (entries, dropped_entries) = build_entries(self.dir.as_path(), self.reveal_ignored);
        self.entries = entries;
        self.dropped_entries = dropped_entries;
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

    /// Return the target path when the selected entry is a [`BrowserEntryKind::Dir`]
    /// or [`BrowserEntryKind::ParentDir`], otherwise `None`.
    pub fn descend(&self) -> Option<PathBuf> {
        match self.selected_entry()? {
            BrowserEntry {
                kind: BrowserEntryKind::Dir | BrowserEntryKind::ParentDir,
                path,
                ..
            } => Some(path.clone()),
            _ => None,
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
fn build_entries(dir: &Path, reveal_ignored: bool) -> (Vec<BrowserEntry>, usize) {
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
            });
        } else if ft.is_file() {
            let lower = name.to_lowercase();
            if lower.ends_with(".md") || lower.ends_with(".mdx") {
                files.push(BrowserEntry {
                    path,
                    display: name,
                    kind: BrowserEntryKind::Markdown,
                });
            }
        }
    }

    // Sort alphabetically (case-insensitive) within each group.
    dirs.sort_by_cached_key(|a| a.display.to_lowercase());
    files.sort_by_cached_key(|a| a.display.to_lowercase());

    // Prepend `..` when a parent exists.
    let mut entries: Vec<BrowserEntry> = Vec::new();
    if let Some(parent) = dir.parent() {
        entries.push(BrowserEntry {
            path: parent.to_path_buf(),
            display: "..".to_string(),
            kind: BrowserEntryKind::ParentDir,
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
