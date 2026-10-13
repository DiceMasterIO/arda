---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: eab2eaa34e3d
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top)Dockerfile"
  - ":(top)crates/*/Cargo.toml"
  - ":(top)crates/*/src/lib.rs"
  - ":(top)crates/arda-cli/src/**"
  - ":(top)crates/arda-server/src/**"
  - ":(top)crates/arda-settle/src/main.rs"
  - ":(top)crates/arda-town/src/main.rs"
  - ":(top)crates/arda-refine/src/bin/**"
  - ":(top)crates/arda-midzoom/src/bin/**"
  - ":(top)crates/arda-blocks/src/**"
  - ":(top)crates/arda-people/src/build.rs"
  - ":(top)apps/viewer/**"
  - ":(top)tools/**"
  - ":(top)crates/arda-settle/src/pipeline.rs"
  - ":(top)crates/arda-refine/src/block.rs"
  - ":(top)crates/arda/src/world.rs"
  - ":(top)deny.toml"
  - ":(top).github/workflows/ci.yml"
  - ":(top)contract-kit/**"
  - ":(top).github/workflows/release.yml"
  - ":(top)tools/*.sh"
  - ":(top)tools/art-gen/safety.mjs"
  - ":(top)apps/viewer/licence-policy.json"
  - ":(top)apps/viewer/scripts/**"
---

# Architecture

The workspace is 23 Rust crates under `crates/` (`Cargo.toml:2` globs `crates/*`), one TypeScript app `apps/viewer/`, Node tooling `tools/art-gen/`, the Node contract-kit replay checker `contract-kit/check.mjs` (`contract-kit/check.mjs:1`), shell release scripts in `tools/` (`tools/release.sh:1`) and two offline Python table generators in `tools/`. The root package `arda-workspace-tests` has no library or binary of its own and hosts the workspace integration tests in `tests/` (`Cargo.toml:76`, `Cargo.toml:104`).

## Layers

Tiers are derived from each crate's `[dependencies]` (dev-dependencies excluded) and cross-checked against the `arda_*::` paths actually used in each `src/`; the two agree for every crate. A crate depends only on crates in lower tiers.

| Tier | Crate | Directory | Workspace dependencies (normal) | Responsibility |
|---|---|---|---|---|
| 0 | arda-core | `crates/arda-core/` | none (`crates/arda-core/Cargo.toml:10`) | Shared types, keyed PRNG, the only world byte codec (`crates/arda-core/src/lib.rs:1`) |
| 0 | arda-ids | `crates/arda-ids/` | none (`crates/arda-ids/Cargo.toml:10`) | Shared ids, canonical enums and the stable hash for product layers (`crates/arda-ids/src/lib.rs:1`) |
| 0 | arda-names | `crates/arda-names/` | none (`crates/arda-names/Cargo.toml:10`) | Phonotactic naming languages (`crates/arda-names/src/lib.rs:1`) |
| 0 | arda-society | `crates/arda-society/` | none (`crates/arda-society/Cargo.toml:10`) | Realm politics, economy, factions, history, campaign hooks (`crates/arda-society/src/lib.rs:3`) |
| 0 | arda-tactical | `crates/arda-tactical/` | none (`crates/arda-tactical/Cargo.toml:10`) | Asset catalogue, validator, placeholder library, `TacticalLayout`, compositor (`crates/arda-tactical/src/lib.rs:1`) |
| 0 | arda-wfc | `crates/arda-wfc/` | none (`crates/arda-wfc/Cargo.toml:10`) | Generic deterministic wave-function collapse (`crates/arda-wfc/src/lib.rs:1`) |
| 1 | arda-gen | `crates/arda-gen/` | arda-core (`crates/arda-gen/Cargo.toml:11`) | Continent, area, formation, hydrology stages; orchestrator is the only disk caller (`crates/arda-gen/src/lib.rs:1`, `crates/arda-gen/src/lib.rs:4`) |
| 1 | arda-render | `crates/arda-render/` | arda-core (`crates/arda-render/Cargo.toml:11`) | PNG/JSON export of stored worlds; never depends on `arda-gen` (`crates/arda-render/src/lib.rs:1`) |
| 1 | arda-npc | `crates/arda-npc/` | arda-ids, arda-names (`crates/arda-npc/Cargo.toml:11`) | NPC populations with jobs, personalities, SRD 5.1 sheets (`crates/arda-npc/src/lib.rs:1`) |
| 1 | arda-scene | `crates/arda-scene/` | arda-tactical (`crates/arda-scene/Cargo.toml:11`) | SRD movement/sight/cover/light scene from a `TacticalLayout` (`crates/arda-scene/src/lib.rs:1`) |
| 2 | arda | `crates/arda/` | arda-core, arda-gen, arda-render (`crates/arda/Cargo.toml:11`) | Public world facade: generation, `World` load/query, exports (`crates/arda/src/lib.rs:1`) |
| 2 | arda-art-import | `crates/arda-art-import/` | arda-tactical (`crates/arda-art-import/Cargo.toml:11`) | Raw AI images to a valid tactical asset library (`crates/arda-art-import/src/lib.rs:1`) |
| 2 | arda-dungeon | `crates/arda-dungeon/` | arda-scene, arda-tactical (`crates/arda-dungeon/Cargo.toml:11`) | Dungeon and cave battle maps (`crates/arda-dungeon/src/lib.rs:1`) |
| 2 | arda-fields | `crates/arda-fields/` | arda-scene, arda-tactical (`crates/arda-fields/Cargo.toml:12`) | 5-ft land use around settlements (`crates/arda-fields/src/lib.rs:1`) |
| 2 | arda-ways | `crates/arda-ways/` | arda-scene, arda-tactical (`crates/arda-ways/Cargo.toml:11`) | 5-ft roads, verges, bridges, fords, ferries (`crates/arda-ways/src/lib.rs:1`) |
| 2 | arda-town | `crates/arda-town/` | arda-scene, arda-tactical, arda-wfc (`crates/arda-town/Cargo.toml:18`) | Town street/plot plans and 5-ft building blocks (`crates/arda-town/src/lib.rs:1`) |
| 3 | arda-refine | `crates/arda-refine/` | arda, arda-core, arda-wfc, arda-scene, arda-tactical (`crates/arda-refine/Cargo.toml:15`) | One 100 m world cell as a 64 × 64 block of 5-ft squares (`crates/arda-refine/src/lib.rs:1`) |
| 3 | arda-settle | `crates/arda-settle/` | arda, arda-core, arda-render, arda-ids, arda-names (`crates/arda-settle/Cargo.toml:12`) | Settlements, roads, realms, names, gazetteer; writes `<world>/society/` (`crates/arda-settle/src/lib.rs:1`) |
| 4 | arda-people | `crates/arda-people/` | arda, arda-core, arda-ids, arda-names, arda-npc, arda-refine, arda-settle, arda-society, arda-town (`crates/arda-people/Cargo.toml:11`) | Town plans on world terrain, society, stored notables (`crates/arda-people/src/lib.rs:1`) |
| 5 | arda-blocks | `crates/arda-blocks/` | arda, arda-refine, arda-scene, arda-tactical, arda-ways, arda-fields, arda-town, arda-ids, arda-people, arda-settle (`crates/arda-blocks/Cargo.toml:11`) | World-derived tactical blocks: refine + ways/fields/town overlays (`crates/arda-blocks/src/lib.rs:1`) |
| 6 | arda-midzoom | `crates/arda-midzoom/` | arda, arda-blocks, arda-core, arda-fields, arda-ids, arda-people, arda-refine, arda-render, arda-settle, arda-town, arda-ways (`crates/arda-midzoom/Cargo.toml:15`) | On-demand ~10 m relief refinement and tiles (`crates/arda-midzoom/src/lib.rs:1`) |
| 7 | arda-cli | `crates/arda-cli/` | arda, arda-midzoom, arda-people, arda-settle, arda-tactical, arda-dungeon, arda-scene, arda-art-import (`crates/arda-cli/Cargo.toml:15`) | The `arda` binary (`crates/arda-cli/Cargo.toml:11`) |
| 7 | arda-server | `crates/arda-server/` | arda, arda-core, arda-npc, arda-blocks, arda-dungeon, arda-ids, arda-people, arda-midzoom, arda-refine, arda-scene, arda-settle, arda-society, arda-tactical, arda-town (`crates/arda-server/Cargo.toml:18`) | World-query contract and HTTP service (`crates/arda-server/src/lib.rs:1`) |

Dev-only workspace edges (tests and examples, not production): arda-blocks → arda-society (`crates/arda-blocks/Cargo.toml:26`), arda-midzoom → arda-tactical (`crates/arda-midzoom/Cargo.toml:32`), arda-settle → arda-npc, arda-society (`crates/arda-settle/Cargo.toml:30`), arda-server → arda-names (`crates/arda-server/Cargo.toml:51`; its only `src/` mentions are doc comments and tests, `crates/arda-server/src/npc_dto/input.rs:106`). The root test package dev-depends on arda, arda-core, arda-midzoom, arda-npc, arda-people, arda-server, arda-settle, arda-town (`Cargo.toml:87`).

Feature `schema` (optional `schemars` derives) exists on arda-ids, arda-names, arda-settle, arda-society, arda-town; arda-server enables it on arda-settle, arda-society and arda-town (`crates/arda-server/Cargo.toml:28`, `crates/arda-settle/Cargo.toml:27`).

Two dependency stacks meet at tier 3:

- World stack: `arda-core` → `arda-gen`/`arda-render` → `arda` → (`arda-refine`, `arda-settle`) → `arda-people` → `arda-blocks` → `arda-midzoom`.
- Tactical stack: `arda-tactical` → `arda-scene` → (`arda-ways`, `arda-fields`, `arda-dungeon`, `arda-town` with `arda-wfc`) → joined into the world stack by `arda-refine` (tier 3) and `arda-blocks` (tier 5).
- Naming/people stack: `arda-ids`, `arda-names` → `arda-npc`; `arda-society` stands alone and is consumed by `arda-people`.

Out-of-workspace parts:

| Part | Directory | Depends on | Direction |
|---|---|---|---|
| apps/viewer | `apps/viewer/` | `arda-server` over HTTP; ts-rs bindings at `bindings/ts/arda/` through the `@arda` alias (`apps/viewer/vite.config.ts:8`, `apps/viewer/vite.config.ts:14`) | No Rust crate depends on it |
| tools/art-gen | `tools/art-gen/` | A local Slopify HTTP server (`tools/art-gen/generate.mjs:22`); writes a folder plus `import.toml` that `arda tactical import` reads (`tools/art-gen/generate.mjs:564`) | Consumed by arda-art-import via the CLI only as files |
| tools/*.py | `tools/generate_spectral_tables.py`, `tools/generate_water_forcing_tables.py` | none | Offline generators; the spectral one writes `crates/arda-gen/src/spectral_tables.rs` (`tools/generate_spectral_tables.py:6`); the water one writes the path given as `argv[1]` (`tools/generate_water_forcing_tables.py:70`) |

## Module boundaries

Boundaries are enforced only by Cargo manifests (a crate can name only its declared dependencies). No dependency-rule checker exists: CI runs tests, MSRV check, fmt, clippy and cargo-deny (`.github/workflows/ci.yml:17`, `.github/workflows/ci.yml:27`, `.github/workflows/ci.yml:37`, `.github/workflows/ci.yml:39`), and `deny.toml` bans no workspace edges (`deny.toml:9`). Workspace lints deny `unsafe_code`, `missing_docs`, `unwrap_used`, `expect_used` in every crate (`Cargo.toml:50`, `Cargo.toml:56`).

| Module | Public surface | May not import / not imported by |
|---|---|---|
| arda-core | Modules `cell`, `config`, `continent`, `coords`, `error`, `fixed`, `formats`, `hydrology`, `objects`, `playa`, `rng`, `terrain`, `water`; re-exports codecs `encode_cells`/`decode_cells`, `TerrainFileReader`, rng `derive`/`rng`/`SeedKey` (`crates/arda-core/src/lib.rs:8`, `crates/arda-core/src/lib.rs:34`, `crates/arda-core/src/lib.rs:53`) | Imports no workspace crate. Imported by arda-gen, arda-render, arda, arda-refine, arda-settle, arda-people, arda-midzoom, arda-server |
| arda-ids | `biome`, `cover`, `function`, `hash`, `ids`, `land_use`, `road`; `BuildingId`, `NpcId`, `RealmId`, `SettlementId`, `digest`/`hash`/`subseed` (`crates/arda-ids/src/lib.rs:19`, `crates/arda-ids/src/lib.rs:30`) | Imports no workspace crate |
| arda-names | `Language`, `Preset`, `Tongue`, `place_name`, `person_name`, `given_name`, `NameScope` (`crates/arda-names/src/lib.rs:47`) | Imports no workspace crate |
| arda-society | `simulate_society(seed, &WorldSettlements) -> Society` (`crates/arda-society/src/lib.rs:136`); `Society`, `SettlementSociety`, `RealmSociety`, `FORMAT_VERSION` (`crates/arda-society/src/lib.rs:57`) | Imports no workspace crate; it reads settlement data through its own `input::WorldSettlements` rather than arda-settle types (`crates/arda-society/src/lib.rs:54`) |
| arda-tactical | `Catalog`, `Library`, `TacticalLayout`, `render`/`render_with`/`render_region_tinted`, `WorldTint`, `Issue`/`Rule`/`Thresholds` (`crates/arda-tactical/src/lib.rs:26`) | Imports no workspace crate; it knows nothing of worlds |
| arda-wfc | `solve`, `Problem`, `Solution`, `Rules`, `TileSet`, `violations` (`crates/arda-wfc/src/lib.rs:22`) | Imports no workspace crate |
| arda-gen | `area`, `continent`, `formation`, `hydrology`, `noise`, `orchestrator`, `spectral`, `spectral_composition`; root `generate_world`, `generate_world_with_limits`, `GenError`, `HydrologyLimits` (`crates/arda-gen/src/lib.rs:11`, `crates/arda-gen/src/lib.rs:20`) | Imports only arda-core. Imported only by `arda` |
| arda-render | `carto`, `json`; Atlas/overview APIs, `ImageQuality`, `AreaImageScale`, the `BandOverlay` hook for painted bands (`crates/arda-render/src/lib.rs:29`), `RenderError` (`crates/arda-render/src/lib.rs:16`, `crates/arda-render/src/lib.rs:43`) | Never imports arda-gen (`crates/arda-render/src/lib.rs:1`). Imported by `arda`, arda-midzoom and arda-settle |
| arda-npc | `Generator`, `generate_population`, `npc`, `commoner`, `Population`, `Sheet`, `Resident`, `generate_one`, `OneRequest` (`crates/arda-npc/src/lib.rs:43`, `crates/arda-npc/src/lib.rs:44`, `crates/arda-npc/src/lib.rs:52`, `crates/arda-npc/src/lib.rs:217`) | Imports only arda-ids, arda-names |
| arda-scene | `build_scene`, `SceneIndex`, `RulesSidecar`, `scene_debug_png` (`crates/arda-scene/src/lib.rs:34`, `crates/arda-scene/src/lib.rs:51`) | Imports only arda-tactical |
| arda | `generate`, `generate_with_limits`, `generate_from_fine_source`, `generate_from_fine_recipe`, `export_area`, `export_area_with_scale`, `export_overview`, the `*_with_overlay` / `*_with_quality` / `*_with_look` exports (`crates/arda/src/lib.rs:22`; `crates/arda/src/export_quality.rs:56`, `crates/arda/src/export_quality.rs:178`), `World`, `Area`, `MapStyle`, `ExportFormat` (`crates/arda/src/lib.rs:29`, `crates/arda/src/lib.rs:45`, `crates/arda/src/lib.rs:86`, `crates/arda/src/lib.rs:135`, `crates/arda/src/lib.rs:201`) | Consumers are documented to depend on this crate only (`crates/arda/src/lib.rs:1`); arda-refine, arda-settle, arda-people, arda-midzoom and arda-server also import arda-core directly |
| arda-art-import | `import(&ImportOptions) -> Report` plus 20 public pipeline modules (`crates/arda-art-import/src/lib.rs:25`, `crates/arda-art-import/src/lib.rs:97`) | Imports only arda-tactical. Imported only by arda-cli |
| arda-dungeon | `generate(&Params) -> Dungeon`, `site_seed`, `Kind` (`crates/arda-dungeon/src/lib.rs:229`, `crates/arda-dungeon/src/lib.rs:241`) | Imported by arda-cli and arda-server |
| arda-fields | `fields_window`, `generate`, `FieldsWindow` (`crates/arda-fields/src/lib.rs:43`) | Imported only by arda-blocks and arda-midzoom |
| arda-ways | `apply_ways`, `WaysOutput`, `Sidecar` (`crates/arda-ways/src/lib.rs:29`, `crates/arda-ways/src/lib.rs:101`) | Imported only by arda-blocks and arda-midzoom |
| arda-town | `generate`, `TownPlan`, `TownSite`, `TerrainInput`, `block` module (`crates/arda-town/src/lib.rs:17`, `crates/arda-town/src/lib.rs:31`) | Imports no world crate: plans take a `TownSite` and `TerrainInput` (`crates/arda-town/src/lib.rs:3`) |
| arda-refine | `refine`, `refine_block`, `refine_window`, `Block`, `Source`, `CellKey`, `WorldSource`, `MAX_WINDOW`, module `coastwet` (`crates/arda-refine/src/lib.rs:55`, `crates/arda-refine/src/lib.rs:62`, `crates/arda-refine/src/lib.rs:95`) | Reads worlds through the `Source` trait; `WorldSource` adapts `arda::World` |
| arda-settle | `generate(world_dir, budget)`, `write`, `encode`, `run`, `Society`, `SOCIETY_DIR` (`crates/arda-settle/src/lib.rs:49`, `crates/arda-settle/src/lib.rs:56`, `crates/arda-settle/src/lib.rs:171`) | Reads the stored world via `arda::World::load` (`crates/arda-settle/src/lib.rs:172`) |
| arda-people | `build`, `compute`, `World`, `SocietyFiles`, `NotablesFile` (`crates/arda-people/src/lib.rs:32`) | Reads `<world>/society/` written by arda-settle (`crates/arda-people/src/build.rs:60`) |
| arda-blocks | `Pipeline`, `Composed`, `Overlays`, `SQUARES_PER_CELL`, modules `demo`, `overlays`, `pipeline`, `society` (`crates/arda-blocks/src/lib.rs:21`, `crates/arda-blocks/src/lib.rs:26`) | Imported by arda-midzoom and arda-server |
| arda-midzoom | `ReliefWorld`, `Landscape`, `Pyramid`, `render_tile`, `render_window`, `refine_tile`, module `export` (`SocietyMode`, `export_area`, `export_overview`: society-aware PNG exports, `crates/arda-midzoom/src/export.rs:15`, `crates/arda-midzoom/src/export.rs:62`, `crates/arda-midzoom/src/export.rs:94`) (`crates/arda-midzoom/src/lib.rs:45`, `crates/arda-midzoom/src/lib.rs:50`, `crates/arda-midzoom/src/lib.rs:60`) | Imported by arda-server and arda-cli |
| arda-cli | No library; private subcommand modules `dungeon`, `generate`, `society`, `tactical`, `tactical_import` (`crates/arda-cli/src/main.rs:12`) | Nothing imports it; it calls `arda_midzoom::export` for society-aware overview and area PNGs (`crates/arda-cli/src/main.rs:8`) |
| arda-server | Library `router`, `AppState`, `ServerConfig`, `WorldQuery`, `CellSample`, `PointSample`, `CONTRACT_VERSION`, `BlockSource` plus 26 public modules, among them `gazetteer`, `realms`, `probe`, `versions` and `schema` (`crates/arda-server/src/lib.rs:12`, `crates/arda-server/src/lib.rs:39`) | Imported only by the root test package (`Cargo.toml:93`) |

Examples (shallow: not inventoried beyond names): `crates/arda/examples/`, `crates/arda-core/examples/`, `crates/arda-gen/examples/` (11 files), `crates/arda-art-import/examples/`, `crates/arda-fields/examples/`, `crates/arda-names/examples/`, `crates/arda-npc/examples/`, `crates/arda-scene/examples/`, `crates/arda-server/examples/`, `crates/arda-society/examples/`, `crates/arda-ways/examples/crossings/`.

## Entry points

| Process | Entry `file:line` | Commands / behaviour |
|---|---|---|
| `arda` (crate arda-cli) | `crates/arda-cli/src/main.rs:399` | clap `Command` enum (`crates/arda-cli/src/main.rs:34`), dispatched at `crates/arda-cli/src/main.rs:401`–`crates/arda-cli/src/main.rs:449` |
| `arda generate` | `crates/arda-cli/src/main.rs:36` → `run_generate` `crates/arda-cli/src/generate.rs:45` | `--terrain legacy` (default, `crates/arda-cli/src/main.rs:51`) calls `arda::generate`; `--terrain fine` calls `generate_from_fine_recipe` (`crates/arda-cli/src/generate.rs:85`) |
| `arda preview` | `crates/arda-cli/src/main.rs:73` → `run_preview` `crates/arda-cli/src/main.rs:252` | Generates then writes an overview via `export_overview` (`--px`) or the society-aware `arda_midzoom::export::export_overview` (`crates/arda-cli/src/main.rs:284`, `crates/arda-cli/src/main.rs:285`); `--society auto|off` (`crates/arda-cli/src/main.rs:94`) |
| `arda export` | `crates/arda-cli/src/main.rs:101` → `run_export` `crates/arda-cli/src/main.rs:304` | Overview or area as PNG/JSON; PNGs take `--society auto|off` (`crates/arda-cli/src/main.rs:129`) and go through `arda_midzoom::export` when the world has `society/` (`crates/arda-cli/src/main.rs:345`). `--block` is kept only to refuse with a pointer to `arda-refine render` and the tactical cell route (`crates/arda-cli/src/main.rs:314`) |
| `arda settle` | `crates/arda-cli/src/main.rs:137` → `society::settle` `crates/arda-cli/src/society.rs:28` | `arda_settle::generate` writes `<world>/society/` |
| `arda society build` | `crates/arda-cli/src/main.rs:144` → `society::run` `crates/arda-cli/src/society.rs:41` | `arda_people::build` writes `society.json` and `notables.json` |
| `arda tactical validate\|import\|placeholders\|render` | `crates/arda-cli/src/main.rs:149` → `tactical::run` `crates/arda-cli/src/tactical.rs:65` | Library validation, `arda_art_import::import` (`crates/arda-cli/src/tactical_import.rs:41`), placeholder library, layout render |
| `arda dungeon` | `crates/arda-cli/src/main.rs:155` → `dungeon::run` `crates/arda-cli/src/dungeon.rs:82` | Layout JSON, optional rules/meta sidecars, scene JSON and PNG |
| `arda-server` | `crates/arda-server/src/main.rs:143` → `run` `crates/arda-server/src/main.rs:69` | Tokio multi-thread runtime; `AppState::open` on the blocking pool, binds `--host` (default `127.0.0.1`) `--port` (default 8787), serves until Ctrl-C or, on Unix, SIGTERM (`crates/arda-server/src/main.rs:86`, `crates/arda-server/src/main.rs:101`, `crates/arda-server/src/main.rs:108`). `--probe` skips serving and runs the std-only loopback `GET /v1/health` check for container healthchecks (`crates/arda-server/src/main.rs:131`, `crates/arda-server/src/probe.rs:1`); `--bake-workers` and `--bake-result-mib` size the bake lane |
| `arda-settle` | `crates/arda-settle/src/main.rs:42` | `generate --world` (`arda_settle::generate`), `render --world --out` (`arda_settle::render::render`) (`crates/arda-settle/src/main.rs:45`, `crates/arda-settle/src/main.rs:64`) |
| `arda-town` | `crates/arda-town/src/main.rs:260` → `run` `crates/arda-town/src/main.rs:161` | `render`, `block`, `info`, `export` over the synthetic sample sites, not stored worlds (`crates/arda-town/src/main.rs:1`, `crates/arda-town/src/main.rs:21`) |
| `arda-refine` | `crates/arda-refine/src/bin/arda-refine.rs:84` | `render`, `stats`, `coastwet`, `survey` over a stored world (`crates/arda-refine/src/bin/arda-refine.rs:19`, `crates/arda-refine/src/bin/arda-refine.rs:97`) |
| `arda-midzoom` | `crates/arda-midzoom/src/bin/arda-midzoom.rs:41` | One relief crop or a timed `k × k` tile block from a stored world (`crates/arda-midzoom/src/bin/arda-midzoom.rs:74`) |
| `tools/art-gen/generate.mjs` | `main` `tools/art-gen/generate.mjs:621`, invoked at `tools/art-gen/generate.mjs:944` | Selects prompt-pack assets, submits Slopify batches (only with `--yes`), downloads images, writes `import.toml` (`tools/art-gen/generate.mjs:8`) |
| `tools/art-gen/build-prompts.mjs` | `tools/art-gen/build-prompts.mjs:1` | Rebuilds `prompts.json` from `checklist.csv` and `expansion.mjs`; `--check` mode (`tools/art-gen/build-prompts.mjs:4`) |
| Viewer | `apps/viewer/index.html:10` → `apps/viewer/src/main.tsx:9` | Vite dev server port 5173 (`apps/viewer/vite.config.ts:17`); scripts `dev`, `build`, `preview`, `test` in `apps/viewer/package.json` |
| Docker image | `Dockerfile:15`, `Dockerfile:24` | Builds `arda-cli` and `arda-server` (`Dockerfile:21`), non-root user, optional AI library stacked under `/opt/arda/library/ai` via `--build-context artlib=` (`Dockerfile:11`); entrypoint `arda-entrypoint` (`Dockerfile:43`) runs `arda-server --world ${ARDA_WORLD:-/worlds/world} --host 0.0.0.0 --port 8787` (`tools/docker-entrypoint.sh:22`); `HEALTHCHECK` runs `arda-server --probe` (`Dockerfile:41`) |
| Release workflow | `.github/workflows/release.yml:5`, triggered by `v*` tags (`.github/workflows/release.yml:9`) | Jobs `verify` (`.github/workflows/release.yml:24`), `binaries` (`.github/workflows/release.yml:59`), `image` (`.github/workflows/release.yml:95`), `release` (`.github/workflows/release.yml:173`); helper scripts `tools/release.sh`, `tools/package-release.sh`, `tools/smoke-binaries.sh`, `tools/docker-smoke.sh` |
| `tools/release-gates.sh` | `tools/release-gates.sh:1` | Builds once and runs every wall-clock release gate (`GATE <name> <measured_ms> <budget_ms>` lines), writes a Markdown receipt (`tools/release-gates.sh:7`, `tools/release-gates.sh:19`, `tools/release-gates.sh:88`) |
| `contract-kit/check.mjs` | `contract-kit/check.mjs:1` | Replays every manifest case against a running arda-server and compares status, content type, listed headers and body (`contract-kit/check.mjs:1`) |
| `tools/art-gen/safety.mjs` | `tools/art-gen/safety.mjs:1` | Spend and image caps and POST reconciliation used by `generate.mjs` (`tools/art-gen/safety.mjs:1`) |

The public library entry points of `arda` (generation, `World::load` `crates/arda/src/world.rs:33`, exports) are the in-process API every Rust consumer above uses.

## Communication

### HTTP (arda-server)

Transport is axum 0.8 over hyper HTTP/1 only (`crates/arda-server/Cargo.toml:32`, `crates/arda-server/Cargo.toml:44`). No websocket, SSE, message queue or event bus exists. Every route is registered in `router` (`crates/arda-server/src/routes.rs:28`) under the `/v1` prefix (`crates/arda-server/src/routes.rs:72`); `/v1/tactical` is a nested router (`crates/arda-server/src/tactical/routes.rs:37`). Unmatched paths go to `not_found` (`crates/arda-server/src/routes.rs:73`, `crates/arda-server/src/routes.rs:195`). CORS admits only `localhost`, `127.0.0.1` and `[::1]` origins, methods GET/HEAD/POST/DELETE/OPTIONS, and exposes `ETag`, `X-Arda-Cache`, `Location` and the sheet-mapping header (`crates/arda-server/src/routes.rs:103`). World work runs on Tokio's blocking pool through `blocking` (`crates/arda-server/src/routes.rs:124`). Connection limits and header read timeout are applied in `serve` (`crates/arda-server/src/serve.rs:41`).

Payload field tables live in `09-interfaces.md`; each row cites its section there.

| Method | Path | Handler | Payload (request → response) |
|---|---|---|---|
| GET | `/v1/health` | `crates/arda-server/src/routes.rs:199` | none → `Health`; `09-interfaces.md § GET /v1/health` |
| GET | `/v1/world` | `crates/arda-server/src/routes.rs:208` | none → `WorldInfo` (carries `versions`, `crates/arda-server/src/versions.rs:1`); `09-interfaces.md § GET /v1/world` |
| GET | `/v1/cell/{gx}/{gy}` | `crates/arda-server/src/routes.rs:216` | path `gx`,`gy: u32` → `CellSample`; `09-interfaces.md § GET /v1/cell/{gx}/{gy}` |
| GET | `/v1/area/{ax}/{ay}/cells` | `crates/arda-server/src/routes.rs:228` | path + `format` query → JSON `AreaCells` or `ARDACOLS` binary; `09-interfaces.md § GET /v1/area/{ax}/{ay}/cells` |
| GET | `/v1/area/{ax}/{ay}/rivers` | `crates/arda-server/src/routes.rs:276` | path → `AreaRivers`; `09-interfaces.md § GET /v1/area/{ax}/{ay}/rivers` |
| GET | `/v1/area/{ax}/{ay}/lakes` | `crates/arda-server/src/routes.rs:292` | path → `AreaLakes`; `09-interfaces.md § GET /v1/area/{ax}/{ay}/lakes` |
| GET | `/v1/point` | `crates/arda-server/src/routes.rs:308` | query `x_m`,`y_m: f64` → `PointSample`; `09-interfaces.md § GET /v1/point` |
| GET | `/v1/overview.png` | `crates/arda-server/src/routes.rs:321` | query (incl. `overlay=society|none`, `crates/arda-server/src/routes.rs:344`) → PNG; `09-interfaces.md § GET /v1/overview.png` |
| GET | `/v1/tiles/overview/{z}/{x}/{y}` | `crates/arda-server/src/routes.rs:369` | path (`{y}` carries the image extension) + query (`overlay`, `labels`, `crates/arda-server/src/routes.rs:344`, `crates/arda-server/src/routes.rs:357`) → PNG/WebP tile; `09-interfaces.md § GET /v1/tiles/overview/{z}/{x}/{y}.webp`, `§ GET /v1/tiles/overview/{z}/{x}/{y}.png` |
| GET | `/v1/tiles/relief/{z}/{x}/{y}` | `crates/arda-server/src/routes.rs:410` | path + `labels` query → WebP tile; `09-interfaces.md § GET /v1/tiles/relief/{z}/{x}/{y}.webp` |
| POST | `/v1/npc/population` | `crates/arda-server/src/routes.rs:430` | JSON `PopulationRequest` (body limit `npc::MAX_BODY_BYTES`, `crates/arda-server/src/routes.rs:42`) → population JSON; `09-interfaces.md § POST /v1/npc/population` |
| POST | `/v1/npc/generate` | `crates/arda-server/src/npc.rs:217` | JSON body (limit `npc::MAX_GENERATE_BODY_BYTES`, `crates/arda-server/src/npc.rs:33`, `crates/arda-server/src/routes.rs:46`) → one generated NPC; `09-interfaces.md § POST /v1/npc/generate` |
| GET | `/v1/npc/demo` | `crates/arda-server/src/routes.rs:451` | none → demo population; `09-interfaces.md § GET /v1/npc/demo` |
| GET | `/v1/npc/demo/{npc_id}` | `crates/arda-server/src/routes.rs:458` | path → one NPC; `09-interfaces.md § GET /v1/npc/demo/{npc_id}` |
| GET | `/v1/npc/{npc_id}` | `crates/arda-server/src/npcs/mod.rs:217` | path → stored NPC; `09-interfaces.md § GET /v1/npc/{npc_id}` |
| GET | `/v1/npcs` | `crates/arda-server/src/npcs/mod.rs:127` | query → `NpcPage`; `09-interfaces.md § GET /v1/npcs` |
| GET | `/v1/buildings/{building}/residents` | `crates/arda-server/src/npcs/mod.rs:155` | path → NPC list; `09-interfaces.md § GET /v1/buildings/{building}/residents` |
| GET | `/v1/buildings/{building}/workers` | `crates/arda-server/src/npcs/mod.rs:164` | path → NPC list; `09-interfaces.md § GET /v1/buildings/{building}/workers` |
| GET | `/v1/settlements` | `crates/arda-server/src/people.rs:51` | query → settlement list; `09-interfaces.md § GET /v1/settlements` |
| GET | `/v1/settlements/{id}` | `crates/arda-server/src/people.rs:72` | path → settlement; `09-interfaces.md § GET /v1/settlements/{id}` |
| GET | `/v1/settlements/{id}/plan` | `crates/arda-server/src/people.rs:89` | path → town plan; `09-interfaces.md § GET /v1/settlements/{id}/plan` |
| GET | `/v1/settlements/{id}/npcs` | `crates/arda-server/src/people.rs:106` | path → NPC list; `09-interfaces.md § GET /v1/settlements/{id}/npcs` |
| GET | `/v1/gazetteer` | `crates/arda-server/src/gazetteer.rs:68` | query `kind`, `bbox`, `min_rank` → named natural features and settlements as map labels (404 without `society/gazetteer.json`); `09-interfaces.md § GET /v1/gazetteer` |
| GET | `/v1/realms` | `crates/arda-server/src/realms.rs:67` | none → realm list; `09-interfaces.md § GET /v1/realms` |
| GET | `/v1/realms/{id}` | `crates/arda-server/src/realms.rs:92` | path → realm detail; `09-interfaces.md § GET /v1/realms/{id}` |
| GET | `/v1/schema` | `crates/arda-server/src/schema/routes.rs:34` | none → schema index; `09-interfaces.md § GET /v1/schema` |
| GET | `/v1/schema/{name}` | `crates/arda-server/src/schema/routes.rs:42` | path → JSON Schema; `09-interfaces.md § GET /v1/schema/{Name}.json` |
| GET | `/v1/tactical/layouts` | `crates/arda-server/src/tactical/routes.rs:186` | none → `TacticalLayouts`; `09-interfaces.md § GET /v1/tactical/layouts` |
| GET | `/v1/tactical/layout/{file}` | `crates/arda-server/src/tactical/routes.rs:193` | path (`name` or `name.png`) → layout JSON or PNG; `09-interfaces.md § GET /v1/tactical/layout/{name}`, `§ GET /v1/tactical/layout/{name}.png` |
| GET | `/v1/tactical/layout/{name}/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/routes.rs:243` | path → WebP tile; `09-interfaces.md § GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp` |
| POST | `/v1/tactical/render` | `crates/arda-server/src/tactical/routes.rs:281` | `TacticalLayout` JSON body (limit `max_body_bytes`, `crates/arda-server/src/tactical/routes.rs:44`) → PNG; `09-interfaces.md § POST /v1/tactical/render` |
| GET | `/v1/tactical/cell/{gx}/{gy}` | `crates/arda-server/src/tactical/world_routes.rs:149` | path (`gy` or `gy.png`) + query → `TacticalBlockDto` JSON or PNG; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}`, `§ GET /v1/tactical/cell/{gx}/{gy}.png` |
| GET | `/v1/tactical/cell/{gx}/{gy}/scene` | `crates/arda-server/src/tactical/scene.rs:94` | path + query → `TacticalScene`; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}/scene` |
| GET | `/v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/world_routes.rs:190` | path + query → WebP tile; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp` |
| GET | `/v1/tactical/window` | `crates/arda-server/src/tactical/world_routes.rs:277` | query → window JSON; `09-interfaces.md § GET /v1/tactical/window` |
| GET | `/v1/tactical/window.png` | `crates/arda-server/src/tactical/world_routes.rs:341` | query → PNG; `09-interfaces.md § GET /v1/tactical/window.png` |
| GET | `/v1/tactical/window/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/world_routes.rs:301` | path + query → WebP tile; `09-interfaces.md § GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp` |
| GET | `/v1/tactical/library` | `crates/arda-server/src/tactical/world_routes.rs:371` | none → `TacticalLibraryDto`; `09-interfaces.md § GET /v1/tactical/library` |
| POST | `/v1/tactical/prefetch` | `crates/arda-server/src/tactical/routes.rs:68` | JSON `PrefetchRequest` → 202 `PrefetchAccepted`; `09-interfaces.md § POST /v1/tactical/prefetch` |
| POST | `/v1/tactical/bake` | `crates/arda-server/src/tactical/bake_routes.rs:54` | JSON `BakeRequest` (body limit `max_body_bytes`, `crates/arda-server/src/tactical/bake_routes.rs:38`) → `202 BakeAccepted` (`200` once finished); `09-interfaces.md § POST /v1/tactical/bake` |
| GET | `/v1/tactical/bake/{id}` | `crates/arda-server/src/tactical/bake_routes.rs:98` | path → `BakeStatus`; `09-interfaces.md § GET /v1/tactical/bake/{id}` |
| DELETE | `/v1/tactical/bake/{id}` | `crates/arda-server/src/tactical/bake_routes.rs:110` | path → `204`, frees the tile set; `09-interfaces.md § POST /v1/tactical/bake` |
| GET | `/v1/tactical/bake/{id}/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/bake_routes.rs:118` | path → WebP tile (strong `ETag`, `Cache-Control: no-store`); `09-interfaces.md § GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp` |
| GET | `/v1/tactical/dungeon` | `crates/arda-server/src/tactical/dungeon.rs:78` | query → dungeon layout JSON; `09-interfaces.md § GET /v1/tactical/dungeon` |
| GET | `/v1/tactical/dungeon.png` | `crates/arda-server/src/tactical/dungeon.rs:110` | query → PNG; `09-interfaces.md § GET /v1/tactical/dungeon.png` |
| GET | `/v1/tactical/dungeon/scene` | `crates/arda-server/src/tactical/dungeon.rs:91` | query → scene JSON; `09-interfaces.md § GET /v1/tactical/dungeon/scene` |

Wire types are exported as TypeScript by ts-rs into `bindings/ts/arda/` (`09-interfaces.md § bindings/ts/arda`); `cargo test -p arda-server bindings` fails when they are stale (`crates/arda-server/src/bindings.rs:1`). The schema freeze guard compares each JSON Schema with a committed baseline and lists breaking changes (`crates/arda-server/src/schema/compat.rs:1`); version families are in `crates/arda-server/src/versions.rs:1`.

### HTTP clients

| Client → server | Send site | Calls | Payload |
|---|---|---|---|
| Viewer → arda-server | `ArdaClient` `apps/viewer/src/api/client.ts:148`; JSON reads via `contractJson` `apps/viewer/src/api/client.ts:218` | health, world, cell, point, area cells (JSON and `?format=bin`), rivers, lakes, overview/relief tile templates, tactical layouts/layout/tiles/library/cell/scene/render/prefetch (`apps/viewer/src/api/client.ts:223`–`apps/viewer/src/api/client.ts:381`) | Same types as the server rows above, imported from `@arda` (`apps/viewer/src/api/client.ts:20`); binary area layout parsed by `parseAreaColumnsBin` (`apps/viewer/src/api/areaBin.ts:27`). The viewer calls none of the NPC, settlement, schema, window or dungeon routes |
| art-gen → Slopify (external, `SLOPIFY_URL`, default `http://127.0.0.1:6969`) | `client` `tools/art-gen/generate.mjs:444`, `fetch` at `tools/art-gen/generate.mjs:449` | `/api/health`, `/api/channels`, `/api/prompts`, `/api/providers`, `/api/staging/images`, `/api/projects`, `/api/projects/batch`, `/api/projects/{id}`, `/api/projects/{id}/cancel` (`tools/art-gen/generate.mjs:489`–`tools/art-gen/generate.mjs:931`) | Batch body built by `batchBody` (`tools/art-gen/generate.mjs:296`), single-project body by `projectBody` (`tools/art-gen/generate.mjs:345`); GETs retry with delays `[2,5,10,20,30,60]` s, writes do not (`tools/art-gen/generate.mjs:442`) |

### In-process and file handoffs

Everything else is synchronous in-process calls plus files.

| Send site → receive site | Request | Return |
|---|---|---|
| `run_generate` `crates/arda-cli/src/generate.rs:85` → `arda::generate` `crates/arda/src/lib.rs:45` → `arda_gen::generate_world` | `seed: u64`, `GenerateConfig`, `out: &Path` | `Manifest` or `GenError`; world directory on disk |
| `run_generate` `crates/arda-cli/src/generate.rs:95` → `arda::generate_from_fine_recipe` `crates/arda/src/lib.rs:86` → `arda_gen::orchestrator::generate_world_with_fine_recipe` | `seed`, `GenerateConfig`, `out`, `FineDeliveryLimits`, `recipe_version: u16` (4–7) | `Manifest` or `GenError::Validation` for an unsupported recipe (`crates/arda/src/lib.rs:93`) |
| `arda_settle::generate` `crates/arda-settle/src/lib.rs:171` → `arda::World::load`, `pipeline::run` | `world_dir: &Path`, `budget: u64` | `(Society, PathBuf)`; writes `settlements.json`, `roads.json`, `realms.json`, `names.json`, `gazetteer.json`, `landuse.bin`, `realms.bin`, `roads.bin`, `stats.json` under `<world>/society/` (`crates/arda-settle/src/lib.rs:73`–`crates/arda-settle/src/lib.rs:143`) |
| `arda_people::build` `crates/arda-people/src/build.rs:45` → `compute` `crates/arda-people/src/build.rs:58` → `arda_society::simulate_society` `crates/arda-people/src/build.rs:78` → `arda_npc::Generator::with_notables` `crates/arda-people/src/build.rs:94` | `world_dir`; reads `<world>/society/` files written by arda-settle | `BuildReport`; writes `society.json` and `notables.json` (`crates/arda-people/src/build.rs:48`) |
| `Pipeline::window` `crates/arda-blocks/src/pipeline.rs:165` → `arda_refine::refine` `crates/arda-refine/src/block.rs:167` (per cell, cached, `crates/arda-blocks/src/pipeline.rs:112`) | `(gsx0, gsy0, w, h)` window in squares, `&Library`, optional `&dyn Overlays` | `Composed { layout: TacticalLayout, rules: RulesSidecar, meta: MapMeta, owners }` (`crates/arda-blocks/src/pipeline.rs:23`) or `BlocksError` |
| `arda-blocks` society overlays → `arda_ways::apply_ways` `crates/arda-blocks/src/society/layers.rs:166`, `arda_fields::generate` `crates/arda-blocks/src/society/layers.rs:350`, `arda_town::block::generate` `crates/arda-blocks/src/society/town.rs:145` | layout window, world roads/rivers/land use and town plans from `arda_people::World` | Overlay squares composed by `compose_all` (`crates/arda-blocks/src/overlays.rs:296`) |
| Bake handler `crates/arda-server/src/tactical/bake_routes.rs:54` → `Baker::submit` `crates/arda-server/src/tactical/bake.rs:189` → worker `work` `crates/arda-server/src/tactical/bake.rs:349` | `BakeRequest` (client-edited cell layout) on a bounded queue of `QUEUE_JOBS` (`crates/arda-server/src/tactical/bake.rs:35`); the cell is spliced into its apron block by `crates/arda-server/src/tactical/splice.rs` | Content-addressed job id in `202 BakeAccepted`; finished tile sets wait in the byte-budgeted `bake_store` until `DELETE` or eviction (`crates/arda-server/src/tactical/bake.rs:1`) |
| Overview export with society → `arda_midzoom::export::export_overview` `crates/arda-midzoom/src/export.rs:62` | world, quality, `(MapStyle, look)`, `SocietyMode` | PNG path; overviews take `arda_settle::render::band::SocietyBandOverlay`, which paints land tone, roads and tier symbols into the streamed bands through `arda_render::BandOverlay` (`crates/arda-midzoom/src/export.rs:73`, `crates/arda-settle/src/render/band.rs:177`, `crates/arda-render/src/overlay.rs:18`); areas take the relief tiles' worked land (`crates/arda-midzoom/src/export.rs:94`) |
| Server relief tiles → `arda_midzoom::render_tile` `crates/arda-server/src/relief.rs:149` | `ReliefWorld`, `Pyramid`, `z, x, y` | RGBA tile, encoded by the server |
| Prefetch handler `crates/arda-server/src/tactical/routes.rs:80` → `Prefetcher::submit` `crates/arda-server/src/tactical/prefetch.rs:198` → worker `work` `crates/arda-server/src/tactical/prefetch.rs:314` | internal `Job { req: BlockRequest, ppsq: u32, image: bool, world: (i64, i64) }` on a mutex+condvar `VecDeque` (`crates/arda-server/src/tactical/prefetch.rs:141`, `crates/arda-server/src/tactical/prefetch.rs:149`); `BlockRequest { gsx0, gsy0: i64, w, h: u32, demo_at: Option<[i64; 2]> }` (`crates/arda-server/src/tactical/block.rs:34`) | No return to the client beyond 202; workers fill the same caches requests read (`crates/arda-server/src/tactical/prefetch.rs:14`) |
| `generate.mjs` → `arda tactical import` | files on disk: downloaded raw images plus `import.toml` (`tools/art-gen/generate.mjs:564`, `tools/art-gen/generate.mjs:594`) | `arda_art_import::import` reads `raw_dir` and writes a library plus `report.md` (`crates/arda-cli/src/tactical_import.rs:51`, `crates/arda-cli/src/tactical_import.rs:74`) |

## Composition

No DI container exists in any crate; wiring is plain values, `Arc`s and trait objects built in `main` or `open` functions.

- `arda` CLI: `main` parses clap once and matches `Command` to private handlers (`crates/arda-cli/src/main.rs:399`); handlers call library functions directly (`crates/arda-cli/src/society.rs:29`, `crates/arda-cli/src/tactical.rs:94`, `crates/arda-cli/src/dungeon.rs:88`).
- arda-server: `main` builds `ServerConfig` from flags (`crates/arda-server/src/main.rs:58`), then `AppState::open` (`crates/arda-server/src/lib.rs:204`) wires, in order: admission (`config.admit`), optional sheet mapping, `WorldQuery::open`, `Tactical::open` with the library stack, `arda_people::shared::SharedSource::open`, an optional `arda_people::World` when `society/settlements.json` exists (with `warm_towns`), `arda_blocks::society::SocietyOverlays`, an `arda_midzoom::Landscape`, the `RefineBlocks` block source installed via `set_block_source` (`crates/arda-server/src/tactical/mod.rs:231`), `Overview`, `Relief` (gazetteer labels enabled on both when `society/gazetteer.json` loaded, `crates/arda-server/src/lib.rs:258`), the `area_builds` semaphore, the `Prefetcher` and the `Baker` (`crates/arda-server/src/lib.rs:205`–`crates/arda-server/src/lib.rs:274`). `router(Arc<AppState>)` attaches the state (`crates/arda-server/src/routes.rs:75`). `BlockSource` is the replaceable seam for tactical blocks (`crates/arda-server/src/tactical/block.rs:115`, `crates/arda-server/src/lib.rs:299`).
- `RefineBlocks::with_society` composes an `arda_blocks::Pipeline` over a `SharedSource` with optional society overlays (`crates/arda-server/src/tactical/refine_blocks.rs:74`).
- arda-midzoom binary: `World::load` → `ReliefWorld::new` → optional `Landscape::open` → `render_window` (`crates/arda-midzoom/src/bin/arda-midzoom.rs:43`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:50`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:53`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:74`).
- arda-settle: `generate` loads the world, stitches a grid (`load::load_grid`), runs `pipeline::run`, then writes all files only after every encoding succeeds (`crates/arda-settle/src/lib.rs:172`–`crates/arda-settle/src/lib.rs:180`, `crates/arda-settle/src/lib.rs:156`). `pipeline::run` sequences tags, suitability, placement, seats, cultures, land use, cost surface, roads, crossings, realms and naming (`crates/arda-settle/src/pipeline.rs:56`).
- arda-people: `plan_all` plans settlements on scoped threads in chunks, preserving settlement order (`crates/arda-people/src/build.rs:122`); society simulation and NPC generation follow sequentially (`crates/arda-people/src/build.rs:78`).
- arda-blocks: `Pipeline::refine_all` refines missing cells on scoped threads, then `assemble` joins blocks and overlays are composed (`crates/arda-blocks/src/pipeline.rs:127`, `crates/arda-blocks/src/pipeline.rs:196`).
- Viewer: `createRoot(...).render(<App/>)` (`apps/viewer/src/main.tsx:9`); `App` holds one `ArdaClient` memoised on the API base (`apps/viewer/src/App.tsx:26`).

## Frontend

`apps/viewer` is a developer viewer for the arda-server v1 API (`apps/viewer/package.json:5`), not a player-facing product.

| Aspect | Fact |
|---|---|
| Rendering model | Client-only SPA: one `index.html` with `#root` and a module script (`apps/viewer/index.html:9`, `apps/viewer/index.html:10`); React 19 `createRoot` under `StrictMode` (`apps/viewer/src/main.tsx:9`). No SSR/SSG |
| Build | Vite with `@vitejs/plugin-react`; `build` = `tsc -b && vite build`; Vitest in node environment for `src/**/*.test.ts` (`apps/viewer/package.json:10`, `apps/viewer/vite.config.ts:12`, `apps/viewer/vite.config.ts:20`); `npm run licences` checks the npm runtime licence policy and `licenceNotices()` writes bundle notices (`apps/viewer/package.json:15`, `apps/viewer/vite.config.ts:12`, `apps/viewer/licence-policy.json:1`) |
| Routing | Hash routing without a router library: `VIEWS = world, area, tactical, cell, editor` read from `location.hash` and a `hashchange` listener (`apps/viewer/src/App.tsx:12`, `apps/viewer/src/App.tsx:17`, `apps/viewer/src/App.tsx:33`). `tactical`, `cell`, `editor` render without `/v1/world` (`apps/viewer/src/App.tsx:15`) |
| Views | `WorldView` (Leaflet overview/relief pyramid, `apps/viewer/src/views/WorldView.tsx:15`), `AreaView` (canvas heatmap, `apps/viewer/src/views/AreaView.tsx:28`), `TacticalView` (layout pyramid, `apps/viewer/src/views/TacticalView.tsx:28`), `TacticalCellView` (world tactical cells with prefetch, `apps/viewer/src/views/TacticalCellView.tsx:21`), `LayoutEditor` (edits ground and POSTs to `/v1/tactical/render`, `apps/viewer/src/views/LayoutEditor.tsx:24`), `CellInspector` (`apps/viewer/src/views/CellInspector.tsx:4`) |
| Map library | Leaflet 1.9 with a flat CRS aligned to the server pyramid (`apps/viewer/package.json:18`, `apps/viewer/src/geo/leafletCrs.ts:3`); `TacticalMap` loads tiles with `fetch` to read `ETag` and `X-Arda-Cache` (`apps/viewer/src/components/TacticalMap.tsx:28`) |
| Design system | None: one hand-written `src/styles.css` plus Leaflet's CSS (`apps/viewer/src/main.tsx:3`, `apps/viewer/src/main.tsx:4`); no component library dependency (`apps/viewer/package.json:17`) |
| API-client seam | `ArdaClient` (`apps/viewer/src/api/client.ts:148`) with base resolved from `?api=`, then `localStorage` key `arda-viewer.apiBase`, then `VITE_ARDA_API`, then `http://localhost:8787` (`apps/viewer/src/config.ts:9`, `apps/viewer/src/api/client.ts:52`); response types come from ts-rs bindings via the `@arda` alias (`apps/viewer/vite.config.ts:14`), with runtime shape checks in `apps/viewer/src/api/tactical.ts` and `apps/viewer/src/api/scene.ts` |

No other human-facing UI exists: the Rust binaries are CLIs and arda-server serves JSON and images only.
