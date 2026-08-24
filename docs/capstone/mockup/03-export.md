---
scenarios: [export-vtt, inspect-volume]
implements: [Q16, Q17, Q22, Q12]
---

# 03 — `arda export` (CLI): images + JSON on demand

The human/VTT-facing output (§Q16, §Q17: "export the images and json
properties of each grid/cell"). Runs against a generated world; block
images are rendered here, never in the batch (§Q22). Deterministic: the
same request always yields byte-identical artifacts (§Q9). Command
shape assumed; artifact kinds interview-backed.

## Layout

```text
$ arda export --world ./worlds/w42 --area 03_11 --format png,json --out ./maps
export — area 03_11 (51.2 km, 512×512 cells)
  area-map.png          overview render, 1 px/cell
  cells.json            per-cell properties, §Q11 schema
  objects.json          settlements, rivers, roads, crossings, passes
done — 3 files, 71 MB

$ arda export --world ./worlds/w42 --area 03_11 --cell 300,128 --format png,json
export — cell 300,128 (100 m block, 64×64 squares of 5 ft)
  block-300_128.png     battle-map render from stored tile-IDs (§Q12 WFC output)
  block-300_128.json    square grid: tile IDs + cell properties
done — 2 files, 9 MB
```

Element tree: command → target selector (world / area / cell) → format
selector → per-artifact result lines.

## Elements

| Element | Exact form | Does | Traces to |
|---|---|---|---|
| `export` subcommand | `arda export` | Renders images and JSON from a stored world; only reader of `blocks/*.tiles.zst` besides the crate | Q17, Q22 (name assumed) |
| `--world <dir>` | required | The generated world directory (02) | Q17 |
| `--area <ax_ay>` | optional; omit = continent scope | Selects the 51.2 km tile | Q8 (addressing scheme assumed) |
| `--cell <x,y>` | optional, needs `--area` | Selects one 100 m block → battle-map export | Q8, Q16 |
| `--format png,json` | default both | §Q17's two artifact kinds; PNG at 8 px/square for blocks, 1 px/cell for areas (resolutions assumed) | Q17 |
| `--out <dir>` | default `.` | Where artifacts land (docker: the volume) | Q17 |
| continent scope | no `--area` | Exports continent overview image + region/settlement JSON | Q22 (assumed) |

## States

- **Success**: artifacts listed with sizes; re-export overwrites byte-identically (§Q9).
- **Empty**: world directory missing/partial → refuse, point at `generate` (assumed).
- **Loading**: block renders are seconds-per-cell; area-scope image ~1 min (derived from §Q22 sizes; assumed).
- **Error**: out-of-range `--area`/`--cell` → exit non-zero naming valid ranges from `world.json`.
