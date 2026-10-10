---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 8ffa91a241fb
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
---

# Data flow

The pipeline has two halves. Three offline CLI batches write files under one world directory: `arda generate`, `arda settle` and `arda society build` (`crates/arda-cli/src/main.rs:360`, `crates/arda-cli/src/society.rs:1`). `arda-server` reads those files and computes everything else per request, behind bounded in-memory caches (`crates/arda-server/src/lib.rs:171`). Physical generation internals (tectonics, formation recipes, the hydrology solve) are covered in `logic/01`–`02` and `01-architecture.md`. This chapter traces them only as far as their published files.

## Lifecycles

### Persisted vs computed

| Product | Persisted on disk (writer) | Computed on demand (reader) |
|---|---|---|
| Manifest | `world.json`, renamed into place last (`crates/arda-gen/src/orchestrator/publication.rs:122`, `crates/arda-core/src/formats/manifest.rs:88`) | — |
| Area cells/objects | `areas/<ax>_<ay>/cells.bin`, `objects.bin` (`crates/arda-gen/src/orchestrator/final_writes.rs:136`) | Decoded per request into `WorldQuery.areas` (`crates/arda-server/src/query.rs:158`) |
| Area water forms | `areas/<ax>_<ay>/water.bin`, formed recipes only (`crates/arda-gen/src/orchestrator.rs:368`) | — |
| Hydrology tables | `hydrology/*.bin` (`crates/arda-gen/src/orchestrator/global_output.rs:52`) | — |
| Continent | `continent/overview.bin`, `continent/objects.bin` (`crates/arda-gen/src/orchestrator.rs:424`, `crates/arda-gen/src/orchestrator.rs:430`) | — |
| Fine terrain | `terrain/fine.bin`, copied from the staged source (`crates/arda-core/src/formats/manifest.rs:43`, `crates/arda-gen/src/orchestrator.rs:198`) | Per-area windows in `WorldQuery.windows` (`crates/arda-server/src/query.rs:206`) |
| Skeleton blocks | `blocks/<ax>_<ay>.tiles.zst`, land cells at stride `SKELETON_BLOCK_STRIDE` (`crates/arda-gen/src/orchestrator/final_writes.rs:144`) | Read only by `arda export --block` (`crates/arda/src/lib.rs:224`). The server never reads them. |
| Settlements, roads, realms, names, land use | `society/settlements.json`, `roads.json`, `realms.json`, `names.json`, `landuse.bin`, `realms.bin`, `roads.bin`, `stats.json` (`crates/arda-settle/src/lib.rs:61`) | Rasters decoded once at server start (`crates/arda-server/src/society_cells.rs:47`) |
| Society simulation | `society/society.json` (`crates/arda-people/src/build.rs:48`) | Loaded once at server start (`crates/arda-people/src/people.rs:79`) |
| Notables | `society/notables.json`, notables only (`crates/arda-people/src/build.rs:49`) | Indexed by `NpcId` at start (`crates/arda-people/src/people.rs:98`) |
| Commoners, households, roster | Not stored | Regenerated per request by `Generator::with_notables` (`crates/arda-people/src/people.rs:253`, `crates/arda-server/src/npcs/mod.rs:206`) |
| Town plans | Not stored (computed during `society build`, then discarded) | `World::plan`, LRU of 64 (`crates/arda-people/src/people.rs:18`, `crates/arda-people/src/people.rs:137`) |
| Refined tactical blocks | Not stored | `Pipeline.cache`, 256 cells (`crates/arda-blocks/src/pipeline.rs:112`, `crates/arda-server/src/tactical/refine_blocks.rs:24`) |
| Composed blocks, renders, PNG/WebP/JSON bodies | Not stored | `ByteLru` caches (`crates/arda-server/src/tactical/mod.rs:157`) |
| Dungeons | Only by `arda dungeon --out` (`crates/arda-cli/src/dungeon.rs:94`) | Regenerated per request. Only the PNG is cached (`crates/arda-server/src/tactical/dungeon.rs:72`). |
| Overview PNG and tiles | Not stored. A render goes through a temp scratch directory that is removed on drop (`crates/arda-server/src/overview.rs:264`, `crates/arda-server/src/overview.rs:291`). | `Overview.pngs`, `tiles` and `base` (`crates/arda-server/src/overview.rs:113`) |
| Mid-zoom relief tiles | Not stored | `Relief.tiles` (`crates/arda-server/src/relief.rs:114`) |
| Tactical asset library | `catalog.json` plus images, from `arda tactical import` (`crates/arda-art-import/src/lib.rs:97`) or the placeholder generator | Loaded and pre-scaled at start (`crates/arda-server/src/tactical/mod.rs:188`) |

### 1. `arda generate` (continent → areas → blocks)

1. CLI `run_generate` picks the source. `--terrain` defaults to `Legacy` (`crates/arda-cli/src/main.rs:51`). Legacy calls `arda::generate` and fine calls `generate_from_fine_recipe`, default recipe 7 (`crates/arda-cli/src/generate.rs:85`, `crates/arda-cli/src/generate.rs:95`, `crates/arda-core/src/formats/manifest.rs:58`).
2. The fine path admits resources and refuses an occupied output (`crates/arda-gen/src/orchestrator/fine_delivery.rs:280`). It then loops over up to `CONTINENT_ATTEMPTS` candidates (`crates/arda-gen/src/orchestrator/fine_delivery.rs:286`). Each candidate is either formed (`fine_formation::generate`, `:294`) or spectral then valley-carved (`fine_source::generate`, `:307`; `fine_valleys::carve`, `:330`). A candidate must pass the land gate and the river gate (`:369`). The accepted file goes to `generate_world_from_formed_terrain` or `generate_world_from_fine_terrain` (`:382`, `:391`). The private source stage is removed either way (`:400`, `:408`).
3. `generate_world_inner` (`crates/arda-gen/src/orchestrator.rs:173`) does the following, in order:
   - admits resources (`:185`)
   - opens the transaction (`WorldOutput::begin`, `:196`) and copies the fine field in (`:198`)
   - runs the continent attempt loop with climate/hydrology/river gate (`:218`)
   - builds `SharedTerrain` (`:281`, `:290`) and writes prepared tiles per area to scratch (`:294`)
   - runs the shared water solve (`:302`) and builds the final index (`:321`)
4. Per area, the orchestrator composes cells and objects (`area_output::compose`, `:337`), applies ground ecology (`fine_materials::apply`, `:351`), then calls `write_area` (`:353`). `write_area` writes `cells.bin` and `objects.bin` and fills the skeleton block archive: `constraints_for`/`fill_block` at stride 64 on land cells (`crates/arda-gen/src/orchestrator/final_writes.rs:127`).
5. Next come water forms (`crates/arda-gen/src/orchestrator.rs:368`), global hydrology tables (`:380`), and the continent overview and objects (`:424`, `:430`).
6. `output.commit` renames the manifest last (`crates/arda-gen/src/orchestrator.rs:451`, `crates/arda-gen/src/orchestrator/publication.rs:122`). The manifest's `settlement_count` is written as 0 (`crates/arda-gen/src/orchestrator.rs:446`), and no later stage rewrites `world.json`.

### 2. `arda settle` (settlements, roads, realms, land use)

1. `society::settle` calls `arda_settle::generate` with a 16 GiB budget (`crates/arda-cli/src/society.rs:28`, `crates/arda-settle/src/grid.rs:15`).
2. `generate` loads the world. `load_grid` reads every area uncached (`World::read_area`) into one global `Grid` (`crates/arda-settle/src/lib.rs:160`, `crates/arda-settle/src/load.rs:20`).
3. `pipeline::run` (`crates/arda-settle/src/pipeline.rs:53`) runs these stages, in order:
   - site tags, suitability and placement (`:55`)
   - realm seats, allegiance and primacy (`:65`)
   - cultures and settlement profiles (`:72`)
   - land use (`landuse::allocate`, `:78`)
   - cost surface and roads (`:79`)
   - crossings and passes (`:81`)
   - realm partition (`:83`)
   - naming (`naming::name_world`, `:93`)
   - stats (`:103`)
4. `write` encodes every file before writing the first, then writes them one by one with `std::fs::write` into `<world>/society/` (`crates/arda-settle/src/lib.rs:145`).

### 3. `arda society build` (town plans, society, notables)

1. `society::run` calls `arda_people::build` (`crates/arda-cli/src/society.rs:41`, `crates/arda-people/src/build.rs:45`).
2. `compute` opens a `SharedSource` and reads `settlements.json` and `roads.json` (`crates/arda-people/src/build.rs:58`, `crates/arda-people/src/files.rs:90`).
3. `plan_all` draws every settlement's plan on scoped threads, in chunks of 4 × the core count, keeping results in order (`crates/arda-people/src/build.rs:122`).
   - Each plan samples terrain (`Surroundings::around`) and refined rivers from the world (`crates/arda-people/src/town.rs:99`), then calls `arda_town::generate` (`crates/arda-people/src/town.rs:136`, `crates/arda-town/src/plan/mod.rs:60`).
   - A refused plan is `Ok(None)`.
4. The plan buildings become `WorldSettlements.buildings` (`crates/arda-people/src/build.rs:76`). Then `arda_society::simulate_society` runs (`crates/arda-people/src/build.rs:78`, `crates/arda-society/src/lib.rs:134`). Its stages, in order:
   - economy (`crates/arda-society/src/economy/mod.rs:118`)
   - government (`crates/arda-society/src/politics/government.rs:21`)
   - history (`crates/arda-society/src/history/run.rs:47`)
   - realm states (`crates/arda-society/src/politics/realm.rs:103`)
   - relations (`crates/arda-society/src/politics/relations.rs:89`)
   - per-settlement factions (`crates/arda-society/src/politics/factions.rs:99`) and offices/roles (`crates/arda-society/src/roles.rs:115`)
   - settlement hooks and region (realm) hooks (`crates/arda-society/src/lib.rs:201`, `crates/arda-society/src/lib.rs:206`)
5. `history.json`, `factions.json`, `roles.json` and the other tables are compile-time embedded inputs under `crates/arda-society/data/`, not outputs (`crates/arda-society/src/tables.rs:315`). The simulated history is serialised inside `society.json` (`crates/arda-society/src/lib.rs:125`).
6. Per settlement, buildings come from the plan (`BuildingSource::Plan`). Without a plan they come from the settlement's mix (`BuildingSource::Mix`) (`crates/arda-people/src/build.rs:88`). Society roles become `NotableSlot`s, and `Generator::with_notables(...).population()` builds the population (`crates/arda-people/src/build.rs:94`, `crates/arda-npc/src/lib.rs:74`, `crates/arda-npc/src/lib.rs:142`). Only `pop.npcs` (the notables) is kept.
7. `society.json` and then `notables.json` are written (`crates/arda-people/src/build.rs:48`).

### 4. Server start

1. `main` runs `AppState::open` on a blocking thread, then binds and serves (`crates/arda-server/src/main.rs:71`, `crates/arda-server/src/main.rs:86`). Startup steps, in order:
   - memory admission against the 16 GiB ceiling (`crates/arda-server/src/lib.rs:130`)
   - sheet mapping (`:174`)
   - `WorldQuery::open` (`:179`), which loads the world, the fine reader and the society rasters (`crates/arda-server/src/query.rs:78`)
   - `Tactical::open`, which loads and validates the library stack and pre-scales textures for every ppsq (`crates/arda-server/src/tactical/mod.rs:188`)
2. When `society/settlements.json` exists, `arda_people::World::with_source` loads `society.json` and `notables.json` if present (`crates/arda-server/src/lib.rs:187`, `crates/arda-people/src/people.rs:79`). `warm_towns` then pre-draws up to 32 plans for the most populous towns and cities (`crates/arda-server/src/lib.rs:198`, `crates/arda-people/src/people.rs:169`).
3. `SocietyOverlays::open` converts roads and crossings and reads `landuse.bin` (`crates/arda-server/src/lib.rs:200`, `crates/arda-blocks/src/society/mod.rs:115`). The same overlays feed the relief `Landscape` (`crates/arda-server/src/lib.rs:208`).
4. `RefineBlocks::with_society` becomes the block source (`crates/arda-server/src/lib.rs:211`). `Overview::new` sets up the overview (`:214`). `Relief::open` follows (`:215`), and its `ReliefWorld` doubles as the tactical world-tint source (`:221`).
5. Prefetch workers are not started at boot. The first `POST /prefetch` starts them (`crates/arda-server/src/tactical/prefetch.rs:230`).
6. Three independent `arda::World` instances are loaded (`crates/arda-server/src/query.rs:79`, `crates/arda-people/src/shared.rs:26`, `crates/arda-server/src/relief.rs:70`). Each keeps its own caches.

### 5. Tactical cell request (`GET /v1/tactical/cell/{gx}/{gy}[.png]`)

1. The handler parses `gx`/`gy`, ppsq and flags and builds a `BlockRequest`, then hands off to `blocking` (`spawn_blocking`) (`crates/arda-server/src/tactical/world_routes.rs:147`, `crates/arda-server/src/routes.rs:106`).
2. **PNG path** (`world_png_look`, `crates/arda-server/src/tactical/cells.rs:116`):
   - Grows the request by `APRON` = 6 squares (`crates/arda-server/src/tactical/cells.rs:23`, `crates/arda-server/src/tactical/cells.rs:83`).
   - Gets the block from `RefineBlocks::block`, checking the composed cache first (`crates/arda-server/src/tactical/refine_blocks.rs:166`).
   - Builds a cropped `RenderKey`: seed, origin anchor, BLAKE3 of the layout JSON, ppsq, grid, library version, crop and world grade (`crates/arda-server/src/tactical/mod.rs:337`).
   - Calls `png` (`crates/arda-server/src/tactical/mod.rs:363`).
3. **Cold block** (`compose`, `crates/arda-server/src/tactical/refine_blocks.rs:93`):
   - A whole-cell request on open sea returns `NoBlock`.
   - `Pipeline::window` refines every covered cell on scoped threads, each through the per-cell cache, calling `arda_refine::refine` (`crates/arda-blocks/src/pipeline.rs:165`, `crates/arda-blocks/src/pipeline.rs:112`, `crates/arda-refine/src/block.rs:167`).
   - `assemble` joins the cells into one layout (`crates/arda-blocks/src/pipeline.rs:196`).
   - `compose_all` runs the ways, fields and town providers concurrently and composes them in that fixed order (`crates/arda-blocks/src/overlays.rs:296`).
4. **Overlay providers:**
   - Ways: `arda_ways::apply_ways` over roads filtered to a 600 m margin (`crates/arda-blocks/src/society/mod.rs:165`, `crates/arda-blocks/src/society/layers.rs:156`).
   - Fields: `arda_fields::generate` over `landuse.bin` (`crates/arda-blocks/src/society/layers.rs:282`, `crates/arda-fields/src/window.rs:100`).
   - Town: `plans_near` calls `World::plan` for each settlement in reach (cached, drawn in parallel on a miss). Then `arda_town::block::generate` and `fallback::resolve` cut that plan's squares for the window (`crates/arda-blocks/src/society/town.rs:30`, `crates/arda-blocks/src/society/town.rs:127`).
5. **Render.** `png` checks the `pngs` cache, then calls `raw` (`crates/arda-server/src/tactical/raw.rs:77`). `raw` works through these steps:
   - checks the `renders` cache
   - waits on any in-flight render of the same key, then marks the key in-flight
   - takes the request-lane mutex
   - admits the layout (squares, pixels, work) (`crates/arda-server/src/tactical/mod.rs:296`)
   - calls the compositor: `arda_tactical::render_region`, or `render_region_tinted` with `world_grade=1` (`crates/arda-tactical/src/compose/mod.rs:270`, `crates/arda-tactical/src/compose/mod.rs:288`)
   - the PNG encode then runs under the single shared encode slot (`crates/arda-server/src/tactical/mod.rs:372`)
6. **JSON path** (`world_body`, `crates/arda-server/src/tactical/cells.rs:161`): checks the `bodies` cache, then runs block → `arda_scene::build_scene` (`crates/arda-scene/src/lib.rs:51`) → `TacticalBlockDto` → cache.
7. **Response.** `image` sets a strong `ETag` (BLAKE3), `Cache-Control: no-cache`, `X-Arda-Cache: hit|miss` and `Server-Timing`. It answers 304 when `If-None-Match` matches (`crates/arda-server/src/tactical/routes.rs:142`).
8. **Tiles** (`/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp`, `crates/arda-server/src/tactical/world_routes.rs:188`): checks the `tiles` cache, then the `pyramids` cache (built from `raw`), then cuts and WebP-encodes the tile (`crates/arda-server/src/tactical/mod.rs:409`).
9. **Scene** (`/cell/{gx}/{gy}/scene`): block → `build_scene` → NPC tokens placed from `notables_of` for each settlement whose buildings appear in the rules (`crates/arda-server/src/tactical/scene.rs:57`). Scenes are not cached.
10. **Windows** (`/window`, `/window.png`) take up to 3 × 3 cells or 192 squares a side and go through the same path (`crates/arda-server/src/tactical/world_routes.rs:230`).

### 6. Prefetch (`POST /v1/tactical/prefetch`)

1. The handler calls `Prefetcher::submit` and answers 202 at once (`crates/arda-server/src/tactical/routes.rs:63`, `crates/arda-server/src/tactical/prefetch.rs:198`).
2. `submit` validates the radius (1..=2) and ppsq, and starts the workers if this is the first call. It walks `rings` nearest-first, clipped to the world (`crates/arda-server/src/tactical/prefetch.rs:105`).
3. Each cell is reported as one of three statuses:
   - `Pending`: already known
   - `Dropped`: the queue holds `QUEUE_CELLS` = 64 jobs, or there are no workers
   - `Queued`
4. `image` is true only for the first `image_slots` cells, the number whose render and pyramid fit the caches beside the open cell (`crates/arda-server/src/tactical/prefetch.rs:130`).
5. Workers (default 2, max 8) pop jobs and call `Tactical::warm`. `warm` fills the body cache, and when `image` is set also the render and pyramid in the prefetch lane (`crates/arda-server/src/tactical/prefetch.rs:314`, `crates/arda-server/src/tactical/prefetch.rs:363`). Keys equal the request keys, so a later request is a cache hit.
6. The viewer posts radius 1 whenever the cell changes (`apps/viewer/src/views/TacticalCellView.tsx:275`).

### 7. Overview and relief tiles

1. `/v1/overview.png` (`crates/arda-server/src/routes.rs:302`): `Overview::png` checks the `pngs` cache, then serialises renders behind `render` (`crates/arda-server/src/overview.rs:195`). `render` calls `arda::export_overview_with_look` into a temp scratch directory, reads the file back and drops the directory (`crates/arda-server/src/overview.rs:291`).
2. `/v1/tiles/overview/{z}/{x}/{y}` (`crates/arda-server/src/routes.rs:320`):
   - Checks the `tiles` cache.
   - On a miss, `base` decodes the `tile_base_px` (default 4096) Atlas or AtlasOblique PNG once and keeps it in one of two slots (`crates/arda-server/src/overview.rs:214`).
   - `tiles::cut` cuts the tile, which is then PNG- or WebP-encoded (`crates/arda-server/src/overview.rs:236`).
3. `/v1/tiles/relief/{z}/{x}/{y}.webp` (`crates/arda-server/src/routes.rs:349`): `Relief::tile` checks the cache, then calls `arda_midzoom::render_tile` for levels past the overview's native zoom and encodes WebP (`crates/arda-server/src/relief.rs:114`, `crates/arda-midzoom/src/tile.rs:41`). Relief exists only for manifests with fine recipe ≥ 5 (`crates/arda-server/src/relief.rs:77`).

### 8. Contract, settlement and NPC requests

1. `/v1/cell/{gx}/{gy}` and `/v1/point` read from three caches (`crates/arda-server/src/query.rs:251`, `crates/arda-server/src/query.rs:309`):
   - the decoded area (`crates/arda-server/src/query.rs:158`)
   - the derived coast/river/lake layer, which loads the 3 × 3 neighbourhood on a miss (`:177`)
   - the fine window (`:206`)
   - The society raster values are added on top.
2. `/v1/area/{ax}/{ay}/cells` takes an `area_builds` semaphore permit (default 2) before building and encoding 262,144 samples. It is not cached (`crates/arda-server/src/routes.rs:209`, `crates/arda-server/src/query.rs:263`).
3. Settlement routes read `files`, `society` and `World::plan` (`crates/arda-server/src/people.rs:51`, `crates/arda-server/src/people.rs:89`).
   - `/settlements/{id}/npcs` returns stored notables only (`crates/arda-server/src/people.rs:106`).
   - `/v1/npc/{id}` resolves a stored notable by id first. Otherwise it rebuilds the settlement's `Generator` from profile, buildings (plan or mix) and slots, and expands one commoner (`crates/arda-server/src/npcs/mod.rs:177`, `crates/arda-server/src/npcs/page.rs:87`).
   - `/v1/npcs` and `/buildings/{id}/residents|workers` page through regenerated rosters (`crates/arda-server/src/npcs/page.rs:134`).
   - NPC bodies pass through the optional sheet mapping (`crates/arda-server/src/lib.rs:248`).

### 9. Dungeons

1. `/v1/tactical/dungeon[.png|/scene]` chooses a seed from `?seed` or from `site_seed(world seed, gx, gy, kind)` and calls `arda_dungeon::generate` per request (`crates/arda-server/src/tactical/dungeon.rs:72`, `crates/arda-dungeon/src/lib.rs:227`, `crates/arda-dungeon/src/lib.rs:239`).
2. The PNG uses a `RenderKey` with `Anchor::Name`, so the image goes through the `pngs` and `renders` caches. The level itself is regenerated on every request (`crates/arda-server/src/tactical/dungeon.rs:138`).
3. The CLI `arda dungeon` writes layout, rules, meta and scene JSON files (`crates/arda-cli/src/dungeon.rs:94`).

### 10. Viewer

1. `App` routes by URL hash to World, Area, Tactical, Cell and Editor views (`apps/viewer/src/App.tsx:17`).
2. `ArdaClient` builds `/v1/...` URLs against a base held in `localStorage`. Images are fetched with `fetch` and keep `x-arda-cache` (`apps/viewer/src/api/client.ts:152`, `apps/viewer/src/api/client.ts:195`, `apps/viewer/src/config.ts:9`).
3. The World view stacks Leaflet tile layers: overview tiles, upscaled as a backdrop, with relief tiles past native zoom on top (`apps/viewer/src/views/WorldView.tsx:52`, `apps/viewer/src/views/WorldView.tsx:62`).
4. The Cell view fetches the block JSON, the scene and the image, and fires prefetch (`apps/viewer/src/views/TacticalCellView.tsx:144`, `apps/viewer/src/views/TacticalCellView.tsx:275`).

### 11. Art library (offline)

1. `tools/art-gen` (Node scripts) produces raw images.
2. `arda tactical import` runs `arda_art_import::import`, which cuts out, mattes, fits and grades the images and writes `catalog.json` plus images (`crates/arda-cli/src/tactical_import.rs:1`, `crates/arda-art-import/src/lib.rs:97`).
3. The server loads a `top:…:bottom` library stack at start (`crates/arda-server/src/main.rs:40`, `crates/arda-tactical/src/library_stack.rs:95`).

## State

| Store | Owner (site) | Key | Bound | Mutated by |
|---|---|---|---|---|
| World directory files | offline CLI | path | disk | `arda generate`, `arda settle`, `arda society build` only. The server never writes it. |
| `World.areas` / `blocks` | `crates/arda/src/world.rs:23` | area | unbounded `OnceLock` per area, never evicted | First `World::area` call. `WorldSource::cell` uses it (`crates/arda-refine/src/world_source.rs:189`). |
| `WorldSource.index` / `fine` | `crates/arda-refine/src/world_source.rs:52` | area / fine node | index unbounded; fine cleared at 2²¹ entries (`:15`) | refine and people reads |
| `WorldQuery.areas` / `derived` / `windows` | `crates/arda-server/src/query.rs:58` | `AreaCoord` | 1 GiB / 256 MiB / 512 MiB `ByteLru` (`:36`) | contract routes |
| `SocietyCells` | `crates/arda-server/src/society_cells.rs:47` | global cell | 8 bytes per cell, decoded at start | never |
| `arda_people::World` files, society, notables | `crates/arda-people/src/people.rs:43` | settlement / `NpcId` | read once at start | never |
| `World.plans` | `crates/arda-people/src/people.rs:137` | settlement id | 64 entries, LRU by tick | `plan`, `warm_plans`, town overlay |
| `SocietyOverlays` roads/land use/channels | `crates/arda-blocks/src/society/mod.rs:91` | — | built at start; `ChannelCache` filled on use | ways overlay |
| `Pipeline.cache` | `crates/arda-blocks/src/pipeline.rs:72` | `CellKey` | 256 refined blocks, LRU | `refined` |
| `RefineBlocks.composed` / `demos` | `crates/arda-server/src/tactical/refine_blocks.rs:33` | `BlockRequest` / anchor | 256 MiB `ByteLru` / 16 anchors | `block`, `demo` |
| `Tactical.renders` / `pyramids` | `crates/arda-server/src/tactical/mod.rs:157` | `RenderKey` | 768 MiB / 1 GiB (`:80`) | `raw_in`, `pyramid_in` |
| `Tactical.pngs` / `tiles` / `bodies` | `crates/arda-server/src/tactical/mod.rs:159` | `RenderKey`, tile key, `BodyKey` | 256 MiB each | `png`, `tile`, `world_body` |
| `Lanes` (request, prefetch, encode mutexes, in-flight set) | `crates/arda-server/src/tactical/raw.rs:31` | `RenderKey` | one render per lane, one encode | `raw_in`, `encode_slot` |
| Prefetch `Queue` | `crates/arda-server/src/tactical/prefetch.rs:149` | `(BlockRequest, ppsq)` | 64 jobs | `submit`, workers |
| `Overview.pngs` / `tiles` / `base` | `crates/arda-server/src/overview.rs:113` | quality+style / zxy+format / oblique | `cache_bytes` each; 2 base slots | `png`, `tile`, `base` |
| `Relief.tiles`; `ReliefWorld` contexts | `crates/arda-server/src/relief.rs:35`, `crates/arda-midzoom/src/world.rs:22` | zxy | 128 MiB `ByteLru` | `tile` |
| `area_builds` semaphore | `crates/arda-server/src/lib.rs:155` | — | 2 permits | `/area/.../cells` |
| Connection semaphore | `crates/arda-server/src/serve.rs:47` | — | 64 connections | accept loop |
| Viewer component state | React `useState` / `useAsync` (`apps/viewer/src/hooks.ts:38`) | — | per view | user actions; responses |
| Viewer API base | `localStorage` (`apps/viewer/src/config.ts:13`) | one key | — | `saveApiBase` |

- Every `ByteLru` evicts least recently used entries by tick and refuses to hold a single value larger than its capacity (`crates/arda-server/src/cache.rs:51`).
- Only raw renders are single-flight (`crates/arda-server/src/tactical/raw.rs:87`). Concurrent misses on any other cache compute the value twice and both insert.
- Server-state vs UI state on the client: the viewer has no client data store library. Server data lives in per-view React state, and browser HTTP revalidation uses the server's `ETag` with `Cache-Control: no-cache` (`crates/arda-server/src/routes.rs:143`).

## Side-effect boundaries

| Boundary | Site | Isolation |
|---|---|---|
| Generation writes | `WorldOutput` create-new layers under a scratch directory, then manifest rename (`crates/arda-gen/src/orchestrator/publication.rs:58`, `crates/arda-gen/src/orchestrator/publication.rs:95`) | all writes go through `WorldOutput`; I/O counted by `FinalWrites` (`crates/arda-gen/src/orchestrator.rs:186`) |
| Fine source staging | `SourceStage` (`crates/arda-gen/src/orchestrator/fine_delivery.rs:282`) | private directory beside the output |
| Settle writes | `std::fs::write` per file (`crates/arda-settle/src/lib.rs:148`) | none beyond encode-before-write |
| Society build writes | `write_json`, `files::write` (`crates/arda-society/src/output.rs:20`, `crates/arda-people/src/files.rs:79`) | none |
| Server world reads | `World::load`, `read_area`, the fine reader behind a `Mutex` (`crates/arda-server/src/query.rs:56`) | read-only |
| Server temp files | overview render scratch under `std::env::temp_dir()` (`crates/arda-server/src/overview.rs:296`) | unique create-dir names; removed on drop |
| Server network | axum on hyper http1 with header timeout and connection cap (`crates/arda-server/src/serve.rs:41`); CORS limited to localhost origins (`crates/arda-server/src/routes.rs:92`) | — |
| Server logging | `eprintln!` per tactical request and prefetch job (`crates/arda-server/src/tactical/routes.rs:132`, `crates/arda-server/src/tactical/prefetch.rs:339`) | stderr only |
| CPU work off the async runtime | every handler body runs in `spawn_blocking` (`crates/arda-server/src/routes.rs:106`); prefetch on its own OS threads (`crates/arda-server/src/tactical/prefetch.rs:272`) | — |

There are no outbound network calls, databases or message queues anywhere in the pipeline. The viewer's only I/O is `fetch` to the configured API base plus `localStorage`.

## Failure paths

- **Generate.** Admission failure happens before `WorldOutput::begin` and leaves the output untouched (`crates/arda-gen/src/orchestrator.rs:185`). Any later error leaves a partial directory without `world.json`, and a rerun needs a fresh path (`crates/arda-gen/src/orchestrator/publication.rs:120`). Rejected fine candidates are deleted; after the last attempt, a `Validation` error carries the last gate message (`crates/arda-gen/src/orchestrator/fine_delivery.rs:408`).
- **Settle.**
  - Encoding fails before any write, so a refused allocation leaves `society/` unchanged.
  - An I/O error partway through the write loop leaves a mix of new and old files (`crates/arda-settle/src/lib.rs:145`).
  - Settle does not check whether `society build` outputs exist. After a re-settle, stale `society.json` and `notables.json` stay on disk.
- **Society build.**
  - A planner thread panic becomes `PeopleError::World` (`crates/arda-people/src/build.rs:154`).
  - A refused town plan falls back to mix buildings (`crates/arda-people/src/town.rs:136`).
  - `society.json` is written before `notables.json`. A failure between the two leaves a new society beside old notables (`crates/arda-people/src/build.rs:48`).
- **Server start.** Admission, sheet mapping, world load, library validation, overlays and relief errors all abort startup with exit code 1 (`crates/arda-server/src/lib.rs:171`, `crates/arda-server/src/main.rs:97`). A world without `society/settlements.json` starts with `people = None`, and settlement and NPC routes then answer 404 (`crates/arda-server/src/people.rs:30`). `warm_plans` ignores failures (`crates/arda-people/src/people.rs:182`).
- **Request.**
  - Handler errors map to status codes in `ServerError::classify` (`crates/arda-server/src/error.rs:114`): open sea → 404 `no_block`, bad window → 400, the `PendingBlocks` "not yet" source → 501, oversize → 413, invalid layout → 422, everything else → 500.
  - A panic inside `spawn_blocking` becomes a 500 `internal` (`crates/arda-server/src/routes.rs:110`).
  - A poisoned cache mutex becomes a 500 (`crates/arda-server/src/error.rs:236`). Lane mutexes and the in-flight set ignore poisoning, and the `InFlight` drop guard clears the marker after a panicked render (`crates/arda-server/src/tactical/raw.rs:48`, `crates/arda-server/src/tactical/raw.rs:104`).
  - A client disconnect does not cancel a running blocking closure. Its result is still inserted into the cache.
- **Connections.** A slow request head times out and closes only that connection. At the connection cap, further clients wait (`crates/arda-server/src/serve.rs:47`, `crates/arda-server/src/serve.rs:65`).
- **Prefetch.**
  - Job errors, including open sea, are logged and dropped (`crates/arda-server/src/tactical/prefetch.rs:335`).
  - A full queue or zero workers reports `Dropped`.
  - Workers exit when the `Prefetcher` drops or the `AppState` is gone (`crates/arda-server/src/tactical/prefetch.rs:306`, `crates/arda-server/src/tactical/prefetch.rs:319`).
  - A worker thread spawn failure is a 500 on the submitting request (`crates/arda-server/src/tactical/prefetch.rs:279`).
- **Relief.** A world without formed fine terrain returns 404. A midzoom `ResourceLimit` returns 500 `resource_limit` (`crates/arda-server/src/relief.rs:47`, `crates/arda-server/src/relief.rs:115`).
- **Viewer.** Effects abort in-flight fetches with `AbortController` on cell change or unmount. A prefetch failure only sets a status string (`apps/viewer/src/views/TacticalCellView.tsx:277`).

Coverage note: the internals of `arda_town::generate`, `arda_refine::refine` layers, the `arda_tactical` compositor passes and `arda_midzoom` refinement were traced only at their entry points (shallow).
