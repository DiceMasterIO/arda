---
generated_date: 2026-08-24
scenarios: [batch-generate, inspect-volume]
implements: [Q8, Q11, Q17, Q22]
generated_at_commit: 8d3c9d9
---

# 02 — The world directory (output volume)

What `generate` leaves behind and what a docker volume exposes for
"look at or import or copy the maps generated" (§Q17). This layout is
the loadable form (§Q17 "generating and loading"); human-readable
images/JSON come from `export` (03). All file names and formats below
are assumed; the three-tier content split is interview-backed.

## Layout

```text
worlds/w42/
├── world.json            # manifest: seed, config, arda version, stats
├── continent/
│   ├── overview.bin      # coarse continent grid: relief, climate, regions
│   └── objects.bin       # inter-area rivers, region boundaries, road exits
├── areas/
│   └── <ax>_<ay>/        # one per 51.2 km tile, e.g. 03_11/
│       ├── cells.bin     # 512×512 × per-cell properties (§Q11 list)
│       └── objects.bin   # river segments, lakes, settlements, roads,
│                         # crossings, passes — §Q11 object lists
└── blocks/
    └── <ax>_<ay>.tiles.zst   # 262,144 × 64×64 tile-ID grids, compressed
```

Element tree: manifest → one continent layer → N area layers → N block
archives; tiers share one coordinate system (§Q8).

## Elements

| Element | Contains | Traces to |
|---|---|---|
| `world.json` | seed, config echo, version, validation stats — enough to regenerate or verify | Q9, Q13 (name assumed) |
| `continent/` | The tier that replaced per-area hand-written regional descriptions; feeds each area's edge inputs internally | Q15, Q22 |
| `areas/<ax>_<ay>/cells.bin` | Every §Q11 per-cell fact: height, land/sea/lake, cover, slope/aspect, temperature, rainfall, moisture, forest density, drainage, watercourse order/width, height-above-river, wetness, road class, built-on | Q11 |
| `areas/<ax>_<ay>/objects.bin` | Named settlements with tier/population/site-tags; river segments; lakes; roads; crossings; passes | Q11 |
| `blocks/<ax>_<ay>.tiles.zst` | Materialized tactical layer: tile-ID per 5-ft square, ~0.2–0.4 GB/area compressed | Q22 |

## States

- **Complete**: all 190 area entries + block archives present; `world.json` stats match; ~61 GB at default size (§Q22 arithmetic).
- **Partial** (aborted batch): manifest absent or unstamped — loaders refuse the directory (assumed).
- **Tampered/edited**: not supported; the world is a pure function of seed+config, edits are overwritten by regeneration (§Q9).
