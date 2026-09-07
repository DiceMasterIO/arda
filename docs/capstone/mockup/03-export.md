---
generated_date: 2026-09-07
scenarios: [export-vtt, inspect-volume]
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 03 — `arda export` (CLI): images + JSON on demand

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

The human/VTT-facing output (: "export the images and json
properties of each grid/cell"). Runs against a generated world; block
images are rendered here, never in the batch. Deterministic: the
same request always yields byte-identical artifacts. Command
shape assumed; artifact kinds confirmed.

## Layout

```text
$ arda export --world./worlds/w42 --area 03_11 --format png,json --out./maps
export — area 03_11 (51.2 km, 512×512 cells)
  area-map.png          overview render, 1 px/cell
  cells.json            per-cell properties, artifact schema
  objects.json          settlements, rivers, roads, crossings, passes
done — 3 files, 71 MB

$ arda export --world./worlds/w42 --area 03_11 --cell 300,128 --format png,json
export — cell 300,128 (100 m block, 64×64 squares of 5 ft)
  block-300_128.png     battle-map render from stored tile-IDs (WFC output)
  block-300_128.json    square grid: tile IDs + cell properties
done — 2 files, 9 MB
```

Element tree: command → target selector (world / area / cell) → format
selector → per-artifact result lines.

## Elements

| Element | Exact form | Does | Status / notes |
| --- | --- | --- | --- |
| `export` subcommand | `arda export` | Renders images and JSON from a stored world; only reader of `blocks/*.tiles.zst` besides the crate | (name assumed) |
| `--world <dir>` | required | The generated world directory (02) | Confirmed design |
| `--area <ax_ay>` | optional; omit = continent scope | Selects the 51.2 km tile | (addressing scheme assumed) |
| `--cell <x,y>` | optional, needs `--area` | Selects one 100 m block → battle-map export | Confirmed design |
| `--format png,json` | default both | Two artifact kinds; PNG at 8 px/square for blocks, 1 px/cell for areas (resolutions assumed) | Confirmed design |
| `--out <dir>` | default `.` | Where artifacts land (docker: the volume) | Confirmed design |
| continent scope | no `--area` | Exports continent overview image + region/settlement JSON | (assumed) |

## States

- **Success**: artifacts listed with sizes; re-export overwrites byte-identically.
- **Empty**: world directory missing/partial → refuse, point at `generate` (assumed).
- **Loading**: block renders are seconds-per-cell; area-scope image ~1 min (derived from the planned grid sizes; assumed).
- **Error**: out-of-range `--area`/`--cell` → exit non-zero naming valid ranges from `world.json`.
