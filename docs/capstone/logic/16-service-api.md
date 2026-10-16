---
generated_date: 2026-09-30
scenario: service-api
status: implemented in crate arda-server (every route of §api-routes; frozen for 1.0 under [versioning.md](../versioning.md)); developer viewer in apps/viewer
goals: 41, 57, 65, 66, 67, 68, 69, 70
---

# 16 — Service API: HTTP routes, contract versioning, caching and TS bindings

> Normative design for `arda-server`, the Rust HTTP service the maintainer's TS/React game calls (goal 65). It serves the world contract, the overview, settlements, roads, realms, history and NPCs, and on-demand tactical maps: layout, painted tiles and scene data. The byte-level reference for every route is `crates/arda-server/API.md`; this file states the rules every route follows. What is frozen for 1.x, and what each release may change, is [versioning.md](../versioning.md). It composes [08](08-settlements-roads-realms.md), [09](09-tactical-refinement.md), [10](10-town-layout.md), [11](11-tactical-art-compositor.md), [12](12-scene-data.md), [13](13-npc-population.md), [14](14-society-history.md) and [15](15-naming.md).

Code cites rules as `// logic/16 §<rule>`.

## Trigger & preconditions

- Trigger: `arda-server --world <dir> [--library <dir>] [--port 8787] [--host 127.0.0.1]` and HTTP requests.
- Preconditions at start: the world loads (`World::load`); the tactical library validates (11 §validator); `society/` is optional (routes that need it answer `404 not_found` naming the missing file); cache budgets pass admission against the 16 GiB ceiling (§api-cache). Any failure refuses to start with the reason on stderr.

## Rules

### §api-conventions

- Every route is under `/v1`. JSON unless noted. Coordinates follow 08 §world-frame (cells) and 09 §square-frame (squares).
- **Big integers**: every u64 on the wire (seed, reach and basin ids, render keys) is a decimal string, because it does not fit a JS number. Settlement, building, realm and road ids are also strings on the wire even though they are small, so the TS types never change when a world grows (the domain files under `society/` keep numbers). `NpcId` is `"<settlement>.<building>.<index>"` (13 §npc-id).
- **Determinism**: the same request against the same world, society and library returns identical bytes, cold or cached (goal-prompt §8).
- **CORS**: `GET`, `HEAD`, `POST`, `DELETE` and `OPTIONS` from `http(s)://localhost`, `127.0.0.1` and `[::1]` on any port; other origins get no CORS headers.
- **Security**: no authentication in v1; the server binds to loopback by default. How the game authenticates to and hosts the service is an open question (goal-prompt §6) and is not decided here.

### §api-versioning

The normative policy is [versioning.md](../versioning.md): the route-by-route stability review, every version family, what bumps what, open and closed enums, the deprecation window, output stability across releases, worlds, libraries and binaries. In short:

| Family (`/v1/world` `versions` key) | Current | Bumps when |
|---|---|---|
| `contract_version` | **3** (`crates/arda-server/src/contract/mod.rs:31`) | a cell field's meaning, unit or presence changes |
| `tactical_format` | 1 | the `TacticalBlock` or `TacticalScene` envelope changes (the envelope carries the service-added `origin_gs` and `tokens`); also the dungeon, library and prefetch bodies |
| `scene_format` (`scene.format_version`) | 1 | the `arda-scene` schema changes (12) |
| `rules_sidecar_format` (`rules.format_version`) | 2 | the rules sidecar changes (09, 12) |
| `layout_schema` | 1 | `TacticalLayout` changes meaning (the optional `origin` was additive and did not bump it) |
| `bake_format`, `npc_format`, `npc_page_format`, `settlement_format`, `realm_format`, `gazetteer_format`, `schema_index_format` | 1 | their bodies change shape (versioning.md) |
| `columnar_layout_version` | 2 | the binary `ARDACOLS` layout changes |
| `library_catalogue_format`, `sheet_mapping_format` | 1 | the input files this server reads change |

Rules: clients reject a version they do not know. Adding an optional field, a route or a value of an open enum never bumps a version; renaming, removing, re-typing or re-meaning a field, or a new value of a closed enum, does. A change that breaks a route's path, parameters or envelope moves the route to `/v2`, and `/v1` stays served **until both sides (Arda and DiceMaster) agree to drop it** (the DiceMaster contract's rule; it replaces the earlier "one release"), and at least one minor release. `GET /v1/world` lists every family's current version in `versions`, with `arda_version` (the generator that wrote the world) and `server_version` (the build serving it), so the game checks once at start; `index.ts` exports the same numbers. The freeze guard (`src/schema/compat.rs`, baseline `bindings/schema-baseline/v1/`) fails on a breaking schema change that no family bump excuses.

Contract 2 changes: `road` gains the legend value `footpath` and is read from `society/roads.bin` (08 §roads) when present; `built_by` is read from `society/landuse.bin` owners and is a settlement id string or `null` (§api-cell-society); the sea-cell snowline is computed from sea level, not the seafloor (the journal's noted defect). Contract 3 is §api-cell-society.

### §api-routes

Implemented (registered in `crates/arda-server/src/routes.rs` and `src/tactical/routes.rs` with `src/tactical/bake_routes.rs`; bodies and errors in `crates/arda-server/API.md`; stability per route in [versioning.md](../versioning.md)):

| Route | Response (TS type) | Source |
|---|---|---|
| `GET /v1/health` | `Health` `{status, contract_version, seed, server_version}` | — |
| `GET /v1/world` | `WorldInfo` (with `server_version` and `versions`) | 01, §api-versioning |
| `GET /v1/cell/{gx}/{gy}`, `GET /v1/point?x_m=&y_m=` | `CellSample`, `PointSample` | 05, §api-cell-society |
| `GET /v1/area/{ax}/{ay}/cells[?format=json\|bin]`, `/rivers`, `/lakes` | `AreaCells` or `ARDACOLS`, `AreaRivers`, `AreaLakes` | 05 |
| `GET /v1/overview.png[?quality=&style=&labels=]` | PNG; `labels=1` (default 0, plain bytes unchanged) draws the gazetteer's names exactly as the pyramid level of that size's tiles do (long edge 4,096 = level 4), up to quality 4,096; 400 on the oblique look or a world without a gazetteer | 04, 17 §labels |
| `?overlay=society\|political\|none` on `GET /v1/overview.png` and the overview tiles | land use, roads and tier symbols painted into the overview pyramid (implemented); `political` adds realm edge bands and dash-dot borders with ivory roads (§political-map); default `society` when the world has `society/`, never for the oblique look; `society` or `political` without society files or with oblique is 400; render, pyramid and tile cache keys include it | 04 §overlay |
| `GET /v1/tiles/overview/{z}/{x}/{y}.webp\|.png[?oblique=&labels=]` | 256 px tiles | §api-tiles |
| `GET /v1/tiles/relief/{z}/{x}/{y}.webp[?labels=]` | 256 px mid-zoom relief tiles | 17 |
| `GET /v1/settlements[?tier=&realm=]` | `SettlementList` `{format_version, settlements}` | 08 |
| `GET /v1/settlements/{id}` | `SettlementDetail` `{record, society}` | 08, 14 |
| `GET /v1/settlements/{id}/plan` | `TownPlan` | 10 |
| `GET /v1/settlements/{id}/npcs` | `SettlementNpcs` (stored notables) | 13 |
| `GET /v1/npc/{npc_id}[?settlement=]` | `Npc` with its sheet | 13 |
| `GET /v1/npcs?settlement=&building=&job=&realm=&notable=&limit=&cursor=` | `NpcPage` | 13 §npc-queries |
| `GET /v1/buildings/{building}/residents`, `/workers` | `NpcPage` | 13 |
| `POST /v1/npc/population` | `Population` for a posted `PopulationRequest` | 13 |
| `POST /v1/npc/generate` | `Npc`: one new NPC from optional `GenerateNpcRequest` inputs (Arda's own: `seed`, `job`, `office`, `rank`, `ancestry`, `sex`, `tier`, `culture`, `settlement_id`, `notable`), stored nowhere, synthetic ids; body ≤ 16 KiB, JSON only (415); 422 `unsatisfiable` | 13 §npc-generate-one |
| `GET /v1/npc/demo`, `/v1/npc/demo/{npc_id}` | `NpcDemo`, `Npc` of the `arda-npc` sample town (debug) | 13 |
| `GET /v1/gazetteer[?kind=&bbox=&min_rank=]` | `Gazetteer` (§api-gazetteer): natural features then settlement points in one list; `kind` is a comma list of kind keys, `bbox=x0,y0,x1,y1` metres keeps what meets the box, `min_rank=N` keeps rank N or more important; 400 on a bad value, 404 without `society/` or `gazetteer.json` | 08, 15 |
| `GET /v1/realms` | `RealmList`: `{format_version, realms: [RealmSummary]}`, the `realms.json` records without `borders` plus `style`, `rank`, `government_label`, `ruler` when `society.json` exists, and `colour` (`#rrggbb`, the realm's political-map colour, §political-map) | 08, 14 |
| `GET /v1/realms/{id}` | `RealmDetail`: `{record (with borders), politics, relations, events, reigns}`; 404 unknown id, 400 non-numeric | 08, 14 |
| `GET /v1/schema`, `GET /v1/schema/{Name}.json` | `SchemaIndex`, one JSON Schema (§api-schema) | §api-schema |
| `GET /v1/tactical/layouts`, `/layout/{name}`, `/layout/{name}.png`, `/layout/{name}/tiles/{z}/{x}/{y}.webp` | built-in sample layouts (debug) | 11 |
| `POST /v1/tactical/render[?ppsq=&grid=&origin=X,Y]` | PNG of a posted layout (tooling; unstable; body limit) | 11 |
| `GET /v1/tactical/cell/{gx}/{gy}[?ppsq=]` | `TacticalBlock` `{tactical_format, layout_schema, origin, size, render_seed, layout, rules, meta, scene, tiles}` | 09, 10 |
| `GET /v1/tactical/cell/{gx}/{gy}/scene[?time=day\|night]` | `TacticalScene` `{tactical_format, origin_gs, time, scene, tokens}` | 12, 13 |
| `GET /v1/tactical/cell/{gx}/{gy}.png[?ppsq=&grid=&world_grade=]` | PNG | 11 |
| `GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp[?ppsq=]` | 512 px lossless WebP tile | 11, §api-tiles |
| `GET /v1/tactical/window?gx0=&gy0=&w=&h=` or `?gsx=&gsy=&w=&h=` | `TacticalBlock` for a window of 1–3 × 1–3 cells (`gx0`/`gy0`, cells) or 1–192 × 1–192 squares (`gsx`/`gsy`, world squares) | 09 Branches |
| `GET /v1/tactical/window.png?…`, `/window/tiles/{z}/{x}/{y}.webp?…` | PNG and 512 px WebP tiles of the window render; the pyramid the window body's `tiles` describes | 11, §api-tiles |
| `GET /v1/tactical/library` | `TacticalLibraryDto` (the loaded catalogue for layout editors) | 11 |
| `POST /v1/tactical/prefetch` | `202 PrefetchAccepted` for a posted `PrefetchRequest {gx, gy, radius?, ppsq?, demo_overlays?}` | §api-prefetch |
| `POST /v1/tactical/bake` | `202 BakeAccepted {format_version, job_id, state, status_url}` (`200` once finished) for a posted `BakeRequest {gx, gy, ppsq?, world_grade?, layout}` (1.0 task 04) | §api-tactical, §api-prefetch |
| `GET /v1/tactical/bake/{id}` | `BakeStatus {format_version, job_id, state, gx, gy, origin_gs, world_grade, tiles, tile_count, total_bytes, error}` | §api-tactical |
| `GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp` | 512 px lossless WebP tile of the baked set, `Cache-Control: no-store` | §api-tiles |
| `DELETE /v1/tactical/bake/{id}` | `204`; frees the set | §api-cache |
| `GET /v1/tactical/dungeon?seed=\|gx=&gy=[&kind=&w=&h=]`, `/dungeon.png`, `/dungeon/scene` | `DungeonDto`, PNG, `SceneDto` | 18 |

`?demo_overlays=` and `?demo_at=` on the tactical cell, window and prefetch routes are debug options (versioning.md).

Planned, not implemented (additive when they come; not designed here): roads clipped to a box (`GET /v1/roads?bbox=`), a transparent overlay tile pyramid of tier symbols, roads and borders aligned with the overview (goal 41; `?overlay=political` paints borders into the opaque pyramid today), and building lists per settlement (`/v1/settlements/{id}/buildings`; the plan's `buildings` serve today).

### §api-cell-society

When `society/` exists, `/v1/cell` and `/v1/area/.../cells` fill `road` from `roads.bin` (legend `none`, `footpath`, `track`, `road`, `highway`) and `built_by` from `landuse.bin` owners; they add `land_use` (08 §landuse keys) and `realm_id` (from `realms.bin`). The binary `ARDACOLS` layout gains the columns `land_use` (u8), `realm_id` (u32, 0 none) and changes `road` codes to the society raster codes; its layout version becomes 2. Implemented as contract 3 (the shape of `built_by` changed from a number to a string, and two fields were added); a `society/` written before `roads.bin` keeps the stored `road`.

### §api-tactical

A tactical request resolves in this order: validate coordinates → cache lookup (§api-cache) → `refine_block` with town, ways and fields reservations (09 §reservations) → scene (12) with tokens (12 §scene-tokens) → images on demand. The server's `BlockSource` seam returns the whole `RefinedBlock` (layout, rules sidecar, meta), not the layout alone, so scene and image are built from one layout (goal 48). Sea blocks answer `404 no_block` (`BlockSource::check`, asked without composing). Images are rendered from a window with a 6-square apron and cropped (11 §seam-art); walking off an edge loads the neighbour, which joins invisibly (goal 67).

**One composition per apron window** (review 3.5-8). A cell's or window's JSON body and its `/scene` are cut (`Block::cut`, `arda_blocks::crop`) from the very apron block its image is drawn from, so body, scene and image never disagree and a cell is composed once; the composition itself is made over the window grown by 8 squares and cut (09 §window-crop), so every window agrees on its borders. Scenes and the folded rules resolve placements with the world render seed, the image's (review 3.6-4); the dungeon scene with the render seed of its level's name. NPC tokens are placed per building over the whole building as composed and listed by the block they stand in (12 §scene-tokens).

**Bake** (`POST /v1/tactical/bake`, 1.0 task 04, closes OI-44). A client posts an edited cell layout (exactly 64 × 64 squares, `origin = [gx·64, gy·64]`, checked against the library; 422 `invalid_layout` otherwise; windows are not bakeable). The server takes the cell's apron block as the cell image does, splices the posted cell into its centre (`tactical/splice.rs`, the rules of the cut the cell route sends, review 3.6-9: squares inside the cell, walls on every edge of the cell including its four border edges, and placements and lights anchored in it, half-open, from the post; everything else from the apron, item order kept; posted items the cell cannot hold are counted in the job's `ignored`, never dropped silently), renders the apron cropped to the cell with the world render seed, builds the pyramid and encodes every tile. An unedited layout bakes to the cell route's tiles byte for byte; edges join the unbaked neighbours. The job id is the first 32 hex digits of BLAKE3 over the release, the world seed, `library_version` and the canonical request, so a re-post returns the same job and another release another; the bodies carry `generator_version`. The client downloads the tiles, stores them under its own campaign, cell and version, and deletes the job.

### §political-map

The political map is the default look of `arda-settle render` and the `political` overlay of overview exports, `/v1/overview.png` and overview tiles (logic/04 §overlay); the society overlay alone keeps its classic look, so default overview bytes change only by the hamlets (below).

- **One colour per realm.** `render::palette::Palette::of(realms.json)` takes its colours from a hand-curated categorical palette of sixteen (`palette::BASE`, OKLab, farthest-first): navy, lemon yellow, hot pink, periwinkle, umber, orange, turquoise, crimson, charcoal, orchid, steel blue, rose, plum, cerulean, berry, mauve. None is olive or green (each is at least 0.15 OKLab from the Atlas lowland), and every pair is at least 0.12 OKLab apart (about six just-noticeable differences; the first twelve, a default-size world, are the best twelve families, so a world of `n <= 16` realms takes the first `n` and no two share a hue family). Beyond sixteen the set extends deterministically with a deep variant of each (lightness ×0.72), then a pale variant (lightness lifted 45 % to white), then repeats past forty-eight. Realms with the most neighbours choose first, each taking the free colour farthest from its coloured neighbours (plain OKLab distance); pairwise swaps then raise the smallest neighbour difference while any helps (up to 96 realms). Colours are stored as OKLab triples and turned to sRGB with `+ − × ÷`, `sqrt` and a Newton gamma, so they are the same on every platform; `/v1/realms` serves them as `colour`. Realm names on the offline page are drawn in the realm's colour darkened to 0.62 (`text::REALM_INK`) over the usual halo; overview tiles carry no realm names.
- **Edge band.** Each realm's colour as an inner band along its land borders: opacity 1.0 at the border fading as `t^0.85` to nothing at `band_width` pixels (`t = 1 − d / band`, `d` the distance to the nearest smoothed border line), no area fill. The width is `0.65 %` of the image's long edge, between 10 and 90 px (27 px on a 4,096 px world map; `political::band_width`), a function of the whole image or zoom level's size and not of a tile's, so the band at a pixel depends on its position alone (`render::political`) and tiles join. The legend swatch keeps the `11 · s` px scale.
- **Border line.** The `realms.json` borders (cell edges, Chaikin-smoothed) as a dark dash-dot line (`[44, 22, 34]`, `1.7 · s` wide; dash 8, gap 3, dot 1.2, gap 3, times `s`, from each polyline's start) over a pale halo (`[255, 250, 232]` at 0.6, `5.6 · s` wide), drawn over the roads.
- **Roads** (`RoadStyle::Ivory`): ivory fill in a thin dark sepia casing, width by class (highway 6.0/3.8, road 3.2/1.7 casing/fill at `s = 1`); tracks dashed (5.5 on, 3.2 off) and footpaths dotted (1.2 on, 2.8 off) in sepia over a faint pale halo.
- **Hamlets** are left off at world scale (coarser than 30 m/px) with every road built to them (logic/04 §overlay); area exports and relief tiles keep them.
- **Legend** (`render::legend`): lists only what the page draws (no hamlet or track entries at world scale); its road swatches are painted by the map's road painter and the border swatch by `PoliticalLayer::swatch` with the map's constants, two neighbouring realms' colours either side of the line. The title block wraps the realm list to the panel (at most three lines).
- `arda-settle render --style classic` keeps the earlier page (a tint per realm deepening into a ribbon, dashed purple borders, red and yellow roads), with the new colours.

### §api-tiles

- Overview: 256 px lossless WebP (and PNG) slippy tiles cut from one cached Atlas render at `base_px = 256 · 2^max_zoom` (default 4096), `z = 0` the whole world (implemented).
- Tactical: lossless WebP (goal 68), 512 px tiles, one pyramid per block or window (`/cell/{gx}/{gy}/tiles/…` and `/window/tiles/…`, each described by its JSON body's `tiles`). Level `max_zoom` is the full render (a block at 128 px per square is 8192 px, so `max_zoom = 4`); each lower level halves with the compositor's integer premultiplied area average, rounding up; edge tiles are padded transparent.
- `?ppsq=` accepts 64, 96 or 128 (default 128, 11 §ppsq).

### §api-cache

- Every generated body is cached in a byte-bounded LRU keyed by `RenderKey = BLAKE3(world identity, society inputs hash, generator versions, library_version, route, coordinates, options)`. World identity is the seed plus the BLAKE3 of `world.json`; generator versions are the crate versions of `arda-refine`, `arda-town`, the ways and fields crates, `arda-scene` and `arda-npc`. This realises goal 67's "cached by (seed, coordinates, catalogue version)" and also invalidates on a generator change.
- `ETag` is 32 hex digits of a key over the inputs (quoted, `crates/arda-server/src/etag.rs`, review 3.6-11): the server version (every generator crate shares it), the world identity (seed, BLAKE3 of `world.json` and of every file in `society/`), `library_version`, the route and the request's path and query (a built-in layout's image and a posted layout key on the render key instead, so they share a tag). `If-None-Match` answers `304` before anything is composed or rendered, cold or warm; it never hashes the bytes. Tactical bodies and images (cells, windows, `/scene`, dungeons, built-in layouts, posted renders), overview and relief images carry `Cache-Control: no-cache` and are revalidated by `ETag`: their bytes depend on the served world and library, which the URL does not name (review round 2 #44; `public, max-age=3600` before).
- Budgets (defaults; flags only for areas and fine windows, [07-operations](../07-operations.md) Infrastructure): decoded areas 384 MiB (`--area-cache-mib`), derived coast/river/lake layers 128 MiB, fine windows 256 MiB (`--fine-cache-mib`); overview PNGs 256 MiB and overview tiles 256 MiB; relief tiles 128 MiB; tactical renders 768 MiB, tile pyramids 1 GiB, encoded PNGs, encoded tiles and JSON bodies 160 MiB each; composed tactical blocks 256 MiB, charged with their per-square `ext` maps; finished bakes `--bake-result-mib` (384 MiB). The query and encoded budgets were 1 GiB, 256 MiB, 512 MiB and 256 MiB before the serving-concurrency fix (review 3.6-8), trimmed to admit the tactical pipeline. At start the worst case of all budgets plus one transient request per worker, plus the pipeline's fixed bounds (2 concurrent compositions at 192 MiB, 256 refined blocks at 512 KiB, two refine-source generations of at most 6 decoded areas each with their fine-height memo, 64 town plans at 4 MiB), must fit 16 GiB; once the world and library are open, their measured residents (decoded library and scaled textures, `society/` files at twice their size, the shore layer three times, a town plan above 4 MiB) are added and the server refuses to start above the ceiling (goal-prompt §8). The refine source reopens the world after 6 distinct areas, so decoded areas never accumulate; query and relief share one base `World`.
- Every tactical build is single-flight per key (`crates/arda-server/src/flight.rs`): a full-resolution render, pyramid, PNG, tile, JSON body or block composition runs once per key at a time and concurrent requests for the same key wait for it (review 3.6-1). Requests are cancellable: a request whose client disconnects leaves the request lane, the encode slot or another build's queue and starts no further stage; a stage already running finishes and caches its result, so bytes stay deterministic (review 3.6-2). A tile pyramid shares its render as its deepest level and the render cache then lets the render go, so a render is held once; a PNG of the same key reads it from the pyramid (review 3.6-5). Compositions refine their cells and run their three overlay layers on the shared, bounded rayon pool, never a thread per request (review 3.6-12).
- Bakes bypass every shared cache: `POST /v1/tactical/bake` reads and writes no render, pyramid, PNG or tile entry, and keeps only the encoded tiles of finished jobs, in memory, within `--bake-result-mib` (default 384 MiB, oldest finished set evicted first, no time-to-live) until `DELETE`. Admission adds, when baking is on, one transient render with its pyramid, the parsed bodies of the queued jobs and the result budget (about 1.9 GiB with defaults, 15.9 GiB in all), and refuses a result budget below one worst-case 128-ppsq set (341 tiles, about 362 MiB).

### §api-prefetch

`POST /v1/tactical/prefetch` enqueues a cell's 8 neighbours at low priority (`radius` 1, or 2 for the 24 cells of two rings), nearest ring first and cardinal neighbours before diagonals, which also take the image slots first. The queue is bounded (64 cells) and deduplicated; when full, its oldest waiting cells give way to the fresh ones (review 3.6-5). Prefetching never changes any response's bytes. Prefetch stops at sea cells and the world edge (goal 67). A prefetch or bake job that panics is caught: prefetch drops it, bake marks it `failed` with `internal`, and the worker carries on (review 3.6-7).

The bake lane is prefetch's sibling: its own queue (at most 8 waiting jobs, a full queue answers 503 `busy`), one worker (`--bake-workers`, 0 or 1; 0 answers 501), its own render lock beside the request and prefetch lanes, and tile encodes taken one at a time through the shared encode slot, so a bake never holds an interactive request for more than one tile encode and never changes any other response's bytes.

### §api-errors

Every error is `{"error": {"code", "status", "message"}}`; clients match on `code`:

| Status | `code` | When |
|---|---|---|
| 400 | `bad_request` | unparsable path or query value, unknown option |
| 400 | `out_of_range` | a cell, area, square or point outside the world |
| 404 | `not_found` | unknown route, id, tile outside the pyramid, missing layer or society file |
| 404 | `no_block` | a cell with no land square (open sea) |
| 413 | `payload_too_large` | a window, render or posted body above the limits |
| 415 | `unsupported_media_type` | a `POST` body without `Content-Type: application/json` |
| 422 | `invalid_layout` | a layout or sidecar inconsistent with the library (11, 12) |
| 422 | `unsatisfiable` | `POST /v1/npc/generate` inputs that are valid but no inhabitant matches (13 §npc-generate-one) |
| 500 | `resource_limit` | an allocation or budget refused the work |
| 500 | `sheet_mapping` | `--sheet-mapping` could not reshape a served NPC (§api-sheet-mapping) |
| 501 | `not_implemented` | the `PendingBlocks` test seam (debug; the body adds `planned_source`). Also `POST /v1/tactical/bake` with `--bake-workers 0` (no `planned_source`) |
| 503 | `busy` | a bounded background queue is full (`POST /v1/tactical/bake` with 8 jobs waiting); retry later |
| 500 | `internal` | anything else |

### §api-bindings

- TypeScript types are generated from the Rust DTOs with ts-rs into `bindings/ts/arda/` at the workspace root, committed, re-exported by `index.ts` together with `API_VERSION` and every family version (goal 66). `cargo test -p arda-server bindings` fails when the committed files are stale; `ARDA_BLESS_BINDINGS=1` regenerates them.
- Domain crates (`arda-npc`, `arda-scene`, `arda-tactical`, `arda-town`) stay free of ts-rs. The server owns DTO mirrors with `From` conversions, and a test per mirror asserts that the mirror's JSON equals the domain type's serde JSON for a fixture (so the game sees exactly the domain schema, goal 69), except where §api-conventions turns u64 into strings.
- NPC sheets (13 §npc-game-schema) and scenes are exported under the names the game uses: `Npc`, `Sheet`, `SceneDto` (with `Runs<T>` for the `[count, value]` grids, `SqDto` for `[x, y]` squares and string seeds), `TacticalBlockDto` (whose `scene` is `SceneDto | null`), `TacticalScene`, `Token`.

### §api-schema

- Every public body has a JSON Schema (draft 2020-12), generated by `schemars` from the same Rust types as the TypeScript bindings: the server DTOs and mirrors, and the domain records `arda_settle::model::Settlement`, `arda_society::SettlementSociety` and `arda_town::TownPlan` through each crate's optional `schema` feature (the domain crates stay free of ts-rs). A schema is named after its TypeScript type.
- The files are committed in `bindings/schema/` (`<Name>.json` plus `index.json`) and served byte for byte at `GET /v1/schema` and `GET /v1/schema/{Name}.json` (`application/schema+json`; unknown names 404). The index has its own schema, `SchemaIndex`; the dungeon body is `DungeonDto` (a mirror of `arda_dungeon::Dungeon`, seed a decimal string) and `/v1/tactical/dungeon/scene` is a bare `SceneDto`. Rendering is deterministic; the staleness test and `ARDA_BLESS_BINDINGS=1` cover them with the TS bindings.
- Real responses of every route on the MICRO fixture validate against the served schemas (`tests/world_api/schemas.rs`).
- The contract kit (`contract-kit/`, 06-testing § Contract kit) records the exact bodies of every stable route on the canonical MICRO-42 world, validated against these schemas, for clients to replay and to check a live server with `contract-kit/check.mjs`.
- The index names each schema's version `family` (a `versions` key, or `api_version`) and its `stability` (`stable`, `unstable`, `debug`; versioning.md). The freeze guard compares the schemas with the committed baseline `bindings/schema-baseline/v1/` and fails on a breaking change no family bump excuses; `ARDA_BLESS_SCHEMA_BASELINE=1` re-baselines (06-testing).

### §api-sheet-mapping

- `--sheet-mapping <file.json>` (goal 69) reshapes every `Npc` the server sends (`/v1/npc*`, `/v1/npcs`, the building routes, settlement notables, the demo and posted populations) through a declarative mapping: JSON Pointer `to`/`from`, `const`, `template`, `fields` sub-rules over arrays and objects with `where` filters, `append`, `optional`/`default`, and conversions `table`, `keys`, `scale`, `format`, `case`, `replace`, `join`, `split`, `to_string`, `to_number`, `each`. `base` is `copy` (then `drop`) or `empty`.
- The mapping is checked when the server starts (unknown keys, pointers, tables, one source per rule, name) and dry-run over probe NPCs (the `arda-npc` market town's notables and every 37th inhabitant); any failure refuses startup, before the world loads.
- Mapped bodies carry `X-Arda-Sheet-Mapping: <name>`; without a mapping every body is byte-identical to an unmapped server. Envelopes, refs and every other route are unchanged; scene tokens keep resolving by `npc_id`. A runtime failure (a strict table missing a value no probe had) is 500 `sheet_mapping`.
- Examples: `crates/arda-server/mappings/identity.json` and `5e-srd-monster.json` (the open 5e SRD JSON monster layout, written from public field conventions only).

### §api-gm-path

The GM tool path (goal 70) is a sequence of these routes, exercised by the developer viewer (feat/viewer) and the end-to-end test: `/v1/world` → overview and overlay tiles to browse → `/v1/settlements?bbox=` or `/v1/cell` to pick a place → `/v1/tactical/cell/{gx}/{gy}` and its tiles for the map → `/scene` for rules and tokens → `/v1/npc/{id}` for any token.

## Steps

1. Parse and validate the request; map errors per §api-errors.
2. Compute the `RenderKey`; answer from cache or `304` when possible.
3. Otherwise build the body from the owning crates (routes table), store it, answer.
4. For tactical cells, enqueue prefetch (§api-prefetch).

## Branches

- No `society/`: settlement, NPC and overlay routes answer `404 not_found` naming the file; tactical cells are natural only (09 Branches).
- A block source that cannot build blocks (the `PendingBlocks` test seam, never installed by the default server): `501 not_implemented` with `planned_source`.

## Unhappy paths

Listed in §api-errors. A panic in a handler is a bug: it is caught at the task boundary, answered `500 internal`, and never poisons the caches (entries are inserted only after a body is complete).

## State transitions

In memory only: cache entries appear and are evicted; the prefetch queue fills and drains. The server never writes to the world directory.

## Invariants

1. Determinism: two identical requests give identical bytes and `ETag`s, cold and warm [65, 67].
2. Every route answers against a MICRO seed-42 world with `society/`, with a typed error for every invalid input (tests in-process, no mocks of Arda) [65].
3. Committed TS bindings are fresh; each DTO mirror serialises like its domain type [66, 69].
4. Tiles: every tile inside the pyramid decodes (PNG or WebP) to its documented size; tiles outside answer 404 [68].
5. Seams: the east column of tactical tile pixels of block `(gx, gy)` and the west column of block `(gx+1, gy)` at full zoom differ by no more than the WebP lossless round trip (zero) from the same columns of a 2 × 1 window render [67]; a cell's whole image equals its part of the window image, plain and with the world grade (`tests/world_api/seams.rs`), and a window's body is its cells' bodies side by side (`one_composition.rs`).
6. Budgets: the admitted worst case never exceeds 16 GiB; the server refuses to start rather than trim [goal-prompt §8].
7. Performance (release, 1.0 task 09 gates `crates/arda-server/tests/world_api/gates.rs`, best of 3, society fixture, run by `tools/release-gates.sh`): a cached cell tile in < 20 ms (measured 0.8 ms); a cold block layout plus scene in < 500 ms (159 ms); a cold quarter-block window render at 128 px per square in < 1 s (502 ms); also `AppState::open` < 2 s (1.19 s), `World::load` < 100 ms, area cells < 500 ms JSON and < 250 ms bin (160 and 109 ms cold), a warm overview tile < 20 ms. Ratified as stated; figures from a shared 32-thread machine at load 12-14 (07-operations "Recorded measurements").
8. Tokens: every token of every scene resolves through `/v1/npc/{id}` [57, 69].

## Outcomes & side effects

HTTP responses; stdout request logging at the binary boundary only (standards: no logging framework in library crates). No files are written.

## Dimensions not in play

- Authentication, multi-tenant hosting and rate limiting (open question).
- Writes: the API is read-only; a GM's edits live in the game.
- Streaming or websockets; animation.
