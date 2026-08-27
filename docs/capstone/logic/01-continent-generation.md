---
generated_date: 2026-08-27
scenario: continent-generation
traces: [Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9]
generated_at_commit: 8be0a0a
absorbed_from: features/02-continent-climate-hydrology@2026-08-26, features/03-climate-driven-refinement@2026-08-27
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

## Observed — climate and hydrology (2026-08-26, feature 02)

Built shape of steps 5–6 plus persistence (`continent/climate.rs`,
`continent/hydrology.rs`, `arda-core::formats::overview`). Steps 7–9
(human geography, naming, validation additions) remain unbuilt; step
10's entering rivers still have no producer — feature 03.

5. **Climate** runs on the 1 km grid, all integer. Temperature:
   sea-level mean linear across the configured band anchored at 18 °C /
   35°N → 6 °C / 55°N (extrapolated outside), −6.5 °C/km lapse,
   −1 centi-°C per km distance-to-sea capped at 300 (4-connected BFS);
   **shipped deviation:** the lapse floors elevation at 0, so sea cells
   store latitude-and-continentality-only temperature. Regime by band
   position + elevation, in order: tropical < 23.5°; mediterranean
   < 42° and < 1,000 m; boreal where T < 3 °C; else temperate.
   Rainfall: iterative advection-diffusion, uniform due-west wind,
   fixed 1.5 × width passes, 12.5% lateral diffusion (**shipped
   deviation:** computed as `a − a/8 + mean4/8`, ≤1 unit above the
   literal `(7a+mean4)/8` per pass), recharge 1/8-of-deficit over
   water, land release 1/512 base + 1/64 per 100 m climb clamped at
   1/16 — **shipped deviation:** climb measures against
   `max(upwind height, 0)` so a deep offshore shelf is not a cliff.
   Conversion is pass-count-normalised (`(R/passes) × C_NORM >> 4`,
   C_NORM = 153), calibrated to a 799 mm/yr land mean on seed 42 at
   default size (spike report, feature 02 R8).
6. **Hydrology**: one final priority-flood + D8-steepest-descent route
   on the post-erosion surface (reusing step 2's `fill`/`accumulate` —
   climate never feeds back into erosion), accumulating catchment
   (1 cell = 1 km²) and discharge = Σ(upstream land rainfall) × 125 /
   7,884 L/s (the artifact's ~0.5 runoff over 1 km², exactly
   500,000/31,536,000). River objects form above
   `max(300, land_km2/30)` km² — **amends §Q7's assumed fixed
   ~3,000 km²**, which this rule reproduces at default-world land
   areas while keeping MICRO fixtures populated. Mouths are traced in
   row-major order; the main stem follows the largest-catchment inflow
   (**shipped deviation from feature 02 spec R4:** ties resolve to the
   earliest row-major child, not the fixed neighbour order); ids are
   1-based in creation order so `feeds` is acyclic by construction;
   courses run source → mouth. No endorheic basins exist by
   construction (the rim is ocean and the routing surface is filled
   from it). Sea cells store zero catchment and discharge.

Persistence: `continent/overview.bin` is real — 18-byte little-endian
records (height, temperature, rainfall, regime, downstream 0–7 with
255 = none, catchment, discharge) behind `ARDAOVR\0` + dims;
`continent/objects.bin` carries the rivers section behind `ARDACOB\0`.
`FORMAT_VERSION` is 3; loaders refuse any other major with the
regenerate remedy. Five structural invariants and a 60 s default-size
stage bench gate the tier; climate realism is measured, never gated
(spike report; Horton ratios were thin at default size — only seed 1
yielded a measurable N1/N2 = 3.0, an R8 observation for step 12).

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

## Observed — step 10 built, step 9 partially gated (2026-08-27, feature 03)

Step 10's tile bundles now carry their full payload, and step 9 gained the
river half of its gate. Steps 7 (human geography) and 8 (naming) remain
unbuilt, so `named_river_count` stays 0.

10. **Tile bundles.** Beyond edge heights, a bundle carries: **entering rivers**, 53×53 1 km patches of rainfall, regime and the continent routing surface, a prevailing-wind octant, and the west-edge moisture column. All are pure functions of continent data and the seed, so adjacent tiles stay mirror-consistent and areas still generate in any order.
    - **Entering rivers** are continent D8 edges crossing a tile's exact 100 m boundary line inward, with upstream catchment ≥ 3 km² (the artifact's channel scale). Because tiles are 51.2 km, 1 km cells straddle tile lines, so crossings are judged against the boundary line rather than km-cell footprints; cell centres sit at `k·10+5` and lines at multiples of 512, so sidedness is total and each edge belongs to exactly one tile — the downstream one. A corner crossing resolves to the north/south line. The seed is the lowest-relief cell of the crossing's entry window; a window that is entirely sea drops the crossing (the river already reached the sea). Crossings sharing a seed cell merge: catchment and discharge sum, **order takes the maximum of the contributors' orders — deliberately not the order of the summed catchment**, since the map is not additive.
    - **Order continuity** uses `g(c) = 1 + ilog2(c/3)/2` clamped to 1..=12. Both sides of a seam compute `g` from the same catchment, so an exit order and the neighbour's entry order agree by construction, without either tile reading the other's output.
9. **Validation** now also requires ≥1 continent river reaching the sea (`feeds: None`), checked inside the existing ≤5 reroll ladder and after the cheaper land-fraction gate. The river count is recorded in the manifest.

**Recorded approximations.** A river that loops out of and back into the same
tile double-counts its own upstream (rare at 51 km tiles). The coarse tree and
the fine area routing agree about *where* water crosses a seam 53–94% of the
time depending on catchment size — large crossings align, small tributaries
jog by a median of 8 cells (~800 m, sub-pixel at map scale). That is the
inherent 1 km-vs-100 m tier gap, not a defect: closing it exactly would
require reading the neighbour's fine output, which the order-independence
invariant forbids.
