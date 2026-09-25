---
generated_date: 2026-09-23
scenario: area-generation
artifact: ../mockup-artifact.md
generated_at_commit: 4ea2271cd809
absorbed_from: features/03-climate-driven-refinement@2026-08-27, features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-23-terrain-corrections@2026-09-23
---

# 02 — Area generation

## Shared terrain correction — authorized 2026-09-08, integration verification in progress

The user requested corrections after the first-look review established square
boundary troughs, repetitive drainage and numerous shallow physical ponds. This
correction supersedes the publication-area pin/taper prescription below; shared
coordinate identity, deterministic generation and water continuity remain required.

- Evolve the modeled fine terrain as one rectangle before slicing prepared areas.
  Publication-area boundaries do not constrain uplift, incision, creep or collapse.
  Only the actual modeled outer rim remains fixed. One regional maximum normalizes
  uplift across the domain; a per-area maximum must not change physical rates.
- Preserve physical depressions during incision, sea/land sign and integer water
  accounting. Routing epsilon is temporary and never becomes a physical bed.
- Apply uplift/creep to a provisional bed, then solve linear stream-power incision
  downstream first using each receiver's updated bed and the full fractional slope.
  A cap against the receiver's previous height can strand an upstream cell when
  that receiver rises in the same step. Use the implicit linear solution with the
  existing area exponent/coefficient and integer rounding that cannot invert a
  positive link. This prevents incision-created numerical pits; it does not erase
  pre-existing physical basins or promise a calibrated natural lake count. The
  numerical approach is consistent with the [Fastscapelib stream-power scheme](https://fastscapelib.readthedocs.io/en/latest/api_cpp/eroder.html).
- Admit every whole-domain evolution allocation before generation creates output;
  resource refusal is explicit rather than silently subdividing the physical model.
  Include a conservative declared work allowance for shared sampling, both floods,
  MFD, uplift/creep, implicit incision and the bounded collapse scans, including
  logarithmic heap comparisons. Work units are bounded transitions/comparisons,
  not individual CPU instructions or a runtime guarantee.
- Test the finest area-detail octave and differently oriented noise bases against
  retained terrain. Choose a supported terrain correction rather than suppressing
  lake/channel records by size or display threshold. Water forcing is unchanged in
  this first terrain comparison.
- Verify an internal former area boundary, a crossing catchment and a genuine bowl;
  then check a MICRO world and the reported default-world examples. Numerical tests
  and a visual comparison are separate acceptance evidence.

The retained feature folder records the baseline, correction plan and measurements.
This section records the correction rule; completion is recorded only after verification.

**Current implementation — 2026-09-08.**

### Regional detail correction

Fine relief must follow local topographic variation rather than absolute altitude.
Let C(x,y) be the existing bounded regional interpolation. Sample its eight D8
neighbors at offsets of ±10 fine cells (1 km cardinal spacing), and let R be the
maximum absolute difference from C(x,y), computed in i64. Land detail amplitude
is min(R, 90,000 mm); sea amplitude remains 1,500 mm. Apply the existing four-octave
noise and final regional land/sea sign clamp. This is a local-relief envelope,
not a slope estimate: diagonal samples are not distance-normalized.

Raising an all-land regional shape must not increase its added roughness. A flat
regional surface adds no artificial hollows. Evaluating the same continuous
surface at shifted absolute coordinates keeps the envelope continuous across
1 km interpolation lines and 512-cell publication cuts, apart from existing
millimetre quantization. Public coordinate offsets saturate at i32 limits.
No lake count, minimum depth, regional quota, water forcing or display threshold
is changed by this rule. Substantial regional bowls remain part of the terrain.

Shared preparation first fills its existing regional/uplift array, then samples
detail from that immutable cache. Samples beyond the true modeled rectangle use
the canonical regional interpolator, not a clamped cache edge. No extra dense
array is allocated. Radius-ten D8 fallbacks number 60(W+H)−400 for W,H≥10,
bounded by 0.1875N for admitted axes W,H≥640. Eight cache reads per cell and
fallback interpolation add under 16N bounded visits/evaluations. Public bundle
sampling adds under 49N using the retained bound on tiles and entering-river
windows. That local-detail correction raised the initial work allowance from
256N to 512N at the time. The current structural-relief addition raises it to 640N;
the shared duration is now 160 steps. The 53N dense owned-memory accounting
remains unchanged because the added gate reuses the coarse/uplift array.
This admission convention counts bounded source visits/cubic evaluations, not
individual machine instructions. Sources: `continent/bundles.rs`,
`continent/area_detail.rs`, `area/prepare.rs`, `area/evolution.rs`.

### Structural relief and shared duration — installed 2026-09-23

After the sign-preserving four-octave detail sample, add an integer structural
delta to the initial physical bed. At absolute 100 m cell coordinates `(x,y)`,
`delta_mm = 23×value_noise(seed⊕broad_salt,x,y,80) +
10×value_noise(seed⊕detail_salt,x,y,40)`. The two lattice spacings are 8 and
4 km, not their dominant wavelengths; the raw magnitude is at most 1,081,344 mm.
The same fixed seed and absolute coordinates are used from any area. Source:
[structural_relief.rs:9](../../../crates/arda-gen/src/continent/structural_relief.rs:9),
[structural_relief.rs:22](../../../crates/arda-gen/src/continent/structural_relief.rs:22).

Compute a nearby coarse range across the center and eight samples at offsets
`(±50,0)`, `(0,±50)` and `(±35,±35)` fine cells, approximately 5 km away. Its
Q16 multiplier is `clamp((range_mm−400,000)×65,536/1,600,000,0,65,536)`.
Apply `delta_mm×gate/65,536` with integer truncation and clamp the final sum to
i32. When the previous initial height or the center coarse height is nonpositive,
leave that initial height unchanged. Positive land can cross zero; the later
shared physical ocean/annual-water solve classifies the resulting bed, with no
shoreline clamp. The unperturbed coarse field still drives uplift. Source:
[structural_relief.rs:30](../../../crates/arda-gen/src/continent/structural_relief.rs:30),
[structural_relief.rs:52](../../../crates/arda-gen/src/continent/structural_relief.rs:52),
[prepare.rs:39](../../../crates/arda-gen/src/area/prepare.rs:39).

`boundary_height` computes this initial sample directly. Shared preparation
reuses its existing dense coarse/uplift field for gate samples inside the modeled
rectangle; outside samples use the same direct coarse interpolator rather than
clamping to a tile or modeled edge. No extra dense field is allocated. Shared
evolution then runs 160 iterations; the separate tile-only diagnostic kernel
retains its 40-step constant. Declared terrain work is
`N×(640 + 160×(184 + 8×9 + 8×ceil(log2 N)))` bounded units, where N is the
modeled fine-cell count; the default whole-generation admission is
4,685,132,212,727 units under the unchanged 2^48 default cap. Terrain memory
remains 53N owned bytes plus headers. Sources:
[bundles.rs:30](../../../crates/arda-gen/src/continent/bundles.rs:30),
[prepare.rs:104](../../../crates/arda-gen/src/area/prepare.rs:104),
[evolution.rs:20](../../../crates/arda-gen/src/area/evolution.rs:20),
[evolution.rs:98](../../../crates/arda-gen/src/area/evolution.rs:98),
[admission receipt](../features/2026-09-23-terrain-corrections/evidence/integer-regional-relief-probe/candidate-160/admission-measurement.json).

**Legacy outside-neighbor correction — installed and verified in candidate05.**
The area-only water API must read the physical cell at the requested adjacent
absolute coordinate. Existing `TileBundle.north` and `.west` describe the area's
own first row/column; they remain unchanged for canonical seam/pinning callers.
Add separately sampled north/west outside strips at local `y=-1` / `x=-1` and
four outside corners at `(-1,-1)`, `(512,-1)`, `(-1,512)`, `(512,512)`. Existing
south/east samples already describe local `y=512` / `x=512` and are reused.
Sample the same canonical `boundary_height` used by the adjacent legacy tile's
pinned rim. Routing handles the complete D8 neighbor ring and refuses coordinates
beyond it; it must not substitute the source edge height for its neighbor.
Preserve existing entry fluxes, erosion, noise, surfaces and thresholds. The
shared world solver reads its full domain directly and does not consume these
legacy outside samples. Sources: `continent/bundles.rs`, `area/water.rs`, and
`features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/legacy-seam-c05-diagnosis.md`.

**Legacy outside-neighbor admission.** One sequential bundle adds two 512-i32
allocations, four inline i32 corners and two vector headers: 4,160 owned bytes
on the 64-bit target. These fit the existing 128 MiB preparation allowance;
the writer does not retain the bundle and the samples add no persisted bytes.
For modeled cells N and tiles T including fringe, supported axes W,H≥640 imply
T≤N/65,536. Charging 256 bounded transitions for each of the 1,028 new samples
costs at most 4.015625N, fitting the former initial 256N work allowance after the existing
shared sampling/initialization bound of at most 240N. The fixed-loop derivation
and unchanged-stage scope are retained in
`features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/legacy-neighbor-admission-review.md`.
These are logical transitions, not CPU instruction or elapsed-time estimates;
changing sequential preparation or the sampler requires revisiting the bounds.

Published areas come from **shared physical evolution and preparation → one shared annual
water solve → immutable area composition**. An exported area has 512 × 512
cells of 100 m; its final water cannot be generated independently from a tile
bundle. This section is the current behavior. Earlier prescriptions and dated
observations below remain historical/deferred, and do not override these Steps
or Invariants. Golden and physical-water checks for the installed correction
are recorded in [testing](../06-testing.md); reference-quality visual acceptance remains open.

## Trigger & preconditions

The batch has accepted its continent and admitted the complete domain and
resource capacities. All modeled fine cells, including private fringe cells,
must be prepared before the shared solve; all shared stages must finish before
final areas are composed. `area::generate_area` remains a typed local diagnostic
API, including its old coarse inflows and lake filters. The public world
orchestrator uses `SharedTerrain::build`, `prepare_area_terrain`, `shared_solve::solve` and
`area_output::compose` instead. Source:
[orchestrator.rs:406](../../../crates/arda-gen/src/orchestrator.rs:406),
[area/mod.rs:658](../../../crates/arda-gen/src/area/mod.rs:658).

Canonical preparation consumes coarse relief and bundle coordinates, rainfall and
temperature reference. Coarse entering rivers, filled routing and basin patches
do not seed shared physical evolution or published annual water. They remain
inputs to the tile-only local diagnostic API (`crates/arda-gen/src/area/prepare.rs:39`,
`crates/arda-gen/src/area/prepare.rs:115`, `crates/arda-gen/src/orchestrator.rs:412`,
`crates/arda-gen/src/area/water.rs:199`). The 1 km stage now retains hillslope
creep only; stream-power incision belongs to the existing shared 100 m evolution.

## Steps

1. **Define the physical domain and sample initial terrain.** Each modeled axis
   covers `max(requested_km × 10, exported_area_count × 512)` cells. Sample one
   complete fine rectangle, including the real right/bottom fringe; no padding
   outside that rectangle participates in evolution. The fringe participates in
   terrain, water and catchments without adding final areas. Canonical samples
   use absolute coordinates and bounded, affine-preserving coarse interpolation;
   four detail octaves with 40/20/10/5-cell periods preserve the coarse sign.
   Their amplitude follows the local regional relief rule above; height alone
   does not increase roughness or prescribe a lake district. The subsequent
   structural rule above may move initially positive land below zero.
   Neighbor bundle boundaries naming the same absolute sample agree, rather
   than forcing every pair of adjacent cell centers to have equal heights.
   Source: [prepared_domain.rs:42](../../../crates/arda-gen/src/hydrology/prepared_domain.rs:42),
   [bundles.rs:31](../../../crates/arda-gen/src/continent/bundles.rs:31),
   [terrain_interpolation.rs:38](../../../crates/arda-gen/src/terrain_interpolation.rs:38).
2. **Evolve the physical bed, then freeze it.** The 160-step shared erosion loop
   fixes only the true modeled outer rim. Uplift, catchments, creep and collapse
   cross every publication boundary without a taper. Uplift is driven by coarse
   relief, normalized once for the domain, with 900 mm peak per step. Incision uses
   100,000 mm cardinal and 141,400 mm diagonal horizontal distances, fractional
   MFD contributing area with exponent 1.1. Apply uplift/creep first, then solve
   linear stream power downstream first against the receiver's updated bed.
   Floor removed material, preventing rounding from inverting a positive slope.
   Exact physical spill elevation protects real depressions from incision;
   a numerical routing increment alone does not imply lake depth. Creep and
   collapse remain in the loop. The current erosion loop leaves initially
   nonpositive beds unchanged and keeps positive updates at least 1 mm; structural relief
   can create a nonpositive initial bed from shallow positive land before this
   loop. This is a terrain rule, not the later marine-classification test.
   Slice immutable physical heights into prepared records; only the valid extent of a partial tile is
   saved, and zero padding never enters the model. Drop the shared terrain buffer
   before the annual water solve. Save physical signed-mm
   height, canonical annual rainfall and the unclamped lapse-removed temperature
   reference. No filled routing elevations are written as physical terrain.
   Source: [evolution.rs](../../../crates/arda-gen/src/area/evolution.rs),
   [mfd.rs:1](../../../crates/arda-gen/src/area/mfd.rs:1),
   [prepare.rs:44](../../../crates/arda-gen/src/area/prepare.rs:44).
3. **Resolve one fine marine mask and physical routing authority.** Flood D8
   connectivity through nonpositive cells from the modeled outer rim. A
   disconnected negative bed remains nonmarine. Physical receivers descend in
   bed height or, on equal-height plateaus, decrease a separate flat-distance
   rank. Resolve an immutable original terminal owner for every cell. Actual
   D8 owner-pair saddle witnesses and actual outer-rim exits form the global
   minimum-spanning tree and nested basin hierarchy; zero-height join intervals
   do not invent storage. Neither area rims nor corners close an otherwise
   connected basin. Source:
   [ocean.rs:96](../../../crates/arda-gen/src/hydrology/ocean.rs:96),
   [routing.rs:93](../../../crates/arda-gen/src/hydrology/routing.rs:93),
   [routing.rs:486](../../../crates/arda-gen/src/hydrology/routing.rs:486),
   [shared_solve.rs:428](../../../crates/arda-gen/src/orchestrator/shared_solve.rs:428).
4. **Select a representative annual water extent.** Finalize temperature from
   the prepared reference using signed physical elevation on nonmarine cells
   and sea-level lapse on marine cells. Let `P` be a cell's annual precipitation
   and `E` the sum of twelve climatological monthly Hamon evaporation depths,
   both converted to whole litres for the 100 m square. Baseline land loss is
   `A = min(P/2, E)`, dry runoff is `R = P − A`, and the extra cost of maintaining
   a fully wet cell is `D = E − A ≥ 0`. Group exact bed/owner bands on the
   witnessed hierarchy and install each cell's source exactly once. Start from
   empty and pay ascending support costs, with deterministic spill/merge events.
   A wet cell contributes `P − E` instead of `P − A`; direct lake precipitation
   replaces land runoff and is never added on top of it. This is an annual
   support-cost model, not elapsed reservoir filling, seasonal storage,
   groundwater exchange or a snow-state simulation. Geometric volume follows
   from physical bed and representative surface separately. Source:
   [temperature.rs:51](../../../crates/arda-gen/src/area/temperature.rs:51),
   [annual_aggregation.rs:51](../../../crates/arda-gen/src/hydrology/annual_aggregation.rs:51),
   [annual.rs:537](../../../crates/arda-gen/src/hydrology/annual.rs:537).
5. **Apply explicit flat-band and integer-surface rules.** Select the dry,
   partially supported, fully paid or spill case using the Branches rules below.
6. **Derive actual flows and channel metrics.** Normalize final signed leaf
   balances across actual witnessed edges; historical filling pours are not
   final discharge. The fine receiver forest plus selected spill edges carries
   whole-litre annual balances, omitting invisible edges internal to one wet
   lake. A spill can create real dry divergence; sea/domain export uses an
   actual cell or an explicit outside account, never a fabricated coordinate.
   Annual mean discharge is annual litres divided by 31,536,000 seconds.
   Actual edge channels begin at 40 L/s. Connected lakes are collapsed for
   drainage and Strahler propagation. A dry cell stores maximum outgoing flow,
   or incoming plus local runoff at an absorbing terminus; that combined flow
   can reach the threshold even when individual incoming edges do not. Point
   records cover channel termini without inventing a step. The downstream channel reference
   for height-above-river follows the shortest directed route, stops at standing
   water/exterior, and resolves ties canonically. Source:
   [annual_transfers.rs:108](../../../crates/arda-gen/src/hydrology/annual_transfers.rs:108),
   [fine_flow.rs:439](../../../crates/arda-gen/src/hydrology/fine_flow.rs:439),
   [flow_metrics.rs:564](../../../crates/arda-gen/src/hydrology/flow_metrics.rs:564),
   [extraction.rs:178](../../../crates/arda-gen/src/hydrology/extraction.rs:178).
7. **Compose immutable final areas and explain cross-area water.** Global
   Basin/Reach/Catchment/Junction identities and actual adjacent-cell crossing
   records survive local fragmentation. Index channel widths and endpoint joins
   into each area's required geometry halo, including centerlines outside the
   area whose rendered width reaches it. Composition reads prepared cells and
   solved shared records, never a neighboring final area file. `Sea`, strictly
   positive-depth `Lake`, or `Land` classification determines final fields.
   Physical height stays the bed; final temperature is persisted. Saved nonland
   rainfall and dry river fields are zero, while the full precipitation used
   by annual accounting remains represented in the shared ledger. Current cover
   is bare on water and grass/marsh on land; richer ecology/social fields remain
   deferred. Source:
   [final_index.rs:137](../../../crates/arda-gen/src/hydrology/final_index.rs:137),
   [hydrology.rs:238](../../../crates/arda-core/src/hydrology.rs:238),
   [area_output.rs:289](../../../crates/arda-gen/src/hydrology/area_output.rs:289),
   [shared_compose.rs:125](../../../crates/arda-gen/src/area/shared_compose.rs:125).

## Branches

If supply pays only part of a same-bed band's cost, stop the surface at that bed. Only cells
strictly below it are geometrically wet; the consumed remainder is a
separate annual marginal/shallow-water evaporation account bounded by that
band's full additional cost. No fractional lake cells or exposure state are saved.
Paying a complete band selects `bed + 1 mm` as its lowest positive-depth
representative; it advances further only when remaining support and the
next event permit. A zero-supply basin is dry; positive supply with zero
additional loss can reach its spill. A joining band with zero positive
depth does not fuse disconnected children into one actual lake identity.
There is no public 300-cell/4 m lake filter. Exact marginal shares are
assigned deterministically and debited once at immutable leaf terminals.
Source: [annual.rs:578](../../../crates/arda-gen/src/hydrology/annual.rs:578),
[annual.rs:714](../../../crates/arda-gen/src/hydrology/annual.rs:714),
[fine_flow.rs:511](../../../crates/arda-gen/src/hydrology/fine_flow.rs:511).

## Unhappy paths

On topology, forcing, arithmetic, resource or I/O error, the batch fails with the original typed
cause and leaves an unloadable partial directory. A clean rerun is required;
panic-only failure and independent tile resume are not the public contract.
Source: [orchestrator.rs:449](../../../crates/arda-gen/src/orchestrator.rs:449),
[publication.rs:118](../../../crates/arda-gen/src/orchestrator/publication.rs:118).

The same whole-batch limits described in the current continent chapter apply:
16 GiB owned payload, 256 GiB spatial/combined scratch, explicit leaf/band/record/
reference counts, and metered work/I/O. They are capacity ceilings, not a promise
that arbitrary maximum-size terrain completes under default settings.

## State transitions

Preparation creates private indexed physical tiles. The shared solve creates
private routing/flow/hierarchy state; final composition writes format-4 area
cells/objects with copies of relevant authoritative global water records.
Complete area/global/continent outputs and required flushes precede closing
scratch handles and publishing the manifest last. Current batch execution is
sequential for preparation and area composition.
Source: [orchestrator.rs:449](../../../crates/arda-gen/src/orchestrator.rs:449),
[publication.rs:118](../../../crates/arda-gen/src/orchestrator/publication.rs:118).

## Invariants

- Physical bed, temporary erosion fill, global plateau rank and representative
  water surface have different meanings. Ranking epsilon never creates depth,
  a basin or a persisted water level.
- Original catchment ownership is immutable even when lakes merge or final
  signed flows diverge. Fine-area crossings and leave/reenter paths consume
  shared source and flux accounts once, without injected coarse duplicates.
- All positive-depth fragments of one connected lake share its global identity
  and surface. Computational equal-height joins alone are not wet connectivity.
- The exact whole-domain ledger is dry-land precipitation plus lake
  precipitation = dry-land loss plus lake evaporation plus marginal evaporation
  plus sea export plus domain export. Internal transfers cancel before totals.
- The 1 mm representative choice is explicit raster quantization, not an
  annual storage state or a hidden minimum-depth realism filter. No universal
  Horton ratio or 50% diagonal-direction target is asserted for every area.
- Limits refuse unsupported work with typed errors; they do not trim lakes,
  change coefficients, discard crossings or publish incomplete geometry.

## Outcomes & side effects

A completed area retains physical bed and finalized climate fields, strict
Land/Sea/Lake classification, river metrics, local objects and copies of relevant
global hydrology records. Step 7's immutable composition supplies these fields;
shared IDs/crossings and exact source/flux accounts remain authoritative beyond
the local tile. Publication is a whole-world outcome, not an independent area
resume point. Source:
[shared_compose.rs:125](../../../crates/arda-gen/src/area/shared_compose.rs:125),
[area_output.rs:289](../../../crates/arda-gen/src/hydrology/area_output.rs:289).

## Dimensions not in play

Treeline/vegetation ecology, settlement placement and density, population/rank
sizes, fields/land use, named settlements and regions, roads, bridges/fords/
ferries and trunk-corridor prescriptions below remain deferred. Current grass/
marsh classification is not that vegetation model. Tactical output retains
sampled land blocks at stride 64 in both axes and the current 24-tile fill;
it is not full tactical coverage or a completed asset expansion. Source:
[shared_compose.rs:175](../../../crates/arda-gen/src/area/shared_compose.rs:175),
[orchestrator.rs:300](../../../crates/arda-gen/src/orchestrator.rs:300).

## Retained earlier design and dated observations

> History only: the prescriptions and measurements below describe earlier
> designs or their stated observation dates. References to “Steps”, “Invariants”,
> implemented behavior, unbuilt work or format versions are local to that history.
> They do not override the current implementation above. Social, vegetation,
> road and naming prescriptions that remain unbuilt are retained as deferred work.

Second batch stage, run once per 51.2 km tile (512×512 cells of 100 m).
The normative rules are the artifact's own sections — `Relief`, `Water`,
`Climate`, `Vegetation and ground cover`, `Where people settle`,
`Land use around settlements`, `Roads`, `What the finished map knows`,
`How plausibility is checked` in `../mockup-artifact.md` — confirmed
verbatim. This file records only what the continent
tier changes and the scenario frame; it does not restate the artifact.

### Trigger & preconditions

- Trigger: batch step [2/3], after continent validation passes; one run per tile, any order (tiles are independent — `01-continent-generation.md` invariants).
- Preconditions: the tile's input bundle exists (edge heights, entering rivers with catchment/discharge, climate regime, wind, edge moisture, settlement density, road-exit points).

### Steps

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

### Branches

- Per the artifact: ground-cover override order; settlement tier placement; crossing classification (bridge/ford/ferry); lake pass-through.
- Trunk roads must reach the bundle's road-exit points (replaces "edges the region's roads leave through") and connect to corridor crossings.

### Unhappy paths

- A tile with no valid settlement site (all-mountain/all-marsh): allowed — settlement count follows density × habitable land; zero settlements ⇒ no fields, roads = through-corridors only (artifact's rules degrade gracefully; assumed reading).
- All-ocean tiles: skip stages after relief/water; cells are sea, no objects (assumed).
- Interrupt: re-run of a tile is deterministic and independent; restart, no resume.

### State transitions

None — creates `areas/<ax>_<ay>/` (cells + objects, `mockup/02-world-layout.md`) from the bundle; never mutates continent state or other tiles.

### Invariants

- Determinism per (seed, tile coords, bundle) — and the bundle itself is seed-derived.
- Edge agreement with all four neighbors, by construction.
- The artifact's validation statistics hold per tile: Horton ratio 3–5, Hack exponent ~0.55, settlement rank-size, road sinuosity 1.2–1.4, ~1 ha farmland per inhabitant.
- No settlement on floodplain (<1.5 m above watercourse); no road across lake or sea.

### Outcomes & side effects

- Success: `areas/<ax>_<ay>/cells.bin` (every per-cell fact in the artifact's "What the finished map knows") + `objects.bin` (river segments, lakes, named settlements with tier/population/site-tags, roads, crossings, passes). Settlement names use the artifact's site-suffix scheme; region/river names from the continent are honored where objects continue across tiles.
- Failure: none defined beyond process death — inputs were validated upstream; a panic on one tile halts the batch naming the tile (assumed).

### Observed — climate-driven water stage (2026-08-27, feature 03)

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

### Dimensions not in play

The retained design did not record a dimension-by-dimension exclusion list. This provenance repair leaves those exclusions unspecified rather than inventing decisions.
