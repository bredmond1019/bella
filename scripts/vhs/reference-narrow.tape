# reference-narrow.tape — full reference screenshot set at a narrow terminal
# size (~65 cols), to check wrap/reflow behavior. Companion to
# reference-wide.tape. Run from the repo root:
#   vhs scripts/vhs/reference-narrow.tape

Output planning/artifacts/screenshots/narrow.gif

Set Shell "bash"
Set FontSize 16
Set Width 700
Set Height 500
Set Padding 20
Set Theme "Dracula"

Type "cargo run --quiet -p bella -- scripts/vhs/fixtures/demo.md"
Enter
Sleep 2s
Screenshot planning/artifacts/screenshots/narrow_reader_demo_top.png

Down 20
Sleep 300ms
Screenshot planning/artifacts/screenshots/narrow_reader_demo_table.png
Type "q"
Sleep 800ms

Type "cargo run --quiet -p bella -- ."
Enter
Sleep 2s
Screenshot planning/artifacts/screenshots/narrow_browser_root.png
Type "q"
Sleep 500ms
