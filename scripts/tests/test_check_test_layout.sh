#!/usr/bin/env bash
#
# test_check_test_layout.sh — fixture suite over scripts/check_test_layout.sh.
#
# Operates entirely on a throwaway `mktemp -d` fixture tree (never on the real
# crates/) so it is safe to run concurrently with anything else touching this
# repo. Covers the clean case (only tests/it/main.rs present -> exit 0) and
# the stray-file case (a bare crates/*/tests/*.rs -> exit 1, path reported).
#
#   ./scripts/tests/test_check_test_layout.sh
#
# Exit status 0 = all cases pass; non-zero = at least one failure.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECK_SCRIPT="$REPO_ROOT/scripts/check_test_layout.sh"

fail=0
n=0
check() { # check <description> <result: 0=pass>
  n=$((n + 1))
  if [ "$2" -eq 0 ]; then printf 'PASS (%d): %s\n' "$n" "$1"
  else printf 'FAIL (%d): %s\n' "$n" "$1"; fail=1; fi
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# --- fixture tree: two crates, each with a well-formed tests/it/main.rs -------------------
mkdir -p "$WORK/crates/alpha/tests/it"
mkdir -p "$WORK/crates/beta/tests/it"
cat > "$WORK/crates/alpha/tests/it/main.rs" <<'EOF'
mod render;
EOF
cat > "$WORK/crates/alpha/tests/it/render.rs" <<'EOF'
#[test]
fn it_works() {}
EOF
cat > "$WORK/crates/beta/tests/it/main.rs" <<'EOF'
mod smoke;
EOF
cat > "$WORK/crates/beta/tests/it/smoke.rs" <<'EOF'
#[test]
fn smoke() {}
EOF

# --- git-status invariant helper (nothing outside $WORK is touched) -----------------------
if command -v git >/dev/null 2>&1 && git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  GIT_STATUS_BEFORE="$(git -C "$REPO_ROOT" status --porcelain scripts/ crates/ 2>/dev/null)"
else
  GIT_STATUS_BEFORE=""
fi

# --- Case 1: clean fixture tree (only tests/it/main.rs) -> exit 0 -------------------------
bash "$CHECK_SCRIPT" "$WORK" > "$WORK/out1.log" 2>&1
RC1=$?
if [ "$RC1" -eq 0 ]; then R=0; else R=1; fi
check "clean tree (only tests/it/main.rs per crate) exits 0" "$R"

# --- Case 2: a stray crates/*/tests/*.rs (direct child of tests/) -> exit 1, path reported -
echo '// throwaway' > "$WORK/crates/alpha/tests/zz.rs"
bash "$CHECK_SCRIPT" "$WORK" > "$WORK/out2.log" 2>&1
RC2=$?
if [ "$RC2" -ne 0 ] && grep -q "crates/alpha/tests/zz.rs" "$WORK/out2.log"; then R=0; else R=1; fi
check "stray tests/*.rs file exits non-zero and names the offending path" "$R"

# --- Case 3: removing the stray file restores exit 0 --------------------------------------
rm "$WORK/crates/alpha/tests/zz.rs"
bash "$CHECK_SCRIPT" "$WORK" > "$WORK/out3.log" 2>&1
RC3=$?
if [ "$RC3" -eq 0 ]; then R=0; else R=1; fi
check "removing the stray file restores exit 0" "$R"

# --- Case 4: a crate with no tests/ directory at all is not a failure ---------------------
mkdir -p "$WORK/crates/gamma/src"
bash "$CHECK_SCRIPT" "$WORK" > "$WORK/out4.log" 2>&1
RC4=$?
if [ "$RC4" -eq 0 ]; then R=0; else R=1; fi
check "a crate with no tests/ directory does not fail the check" "$R"

# --- invariant: this suite never touches the real repo tree -------------------------------
if command -v git >/dev/null 2>&1 && git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  GIT_STATUS_AFTER="$(git -C "$REPO_ROOT" status --porcelain scripts/ crates/ 2>/dev/null)"
  if [ "$GIT_STATUS_BEFORE" = "$GIT_STATUS_AFTER" ]; then R=0; else R=1; fi
else
  R=0
fi
check "repo tree (scripts/, crates/) is unchanged by this run" "$R"

if [ "$fail" -eq 0 ]; then
  echo "All $n checks passed."
else
  echo "$n checks run; failures above."
fi
exit "$fail"
