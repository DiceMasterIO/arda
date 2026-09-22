---
generated_date: 2026-09-22
generated_at_commit: 342d03e55120
content_hash: fd283f097e0a
paths_covered:
  - ":(top)crates/**"
  - ":(top)tests/**"
  - ":(top).github/workflows/**"
  - ":(top)Cargo.toml"
  - ":(top)Dockerfile"
  - ":(top)rust-toolchain.toml"
absorbed_from:
  - features/2026-09-07-area-water-terrain-realism@2026-09-08
  - features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# Implementation status and open items

## Current status — audited 2026-09-22

Source audit: rendering implementation at `342d03e55120`; the generator, saved
formats and golden fixtures are unchanged from base `23dfb0999672`. This section supersedes the older pending/default-field claims
below and in the original build plan. Implementation, recorded verification
and final visual acceptance are separate statuses.

### Implemented work

| Capability | Current implementation and evidence |
|---|---|
| Continental terrain and climate | Plate/tectonic relief, erosion, coastal shaping, temperature/rainfall, coarse drainage and rivers, and bounded deterministic validation/rerolls. `crates/arda-gen/src/continent/`, `crates/arda-gen/src/orchestrator.rs:357`. |
| Shared fine terrain | One modeled 100 m physical domain is initialized and evolved before areas are sliced; erosion crosses internal publication boundaries. `crates/arda-gen/src/area/prepare.rs:21`, `crates/arda-gen/src/area/evolution.rs:189`. |
| Regional detail correction | Fine-relief amplitude follows surrounding regional height differences instead of absolute altitude. C06 is installed, not pending terrain work. `crates/arda-gen/src/area/prepare.rs:48`, `crates/arda-gen/src/continent/bundles.rs:46`; commit `ed4875d`. |
| Shared annual hydrology | Fine drainage topology, physical basin hierarchy, rainfall/runoff/evaporation support, lake surfaces, annual transfers/accounting, connected reaches and cross-area records. Final areas consume shared results. `crates/arda-gen/src/orchestrator/shared_solve.rs:382`, `crates/arda-gen/src/hydrology/annual.rs:135`, `crates/arda-gen/src/orchestrator.rs:422`. |
| Produced area fields | Height, terrain, slope/aspect, temperature, rainfall, drainage, discharge, channel order/width, height above river and wetness have producers; they are not all defaults. `crates/arda-gen/src/area/shared_compose.rs:165`. |
| Basic ground cover | The canonical composer emits Bare on non-land, Marsh on qualifying floodplain and Grass otherwise. Full biome/forest vegetation is a separate unfinished layer. `crates/arda-gen/src/area/shared_compose.rs:175`. |
| Saved world data | Format-4 publication stores real continent overview/climate/drainage, area cells/objects, global water records and sampled blocks; the completion manifest is published last. `crates/arda-gen/src/orchestrator.rs:450`, `crates/arda-gen/src/orchestrator.rs:480`, `crates/arda-core/src/formats/`. |
| Resource admission | Resource limits are checked before output creation and applied through preparation, solve and publication. `crates/arda-gen/src/orchestrator.rs:338`. |
| Lazy loading | Manifest-first loading, requested area/archive caches and owned area reads for exports exist. Whole-world eager loading is no longer the implementation. `crates/arda/src/world.rs:93`, `crates/arda/src/world.rs:192`, `crates/arda/src/world.rs:208`. |
| Cartographic PNG exports | Area/world maps render saved terrain and physical water geometry. Configurable quality defaults to 8K and accepts 512–32768 pixels; quality exports stream rows/bands through temporary-file publication. Higher image resolution does not regenerate terrain. `crates/arda-render/src/quality.rs:11`, `crates/arda-render/src/overview/streaming.rs:30`, `crates/arda/src/export_quality.rs:20`; commit `c577528`. |
| Atlas rendering first pass | Natural elevation palette, neighbor-aware relief shading, sea-depth colour and per-output-pixel palette/light interpolation; Classic remains the default. Bounded 512–32K exports preserve saved 100 m geography and water masks. `crates/arda-render/src/atlas.rs:164`, `crates/arda/src/lib.rs:28`, `crates/arda/src/export_quality.rs:35`. |
| JSON and developer surfaces | Versioned area/block JSON, reusable Rust facade, CLI `generate`/`preview`/`export`, and Docker build definition exist. `crates/arda-render/src/json.rs`, `crates/arda/src/lib.rs`, `crates/arda-cli/src/main.rs`, `Dockerfile`. |
| Tactical prototype | Seeded 64×64 blocks, 24 tile kinds, bounded WFC attempts, relaxed-fill markers, compressed persistence and symbolic PNG/JSON export exist. Coverage is still sampled. `crates/arda-gen/src/block/`, `crates/arda-gen/src/orchestrator.rs:300`, `crates/arda-render/src/symbolic.rs`. |
| Verification infrastructure | Physical/annual/boundary/resource/codec/render tests and a pinned golden world exist. CI defines Linux/macOS/Windows tests, MSRV, formatting, Clippy and dependency checks. Latest run status is recorded separately below. `crates/*/tests/`, `tests/golden_world.rs`, `.github/workflows/ci.yml`. |

### Remaining implementation and scope boundaries

| Area | Current boundary |
|---|---|
| Rich vegetation and soil moisture | `Cell.moisture` and `forest_density` lack production assignments. Current Bare/Grass/Marsh cover does not implement the planned ecological vegetation system. `crates/arda-core/src/cell.rs`, `crates/arda-gen/src/area/shared_compose.rs:217`. |
| Human geography and society | Settlement/road/building/name/realm/NPC generation remains deferred. `road` and `built_by` default; manifest settlement and named-river counts are zero. `crates/arda-gen/src/area/shared_compose.rs:217`, `crates/arda-gen/src/orchestrator.rs:527`; retained design `logic/06-society-generation.md`. |
| Richer world and area presentation | Classic and the Atlas relief/palette first pass are implemented. Rich ecological cover, settlement outlines, main roads, building outlines and local paths remain work. Detailed steps: OI-12/OI-14 below. |
| Complete tactical scenes | Preliminary constraints/WFC do not supply coherent furnished buildings, continuous site layouts, detailed assets or movement/collision data. Blocks use a 64-cell sampling stride; accessing one decompresses its area's block archive. `crates/arda-gen/src/block/constraints.rs`, `crates/arda-gen/src/block/wfc.rs`, `crates/arda-gen/src/orchestrator.rs:300`, `crates/arda/src/world.rs:274`. |
| Serving and browser integration | No `serve` subcommand or browser app is implemented. Full area JSON has no demonstrated browser parse/loading budget. `crates/arda-cli/src/main.rs`, `crates/arda-render/src/json.rs`, `07-operations.md`. |
| Full statistical calibration | Existing physical, drainage, boundary and determinism checks do not complete the proposed combined Horton/Hack/rank-size/sinuosity/farmland suite. `06-testing.md`, retained step 12 in `implementation.md`. |
| Static model scope | Annual water is implemented. Seasons, snow storage, groundwater and dynamic floods are outside the current model; their absence does not make annual hydrology pending. Terrain samples remain 100 m apart. `crates/arda-gen/src/hydrology/annual.rs:135`, `crates/arda-core/src/coords.rs:10`. |

### Verification and visual acceptance

- **Recorded C06 verification, September 8:** the durable ledger reports five frozen worlds, 38 selected JSON checks, 238 exports, exact annual balances and matching IDs/surfaces on 20,712 both-wet neighboring pairs. Its combined broad/focused/approved-golden record reports 590 passing tests. Source: `changelog.md`, `snapshot/2026-09-07-area-water-terrain-realism@c06-committed-cleanup` and `fix/2026-09-07-area-water-terrain-realism@lake-district-c06-verification`.
- **Later export verification:** commit `c577528` records full 32K area/world exports, 611 workspace tests and 74 final targeted checks, formatting, strict Clippy, Rust 1.96.1 and cached dependency checks. These counts can overlap; they are not added together. Generation, saved formats and goldens were unchanged by that commit.
- **Fresh Atlas verification, September 22:** source `342d03e55120` passes 648 Linux workspace tests (8 ignored), formatting, strict Clippy, Rust 1.96.1 checks and dependency audit. Three old-executable Classic comparisons match byte-for-byte. The retained seed-42 200×300 km panel includes matched 8K views of five areas and repeated deterministic 32K area/overview exports; all 55 saved files and the five re-exported area JSONs are unchanged. Two three-lens GPT-6 Sol review rounds found no issues. Local acceptance/evidence: `features/2026-09-22-geographical-rendering-first-pass/acceptance.md`.
- **Base CI, checked September 22:** [run 34260968213, attempt 1](https://github.com/DiceMasterIO/arda/actions/runs/34260968213), on base commit `23dfb0999672c8f15deee6d94dc5dee89b946ce8`, completed September 8 with a Windows failure. Linux/macOS test jobs, lint, MSRV and dependency checks passed. The [Windows job](https://github.com/DiceMasterIO/arda/actions/runs/34260968213/job/102178507146) failed `micro_world_matches_the_golden_fingerprint`: actual text used LF and expected text CRLF. All 34 logged fingerprint entries match after line-ending normalization; the same-seed repeat test passed. This is an unresolved golden-text comparison failure, not evidence of different generated fingerprints. No newer run/retry was present when checked.
- **Delivered visual correction:** the C06 record reports seed436342 changing from 2,653 to 186 lakes, and 50 km squares with at least 50 lake anchors changing from 18 to zero. The repeated lake-district artifact was corrected. Those observations are not lake-count quotas for every world.
- **Last recorded visual gaps:** parallel drainage/ravines, angular shorelines and large rectangular regional basins remained open on September 8. No later terrain/hydrology behavior change was found establishing their resolution. The new Atlas panel still shows straight/parallel drainage forms and 100 m shoreline steps; it does not establish their cause or resolve the wider rectangular-basin finding.
- **Evidence available here:** the original terrain-realism feature reports/gallery remain absent; their absence does not undo committed work. A new retained Atlas feature folder now contains its own generated world, actual PNGs, hashes, test receipts and acceptance report. These are newly dated evidence, not a reconstruction of the old gallery.
- **Completion boundary:** the broader terrain-realism feature has no completion marker in the durable ledger because overall visual acceptance remained open. Its implemented terrain, water, storage, loading and rendering must not be treated as unstarted.

Current build-order reconciliation: [implementation.md](implementation.md).
Current contracts: [models](02-models.md), [testing](06-testing.md),
[operations](07-operations.md) and [export behavior](logic/04-export.md).

## Detailed work queue — 2026-09-22

This is the actionable queue for the remaining work, incorporating the requested
world, area and illustrated tactical maps. The implementation evidence above is
the baseline. Steps below describe the remaining scope except where a dated delivery is explicitly recorded; a
decision listed inside a task must be settled before implementing its dependent
steps. Existing approved plans and the dated evidence remain available below.
New module names explicitly marked **proposed** do not exist yet.

Each item records the starting point, dependencies, code targets, ordered steps
and a completion gate. Closing an item requires its evidence, not merely code
that compiles. Update its status and link the resulting verification when it
ships. The [map roadmap](../tactical-map-roadmap.md) explains the visual goal;
this queue provides the detailed delivery order and includes non-map leftovers.

### Required result at each scale

| Scale | Required result | Source of truth |
|---|---|---|
| World | Substantially richer geographical detail; city, town and village **outlines**, main roads and other features observable at that distance. | Saved terrain/water/ecology plus canonical settlement extents and regional routes. |
| Area | Building outlines, smaller roads/paths and the larger world features, legible at useful settlement zoom. | The same shared site plan, with finer geometry than the 100 m terrain raster. |
| Tactical | Full terrain, structures, entrances, interiors, furnishings, vegetation and props, with movement/visibility geometry. | Refinement of that shared plan; artwork illustrates the semantic scene. |

World and area maps must work before tactical interiors or furniture exist.
Society and geography establish settlements, functions, populations and access;
site planning supplies streets and building shells; society binds inhabitants
and workplaces to those records. Only bounded feasibility feedback is needed.
A building that cannot fit its required function may cause an explicit shared
layout revision; tactical generation must not silently redraw published maps.

Arda owns the reproducible generated base and semantic geometry. DiceMaster
owns its artwork presentation, game-rule interpretation and campaign changes.
WFC assembles compatible pieces where useful; it does not replace terrain,
town planning, architecture, furnishing rules or an illustrated renderer.

### Delivery order and dependencies

The IDs are stable references, not a demand to finish every row serially.
In particular, start the asset/camera proof early, and improve world relief
rendering while human geography is being built.

| Milestone | Work items | Exit result |
|---|---|---|
| A — trustworthy baseline | OI-01–OI-05 | Reproducible current images and CI; each historical terrain finding has a current disposition. Confirmed fixes can proceed alongside unrelated work. |
| B — shared data and ecology | OI-06–OI-08; begin OI-23 | Stable coordinates, versioned layout contract, staged generation and meaningful vegetation inputs. |
| C — inhabited, richer world | OI-09–OI-13 | Finalized shared settlement/site layout, land use and main roads on a substantially improved geographical render. Rendering development starts before layout finalization. |
| D — coherent area maps | OI-14–OI-15 | Streets/building outlines and linked society using that shared plan; world/area exports work without tactical files. |
| E — visual feasibility, in parallel from A | OI-21 | One structured, asset-backed benchmark proves camera, scale, art and gameplay readability early. |
| F — generated tactical places | OI-16–OI-20, OI-22, complete OI-23 | Detailed, furnished, illustrated sites that preserve the shared layout and publish real geometry. |
| G — playable delivery at scale | OI-24–OI-28 | Coverage, bounded loading, serving and a working DiceMaster scene within measured budgets. |
| H — acceptance and distribution | OI-29–OI-30 | Reproducible quality reports, full CI and verified release artifacts. Checks also accompany every earlier milestone. |
| I — expand content | OI-31 | More environments and building families meet the same acceptance bar. |

### OI-01 — Fix the Windows golden-text comparison

**Status:** open, diagnosed. **Depends on:** none. **Targets:**
`tests/golden_world.rs`, `tests/golden/micro-42.txt`, `.github/workflows/ci.yml`.
The exact-HEAD Windows failure concerns CRLF versus LF; all 34 logged hashes
match after normalization. This does not justify regenerating the golden.

1. Add a focused comparison fixture demonstrating equivalent LF/CRLF text and a genuinely different fingerprint that must still fail.
2. Choose canonical text comparison or an explicit checkout-line-ending policy. Normalize only transport line endings; preserve fingerprint keys, values, record count and meaningful content.
3. Apply the narrow fix. Keep independent same-seed generation comparisons; do not bless new hashes to conceal a portability failure.
4. Run the focused check and golden comparison, then obtain a fresh three-platform CI result for the corrected commit.
5. Record the exact commit/run and any remaining failure. Keep previous successful evidence and the September 8 failure dated separately.

**Done when:** equivalent line endings pass, changed fingerprints fail, and the
fresh CI run passes every configured job. The approved hashes remain unchanged.

### OI-02 — Restore a reproducible visual and measurement baseline

**Status:** partially evidenced by the September 22 Atlas panel: one retained seed-42 200×300 km world, five selected areas, matched styles and both seam directions. The multi-seed/default/MICRO/climate gallery below remains open. **Depends on:** current generator; OI-01 only
for a clean cross-platform gate. **Targets:** existing examples/tests under
`crates/arda/` and `crates/arda-gen/tests/`, `docs/capstone/06-testing.md`;
a gallery/report harness is **proposed**.

1. Attempt to locate the original feature evidence through its recorded paths. If unavailable, retain its ledger citations and generate a newly dated baseline; never label recreated images as original evidence.
2. Freeze a manageable panel including default seeds 42 and 436342, MICRO controls and additional representative climates/landforms. Record configuration, source revision and generator version.
3. Save world/area views at declared, matched physical scales and selected JSON/geometry measurements. Preserve enough input data to rerender without regenerating the world.
4. Mark each old finding—parallel ravines, angular shores, rectangular basins—as reproduced, absent or inconclusive. Distinguish 100 m quantization, image presentation and underlying terrain shape.
5. Capture intermediate stages only where diagnosis needs them: coarse heights, initialized/evolved fine relief, drainage/basins, wet membership and final rendered output. Keep diagnostic tools outside production paths unless they earn a supported interface.
6. Store commands, locations, timings and hashes with the results. Separate deterministic numerical assertions from visual review notes; record missing evidence explicitly.

**Done when:** another developer can reproduce the current gallery and its
measurements, and every historical visual finding has a current disposition.

### OI-03 — Diagnose and correct reproduced parallel drainage/ravines

**Status:** straight/parallel drainage forms remain visible in the September 22 Atlas panel; their cause is not established for current code. **Depends on:** OI-02. **Targets:**
`crates/arda-gen/src/continent/{mod.rs,bundles.rs}`,
`crates/arda-gen/src/terrain_interpolation.rs`,
`crates/arda-gen/src/area/{prepare.rs,evolution.rs}` and hydrology diagnostics.

1. Trace a reproduced regular pattern back to the first stage that introduces it. Measure valley convergence, drainage orientation and incision along the same slopes before and after evolution.
2. Recheck current interpolation: `coarse_height` already calls the bounded, affine-preserving sampler. The older smoothstep-bilinear diagnosis is historical; the separate 4 km→1 km sampler still uses bilinear interpolation.
3. Compare controlled planes, rotated slopes, ridges, converging valleys and real catchments to separate natural parallel drainage from repeated lattice artifacts.
4. Change the demonstrated source—relief construction, interpolation, convergence or erosion behavior—using controlled alternatives. Do not prescribe a noise replacement merely because the old inventory named one.
5. Preserve C06's regional-relief amplitude, genuine bowls, shared-domain continuity, annual accounting and deterministic resource bounds. Reject improvements that introduce speckled coasts or artificial pond fields.
6. Compare the same views across the frozen panel and holdout seeds. Record mechanism, visual improvement, numerical effects and any unresolved cases.

**Done when:** the reproduced defect improves at the relevant scale without
regressing physical/visual controls. No arbitrary diagonal-flow percentage,
river-count quota or noise-only metric substitutes for that evidence. If OI-02
cannot reproduce it, close as not reproduced with evidence instead of changing code.

### OI-04 — Diagnose and improve angular shorelines

**Status:** 100 m coast/lake steps are visible in the September 22 native crops and deliberately preserved by Atlas. Wider shoreline diagnosis and acceptance remain open. **Depends on:** OI-02. **Targets:** current terrain/coast
construction, saved water geometry, `crates/arda-render/src/{carto.rs,channels.rs,overview.rs}`.

1. Inspect world and area views of the same coast/lake boundary against the saved wet cells and physical surfaces. Identify coarse terrain angles, 100 m stair steps and rasterization artifacts separately.
2. Decide which scale needs additional physical geometry and which needs better presentation. Larger PNGs alone cannot resolve either missing physical samples or poor edge styling.
3. For generation defects, correct the earliest responsible terrain/water stage and rerun relevant physical controls. For display artifacts, derive a consistent contour/coverage treatment from the authoritative geometry.
4. Preserve islands, narrow channels, mouth connections, lake identity and supported water levels. Never smooth a visible shoreline across a playable crossing while leaving collision/water elsewhere.
5. Test coastlines, small ponds, large lakes, mouths and area/band seams at several export qualities. Keep streaming and buffered rendering consistent.
6. Record the remaining quantization limit and any finer geometry proposed for tactical shorelines; do not describe cosmetic smoothing as a higher-resolution world simulation.

**Done when:** current shoreline evidence meets the declared world/area visual
target, topology remains correct, and any retained physical limit is explicit.

### OI-05 — Diagnose and correct reproduced rectangular regional basins

**Status:** historical finding awaiting OI-02. **Depends on:** OI-02.
**Targets:** continental relief, shared fine preparation/evolution, fine basin
topology and annual lake support in `crates/arda-gen/src/hydrology/`.

1. Trace each reproduced rectangular waterbody through final wet membership, basin geometry, fine terrain and coarse relief. Separate the shape of a valid filled basin from incorrect water support.
2. Check whether its sides follow real input relief, coarse interpolation, the modeled outer boundary or an internal publication boundary. Internal area boundaries must not behave as physical dams.
3. Correct the demonstrated stage. Do not clip or delete a lake merely because it is large or rectangular, and do not reinstate obsolete 100-cell/2 m thresholds.
4. Preserve real flat-bottomed bowls, connected basin hierarchy, positive-depth storage, exact annual balances, downstream transfers and global feature identities.
5. Compare default seed 42 and the wider panel, including seed436342's corrected lake district and crossing/corner basins. Measure shape and physical support separately.
6. Publish matched before/after evidence and residual resolution limits. If the finding is absent, record that disposition without speculative solver changes.

**Done when:** each reproduced artificial basin shape has an evidenced fix,
valid large lakes survive, and no cross-area water or annual-budget regression appears.

### OI-06 — Resolve coordinates and define shared layout/scene contracts

**Status:** design and implementation open. **Depends on:** none for contract
work. **Targets:** `crates/arda-core/src/{coords.rs,objects.rs,tiles.rs}` and
`crates/arda-core/src/formats/`; shared layout/scene types are **proposed**.

1. Resolve the physical mismatch: a 100 m terrain cell versus 64×1.524 m = 97.536 m tactical blocks. Evaluate an independent global five-foot grid or another explicit mapping; do not adopt one implicitly or stretch artwork as a fix.
2. Define origins, axes, physical units, rounding, negative/boundary coordinates, ownership and conversions among world positions, terrain cells, tactical chunks, rendered points and pointer picks.
3. Define stable settlement, route, crossing, parcel, building and object identities. Specify global ownership and cross-area references; reconcile global settlement IDs with today's `built_by: u16` field.
4. Store settlement extents, road alignments, street connections, parcels, footprints and required entrances as subcell geometry. The 100 m raster must not restrict a building to a 100 m square.
5. Define detailed scene surfaces, heights, water, walls/openings, doors, sparse objects, footprints and interactions. Support bridge decks over water explicitly; decide supported multi-floor cases before promising them.
6. Define optional versus required layers, layout revisions and completion states. Arda supplies semantic facts/geometry; DiceMaster owns rule-specific interpretation, assets and mutable campaign overlays.
7. Specify cross-boundary ownership and valid references before formats are published. Update the retained cell-plus-eight-neighbors block-input restriction to admit the canonical site plan where necessary.

**Done when:** contract fixtures round-trip under OI-23, neighboring chunks align
without drift, and one feature can be referenced consistently at every scale.

### OI-07 — Stage generation around shared geography and independent map outputs

**Status:** open. **Depends on:** OI-06; implement producer hooks alongside
OI-08–OI-20. **Targets:** `crates/arda-gen/src/orchestrator.rs`,
`crates/arda-gen/src/continent/bundles.rs`, public generation/loading interfaces.
Current `write_area` also generates sampled blocks; the new dependencies need
explicit orchestration rather than additional work hidden inside that function.

1. Document the dependency graph: physical world → ecology → settlements/land use/regional routes → local site plan → society binding and tactical refinement. Keep layout independent from artwork.
2. Define the planning domain and deterministic ownership for settlements/routes that cross areas. Introduce the shared inputs absent from current `TileBundle`, such as population allocation and route exits.
3. Split shared-plan production from tactical interiors/furnishings. Make completed world/area maps queryable and exportable even when tactical detail is absent.
4. Choose private intermediate staging or versioned optional sidecars so shared records can be finalized before publication. Preserve create-new semantics, bounded admission and the manifest-last completion guarantee.
5. Add bounded feedback for insufficient farmland, inaccessible sites and impossible building programs. Record shortfalls or revise the shared plan explicitly; do not regenerate the whole physical world for every failed room layout.
6. Specify layout-version invalidation and atomic publication of deliberate revisions. A completed world must never expose a half-updated combination of old roads and new footprints.
7. Exercise generation order changes and cross-border sites; keep physical terrain/water unchanged when only population or cosmetic settings change.

**Done when:** world/area exports need no tactical files, all downstream consumers
read the same finalized shared plan, and partial output is never advertised as complete.

### OI-08 — Produce ecological moisture, forest density and richer vegetation

**Status:** open beyond implemented Bare/Grass/Marsh cover. **Depends on:** saved
physical/climate fields; OI-06/OI-23 for schema changes. **Targets:**
`crates/arda-gen/src/area/{shared_compose.rs,temperature.rs}`,
`crates/arda-core/src/cell.rs`; ecology/vegetation modules are **proposed**.

1. Define ecological moisture units and normalization. Reconcile `Cell.moisture`'s soil-moisture meaning with the design's precipitation/evaporation-demand index; distinguish both from topographic wetness and atmospheric moisture.
2. Reuse produced rainfall, temperature, annual forcing, slope/aspect, HAND and wetness. Define dry/zero-demand behavior without silently replacing the accepted annual water calculation.
3. Implement deterministic vegetation suitability from warmth, moisture, elevation, soil/convexity proxies, aspect and water proximity, including regime-specific tree limits and cold/mixed/warm forest types. Decide whether aspect adjusts ecology only or persisted climate, and verify hydrology if the latter changes.
4. Generate pre-clearing forest potential and density using absolute-coordinate patchiness and neighboring terrain context, so area boundaries do not reveal themselves.
5. Define precedence for wetlands, exposed rock, alpine ground, permanent snow/ice cover, coasts and water. Static snow/ice classification is distinct from deferred dynamic snow storage. Decide which richer vegetation/surface classes need new fields versus objects; the current seven-value `Cover` enum includes Ice but does not represent every planned biome or land use.
6. Persist/export the new fields and provide diagnostic cover/moisture views. Preserve natural potential separately when OI-10 later clears or farms land.
7. Validate explainable climate/slope/aspect fixtures, cold-climate and treeline cases, density ranges, edge consistency and multi-seed distributions, while confirming authoritative water geometry is unchanged.

**Done when:** moisture/forest fields have meaningful producers and vegetation
forms coherent, reproducible distributions supported by the generated climate.

### OI-09 — Generate settlement placement, population and geographic names

**Status:** open. **Depends on:** OI-06–OI-08. **Targets:** existing configuration,
objects, bundles and orchestration; `continent/people.rs`, `continent/naming.rs`
and an area settlement producer are **proposed** under `crates/arda-gen/src/`.

1. Define coarse habitability/population allocation from configured density, including the area denominator, rounding, uninhabitable land and cross-area accounting. The artifact's roughly 40,000 inhabitants describes one area, not the entire default continent.
2. Compute suitability for water access, arable catchment, buildable slopes, safe elevation, sheltered coast, confluences and viable crossings. Apply refusal rules before positive scores.
3. Handle missing downstream-channel/HAND data explicitly; an absent measurement must not be interpreted as a proven safe terrace. Define acceptable flood exposure using the available static model.
4. Allocate population among hamlet/village/town tiers and any intended city tier, with explicit ranges, rank-size targets and deterministic rounding. Place larger centers first and enforce spacing across area boundaries.
5. Record unallocated population when suitable sites run out. Establish initial settlement extents compatible with land availability; finalize them with OI-10/OI-13 through the shared plan.
6. Assign stable IDs, truthful site tags and settlement names. Name connected major river courses, regions, ranges and seas/bays without giving every reach fragment a different river identity/name.
7. Persist objects, fill supported ownership fields and compute manifest statistics from real entities. Add queries/exports rather than leaving names only in a rendered label.

**Done when:** population accounting balances with explicit shortfalls, sites and
spacing are valid across boundaries, and stable IDs/names survive save/load.

### OI-10 — Allocate built-up land, fields and pasture

**Status:** open. **Depends on:** OI-08/OI-09 and shared schemas. **Targets:**
`crates/arda-core/src/{cell.rs,objects.rs}`, area composition;
a land-use producer is **proposed**.

1. Represent settlement extents, built-up land, farmland, pasture and ownership separately from natural vegetation. Do not encode fields as ordinary grass without retaining their meaning.
2. Resolve the retained 0.8 ha/person farm-allocation rule versus the approximately 1 ha/person plausibility target; document population denominator, productivity assumptions and shortages before calibration.
3. Allocate buildable extents by population/tier and suitable fields nearby, preferring eligible open ground before clearing forest. Allocate pasture where appropriate.
4. Resolve overlapping demands and area-boundary parcels deterministically. Preserve unique ownership and prohibit double-counting the same farm support for two settlements.
5. Apply explicit precedence among buildings, fields, pasture and natural vegetation. Preserve pre-clearing forest potential and protected water/terrain constraints.
6. Record unmet land demand and pass it to bounded settlement/site feasibility handling. Integrate roads/building parcels without silently stealing already-accounted land.
7. Expose allocated area, supported residents and shortages in queries, exports and calibration reports.

**Done when:** allocations are valid, contiguous where required, uniquely owned,
measurable and consistent across world/area views and population changes.

### OI-11 — Generate regional roads, crossings and passes

**Status:** open; `Cell.road` currently defaults. **Depends on:** OI-06–OI-10.
**Targets:** objects, bundles and orchestration; regional corridor planning and
`crates/arda-gen/src/area/roads.rs` are **proposed**.

1. Define a reproducible route-cost surface from terrain, grade, cover, land use and physical water widths. Convert the design's percentage-grade limits correctly; 30% grade is not 30 degrees.
2. Connect intended town centers with trunk routes, then villages and hamlets with lower classes. Reuse routes and crossing sites where appropriate instead of producing independent straight spokes.
3. Establish shared corridor IDs, precise area exits, widths/classes and graph connectivity. Preserve those records through later local street generation.
4. Handle islands and unreachable components explicitly. Roads cannot silently traverse sea/lakes; any ferry requires separate endpoints and its own connection semantics.
5. Classify water intersections as ford, bridge or ferry using actual channel geometry and route context. Distinguish these transport crossings from existing hydrology boundary `SharedCrossing` records.
6. Reserve approaches, landing points and enough space for supported bridge geometry. Derive passes from the route elevation profile and keep the evidence for crossing choices inspectable.
7. Validate route connectivity, forbidden surfaces, boundary agreement and generation-order independence. Export connected geometry for all three map scales.

**Done when:** intended reachable settlements connect through valid routes and
crossings, with consistent geometry and stable identities across areas.

### OI-12 — Substantially improve world-map geographical rendering

**Status:** first geographical rendering pass delivered September 22 at `342d03e55120`: earthy elevation palette, neighbor-aware shading, sea-depth colours and class-filtered per-output-pixel palette/light interpolation. The broader milestone remains open for ecological/human layers and multi-seed acceptance.
**Depends on:** existing physical data to start; OI-08–OI-11 for ecological/human
inputs; OI-13 to finalize published settlement extents; OI-02 for comparison. **Targets:**
`crates/arda-render/src/{channels.rs,overview.rs,overview/streaming.rs}`,
`crates/arda/src/export_quality.rs`; `crates/arda-render/src/atlas.rs` and `crates/arda/src/atlas.rs` now implement the relief/style context. Ecological/human overlays remain proposed.

1. Establish a separate world-scale reference, normal viewing size and comparison panel. The close-up tactical illustration alone does not define acceptable continent cartography.
2. **Delivered first pass:** consistent relief shading and natural elevation colours, using two-cell context from eight neighbors and explicit outer-world edges. Palette and lighting interpolate separately at each output pixel; water classes remain authoritative.
3. Improve coast/lake/river hierarchy, edge coverage and readability while preserving physical locations and water extents. Keep confirmed geometry corrections in OI-03–OI-05 separate from shading work.
4. Add forest, farmland and built-up appearance from real generated records as producers arrive. Define scale-dependent detail and contrast so geography remains readable under human overlays.
5. Render city/town/village extents and main roads from the shared geometry. Preserve physical extent; specify subpixel symbols/minimum strokes as presentation rules rather than enlarging the saved settlement.
6. Pass human/ecological object geometry into world rendering when its producers exist. Classic overview callbacks supply `AreaCells`; Atlas callbacks supply `(AreaCells, AtlasTerrain)`. Neither supplies settlement/building/road object geometry.
7. **Delivered for Atlas terrain:** bounded neighboring context and streamed rows/256-row bands, with preserved publication and quality limits. Repeat 32K exports peaked at about 43 MiB (area) and 65 MiB (overview) on the verification host. Future overlays must preserve these bounded structures.
8. Review matched-scale before/after outputs with and without human overlays, including multiple seeds and seams. Record visible improvement, resource cost and unresolved generation limits.

**Done when:** world views are substantially richer in geographical detail,
settlement outlines/main roads are readable and correctly placed, and progress
is demonstrably more than a larger image or extra road lines.

### OI-13 — Generate shared local streets, parcels and building footprints

**Status:** open. **Depends on:** OI-06–OI-11. **Targets:** shared objects and
staging; site-layout modules are **proposed** in `crates/arda-gen/src/`.

1. Convert settlement population, function, land allocation, terrain and regional approaches into a feasible local building/street program.
2. Lay out primary local streets, smaller roads/paths, public spaces and service access while preserving regional route endpoints and water crossings.
3. Partition suitable land into parcels and place purpose-specific building footprints, orientations and entrance/access anchors. Respect slopes, banks, setbacks and minimum usable interior dimensions.
4. Produce each cross-boundary street, building and bridge once under canonical ownership. Slice/reference its geometry in neighboring area/chunk outputs without independent rerolls.
5. Resolve insufficient space through bounded program adjustment or explicit layout revision. Finalize the settlement extent and population support before publishing the shared plan.
6. Publish building IDs, purpose and shell geometry before interiors. Reserve required access connections so later furnishing/WFC cannot erase them.
7. Check connected street/entrance graphs, footprint overlaps and alignment with the broader map. Demonstrate exports when no tactical layer has been generated.

**Done when:** a coherent site plan supplies all area-map outlines and tactical
inputs, with one shared identity/geometry for every building and route.

### OI-14 — Render useful area maps with buildings and local routes

**Status:** open overlays and viewing support. **Depends on:** OI-13 and relevant
OI-12 terrain styling. **Targets:** `crates/arda-render/src/{carto.rs,json.rs}`,
quality exports and public area queries; view-window/crop support is **proposed**.

1. Extend the existing cartographic path, which already receives `AreaObjects`, world origin and output scale, to draw local roads/paths and individual building outlines.
2. Define layering, clipping, line weights and minimum visible features. Preserve the world map's routes and settlement extents while revealing finer shared geometry.
3. Support settlement crops or view windows with correct physical scale. A full 51.2 km area at 8K is 6.25 m/pixel; that alone is insufficient for inspecting many small building outlines.
4. Expose building/route shapes through the versioned area JSON/object contract. Never require the consumer to trace an exported PNG to recover geometry.
5. Handle buildings/paths crossing areas and streamed bands without duplicates, gaps or clipped entrances. Reuse the canonical ownership rules.
6. Check representative village/town/dense-site crops and compare them with world and tactical views. Export successfully before interiors/furniture exist.

**Done when:** building outlines and local routes are legible at the declared
view scale and exactly match the shared plan later consumed by tactical generation.

### OI-15 — Bind realms and society to the shared geography

**Status:** open. **Depends on:** OI-09–OI-11/OI-13 and versioned schemas.
**Targets:** retained [society design](logic/06-society-generation.md);
`crates/arda-gen/src/society/` and Realm/Building/NPC codecs are **proposed**.

1. Define durable realm, building and NPC identities/references, and decide which game-neutral facts Arda stores versus any optional rule-system data.
2. Select realm seats and assign territory over the produced road/terrain cost surface. Treat retained realm-count and notable-count values as assumed, tunable defaults.
3. Handle zero-town worlds, disconnected islands, ties and inaccessible components. If borders snap toward rivers/ridges, preserve valid partitions and unique land ownership.
4. Create society's building records from OI-13's canonical footprint/purpose records. Move the retained post-block building dependency upstream; do not create a second building-layout generator.
5. Generate tier-appropriate notables and bind homes/workplaces/roles to real buildings and settlements. Add household relationships where required by the consumer without equating building count with population.
6. Derive other inhabitants on demand from stable world/settlement/entity keys. Stored NPC data should scale with settlements; reconstructions must not depend on request order.
7. Add exports/queries and content attribution when rule-system content is introduced. Resolve building-capacity shortages through the bounded shared-plan process, not endless world↔tactical regeneration.

**Done when:** realm ownership is valid, all society references resolve, building
geometry is shared with maps, and inhabitants reconstruct deterministically.

### OI-16 — Refine tactical terrain, banks and stacked movement surfaces

**Status:** open beyond tile-family selection. **Depends on:** OI-06/OI-13;
uses existing physical water/terrain. **Targets:** block input constraints and
**proposed** local terrain/surface producers under `crates/arda-gen/src/block/`.

1. Sample saved physical heights, water surfaces, channels, wetness and slope in the chosen coordinate system; carry layout routes, crossings and foundations into local constraints.
2. Add controlled banks, terraces, foundations, ditches, paths, steps and ramps. Interpolating 100 m samples is a base surface, not enough detail for the reference scene.
3. Preserve authoritative boundary conditions and water connectivity. Any fine bank shape must agree with world/area water semantics at crossings and shared edges.
4. Generate bridge decks, supports/clearance, approaches, docks and landings as explicit surfaces. Water and a walkable deck may occupy the same horizontal location.
5. Build traversable surface connectivity and declared elevation transitions before decoration. Distinguish impassable cliffs, wading water and supported routes through semantic attributes.
6. Validate boundary/corner scenes, roads meeting doors, both bridge approaches and water underneath. Keep geometry independent of PNG quality and chosen artwork.

**Done when:** local terrain supports coherent playable places, neighboring
scenes join, and supported vertical/crossing cases have explicit geometry.

### OI-17 — Generate interiors within the published building shells

**Status:** open. **Depends on:** OI-13/OI-16 and scene contracts.
**Targets:** **proposed** building/interior producers and the shared building record.

1. Consume each building's footprint, purpose, required entrances and access anchors. Define building-family rules for houses, farms, inns, warehouses and other initial supported functions.
2. Partition rooms and corridors with minimum usable sizes, wall thickness and circulation clearance. Fit the published shell instead of moving it to make a random layout succeed.
3. Place openings, doors, floors and supported stairs/elevation changes; distinguish visual roof treatment from the actual structure.
4. Generate a large building once, then reference/slice it across tactical chunks. Cross-chunk walls and rooms must not reroll independently.
5. Derive visual structural pieces and obstruction/movement geometry from that same layout. Keep required exits and intended room access verifiable before furniture placement.
6. For impossible programs, use bounded alternatives or an explicit shared-plan revision; invalidate dependent views if a footprint changes.

**Done when:** interiors fit their area-map outlines, required entrances connect
to local roads, intended rooms are accessible and chunk boundaries lose no structure.

### OI-18 — Replace permissive tactical WFC with meaningful constrained assembly

**Status:** prototype exists; full constraints/propagation open. **Depends on:**
OI-06/OI-13 and the relevant OI-16/OI-17 input masks. **Targets:**
`crates/arda-core/src/tiles.rs`, `crates/arda-gen/src/block/{constraints.rs,wfc.rs}`.

1. Define the semantic vocabulary from the supported terrain/building families. The retained 200+ tile target is a coverage ambition; increasing IDs alone does not improve composition.
2. Introduce directional edge compatibility, rotations and validated opposite-edge joins for structural pieces. Keep globally planned buildings/routes outside purely local adjacency unless a deliberate solver design incorporates them.
3. Supply per-position domains, fixed boundary/entrance pins, terrain masks, reserved routes and structure constraints. Replace the current one-allowed-list-for-all-squares input.
4. Propagate every domain reduction through a work queue until stable or contradictory. Current filtering reaches only uncollapsed immediate neighbors of the latest choice.
5. Add purposeful deterministic weights and tie-breaking. Separate semantic selection from cosmetic variation so changing decorative art cannot move walls or roads.
6. Measure domain propagation and cell selection at realistic vocabulary sizes; optimize bounded data structures from that evidence instead of copying the prototype's repeated full scan.
7. Validate multi-hop propagation, impossible pins, directional joins, rotation equivalence, repeatability and whole-site route constraints across seeds. Check accessibility separately where adjacency cannot guarantee it.

**Done when:** generated assemblies have coherent spatial structure and obey the
shared plan, with demonstrated propagation and bounded deterministic execution.

### OI-19 — Make contradictions, retries and fallback meaningful

**Status:** open; existing eight-attempt/first-tile fallback is insufficient for
future structural content. **Depends on:** OI-18 and mandatory layout constraints.
**Targets:** `crates/arda-gen/src/block/wfc.rs`, error/completion metadata and exports.

1. Separate mandatory boundaries, water, shells, entrances and routes from optional furnishing density or decorative preferences.
2. Construct nonempty incompatible-domain fixtures that genuinely exercise contradiction propagation and retries. The historical self-compatible-tile argument is not a general guarantee that greedy solving succeeds.
3. Define a bounded relaxation sequence that relaxes only optional preferences. Preserve mandatory geometry through every attempt and record its deterministic seed/attempt.
4. Decide whether impossible mandatory inputs produce a typed unavailable-site result or a proven safe simpler layout. Reconcile that choice with the retained design's promise that block fill cannot fail.
5. Export failure/relaxation reason and quality state. Never advertise a blank or uniform fallback as complete playable content just because an array was produced.
6. Revalidate required connectivity and boundaries after fallback; measure fallback frequency by environment in the seed panel and investigate systemic causes.

**Done when:** real contradictions exercise the policy, work is bounded and every
published playable fallback still satisfies mandatory geometry/access requirements.

### OI-20 — Add semantic attributes, POIs and purposeful furnishings

**Status:** open; current tile definitions are ID/name/group only.
**Depends on:** OI-16/OI-17, contracts and relevant OI-18 assembly.
**Targets:** `crates/arda-core/src/tiles.rs`, scene object formats,
`crates/arda-render/src/json.rs`; furnishing/POI producers are **proposed**.

1. Define materials, traversal classes, movement inputs, obstruction/cover geometry, heights and hazard/interaction tags. Keep rule-specific calculations in the consuming game.
2. Add stable sparse records for furniture, containers, large vegetation, shrines, campsites and other supported POIs. Specify footprints, orientation, ownership and any interaction anchors.
3. Place objects in functional groups: cargo near loading access, shelves against compatible walls, tables with chair space, vegetation where terrain allows it.
4. Reserve circulation and interaction clearances; recheck entrances, room paths, road widths and bridge approaches after furnishing.
5. Separate meaningful objects from visual scatter and give each deterministic seed domains. Cosmetic grass, stones and dirt must not acquire collision merely because their sprites overlap squares.
6. Export attributes and objects directly, with complete coverage of supported semantic kinds. Validate object references, bounds and save/load identity.

**Done when:** furnished scenes are believable and navigable, meaningful objects
have usable semantics, and cosmetic variation leaves gameplay geometry unchanged.

### OI-21 — Prove the camera, asset kit and illustrated target early

**Status:** open; run in parallel from milestone A. **Depends on:** an initial
OI-06 scene fixture, without waiting for full procedural generation.
**Targets:** DiceMaster's visual specification and asset pipeline; any Arda
asset-export contract is coordinated through OI-23. This is cross-repository
future work, not a claim that a renderer already exists.

1. Reconcile the supplied overhead illustrated map with DiceMaster's flat-fill/symbol specification and proposed 20° camera tilt. Compare camera choices using the same fixture before creating a large library.
2. Fix physical asset scale, normal play zoom, permitted rotations and lighting/shadow conventions. Test useful pixel densities rather than assuming more pixels make better artwork.
3. Create a small cohesive kit for a riverside warehouse/customs house: ground, banks, water edges, floors, wall joins, doors, bridge/dock pieces, trees and cargo/furniture.
4. Define a versioned catalog with asset IDs, semantic mapping, dimensions, pivots/anchors, layers, rotations, variants and source/redistribution information. Gameplay footprints remain authoritative scene data.
5. Build one authored structured scene with a bridge above water, interior, doorway, canopy and furnishing groups. Assemble reusable assets; a single painted background does not prove the pipeline.
6. Review with tokens, grid toggle, paths, fog and targeting overlays at normal zoom. Check seams, transparent margins, scale, repetition, baked shadows and readability.
7. Record the accepted benchmark, missing asset families and measured texture costs. Introduce import checks/atlas packaging based on that proof before expanding production.

**Done when:** an actual rendered, structured fixture convincingly approaches
the reference's visual richness and still communicates playable geometry.

### OI-22 — Render generated tactical scenes with layered illustration

**Status:** open; `symbolic.rs` remains a useful diagnostic renderer.
**Depends on:** OI-16–OI-21 and OI-23 contracts. **Targets:** DiceMaster's future
scene renderer and optional asset-backed offline export; `tileset.rs`-style
rendering is **proposed**, not delivered code.

1. Feed semantic scenes through the proven asset catalog, with deterministic visual variants and explicit handling for missing assets.
2. Render terrain blends, water/banks, floors, structures, furnishings, vegetation and shadows in coherent layers. Allow one object to span several squares and several layers to occupy one square.
3. Define depth sorting, cross-chunk rendering ownership, roof/canopy visibility and transparent cutaways. Large trees/bridges cannot be clipped to their anchor square.
4. Add controlled wear, edge blending, contact shading and scatter without moving geometry or duplicating incompatible shadows.
5. Keep grid, tokens, selections, fog, path previews and target templates as independent readable overlays. Preserve the intended exploration mode and camera interactions.
6. Compare generated instances with OI-21's authored benchmark across seeds, chunk boundaries and supported biomes. Retain symbolic/semantic debug views for diagnosing layout versus artwork problems.

**Done when:** generated sites meet the visual benchmark, remain readable in play
and display the same geometry used by simulation, with no baked-in token/grid state.

### OI-23 — Version and persist shared layouts, scenes and complete exports

**Status:** format-4 and current JSON implemented; new payloads open.
**Depends on:** OI-06; evolves alongside each producer, not only after rendering.
**Targets:** `crates/arda-core/src/{formats/,rng.rs}`, `crates/arda/src/world.rs`,
`crates/arda-render/src/json.rs` and facade exports.

1. Specify binary/JSON versions for shared plans and detailed scenes, including units/origin, world identity, layout revision, generator identity and layer completion. Decide explicit refusal versus migration for old formats.
2. Preserve stable IDs and canonical record ordering. Add deterministic stage domains for layout, interiors, furnishings and cosmetic variants without accidentally renumbering existing random streams.
3. Extend object/scene codecs and JSON with produced climate/ecology, settlements, roads, buildings, realms, NPC references, POIs and geometry as applicable. Current area JSON omits some already persisted cell fields; audit the contract field by field.
4. Distinguish absent, ungenerated, unsupported and corrupt layers. Validate references, enum values, counts, coordinates, lengths and decompression/allocation limits before exposing objects.
5. Define query surfaces at world/site/chunk scale rather than forcing every consumer to load an entire area's explicit JSON. Keep full exports available where appropriate.
6. Preserve immutable base-world publication and existing failure-safe PNG export. Cache representations by world/layout/generator/render/catalog versions and quality, not seed alone.
7. Keep campaign changes separate and keyed to compatible stable identities/layout versions. Define behavior when a saved campaign references a revised or unavailable base scene.
8. Verify round trips, repeatability across request order, unsupported-format refusal and failure publication paths. Pin artwork versions separately where reproducible presentation is required.

**Done when:** every implemented producer reaches a documented bounded query/
export, references round-trip, and neither artwork changes nor partial writes
silently alter published geometry or saved campaign interpretation.

### OI-24 — Replace sampled tactical coverage with an explicit delivery strategy

**Status:** open; current materialization uses a 64-cell stride and land-only
sampling. **Depends on:** representative detailed scenes and OI-23 measurements.
**Targets:** `crates/arda-gen/src/orchestrator.rs`, generation/query contracts.

1. Measure time, memory, storage and failure rates for representative detailed scenes before removing the stride. Calculate whole-world cost using the chosen physical chunk mapping.
2. Choose offline complete materialization, deterministic on-demand generation or a hybrid. Offline per-land-cell generation is the retained design; a different strategy needs an explicit documented decision.
3. Define coverage for wilderness, settlements, coasts, water crossings and intentionally unsupported environments. A bridge over a water cell cannot disappear under a land-only coverage rule.
4. Ensure large-site ownership and seed keys make adjacent results independent of request order and generation concurrency.
5. Specify completion/progress, interruption/restart behavior and resource admission for the chosen strategy. Do not silently treat current absence as a valid empty scene.
6. Exercise random coordinates, long journeys and boundary-spanning sites. Measure complete/pending/unavailable outcomes and retain diagnostic failure reasons.

**Done when:** every supported reachable location has a defined, tested path to
playable content, with declared generation cost and no accidental stride-sized holes.

### OI-25 — Bound per-scene I/O, decoding and caches

**Status:** lazy world/area loading exists; whole-area block decoding and retained
caches remain limits. **Depends on:** OI-23/OI-24 strategy. **Targets:**
`crates/arda-core/src/formats/blocks.rs`, `crates/arda/src/world.rs`, consumer caches.

1. Profile current archive reads, decompression and retained memory using realistic detailed payloads. Preserve manifest-first loading and owned area reads already available.
2. Introduce independently compressed frames with validated offsets/indexes, or another measured bounded-read layout. One chunk request must not require decoding every scene in a large area.
3. Validate offset/length bounds, duplicate keys, truncated/corrupt frames and decompression limits. Distinguish missing coverage from corrupted data.
4. Add bounded caching and eviction appropriate to library/server/browser ownership. Current `OnceLock` caches retain loaded archives; lazy access alone does not bound a long journey.
5. Prefetch nearby chunks/assets and cancel stale work without changing deterministic results. Handle cross-chunk objects through references with defined lifetimes.
6. Measure cold/warm access and long-session memory, including maximum-size sites and bad-file fixtures.

**Done when:** requesting a scene reads/decompresses bounded data, travel memory
stays within recorded budgets and missing/corrupt content produces useful errors.

### OI-26 — Implement read-only serving and the real DiceMaster transport

**Status:** open; CLI `generate`/`preview`/`export` and Rust queries already exist.
**Depends on:** OI-23/OI-25 and the chosen OI-24 behavior. **Targets:**
`crates/arda-cli/src/main.rs`, facade/load/export code; a server module is
**proposed**. [Serve design](mockup/06-serve.md) contains assumed endpoint names.

1. Reconcile those endpoint sketches with actual combined area JSON and the new world/site/scene payloads. Define supported representations, version negotiation, optional layers and asset delivery ownership.
2. Implement a read-only server reusing library loading/serialization/rendering. Decide whether the retained synchronous server model meets measured concurrency needs before adding runtime complexity.
3. Validate startup/completion, coordinate bounds, content types and missing/corrupt/unsupported layer responses. If on-demand generation is chosen, define its separate cache/job lifecycle without mutating the published base unexpectedly.
4. Define strong ETags or equivalent validation from representation identity/content. `(seed,path)` alone omits layout, generator, style and quality changes.
5. Bound cache size, request work, concurrent expensive exports and geometry/payload sizes. Specify browser origin/CORS and deployment configuration where required by the consumer.
6. Implement conditional requests and integrate DiceMaster's loaders. Test cold load, revisit, mismatched versions, absent chunk and delayed/cancelled navigation.
7. Document startup, shutdown, port configuration, world mounts and errors. Confirm responses agree with the corresponding CLI/library representations.

**Done when:** the consumer loads a real scene through the implemented contract,
caching is correct, and serving does not rewrite immutable world files.

### OI-27 — Integrate gameplay geometry, fog and campaign changes

**Status:** integration open in DiceMaster. **Depends on:** OI-16–OI-23/OI-26.
**Targets:** shared scene contract and DiceMaster's client/server spatial systems;
the sibling architecture is a design reference, not proof of implementation.

1. Bind client previews and authoritative server calculations to the same geometry and compatible scene/rule versions. Resolve movement, sight, cover and targeting from semantic surfaces/obstacles.
2. Verify coordinate conversion, picking, token footprints/heights and chunk transitions under the chosen camera. A visually plausible click must select the corresponding physical location.
3. Exercise walls/doors, furniture, trunks versus canopies, bridge decks/water and supported stairs with explicit expected paths and sight cases.
4. Integrate fog, visibility and explored-state persistence without exposing hidden scene information through rendering layers or client-only authority.
5. Apply mutable door/object/damage changes as campaign overlays with stable identities. Reload and revisit the same place without losing state or modifying Arda's immutable base.
6. Test a complete world→area→tactical journey and back, including a layout-version incompatibility and unavailable scene. Shared footprints/routes must remain consistent through zoom changes.

**Done when:** the map behaves as it looks, client previews agree with authority,
and exploration/campaign state survives revisits and boundary transitions.

### OI-28 — Measure and meet generation, transport and rendering budgets

**Status:** admission/streamed exports implemented; full-content and interactive
budgets unproven. **Depends on:** representative output from each producer;
measure throughout, then close after OI-24–OI-27. **Targets:** generation resource
accounting, export/loading benchmarks and DiceMaster instrumentation.

1. Inventory current measured budgets and retained targets separately. Existing release benches (continent ≤60 s, area erosion ≤30 s) are different workloads from proposed full-world generation ≤12 h, area export ≤60 s, block export ≤5 s and world load ≤100 ms targets.
2. Define hardware, world/scene sizes, cold/warm state and exactly what each timer includes. Ratify or revise proposed targets with evidence; do not silently apply a stage limit to complete detailed generation.
3. Extend pre-output resource admission to population planning, geometry counts, WFC domains, objects, archive indexes and export overlays. Bound new dense allocations and retry work.
4. Measure payload/network transfer, decode/parse, first useful view, CPU/RAM, texture/GPU memory and frame time separately. Full explicit area JSON can be tens of megabytes; demonstrate the browser path rather than assuming it is cheap.
5. Validate the retained DiceMaster targets of 60 fps desktop, 30 fps mobile, INP ≤200 ms and play-code ≤1 MB gzipped on declared devices. Specify separate asset/texture/network budgets; they are not included automatically in the code budget.
6. Use chunking, prefetch/eviction, levels of visual detail and appropriate atlases based on measurements. Reduce cosmetic cost on constrained devices while preserving identical meaningful geometry.
7. Test dense settlements, water/bridge boundaries, cold starts and long travel. Publish bottlenecks, achieved numbers and any explicit target changes.

**Done when:** the chosen delivery strategy meets recorded budgets on declared
workloads/devices without losing streaming, bounded memory or playable geometry.

### OI-29 — Complete statistical calibration and cross-scale acceptance

**Status:** physical/accounting/boundary/determinism tests exist; combined
calibration and new-content acceptance remain open. **Depends on:** each metric's
producer; starts with OI-02 and grows with milestones. **Targets:**
`crates/arda-gen/tests/continent_measures.rs`, existing tests, **proposed**
saved-world calibration/report suites, `docs/capstone/06-testing.md`.

1. Inventory existing checks so numerical accounting, persistence and determinism gates are retained. Separate new missing statistical coverage from tests already delivered.
2. Define metrics and sampling scale: Horton counts, Hack fits, drainage/shore/lake shape, settlement rank-size, road/straight-line ratios, cover fractions and farmland per supported resident.
3. Handle clipped networks, low sample counts, islands, absent settlements and unreachable routes explicitly. Measure canonical fine hydrology as well as any coarse probes; never mix their definitions silently.
4. Treat retained Horton 3–5, Hack near 0.55, open-ground road ratios 1.2–1.4 and farmland targets as scoped calibration hypotheses. Resolve OI-10's farm target and justify tolerances; no single statistic must be forced onto every area.
5. Use controlled fixtures plus frozen calibration and separate acceptance seeds. Report exclusions, sample sizes and uncertainty for larger offline surveys; keep smaller deterministic regression controls suitable for CI.
6. Add structural gates for settlement spacing, road/entrance connectivity, room accessibility, fallback validity, object references, coverage and request-order independence.
7. Pair those reports with matched world/area/tactical galleries. Include cross-boundary towns/buildings/bridges, several biomes and difficult seeds; verify the same IDs/geometry and independent pre-tactical world/area exports.
8. Retain before/after evidence and unresolved findings. Update golden baselines only for intentional reviewed behavior changes under the project's approval rule; never use rebaselining to hide unexplained drift.

**Done when:** every applicable metric has a reproducible definition and justified
acceptance scope, structural/physical checks pass, and visual review meets each
scale's target without unsupported claims from numerical tests alone.

### OI-30 — Finish CLI, packaging and release acceptance

**Status:** facade/CLI/Docker definition and CI delivered; release automation
and full-content acceptance open. **Depends on:** OI-01 and the capabilities
included in the release. **Targets:** current facade/CLI/Dockerfile,
`.github/workflows/`; release automation is **proposed**.

1. Audit the existing CLI/public API against newly implemented layers and queries. Add only missing entry points/options/help/errors; do not rebuild delivered generate/preview/export commands.
2. Decide supported binary targets, crate/image publication, version tags and format compatibility policy. Separate a tested local image from a published multi-architecture release.
3. Implement the planned release workflow with pinned/reproducible build inputs and appropriate artifact/version metadata. Keep credentials and actual publication in the release process.
4. Smoke-test install/load/generate/export paths on target platforms. Exercise the non-root Docker image with mounted world/input/output volumes; document UID/permissions and add serving configuration when OI-26 exists.
5. Verify packaged artifacts can read their advertised saved format and produce the same representations as source builds. Check image/binary architecture and startup errors.
6. Link actual CI/release runs and update README, operations, API and status docs from verified results. Retain historical failures as dated evidence, not current blanket status.

**Done when:** chosen release artifacts are built and verified through the
documented workflow, distribution status is accurate, and all release gates pass.

### OI-31 — Expand environments and building families after the first complete site

**Status:** later content expansion. **Depends on:** a generated, playable
reference-quality site and OI-29 acceptance machinery. **Targets:** generator
templates/rules, semantic vocabulary and DiceMaster's asset catalog.

1. Choose successive supported families: wilderness/forest, farms/villages, town streets and civic/commercial buildings, then additional climate/cultural variants.
2. Add semantic layout and gameplay requirements before artwork. Ruins, caves, mines or multi-level sites require explicit supported generation/geometry decisions where the current project does not produce them.
3. Extend ecology/land-use/site rules, purposeful furnishing groups and the cohesive asset kit together. Preserve scale, camera, lighting and vocabulary completeness.
4. Exercise each family across multiple seeds and boundaries, including difficult terrain and constrained sites. Track missing assets and fallback frequency by family.
5. Reuse cross-scale IDs, layout publication, performance and visual gates; reject diversity that reintroduces incoherent roads, repeated noise or unusable interiors.
6. Publish a coverage matrix listing supported, partial and unsupported combinations. Advance each family only when generated examples meet the benchmark in normal play.

**Done when:** each advertised family has reproducible generated examples,
complete semantics/assets and the same geometry, visual and performance acceptance.

### Scope boundaries and conditional extensions

These are explicit future choices, not hidden blockers for the map pipeline.
They must not make delivered annual hydrology or loading appear unimplemented.

| Boundary | Current disposition | Steps if the scope is activated |
|---|---|---|
| Seasons, snow storage, groundwater, dynamic floods | Outside the representative static annual model. | Specify the process/time step and required inputs; extend water/energy stores and exchanges; add versioned state/output; verify conservation and equilibrium/transient fixtures; expose the resulting seasonal gameplay/render meaning. Do not fold this into OI-08's ecological moisture by accident. |
| Finer world physical raster | Current shared terrain is 100 m; subcell object geometry and local tactical refinement are separately required. | Establish a reproduced need that OI-04/OI-16 cannot meet; measure finer-domain cost; choose multiresolution/global policy; redefine terrain/water boundaries and formats; validate conservation, seams and resource admission. Higher PNG quality is not this feature. |
| Region-file import/front door | Retained optional design, not required for generated worlds. | Specify supported source schema/units; validate and map inputs to canonical world/layout records; define deterministic conflict/error handling; test the same exports and geometry invariants as native generation. |
| Mutable in-process game API | Arda's generated base is immutable; campaign mutations belong to DiceMaster. | First specify ownership and transaction/persistence semantics; build an overlay keyed to stable IDs/layout revisions; validate replay/concurrency/version conflicts. Do not replace the existing read-only query API just to store door state. |
| General checkpoint/resume and parallel world preparation | Current physical preparation/composition is sequential; no broad resumability promise is made here. OI-24 still must define interruption behavior for its chosen coverage strategy. | Profile actual need; identify deterministic checkpoints/partition boundaries; version and validate partial state; test interrupted/resumed equivalence and bounded resources before advertising support. |

### Coverage of every earlier inventory item

Closed items are preservation obligations, not new implementations. The historical
text below is retained verbatim so its evidence and changing diagnoses remain
traceable; its present-tense claims do not override this queue.

| Earlier item | Current disposition and action |
|---|---|
| #1 — tile-sized major-river cap | Closed by entering context and later shared hydrology. Preserve upstream catchment/flow and crossing identity through OI-03–OI-05, OI-16 and OI-29. Do not rebuild a one-area solver. |
| #2 — salt-and-pepper WFC | OI-13/OI-16/OI-17 establish spatial plans; OI-18 implements constrained assembly; OI-20–OI-22 supply furnishing/art. |
| #3 — fallback not meaningfully exercised | OI-19 adds real contradictory fixtures, bounded relaxation and explicit quality/failure states. The old self-compatibility explanation is not a future correctness proof. |
| #4 — noise/terrain directional diagnosis | OI-02/OI-03 reproduce and localize current defects; OI-04/OI-05 cover the other later visual findings. The old interpolation/noise diagnoses are not current prescribed fixes. |
| #5 — rainfall/discharge stand-ins | Closed. Preserve produced climate, rainfall-driven flow and annual accounting in OI-08/OI-16/OI-29; do not restore the old fixed conversion as the final annual model. |
| #6 — empty overview | Closed; current format-4 records supersede historical format-3/18-byte records. OI-12 enriches presentation and object inputs; OI-23 preserves real saved-world provenance. |
| #7 — sparse block stride | OI-24 chooses and implements coverage, supported by OI-25/OI-28 measurements. |
| #8 — eager world loading | Closed by manifest-first/lazy/owned reads. OI-25 targets per-archive decoding and retained-cache limits, not a nonexistent whole-world eager load. |
| #9 — missing statistical suite | Physical tests exist; OI-29 completes the combined statistical, structural and visual acceptance work. |
| #10 — old lake-size/depth thresholds | Superseded by physical fine basins and annual water support. OI-05/OI-29 preserve positive-depth storage/accounting and measure shapes; do not recalibrate obsolete thresholds. |
| #11 — pinned/tapered internal erosion rim | Resolved by shared-domain evolution. OI-03–OI-05/OI-29 preserve internal-boundary equivalence and absence of seam water rims. The true modeled outer rim is a different boundary. |
| #12 — cross-area lake authority | Resolved by shared topology/global IDs/surfaces, including natural checks. Preserve boundary/corner identity and physical surface consistency under OI-05/OI-16/OI-23/OI-29. |
| Later cell-field gaps | OI-08 supplies moisture/forest ecology; OI-09–OI-11/OI-13 supply settlement ownership/roads/land use. Temperature, rainfall and water metrics already have producers. |
| Later CI/visual evidence gaps | OI-01/OI-02; missing old local reports do not reverse delivered features or imply all previous checks failed. |
| “Not defects” records | Identical empty ocean archives can be valid; preserve semantic correctness rather than requiring every file hash to differ. The historical pinned-seam smoothness measurements are dated, not current production terrain requirements. |

| Original build-plan step | Detailed remaining work |
|---|---|
| 0–3 | Delivered foundation; preserve it through relevant regression checks. |
| 4 — continent full | Human allocation/naming OI-09/OI-11; remaining physical findings OI-02–OI-05; statistical acceptance OI-29. Climate/drainage/validation already exist. |
| 5 — area stages | OI-08–OI-14; delivered shared physical fields/basic cover remain credited. |
| 6 — complete blocks/vocabulary | OI-06/OI-07/OI-16–OI-19/OI-24; convergence alone is not visual or gameplay acceptance. |
| 7 — society | OI-13/OI-15/OI-17: one upstream shell/layout authority and downstream interior/social binding. |
| 8 — attributes/POIs | OI-20, with persistence OI-23 and gameplay OI-27. |
| 9 — full export | OI-12/OI-14/OI-22/OI-23; retain existing PNG/JSON and quality streaming. |
| 10 — facade/CLI/Docker | OI-25/OI-28/OI-30; delivered APIs/tooling are not rebuilt. |
| 11 — serve | OI-26 and actual consumer integration OI-27. |
| 12 — validation/performance | OI-01/OI-02/OI-28/OI-29 and each item's specific completion gate. |

| Companion roadmap step | Detailed queue coverage |
|---|---|
| 1 — visual target | OI-02/OI-12/OI-21 |
| 2 — scale/coordinates | OI-06 |
| 3 — scene/ownership | OI-06/OI-07/OI-23 |
| 4 — starter art | OI-21 |
| 5 — authored proof | OI-21/OI-27 |
| 6 — world-to-site inputs | OI-08–OI-11/OI-15 |
| 7 — shared site layout | OI-13/OI-14 |
| 8 — tactical terrain/water | OI-16 |
| 9 — interiors | OI-17 |
| 10 — WFC | OI-18/OI-19 |
| 11 — furniture/decor | OI-20 |
| 12 — rich rendering | OI-12/OI-14/OI-22 |
| 13 — versioned persistence | OI-07/OI-23 |
| 14 — gameplay | OI-27 |
| 15 — coverage | OI-24 |
| 16 — streaming/performance | OI-25/OI-26/OI-28 |
| 17 — multi-seed acceptance | OI-02/OI-29 |
| 18 — content expansion | OI-31 |


## Historical C06 snapshot — 2026-09-08

The snapshot below preserves the original observations and measurements. Its
present-tense statements refer to September 8. Linked feature-local reports and
images are not present in this checkout; current implementation and verification
status are recorded above.

The area-water-terrain implementation is installed. Candidate06 corrects the
altitude-dependent detail amplitude that repeatedly manufactured shallow basins
on gentle high terrain. Detail now follows surrounding regional height differences,
using the existing bounded sampler and noise. The annual water rules, saved format
and renderer are unchanged.

The reported seed436342 changes from 2,653 to 186 lakes; fixed 50 km squares with
at least 50 lake anchors fall from 18 to zero. Its former densest square changes
from 158 to 3. These are measurements, not production quotas. The 16K comparison
visibly removes the repeated pond patches. Total wet area changes from 5,028.39 to
2,558.01 km²; substantial regional lakes remain.

All five frozen worlds, 38 selected JSONs and 238 exports pass, including repeated
bytes, unchanged saved worlds, exact annual balances and shared wet-boundary
identity/surface checks. The combined workspace, focused repairs and fresh
approved C06 golden comparison cover 590 passing tests. The terrain correction
and its deterministic baseline are committed as `ed4875d`. Parallel ravines, angular shorelines and large
rectangular regional basins remain open; the overall feature is not marked done.
Evidence: [C06 comparison](features/2026-09-07-area-water-terrain-realism/reports/lake-district-correction--data-comparison--REPORT.md)
and [maps](features/2026-09-07-area-water-terrain-realism/output/current/lake-district-correction--gallery-c06.md).

| Item | Current evidence and disposition |
|---|---|
| Terrain shape and incision (old #4) | Candidate04 evolves one physical rectangle, applies implicit downstream-first incision, removes the unresolved two-cell detail octave and uses a radial coast mask. A frozen-input replay reproduced the numerical pit defect exactly and the implicit update removed it while controlled physical bowls survived. The radial mask removes a verified planar coarse flank; current default and MICRO images still show regular parallel drainage. The large rectangular seed42 lake follows an existing coarse basin. No target direction percentage or arbitrary river-count reduction is used; overall visual acceptance remains open. |
| Lazy loading (old #8) | Implemented: manifest-only `World::load`, requested area/archive caches, uncached owned area reads for exports. Actual missing/corrupt/sparse-file fixtures verify later I/O failures and admission checks. |
| Lake thresholds (old #10) | Canonical generation uses fine-grid depressions and an annual water-support calculation, including positive-depth physical storage, rainfall, runoff and evaporation. The historical 100-cell/2 m rule no longer controls final world lakes. |
| Cross-area lake authority (old #12) | Shared fine topology, global IDs, physical surfaces and copied feature records replace nearest-coarse-basin/max-surface reconciliation. Constructed boundary/corner controls and all 38 selected natural record comparisons pass. |
| Tactical noise and fallback (old #2, #3, #7) | Remain open. Existing 24-tile WFC, permissive adjacency and 64-cell sampling stride are unchanged. Detailed assets, movement/collision geometry, NPCs and server/browser transport require later work. |
| Erosion rim (old #11) | Candidate02 seed436342 area3,10 had 50.1% wet rim cells versus 6.2% wet interior. Production now evolves terrain across publication boundaries without pins or taper. Candidate04 gives 4.89% wet rim versus 4.95% interior; candidate05 gives 6.51% versus 6.45%. Across all 513 candidate05 default areas, every one of 21,772 adjacent pairs wet on both sides has matching IDs/surfaces; candidate06 checks 20,712 such pairs with no mismatch. The continuous water square is absent in the current preview. Only the true modeled outer rim remains fixed. See `features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--visual-default436342-c05.md`. |
| Statistical calibration (old #9) | Existing drainage, cross-tile, terrain and determinism suites run. This work adds controlled physical/annual/resource/geometry checks and a frozen natural panel; it does not supply the full proposed Horton/Hack/rank-size/sinuosity/farmland calibration suite. |
| Cell producers | Temperature, rainfall, wetness, water and terrain metrics now have producers. Cell.moisture, full vegetation, human geography, roads, buildings and society remain deferred. |

Accepted physical/rendering limits are the 100 m terrain lattice, quantized pond footprints, thin channels rendered with physical area coverage, coarse angular shorelines, sequential world preparation/composition, and a representative static annual balance. Snow storage, groundwater, seasons and dynamic floods are absent. Full area JSON is intentionally explicit and can be tens of megabytes; no browser parse or network-loading budget has been demonstrated. See the current [models](02-models.md), [testing](06-testing.md), [operations](07-operations.md) and [export behavior](logic/04-export.md).

## Historical inventory — through 2026-08-27

The entries below preserve the diagnosis and measurements that motivated later work. The current table above supersedes their implementation-status claims.

## Blocking realism

| # | Item | Evidence | Owner |
| --- | --- | --- | --- |
| ~~1~~ | **Closed by feature 03** (2026-08-27): entering rivers seed each tile from the continent drainage tree. Measured max area-cell catchment at default size 50,070 km² against the old one-tile ceiling of 2,621 km². | Max catchment anywhere = 2,207 km²; hard ceiling is one tile at 2,621 km². A UK-scale major basin is 9,385 km² — **3.6× larger than a whole tile**, so the continent cannot produce even one of the 22–30 systems a UK-sized landmass should have. | `continent/bundles.rs` (bundle has only edge heights), `area/water.rs` |
| 2 | **The WFC tactical layer is salt-and-pepper noise.** Converges, satisfies adjacency, deterministic — and has no spatial structure, because `may_adjoin` lets anything within one "wetness rank" touch, so nearly every tile is compatible with nearly every other and collapse degenerates to uniform random choice. | `export --block 3,7,192,256` on any world. | `arda-core/src/tiles.rs::may_adjoin`, `arda-gen/src/block/wfc.rs` |
| 3 | **The relaxed-fallback ladder is unreachable.** Every tile is adjacency-compatible with itself, so any non-empty constraint set tiles trivially; only an empty set fires the fallback. `logic/03` specifies a ladder that cannot trigger in practice. | Test `an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill` has to pass an empty set. | `arda-gen/src/block/wfc.rs` |
| 4 | **Value noise is anisotropic.** Its gradients favour the square lattice's axes, so steepest descent does too and rivers tend to straight runs. | ~17% of flow directions diagonal against an isotropic ~50%, measured on raw relief before any erosion. | `arda-gen/src/noise.rs` |

On (4): **the recorded diagnosis was wrong, and is corrected here.** A
measured sweep (2026-08-27) found the diagonal-flow share is 23.2%
post-erosion / 38.1% pre-erosion, not the "~17%" recorded — and zeroing the
value-noise detail term entirely barely moves it (21.3%), which proves the
detail noise is **not** the dominant source. The axis bias comes from
`coarse_height`'s own separable smoothstep-bilinear sample of the 1 km grid,
which is what every area cell's regional trend is built on.

Attempts, all measured: per-octave domain rotation (the previously "untried
alternative") moves the share to 23.9% — the floor-then-lattice-lookup
reconstructs an axis-aligned staircase, defeating the rotation; rotation plus
per-octave offset, 23.8%; domain-warping the detail term alone, 23.4–24.0%.
Warping `coarse_height`'s sample position as well reaches 28.6% and visibly
reduces the combs at area zoom — **but it was rejected on the render**: at
default size it speckles the coastline, scatters noise-like micro-lakes
through the interior, and weakens the trunk hierarchy. That is the same class
of regression that killed the first attempt, so it was reverted rather than
shipped.

What the evidence now points at: the combs are strongest on smooth mountain
flanks, where a near-planar slope sends every cell the same way and no
convergence forms. That is a *terrain-shape* problem (too little fine-scale
valley structure for erosion to organise), not purely a noise-isotropy one.
A real fix likely needs the erosion budget or the relief construction
revisited, which is a feature with a spike, not a constant to tune.

## Stand-ins awaiting a producer

| # | Item | Current behaviour | Unblocked by |
| --- | --- | --- | --- |
| ~~5~~ | **Closed by feature 03** (2026-08-27): `Cell.rainfall` is written from the bundle's 1 km rainfall patch; discharge is `Σupstream rain × 125/788,400` L/s plus entering rivers, and channels initiate at 40 L/s. The artifact's "3 km² ≈ 40 L/s" equivalence is now emergent rather than assumed. | — | — |
| ~~6~~ | **Closed by feature 02** (2026-08-26): `overview.bin` carries real 18 B/cell records (relief, climate, drainage) and `continent/objects.bin` carries rivers, at format 3. | — | — |
| 7 | Blocks are materialised on a **64-cell stride**, not one per land cell. | `mockup/02` specifies per-land-cell. | Build-order step 6 |
| 8 | `World::load` reads **every** area and block eagerly. | `logic/05` specifies lazy access with an O(accessed) cache. | Build-order step 10 |

## Calibration held open

| # | Item | Note |
| --- | --- | --- |
| 9 | **No statistical validation suite.** No Horton, Hack, rank-size, sinuosity, or farmland gate exists. | Erosion and lake constants are calibrated only against the artifact's two stated equilibrium anchors (1 km² → 9% slope, 100 km² → 1%) and against Earth's hypsometric curve — not against network statistics. Build-order step 12. |
| 10 | **Lake thresholds** (100 cells, 2 m) were derived from basin distributions measured on **pre-erosion** relief. | Erosion reshapes that distribution; re-derive when (9) lands. |
| 11 | **Tile-edge taper band.** Area erosion ramps to zero over 32 cells at the pinned rim, so the 35° repose rule is not enforced there (measured tan×1000 of 1,324 inside the band against exactly 700 at full strength). | Structural: per-tile erosion must freeze tile edges for neighbours to agree byte-for-byte. Seamless tiled erosion needs a global pass or a proven-decay overlap scheme. |
| 12 | A basin straddling a tile seam: **mechanism now exact, precondition unobserved** (2026-08-27). The continent tier emits lake identity — `ContinentHydrology.basin_surface` gives every 1 km cell inside a filled depression that depression's single surface — and a near-rim area basin takes its lake surface from a nearest-cell (never interpolated) lookup of that constant. Two fragments of one depression therefore agree **exactly**, verified at 0 mm on a constructed two-tile case against the real `compose` path. | Residues, both unobserved: no fixture has yet produced a straddling basin whose cells see a continent depression at all (0 in a 4,280 seed/seam sweep), so the exact path is proven on constructed input rather than natural data; and a fragment whose rim abuts two *different* depressions takes a max of two constants, which is span-dependent again. Where the continent tier sees no depression (sub-km pits invisible at 1 km) the old bilinear rule still applies, pinned at 723 mm on the synthetic case. |

## Groomed but not built

- **Feature 02** — continent climate and hydrology: **built** (2026-08-26,
  commits c2c0d2a..9c48e00; spike S1 discharged by the recorded Horton
  measurements). (1), (5), (12) are now unblocked, not closed.
- **Feature 03** — climate-driven refinement: **built** (2026-08-27,
  commits 78bbff5..8be0a0a). Closed (1) and (5); improved (12); also
  landed the step-10 bundle payload, the step-9 river gate, and
  order-banded river rendering. Five review rounds to dry.

**Next, in the order the evidence suggests:** (4) the value-noise
anisotropy is now the most visible remaining river defect — at area
zoom, streams still run as straight parallel combs along the lattice
axes, which is what "rivers look like fjords" describes at close range.
Cross-tile continuity and hierarchy are fixed; the *shape* of an
individual stream is not. The recorded fix (isotropic gradient noise)
was tried once and reverted for blocky coastlines, so it needs its own
feature rather than an in-place patch.

## Historical unbuilt-step inventory — August checkpoint

This table preserves the August checkpoint, not the current work queue. Its
default-temperature/rainfall claim and blanket pending export/facade stages
were superseded by later implementation. Use the current status above and the
reconciled states in `implementation.md` for remaining work.

| Step | Scope |
| --- | --- |
| 4 | Continent full — **climate, hydrology objects, and S1 done** (feature 02); remainder: human geography, naming, validation stats. |
| 5 | Area stages 3–7: climate, vegetation, settlement, land use, roads. Also `Cell.temperature`, `rainfall`, `moisture`, `forest_density`, `road`, `built_by`, all still at `Default`. |
| 6 | Blocks full: 200+ tile vocabulary. Spike S2 first — note S2 was scoped to *convergence*, and (2) above shows convergence was never the risk. |
| 7 | Society: realms, buildings, NPC sheets. |
| 8 | Tile attribute table + POI layer. |
| 9–12 | Export full, facade/CLI/docker polish, `arda serve`, statistical gates. |

## Not defects

Recorded so they are not re-investigated: the pinned tile seam is **smoother**
than the interior (p90 2,009 mm at the seam against 7,400 mm ten cells in,
land-to-land), and two all-ocean tiles legitimately share identical
`objects.bin` and block-archive hashes.
