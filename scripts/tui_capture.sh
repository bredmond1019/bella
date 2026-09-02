#!/usr/bin/env bash
# tui_capture.sh — drive a live `bella` TUI in a detached tmux session and dump
# its rendered screen as text, for an agent to eyeball layout/content without
# a human at a real terminal.
#
# Uses `bastion new`/`capture`/`kill` for session lifecycle (they already wrap
# tmux with bella's trust/degradation handling), but sends keystrokes with raw
# `tmux send-keys` rather than `bastion send`: bastion's `send` verb always
# appends Enter after the literal text (term-core's send_keys_args, engine-rs
# crates/term-core/src/tmux.rs:118), and in bella Enter is bound to
# Action::Follow / Action::BrowserDescend — so every scripted keystroke would
# also try to open a link or descend into a file. Raw tmux send-keys lets us
# send named keys (Down, Enter, Tab, Escape) or literal chars without that.
#
# This captures TEXT only (via `tmux capture-pane -p`), not a rendered image —
# good for structural/content review (missing sections, broken wrapping,
# wrong content) but blind to color/theme/glyph-level polish. Use
# scripts/vhs/*.tape (VHS) for pixel-level visual review.
#
# Usage:
#   scripts/tui_capture.sh <target-file-or-dir> [key ...]
#
# Each [key] is either a literal single char to send, or one of tmux's named
# keys (Down/Up/Enter/Escape/Tab/BTab/PPage/NPage/Home/End/C-c/C-d/C-u).
# A short pause is inserted between keys so bella's render worker catches up.
#
# Examples:
#   scripts/tui_capture.sh README.md                  # just open + capture
#   scripts/tui_capture.sh README.md Down Down Down    # scroll and capture
#   scripts/tui_capture.sh . Down Down Enter q         # browser: descend into 2nd entry
#
# Always kills its session on exit, even on failure.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSION="bella-tui-capture-$$"
SETTLE_SECS="${TUI_CAPTURE_SETTLE:-0.4}"

if [[ $# -lt 1 ]]; then
    echo "usage: $(basename "$0") <target-file-or-dir> [key ...]" >&2
    exit 2
fi

TARGET="$1"
shift
KEYS=("$@")

cleanup() {
    bastion kill "$SESSION" >/dev/null 2>&1 || true
}
trap cleanup EXIT

bastion new "$SESSION" --dir "$REPO_ROOT" >/dev/null 2>&1
sleep 0.2

tmux send-keys -t "$SESSION" -l -- "cargo run --quiet -p bella -- $TARGET"
tmux send-keys -t "$SESSION" Enter

# Give the initial build/render time to settle before driving keys.
sleep "${TUI_CAPTURE_BUILD_SETTLE:-2}"

for key in "${KEYS[@]+"${KEYS[@]}"}"; do
    tmux send-keys -t "$SESSION" "$key"
    sleep "$SETTLE_SECS"
done

sleep "$SETTLE_SECS"
bastion capture "$SESSION" 2>/dev/null
