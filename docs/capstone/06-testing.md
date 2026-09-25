---
generated_at_commit: 14a70b9144dd
generated_date: 2026-09-25
content_hash: 37c2cfb554d8
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/src/**", ":(top)crates/*/tests/**", ":(top)crates/*/benches/**", ":(top)tests/**", ":(top).github/**", ":(top)crates/*/examples/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22, features/2026-09-23-terrain-corrections@2026-09-23
---

# Testing

Latest scoped Linux verification passed 250 core/render/facade tests, 12 source/composition tests and 3 fine publication tests; all-target workspace Clippy with warnings denied, formatting and diff checks passed. Recipe-2 source and all 35 small-world files repeat exactly; every saved cell and continent node matches the canonical field. Annual water closes at 11,546,349,420,000 L with zero outer-domain export. Normal Atlas area/overview exports now read fine windows; numerical seam and water-class tests accompany actual 4K exports. The user subsequently accepted the appearance. Full seed-42 500×1000 km qualification passes all 44,826,624 exported heights, 500,000 continent heights, receiver topology, saved-table decoding and exact annual closure at 159,795,615,670,000 L; seeds 43/44 pass the same small-world checks. The 15,522×32,768 actual export and separate overview/highland inspection are complete. All 523 original files remain unchanged. [Final evidence and explicit limits](features/2026-09-23-terrain-corrections/evidence/accepted-look-qualification/README.md). [Current evidence and limits](features/2026-09-23-terrain-corrections/evidence/fine-atlas-integration/README.md).

Earlier Linux verification: full workspace run 730 passed, 8 ignored (862.18 s); subsequent final fine-input/publication checks 16 passed; all-target workspace Clippy with warnings denied and formatting passed. These results do not imply cross-platform CI, MSRV or dependency-audit completion.

The opt-in complete-world path has an exact fresh-process replay: all 35 files of seed-42 102×204 km attempt 0 match, including the canonical source copy. Independent readback verifies all 2,097,152 exported cell heights and 20,808 continent heights against the fine file. Continent receivers are in-bounds and acyclic, linked land catchment/discharge are nondecreasing, saved global tables validate, and the annual ledger closes exactly at 11,449,890,620,000 L. These recipe-1 numerical/structural checks predate fine Atlas integration; the inherited flat-region limitation is documented in the accepted final delivery. [Complete-world verification](features/2026-09-23-terrain-corrections/evidence/fine-world-integration/verification.md).

Earlier source qualification remains preserved: native integer filter/oracle parity, fixed-gain physical scale, continuous macro gate, full world seed identity, and exact saved-file render replay. The single unchanged-erosion handoff trial is negative visual evidence: regular deep incision and smooth faces reappear, so it is not part of the opt-in path. [Source persistence](features/2026-09-23-terrain-corrections/evidence/fine-terrain-persistence/README.md), [evolution comparison](features/2026-09-23-terrain-corrections/evidence/spectral-evolution-handoff/README.md).

> The previous exact-HEAD CI result is recorded in [current status](open-items.md). Linux/macOS,
> lint, MSRV and dependency checks passed; Windows failed a golden-text
> LF/CRLF comparison with matching normalized fingerprint entries. Historical
> successful measurements below do not mean the latest CI run was fully green.
> The current inventory was refreshed from source on September 22; no tests
> or exports were rerun during that earlier documentation refresh. The Atlas
> feature has fresh local Linux checks and real saved-world exports below;
> no new Windows CI result is claimed.

## Layout

| Suite | Evidence | Coverage |
|---|---|---|
| Core and renderer units | `crates/arda-core/src/`, `crates/arda-render/src/` | Coordinates, checked widths/volumes, format-4 records, physical channel coverage, point termini, halo joins, depth shading and malformed-input refusals. |
| Physical routing and storage | `crates/arda-gen/src/hydrology/`, `crates/arda-gen/src/orchestrator/` | Actual plateau receivers, marine connectivity, witnessed saddles, MST/hierarchy, equal-height events, paged backends and resource failures. |
| Annual water and fine flow | `annual_aggregation_tests.rs`, `annual_support_tests.rs`, `annual_transfers_tests.rs`, `fine_flow_tests.rs`, `flow_metrics_tests.rs` under `crates/arda-gen/src/hydrology/` | Hand-calculated annual source/loss balances, strict positive-depth lake identity, marginal shoreline loss, signed physical transfers, channel order, catchments, HAND and eight-way confluences. |
| Combined public pipeline | `crates/arda-gen/src/orchestrator/shared_solve_tests.rs`, `entrypoint_tests.rs`, `publication_tests.rs` | Real prepared terrain through shared solve, final index, area/global records and actual codecs; pre-output admission, typed failures and manifest-last publication. |
| Local area diagnostics | `crates/arda-gen/tests/drainage_invariants.rs`, `cross_tile.rs` | The retained local `area::generate_area` API: coarse inflow derivation, local filters, drainage, climate, determinism and seam fixtures. These tests do not define published shared lake behavior. |
| Continent hydrology | `crates/arda-gen/tests/continent_hydrology.rs` | Persistence, ocean-reaching paths, courses, catchment/discharge and determinism. |
| Exploratory measurements | `crates/arda-gen/tests/continent_measures.rs`, `anisotropy_probe.rs` | Ignored rainfall/Horton and directional probes; not default statistical acceptance gates. |
| Facade/CLI | `crates/arda/tests/round_trip.rs`, `area_exports.rs`, `crates/arda-cli/tests/cli.rs`, `crates/arda/src/world.rs` | Round trips, lazy bounded loading, exports, malformed dimensions/layers, partial-world refusal, sampling and command refusal paths. |
| Golden world | `tests/golden_world.rs:69` | Shared initial MICRO fingerprint, approved-file comparison and an independently generated second world for repeatability; root Cargo.toml declares the target explicitly. |
| Image quality and streamed output | `crates/arda-render/src/quality.rs:84`, `crates/arda-render/src/overview/streaming.rs:302`, `crates/arda-render/src/channels/tests.rs:59`, `crates/arda/tests/area_exports.rs:100`, `crates/arda/src/export_quality.rs:124` | 512–32768 parsing/defaults/aspect ratio, arbitrary-size area coverage, decoded streamed/buffered pixel equality across area/band seams, 32K admission and selected coverage windows, unchanged saved input, repeat exports and failure-safe final PNG publication. |
| Atlas presentation | `crates/arda-render/src/atlas/tests.rs`, `crates/arda-render/src/carto.rs`, `crates/arda-render/src/overview.rs`, `crates/arda-render/src/overview/streaming.rs`, `crates/arda/tests/area_exports.rs`, `crates/arda-cli/tests/cli.rs` | Exact pixel-center and mixed-axis kernels, planar eight-neighbor edge/corner relief, saved-wetness land tint and unchanged water ownership, buffered/streamed overview pixels across unequal partitions and 256-row bands, missing/corrupt neighbor refusal, Classic byte compatibility, CLI style routing and pre-I/O refusals. |
| Release budgets | `crates/arda-gen/benches/area_erosion.rs:20`, `continent_stage.rs:17` | One area's erosion must fit 30 s; the default coarse continent stage must fit 60 s. These are not full-world deadlines. |

The public generation authority is canonical terrain preparation followed by one
shared annual solve and immutable area composition. Local diagnostic fixtures
remain useful without making their old lake thresholds or coarse inflows rules
for saved worlds. The current steps and invariants are in
[area generation](logic/02-area-generation.md),
with the call sequence in
[orchestrator.rs:406](../../crates/arda-gen/src/orchestrator.rs:406).

## Commands and CI

| Check | Command |
|---|---|
| Formatting | `cargo fmt --all --check` |
| Strict lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Workspace | `cargo test --workspace --all-features` |
| Explicit golden/repeat gate | `cargo test --test golden_world -- --nocapture` |
| Doctests independently of a failed integration target | `cargo test --workspace --doc --all-features` |
| Required MSRV as pinned on 2026-09-08 | `cargo +1.96.1 check --workspace --all-targets --all-features --locked` |
| Dependency policy | `cargo deny check` |
| Erosion budget workload | `cargo bench -p arda-gen --bench area_erosion -- --test` |
| Coarse continent budget workload | `cargo bench -p arda-gen --bench continent_stage -- --test` |

CI runs the full workspace suite on Linux, macOS and Windows. Both golden-world
tests are included in that command; CI does not invoke them a second time.
Ubuntu also runs the pinned Rust 1.96.1 MSRV check, formatting, strict Clippy and
cargo-deny. The MSRV pin records the latest-stable-minus-two policy for this dated
revision; it is not an automatically advancing expression.
Source: [ci.yml:5](../../.github/workflows/ci.yml:5),
[Cargo.toml:49](../../Cargo.toml:49) (`profile.test` uses optimization
level 2), [deny.toml](../../deny.toml).

A local `stable` alias does not update an installed toolchain automatically.
Before local verification, run `rustup update stable` and check `rustc --version`
and `cargo clippy --version` against CI. The 2026-09-08 push exposed Clippy
diagnostics on CI's Rust 1.98 that the local Rust 1.91 installation did not report.
An isolated installation of the same CI version is also suitable for verification.

Filesystem regressions compare native paths rather than assuming `/` separators.
The oversized-area-layer test uses real files with small injected byte caps:
exactly-at-cap valid layers load, and one additional byte triggers the same
metadata admission check used with production caps. It does not depend on sparse
file allocation or create a terabyte-sized file on Windows.

The golden test has an explicit `ARDA_BLESS` write path. Running ordinary checks
does not authorize replacing expected bytes. Candidate05 and the subsequent
regional-detail candidate06 baseline were separately approved and committed.
Reproducibility and geographic/visual acceptance remain separate checks.
Source: [golden_world.rs:79](../../tests/golden_world.rs:79).

The facade and CLI integration suites each generate their seed-42 MICRO fixture
once per test executable. They retain an immutable snapshot of its files in
memory and restore separate temporary directories for consuming tests. Tests
that remove manifests or attempt writes cannot alter another test's fixture.
The snapshots are rebuilt on every test invocation, not read from a persistent
cache. The facade repeat-generation test still creates an independent second
world, the alternate-seed test generates seed 43, and CLI preview performs its
own generation. The two golden tests share their first generated fingerprint
and independently generate the comparison world for repeatability.

For a completed unfiltered CI test job, this reduces successful full-world
generations from 27 to 8 per platform (81 to 24 across the three-platform matrix):
facade round-trip tests 11 to 3, CLI tests 9 to 2, golden tests 3 to 2, the unchanged
continent persistence test 1, and removal of the extra three-generation golden
invocation. These are execution counts, not a promised percentage reduction in
CI wall time. Refusal tests, coarse/area-only tests and all test cases remain.

## Doubles

Pure fixtures and temporary directories are used; no mocking framework is declared
in `crates/*/Cargo.toml`. Small complete-grid tests invoke actual routing, disk
stores, hierarchy, annual solver and codecs rather than copied algorithms.
Capacity tests admit a workload and exercise one-under refusals; corrupted-store
fixtures test typed validation and source chains. Source:
[annual_integration_tests.rs](../../crates/arda-gen/src/hydrology/annual_integration_tests.rs),
[shared_solve_tests.rs](../../crates/arda-gen/src/orchestrator/shared_solve_tests.rs),
[generation_limits_tests.rs](../../crates/arda-gen/src/orchestrator/generation_limits_tests.rs),
[error_chain_tests.rs](../../crates/arda-gen/src/orchestrator/error_chain_tests.rs).

Forcing checks use checked-in integer climatology and exact annual arithmetic.
Twelve monthly evaporation depths are summed to the annual support cost; no
seasonal storage/calendar convergence is part of this model. The offline table
generator is maintenance tooling, not a production-build or runtime dependency.
Source: [forcing.rs](../../crates/arda-gen/src/hydrology/forcing.rs),
[annual_aggregation.rs:53](../../crates/arda-gen/src/hydrology/annual_aggregation.rs:53),
[generate_water_forcing_tables.py:1](../../tools/generate_water_forcing_tables.py:1).

## Earlier feature verification — 2026-09-08, candidate02

| Evidence | Result and boundary |
|---|---|
| [Workspace log](features/2026-09-07-area-water-terrain-realism/verification/workspace-tests.log) | Earlier implementation run: 540 passed, 1 failed, 5 ignored. The sole failure was the unchanged old golden fingerprint; repeat-generation equality passed. This is not a claim that the latest full suite passed. |
| [Formatting](features/2026-09-07-area-water-terrain-realism/verification/fmt-current.log), [Clippy](features/2026-09-07-area-water-terrain-realism/verification/clippy-current.log), [doctests](features/2026-09-07-area-water-terrain-realism/verification/doctests-current.log) | Current unprofiled-source checks passed. |
| [MSRV check](features/2026-09-07-area-water-terrain-realism/verification/tooling/msrv-check-run.json) | Official Rust 1.96.1, isolated temporary toolchain and target directory, locked workspace/all-target/all-feature check passed; configured default toolchain unchanged. |
| [Dependency policy](features/2026-09-07-area-water-terrain-realism/verification/tooling/cargo-deny-run.json) | cargo-deny 0.20.2 passed with four nonfatal existing warnings and no lockfile change. |
| [Release budget gates](features/2026-09-07-area-water-terrain-realism/verification/release-budget-gates.log) | Both Criterion `--test` workloads and their explicit timing assertions passed. This is one-shot gate evidence, not a sampled benchmark distribution. |
| [Full shared replay](features/2026-09-07-area-water-terrain-realism/verification/shared-replay/validation.json) | Default seed 42, 50 million prepared cells: complete shared solve passed, including annual ledger and actual sea/domain export reconciliation. Prepared inputs stayed byte-identical. Preparation and final area/publication stages were excluded. |
| [Frozen candidate panel](features/2026-09-07-area-water-terrain-realism/verification/candidate-02-summary.json) | Five fixed configurations and 38 frozen areas, two read-only saved-export passes and 238 total export commands. All five worlds and 238 exports passed; all 38 selected area JSON files decoded, copied shared records matched, repeat exports were byte-identical, and source worlds stayed unchanged. No easier panel was substituted. |
| [Tactical probe](features/2026-09-07-area-water-terrain-realism/verification/tactical-timing/report.json) | Three actual sampled land blocks regenerated identically to saved blocks; PNG/JSON matched saved CLI exports. Exploratory timings separate load, current WFC, encode and serialize work. |

These are local Linux results under concurrent host load. No new remote CI run
or cross-platform pass is claimed. The user's first-look review reopened visual
acceptance; the authorized terrain correction requires fresh integrated evidence.
At that checkpoint, golden replacement and final checks were still pending.
The later C05/C06 sections record their completion; the overall feature remains
open for visual acceptance. The dated execution and review history is retained
in [implementation progress](features/2026-09-07-area-water-terrain-realism/history/implementation-progress.md)
and [review ledger](features/2026-09-07-area-water-terrain-realism/review-ledger.md).

## Earlier terrain correction verification — candidate04

The shared domain kernel has 12 passing controls, including a reproduced 28 mm
incision-created pit, its implicit correction, crossing catchments, internal
publication cuts, deterministic bounds and a broad physical bowl surviving all
40 steps. Four area-detail and four radial coast regressions pass. Preparation
tests cover real fringe cells and reversed slicing order; admission accounts for
all dense evolution buffers before output creation. A format/strict workspace
Clippy pass covers the corrected source. An independent numerical review found
no actionable issue in implicit incision or the radial coast change. Full source
review then identified missing logical-work admission for shared terrain; its
red/green caller-budget regression passes after the conservative static bound was
added. Two subsequent full source review rounds found no new material issue.

The candidate04 local workspace run passes **567 tests**, with eight ignored controls
and only the protected old golden expectation filtered out. Fresh full-world
regeneration equality and doctests ran. Current formatting, strict workspace
Clippy, Rust 1.96.1 MSRV and both existing release budget gates pass. The complete
test command and counts are in
[current suite summary](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/workspace-current-summary.json).

Candidate04 MICRO42 completes in 63.50 s and its saved global table contains no
basins/lakes; MICRO99 retains four lakes. These counts alone are not natural
acceptance. The full five-world frozen panel completed all 243 commands: five
generations and 238 saved exports. All 38 selected JSONs decoded, copied global
records agreed, repeat exports matched and saved worlds stayed unchanged. The
current final binary independently reproduced all 34 MICRO42 saved-file
fingerprints from candidate04. Historical legacy seam statistics drifted with the
changed continent; their independently rederived fixtures pass while preserving
the flow, surface, merger and outlet assertions. Golden hashes are unchanged.
Evidence is retained under
[terrain correction](features/2026-09-07-area-water-terrain-realism/).

### Candidate05 integration — source and saved-data checks complete

The plate-center-normal projection and incident-kind mask add ten regressions
that fail the extracted old classifier; all14 tectonic tests pass. All five
frozen configurations pass the complete coarse production acceptance gate at
attempt0. The legacy outside-neighbor correction passes15 water,14 bundle and
all8 cross-tile tests. The fixed three-context survey retains its thresholds:
119 crossings,11/13 large matches,14 nonmatches and median distance2 cells.
The individual first-accepted MICRO42 case still has2/4 large matches; this
legacy coarse/fine approximation is separate from production shared routing.

The final complete workspace run passes **583 tests**, with zero failures,
eight ignored controls and only the protected old golden expectation filtered.
Formatting, strict Clippy, isolated official Rust1.96.1 and both release benchmark
workloads pass. Two consecutive independent current-source review rounds are
dry; all157 receipt hashes match. The initial MSRV invocation omitted the existing
isolated toolchain environment and failed before compilation; its corrected
isolated invocation passed. No toolchain/default or dependency was modified.

The final neighbor binary, prior unprofiled binary and profiled panel MICRO42
match all34 saved-file fingerprints. All five frozen cases have completed
repeat exports and unchanged-world checks: 38 selected JSONs and 243 commands.
Visual acceptance remains open for regional lake shape and repetitive ravines.
See [current suite summary](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/workspace-c05-final-summary.json),
[source review](features/2026-09-07-area-water-terrain-realism/reviews/terrain-correction--c05-root-review.md)
and [saved-world equality](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/micro42-c05-neighbor-equality.json).

On 2026-09-08 the user explicitly approved the exact candidate05 golden update:
seven new hydrology files, 26 changed fingerprints, one unchanged and none removed.
The previously filtered `micro_world_matches_the_golden_fingerprint` test then
passed independently against a fresh generation in 43.82 s. Together with the
unchanged-source 583-test run above, this covers 584 passing tests; it is not a
claim that all tests were rerun in one invocation after the snapshot edit.
Golden SHA256: `6ce5613f2276c8778da1a3f680b375ef8b116ddc49fa26cd6bb70856e6b15ecf`.
Offline forcing-table regeneration also reproduced the checked-in Rust constants
exactly after formatting. The deterministic baseline is accepted; repetitive
drainage and regional lake-shape acceptance remain open. The result is retained
in the local [golden log](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/golden-c05-approved.log).

## Regional-detail correction — candidate06 verified and committed

Land detail now follows surrounding regional height differences rather than
absolute altitude. Regression controls require vertical-translation invariance,
a flat region without artificial hollows, retained downhill neighbors on gentle
slopes, continuous amplitude across kilometre lines, full signed bounds and a
substantial regional bowl. Cached preparation matches canonical direct sampling
at publication cuts and the true partial rim. The former caller work ceiling
fails admission before output creation under the newly accounted sampler cost.

The first broad run passed 587 tests, failed two terrain-dependent legacy fixtures,
and ignored 8 controls; only the protected golden expectation was filtered.
After fixture correction, all 15 water and 8 cross-tile tests pass. The fixed
three-context survey still covers 119 crossings: 12/13 large crossings match and
the combined nonmatch median is 4 cells, under the unchanged 70%/20-cell gates.
Exact independent source-path inflow equalities replace the obsolete 3/4
coarse-versus-fine snapshot; the measured 2/4 comparison remains documented.
These are combined run/rerun results, not one all-green invocation.

Formatting, strict Clippy, official Rust 1.96.1 and the existing erosion budget
workload pass. Two production-source review rounds and an independent subsequent
fixture review found no actionable issues. At the generation verification
checkpoint, production source, dependencies and renderer hashes matched the
frozen binary; only the repaired test files differed.
Two independently generated MICRO42 worlds match all 34 saved-file fingerprints.
The user subsequently approved the exact C06 baseline: 24 changed fingerprints,
10 unchanged and no added/removed files. The fresh golden comparison passes in
44.50 s against SHA256 `3e86ce4c247244b65a6aa8bb6682204b41fa91208ad18f3cd7da70f7cc04e2eb`.
Together with the broad run and focused repairs, this covers 590 passing tests.
The correction and baseline are committed as `ed4875d`.

The subsequent feature-folder cleanup moved the reusable exact-size renderer
into the canonical crate and a workspace example. All 49 renderer tests pass,
including four added checks for invalid dimensions, uneven pixel partitions,
enlarged edge coverage and ordinary/exact PNG equivalence. The example also
reproduces the retained C05 7,761×16,384 PNG byte for byte. Workspace formatting,
strict all-target/all-feature Clippy and the official Rust 1.96.1 check pass
after the move and cleanup. No generation rule changed during that move.

The read-only Capstone check reports zero covered-source drift and no unfolded
ledger fragments. Its existing nine missing-version metadata warnings and eight
scenario-heading findings for the logic index remain; the plugin manifest is
unreadable, so no version stamp was invented. See the retained
[documentation check](features/2026-09-07-area-water-terrain-realism/verification/lake-district-correction/map-check-after-cleanup.txt).

The complete five-world panel passed all 243 commands, including 238 exports and
38 selected JSON comparisons. Repeat exports match and saved worlds are unchanged.
All five annual balances close exactly. Across the three default worlds, all
20,712 adjacent pairs wet on both sides have matching lake identities and surfaces.
The reported seed changes from 2,653 to 186 lakes, with its repeated pond patterns
visibly removed. The native 16K world and four 4096² area maps are retained.
Parallel drainage and large regional basin geometry remain separate open findings.
Evidence: [correction verification](features/2026-09-07-area-water-terrain-realism/),
[fixture derivation](features/2026-09-07-area-water-terrain-realism/reports/lake-district-correction--cross-tile-c06-fixtures.md).

## Geographical rendering first pass — 2026-09-22

The Atlas renderer tests cover exact integer lighting and sampling, 512/513 area
scales, a 511×513 mixed-axis overview, adjacent interpolated rows, and continuous
planar gradients across area edges and four-area corners. Area water tests retain
validated lake depth and saved channel coverage; overview water was categorical at this first-pass stage.
The facade tests use encoded saved-area grids to exercise all eight halo directions,
two-cell diagonal context, unavailable and corrupt in-world neighbors, repeat PNG
bytes, unchanged saved layers, and failure-safe publication. CLI process tests
cover area, overview, `--detail`, preview, Classic omission/equivalence and style
refusals before output-directory creation. Sources:
[atlas tests](../../crates/arda-render/src/atlas/tests.rs),
[overview streaming tests](../../crates/arda-render/src/overview/streaming.rs),
[facade export tests](../../crates/arda/tests/area_exports.rs),
[CLI tests](../../crates/arda-cli/tests/cli.rs).

The retained seed-42 200×300 km world has a 3×5 area grid. The fixed panel selects
five areas for relief, coast, lake, channel and both seam orientations; its matched
Classic/Atlas 8K overview is 4915×8192 pixels. Two Atlas 32K area exports are
32768×32768, and two 32K overviews are 19661×32768. Each same-style 32K pair
has an identical SHA-256; the three checked Classic 512-pixel exports also match
their pre-feature bytes. All 55 saved-world file hashes and five selected area
JSON files stayed unchanged across the exports. These results establish repeatable
PNG export from this saved world, not additional generated terrain detail or
cross-platform rendering. Evidence:
[selection](features/2026-09-22-geographical-rendering-first-pass/evidence/selection.json),
[image dimensions and hashes](features/2026-09-22-geographical-rendering-first-pass/evidence/images.json),
[Classic comparisons](features/2026-09-22-geographical-rendering-first-pass/evidence/classic-compatibility.json),
[world hashes](features/2026-09-22-geographical-rendering-first-pass/evidence/world-after.json).

At source HEAD `342d03e55120`, the local Linux workspace tests, formatting,
strict workspace Clippy and Rust 1.96.1 locked all-target/all-feature check exited
zero. The dependency-policy check also exited zero before final integration;
manifests, lockfile and policy file were unchanged afterward. Task receipts retain
focused renderer, facade and CLI test runs. This is distinct from the previously
recorded Windows CI golden-text LF/CRLF failure; no fresh Windows pass was run.
Evidence: [gate results](features/2026-09-22-geographical-rendering-first-pass/evidence/gates/results.json),
[task receipts](features/2026-09-22-geographical-rendering-first-pass/evidence/task-1.md).

## Terrain and shoreline correction verification — 2026-09-23

Source `757b2ab` includes exact Euclidean tectonic belts, preserved negative ocean-rim depths, coarse hillslope-only evolution and shared Atlas sea/land shoreline reconstruction. Two independent GPT-6 Sol review rounds after the integration-test fixes found no new actionable defects. Format, strict workspace Clippy, Rust 1.96.1 locked all-target/all-feature checks and offline cargo-deny pass locally; cargo-deny retains four nonfatal policy warnings. No new remote or Windows CI pass is claimed.

The broad non-golden workspace run at `c23bcbf` passed 659 tests with 8 ignored. Subsequently the added production-width test and strengthened accepted-world persistence check passed the 18-test tectonics and five-test continent suites. Strict golden comparison and independent repeat generation both passed after the authorized terrain baseline update (102.83 s). These are combined broad and focused results, not one newly rerun workspace invocation. Golden changes affect 32 of 34 fingerprints, with no file added or removed.

Focused fixtures compare tectonic distance against an independent nearest-source oracle and transpose, pin physical belt widths, and exercise actual convergent plates beyond the immediate diffusion ring. Coarse creep tests preserve seafloor, affine slopes and symmetric diffusion (`crates/arda-gen/src/continent/tectonics_tests.rs:185`, `crates/arda-gen/src/continent/tectonics.rs:393`, `crates/arda-gen/src/continent/erode.rs:176`). Persisted seed-42 MICRO overview fields and coarse river objects equal regenerated accepted attempt 2 (`crates/arda-gen/tests/continent_hydrology.rs:34`). Atlas tests exercise zero heights, narrow islands/straits, lakes, checkerboards, real halo context, area/overview agreement and mixed-style river clipping across streamed band seams (`crates/arda-render/src/atlas/tests.rs:566`, `crates/arda-render/src/channels/tests.rs:113`, `crates/arda-render/src/overview/streaming.rs:328`).

The integrated seed-42 small/default worlds and seed-99 MICRO holdout all close their saved annual budgets exactly. The default world has 118 lakes, 799,834 global reaches and 2,123 crossings; all decoded crossing flow/account fields match their canonical reach. This audit covers metadata/lake/reach/crossing tables, not every table or every adjacent wet cell. The default world completed in 1,640.71 s. It was generated by the frozen candidate with coarse incision coefficient zero, which is behaviorally equivalent to the committed creep-only loop, and rendered with the later shoreline-corrected production binary.

The actual 15,522×32,768 default Atlas PNG is 129,037,747 bytes. Its repeat is byte-identical, takes 65.65 s and peaks at 57,240 KiB RSS. All 523 world-file hashes are unchanged by the repeat export. Separate 32K area/overview exports from the retained small world peak at 44,388/66,860 KiB respectively. Classic's selected comparison remains byte-identical. The six-area default gallery retains the baseline coordinates; whole-map and detailed views still show broad smooth mountain forms, parallel valleys and a large regional lake. Numerical validity and deterministic export do not establish full visual acceptance.

Evidence: [gates](features/2026-09-23-terrain-corrections/evidence/final-gates/results.json), [water audit](features/2026-09-23-terrain-corrections/evidence/default-world-water-audit.json), [32K repeat](features/2026-09-23-terrain-corrections/evidence/fine-incision-candidate/full-world-32k-repeat/verification.json), [review ledger](features/2026-09-23-terrain-corrections/review-ledger.md).

## Coverage shape

Image-quality controls validate 512–32768 edges, the 8192 default and area-grid aspect ratios. Streaming overview tests compare decoded pixels with buffered output at area and 256-row band seams; encoded byte equality between those two APIs is not required. The 32K unit admission check validates dimensions and bounded-band allocation without encoding a full image. Channel tests exercise selected rows/coverage at 32K, and facade tests use real saved-layer fixtures to check unchanged inputs, repeated exports and preservation of the prior PNG on failure (`crates/arda-render/src/quality.rs:84`, `crates/arda-render/src/overview/streaming.rs:302`, `crates/arda-render/src/channels/tests.rs:59`, `crates/arda/tests/area_exports.rs:100`, `crates/arda/src/export_quality.rs:124`).

Commit `c577528` separately records completed local 32K area/world exports, 611 workspace tests and 74 final targeted checks; those counts can overlap. The historical record is not a new run or an exact-HEAD CI success; see [current status](open-items.md).

Implemented gates cover physical topology, exact annual accounting, storage
refusal, deterministic output and saved rendering. Maximum-size admission tests
check arithmetic/capacity contracts; they do not establish successful natural
generation or a runtime bound for every 4000 × 4000 km terrain. Both the candidate05
and subsequent candidate06 panels pass saved-data checks. Visual inspection and the direct boundary probe
support the flooded-rim correction: the reported area now has 6.51% wet rim
against 6.45% interior, with matching shared lake IDs/surfaces across wet border
pairs. Repetitive drainage and rectangular coarse basins remain unresolved visual
findings; numerical passes do not close their acceptance. Evidence:
[current panel](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/candidate05-summary.json),
[reported-area review](features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--visual-default436342-c05.md),
[seed42 geometry](features/2026-09-07-area-water-terrain-realism/reports/terrain-correction--visual-default42-c05.md).

The earlier design's Horton/Hack/settlement rank-size/road sinuosity/farmland
statistical suite, a sustained load/export latency suite and full tactical
continuity/layout acceptance are deferred. The current block generator uses 24
tiles and only sampled land cells; the proposed 200+ tile vocabulary and society
stages remain unbuilt. Earlier dated measurements in the logic chapters retain
their historical meaning and do not establish those gates.
Source: [tiles.rs](../../crates/arda-core/src/tiles.rs),
[orchestrator.rs:136](../../crates/arda-gen/src/orchestrator.rs:136),
[area generation](logic/02-area-generation.md).

## Atlas material continuation — 2026-09-23

Source `3c06909` changes only Atlas land palette/light derivation and its renderer tests. Saved slope magnitude controls exposed rock; continuous altitude/slope weights control snow appearance. Existing area/overview interpolation, halo, water ownership, Classic routing and streaming allocations remain unchanged.

Verification: 89 renderer tests, 56 facade/CLI tests across six suites (141.93 s), formatting and strict workspace Clippy pass. Two fresh independent GPT-6 Sol reviews found no actionable defects. Extreme central gradients are tested; reviewers also checked the doubled one-sided i32-span arithmetic bounds. Existing seam tests prove shared gradient/light behavior; a dedicated high-slope rock-colour seam assertion was noted as optional additional coverage, not a demonstrated defect.

The final binary exported the same immutable default seed-42 world as a 2K overview and matched areas (5,4), (1,6), (2,6). The integrated full 15,522×32,768 overview took 66.92 s and has SHA-256 `a920b5eb8d983870e48abdcc32fa895d3260427f472a279541e1dabc18dbd25a`; command and receipt are retained in `features/2026-09-23-terrain-corrections/evidence/material-probe/integrated/full-world-32k/verification.json`. This is an updated export, not a regenerated world or a repeated resource benchmark. No full workspace, MSRV, dependency-policy or remote CI rerun is claimed for this small continuation.

Actual renders show clearer saved valleys and less blanket-white highland. Broad smooth mountain forms, parallel channels and the large physical basin remain unresolved. Isolated terrain trials retain numeric inputs and reproducible evidence under `fine-time-probe`, `fine-receiver-probe`, `fine-threshold-probe` and `regional-relief-probe`; none changes production generation or its golden files.

## Atlas lake-depth continuation — 2026-09-23

Source `0fc9666` passes 94 renderer library tests, 12 public area-export tests and 14 CLI process tests. Formatting, strict Clippy for `arda-render` and `arda` with all targets, and whitespace checks pass. Two independent Sol code reviews found no confirmed defects. Their noted test gaps were then covered by a diagonal lake-depth corner fixture and public area/buffered/streamed mixed-axis overview assertions, inspected by the root reviewer.

Matched 2K exports of the saved default seed-42 world were inspected against clean archived HEAD `820af9d`: the overview changes 147,889 pixels; lake-interior area 4,5 changes 4,194,304 and lake-margin area 4,7 changes 1,926,688. An initially shared build target reused the new executable and invalidated the first baseline comparison; those files were replaced after a separate clean-target build. Classic area 4,7 remains byte-identical. The current 32K output is 15,522×32,768, produced in 69.34 s at 76,904 KiB peak child RSS; no repeat run or full workspace/remote CI result is claimed for this continuation. [Evidence and exact receipts](features/2026-09-23-terrain-corrections/evidence/atlas-lake-depth/README.md).

Controlled terrain trials remain experimental. Coupled D-infinity, jittered coarse meshes and sediment infill did not establish the requested morphology. At that stage, ordinary intermediate-scale relief plus longer evolution had produced larger branching valleys only on a fixed crop. The subsequent structural-relief verification below covers completed worlds; fine parallel gullies remain. Reference-quality visual acceptance remains open.

## Structural-relief continuation — 2026-09-23

Source `4ea2271` adds globally sampled 8/4 km structural relief before 160 shared evolution iterations. The legacy tile evolution remains 40 iterations. Physical fixtures cover zero/quarter/full relief gates, preservation of nonpositive initial/coarse inputs, permitted land-to-nonpositive crossings, integer extremes and the analytical delta bound (`crates/arda-gen/src/continent/structural_relief.rs:82`). The preparation test asserts active, nonsaturated gates across cuts and actual outside coordinates, and rejects an incorrectly clamped outside sampler (`crates/arda-gen/src/area/prepare.rs:225`). The legacy crossing survey permits an all-match result while preserving independent entering oracles, sample floors and mismatch-distance bounds; a deterministic control covers nearest-window distance (`crates/arda-gen/tests/cross_tile.rs:538`).

The cleaned seven-file proposal passed 674 non-golden workspace tests with eight ignored, focused tests, strict all-package Clippy, formatting, Rust 1.96.1 locked/offline all-target/all-feature checks and cached offline dependency policy. Two independent component reviews found no confirmed defect. Its MICRO99 output matches all 34 frozen-candidate files byte-for-byte. These are retained private-proposal checks, not a newly repeated complete root workspace run or remote/Windows CI result. Evidence: `features/2026-09-23-terrain-corrections/evidence/integer-regional-relief-probe/candidate-160/production-proposal/`.

The full seed-42 500×1000 km candidate completed in 3256.006 s with 1,850,884 KiB peak child RSS. Its frozen saved-data auditor checks all seven global hydrology tables, exact annual source/sink equality of 153,308,678,440,000 L, crossing authority and 12,356 both-wet published seam pairs. It records 171 published areas, 87 global lakes and 734,623 published channel cells. These saved consistency checks do not independently reproduce every physical equation. Earlier small42 and MICRO99 audits also pass, but their zero both-wet seam counts are vacuous; the default world supplies the nonempty seam check. Evidence: `candidate-160/validation/candidate-default-seed42.json` under the same evidence directory.

Matched actual overview, highland, river, coast and lake panels show deeper branching valleys. Broad smooth crests, repeated fine gullies and plain lowlands remain; component acceptance does not close the reference-quality objective. The prepared highland snapshot and published heights match in all 262,144 cells (`candidate-160/root-default-visual-verdict.json`, `candidate-160/prepared-highland-publication-check.json`).

The 15,522×32,768 Atlas PNG is 206,066,791 bytes. Two completed exports took 71.31 s and 71.40 s and match byte-for-byte; peak child RSS was 66,416/66,172 KiB. A turn interruption stopped the original repeat, which was rerun into a separate directory after confirming the exporter was no longer running. All 523 published world files retain identical before/after hashes. Receipt: `candidate-160/full-world-32k/receipt.json` under the same evidence directory.

After explicit user approval, the root checkout received the exact reviewed seven source/test files and the 34-entry golden fixture. All installed source SHA-256 values match the reviewed proposal; the fixture SHA-256 is `2801a96797ec2a2fb276d82a1ae9d99023bad7d6d5e0fd370d033cbc5eb75152`. `cargo test --release -p arda-workspace-tests --test golden_world --offline` passes both tests (192.10 s): strict saved fingerprint and independent same-seed repeat. Formatting and `git diff --check` pass. The intended fingerprint change affects 23 of 34 files, with no additions or removals: 12 area files, four block files and seven hydrology tables. Both continent files, world.json, four area files and four block files retain their fingerprints. Root integration receipts are under `candidate-160/production-adoption/` in the evidence directory above.

## Atlas saved-wetness response — 2026-09-23

Source `3f95fcc` copies the existing saved wetness byte for target cells and all
eight neighboring halo directions, then uses it to tint Atlas land before rock
and snow material blending. The Q12 weight is `wet×2048/(wet+12)` toward
`[81,126,73]`; zero wetness retains the former base colour. Tests cover the
weight response, matching cardinal/diagonal halo and interior samples, unchanged
sea/lake colour and terrain ownership, and decoded buffered-versus-streamed
mixed-axis overview pixels with distinct nonzero wetness (`crates/arda-render/src/atlas/tests.rs:516`,
`crates/arda-render/src/atlas/tests.rs:523`,
`crates/arda-render/src/atlas/tests.rs:591`,
`crates/arda-render/src/atlas/tests.rs:634`). The moved layout test still
asserts a 16-byte `Option<SourceSample>` and fixed 516²/514² grid lengths
(`crates/arda-render/src/atlas/tests.rs:1118`).

The source-change verification passed 98 renderer library tests, 12 public
area-export tests and the focused mixed-axis test after a test-only Clippy loop
correction. Formatting, whitespace checks and strict offline Clippy for
`arda-render --all-targets` passed. Two independent source reviews found no
actionable issue. These focused local checks are not a new full workspace,
MSRV, golden-world or remote CI result. The gate commands and logs are in the
[wetness verification receipt](features/2026-09-23-terrain-corrections/evidence/wetness-response-probe/production/verification.json);
the receipt records the committed source and completed checks.

The committed renderer produced two byte-identical 15,522 × 32,768 PNGs (327,031,502 bytes each) in 76.16 s and 76.05 s, at 66,516 and 66,284 KiB peak child RSS. All 523 saved world files (1,943,353,639 bytes) retain identical before/after hashes. The 970 × 2048 preview is downsampled from the completed 32K PNG, not a separate 2K export. Root inspected it; smooth principal ridges and repetitive fine gullies remain. Receipt: `features/2026-09-23-terrain-corrections/evidence/wetness-response-probe/production/full-world-32k/receipt.json`.


The subsequent isolated convergence-magnitude and adapted multiscale-preset trials were rejected after numerical controls and root visual inspection. They are diagnostic evidence, not production code or new full-world verification. The latter preserves the released four-stage counts and checks scalar flow, erosion, thermal and sediment rules, but differs from Arda in initialization, kernel and floating-point/crop boundaries; its negative outcome is limited to that adapted preset. Reports: `features/2026-09-23-terrain-corrections/evidence/crest-profile-diagnostic/README.md` and `features/2026-09-23-terrain-corrections/evidence/multiscale-erosion-probe/README.md`.

## Atlas overview river readability — 2026-09-23

Commit `a9ce4fefd566` centers scale-aware blue Mid and Dark Atlas overview
river symbols using a reusable 3/4 chamfer distance field. The radius is
`ceil(max(width,height)/2048)/2` rounded up for Mid and
`ceil(max(width,height)/2048)` for Dark, reaching 8 and 16 output pixels
respectively at 32K. Streaming encodes 256 output rows with at most 16 halo
rows above and below, so symbols cross both band and area seams. Classic
keeps its prior right/down one-pixel Dark extension
(historical commit `a9ce4fe`; this sizing rule was superseded below).

That earlier component passed 101 renderer tests, 12 facade area-export
tests, strict renderer Clippy, workspace formatting and diff checks. Two
independent dry reviews found no surviving issue. The actual default-world
15,522×32,768 Atlas PNG was exported twice: 78.9093 and 79.3596 s,
325,652,273 bytes each, identical SHA-256
`a60ed031cf7d7d95c18e1cf2e520b85161c7d5ea6f1ba7b1bceee86f3fea591d`.
The 523 saved world files (1,943,353,639 bytes) were unchanged, and a
selected Classic 2K export was byte-identical to its prior result. Root
inspection found the main rivers still blue at fitted overview size.
Mountain form was unchanged, so broad visual acceptance remains open.
Sources: [verification](features/2026-09-23-terrain-corrections/evidence/readability-pass/verification.json),
[export receipt](features/2026-09-23-terrain-corrections/evidence/readability-pass/render-receipt.json).

## Saved-width Atlas river presentation — 2026-09-23

Commit `92689f8a8142` replaced the preceding fixed Mid/Dark symbol sizes
with saved-width sizes: Mid `4+min(width_dm/150,3)`, Dark
`7+min(width_dm/180,6)`, and Light zero. The Mid/Dark maxima at 32K are
7/13 pixels. Fractional negative 3/4 chamfer seeds retain width variation
in a 2K overview; one-pixel alpha banks and a muted edge-to-core profile
blend over the original Atlas terrain. The water profile is display
shading, not measured depth. The streaming halo is at most 14 rows on
each side of 256 output rows. A zero-width fallback applies only to
synthetic Atlas callers; overflow saturates to the largest symbol
(`crates/arda-render/src/overview.rs:28`,
`crates/arda-render/src/overview.rs:86`,
`crates/arda-render/src/overview.rs:428`,
`crates/arda-render/src/overview/streaming.rs:92`).

The component passed 103 renderer and 12 public facade area-export tests,
workspace formatting, strict renderer Clippy and diff checks. Two fresh
independent dry reviews found no surviving issue. Two actual default-world
15,522×32,768 Atlas exports were byte-identical: 326,768,717 bytes,
SHA-256 `141284ec438b22e52b6bd27110b6d2d1d9d3724d861a5aebc2912d064176f328`,
80.7532 and 80.6843 s. All 523 saved world files
(1,943,353,639 bytes) remained unchanged; selected Classic 2K output
matched the prior exporter byte-for-byte. Root inspected actual fitted
and native-pixel comparisons and accepted a partial river naturalness
improvement. Mountain appearance was unchanged and reference-quality
visual acceptance remains open. Evidence:
[natural river summary](features/2026-09-23-terrain-corrections/evidence/natural-river-pass/README.md),
[32K receipt](features/2026-09-23-terrain-corrections/evidence/natural-river-pass/surface-receipt.json),
[native comparison](features/2026-09-23-terrain-corrections/evidence/natural-river-pass/surface-native-comparison.jpg).

## Connected Atlas overview channels — 2026-09-23

Source `14a70b9144dd` passes 108 renderer tests and 12 public facade export tests, workspace formatting, strict `arda-render`/`arda` all-target Clippy and diff checking. Two final independent GPT-6 Sol reviews are dry. Buffered/streamed tests cover rectangular area/band seams, confluence geometry, water ownership and bounded context refusals. The analytic unequal-span rectangle oracle matches all 131,841 pixels. Two retained dense-pixel fixtures independently match GEOS alpha and maximum discharge within Q20 rounding tolerance. Legacy area coverage remains unchanged.

A full-world minimum-quality 512 refusal exposed excessive union fragmentation. The overview-only 64-piece subdivision trigger fixes it without changing widths or raising work/depth caps. The final 2K changes only one channel unit in one pixel versus the earlier 4096-trigger candidate; final 32K bytes are identical. The minimum export succeeds in 13.27 s. Two final 15,522×32,768 exports take 93.1878/92.6213 s and match SHA-256 `cc1a666b5376e087f63ea891c65e3eccac1c5773ad480173d4eb7652f14d79cb` (329,746,127 bytes). Classic 2K and physical area 1725 controls are unchanged; all 523 saved files retain their hashes. These are local measurements, not universal resource guarantees. Root accepts a partial river-geometry improvement; mountains and broad reference-quality acceptance remain open. [Evidence and actual images](features/2026-09-23-terrain-corrections/evidence/connected-river-integration/README.md), [integration receipt](features/2026-09-23-terrain-corrections/evidence/connected-river-integration/production-integration.json).
