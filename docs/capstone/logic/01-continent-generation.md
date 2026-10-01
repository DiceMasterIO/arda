---
generated_date: 2026-09-23
scenario: continent-generation
generated_at_commit: 757b2ab5418b
absorbed_from: features/02-continent-climate-hydrology@2026-08-26, features/03-climate-driven-refinement@2026-08-27, features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-23-terrain-corrections@2026-09-23
---

# 01 — Continent generation

## Radial coast correction — implemented 2026-09-08

The first-look terrain correction traced a parallel river comb to the regional
coast mask. Its former `max(nx, ny)` radius created identical height rows on a
square flank. Use deterministic integer Euclidean distance in the normalized
coordinate frame for this radial mask. Keep the mask's core, edge, blending
weights and explicit ocean rim; verify existing land-fraction/river gates on the
retained MICRO and default seeds. This changes regional geography and requires
new overview checks; it is not a rendered river-path perturbation. Other broad
tectonic flanks can still support parallel drainage.

**Current implementation — 2026-09-23.**

**Boundary classification and belt-distance corrections — installed.**
Relative plate motion is classified by its component along the normal between
the moved plate centers, rather than a cardinal raster neighbor direction.
This is the normal of the underlying unwarped Voronoi pair; the existing spatial
warp remains an approximation and does not have its local derivative modeled.
Tangential motion has zero normal component and adds no collision, arc or rift
forcing. At junctions, retain every incident kind in an order-independent mask
so visiting neighbors in a different order cannot replace collision with rift.
Each belt uses distance to its own boundary kind. The September 23 correction
uses an exact separable squared Euclidean transform and integer-square-root Q10
distance through the cubic profile, replacing the former Chebyshev metric.
Amplitudes, belt widths, drift, diffusion and normalization remain unchanged. Source and red/green
evidence: `crates/arda-gen/src/continent/tectonics.rs` and
`features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/regional-relief-diagnostic/`.

The continent stage supplies the accepted coarse relief, climate and overview
rivers for a batch. Published area water is solved later over the complete
prepared 100 m domain. This section's Steps and Invariants describe current
behavior; the earlier design and observations below are retained history.
The September 23 correction passes local physical, deterministic and golden
checks; natural-landscape visual acceptance remains open. Historical verification
below retains its original scope.

## Trigger & preconditions

`arda generate`, `arda::generate`, or `generate_world_with_limits` receives a
u64 seed, validated configuration and absent or empty output directory. The
default is 500 × 1000 km, 35–55°N and 15 people/km². Supported requested axes
are 64–4000 km. The explicit-limit API uses the same physical model as the
default API. Source: [config.rs:83](../../../crates/arda-core/src/config.rs:83),
[orchestrator.rs:338](../../../crates/arda-gen/src/orchestrator.rs:338).

## Steps

1. **Admit the complete batch before creating output.** Derive the modeled
   domain and conservative stage reservations, including prepared files,
   routing/hierarchy/flow scratch, annual bands, saved records and area
   references. Reserve a fresh output transaction only after admission succeeds.
   An existing nonempty world is never overwritten. Source:
   [generation_limits.rs:156](../../../crates/arda-gen/src/orchestrator/generation_limits.rs:156),
   [publication.rs:58](../../../crates/arda-gen/src/orchestrator/publication.rs:58).
2. **Generate and validate the coarse continent.** Seeded plate/tectonic relief
   produces the 4 km simulation and 1 km working surface. The latter receives
   25 nine-point hillslope-creep passes over positive interior cells; the rim
   and nonpositive cells stay fixed. Stream-power incision occurs later in
   the existing 40-step shared 100 m evolution (`crates/arda-gen/src/continent/erode.rs:19`,
   `crates/arda-gen/src/area/evolution.rs:189`). The forced rim uses `h.min(-1)` to preserve
   already-negative seafloor depths (`crates/arda-gen/src/continent/mod.rs:209`). Climate, coarse hydrology and rivers are retained from the
   accepted attempt; the acceptance/retry branches are below. There is no implemented 1500 m range gate, human-geography
   stage or naming stage. Source:
   [continent/mod.rs:71](../../../crates/arda-gen/src/continent/mod.rs:71),
   [orchestrator.rs:366](../../../crates/arda-gen/src/orchestrator.rs:366).
3. **Classify coarse ocean and compute climate.** Ocean is the D8-connected
   component of nonpositive heights reached from the coarse outer rim; a closed
   negative depression is not an ocean moisture source. Mean annual temperature
   uses absolute latitude symmetrically between hemispheres, a 6.5°C/km lapse
   and the existing distance-to-sea continentality adjustment. Marine lapse
   elevation is zero; nonmarine elevation is signed, including negative land.
   Regime selection tests cold/boreal first, then tropical, Mediterranean and
   temperate. Rainfall remains deterministic integer moisture advection over
   coarse terrain. Source:
   [climate.rs:42](../../../crates/arda-gen/src/continent/climate.rs:42),
   [climate.rs:219](../../../crates/arda-gen/src/continent/climate.rs:219).
4. **Provide canonical preparation inputs.** Tile bundles remain pure functions
   of seed, accepted continent and absolute coordinates. They retain coarse
   entering-river and routing fields for the older local diagnostic path;
   those fields do not inject catchment or discharge into published shared
   water. Preparation uses canonical relief, rainfall and a lapse-removed
   annual temperature reference. Coarse samples outside its grid clamp at the
   rim. Source:
   [bundles.rs:149](../../../crates/arda-gen/src/continent/bundles.rs:149),
   [prepare.rs:14](../../../crates/arda-gen/src/area/prepare.rs:14),
   [temperature.rs:20](../../../crates/arda-gen/src/area/temperature.rs:20).
5. **Prepare the entire modeled rectangle, then solve shared water.** Each axis
   is `max(requested_km × 10, exported_area_count × 512)` fine cells. Full
   512² tiles are evaluated; only private persistence crops the final fringe.
   The default therefore models 5000 × 10000 cells in 200 prepared tiles while
   preserving 171 exported areas. MICRO preserves its existing 1024 × 2048
   exported cells even though its request is 102 × 204 km. Fringe cells can
   contribute upstream water and allow a course to leave and reenter exported
   areas; they do not create extra exported areas. Fine ocean, receivers,
   actual saddles, nested basins, annual support and final flows are solved
   before area composition, as specified in the current section of
   [area generation](02-area-generation.md). Source:
   [prepared_domain.rs:42](../../../crates/arda-gen/src/hydrology/prepared_domain.rs:42),
   [orchestrator.rs:406](../../../crates/arda-gen/src/orchestrator.rs:406).
6. **Complete the batch before publishing the continent/world.** The saved
   format-4 continent overview and river objects accompany composed areas and
   global hydrology records. Every required output must succeed and private
   readers/stores must close before the final manifest rename. A continent
   stage passing its gates alone does not make a loadable world. Source:
   [orchestrator.rs:462](../../../crates/arda-gen/src/orchestrator.rs:462),
   [publication.rs:118](../../../crates/arda-gen/src/orchestrator/publication.rs:118).

## Branches

A coarse candidate is accepted when 25–90% of its cells have positive elevation
and at least one extracted continent river reaches the sea. A failed candidate
uses the next deterministic attempt, up to five total. Accepted candidates
proceed to full-domain preparation, including hydrology-only fringe cells where
the requested extent exceeds exported areas; exported overshoot is retained.
Source: [orchestrator.rs:366](../../../crates/arda-gen/src/orchestrator.rs:366),
[prepared_domain.rs:42](../../../crates/arda-gen/src/hydrology/prepared_domain.rs:42).

## Unhappy paths

Config/resource rejection happens before output creation. Exhausted continent
attempts name the failed land or sea-river gate. Later terrain, topology,
forcing, arithmetic, storage and resource failures propagate as typed errors;
partial output has no final manifest and cannot be loaded. Interruptions
require a clean rerun: private scratch is not an automatic resume contract.
Publication provides process-interruption completion semantics, not a promise
of power-loss durability.

Default capacities are 100,000 closed leaves, 6,000,000 annual support bands,
4,000,000 feature records, 16,000,000 area references, 4,000,000 outward lake
edges, 16 GiB owned-payload RAM, and 256 GiB each for spatial and combined
scratch. Explicit logical-work, requested-I/O-byte and file-operation ceilings
also apply. Owned-payload admission is not a process-RSS guarantee. Supporting
a 4000 km axis does not guarantee that every possible terrain fits the default
counts. Source:
[types.rs:84](../../../crates/arda-gen/src/hydrology/types.rs:84),
[generation_limits.rs:147](../../../crates/arda-gen/src/orchestrator/generation_limits.rs:147).

## State transitions

Validated request → admitted output transaction → coarse candidate → accepted
coarse relief/climate/rivers → complete prepared fine domain → shared solution →
final area/global/continent layers → published manifest. A rejected candidate
returns to the coarse-candidate state within the attempt cap; a terminal failure
leaves the transaction unpublished. Source:
[orchestrator.rs:344](../../../crates/arda-gen/src/orchestrator.rs:344),
[publication.rs:122](../../../crates/arda-gen/src/orchestrator/publication.rs:122).

## Invariants

- Identical seed/configuration and accepted inputs give identical generation,
  including the retry sequence. Resource capacities may accept or refuse the
  work; they never alter a supported physical result.
- Coarse overview drainage and final 100 m water are distinct authorities. The
  coarse filled routing surface is not a physical fine lake surface. Closed
  fine basins may retain water or remain dry; published land is not required
  to drain directly to sea.
- Shared fine topology is available before composition. Exact crossings and
  leave/reenter accounting require no reads of neighboring **final** area
  files. The older claim that this would necessarily violate independence is
  superseded.
- A canonical tile preparation is independent of other prepared tiles. Final
  area water also depends on the completed shared solution, not its bundle
  alone. The current public batch prepares and composes sequentially.
- No synthesized incoming river is counted a second time at an area boundary.
  Actual adjacent fine cells and their global identities define crossings.

## Outcomes & side effects

The accepted coarse grid, climate, overview rivers and pure tile bundles feed the
remaining batch. They become a loadable world only through Step 6's complete
publication. The output directory and temporary scratch belong to that batch;
there is no neighboring-final-area read or separately published coarse-only
checkpoint. Source:
[orchestrator.rs:398](../../../crates/arda-gen/src/orchestrator.rs:398).

## Dimensions not in play

Human geography, settlement density placement, trunk corridors, region/river
naming and their richer object outputs remain deferred prescriptions. Current
manifest settlement and named-river counts remain zero; the configured mean
density is not evidence that those systems have run. Source:
[orchestrator.rs:518](../../../crates/arda-gen/src/orchestrator.rs:518).

## Retained earlier design and dated observations

> History only: the prescriptions and measurements below describe earlier
> designs or their stated observation dates. References to “Steps”, “Invariants”,
> implemented behavior, unbuilt work or format versions are local to that history.
> They do not override the current implementation above. Social, vegetation,
> road and naming prescriptions that remain unbuilt are retained as deferred work.

First stage of the batch (`mockup/01-generate.md` step [1/3]). Produces
the continent grid, continent objects, and every area tile's input
bundle. The Steps and Invariants sections retain the chosen design; dated observations record later implementation history. Named constants are tunable defaults. Current code behavior is mapped in `../04-data-flow.md`.

### Trigger & preconditions

- Trigger: `arda generate` batch start; also callable via the crate .
- Preconditions: validated config (continent size, default 500×1000 km; latitude band, default 35–55°N; mean density, default 15 people/km²) and a u64 seed. Nothing else — no prior state exists.

### Steps

1. **Plate seeding**: on a domain ~2× the visible continent, seed 8–14 plates as Voronoi regions from the seed. Each plate gets a crust type (continental/oceanic) and a drift vector. The domain rim is forced oceanic so map edges are guaranteed ocean.
2. **Time-stepped tectonics, coupled**: 4 km sim grid (125×250 cells), ~100 steps of ~1 Myr. Per step: plates move along drift vectors (vectors re-roll slightly every ~20 steps); boundary uplift applies by relative motion — cont-cont convergent → range, ocean-cont → subduction arc + volcanoes, divergent → rift, transform → fault zone; then coarse stream-power erosion and drainage respond, so rivers and valleys co-evolve with the ranges — the artifact's causality rule at continental scale.
3. **Coast**: base elevation from crust type (isostasy), modified by accumulated tectonics + erosion; a fixed sea level cuts the coastline. Shelf seas, bays, capes are emergent.
4. **Upsample**: final relief interpolated 4 km → 1 km working grid (noise-refined, seed-derived).
5. **Climate**: prevailing wind from the latitude belt (westerlies in the default band); moisture-advection pass over the 1 km relief (same rules as the area tier's rain — release on climb, recharge over water) → rainfall + rain shadows; temperature = latitude + 6.5 °C/km lapse + continentality (distance to sea). Every 1 km cell gets a climate regime (mediterranean/temperate/boreal/tropical by band position and elevation).
6. **Hydrology**: walk the final drainage tree; watercourses with catchment ≥ ~3,000 km² (assumed default) become continent river objects with course, catchment, discharge.
7. **Human geography**: habitability score per 1 km cell (climate, relief, coast, river valleys) → settlement-density map scaled to the configured mean; trunk-corridor network = least-cost paths between high-density basins over the 1 km relief (cost rules as the artifact's roads, coarse).
8. **Naming**: continent, regions/provinces (density basins bounded by ranges/rivers/seas), mountain ranges, major rivers (one name for the whole course), seas/bays — site-derived naming per the artifact's settlement-name philosophy.
9. **Validation**: land fraction within 25–90%, ≥1 range above 1,500 m, ≥1 major river reaching the sea (all tunable).
10. **Tile bundles**: for each 51.2 km tile, emit its input bundle — edge heights, entering rivers (edge position, catchment, discharge), climate regime + wind + edge moisture, settlement density, road-exit points where trunk corridors cross tile edges. Bundles are computed from coarse data + seed only, so adjacent tiles are mirror-consistent by construction.

### Observed — continent rebuild (2026-08-26)

Built shape of steps 1-4. Steps 5-9 (climate, hydrology objects, human
geography, naming, validation stats) remain unbuilt.

1. **Plate seeding.** Count scales with domain area (~1 per 2,800 sim cells), so boundary density — and with it the spacing of mountain belts — is the same at any continent size; a fixed 8-14 made the default world a flat plain while a quarter-size one came out mountainous. Crust follows position rather than chance: continental in the core, oceanic beyond 0.8 of the half-width, mixed between. Leaving it to chance meant the central plates were sometimes all oceanic and the continent failed to exist.
2. **Boundary lookup is domain-warped.** Raw Voronoi gives straight-line plate boundaries, and since the coast follows them the continent came out a polygon.
3. **Tectonics raises belts, not lines.** Uplift spreads across an orogenic belt either side of each boundary — 9 cells for collisions, 6 for arcs — with a cubic falloff. A one-cell ridge plus heavy diffusion produced a 118 m plateau across a UK-sized landmass. Plates drift each step (`centre + drift * step`), so belts migrate and the continent becomes a collage of orogens rather than one range. Uplift is normalised so its 99.5th percentile hits a target relief, making hypsometry independent of continent size, and subsidence is floored so a rift cannot drown the interior.
4. **Coast.** Crust is blurred into a **shelf gradient** so the continent meets the ocean over tens of kilometres instead of one 4 km cell, then blended with a centred mask. Sea level is crossed at mid-continentality, not at 0.87 as a straight interpolation gives. Noise is applied on **both** sides of sea level; restricting it to land left the sea floor as pure bilinear interpolation and the coastline followed the 4 km grid as rectangular steps.
5. **Coarse erosion runs here** (step 2's "erosion and drainage respond"), globally on the 1 km grid: priority-flood, D8 by steepest descent, stream-power incision clamped so a channel never cuts below what it drains into, and isotropic hillslope creep. Running it globally rather than per-tile is what lets valleys cross tile boundaries and leaves no pinned-rim seams.

Measured against Earth (seeds 42 and 7, default size): median land elevation 303-337 m against 330 m; land above 500 m 37-41% against 33%; lake area 0.5-1.1% against ~1%.

**Known limitation.** Terrain is built from value noise on a square lattice, which is anisotropic — its gradients favour the lattice axes, so steepest descent does too and only ~17% of flow directions are diagonal against an isotropic 50%. Isotropic gradient noise is the fix; one attempt produced blocky coastlines and was reverted.

### Observed — climate and hydrology (2026-08-26, feature 02)

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
   `max(300, land_km2/30)` km² — **amends the assumed fixed
   ~3,000 km²**, which this rule reproduces at default-world land
   areas while keeping MICRO fixtures populated. Mouths are traced in
   row-major order; the main stem follows the largest-catchment inflow
   (**shipped deviation from feature 02 spec R4:** ties resolve to the
   earliest row-major child, not the fixed neighbour order); ids are
   1-based in creation order so `feeds` is acyclic by construction;
   courses run source → mouth. No endorheic basins exist by
   construction (the rim is ocean and the routing surface is filled
   from it). Sea cells store zero catchment and discharge.

**§Q6 subtropical highs (recipe 7, 2026-10-01).** The advected rainfall
above has no latitude term, so a continent at 20° was as wet as one at
45°: MICRO seeds at 35–55° get 1,050–1,400 mm everywhere and no basin is
ever arid. Recipe-7 worlds scale each row's rainfall by the subsiding
branch of the Hadley cell (`continent::aridity::subtropical_permille`):
100% poleward of 38°, falling linearly to 30% at 26°, 30% from 26° to
15°, rising to 120% at 5° and the equator (the equatorial trough).
Earth's great deserts lie in that 15–30° belt. The default 35–55° band
loses at most 12% at its southern edge; `--latitude 15,35` puts a MICRO
continent's southern half in the dry belt (seed 74: land mean 382 mm, against 1,297 mm unscaled). Earlier recipes keep the
unscaled field. Formation reads the same field (logic/02 §fine-formation
climate runoff).

Persistence: `continent/overview.bin` is real — 18-byte little-endian
records (height, temperature, rainfall, regime, downstream 0–7 with
255 = none, catchment, discharge) behind `ARDAOVR\0` + dims;
`continent/objects.bin` carries the rivers section behind `ARDACOB\0`.
`FORMAT_VERSION` is 3; loaders refuse any other major with the
regenerate remedy. Five structural invariants and a 60 s default-size
stage bench gate the tier; climate realism is measured, never gated
(spike report; Horton ratios were thin at default size — only seed 1
yielded a measurable N1/N2 = 3.0, an R8 observation for step 12).

### Branches

- Boundary type (step 2) is decided solely by the two plates' crust types and relative motion.
- Validation failure (step 9) → re-run steps 1–8 with derived subseed `(seed, attempt)`; attempt < 5 else hard error.

### Unhappy paths

- Degenerate continent: caught by step 9; ≤5 deterministic rerolls, then exit non-zero naming the failed check.
- Invalid config (size/latitude/density out of range): refused before step 1 (mockup 01 States).
- Interrupt mid-run: no partial continent is consumable; re-run restarts identically (determinism).

### State transitions

None — this scenario has no persistent entities before it runs; it creates the world's first state (`continent/` layer + tile bundles, `mockup/02-world-layout.md`).

### Invariants

- Same (seed, config) → identical continent, including reroll sequence.
- Every map-edge cell is ocean.
- Every land cell drains to the sea via the tree (coupled erosion guarantees it;).
- A tile bundle depends on coarse data + seed only — never on any area's fine output: areas may then generate in any order, independently.
- Area-tier detail refines coarse features but never relocates them.

### Outcomes & side effects

- Success: `continent/` grid (1 km relief, climate, drainage, density) + objects (plates' final geometry, ranges, major rivers, regions, seas — all named) + one input bundle per tile; batch proceeds to area generation.
- Failure: non-zero exit after reroll exhaustion or config refusal; nothing written beyond an inspectable partial directory.

### Observed — step 10 built, step 9 partially gated (2026-08-27, feature 03)

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

### Dimensions not in play

The retained design did not record a dimension-by-dimension exclusion list. This provenance repair leaves those exclusions unspecified rather than inventing decisions.
