# arda-server HTTP API (v1)

`arda-server` serves one stored arda world read-only over HTTP. The TypeScript game and the
tactical stages read the world through the **cell contract** described here. It also serves
tactical battle maps (layout JSON, PNG renders and a WebP tile pyramid) under
[`/v1/tactical`](#tactical-maps).

```sh
cargo build --release -p arda-server
target/release/arda-server --world out/micro42 --port 8787 --library assets/tactical/placeholder
```

| Flag | Default | Meaning |
|---|---|---|
| `--world <dir>` | required | World directory (holds `world.json`) |
| `--port` | `8787` | TCP port |
| `--host` | `127.0.0.1` | Bind address |
| `--area-cache-mib` | `1024` | Decoded-area cache budget |
| `--fine-cache-mib` | `512` | Fine-terrain window cache budget |
| `--max-overview-px` | `8192` | Largest `overview.png` long edge clients may request |
| `--tile-base-px` | `4096` | Tile-pyramid base edge; `256 · 2^max_zoom`, at least 512 |
| `--library <dir>` | `assets/tactical/placeholder` | Tactical asset library (holds `catalog.json`), relative to the working directory. It is loaded once and validated with the arda-tactical loader; any validation issue stops startup |
| `--prefetch-workers` | `2` | Background threads warming prefetched tactical cells (at most 8; `0` disables prefetch). Each worker is admitted against the 16 GiB ceiling as one more transient render |

## Conventions

- **Versioning.** Every route lives under `/v1`. Cell-bearing bodies carry
  `contract_version` (currently **2**). Clients must reject a version they don't know. A
  change of meaning, unit or presence of any field bumps it.
- **Determinism.** The same request against the same world always returns identical bytes,
  whether it's served cold or from cache.
- **Coordinates.** x runs east and y runs south from the modeled world's north-west corner.
  - Global cell `(gx, gy)` covers `[gx·100, gx·100 + 100) × [gy·100, gy·100 + 100)` m, and its
    centre is `(gx·100 + 50, gy·100 + 50)` (vocabulary I1, contract 2). A point `(x, y)` lies in
    cell `(floor(x/100), floor(y/100))`.
  - The world's stored per-cell values (height, climate, hydrology) are point samples of the
    fine terrain, and they belong to the cell's centre. The fine lattice is read in that frame:
    lattice node `(0, 0)` lies at the centre of cell `(0, 0)`, `(50, 50)` m
    (`arda_core::FINE_FRAME_OFFSET_UM`), so `fine.centre_m` is the stored cell's own height and
    `/v1/point`, `/v1/cell` and the tactical blocks all read one surface (I1, integration phase D).
  - Area `(ax, ay)` holds global cells `ax·512 … ax·512+511` (and likewise for y).
  - Local `(cx, cy) = (gx mod 512, gy mod 512)`.
  - Valid global cells are `0 ≤ gx < areas_wide·512` and `0 ≤ gy < areas_high·512`.
- **Big integers.** u64 identities (`seed`, reach and basin ids) are **decimal strings**,
  because they don't fit a JS number.
- **CORS.** `GET`, `HEAD`, `POST` and `OPTIONS`, with the `Content-Type` and
  `If-None-Match` request headers, are allowed from `http(s)://localhost`, `127.0.0.1` and
  `[::1]` on any port. `ETag` and `X-Arda-Cache` are exposed. Other origins get no CORS
  headers.
- **Connections.** A client must send a complete request head within
  `--header-timeout-s` (10 s), including between keep-alive requests, or it is
  disconnected. At most `--max-connections` (64) are served at once; later
  clients wait in the listen backlog until one closes.
- **Caching.** Caches are LRU with byte budgets: decoded areas, derived coast/river/lake
  layers, fine windows, overview PNGs and tiles.
  - At startup the server admits the configuration only if the worst case (full caches, plus
    the decoded pyramid base, plus `area_builds` (2) concurrent area builds, plus one
    buffered response body per allowed connection) fits the 16 GiB ceiling (goal-prompt §8).
    Otherwise it refuses to start. Area requests beyond `area_builds` wait for a slot.
  - PNG responses carry `Cache-Control: public, max-age=3600`.
  - The tactical caches (renders, pyramids, encoded PNGs, encoded tiles) are admitted the
    same way, together with one transient render at the pixel limit.

## Errors

Every error is JSON with the right HTTP status:

```json
{"error":{"code":"out_of_range","status":400,"message":"cell 5000,1 is outside the world (1023,2047 is the last)"}}
```

| Status | `code` | When |
|---|---|---|
| 400 | `bad_request` | Unparsable path or query value, missing parameter, unknown `format` or `style`, `quality` outside 512 px…limit |
| 400 | `out_of_range` | Cell, area or point outside the world |
| 400 | `bad_request` | Malformed JSON body, bad `world_seed`, or NPC inputs the generator refuses (foreign or duplicate buildings, too little housing) |
| 404 | `not_found` | Unknown route, tile outside the pyramid, missing layer file, unknown NPC id |
| 413 | `payload_too_large` | A request body above 2 MiB, a population above 50,000 or more than 20,000 buildings |
| 400 | `bad_request` | Tactical: `ppsq` not 64, 96 or 128 (world images also accept 16 and 32), `grid` or `demo_overlays` not 0 or 1, a bad `demo_at`, a missing `/window` parameter, a posted body that isn't a `TacticalLayout` (malformed JSON, unknown field) |
| 404 | `not_found` | Unknown route, tile outside the pyramid, missing layer file, unknown tactical layout |
| 404 | `no_block` | Tactical: an open-sea cell with no land square (logic/16 §api-errors) |
| 413 | `payload_too_large` | Tactical: posted body over 4 MiB, layout over 41 616 squares (a 3 × 3-cell window plus its apron), render over 96 Mi pixels, a `/window` side over 3 cells, more than 4 placements a square (+64), or free lights summing to more than 16× the render's pixels |
| 415 | `unsupported_media_type` | Tactical: `POST /render` without `Content-Type: application/json` |
| 422 | `invalid_layout` | Tactical: layout inconsistent with itself or the loaded library (square count, ground without texture, unknown kit or asset, forbidden rotation or mirror), a name over 128 bytes or with control characters, a placement or light more than 16 squares off the map) |
| 500 | `resource_limit` | An allocation or window limit refused the work |
| 500 | `internal` | Anything else |
| 501 | `not_implemented` | `/v1/tactical/cell/{gx}/{gy}` served by the `PendingBlocks` source (body adds `planned_source`); the default server uses `RefineBlocks` |

The `message` is for humans and isn't stable. Match on `code`. TS type: `ApiError`; the 501
body is `NotYetError`, which adds `planned_source`.

## Endpoints

### `GET /v1/health`

```sh
curl http://127.0.0.1:8787/v1/health
```

```json
{"status":"ok","contract_version":2,"seed":"42"}
```

### `GET /v1/world`

Returns the seed, the size and grids from the manifest, and the tile pyramid. TS type: `WorldInfo`.

```sh
curl http://127.0.0.1:8787/v1/world
```

```json
{"contract_version":2,"api_version":"v1","seed":"42","arda_version":"0.4.0","format_version":4,
 "size_km":{"width":102,"height":204},"latitude":{"south_deg":35,"north_deg":55},
 "areas_wide":2,"areas_high":4,"area_cells":512,"cell_size_m":100,"cells_wide":1024,"cells_high":2048,
 "fine_terrain":{"recipe_version":6,"spacing_m":39.0625},
 "tiles":{"tile_px":256,"max_zoom":4,"base_px":4096,"image_width_px":2048,"image_height_px":4096,"relief_max_zoom":12}}
```

### `GET /v1/cell/{gx}/{gy}`

Returns one `CellSample` (see [Cell contract](#cell-contract)). `gx` and `gy` are u32.

```sh
curl http://127.0.0.1:8787/v1/cell/512/1036
```

```json
{"contract_version":2,"gx":512,"gy":1036,"ax":1,"ay":2,"cx":0,"cy":12,"x_m":51200.0,"y_m":103600.0,"centre_x_m":51250.0,"centre_y_m":103650.0,
 "height_m":78.586,"terrain":"land","cover":"forest","slope_deg":0.0,"aspect_deg":0.0,"temperature_c":11.56,"rainfall_mm":2227.0,"moisture":1.0,"wetness":1.0,"forest_density":0.7529411764705882,"drainage_area_km2":65.95,"discharge_m3s":2.865,"watercourse_order":4,"watercourse_width_m":6.7,"height_above_river_m":0.0,
 "road":"none","built_by":null,
 "coast":{"is_coast":false,"distance_m":4517.7427992306075},
 "snow":{"fraction":0.0,"peak_fraction":0.0,"perennial":false,"snowline_m":2703.201384615385},
 "river":{"segment_id":210,"global_reach_id":"35596688953344","order":4,"width_m":6.7,"discharge_m3s":2.865},
 "lake":null,
 "fine":{"centre_m":96.024,"min_m":72.452,"max_m":136.902}}
```

This is the real response from the seed-42 MICRO world.

### `GET /v1/point?x_m=&y_m=`

Returns the height at arbitrary world metres, plus the `CellSample` of the cell that contains
the point. TS type: `PointSample`.

```sh
curl 'http://127.0.0.1:8787/v1/point?x_m=51234.5&y_m=103617.25'
```

- Requires `0 ≤ x_m < cells_wide·100` and likewise for `y_m`. Both must be finite decimals.
- On fine worlds, `height_m` is the fine terrain's integer bilinear interpolation, rounded
  once to the millimetre. It is bit-identical to `TerrainFileReader::sample` at the lattice
  point `round(x_m·10⁶) − 50·10⁶` µm (and likewise for y; the I1 frame above), and
  `height_source` is `"fine"`. The lattice covers the first cell's centre to the last cell's
  centre; points in the outer half cells read the surface clamped to that edge.
- On older worlds, it is bilinear over the four surrounding cell centres (the outermost half
  cell holds the edge value), and `height_source` is `"cells"`.

```json
{"contract_version":2,"x_m":51234.5,"y_m":103617.25,"height_m":78.931,"height_source":"fine","cell":{"gx":512,"gy":1036,"...":"..."}}
```

### `GET /v1/area/{ax}/{ay}/cells[?format=json|bin]`

Returns all 512 × 512 samples of an area in columnar form, row-major
(`index = cy·512 + cx`). Each column is exactly the matching `CellSample` field of that cell,
in **f32** precision. Every `/cell` in the area agrees with it.

```sh
curl -o cells.json http://127.0.0.1:8787/v1/area/1/2/cells
curl -o cells.bin 'http://127.0.0.1:8787/v1/area/1/2/cells?format=bin'
```

**JSON** (default, `application/json`, about 38 MB). TS type: `AreaCells`.

```json
{"contract_version":2,"ax":1,"ay":2,"gx0":512,"gy0":1024,"width":512,"height":512,
 "legend":{"terrain":["sea","land","lake"],"cover":["bare","grass","scrub","forest","marsh","rock","ice"],
           "road":["none","track","road","highway"]},
 "columns":{"height_m":[...262144 numbers...],"terrain":[...],"coast_distance_m":[0.0,null,...], "...":[]}}
```

**Binary** (`?format=bin`, `application/octet-stream`, about 26 MB): `ARDACOLS` layout
version 1. All numbers are little-endian.

| Offset | Type | Field |
|---|---|---|
| 0 | `[u8; 8]` | magic `ARDACOLS` |
| 8 | u32 | layout version (1) |
| 12 | u32 | contract version |
| 16 | i32 | ax |
| 20 | i32 | ay |
| 24 | u32 | gx0 |
| 28 | u32 | gy0 |
| 32 | u32 | width (512) |
| 36 | u32 | height (512) |
| 40 | u32 | column count *N* |
| 44 | u32 | data offset *D* (a multiple of 4) |
| 48 | table | *N* × { u8 name length, name (ASCII), u8 dtype } |
| … | zero padding | up to *D* |
| *D* | data | columns in table order, each `width·height` elements, no gaps |

- **Dtypes.** `1` is u8, `2` is u32, and `3` is f32 (`NaN` means `null`).
- **Alignment.** Every column starts 4-byte aligned, so JS can wrap f32 and u32 columns in
  `Float32Array` and `Uint32Array` views directly.

Columns, in wire order:

| Column | dtype | Contract field |
|---|---|---|
| `height_m` | f32 | `height_m` |
| `terrain` | u8 | `terrain`, as a legend code (0 sea, 1 land, 2 lake) |
| `cover` | u8 | `cover`, as a legend code |
| `slope_deg` | f32 | `slope_deg` |
| `aspect_deg` | f32 | `aspect_deg` |
| `temperature_c` | f32 | `temperature_c` |
| `rainfall_mm` | f32 | `rainfall_mm` |
| `moisture` | f32 | `moisture` |
| `wetness` | f32 | `wetness` |
| `forest_density` | f32 | `forest_density` |
| `drainage_area_km2` | f32 | `drainage_area_km2` |
| `discharge_m3s` | f32 | `discharge_m3s` |
| `watercourse_order` | u8 | `watercourse_order` |
| `watercourse_width_m` | f32 | `watercourse_width_m` |
| `height_above_river_m` | f32 | `height_above_river_m` |
| `road` | u8 | `road`, as a legend code |
| `built_by` | u32 | `built_by`, 0 for none |
| `is_coast` | u8 | `coast.is_coast` |
| `coast_distance_m` | f32 | `coast.distance_m` (NaN is null) |
| `snow_fraction` | f32 | `snow.fraction` |
| `snow_peak_fraction` | f32 | `snow.peak_fraction` (NaN is null) |
| `snow_perennial` | u8 | `snow.perennial` |
| `snowline_m` | f32 | `snow.snowline_m` |
| `river_segment` | u32 | `river.segment_id`, 0 for none |
| `lake_id` | u32 | `lake.lake_id`, 0 for none |
| `lake_depth_m` | f32 | `lake.depth_m` (NaN is null) |
| `fine_centre_m` | f32 | `fine.centre_m` (NaN is null) |
| `fine_min_m` | f32 | `fine.min_m` (NaN is null) |
| `fine_max_m` | f32 | `fine.max_m` (NaN is null) |

To resolve `river_segment` and `lake_id` values, use the two endpoints below.

### `GET /v1/area/{ax}/{ay}/rivers`

Lists the saved river segments of the area, in stored order. Cells are global `[gx, gy]`
pairs. TS type: `AreaRivers`.

```sh
curl http://127.0.0.1:8787/v1/area/1/1/rivers
```

```json
{"contract_version":2,"ax":1,"ay":1,"rivers":[
 {"id":1,"global_id":"21406117007680","order":1,"width_m":0.9,"discharge_m3s":0.055,"feeds":null,"ends":"sea","course":[[552,623]]}]}
```

`ends` is one of `junction`, `sea`, `lake`, `off_tile`, `basin` or `divergence`.

### `GET /v1/area/{ax}/{ay}/lakes`

Lists the saved lakes of the area. TS type: `AreaLakes`. The seed-42 MICRO world has no
lakes, so the example below shows the shape with illustrative values.

```sh
curl http://127.0.0.1:8787/v1/area/0/0/lakes
```

```json
{"contract_version":2,"ax":0,"ay":0,"lakes":[
 {"id":1,"global_id":"123","surface_m":412.5,"max_depth_m":8.25,"outlet":[100,200],"cells":[[100,201],[101,201]]}]}
```

### `GET /v1/overview.png[?quality=&style=]`

Returns the whole-world overview as PNG. It's the same bytes as
`arda export --overview --quality <q> --style <style>`.

- `quality` is the long edge in pixels, or `NK` where 1K is 1024 pixels. The range is
  512 to `--max-overview-px`, and the default is `2048`.
- `style` is `atlas` (the default), `classic` or `atlas-oblique`. `atlas-oblique` (goal 24,
  opt-in, recipe-5+ worlds only) is the Atlas render seen slightly obliquely: ground moves
  north by half its height (coasts and sea stay exactly put), with gentle aerial
  perspective, valley occlusion and sky light. It is `arda export --style atlas-oblique`.

Renders are cached per `(quality, style)`, and only one render runs at a time.

```sh
curl -o overview.png 'http://127.0.0.1:8787/v1/overview.png?quality=2K&style=atlas'
```

### `GET /v1/tiles/overview/{z}/{x}/{y}.webp` and `.png`

Returns a 256 px RGBA slippy tile as **lossless WebP** (goal 68; encoded by the pure-Rust
`image-webp`) or as PNG, which stays served for older clients. Both encode the same
pixels, are cached separately per format and carry `Cache-Control: public, max-age=3600`. The tiles are cut from one cached Atlas render at
`quality = base_px`, where `base_px = 256·2^max_zoom` and defaults to 4096, so `max_zoom`
is 4.

- The overview image sits at the top-left of a `base_px` square. The rest of the square is
  transparent.
- Zoom `z` has `2^z × 2^z` tiles, and `z = 0` is the whole world in one tile.
- Each tile pixel is the integer box-filter mean of its `2^(max_zoom−z)`-square source
  block. Alpha is the fraction of that block the image covers.
- `/v1/world` → `tiles` gives the pyramid geometry, and `image_width_px`/`image_height_px`
  show where the content ends.
- `z > max_zoom`, `x ≥ 2^z`, `y ≥ 2^z` and a suffix other than `.webp` or `.png` all
  return **404 `not_found`**.
- `?oblique=1` (goal 24, opt-in) cuts the tile from the `atlas-oblique` render instead;
  `?oblique=0` or no parameter is the default pyramid, byte for byte. The relief levels
  past `max_zoom` are always top-down, so a client showing the oblique pyramid should not
  mix in relief tiles (ground would jump by up to half its height at the switch).
```sh
curl -o tile.webp http://127.0.0.1:8787/v1/tiles/overview/2/1/3.webp
curl -o tile.png http://127.0.0.1:8787/v1/tiles/overview/2/1/3.png
```

### `GET /v1/tiles/relief/{z}/{x}/{y}.webp`

Returns a 256 px lossless WebP tile of the **mid-zoom relief** levels, which continue the
overview pyramid past its native zoom (`max_zoom + 1 ..= tiles.relief_max_zoom`, eight
levels). Level `z` has `longest_side / (256·2^z)` metres per pixel, where the longest side
is `max(areas_wide, areas_high) × 51.2 km`. At the default 4096 px base on MICRO, z = 5 is
25 m/px and z = 9 is 1.5625 m/px, about one pixel per tactical square.

- Heights come from `arda-midzoom`: the stored 39.0625 m field refined on demand to
  9.765625 m (19.53 m at z where pixels are coarser than 16 m) with drainage-aligned
  gullies and ribs. Every stored 39 m cell mean stays where it is.
- Pixels use the formed Atlas palette and light, so the look matches the overview at the
  switch-over. Rivers, lakes, coasts and marsh pools come from the tactical layer's own water
  geometry (`arda_refine::region`), so at z = 9 every water pixel is a water square of
  `/v1/tactical/cell` and the reverse (logic/17 §water).
- Each pixel is a pure function of its global position: tiles are deterministic, cached
  (128 MiB by default) and join pixel-exactly. Pixels outside the world are transparent.
- A cold tile renders in about 10–50 ms.
- `z ≤ max_zoom`, `z > relief_max_zoom`, `x ≥ 2^z`, `y ≥ 2^z`, a non-`.webp` suffix or a
  world without formed (recipe-5) fine terrain return **404 `not_found`**.
```sh
curl -o relief.webp http://127.0.0.1:8787/v1/tiles/relief/7/55/95.webp
```


## Settlements and world NPCs

Served when the world has a `society/` directory (`arda settle --world`, then
`arda society build --world`); otherwise every route here is 404 `not_found`. Ids are
decimal strings (I5). Bodies are the domain types' JSON; TS mirrors other than `Npc` are
pending (A12).

- `GET /v1/settlements[?tier=&realm=]`: `{format_version, settlements: [record…]}`, the
  `arda-settle` records (logic/08), filtered by tier and realm id.
- `GET /v1/settlements/{id}`: `{record, society}`; `society` is the settlement's
  `arda-society` record (founding, buildings, economy, factions, offices, hooks) or null.
- `GET /v1/settlements/{id}/plan`: its `arda-town` `TownPlan` in world metres (404 when the
  planner refused the site). The plan is drawn on arda-refine's rivers; `bridges` lists its
  decks as `{street, along_x, rows: [[across, from, to]]}` in global squares.
- `GET /v1/settlements/{id}/npcs`: `{settlement_id, npcs}`, its stored notables (`Npc`).
- `GET /v1/npc/{npc_id}[?settlement=]`: any inhabitant of the world with its sheet (`Npc`),
  see [People](#people).

```sh
curl -s "http://localhost:8787/v1/settlements?tier=village" | jq '.settlements | length'
curl -s http://localhost:8787/v1/settlements/118/npcs | jq '.npcs[].job.title'
```

### People

Goals 51 and 57 (logic/13 §npc-id, §npc-queries). Only notables are stored (goal 56);
everyone else is regenerated on request from (world seed, settlement, home building,
resident index) and is the same person every time. No route materialises commoners
world-wide: a query builds at most 16 settlement skeletons per page (a few integers per
inhabitant, dropped after the page) and expands only the people it returns.

**Names.** A person's **reference** is `"<settlement>.<building>.<index>"` in decimal: the
`index`-th resident of home building `building`. Their `NpcId` (the decimal u64 in `Npc.id`
and in scene tokens) is `NpcId::from_parts(settlement, building, index)`, a hash, so it can't
be decoded without the settlement. A **building** is `"<settlement>.<building>"`, because
building ids are unique only within their settlement.

- `GET /v1/npc/{id}[?settlement=]`: `id` is a reference (anyone, commoners included) or a
  u64 id. A u64 id resolves on its own for a stored notable; for a commoner add
  `?settlement=`. A notable resolves to its stored copy, whichever form names it. Nobody by
  that name is **404**; a malformed id, or a reference outside `?settlement=`, is **400**.
- `GET /v1/npcs?settlement=&building=&job=&realm=&notable=&limit=&cursor=`: a page of people,
  TS `NpcPage` `{format_version: 1, npcs: [NpcEntry], next_cursor, scanned_settlements}`, where
  `NpcEntry` is `{npc_ref, settlement_id, notable, npc: Npc}`. Every filter is optional:
  - `settlement`: one settlement (404 if unknown); otherwise every settlement in id order;
  - `building`: `<settlement>.<building>` (or a bare id with `settlement`): people who live or
    work there; 404 if the settlement has no such building;
  - `job`: an occupation key (`farmer`, `smith`, `innkeeper`, …, `Npc.job.key`);
  - `realm`: settlements of this realm id;
  - `notable`: `true` or `false`;
  - `limit`: 1–200, default 50;
  - `cursor`: the previous page's `next_cursor` (`null` on the last page). A page ends when
    it holds `limit` people or has scanned 16 settlements, so a page may be short, even
    empty, with a cursor.

  Order is settlement id, then home building id, then resident index. Pages are
  deterministic, and following the cursors visits every match exactly once. Unknown
  parameters and malformed values are **400**.
- `GET /v1/buildings/{building}/residents[?settlement=&limit=&cursor=]` and `/workers`: the
  people who live, or work, in a building, as an `NpcPage`. `building` is
  `<settlement>.<building>`, or the bare id with `?settlement=`.

```sh
B=http://localhost:8787/v1
curl -s "$B/npcs?settlement=118&notable=false&limit=2" | jq '.npcs[] | {npc_ref, name: .npc.name.given, job: .npc.job.key}'
curl -s "$B/npc/118.5.2" | jq '.job, .sheet.hit_points'
curl -s "$B/npcs?realm=1&job=smith&limit=20" | jq '.next_cursor'
curl -s "$B/buildings/118.5/residents" | jq '[.npcs[].npc_ref]'
```

## Tactical maps

Battle maps on the 5-ft grid (goals 48, 65–68), built from the arda-tactical compositor and
the `--library` catalogue, with lighting on.

- **Anchor and seed.** Every render is anchored to a world origin (the layout's top-left
  square in world squares, 64 per cell, convention I2) or, for the built-in demo layouts and
  posted layouts without `origin`, to the layout name. The compositor seed is the first
  8 bytes (little-endian) of BLAKE3 over `"arda-server/tactical-render-seed/v1"`, the world
  seed (u64 LE), then `"W"` for any world origin or `"N"` with the name. World-anchored
  renders therefore share one seed per world (logic/11 §seam-art 3): the layout's own
  `origin` makes the compositor hash world square coordinates, so the art differs from place
  to place yet joins across block edges.
- **Seamless world images.** A cell or window image is rendered from the requested squares
  plus an **apron of 6 squares** from the neighbouring blocks on every side (clipped at the
  world edge), then cropped (logic/11 §seam-art 2). The pixels of an independently rendered
  cell equal the same pixels of any window render that contains it (logic/16 Invariant 5,
  tested for a 2 × 1 window and checked for a 3 × 3 window on MICRO seed 42).
- **Cache key.** Renders are cached by `(world seed, anchor, BLAKE3 of the layout's JSON,
  ppsq, grid, library_version, crop)` (goal 67); for world images the layout is the apron
  layout and `crop` is the requested part. A posted layout equal to a built-in one, without
  `origin`, shares its renders. World JSON bodies are cached by `(window, demo anchor,
  ppsq)`, and composed blocks and refined 64 × 64 blocks in their own LRUs. Identical
  requests return identical bytes, cold or cached. Requests render one at a time, and
  prefetch workers render one at a time in their own lane (see
  [prefetch](#post-v1tacticalprefetch)); a key never renders twice at once, since a request
  for a key already rendering waits for that render.
- **Goal 67 check.** The key carries the world seed (`world_seed`), the coordinates (the
  `Origin` anchor, in world squares, and the `crop`) and the catalogue version
  (`library_version`); `tests/world_api/prefetch.rs` asserts all three for two neighbouring
  cells. JSON bodies are keyed by `(window, demo anchor, ppsq)` inside one server, which
  serves one world and one library.
- **Headers.** Every image carries a strong `ETag` (a BLAKE3 prefix of the bytes);
  `If-None-Match` with it returns **304** with no body. `X-Arda-Cache` is `hit` or `miss`
  and `Server-Timing: tactical;dur=<ms>` gives the server time. Images of built-in layouts
  and of world cells and windows (images and JSON) are `Cache-Control: public,
  max-age=3600`; `POST /render` responses are `no-cache` (revalidate by ETag).
- **Options.** `ppsq` (output pixels per square) is `64`, `96` or `128` (the default);
  `grid` is `0` (the default) or `1`.
- **Timing logs.** Each request logs one line to stderr: route, layout, options, cache hit
  or miss, bytes and milliseconds. Cache misses also log the render, PNG-encode and pyramid
  stages. On a release build, a cached riverside PNG (3840 × 2176, 14 MB) takes about
  0.06 ms in the server and about 2 ms over loopback curl. A cold render takes about 3 s.

The examples assume `B=http://localhost:8787/v1/tactical`.

### `GET /v1/tactical/layouts`

Lists the built-in test layouts in name order, with their size in squares and their tile
pyramid at the default ppsq. TS type: `TacticalLayouts`.

```sh
curl -s $B/layouts
```

```json
{"world_seed":"42","library_version":"0.1.0+seed58","ppsq_options":[64,96,128],"default_ppsq":128,
 "layouts":[{"name":"riverside","width":30,"height":17,
   "tiles":{"tile_px":512,"ppsq":128,"image_width_px":3840,"image_height_px":2176,"max_zoom":3,"format":"webp"}},
  {"name":"stone_warehouse","width":18,"height":13,"tiles":{"...":"..."}},
  {"name":"timber_house","width":16,"height":13,"tiles":{"...":"..."}},
  {"name":"wall_junctions","width":16,"height":9,"tiles":{"...":"..."}}]}
```

### `GET /v1/tactical/layout/{name}`

Returns the `TacticalLayout` JSON the compositor draws: row-major `squares` (ground key,
elevation, water depth), wall `walls` on square edges, `placements` and `lights`. TS type:
`TacticalLayoutDto`, a field-for-field mirror of `arda_tactical::TacticalLayout`. An unknown
name is **404 `not_found`**.

```sh
curl -s $B/layout/riverside
```

### `GET /v1/tactical/layout/{name}.png[?ppsq=64|96|128&grid=0|1]`

Returns the whole map as one RGBA PNG of `width·ppsq × height·ppsq` pixels.

```sh
curl -s -o riverside.png "$B/layout/riverside.png?ppsq=128&grid=1"
curl -s -o /dev/null -w '%{http_code}\n' -H 'If-None-Match: "<etag>"' "$B/layout/riverside.png?ppsq=128&grid=1"   # 304
```

### `GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp[?ppsq=&grid=]`

Returns a 512 px **lossless WebP** tile (encoded by the pure-Rust `image-webp` crate,
MIT/Apache-2.0) of the same render, for smooth pan and zoom.

- `max_zoom` is the least `z` with `512·2^z ≥ max(image_width_px, image_height_px)`. Zoom
  `max_zoom` is the render at full resolution, and each lower zoom halves the one above it
  (rounding up) with an integer premultiplied area average.
- Level `z` is `ceil(W / 2^(max_zoom−z)) × ceil(H / 2^(max_zoom−z))` pixels and has
  `ceil(w_z/512) × ceil(h_z/512)` tiles, anchored at the top-left. Edge tiles are padded to
  512 × 512 with transparent pixels.
- `z > max_zoom`, `x` or `y` past the level's tile grid, a non-`.webp` suffix and an
  unknown layout all return **404 `not_found`**.

```sh
curl -s -o tile.webp "$B/layout/riverside/tiles/3/2/1.webp"
curl -s -o whole.webp "$B/layout/riverside/tiles/0/0/0.webp"
```

### `POST /v1/tactical/render[?ppsq=&grid=&origin=X,Y]`

Renders a client-supplied `TacticalLayout` JSON body and returns the PNG.

- `origin` is the layout's top-left square in world squares. It anchors the seed and the
  cache key; without it, the layout name does. `origin` on a `GET` route is 400.

- The body is at most **4 MiB** and must parse as a `TacticalLayout` (unknown fields are
  rejected); otherwise the response is 413 or 400.
- The layout is at most **16 384 squares** (128 × 128), and the render at most **24 Mi
  pixels** (for example 32 × 32 squares at 128 ppsq), or the response is **413
  `payload_too_large`**.
- The body must be sent as `Content-Type: application/json` (415 otherwise), which makes
  browsers preflight it against the localhost-only CORS policy.
- The layout is checked against the loaded library first, and an inconsistent one returns
  **422 `invalid_layout`** with the first problem in `message`.

```sh
curl -s $B/layout/riverside > riverside.json
curl -s -o posted.png -X POST -H 'content-type: application/json' --data-binary @riverside.json "$B/render?ppsq=64"
```

### `GET /v1/tactical/cell/{gx}/{gy}[?ppsq=&demo_overlays=0|1&demo_at=GX,GY]`

The world-derived tactical block at global cell `(gx, gy)` (logic/16 §api-tactical, adapter
A9), built on demand by `RefineBlocks`: arda-refine refines the cell (5-ft squares, WFC
ground, rivers, scatter; blocks cached per cell), arda-blocks composes the overlays, and the
server derives the scene from the same layout (goal 48). The cell must lie inside the world
(400 `out_of_range`); an open-sea cell is 404 `no_block`. TS type: `TacticalBlockDto`.

```json
{"tactical_format":1,"layout_schema":1,"origin":[33920,51840],"size":[64,64],
 "render_seed":"1234…","layout":{"name":"cell_530_810","width":64,"height":64,
 "squares":[{"ground":"grass","elevation_ft":35,"water_depth_ft":0},…],"walls":[],
 "placements":[…],"lights":[],"origin":[33920,51840]},
 "rules":{"format_version":2,"width":64,"height":64,"squares":[{"difficult":false,…},…],"edges":[]},
 "meta":{"source":"arda-refine","relaxed":"false","review_squares":"0","overlays":"",…},
 "scene":{"format_version":1,"seed":"1234…","origin_gs":[33920,51840],"movement":[[4096,"normal"]],…},
 "tiles":{"tile_px":512,"ppsq":128,"image_width_px":8192,"image_height_px":8192,"max_zoom":4,"format":"webp"}}
```

- `layout` is `arda-tactical`'s `TacticalLayout`; its optional `origin` (I10) is the world
  square of square `(0, 0)`, here `[64·gx, 64·gy]`.
- `rules` is `arda-scene`'s `RulesSidecar` **format 2**, the one per-square rules schema
  (I9; TS `RulesSidecarDto`): `difficult`, `water_depth_ft`, `cover`, `blocks_sight`,
  `lightly_obscured`, `blocks_movement`, `deck`, each optional, plus an `ext` map for layer
  extras (`feature`, `road_class`, `deck_elevation_ft` from ways; `crop`, `field`, `furrow`
  from fields; `building` from town) and `edges` for parapets, hedges and field walls.
- `scene` is `arda-scene`'s `Scene` (format 1) of the same layout, library and render seed,
  with `seed` as a decimal string and the block's `origin_gs` (I17).
- `meta` is provenance: `source`, `generator`, `relaxed` (goal 47 review flag: arda-refine's
  relaxed blocks or the town WFC's relaxed interiors and outdoor chunks, `TownBlock.relaxed`),
  `review_squares` (squares under a relaxed fill, both sources; windows report theirs the same
  way), `overlays` (the layers that claimed squares), `demo_at`, `fine_frame`
  (`i1`: the fine lattice read in the I1 cell frame, as `/v1/point` reads it).
- `tiles` describes the WebP pyramid at `?ppsq=` (default 128).
- **Overlays.** When the world has a `society/` directory (`arda settle`, `arda society
  build`), blocks carry its roads and crossings (arda-ways), fields from `landuse.bin`
  (arda-fields) and each settlement's town plan (arda-town), composed ways, fields, town by
  the reservation precedence of logic/09; `meta.society` is `1`, and town squares carry
  `ext.building` and `ext.settlement`. `?demo_overlays=1` instead composes the synthetic
  samples of arda-ways (Thornby's roads),
  arda-fields (`village_strips`) and arda-town (the Thornby plan), in that order and by the
  reservation precedence of logic/09 (town over ways over water over fields over natural
  ground; no layer dries water it does not bridge), with Thornby's market at the centre of
  cell `demo_at` (default: this cell). Cells requested with the same `demo_at` join exactly.

```sh
curl -s $B/cell/530/810 | jq '.meta, .size, .rules.format_version'
curl -s "$B/cell/530/810?demo_overlays=1" | jq .meta.overlays
curl -s $B/cell/0/0      # {"error":{"code":"no_block","status":404,…}}
```

### `GET /v1/tactical/cell/{gx}/{gy}/scene[?time=day|night&demo_overlays=&demo_at=]`

The scene of the same composed block (goal 48) and its NPC tokens (adapter A13, logic/12
§scene-tokens): `{tactical_format, origin_gs, time, scene, tokens}`. A token is
`{npc_id, name, x, y, building_id, settlement_id, kind}`: a stored notable of a settlement
whose plan buildings lie in the block, standing by day in its workplace (`worker`), else and
by night at home (`resident`), on a free floor square nearest the building's centre. Every
`npc_id` resolves with `GET /v1/npc/{npc_id}`. TS mirrors for the scene and tokens are
pending (A12).

```sh
curl -s $B/cell/618/689/scene | jq '.tokens[0]'
# {"npc_id":"10166616894566488805","name":"Brelil \"Don\" Fyler","x":24,"y":9,
#  "building_id":"5","settlement_id":"118","kind":"worker"}
```

### `GET /v1/tactical/cell/{gx}/{gy}.png[?ppsq=16|32|64|96|128&grid=0|1&demo_overlays=&demo_at=&world_grade=0|1]`

The painted block, rendered with its apron and cropped (see "Seamless world images").
`64·ppsq` pixels a side.

`?world_grade=1` (goal 49, opt-in; also on the cell tiles and `/window.png`) pulls the
ground and water layers toward the world map's own colours at their location, before
any sprite is drawn: the formed relief shader is sampled on a world-anchored 12.5 m
lattice (8 squares) and each channel is scaled by `1 − s + s·world/mean` (`s` = 0.7; a
quarter on streets and floors, half on farmland), where `mean` is the regional texture
mean, so grain and material contrast survive. Graded images keep the seam invariant;
without the flag images are byte-identical to before. Prefetch warms ungraded images
only.

```sh
curl -s -o cell.png "$B/cell/530/810.png?ppsq=64"
```

### `GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp[?ppsq=&grid=&demo_overlays=&demo_at=]`

512 px lossless WebP tiles of the block image, `z = max_zoom` the full render (a block at
128 px per square is 8192 px, `max_zoom = 4`). Outside the pyramid: 404 `not_found`.

### `GET /v1/tactical/window?gx0=&gy0=&w=&h=[&ppsq=&demo_overlays=&demo_at=]` and `/window.png`

A window of `w × h` cells (1–3 each; 413 above) starting at cell `(gx0, gy0)`: the same body
as a cell with `size` `[64·w, 64·h]`, or its PNG (`ppsq` default 32 for windows). A window
equals the concatenation of its cells square for square, and its image equals theirs pixel
for pixel. `demo_at` defaults to `(gx0, gy0)`.

With `?gsx=&gsy=&w=&h=` instead, the window is given in world squares (logic/16): any
`w × h` squares (1–192 each; 413 above) from square `(gsx, gsy)`, for example a 32 × 32
quarter-block battle map across a cell corner. `demo_at` defaults to the cell holding
`(gsx, gsy)`.

```sh
curl -s -o window.png "$B/window.png?gx0=529&gy0=809&w=3&h=3&ppsq=32"
curl -s -o quarter.png "$B/window.png?gsx=33904&gsy=51824&w=32&h=32&ppsq=64"
```

### `POST /v1/tactical/prefetch`

Warms the neighbours of a cell in the background (goal 67; logic/16 §api-prefetch) and answers
**202** at once. Body (TS `PrefetchRequest`, `Content-Type: application/json`, unknown fields
rejected):

```json
{"gx": 618, "gy": 689, "radius": 1, "ppsq": 64, "demo_overlays": false}
```

- `radius` is 1 (the 8 neighbours, the default) or 2 (the 24 cells of two rings); anything
  else is 400. `ppsq` takes the cell route's values (default 128). The centre must lie inside
  the world (400 `out_of_range`); rings are clipped at the world edge.
- Cells are queued nearest ring first, row-major within a ring. The queue holds at most 64
  cells, deduplicated; a cell already queued or warming is `pending`, and a full queue
  `dropped` the rest. Nothing is ever trimmed silently: every cell is listed with its outcome.
- `--prefetch-workers` threads (default 2) take cells from the queue. Each builds the cell's
  composed block and JSON body at `ppsq` and, for the nearest cells whose render and tile
  pyramid fit the render and pyramid caches beside the open cell (`image: true`; about 10 at
  64 px per square and 1 at 128 with the default budgets), renders its image (grid off) and
  builds the pyramid. Open-sea cells have nothing to warm (`no_block`).
- Prefetching never blocks a request: the handler only touches the queue, and workers render
  in their own lane. It never changes a response's bytes; it only turns later misses into
  hits (`X-Arda-Cache: hit` on the JSON, and tiles cut from a cached pyramid).
- Each warmed cell logs one stderr line: cell, ppsq, image flag, outcome and milliseconds.

Response (TS `PrefetchAccepted`):

```json
{"radius":1,"ppsq":64,"queue_len":8,
 "cells":[{"gx":617,"gy":688,"status":"queued","image":true},{"gx":618,"gy":688,"status":"queued","image":true},…]}
```

```sh
curl -s -X POST -H 'content-type: application/json' -d '{"gx":618,"gy":689,"ppsq":64}' $B/prefetch | jq '.cells | length'
```

**Walking off an edge.** Cell images are rendered with an apron from the neighbouring blocks
(see "Seamless world images"), so the cell east of `(gx, gy)` is simply `(gx + 1, gy)`: its
west edge joins this cell's east edge pixel for pixel, and once prefetched it loads from cache.

### `GET /v1/tactical/library`

The loaded catalogue for layout editors: `grounds` (`key`, texture `variants`, `water`),
`wall_kits` (`kit`, `roles`) and placeable `assets` (`id`, `class`, `layer`, `footprint`,
`rotations`, `tags` as `key:value` strings, `cover`, `blocks_sight`, `blocks_movement`,
`difficult_terrain`). TS type: `TacticalLibraryDto`.

### Timing (goal 50; release, MICRO seed 42, `ppsq=64`, 32 threads)

| Request | Cold | Warm |
|---|---|---|
| block PNG, village centre, freshly opened server (plan, refine, compose, render, encode) | ~420 ms | ~10 ms |
| block PNG, city centre (its plan drawn at startup) | ~340 ms | ~10 ms |
| block PNG next to one already served | ~280 ms | ~10 ms |
| quarter-block window PNG (32 × 32 squares) | ~95 ms | ~5 ms |
| a WebP tile after the render | ~75 ms | ~1 ms |

A block image composes its window with a 6-square apron (up to 9 refined cells, cached), runs
the ways, fields and town overlays at once, renders only the block's own pixels of the
76 × 76-square layout (the apron still shapes them: heights, sprites, shadows and blurs;
~200 ms), and encodes the PNG in parallel strips (~35 ms; RGB when opaque). At startup the
server scales the ground textures for every ppsq and draws the town and city plans (about
0.5 s on MICRO), so no request waits for them. `cargo run --release -p arda-server --example
tactical_timing -- <world> <gx> <gy>` prints the stages; the release gate
`tests/e2e_village.rs` holds the cold block under 500 ms and the quarter window under 250 ms
(best of three fresh servers).

### The block seam

```rust
pub trait BlockSource: Send + Sync + std::fmt::Debug {
    fn block(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError>;
}

pub struct BlockRequest { pub gsx0: i64, pub gsy0: i64, pub w: u32, pub h: u32, pub demo_at: Option<[i64; 2]> }

pub struct Block {
    pub layout: TacticalLayout,           // with origin
    pub rules: Option<arda_scene::RulesSidecar>,
    pub origin: [i64; 2],
    pub meta: BTreeMap<String, String>,
}
```

`AppState::open` plugs in `RefineBlocks` over `--world`; `with_block_source` replaces it.
`BlockError::NotYet` maps to 501, `NoBlock` to 404 `no_block`, `Window` to 400 and `Failed`
to 500. The real overlays plug into `arda_blocks::Overlays` (methods `ways`, `fields`,
`town`, each returning a layer of the window with its format-2 rules and an ownership mask)
once settlement data is integrated.

### `POST /v1/npc/population`

Generates the population of one settlement with `arda-npc` (`logic/13`). Body
(TS `PopulationRequest`):

```json
{"world_seed": "10673758", "settlement": { SettlementProfile }, "buildings": [ BuildingSpec, … ]}
```

- `world_seed` is a decimal u64 string. Every id (`id`, `settlement_id`, `realm_id`) is a
  decimal u64 string; numbers are accepted on input too.
- `SettlementProfile` is the `arda-settle` settlement record shape: extra record fields are
  ignored, and `biome` is one of `temperate`, `warm_temperate`, `temperate_forest`,
  `boreal_forest`, `highland`, `alpine`, `wetland`, `steppe`, `coastal`.
- `BuildingSpec.function` is a plain snake_case string (`inn`, `market_hall`, `workshop`, …).
  A workshop's craft is a free tag in the optional `tags` array, `craft:weaving`; a workshop
  without one draws its craft from its own seed key.
- Limits: the body is at most **2 MiB**, `population` at most **50,000** and the building
  list at most **20,000** entries (413 `payload_too_large`). Housing below the population is
  refused, never trimmed (400).

Response: `Population` (TS `Population`): households, the roster (one compact entry per
inhabitant) and the notables as full `Npc` records with SRD 5.1 sheets. Commoners are not
stored; regenerate them by id. The same body always returns the same bytes.

### `GET /v1/npc/demo`

The market-town example of `arda-npc` (`sample::market_town`, the `town` example): Wendlebrook,
1,500 people. Response `NpcDemo`: `world_seed`, `settlement`, `buildings` and `population`.
Built once per process and then served from memory.

### `GET /v1/npc/demo/{npc_id}`

Regenerates one inhabitant of the demo town from scratch, notable or commoner, as an `Npc`.
`npc_id` is the decimal u64 `NpcId` from the population (`NpcId::from_parts(settlement,
home building, resident index)`, `arda-ids`). The result equals the notable's entry in
`/v1/npc/demo` byte for byte, and for a commoner it matches the roster's job and workplace.
An unknown id is 404 `not_found`; a non-u64 id is 400 `bad_request`.

## Cell contract

Rust: `arda_server::contract::CellSample`, version `CONTRACT_VERSION = 2`.

Contract 2 (integration phase A) moved the cell frame to vocabulary I1: `x_m`/`y_m` are the
cell's north-west corner, `centre_x_m`/`centre_y_m` are new, fine heights are taken over
`[x_m, x_m + 100]` around the centre, `/v1/point` picks the containing cell, and the sea-cell
snowline is measured from sea level. Phase D reads the fine lattice in the same frame (lattice
origin at the first cell's centre), so fine heights moved by 50 m on both axes without a change
of meaning, unit or presence; the contract version stays 2.
TS: `bindings/ts/arda/CellSample.ts`.

| Field | Unit | Source |
|---|---|---|
| `contract_version` | — | `CONTRACT_VERSION` |
| `gx`, `gy` | cell | request |
| `ax`, `ay`, `cx`, `cy` | area, cell | `gx div 512`, `gx mod 512` (same for y) |
| `x_m`, `y_m` | m | the cell's north-west corner, `gx·100` and `gy·100` |
| `centre_x_m`, `centre_y_m` | m | the cell centre, `gx·100 + 50` and `gy·100 + 50` |
| `height_m` | m above sea level | `Cell::height` (mm) / 1000 |
| `terrain` | `sea`, `land`, `lake` | `Cell::terrain` |
| `cover` | `bare`, `grass`, `scrub`, `forest`, `marsh`, `rock`, `ice` | `Cell::cover` |
| `slope_deg` | degrees | `Cell::slope_milli_deg` / 1000 |
| `aspect_deg` | degrees, 0–359, 0 on flat ground | `Cell::aspect_deg` |
| `temperature_c` | °C, mean annual | `Cell::temperature` (centi-°C) / 100 |
| `rainfall_mm` | mm per year | `Cell::rainfall` |
| `moisture` | 0–1 | `Cell::moisture` / 255 |
| `wetness` | 0–1 | `Cell::wetness` / 255 |
| `forest_density` | 0–1 | `Cell::forest_density` / 255 |
| `drainage_area_km2` | km² | `Cell::drainage_area_cells` × 0.01 |
| `discharge_m3s` | m³/s | `Cell::discharge` (L/s) / 1000 |
| `watercourse_order` | Strahler, 0 for none | `Cell::watercourse_order` |
| `watercourse_width_m` | m | `Cell::watercourse_width_dm` / 10 |
| `height_above_river_m` | m | `Cell::height_above_river_dm` / 10 |
| `road` | `none`, `track`, `road`, `highway` | `Cell::road` |
| `built_by` | settlement id or `null` | `Cell::built_by` (0 means null) |
| `coast.is_coast` | bool | derived: a land cell with a **sea** cell among its 8 neighbours. Lake shores aren't coast. |
| `coast.distance_m` | m, or `null` | derived: exact Euclidean centre-to-centre distance to the nearest coast cell, over the area plus a 51-cell neighbour halo. `null` beyond 5 km. Cells outside the world count as neither sea nor coast. |
| `snow.fraction` | 0–1 | derived **proxy**, see below |
| `snow.peak_fraction` | 0–1, or `null` | the proxy at the footprint's highest fine point; `null` without fine terrain |
| `snow.perennial` | bool | `fraction ≥ 0.5`, or ice cover |
| `snow.snowline_m` | m | `surface + (temperature_c + 5.5) / 0.0065`, where the surface is `height_m` on land and lakes and sea level (0 m) on sea cells, whose saved temperature is the sea-surface value |
| `river` | object or `null` | the saved `RiverSegment` whose `course` lists this cell. Where courses overlap, the highest order wins, then the lowest id. Fields: `segment_id`, `global_reach_id`, `order`, `width_m`, `discharge_m3s`. |
| `lake` | object or `null` | the saved `Lake` whose `cells` list this cell (lowest id wins). Fields: `lake_id`, `global_basin_id`, `surface_m`, `depth_m` (surface − height, ≥ 0), `max_depth_m`. |
| `fine.centre_m` | m | `terrain/fine.bin` bilinear at the cell centre (39.0625 m lattice; clamped to coverage in the last row and column) |
| `fine.min_m`, `fine.max_m` | m | exact extremes of the bilinear fine surface over the footprint, clipped to coverage. The footprint is `[x_m, x_m + 100] × [y_m, y_m + 100]`; they're evaluated at its corners and every lattice line crossing it. |

Arda does not simulate snowpack. The **snow proxy** mirrors the Atlas formed renderer
(`logic/04` §atlas-formed snow):

- Ice cover counts as 1. Sea and lake count as 0.
- Otherwise, snow ramps in with smoothstep as the mean annual temperature goes from −4 °C
  to −7 °C. That value is multiplied by `1 − 0.75·smoothstep(0.7, 1.3, tan slope)`, because
  cliffs shed snow.
- Altitude enters through a 6.5 °C/km lapse rate.

`sample_point` is available to Rust callers as `WorldQuery::sample_point(x_m, y_m)`, and over
HTTP as `/v1/point`.

## TypeScript bindings

The bindings are generated from the Rust DTOs by ts-rs into `bindings/ts/arda/` at the
workspace root, which is committed. `index.ts` re-exports every type, along with
`CONTRACT_VERSION` and `API_VERSION`. The NPC types (`Npc`, `Sheet`, `Population`,
`SettlementProfile`, `BuildingSpec`, …) are mirrors owned by the server (`src/npc_dto/`),
because `arda-npc` stays free of ts-rs; a test per mirror proves it reads and writes the
domain JSON unchanged.

```ts
import type { CellSample, Npc, Population, TacticalLayoutDto, TacticalLayouts, WorldInfo } from "./bindings/ts/arda";
```

`cargo test -p arda-server bindings` fails when the committed files are stale. To regenerate
them:

```sh
ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings
```
