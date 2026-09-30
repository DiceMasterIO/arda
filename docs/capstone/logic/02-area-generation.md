---
generated_date: 2026-09-23
scenario: area-generation
artifact: ../mockup-artifact.md
generated_at_commit: 4ea2271cd809
absorbed_from: features/03-climate-driven-refinement@2026-08-27, features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-23-terrain-corrections@2026-09-23
---

# 02 — Area generation

> Evidence consolidated on September 25: the [study](../features/2026-09-23-terrain-corrections/STUDY.md) and [experiment catalogue](../features/2026-09-23-terrain-corrections/EXPERIMENTS.md) replace the raw terrain-corrections archive. Historical filenames below identify removed experiments; current evidence links lead to their retained summaries. Original/final worlds and the 32K output remain under `out/terrain-delivery/`.


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
[admission receipt](../features/2026-09-23-terrain-corrections/EXPERIMENTS.md#integer-regional-relief-probe).

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

## Fine recipe-5 formation

Recipe 5 replaces the recipe-3 spectral source and recipe-4 valley carving with
stream-power landscape formation from the tectonic macro surface. Its output is
the same canonical 39.0625 m fine terrain file, so continent derivation,
climate, hydrology, publication and export are unchanged downstream.
Source: [formation/](../../../crates/arda-gen/src/formation/mod.rs),
[fine_formation.rs](../../../crates/arda-gen/src/orchestrator/fine_formation.rs).

### §fine-formation levels

Six whole-domain levels at 1250, 625, 312.5, 156.25, 78.125 and 39.0625 m
(300/120/80/50/40/30 iterations). Level 0 starts from the macro surface plus a
four-octave perturbation masked by macro relief. Each finer level warps and
Catmull-Rom-upsamples its parent (§warp), adds band-limited relief, and evolves
with its blurred envelope pinned to the parent (radius four level cells; ten
kilometres against the macro surface at level 0).

### §fine-formation warp

Upsampling samples the parent at a position displaced by smooth value noise of
amplitude 0.6 and wavelength 5 parent cells. The peak derivative stays below one,
so the mapping never folds. Straight coarse valleys bend, and bends compound
across levels.

### §fine-formation incision

Per iteration: envelope correction λ·(blur(target) − blur(z)) plus a relative
uplift of 0.02 × talus × d masked by macro relief and modulated along ranges
(0.2–1.6×); depression fill (§drainage); receivers; contributing area; implicit
detachment-limited stream power (m = ½, n = 1, K dt = 0.02 in metre units,
erodibility 0.25–1.75×) solved downstream-first; a talus cap of
0.85 × (39.0625 m / d)^0.2; and, at the three coarse levels only, linear
diffusion (D dt / d² = 0.02) that sets a physical valley spacing. No diffusion
runs at fine levels: V-valleys and sharp crests come from uplift against the
talus cap, not from smoothing. Lowland soil creep (diffusion number 0.05 × the
lowland factor) runs at every level, so plains stay gentle while mountains
keep sharp crests; the relative-uplift mask has a 2% floor on flat macro
terrain. Non-fixed cells never fall below 0.5 m above the formation base
level.

### §fine-formation hillslopes

At the three fine levels (156, 78 and 39 m) flow must converge (maintainer, 2026-09-29: dense parallel rills "would mean water flowed in hundreds of separate directions"):
- **Channel initiation.** A cell incises only once its contributing area reaches 0.25 km² × (rock strength)², roughly 0.1–0.5 km², scaled by:
  - maturity, 1–2× (it was 1–4×: a mature patch on the seed-42 full-size divide raised the threshold 3.7× and left a smooth crest band 3–4 km wide);
  - low relief, up to 24×, cubic in the missing belt relief (a coastal range of 300 m belt relief now gets 1.4×, where the linear ramp gave 4.9×; low hills keep 11–24×);
  - strong relief, down to 0.35×, cubic in the relief mask;
  - steepness: divided by (S / 0.4)² on slopes steeper than 22°, at most 4× (Montgomery–Dietrich: a channel head needs less area the steeper the slope).

  Below that, hillslopes do not incise and diffuse at 0.04 per iteration.
- **Contributing area** is accumulated on steepest-descent receivers, so water converges. The implicit solve and talus cap keep 0.7-randomised receivers, whose averaging keeps cap geometry isotropic.
- **No band-limited seed noise** is injected at these levels. It seeded pits that each started their own rill.

Rock strength is three rotated octaves (1.5 km, 400 m, 150 m).

### §fine-formation seams

The D8 talus cap leaves one-cell seams, metres deep, where the capping direction changes between neighbouring cells. After the finest level, two passes of a 3×3 binomial (1-2-1) filter over non-fixed land remove them. Crests, spurs and valleys, several cells wide, keep their shape.

### §fine-formation drainage

Depressions are filled with a priority flood over an exact millimetre bucket
queue plus a gradient (2 mm per cell at 39 m). Between exact steps (every fifth
iteration and each level's final drainage), a local fill floods outward from
each closed pit to its first lower cell and grades the flooded cells toward it.
Two local rounds clear almost all pits; any mutually draining remainder acts as
a sink for one iteration. Receivers are steepest descent,
or with probability 0.7 a slope-weighted random downhill neighbour chosen by a
deterministic hash that changes each iteration, so eight-direction
quantisation averages away. Order is upstream-first (Kahn); area is in weighted
finest-cell units.

### §fine-formation masks

Local standard deviation of land height over ~25 km of the macro surface
measures tectonic relief. It saturates masks at 400 m, and settings below 250 m
count as full lowland. The level masks (relative uplift, lowland creep, level
noise, talus relief scale and channel initiation) read **belt relief**: the
highest local relief within a 12 km disc, softened by a short blur. A standard
deviation measures the regional gradient, so it dips along every broad crest
and valley axis. Read directly, a main divide counted as low hills, with up to
7× the channel-initiation area, and stayed a smooth, uneroded band along the
macro crest (seed-42 MICRO, ~4 km wide at 70% of the width). Bathymetry and
coast infill keep the unmodified relief.

### §fine-formation floodplain

At levels 2–5, each cell is compared with the first channel (≥ 2 km²) it drains
to. With fill depth D = 2.5 m × (A/km²)^¼ × lowland, cells less than ½D above
that channel become an alluvial floor at 4% of their former height above it
plus 50 mm. Between ½D and 1½D the fill fades out along a smoothstep, so bluffs
are soft rather than grid-stepped ledges; above that, hillslopes keep their shape.

### §fine-formation continent

Recipe 5 builds its macro geography with `generate_continent_attempt_formed`. It keeps the fine tectonic profile, but classifies plate crust by an elliptical radius from the domain centre with a ±12% per-plate jitter, and decides its margin plates by a per-plate hash. It still consumes the shared margin coin exactly when the square rule would, so plate count, centres and drifts are identical to the shared seeding. The shared seeding classifies by a Chebyshev square, whose axis-aligned zone edges made rectangular collision belts and basins. Plate count, centres and drifts use identical random draws.

### §fine-formation plate drift

Recipe-5 tectonics (`run_tectonics_formed`) keeps each plate's shared drift speed but gives it one of 64 hash-chosen headings, in 1/16-cell units. Integer drift in {−2..2}² moved plates only along axes or diagonals, which swept collision belts into parallelograms with axis-aligned sides: the boxy inland sea.

### §fine-formation macro warp

Formation reads the macro surface and its relief through a domain warp: two
octaves of value noise (±20 km at 120 km, ±6 km at 45 km; peak slope about
0.9, so the warp never folds), tapered to zero within 40 km of the domain edge so the forced ocean rim holds. It bends range crests,
basin walls and macro coasts that the plate model draws along near-straight
edges. All masks, targets, basins and coast infill use the same warped view.
Heights are sampled Catmull-Rom but clamped to the range of the four enclosing
macro nodes. The unclamped cubic undershot the lowstand beside a 1.5 km coast
cliff, and the level cells that sampled the undershoot became a straight line
of fixed sea inside the land: a long, straight trench and ridge along the
seed-42 east coast.

### §fine-formation climate relief

For recipe ≥ 5, rainfall advection reads a box-smoothed copy of the formed 1 km land relief (5 km radius, 2 passes; sea unchanged), because orographic precipitation responds to topography at ~10 km and above. Temperature, regime and ocean masks keep the true grid. Legacy and recipe-4 climate are unchanged.

### §fine-formation margins

Recipe-5 formation replays the plate drift of `run_tectonics_formed` without its uplift (`continent::margins`). For every 4 km simulation cell it records how close each kind of boundary has been over the 20 steps, weighted toward recent steps: **collision** (continent–continent, closing), **arc** (ocean–continent, closing), **island arc** (ocean–ocean, closing, on the overriding plate chosen by a per-pair hash) and **rift** (opening). Proximity is a cubic belt out to 24 km, 0–255. The macro heights are unchanged, so earlier recipes and the continent stay byte-identical.

- **Margin activity** per macro cell is the strongest convergence of any kind.
- **Volcanic island arcs** (goal 17): at the final step's arc axes (ocean–ocean arcs, and the continental edge of ocean–continent arcs), where both the macro surface and its margin profile lie at least 1 km deep, at least 20 km inside the domain, stratovolcano cones rise on the 1 km macro lattice: summits 300–1,400 m, base radius 9–15 km; a straight subaerial cone from the summit to a shore at 0.35 R and submarine flanks `floor × (1 − (1 − s)²)` beyond it (a concave `(1 − r/R)²` cone from a 1–4 km deep floor stood above sea only within 0.4 km of its summit, and every full-size seed-42 summit ended under water); at least 18 km apart, with about a third of axis cells skipped so arcs have gaps. Their flanks count as full relief, so formation erodes them as mountains. Seed-42 MICRO has none (its only arc axes sit near passive shelves); where no subduction reaches deep ocean, no volcanic islands form.

### §fine-formation bathymetry

On the 1 km macro lattice, before formation, open-ocean depth may only deepen toward a margin profile: a roughly linear shelf to −130 m, 60 km wide off passive coasts and 8 km off active ones; a 40 km smoothstep slope; and a 4 km abyss. A coast is active by the larger of two labels, each smoothed over 15 km: its coastal relief (≥ 400 m is fully active) and its tectonic margin activity (§margins). Distance to the coast is Euclidean (a two-pass vector transform carrying each cell's offset to its nearest coast cell), so profile isolines are round rather than chamfer octagons. Coasts, land fraction and enclosed basins are unchanged. The profile is kept.

After the coastal infill, on the formed lattice, **passive shelves also shallow** (goal 18): open sea deeper than the profile, seen through the macro warp, is raised toward it by `(255 − activity)/255`. On a passive margin the shelf is a sediment prism built out over the margin; on an active one nothing is raised. A raised cell stays at least 0.6 m deep, so no land appears and the coast does not move.

### §fine-formation canyons

Off every river mouth with at least 150 km² of catchment, an axis runs down the seafloor smoothed over 1.2 km, in half-cell steps for up to 50 km. Its heading is three parts the previous heading to one part the downhill gradient (never uphill), turned by up to ±22.5° by 7 km value noise. A V-shaped canyon is cut along it:
- relief 120 m, plus 60 m per doubling of catchment, capped at 500 m;
- none shallower than 60 m (the inner shelf), full from 300 m, fading out between 2.5 and 3.5 km (the fan);
- walls reach four times the relief to each side (about 14°), at least 300 m;
- the last 30% of the 50 km fades the relief out, so no canyon ends in a wall.

An earlier D8 path, averaged over ±1 km, still ran as straight axis-aligned and 45° trenches up to 80 km across the filled shelf prism (seed-42 MICRO); a continuous heading never follows a lattice direction. Only open sea is cut, so land, drainage and the water ledger are untouched.

### §fine-formation basins

On the 1,250 m base lattice the (warped) macro surface is filled against the open sea (below −120 m) and the rim. Each component at least 50 m below its spill and at least 256 cells (≈ 400 km²) in size is a tectonic basin with one **endorheic sink**: its deepest cell, with ties broken toward the component centroid. A 1.5 km disc at each sink stays fixed at the macro floor in every level, in the final fine fill and in the sampled pass. The rest of the basin forms normally and drains to the sink, and the annual water balance decides whether a lake, an inland sea or a dry playa surrounds it (goal 6). Fixing whole basins at the flat macro floor was rejected: dry basins then carried straight D8 rivers across a planar floor (seed-42 straight-run share 413‰).

### §fine-formation basin audit

Only the **lowstand ocean** is base level: cells at or below −120 m connected (8-neighbour) to the domain rim through such cells. A closed macro depression deeper than the lowstand used to be fixed as sea floor during formation; the final fills then lifted it 150–200 m to its spill as one planar plain (seed-42 full size: a 60 km plain at 63 m; 250 × 500 km: 52 m) whose rivers ran along grid geodesics. It now forms as land and is audited like any basin.

Before basins are found, every large closed depression of the 1 km macro surface (at least 50 m below its spill and 400 km²) and every plateau (at least 800 m high, local relief under 150 m, 400 km²) is given a tectonic cause from §margins, using the strongest boundary proximity anywhere in it (at least 48 of 255):
- **basins**: rift, then foreland (collision), then fore- or back-arc (arc);
- **plateaus**: orogenic (collision), then arc, then rift shoulder.

A basin with no cause is a macro artefact: it is filled to its spill level, with a 30 mm per cell gradient to its outlet, before formation. It then forms as ordinary drained terrain and never becomes an endorheic sink (goal 6). Explained basins, plateaus (causeless ones are recorded as unexplained) and every glacial trough are stored with the shore layer (§shore), so each lake or inland sea can be traced to its cause.

### §fine-formation rock strength

The talus slope at every level is multiplied by 0.63–1.37 from two octaves of
value noise (1.5 km and 400 m) fixed in world coordinates, so stronger rock
stands as steeper ribs and faces.

### §fine-formation drowned coasts

Formation erodes against a glacial lowstand base level 120 m below sea level:
only macro ocean deeper than that, and the domain rim, are fixed. At the end the
sea returns to 0. Drowned cells further inland of the macro coast than
1.5 km + 6 km × coastal relief are infilled (estuarine sediment) to 0.3 m plus
0.15 m per km inland, so open rias remain near the coast and longer on steep
coasts.

### §fine-formation terraces

After the coastal infill, lowland valley sides are terraced and interfluves flattened (goal 5). Routing runs on a filled copy of the lattice (1 mm per cell), so floodplain flats still route. Each cell is compared with the first river of at least 20 km² it drains to. Its height above that river, `hand`, is remapped by a staircase, and the change is weighted by lowland:
- **Scale.** The floodplain depth `D = 2.5 m × (A/km²)^¼` scales every step.
- **Terrace flight.** It starts at 0.6 D above the river, with steps of 1.5 D: one step below 100 km², two below 1,000 km², three above.
- **Treads and risers.** Each step's riser takes its last 30%. A tread keeps 10% of the original gradient, so the map stays strictly increasing and drainage directions survive. The top riser is the valley bluff.
- **Interfluves.** Above the bluff, height is compressed to 45%, so plains between valleys are broad and flat.
- **Lowland weight.** It is the larger of two terms:
  - the macro lowland mask (§masks);
  - a local plain term: rivers below 100 m (fading out by 300 m) in ground whose height deviation over 2.5 km is under 20 m (fading out by 60 m).
  Hill country and uplands keep their relief (goal 4).

- **Wandering risers.** The staircase is read at a height shifted by up to ±0.3 steps with smooth, rotated noise (1.8 km and 700 m), and the shift is taken back off afterwards. A riser is then a scalloped scarp rather than a straight line along the height contour of a planar valley side (seed-3 MICRO), and interfluves undulate by about ±0.5 of that shift.

The change is box-smoothed over about 80 m, because neighbours that drain to different rivers would otherwise step at their divide. Terraced land never drops below 0.3 m. The final drainage guarantee fills the few shallow hollows the smoothing leaves.

### §fine-formation flats

After the first final fill, before the deltas, every fill flat is regraded (`formation::flats`). A fill leaves its flooded cells at the spill level plus a millimetre gradient counted in grid steps, so flow across it follows grid geodesics: straight axis-parallel and 45° lines radiating from the outlet.
- **Flats.** Non-fixed land cells whose steepest drop to a neighbour is 1–8 mm (fills grade 1–2 mm per step, estuarine infill about 6 mm).
- **Regrade.** Heights become a cost-weighted geodesic from the flats' outlets (flat cells beside a lower cell off the flat keep their height). A step costs 1–5 mm (0.03–0.13‰, √2 on diagonals), from two rotated octaves of value noise (2.4 km and 900 m), squared toward the cheap end. Geodesics bend into the cheap corridors and coalesce there, so rivers wander and converge across the plain.
- Every regraded cell keeps a strictly lower parent; the fill that follows lifts the few hillslope cells a raised flat leaves as pits.

### §fine-formation deltas

Superseded by §world-water deltas (2026-09-30), the one delta rule. Two earlier
rules existed: a 2,000 km² rule that built no visible delta (no MICRO river
reaches 2,000 km², and its fan sat inside the drowned lowstand valley before
the shore rework smoothed it), and a 500 km² fan split by distributaries cut
1.5 m below sea level. Both are removed from the source (`coast::deltas` is
gone); the island split of the second survives in §world-water deltas.

### §fine-formation shore

Heights within ±50 m of sea level are blended toward a 625 m local mean, weighted
by closeness to sea level (wave reworking). A final fine-lattice fill then
guarantees that every fine land cell drains to the sea. Both this fill and the
sampled pass fix only the *open* sea: cells at or below sea level connected to
the domain rim. Enclosed below-sea pockets are lifted to their spill level
rather than published as lakes.

### §fine-formation littoral

After wave reworking, the finest lattice gets the landforms waves and sediment build (goals 15–17). Every shore has a **coast setting**, computed on 312.5 m coarse cells (`formation::coastal`). A coarse cell is coastal when it holds land and open sea, or holds one next to a cell holding the other. Any cell within 6 km takes the values of its nearest coastal cell:
- **Wave exposure**, 0–255: the mean open-water fetch over 16 compass directions, capped at 60 km. The domain edge counts as open ocean. It is blurred over about 1 km and read bilinearly, so it varies smoothly along the shore.
- **Sediment supply**, 0–255: roughly 30 per doubling of the catchment of river mouths within 6 km, which taper with distance. A mouth counts from 1 km². Weak, exposed rock adds exposure/4 of its own waste.
- **Hinterland**: the highest land within about 600 m.
- **Nearshore depth**: the mean open-sea depth within about 2 km.
- **Rock strength**: the §rock strength field.

The littoral pass (`formation::littoral`) then shapes the coast:
1. **Headlands and cliffs.** On exposed coasts (exposure ≥ 40) backed by high ground (hinterland ≥ 15 m), the shore retreats. Retreat = (1.5 km × weakness² + 2.5 km × convexity × (0.2 + 0.8 × weakness)) × exposure/160, plus 30 m:
   - weakness is 0–1 from the fine rock field;
   - convexity is the open-sea share within 1.5 km above 45%, rescaled to 0–1.
   Cut land becomes a wave-cut platform that deepens from 1.5 m at the cliff foot by 15 m per km outward. The last 90 m rise linearly to the old surface, so the cliff line is a smooth contour. Weak rock becomes coves and strong rock stays as points. Inside an arc cone's base radius (§margins) the shore retreats only the 30 m minimum: young basalt, and a small island is convex all round, so the full attack planed every arc summit of the full-size seed-42 world to a shoal 40-80 m deep.
2. **Bay beaches.** On sandy coasts (sediment ≥ 90, hinterland ≤ 80 m), a morphological closing of the land mask with a 200 m radius seals coves and drowned-valley mouths narrower than 400 m with a 0.8 m berm. Straight shores are unchanged. Mouths of rivers of at least 20 km² stay open within 600 m, as estuaries.
3. **Barrier islands and spits.** On low (hinterland ≤ 25 m), sandy (sediment ≥ 120), exposed (≥ 60) coasts with a nearshore mean depth of at most 15 m, a sand ridge rises offshore:
   - it lies 0.9–1.6 km out (12 km noise) and is 120–200 m wide, with a crest from 1 m to 3.5 m;
   - it is built only in water shallower than 10 m;
   - it is broken by tidal inlets where a 7 km noise dips low, and within 2 km of rivers of at least 20 km².
   The water behind stays a lagoon. Where the ridge's offset pinches to the shore it attaches as a spit.

### §fine-formation shore classes

Last, on the published surface, every shore cell of the 100 m prepared grid is classified (`formation::shore`, goals 15 and 16). A shore cell is land next to open sea, or open sea next to land. The drivers are:
- the rise of land within 200 m;
- the open-sea share within 1 km;
- the largest river mouth within 1.5 km;
- the coast setting.

Land cells take the first class that applies:
1. **Salt marsh.** A river mouth of at least 20 km² in a bay below 3 m, a sheltered (exposure < 70), sediment-rich (≥ 60) shore rising less than 4 m, or a delta-fan shore below exposure 100 rising less than 10 m (distributary levees and interdistributary bays).
2. **Beach, in a bay.** The open-sea share is below 45% and the land rises less than 30 m: waves diverge in bays and drop sand.
3. **Cliff.** The land rises at least 18 m and exposure is at least 30.
4. **Beach, elsewhere.** The land rises less than 10 m, and it is a built sand body (barrier or spit sand within one cell, or a delta fan), or a river mouth of at least 20 km² lies within 1.5 km, or, off headlands (open-sea share up to 55%), sediment is at least 60. Convex shores focus wave energy and longshore drift strips their sand into the bays: at full size (seed 42) the old rule, sediment ≥ 120 within 6 km, made low headlands beaches far from any mouth (200‰ on headlands against 148‰ in bays).
5. **Rocky shore.** Everything else.

A beach is **shingle** on high-energy coasts (exposure ≥ 140) below strong rock (≥ 136) with modest river sediment (< 150); otherwise it is **sand**.

The survey's open sea is the hydrology's: 8-connected to the rim. A 4-connected survey left diagonal inlets and fjord channels without a class (7,263 land shore cells at full size).

Water cells are **estuary** at a mouth of at least 20 km² when the open-sea share is below 55%, and **tidal flat** when shallower than 2 m, sheltered (< 80) and sediment-fed (≥ 60). Other open-sea shore cells take the class of their highest land neighbour, so every waterline carries material on both sides.

**Island census.** Land components of the grid other than the largest are islands. Each takes its cause from what built it: **volcanic** (near a §margins cone), **delta** (at least half inside a delta fan), **barrier** (at least 30% barrier sand), otherwise **continental** (a drowned ridge or detached block).

The classes (run-length rows), the islands and the §basin audit landforms are published as `terrain/shore.bin` (`arda_core::ShoreLayer`, BLAKE3-checked). It is staged beside the fine candidate and copied into the world with it. Worlds without the file (all earlier recipes) load unchanged.

### §fine-formation glacial lakes

After every drainage guarantee, a glacier model runs on the final lattice:
1. Ice area accumulates along drainage from land above a glacial-maximum
   equilibrium-line altitude of 1,900 m.
2. A glacier terminus is the last land cell whose ice area is at least 60% of
   its catchment (the accumulation-area ratio).
3. Glaciers with at least 10 km² of ice carve an overdeepened U-shaped trough
   upstream of the terminus along the main donor path.
   - Length: 0.8 × √(ice area), at most 30 km.
   - Depth: 20 m + 5 m × √(km²), at most 200 m.
   - Half-width: 150 m + 40 m × √(km²), at most 2 km.
   - A 15 m moraine lip is built across the valley just below the terminus.
4. A terminus within 30 m of sea level is tidewater: no lip is built, and the
   trough floods as a fjord.

These basins are deliberately closed. The annual water balance decides whether
each holds a lake and where it spills. Overlapping carve discs along a winding
trunk also leave secondary hollows. So after carving, the fine fill and the
100 m sampled pass run again, with a 200 m disc at each trough's lowest carved
cell protected as a sink (lake bed or fjord floor).

### §fine-formation sampled drainage

The 100 m prepared bed is a bilinear point sample of the fine field. A final pass
samples that lattice exactly as `TerrainField::sample` does, fills it, and adds
each filled sample's deficit to its four fine support nodes (maximum per node),
repeating until the sampled lattice drains. This prevents speckle lakes from
resampling (seed-42 MICRO: 1,055 lakes to 1).
- **Sea.** The pass fixes the sea as the hydrology finds it: the 8-connected component of samples at or below sea level joined to the rim. With a 4-connected sea, a lagoon joined across a diagonal shallow inlet was filled to the inlet's depth by raising only its samples' support nodes, a 100 m grid texture (seed-5 MICRO).
- **Pockets.** A sample lifted by 0.5 m or more lies in a closed pocket: every fine node nearer to it than to any other sample is levelled to its filled height, not only its four support nodes.
- The fine fill and the sampled pass run once more after the glacial troughs, always, so the levelled pockets are graded at 39 m too.

### §world-water

After the shore rework and the first final fill, the formed lattice is shaped
for rivers and lakes (`formation::water`, goals 8-13). Every closed basin made
here becomes a protected sink for the later fills and the sampled pass; the
annual water balance still decides whether it holds a lake. The passes only
reshape the bed, so water stays conserved exactly by the shared hydrology.

**Network.** Steepest-descent receivers on the drained lattice; main stems of
at least 3 km², where the larger donor continues at each confluence. Each
stream carries a centreline smoothed over ±400 m, its down-valley direction and
its bed profile. Channels are sized from a nominal 500 mm/yr runoff, since
climate does not exist yet. Streams are shaped largest first.

**Hydraulic geometry.** Bankfull width is `4 m × Q^0.5` and depth
`0.3 m × Q^0.4`, with `Q` the mean discharge in m³/s. The braiding threshold is
Leopold–Wolman, `S = 0.0125 × (4Q)^-0.44`, taking bankfull discharge as four
times the mean. All three are integer (`arda_core::water`).

**Braids.** A reach of at least 20 km² braids where its slope over ±500 m
exceeds the threshold but stays below 4%, and bed load is plentiful:
- a piedmont on low relief (mask < 150) that has left high relief (mask ≥ 200,
  or 60 above its own) within 15 km, or has fallen 500 m over its last 10 km;
- or a proglacial reach, at any relief, whose course rose above 2,300 m within
  25 km.

The floor must also open to at least 40% of the belt. Runs shorter than
1.5 km (gaps up to 300 m bridged) are skipped. The belt (6 × width each side,
120 m to 1 km) is planed to the bed with 0.3 m bars, cutting at most 6 m. Three
interweaving sinusoidal threads are carved, 0.8 m deep for the main thread and
0.4 m for the others. Belt cells are recorded at 100 m.

**Meanders.** On streams of at least 80 km², the target sinuosity is
`1 + 1.2 (1 − r)²`, with `r` the slope over the braiding threshold (capped at
1), smoothed over one wavelength. The valley wavelength is `11 × width`,
never below 800 m (the 100 m layer cannot resolve shorter bends), ±20% along
the stream.
- **Curve.** A Kinoshita curve: heading `ω cos φ + ω³ (Js sin 3φ − Jf cos 3φ)`
  with `Js = 1/12` and `Jf = 1/64`, and `ω` from `S = 1/J0(ω)`. Bend amplitude
  varies ±28% over about 1.3 bends, and bend length ±25% over about 2 bends.
  The curve is pulled back towards the floor middle and joins the old course
  at both ends of a span.
- **Room.** Bends are limited to the floor, measured as ground within
  1.5 m + 4 m of the bed. Spans need 1.5 wavelengths and end one wavelength
  before the stream does.
- **Belt.** The belt (bends + channel + 0.2 λ) is planed to 0.3 m above the
  bed, rising 3‰ away from its axis, cutting at most 5.5 m, which leaves
  soft bluffs (lateral migration).
- **Course.** The new course is carved one bankfull depth (at least 1.2 m) into
  the floor, at least 60 m half-width, with a monotone bed. It runs on at that
  level along the old course until the old bed falls below it.

**Oxbows.** Channels are drained again first, then cutoffs are carved, so a
loop never joins a pit the channels left.
- **Placement.** Where the course swings to at least 70% of its amplitude at a
  bend with sinuosity at least 1.3 (more often the more sinuous: half the
  eligible bends at 2.2), at most one loop in two wavelengths, a crescent
  of radius 0.2 λ (at least 150 m, 240° of arc) lies beyond the bend. Its tips point back at
  the course, two carve radii plus 100 m from it, a rim the 100 m bilinear
  sample keeps.
- **Floor.** The crescent needs a floor within 5.5 m of the bed and at least
  1.5 m above sea level. A floodplain pad is planed around it, rising 3‰ away
  from the belt axis so it drains to the river.
- **Capture.** On the re-drained surface, the loop may capture side streams
  (under 5 km² or a fifth of its river) but never the river.
- **Carve.** Its bottom lies one carve depth plus 0.3 m below the lowest
  floor, flat over the inner 85% of a half-width of at least 90 m, so the
  100 m samples read one depth and a partly filled loop is still one lake.
  The whole crescent is a chain of protected discs. With only a 75 m mid-arc
  disc protected, the fills and the sampled pass lifted every part of a
  loop that did not drain to that point at 100 m (seed-42 full size: 59
  lakes of 1–3 cells, 62 of the 63 smallest being oxbows).
- **Whole or none.** A loop whose 100 m samples 0.3 m below its rim number
  fewer than eight (0.08 km²), fall in more than one 8-connected piece, or
  would not hold at least 0.3 m of water before spilling at the lowest
  sample around them (breached by a delta distributary or another channel),
  is undone.
- **Scale.** With the 800 m wavelength floor, loops only fit on floodplains
  about 1.5 km wide or more. Seed-5 MICRO has one oxbow lake; seeds 42, 3
  and 7 have none.

**Karst.** The rock-type proxy for carbonate is two rotated octaves of value
noise (48 km and 17 km) fixed in world coordinates, above a threshold that
covers about a fifth of the land (189‰ on seed-42 MICRO). It is independent of
relief.
- **Poljes.** Candidates lie about every kilometre along karst streams of
  3-400 km² with valley slope below 0.6%, at least 30 m above the sea, where
  the ground within 5 m of the bed reaches 250 m to either side on average
  (a broad floor, not the thalweg of a V-valley).
  - Picked in hash order, at least 8 km apart, at most one per 150 km² of karst.
  - The valley is pulled down smoothly (most at the centre) to a flat floor
    5-14 m below the downstream rim, inside an ellipse 1.2-5 km long with a
    ±25% lobed outline. A full polje lake therefore has the stream as its
    inlet and spills over the old floor downstream.
  - A basin flooding fewer than 160 fine nodes is undone, so it never becomes
    a speckle lake.
- **Dolines.** A 300 m jittered grid (45% of karst sites, away from channels,
  slope below 25%) records dolines 25-90 m across. They are not carved: at the
  100 m hydrology scale each would be a pit filled by the balance, but dolines
  drain to groundwater, which the shared model does not carry.

**Deltas.** A mouth of at least 150 km² on a low coast (relief mask below
100/255, rising linearly from 1,000 km² to 160/255 at 2,000 km², since a large
river's sediment outpaces a moderately steep nearshore) builds a delta; steep coasts keep their rias and estuaries. It runs
after the shore rework, which would otherwise smooth it away.
- **Sediment.** The Holocene volume is 437,500 m³ per km² of catchment (200
  t/km²/yr over 7 kyr at 1.6 t/m³, half trapped at the mouth).
- **Lobe.** The lobe points down the river's last 2 km (weight 2) blended with the macro
  shelf normal (weight 1), and its reach falls as `√cos θ`, varied ±30% by a
  smooth function of the bearing from the apex (about five sub-lobes across
  the fan). The earlier ±40% positional noise at 1.2 km punched holes in the
  front and left blotchy shoals. Its
  radius is bisected until filling open water (to 130 m, below the lowstand) to a
  1.5 m mean surface uses the volume, so shallow shelves hold wide deltas and
  deep water small bayhead deltas.
- **Surface.** Only the part of the lobe connected to the apex builds (no
  offshore islets). It falls from 3 m at the apex to 0.2 m at the lobe
  radius, concave (`(1 − d/r)²`), and never lowers land. A delta front
  shoals water to 8 m over 0.3 × reach beyond it.
- **Distributaries** (`water::delta_net`). A tree grows from the apex: each
  branch walks down the fan with a sine-generated heading (28–36° at eleven
  widths, at least 800 m) that also wanders ±25° over knots a quarter of the
  reach apart, pulled a quarter towards the radial direction. While 30% of
  the reach lies ahead it forks at 30–50% of it into two branches ±18–30°
  apart, sharing its discharge 45–65%; lobes forking 1, 2 or 3 times reach
  3, 8 and 16 km. A branch ends where it leaves the fan onto older land, or
  half a wavelength beyond the front. Width follows `4 m × Q^0.5` of its share
  (half-width at least 60 m), depth 1–2.2 m.
- **Levees.** Before the channels are cut, each raises a levee on the plain:
  1.4 m above the plain at the apex, 0.4 m at the lobe edge, falling
  quadratically over four half-widths (at least 250 m). The interdistributary
  basins between them stay low; the final fills drain them.
- **Tidal reach and delta islands** (goal 17). Where the plain lies below
  1.5 m, a channel is cut to 1.5 m below sea level, at least 90 m half-wide
  so the 100 m grid sees continuous water. The distal plain is split into
  islands by open water; a delta of 1,000 km² or more whose trunk and a
  fork reach open water counts as split into delta islands.
- **Mouths.** Several mouths into one bay share one delta (largest first).
- **Order.** Deltas build after the littoral pass (§littoral), whose
  convexity-driven headland retreat would otherwise cut a fresh lobe back into
  cliffs; their lobes may bury barrier sand in front of the mouth.

**Publication.** Worlds formed in the run write `areas/<ax>_<ay>/water.bin`
(`arda_core::formats::water`), with forms in the order of the area's rivers and
lakes.
- **Segments.** Bankfull width and depth come from saved discharge. Sinuosity
  and slope are measured over a 2 km window centred on the segment; the window
  follows the largest feeder upstream and the chain downstream. The pattern is
  braided when at least half of the course lies on recorded belt cells,
  meandering at sinuosity ≥ 1.25 below the braiding slope, and straight
  otherwise.
- **Lakes.** Each lake takes the origin of the sink it holds (tectonic,
  glacial, oxbow or karst), resolved across areas by basin. It is terminal,
  so evaporation-balanced and saline, when its connected lake has zero annual
  outflow: an arid endorheic basin.
- **Other.** Deltas are listed by apex and dolines by cell.

The final-write admission counts four calls per area for the layer. Loading
checks the counts against the area's rivers and lakes; worlds without the
layer load with no forms.

### Invariants

- Same seed, attempt and domain give a byte-identical field for any thread
  count.
- Every fine land cell and every 100 m sample has a lower neighbour or is sea.
- Peak RAM is admitted before allocation: about 33 bytes per finest cell plus the
  parent level (planned ≈10.8 GB for 500×1000 km).

## Fine recipe-4 ground material

The opt-in recipe-4 world applies a material pass after the shared annual water
solve and before area publication. It replaces the saturating flat-ground
wetness calculation with a logarithmic drainage/slope score, estimates soil
moisture from rainfall relative to continuous centi-degree evaporation demand,
and scores canopy from temperature, water balance, slope, aspect and slow
absolute-coordinate patches. Drainage ownership and height above the river
remain saved diagnostics; their angular routing boundaries do not tint the
recipe-4 vegetation palette.
Physics overrides classify ice, rock and marsh before forest, scrub, bare or
grass. These fields are persisted in the existing cell format; older recipes
and legacy generation retain their previous composition. Recipe 5 keeps this pass for
moisture and wetness, but its Atlas land colour no longer uses the per-cell canopy
classes (logic/04 §atlas-formed). Source:
[fine_materials.rs](../../../crates/arda-gen/src/orchestrator/fine_materials.rs).

## Dimensions not in play

The recipe-4 material pass is a first ecological layer, not full succession,
species, soil or land-use modeling. Settlement placement and density,
population/rank sizes, fields/land use, named settlements and regions, roads,
bridges/fords/ferries and trunk-corridor prescriptions below remain deferred.
Tactical output retains
sampled land blocks at stride 64 in both axes and the current 24-tile fill;
it is not full tactical coverage or a completed asset expansion. Source:
[fine_materials.rs](../../../crates/arda-gen/src/orchestrator/fine_materials.rs),
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
