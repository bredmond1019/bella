# reference-wide.tape — full reference screenshot set at a wide terminal size.
# Output: planning/artifacts/screenshots/wide_*.png (gitignored generated dir;
# the PNGs themselves are committed as the reference set — see that
# directory's own note).
#
# PREREQUISITE: run `cargo build --release` before this tape. Every scene
# below drives target/release/bella directly, NOT cargo's own dev-build-and-
# run subcommand. Under that subcommand, compilation happens INSIDE the
# tape's Sleep settle window, so under concurrent cargo lanes (other agents
# building at the same time) the app has not even started by the time VHS
# takes the screenshot — that is how
# BE.ticket.vhs-capture-trustworthiness's blank narrow_reader_demo_table.png
# got through BE.7.L's own gate. Taking the build out of the tape removes
# that failure mode entirely.
#
# PINNED TO scripts/vhs/scenes.toml — this tape's `Screenshot` targets and
# scenes.toml's `[[scene]]` names must agree, or `scripts/check_scenes.sh`'s
# parity assertion (BE.7.L task 5) fails naming the missing side. Scenes
# this tape owns (the `wide_*` half of the manifest):
#   wide_reader_demo_top, wide_reader_demo_tasks_code, wide_reader_demo_table,
#   wide_reader_search, wide_reader_readme_top, wide_browser_root,
#   wide_browser_selection, wide_browser_nested
#
# SETTLE TIME (re-measured 2026-09-02 against the PREBUILT binary, on this
# machine under concurrent-lane load, `uptime` load averages observed
# 5-17): with the build already out of the window, blank/near-empty
# captures still occurred intermittently at every settle value tried —
# 2s, 3s, 4s, 6s, and even 8s — because the remaining variable is generic
# VHS/tmux screenshot-vs-render-frame CPU contention under a loaded box,
# not app startup time. No shorter value than the settle already in place
# produced a reliably clean run, so reader-mode settle stays at Sleep 4s
# and BROWSER-mode settle stays at Sleep 8s+ (its directory-tree walker is
# the slower path to first render). Do NOT "tidy" either value down without
# re-running this same measurement on a quiet box. If a capture still comes
# back blank at these settle times, retry the tape rather than lowering the
# floor at commit time — see check_vhs_fresh.sh's per-scene floor.
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
Type "target/release/bella scripts/vhs/fixtures/demo.md"
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
Type "target/release/bella README.md"
Enter
Sleep 4s
Screenshot planning/artifacts/screenshots/wide_reader_readme_top.png
Type "q"
Sleep 800ms

# --- Browser mode: repo root --- (Sleep 8s: BROWSER mode's directory-tree
# walker needs the longer settle — see header comment; do not shorten.)
Type "target/release/bella ."
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
