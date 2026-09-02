#!/usr/bin/env bash
# check_test_layout.sh — fails if any crates/*/tests/*.rs exists other than
# crates/<crate>/tests/it/main.rs.
#
# BE.7.M consolidated each crate's integration tests into a single binary
# (crates/<crate>/tests/it/main.rs, one `mod <name>;` per moved file) so that
# adding integration test files does not multiply the number of link steps —
# measured on this repo 2026-09-01: 8 extra binaries in bella-engine took a
# one-line-edit rebuild from 2.5s to 4.2s, 6 extra in bella from 1.9s to
# 3.1s. This check exists so the next block cannot silently re-add a stray
# `tests/<name>.rs` binary and undo that.
#
# A "stray file" is anything matching `crates/*/tests/*.rs` at the top level
# of a crate's tests/ directory — i.e. any *.rs file that is a direct child
# of tests/, since only tests/it/main.rs is allowed to sit there. Files
# nested under tests/it/ (main.rs and its `mod`-ed siblings) are fine and are
# not matched by this glob.
#
# Usage: ./scripts/check_test_layout.sh [ROOT]
#   ROOT defaults to the repo root (parent of this script's directory).
#
# Exit 0 — only tests/it/main.rs exists in every crate's tests/ directory
#          (or a crate has no tests/ directory at all).
# Exit 1 — a stray crates/*/tests/*.rs file was found; each offending path is
#          printed on its own line.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${1:-$(cd "$SCRIPT_DIR/.." && pwd)}"

shopt -s nullglob

stray=()
for tests_dir in "$ROOT"/crates/*/tests; do
  [ -d "$tests_dir" ] || continue
  for f in "$tests_dir"/*.rs; do
    stray+=("$f")
  done
done

if [ "${#stray[@]}" -gt 0 ]; then
  echo "FAIL: stray tests/*.rs file(s) outside tests/it/main.rs:" >&2
  for f in "${stray[@]}"; do
    echo "  $f" >&2
  done
  echo "  fix: move the test module into tests/it/ and add a \`mod <name>;\` line" >&2
  echo "  to tests/it/main.rs instead of leaving it as its own tests/*.rs binary." >&2
  exit 1
fi

exit 0
