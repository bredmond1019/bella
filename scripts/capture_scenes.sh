#!/usr/bin/env bash
# capture_scenes.sh — drive the real `bella` release binary through a plain
# tmux session per scripts/vhs/scenes.toml, writing tmux capture-pane text
# output to tests/scenes/<name>.txt (or --out <dir>).
#
# Session lifecycle is plain tmux (new-session/capture-pane/kill-session) —
# deliberately NOT the ops CLI's own tmux wrapper that scripts/tui_capture.sh
# uses for session lifecycle convenience. A gated check in THIS repo must
# never depend on that ops CLI being installed (standing rule 5, D3): bella
# has to stay testable stand-alone, outside the monorepo. Only the session
# lifecycle differs from tui_capture.sh — the raw `tmux send-keys` keystroke
# handling (never the ops CLI's own send verb, which always appends Enter
# and would trigger bella's Follow/BrowserDescend binding) is lifted as-is.
#
# Uses the RELEASE binary at a fixed path (`target/release/bella`), built
# once up front if missing, NOT the cargo dev-run subcommand — its build
# chatter lands inside the captured pane and is the single largest source of
# capture nondeterminism.
#
# Keys are sent ONE per `tmux send-keys` call with an explicit sleep between
# them — never as a repeat-count (e.g. never `tmux send-keys Down -N 30`).
# This is a load-bearing regression guard, not a style choice: BE.7.B task 3
# reproducibly blanked a VHS capture with `Down 34`/`Down 38` while
# single-stepped `Down` was stable across three runs (carryover
# `rapid-keypresses-blank-the-render`). The settle interval between keys is
# a per-scene manifest field (scenes.toml [defaults].key_settle, overridable
# per scene), not a constant baked into this script.
#
# The tmux session name is keyed on this process's pid so two concurrent
# invocations (e.g. two lanes running the harness at once) cannot collide —
# same reasoning as BE.7.M's `unique_temp_dir`.
#
# Usage:
#   scripts/capture_scenes.sh [scene-name] [--out <dir>]
#
# With no scene-name, every scene in the manifest is captured. --out
# defaults to tests/scenes/ (the committed baseline directory); pass a temp
# directory to re-capture without touching the baselines (this is exactly
# what check_scenes.sh does).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$REPO_ROOT/scripts/vhs/scenes.toml"
BINARY="$REPO_ROOT/target/release/bella"
OUT_DIR="$REPO_ROOT/tests/scenes"
ONLY_SCENE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out)
            OUT_DIR="$2"
            shift 2
            ;;
        --out=*)
            OUT_DIR="${1#--out=}"
            shift
            ;;
        -h|--help)
            echo "usage: $(basename "$0") [scene-name] [--out <dir>]" >&2
            exit 0
            ;;
        *)
            if [[ -n "$ONLY_SCENE" ]]; then
                echo "error: unexpected extra argument '$1'" >&2
                exit 2
            fi
            ONLY_SCENE="$1"
            shift
            ;;
    esac
done

if ! command -v tmux >/dev/null 2>&1; then
    echo "capture_scenes.sh: tmux not found on PATH" >&2
    exit 1
fi

if [[ ! -f "$MANIFEST" ]]; then
    echo "capture_scenes.sh: manifest not found at $MANIFEST" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

if [[ ! -x "$BINARY" ]]; then
    echo "capture_scenes.sh: building release binary ($BINARY not found)..." >&2
    (cd "$REPO_ROOT" && cargo build --release --quiet -p bella)
fi

# Parse the manifest with Python's stdlib tomllib and emit one TSV line per
# scene: name<TAB>target<TAB>width<TAB>height<TAB>settle<TAB>base64(keys-json)
# so bash never has to parse TOML itself. Keys are base64-encoded JSON to
# survive the TSV round-trip regardless of what characters they contain.
SCENES_TSV="$(python3 - "$MANIFEST" "$ONLY_SCENE" <<'PYEOF'
import base64
import json
import sys
import tomllib

manifest_path, only_scene = sys.argv[1], sys.argv[2]
with open(manifest_path, "rb") as f:
    data = tomllib.load(f)

defaults = data.get("defaults", {})
launch_settle = defaults.get("launch_settle", 2.0)
key_settle = defaults.get("key_settle", 0.4)

scenes = data.get("scene", [])
if only_scene:
    scenes = [s for s in scenes if s.get("name") == only_scene]
    if not scenes:
        sys.exit(f"capture_scenes.sh: no scene named '{only_scene}' in manifest")

for s in scenes:
    name = s["name"]
    target = s["target"]
    width = s.get("width")
    height = s.get("height")
    settle = s.get("settle", key_settle)
    keys = s.get("keys", [])
    keys_b64 = base64.b64encode(json.dumps(keys).encode()).decode()
    print(f"{name}\t{target}\t{width}\t{height}\t{settle}\t{launch_settle}\t{keys_b64}")
PYEOF
)"

if [[ -z "$SCENES_TSV" ]]; then
    echo "capture_scenes.sh: no scenes to capture" >&2
    exit 1
fi

capture_one() {
    local name="$1" target="$2" width="$3" height="$4" settle="$5" launch_settle="$6" keys_b64="$7"
    local session="bella-scene-$$-${name}"

    cleanup() {
        tmux kill-session -t "$session" >/dev/null 2>&1 || true
    }
    trap cleanup EXIT

    tmux new-session -d -s "$session" -x "$width" -y "$height" -c "$REPO_ROOT"
    tmux send-keys -t "$session" -l -- "$BINARY $target"
    tmux send-keys -t "$session" Enter

    sleep "$launch_settle"

    while IFS= read -r key; do
        [[ -z "$key" ]] && continue
        if [[ "${#key}" -eq 1 ]]; then
            # Single character (e.g. "/", "t", "k") — send literally so tmux
            # never tries to interpret it as a named key.
            tmux send-keys -t "$session" -l -- "$key"
        else
            # Multi-character token (e.g. "Down", "Enter", "Escape") — a
            # tmux named key.
            tmux send-keys -t "$session" "$key"
        fi
        sleep "$settle"
    done < <(python3 -c "import base64,json,sys; print('\n'.join(json.loads(base64.b64decode(sys.argv[1]))))" "$keys_b64")

    sleep "$settle"
    tmux capture-pane -t "$session" -p > "$OUT_DIR/${name}.txt"

    tmux kill-session -t "$session" >/dev/null 2>&1 || true
    trap - EXIT
}

CAPTURED=0
while IFS=$'\t' read -r name target width height settle launch_settle keys_b64; do
    [[ -z "$name" ]] && continue
    echo "capture_scenes.sh: capturing '$name' ($target @ ${width}x${height})..." >&2
    capture_one "$name" "$target" "$width" "$height" "$settle" "$launch_settle" "$keys_b64"
    CAPTURED=$((CAPTURED + 1))
done <<< "$SCENES_TSV"

echo "capture_scenes.sh: wrote $CAPTURED scene(s) to $OUT_DIR" >&2
