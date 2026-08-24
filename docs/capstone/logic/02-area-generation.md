---
generated_date: 2026-08-24
scenario: area-generation
traces: [Q7, Q8, Q11]
artifact: ../mockup-artifact.md
---

# 02 — Area generation

Second batch stage, run once per 51.2 km tile (512×512 cells of 100 m).
The normative rules are the artifact's own sections — `Relief`, `Water`,
`Climate`, `Vegetation and ground cover`, `Where people settle`,
`Land use around settlements`, `Roads`, `What the finished map knows`,
`How plausibility is checked` in `../mockup-artifact.md` — confirmed
verbatim (mockup Q14, §Q11). This file records only what the continent
tier changes and the scenario frame; it does not restate the artifact.

## Trigger & preconditions

- Trigger: batch step [2/3], after continent validation passes; one run per tile, any order (tiles are independent — `01-continent-generation.md` invariants).
- Preconditions: the tile's input bundle exists (edge heights, entering rivers with catchment/discharge, climate regime, wind, edge moisture, settlement density, road-exit points).

## Steps

Artifact stage order, binding: relief → water → climate → vegetation →
settlement → land use → roads; each stage reads only prior stages'
outputs. Three amendments (§Q7, §Q8):

1. **Relief start**: initial surface = upsampled 1 km continent relief for this tile; uplift pattern derived from the coarse tectonic history. Replaces the artifact's crude ramp + two-noise coastal ramp — the coast is already in the coarse relief.
2. **Inputs**: every "regional description" item the artifact lists is read from the tile bundle, never from user config. Entering rivers are sealed in at their edge cells per the artifact.
3. **Edges**: edge-cell heights and every river entry/exit position are pinned to coarse-derived values (computed from coarse data + seed only) throughout the erosion run. Replaces "computed slightly larger and trimmed". Adjacent tiles therefore agree on every shared edge without reading each other.

All artifact constants stand as written (35° collapse, 40 L/s
watercourse threshold, Strahler ordering, floodplain bands 1 m/2.5 m/
2–15 m, treeline scoring, settlement refusals — 14° slope, 1.5 m above
watercourse — rank-size tiers, 8 km town spacing, 0.8 ha/person,
road-cost table, top-down network build). Scoring weights the artifact
gives only qualitatively are implementation-assumed and tunable (§Q11).

## Branches

- Per the artifact: ground-cover override order; settlement tier placement; crossing classification (bridge/ford/ferry); lake pass-through.
- Trunk roads must reach the bundle's road-exit points (replaces "edges the region's roads leave through") and connect to corridor crossings (§Q7).

## Unhappy paths

- A tile with no valid settlement site (all-mountain/all-marsh): allowed — settlement count follows density × habitable land; zero settlements ⇒ no fields, roads = through-corridors only (artifact's rules degrade gracefully; §Q11 assumed reading).
- All-ocean tiles: skip stages after relief/water; cells are sea, no objects (assumed).
- Interrupt: re-run of a tile is deterministic and independent; restart, no resume (mockup Q9).

## State transitions

None — creates `areas/<ax>_<ay>/` (cells + objects, `mockup/02-world-layout.md`) from the bundle; never mutates continent state or other tiles.

## Invariants

- Determinism per (seed, tile coords, bundle) — and the bundle itself is seed-derived.
- Edge agreement with all four neighbors, by construction (§Q8).
- The artifact's validation statistics hold per tile: Horton ratio 3–5, Hack exponent ~0.55, settlement rank-size, road sinuosity 1.2–1.4, ~1 ha farmland per inhabitant.
- No settlement on floodplain (<1.5 m above watercourse); no road across lake or sea.

## Outcomes & side effects

- Success: `areas/<ax>_<ay>/cells.bin` (every per-cell fact in the artifact's "What the finished map knows") + `objects.bin` (river segments, lakes, named settlements with tier/population/site-tags, roads, crossings, passes). Settlement names use the artifact's site-suffix scheme; region/river names from the continent are honored where objects continue across tiles (§Q8).
- Failure: none defined beyond process death — inputs were validated upstream; a panic on one tile halts the batch naming the tile (assumed).
