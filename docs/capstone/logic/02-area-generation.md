---
generated_date: 2026-09-07
scenario: area-generation
artifact: ../mockup-artifact.md
generated_at_commit: b0ce261
absorbed_from: features/03-climate-driven-refinement@2026-08-27
capstone_version: 6.4
---

# 02 — Area generation

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

Second batch stage, run once per 51.2 km tile (512×512 cells of 100 m).
The normative rules are the artifact's own sections — `Relief`, `Water`,
`Climate`, `Vegetation and ground cover`, `Where people settle`,
`Land use around settlements`, `Roads`, `What the finished map knows`,
`How plausibility is checked` in `../mockup-artifact.md` — confirmed
verbatim. This file records only what the continent
tier changes and the scenario frame; it does not restate the artifact.

## Trigger & preconditions

- Trigger: batch step [2/3], after continent validation passes; one run per tile, any order (tiles are independent — `01-continent-generation.md` invariants).
- Preconditions: the tile's input bundle exists (edge heights, entering rivers with catchment/discharge, climate regime, wind, edge moisture, settlement density, road-exit points).

## Steps

Artifact stage order, binding: relief → water → climate → vegetation →
settlement → land use → roads; each stage reads only prior stages'
outputs. Three amendments:

1. **Relief start**: initial surface = upsampled 1 km continent relief for this tile; uplift pattern derived from the coarse tectonic history. Replaces the artifact's crude ramp + two-noise coastal ramp — the coast is already in the coarse relief.
2. **Inputs**: every "regional description" item the artifact lists is read from the tile bundle, never from user config. Entering rivers are sealed in at their edge cells per the artifact.
3. **Edges**: edge-cell heights and every river entry/exit position are pinned to coarse-derived values (computed from coarse data + seed only) throughout the erosion run. Replaces "computed slightly larger and trimmed". Adjacent tiles therefore agree on every shared edge without reading each other.

All artifact constants stand as written (35° collapse, 40 L/s
watercourse threshold, Strahler ordering, floodplain bands 1 m/2.5 m/
2–15 m, treeline scoring, settlement refusals — 14° slope, 1.5 m above
watercourse — rank-size tiers, 8 km town spacing, 0.8 ha/person,
road-cost table, top-down network build). Scoring weights the artifact
gives only qualitatively are implementation-assumed and tunable.

## Branches

- Per the artifact: ground-cover override order; settlement tier placement; crossing classification (bridge/ford/ferry); lake pass-through.
- Trunk roads must reach the bundle's road-exit points (replaces "edges the region's roads leave through") and connect to corridor crossings.

## Unhappy paths

- A tile with no valid settlement site (all-mountain/all-marsh): allowed — settlement count follows density × habitable land; zero settlements ⇒ no fields, roads = through-corridors only (artifact's rules degrade gracefully; assumed reading).
- All-ocean tiles: skip stages after relief/water; cells are sea, no objects (assumed).
- Interrupt: re-run of a tile is deterministic and independent; restart, no resume.

## State transitions

None — creates `areas/<ax>_<ay>/` (cells + objects, `mockup/02-world-layout.md`) from the bundle; never mutates continent state or other tiles.

## Invariants

- Determinism per (seed, tile coords, bundle) — and the bundle itself is seed-derived.
- Edge agreement with all four neighbors, by construction.
- The artifact's validation statistics hold per tile: Horton ratio 3–5, Hack exponent ~0.55, settlement rank-size, road sinuosity 1.2–1.4, ~1 ha farmland per inhabitant.
- No settlement on floodplain (<1.5 m above watercourse); no road across lake or sea.

## Outcomes & side effects

- Success: `areas/<ax>_<ay>/cells.bin` (every per-cell fact in the artifact's "What the finished map knows") + `objects.bin` (river segments, lakes, named settlements with tier/population/site-tags, roads, crossings, passes). Settlement names use the artifact's site-suffix scheme; region/river names from the continent are honored where objects continue across tiles.
- Failure: none defined beyond process death — inputs were validated upstream; a panic on one tile halts the batch naming the tile (assumed).

## Observed — climate-driven water stage (2026-08-27, feature 03)

The Steps section's two long-standing prescriptions are now actually
implemented rather than approximated: entering rivers really are "sealed in at
their edge cells", and the artifact's **40 L/s watercourse threshold** is the
literal initiation rule. Climate, vegetation, settlement, land use and roads
remain unbuilt.

- **Rainfall** per cell is a smoothstep-bilinear sample of the bundle's 1 km rainfall patch — no added noise, because rain has no meaningful 100 m texture. Non-land cells store 0.
- **Discharge** is `Σ(upstream land rainfall) × 125 / 788,400` L/s plus any entering rivers' discharge, both accumulated in u64 down the existing routing tree and converted once per cell. At 800 mm/yr a 300-cell catchment yields ≈38 L/s, so the artifact's own "40 L/s ≈ 3 km² in a temperate climate" equivalence is now *emergent* rather than assumed — and a dry region legitimately produces no channel at all.
- **Entering rivers** seed their boundary cell with `catchment_km2 × 100` cells of drainage, their discharge, and a Strahler floor of `g(catchment)`. Where the two rules disagree — an arid crossing whose seed carries < 40 L/s — climate-driven initiation wins and the order floor does not propagate: the watercourse genuinely is not a watercourse there.
- **Seam lakes.** A basin touching the tile rim takes its surface from the shared continent routing surface (the max sample over its rim cells) instead of its local spill, so both sides derive the level from the same data. Membership stays local and is *trimmed* — a cell at or above that surface is not submerged and leaves the lake; a basin trimmed empty is not a lake. Outlets are derived afterwards, from the final lake set, so an outlet never names a cell another lake submerges.
- **Height above river** stores `u16::MAX` where no watercourse lies below a cell, so such land classifies **Dry**. Before this feature it stored 0 and every channel-less land cell was misclassified as marsh — invisible while channels were near-ubiquitous, wrong the moment initiation became climate-driven.

**Amendment to the Invariants section.** "Edge agreement with all four
neighbours" now covers entering rivers and the shared climate/routing patches,
not just edge heights. For straddling lakes the continent tier now supplies
**lake identity**: `basin_surface` gives every 1 km cell inside a filled
depression that depression's single surface, and a near-rim basin reads it by
nearest cell (never interpolated — the field is piecewise constant). Two
fragments of one depression therefore take the identical number whatever
their contact spans, measured at exactly 0 mm on a constructed two-tile case.
Two residues stay recorded in `open-items.md` #12 rather than guarded: no
fixture has yet produced a straddling basin that sees a continent depression
at all, so the exact path is proven on constructed input; and a rim abutting
two different depressions takes a max of two constants. Where the continent
tier sees no depression, the older bilinear rule still applies.

## Dimensions not in play

The retained design did not record a dimension-by-dimension exclusion list. This provenance repair leaves those exclusions unspecified rather than inventing decisions.
