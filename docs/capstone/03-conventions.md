---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 00ffb9412f1e
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top)deny.toml"
  - ":(top)rust-toolchain.toml"
  - ":(top)CHANGELOG.md"
  - ":(top).github/workflows/**"
  - ":(top)crates/*/Cargo.toml"
  - ":(top)crates/*/src/**"
  - ":(top)crates/*/tests/**"
  - ":(top)crates/*/data/**"
  - ":(top)crates/arda-server/mappings/**"
  - ":(top)tests/**"
  - ":(top)apps/viewer/tsconfig.app.json"
---

# Conventions

Scope: the 23 Rust crates under `crates/`, the root workspace package, CI and the design-ledger fragments. The TS viewer (`apps/viewer/`) and the Node art tooling (`tools/art-gen/`) are covered only where named (shallow).

## Paradigm

Every crate is a library of plain functions over owned or borrowed data; a stage takes its inputs as arguments and returns a value. Entry points follow a `generate`/`refine`/`build_*` naming pattern:

| Crate | Entry point | Site |
|---|---|---|
| `arda` | `generate` (world) | `crates/arda/src/lib.rs:44` |
| `arda-settle` | `generate(world_dir, budget)` | `crates/arda-settle/src/lib.rs:160` |
| `arda-society` | `simulate_society` | `crates/arda-society/src/lib.rs:134` |
| `arda-town` | `plan::generate`, `block::generate` | `crates/arda-town/src/plan/mod.rs:60`, `crates/arda-town/src/block/mod.rs:189` |
| `arda-refine` | `refine_block`, `refine` | `crates/arda-refine/src/lib.rs:67`, `crates/arda-refine/src/block.rs:167` |
| `arda-fields` | `fields_window`, `generate` | `crates/arda-fields/src/window.rs:59`, `crates/arda-fields/src/window.rs:100` |
| `arda-ways` | `apply_ways` (mutates a passed layout) | `crates/arda-ways/src/lib.rs:101` |
| `arda-dungeon` | `generate(&Params)` | `crates/arda-dungeon/src/lib.rs:227` |
| `arda-scene` | `build_scene` | `crates/arda-scene/src/lib.rs:51` |
| `arda-midzoom` | `refine_tile` | `crates/arda-midzoom/src/lib.rs:58` |
| `arda-tactical` | `placeholders::generate`, `compose::render*` | `crates/arda-tactical/src/placeholders/mod.rs:238`, `crates/arda-tactical/src/lib.rs:28` |

Product crates exchange data through JSON files and serde types, not through each other's Rust types. A consumer crate declares its own mirror of the producer's records in `src/input.rs` (or `src/site.rs`), field for field, and ignores fields it does not use: `crates/arda-ways/src/input.rs:4`, `crates/arda-fields/src/input.rs:1`, `crates/arda-society/src/lib.rs:8`, `crates/arda-npc/src/input.rs:125`. The mirrors repeat types under the same name in several crates: `Tier` (`crates/arda-settle/src/model.rs:12`, `crates/arda-town/src/site.rs:28`, `crates/arda-npc/src/input.rs:17`, `crates/arda-society/src/input.rs:12`, `crates/arda-fields/src/input.rs:149`, `crates/arda-server/src/npc_dto/input.rs:11`), `RoadClass` (seven definitions, e.g. `crates/arda-ways/src/input.rs:21`), `BuildingSpec` (four, e.g. `crates/arda-npc/src/input.rs:168`). `arda-ids` holds the canonical enums and ids that must agree byte for byte (`crates/arda-ids/src/lib.rs:3`).

The server is the one async component: axum handlers on tokio, with blocking world work moved to `spawn_blocking` (`crates/arda-server/src/routes.rs:110`, `crates/arda-server/src/main.rs:71`) and data-parallel work on rayon (`crates/arda-server/src/query.rs:15`).

Lazy shared state uses `OnceLock`: the facade's per-area caches (`crates/arda/src/world.rs:23`), embedded data tables parsed on first use (`crates/arda-npc/src/data/mod.rs:25`, `crates/arda-names/src/filter.rs:31`), and per-plan derived values (`crates/arda-town/src/plan/types.rs:338`, `crates/arda-town/src/plan/types.rs:347`). No `lazy_static`, `LazyLock` or `thread_local` in `crates/`.

Arithmetic: world generation, export and the core codec are integer/fixed-point (no `f32`/`f64` tokens in `arda-core`, `arda`, `arda-ids`, `arda-names`, `arda-npc`, `arda-society`; three in `arda-gen`). The tactical and settlement crates compute in floating point: token counts of `f64|f32` in `src/` are `arda-tactical` 716, `arda-refine` 359, `arda-fields` 328, `arda-settle` 270, `arda-town` 269, `arda-ways` 179, `arda-art-import` 141, `arda-server` 132, `arda-midzoom` 90. `arda-midzoom` documents its tile values as integer (`crates/arda-midzoom/src/lib.rs:20`).

## Naming

| Convention | Observed form | Example |
|---|---|---|
| Crate names | `arda-<domain>`; the facade is `arda`, the CLI binary is `arda` | `crates/arda-cli/Cargo.toml:10` |
| Crate error type | `<Domain>Error` in `src/error.rs` (or `lib.rs`), one per crate | `crates/arda-town/src/error.rs:8`, `crates/arda-wfc/src/lib.rs:29` |
| Result alias | `<Domain>Result<T>` where present (2 crates) | `crates/arda-server/src/error.rs:231`, `crates/arda-art-import/src/error.rs:46` |
| Per-crate RNG module | `src/rng.rs` (`arda-core`, `arda-names`, `arda-npc`, `arda-settle`, `arda-society`, `arda-town`, `arda-dungeon`); `src/hash.rs` in `arda-wfc`, `arda-refine`, `arda-ids` | `crates/arda-town/src/rng.rs:1` |
| Units in names | Suffixes `_mm`, `_m`, `_dm`, `_um`, `_km`, `_deg`, `_milli_deg`; newtypes `HeightMm`, `DischargeMilli` | `crates/arda-core/src/cell.rs:103`, `crates/arda-core/src/fixed.rs:9` |
| Grid-distinct coordinates | Separate newtypes per grid (`AreaCoord`, `CellCoord`, `CellKey`) | `crates/arda-core/src/coords.rs:3`, `crates/arda-refine/src/source.rs:15` |
| Serialized enums | `#[serde(rename_all = "snake_case")]` (120 uses, 59 files); no other `rename_all` style | `crates/arda-ids/src/road.rs:11` |
| Strict wire types | `#[serde(deny_unknown_fields)]` (87 uses, 18 files), on tactical, scene and server DTOs | `crates/arda-tactical/src/layout.rs:14` |
| Ids on the wire | u64 newtypes serialized as decimal JSON strings, integers accepted on input | `crates/arda-ids/src/ids.rs:3` |
| Format versions | Every persisted or served document has a `*_VERSION`/`*_FORMAT` constant (26 in `src/`) | `crates/arda-core/src/formats/mod.rs:20`, `crates/arda-server/src/contract/mod.rs:31`, `crates/arda-scene/src/sidecar.rs:32` |
| Per-crate README | 11 crates carry a `README.md` documenting their JSON schema or format | `crates/arda-tactical/src/lib.rs:5`, `crates/arda-scene/src/lib.rs:14` |
| `#[must_use]` | 1,731 uses in 430 files on pure functions and constructors | `crates/arda-ids/src/hash.rs:46` |

## Typing

Rust edition 2021 for every crate via `version.workspace`/`edition.workspace` (`Cargo.toml:7`). Toolchain `stable` with `rustfmt` and `clippy` (`rust-toolchain.toml:2`); CI checks MSRV 1.96.1 (`.github/workflows/ci.yml:22`). All 23 crates and the root package set `[lints] workspace = true` (`Cargo.toml:128`, e.g. `crates/arda-server/Cargo.toml:56`). Workspace lints (`Cargo.toml:50`):

```toml
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"
[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
cast_possible_truncation = "warn"
cast_possible_wrap = "warn"
cast_sign_loss = "warn"
needless_pass_by_value = "warn"
semicolon_if_nothing_returned = "warn"
```

CI runs `cargo clippy --workspace --all-targets -- -D warnings` (`.github/workflows/ci.yml:33`), so the `warn` lints fail the build. No `clippy.toml`, `rustfmt.toml` or `.rustfmt.toml` exists; `cargo fmt --all --check` runs with defaults (`.github/workflows/ci.yml:32`). 13 of 22 library crates repeat `#![deny(unsafe_code)]` at the crate root (e.g. `crates/arda-ids/src/lib.rs:16`); `arda-society` also repeats `#![deny(missing_docs)]` (`crates/arda-society/src/lib.rs:15`). 21 library roots carry `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` (e.g. `crates/arda-core/src/lib.rs:6`); `arda-society` does not.

Escape hatches, lexical counts over the 982 tracked `.rs` files under `crates/` (regex on source with `//` comments stripped; "production" = `src/` files whose path has no `test` segment, cut at the first `#[cfg(test)] mod … {`; 718 such files):

| Escape hatch / abstraction | All files | Production | Locations |
|---|---|---|---|
| `as` numeric casts | 3,213 | 2,522 (`arda-gen` 729, `arda-tactical` 511, `arda-settle` 204) | `crates/arda-art-import/src/cleanup.rs:27` |
| `#[allow(..)]` / `cfg_attr(.., allow(..))` | 609 | 438 (`arda-tactical` 74, `arda-gen` 61, `arda-fields` 60) | `crates/arda-art-import/src/cleanup.rs:5` |
| `.unwrap()` / `.expect(` | 4,286 | 16, all inside `#[cfg(test)]` items or files with a crate-level test allow | `crates/arda-core/src/formats/mod.rs:91`, `crates/arda-gen/src/area/terrain_correction_probe.rs:2` |
| `panic!`/`unreachable!`/`todo!`/`unimplemented!` | 76 | 13 (`arda-render` 12 `unreachable!` in fallible-conversion closures, `arda-cli` 1) | `crates/arda-render/src/atlas/palette.rs:62`, `crates/arda-cli/src/main.rs:354` |
| `unwrap_or*` fallbacks | — | 1,408 | `crates/arda-art-import/src/cleanup.rs:64` |
| `unsafe` blocks | 0 | 0 | None found |
| `#[expect(..)]` | 0 | 0 | None found |
| Trait declarations | 13 | 13 | table below |

Lints most often allowed (attribute arguments): `cast_possible_truncation` 161, `cast_precision_loss` 160, `unwrap_used` 129, `expect_used` 113, `cast_sign_loss` 46, `too_many_arguments` 31, `missing_docs` 29, `dead_code` 20, `cast_possible_wrap` 15, `too_many_lines` 10.

Traits are few and serve as seams for sources and storage:

| Trait | Site | Role |
|---|---|---|
| `Wire`, `FixedRecord` | `crates/arda-core/src/formats/hydrology.rs:115`, `crates/arda-core/src/formats/hydrology/table.rs:48` | Fixed-width record codec |
| `FlowStore`, `Store`, `RoutingStore` | `crates/arda-gen/src/hydrology/fine_flow.rs:79`, `crates/arda-gen/src/hydrology/hierarchy/rows.rs:125`, `crates/arda-gen/src/hydrology/routing.rs:215` | Memory vs file-backed hydrology stores |
| `Source` | `crates/arda-refine/src/source.rs:61` | World or synthetic cell data for refinement |
| `Terrain` (×3), `LandUseMap` | `crates/arda-fields/src/input.rs:123`, `crates/arda-midzoom/src/source.rs:30`, `crates/arda-ways/src/input.rs:221`, `crates/arda-fields/src/input.rs:33` | Terrain callbacks into the tactical crates |
| `Overlays` | `crates/arda-blocks/src/overlays.rs:117` | Ways/fields/town overlay provider (society or demo) |
| `BlockSource` | `crates/arda-server/src/tactical/block.rs:115` | Server's tactical block provider |
| `Element` | `crates/arda-server/src/columnar.rs:120` | Columnar layer element encoding |

Trait objects: 151 `dyn ` occurrences outside test files (e.g. `&dyn Source` in `crates/arda-refine/src/block.rs:167`). No `#[non_exhaustive]` anywhere. JSON Schema derives (`schemars`) sit behind an optional `schema` feature in product crates (72 `cfg_attr(feature = "schema", ..)`, e.g. `crates/arda-ids/Cargo.toml:17`); `arda-server` derives `JsonSchema` and `ts_rs::TS` unconditionally on its DTOs (`crates/arda-server/src/tactical/tokens.rs:24`).

Viewer (shallow): `apps/viewer/tsconfig.app.json:10` sets `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`; 2 `any`/ignore/eslint-disable hits across 45 tracked `src` files. CI does not build or lint the viewer (`.github/workflows/ci.yml:1`).

## Determinism and keyed hashing

No ambient randomness or wall clock reaches generated data. `Instant::now` appears only for timing logs and deadlines (`crates/arda-server/src/tactical/routes.rs:67`, `crates/arda-server/src/tactical/prefetch.rs:288`, `crates/arda-refine/src/bin/arda-refine.rs:106`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:52`); no `thread_rng`, `rand::random`, `OsRng` or `SystemTime::now` in `src/`. Every random value is a pure function of the world seed and a key naming the decision, so draw order does not change output.

| Scheme | Construction | Users | Site |
|---|---|---|---|
| `arda-core` stream | BLAKE3 over domain, seed, `SeedKey{tier, stage, x, y, attempt}` → ChaCha8 | `arda-gen` (only `rand_chacha` user) | `crates/arda-core/src/rng.rs:84`, `crates/arda-core/src/rng.rs:100` |
| `arda-ids::hash` | SplitMix64-finaliser chain over `(seed, tag, args…)`, `tag` = FNV-1a of a rule name | shared integer hash | `crates/arda-ids/src/hash.rs:56`, `crates/arda-ids/src/hash.rs:68` |
| `arda-ids::digest`, `subseed` | BLAKE3 of concatenated parts; `subseed` uses derive-key context `"arda-ids subseed v1"` | `arda-npc` streams, `NpcId::from_parts` | `crates/arda-ids/src/hash.rs:82`, `crates/arda-ids/src/hash.rs:98`, `crates/arda-npc/src/rng.rs:57` |
| `arda-wfc::hash` | `mix`, `hash2`, `hash3`, `Stream`; re-exported unchanged by `arda-refine` | tactical WFC, refine | `crates/arda-wfc/src/hash.rs:16`, `crates/arda-refine/src/hash.rs:7` |
| Crate-local SplitMix64 / xoshiro streams | Seeded from a keyed hash of `(seed, tag/domain, keys)` | `arda-settle`, `arda-society` (BLAKE3-keyed), `arda-town`, `arda-names`, `arda-dungeon`, `arda-tactical` placeholders | `crates/arda-settle/src/rng.rs:14`, `crates/arda-society/src/rng.rs:8`, `crates/arda-town/src/rng.rs:45`, `crates/arda-names/src/rng.rs:33`, `crates/arda-dungeon/src/rng.rs:1`, `crates/arda-tactical/src/placeholders/material.rs:17` |
| `arda-midzoom` copy | Own `mix`/`hash2` in its fixed-point module | midzoom detail | `crates/arda-midzoom/src/fixed.rs:94`, `crates/arda-midzoom/src/fixed.rs:103` |

Ordered collections: `BTreeMap`/`BTreeSet` 846 occurrences in 197 files; `HashMap`/`HashSet` 28 in 10 files (e.g. `crates/arda-midzoom/src/source.rs`, `crates/arda-refine/src/region.rs`, `crates/arda-gen/src/formation/arid.rs`). Tactical outputs hash at world square coordinates (`TacticalLayout::origin`, `crates/arda-tactical/src/layout.rs:37`) so adjacent windows join.

## Error handling

Libraries return typed `Result`s built with `thiserror` (74 `derive(.. Error)` sites in 68 files; `#[from]` 102, `#[source]` 65, `#[error(transparent)]` 77). One error enum per product crate, 47 `pub enum *Error` inside `arda-gen`, mostly per hydrology and orchestrator stage, rolled up into `GenError` at `crates/arda-gen/src/orchestrator/error.rs:7`. `arda-ids` has no error type. Embedded-data loaders map parse failures to a crate error naming the file (`crates/arda-society/src/tables.rs:299`, `crates/arda-npc/src/data/mod.rs:27`).

| Boundary | Mechanism | Site |
|---|---|---|
| HTTP | `ServerError` → `(StatusCode, code)` via `classify`; JSON `ApiError` body with stable `code`; absolute paths redacted from messages | `crates/arda-server/src/error.rs:18`, `crates/arda-server/src/error.rs:112`, `crates/arda-server/src/error.rs:181` |
| `arda` CLI | `anyhow::Result` with `.context(..)` | `crates/arda-cli/src/main.rs:360`, `crates/arda-cli/src/main.rs:236` |
| `arda-town`, `arda-settle`, `arda-server` binaries | `fn main() -> ExitCode` | `crates/arda-town/src/main.rs:260`, `crates/arda-settle/src/main.rs:42`, `crates/arda-server/src/main.rs:94` |
| `arda-refine`, `arda-midzoom` dev binaries | `Box<dyn std::error::Error>` | `crates/arda-refine/src/bin/arda-refine.rs:85`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:41` |

Logging: no `log` or `tracing` crate. The server writes `eprintln!` lines for 5xx errors and per-request tactical timings (`crates/arda-server/src/error.rs:217`, `crates/arda-server/src/tactical/routes.rs:138`); CLIs print progress with `println!`. Panics remain possible on invariant-backed indexing; no panic-recovery boundary.

## Dependency injection

Plain arguments carry seeds, configs, input records and output paths; there is no container. Variation points are trait objects or generic parameters passed in by the caller (`&dyn Source` for refine, `Overlays` for `arda-blocks::Pipeline` at `crates/arda-blocks/src/pipeline.rs:70`, terrain callbacks for fields/ways/midzoom, store generics in `arda-gen` hydrology). `arda-blocks` ships two `Overlays` implementations: `SocietyOverlays` from world data and `DemoOverlays` from synthetic samples (`crates/arda-blocks/src/society/mod.rs:91`, `crates/arda-blocks/src/lib.rs:15`).

The server builds one `AppState` at startup (`crates/arda-server/src/lib.rs:143`, `AppState::open` at `crates/arda-server/src/lib.rs:171`) and hands it to every handler as `State<Arc<AppState>>`, aliased `Shared` per route module (`crates/arda-server/src/routes.rs:22`). Process-wide statics hold only immutable embedded data parsed once (`OnceLock`, above).

## Rule citations in code

Code cites the normative scenario docs as `logic/NN §rule` in `//`, `///` and `//!` comments: 538 `logic/NN` mentions in 261 files, 297 with an explicit `§rule` (e.g. `crates/arda-ids/src/hash.rs:1`, `crates/arda-render/src/atlas/fine/relief.rs:1`, `crates/arda-blocks/src/overlays.rs:23`). Counts by chapter: 02 170, 01 66, 04 64, 17 47, 16 42, 09 34, 10 21, 13 19, 12 16, 03 15, 08 13, 06 10, 05 7, 14 6, 11 5, 15 3. Other citation forms: goal numbers (`goals 42, 46`, 315 mentions, e.g. `crates/arda-blocks/src/lib.rs:1`), integration adapters `A<n>` and invariants `I<n>` (`crates/arda-ids/src/lib.rs:1`), `vocabulary.md` (`crates/arda-ids/src/ids.rs:1`) and `code-prefs.md` §Q (16 mentions, `crates/arda-core/src/coords.rs:4`). `vocabulary.md`, `code-prefs.md` and `docs/goal-prompts/` are not tracked in the repository; only `docs/capstone/` is.

## Data files

Embedded at compile time with `include_str!`/`include_bytes!` and parsed on first use; nothing is read from disk at runtime.

| Crate | Files | Loader |
|---|---|---|
| `arda-npc` | `data/srd/{races,classes,spellcasting,spells,stat_blocks,equipment,languages}.json` (SRD 5.1, provenance in `data/srd/SOURCE.md`), `data/content/{personality,occupations,cultures,tiers,backgrounds}.json` | `crates/arda-npc/src/data/mod.rs:45` |
| `arda-society` | `data/{goods,politics,factions,roles,history,hooks,names}.json`, cross-checked by `Tables::check` | `crates/arda-society/src/tables.rs:309` |
| `arda-server` | `mappings/{identity,5e-srd-monster}.json` (NPC sheet mappings) | `crates/arda-server/src/sheet_map/tests.rs:4` |
| `arda-settle` | `assets/fonts/NotoSerif*.ttf` | `crates/arda-settle/src/render/font.rs:14` |
| `arda-names` | `src/blocklist.txt` | `crates/arda-names/src/filter.rs:11` |
| `arda-gen` | `src/area/mfd-p11-q32.bin` (precomputed flow table) | `crates/arda-gen/src/area/mfd.rs:17` |
| `arda-render` | `src/coverage_fixtures.json`, `src/overview/fixtures/*.json` (test fixtures) | `crates/arda-render/src/channel_geometry.rs:431` |

## Test layout

| Kind | Convention | Site |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests;` in a sibling `tests.rs` (48 files) or `<name>_tests.rs`/`tests_<name>.rs` (40+), or inline `mod tests { .. }` (225) | `crates/arda-tactical/src/compose/tests_variants.rs`, `crates/arda-gen/src/area/water_neighbor_tests.rs:1` |
| Integration tests | `crates/<crate>/tests/*.rs`; shared helpers in `tests/common/mod.rs` or `tests/support/` | `crates/arda-town/tests/common/mod.rs`, `crates/arda-fields/tests/support/shape.rs` |
| Workspace tests | Root package `arda-workspace-tests` (lib `tests/empty_lib.rs`) hosts `golden_world`, `golden_recipes`, `arid_recipe7`, `e2e_village`, `zoom_continuity`, `fixture_dir` | `Cargo.toml:76`, `Cargo.toml:104` |
| Golden fingerprints | `tests/golden/*.txt`; rewrite with `ARDA_BLESS=1` | `tests/golden_world.rs:82`, `tests/golden_recipes.rs:102` |
| Committed schemas | `bindings/schema/*.json` (148 files under `bindings/`) checked fresh; rewrite with `ARDA_BLESS_BINDINGS=1` | `crates/arda-server/src/schema/tests.rs:7` |
| World-backed tests | `$ARDA_TEST_WORLD`, else `<workspace>/out/micro42` | `crates/arda-blocks/tests/settlements.rs:8` |
| Shared fixtures | Built under `target/` in a private sibling then renamed; stamped with a fingerprint of library inputs and rebuilt when stale | `tests/support/fixture_dir.rs:1` |
| Release gates | 31 `#[ignore]`, mostly `"wall-clock timing; release gate"` and world-dependent gates run with `--release -- --ignored` | `tests/zoom_continuity.rs` |
| Mocks | None; temp directories are the substitute (`TempDir`, `#[cfg(test)]`) | `crates/arda-core/src/formats/mod.rs:81` |
| Test profile | `[profile.test] opt-level = 2` | `Cargo.toml:68` |

No `proptest`, `quickcheck` or `insta` dependency. CI runs `cargo test --workspace --all-features --no-fail-fast` on Linux, macOS and Windows (`.github/workflows/ci.yml:9`, `.github/workflows/ci.yml:15`).

## Supply chain

`deny.toml` allows MIT, MIT-0, Apache-2.0 (and LLVM exception), CC0-1.0, Unicode-3.0, BSD-3-Clause, Zlib (`deny.toml:4`); `multiple-versions = "warn"` (`deny.toml:8`); yanked crates denied with one ignored advisory, `RUSTSEC-2026-0192` (ttf-parser unmaintained, `deny.toml:19`); unknown registries and git sources denied (`deny.toml:23`). CI runs `cargo-deny-action@v2` (`.github/workflows/ci.yml:39`). Shared dependency versions live in `[workspace.dependencies]` (`Cargo.toml:11`); `arda-server` and the root dev-dependencies pin axum/tokio/hyper directly (`crates/arda-server/Cargo.toml:31`).

## Changelog fragments

Each change adds `docs/capstone/changelog.d/YYYY-MM-DD-<slug>.md`; writing runs on `main` fold them into `docs/capstone/changelog.md` newest-first as `## YYYY-MM-DD - <title>` (some slugs carry a stage prefix: `groom-`, `plan-`, `implement-`, `map-`). A fragment opens `# YYYY-MM-DD — <title>` and lists bullets prefixed by the touched crate or tool, then `Why`, `After (MICRO 42)` and `Tests` bullets (folded entry: `docs/capstone/changelog.md:31`). `docs/capstone/changelog/` holds older dated entries. User-facing release notes are in the root `CHANGELOG.md`, one `## <version> — <date>` section per release, pointing at the ledger (`CHANGELOG.md:3`, `CHANGELOG.md:6`).
