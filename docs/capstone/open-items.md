---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 95802ea015e5
paths_covered:
  - ":(top)crates/**"
  - ":(top)tests/**"
  - ":(top).github/workflows/**"
  - ":(top)Cargo.toml"
  - ":(top)Dockerfile"
  - ":(top)rust-toolchain.toml"
  - ":(top).gitattributes"
  - ":(top)tools/art-gen/**"
  - ":(top)apps/viewer/**"
  - ":(top)assets/tactical/**"
  - ":(top)contract-kit/**"
---

# Open items

Only open work is listed. Items finished by v0.8.1 (OI-01, OI-07–OI-11, OI-13, OI-15, OI-17–OI-27, OI-32, OI-33, OI-39, OI-40, OI-44, OI-45) and the dated milestone logs, verification receipts and historical inventories were removed on 2026-10-10; they remain in git history before this change and in [changelog.md](changelog.md). Item numbers are stable and are not reused.

## Current state

**Product state at `022ce7cb5707` (v0.8.1, 2026-10-09).** Arda is now a world generator plus human geography, society, tactical maps, an HTTP server and a browser viewer. Since the previous stamp the workspace has grown from five crates to 23 (`crates/`).

| Layer | Implemented at `022ce7cb5707` |
|---|---|
| Ground ecology | `fine_materials::apply` (`crates/arda-gen/src/orchestrator/fine_materials.rs:13`) writes `moisture` (:56), `forest_density` (:80) and `cover` (:82-99). It runs for legacy terrain and for fine recipe ≥ 4 (`crates/arda-gen/src/orchestrator.rs:350-351`, commit `e06fcf3`). |
| Settlements, land use, roads, realms, names | `arda_settle::pipeline::run` (`crates/arda-settle/src/pipeline.rs:53`) runs these stages in order: placement (:57), seats and primacy (:64-71), land use (:78), roads (:80), crossings and passes (:81-82), realms (:83), names (:93). It writes `settlements.json`, `roads.json`, `realms.json`, `names.json`, `landuse.bin`, `realms.bin` and `roads.bin` under `society/` (`crates/arda-settle/src/lib.rs:72-131`). |
| Society and NPCs | `arda society build` writes `society.json` and `notables.json` (`crates/arda-people/src/files.rs:15-17`). Commoners are regenerated on demand (`crates/arda-people/src/people.rs:3`). |
| Town plans | `TownPlan` (`crates/arda-town/src/plan/types.rs:293`) holds streets, bridges, plots, districts, wall, castle, buildings and `fabric` (:343). |
| Tactical maps | Any land cell is refined on demand into 64×64 squares (`crates/arda-blocks/src/pipeline.rs:15`, `crates/arda-server/src/tactical/refine_blocks.rs:93-146`) and composited from a tactical art library. Each map has a rules sidecar (`crates/arda-scene/src/sidecar.rs:37`). |
| Dungeons and caves | `arda_dungeon::generate`, with `Kind::Dungeon` and `Kind::Cave` (`crates/arda-dungeon/src/lib.rs:54-59`). |
| Serving | `arda-server` serves one world under `/v1` (`crates/arda-server/src/main.rs:1-19`, `crates/arda-server/src/routes.rs:28-76`, `crates/arda-server/src/tactical/routes.rs:37-63`). |
| Viewer | `apps/viewer` (React, Leaflet, Vite) has the views world, area, tactical, cell and editor (`apps/viewer/src/App.tsx:12`). |
| Art pipeline | `tools/art-gen` holds a 1,516-asset prompt pack in tiers 0–5 (`tools/art-gen/prompts.json` `tiers.counts`: 225/415/467/318/34/57) and a Slopify driver (`tools/art-gen/generate.mjs`). `arda tactical import` turns its output into a library (`crates/arda-art-import/`). |

- **CI configuration at `022ce7cb5707`:** the test job runs on `ubuntu-latest`, `macos-latest` and `windows-latest` with `cargo test --workspace --all-features --no-fail-fast` (`.github/workflows/ci.yml:9`, `:15`). Other jobs: MSRV `1.96.1` (`:22-23`), fmt and `clippy -D warnings` (`:32-33`), and `cargo-deny` (`:39`). `.github/workflows/` contains only `ci.yml`. `CHANGELOG.md` 0.8.0 records CI green on all three platforms. No run URL is recorded in this file for that result.

## Remaining scope

| Area | Current boundary |
|---|---|
| Richer world and area presentation | Classic and the Atlas relief/palette first pass are implemented. **Audit `022ce7cb5707`:** relief tiles draw land use, parcels, hedgerows, roads and settlement plans by metres per pixel (`crates/arda-midzoom/src/land/mod.rs:1-18`), served through `crates/arda-server/src/relief.rs:58-80`. Settlement symbols, roads, realm borders and labels exist only in the offline `arda-settle render` PNG (`crates/arda-settle/src/render/mod.rs:1-12`, `:196-245`). `arda-render` overview and Atlas exports draw no settlements or roads. **Since the stamp:** `?overlay=society|none` and `arda export --society` paint land-use tone, roads by class and tier symbols on overview PNGs, overview tiles and area PNGs (`crates/arda-render/src/overlay.rs`, `crates/arda-settle/src/render/band.rs`, `crates/arda-midzoom/src/land/export.rs`, `crates/arda-server/src/routes.rs:321-407`); gazetteer labels are drawn on overview and relief tiles (`?labels=0` opts out; `crates/arda-server/src/routes.rs:357-366`, OI-34). Realm tints and borders and forest layers are not drawn on exports. |
| Complete tactical scenes | 2026-09-25 record: the preliminary WFC archive on a 64-cell stride was retired in 1.0. **Audit `022ce7cb5707`:** on-demand refined maps with town plans, furnished interiors from the `arda-wfc` solver (`crates/arda-wfc/src/solve.rs:1-15`), a layered art compositor and rules sidecars with walls, doors and vision blockers (`crates/arda-scene/src/lib.rs:5`, `crates/arda-scene/src/los.rs:194`). Multi-storey buildings: not represented in `TacticalLayout` or the scene. The sampled stride archive no longer exists (OI-43 closed). |
| Full statistical calibration | Existing physical, drainage, boundary and determinism checks do not complete the proposed combined Horton/Hack/rank-size/sinuosity/farmland suite. `06-testing.md`, retained step 12 in `implementation.md`. **Audit `022ce7cb5707`:** still not complete. Additions since 2026-09-25: the `plains_metrics` example (`crates/arda/examples/plains_metrics.rs`), settlement and realm statistics in `stats.json` (`crates/arda-settle/src/lib.rs:131`), and wall-clock release gates marked `#[ignore]` (for example `tests/e2e_village.rs:296`, `crates/arda-refine/tests/perf.rs:30`). |
| Static model scope | Annual water is implemented. Seasons, snow storage, groundwater and dynamic floods are outside the current model; their absence does not make annual hydrology pending. Terrain samples remain 100 m apart. `crates/arda-gen/src/hydrology/annual.rs:135`, `crates/arda-core/src/coords.rs:9`. |

## Work queue

Each item keeps its 2026-09-22 plan and its audit at `022ce7cb5707`; the audit line says what is done and what remains. A dependency on a removed item means that item is done.

### OI-03 — Decide whether near-straight river reaches follow the grid axes

**Re-check 2026-10-13 (recipes 7 and 8, 11-world panel; tool `crates/arda/examples/terrain_findings`, script `run.sh` beside it, evidence in the local `features/2026-10-terrain-recheck/`).** Closed by that evidence: OI-02 (baseline delivered), OI-04 (angular shorelines absent: 5 km axis+45 chord ratio 0.90 to 1.10 against a 1.4 threshold, rotated control agrees; the residual limit is the 100 m raster, whose stairs show in plain area exports), OI-05 (rectangular basins absent: the seed-42 500×1000 km world has no lake above 14 km², the old 30,709 km² lake is gone, and no panel world has a lake of 500 km² or more). Parallel ravines are not reproduced as a lattice (highland-window peak-to-background ratio at most 13.7 against a flag at 20), as exact runs (longest 76 cells, none reaches 20 km) or as a grid-bound descent field (axis+45 ratio 1.00 to 1.01 on the whole 500×1000 km world and its 17° rotated crop). One signal stays unexplained: of 100-step (10 km) river probes, 2.3 to 2.5% stay within 600 m of a straight chord and those lean toward the axes and 45° diagonals (excess 1.2 to 1.7 over the world's own direction mix in the 200×300 and 500×1000 km worlds; at the strict 300 m tolerance only 20 to 21 of about 63,000 probes in each default-size world qualify, 2.6 to 2.8× excess). Tectonic belt edges, bay-spacing regularity and a routing control are not measured.

**Re-measure 2026-10-15 (recipe 9 against 7; same tool, seed 42).** Recipe 9 removes the suspected mechanisms: weighted-geodesic fill grading at every level and every fill, flats regraded after every fill, 1 mm kept in the implicit solve, rotated mask-noise octaves, hashed flat ties in the shared router (logic/02 §recipe 9; review 2026-10-14 3.1-3, 3.1-4, 3.2-13). Measured: MICRO 10 km probes within 600 m of a straight chord 39/1,412 (2.8%) → 28/1,137 (2.5%), axis+45° excess 1.36 → 2.23 (28 probes); 200 × 300 km 75/3,242 (2.3%) → 60/2,907 (2.1%), excess 1.17 → 1.38; at 300 m, 1 → 0 probes; whole-land descent ratio against uniform 0.99 → 0.98 (no grid lean either way). Straight reaches fall by about a tenth, but the few that remain still lean to the axes and diagonals, on samples too small to separate from chance. Not closed: the rotated-router control (step 1) is still missing, and the shared router's flat distance is still breadth-first (its ring diagonals have a single candidate; review 3.2-13 measured 30 → 19 cells on a synthetic flat).

**Depends on:** nothing. **Targets:** `crates/arda-gen/src/formation/{flow.rs,flats.rs,drainage.rs}` (D8 routing of flats is the suspected source; `logic/02` §fine-formation notes flat regrading to avoid axis and 45° lines).

1. Build the missing control: extract channels on the rotated 100 m heights with a depression-filling router (or measure the same terrain on a rotated flow grid) and run the same probes. If the excess follows the rotation it is terrain; if it stays on the grid it is the router.
2. If grid-bound, change the earliest responsible stage under a new recipe version (recipes 5 to 8 are golden-pinned), compare matched views over the frozen panel and a holdout, and keep annual accounting, basin hierarchy and determinism.
3. If terrain, record it as natural alignment and close.

**Done when:** the excess is attributed to router or terrain with a control, and either fixed under a new recipe or closed with the evidence. No diagonal-flow quota or river-count target substitutes for it.

### OI-06 — Resolve coordinates and define shared layout/scene contracts

**Audit `022ce7cb5707`: partly done.**
- **Mapping:** a 100 m cell is 64 × 64 squares (`crates/arda-core/src/coords.rs:9-11`). Working code uses 100/64 = 1.5625 m per square (`crates/arda-fields/src/geom.rs:13-15`, `crates/arda-blocks/src/pipeline.rs:15`, `crates/arda-midzoom/src/water.rs:16`). `SQUARE_SIZE_MM = 1524` remains declared (`crates/arda-core/src/coords.rs:11`).
- **Ids:** settlement ids are strings in contract 3 (`built_by`, `crates/arda-server/src/contract/mod.rs:236`).
- **Layout and scene contracts:** `TacticalLayout` with `WallSegment.tags` (`crates/arda-tactical/src/layout.rs:80-112`), `RulesSidecar` (`crates/arda-scene/src/sidecar.rs:37`), and `SceneDto` and other public bodies as JSON Schemas (`bindings/schema/`, frozen copies in `bindings/schema-baseline/v1/`), with golden responses of a canonical world for the stable routes in `contract-kit/` (2026-10-12).
- **Not represented:** multi-storey surfaces. Each `Square` has one `elevation_ft` (`crates/arda-tactical/src/layout.rs:43-58`). A bridge is a `Floor`-layer prop (`crates/arda-tactical/src/catalog.rs:258-271`) plus a sidecar `deck` flag (`crates/arda-scene/src/sidecar.rs:120-123`). Deck height travels only as `ext.deck_elevation_ft` (`crates/arda-ways/src/sidecar.rs:91-96`).

**Status (2026-09-25 record):** design and implementation open. **Depends on:** none for contract
work. **Targets:** `crates/arda-core/src/{coords.rs,objects.rs}` (`tiles.rs` was deleted with the block archive, 2026-10-13) and
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

### OI-12 — Substantially improve world-map geographical rendering

**Audit `022ce7cb5707`:**
- `arda-render` overview and Atlas exports draw no settlements, roads or land use (at the stamp; see Task 15 below).
- Relief tiles served by `arda-server` draw land use, parcels, roads and settlement plans by pixel size (`crates/arda-midzoom/src/land/mod.rs:1-18`).
- The offline `arda-settle render` draws land use, realm tints and borders, roads by class, settlement/mine/pass/peak symbols and labels over the Atlas overview (`crates/arda-settle/src/render/mod.rs:196-245`). The `arda` CLI and the server did not expose this render at the stamp; they now draw the society overlay (Task 15) but not realm tints, borders or the offline render's own labels.
- Opt-in oblique overview: `--style atlas-oblique` (`crates/arda-render/src/overview/oblique.rs`).

**September 23 continuation verified:** the authorized terrain and shoreline corrections are committed through `757b2ab`, with final integration tests, approved physical golden update, local lint/MSRV/dependency gates and two independent dry review rounds. Actual integrated small/default worlds and MICRO99 retain exact annual budgets; the full 32K default render repeats byte-identically. Current docs describe the corrected behavior. The first-pass renderer is working; broad mountain forms and near-straight river reaches (OI-03) remain terrain-naturalness work; the 2026-10-13 re-check found no large rectangular basin on recipes 7 and 8, so the overall reference-quality objective has not been declared achieved. Local evidence: `features/2026-09-23-terrain-corrections/evidence/README.md`.

**Status (2026-09-25 record):** first geographical rendering pass delivered September 22 at `342d03e55120`: earthy elevation palette, neighbor-aware shading, sea-depth colours and class-filtered per-output-pixel palette/light interpolation. The broader milestone remains open for ecological/human layers and multi-seed acceptance.
**Depends on:** existing physical data to start; OI-08–OI-11 for ecological/human
inputs; OI-13 to finalize published settlement extents; the closed OI-02 panel (`terrain_findings/run.sh`) for comparison. **Targets:**
`crates/arda-render/src/{channels.rs,overview.rs,overview/streaming.rs}`,
`crates/arda/src/export_quality.rs`; `crates/arda-render/src/atlas.rs` and `crates/arda/src/atlas.rs` now implement the relief/style context. Ecological/human overlays remain proposed.

1. Establish a separate world-scale reference, normal viewing size and comparison panel. The close-up tactical illustration alone does not define acceptable continent cartography.
2. **Delivered first pass:** consistent relief shading and natural elevation colours, using two-cell context from eight neighbors and explicit outer-world edges. Palette and lighting interpolate separately at each output pixel; water classes remain authoritative.
3. Improve coast/lake/river hierarchy, edge coverage and readability while preserving physical locations and water extents. Keep confirmed geometry corrections (OI-03) separate from shading work.
4. Add forest, farmland and built-up appearance from real generated records as producers arrive. Define scale-dependent detail and contrast so geography remains readable under human overlays.
5. Render city/town/village extents and main roads from the shared geometry. Preserve physical extent; specify subpixel symbols/minimum strokes as presentation rules rather than enlarging the saved settlement.
6. Pass human/ecological object geometry into world rendering when its producers exist. Classic overview callbacks supply `AreaCells`; Atlas callbacks supply `(AreaCells, AtlasTerrain)`. Neither supplies settlement/building/road object geometry.
7. **Delivered for Atlas terrain:** bounded neighboring context and streamed rows/256-row bands, with preserved publication and quality limits. Repeat 32K exports peaked at about 43 MiB (area) and 65 MiB (overview) on the verification host. Future overlays must preserve these bounded structures.
8. Review matched-scale before/after outputs with and without human overlays, including multiple seeds and seams. Record visible improvement, resource cost and unresolved generation limits.

**Task 15 (2026-10-13):** steps 4–6 delivered for the society layer: overview PNGs and tiles carry land-use tone, roads by class and tier symbols (`arda export --society`, `?overlay=`), streamed in the existing 256-row bands (32K overview peak 103 MiB against 94 MiB plain). Forest and ecological layers, realm tints and borders, and matched multi-seed review (step 8) remain; labels belong to the gazetteer.

**Done when:** world views are substantially richer in geographical detail,
settlement outlines/main roads are readable and correctly placed, and progress
is demonstrably more than a larger image or extra road lines.

### OI-14 — Render useful area maps with buildings and local routes

**Audit `022ce7cb5707`: done on relief tiles, not on area PNG exports.** Relief tiles draw buildings and streets from the plans at ≤ 20 m per pixel (`crates/arda-midzoom/src/land/mod.rs:9-16`, `crates/arda-midzoom/src/land/town.rs:148`). `crates/arda-render/src/carto.rs` area exports draw no buildings or local roads. Building geometry is exposed through the town-plan JSON, not through area JSON.

**Status (2026-09-25 record):** open overlays and viewing support. **Depends on:** OI-13 and relevant
OI-12 terrain styling. **Targets:** `crates/arda-render/src/{carto.rs,json.rs}`,
quality exports and public area queries; view-window/crop support is **proposed**.

1. Extend the existing cartographic path, which already receives `AreaObjects`, world origin and output scale, to draw local roads/paths and individual building outlines.
2. Define layering, clipping, line weights and minimum visible features. Preserve the world map's routes and settlement extents while revealing finer shared geometry.
3. Support settlement crops or view windows with correct physical scale. A full 51.2 km area at 8K is 6.25 m/pixel; that alone is insufficient for inspecting many small building outlines.
4. Expose building/route shapes through the versioned area JSON/object contract. Never require the consumer to trace an exported PNG to recover geometry.
5. Handle buildings/paths crossing areas and streamed bands without duplicates, gaps or clipped entrances. Reuse the canonical ownership rules.
6. Check representative village/town/dense-site crops and compare them with world and tactical views. Export successfully before interiors/furniture exist.

**Task 15 (2026-10-13):** steps 1, 2 and 5 delivered: area PNGs draw parcels, roads, building footprints and streets from the relief tiles' own geometry at ≤ 20 m/px (`export_area_with_overlay`, `arda export --area --society`), identical across area borders. Step 4 confirmed: `/v1/settlements/{id}/plan` carries building footprints and `/v1/area/.../cells` carries `road_class` and `land_use`; area JSON itself still has no building shapes (not widened). Remaining: step 3 crop window (proposal: `arda export --area ax,ay --crop x0,y0,x1,y1` in metres, rendering only that window at its own pixel size, reusing `AreaLandOverlay::paint_window`), vector building outlines below 20 m/px, and the representative-crop comparison with tactical views (step 6).

**Done when:** building outlines and local routes are legible at the declared
view scale and exactly match the shared plan later consumed by tactical generation.

### OI-16 — Refine tactical terrain, banks and stacked movement surfaces

**Audit `022ce7cb5707`: done for single-level terrain.**
- `arda-refine` refines a cell's saved fields into ground, water, banks, terraces and cliff bands, trails and scatter (`crates/arda-refine/src/terrain.rs`, `rivers.rs`, `trails.rs`, `biome.rs`).
- `arda-ways` lays roads, bridges, fords and switchbacks (`crates/arda-ways/src/plan/crossing.rs:80`, `crates/arda-ways/src/plan/switchback.rs:27`).
- Bridge decks are `Floor`-layer props with a sidecar `deck` flag (`crates/arda-scene/src/sidecar.rs:120-123`). A deck square is walked, not waded (`crates/arda-scene/src/squares.rs:219-232`). Stacked walkable surfaces: not represented, because a square has one `elevation_ft` (`crates/arda-tactical/src/layout.rs:43-58`).
- Coast/wetland behaviour: OI-41.

**Status (2026-09-25 record):** open beyond tile-family selection. **Depends on:** OI-06/OI-13;
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

### OI-28 — Measure and meet generation, transport and rendering budgets

**Audit `022ce7cb5707`:** these wall-clock gates are `#[ignore]` release gates and are excluded from CI's `cargo test`:
- a cold block PNG in under 500 ms and a cold quarter window in under 250 ms (`tests/e2e_village.rs:258-296`)
- a refined block in under 250 ms and a quarter block in under 100 ms (`crates/arda-refine/tests/perf.rs:30-54`)
- a 64×64 map at 128 ppsq in under 2 s (`crates/arda-tactical/tests/timing.rs:12-33`)
- a cached tactical PNG in under 5 ms (`crates/arda-server/tests/world_api/tactical_images.rs:122-135`)
- a warm relief tile in under 150 ms (`crates/arda-midzoom/tests/world.rs:166-183`)
- a 20k-person city in under 1 s (`crates/arda-npc/tests/performance.rs:45-63`)

Browser frame-time and INP budgets are not measured in the repo.

**Gates (2026-10-13, task 09):** `tools/release-gates.sh` runs every wall-clock gate above plus new server gates (cached cell tile, cold cell JSON and scene, cold 128 ppsq window, `AppState::open`, `World::load`, area cells JSON and bin, warm overview tile) and the two `arda-gen` benches, and writes a dated receipt; the first 1.0 receipt is in 07-operations "Recorded measurements", the budgets are ratified in logic/16 Invariant 7. Still open: browser frame-time, INP, play-code size, payload/decode/texture-memory measurements (steps 4-7) belong to DiceMaster; full-world generation (12 h), area export (60 s) and block export (5 s) targets are not gated; resource admission for population/geometry/WFC (step 3) is untouched. Admission exists for the bake lane (at most 8 waiting jobs, `--bake-result-mib` budget, start-up refusal below one set's worst case: `crates/arda-server/src/tactical/bake.rs:35`, `crates/arda-server/API.md` § `POST /v1/tactical/bake`) and the society overlay holds the 32K overview to 103 MiB peak (OI-12 Task 15).

**Status (2026-09-25 record):** admission/streamed exports implemented; full-content and interactive
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

**Audit `022ce7cb5707`:**
- Golden pins: recipes 5, 6 and 7 (`tests/golden_recipes.rs`).
- Zoom continuity and land checks: `tests/zoom_continuity.rs`, `tests/zoom_land.rs` (ignored; they need generated worlds).
- Settlement and realm statistics: `crates/arda-settle/src/stats.rs`.
- Plains diagnostics: `crates/arda/examples/plains_metrics.rs`.

The combined Horton/Hack/rank-size/road-ratio suite with acceptance tolerances does not exist.

**Status (2026-09-25 record):** physical/accounting/boundary/determinism tests exist; combined
calibration and new-content acceptance remain open. **Depends on:** each metric's
producer; starts from the OI-02 panel (`crates/arda/examples/terrain_findings/run.sh`) and grows with milestones. **Targets:**
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

**Audit `022ce7cb5707`:**
- Releases v0.2.0–v0.8.1 were cut as version-bump commits (`CHANGELOG.md`).
- At the stamp `.github/workflows/` contained only `ci.yml` and no release workflow, binary publication or image publication existed; `.github/workflows/release.yml` was added 2026-10-12 (status below).
- At the stamp the Dockerfile built and ran only `arda-cli` as uid 10001 (`Dockerfile:1-11`); superseded by the server image below (`Dockerfile:21`, `Dockerfile:34-43`).
- CLI subcommands: `generate`, `preview`, `export`, `settle`, `society build`, `tactical validate|import|placeholders|render` and `dungeon` (`crates/arda-cli/src/main.rs:36-147`).

**Local server image (2026-10-12):** the `Dockerfile` now builds `arda` and `arda-server`
(non-root 10001, `/v1/health` healthcheck, SIGTERM shutdown, `tools/docker-smoke.sh`), delivering
the local-image part of step 4. Publication is task 02's. Maintainer decision D1: the published
GHCR image carries the placeholder library only; the AI-generated library stays an opt-in local
build (`--build-context artlib=…`) and is never required. Open: the `commercial_licence` validator
rule (logic/11) is not implemented (`validate.rs` only checks a non-empty licence), so
`LicenseRef-AI-generated` is not gated.

**Status (2026-10-12):** release automation written (`.github/workflows/release.yml`, `tools/release.sh`,
`tools/package-release.sh`, `tools/smoke-binaries.sh`) and decided: four binary targets, a
placeholder-only `linux/amd64,linux/arm64` GHCR image (D1), GitHub releases drafted for the maintainer to publish,
no crates.io publication (D5, `publish = false`), `repository` corrected (D4). Proven locally: `actionlint`,
`shellcheck`, a release build with the binary smoke, packaging, `tools/docker-smoke.sh` on amd64, the `release.sh`
bump on a throwaway clone. **Waits on a real tag:** the hosted-runner matrix (macOS, Windows, native arm64), the
QEMU arm64 image build, the GHCR push and attestations, and the draft release. Full-content acceptance and the
release gates (task 09) remain open. *(2026-09-25 record: facade/CLI/Docker definition and CI delivered.)* **Depends on:** OI-01 and the capabilities
included in the release. **Targets:** current facade/CLI/Dockerfile,
`.github/workflows/`; release automation is written, untested on a real tag.

1. Audit the existing CLI/public API against newly implemented layers and queries. Add only missing entry points/options/help/errors; do not rebuild delivered generate/preview/export commands.
2. Decide supported binary targets, crate/image publication, version tags and format compatibility policy. Separate a tested local image from a published multi-architecture release.
3. Implement the planned release workflow with pinned/reproducible build inputs and appropriate artifact/version metadata. Keep credentials and actual publication in the release process.
4. Smoke-test install/load/generate/export paths on target platforms. Exercise the non-root Docker image with mounted world/input/output volumes; document UID/permissions and add serving configuration when OI-26 exists.
5. Verify packaged artifacts can read their advertised saved format and produce the same representations as source builds. Check image/binary architecture and startup errors.
6. Link actual CI/release runs and update README, operations, API and status docs from verified results. Retain historical failures as dated evidence, not current blanket status.

**Done when:** chosen release artifacts are built and verified through the
documented workflow, distribution status is accurate, and all release gates pass.

### OI-31 — Expand environments and building families after the first complete site

**Audit `022ce7cb5707`: partly done.**
- Environments: forest, woodland, wetland, coast, steppe, alpine and peak biomes (`crates/arda-refine/src/biome.rs`).
- Settlements: farms, hamlets, villages, towns and cities with district-based materials (`CHANGELOG.md` 0.8.1).
- Underground: dungeons and caves (`crates/arda-dungeon/src/lib.rs:54-59`).
- Not present: mines, ruins and multi-level sites as tactical kinds (OI-37). Dungeons are not placed in the world (OI-36).

**Status (2026-09-25 record):** later content expansion. **Depends on:** a generated, playable
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

## Open items recorded at `022ce7cb5707` (2026-10-10)

These items were added in the 2026-10-10 audit. Each records what exists and what does not at `022ce7cb5707`.

| ID | Item | Exists | Does not exist |
|---|---|---|---|
| OI-34 | Gazetteer / map-label layer (narrowed) | `society/gazetteer.json` (`crates/arda-settle/src/gazetteer/`) names seas, bays, lakes, islands, capes, forests, marshes and mountain ranges, with rivers and anchors, label paths, rank and `min_zoom`; `GET /v1/gazetteer` serves them with settlement points (`crates/arda-server/src/gazetteer.rs`); labels are drawn on overview and relief tiles (`?labels=0` opts out) and the offline PNG (`crates/arda-settle/src/render/tilelabels.rs`, `crates/arda-midzoom/src/labels.rs`; logic/15 §name-what-is-named, logic/17 §labels). | Per-realm area names (realm labels on tiles), desert, steppe and valley features, culture-region records with `anchor_m` (logic/14 §soc-regions), peak labels on tiles, and viewer support (`apps/viewer`). Oblique overview tiles carry no labels. |
| OI-35 | Thin history templates | `crates/arda-society/data/history.json`, `history_text.json`, `history_backstory.json`; linter in `crates/arda-society/src/families.rs`. Every event family has at least 4 templates, every titled family at least 3, lore at least 4; named events, backstories and causal chains exist (`logic/14-society-history.md` §soc-events). | Event kinds in `logic/14` that the code does not emit: `siege`, `bridge_built`, `abbey_founded`, `charter`, `mine_opened`. Names for peace and border-shift events. |
| OI-36 | Dungeon world placement | `site_seed(world_seed, gx, gy, salt)` (`crates/arda-dungeon/src/lib.rs:235-245`) seeds a level from any world cell through `GET /v1/tactical/dungeon?gx=&gy=` (`crates/arda-server/src/tactical/dungeon.rs:1-9`, `:40-57`). | Placement of dungeon or cave entrances in the world: no settlement, world object or society record references a dungeon. `crates/arda-dungeon/README.md:118-123` records entrance placement as a follow-up. `Params` holds only `seed`, `kind`, `width` and `height` (`crates/arda-dungeon/src/lib.rs:86-97`). |
| OI-37 | Mine and ruin dungeon kinds | `Kind::Dungeon` and `Kind::Cave` (`crates/arda-dungeon/src/lib.rs:54-59`). Mines exist as land use and settlement function (`crates/arda-settle/src/landuse.rs:42-43`, `crates/arda-town/src/function.rs:70`). Ruins exist as history text (`crates/arda-society/data/history.json` `ruin_kinds`, `crates/arda-society/src/gazetteer.rs:313`). | A mine or ruin tactical layout kind. `Kind::parse` accepts only `dungeon` and `cave` (`crates/arda-dungeon/src/lib.rs:61-74`). Multi-level maps: one level per map (`crates/arda-dungeon/README.md:127-128`). |
| OI-38 | Stepped cave walls | Cave walls are unit `WallSegment`s on every rock/floor square edge (`crates/arda-dungeon/src/grid.rs:110-140`, called at `crates/arda-dungeon/src/cave.rs:377`). The layout edge model has only `Horizontal` and `Vertical` axes (`crates/arda-tactical/src/layout.rs:69-74`). | Diagonal or curved wall geometry. Cave outlines are axis-aligned steps (`crates/arda-dungeon/README.md:129`). |
| OI-41 | Coast generation mix (recipe 9 partly delivered 2026-10-15) | Tactical coast and wetland ground is calibrated (`arda-refine coastwet`, `crates/arda-refine/src/biome.rs`, `prior.rs`; `logic/09-tactical-refinement.md` §biome-calibration). Recipe 9 (the default) classifies a cliff by the rise within 100 m of the waterline (25 m) on water of exposure 80 or more (`formation/shore.rs` `classify_recipe`, review 3.1-12), reworks only the shore band near open sea (3.1-5) and measures height above drainage to lakes and the sea (3.2-6). MICRO seed 42, recipe 7 → 9: cliff 58.6 → 27.2 % of coast land cells (`terrain_findings`; `coastwet` 58.7 → 24.8 %), sand beach 5.3 → 7.6 % (`coastwet` 4.7 → 6.9 %), rocky shore 31.5 → 62.1 %; land marshy 2.1 → 2.3 %, coastal 1.8 → 2.0 % (`coastwet --stride 4`). Seed 42 at 200 × 300 km: cliff 13.9 → 7.7 %, sand 28.0 → 30.4 %. | About a third sand on MICRO 42: 62 % of its coast cells rise 25 m or more within 100 m on water of exposure below 80 (steep sheltered ria walls), which no classification threshold makes a beach without misreporting the slope; a shoreface/beach profile in formation would. Marsh cover is unchanged (cover marsh belongs to the W6 ground-material work). Recipes 5–8 keep their bytes. |
| OI-47 | Recipe-9 residuals of the 2026-10 review (§3.2) | Recipe 9 delivers review 3.1-1 to 3.1-12 and 3.2-1, -2, -3, -4, -6, -9, -12, -13, -14, -15, -17 (logic/02 §recipe 9, logic/01 §recipe-9 climate). Width and depth are documented as mean-flow geometry (3.2-18). | 3.2-10: crust is mapped from the plates' starting positions while belts follow the drifted plates; moving the crust (and carrying belts with their plates) dropped MICRO land below the 25 % gate (seed 42: 353 → 159‰, 44‰ with advection) and was withdrawn until the centred continental mask is recalibrated. 3.2-18: lake cells still take temperature (and evaporation) at the bed, not the surface (the support solve and the fine-flow pass must change together to keep the ledger exact); deserts get no ephemeral washes below the 40 L/s channel threshold (needs an ephemeral flag on river objects so settlements do not take them for water); legacy terrain writes no `water.bin`; the Mediterranean regime has no consumer (the W6 shared biome module). The shared router's flat distance is breadth-first (OI-03). |
| OI-42 | Multi-world serving | One `--world <dir>` per process (`crates/arda-server/src/main.rs:1-2`, `:17-19`). | Multiple worlds per process, or a world id in any `/v1` route (`crates/arda-server/src/routes.rs:28-76`). Image cache-control assumes URLs that do not name the world (`crates/arda-server/src/routes.rs:157-161`). |
| OI-43 | Sampled block archive (closed 2026-10-13) | Retired: `arda generate` no longer writes `blocks/`; `World::block`, `export_block`, the 24-tile vocabulary and the zstd block format are deleted; `arda export --block` is a hidden flag that fails with the replacement hint. Older worlds with `blocks/` still load (the folder is ignored). | Nothing remains. |

## Open items recorded on `fix/settlement-realism` (2026-10-15)

Found while fixing review 3.3 (settlements, roads, realms) and measuring on a 250 × 500 km legacy world.

| ID | Item | Exists | Does not exist |
|---|---|---|---|
| OI-49 | Default-size worlds cannot be settled | `arda settle` runs on MICRO and on a 250 × 500 km world (4,203 settlements, 20 realms). | A settled 500 × 1000 km world: on the default legacy seed-42 world naming fails with `no unused stream name left in this scope (key 278)` (`crates/arda-names/src/scope.rs` `place_name`), at `main` and on this branch alike, so realm caps, notables size and head-city size at default scale are projected, not measured. |
| OI-50 | Large-city plan windows in overlays (closed 2026-10-15) | `arda-blocks` (`society::town::reaches`) and `arda-midzoom` (`land::town`) pick the plans near a window with `arda_people::town::plan_radius_for`. | Nothing remains. |
| OI-51 | Paged people queries rebuild skeletons | `World::generator` keeps 16 population skeletons; the NPC resolver uses it (review 3.3-10). | `crates/arda-server/src/npcs/page.rs` (`Inputs::generator`) still builds a skeleton per settlement per page. Server admission still charges the whole `notables.json` at twice its size although it is read lazily (`crates/arda-server/src/admission.rs` `Resident::measure`). |
| OI-52 | Settle statistics use libm | Generation in `arda-settle` is integer; the rank-size fit uses the crate's own `num::ln`. | The remaining floating point in `stats.rs` (`f64` medians, sinuosities, `town_pattern`'s `hypot`/`sqrt`) and the render path; `stats.json` is not pinned cross-platform (review 3.3-15, "22 libm sites"). |
| OI-53 | Realm balance at scale | On MICRO seeds the realms meet logic/08 Invariants 5a. | On the 250 × 500 km world the people shares are 17–111 per mille (largest/smallest ≈ 6.5, above 2.5) and the land shares exceed 4.5×, at `main` (14 realms) and on this branch (20); seat count and balancing were tuned on MICRO only. |
| OI-54 | Overlays and serving residuals of the 2026-10 review (§3.5, §3.6) | One composition per apron window serves body, scene and image; every composition is cut from the window grown by 8 squares (logic/09 §window-crop); tokens are placed globally; request-keyed ETags; decks walked at deck level; ferries span or land; fields follow the ways and the refined rivers; zoom gates cover heights and woods (changelog 2026-10-15 overlays and serving). | Floor-layer structures (a bridge's planks) take one take per structure keyed by its smallest anchor within the rendered layout, so a structure cut by an apron edge can draw two takes of a family with several (the placeholder library has none, logic/11 §seam-art 4). Two crossings laid over one another on a skewed river can leave an abutment beside the other's deck (1 climb in 225 deck ends on 40 MICRO bridges). Contending crossing stretches are clustered within the ways' 400 m search, so a chain longer than that can still differ between windows. The compose margin (8 squares) covers footprints up to 8 squares; a library with larger art needs a larger margin. On slopes above 30° the tactical surface stands up to 2.65 m under the relief's (ledges); the zoom gates run on the power-of-two-aligned MICRO world only. A settle ferry no road comes within 150 m of stays an orphan (MICRO 42: 2 of 8). Tokens stand at the building's centre, not in a room suited to the job, and rural buildings (`BuildingId::rural`) house no stored notable. |

## Scope boundaries and conditional extensions

These are explicit future choices, not hidden blockers for the map pipeline.
They must not make delivered annual hydrology or loading appear unimplemented.

| Boundary | Current disposition | Steps if the scope is activated |
|---|---|---|
| Seasons, snow storage, groundwater, dynamic floods | Outside the representative static annual model. | Specify the process/time step and required inputs; extend water/energy stores and exchanges; add versioned state/output; verify conservation and equilibrium/transient fixtures; expose the resulting seasonal gameplay/render meaning. Do not fold this into OI-08's ecological moisture by accident. |
| Finer world physical raster | Current shared terrain is 100 m; subcell object geometry and local tactical refinement are separately required. | Establish a reproduced need that OI-16 and the closed OI-04 re-check (100 m stairs only in plain area views) cannot meet; measure finer-domain cost; choose multiresolution/global policy; redefine terrain/water boundaries and formats; validate conservation, seams and resource admission. Higher PNG quality is not this feature. |
| Region-file import/front door | Retained optional design, not required for generated worlds. | Specify supported source schema/units; validate and map inputs to canonical world/layout records; define deterministic conflict/error handling; test the same exports and geometry invariants as native generation. |
| Mutable in-process game API | Arda's generated base is immutable; campaign mutations belong to DiceMaster. | First specify ownership and transaction/persistence semantics; build an overlay keyed to stable IDs/layout revisions; validate replay/concurrency/version conflicts. Do not replace the existing read-only query API just to store door state. |
| General checkpoint/resume and parallel world preparation | Current physical preparation/composition is sequential; no broad resumability promise is made here. OI-24 still must define interruption behavior for its chosen coverage strategy. | Profile actual need; identify deterministic checkpoints/partition boundaries; version and validate partial state; test interrupted/resumed equivalence and bounded resources before advertising support. |
