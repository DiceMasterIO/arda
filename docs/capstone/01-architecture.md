---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: b8486570eb78
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
---

# Architecture

The workspace is 23 Rust crates under `crates/` (`Cargo.toml:2` globs `crates/*`), one TypeScript app `apps/viewer/`, Node tooling `tools/art-gen/`, and two offline Python table generators in `tools/`. The root package `arda-workspace-tests` has no library or binary of its own and hosts the workspace integration tests in `tests/` (`Cargo.toml:76`, `Cargo.toml:104`).

## Layers

Tiers are derived from each crate's `[dependencies]` (dev-dependencies excluded) and cross-checked against the `arda_*::` paths actually used in each `src/`; the two agree for every crate. A crate depends only on crates in lower tiers.

| Tier | Crate | Directory | Workspace dependencies (normal) | Responsibility |
|---|---|---|---|---|
| 0 | arda-core | `crates/arda-core/` | none (`crates/arda-core/Cargo.toml:9`) | Shared types, keyed PRNG, the only world byte codec (`crates/arda-core/src/lib.rs:1`) |
| 0 | arda-ids | `crates/arda-ids/` | none (`crates/arda-ids/Cargo.toml:10`) | Shared ids, canonical enums and the stable hash for product layers (`crates/arda-ids/src/lib.rs:1`) |
| 0 | arda-names | `crates/arda-names/` | none (`crates/arda-names/Cargo.toml:10`) | Phonotactic naming languages (`crates/arda-names/src/lib.rs:1`) |
| 0 | arda-society | `crates/arda-society/` | none (`crates/arda-society/Cargo.toml:10`) | Realm politics, economy, factions, history, campaign hooks (`crates/arda-society/src/lib.rs:3`) |
| 0 | arda-tactical | `crates/arda-tactical/` | none (`crates/arda-tactical/Cargo.toml:9`) | Asset catalogue, validator, placeholder library, `TacticalLayout`, compositor (`crates/arda-tactical/src/lib.rs:1`) |
| 0 | arda-wfc | `crates/arda-wfc/` | none (`crates/arda-wfc/Cargo.toml:9`) | Generic deterministic wave-function collapse (`crates/arda-wfc/src/lib.rs:1`) |
| 1 | arda-gen | `crates/arda-gen/` | arda-core (`crates/arda-gen/Cargo.toml:10`) | Continent, area, block, formation, hydrology stages; orchestrator is the only disk caller (`crates/arda-gen/src/lib.rs:1`, `crates/arda-gen/src/lib.rs:4`) |
| 1 | arda-render | `crates/arda-render/` | arda-core (`crates/arda-render/Cargo.toml:10`) | PNG/JSON export of stored worlds; never depends on `arda-gen` (`crates/arda-render/src/lib.rs:1`) |
| 1 | arda-npc | `crates/arda-npc/` | arda-ids, arda-names (`crates/arda-npc/Cargo.toml:11`) | NPC populations with jobs, personalities, SRD 5.1 sheets (`crates/arda-npc/src/lib.rs:1`) |
| 1 | arda-scene | `crates/arda-scene/` | arda-tactical (`crates/arda-scene/Cargo.toml:10`) | SRD movement/sight/cover/light scene from a `TacticalLayout` (`crates/arda-scene/src/lib.rs:1`) |
| 2 | arda | `crates/arda/` | arda-core, arda-gen, arda-render (`crates/arda/Cargo.toml:10`) | Public world facade: generation, `World` load/query, exports (`crates/arda/src/lib.rs:1`) |
| 2 | arda-art-import | `crates/arda-art-import/` | arda-tactical (`crates/arda-art-import/Cargo.toml:10`) | Raw AI images to a valid tactical asset library (`crates/arda-art-import/src/lib.rs:1`) |
| 2 | arda-dungeon | `crates/arda-dungeon/` | arda-scene, arda-tactical (`crates/arda-dungeon/Cargo.toml:10`) | Dungeon and cave battle maps (`crates/arda-dungeon/src/lib.rs:1`) |
| 2 | arda-fields | `crates/arda-fields/` | arda-scene, arda-tactical (`crates/arda-fields/Cargo.toml:11`) | 5-ft land use around settlements (`crates/arda-fields/src/lib.rs:1`) |
| 2 | arda-ways | `crates/arda-ways/` | arda-scene, arda-tactical (`crates/arda-ways/Cargo.toml:10`) | 5-ft roads, verges, bridges, fords, ferries (`crates/arda-ways/src/lib.rs:1`) |
| 2 | arda-town | `crates/arda-town/` | arda-scene, arda-tactical, arda-wfc (`crates/arda-town/Cargo.toml:17`) | Town street/plot plans and 5-ft building blocks (`crates/arda-town/src/lib.rs:1`) |
| 3 | arda-refine | `crates/arda-refine/` | arda, arda-core, arda-wfc, arda-scene, arda-tactical (`crates/arda-refine/Cargo.toml:14`) | One 100 m world cell as a 64 × 64 block of 5-ft squares (`crates/arda-refine/src/lib.rs:1`) |
| 3 | arda-settle | `crates/arda-settle/` | arda, arda-core, arda-ids, arda-names (`crates/arda-settle/Cargo.toml:11`) | Settlements, roads, realms, names; writes `<world>/society/` (`crates/arda-settle/src/lib.rs:1`) |
| 4 | arda-people | `crates/arda-people/` | arda, arda-core, arda-ids, arda-names, arda-npc, arda-refine, arda-settle, arda-society, arda-town (`crates/arda-people/Cargo.toml:11`) | Town plans on world terrain, society, stored notables (`crates/arda-people/src/lib.rs:1`) |
| 5 | arda-blocks | `crates/arda-blocks/` | arda, arda-refine, arda-scene, arda-tactical, arda-ways, arda-fields, arda-town, arda-ids, arda-people, arda-settle (`crates/arda-blocks/Cargo.toml:10`) | World-derived tactical blocks: refine + ways/fields/town overlays (`crates/arda-blocks/src/lib.rs:1`) |
| 6 | arda-midzoom | `crates/arda-midzoom/` | arda, arda-blocks, arda-core, arda-fields, arda-ids, arda-people, arda-refine, arda-render, arda-settle, arda-town, arda-ways (`crates/arda-midzoom/Cargo.toml:14`) | On-demand ~10 m relief refinement and tiles (`crates/arda-midzoom/src/lib.rs:1`) |
| 7 | arda-cli | `crates/arda-cli/` | arda, arda-people, arda-settle, arda-tactical, arda-dungeon, arda-scene, arda-art-import (`crates/arda-cli/Cargo.toml:14`) | The `arda` binary (`crates/arda-cli/Cargo.toml:10`) |
| 7 | arda-server | `crates/arda-server/` | arda, arda-core, arda-npc, arda-blocks, arda-dungeon, arda-ids, arda-people, arda-midzoom, arda-refine, arda-scene, arda-settle, arda-society, arda-tactical, arda-town (`crates/arda-server/Cargo.toml:17`) | World-query contract and HTTP service (`crates/arda-server/src/lib.rs:1`) |

Dev-only workspace edges (tests and examples, not production): arda-blocks → arda-society (`crates/arda-blocks/Cargo.toml:25`), arda-midzoom → arda-tactical (`crates/arda-midzoom/Cargo.toml:31`), arda-settle → arda-npc, arda-society (`crates/arda-settle/Cargo.toml:28`), arda-server → arda-names (`crates/arda-server/Cargo.toml:50`; its only `src/` mentions are doc comments and tests, `crates/arda-server/src/npc_dto/input.rs:106`). The root test package dev-depends on arda, arda-core, arda-midzoom, arda-npc, arda-people, arda-server, arda-settle, arda-town (`Cargo.toml:87`).

Feature `schema` (optional `schemars` derives) exists on arda-ids, arda-names, arda-settle, arda-society, arda-town; arda-server enables it on arda-settle, arda-society and arda-town (`crates/arda-server/Cargo.toml:27`, `crates/arda-settle/Cargo.toml:25`).

Two dependency stacks meet at tier 3:

- World stack: `arda-core` → `arda-gen`/`arda-render` → `arda` → (`arda-refine`, `arda-settle`) → `arda-people` → `arda-blocks` → `arda-midzoom`.
- Tactical stack: `arda-tactical` → `arda-scene` → (`arda-ways`, `arda-fields`, `arda-dungeon`, `arda-town` with `arda-wfc`) → joined into the world stack by `arda-refine` (tier 3) and `arda-blocks` (tier 5).
- Naming/people stack: `arda-ids`, `arda-names` → `arda-npc`; `arda-society` stands alone and is consumed by `arda-people`.

Out-of-workspace parts:

| Part | Directory | Depends on | Direction |
|---|---|---|---|
| apps/viewer | `apps/viewer/` | `arda-server` over HTTP; ts-rs bindings at `bindings/ts/arda/` through the `@arda` alias (`apps/viewer/vite.config.ts:7`, `apps/viewer/vite.config.ts:13`) | No Rust crate depends on it |
| tools/art-gen | `tools/art-gen/` | A local Slopify HTTP server (`tools/art-gen/generate.mjs:21`); writes a folder plus `import.toml` that `arda tactical import` reads (`tools/art-gen/generate.mjs:550`) | Consumed by arda-art-import via the CLI only as files |
| tools/*.py | `tools/generate_spectral_tables.py`, `tools/generate_water_forcing_tables.py` | none | Offline generators; the spectral one writes `crates/arda-gen/src/spectral_tables.rs` (`tools/generate_spectral_tables.py:6`); the water one writes the path given as `argv[1]` (`tools/generate_water_forcing_tables.py:70`) |

## Module boundaries

Boundaries are enforced only by Cargo manifests (a crate can name only its declared dependencies). No dependency-rule checker exists: CI runs tests, MSRV check, fmt, clippy and cargo-deny (`.github/workflows/ci.yml:15`, `.github/workflows/ci.yml:23`, `.github/workflows/ci.yml:32`, `.github/workflows/ci.yml:39`), and `deny.toml` bans no workspace edges (`deny.toml:7`). Workspace lints deny `unsafe_code`, `missing_docs`, `unwrap_used`, `expect_used` in every crate (`Cargo.toml:50`, `Cargo.toml:56`).

| Module | Public surface | May not import / not imported by |
|---|---|---|
| arda-core | Modules `cell`, `config`, `continent`, `coords`, `error`, `fixed`, `formats`, `hydrology`, `objects`, `playa`, `rng`, `terrain`, `tiles`, `water`; re-exports codecs `encode_cells`/`decode_cells`, `encode_blocks`, `TerrainFileReader`, rng `derive`/`rng`/`SeedKey` (`crates/arda-core/src/lib.rs:8`, `crates/arda-core/src/lib.rs:34`, `crates/arda-core/src/lib.rs:56`) | Imports no workspace crate. Imported by arda-gen, arda-render, arda, arda-refine, arda-settle, arda-people, arda-midzoom, arda-server |
| arda-ids | `biome`, `cover`, `function`, `hash`, `ids`, `land_use`, `road`; `BuildingId`, `NpcId`, `RealmId`, `SettlementId`, `digest`/`hash`/`subseed` (`crates/arda-ids/src/lib.rs:19`, `crates/arda-ids/src/lib.rs:30`) | Imports no workspace crate |
| arda-names | `Language`, `Preset`, `Tongue`, `place_name`, `person_name`, `given_name`, `NameScope` (`crates/arda-names/src/lib.rs:47`) | Imports no workspace crate |
| arda-society | `simulate_society(seed, &WorldSettlements) -> Society` (`crates/arda-society/src/lib.rs:134`); `Society`, `SettlementSociety`, `RealmSociety`, `FORMAT_VERSION` (`crates/arda-society/src/lib.rs:56`) | Imports no workspace crate; it reads settlement data through its own `input::WorldSettlements` rather than arda-settle types (`crates/arda-society/src/lib.rs:53`) |
| arda-tactical | `Catalog`, `Library`, `TacticalLayout`, `render`/`render_with`/`render_region_tinted`, `WorldTint`, `Issue`/`Rule`/`Thresholds` (`crates/arda-tactical/src/lib.rs:26`) | Imports no workspace crate; it knows nothing of worlds |
| arda-wfc | `solve`, `Problem`, `Solution`, `Rules`, `TileSet`, `violations` (`crates/arda-wfc/src/lib.rs:22`) | Imports no workspace crate |
| arda-gen | `area`, `block`, `continent`, `formation`, `hydrology`, `noise`, `orchestrator`, `spectral`, `spectral_composition`; root `generate_world`, `generate_world_with_limits`, `GenError`, `HydrologyLimits` (`crates/arda-gen/src/lib.rs:11`, `crates/arda-gen/src/lib.rs:21`) | Imports only arda-core. Imported only by `arda` |
| arda-render | `carto`, `json`, `symbolic`; Atlas/overview APIs, `ImageQuality`, `AreaImageScale`, `RenderError` (`crates/arda-render/src/lib.rs:15`, `crates/arda-render/src/lib.rs:41`) | Never imports arda-gen (`crates/arda-render/src/lib.rs:1`). Imported by `arda` and arda-midzoom |
| arda-npc | `Generator`, `generate_population`, `npc`, `commoner`, `Population`, `Sheet`, `Resident` (`crates/arda-npc/src/lib.rs:42`, `crates/arda-npc/src/lib.rs:50`, `crates/arda-npc/src/lib.rs:215`) | Imports only arda-ids, arda-names |
| arda-scene | `build_scene`, `SceneIndex`, `RulesSidecar`, `scene_debug_png` (`crates/arda-scene/src/lib.rs:34`, `crates/arda-scene/src/lib.rs:51`) | Imports only arda-tactical |
| arda | `generate`, `generate_with_limits`, `generate_from_fine_source`, `generate_from_fine_recipe`, `export_area`, `export_area_with_scale`, `export_overview`, `World`, `Area`, `MapStyle`, `ExportFormat` (`crates/arda/src/lib.rs:28`, `crates/arda/src/lib.rs:44`, `crates/arda/src/lib.rs:85`, `crates/arda/src/lib.rs:134`, `crates/arda/src/lib.rs:224`) | Consumers are documented to depend on this crate only (`crates/arda/src/lib.rs:1`); arda-refine, arda-settle, arda-people, arda-midzoom and arda-server also import arda-core directly |
| arda-art-import | `import(&ImportOptions) -> Report` plus 20 public pipeline modules (`crates/arda-art-import/src/lib.rs:25`, `crates/arda-art-import/src/lib.rs:97`) | Imports only arda-tactical. Imported only by arda-cli |
| arda-dungeon | `generate(&Params) -> Dungeon`, `site_seed`, `Kind` (`crates/arda-dungeon/src/lib.rs:227`, `crates/arda-dungeon/src/lib.rs:239`) | Imported by arda-cli and arda-server |
| arda-fields | `fields_window`, `generate`, `FieldsWindow` (`crates/arda-fields/src/lib.rs:43`) | Imported only by arda-blocks and arda-midzoom |
| arda-ways | `apply_ways`, `WaysOutput`, `Sidecar` (`crates/arda-ways/src/lib.rs:29`, `crates/arda-ways/src/lib.rs:101`) | Imported only by arda-blocks and arda-midzoom |
| arda-town | `generate`, `TownPlan`, `TownSite`, `TerrainInput`, `block` module (`crates/arda-town/src/lib.rs:17`, `crates/arda-town/src/lib.rs:31`) | Imports no world crate: plans take a `TownSite` and `TerrainInput` (`crates/arda-town/src/lib.rs:3`) |
| arda-refine | `refine`, `refine_block`, `refine_window`, `Block`, `Source`, `CellKey`, `WorldSource`, `MAX_WINDOW` (`crates/arda-refine/src/lib.rs:54`, `crates/arda-refine/src/lib.rs:61`, `crates/arda-refine/src/lib.rs:94`) | Reads worlds through the `Source` trait; `WorldSource` adapts `arda::World` |
| arda-settle | `generate(world_dir, budget)`, `write`, `encode`, `run`, `Society`, `SOCIETY_DIR` (`crates/arda-settle/src/lib.rs:48`, `crates/arda-settle/src/lib.rs:55`, `crates/arda-settle/src/lib.rs:160`) | Reads the stored world via `arda::World::load` (`crates/arda-settle/src/lib.rs:161`) |
| arda-people | `build`, `compute`, `World`, `SocietyFiles`, `NotablesFile` (`crates/arda-people/src/lib.rs:32`) | Reads `<world>/society/` written by arda-settle (`crates/arda-people/src/build.rs:60`) |
| arda-blocks | `Pipeline`, `Composed`, `Overlays`, `SQUARES_PER_CELL`, modules `demo`, `overlays`, `pipeline`, `society` (`crates/arda-blocks/src/lib.rs:21`, `crates/arda-blocks/src/lib.rs:26`) | Imported by arda-midzoom and arda-server |
| arda-midzoom | `ReliefWorld`, `Landscape`, `Pyramid`, `render_tile`, `render_window`, `refine_tile` (`crates/arda-midzoom/src/lib.rs:43`, `crates/arda-midzoom/src/lib.rs:48`, `crates/arda-midzoom/src/lib.rs:58`) | Imported only by arda-server |
| arda-cli | No library; private subcommand modules `dungeon`, `generate`, `society`, `tactical`, `tactical_import` (`crates/arda-cli/src/main.rs:12`) | Nothing imports it |
| arda-server | Library `router`, `AppState`, `ServerConfig`, `WorldQuery`, `CellSample`, `PointSample`, `CONTRACT_VERSION`, `BlockSource` plus 22 public modules (`crates/arda-server/src/lib.rs:12`, `crates/arda-server/src/lib.rs:35`) | Imported only by the root test package (`Cargo.toml:93`) |

Examples (shallow: not inventoried beyond names): `crates/arda/examples/`, `crates/arda-core/examples/`, `crates/arda-gen/examples/` (11 files), `crates/arda-art-import/examples/`, `crates/arda-fields/examples/`, `crates/arda-names/examples/`, `crates/arda-npc/examples/`, `crates/arda-scene/examples/`, `crates/arda-server/examples/`, `crates/arda-society/examples/`, `crates/arda-ways/examples/crossings/`.

## Entry points

| Process | Entry `file:line` | Commands / behaviour |
|---|---|---|
| `arda` (crate arda-cli) | `crates/arda-cli/src/main.rs:360` | clap `Command` enum (`crates/arda-cli/src/main.rs:34`), dispatched at `crates/arda-cli/src/main.rs:362`–`crates/arda-cli/src/main.rs:408` |
| `arda generate` | `crates/arda-cli/src/main.rs:36` → `run_generate` `crates/arda-cli/src/generate.rs:45` | `--terrain legacy` (default, `crates/arda-cli/src/main.rs:51`) calls `arda::generate`; `--terrain fine` calls `generate_from_fine_recipe` (`crates/arda-cli/src/generate.rs:85`) |
| `arda preview` | `crates/arda-cli/src/main.rs:73` → `run_preview` `crates/arda-cli/src/main.rs:221` | Generates then writes an overview via `export_overview` (`--px`) or `export_overview_with_look` (`crates/arda-cli/src/main.rs:253`) |
| `arda export` | `crates/arda-cli/src/main.rs:97` → `run_export` `crates/arda-cli/src/main.rs:270` | Block, overview or area as PNG/JSON (`crates/arda-cli/src/main.rs:327`) |
| `arda settle` | `crates/arda-cli/src/main.rs:129` → `society::settle` `crates/arda-cli/src/society.rs:28` | `arda_settle::generate` writes `<world>/society/` |
| `arda society build` | `crates/arda-cli/src/main.rs:136` → `society::run` `crates/arda-cli/src/society.rs:41` | `arda_people::build` writes `society.json` and `notables.json` |
| `arda tactical validate\|import\|placeholders\|render` | `crates/arda-cli/src/main.rs:141` → `tactical::run` `crates/arda-cli/src/tactical.rs:65` | Library validation, `arda_art_import::import` (`crates/arda-cli/src/tactical_import.rs:41`), placeholder library, layout render |
| `arda dungeon` | `crates/arda-cli/src/main.rs:147` → `dungeon::run` `crates/arda-cli/src/dungeon.rs:82` | Layout JSON, optional rules/meta sidecars, scene JSON and PNG |
| `arda-server` | `crates/arda-server/src/main.rs:94` → `run` `crates/arda-server/src/main.rs:57` | Tokio multi-thread runtime; `AppState::open` on the blocking pool, binds `--host` (default `127.0.0.1`) `--port` (default 8787), serves until Ctrl-C (`crates/arda-server/src/main.rs:71`, `crates/arda-server/src/main.rs:86`) |
| `arda-settle` | `crates/arda-settle/src/main.rs:42` | `generate --world` (`arda_settle::generate`), `render --world --out` (`arda_settle::render::render`) (`crates/arda-settle/src/main.rs:45`, `crates/arda-settle/src/main.rs:64`) |
| `arda-town` | `crates/arda-town/src/main.rs:260` → `run` `crates/arda-town/src/main.rs:161` | `render`, `block`, `info`, `export` over the synthetic sample sites, not stored worlds (`crates/arda-town/src/main.rs:1`, `crates/arda-town/src/main.rs:21`) |
| `arda-refine` | `crates/arda-refine/src/bin/arda-refine.rs:75` | `render`, `stats`, `survey` over a stored world (`crates/arda-refine/src/bin/arda-refine.rs:19`, `crates/arda-refine/src/bin/arda-refine.rs:88`) |
| `arda-midzoom` | `crates/arda-midzoom/src/bin/arda-midzoom.rs:41` | One relief crop or a timed `k × k` tile block from a stored world (`crates/arda-midzoom/src/bin/arda-midzoom.rs:74`) |
| `tools/art-gen/generate.mjs` | `main` `tools/art-gen/generate.mjs:607`, invoked at `tools/art-gen/generate.mjs:847` | Selects prompt-pack assets, submits Slopify batches (only with `--yes`), downloads images, writes `import.toml` (`tools/art-gen/generate.mjs:8`) |
| `tools/art-gen/build-prompts.mjs` | `tools/art-gen/build-prompts.mjs:1` | Rebuilds `prompts.json` from `checklist.csv` and `expansion.mjs`; `--check` mode (`tools/art-gen/build-prompts.mjs:4`) |
| Viewer | `apps/viewer/index.html:10` → `apps/viewer/src/main.tsx:9` | Vite dev server port 5173 (`apps/viewer/vite.config.ts:16`); scripts `dev`, `build`, `preview`, `test` in `apps/viewer/package.json` |
| Docker image | `Dockerfile:4`, `Dockerfile:11` | Builds `arda-cli` only; entrypoint `arda` |

The public library entry points of `arda` (generation, `World::load` `crates/arda/src/world.rs:34`, exports) are the in-process API every Rust consumer above uses.

## Communication

### HTTP (arda-server)

Transport is axum 0.8 over hyper HTTP/1 only (`crates/arda-server/Cargo.toml:31`, `crates/arda-server/Cargo.toml:43`). No websocket, SSE, message queue or event bus exists. Every route is registered in `router` (`crates/arda-server/src/routes.rs:28`) under the `/v1` prefix (`crates/arda-server/src/routes.rs:61`); `/v1/tactical` is a nested router (`crates/arda-server/src/tactical/routes.rs:37`). Unmatched paths go to `not_found` (`crates/arda-server/src/routes.rs:62`, `crates/arda-server/src/routes.rs:177`). CORS admits only `localhost`, `127.0.0.1` and `[::1]` origins, methods GET/HEAD/POST/OPTIONS, and exposes `ETag`, `X-Arda-Cache` and the sheet-mapping header (`crates/arda-server/src/routes.rs:92`). World work runs on Tokio's blocking pool through `blocking` (`crates/arda-server/src/routes.rs:106`). Connection limits and header read timeout are applied in `serve` (`crates/arda-server/src/serve.rs:41`).

Payload field tables live in `09-interfaces.md`; each row cites its section there.

| Method | Path | Handler | Payload (request → response) |
|---|---|---|---|
| GET | `/v1/health` | `crates/arda-server/src/routes.rs:181` | none → `Health`; `09-interfaces.md § GET /v1/health` |
| GET | `/v1/world` | `crates/arda-server/src/routes.rs:189` | none → `WorldInfo`; `09-interfaces.md § GET /v1/world` |
| GET | `/v1/cell/{gx}/{gy}` | `crates/arda-server/src/routes.rs:197` | path `gx`,`gy: u32` → `CellSample`; `09-interfaces.md § GET /v1/cell/{gx}/{gy}` |
| GET | `/v1/area/{ax}/{ay}/cells` | `crates/arda-server/src/routes.rs:209` | path + `format` query → JSON `AreaCells` or `ARDACOLS` binary; `09-interfaces.md § GET /v1/area/{ax}/{ay}/cells` |
| GET | `/v1/area/{ax}/{ay}/rivers` | `crates/arda-server/src/routes.rs:257` | path → `AreaRivers`; `09-interfaces.md § GET /v1/area/{ax}/{ay}/rivers` |
| GET | `/v1/area/{ax}/{ay}/lakes` | `crates/arda-server/src/routes.rs:273` | path → `AreaLakes`; `09-interfaces.md § GET /v1/area/{ax}/{ay}/lakes` |
| GET | `/v1/point` | `crates/arda-server/src/routes.rs:289` | query `x_m`,`y_m: f64` → `PointSample`; `09-interfaces.md § GET /v1/point` |
| GET | `/v1/overview.png` | `crates/arda-server/src/routes.rs:302` | query → PNG; `09-interfaces.md § GET /v1/overview.png` |
| GET | `/v1/tiles/overview/{z}/{x}/{y}` | `crates/arda-server/src/routes.rs:320` | path (`{y}` carries the image extension) → PNG/WebP tile; `09-interfaces.md § GET /v1/tiles/overview/{z}/{x}/{y}.webp`, `§ GET /v1/tiles/overview/{z}/{x}/{y}.png` |
| GET | `/v1/tiles/relief/{z}/{x}/{y}` | `crates/arda-server/src/routes.rs:349` | path → WebP tile; `09-interfaces.md § GET /v1/tiles/relief/{z}/{x}/{y}.webp` |
| POST | `/v1/npc/population` | `crates/arda-server/src/routes.rs:367` | JSON `PopulationRequest` (body limit `npc::MAX_BODY_BYTES`, `crates/arda-server/src/routes.rs:42`) → population JSON; `09-interfaces.md § POST /v1/npc/population` |
| GET | `/v1/npc/demo` | `crates/arda-server/src/routes.rs:388` | none → demo population; `09-interfaces.md § GET /v1/npc/demo` |
| GET | `/v1/npc/demo/{npc_id}` | `crates/arda-server/src/routes.rs:395` | path → one NPC; `09-interfaces.md § GET /v1/npc/demo/{npc_id}` |
| GET | `/v1/npc/{npc_id}` | `crates/arda-server/src/npcs/mod.rs:217` | path → stored NPC; `09-interfaces.md § GET /v1/npc/{npc_id}` |
| GET | `/v1/npcs` | `crates/arda-server/src/npcs/mod.rs:127` | query → `NpcPage`; `09-interfaces.md § GET /v1/npcs` |
| GET | `/v1/buildings/{building}/residents` | `crates/arda-server/src/npcs/mod.rs:155` | path → NPC list; `09-interfaces.md § GET /v1/buildings/{building}/residents` |
| GET | `/v1/buildings/{building}/workers` | `crates/arda-server/src/npcs/mod.rs:164` | path → NPC list; `09-interfaces.md § GET /v1/buildings/{building}/workers` |
| GET | `/v1/settlements` | `crates/arda-server/src/people.rs:51` | query → settlement list; `09-interfaces.md § GET /v1/settlements` |
| GET | `/v1/settlements/{id}` | `crates/arda-server/src/people.rs:72` | path → settlement; `09-interfaces.md § GET /v1/settlements/{id}` |
| GET | `/v1/settlements/{id}/plan` | `crates/arda-server/src/people.rs:89` | path → town plan; `09-interfaces.md § GET /v1/settlements/{id}/plan` |
| GET | `/v1/settlements/{id}/npcs` | `crates/arda-server/src/people.rs:106` | path → NPC list; `09-interfaces.md § GET /v1/settlements/{id}/npcs` |
| GET | `/v1/schema` | `crates/arda-server/src/schema/routes.rs:34` | none → schema index; `09-interfaces.md § GET /v1/schema` |
| GET | `/v1/schema/{name}` | `crates/arda-server/src/schema/routes.rs:42` | path → JSON Schema; `09-interfaces.md § GET /v1/schema/{Name}.json` |
| GET | `/v1/tactical/layouts` | `crates/arda-server/src/tactical/routes.rs:181` | none → `TacticalLayouts`; `09-interfaces.md § GET /v1/tactical/layouts` |
| GET | `/v1/tactical/layout/{file}` | `crates/arda-server/src/tactical/routes.rs:188` | path (`name` or `name.png`) → layout JSON or PNG; `09-interfaces.md § GET /v1/tactical/layout/{name}`, `§ GET /v1/tactical/layout/{name}.png` |
| GET | `/v1/tactical/layout/{name}/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/routes.rs:238` | path → WebP tile; `09-interfaces.md § GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp` |
| POST | `/v1/tactical/render` | `crates/arda-server/src/tactical/routes.rs:276` | `TacticalLayout` JSON body (limit `max_body_bytes`, `crates/arda-server/src/tactical/routes.rs:44`) → PNG; `09-interfaces.md § POST /v1/tactical/render` |
| GET | `/v1/tactical/cell/{gx}/{gy}` | `crates/arda-server/src/tactical/world_routes.rs:147` | path (`gy` or `gy.png`) + query → `TacticalBlockDto` JSON or PNG; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}`, `§ GET /v1/tactical/cell/{gx}/{gy}.png` |
| GET | `/v1/tactical/cell/{gx}/{gy}/scene` | `crates/arda-server/src/tactical/scene.rs:92` | path + query → `TacticalScene`; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}/scene` |
| GET | `/v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}` | `crates/arda-server/src/tactical/world_routes.rs:188` | path + query → WebP tile; `09-interfaces.md § GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp` |
| GET | `/v1/tactical/window` | `crates/arda-server/src/tactical/world_routes.rs:275` | query → window JSON; `09-interfaces.md § GET /v1/tactical/window` |
| GET | `/v1/tactical/window.png` | `crates/arda-server/src/tactical/world_routes.rs:298` | query → PNG; `09-interfaces.md § GET /v1/tactical/window.png` |
| GET | `/v1/tactical/library` | `crates/arda-server/src/tactical/world_routes.rs:328` | none → `TacticalLibraryDto`; `09-interfaces.md § GET /v1/tactical/library` |
| POST | `/v1/tactical/prefetch` | `crates/arda-server/src/tactical/routes.rs:63` | JSON `PrefetchRequest` → 202 `PrefetchAccepted`; `09-interfaces.md § POST /v1/tactical/prefetch` |
| GET | `/v1/tactical/dungeon` | `crates/arda-server/src/tactical/dungeon.rs:78` | query → dungeon layout JSON; `09-interfaces.md § GET /v1/tactical/dungeon` |
| GET | `/v1/tactical/dungeon.png` | `crates/arda-server/src/tactical/dungeon.rs:110` | query → PNG; `09-interfaces.md § GET /v1/tactical/dungeon.png` |
| GET | `/v1/tactical/dungeon/scene` | `crates/arda-server/src/tactical/dungeon.rs:91` | query → scene JSON; `09-interfaces.md § GET /v1/tactical/dungeon/scene` |

Wire types are exported as TypeScript by ts-rs into `bindings/ts/arda/` (`09-interfaces.md § bindings/ts/arda`); `cargo test -p arda-server bindings` fails when they are stale (`crates/arda-server/src/bindings.rs:1`).

### HTTP clients

| Client → server | Send site | Calls | Payload |
|---|---|---|---|
| Viewer → arda-server | `ArdaClient` `apps/viewer/src/api/client.ts:148`; JSON reads via `contractJson` `apps/viewer/src/api/client.ts:218` | health, world, cell, point, area cells (JSON and `?format=bin`), rivers, lakes, overview/relief tile templates, tactical layouts/layout/tiles/library/cell/scene/render/prefetch (`apps/viewer/src/api/client.ts:223`–`apps/viewer/src/api/client.ts:381`) | Same types as the server rows above, imported from `@arda` (`apps/viewer/src/api/client.ts:20`); binary area layout parsed by `parseAreaColumnsBin` (`apps/viewer/src/api/areaBin.ts:27`). The viewer calls none of the NPC, settlement, schema, window or dungeon routes |
| art-gen → Slopify (external, `SLOPIFY_URL`, default `http://127.0.0.1:6969`) | `client` `tools/art-gen/generate.mjs:433`, `fetch` at `tools/art-gen/generate.mjs:438` | `/api/health`, `/api/channels`, `/api/prompts`, `/api/providers`, `/api/staging/images`, `/api/projects`, `/api/projects/batch`, `/api/projects/{id}`, `/api/projects/{id}/cancel` (`tools/art-gen/generate.mjs:475`–`tools/art-gen/generate.mjs:834`) | Batch body built by `batchBody` (`tools/art-gen/generate.mjs:284`), single-project body by `projectBody` (`tools/art-gen/generate.mjs:333`); GETs retry with delays `[2,5,10,20,30,60]` s, writes do not (`tools/art-gen/generate.mjs:431`) |

### In-process and file handoffs

Everything else is synchronous in-process calls plus files.

| Send site → receive site | Request | Return |
|---|---|---|
| `run_generate` `crates/arda-cli/src/generate.rs:85` → `arda::generate` `crates/arda/src/lib.rs:44` → `arda_gen::generate_world` | `seed: u64`, `GenerateConfig`, `out: &Path` | `Manifest` or `GenError`; world directory on disk |
| `run_generate` `crates/arda-cli/src/generate.rs:95` → `arda::generate_from_fine_recipe` `crates/arda/src/lib.rs:85` → `arda_gen::orchestrator::generate_world_with_fine_recipe` | `seed`, `GenerateConfig`, `out`, `FineDeliveryLimits`, `recipe_version: u16` (4–7) | `Manifest` or `GenError::Validation` for an unsupported recipe (`crates/arda/src/lib.rs:92`) |
| `arda_settle::generate` `crates/arda-settle/src/lib.rs:160` → `arda::World::load`, `pipeline::run` | `world_dir: &Path`, `budget: u64` | `(Society, PathBuf)`; writes `settlements.json`, `roads.json`, `realms.json`, `names.json`, `landuse.bin`, `realms.bin`, `roads.bin`, `stats.json` under `<world>/society/` (`crates/arda-settle/src/lib.rs:72`–`crates/arda-settle/src/lib.rs:132`) |
| `arda_people::build` `crates/arda-people/src/build.rs:45` → `compute` `crates/arda-people/src/build.rs:58` → `arda_society::simulate_society` `crates/arda-people/src/build.rs:78` → `arda_npc::Generator::with_notables` `crates/arda-people/src/build.rs:94` | `world_dir`; reads `<world>/society/` files written by arda-settle | `BuildReport`; writes `society.json` and `notables.json` (`crates/arda-people/src/build.rs:48`) |
| `Pipeline::window` `crates/arda-blocks/src/pipeline.rs:165` → `arda_refine::refine` `crates/arda-refine/src/block.rs:167` (per cell, cached, `crates/arda-blocks/src/pipeline.rs:112`) | `(gsx0, gsy0, w, h)` window in squares, `&Library`, optional `&dyn Overlays` | `Composed { layout: TacticalLayout, rules: RulesSidecar, meta: MapMeta, owners }` (`crates/arda-blocks/src/pipeline.rs:23`) or `BlocksError` |
| `arda-blocks` society overlays → `arda_ways::apply_ways` `crates/arda-blocks/src/society/layers.rs:166`, `arda_fields::generate` `crates/arda-blocks/src/society/layers.rs:350`, `arda_town::block::generate` `crates/arda-blocks/src/society/town.rs:145` | layout window, world roads/rivers/land use and town plans from `arda_people::World` | Overlay squares composed by `compose_all` (`crates/arda-blocks/src/overlays.rs:296`) |
| Server relief tiles → `arda_midzoom::render_tile` `crates/arda-server/src/relief.rs:124` | `ReliefWorld`, `Pyramid`, `z, x, y` | RGBA tile, encoded by the server |
| Prefetch handler `crates/arda-server/src/tactical/routes.rs:75` → `Prefetcher::submit` `crates/arda-server/src/tactical/prefetch.rs:198` → worker `work` `crates/arda-server/src/tactical/prefetch.rs:314` | internal `Job { req: BlockRequest, ppsq: u32, image: bool, world: (i64, i64) }` on a mutex+condvar `VecDeque` (`crates/arda-server/src/tactical/prefetch.rs:141`, `crates/arda-server/src/tactical/prefetch.rs:149`); `BlockRequest { gsx0, gsy0: i64, w, h: u32, demo_at: Option<[i64; 2]> }` (`crates/arda-server/src/tactical/block.rs:34`) | No return to the client beyond 202; workers fill the same caches requests read (`crates/arda-server/src/tactical/prefetch.rs:14`) |
| `generate.mjs` → `arda tactical import` | files on disk: downloaded raw images plus `import.toml` (`tools/art-gen/generate.mjs:550`, `tools/art-gen/generate.mjs:580`) | `arda_art_import::import` reads `raw_dir` and writes a library plus `report.md` (`crates/arda-cli/src/tactical_import.rs:51`, `crates/arda-cli/src/tactical_import.rs:74`) |

## Composition

No DI container exists in any crate; wiring is plain values, `Arc`s and trait objects built in `main` or `open` functions.

- `arda` CLI: `main` parses clap once and matches `Command` to private handlers (`crates/arda-cli/src/main.rs:360`); handlers call library functions directly (`crates/arda-cli/src/society.rs:29`, `crates/arda-cli/src/tactical.rs:94`, `crates/arda-cli/src/dungeon.rs:88`).
- arda-server: `main` builds `ServerConfig` from flags (`crates/arda-server/src/main.rs:58`), then `AppState::open` (`crates/arda-server/src/lib.rs:171`) wires, in order: admission (`config.admit`), optional sheet mapping, `WorldQuery::open`, `Tactical::open` with the library stack, `arda_people::shared::SharedSource::open`, an optional `arda_people::World` when `society/settlements.json` exists (with `warm_towns`), `arda_blocks::society::SocietyOverlays`, an `arda_midzoom::Landscape`, the `RefineBlocks` block source installed via `set_block_source` (`crates/arda-server/src/tactical/mod.rs:224`), `Overview`, `Relief`, the `area_builds` semaphore and the `Prefetcher` (`crates/arda-server/src/lib.rs:172`–`crates/arda-server/src/lib.rs:233`). `router(Arc<AppState>)` attaches the state (`crates/arda-server/src/routes.rs:64`). `BlockSource` is the replaceable seam for tactical blocks (`crates/arda-server/src/tactical/block.rs:115`, `crates/arda-server/src/lib.rs:258`).
- `RefineBlocks::with_society` composes an `arda_blocks::Pipeline` over a `SharedSource` with optional society overlays (`crates/arda-server/src/tactical/refine_blocks.rs:74`).
- arda-midzoom binary: `World::load` → `ReliefWorld::new` → optional `Landscape::open` → `render_window` (`crates/arda-midzoom/src/bin/arda-midzoom.rs:43`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:50`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:53`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:74`).
- arda-settle: `generate` loads the world, stitches a grid (`load::load_grid`), runs `pipeline::run`, then writes all files only after every encoding succeeds (`crates/arda-settle/src/lib.rs:161`–`crates/arda-settle/src/lib.rs:169`, `crates/arda-settle/src/lib.rs:145`). `pipeline::run` sequences tags, suitability, placement, seats, cultures, land use, cost surface, roads, crossings, realms and naming (`crates/arda-settle/src/pipeline.rs:53`).
- arda-people: `plan_all` plans settlements on scoped threads in chunks, preserving settlement order (`crates/arda-people/src/build.rs:122`); society simulation and NPC generation follow sequentially (`crates/arda-people/src/build.rs:78`).
- arda-blocks: `Pipeline::refine_all` refines missing cells on scoped threads, then `assemble` joins blocks and overlays are composed (`crates/arda-blocks/src/pipeline.rs:127`, `crates/arda-blocks/src/pipeline.rs:196`).
- Viewer: `createRoot(...).render(<App/>)` (`apps/viewer/src/main.tsx:9`); `App` holds one `ArdaClient` memoised on the API base (`apps/viewer/src/App.tsx:26`).

## Frontend

`apps/viewer` is a developer viewer for the arda-server v1 API (`apps/viewer/package.json:5`), not a player-facing product.

| Aspect | Fact |
|---|---|
| Rendering model | Client-only SPA: one `index.html` with `#root` and a module script (`apps/viewer/index.html:9`, `apps/viewer/index.html:10`); React 19 `createRoot` under `StrictMode` (`apps/viewer/src/main.tsx:9`). No SSR/SSG |
| Build | Vite with `@vitejs/plugin-react`; `build` = `tsc -b && vite build`; Vitest in node environment for `src/**/*.test.ts` (`apps/viewer/package.json:10`, `apps/viewer/vite.config.ts:11`, `apps/viewer/vite.config.ts:19`) |
| Routing | Hash routing without a router library: `VIEWS = world, area, tactical, cell, editor` read from `location.hash` and a `hashchange` listener (`apps/viewer/src/App.tsx:12`, `apps/viewer/src/App.tsx:17`, `apps/viewer/src/App.tsx:33`). `tactical`, `cell`, `editor` render without `/v1/world` (`apps/viewer/src/App.tsx:15`) |
| Views | `WorldView` (Leaflet overview/relief pyramid, `apps/viewer/src/views/WorldView.tsx:15`), `AreaView` (canvas heatmap, `apps/viewer/src/views/AreaView.tsx:28`), `TacticalView` (layout pyramid, `apps/viewer/src/views/TacticalView.tsx:28`), `TacticalCellView` (world tactical cells with prefetch, `apps/viewer/src/views/TacticalCellView.tsx:21`), `LayoutEditor` (edits ground and POSTs to `/v1/tactical/render`, `apps/viewer/src/views/LayoutEditor.tsx:24`), `CellInspector` (`apps/viewer/src/views/CellInspector.tsx:4`) |
| Map library | Leaflet 1.9 with a flat CRS aligned to the server pyramid (`apps/viewer/package.json:17`, `apps/viewer/src/geo/leafletCrs.ts:3`); `TacticalMap` loads tiles with `fetch` to read `ETag` and `X-Arda-Cache` (`apps/viewer/src/components/TacticalMap.tsx:28`) |
| Design system | None: one hand-written `src/styles.css` plus Leaflet's CSS (`apps/viewer/src/main.tsx:3`, `apps/viewer/src/main.tsx:4`); no component library dependency (`apps/viewer/package.json:16`) |
| API-client seam | `ArdaClient` (`apps/viewer/src/api/client.ts:148`) with base resolved from `?api=`, then `localStorage` key `arda-viewer.apiBase`, then `VITE_ARDA_API`, then `http://localhost:8787` (`apps/viewer/src/config.ts:9`, `apps/viewer/src/api/client.ts:52`); response types come from ts-rs bindings via the `@arda` alias (`apps/viewer/vite.config.ts:13`), with runtime shape checks in `apps/viewer/src/api/tactical.ts` and `apps/viewer/src/api/scene.ts` |

No other human-facing UI exists: the Rust binaries are CLIs and arda-server serves JSON and images only.
