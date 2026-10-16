---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: e2ed331e1a2b
paths_covered:
  - ":(top)crates/arda-cli/src/**"
  - ":(top)crates/arda/src/**"
  - ":(top)crates/arda-gen/src/orchestrator.rs"
  - ":(top)crates/arda-gen/src/orchestrator/**"
  - ":(top)crates/arda-settle/src/**"
  - ":(top)crates/arda-society/src/**"
  - ":(top)crates/arda-people/src/**"
  - ":(top)crates/arda-npc/src/**"
  - ":(top)crates/arda-town/src/**"
  - ":(top)crates/arda-blocks/src/**"
  - ":(top)crates/arda-refine/src/**"
  - ":(top)crates/arda-tactical/src/**"
  - ":(top)crates/arda-scene/src/**"
  - ":(top)crates/arda-dungeon/src/**"
  - ":(top)crates/arda-midzoom/src/**"
  - ":(top)crates/arda-art-import/src/**"
  - ":(top)crates/arda-server/src/**"
  - ":(top)apps/viewer/src/**"
  - ":(top)tools/art-gen/safety.mjs"
---

# Data flow

The pipeline has two halves. Three offline CLI batches write files under one world directory: `arda generate`, `arda settle` and `arda society build` (`crates/arda-cli/src/main.rs:399`, `crates/arda-cli/src/society.rs:1`). `arda-server` reads those files and computes everything else per request, behind bounded in-memory caches (`crates/arda-server/src/lib.rs:204`). Physical generation internals (tectonics, formation recipes, the hydrology solve) are covered in `logic/01`–`02` and `01-architecture.md`. This chapter traces them only as far as their published files.

## Lifecycles

### Persisted vs computed

| Product | Persisted on disk (writer) | Computed on demand (reader) |
|---|---|---|
| Manifest | `world.json`, renamed into place last (`crates/arda-gen/src/orchestrator/publication.rs:122`, `crates/arda-core/src/formats/manifest.rs:88`) | — |
| Area cells/objects | `areas/<ax>_<ay>/cells.bin`, `objects.bin` (`crates/arda-gen/src/orchestrator/final_writes.rs:135`) | Decoded per request into `WorldQuery.areas` (`crates/arda-server/src/query.rs:158`) |
| Area water forms | `areas/<ax>_<ay>/water.bin`, formed recipes only (`crates/arda-gen/src/orchestrator.rs:361`) | — |
| Hydrology tables | `hydrology/*.bin` (`crates/arda-gen/src/orchestrator/global_output.rs:52`) | — |
| Continent | `continent/overview.bin`, `continent/objects.bin` (`crates/arda-gen/src/orchestrator.rs:417`, `crates/arda-gen/src/orchestrator.rs:423`) | — |
| Fine terrain | `terrain/fine.bin`, copied from the staged source (`crates/arda-core/src/formats/manifest.rs:43`, `crates/arda-gen/src/orchestrator.rs:191`) | Per-area windows in `WorldQuery.windows` (`crates/arda-server/src/query.rs:206`) |
| Settlements, roads, realms, names, gazetteer, land use | `society/settlements.json`, `roads.json`, `realms.json`, `names.json`, `gazetteer.json`, `landuse.bin`, `realms.bin`, `roads.bin`, `stats.json` (`crates/arda-settle/src/lib.rs:62`) | Rasters decoded once at server start (`crates/arda-server/src/society_cells.rs:47`); `gazetteer.json` and `realms.json` read once into `SocietyFiles` (`crates/arda-people/src/files.rs:106`, `crates/arda-people/src/files.rs:112`) |
| Society simulation | `society/society.json` (`crates/arda-people/src/build.rs:48`) | Loaded once at server start (`crates/arda-people/src/people.rs:79`) |
| Notables | `society/notables.json`, notables only (`crates/arda-people/src/build.rs:49`) | Indexed by `NpcId` at start (`crates/arda-people/src/people.rs:98`) |
| Commoners, households, roster | Not stored | Regenerated per request by `Generator::with_notables` (`crates/arda-people/src/people.rs:253`, `crates/arda-server/src/npcs/mod.rs:206`) |
| Town plans | Not stored (computed during `society build`, then discarded) | `World::plan`, LRU of 64 (`crates/arda-people/src/people.rs:18`, `crates/arda-people/src/people.rs:137`) |
| Refined tactical blocks | Not stored | `Pipeline.cache`, 256 cells (`crates/arda-blocks/src/pipeline.rs:112`, `crates/arda-server/src/tactical/refine_blocks.rs:24`) |
| Composed blocks, renders, PNG/WebP/JSON bodies | Not stored | `ByteLru` caches (`crates/arda-server/src/tactical/mod.rs:164`) |
| Dungeons | Only by `arda dungeon --out` (`crates/arda-cli/src/dungeon.rs:94`) | Regenerated per request. Only the PNG is cached (`crates/arda-server/src/tactical/dungeon.rs:72`). |
| Overview PNG and tiles (with or without society overlay and labels) | Not stored. A render goes through a temp scratch directory that is removed on drop (`crates/arda-server/src/overview.rs:313`, `crates/arda-server/src/overview.rs:340`). | `Overview.pngs`, `tiles` and `base` (`crates/arda-server/src/overview.rs:118`) |
| Mid-zoom relief tiles | Not stored | `Relief.tiles` (`crates/arda-server/src/relief.rs:138`) |
| Tactical asset library | `catalog.json` plus images, from `arda tactical import` (`crates/arda-art-import/src/lib.rs:97`) or the placeholder generator | Loaded and pre-scaled at start (`crates/arda-server/src/tactical/mod.rs:195`) |

### 1. `arda generate` (continent → areas → terrain)

1. CLI `run_generate` picks the source. `--terrain` defaults to `Fine` since recipe 9 (`crates/arda-cli/src/main.rs:53`). Fine calls `generate_from_fine_recipe`, default recipe 9, and legacy calls `arda::generate` (`crates/arda-cli/src/generate.rs:85`, `crates/arda-cli/src/generate.rs:95`, `crates/arda-core/src/formats/manifest.rs:58`). Recipe 9 also selects its climate (`continent::climate9`) and the shared solve's rules (`hydrology::physics::SolvePhysics::for_recipe`, Budyko runoff, HAND to standing water, hashed flat ties).
2. The fine path admits resources and refuses an occupied output (`crates/arda-gen/src/orchestrator/fine_delivery.rs:280`). It then loops over up to `CONTINENT_ATTEMPTS` candidates (`crates/arda-gen/src/orchestrator/fine_delivery.rs:286`). Each candidate is either formed (`fine_formation::generate`, `:294`) or spectral then valley-carved (`fine_source::generate`, `:307`; `fine_valleys::carve`, `:330`). A candidate must pass the land gate and the river gate (`:369`). The accepted file goes to `generate_world_from_formed_terrain` or `generate_world_from_fine_terrain` (`:382`, `:391`). The private source stage is removed either way (`:400`, `:408`).
3. `generate_world_inner` (`crates/arda-gen/src/orchestrator.rs:166`) does the following, in order:
   - admits resources (`:178`)
   - opens the transaction (`WorldOutput::begin`, `:189`) and copies the fine field in (`:191`)
   - runs the continent attempt loop with climate/hydrology/river gate (`:211`)
   - builds `SharedTerrain` (`:274`, `:283`) and writes prepared tiles per area to scratch (`:287`)
   - runs the shared water solve (`:295`) and builds the final index (`:314`)
4. Per area, the orchestrator composes cells and objects (`area_output::compose`, `:330`), applies ground ecology (`fine_materials::apply`, `:344`), then calls `write_area` (`:346`). `write_area` writes `cells.bin` and `objects.bin`. No block archive is written (retired in 1.0, OI-43); tactical blocks are built on demand by the server.
5. Next come water forms (`crates/arda-gen/src/orchestrator.rs:361`), global hydrology tables (`:373`), and the continent overview and objects (`:417`, `:423`).
6. `output.commit` renames the manifest last (`crates/arda-gen/src/orchestrator.rs:444`, `crates/arda-gen/src/orchestrator/publication.rs:122`). The manifest's `settlement_count` is written as 0 (`crates/arda-gen/src/orchestrator.rs:439`), and no later stage rewrites `world.json`.

### 2. `arda settle` (settlements, roads, realms, land use)

1. `society::settle` calls `arda_settle::generate` with a 16 GiB budget (`crates/arda-cli/src/society.rs:28`, `crates/arda-settle/src/grid.rs:15`).
2. `generate` loads the world. `load_grid` reads every area uncached (`World::read_area`) into one global `Grid` (`crates/arda-settle/src/lib.rs:171`, `crates/arda-settle/src/load.rs:20`).
3. `pipeline::run` (`crates/arda-settle/src/pipeline.rs:56`) runs these stages, in order:
   - site tags, suitability and placement (`:58`)
   - realm seats, allegiance and primacy (`:68`)
   - cultures and settlement profiles (`:75`)
   - land use (`landuse::allocate`, `:81`)
   - cost surface and roads (`:82`)
   - crossings and passes (`:84`)
   - realm partition (`:86`)
   - naming (`naming::name_world`, `:96`), which also builds the gazetteer of named seas, bays, lakes, islands, capes, forests, marshes and ranges (`crates/arda-settle/src/pipeline.rs:46`, `crates/arda-settle/src/naming.rs:289`, `crates/arda-settle/src/gazetteer/mod.rs:1`)
   - stats (`:106`)
4. `write` encodes every file before writing the first, then writes them one by one with `std::fs::write` into `<world>/society/` (`crates/arda-settle/src/lib.rs:156`).

### 3. `arda society build` (town plans, society, notables)

1. `society::run` calls `arda_people::build` (`crates/arda-cli/src/society.rs:41`, `crates/arda-people/src/build.rs:45`).
2. `compute` opens a `SharedSource` and reads `settlements.json` and `roads.json` (`crates/arda-people/src/build.rs:58`, `crates/arda-people/src/files.rs:96`).
3. `plan_all` draws every settlement's plan on scoped threads, in chunks of 4 × the core count, keeping results in order (`crates/arda-people/src/build.rs:122`).
   - Each plan samples terrain (`Surroundings::around`) and refined rivers from the world (`crates/arda-people/src/town.rs:99`), then calls `arda_town::generate` (`crates/arda-people/src/town.rs:136`, `crates/arda-town/src/plan/mod.rs:60`).
   - A refused plan is `Ok(None)`.
4. The plan buildings become `WorldSettlements.buildings` (`crates/arda-people/src/build.rs:76`). Then `arda_society::simulate_society` runs (`crates/arda-people/src/build.rs:78`, `crates/arda-society/src/lib.rs:136`). Its stages, in order:
   - economy (`crates/arda-society/src/economy/mod.rs:118`)
   - government (`crates/arda-society/src/politics/government.rs:21`)
   - history (`crates/arda-society/src/history/run.rs:47`); the disaster stage then chains famines and plagues to their causes (`crates/arda-society/src/history/disasters.rs:87`), and major events carry a unique `name`, a `backstory` and a `cause_kind` from `CAUSE_KINDS` (`crates/arda-society/src/history/mod.rs:89`, `crates/arda-society/src/history/mod.rs:99`, `crates/arda-society/src/history/stories.rs:1`)
   - realm states (`crates/arda-society/src/politics/realm.rs:110`)
   - relations (`crates/arda-society/src/politics/relations.rs:94`)
   - per-settlement factions (`crates/arda-society/src/politics/factions.rs:99`) and offices/roles (`crates/arda-society/src/roles.rs:115`)
   - settlement hooks and region (realm) hooks (`crates/arda-society/src/lib.rs:203`, `crates/arda-society/src/lib.rs:208`)
5. `history.json`, `history_text.json`, `history_backstory.json`, `factions.json`, `roles.json` and the other tables are compile-time embedded inputs under `crates/arda-society/data/`, not outputs (`crates/arda-society/src/tables.rs:318`, `crates/arda-society/src/tables.rs:321`, `crates/arda-society/src/tables.rs:327`); `families::lint` rejects a history table whose families or slots do not match the code (`crates/arda-society/src/tables.rs:387`, `crates/arda-society/src/families.rs:1`). The simulated history is serialised inside `society.json` (`crates/arda-society/src/lib.rs:127`).
6. Per settlement, buildings come from the plan (`BuildingSource::Plan`). Without a plan they come from the settlement's mix (`BuildingSource::Mix`) (`crates/arda-people/src/build.rs:88`). Society roles become `NotableSlot`s, and `Generator::with_notables(...).population()` builds the population (`crates/arda-people/src/build.rs:94`, `crates/arda-npc/src/lib.rs:76`, `crates/arda-npc/src/lib.rs:144`). Only `pop.npcs` (the notables) is kept.
7. `society.json` and then `notables.json` are written (`crates/arda-people/src/build.rs:48`).

### 4. Server start

1. `main` runs `AppState::open` on a blocking thread, then binds and serves (`crates/arda-server/src/main.rs:86`, `crates/arda-server/src/main.rs:101`); `serve` stops on Ctrl-C or SIGTERM (`crates/arda-server/src/main.rs:108`). Startup steps, in order:
   - memory admission against the 16 GiB ceiling (`crates/arda-server/src/lib.rs:154`)
   - sheet mapping (`:207`)
   - `WorldQuery::open` (`:212`), which loads the world, the fine reader and the society rasters (`crates/arda-server/src/query.rs:78`)
   - `Tactical::open`, which loads and validates the library stack and pre-scales textures for every ppsq (`crates/arda-server/src/tactical/mod.rs:195`)
2. When `society/settlements.json` exists, `arda_people::World::with_source` loads `society.json` and `notables.json` if present; `SocietyFiles` also reads `realms.json` and `gazetteer.json` when they exist (`crates/arda-server/src/lib.rs:220`, `crates/arda-people/src/people.rs:79`). `warm_towns` then pre-draws plans for the most populous towns and cities, in batches that read at most 8 areas between them, until the plan cache holds half its 64 plans or 768 MiB (`crates/arda-server/src/lib.rs:231`, `crates/arda-people/src/people.rs:169`).
3. `SocietyOverlays::open` converts roads and crossings and reads `landuse.bin` (`crates/arda-server/src/lib.rs:233`, `crates/arda-blocks/src/society/mod.rs:115`). The same overlays feed the relief `Landscape` (`crates/arda-server/src/lib.rs:241`).
4. `RefineBlocks::with_society` becomes the block source (`crates/arda-server/src/lib.rs:244`). `Overview::new` sets up the overview (`:247`). `Relief::open` follows (`:248`), and its `ReliefWorld` doubles as the tactical world-tint source (`:255`). When the world has `society/gazetteer.json`, `Relief::enable_labels` builds the `Labeler` and the overview shares it (`:257`–`:262`).
5. Prefetch workers are not started at boot. The first `POST /prefetch` starts them (`crates/arda-server/src/tactical/prefetch.rs:230`). The bake worker (`--bake-workers`, 0 or 1) is likewise started by the first accepted `POST /v1/tactical/bake` (`crates/arda-server/src/tactical/bake.rs:215`, `crates/arda-server/src/tactical/bake.rs:233`); `ServerConfig::admit` refuses a bake result budget too small for one worst-case set (`crates/arda-server/src/lib.rs:154`).
6. Three independent `arda::World` instances are loaded (`crates/arda-server/src/query.rs:79`, `crates/arda-people/src/shared.rs:26`, `crates/arda-server/src/relief.rs:75`). Each keeps its own caches.

### 5. Tactical cell request (`GET /v1/tactical/cell/{gx}/{gy}[.png]`)

1. The handler parses `gx`/`gy`, ppsq and flags and builds a `BlockRequest`, then hands off to `blocking` (`spawn_blocking`) (`crates/arda-server/src/tactical/world_routes.rs:149`, `crates/arda-server/src/routes.rs:124`).
2. **PNG path** (`world_png_look`, `crates/arda-server/src/tactical/cells.rs:116`):
   - Grows the request by `APRON` = 6 squares (`crates/arda-server/src/tactical/cells.rs:23`, `crates/arda-server/src/tactical/cells.rs:83`).
   - Gets the block from `RefineBlocks::block`, checking the composed cache first (`crates/arda-server/src/tactical/refine_blocks.rs:166`).
   - Builds a cropped `RenderKey`: seed, origin anchor, BLAKE3 of the layout JSON, ppsq, grid, library version, crop and world grade (`crates/arda-server/src/tactical/mod.rs:344`).
   - Calls `png` (`crates/arda-server/src/tactical/mod.rs:370`).
3. **Cold block** (`compose`, `crates/arda-server/src/tactical/refine_blocks.rs:93`):
   - A whole-cell request on open sea returns `NoBlock`.
   - `Pipeline::window` refines every covered cell on scoped threads, each through the per-cell cache, calling `arda_refine::refine` (`crates/arda-blocks/src/pipeline.rs:165`, `crates/arda-blocks/src/pipeline.rs:112`, `crates/arda-refine/src/block.rs:167`). `Ctx::gather` reads the saved shore class and sea distance of each gathered cell (`Source::shore`, `crates/arda-refine/src/source.rs:134`; `coast_facts`, `crates/arda-refine/src/context.rs:175`), which the coast and marsh signals use (`crates/arda-refine/src/biome.rs:106`).
   - `assemble` joins the cells into one layout (`crates/arda-blocks/src/pipeline.rs:196`).
   - `compose_all` runs the ways, fields and town providers concurrently and composes them in that fixed order (`crates/arda-blocks/src/overlays.rs:296`).
4. **Overlay providers:**
   - Ways: `arda_ways::apply_ways` over roads filtered to a 600 m margin (`crates/arda-blocks/src/society/mod.rs:165`, `crates/arda-blocks/src/society/layers.rs:156`).
   - Fields: `arda_fields::generate` over `landuse.bin` (`crates/arda-blocks/src/society/layers.rs:282`, `crates/arda-fields/src/window.rs:100`).
   - Town: `plans_near` calls `World::plan` for each settlement in reach (cached, drawn in parallel on a miss). Then `arda_town::block::generate` and `fallback::resolve` cut that plan's squares for the window (`crates/arda-blocks/src/society/town.rs:30`, `crates/arda-blocks/src/society/town.rs:127`).
5. **Render.** `png` checks the `pngs` cache, then calls `raw` (`crates/arda-server/src/tactical/raw.rs:83`). `raw` works through these steps:
   - checks the `renders` cache
   - waits on any in-flight render of the same key, then marks the key in-flight
   - takes the request-lane mutex
   - admits the layout (squares, pixels, work) (`crates/arda-server/src/tactical/mod.rs:303`)
   - calls the compositor, which attaches a wall-occlusion mask to each light pool so light stops at closed walls (`crates/arda-tactical/src/compose/mod.rs:420`, `crates/arda-tactical/src/compose/light_los.rs:292`): `arda_tactical::render_region`, or `render_region_tinted` with `world_grade=1` (`crates/arda-tactical/src/compose/mod.rs:271`, `crates/arda-tactical/src/compose/mod.rs:289`)
   - the PNG encode then runs under the single shared encode slot (`crates/arda-server/src/tactical/mod.rs:379`)
6. **JSON path** (`world_body`, `crates/arda-server/src/tactical/cells.rs:161`): checks the `bodies` cache, then runs block → `arda_scene::build_scene` (`crates/arda-scene/src/lib.rs:51`) → `TacticalBlockDto` → cache.
7. **Response.** `image` sets a strong `ETag` (BLAKE3), `Cache-Control: no-cache`, `X-Arda-Cache: hit|miss` and `Server-Timing`. It answers 304 when `If-None-Match` matches (`crates/arda-server/src/tactical/routes.rs:147`).
8. **Tiles** (`/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp`, `crates/arda-server/src/tactical/world_routes.rs:190`): checks the `tiles` cache, then the `pyramids` cache (built from `raw`), then cuts and WebP-encodes the tile (`crates/arda-server/src/tactical/mod.rs:416`).
9. **Scene** (`/cell/{gx}/{gy}/scene`): block → `build_scene` → NPC tokens placed from `notables_of` for each settlement whose buildings appear in the rules (`crates/arda-server/src/tactical/scene.rs:59`). Scenes are not cached.
10. **Windows** (`/window`, `/window.png`, `/window/tiles/{z}/{x}/{y}.webp`) take up to 3 × 3 cells or 192 squares a side and go through the same path (`crates/arda-server/src/tactical/world_routes.rs:232`, `crates/arda-server/src/tactical/world_routes.rs:301`).

### 6. Prefetch (`POST /v1/tactical/prefetch`)

1. The handler calls `Prefetcher::submit` and answers 202 at once (`crates/arda-server/src/tactical/routes.rs:68`, `crates/arda-server/src/tactical/prefetch.rs:198`).
2. `submit` validates the radius (1..=2) and ppsq, and starts the workers if this is the first call. It walks `rings` nearest-first, clipped to the world (`crates/arda-server/src/tactical/prefetch.rs:105`).
3. Each cell is reported as one of three statuses:
   - `Pending`: already known
   - `Dropped`: the queue holds `QUEUE_CELLS` = 64 jobs, or there are no workers
   - `Queued`
4. `image` is true only for the first `image_slots` cells, the number whose render and pyramid fit the caches beside the open cell (`crates/arda-server/src/tactical/prefetch.rs:130`).
5. Workers (default 2, max 8) pop jobs and call `Tactical::warm`. `warm` fills the body cache, and when `image` is set also the render and pyramid in the prefetch lane (`crates/arda-server/src/tactical/prefetch.rs:314`, `crates/arda-server/src/tactical/prefetch.rs:363`). Keys equal the request keys, so a later request is a cache hit.
6. The viewer posts radius 1 whenever the cell changes (`apps/viewer/src/views/TacticalCellView.tsx:275`).

### 7. Overview and relief tiles

1. `/v1/overview.png` (`crates/arda-server/src/routes.rs:321`): `?overlay=society|none` resolves through `Overview::society`: absent, the layer follows the world (on when `society/` exists, never on the oblique look); `society` on a world without it is a 400 (`crates/arda-server/src/routes.rs:344`, `crates/arda-server/src/overview.rs:212`). `Overview::png` checks the `pngs` cache (key quality, style, society), then serialises renders behind `render` (`crates/arda-server/src/overview.rs:236`). `render` calls `arda_midzoom::export::export_overview` with `SocietyMode::On` or `Off` into a temp scratch directory, reads the file back and drops the directory (`crates/arda-server/src/overview.rs:340`).
2. `/v1/tiles/overview/{z}/{x}/{y}` (`crates/arda-server/src/routes.rs:369`):
   - Checks the `tiles` cache.
   - The tile key is `(z, x, y, format, oblique, labelled, society)`; `?labels=0` leaves names off, and the oblique pyramid never carries labels (`crates/arda-server/src/overview.rs:112`, `crates/arda-server/src/routes.rs:357`).
   - On a miss, `base` decodes the `tile_base_px` (default 4096) Atlas or AtlasOblique PNG once per `(oblique, society)` and keeps it (at most three live, since oblique never carries society) (`crates/arda-server/src/overview.rs:255`).
   - `tiles::cut` cuts the tile, `Labeler::draw_rgba` draws the gazetteer's names when the world has them, and the tile is PNG- or WebP-encoded (`crates/arda-server/src/overview.rs:297`, `crates/arda-server/src/overview.rs:301`).
3. `/v1/tiles/relief/{z}/{x}/{y}.webp` (`crates/arda-server/src/routes.rs:410`): `Relief::tile` checks the cache, then calls `arda_midzoom::render_tile` for levels past the overview's native zoom and encodes WebP (`crates/arda-server/src/relief.rs:138`, `crates/arda-midzoom/src/tile.rs:41`); with `labels` on (the default, `?labels=0` off) it draws the gazetteer's names via `arda_midzoom::labels::draw_tile` (`crates/arda-server/src/relief.rs:151`). Relief exists only for manifests with fine recipe ≥ 5 (`crates/arda-server/src/relief.rs:82`).

### 8. Contract, settlement and NPC requests

1. `/v1/cell/{gx}/{gy}` and `/v1/point` read from three caches (`crates/arda-server/src/query.rs:251`, `crates/arda-server/src/query.rs:309`):
   - the decoded area (`crates/arda-server/src/query.rs:158`)
   - the derived coast/river/lake layer, which loads the 3 × 3 neighbourhood on a miss (`:177`)
   - the fine window (`:206`)
   - The society raster values are added on top.
2. `/v1/area/{ax}/{ay}/cells` takes an `area_builds` semaphore permit (default 2) before building and encoding 262,144 samples. It is not cached (`crates/arda-server/src/routes.rs:228`, `crates/arda-server/src/query.rs:263`).
3. Settlement routes read `files`, `society` and `World::plan` (`crates/arda-server/src/people.rs:51`, `crates/arda-server/src/people.rs:89`).
   - `/settlements/{id}/npcs` returns stored notables only (`crates/arda-server/src/people.rs:106`).
   - `/v1/npc/{id}` resolves a stored notable by id first. Otherwise it rebuilds the settlement's `Generator` from profile, buildings (plan or mix) and slots, and expands one commoner (`crates/arda-server/src/npcs/mod.rs:177`, `crates/arda-server/src/npcs/page.rs:87`).
   - `/v1/npcs` and `/buildings/{building}/residents|workers` page through regenerated rosters (`crates/arda-server/src/npcs/page.rs:134`).
   - `/v1/gazetteer` filters the stored `society/gazetteer.json` by `kind`, `bbox` and `min_rank` and adds the settlements as points; a world settled before the gazetteer existed answers 404 (`crates/arda-server/src/gazetteer.rs:68`).
   - `/v1/realms` lists `realms.json` (sorted by id, capped at `MAX_LISTED`) with the politics headline from `society.json` when present; `/v1/realms/{id}` adds borders, relations, events and reigns (`crates/arda-server/src/realms.rs:67`, `crates/arda-server/src/realms.rs:92`).
   - `POST /v1/npc/generate` builds one NPC from optional job, office, rank and ancestry with `arda_npc::generate_one` over the world seed and the body; nothing is stored (`crates/arda-server/src/npc.rs:217`, `crates/arda-npc/src/lib.rs:43`).
   - NPC bodies pass through the optional sheet mapping (`crates/arda-server/src/lib.rs:289`).

### 9. Dungeons

1. `/v1/tactical/dungeon[.png|/scene]` chooses a seed from `?seed` or from `site_seed(world seed, gx, gy, kind)` and calls `arda_dungeon::generate` per request (`crates/arda-server/src/tactical/dungeon.rs:72`, `crates/arda-dungeon/src/lib.rs:229`, `crates/arda-dungeon/src/lib.rs:241`).
2. The PNG uses a `RenderKey` with `Anchor::Name`, so the image goes through the `pngs` and `renders` caches. The level itself is regenerated on every request (`crates/arda-server/src/tactical/dungeon.rs:138`).
3. The CLI `arda dungeon` writes layout, rules, meta and scene JSON files (`crates/arda-cli/src/dungeon.rs:94`).

### 10. Tactical bake (`POST /v1/tactical/bake`)

1. `submit` reads the JSON body (limit `max_body_bytes`, `Content-Type: application/json` required) and parses a `BakeRequest` (`crates/arda-server/src/tactical/bake_routes.rs:54`). `Baker::submit` derives a content-addressed job id, so re-posting the same body returns the same job; it answers 501 when `--bake-workers 0`, 503 `busy` when `QUEUE_JOBS` = 8 jobs wait (`crates/arda-server/src/tactical/bake.rs:189`, `crates/arda-server/src/tactical/bake.rs:35`).
2. The worker splices the client's edited cell into its apron block (`crates/arda-server/src/tactical/splice.rs:1`) and renders it in its own lane, reading and writing none of the shared caches; an unedited layout bakes to the cell route's tiles byte for byte (`crates/arda-server/src/tactical/bake.rs:349`, `crates/arda-server/src/tactical/bake_render.rs:1`).
3. Finished tile sets wait in a byte-budgeted store (`--bake-result-mib`) until `DELETE /v1/tactical/bake/{id}` or eviction of the oldest set; nothing reaches disk (`crates/arda-server/src/tactical/bake_store.rs:1`). `GET .../bake/{id}` returns `BakeStatus`; `GET .../tiles/{z}/{x}/{y}` returns one tile with a strong `ETag` and `Cache-Control: no-store` (`crates/arda-server/src/tactical/bake_routes.rs:98`, `crates/arda-server/src/tactical/bake_routes.rs:118`).

### 11. Viewer

1. `App` routes by URL hash to World, Area, Tactical, Cell and Editor views (`apps/viewer/src/App.tsx:17`).
2. `ArdaClient` builds `/v1/...` URLs against a base held in `localStorage`. Images are fetched with `fetch` and keep `x-arda-cache` (`apps/viewer/src/api/client.ts:152`, `apps/viewer/src/api/client.ts:195`, `apps/viewer/src/config.ts:9`).
3. The World view stacks Leaflet tile layers: overview tiles, upscaled as a backdrop, with relief tiles past native zoom on top (`apps/viewer/src/views/WorldView.tsx:52`, `apps/viewer/src/views/WorldView.tsx:62`).
4. The Cell view fetches the block JSON, the scene and the image, and fires prefetch (`apps/viewer/src/views/TacticalCellView.tsx:144`, `apps/viewer/src/views/TacticalCellView.tsx:275`).

### 12. Art library (offline)

1. `tools/art-gen` (Node scripts) produces raw images; `safety.mjs` plans the worst-case spend against `--max-spend` / `--max-images` before any POST and reconciles a POST whose reply was lost so a dropped connection does not bill twice (`tools/art-gen/safety.mjs:1`, `tools/art-gen/generate.mjs:17`).
2. `arda tactical import` runs `arda_art_import::import`, which cuts out, mattes, fits and grades the images and writes `catalog.json` plus images (`crates/arda-cli/src/tactical_import.rs:1`, `crates/arda-art-import/src/lib.rs:97`).
3. The server loads a `top:…:bottom` library stack at start (`crates/arda-server/src/main.rs:44`, `crates/arda-tactical/src/library_stack.rs:95`).

## State

| Store | Owner (site) | Key | Bound | Mutated by |
|---|---|---|---|---|
| World directory files | offline CLI | path | disk | `arda generate`, `arda settle`, `arda society build` only. The server never writes it. |
| `World.areas` | `crates/arda/src/world.rs:23` | area | unbounded `OnceLock` per area, never evicted | First `World::area` call. `WorldSource::cell` uses it (`crates/arda-refine/src/world_source.rs:192`). |
| `WorldSource.index` / `fine` | `crates/arda-refine/src/world_source.rs:52` | area / fine node | index unbounded; fine cleared at 2²¹ entries (`:15`) | refine and people reads |
| `WorldQuery.areas` / `derived` / `windows` | `crates/arda-server/src/query.rs:58` | `AreaCoord` | 1 GiB / 256 MiB / 512 MiB `ByteLru` (`:36`) | contract routes |
| `SocietyCells` | `crates/arda-server/src/society_cells.rs:47` | global cell | 8 bytes per cell, decoded at start | never |
| `arda_people::World` files, society, notables | `crates/arda-people/src/people.rs:43` | settlement / `NpcId` | read once at start | never |
| `World.plans` | `crates/arda-people/src/people.rs:137` | settlement id | 64 entries and 768 MiB with the draws' reservations (`plan_bytes`: square grid plus twice the JSON; `plan_estimate` reserved before a draw), LRU by tick, newest always kept | `plan`, `warm_plans`, town overlay |
| `SocietyOverlays` roads/land use/channels | `crates/arda-blocks/src/society/mod.rs:91` | — | built at start; `ChannelCache` filled on use | ways overlay |
| `Pipeline.cache` | `crates/arda-blocks/src/pipeline.rs:72` | `CellKey` | 256 refined blocks, LRU | `refined` |
| `RefineBlocks.composed` / `demos` | `crates/arda-server/src/tactical/refine_blocks.rs:33` | `BlockRequest` / anchor | 256 MiB `ByteLru` / 16 anchors | `block`, `demo` |
| `Tactical.renders` / `pyramids` | `crates/arda-server/src/tactical/mod.rs:164` | `RenderKey` | 768 MiB / 1 GiB (`:80`) | `raw_in`, `pyramid_in` |
| `Tactical.pngs` / `tiles` / `bodies` | `crates/arda-server/src/tactical/mod.rs:166` | `RenderKey`, tile key, `BodyKey` | 256 MiB each | `png`, `tile`, `world_body` |
| `Lanes` (request, prefetch, encode mutexes, in-flight set) | `crates/arda-server/src/tactical/raw.rs:35` | `RenderKey` | one render per lane, one encode | `raw_in`, `encode_slot` |
| Prefetch `Queue` | `crates/arda-server/src/tactical/prefetch.rs:149` | `(BlockRequest, ppsq)` | 64 jobs | `submit`, workers |
| `Baker` job table and result store | `crates/arda-server/src/tactical/bake.rs:115` | content-addressed job id | `QUEUE_JOBS` = 8 waiting, one rendering; finished tile sets within `--bake-result-mib` (oldest finished evicted first) | `submit`, the bake worker, `DELETE` |
| `Overview.pngs` / `tiles` / `base` | `crates/arda-server/src/overview.rs:118` | quality+style+society / zxy+format+oblique+labelled+society / (oblique, society) | `cache_bytes` each; `base` holds up to three decoded pyramids | `png`, `tile`, `base` |
| `Relief.tiles`; `ReliefWorld` contexts | `crates/arda-server/src/relief.rs:39`, `crates/arda-midzoom/src/world.rs:22` | zxy+labelled | 128 MiB `ByteLru` | `tile` |
| `area_builds` semaphore | `crates/arda-server/src/lib.rs:186` | — | 2 permits | `/area/.../cells` |
| Connection semaphore | `crates/arda-server/src/serve.rs:47` | — | 32 connections | accept loop |
| Viewer component state | React `useState` / `useAsync` (`apps/viewer/src/hooks.ts:38`) | — | per view | user actions; responses |
| Viewer API base | `localStorage` (`apps/viewer/src/config.ts:13`) | one key | — | `saveApiBase` |

- Every `ByteLru` evicts least recently used entries by tick and refuses to hold a single value larger than its capacity (`crates/arda-server/src/cache.rs:51`).
- Only raw renders are single-flight (`crates/arda-server/src/tactical/raw.rs:93`). Concurrent misses on any other cache compute the value twice and both insert.
- Server-state vs UI state on the client: the viewer has no client data store library. Server data lives in per-view React state, and browser HTTP revalidation uses the server's `ETag` with `Cache-Control: no-cache` (`crates/arda-server/src/routes.rs:161`).

## Side-effect boundaries

| Boundary | Site | Isolation |
|---|---|---|
| Generation writes | `WorldOutput` create-new layers under a scratch directory, then manifest rename (`crates/arda-gen/src/orchestrator/publication.rs:58`, `crates/arda-gen/src/orchestrator/publication.rs:95`) | all writes go through `WorldOutput`; I/O counted by `FinalWrites` (`crates/arda-gen/src/orchestrator.rs:179`) |
| Fine source staging | `SourceStage` (`crates/arda-gen/src/orchestrator/fine_delivery.rs:282`) | private directory beside the output |
| Settle writes | `std::fs::write` per file (`crates/arda-settle/src/lib.rs:159`) | none beyond encode-before-write |
| Society build writes | `write_json`, `files::write` (`crates/arda-society/src/output.rs:20`, `crates/arda-people/src/files.rs:85`) | none |
| Server world reads | `World::load`, `read_area`, the fine reader behind a `Mutex` (`crates/arda-server/src/query.rs:56`) | read-only |
| Server temp files | overview render scratch under `std::env::temp_dir()` (`crates/arda-server/src/overview.rs:349`) | unique create-dir names; removed on drop |
| Server network | axum on hyper http1 with header timeout and connection cap (`crates/arda-server/src/serve.rs:41`); CORS limited to localhost origins (`crates/arda-server/src/routes.rs:103`) | — |
| Server logging | `eprintln!` per tactical request, bake request and prefetch job (`crates/arda-server/src/tactical/routes.rs:137`, `crates/arda-server/src/tactical/prefetch.rs:339`, `crates/arda-server/src/tactical/bake_routes.rs:88`) | stderr only |
| Container healthcheck | `arda-server --probe` opens a loopback TCP connection, sends `GET /v1/health` and exits 0 on `200` (`crates/arda-server/src/probe.rs:1`, `crates/arda-server/src/main.rs:131`) | std-only, no async runtime; serves nothing |
| CPU work off the async runtime | every handler body runs in `spawn_blocking` (`crates/arda-server/src/routes.rs:124`); prefetch on its own OS threads (`crates/arda-server/src/tactical/prefetch.rs:272`) | — |

There are no outbound network calls (the `--probe` check and the offline art-gen client talk to loopback only), databases or message queues anywhere in the pipeline. The viewer's only I/O is `fetch` to the configured API base plus `localStorage`.

## Failure paths

- **Generate.** Admission failure happens before `WorldOutput::begin` and leaves the output untouched (`crates/arda-gen/src/orchestrator.rs:178`). Any later error leaves a partial directory without `world.json`, and a rerun needs a fresh path (`crates/arda-gen/src/orchestrator/publication.rs:120`). Rejected fine candidates are deleted; after the last attempt, a `Validation` error carries the last gate message (`crates/arda-gen/src/orchestrator/fine_delivery.rs:408`).
- **Settle.**
  - Encoding fails before any write, so a refused allocation leaves `society/` unchanged.
  - An I/O error partway through the write loop leaves a mix of new and old files (`crates/arda-settle/src/lib.rs:156`).
  - Settle does not check whether `society build` outputs exist. After a re-settle, stale `society.json` and `notables.json` stay on disk.
- **Society build.**
  - A planner thread panic becomes `PeopleError::World` (`crates/arda-people/src/build.rs:154`).
  - A refused town plan falls back to mix buildings (`crates/arda-people/src/town.rs:136`).
  - `society.json` is written before `notables.json`. A failure between the two leaves a new society beside old notables (`crates/arda-people/src/build.rs:48`).
- **Server start.** Admission, sheet mapping, world load, library validation, overlays and relief errors all abort startup with exit code 1 (`crates/arda-server/src/lib.rs:204`, `crates/arda-server/src/main.rs:150`). A world without `society/settlements.json` starts with `people = None`, and settlement and NPC routes then answer 404 (`crates/arda-server/src/people.rs:30`). `warm_plans` ignores failures (`crates/arda-people/src/people.rs:182`).
- **Request.**
  - Handler errors map to status codes in `ServerError::classify` (`crates/arda-server/src/error.rs:123`): open sea → 404 `no_block`, bad window → 400, the `PendingBlocks` "not yet" source → 501, oversize → 413, invalid layout → 422, an unsatisfiable `npc/generate` body → 422 `unsatisfiable`, a full bake queue → 503 `busy`, baking turned off → 501 `not_implemented`, everything else → 500 (`crates/arda-server/src/error.rs:134`, `crates/arda-server/src/error.rs:150`, `crates/arda-server/src/error.rs:151`).
  - A panic inside `spawn_blocking` becomes a 500 `internal` (`crates/arda-server/src/routes.rs:128`).
  - A poisoned cache mutex becomes a 500 (`crates/arda-server/src/error.rs:250`). Lane mutexes and the in-flight set ignore poisoning, and the `InFlight` drop guard clears the marker after a panicked render (`crates/arda-server/src/tactical/raw.rs:54`, `crates/arda-server/src/tactical/raw.rs:110`).
  - A client disconnect does not cancel a running blocking closure. Its result is still inserted into the cache.
- **Connections.** A slow request head times out and closes only that connection. At the connection cap, further clients wait (`crates/arda-server/src/serve.rs:47`, `crates/arda-server/src/serve.rs:65`).
- **Prefetch.**
  - Job errors, including open sea, are logged and dropped (`crates/arda-server/src/tactical/prefetch.rs:335`).
  - A full queue or zero workers reports `Dropped`.
  - Workers exit when the `Prefetcher` drops or the `AppState` is gone (`crates/arda-server/src/tactical/prefetch.rs:306`, `crates/arda-server/src/tactical/prefetch.rs:319`).
  - A worker thread spawn failure is a 500 on the submitting request (`crates/arda-server/src/tactical/prefetch.rs:279`).
- **Bake.**
  - A repeat of a known body returns the known job; a job that does not fit the result budget fails with `resource_limit`, and a deleted or evicted job answers 404 (`crates/arda-server/src/tactical/bake_store.rs:1`, `crates/arda-server/src/tactical/bake.rs:189`).
  - A worker thread spawn failure is a 500 on the submitting request (`crates/arda-server/src/tactical/bake.rs:237`).
- **Relief.** A world without formed fine terrain returns 404. A midzoom `ResourceLimit` returns 500 `resource_limit` (`crates/arda-server/src/relief.rs:52`, `crates/arda-server/src/relief.rs:150`).
- **Viewer.** Effects abort in-flight fetches with `AbortController` on cell change or unmount. A prefetch failure only sets a status string (`apps/viewer/src/views/TacticalCellView.tsx:277`).

Coverage note: the internals of `arda_town::generate`, `arda_refine::refine` layers, the `arda_tactical` compositor passes and `arda_midzoom` refinement were traced only at their entry points (shallow).
