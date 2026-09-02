# reference-collapse.tape — a single reference screenshot at a terminal
# narrower than the TOC rail's minimum-body-width threshold
# (RAIL_WIDTH + MIN_BODY_WIDTH = 24 + 20 = 44 columns, see
# crates/bella/src/ui.rs), so toggling the rail on has NO visible effect —
# the rail auto-collapses rather than squeezing the body below
# MIN_BODY_WIDTH. Neither reference-wide.tape (~110 cols) nor
# reference-narrow.tape (~65 cols) is narrow enough to cross that
# threshold, so this scene needs its own, narrower `Set Width` — and VHS
# cannot resize an already-running terminal mid-tape (`Set Width`/`Set
# Height` after the session starts is silently ignored), so a dedicated
# tape file is the only way to get a real, narrower VHS capture rather
# than reusing one of the two fixed-size reference tapes.
#
# PREREQUISITE: run `cargo build --release` before this tape (see
# reference-wide.tape's header for why the build must not happen inside
# the tape's settle window).
#
# READINESS: this scene proves an ABSENCE (no "Contents" rail pane
# appears), so there is nothing new to Wait+Screen for after the `t`
# keypress — the doc's own H1 is the readiness signal, matched BEFORE the
# keypress, followed by a settle Sleep to cover the toggle itself.
#
# PINNED TO scripts/vhs/scenes.toml — this tape's `Screenshot` target and
# scenes.toml's `[[scene]]` name must agree, or `scripts/check_scenes.sh`'s
# parity assertion fails naming the missing side. Scene this tape owns:
#   narrow_reader_rail_collapse
#
# Run from the repo root: vhs scripts/vhs/reference-collapse.tape

Output target/vhs/collapse.gif

Set Shell "bash"
Set FontSize 16
Set Width 480
Set Height 500
Set Padding 20
Set Theme "Dracula"

Type "target/release/bella scripts/vhs/fixtures/demo.md"
Enter
# "Bella Reference Demo" is the doc's H1, rendered only once bella has
# painted the file — confirms the app is ready before the rail toggle
# below, which (by design) produces no further on-screen change to wait
# for.
Wait+Screen@30s /Bella Reference Demo/
Type "t"
Sleep 1500ms
Screenshot planning/artifacts/screenshots/narrow_reader_rail_collapse.png
Type "q"
Sleep 500ms
