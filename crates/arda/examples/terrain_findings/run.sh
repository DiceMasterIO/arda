#!/usr/bin/env bash
# Regenerates the terrain re-check panel and measures it (goal 19).
#   run.sh [PANEL_NAME ...]        default: every world in the panel
# Output root: out/recheck (gitignored). Needs jq and sha256sum.
# Worlds that already exist are reused (never deleted); delete one by hand to
# regenerate it. Set VERIFY="micro42-r7 micro99-r8" to regenerate those into
# <name>-again and compare whole-tree digests (byte-for-byte check).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
OUT=${OUT:-out/recheck}
mkdir -p "$OUT/img" "$OUT/records" "$OUT/measures"

# name|seed|size (WxH km or micro)|recipe|extra generate flags
PANEL=(
  "micro99-r7|99|micro|7|"
  "micro99-r8|99|micro|8|"
  "micro42-r7|42|micro|7|"
  "micro42-r8|42|micro|8|"
  "s42-200x300-r7|42|200x300|7|"
  "s42-200x300-r8|42|200x300|8|"
  "s436342-200x300-r7|436342|200x300|7|"
  "s436342-200x300-r8|436342|200x300|8|"
  "s42-200x300-dry-r7|42|200x300|7|--latitude 15,35"
  "s42-500x1000-r7|42|500x1000|7|"
  "s42-500x1000-r8|42|500x1000|8|"
)

cargo build --release -q -p arda-cli
cargo build --release -q -p arda --example terrain_findings
ARDA=target/release/arda
TOOL=target/release/examples/terrain_findings
REV=$(git rev-parse HEAD)

tree_digest() { (cd "$1" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1); }

gen() { # name seed size recipe extra dir
  local size_flag; if [ "$3" = micro ]; then size_flag=--micro; else size_flag="--size $3"; fi
  # shellcheck disable=SC2086
  $ARDA generate --seed "$2" $size_flag --terrain fine --recipe "$4" $5 --out "$6" > "$6.log" 2>&1
}

want() { [ $# -eq 0 ] && return 0; for n in "$@"; do [ "$n" = "$CUR" ] && return 0; done; return 1; }

for row in "${PANEL[@]}"; do
  IFS='|' read -r CUR seed size recipe extra <<< "$row"
  want "$@" || continue
  W="$OUT/$CUR"
  gen_s=0
  # keep the generation time recorded when the world was first made
  [ -f "$OUT/records/$CUR.json" ] && gen_s=$(jq .seconds.generate "$OUT/records/$CUR.json")
  if [ ! -f "$W/world.json" ]; then
    t0=$SECONDS; gen "$CUR" "$seed" "$size" "$recipe" "$extra" "$W"; gen_s=$((SECONDS - t0))
  fi
  t0=$SECONDS; $TOOL "$W" --json "$OUT/measures/$CUR.json" > /dev/null; tool_s=$((SECONDS - t0))
  t0=$SECONDS
  $ARDA export --world "$W" --overview --quality 8k --style atlas --out "$OUT/img/$CUR-atlas8k" > /dev/null
  M="$OUT/measures/$CUR.json"
  aw=$(jq .areas[0] "$M"); ah=$(jq .areas[1] "$M")
  pick() { # jq path of an [x,y] cell pair -> "ax,ay" or fallback
    local v; v=$(jq -r "$1 | if length==2 then \"\(.[0]/512|floor),\(.[1]/512|floor)\" else empty end" "$M" 2>/dev/null || true)
    echo "${v:-$((aw/2)),$((ah/2))}"
  }
  declare -A AREAS=(
    [drainage]=$(pick .measures.worst.drainage.spectral_window_centre_xy)
    [drainage-run]=$(pick .measures.worst.drainage.longest_run_xy)
    [coast]=$(pick .measures.worst.coast.longest_run_xy)
    [basin]=$(pick .measures.worst.basin_centre_xy)
    [ref-centre]="$((aw/2)),$((ah/2))"
    [ref-quarter]="$((aw/4)),$((ah/4))"
  )
  for k in "${!AREAS[@]}"; do
    $ARDA export --world "$W" --area "${AREAS[$k]}" --out "$OUT/img/$CUR-area-$k" > /dev/null
  done
  unset AREAS
  img_s=$((SECONDS - t0))
  digest=$(tree_digest "$W")
  imgs=$(find "$OUT/img" -path "*/$CUR-*" -name '*.png' -print0 | sort -z | xargs -0 sha256sum | jq -R -s -c 'split("\n")|map(select(length>0)|split("  ")|{(.[1]):.[0]})|add')
  jq -n --arg name "$CUR" --arg rev "$REV" --argjson seed "$seed" --arg size "$size" --argjson recipe "$recipe" \
    --arg extra "$extra" --argjson gen "$gen_s" --argjson tool "$tool_s" --argjson img "$img_s" \
    --arg digest "$digest" --slurpfile world "$W/world.json" --arg mhash "$(sha256sum "$M" | cut -d' ' -f1)" \
    --argjson imgs "$imgs" \
    '{name:$name, git_rev:$rev, seed:$seed, size:$size, recipe:$recipe, extra:$extra,
      seconds:{generate:$gen, measure:$tool, images:$img},
      world_json:{format_version:$world[0].format_version, arda_version:$world[0].arda_version},
      world_tree_sha256:$digest, measures_sha256:$mhash, images_sha256:$imgs}' > "$OUT/records/$CUR.json"
  echo "done $CUR (generate ${gen_s}s, measure ${tool_s}s, images ${img_s}s)"
done

for n in ${VERIFY:-}; do
  IFS='|' read -r CUR seed size recipe extra <<< "$(printf '%s\n' "${PANEL[@]}" | grep "^$n|")"
  rm -rf "$OUT/$n-again"; gen "$n" "$seed" "$size" "$recipe" "$extra" "$OUT/$n-again"
  a=$(tree_digest "$OUT/$n"); b=$(tree_digest "$OUT/$n-again")
  [ "$a" = "$b" ] && echo "byte-identical on rerun: $n ($a)" || { echo "MISMATCH: $n"; exit 1; }
  rm -rf "$OUT/$n-again" "$OUT/$n-again.log"
done
