---
generated_date: 2026-08-24
scenario: continent-generation
traces: [Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9]
generated_at_commit: 8d3c9d9
---

# 01 — Continent generation

First stage of the batch (`mockup/01-generate.md` step [1/3]). Produces
the continent grid, continent objects, and every area tile's input
bundle. All rules below are the user's decisions from
`../logic-interview.md` (§Q cited per rule); named constants are
tunable defaults.

## Trigger & preconditions

- Trigger: `arda generate` batch start; also callable via the crate (§mockup Q17).
- Preconditions: validated config (continent size, default 500×1000 km; latitude band, default 35–55°N; mean density, default 15 people/km²) and a u64 seed. Nothing else — no prior state exists (§Q2, mockup Q22).

## Steps

1. **Plate seeding** (§Q5): on a domain ~2× the visible continent, seed 8–14 plates as Voronoi regions from the seed. Each plate gets a crust type (continental/oceanic) and a drift vector. The domain rim is forced oceanic so map edges are guaranteed ocean (mockup Q22).
2. **Time-stepped tectonics, coupled** (§Q3, §Q4): 4 km sim grid (125×250 cells), ~100 steps of ~1 Myr. Per step: plates move along drift vectors (vectors re-roll slightly every ~20 steps); boundary uplift applies by relative motion — cont-cont convergent → range, ocean-cont → subduction arc + volcanoes, divergent → rift, transform → fault zone (§Q5); then coarse stream-power erosion and drainage respond, so rivers and valleys co-evolve with the ranges — the artifact's causality rule at continental scale.
3. **Coast** (§Q5): base elevation from crust type (isostasy), modified by accumulated tectonics + erosion; a fixed sea level cuts the coastline. Shelf seas, bays, capes are emergent.
4. **Upsample** (§Q4): final relief interpolated 4 km → 1 km working grid (noise-refined, seed-derived).
5. **Climate** (§Q6): prevailing wind from the latitude belt (westerlies in the default band); moisture-advection pass over the 1 km relief (same rules as the area tier's rain — release on climb, recharge over water) → rainfall + rain shadows; temperature = latitude + 6.5 °C/km lapse + continentality (distance to sea). Every 1 km cell gets a climate regime (mediterranean/temperate/boreal/tropical by band position and elevation).
6. **Hydrology** (§Q7): walk the final drainage tree; watercourses with catchment ≥ ~3,000 km² (assumed default) become continent river objects with course, catchment, discharge.
7. **Human geography** (§Q7): habitability score per 1 km cell (climate, relief, coast, river valleys) → settlement-density map scaled to the configured mean; trunk-corridor network = least-cost paths between high-density basins over the 1 km relief (cost rules as the artifact's roads, coarse).
8. **Naming** (§Q8): continent, regions/provinces (density basins bounded by ranges/rivers/seas), mountain ranges, major rivers (one name for the whole course), seas/bays — site-derived naming per the artifact's settlement-name philosophy.
9. **Validation** (§Q9): land fraction within 25–90%, ≥1 range above 1,500 m, ≥1 major river reaching the sea (all tunable).
10. **Tile bundles** (§Q7, §Q8): for each 51.2 km tile, emit its input bundle — edge heights, entering rivers (edge position, catchment, discharge), climate regime + wind + edge moisture, settlement density, road-exit points where trunk corridors cross tile edges. Bundles are computed from coarse data + seed only, so adjacent tiles are mirror-consistent by construction.

## Observed — continent rebuild (2026-08-26)

Built shape of steps 1-4. Steps 5-9 (climate, hydrology objects, human
geography, naming, validation stats) remain unbuilt.

1. **Plate seeding.** Count scales with domain area (~1 per 2,800 sim cells), so boundary density — and with it the spacing of mountain belts — is the same at any continent size; a fixed 8-14 made the default world a flat plain while a quarter-size one came out mountainous. Crust follows position rather than chance: continental in the core, oceanic beyond 0.8 of the half-width, mixed between. Leaving it to chance meant the central plates were sometimes all oceanic and the continent failed to exist.
2. **Boundary lookup is domain-warped.** Raw Voronoi gives straight-line plate boundaries, and since the coast follows them the continent came out a polygon.
3. **Tectonics raises belts, not lines.** Uplift spreads across an orogenic belt either side of each boundary — 9 cells for collisions, 6 for arcs — with a cubic falloff. A one-cell ridge plus heavy diffusion produced a 118 m plateau across a UK-sized landmass. Plates drift each step (`centre + drift * step`), so belts migrate and the continent becomes a collage of orogens rather than one range. Uplift is normalised so its 99.5th percentile hits a target relief, making hypsometry independent of continent size, and subsidence is floored so a rift cannot drown the interior.
4. **Coast.** Crust is blurred into a **shelf gradient** so the continent meets the ocean over tens of kilometres instead of one 4 km cell, then blended with a centred mask. Sea level is crossed at mid-continentality, not at 0.87 as a straight interpolation gives. Noise is applied on **both** sides of sea level; restricting it to land left the sea floor as pure bilinear interpolation and the coastline followed the 4 km grid as rectangular steps.
5. **Coarse erosion runs here** (step 2's "erosion and drainage respond"), globally on the 1 km grid: priority-flood, D8 by steepest descent, stream-power incision clamped so a channel never cuts below what it drains into, and isotropic hillslope creep. Running it globally rather than per-tile is what lets valleys cross tile boundaries and leaves no pinned-rim seams.

Measured against Earth (seeds 42 and 7, default size): median land elevation 303-337 m against 330 m; land above 500 m 37-41% against 33%; lake area 0.5-1.1% against ~1%.

**Known limitation.** Terrain is built from value noise on a square lattice, which is anisotropic — its gradients favour the lattice axes, so steepest descent does too and only ~17% of flow directions are diagonal against an isotropic 50%. Isotropic gradient noise is the fix; one attempt produced blocky coastlines and was reverted.

## Branches

- Boundary type (step 2) is decided solely by the two plates' crust types and relative motion (§Q5).
- Validation failure (step 9) → re-run steps 1–8 with derived subseed `(seed, attempt)`; attempt < 5 else hard error (§Q9).

## Unhappy paths

- Degenerate continent: caught by step 9; ≤5 deterministic rerolls, then exit non-zero naming the failed check (§Q9).
- Invalid config (size/latitude/density out of range): refused before step 1 (mockup 01 States).
- Interrupt mid-run: no partial continent is consumable; re-run restarts identically (mockup Q9 determinism).

## State transitions

None — this scenario has no persistent entities before it runs; it creates the world's first state (`continent/` layer + tile bundles, `mockup/02-world-layout.md`).

## Invariants

- Same (seed, config) → identical continent, including reroll sequence (§Q9, mockup Q9).
- Every map-edge cell is ocean (§Q5).
- Every land cell drains to the sea via the tree (coupled erosion guarantees it; §Q4).
- A tile bundle depends on coarse data + seed only — never on any area's fine output (§Q7, §Q8): areas may then generate in any order, independently.
- Area-tier detail refines coarse features but never relocates them (§Q8).

## Outcomes & side effects

- Success: `continent/` grid (1 km relief, climate, drainage, density) + objects (plates' final geometry, ranges, major rivers, regions, seas — all named) + one input bundle per tile; batch proceeds to area generation.
- Failure: non-zero exit after reroll exhaustion or config refusal; nothing written beyond an inspectable partial directory.
