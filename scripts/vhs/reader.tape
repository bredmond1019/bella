# reader.tape — VHS script for a pixel-level visual capture of bella's
# reader mode. Renders to PNG/GIF (real colors, glyphs, spacing) — use this
# for polish review; use scripts/tui_capture.sh for fast text-only checks.
#
# Run from the repo root:
#   vhs scripts/vhs/reader.tape
# Output lands in scripts/vhs/out/reader.gif (+ a still frame .png).
#
# Requires: vhs (brew install vhs), which pulls in ttyd + ffmpeg.

Output scripts/vhs/out/reader.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 800
Set Padding 20
Set Theme "Dracula"

Type "cargo run --quiet -p bella -- README.md"
Enter
Sleep 2s

Screenshot scripts/vhs/out/reader_top.png

Down 10
Sleep 500ms
Screenshot scripts/vhs/out/reader_scrolled.png

Type "/"
Sleep 300ms
Type "quickstart"
Enter
Sleep 500ms
Screenshot scripts/vhs/out/reader_search.png

Escape
Type "q"
Sleep 300ms
