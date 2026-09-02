#!/usr/bin/env bash
# check_scenes.sh — re-capture every scene declared in scripts/vhs/scenes.toml
# into a temp directory (via capture_scenes.sh --out), diff each capture
# against the committed baseline in tests/scenes/<name>.txt, and exit
# non-zero on ANY difference, naming the failing scene and printing the
# diff.
#
# Guarded so an environment without tmux REPORTS rather than hard-blocks —
# the same shape as the `cargo-audit` entry in planning/harness.json.
# Registered there as the `scenes` validation.checks[] entry.
#
# A blank or near-empty re-capture is treated as a HARD ERROR distinct from
# a baseline mismatch: printing a whole-file diff for an empty capture reads
# as "the baseline changed" when the real defect is the known
# `rapid-keypresses-blank-the-render` render-blanking bug, and that
# confusion is exactly how a blank capture gets "fixed" by re-committing the
# blank as the new baseline. The minimum non-whitespace line count is
# deliberately small (3) — real captures carry bella's status line plus
# content; a blanked pane is empty or carries only a shell prompt line.
#
# Usage: bash scripts/check_scenes.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE_DIR="$REPO_ROOT/tests/scenes"
MANIFEST="$REPO_ROOT/scripts/vhs/scenes.toml"
TAPES=("$REPO_ROOT/scripts/vhs/reference-wide.tape" "$REPO_ROOT/scripts/vhs/reference-narrow.tape")
MIN_NONBLANK_LINES=3

if ! command -v tmux >/dev/null 2>&1; then
    echo "check_scenes.sh: tmux not found on PATH — scenes check skipped" >&2
    exit 0
fi

# --- Tape/manifest parity (BE.7.L task 5) -----------------------------
# Every `Screenshot` target basename across both reference tapes must have
# a matching `[[scene]]` name in scenes.toml, and vice versa — a scene
# added to one and not the other is meant to be caught here, not silently
# left to drift.
manifest_names="$(grep -E '^name = "' "$MANIFEST" | sed -E 's/^name = "([^"]+)".*/\1/' | sort -u)"
tape_names="$( (for tape in "${TAPES[@]}"; do
    grep -E '^Screenshot ' "$tape" | sed -E 's#^Screenshot .*/([A-Za-z0-9_]+)\.png#\1#'
done) | sort -u)"

missing_from_tapes="$(comm -23 <(echo "$manifest_names") <(echo "$tape_names"))"
missing_from_manifest="$(comm -13 <(echo "$manifest_names") <(echo "$tape_names"))"

PARITY_FAIL=0
if [[ -n "$missing_from_tapes" ]]; then
    echo "check_scenes.sh: FAIL — scene(s) declared in scenes.toml but not screenshotted by either reference-wide.tape or reference-narrow.tape: $missing_from_tapes" >&2
    PARITY_FAIL=1
fi
if [[ -n "$missing_from_manifest" ]]; then
    echo "check_scenes.sh: FAIL — scene(s) screenshotted by a reference tape but not declared in scripts/vhs/scenes.toml: $missing_from_manifest" >&2
    PARITY_FAIL=1
fi
if [[ "$PARITY_FAIL" -ne 0 ]]; then
    echo "check_scenes.sh: FAILED — tape/manifest parity check failed (see above)." >&2
    exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bella-check-scenes.XXXXXX")"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "check_scenes.sh: re-capturing scenes into $TMP_DIR..." >&2
bash "$REPO_ROOT/scripts/capture_scenes.sh" --out "$TMP_DIR"

FAIL=0
CHECKED=0

for baseline in "$BASELINE_DIR"/*.txt; do
    [[ -e "$baseline" ]] || continue
    name="$(basename "$baseline" .txt)"
    recapture="$TMP_DIR/${name}.txt"
    CHECKED=$((CHECKED + 1))

    if [[ ! -f "$recapture" ]]; then
        echo "check_scenes.sh: FAIL — scene '$name' has a committed baseline but was not re-captured (missing from manifest?)" >&2
        FAIL=1
        continue
    fi

    # Hard error, not a diff: a blank or near-empty capture is the known
    # render-blanking defect, not an intentional baseline change.
    nonblank_lines="$(grep -c '[^[:space:]]' "$recapture" || true)"
    if [[ "$nonblank_lines" -lt "$MIN_NONBLANK_LINES" ]]; then
        echo "check_scenes.sh: CAPTURE FAILED — scene '$name' re-captured as blank or near-empty ($nonblank_lines non-blank line(s), minimum $MIN_NONBLANK_LINES). This is a capture failure, not a baseline mismatch — see carryover 'rapid-keypresses-blank-the-render'. Re-run scripts/capture_scenes.sh $name manually to inspect." >&2
        FAIL=1
        continue
    fi

    if ! diff -u "$baseline" "$recapture" >"$TMP_DIR/${name}.diff" 2>&1; then
        echo "check_scenes.sh: FAIL — scene '$name' does not match its committed baseline ($baseline):" >&2
        cat "$TMP_DIR/${name}.diff" >&2
        FAIL=1
    fi
done

if [[ "$CHECKED" -eq 0 ]]; then
    echo "check_scenes.sh: no baselines found under $BASELINE_DIR — nothing checked" >&2
    exit 1
fi

if [[ "$FAIL" -ne 0 ]]; then
    echo "check_scenes.sh: FAILED — one or more scenes differ from their committed baseline or failed to capture." >&2
    exit 1
fi

echo "check_scenes.sh: OK — $CHECKED scene(s) matched their committed baseline." >&2
