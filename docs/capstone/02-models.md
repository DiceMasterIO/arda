---
mode: prescriptive
generated_date: 2026-08-26
paths_covered: ["crates/arda-core/**"]
generated_at_commit: 9c48e00
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

## Observed — area drainage rewrite (2026-08-26)

Built shape. `format_version` is **2**: `objects.bin` record layouts
changed, so worlds written by format 1 are refused with the regenerate
remedy.

- `Cell` is a 33-byte little-endian row carrying all 17 fields of the artifact list. Populated: `height`, `terrain`, `cover`, `slope_milli_deg`, `aspect_deg`, `drainage_area_cells`, `discharge`, `watercourse_order`, `watercourse_width_dm`, `height_above_river_dm`, `wetness`. Still at `Default` pending the climate stage: `temperature`, `rainfall`, `moisture`, `forest_density`, and pending step 5: `road`, `built_by`.
- `watercourse_order` holds **true Strahler order**: heads are 1, and a cell takes the maximum incoming order plus one when two or more inflows share it. Order is computed over the routing graph, which passes through filled basins, so a river keeps its order across a lake.
- `watercourse_width_dm` follows the artifact's relation `w = 4·sqrt(Q)` — 1 m³/s gives about 4 m, 25 m³/s about 20 m.
- `height_above_river_dm` is Height Above Nearest Drainage, clamped at zero: routing runs on the filled surface while heights are raw, so a cell inside a basin can sit below the channel it drains to. `wetness` is a fixed-point topographic wetness index; **no `ln`, `tan`, or `atan2` reaches a sim path** (see `03-conventions.md`).
- `Lake` has a producer. A filled basin is recorded when it covers ≥ 100 cells **and** is ≥ 2,000 mm deep, carrying `surface`, `depth_mm`, `outlet`, and its cells; its cells take `terrain: Lake`. Sub-threshold basins fill for routing only and stay `Land`.
- `RiverSegment` is one network link — source or junction, downstream to the next junction, the sea, a lake, or the tile edge — and carries `feeds` and `ends` (`Terminus`), so, per the artifact, every segment knows which segment it feeds and how it ends. Channel cells are partitioned exactly once.
- Cells whose `terrain` is not `Land` store zero for `drainage_area_cells`, `discharge`, and `watercourse_order`. Routing still crosses them.

## Observed — continent climate and hydrology (2026-08-26, feature 02)

`format_version` is **3**: the continent layer gained real content, and
older worlds are refused — the manifest gate is now an **exact major
match** in both directions, with the regenerate remedy.

- `ContinentCell` is an 18-byte little-endian record per 1 km cell: `height: HeightMm (i32)`, `temperature: TempCentiC (i16)`, `rainfall: RainfallMm (u16)`, `regime: ClimateRegime (u8)`, `downstream: Option<u8>` (0–7 fixed neighbour order; the shared `NO_DOWNSTREAM = 255` sentinel is exported so `arda-gen` and the codec cannot desync), `catchment_km2: u32`, `discharge: DischargeMilli (u32, L/s)`. `ContinentOverview` (dims + row-major cells) round-trips through `formats::overview` behind `ARDAOVR\0`.
- `ContinentRiver` (id, catchment, mouth discharge, `feeds`, course of `KmCoord`s source→mouth) and `ContinentObjects` persist in `continent/objects.bin` behind `ARDACOB\0`, same tagged-section container as the area file. River ids are 1-based; **0 is reserved** as the on-wire "no river" sentinel for `feeds`.
- `KmCoord` (u16 pair) is the continent-cell index; the dead `ContinentCoord` was deleted in its favour.
- Decoders refuse dishonest input rather than wrapping: all size arithmetic on file-supplied values is checked (`UnexpectedEof` with the genuine, saturated-where-unrepresentable requirement), and dims past `i32::MAX` raise the dedicated `FormatError::DimensionsOverflow`.
- `ClimateRegime` is a closed enum (temperate/mediterranean/boreal/tropical); unknown discriminants are refused naming the field.

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
