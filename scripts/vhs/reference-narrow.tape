# reference-narrow.tape — full reference screenshot set at a narrow terminal
# size (~65 cols), to check wrap/reflow behavior. Companion to
# reference-wide.tape. Run from the repo root:
#   vhs scripts/vhs/reference-narrow.tape
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
# `narrow_browser_root.png` — THIS EXACT SCENE — through a green gate. A
# readiness match removes the class instead of thresholding it: if the
# pattern text is not on screen, the screenshot does not happen at all.
#
# Each pattern is chosen to be text that is NOT on screen before that
# scene's action completes — see the one-line rationale on each Wait+Screen
# line below. A short `Sleep` is kept after a handful of matches, only
# where a paint still needs to settle after the matched text appears; it is
# a minor polish step now, never the primary defence.
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
# this tape owns (the `narrow_*` half of the manifest):
#   narrow_reader_demo_top, narrow_reader_demo_table, narrow_browser_root,
#   narrow_scroll_anchor_after_resize

Output target/vhs/narrow.gif

Set Shell "bash"
Set FontSize 16
Set Width 700
Set Height 500
Set Padding 20
Set Theme "Dracula"

Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
# "Bella Reference Demo" is the doc's H1, rendered only once bella has
# painted the file — the typed shell command above uses a lowercase
# "bella" path segment, so it can't satisfy this match early.
Wait+Screen@30s /Bella Reference Demo/
Sleep 300ms
Screenshot planning/artifacts/screenshots/narrow_reader_demo_top.png

Down 20
# "Task list" is the heading that scrolls into view after Down 20 at this
# narrow width — it sits well below what the initial top screenshot shows.
Wait+Screen@30s /Task list/
Sleep 300ms
Screenshot planning/artifacts/screenshots/narrow_reader_demo_table.png
Type "q"
Sleep 800ms

# BROWSER mode
Type "target/release/bella ."
Enter
# "j/k nav" is unique to the browser-mode status line (reader mode's
# equivalent hint reads "j/k scroll"), so it can only appear once the
# directory-tree walk finishes and browser mode paints its first frame —
# this is the exact scene that shipped a blank capture at BE.7.D despite
# passing its byte-size floor.
Wait+Screen@30s /j\/k nav/
Sleep 300ms
Screenshot planning/artifacts/screenshots/narrow_browser_root.png
Type "q"
Sleep 500ms

# Scroll anchoring (BE.7.D task 3): the scroll_anchor.md fixture at the
# narrow reference size, scrolled down the SAME count as
# wide_scroll_anchor_top in reference-wide.tape, to visually compare the
# two. Named "after_resize" for parity with its text-capture counterpart in
# scripts/vhs/scenes.toml, which resizes LIVE via tmux from wide to this
# narrow size before capturing (VHS cannot do a mid-tape resize — see
# scenes.toml's `keys` field doc) — this PNG is a fresh launch already at
# the narrow size, not an actual resize, but shows the same content.
Type "target/release/bella scripts/vhs/fixtures/scroll_anchor.md"
Enter
Down 4
# "Heading Three" is the fixture's last heading — the anchor text for this
# scene, and a real guard against a blank/stale frame regardless of how
# much the narrower width's reflow moves it relative to Down 4.
Wait+Screen@30s /Heading Three/
Sleep 300ms
Screenshot planning/artifacts/screenshots/narrow_scroll_anchor_after_resize.png
Type "q"
Sleep 500ms
