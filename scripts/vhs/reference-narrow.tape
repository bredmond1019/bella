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

# "Task list" is the heading that scrolls into view after Down 20 at this
# narrow width — it sits well below what the initial top screenshot shows.
# Sent as four Down 5 bursts with a short Sleep between (BE.ticket.vhs-
# wait-for-readiness task 2, measured 2026-09-02), not one Down 20 burst:
# a single 20-keypress burst here reliably triggered the known
# reader-mode render-blanking carryover (rapid-keypresses-blank-the-
# render, explicitly out of scope to FIX here) even after Wait+Screen
# matched "Task list" — the capture itself came back blank at a constant
# 2176 bytes, well under this scene's min_bytes floor, on 6 of 7
# consecutive attempts. Breaking the burst into smaller chunks is a
# capture-cadence workaround, not a change to bella's render code.
Down 5
Sleep 80ms
Down 5
Sleep 80ms
Down 5
Sleep 80ms
Down 5
Wait+Screen@30s /Task list/
Sleep 2000ms
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
Sleep 2000ms
Screenshot planning/artifacts/screenshots/narrow_browser_root.png
Type "q"
Sleep 500ms

# Scroll anchoring (BE.7.D task 3): the scroll_anchor.md fixture at the
# narrow reference size, scrolled to the same anchor heading as
# wide_scroll_anchor_top in reference-wide.tape, to visually compare the
# two. Named "after_resize" for parity with its text-capture counterpart in
# scripts/vhs/scenes.toml, which resizes LIVE via tmux from wide to this
# narrow size before capturing (VHS cannot do a mid-tape resize — see
# scenes.toml's `keys` field doc) — this PNG is a fresh launch already at
# the narrow size, not an actual resize, but shows the same content.
#
# Down 10, not the wide tape's Down 4 (BE.ticket.vhs-wait-for-readiness
# task 2, measured 2026-09-02): at this narrow width the fixture's
# paragraphs wrap onto more display lines, so Down 4 lands at line ~26 of
# 33 and never reaches "Heading Three" — the readiness gate DID ITS JOB
# here, timing out on a Wait that could never match, instead of shipping a
# stale mid-scroll frame the way the old fixed-Sleep tape would have. Down
# 10 reliably scrolls past the end of this fixture's content regardless of
# exact wrap count (the view simply stops at the bottom).
Type "target/release/bella scripts/vhs/fixtures/scroll_anchor.md"
Enter
Down 10
# "Heading Three" is the fixture's last heading — the anchor text for this
# scene, and a real guard against a blank/stale frame regardless of how
# much the narrower width's reflow moves it relative to Down 10.
Wait+Screen@30s /Heading Three/
Sleep 2000ms
Screenshot planning/artifacts/screenshots/narrow_scroll_anchor_after_resize.png
Type "q"
Sleep 500ms
