---
generated_at_commit: 311829e4e5c9
generated_date: 2026-09-08
content_hash: 79c148f86c7d
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/src/**", ":(top)crates/*/tests/**", ":(top)crates/*/benches/**", ":(top)tests/**", ":(top).github/**"]
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

The golden test has an explicit `ARDA_BLESS` write path. Running ordinary checks
does not authorize replacing expected bytes. The current format/physical changes
have a proposed fingerprint, but explicit approval and replacement are still
pending. Reproducibility and geographic/visual acceptance remain separate checks.
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
| [Frozen candidate plan/driver](features/2026-09-07-area-water-terrain-realism/verification/run_candidate.py) | Five fixed configurations and 38 frozen areas, two read-only saved-export passes and 238 total export commands. All five worlds and 238 exports passed; all 38 selected area JSON files decoded, copied shared records matched, repeat exports were byte-identical, and source worlds stayed unchanged. No easier panel was substituted. |
| [Tactical probe](features/2026-09-07-area-water-terrain-realism/verification/tactical-timing/report.json) | Three actual sampled land blocks regenerated identically to saved blocks; PNG/JSON matched saved CLI exports. Exploratory timings separate load, current WFC, encode and serialize work. |

These are local Linux results under concurrent host load. No new remote CI run
or cross-platform pass is claimed. The user's first-look review reopened visual
acceptance; the authorized terrain correction requires fresh integrated evidence.
Explicit approval/replacement of the golden and the final project gate remain pending; this chapter does not
mark the feature implemented. The dated execution and review history is retained
in [implementation progress](features/2026-09-07-area-water-terrain-realism/implementation-progress.md)
and [review ledger](features/2026-09-07-area-water-terrain-realism/review-ledger.md).

## Terrain correction verification — in progress

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
[terrain correction](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/).

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
[source review](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/c05-root-review.md)
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

## Coverage shape

Implemented gates cover physical topology, exact annual accounting, storage
refusal, deterministic output and saved rendering. Maximum-size admission tests
check arithmetic/capacity contracts; they do not establish successful natural
generation or a runtime bound for every 4000 × 4000 km terrain. The complete candidate05
panel passes saved-data checks. Visual inspection and the direct boundary probe
support the flooded-rim correction: the reported area now has 6.51% wet rim
against 6.45% interior, with matching shared lake IDs/surfaces across wet border
pairs. Repetitive drainage and rectangular coarse basins remain unresolved visual
findings; numerical passes do not close their acceptance. Evidence:
[current panel](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/candidate05-summary.json),
[reported-area review](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/visual-default436342-c05.md),
[seed42 geometry](features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/visual-default42-c05.md).

The earlier design's Horton/Hack/settlement rank-size/road sinuosity/farmland
statistical suite, a sustained load/export latency suite and full tactical
continuity/layout acceptance are deferred. The current block generator uses 24
tiles and only sampled land cells; the proposed 200+ tile vocabulary and society
stages remain unbuilt. Earlier dated measurements in the logic chapters retain
their historical meaning and do not establish those gates.
Source: [tiles.rs](../../crates/arda-core/src/tiles.rs),
[orchestrator.rs:136](../../crates/arda-gen/src/orchestrator.rs:136),
[area generation](logic/02-area-generation.md).
