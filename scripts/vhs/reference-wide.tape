# reference-wide.tape — full reference screenshot set at a wide terminal size.
# Output: planning/artifacts/screenshots/wide_*.png (gitignored generated dir;
# the PNGs themselves are committed as the reference set — see that
# directory's own note).
#
# PINNED TO scripts/vhs/scenes.toml — this tape's `Screenshot` targets and
# scenes.toml's `[[scene]]` names must agree, or `scripts/check_scenes.sh`'s
# parity assertion (BE.7.L task 5) fails naming the missing side. Scenes
# this tape owns (the `wide_*` half of the manifest):
#   wide_reader_demo_top, wide_reader_demo_tasks_code, wide_reader_demo_table,
#   wide_reader_search, wide_reader_readme_top, wide_browser_root,
#   wide_browser_selection, wide_browser_nested
#
# SETTLE TIME: reader-mode scenes settle at Sleep 4s. BROWSER-mode scenes
# (`bella -- .`) need Sleep 8s+ — its walker has to read the directory tree
# before the first frame renders, and a shorter settle produces a corrupt,
# near-empty capture (measured 2026-09-02: Sleep 2s -> 2176-byte corrupt
# narrow_browser_root.png; Sleep 8s -> 42017-byte clean capture, same
# machine, same concurrent-lane load). Do NOT "tidy" this back down to 2s.
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
Sleep 4s
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
Sleep 4s
Screenshot planning/artifacts/screenshots/wide_reader_readme_top.png
Type "q"
Sleep 800ms

# --- Browser mode: repo root --- (Sleep 8s: BROWSER mode's directory-tree
# walker needs the longer settle — see header comment; do not shorten.)
Type "cargo run --quiet -p bella -- ."
Enter
Sleep 8s
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
