---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: 6aba3102ce8f
paths_covered: [":(top)crates/**", ":(top)tests/**"]
capstone_version: 6.4
---

# Glossary

## Concepts

| Term | Meaning in Arda | Implementation |
|---|---|---|
| World | A seed/config manifest, area layers and sampled block archives on disk | `crates/arda-gen/src/orchestrator.rs:154` |
| Continent | 4 km tectonic simulation resampled to a 1 km relief/climate/hydrology grid | `crates/arda-gen/src/continent/mod.rs:115` |
| Area / cell | 512² cells of 100 m, a 51.2 km square; unit of Rayon generation | `crates/arda-core/src/coords.rs:7` |
| Block / square | 64² tile IDs; each square is 1,524 mm (five feet). Total 97.536 m is distinct from the nominal 100 m cell. | `crates/arda-core/src/coords.rs:9` |
| TileId | u16 ID into the current 24-entry skeleton vocabulary; not an area coordinate | `crates/arda-core/src/tiles.rs:9` |
| TileBundle | Shared-continent-derived edge heights, entering rivers and 53² climate/routing/basin patches | `crates/arda-gen/src/continent/bundles.rs:127` |
| EnteringRiver | Upstream catchment, discharge and order floor seeded at an area boundary cell | `crates/arda-gen/src/continent/bundles.rs:101` |
| Routing surface | Priority-filled relief used for drainage; stored terrain keeps raw depressions | `crates/arda-gen/src/area/fill.rs` |
| Lake | Filled basin retained after seam adjustment and ≥300 cells / ≥4 m maximum-depth filtering; no water-budget test | `crates/arda-gen/src/area/mod.rs:41` |
| Basin identity | Piecewise-constant continent depression surface used to coordinate seam-lake levels | `crates/arda-gen/src/continent/hydrology.rs:16` |
| Strahler order | Channel hierarchy; equal maximum incoming orders raise the rank, with entering-river floors | `crates/arda-gen/src/area/water.rs` |
| Reach / RiverSegment | Channel link between source/junction and junction/sea/lake/tile edge | `crates/arda-core/src/objects.rs:42` |
| HAND | Height above downstream drainage; no downstream channel stores saturated height-above-river | `crates/arda-gen/src/area/fields.rs:78` |
| Wetness | Integer drainage/slope-derived cell index, distinct from block neighbor wet_fraction | `crates/arda-gen/src/area/fields.rs`, `crates/arda-gen/src/block/constraints.rs:17` |
| Edge taper | 32-cell ramp suppressing erosion at the pinned rim | `crates/arda-gen/src/area/erosion.rs:54` |
| Relaxed block | Eight WFC attempts failed; first allowed tile (or ID 0) fills the block, flagged relaxed | `crates/arda-gen/src/block/wfc.rs:19` |
| Subseed | BLAKE3 key over world seed plus tier/stage/coordinates/attempt, driving ChaCha8 | `crates/arda-core/src/rng.rs:84` |
| format_version | Stored-world compatibility major, currently 3; load requires exact match | `crates/arda-core/src/formats/mod.rs:15` |
| Golden world | MICRO seed fixture with per-file fingerprints used for byte-reproducibility tests | `tests/golden_world.rs:69` |
| Trunk corridor / realm | Designed society/road constructs; no generation implementation yet | `logic/01-continent-generation.md`, `logic/06-society-generation.md`, `crates/arda-gen/src/` |
