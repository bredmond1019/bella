# browser.tape — VHS script for a pixel-level visual capture of bella's
# file browser (directory navigator, BE.1.E). See reader.tape for the
# reader-mode capture and the general VHS usage note.
#
# Run from the repo root:
#   vhs scripts/vhs/browser.tape
# Output lands in scripts/vhs/out/browser.gif (+ still frames).

Output scripts/vhs/out/browser.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 800
Set Padding 20
Set Theme "Dracula"

Type "cargo run --quiet -p bella -- ."
Enter
Sleep 2s

Screenshot scripts/vhs/out/browser_top.png

Down 3
Sleep 300ms
Screenshot scripts/vhs/out/browser_selected.png

Type "q"
Sleep 300ms
