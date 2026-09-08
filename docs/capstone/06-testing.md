---
generated_at_commit: b0f93f22b969
generated_date: 2026-09-08
content_hash: cf6f8d42eb8e
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/src/**", ":(top)crates/*/tests/**", ":(top)crates/*/benches/**", ":(top)tests/**", ":(top).github/**", ":(top)crates/*/examples/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# Testing

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
| Golden world | `tests/golden_world.rs:69` | Fixed MICRO file fingerprint; a separate test at line 91 checks repeat-generation equality. |
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

CI runs workspace tests and the explicit golden gate on Linux, macOS and Windows.
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
Source: [golden_world.rs:69](../../tests/golden_world.rs:69).

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

## Coverage shape

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
