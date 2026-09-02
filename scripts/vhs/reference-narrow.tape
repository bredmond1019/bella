# reference-narrow.tape — full reference screenshot set at a narrow terminal
# size (~65 cols), to check wrap/reflow behavior. Companion to
# reference-wide.tape. Run from the repo root:
#   vhs scripts/vhs/reference-narrow.tape
#
# PINNED TO scripts/vhs/scenes.toml — this tape's `Screenshot` targets and
# scenes.toml's `[[scene]]` names must agree, or `scripts/check_scenes.sh`'s
# parity assertion (BE.7.L task 5) fails naming the missing side. Scenes
# this tape owns (the `narrow_*` half of the manifest):
#   narrow_reader_demo_top, narrow_reader_demo_table, narrow_browser_root
#
# SETTLE TIME: reader-mode scenes settle at Sleep 4s. BROWSER-mode scenes
# (`bella -- .`) need Sleep 8s+ — its walker has to read the directory tree
# before the first frame renders, and a shorter settle produces a corrupt,
# near-empty capture (measured 2026-09-02: Sleep 2s -> 2176-byte corrupt
# narrow_browser_root.png; Sleep 8s -> 42017-byte clean capture, same
# machine, same concurrent-lane load). Do NOT "tidy" this back down to 2s.

Output planning/artifacts/screenshots/narrow.gif

Set Shell "bash"
Set FontSize 16
Set Width 700
Set Height 500
Set Padding 20
Set Theme "Dracula"

Type "cargo run --quiet -p bella -- scripts/vhs/fixtures/demo.md"
Enter
Sleep 4s
Screenshot planning/artifacts/screenshots/narrow_reader_demo_top.png

Down 20
Sleep 800ms
Screenshot planning/artifacts/screenshots/narrow_reader_demo_table.png
Type "q"
Sleep 800ms

# BROWSER mode — Sleep 8s: see header comment; do not shorten.
Type "cargo run --quiet -p bella -- ."
Enter
Sleep 8s
Screenshot planning/artifacts/screenshots/narrow_browser_root.png
Type "q"
Sleep 500ms
