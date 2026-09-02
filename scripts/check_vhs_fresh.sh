#!/usr/bin/env bash
# check_vhs_fresh.sh — freshness + sanity gate on the VHS reference PNGs
# under planning/artifacts/screenshots/.
#
# For each PNG:
#   (a) SANITY — it must be a real capture, not a failed one showing only a
#       shell prompt. Checked two ways: a byte-size floor, AND the presence
#       of a companion text scene (tests/scenes/<same-name>.txt, from
#       scripts/capture_scenes.sh) that carries bella's status line. The
#       status line's literal text varies by mode (e.g. the search overlay
#       replaces it with a `/query [n/m]` prompt line, so a fixed string
#       like "bella ·" is not present in every real scene — see
#       tests/scenes/wide_reader_search.txt) so this is checked the same
#       way scripts/check_scenes.sh already detects a failed/blanked
#       capture: a minimum non-whitespace line count. A companion scene
#       showing only a shell prompt (the known failure mode) has 0-2
#       non-blank lines; every real capture carries bella's rendered
#       content plus a footer line.
#   (b) FRESHNESS — its last commit must not be older than the last commit
#       to any of the render/theme sources: crates/bella/src/ui.rs,
#       crates/bella/src/theme.rs, crates/bella-engine/src/markdown.rs,
#       crates/bella-engine/src/theme.rs, crates/bella-engine/src/palette.rs.
#
# On failure, names the tape to re-run: scripts/vhs/reference-wide.tape for
# a wide_* scene, scripts/vhs/reference-narrow.tape for a narrow_* scene.
#
# MTIME TRAP: git does not preserve filesystem mtimes on clone/checkout, so
# a fresh clone has every PNG newer than every source file and a
# filesystem-mtime freshness check would pass vacuously, every time,
# regardless of whether the PNG is actually stale. This script uses
# `git log -1 --format=%ct -- <path>` (the commit time of the last commit
# that touched the path) for BOTH sides of the comparison instead.
#
# REPO TRAP: planning/ is a symlink into a DIFFERENT git repository (the
# company-brain HQ vault) — see this repo's CLAUDE.md "Symlink warning".
# `git log` run from this repo's worktree cannot see history for paths
# under planning/, because those files belong to the vault repo, not this
# one. So the PNG's real git commit time must be looked up in ITS OWN
# repo (resolved via `realpath` + `git rev-parse --show-toplevel` from the
# PNG's real, symlink-resolved location), while each source file's commit
# time is looked up in this repo as normal. Mixing them up silently
# produces an empty `git log` result for the PNG side and a check that is
# not actually testing what it claims to.
#
# Guarded so an environment without `vhs` REPORTS rather than hard-blocks —
# the same shape as the `cargo-audit` entry in planning/harness.json.
# Registered there as the `vhs-fresh` validation.checks[] entry.
#
# Usage: bash scripts/check_vhs_fresh.sh [screenshots-dir]
#   screenshots-dir defaults to planning/artifacts/screenshots — pass an
#   alternate directory to point the check at a fixture (used by BE.7.L
#   task 4's evidence run).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCREENSHOTS_DIR="${1:-$REPO_ROOT/planning/artifacts/screenshots}"
BASELINE_DIR="$REPO_ROOT/tests/scenes"
MIN_PNG_BYTES=15000
MIN_NONBLANK_LINES=3

SOURCE_FILES=(
    "crates/bella/src/ui.rs"
    "crates/bella/src/theme.rs"
    "crates/bella-engine/src/markdown.rs"
    "crates/bella-engine/src/theme.rs"
    "crates/bella-engine/src/palette.rs"
)

if ! command -v vhs >/dev/null 2>&1; then
    echo "check_vhs_fresh.sh: vhs not found on PATH — vhs-fresh check skipped" >&2
    exit 0
fi

# commit_time_in_own_repo <path> — the commit time (unix seconds) of the
# last commit that touched <path>, resolved in WHICHEVER git repo actually
# owns that path (see REPO TRAP above), not assumed to be $REPO_ROOT.
commit_time_in_own_repo() {
    local path="$1"
    local real toplevel rel ts
    real="$(cd "$(dirname "$path")" && pwd -P)/$(basename "$path")"
    toplevel="$(git -C "$(dirname "$real")" rev-parse --show-toplevel 2>/dev/null || true)"
    if [[ -z "$toplevel" ]]; then
        echo ""
        return
    fi
    rel="${real#"$toplevel"/}"
    ts="$(git -C "$toplevel" log -1 --format=%ct -- "$rel" 2>/dev/null || true)"
    echo "$ts"
}

# Freshest commit time across the render/theme sources that actually exist.
newest_source_ts=0
newest_source_path=""
for src in "${SOURCE_FILES[@]}"; do
    src_path="$REPO_ROOT/$src"
    [[ -f "$src_path" ]] || continue
    ts="$(commit_time_in_own_repo "$src_path")"
    [[ -n "$ts" ]] || continue
    if [[ "$ts" -gt "$newest_source_ts" ]]; then
        newest_source_ts="$ts"
        newest_source_path="$src"
    fi
done

if [[ "$newest_source_ts" -eq 0 ]]; then
    echo "check_vhs_fresh.sh: FAIL — could not determine a commit time for any render/theme source; refusing to pass vacuously." >&2
    exit 1
fi

shopt -s nullglob
pngs=("$SCREENSHOTS_DIR"/*.png)
shopt -u nullglob

if [[ "${#pngs[@]}" -eq 0 ]]; then
    echo "check_vhs_fresh.sh: FAIL — no PNGs found under $SCREENSHOTS_DIR" >&2
    exit 1
fi

FAIL=0
CHECKED=0

for png in "${pngs[@]}"; do
    name="$(basename "$png" .png)"
    CHECKED=$((CHECKED + 1))

    case "$name" in
        wide_*)   tape="scripts/vhs/reference-wide.tape" ;;
        narrow_*) tape="scripts/vhs/reference-narrow.tape" ;;
        *)        tape="scripts/vhs/reference-wide.tape or scripts/vhs/reference-narrow.tape" ;;
    esac

    # (a) SANITY — byte-size floor.
    size="$(wc -c <"$png" | tr -d '[:space:]')"
    if [[ "$size" -lt "$MIN_PNG_BYTES" ]]; then
        echo "check_vhs_fresh.sh: FAIL — '$name.png' is only $size bytes (minimum $MIN_PNG_BYTES) — looks like a failed capture, not a real render. Re-run $tape." >&2
        FAIL=1
        continue
    fi

    # (a) SANITY — companion text scene must exist and contain the status line.
    companion="$BASELINE_DIR/${name}.txt"
    if [[ ! -f "$companion" ]]; then
        echo "check_vhs_fresh.sh: FAIL — '$name.png' has no companion text scene at tests/scenes/${name}.txt — cannot confirm it is a real capture. Re-run $tape." >&2
        FAIL=1
        continue
    fi
    nonblank_lines="$(grep -c '[^[:space:]]' "$companion" || true)"
    if [[ "$nonblank_lines" -lt "$MIN_NONBLANK_LINES" ]]; then
        echo "check_vhs_fresh.sh: FAIL — companion scene tests/scenes/${name}.txt has only $nonblank_lines non-blank line(s) (minimum $MIN_NONBLANK_LINES) — does not look like it carries bella's real status line/content. Re-run $tape." >&2
        FAIL=1
        continue
    fi

    # (b) FRESHNESS — PNG's own commit time vs. the newest source commit time.
    png_ts="$(commit_time_in_own_repo "$png")"
    if [[ -z "$png_ts" ]]; then
        echo "check_vhs_fresh.sh: FAIL — could not determine a commit time for '$name.png' (not committed in its own repo?). Re-run $tape." >&2
        FAIL=1
        continue
    fi
    if [[ "$png_ts" -lt "$newest_source_ts" ]]; then
        echo "check_vhs_fresh.sh: FAIL — '$name.png' (committed $png_ts) is older than $newest_source_path (committed $newest_source_ts) — the reference set is stale. Re-run $tape." >&2
        FAIL=1
        continue
    fi
done

if [[ "$FAIL" -ne 0 ]]; then
    echo "check_vhs_fresh.sh: FAILED — one or more reference PNGs failed sanity or freshness." >&2
    exit 1
fi

echo "check_vhs_fresh.sh: OK — $CHECKED PNG(s) sane and no older than $newest_source_path ($newest_source_ts)." >&2
