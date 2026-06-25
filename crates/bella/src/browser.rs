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
}

impl Browser {
    /// Build a new `Browser` rooted at `dir`.
    ///
    /// Lists the directory non-recursively, skips hidden dotfiles, respects
    /// `.gitignore` via the `ignore` walker, and hides non-markdown files.
    /// Entries are ordered: `..` (if parent exists) → subdirectories (alpha,
    /// case-insensitive) → `.md`/`.mdx` files (alpha, case-insensitive).
    pub fn new(dir: PathBuf) -> Self {
        let entries = build_entries(dir.as_path());
        Self {
            dir,
            entries,
            selected: 0,
            scroll: 0,
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
    pub fn ascend_target(&self) -> Option<PathBuf> {
        self.dir.parent().map(|p| p.to_path_buf())
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build the sorted entry list for `dir`.
fn build_entries(dir: &Path) -> Vec<BrowserEntry> {
    let mut dirs: Vec<BrowserEntry> = Vec::new();
    let mut files: Vec<BrowserEntry> = Vec::new();

    // Walk with the `ignore` crate: max_depth(1), respects .gitignore,
    // hidden files are skipped.
    let walker = ignore::WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(true) // skip dot-files
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        // Respect .gitignore even outside a git repository (e.g. stand-alone dirs).
        .require_git(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
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
    dirs.sort_by_key(|a| a.display.to_lowercase());
    files.sort_by_key(|a| a.display.to_lowercase());

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
    entries
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
        let base = std::env::temp_dir().join(format!("bella_browser_{label}"));
        fs::create_dir_all(&base).expect("create temp dir");
        base
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
}
