---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/arda-core/**"]
---

> Prescriptive — written from the design interview, not from code.

# Models

All types planned in `crates/arda-core`. Storage is the world directory
(`mockup/02-world-layout.md`): custom fixed-layout little-endian binary
+ zstd, JSON manifest (`architecture-interview.md §Q5`). Worlds are
immutable after generation; single writer is the batch (§D5).

## Entities

| Entity | Storage | Purpose |
|---|---|---|
| `World` (manifest) | `world.json` | Seed, config echo, format_version, coordinate ranges, validation stats (`logic/01` step 9) |
| `ContinentGrid` | `continent/overview.bin` | 1 km cells: relief, climate, drainage, density (`logic/01`) |
| `Plate`, `Range`, `Region`, `Sea` | `continent/objects.bin` | Tectonic result + named features (`logic/01` §Q5, §Q8) |
| `ContinentRiver` | `continent/objects.bin` | Catchment ≥ ~3,000 km², named whole-course (`logic/01` §Q7) |
| `TileBundle` | `continent/objects.bin` | Per-area inputs: edge heights, entering rivers, regime, wind, density, road exits (`logic/01` step 10) |
| `AreaCells` | `areas/<ax>_<ay>/cells.bin` | 512×512 cells, the artifact's full field list |
| `RiverSegment`, `Lake`, `Settlement`, `Road`, `Crossing`, `Pass` | `areas/<ax>_<ay>/objects.bin` | The artifact's object lists (`logic/02`) |
| `BlockArchive` | `blocks/<ax>_<ay>.tiles.zst` | 64×64 tile-IDs per land cell, 2 B/square, + relaxed marks (`logic/03` §Q12) |
| `TileSet` | in `arda-core` (static) | The 200+ tile vocabulary, adjacency rules (§Q12), and per-tile static attributes: material, traversable, move cost, cover, hazard tags (`build-interview.md §Q5`) |
| `Realm` | `continent/objects.bin` | Name, seat town, member settlements, border course (`logic/06`, build §Q2) |
| `Building` | `areas/<ax>_<ay>/objects.bin` | Type, footprint squares, settlement id, occupant NPC ids (build §Q3) |
| `Npc` | `areas/<ax>_<ay>/objects.bin` | Named notables with SRD 5.1-style sheets; commoners derived on demand, never stored (build §Q4) |
| `Poi` | block layer | Sparse per-block items/POIs: containers, campsites, shrines, hazard sources (build §Q5) |

## Fields and types

Per-cell fields = the artifact's "What the finished map knows" list
verbatim (height, land/sea/lake, cover, slope, aspect, temperature,
rainfall, moisture, forest density, drainage area, discharge,
watercourse order/width, height-above-river, wetness, road class,
built-by). Field-by-field Rust types and bit widths are deferred to
build (`logic-interview.md §Q14`); constraints already fixed: fixed-point
or integer representations in sim-facing fields (§Q4 determinism),
2-byte tile IDs (§Q12). IDs: entities are addressed by (seed, tier,
coords) — no synthetic IDs (§D5).

## Relationships

Continent → TileBundle (1 per tile) → Area (consumes bundle only —
never neighbor areas; `logic/01` invariant). Area cells ↔ objects by
cell coords; blocks keyed by (area coords, cell coords). Continental
rivers/names referenced by area objects where courses continue across
tiles (§Q8 logic).

## Boundaries

Three representations, conversions in fixed places (`§Q5`):
in-memory types (arda-core) ↔ binary layers (arda-core::formats, the
only byte codec) ↔ export JSON (arda-render, snake_case SI schema with
`schema_version`, additive-only — `logic/04` §Q14).

## Validation

- Config validated before generation (ranges; `logic/01` preconditions).
- Continent validated post-run: land 25–90%, ≥1 range, ≥1 sea river, reroll ≤5 (`logic/01` §Q9).
- Statistical gates per area (`logic/02` invariants).
- Load validates manifest presence, format_version, archive integrity lazily (`logic/05`).

## Schema

No database. Binary layouts are the schema: hand-specified
little-endian, one spec section per layer file to be documented
alongside `arda-core::formats` (§Q5); `format_version` in the manifest
gates compatibility, major-versioned independently of crate semver
(§Q6). Evolution: additive within a major; breaking layout change bumps
the format major and loaders refuse with the regenerate remedy
(`logic/05`).
