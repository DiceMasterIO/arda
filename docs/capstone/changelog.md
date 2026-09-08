---
generated_date: 2026-08-24
---

## 2026-09-08 - snapshot: C06 correction and feature-code cleanup
key: snapshot/2026-09-07-area-water-terrain-realism@c06-committed-cleanup

- `ed4875d`: committed the local-relief terrain correction, resource admission and regressions under the Regional detail correction rule in `logic/02-area-generation.md`.
- `tests/golden/micro-42.txt`: user separately approved the exact C06 candidate; 24 fingerprints change, 10 remain, none are added/removed. Fresh comparison passes in 44.50 s; combined broad/focused/golden evidence covers 590 passing tests.
- `b0f93f2`: preserved the 16K capability in canonical `arda-render::OverviewRaster::new_exact` and `crates/arda/examples/export_world_16k.rs`, removing the need for a copied renderer under feature docs.
- Exact overview keeps the former adapter's 16,384-axis/134,217,728-total-pixel bounds; regular overview limits remain unchanged. The migrated example reproduces the C05 7,761×16,384 PNG byte for byte in 3.718 s.
- The exporter move passes 49 renderer tests (four new), two dry code reviews, workspace formatting, strict all-target/all-feature Clippy and the official Rust 1.96.1 check. It changes no generation rule or dependency.
- `docs/map-legend.html` and `docs/map-legend.md`: saved the accepted colour swatches as an offline standalone reference and documented elevation, depth and discharge semantics; linked from the reference index and operations.
- User explicitly authorized removal of feature-local Rust and then Python. Dependency audit found no production/workspace/CI source dependency on that folder; the offline forcing-table generator already lives under `tools/`.
- Removed 419 Rust source files, 70 Python scripts, 24 source patches, spike manifests, native helpers, bytecode and 16 Cargo build caches; removed 6,449,946,322 bytes. No Rust/Python source files remain under the feature folder.
- Preserved the feature folder, all 860 PNGs and non-code evidence. The local cleanup receipt records removed paths/sizes/source hashes; the feature README marks old replay commands as historical. Unique ignored spike sources were not Git-backed.
- Refreshed factual source stamps against `b0f93f22b969`, accepted-baseline status and the area/export scenarios; retained the earlier verification entries unchanged.
- Parallel drainage, angular shorelines and large rectangular regional basins remain open. Tactical assets and NPCs remain deferred; this snapshot does not mark the overall feature complete.

## 2026-09-08 - repeated lake-district correction verification
key: fix/2026-09-07-area-water-terrain-realism@lake-district-c06-verification

- Replaced absolute-altitude fine-detail amplitude with surrounding regional height differences, capped at 90 m; sea detail, noise octaves, annual water rules and rendering remain unchanged. Implements logic/02 Regional detail correction.
- Reused the admitted regional/uplift cache with canonical outside sampling; increased initial logical-work admission from 256N to 512N without another dense allocation.
- Added translation, flat/slope/bowl, continuity, numeric-bound, cache-equivalence and pre-output admission controls; retained real boundary fixtures and fixed statistical gates while replacing a diagnostic snapshot with independent exact inflow conservation.
- Two production-source review rounds and a subsequent independent fixture review are dry. The broad run plus focused repairs cover 589 passing tests; formatting, strict Clippy, official Rust 1.96.1 and the erosion budget pass.
- Completed five fixed worlds, 38 selected JSON checks and 238 exports; repeated exports match, saved worlds remain unchanged, all annual balances close exactly and all 20,712 both-wet adjacent pairs share IDs/surfaces.
- Reported seed436342 changes from 2,653 to 186 lakes and from 5,028.39 to 2,558.01 km² wet area; fixed 50 km squares with at least 50 anchors change from 18 to zero. These observations are not lake-count quotas.
- Preserved native 7,761×16,384 world and four 4096² area PNGs; default generation took 38–44 minutes, the reported 16K export 3.576 seconds and detailed areas 0.301–0.385 seconds each.
- Refreshed architecture, models, data flow, testing, operations, open items and area-generation logic. Retained all feature state, old candidates, images and evidence by explicit user instruction.
- Parallel drainage, angular shorelines, large rectangular regional basins and unmodeled geological/groundwater processes remain open. This is corrective verification, not the overall feature completion marker.
- The separate C06 MICRO42 candidate changes 24 fingerprints, leaves 10 unchanged and adds/removes no files; exact SHA256 3e86ce4c247244b65a6aa8bb6682204b41fa91208ad18f3cd7da70f7cc04e2eb. Explicit approval, final golden test and commit remain pending; the approved C05 golden is untouched.

## 2026-09-08 - snapshot: area-water-terrain implementation
key: snapshot/2026-09-07-area-water-terrain-realism@candidate05-approved-golden

- Committed the coordinated shared terrain, annual hydrology, format-4 storage, lazy readers and physical PNG/JSON exports as `2e79ca6`; kept the toolchain CI gate and reference documentation separate.
- `tests/golden/micro-42.txt`: user explicitly approved the verified candidate05 baseline, adding seven records and changing 26; three independent saved worlds match, and the fresh golden comparison passes in 43.82 s.
- `06-testing.md`: recorded 583 passing workspace checks plus the separately passing approved golden; retained ignored controls, verification scope and open visual acceptance.
- `07-operations.md`: recorded the world-only 7761×16384 PNG, 12.39 MB, 3.670 s, and 556,592 KiB peak RSS from the isolated native renderer; production render limits and all source-world hashes remain unchanged.
- `tools/generate_water_forcing_tables.py`: offline generation reproduces checked-in Rust constants; Python is not a production build or runtime dependency.
- Folded accumulated ledger fragments on main and refreshed factual source stamps against the committed implementation.
- Documentation check reports zero source-file drift and zero unfolded fragments; retained nine missing-version metadata warnings and eight scenario-heading false positives for the logic index, with the raw report preserved locally.
- Preserved the feature folder, rendered maps, review records and prior candidate evidence locally under the existing ignore rules.
- Natural visual acceptance remains open for repetitive ravines and regional lake shapes; tactical assets and NPC generation remain deferred. This commit records current work without marking the feature complete.

## 2026-09-08 - plan: 2026-09-07-area-water-terrain-realism
key: plan/2026-09-07-area-water-terrain-realism@Q15

- `features/2026-09-07-area-water-terrain-realism/plan.md`: user approved the reviewed26-task static terrain/water implementation plan with “ok then start implementing”.
- File map: core formats and identities; terrain/shared hydrology and generation; persisted global/area data; lazy facade loading; PNG/JSON renderer and CLI; existing regression tests and required evidence tools.
- Approved specification checksum: `126d6cc76437bb6590d84d095d7ec823b5c64328`; coverage includes12 requirements and13 behavior rules.
- Task1: Install one format-4 core and annual public schema.
- Task2: Install static hydrology types and annual support kernel.
- Task3: Correct regional interpolation and coastline signs.
- Task4: Correct incision and conserve hillslope contributions.
- Task5: Supply valid canonical annual climate inputs.
- Task6: Prepare and page the complete physical domain.
- Task7: Resolve fine ocean, flats, owners and actual saddles.
- Task8: Build and sort the witnessed physical MST.
- Task9: Persist the coalesced physical hierarchy.
- Task10: Bind spills to real source membership and receiving cells.
- Task11: Aggregate canonical cells into annual physical bands.
- Task12: Normalize supported physical transfers and global lake records.
- Task13: Route exact annual balances on the fine physical tree.
- Task14: Derive catchment, channel order, drainage and HAND.
- Task15: Extract real reaches, divergent endpoints and crossings.
- Task16: Build bounded global and exported-area record indices.
- Task17: Stream the canonical prepared annual source.
- Task18: Connect the actual shared solve.
- Task19: Compose one area from immutable shared state.
- Task20: Build standalone saved area contexts.
- Task21: Stream global tables and publish only completed output.
- Task22: Admit resources and wire the real public generator.
- Task23: Load saved layers only when requested.
- Task24: Render saved physical channels and expose explicit detail exports.
- Task25: Verify the integrated candidate and frozen natural panel.
- Task26: Absorb verified contracts and retain the feature evidence.
- Preserve original main checkout and pinned geography, deterministic integer results, read-only saved exports and clean-rerun failures.
- Preserve the entire feature folder, research, prototypes and baseline permanently; no feature cleanup deletion.
- Tactical assets/WFC/gameplay schemas remain separate; no seasonal simulation or groundwater programme is introduced.
- Required integrated natural/world/project checks remain pending; golden expectations have no new modification authorization.
- Execute locally with focused independent subagents where dependencies permit; retain usage checkpoints and all evidence.

## 2026-09-08 - fix verification: area water and terrain realism
key: fix/2026-09-07-area-water-terrain-realism@terrain-correction-verification

- Installed shared rectangular terrain evolution, implicit downstream-first incision, four resolved detail octaves and a radial coast mask; retained exact controlled regressions and all previous feature evidence.
- Added conservative terrain work to generation admission; current source has two consecutive dry review rounds and 567 passing workspace tests with the protected golden expectation excluded. Golden hashes remain unchanged.
- Completed five fixed worlds, 38 selected area JSON checks and 238 saved export commands; repeat exports match and saved worlds remain unchanged. The final binary independently reproduces candidate04 MICRO42's 34 saved-file fingerprints.
- Original reported area's wet rim falls from 50.10% to 4.89%, against 4.95% current interior; 87 wet border pairs share lake IDs and levels. The continuous water-square artifact is absent in the inspected current image.
- Default global lake totals change from 11,523/8,829/13,782 to 1,298/825/2,249 for seeds 42/7/436342; all three annual budget residuals are zero. These numerical observations do not establish natural abundance or morphology.
- Updated testing, operations and open-items references; retained current world/two-area/three-tactical gallery and PNG/JSON evidence under the preserved feature folder.
- Current default driver times are 45–50 minutes under three-world concurrency, with approximately 1.762 GiB peak RSS per process. Saved default area previews/details/JSON complete below one second each; future tactical assets and browser/network delivery are not measured.
- Feature remains incomplete: repetitive drainage, rectangular coarse basins, protected golden approval and final wrap remain open. This fragment is verification history, not an implement completion marker.

## 2026-09-08 - candidate05 correction verification and detailed renders
key: fix/2026-09-07-area-water-terrain-realism@candidate05-verification

- Corrected tectonic boundary classification using widened moved-center-normal motion and incident-kind union; preserved unrelated forcing, belt profiles and the documented warped-interface approximation.
- Corrected the legacy area water helper to read actual north/west outside neighbors and all four diagonal corners while preserving canonical edge identities; fixed three-context survey retains original thresholds and measures 11/13 large matches, median mismatch two cells.
- Verified primary shared prepared data and all 34 saved MICRO42 fingerprints are unchanged by the legacy-neighbor repair; added bundle samples fit existing admitted memory/work bounds.
- Completed 583 passing workspace tests with zero failures, eight ignored controls and only the protected golden expectation filtered. Formatting, strict Clippy, isolated official Rust 1.96.1 and both existing benchmark workloads pass; two current-source review rounds are dry.
- Completed all five frozen worlds, 38 selected area JSONs and 243 generation/export commands; repeated exports agree and source worlds remain unchanged. All three default annual budgets balance exactly.
- Audited all 513 published default areas; all 21,772 adjacent pairs wet on both sides match lake ID and surface. Reported area 3,10 now has 6.51% wet rim versus 6.45% interior, compared with the prior 50.10% rim concentration.
- Recorded default driver generation times of 36m52s, 43m07s and 47m26s under concurrent load, around 1.762 GiB RSS per process. Separate terrain/water/area/export stages are retained; the host clock discrepancy is disclosed.
- Produced a detailed 4608×9728 world PNG in 2.134s through the existing renderer, verifying all 523 saved-world file hashes unchanged. Delivery contains four 4096² area renders and excludes tactical images per the latest user instruction; earlier tactical artifacts remain preserved.
- Refreshed models, testing, operations, logic and open-items references. Preserved the complete feature folder, all prior candidates, failures and rejected experiments.
- Natural visual acceptance remains open for repetitive ravines, regional rectangular lakes and tiny-stream readability; the original area 6,13 is ocean in this candidate. Goldens remain unchanged and implemented remains false. This is a verification record, not a completed-feature marker.

## 2026-09-07 - groom: 2026-09-07-area-water-terrain-realism
key: groom/2026-09-07-area-water-terrain-realism@Q12

- `docs/capstone/features/2026-09-07-area-water-terrain-realism/spec.md`: approved twelve requirements for physical channel rendering, terrain/erosion diagnosis and correction, connected lake/river accounting, deterministic persistence and measured verification.
- Approved architecture amendment: prepare detailed terrain, resolve shared hydrology, then compose independent final areas; replaces the coarse-only single-pass input contract while retaining pinned terrain and deterministic outputs.
- Approved area output choice: add 4096×4096 detail with physical channel widths and retain 512×512 previews with separately measured readability marks.
- Physical-model scope: seasonal and subsurface comparisons are required planning investigations; the production model follows their demonstrated effect and supported inputs, without preselecting added simulation complexity.
- Fixed comparison panel: default seeds 436342, 42 and 7 plus MICRO 42 and 99, with 38 area cases frozen before tuning and identical-scale overview/area evidence.
- Rejected: renderer/threshold-only completion, coarse masks as proof of fine-scale continuity, regional warping or blanket breaching as assumed remedies, and universal lake-fraction/diagonal-flow targets.
- Verification requirements: physical units, connected basin membership and mass balance, actual tile rims/corners, deterministic order independence, required project checks, and measured shared-stage scaling before plan approval.
- Out of scope: tactical and society generation, unrelated rewrites, new checkout/branch; preserve the local ignored generation-issues.md and baseline artifacts.
- Compatibility follows the concrete persistence design; incompatible changes require format major 4. Explain intended golden-output changes before updating expectations; implementation still requires plan approval.

## 2026-09-07 - groom: 2026-09-07-area-water-terrain-realism
key: groom/2026-09-07-area-water-terrain-realism@Q14

- `features/2026-09-07-area-water-terrain-realism/spec.md`: replace the earlier scientific investigation obligations with the user's clarified static VTT terrain/water scope; retain fine topology, shared identity/flux, physical exports, determinism and required project checks.
- `features/2026-09-07-area-water-terrain-realism/history/spec-scientific-scope-history.md`: preserve the original approved specification as superseded history.
- `features/2026-09-07-area-water-terrain-realism/history/PAUSED.md` and `planning-progress.md`: record explicit resume and preserve the previous research checkpoint.
- Select representative annual water semantics; seasonal persistence, dated snapshots and periodic-state accuracy are not claims of the current feature.
- Exclude scientific seasonal calendars, dynamic groundwater reservoirs, litre-level return proofs and billion-cell convergence research from required completion.
- Keep tactical WFC/assets, gameplay collision/NPC schemas and society implementation for subsequent features.
- Reuse tested terrain, topology, climate, format, loader and renderer payloads where they support the revised scope.
- Retain Capstone task-based execution and usage checks, with a review point around 50% total account usage from the observed 33% starting point.
- Implementation-plan approval remains outstanding; production code and golden expectations remain unchanged.

## 2026-09-07 - feature: 2026-09-07-area-water-terrain-realism
key: feature/2026-09-07-area-water-terrain-realism@Q15

- `features/2026-09-07-area-water-terrain-realism/`: preserve the entire feature folder and all contents after completion, as explicitly required by the user.
- Override Capstone implement Phase D step 6 and completed-folder cleanup; completion does not authorize deleting or moving this directory.

## 2026-09-07 - map: local report policy
key: map/local-report-policy@987aeca04c77

- `docs/capstone/.gitignore`: ignore only the local generation-issues.md report at the user’s request; retain the file on disk.
- `docs/capstone/00-index.md`: mark the report optional and local only, without a link implying it ships in Git.
- User authorized committing the documentation refresh and version metadata, excluding the issue report; no push requested.

## 2026-09-07 - map: version metadata
key: map/version-metadata@987aeca04c77-6.4

- `docs/capstone/01-architecture.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/02-models.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/03-conventions.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/04-data-flow.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/05-dependencies.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/06-testing.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/07-operations.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/08-glossary.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/standards.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/implementation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/open-items.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup-artifact.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/generation-issues.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/01-continent-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/02-area-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/03-block-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/04-export.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/05-load-query.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/06-society-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/07-preview.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/01-generate.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/02-world-layout.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/03-export.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/04-crate-api.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/05-docker.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/06-serve.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/README.md`: stamped capstone_version 6.4, supplied by the user.
- User supplied the installed version, resolving the prior missing-version warning without changing the skill installation.

## 2026-09-07 - map: all
key: map/all@987aeca04c77

- `docs/capstone/01-architecture.md`: Prescriptive-to-observed refresh: crate boundaries, complete command inventory and actual communication; planned serve retained.
- `docs/capstone/02-models.md`: Prescriptive-to-observed refresh: domain/working-state/DTO field tables, formats and validation; absent society types and unimplemented fields retained explicitly.
- `docs/capstone/03-conventions.md`: Prescriptive-to-observed refresh: enforced lints, counted casts/allow attributes, deterministic inputs and observed error fallbacks.
- `docs/capstone/04-data-flow.md`: Prescriptive-to-observed refresh: full batch, eager load and rendering paths; lake/WFC behavior and planned-vs-built divergences.
- `docs/capstone/05-dependencies.md`: Prescriptive-to-observed refresh: manifest/lockfile resolutions; tiny_http kept as picked but not installed, with original rationale.
- `docs/capstone/06-testing.md`: Prescriptive-to-observed refresh: structural/golden suites and ignored probes; planned statistical acceptance gates distinguished.
- `docs/capstone/07-operations.md`: Prescriptive-to-observed refresh: actual CLI, container, config and CI; serve/release automation retained as plans.
- `docs/capstone/08-glossary.md`: Prescriptive-to-observed refresh: current scales, hydrology, block sampling, vocabulary and format semantics.
- `docs/capstone/standards.md`: Migrated code-prefs.md and removed working-record provenance; preserved normative rules and left undecided domains explicit.
- `docs/capstone/implementation.md`: Repointed renamed standard and removed working-record provenance; retained build plan, status history, assumptions and constraints.
- `docs/capstone/open-items.md`: Removed working-record provenance only; existing diagnosis and deferred items were not re-reviewed.
- `docs/capstone/mockup-artifact.md`: Removed provenance from the generated wrapper; user-provided artifact body retained verbatim.
- `docs/capstone/logic/07-preview.md`: Filled the only missing implemented scenario: preview generation, eager load, overview rendering and partial-failure behavior.
- `docs/capstone/generation-issues.md`: User-requested separate diagnosis record: source behavior, two saved seed-436342 samples, earlier anisotropy measurements, unknown causes and planned society.
- `docs/capstone/.gitignore`: Initializer removed obsolete project-config ignore and added current UI preview/raster ignore rules.
- `docs/capstone/00-index.md`: Migrated root DESIGN.md, rebuilt module/topic/companion tables, indexed every final output and removed freshness columns.
- `docs/capstone/logic/01-continent-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/02-area-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/03-block-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/04-export.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/05-load-query.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/06-society-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/mockup/01-generate.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/02-world-layout.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/03-export.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/04-crate-api.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/05-docker.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/06-serve.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/README.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- Refresh triggers: all eight topic chapters were prescriptive with tracked source present; final-output provenance and the index/standards shape also required migration. No topic chapter skipped as current.
- Absent: interfaces (no implemented sibling-repository contract) and uiux extraction (no frontend); CLI/API retained mockups remain indexed.
- Preserved decisions: synchronous future HTTP, handmade deterministic simulation, lazy future loading, full tactical continuity, future society and researched dependency choices.
- Unknown: installed skill manifest unavailable, so capstone_version omitted rather than guessed; project config personal keys left unchanged, ignored for expertise resolution.
- Validation: documentation schema/link/source-pointer checks only; no world generation, Rust-suite run, statistical sweep, source change or golden re-blessing.
- Cleanup: reused five verified draft chapters from the stopped rejected copy, then removed only /home/gent/.codex/worktrees/0a95/arda; saved checkout remains on main.

# Changelog

## 2026-08-27 — fix: open-items #12 and #4 investigation
key: fix/open-items-12-and-4@2026-08-27

- `crates/arda-gen/src/continent/hydrology.rs`, `continent/bundles.rs`, `area/mod.rs` — **open-items #12** gets the mechanism it needed: the continent tier now emits lake identity (`ContinentHydrology.basin_surface` — every 1 km cell inside a filled depression carries that depression's single surface, `NO_BASIN` outside), the bundle carries it as a `basin_km` patch, and a near-rim area basin takes its lake surface from a **nearest-cell** lookup of that constant (never interpolated: the field is piecewise constant, and bilinear blurring across depression edges would reintroduce the span dependence being removed). Two fragments of one depression now agree **exactly** — measured 0 mm against the real `compose` path, where the previous rule measured 723 mm. Components are grouped 8-connected, matching `erode::fill`'s own spillover, so grouping can never split a plateau the flood treated as one water body. Nothing is persisted: `FORMAT_VERSION` stays 3 and the golden fixture is byte-identical (MICRO has no near-rim basin over a continent depression).
- Recorded rather than overclaimed, after review: two residues remain, both unobserved. No fixture has yet produced a straddling basin whose cells see a continent depression at all (0 in a 4,280 seed/seam sweep), so the exact path is proven on constructed input rather than natural data; and a fragment whose rim abuts two *different* depressions takes a max of two constants, which is span-dependent again. Where the continent tier sees no depression — sub-kilometre pits invisible at 1 km — the older bilinear rule still applies.
- **open-items #4 (value-noise anisotropy): investigated, diagnosis corrected, no fix shipped.** The recorded "~17% diagonal flow" did not reproduce: measured 23.2% post-erosion / 38.1% pre-erosion. Zeroing the value-noise detail term entirely barely moves it (21.3%), which proves the detail noise is **not** the dominant source — `coarse_height`'s own separable smoothstep-bilinear sample of the 1 km grid is. Candidates measured: per-octave domain rotation (the previously "untried alternative") 23.9%, rotation + per-octave offset 23.8%, warping the detail term alone 23.4–24.0%, warping `coarse_height`'s sample position too 28.6%. The last one visibly reduces the combs at area zoom **but was rejected on the render**: at default size it speckles the coastline, scatters noise-like micro-lakes through the interior, and weakens the trunk hierarchy — the same class of regression that killed the first attempt, so it was reverted rather than shipped. Evidence now points at terrain shape, not noise isotropy: the combs are worst on near-planar mountain flanks where no drainage convergence can form. A real fix needs the erosion budget or the relief construction revisited — a feature with a spike, not a constant to tune.
- Docs: `open-items.md` (#12 restated precisely, #4's diagnosis corrected with the full candidate table), `02-models.md` (continent lake identity), `logic/02-area-generation.md` (the Invariants amendment now describes the shared-surface rule and its residues).

## 2026-08-27 (6b65f7c6a0c0..f56314f58bb2)

Architecture-level delta for feature 03, base = the head of the previous
ranged entry.

- **New module surface:** none — feature 03 added no crate and no module. `crates/arda-gen/src/continent/mod.rs` gained the `Continent { grid, climate, hydrology }` context and `build_continent`, which replaced `&ContinentGrid` in `bundle_for` and `generate_area`. That is the one architectural change: the continent tier's outputs now flow to the area tier instead of being computed and discarded at the write site.
- **Data contracts:** `TileBundle` reached its full `logic/01` step-10 shape (entering rivers, 53×53 rainfall/regime/routing-surface patches, wind octant, west-edge moisture). `ValidationStats` gained `river_count` (`#[serde(default)]`, additive). `fill::Basin` lost its `outlet` field and `spill_cell` was deleted once outlets moved to a tile-wide second pass. `FORMAT_VERSION` stays **3** — no binary layout changed anywhere (33 B cell record, 18 B overview record untouched), and the export JSON schema is unmodified.
- **Generation pipeline:** the continent tier's reroll ladder gained a second gate (≥1 river reaching the sea), ordered after the cheap land-fraction gate. The area tier's water stage changed rule, not shape: channel initiation moved from a 300-cell catchment to 40 L/s discharge, discharge became rainfall-driven, and entering rivers seed boundary cells with upstream catchment, discharge, and a Strahler floor.
- **Rendering:** `arda-render/src/carto.rs` gained order bands (three colours, 1/2/3 px widths, floor at order 3) and a widening pass; `arda-cli`'s generate footer reports the river count. No new dependency, no dependency-direction change.
- **Test surface:** `crates/arda-gen/tests/cross_tile.rs` (new) holds the cross-tile invariants; `drainage_invariants.rs` gained a lake-outlet sweep. The golden fixture was re-blessed twice on the branch — once for the feature's cell changes, once for a defect the review loop caught before merge.

Stale-topic check: `03-conventions.md`, `05-dependencies.md`,
`07-operations.md`, `08-glossary.md` still carry the 8d3c9d9 stamp. A `sync`
run would settle them; 08 in particular now lacks entries for entering river,
climate regime at the area tier, and the discharge-based watercourse rule.

## 2026-08-27 — implement: 03-climate-driven-refinement@Q9
key: implement/03-climate-driven-refinement@Q9

- Executed all nine plan tasks on `feat/climate-driven-refinement` (base 78bbff5, head 8be0a0a, 19 commits), subagent-driven with a per-task review gate: (1) expose the continent moisture store, (2) thread a `Continent { grid, climate, hydrology }` context to bundles and areas — a byte-inert refactor gated by the golden fixture staying green, (3) derive the step-10 bundle payload and entering rivers, (4) drive area discharge and channels from rainfall and inflow, (5) clamp seam lakes to the continent spill level, (6) gate generation on a sea-reaching river, (7) band river rendering by order, (8) the cross-tile invariant suite, (9) the golden re-bless. Source touched: `crates/arda-gen/src/{continent/{climate,mod,bundles}.rs, area/{water,mod,fill}.rs, orchestrator.rs}`, `crates/arda-core/src/formats/manifest.rs`, `crates/arda-render/src/carto.rs`, `crates/arda-cli/src/main.rs`, `crates/arda-gen/tests/{cross_tile,drainage_invariants}.rs`, `tests/golden/micro-42.txt`. 228 workspace tests green; clippy `-D warnings` and fmt clean.
- **Measured outcome:** max area-cell catchment at the default 500×1000 km size rose from 2,207 km² (capped by one tile's 2,621 km² ceiling) to **50,070 km²** — 19× the old ceiling and 5× the UK-scale major-basin reference open-items #1 cited. 3,335 entering rivers seed 171 tiles; the world yields 4 major river systems at seed 42 and 8 at seed 7.
- Review loop: **5 rounds to dry** (rounds 4 and 5 consecutive dry at 8be0a0a). 12 findings confirmed — 0 Critical, 4 Important, 8 Minor — all fixed (fa95cf4, f6f113a, 8b4d1ed, 8be0a0a) or accepted-and-recorded; 0 refuted. Two Important findings were defects in the loop's *own* earlier fixes, which is the case for requiring two dry rounds. The headline catch: discharge-driven initiation made channel-less land a designed regime, exposing that `hand()` conflated "is a channel" with "no channel below", so every land cell on an arid tile persisted as **Marsh** — the opposite of desert. 18,508 MICRO land cells were wrong; `hand` now returns a `u32::MAX` sentinel, restoring what feature 01's R9 already prescribed.
- Deviations from the approved plan, each measured before overruling: the plan's seam-lake BFS membership expansion flooded 20–88% of a tile, so membership stays local per §Q5's "minimal blast radius", plus a trim of cells at/above the clamped surface (down-clamps measured on 27 of 63 near-rim basins, 26 of them producing depth-0 drops); the plan's render floor of order 4 kept only 5.58% of channel cells, so cutoffs were calibrated to 3/5/7 (17.94%) against a real default-world histogram; invariant (b) ships as "EXISTS a tile with a whole-tile increase AND EVERY seeded path increases" because the literal single-tile form is false where an interior watershed dominates; `CHANNEL_THRESHOLD_CELLS` was deleted rather than retained once it became vestigial.
- **Claim corrected, not overstated:** open-items #12 (seam-straddling basins) is **materially improved, not exactly closed**. Both sides now derive the lake level from shared continent data instead of independent local spills, but each takes the maximum over its own rim cells, so fragments with different contact spans can still differ (synthetic case 723 mm; the spec's pre-approved fallback measured *worse* at 901 mm and was not adopted). A sweep of 4,280 (seed, seam) combinations found **zero** natural straddling pairs. Exact agreement needs continent-tier lake identity, which feature 02 §Q1 placed out of scope.
- Chapters refreshed to 8be0a0a: `01-architecture.md` (the `Continent` context closes feature 02's computes-and-discards seam), `02-models.md` (`river_count`, `Cell.rainfall`'s producer, the `u16::MAX` HAND sentinel, the full `TileBundle` payload with three fields documented as awaiting the vegetation stage), `04-data-flow.md` (continent tier ends at the step-9 gate; cross-tile inflow exists), `06-testing.md` (228 tests, measured calibration, recorded fixture limits), `open-items.md` (#1 and #5 closed, #12 reclassified, #4 named as the next target).
- Absorbed per the spec's Reference impact: `logic/01-continent-generation.md` — step 10 built (crossing geometry against the exact 100 m boundary line, the merge rule taking max-of-parts order, `g(catchment)` continuity) and step 9's river gate, with the loop-back double-count and the 53–94% seam alignment recorded as approximations; `logic/02-area-generation.md` — the water stage's rainfall/discharge/40 L/s initiation, entering seeds, seam-lake clamp and trim, and the Dry-not-Marsh correction, plus an amendment noting the straddling-lake case is improved but not exact; `mockup/01-generate.md` — the validation footer's river count. All stamped `absorbed_from`.
- Left open: open-items #4 (value-noise anisotropy) is now the most visible remaining river defect — at area zoom streams still run as straight lattice-aligned combs, which is what "rivers look like fjords" describes at close range; cross-tile continuity and hierarchy are fixed, individual stream *shape* is not. Also still open: `regime_km`/`wind`/`west_moisture` ride in the bundle awaiting the vegetation stage; naming (step 8) keeps `named_river_count` at 0; statistical gates remain step 12.

## 2026-08-27 — plan: 03-climate-driven-refinement@Q9
key: plan/03-climate-driven-refinement@Q9

- `docs/capstone/features/03-climate-driven-refinement/plan.md` — nine TDD tasks: (1) `ContinentClimate.moisture` output, (2) the `Continent { grid, climate, hydrology }` context threaded through `bundle_for`/`generate_area`/orchestrator as a byte-inert refactor gated by the golden fixture staying green, (3) the step-10 bundle payload with the complete entering-river derivation (100 m boundary-line geometry, corner tie to the horizontal line, all-sea window drop, BTreeMap merge, `entering_order` floor-log map) plus an independent-derivation sum test, (4) area rainfall sampling, drainage/discharge/order seeding, u64 rain accumulation, 40 L/s initiation, compose rewiring, (5) seam-lake clamp with the pre-approved fallback path, (6) step-9 river gate + serde-defaulted `river_count`, (7) order-banded rendering with a widening pass, (8) `tests/cross_tile.rs` R12 invariants with skeleton + verbatim (b)/(e) bodies, (9) the single authorized golden re-bless with a continent-layer-must-not-drift checklist. File map spans arda-gen, one arda-core field, carto, the CLI footer, both benches.
- Review passes fixed five drafting defects before the gate: a junk assertion and an elided predicate in the crossing task (now complete code), an all-sea-window false-failure in the seam sum test, `compose`'s signature changing twice across tasks, a u16 constant-eval overflow in the skeleton, and a missing coverage row for the seed-drains-back unhappy path.
- Constraints pinned: golden green through Task 3 and red exactly Tasks 4–8; `FORMAT_VERSION` stays 3; `named_river_count` stays 0; no RNG/float/hash-order in the new sim paths (BTreeMap, integer arithmetic); exact constants (≥3 km² crossings, g ratio 4 clamp 12, Σrain×125/788,400, 40 L/s, 53×53 patches, bands 4–5/6–7/≥8).

## 2026-08-26 — groom: 03-climate-driven-refinement@Q9
key: groom/03-climate-driven-refinement@Q9

- `docs/capstone/features/03-climate-driven-refinement/spec.md` — thirteen requirements (R1–R13) making the area tier consume feature 02: a `Continent { grid, climate, hydrology }` context threaded once through `bundle_for`/`generate_area`; `TileBundle` reaching its full `logic/01` step-10 shape (entering rivers from ≥3 km² continent-tree crossings defined against the exact 100 m boundary line so both seam sides derive identical sets, seeded at the lowest edge cell with catchment×100 cells + discharge and an order floor `g(catchment) = 1 + ilog2(c/3)/2`; 53×53 rainfall/regime/filled patches; wind octant; west-edge moisture via a new `ContinentClimate.moisture` output); `Cell.rainfall` bilinear producer; discharge `Σrain × 125/788,400 + entering` L/s; channel initiation at 40 L/s replacing the 300-cell rule; edge-touching basins clamped to the shared continent filled surface; order-banded river rendering with the overview floor raised 3→4; the step-9 ≥1-sea-river reroll gate with a `river_count` stat. Closes open-items #1, #5, #12 when built. One golden re-bless authorized (§Q9); FORMAT_VERSION stays 3.
- User chose maximal scope at §Q1 (all four offered satellites in, including bundle climate fields against the YAGNI recommendation and the step-9 gate) — recorded as a deliberate override mirroring feature-01 §Q8. Rejected along the way: self-contained bundles and per-tile climate recomputation; seeding all crossings; persisted continent Strahler (1 km confluence counts misalign with 100 m orders); fbm rainfall detail; dual initiation thresholds; routing-surface max-blend; whole-tile moisture patch; log-discharge render scaling; stricter reroll gate.
- Recorded risks/approximations: a river looping out and back into its own tile double-counts upstream (accepted); seam-lake byte-agreement depends on matching boundary-contact sets — invariant (d) verifies at implement time with a pre-approved fallback rule (clamp at the basin's lowest boundary cell) requiring no re-gate.
- Out of scope as decisions: naming (step 8, `named_river_count` stays 0), human geography (step 7), vegetation-stage consumers of the new bundle fields, area temperature/moisture/forest fields, statistical gates (step 12), export JSON schema changes.

## 2026-08-26 (8d3c9d930157..6b65f7c6a0c0)

Architecture-level delta across 86 files (the drainage/continent rebuild
plus feature 02), base = oldest topic stamp since no prior ranged entry
exists.

- **New modules:** `arda-gen/src/continent/{climate,hydrology}.rs` (logic/01 steps 5–6: integer advection-diffusion climate; continent drainage tree + river extraction) and `arda-core/src/{continent.rs, formats/overview.rs}` (continent-tier types; `continent/overview.bin` codec). `arda-gen/src/continent/{plates,tectonics,coast,erode}.rs` were rebuilt in place (belt-based uplift, warped Voronoi, shelf-gradient coast, global 1 km stream-power erosion).
- **World format major 2 → 3:** `continent/overview.bin` carries real 18 B/cell records and `continent/objects.bin` a rivers section; the manifest gate became exact-match (older worlds now refused, `formats/manifest.rs`). New error variant `FormatError::DimensionsOverflow`; decoders refuse crafted headers via fully checked size arithmetic.
- **New commands/surfaces:** `arda-cli` gained `preview` and `export --overview` / `--block` (whole-world PNG, WFC block layer). No new external dependencies; no dependency-direction changes (`arda-render` still never depends on `arda-gen`).
- **New test/bench surfaces:** `arda-gen/tests/{drainage_invariants,continent_hydrology,continent_measures}.rs`, `benches/{area_erosion,continent_stage}.rs` (30 s/tile and 60 s/stage release gates); golden fixture re-blessed twice (area drainage rewrite at format 2; continent layer at format 3).
- **Area tier rewritten** (`arda-gen/src/area/**`): four-process erosion, priority-flood fill with lake emission, D8 by steepest descent, Strahler orders, network-link reaches, HAND/TWI fields — per-cell layouts changed with format 2.

Stale-topic check: `03-conventions.md`, `05-dependencies.md`,
`07-operations.md`, `08-glossary.md` still carry the 8d3c9d9 stamp; a
`sync` run would settle them (03's no-libm consequence and 08's new
continent-tier terms are the visible gaps).

## 2026-08-26 — implement: 02-continent-climate-hydrology@Q8
key: implement/02-continent-climate-hydrology@Q8

- Executed all ten plan tasks on `feat/drainage-correctness` (base c2c0d2a, head 9c48e00, 22 commits), subagent-driven with a per-task review gate: (1) continent types + overview codec, (2) continent objects codec, (3) exact-match format gate, (4) temperature/regime/sea-distance, (5) advection-diffusion rainfall, (6) drainage tree with dual loads, (7) river extraction, (8) orchestrator persistence + invariant suite, (9) 60 s stage bench, (10) FORMAT_VERSION 3 + C_NORM calibration (256→153, seed-42 land mean 799 mm/yr) + the single authorized golden re-bless. Source touched: `crates/arda-core/{continent.rs, coords.rs, error.rs, lib.rs, formats/{mod,manifest,objects,overview}.rs}`, `crates/arda-gen/{Cargo.toml, src/continent/{climate,hydrology,erode,mod}.rs, src/orchestrator.rs, tests/{continent_hydrology,continent_measures}.rs, benches/continent_stage.rs}`, `tests/golden/micro-42.txt`. 194 workspace tests green; clippy `-D warnings` and fmt clean.
- Review loop (ledger, not indexed): **7 rounds to dry** (rounds 6–7 consecutive dry at 9c48e00). 19 branch-level findings confirmed — 1 Critical (EOF-guard add-wrap at the exact `usize` window, proven reachable at dims 238,795,480 × 4,291,618,565), 5 Important (unchecked dims multiply; fabricated `expected: usize::MAX`; `width as i32` wrap decoding u32::MAX to −1; duplicated EOF guard; half-tested dims guard), 13 Minor — all fixed across five waves (97e227a, 9a21416, 6405ad9, 9e33d52, 9c48e00) or recorded/accepted (6: diffuse-step floor decomposition, row-major tie-break, climb/lapse elevation floors → absorbed as shipped rules; objects.rs ~500-line soft cap and course-helper duplication → accepted). 0 refuted; 10 task-level minors triaged accept-as-recorded. A new honest `FormatError::DimensionsOverflow` variant and a shared pub `NO_DOWNSTREAM` sentinel came out of the loop.
- Chapters refreshed to 9c48e00: `01-architecture.md` (climate/hydrology modules, formats::overview), `02-models.md` (format 3, 18 B ContinentCell, continent objects, decoder refusal posture), `04-data-flow.md` (continent stage through step 6 + persistence; the computes-and-discards seam Feature 03 must budget for), `06-testing.md` (194 tests, invariant suite, probes, bench, re-bless scope), `open-items.md` (#6 closed; #1/#5/#12 marked unblocked-by-02/owned-by-03; build step 4 partially done).
- Absorbed per the spec's Reference impact: `logic/01-continent-generation.md` — new Observed section for steps 5–6 recording the shipped rules including the four deviations (lapse and climb floors, diffuse decomposition, row-major main-stem ties) and the threshold rule `max(300, land_km2/30)` amending §Q7's assumed fixed ~3,000 km²; `mockup/02-world-layout.md` — overview.bin/objects.bin rows real since format 3. Both stamped `absorbed_from`.
- Also repaired: `docs/capstone/.gitignore` carried the legacy `changelog.md` ignore line, silently untracking this ledger since 435a6fe; line removed so the ledger follows `docs_in_git: commit` again.
- Left open for Feature 03 (not groomed): bundle entering rivers + climate fields, area `Cell.rainfall`, cross-seam basin fill, rainfall-driven discharge and channel initiation; plus the recorded seam note that `generate_world` discards the climate/hydrology structs after persisting.

## 2026-08-26 — plan: 02-continent-climate-hydrology@Q8
key: plan/02-continent-climate-hydrology@Q8

- `docs/capstone/features/02-continent-climate-hydrology/plan.md` — ten TDD tasks with full test and implementation code: (1) continent types + `overview.bin` codec, (2) `continent/objects.bin` rivers codec with its own `ARDACOB\0` magic, (3) exact-match format gate replacing the newer-only check at `manifest.rs:94`, (4) temperature/regime/BFS distance-to-sea, (5) advection-diffusion rainfall, (6) drainage tree reusing `erode.rs`'s `fill`/`accumulate` made `pub(crate)` with dual catchment/discharge loads and the routing surface exposed for feature 03, (7) river extraction with the scaled threshold and largest-catchment main stems, (8) orchestrator persistence + the R7 invariant suite, (9) 60 s continent-stage bench mirroring `area_erosion.rs`, (10) `FORMAT_VERSION` 3, `C_NORM` calibration, R8 measurement probes into `spike-report.md`, and the single authorized golden re-bless. File map: 7 arda-core paths, 8 arda-gen paths, the golden fixture, the spike report. Coverage table maps all 11 requirements, every behavior rule, all 7 unhappy paths, and the global constraints to named tasks.
- Review passes fixed three drafting defects before the gate: rainfall conversion pass-count-normalized so one calibration serves 10-wide unit grids, MICRO, and the default world (spec's Behavior formula amended accordingly, §Q3/§Q4 targets unchanged); `as usize` casts replaced with `usize::try_from` in the orchestrator snippet; the Horton probe's stream-count array sized from the observed maximum order instead of a fixed cap.
- Constraints pinned: golden fixture touched only in task 10 (§Q6 sign-off); golden expected red between tasks 8–10 with per-crate test commands named; `stats.named_river_count` stays 0 (§Q1 deferral); no RNG anywhere in the new sim paths; `TileBundle`/area/render files absent from the file map by design (R11).

## 2026-08-26 — groom: 02-continent-climate-hydrology@Q8
key: groom/02-continent-climate-hydrology@Q8

- `docs/capstone/features/02-continent-climate-hydrology/spec.md` — eleven requirements (R1–R11) building `logic/01` steps 5–6 plus real persistence: uniform-westerly advection-diffusion rainfall (fixed 1.5×width passes, 12.5% diffusion, recharge over water, base + orographic release), band-linear temperature with 6.5 °C/km lapse and 300 km continentality, regime by band position + elevation, one final fill+route on the post-erosion 1 km surface with dual catchment/discharge accumulation, river objects above a land-scaled threshold `max(300, land_km2/30)`, an 18 B/cell `overview.bin` codec plus an `objects.bin` rivers section at `FORMAT_VERSION` 3, five gated structural invariants, measured-not-gated climate stats, Horton on seeds {1,7,42,99} discharging spike S1, a ≤60 s continent-stage bench, and a single end-of-feature golden re-bless with sign-off recorded at §Q6.
- Rejected and why: rainfall-coupled erosion (reorders logic/01's step sequence, risks measured hypsometry); 4 km routing (courses quantize, disagrees with the 1 km relief areas sample); single upwind sweep and graded/warped wind fields (user chose the softer scheme; extra calibration axes, noise in a sim path); Köppen-lite regimes (amends logic/01 §Q6's recorded rule); heightless overview.bin and unversioned sidecar (readers re-run the stage; version skew moves into code); fixed 3,000 km² threshold (MICRO fixtures get zero rivers); gating rainfall bounds now (fits thresholds to current output).
- Out of scope as decisions: steps 7–8, step-9 gate wiring, every `TileBundle` change, render/overlay work, continent-tier lakes, area `Cell.rainfall` — all owned by Feature 03 or build-order step 4's remainder. Open-items #6 closes here; #1, #5, #12 are unblocked, not closed.
- Groom's staleness pass also stamp-refreshed `02-models.md` and `04-data-flow.md` (separate sync entry below).

## 2026-08-26 — sync: 02-models.md+04-data-flow.md
key: sync/02-models+04-data-flow@c2c0d2a

- `docs/capstone/02-models.md`, `docs/capstone/04-data-flow.md` — stamp refresh only, performed by groom's staleness pass for feature 02. Both chapters' Observed sections were written during the drainage/continent rebuild but their `generated_at_commit` stamps still read 8d3c9d9 (pointer drift). Content verified against HEAD: `FORMAT_VERSION` is 2 (`formats/mod.rs:13`), the D8/fill/outlet behaviour matches `area/water.rs`, the bundle carries edge heights only (`continent/bundles.rs`), and cross-tile inflow has no producer. Stamps bumped to c2c0d2a; no body text changed.

## 2026-08-26 — docs: open-items
key: docs/open-items@2026-08-26

- `docs/capstone/open-items.md` — single index of everything known-incomplete, each with its evidence and owning code: four blocking-realism defects (cross-tile rivers, WFC noise, unreachable relaxed fallback, value-noise anisotropy), four stand-ins awaiting a producer, four calibration items held open, the two ungroomed successor features, and the unbuilt build-order steps. Also records two findings as *not* defects so they are not re-investigated.

## 2026-08-26 — build: code@Q5 (drainage and continent)
key: build/code@Q5-drainage

- `crates/arda-gen/src/area/**` — rewrote the area stage against the artifact's own Relief and Water sections, which `logic/02` designates normative and which had not been read when the stage was first built. Four coupled erosion processes (uplift, stream-power incision, hillslope creep, 35-degree talus collapse) replace one; priority-flood filling with lake emission; D8 by steepest descent rather than steepest drop; true Strahler order; reaches as network links carrying `feeds` and `ends`; HAND, TWI, slope, aspect, and floodplain-driven marsh cover. Constants taken from the artifact rather than invented: 300-cell channel threshold (its 3 km² ≈ 40 L/s), `w = 4·sqrt(Q)` channel width, the 1 m / 2.5 m / 2–15 m floodplain bands.
- `crates/arda-gen/src/continent/**` — rebuilt. Plate count scales with domain area; crust follows position; Voronoi lookup is domain-warped; uplift spreads across orogenic belts and plates drift so belts migrate; uplift is normalised to a target relief and subsidence floored; the crust field is blurred into a shelf and blended with a centred mask; noise applies on both sides of sea level; and coarse erosion now runs globally on the 1 km grid.
- `crates/arda-render/**`, `crates/arda-cli/**` — hypsometric palette replacing a linear ramp that saturated to white above 2,040 m and hid a 118 m plateau; whole-world overview render; `preview` subcommand; `export --overview` and `export --block`, the latter exposing the WFC tile layer which had no command at all.
- `crates/arda-gen/tests/drainage_invariants.rs` — eight global invariants, the class of assertion whose absence let a stage with 93% of catchment in interior pits pass 113 tests. `crates/arda-gen/benches/area_erosion.rs` gates erosion at 30 s/tile; measured 2.17 s.
- Rejected along the way, with reasons: isotropic gradient noise (blocky coastlines, reverted — the lattice anisotropy it would fix is recorded as a known limitation, ~17% diagonal flow against 50%); smoothstep belt profiles (flat-topped, made plateaus not ranges); heavier hillslope creep and lower uplift as lake remedies (both made lakes worse by reducing the relief that drives incision).
- Left open: rivers still cannot cross tile boundaries, so no catchment can exceed one tile's 2,621 km² — a UK-scale major basin is 9,385 km², 3.6x larger, and the continent cannot yet produce one. `Cell.rainfall` still has no producer, so discharge and channel initiation remain catchment-driven stand-ins. The tactical WFC layer converges but produces salt-and-pepper noise: its adjacency rule is too permissive for structure to emerge.
- `format_version` is 2: `objects.bin` record layouts changed for `feeds`/`ends` and lake depth/outlet. Golden fixture re-blessed.

## 2026-08-25 — groom: 01-area-drainage-correctness@Q15
key: groom/01-area-drainage-correctness@Q15

- `docs/capstone/features/01-area-drainage-correctness/spec.md` — twelve requirements (R1–R12) closing ten of the thirteen findings from the river pipeline audit, plus the process finding that no global drainage invariant existed. Chosen approach: priority-flood depressions to a *routing* surface with stored heights left raw; lakes emitted above 100 cells and 2,000 mm (~20/tile, measured from 859 basins/tile with median 2 cells and 208 mm); true Strahler order; D8 by drop÷distance via integer cross-multiplication; reaches as network links partitioning channel cells exactly once; outflow routed against the bundle's real neighbour heights; two-pass erosion with stream-power carving and magnitude-capped droplets detailing; HAND plus a fixed-point TWI with no libm in any sim path.
- Rejected and why: breach/carve depressions (mutates stored relief, ~2,048 gorges/tile, breaks the pinned-edge guarantee at seams); fill without lakes (leaves `Lake` producer-less); Shreve magnitude (Horton's ratio is defined on Strahler orders); droplet-only erosion (permitted by §Q4 with explicit ordering, but its network statistics are not guaranteed to hit the Horton/Hack/sinuosity bands, and it would run a different erosion model from the continent tier); diffusion-only smoothing (does not concentrate incision); deduplicating head-to-terminus courses (coverage would depend on iteration order); unconditional edge outlets and off-tile-as-sea-level (water leaves uphill, or every edge becomes a fabricated cliff); bespoke banded wetness (not a recognised index); bringing step 12's statistical gates forward (their thresholds would be fitted to current output).
- Scope reopened at §Q8 after the gate: "no out of scope" superseded §Q1's deferrals. Erosion and the derived river fields moved **into** this feature — both are computable from relief and drainage, and shelving them was an error. Three items stayed out with verified data dependencies rather than preferences: cross-tile inflow, rainfall-driven discharge, and climate-driven channel initiation all require producers that do not exist (`Cell.rainfall` has no writer anywhere in `arda-gen`; the continent stage stops after step 4). They become **feature 02** (continent climate and hydrology, `logic/01` steps 5–6, with spike S1) and **feature 03** (climate-driven refinement plus inflow), neither yet groomed.
- Two decisions were recorded late, at §Q12 and §Q13: the D8 rule change and sea-cell hygiene had been carried in the gate summary and asserted by a test, but never given decision entries, so no requirement could trace to them. Two further decisions were surfaced only by a coverage check: §Q14 caps both erosion passes at 30 s/tile enforced by a `criterion` bench (rejected: 2 min/tile, which would breach step 12's 12 h batch ceiling once blocks are per-cell), and §Q15 authorises re-blessing `tests/golden/micro-42.txt` once as the final task, which `code-prefs.md` §Q9 requires be explicit.
- Left open: the lake thresholds and every erosion constant were measured or chosen on pre-erosion noise relief and are re-derived when the statistical gates land at step 12; a basin straddling a tile seam still fills independently on each side and may reach different spill levels until feature 02 supplies a continent drainage tree.

## 2026-08-25 — doctor: ledger catch-up
key: doctor/torn-writes@2026-08-25

Seven stages carried a `formalized` or `plan_approved` marker with no
matching changelog key — torn writes under core.md's ledger rule. The
entries below are reconstructed from each interview's recorded
decisions; no stage was re-run. They were written late, so read them as
a summary of the decision record rather than a contemporaneous receipt.

## 2026-08-25 — build: code@Q5
key: build/code@Q5

- `crates/arda-core/**` — coords/fixed/rng/error/config/cell/objects/tiles plus `formats::{manifest,cells,objects,blocks}`, the workspace's only byte codec. `cells.bin` row pinned at 33 bytes carrying the full artifact field list (skeleton fills relief+water only, rest default); `objects.bin` is a tagged section container whose unknown kinds are skipped, so step 5's settlements/roads/crossings/passes are additive within format major 1. Rejected: `unsafe` transmute for row layout (code-prefs §Q1), a dev-dependency temp-dir crate (hand-rolled instead).
- `crates/arda-gen/**` — hand-rolled integer value noise; continent stage (Voronoi plates, kinematic-lite 20-step coupled tectonics, isostasy, coast, 4km→1km upsample); tile bundles; area relief+water; 24-tile WFC block fill; orchestrator with rayon fan-out. Deferred to step 4: climate, hydrology objects, human geography, naming, and the full 100-step tectonics loop behind spike S1. Deferred to step 5: the five remaining area stages. Deferred to step 6: the 200+ tile vocabulary behind spike S2.
- `crates/arda-render/**`, `crates/arda/**`, `crates/arda-cli/**` — symbolic block PNG, cartographic area PNG, versioned JSON (schema_version 1); `World`/`Area` facade; `generate`/`export` subcommands.
- `tests/golden_world.rs` + `tests/golden/micro-42.txt` — 26-file blake3 fingerprint, wired into the 3-OS CI matrix as the §Q4 gate. Not yet observed green on macOS/Windows — only Linux has run.
- Divergences from `05-dependencies.md`, each needing a chapter update: `blake3` promoted from dev/tooling to an `arda-core` runtime dependency (the subseed derivation needs it); `thiserror` added workspace-wide (code-prefs §Q4 mandates thiserror-style derives, the chapter names no error crate); `anyhow` added to `arda-cli` only (code-prefs §Q4 permits it there).
- Divergence from `implementation.md`: its orchestrator sketch omitted `logic/01` §Q9's validation reroll. Added — seed 43 fails the land-fraction gate at attempt 0 and is rerolled deterministically, ≤5 attempts.
- Known-weak, left open: area drainage does no depression filling, so channels emerge as short disconnected fragments rather than networks reaching the sea. The Horton/Hack/rank-size gates that would catch this are build-order step 12. Blocks are sampled on a 64-cell stride, not one per land cell. Areas and blocks load eagerly, not lazily as `logic/05` specifies. All three marked in-code.

## 2026-08-25 — build: plan@Q5
key: build/plan@Q5

- `docs/capstone/implementation.md` — approved build plan: workspace layout, three load-bearing seam sketches (subseeding, stage purity, pinned edges), and a 13-step build order with per-step verification. Five late decisions folded in from this stage's own interview: `serve` (Q1), cost-distance realms (Q2), buildings (Q3), SRD 5.1-style NPCs (Q4), tile attributes plus a POI layer (Q5). Deferred as D9 and explicitly not planned: tileset-manifest renderer beyond a stub, `--memory` tuning past an areas-in-flight cap, region-file front door, runtime in-process game API.

## 2026-08-25 — stack: all@Q8
key: stack/all@Q8

- `docs/capstone/05-dependencies.md` — eight capabilities picked: `zstd`, `png`, `serde`/`serde_json`, `rand_chacha` (ChaCha8), `rayon`, `clap` v4, hand-rolled Q-format fixed-point, and hand-rolled noise/erosion/hydrology/WFC. Dev/tooling: `criterion`, `blake3`, clippy/rustfmt, `cargo-deny`. Rejected: the `fixed` crate, in favour of hand-rolled Q-format newtypes; any third-party crate for sim arithmetic (code-prefs §Q2 forbids it). Zero paid services.

## 2026-08-25 — code-prefs: all@Q9
key: code-prefs/all@Q9

- `docs/capstone/code-prefs.md` — nine normative domains: `deny(unsafe_code)`/`deny(missing_docs)` with unwrap banned outside tests; commodity adopted but determinism-critical code hand-rolled; data-oriented functional core with traits only when a second implementation exists; per-crate thiserror-style enums with `anyhow` confined to the CLI; package-by-stage layout; test-first with no mocks ever; rustfmt defaults and clippy `-D warnings`; conventional commits; and agent rules forbidding ambient randomness, wall-clock time, and hash-iteration-order dependence in sim paths.

## 2026-08-25 — architecture: all@Q8
key: architecture/all@Q8

- `docs/capstone/01-architecture.md`, `02-models.md`, `03-conventions.md`, `04-data-flow.md`, `06-testing.md`, `07-operations.md`, `08-glossary.md` — five-crate workspace with a one-way dependency direction and `arda-render` barred from depending on `arda-gen`. The defining decision is §Q4: cross-platform bit-exact determinism, a one-way door binding every later stage to integer/fixed-point sim arithmetic and a counter-based PRNG keyed `(seed, tier, stage, coords, attempt)`. Rejected: same-binary-only determinism, statistical-only determinism, FlatBuffers/Cap'n Proto/Arrow, SQLite, a plugin architecture. Risks logged R1–R4, with spikes S1 (tectonics plausibility) and S2 (WFC convergence) gating build-order steps 4 and 6.

## 2026-08-25 — design: skipped
key: design/all@skipped

- No `docs/capstone/design/` written. The product has no human-facing UI — surfaces are `[api, cli]` — so the stage was formalized as `skipped: no-ui` rather than producing empty chapters. Revisit only if a UI surface is added.

## 2026-08-25 — logic: all@Q16
key: logic/all@Q16

- `docs/capstone/logic/01-continent-generation.md` … `06-society-generation.md` — six scenarios pinned trigger by trigger: continent (plates, coupled tectonics, coast, climate, hydrology, human geography, naming, validation with ≤5 subseeded rerolls, tile bundles), area, block (WFC ≤8 retries then a marked relaxed fill), export, load-query, and society. The load-bearing invariant across all six: a tile bundle depends on coarse data and the seed only, never on any area's fine output, which is what lets areas generate in any order. Society was added after the other five, which is why the interview's scenario checklist needed today's catch-up.

## 2026-08-25 — mockup: all@Q22
key: mockup/all@Q22

- `docs/capstone/mockup-artifact.md` plus `docs/capstone/mockup/01-generate.md` … `06-serve.md` — the product defined across six surfaces: the `generate` batch, the world directory layout, `export`, the crate API, docker, and (added at the build gate) `serve`. Scope decisions recorded: offline batch preparation only, no runtime or streaming API in v1, worlds disposable and regenerable from a seed, and ~61 GB at the default continent size accepted.
