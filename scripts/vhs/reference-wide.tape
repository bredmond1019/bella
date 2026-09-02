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
# PINNED TO scripts/vhs/scenes.toml — this tape's `Screenshot` targets and
# scenes.toml's `[[scene]]` names must agree, or `scripts/check_scenes.sh`'s
# parity assertion (BE.7.L task 5) fails naming the missing side. Scenes
# this tape owns (the `wide_*` half of the manifest):
#   wide_reader_demo_top, wide_reader_demo_tasks_code, wide_reader_demo_table,
#   wide_reader_search, wide_reader_readme_top, wide_browser_root,
#   wide_browser_selection, wide_browser_nested, wide_browser_reveal_off,
#   wide_browser_reveal_on, wide_browser_planning_symlink, wide_scroll_anchor_top
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
# text-capture counterpart in scripts/vhs/scenes.toml (same scene name,
# same fixture, same scroll count, at width 80x10) is the one that ALSO
# resizes live via tmux before capturing — VHS cannot do that mid-tape (see
# scenes.toml's `keys` field doc), so this PNG only supplies the "before"
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
