---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 90ef2e787c56
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
  - ":(top)contract-kit/**"
  - ":(top)tools/release-gates.sh"
  - ":(top)tools/release.sh"
  - ":(top)apps/viewer/scripts/**"
  - ":(top)bindings/schema-baseline/**"
---

# Testing

Counts below are `#[test]` / `#[tokio::test]` attributes counted from source at
the commit this chapter is stamped with (no test run): 1,281 in `crates/*/src/`,
590 in `crates/*/tests/`, 17 in root `tests/`, 1,888 in total, of which 36 carry
`#[ignore]` and 105 are `#[tokio::test]`. Viewer and art-gen counts are
`it(`/`test(` calls.

## Layout

### Where tests live

| Suite | Location | Mapping to source |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` blocks and sibling `*_tests.rs` / `tests/` modules inside `crates/<crate>/src/` | Same module as the code (e.g. `crates/arda-gen/src/formation/form_tests.rs:24`, `crates/arda-server/src/bindings.rs:236`) |
| Crate integration tests | `crates/<crate>/tests/*.rs`, one test binary per top-level file; subdirectories are modules (`crates/arda-server/tests/world_api/main.rs:6`) | Public API of that crate only |
| Workspace tests | root `tests/`, hosted by the root package `arda-workspace-tests` (`Cargo.toml:77`), `autotests = false` (`Cargo.toml:78`), empty lib `tests/empty_lib.rs` (`Cargo.toml:84`) | Cross-crate: generation → settle → society → server |
| Benches | `crates/arda-gen/benches/area_erosion.rs`, `continent_stage.rs`, `harness = false` (`crates/arda-gen/Cargo.toml:21`) | arda-gen timing budgets |
| Doctests | one fence at `crates/arda-names/src/lib.rs:8`; every other fence under `crates/*/src/` is `text` (e.g. `crates/arda-ids/src/hash.rs:13`) | arda-names README example |
| Viewer | `apps/viewer/src/**/*.test.ts`, Vitest, `environment: "node"`, include `src/**/*.test.ts` (`apps/viewer/vite.config.ts:20`) | Co-located with `.ts` source |
| Art tooling | `tools/art-gen/test/art-gen.test.mjs`, `mock-slopify.test.mjs`, `safety.test.mjs`, `node:test`; stub server helper `fake-slopify.mjs` | `tools/art-gen/*.mjs` (`safety.mjs` for the spend caps) |

Root `[[test]]` targets are declared explicitly (`Cargo.toml:104`–`126`):
`golden_world`, `golden_recipes`, `arid_recipe7`, `e2e_village`, `zoom_continuity`,
`fixture_dir`. `tests/zoom_land.rs` is not a target of its own; it is the module
`mod zoom_land;` of `zoom_continuity` (`tests/zoom_continuity.rs:14`; its module header says so, `tests/zoom_land.rs:1`).
`tests/support/fixture_dir.rs` is included by `#[path]` (`tests/fixture_dir.rs:7`).

### Per-crate inventory

| Crate | Unit (`src/`) | Integration (`tests/`) | Ignored | Integration files |
|---|---:|---:|---:|---|
| `arda` | 11 | 21 | 0 | `round_trip.rs`, `area_exports.rs` (+`area_exports/reload.rs`) |
| `arda-art-import` | 30 | 12 | 0 | `holes.rs`, `import.rs`, `fixtures/mod.rs` |
| `arda-blocks` | 4 | 7 | 0 | `compose.rs`, `settlements.rs` |
| `arda-cli` | 15 | 17 | 0 | `cli.rs`, `tactical.rs` |
| `arda-core` | 105 | 0 | 0 | none |
| `arda-dungeon` | 0 | 10 | 0 | `levels.rs`, `golden/renders.txt` |
| `arda-fields` | 20 | 24 | 0 | `complete.rs`, `cores.rs`, `landuse.rs`, `mines.rs`, `partition.rs`, `rules.rs`, `seams.rs`, `support/shape.rs` |
| `arda-gen` | 549 | 29 | 9 | `anisotropy_probe.rs`, `coast_measurement.rs`, `continent_hydrology.rs`, `continent_measures.rs`, `cross_tile.rs` (+`oracles`, `seam_lakes`, `seams`), `drainage_invariants.rs` |
| `arda-ids` | 12 | 0 | 0 | none |
| `arda-midzoom` | 8 | 19 | 2 | `checks.rs`, `export.rs`, `labels.rs`, `land.rs`, `world.rs` |
| `arda-names` | 15 | 25 | 1 | `blocked_roots.rs`, `determinism.rs`, `dialects.rs`, `distinctness.rs`, `feature_kinds.rs`, `json_ids.rs`, `performance.rs`, `phonotactics.rs`, `serde_keys.rs`, `uniqueness.rs` |
| `arda-npc` | 7 | 57 | 1 | 15 files incl. `determinism.rs`, `performance.rs`, `stat_blocks.rs`, `srd_math.rs`, `common/mod.rs` |
| `arda-people` | 5 | 5 | 1 | `interiors.rs`, `notable_mix.rs`, `rivers.rs`, `town_wfc.rs` |
| `arda-refine` | 43 | 41 | 4 | 13 files incl. `determinism.rs`, `perf.rs`, `real_world.rs`, `seams.rs`, `schema.rs` |
| `arda-render` | 147 | 0 | 0 | none |
| `arda-scene` | 12 | 49 | 0 | `derive.rs`, `hostile.rs`, `light_parity.rs`, `rules.rs`, `scene.rs`, `walls_los.rs`, `common/mod.rs` |
| `arda-server` | 116 | 116 | 8 | `contract_kit.rs` (5), `serve.rs` (4) and `world_api/` (107 across 21 modules, 7 of the ignored in `gates.rs`) |
| `arda-settle` | 32 | 39 | 2 | `band.rs`, `gazetteer.rs`, `memory.rs`, `micro.rs`, `names.rs`, `names_stable.rs`, `realms_micro.rs`, `render.rs`, `society.rs` (+`society/realms.rs`), `tilelabels.rs`, `write.rs` |
| `arda-society` | 4 | 43 | 2 | 10 files incl. `determinism.rs`, `scale.rs`, `trade_scale.rs` |
| `arda-tactical` | 115 | 1 | 1 | `timing.rs` |
| `arda-town` | 20 | 41 | 0 | `blocks.rs`, `interiors.rs`, `plan.rs`, `rivers.rs`, `wfc.rs`, `wfc_rows.rs`, `common/mod.rs` |
| `arda-ways` | 5 | 29 | 0 | `fallback.rs`, `ids.rs`, `reach.rs`, `rules_sidecar.rs`, `standing.rs`, `ways.rs` (+`ways/refusals.rs`), `wet.rs` |
| `arda-wfc` | 6 | 5 | 0 | `solve.rs` |
| root `tests/` | — | 18 | 5 | `golden_world.rs` (2), `golden_recipes.rs` (6), `arid_recipe7.rs` (2), `e2e_village.rs` (1), `zoom_continuity.rs` + `zoom_land.rs` (4), `fixture_dir.rs` (3) |
| `apps/viewer` | — | 86 + 6 | — | 12 Vitest files: `api/{areaBin,client,scene,tactical}`, `format`, `hooks`, `geo/{cellWalk,coords,handoff,relief,tacticalTiles}`, `render/tacticalOverlay`; plus `scripts/check-licences.test.mjs` (6, `node --test scripts/`) |
| `tools/art-gen` | — | 43 | — | `art-gen.test.mjs` (28), `mock-slopify.test.mjs` (3), `safety.test.mjs` (12); shared stub `fake-slopify.mjs` |

### Commands

| Check | Command | Source |
|---|---|---|
| Workspace tests (CI) | `cargo test --workspace --all-features --no-fail-fast` | `.github/workflows/ci.yml:17` |
| Workspace tests (README) | `cargo test --workspace --all-features` | `README.md:432` |
| Formatting | `cargo fmt --all --check` | `.github/workflows/ci.yml:37` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | `.github/workflows/ci.yml:38` |
| MSRV | `cargo check --workspace --all-targets --all-features --locked` on toolchain 1.96.1 | `.github/workflows/ci.yml:26` |
| Release verify (tag) | `tools/release.sh --check`, fmt, clippy `--locked`, `cargo test --workspace --locked` on the tag's commit | `.github/workflows/release.yml` job `verify` |
| Dependency policy | `EmbarkStudios/cargo-deny-action` (v2.1.1, SHA-pinned) against `deny.toml` | `.github/workflows/ci.yml:44` |
| Golden world bless | `ARDA_BLESS=1 cargo test --test golden_world` | `tests/golden_world.rs:4` |
| Recipe goldens bless | `ARDA_BLESS=1 cargo test --test golden_recipes` | `tests/golden_recipes.rs:12` |
| Bindings/schema freshness | `cargo test -p arda-server bindings` / `schema`; bless with `ARDA_BLESS_BINDINGS=1` | `crates/arda-server/src/bindings.rs:4`, `crates/arda-server/src/schema/mod.rs:9` |
| Schema freeze baseline | `ARDA_BLESS_SCHEMA_BASELINE=1 cargo test -p arda-server --lib frozen_schemas` rewrites `bindings/schema-baseline/v1/` (schemas plus `versions.json`); an explicit step, committed on its own, after a family bump or to adopt additions ([versioning.md](versioning.md) § Freeze guard) | `crates/arda-server/src/schema/tests.rs:161` |
| Dungeon render bless | `ARDA_BLESS_DUNGEON=1 cargo test -p arda-dungeon` | `crates/arda-dungeon/tests/levels.rs:208` |
| Contract kit bless | `ARDA_BLESS_CONTRACT=1 cargo test --release -p arda-server --test contract_kit` rewrites `contract-kit/golden/`, the manifest's hashes, statuses, listed headers and fixture identity, and `contract-kit/world.txt`; never adds or removes cases (`tools/release.sh <version>` runs it) | `crates/arda-server/tests/contract_kit.rs` |
| Contract kit live check | `node contract-kit/check.mjs --base http://127.0.0.1:8787` against a server on the canonical world; checker tests `node --test contract-kit/test/` | `contract-kit/README.md` |
| Release gates (ignored; all wall-clock budgets, one receipt) | `tools/release-gates.sh` (add `--with-benches` for the `arda-gen` benches; exit 0 on pass) | `tools/release-gates.sh`, receipt `target/release-gates/receipt-<commit>.md` |
| One gate crate by hand | e.g. `cargo test --release -p arda-server --test world_api -- --ignored --nocapture --test-threads=1` | `crates/arda-server/tests/world_api/gates.rs` |
| Bench budgets | `cargo bench -p arda-gen --bench area_erosion` / `continent_stage` (also run by `--with-benches`) | `crates/arda-gen/Cargo.toml:21` |
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
| `test` | `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` (`ci.yml:7`–`9`) | `dtolnay/rust-toolchain` (SHA-pinned), `stable` | workspace tests, including both golden targets |
| `msrv` | `ubuntu-latest` | `1.96.1` (`ci.yml:26`) | locked all-target check, no tests |
| `lint` | `ubuntu-latest` | stable + rustfmt, clippy | fmt, clippy `-D warnings` |
| `deny` | `ubuntu-latest` | action-provided | licences, bans, advisories, sources (`deny.toml`) |
| `viewer` | `ubuntu-latest`, working directory `apps/viewer` | Node 26 (`actions/setup-node`, SHA-pinned) | `npm ci`, `npm run licences`, `typecheck`, `lint`, `npm test` (Vitest plus `node --test scripts/`), `build` |

Not run by CI: `#[ignore]` tests (no `--ignored` step), Criterion benches, and the art-gen `node --test` suite.

A second workflow, `.github/workflows/release.yml`, runs on `v*` tags and on manual dispatch (a dry run that never pushes or releases):

| Job | Runner | Runs |
|---|---|---|
| `verify` | `ubuntu-latest`, Rust 1.98.1 | tag equals `Cargo.toml` version, `tools/release.sh --check`, a `CHANGELOG.md` section exists, fmt, clippy, `cargo test --workspace --locked` |
| `binaries` | matrix: `x86_64-unknown-linux-gnu` (`ubuntu-22.04`), `aarch64-unknown-linux-gnu` (`ubuntu-22.04-arm`), `aarch64-apple-darwin` (`macos-latest`), `x86_64-pc-windows-msvc` (`windows-latest`) | `cargo build --release --locked`, `tools/smoke-binaries.sh` (version checks everywhere; generate seed 42 and serve `/v1/health` on Linux), `tools/package-release.sh`, upload |
| `image` | `ubuntu-latest` | `tools/docker-smoke.sh` on the amd64 placeholder image, then `buildx` `linux/amd64,linux/arm64` with SBOM and max provenance; pushes to GHCR only on a real tag |
| `release` | `ubuntu-latest`, tags only | `SHA256SUMS`, notes from `tools/release.sh --notes`, `gh release create --draft` |

The ignored wall-clock release gates are not part of either workflow; the maintainer runs them before tagging (task 09's gate runner).

### Golden world test

`tests/golden_world.rs` generates MICRO seed 42 with `arda::generate` into a temp
directory once per process (`LazyLock`, `tests/golden_world.rs:69`) and
fingerprints every file as `<relative path>  <blake3 hex>`, sorted, with `\` →
`/` normalisation (`tests/golden_world.rs:38`, `:63`).

| Test | Line | Assertion |
|---|---|---|
| `micro_world_matches_the_golden_fingerprint` | `tests/golden_world.rs:79` | fingerprint equals `tests/golden/micro-42.txt` (26 entries); with `ARDA_BLESS` set it rewrites the file and returns (`:82`) |
| `regenerating_produces_the_same_fingerprint` | `tests/golden_world.rs:99` | a second, independent generation matches the first |

`tests/golden_recipes.rs` pins the fine-terrain recipes: each `check(recipe)`
(`tests/golden_recipes.rs:67`) generates MICRO 42 with
`generate_from_fine_recipe`, renders an Atlas 4K overview and compares the file
fingerprints plus the PNG hash with `tests/golden/micro-42-recipe<N>.txt`
(`:103`). Tests: `recipe_5_replays_v0_1_byte_for_byte` (`:156`),
`recipe_6_replays_v0_2_byte_for_byte` (`:161`), `recipe_7_is_pinned` (`:167`),
`recipe_8_is_pinned` (`:174`), `recipe_9_is_pinned` (`:181`),
`unknown_recipes_are_refused_before_output` (`:186`). The module doc states
recipe-5 hashes must never be re-blessed (`:13`). `world.json` is excluded from
the hash and checked field by field (`:8`). The arda-gen unit test
`formation::mountain_tests` adds a mountainous recipe-9 world (a 40 km island
with a 3.6 km massif, about 5 s) to the lowland MICRO 42 fixtures.

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
| `target/arda-server-fixture/micro42` | `crates/arda-server/tests/world_api/support.rs:32` (`$ARDA_SERVER_TEST_WORLD`, then `out/micro42` if it loads at the current `FINE_TERRAIN_RECIPE_VERSION` and seed 42, `:21`, `:38`, else this fixture, `:42`); `crates/arda-settle/tests/micro.rs:33` (`$ARDA_TEST_WORLD` first, `:25`) |
| `target/arda-server-fixture/micro42-society` | `crates/arda-server/tests/world_api/support.rs:175`: generate + `arda_settle::generate` + `arda_people::build`, used when the base world has no `society/notables.json` (`:159`) |
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
| Server release gates (new, `crates/arda-server/tests/world_api/gates.rs`; best of 3, society fixture) | cached cell WebP tile (< 20 ms), cold cell JSON plus scene (< 500 ms), cold 32×32 window PNG at 128 ppsq (< 1 s), `AppState::open` (< 2 s), `World::load` (< 100 ms), area cells JSON (< 500 ms) and bin (< 250 ms), warm overview tile (< 20 ms) |
| Wall-clock release gates (`"wall-clock timing; release gate"`, each prints a `GATE name measured_ms budget_ms` line) | `tests/e2e_village.rs:304`, `crates/arda-tactical/tests/timing.rs:12`, `crates/arda-midzoom/tests/checks.rs:335`, `crates/arda-midzoom/tests/world.rs:166`, `crates/arda-names/tests/performance.rs:12`, `crates/arda-npc/tests/performance.rs:45`, `crates/arda-refine/tests/perf.rs:30`, `:46`, `crates/arda-server/tests/world_api/tactical_images.rs:122` |
| Other timing | `crates/arda-society/tests/scale.rs:80` (4,000 settlements, `ARDA_SOCIETY_TILES`), `:113` (2,000 vs 8,000 linearity) |
| Root release gates (generate worlds; not timing) | `tests/zoom_continuity.rs:162`, `:230`, `tests/zoom_land.rs:180`, `:217` |
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
| `committed_bindings_are_fresh` | `crates/arda-server/src/bindings.rs:237` | ts-rs output equals `bindings/ts/arda/` (134 files) by file set and content; `ARDA_BLESS_BINDINGS` rewrites (`:240`) |
| `u64_ids_are_strings_not_bigints` | `crates/arda-server/src/bindings.rs:270` | no `bigint` in any binding |
| `npc_types_are_exported_with_string_ids` | `crates/arda-server/src/bindings.rs:278` | NPC types and `index.ts` re-exports |
| `tactical_layout_binding_describes_the_layout_json` | `crates/arda-server/src/bindings.rs:339` | layout/asset binding shapes |
| `committed_schemas_are_fresh` | `crates/arda-server/src/schema/tests.rs:5` | schemars output equals `bindings/schema/` (46 files); same bless switch (`:8`) |
| `rendering_is_deterministic_and_every_schema_is_2020_12` | `crates/arda-server/src/schema/tests.rs:38` | repeat render equality, `$schema` dialect |
| `crates/arda-server/tests/world_api/schemas.rs` (9 tests) | `:27` | live responses validated against the served `/v1/schema/{name}.json` with `jsonschema::draft202012` |
| `index_exports_every_version_family` | `crates/arda-server/src/bindings.rs:308` | `index.ts` exports `API_VERSION` and one constant per `VersionsDto` family |
| `frozen_schemas_keep_their_shape_unless_their_family_is_bumped` | `crates/arda-server/src/schema/tests.rs:161` | freeze guard: every baseline schema in `bindings/schema-baseline/v1/` against the current one (`src/schema/compat.rs`): a removed property, a newly required one, a changed `type`/`format`/`const`/`$ref`, a removed enum value or variant, or a value added to a closed enum fails unless the owning family's version differs from the baseline's `versions.json`; `ARDA_BLESS_SCHEMA_BASELINE=1` re-baselines |
| `every_schema_names_a_known_family` | `crates/arda-server/src/schema/tests.rs:215` | each catalogue entry's family is a `versions` key or `api_version` |
| `every_open_enum_exists_and_the_policy_page_lists_it` | `crates/arda-server/src/schema/tests.rs:226` | `OPEN_ENUMS` are real `$defs` and `versioning.md` lists each with its fallback |
| `schema::compat::tests` (9 tests) | `crates/arda-server/src/schema/compat.rs` | one synthetic schema pair per rule: removal, newly required, type/format change, enum removal, closed-enum addition fail; an added optional property and an open-enum addition pass; a family bump excuses; `$defs` ownership |
| `every_stable_route_of_the_policy_answers` | `crates/arda-server/tests/world_api/inventory.rs` | route inventory: every **stable** row of `versioning.md` has a probe and answers on the MICRO society world with neither `404` (except an unknown bake id from the handler) nor `405` |
| `world_and_health_name_the_server_build_and_every_version_family` | `crates/arda-server/tests/world_api/endpoints.rs` | `server_version` on `/v1/health` and `/v1/world`, `arda_version` from the manifest, and every `versions` entry equal to its constant |

The viewer imports the committed bindings through the `@arda` alias
(`apps/viewer/vite.config.ts:8`); it does not regenerate them.

### Contract kit

`contract-kit/` (1.0 task 10; `contract-kit/README.md`) pins the bytes a `/v1` client such as
DiceMaster may rely on: 55 cases over 42 of the 43 stable routes of
[versioning.md](versioning.md) (the finished bake's tiles depend on when the background job
ends), each with its status, content type, listed headers (`etag`, `cache-control`,
`location`) and either a golden body under `contract-kit/golden/` or the SHA-256 and size
(images, the area columns, the window body). The world is the canonical one: MICRO seed 42,
fine terrain, settled and society-built, whose file fingerprint is `contract-kit/world.txt`;
the library is the committed placeholder. About 2.8 MB.

| Test | Line | Checks |
|---|---|---|
| `every_case_matches_its_golden` | `crates/arda-server/tests/contract_kit.rs` | every case replayed in-process, in manifest order, against the stamped fixture `target/arda-server-fixture/micro42-society` with `ServerConfig::new` defaults (the CLI's) and the placeholder library: status, content type, headers, golden bytes or hash; hashed JSON bodies validate against their schema; `ARDA_BLESS_CONTRACT` rewrites |
| `every_json_golden_validates_against_its_schema` | same file | each golden against the served `/v1/schema/{schema}.json` (`jsonschema::draft202012`) |
| `the_fixture_identity_is_the_served_world` | same file | manifest `fixture` (seed, recipe, versions, `library_version`, BLAKE3 of `world.json`) equals the served world, and `world.txt` the fixture's fingerprint |
| `cases_are_unique_and_well_formed` | same file | unique ids, one of golden or hash, `If-None-Match` and bake-delete references point at earlier cases |
| `every_stable_route_of_the_policy_has_a_case` | same file | every **stable** row of versioning.md has a case, and every case's route is stable |
| `contract-kit/test/check.test.mjs` (5 tests) | `node --test contract-kit/test/` | the live checker against a `node:http` stub: pass, a changed JSON value named by its path, a changed image and its 304, an unreachable server (exit 2) |

The live acceptance: build the canonical world with the release CLI (`arda generate --seed 42
--micro --terrain fine`, `arda settle`, `arda society build`), serve it with
`arda-server --world <dir>`, run `node contract-kit/check.mjs --base <url>`. On 0.8.1 the CLI
world was byte-identical to the test fixture (47 files), and the checker passed cold (about
9 s) and warm (under 1 s).

### Determinism tests

| Area | Tests |
|---|---|
| Whole world | `tests/golden_world.rs:99`, `tests/golden_recipes.rs:67`, `crates/arda/tests/round_trip.rs:154`, `crates/arda-gen/tests/continent_hydrology.rs:219` |
| Thread-count independence (rayon pool of 1 vs default) | `crates/arda-gen/src/formation/form_tests.rs:24`, `crates/arda-gen/src/formation/lattice.rs:287`, `crates/arda-gen/src/formation/plains_tests.rs:132`, `crates/arda-gen/src/formation/recipe_tests.rs:53` |
| Reroll sequence | `crates/arda-gen/src/orchestrator/tests.rs:24` |
| Render bytes on repeat | `crates/arda-render/src/lib.rs:183`, `:222`; `crates/arda/tests/area_exports.rs:351`; `crates/arda-dungeon/tests/levels.rs:210` (pinned blake3 of two renders, `tests/golden/renders.txt`) |
| Downstream generators | `crates/arda-society/tests/determinism.rs:11`, `crates/arda-refine/tests/determinism.rs:15` (+ order independence `:25`, window = blocks `:50`), `crates/arda-names/tests/determinism.rs:28` (+ `golden_names_are_stable` `:68`), `crates/arda-npc/tests/determinism.rs:11` (+ building/expansion order `:22`, `:45`), `crates/arda-wfc/tests/solve.rs:48`, `crates/arda-tactical/src/compose/tests_pose.rs:78`, `crates/arda-server/src/tiles.rs:243` |

## Doubles

No mocking framework is declared: the only dev-dependencies are `serde_json`,
`criterion`, `blake3`, `rayon`, `tokio`, `http-body-util`, `tower`,
`jsonschema` and sibling workspace crates (`crates/*/Cargo.toml`
`[dev-dependencies]`; root `Cargo.toml:87`). Collaborators are real; fakes are
synthetic inputs.

| Technique | Examples |
|---|---|
| Synthetic inputs shipped in the library (`pub mod`) so downstream tests reuse them | `arda_refine::synthetic` (`crates/arda-refine/src/lib.rs:47`, re-exported by `crates/arda-refine/tests/common/mod.rs:5`), `arda_society::synthetic` (`crates/arda-society/src/lib.rs:36`), `arda_settle::synthetic` (`crates/arda-settle/src/lib.rs:44`), `arda_fields::synthetic` (`crates/arda-fields/src/lib.rs:34`), `arda_town::samples` (`crates/arda-town/src/lib.rs:26`), `arda_npc::sample` (`crates/arda-npc/src/lib.rs:29`) |
| Per-binary shared fixtures in `tests/common/mod.rs` | `crates/arda-npc/tests/common/mod.rs:1`, `crates/arda-scene/tests/common/mod.rs:1`, `crates/arda-town/tests/common/mod.rs:1` (four plans built once per binary), `crates/arda-ways/tests/common/mod.rs:1`, `crates/arda-society/tests/common/mod.rs:1` |
| Images generated in code instead of binary fixtures | `crates/arda-art-import/tests/fixtures/mod.rs:1` |
| Real generated worlds | the `target/` fixtures and `LazyLock` snapshots in Layout |
| In-process HTTP | `tower::ServiceExt` requests against `arda_server::router` without a socket (`crates/arda-server/tests/world_api/support.rs:15`); one shared `AppState` so caches warm across tests (`:71`) |
| Real loopback socket | `crates/arda-server/tests/serve.rs:13` binds `127.0.0.1:0` to test header timeouts and connection limits with shortened `ServeLimits` (`:20`) |
| Committed asset library | `assets/tactical/placeholder` (`crates/arda-server/tests/world_api/support.rs:66`, `crates/arda-tactical/tests/timing.rs:14`) |
| Temp directories with `Drop` cleanup | `tests/golden_world.rs:15`, `tests/golden_recipes.rs:19` |
| Stand-in HTTP service (JS) | `tools/art-gen/test/mock-slopify.test.mjs:1` replaces Slopify's API on `127.0.0.1`; `art-gen.test.mjs` runs `generate.mjs --dry-run` with `fetch` replaced by a failing function and skips Slopify-schema checks without `SLOPIFY_DIR` (`tools/art-gen/test/art-gen.test.mjs:4`–`10`) |

## Coverage shape

From file inspection (no coverage tool is configured in the workspace or CI).
Module breakdowns below count test attributes per top-level `src/` entry;
individual test bodies were not read (shallow).

| Area | Tests | Shape |
|---|---:|---|
| `arda-gen` | 549 unit + 29 integration | `hydrology/` 159, `continent/` 100, `orchestrator/` 92, `area/` 84, `formation/` 80, spectral/noise files 34; 9 of 29 ignored tests are measurement probes here |
| `arda-render` | 147 unit, 0 integration | `atlas/` 51, `overview/` 34, `carto/` 15, `channels/` 14, `channel_geometry/` 13 |
| `arda-server` | 116 unit + 116 integration | `tactical/` 31, `contract/` 13, `sheet_map/` 9; `world_api/` endpoints 12, tactical 11, cells 9, schemas 9, gates 7 |
| `arda-core` | 105 unit, 0 integration | `formats/` 71 |
| `arda-tactical` | 115 unit + 1 ignored integration | `compose/` 66, `validate_tests.rs` 20 |
| `arda-npc`, `arda-scene`, `arda-town`, `arda-refine`, `arda-society`, `arda-ways` | 29–48 integration each | mostly black-box through `tests/`, few unit tests (`arda-society` 4, `arda-ways` 5) |
| `arda-dungeon` | 0 unit, 10 integration | `crates/arda-dungeon/src/` (7 files) has no `#[test]` |
| `arda-ids` | 12 unit, 0 integration | |
| `arda-people` | 5 unit + 5 integration | top-level `build.rs`, `people.rs`, `rivers.rs`, `town.rs` carry no unit tests |
| `arda-settle` | 32 unit + 39 integration | 21 top-level `src/` files carry no unit tests (e.g. `pipeline.rs`, `suitability.rs`, `grid.rs`); covered through `tests/micro.rs`, `realms_micro.rs` |
| `arda-cli` | 15 unit + 17 integration | `generate.rs`, `society.rs`, `tactical.rs`, `tactical_import.rs`, `dungeon.rs` carry no unit tests; `tests/cli.rs` (13) and `tests/tactical.rs` (4) run the binary |
| `arda` facade | 11 unit + 21 integration | `world.rs`, `atlas.rs` carry no unit tests; `round_trip.rs` and `area_exports.rs` exercise them |
| `arda-server` modules without unit tests | — | `derived.rs`, `dto.rs`, `gazetteer.rs`, `npc.rs`, `people.rs`, `realms.rs`, `relief.rs`, `serve.rs`, `versions.rs` (`serve.rs` has `tests/serve.rs`; the others are reached through `world_api/`) |
| Cross-crate pipeline | 17 root | golden world and recipes run in CI; village end-to-end and zoom continuity are ignored release gates |
| Viewer | 86 + 6 | `api/client.test.ts` 24, `api/tactical.test.ts` 17, `geo/coords.test.ts` 13; React components (`.tsx`) have no tests |
| Art tooling | 43 | prompts, payload schemas, dry-run, mocked submit/poll/download paths, spend-cap safety (`safety.test.mjs`) |

No property-based testing crate, fuzzing target, snapshot crate or coverage tool
is declared in any `Cargo.toml` or in `apps/viewer/package.json`.
