# reference-wide.tape — full reference screenshot set at a wide terminal size.
# Output: planning/artifacts/screenshots/wide_*.png (gitignored generated dir;
# the PNGs themselves are committed as the reference set — see that
# directory's own note).
#
# Run from the repo root: vhs scripts/vhs/reference-wide.tape

Output planning/artifacts/screenshots/wide.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 800
Set Padding 20
Set Theme "Dracula"

# --- Reader mode: demo fixture (headings, styles, links, lists, tasks) ---
Type "cargo run --quiet -p bella -- scripts/vhs/fixtures/demo.md"
Enter
Sleep 2s
Screenshot planning/artifacts/screenshots/wide_reader_demo_top.png

Down 18
Sleep 300ms
Screenshot planning/artifacts/screenshots/wide_reader_demo_tasks_code.png

Down 14
Sleep 300ms
Screenshot planning/artifacts/screenshots/wide_reader_demo_table.png

# --- Search ---
Type "/"
Sleep 200ms
Type "task"
Enter
Sleep 400ms
Screenshot planning/artifacts/screenshots/wide_reader_search.png
Escape
Type "q"
Sleep 800ms

# --- Reader mode: real README (prose density check) ---
Type "cargo run --quiet -p bella -- README.md"
Enter
Sleep 2s
Screenshot planning/artifacts/screenshots/wide_reader_readme_top.png
Type "q"
Sleep 800ms

# --- Browser mode: repo root ---
Type "cargo run --quiet -p bella -- ."
Enter
Sleep 2s
Screenshot planning/artifacts/screenshots/wide_browser_root.png

# Browser mode: nested + selection
Down 2
Sleep 500ms
Screenshot planning/artifacts/screenshots/wide_browser_selection.png

Enter
Sleep 1s
Screenshot planning/artifacts/screenshots/wide_browser_nested.png
Type "q"
Sleep 500ms
