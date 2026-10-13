#!/usr/bin/env bash
# Runs every wall-clock release gate on the release machine and writes one
# Markdown receipt (1.0 task 09; OI-28, logic/16 Invariant 7).
#
#   tools/release-gates.sh                 build once, run every gate
#   tools/release-gates.sh --with-benches  also run the arda-gen budget benches (minutes)
#   tools/release-gates.sh --from-log FILE parse a saved gate log only (no run)
#
# Every gate prints `GATE <name> <measured_ms> <budget_ms>` before it asserts.
# The receipt goes to stdout and target/release-gates/receipt-<commit>.md.
# Exit status: 0 every gate present and under budget; 1 a gate failed, a
# cargo target failed, or an expected GATE line is missing; 2 usage or build
# error. Run it on an idle machine; the receipt records the load average.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Gate names every run must report (benches only with --with-benches).
EXPECTED=(
  refine_block refine_quarter_block tactical_64x64_128ppsq
  server_cached_layout_png server_cached_cell_tile server_cold_cell_json_scene
  server_cold_window_128 server_open world_load server_area_cells_json
  server_area_cells_bin server_warm_overview_tile
  midzoom_warm_tile_z7 midzoom_refine_nodes npc_city_20k names_100k
  e2e_cold_block_png_64 e2e_cold_quarter_window_64
)
BENCH_EXPECTED=(gen_area_erosion gen_continent_stage)

with_benches=0
from_log=""
while [ $# -gt 0 ]; do
  case "$1" in
    --with-benches) with_benches=1 ;;
    --from-log) shift; from_log="${1:?--from-log needs a file}" ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

out=target/release-gates
mkdir -p "$out"
log="$out/gates.log"
load_before="$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo n/a)"
failed_targets=()

run() { # run <label> <cargo args...>
  local label="$1"; shift
  echo "== $label" | tee -a "$log" >&2
  if ! "$@" >>"$log" 2>&1; then
    failed_targets+=("$label")
    echo "   cargo target failed: $label" >&2
  fi
}

if [ -n "$from_log" ]; then
  cp "$from_log" "$log"
else
  : >"$log"
  echo "building release test binaries once" >&2
  cargo test --release --no-run --workspace >/dev/null 2>&1 \
    || { echo "release build failed" >&2; exit 2; }
  gate=(-- --ignored --nocapture --test-threads=1)
  run e2e_village cargo test --release --test e2e_village "${gate[@]}"
  run arda-refine cargo test --release -p arda-refine --test perf "${gate[@]}"
  run arda-tactical cargo test --release -p arda-tactical --test timing "${gate[@]}"
  run arda-server cargo test --release -p arda-server --test world_api "${gate[@]}"
  run arda-midzoom-world cargo test --release -p arda-midzoom --test world "${gate[@]}"
  run arda-midzoom-checks cargo test --release -p arda-midzoom --test checks "${gate[@]}"
  run arda-npc cargo test --release -p arda-npc --test performance "${gate[@]}"
  run arda-names cargo test --release -p arda-names --test performance "${gate[@]}"
  if [ "$with_benches" = 1 ]; then
    run bench-area-erosion cargo bench -p arda-gen --bench area_erosion
    run bench-continent-stage cargo bench -p arda-gen --bench continent_stage
  fi
fi
load_after="$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo n/a)"

expected=("${EXPECTED[@]}")
if [ "$with_benches" = 1 ]; then expected+=("${BENCH_EXPECTED[@]}"); fi

# GATE lines can follow a `test name ... ` prefix, so match anywhere on the line.
gates="$(grep -oE 'GATE [A-Za-z0-9_]+ [0-9.]+ [0-9.]+' "$log" || true)"
mems="$(grep -oE 'MEM [A-Za-z0-9_]+ [0-9]+' "$log" || true)"

commit="$(git rev-parse --short=9 HEAD)"
dirty=""; if [ -n "$(git status --porcelain --untracked-files=no)" ]; then dirty="-dirty"; fi
receipt="$out/receipt-$commit.md"
status=0
{
  echo "# Release gates $(date -u +%F)"
  echo
  echo "- commit: \`$commit$dirty\`"
  echo "- date: $(date -u '+%F %H:%M UTC')"
  echo "- machine: $(lscpu | sed -n 's/^Model name:[[:space:]]*//p'), $(nproc) threads, $(free -g | awk '/^Mem:/{print $2}') GiB RAM, kernel $(uname -r)"
  echo "- rustc: $(rustc --version)"
  echo "- load average (1/5/15 min) before: $load_before, after: $load_after"
  echo
  echo "| Gate | Measured ms | Budget ms | Result |"
  echo "|---|---:|---:|---|"
  for name in "${expected[@]}"; do
    line="$(printf '%s\n' "$gates" | grep -E "^GATE $name " | tail -1 || true)"
    if [ -z "$line" ]; then
      echo "| $name | - | - | FAIL (no GATE line) |"; status=1; continue
    fi
    read -r _ _ measured budget <<<"$line"
    if awk -v m="$measured" -v b="$budget" 'BEGIN{exit !(m < b)}'; then
      echo "| $name | $measured | $budget | pass |"
    else
      echo "| $name | $measured | $budget | FAIL (over budget) |"; status=1
    fi
  done
  if [ -n "$mems" ]; then
    echo
    echo "Memory: $(printf '%s\n' "$mems" | awk '{printf "%s%s %.0f MiB", (NR>1?"; ":""), $2, $3/1024}')"
  fi
  if [ "${#failed_targets[@]}" -gt 0 ]; then
    echo
    echo "Failed cargo targets: ${failed_targets[*]}"
    status=1
  fi
  echo
  if [ "$status" = 0 ]; then echo "Result: PASS"; else echo "Result: FAIL"; fi
} | tee "$receipt"
# tee's pipeline hides the loop's status; recompute from the receipt.
if grep -q '^Result: PASS' "$receipt"; then exit 0; else exit 1; fi
