---
generated_date: 2026-09-07
scenarios: [batch-generate, inspect-volume]
generated_at_commit: 9c48e00
absorbed_from: features/02-continent-climate-hydrology@2026-08-26
capstone_version: 6.4
---

# 02 — The world directory (output volume)

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

What `generate` leaves behind and what a docker volume exposes for
"look at or import or copy the maps generated". This layout is
the loadable form ("generating and loading"); human-readable
images/JSON come from `export` (03). All file names and formats below
are assumed; the three-tier content split is confirmed.

## Layout

```text
worlds/w42/
├── world.json            # manifest: seed, config, arda version, stats
├── continent/
│   ├── overview.bin      # 1 km grid, real since format 3: relief, climate, drainage (18 B/cell)
│   └── objects.bin       # inter-area rivers (real since format 3); region boundaries, road exits pending
├── areas/
│   └── <ax>_<ay>/        # one per 51.2 km tile, e.g. 03_11/
│       ├── cells.bin     # 512×512 × per-cell properties (artifact field list)
│       └── objects.bin   # river segments, lakes, settlements, roads,
│                         # crossings, passes — object lists
└── blocks/
    └── <ax>_<ay>.tiles.zst   # 262,144 × 64×64 tile-ID grids, compressed
```

Element tree: manifest → one continent layer → N area layers → N block
archives; tiers share one coordinate system.

## Elements

| Element | Contains | Status / notes |
| --- | --- | --- |
| `world.json` | seed, config echo, version, validation stats — enough to regenerate or verify | (name assumed) |
| `continent/` | The tier that replaced per-area hand-written regional descriptions; feeds each area's edge inputs internally | Confirmed design |
| `areas/<ax>_<ay>/cells.bin` | Every per-cell fact: height, land/sea/lake, cover, slope/aspect, temperature, rainfall, moisture, forest density, drainage, watercourse order/width, height-above-river, wetness, road class, built-on | Confirmed design |
| `areas/<ax>_<ay>/objects.bin` | Named settlements with tier/population/site-tags; river segments; lakes; roads; crossings; passes | Confirmed design |
| `blocks/<ax>_<ay>.tiles.zst` | Materialized tactical layer: tile-ID per 5-ft square, ~0.2–0.4 GB/area compressed | Confirmed design |

## States

- **Complete**: all 190 area entries + block archives present; `world.json` stats match; ~61 GB at default size (arithmetic).
- **Partial** (aborted batch): manifest absent or unstamped — loaders refuse the directory (assumed).
- **Tampered/edited**: not supported; the world is a pure function of seed+config, edits are overwritten by regeneration.
