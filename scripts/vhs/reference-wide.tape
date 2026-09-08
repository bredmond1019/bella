# reference-wide.tape — full reference screenshot set at a wide terminal size.
# Output: planning/artifacts/screenshots/wide_*.png (gitignored generated dir;
# the PNGs themselves are committed as the reference set — see that
# directory's own note).
#
# PREREQUISITE: run `cargo build --release` before this tape. Every scene
# below drives target/release/bella directly, NOT cargo's own dev-build-and-
# run subcommand. Under that subcommand, compilation happens INSIDE the
# tape's settle window, so under concurrent cargo lanes (other agents
# building at the same time) the app has not even started by the time VHS
# takes the screenshot — that is how
# BE.ticket.vhs-capture-trustworthiness's blank narrow_reader_demo_table.png
# got through BE.7.L's own gate. Taking the build out of the tape removes
# that failure mode entirely.
#
# READINESS GATE (BE.ticket.vhs-wait-for-readiness, attempt 2): every
# Screenshot below is preceded by `Wait+Screen@30s /pattern/`, not a fixed
# `Sleep`. A fixed Sleep only narrows the window a blank/stale frame can
# land in — it does not remove it, and two prior attempts on this lane
# (a longer sleep, then per-scene byte-size floors) both shipped a blank
# `narrow_browser_root.png` through a green gate. A readiness match removes
# the class instead of thresholding it: if the pattern text is not on
# screen, the screenshot does not happen at all — VHS blocks (up to the 30s
# cap) or the tape fails, instead of silently capturing whatever frame
# happened to be current.
#
# Each pattern is chosen to be text that is NOT on screen before that
# scene's action completes — see the one-line rationale on each Wait+Screen
# line below. Every Screenshot's settle Sleep is 1500ms (BE.ticket.vhs-
# wait-for-readiness task 2, measured 2026-09-02, bumped from the
# original 300ms): under real concurrent-lane system load on this
# machine (measured load average up to 13, several other agent sessions
# building/running at once), a handful of scenes intermittently produced
# a genuinely blank shell-prompt capture EVEN AFTER Wait+Screen had
# already matched the ready-state pattern — visually confirmed while
# re-capturing this set, not a floor artifact. The longer post-match
# settle reduced but did not eliminate this; the min_bytes floor plus a
# mandatory visual check of every PNG (this task's own acceptance
# criteria) remains the real backstop. A short `Sleep` is kept after a
# handful of matches, only where a paint still needs to settle after the
# matched text appears (documented per-case); it is a minor polish step
# defence.
#
# THE PATTERN MUST BE ASCII. Measured 2026-09-02: a probe waiting on
# `/bella ·/` — the middle dot exactly as it appears in bella's own status
# line — silently produced NO screenshot and exited 0, while an otherwise
# identical probe waiting on `/j\/k nav/` produced a real capture. This is
# why every pattern below is built only from the ASCII portions of bella's
# status lines (never the `·` field separator) and from plain document
# text/filenames.
#
# PINNED TO celia.toml — this tape's `Screenshot` targets and
# celia.toml's `[[scene]]` names must agree, or the `scenes` harness check (celia)'s
# parity assertion (BE.7.L task 5) fails naming the missing side. Scenes
# this tape owns (the `wide_*` half of the manifest):
#   wide_reader_demo_top, wide_reader_demo_tasks_code, wide_reader_demo_table,
#   wide_reader_search, wide_reader_readme_top, wide_browser_root,
#   wide_browser_selection, wide_browser_nested, wide_browser_reveal_off,
#   wide_browser_reveal_on, wide_browser_planning_symlink, wide_scroll_anchor_top,
#   wide_reader_rail_on, wide_reader_rail_off, wide_reader_metadata_populated,
#   wide_reader_metadata_empty, wide_browser_status_line_carries_a_message,
#   wide_diagnostics_overlay_open, wide_reader_related_states,
#   wide_tree_expanded_one_level, wide_tree_root_jail_refusal
#
# Run from the repo root: vhs scripts/vhs/reference-wide.tape

Output target/vhs/wide.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 800
Set Padding 20
Set Theme "Dracula"

# --- Reader mode: demo fixture (headings, styles, links, lists, tasks) ---
Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
# "Bella Reference Demo" is the doc's H1, rendered only once bella has
# painted the file — the typed shell command above uses a lowercase
# "bella" path segment, so it can't satisfy this match early.
Wait+Screen@30s /Bella Reference Demo/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_demo_top.png

Down 18
# "Code block" only scrolls into view after Down 18 — it sits below the
# task list, well past what the initial top screenshot shows.
Wait+Screen@30s /Code block/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_demo_tasks_code.png

Down 14
# "Horizontal rule" is the document's last heading before its closing
# line — only on screen once scrolled past the table, i.e. only after
# this second Down.
Wait+Screen@30s /Horizontal rule/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_demo_table.png

# --- Search ---
Type "/"
Sleep 200ms
Type "task"
Enter
# The committed search status "/task [" (query + a space + the opening
# match-count bracket) only renders once Enter commits the search; while
# typing, the status line instead ends in an underscore cursor ("/task_"),
# so this can't match early even though "task" text is already in the doc.
Wait+Screen@30s /\/task \[/
Screenshot planning/artifacts/screenshots/wide_reader_search.png
Escape
Type "q"
Sleep 800ms

# --- Reader mode: real README (prose density check) ---
Type "target/release/bella README.md"
Enter
# "full mouse support" is prose from the README's first paragraph — never
# present in the typed shell command line it replaces on screen.
Wait+Screen@30s /full mouse support/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_readme_top.png
Type "q"
Sleep 800ms

# --- Browser mode: repo root ---
Type "target/release/bella ."
Enter
# "j/k nav" is unique to the browser-mode status line (reader mode's
# equivalent hint reads "j/k scroll"), so it can only appear once the
# directory-tree walk finishes and browser mode paints its first frame.
Wait+Screen@30s /j\/k nav/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_root.png

# Browser mode: nested + selection
Down 2
# The status line's position counter moves from "1/" (initial selection)
# to "3/" once the cursor has moved two entries down — "3/" is not a
# substring of the prior "1/11" state, so this can't match before the move.
Wait+Screen@30s /3\//
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_selection.png

Enter
# "architecture.md" is a file inside the repo's docs/ directory — it is
# not listed anywhere in the repo-root listing shown by the two prior
# screenshots, so it can only be on screen once bella has descended into
# docs/ and repainted its listing.
Wait+Screen@30s /architecture\.md/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_nested.png
Type "q"
Sleep 500ms

# Browser mode: reveal toggle ('r') — the reveal_demo fixture, off then on.
Type "target/release/bella scripts/vhs/fixtures/reveal_demo"
Enter
# "reveal (off)" is bella's default reveal-state hint text, only present
# once the browser status line renders for this fixture.
Wait+Screen@30s /reveal \(off\)/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_reveal_off.png

Type "r"
# "reveal (on)" only appears after the 'r' toggle above flips the state —
# the prior screenshot's screen read "reveal (off)" instead.
Wait+Screen@30s /reveal \(on\)/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_reveal_on.png
Type "q"
Sleep 500ms

# Browser mode: a directory reached through the planning/ symlink.
Type "target/release/bella planning"
Enter
# This ticket's own spec directory name is a listing entry unique to
# planning/ — it does not exist anywhere else bella has browsed in this
# tape, so it can only be on screen once this symlinked directory is open.
Wait+Screen@30s /BE\.ticket\.vhs-wait-for-readiness/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_planning_symlink.png
Type "q"
Sleep 500ms

# Scroll anchoring (BE.7.D task 3): the scroll_anchor.md fixture, scrolled
# to a mid-document position, at bella's normal wide reference size. Its
# text-capture counterpart in celia.toml (same scene name,
# same fixture, same scroll count, at width 80x10) is the one that ALSO
# resizes live via tmux before capturing — VHS cannot do that mid-tape (see
# celia.toml's `keys` field doc), so this PNG only supplies the "before"
# half of the pair; the "after resize" half is narrow_scroll_anchor_after_resize
# in reference-narrow.tape, captured at a fresh narrow launch instead of a
# live resize, for the same reason.
Type "target/release/bella scripts/vhs/fixtures/scroll_anchor.md"
Enter
Down 4
# The fixture's short, three-heading body already fits entirely in this
# wide viewport at load, so Down 4 doesn't change what's visible — "Heading
# Three" is the anchor text for this scene either way, and this Wait still
# guards against a genuinely blank/stale frame (the class of bug this
# ticket exists to remove), even though it isn't a post-scroll delta here.
Wait+Screen@30s /Heading Three/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_scroll_anchor_top.png
Type "q"
Sleep 500ms

# --- TOC rail (BE.7.E task 3): rail-on and rail-off at a wide terminal ---
Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
Wait+Screen@30s /Bella Reference Demo/
Type "t"
# "Contents" is the rail pane's own bordered title (see ui::draw_rail) —
# text that exists on screen ONLY once the rail is toggled open, so this
# can't match before the keypress above.
Wait+Screen@30s /Contents/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_rail_on.png

# Rail off (BE.7.K task 3, measured 2026-09-08): originally toggled back
# closed on the SAME session with a fixed Sleep (no readiness guard,
# because closed is the ABSENCE of "Contents" — nothing to Wait on). That
# produced a deterministic blank capture across three consecutive re-runs
# of this tape while re-capturing for this block, every other scene
# eventually passing on retry — this one never did. Rail-off is just
# bella's DEFAULT state, so a fresh launch (no toggle at all) is both a
# readiness-guardable capture (reuse the same top-of-file pattern) AND a
# more faithful "rail off" scene than a toggle-closed one.
Type "q"
Sleep 800ms
Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
Wait+Screen@30s /Bella Reference Demo/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_rail_off.png
Type "q"
Sleep 500ms

# --- Metadata pane (BE.7.F task 3): populated, then the empty state ---
Type "target/release/bella scripts/vhs/fixtures/metadata_demo.md"
Enter
Wait+Screen@30s /Bella Metadata Demo/
Type "t"
# "Metadata" is the pane's own bordered section title (see
# ui::draw_rail_metadata) — on screen only once the rail is toggled open,
# so this can't match before the keypress above.
Wait+Screen@30s /Metadata/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_metadata_populated.png
Type "q"
Sleep 500ms

Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
Wait+Screen@30s /Bella Reference Demo/
Type "t"
# "(no frontmatter)" is the Metadata section's explicit empty-state text
# (METADATA_EMPTY_STATE) — demo.md carries no frontmatter fence, so this
# only ever appears once the rail is toggled open on this fixture.
Wait+Screen@30s /no frontmatter/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_metadata_empty.png
Type "q"
Sleep 500ms

# --- Diagnostics (BE.7.K task 3): walk-error drop reaches the message log ---
#
# Launching directly inside dropped_demo/ would show the drop count TWICE
# at this width — once from `Browser.dropped_entries`'s own pre-existing
# direct indicator, once more from the durable-log message this task adds
# — which together run past this tape's 1200px window before "entries
# dropped" is fully on screen (`Set Width` cannot fix this: VHS sizes the
# pty once at session start, a later `Set Width` mid-tape does not resize
# the running shell). Ascending back out to the clean parent directory
# avoids the collision AND demonstrates the actual point of this block:
# the message survives the keypress that moved browser.dropped_entries
# back to 0, which a plain `status_message` could not have done.
Type "target/release/bella scripts/vhs/fixtures/dropped_demo"
Enter
# "visible.md" is the fixture's one readable entry (with the `.md` suffix
# that distinguishes it from reveal_demo's earlier bare "visible" dir
# entry) — on screen once the initial listing paints.
Wait+Screen@30s /visible\.md/
Backspace
# Now rooted at the clean parent (scripts/vhs/fixtures, dropped_entries
# == 0 there), so "entries dropped" can only be the durable-log message —
# proof the log outlived the ascend.
Wait+Screen@30s /entries dropped/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_browser_status_line_carries_a_message.png

Type "m"
# "Messages" is the diagnostics overlay's own bordered title (see
# ui::draw_diagnostics_overlay) — on screen only once the overlay is
# toggled open by the keypress above.
Wait+Screen@30s /Messages/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_diagnostics_overlay_open.png
Type "m"
Type "q"
Sleep 500ms

# --- related: rows (BE.7.G task 4): resolved, unresolved, ambiguous ---
#
# Opening the rail on a document with a non-empty `related:` list starts
# the real, whole-fleet doc_id index build (BE.7.G task 2's background
# worker): `resolve_corpus_root` walks up from ANY path under this fleet
# checkout to the nearest `brain.toml`, which is the HQ root, not
# core/bella alone — measured 42.73s / 42,683 files on this machine
# (see crates/bella/src/app.rs::real_corpus_root_build_budget). The
# Wait+Screen timeout below is set well past that measured number, not
# this tape's usual 30s default, for exactly that reason.
Type "target/release/bella scripts/vhs/fixtures/related_corpus/related_demo.md"
Enter
Wait+Screen@30s /Bella Related Demo/
Type "t"
# "(unresolved)" is RelatedRowState::Unresolved's own ASCII suffix (see
# ui::metadata_row_display) — it can only be on screen once the
# background doc_id index build has landed and every related: row in this
# frame has been re-resolved together, so it also proves the Resolved and
# Ambiguous rows beside it are final, not mid-build (the resolved marker
# `→` and ambiguous marker `≠` are non-ASCII, which this repo's tapes
# never wait on — see the ASCII-pattern note at the top of this file).
Wait+Screen@120s /\(unresolved\)/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_reader_related_states.png
Type "q"
Sleep 500ms

# --- Tree pane (BE.7.H task 4): expanded one level, and the root-jail
# refusal its own `..` row hits unconditionally --------------------------
#
# Both scenes launch on demo.md (this tape's default reader fixture) and
# open the rail with "t", then give it keyboard focus with "T" (lands on
# whichever section was last focused — a fresh launch is always
# Contents), then two Tabs cycle Contents -> Metadata -> Tree, landing on
# Tree with its selection reset to row 0: the tree's own `..`
# (`BrowserEntryKind::ParentDir`). See celia.toml's matching comment for
# why the tree's listing (rooted at the fleet's `brain.toml` ancestor,
# not core/bella alone) puts `_planning` first among its dirs.
#
# ABSOLUTE path via shell `$(pwd)` expansion, deliberately, unlike every
# other `Type "target/release/bella ..."` line above in this tape.
# `bella_engine::browser::resolve_corpus_root` walks `Path::parent()` up
# from the invoked path looking for `brain.toml`, and `Path::parent()` on
# a RELATIVE path stops at `""` without ever reaching the real filesystem
# root — so a relative `scripts/vhs/fixtures/demo.md` argument can never
# let the walk climb as far as `agentic-portfolio/`, and the Tree pane's
# corpus root resolves wrong (empty listing). `$(pwd)` is typed into a
# REAL shell here (VHS's `Type` sends literal keystrokes to the pty), so
# it expands exactly as it would at an interactive prompt. Same
# workaround as celia.toml's matching scenes, and the same caveat: this
# fixes the CAPTURE, not `resolve_corpus_root` itself, which is outside
# this task's files.
Type "target/release/bella $(pwd)/scripts/vhs/fixtures/demo.md"
Enter
Wait+Screen@30s /Bella Reference Demo/
Type "t"
# "Tree" is the rail's third section's own bordered title (see
# ui::draw_rail_tree) — on screen only once the rail is toggled open, so
# this can't match before the keypress above.
Wait+Screen@30s /Tree/
Type "T"
Tab
Tab
Down
Enter
# "base-template" is a child of `_planning` (the fleet root's
# alphabetically-first non-hidden dir, per celia.toml's comment) — it is
# not on screen anywhere in this frame until `_planning` is expanded, so
# this can only match once the Down+Enter above has expanded it in place.
Wait+Screen@30s /base-template/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_tree_expanded_one_level.png
Type "q"
Sleep 500ms

# Same `$(pwd)` absolute-path workaround as the scene above, and for the
# identical reason (relative-path corpus-root resolution) — see that
# scene's comment.
Type "target/release/bella $(pwd)/scripts/vhs/fixtures/demo.md"
Enter
Wait+Screen@30s /Bella Reference Demo/
Type "t"
Wait+Screen@30s /Tree/
Type "T"
Tab
Tab
Enter
# "nothing to ascend to" is `App::activate_tree_entry`'s own refusal
# text for the tree's ParentDir row (set_status, Severity::Info) — it
# cannot be on screen before this Enter, and it only fires here because
# the tree's `dir` is always constructed equal to its own
# `root_boundary` (wired from the invoked path by main.rs), so its `..`
# row refuses unconditionally rather than actually ascending.
Wait+Screen@30s /nothing to ascend to/
Sleep 1500ms
Screenshot planning/artifacts/screenshots/wide_tree_root_jail_refusal.png
Type "q"
Sleep 500ms
