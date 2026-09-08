#!/usr/bin/env bash
# Re-capture the VHS reference set until every scene clears its per-scene
# min_bytes floor, then regenerate celia's capture manifest.
#
# WHY THIS EXISTS. VHS/ttyd renders a blank frame for a random subset of
# scenes under CPU load — a real, measured failure, not a scene defect: the
# tapes already gate every Screenshot on `Wait+Screen` AND a settle, and the
# blanking happens below both, in the capture itself.
#
# THE ALGORITHM MATTERS MORE THAN THE RETRY COUNT. Demanding one run in which
# all ~24 scenes succeed simultaneously is a lottery: an SDLC engine did that
# 155 times in 78 minutes and never won. This ACCUMULATES instead — keep every
# capture that clears its floor, restore the rest, repeat. Each scene then only
# has to succeed once across runs, and it has converged in ONE run every time
# since.
#
# THE RESTORE BASELINE MUST COME FROM GIT. An earlier version of this loop
# snapshotted the working tree as its baseline, so once a scene blanked it
# "restored" one blank from another and made the damage permanent. `git
# checkout` is the only source known-good.
#
# Usage:  bash scripts/recapture_scenes.sh [max_runs]     (default 3)
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
HQ=$(cd ../.. && pwd)
VAULT=core/_planning/bella/artifacts/screenshots
SHOT=planning/artifacts/screenshots
CELIA=(cargo run --quiet --release --manifest-path ../celia/Cargo.toml -p celia-cli --)
MAXRUNS=${1:-3}
PER_TAPE_TIMEOUT=${PER_TAPE_TIMEOUT:-300}

FLOORS=$(mktemp); trap 'rm -f "$FLOORS"' EXIT
python3 - "$FLOORS" <<'PY'
import re, json, sys
txt = open('celia.toml').read(); out = {}
for blk in re.split(r'\n\[\[scene\]\]', txt):
    n = re.search(r'^\s*name\s*=\s*"([^"]+)"', blk, re.M)
    m = re.search(r'^\s*min_bytes\s*=\s*(\d+)', blk, re.M)
    if n and m: out[n.group(1)] = int(m.group(1))
json.dump(out, open(sys.argv[1], 'w'))
PY

under() { python3 -c "
import json,os,sys
f=json.load(open('$FLOORS'))
print(' '.join(n for n,fl in f.items()
      if os.path.getsize(os.path.join('$SHOT',n+'.png')) < fl
      if os.path.exists(os.path.join('$SHOT',n+'.png'))))"; }

for run in $(seq 1 "$MAXRUNS"); do
  for tape in reference-wide.tape reference-narrow.tape; do
    [ -f "scripts/vhs/$tape" ] || continue
    vhs "scripts/vhs/$tape" >/dev/null 2>&1 &
    vpid=$!; guard=$(( $(date +%s) + PER_TAPE_TIMEOUT ))
    while kill -0 $vpid 2>/dev/null; do
      if [ "$(date +%s)" -gt "$guard" ]; then
        kill -9 $vpid 2>/dev/null; pkill -9 -f ttyd 2>/dev/null
        echo "  $tape exceeded ${PER_TAPE_TIMEOUT}s — killed, continuing"; break
      fi
      sleep 5
    done
    wait $vpid 2>/dev/null
    # Keep what cleared its floor; restore the rest from git (never the tree).
    for s in $(under); do
      git -C "$HQ" checkout -- "$VAULT/$s.png" 2>/dev/null
    done
  done
  bad=$(under)
  if [ -z "$bad" ]; then
    echo "ALL SCENES ABOVE FLOOR after run $run"
    "${CELIA[@]}" capture --manifest celia.toml >/dev/null 2>&1 \
      && echo "capture manifest regenerated: tests/scenes/images/capture-manifest.toml" \
      || echo "WARNING: manifest regeneration failed — run celia capture by hand"
    exit 0
  fi
  echo "after run $run, still below floor:$bad"
done
echo "EXHAUSTED $MAXRUNS runs; still below floor:$bad"
exit 1
