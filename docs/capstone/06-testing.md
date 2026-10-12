---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 7e1d16327067
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top).gitattributes"
  - ":(top)deny.toml"
  - ":(top).github/**"
  - ":(top)tests/**"
  - ":(top)crates/*/src/**"
  - ":(top)crates/*/tests/**"
  - ":(top)crates/*/benches/**"
  - ":(top)apps/viewer/package.json"
  - ":(top)apps/viewer/vite.config.ts"
  - ":(top)apps/viewer/src/**"
  - ":(top)tools/art-gen/test/**"
---

# Testing

Counts below are `#[test]` / `#[tokio::test]` attributes counted from source at
`022ce7cb5707` (no test run): 1,247 in `crates/*/src/`, 509 in `crates/*/tests/`,
17 in root `tests/`, 1,773 in total, of which 29 carry `#[ignore]` and 67 are
`#[tokio::test]`. Viewer and art-gen counts are `it(`/`test(` calls.

## Layout

### Where tests live

| Suite | Location | Mapping to source |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` blocks and sibling `*_tests.rs` / `tests/` modules inside `crates/<crate>/src/` | Same module as the code (e.g. `crates/arda-gen/src/formation/form_tests.rs:24`, `crates/arda-server/src/bindings.rs:204`) |
| Crate integration tests | `crates/<crate>/tests/*.rs`, one test binary per top-level file; subdirectories are modules (`crates/arda-server/tests/world_api/main.rs:5`) | Public API of that crate only |
| Workspace tests | root `tests/`, hosted by the root package `arda-workspace-tests` (`Cargo.toml:77`), `autotests = false` (`Cargo.toml:78`), empty lib `tests/empty_lib.rs` (`Cargo.toml:84`) | Cross-crate: generation → settle → society → server |
| Benches | `crates/arda-gen/benches/area_erosion.rs`, `continent_stage.rs`, `harness = false` (`crates/arda-gen/Cargo.toml:20`) | arda-gen timing budgets |
| Doctests | one fence at `crates/arda-names/src/lib.rs:8`; every other fence under `crates/*/src/` is `text` (e.g. `crates/arda-ids/src/hash.rs:13`) | arda-names README example |
| Viewer | `apps/viewer/src/**/*.test.ts`, Vitest, `environment: "node"`, include `src/**/*.test.ts` (`apps/viewer/vite.config.ts:19`) | Co-located with `.ts` source |
| Art tooling | `tools/art-gen/test/art-gen.test.mjs`, `mock-slopify.test.mjs`, `node:test` | `tools/art-gen/*.mjs` |

Root `[[test]]` targets are declared explicitly (`Cargo.toml:104`–`126`):
`golden_world`, `golden_recipes`, `arid_recipe7`, `e2e_village`, `zoom_continuity`,
`fixture_dir`. `tests/zoom_land.rs` is not a target of its own; it is the module
`mod zoom_land;` of `zoom_continuity` (`tests/zoom_continuity.rs:14`).
`tests/support/fixture_dir.rs` is included by `#[path]` (`tests/fixture_dir.rs:7`).

### Per-crate inventory

| Crate | Unit (`src/`) | Integration (`tests/`) | Ignored | Integration files |
|---|---:|---:|---:|---|
| `arda` | 11 | 21 | 0 | `round_trip.rs`, `area_exports.rs` (+`area_exports/reload.rs`) |
| `arda-art-import` | 30 | 12 | 0 | `holes.rs`, `import.rs`, `fixtures/mod.rs` |
| `arda-blocks` | 4 | 7 | 0 | `compose.rs`, `settlements.rs` |
| `arda-cli` | 15 | 18 | 0 | `cli.rs`, `tactical.rs` |
| `arda-core` | 115 | 0 | 0 | none |
| `arda-dungeon` | 0 | 9 | 0 | `levels.rs`, `golden/renders.txt` |
| `arda-fields` | 20 | 24 | 0 | `complete.rs`, `cores.rs`, `landuse.rs`, `mines.rs`, `partition.rs`, `rules.rs`, `seams.rs`, `support/shape.rs` |
| `arda-gen` | 557 | 29 | 9 | `anisotropy_probe.rs`, `coast_measurement.rs`, `continent_hydrology.rs`, `continent_measures.rs`, `cross_tile.rs` (+`oracles`, `seam_lakes`, `seams`), `drainage_invariants.rs` |
| `arda-ids` | 12 | 0 | 0 | none |
| `arda-midzoom` | 8 | 16 | 2 | `checks.rs`, `land.rs`, `world.rs` |
| `arda-names` | 15 | 22 | 1 | `blocked_roots.rs`, `determinism.rs`, `dialects.rs`, `distinctness.rs`, `json_ids.rs`, `performance.rs`, `phonotactics.rs`, `serde_keys.rs`, `uniqueness.rs` |
| `arda-npc` | 7 | 48 | 1 | 13 files incl. `determinism.rs`, `performance.rs`, `stat_blocks.rs`, `srd_math.rs`, `common/mod.rs` |
| `arda-people` | 3 | 5 | 1 | `interiors.rs`, `notable_mix.rs`, `rivers.rs`, `town_wfc.rs` |
| `arda-refine` | 34 | 37 | 4 | 11 files incl. `determinism.rs`, `perf.rs`, `real_world.rs`, `seams.rs`, `schema.rs` |
| `arda-render` | 147 | 0 | 0 | none |
| `arda-scene` | 12 | 47 | 0 | `derive.rs`, `hostile.rs`, `rules.rs`, `scene.rs`, `walls_los.rs`, `common/mod.rs` |
| `arda-server` | 84 | 73 | 1 | `serve.rs` (3) and `world_api/` (70 across 15 modules) |
| `arda-settle` | 29 | 28 | 2 | `memory.rs`, `micro.rs`, `names.rs`, `realms_micro.rs`, `render.rs`, `society.rs` (+`society/realms.rs`), `write.rs` |
| `arda-society` | 4 | 37 | 2 | 10 files incl. `determinism.rs`, `scale.rs`, `trade_scale.rs` |
| `arda-tactical` | 109 | 1 | 1 | `timing.rs` |
| `arda-town` | 20 | 41 | 0 | `blocks.rs`, `interiors.rs`, `plan.rs`, `rivers.rs`, `wfc.rs`, `wfc_rows.rs`, `common/mod.rs` |
| `arda-ways` | 5 | 29 | 0 | `fallback.rs`, `ids.rs`, `reach.rs`, `rules_sidecar.rs`, `standing.rs`, `ways.rs` (+`ways/refusals.rs`), `wet.rs` |
| `arda-wfc` | 6 | 5 | 0 | `solve.rs` |
| root `tests/` | — | 17 | 5 | `golden_world.rs` (2), `golden_recipes.rs` (5), `arid_recipe7.rs` (2), `e2e_village.rs` (1), `zoom_continuity.rs` + `zoom_land.rs` (4), `fixture_dir.rs` (3) |
| `apps/viewer` | — | 86 | — | 12 files: `api/{areaBin,client,scene,tactical}`, `format`, `hooks`, `geo/{cellWalk,coords,handoff,relief,tacticalTiles}`, `render/tacticalOverlay` |
| `tools/art-gen` | — | 31 | — | `art-gen.test.mjs` (28), `mock-slopify.test.mjs` (3) |

### Commands

| Check | Command | Source |
|---|---|---|
| Workspace tests (CI) | `cargo test --workspace --all-features --no-fail-fast` | `.github/workflows/ci.yml:15` |
| Workspace tests (README) | `cargo test --workspace --all-features` | `README.md:403` |
| Formatting | `cargo fmt --all --check` | `.github/workflows/ci.yml:32` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | `.github/workflows/ci.yml:33` |
| MSRV | `cargo check --workspace --all-targets --all-features --locked` on toolchain 1.96.1 | `.github/workflows/ci.yml:22` |
| Dependency policy | `EmbarkStudios/cargo-deny-action@v2` against `deny.toml` | `.github/workflows/ci.yml:39` |
| Golden world bless | `ARDA_BLESS=1 cargo test --test golden_world` | `tests/golden_world.rs:4` |
| Recipe goldens bless | `ARDA_BLESS=1 cargo test --test golden_recipes` | `tests/golden_recipes.rs:10` |
| Bindings/schema freshness | `cargo test -p arda-server bindings` / `schema`; bless with `ARDA_BLESS_BINDINGS=1` | `crates/arda-server/src/bindings.rs:4`, `crates/arda-server/src/schema/mod.rs:9` |
| Dungeon render bless | `ARDA_BLESS_DUNGEON=1 cargo test -p arda-dungeon` | `crates/arda-dungeon/tests/levels.rs:193` |
| Release gates (ignored) | e.g. `cargo test --release --test e2e_village -- --ignored`; `cargo test --release -p arda-tactical --test timing -- --ignored --nocapture` | `tests/e2e_village.rs:296`, `crates/arda-tactical/tests/timing.rs:4` |
| Bench budgets | `cargo bench -p arda-gen --bench area_erosion` / `continent_stage` | `crates/arda-gen/Cargo.toml:20` |
| Viewer | `npm test` (`vitest run`) in `apps/viewer` | `apps/viewer/package.json` `scripts.test` |
| Art tooling | `node --test tools/art-gen/test/` | `tools/art-gen/test/art-gen.test.mjs:3` |

`[profile.test]` sets `opt-level = 2` for every test build (`Cargo.toml:68`).
`.gitattributes` forces `eol=lf` on every text file so golden fixtures hash
identically on Windows runners (`.gitattributes:4`).

### CI matrix

The single workflow `.github/workflows/ci.yml` runs on `push` and `pull_request`
(`ci.yml:2`) with five jobs:

| Job | Runner | Toolchain | Runs |
|---|---|---|---|
| `test` | `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` (`ci.yml:7`–`9`) | `dtolnay/rust-toolchain@stable` | workspace tests, including both golden targets |
| `msrv` | `ubuntu-latest` | `1.96.1` (`ci.yml:22`) | locked all-target check, no tests |
| `lint` | `ubuntu-latest` | stable + rustfmt, clippy | fmt, clippy `-D warnings` |
| `deny` | `ubuntu-latest` | action-provided | licences, bans, advisories, sources (`deny.toml`) |
| `viewer` | `ubuntu-latest`, working directory `apps/viewer` | Node 26 (`actions/setup-node`, SHA-pinned) | `npm ci`, `npm run licences`, `typecheck`, `lint`, `npm test` (Vitest plus `node --test scripts/`), `build` |

Not run by CI: `#[ignore]` tests (no `--ignored` step), Criterion benches, and the art-gen `node --test` suite.
There is no other workflow file under `.github/workflows/`.

### Golden world test

`tests/golden_world.rs` generates MICRO seed 42 with `arda::generate` into a temp
directory once per process (`LazyLock`, `tests/golden_world.rs:69`) and
fingerprints every file as `<relative path>  <blake3 hex>`, sorted, with `\` →
`/` normalisation (`tests/golden_world.rs:38`, `:63`).

| Test | Line | Assertion |
|---|---|---|
| `micro_world_matches_the_golden_fingerprint` | `tests/golden_world.rs:79` | fingerprint equals `tests/golden/micro-42.txt` (34 entries); with `ARDA_BLESS` set it rewrites the file and returns (`:82`) |
| `regenerating_produces_the_same_fingerprint` | `tests/golden_world.rs:99` | a second, independent generation matches the first |

`tests/golden_recipes.rs` pins the fine-terrain recipes: each `check(recipe)`
(`tests/golden_recipes.rs:65`) generates MICRO 42 with
`generate_from_fine_recipe`, renders an Atlas 4K overview and compares the file
fingerprints plus the PNG hash with `tests/golden/micro-42-recipe<N>.txt`
(`:101`). Tests: `recipe_5_replays_v0_1_byte_for_byte` (`:154`),
`recipe_6_replays_v0_2_byte_for_byte` (`:159`), `recipe_7_is_pinned` (`:165`),
`recipe_8_is_pinned` (`:172`), `unknown_recipes_are_refused_before_output`
(`:177`). The module doc states recipe-5 hashes must never be re-blessed
(`:11`). `world.json` is excluded from the hash and checked field by field (`:7`).

### Shared world fixtures under `target/`

`tests/support/fixture_dir.rs` builds a fixture once and shares it across test
processes:

- `ensure(dir, ready, build)` (`tests/support/fixture_dir.rs:38`) returns at once
  when the fixture is stamped and `ready`; otherwise it takes an exclusive lock on
  `<name>.lock` beside it (`:50`–`56`), re-checks, builds into
  `<name>.tmp-<pid>-<n>` (`:65`), writes the stamp, moves any existing directory
  aside to `<name>.stale-<pid>-<n>` and renames the temp directory into place
  (`:69`–`80`).
- The stamp file is `.fixture-stamp` (`:28`). Its content is `fingerprint()`
  (`:94`): a `DefaultHasher` over `FORMAT` (currently `1`, `:25`), root
  `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and every file under each
  `crates/*` member except its top-level `tests`, `benches` and `examples`
  directories (`:139`–`150`). A stamp mismatch counts as not ready, so any
  library source, data, manifest, lock or toolchain change rebuilds the fixture;
  test-file edits do not, and a builder change in a test file bumps `FORMAT`
  (`:13`–`16`).
- `tests/fixture_dir.rs` covers it: six racing builders leave one complete
  fixture and no leftovers (`tests/fixture_dir.rs:33`), an incomplete directory is
  replaced (`:62`), a fixture with an old or missing stamp is rebuilt (`:74`).

| Fixture directory | Users (resolution order) |
|---|---|
| `target/arda-server-fixture/micro42` | `crates/arda-server/tests/world_api/support.rs:31` (`$ARDA_SERVER_TEST_WORLD`, then `out/micro42` if it loads at the current `FINE_TERRAIN_RECIPE_VERSION` and seed 42, `:21`, `:38`, else this fixture, `:42`); `crates/arda-settle/tests/micro.rs:33` (`$ARDA_TEST_WORLD` first, `:25`) |
| `target/arda-server-fixture/micro42-society` | `crates/arda-server/tests/world_api/support.rs:173`: generate + `arda_settle::generate` + `arda_people::build`, used when the base world has no `society/notables.json` (`:159`) |
| `target/arda-server-fixture/micro<seed>` | `crates/arda-settle/tests/realms_micro.rs:34` (`$ARDA_TEST_WORLD_<seed>` first, `:22`) |
| `target/arda-people-fixture/micro42` | `crates/arda-people/tests/interiors.rs:47`, `rivers.rs:39`, `notable_mix.rs:37`, `crates/arda-blocks/tests/settlements.rs:43` (`$ARDA_TEST_WORLD` first) |
| `target/arda-midzoom-fixture/micro42` | `crates/arda-midzoom/tests/world.rs:43` (`$ARDA_MIDZOOM_TEST_WORLD` first, `:36`) |

Root release-gate tests build per-process worlds without the stamp mechanism:
`target/e2e-village-<pid>` (`tests/e2e_village.rs:79`, recipe 6, settled and
society-built) and `target/zoom-continuity-<tag>-<pid>`
(`tests/zoom_continuity.rs:62`, guarded by an in-process `Mutex<BTreeSet>`),
overridable by `ARDA_ZOOM_WORLD` / `ARDA_ZOOM_ARID_WORLD`.

The facade and CLI suites generate MICRO 42 once per test binary into a
`LazyLock` in-memory snapshot and restore copies into separate temp dirs
(`crates/arda/tests/round_trip.rs:36`, `crates/arda-cli/tests/cli.rs:63`).

### Ignored tests

| Category | Tests |
|---|---|
| Wall-clock release gates (`"wall-clock timing; release gate"`) | `crates/arda-midzoom/tests/checks.rs:335`, `crates/arda-midzoom/tests/world.rs:166`, `crates/arda-names/tests/performance.rs:12`, `crates/arda-npc/tests/performance.rs:45`, `crates/arda-refine/tests/perf.rs:30`, `:43`, `crates/arda-server/tests/world_api/tactical_images.rs:122` |
| Other timing | `crates/arda-tactical/tests/timing.rs:12` (64×64 squares at 128 px < 2 s), `crates/arda-society/tests/scale.rs:80` (4,000 settlements, `ARDA_SOCIETY_TILES`), `:113` (2,000 vs 8,000 linearity) |
| Root release gates (generate worlds) | `tests/e2e_village.rs:296`, `tests/zoom_continuity.rs:162`, `:230`, `tests/zoom_land.rs:175`, `:212` |
| Need an external world | `crates/arda-people/tests/town_wfc.rs:10` (`ARDA_WORLD`), `crates/arda-refine/tests/real_world.rs:22`, `:67` (`ARDA_REFINE_WORLD`, default `../../out/micro42`) |
| Extra seeds | `crates/arda-settle/tests/realms_micro.rs:152` (seed 3), `:158` (seed 7) |
| Measurement probes (arda-gen) | `crates/arda-gen/src/area/terrain_correction_probe.rs:15`, `src/area/lakes/tests/diagonal.rs:84`, `src/continent/tests.rs:128`, `tests/anisotropy_probe.rs:293`, `:308`, `tests/coast_measurement.rs:109`, `:158` (`ARDA_COAST_MEASUREMENT_OUT`), `tests/continent_measures.rs:22`, `:58` |

The NPC timing bound was moved into its own ignored test after it flaked under
parallel workspace load (`crates/arda-npc/tests/performance.rs:4`).
Benches carry explicit assertions: one area's erosion under 30 s
(`crates/arda-gen/benches/area_erosion.rs:20`, `:50`), the coarse continent stage
within 60 s (`crates/arda-gen/benches/continent_stage.rs:17`, `:37`).

### Schema and bindings checks

| Test | Line | Checks |
|---|---|---|
| `committed_bindings_are_fresh` | `crates/arda-server/src/bindings.rs:205` | ts-rs output equals `bindings/ts/arda/` (117 files) by file set and content; `ARDA_BLESS_BINDINGS` rewrites (`:208`) |
| `u64_ids_are_strings_not_bigints` | `crates/arda-server/src/bindings.rs:238` | no `bigint` in any binding |
| `npc_types_are_exported_with_string_ids` | `crates/arda-server/src/bindings.rs:246` | NPC types and `index.ts` re-exports |
| `tactical_layout_binding_describes_the_layout_json` | `crates/arda-server/src/bindings.rs:276` | layout/asset binding shapes |
| `committed_schemas_are_fresh` | `crates/arda-server/src/schema/tests.rs:4` | schemars output equals `bindings/schema/` (31 files); same bless switch (`:7`) |
| `rendering_is_deterministic_and_every_schema_is_2020_12` | `crates/arda-server/src/schema/tests.rs:37` | repeat render equality, `$schema` dialect |
| `crates/arda-server/tests/world_api/schemas.rs` (7 tests) | `:27` | live responses validated against the served `/v1/schema/{name}.json` with `jsonschema::draft202012` |

The viewer imports the committed bindings through the `@arda` alias
(`apps/viewer/vite.config.ts:7`); it does not regenerate them.

### Determinism tests

| Area | Tests |
|---|---|
| Whole world | `tests/golden_world.rs:99`, `tests/golden_recipes.rs:65`, `crates/arda/tests/round_trip.rs:151`, `crates/arda-gen/tests/continent_hydrology.rs:219` |
| Thread-count independence (rayon pool of 1 vs default) | `crates/arda-gen/src/formation/form_tests.rs:24`, `crates/arda-gen/src/formation/lattice.rs:287`, `crates/arda-gen/src/formation/plains_tests.rs:132`, `crates/arda-gen/src/formation/recipe_tests.rs:53` |
| Reroll sequence | `crates/arda-gen/src/orchestrator/tests.rs:24` |
| Render bytes on repeat | `crates/arda-render/src/lib.rs:197`, `:206`, `:265`; `crates/arda/tests/area_exports.rs:351`; `crates/arda-dungeon/tests/levels.rs:195` (pinned blake3 of two renders, `tests/golden/renders.txt`) |
| Downstream generators | `crates/arda-society/tests/determinism.rs:11`, `crates/arda-refine/tests/determinism.rs:15` (+ order independence `:25`, window = blocks `:50`), `crates/arda-names/tests/determinism.rs:28` (+ `golden_names_are_stable` `:68`), `crates/arda-npc/tests/determinism.rs:11` (+ building/expansion order `:22`, `:45`), `crates/arda-wfc/tests/solve.rs:48`, `crates/arda-tactical/src/compose/tests_pose.rs:78`, `crates/arda-server/src/tiles.rs:243` |

## Doubles

No mocking framework is declared: the only dev-dependencies are `serde_json`,
`criterion`, `blake3`, `rayon`, `tokio`, `http-body-util`, `tower`,
`jsonschema` and sibling workspace crates (`crates/*/Cargo.toml`
`[dev-dependencies]`; root `Cargo.toml:86`). Collaborators are real; fakes are
synthetic inputs.

| Technique | Examples |
|---|---|
| Synthetic inputs shipped in the library (`pub mod`) so downstream tests reuse them | `arda_refine::synthetic` (`crates/arda-refine/src/lib.rs:46`, re-exported by `crates/arda-refine/tests/common/mod.rs:5`), `arda_society::synthetic` (`crates/arda-society/src/lib.rs:35`), `arda_settle::synthetic` (`crates/arda-settle/src/lib.rs:43`), `arda_fields::synthetic` (`crates/arda-fields/src/lib.rs:34`), `arda_town::samples` (`crates/arda-town/src/lib.rs:26`), `arda_npc::sample` (`crates/arda-npc/src/lib.rs:28`) |
| Per-binary shared fixtures in `tests/common/mod.rs` | `crates/arda-npc/tests/common/mod.rs:1`, `crates/arda-scene/tests/common/mod.rs:1`, `crates/arda-town/tests/common/mod.rs:1` (four plans built once per binary), `crates/arda-ways/tests/common/mod.rs:1`, `crates/arda-society/tests/common/mod.rs:1` |
| Images generated in code instead of binary fixtures | `crates/arda-art-import/tests/fixtures/mod.rs:1` |
| Real generated worlds | the `target/` fixtures and `LazyLock` snapshots in Layout |
| In-process HTTP | `tower::ServiceExt` requests against `arda_server::router` without a socket (`crates/arda-server/tests/world_api/support.rs:15`); one shared `AppState` so caches warm across tests (`:71`) |
| Real loopback socket | `crates/arda-server/tests/serve.rs:13` binds `127.0.0.1:0` to test header timeouts and connection limits with shortened `ServeLimits` (`:20`) |
| Committed asset library | `assets/tactical/placeholder` (`crates/arda-server/tests/world_api/support.rs:66`, `crates/arda-tactical/tests/timing.rs:14`) |
| Temp directories with `Drop` cleanup | `tests/golden_world.rs:15`, `tests/golden_recipes.rs:17` |
| Stand-in HTTP service (JS) | `tools/art-gen/test/mock-slopify.test.mjs:1` replaces Slopify's API on `127.0.0.1`; `art-gen.test.mjs` runs `generate.mjs --dry-run` with `fetch` replaced by a failing function and skips Slopify-schema checks without `SLOPIFY_DIR` (`tools/art-gen/test/art-gen.test.mjs:4`–`10`) |

## Coverage shape

From file inspection (no coverage tool is configured in the workspace or CI).
Module breakdowns below count test attributes per top-level `src/` entry;
individual test bodies were not read (shallow).

| Area | Tests | Shape |
|---|---:|---|
| `arda-gen` | 557 unit + 29 integration | `hydrology/` 159, `continent/` 100, `orchestrator/` 92, `area/` 84, `formation/` 80, spectral/noise files 34, `block/` 8; 9 of 29 ignored tests are measurement probes here |
| `arda-render` | 147 unit, 0 integration | `atlas/` 51, `overview/` 34, `carto/` 15, `channels/` 14, `channel_geometry/` 12 |
| `arda-server` | 84 unit + 73 integration | `tactical/` 20, `contract/` 13, `sheet_map/` 9; `world_api/` endpoints 11, tactical 11, cells 8, schemas 7 |
| `arda-core` | 115 unit, 0 integration | `formats/` 79 |
| `arda-tactical` | 109 unit + 1 ignored integration | `compose/` 60, `validate_tests.rs` 20 |
| `arda-npc`, `arda-scene`, `arda-town`, `arda-refine`, `arda-society`, `arda-ways` | 29–48 integration each | mostly black-box through `tests/`, few unit tests (`arda-society` 4, `arda-ways` 5) |
| `arda-dungeon` | 0 unit, 9 integration | `crates/arda-dungeon/src/` (7 files) has no `#[test]` |
| `arda-ids` | 12 unit, 0 integration | |
| `arda-people` | 3 unit + 5 integration | top-level `build.rs`, `files.rs`, `people.rs`, `rivers.rs`, `town.rs` carry no unit tests |
| `arda-settle` | 29 unit + 28 integration | 22 top-level `src/` files carry no unit tests (e.g. `pipeline.rs`, `suitability.rs`, `grid.rs`); covered through `tests/micro.rs`, `realms_micro.rs` |
| `arda-cli` | 15 unit + 18 integration | `generate.rs`, `society.rs`, `tactical.rs`, `tactical_import.rs`, `dungeon.rs` carry no unit tests; `tests/cli.rs` (14) and `tests/tactical.rs` (4) run the binary |
| `arda` facade | 11 unit + 21 integration | `world.rs`, `atlas.rs` carry no unit tests; `round_trip.rs` and `area_exports.rs` exercise them |
| `arda-server` modules without unit tests | — | `derived.rs`, `dto.rs`, `npc.rs`, `people.rs`, `relief.rs`, `serve.rs` (`serve.rs` has `tests/serve.rs`; the others are reached through `world_api/`) |
| Cross-crate pipeline | 17 root | golden world and recipes run in CI; village end-to-end and zoom continuity are ignored release gates |
| Viewer | 86 | `api/client.test.ts` 24, `api/tactical.test.ts` 17, `geo/coords.test.ts` 13; React components (`.tsx`) have no tests |
| Art tooling | 31 | prompts, payload schemas, dry-run and mocked submit/poll/download paths |

No property-based testing crate, fuzzing target, snapshot crate or coverage tool
is declared in any `Cargo.toml` or in `apps/viewer/package.json`.
