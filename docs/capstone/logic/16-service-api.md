---
generated_date: 2026-09-30
scenario: service-api
status: normative design; world routes implemented on feat/world-server, tactical routes in progress on feat/server-tactical (crate arda-server), developer viewer on feat/viewer
goals: 41, 57, 65, 66, 67, 68, 69, 70
---

# 16 — Service API: HTTP routes, contract versioning, caching and TS bindings

> Normative design for `arda-server`, the Rust HTTP service the maintainer's TS/React game calls (goal 65). It serves the world contract, the overview, settlements, roads, realms, history and NPCs, and on-demand tactical maps: layout, painted tiles and scene data. The byte-level reference for the implemented world routes is `crates/arda-server/API.md` on feat/world-server; this file states the rules every route follows and the routes the product layers add. It composes [08](08-settlements-roads-realms.md), [09](09-tactical-refinement.md), [10](10-town-layout.md), [11](11-tactical-art-compositor.md), [12](12-scene-data.md), [13](13-npc-population.md), [14](14-society-history.md) and [15](15-naming.md).

Code cites rules as `// logic/16 §<rule>`.

## Trigger & preconditions

- Trigger: `arda-server --world <dir> [--library <dir>] [--port 8787] [--host 127.0.0.1]` and HTTP requests.
- Preconditions at start: the world loads (`World::load`); the tactical library validates (11 §validator); `society/` is optional (routes that need it answer `404 not_found` naming the missing file); cache budgets pass admission against the 16 GiB ceiling (§api-cache). Any failure refuses to start with the reason on stderr.

## Rules

### §api-conventions

- Every route is under `/v1`. JSON unless noted. Coordinates follow 08 §world-frame (cells) and 09 §square-frame (squares).
- **Big integers**: every u64 on the wire (seed, reach and basin ids, render keys) is a decimal string, because it does not fit a JS number. Settlement, building, realm and road ids are also strings on the wire even though they are small, so the TS types never change when a world grows (the domain files under `society/` keep numbers). `NpcId` is `"<settlement>.<building>.<index>"` (13 §npc-id).
- **Determinism**: the same request against the same world, society and library returns identical bytes, cold or cached (goal-prompt §8).
- **CORS**: `GET`, `HEAD`, `OPTIONS` (and `POST` for `/v1/tactical/render`) from `http(s)://localhost`, `127.0.0.1` and `[::1]` on any port; other origins get no CORS headers.
- **Security**: no authentication in v1; the server binds to loopback by default. How the game authenticates to and hosts the service is an open question (goal-prompt §6) and is not decided here.

### §api-versioning

Three independent version numbers, each carried in the bodies that depend on it:

| Family | Field | Current | Bumps when |
|---|---|---|---|
| world cell contract | `contract_version` | 1 → **2** with this spec | a field's meaning, unit or presence changes |
| tactical response | `tactical_format` | 1 | the `TacticalBlock` or `TacticalScene` envelope changes (the envelope carries the service-added `origin_gs` and `tokens`) |
| scene | `scene.format_version` | 1 | the `arda-scene` schema changes (12) |
| layout schema | `layout_schema` | 1 | `TacticalLayout` changes meaning (the planned optional `origin` field is additive and does not bump it) |

Rules: clients reject a version they do not know. Adding an optional field never bumps a version; renaming, removing or re-meaning one does. A change that breaks a route's shape moves the route to `/v2`, and `/v1` stays served for one release. `GET /v1/world` lists every family's current version so the game can check once at start.

Contract 2 (this spec) changes: `road` gains the legend value `footpath` and is read from `society/roads.bin` (08 §roads) when present; `built_by` is read from `society/landuse.bin` owners and is a settlement id string or `null` (§api-cell-society); the sea-cell snowline is computed from sea level, not the seafloor (the journal's noted defect). Without `society/`, both fields keep contract-1 values and `contract_version` is still 2.

### §api-routes

Implemented on feat/world-server (see `crates/arda-server/API.md`): `/v1/health`, `/v1/world`, `/v1/cell/{gx}/{gy}`, `/v1/area/{ax}/{ay}/cells[?format=bin]`, `/v1/area/{ax}/{ay}/rivers`, `/v1/area/{ax}/{ay}/lakes`, `/v1/point?x_m=&y_m=`, `/v1/overview.png[?quality=&style=]`, `/v1/tiles/overview/{z}/{x}/{y}.png`.

In progress on feat/server-tactical and used by feat/viewer: `/v1/tactical/layouts`, `/v1/tactical/layout/{name}`, `/v1/tactical/layout/{name}.png`, `/v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp`, `/v1/tactical/cell/{gx}/{gy}`.

Added by this spec (goals 57, 65, 67, 70):

| Route | Response (TS type) | Source |
|---|---|---|
| `GET /v1/tactical/cell/{gx}/{gy}` | `TacticalBlock` `{tactical_format, layout_schema, origin_gs, cell, layout, rules, meta, tiles: TacticalTilesDto, scene_url}` | 09, 10 |
| `GET /v1/tactical/cell/{gx}/{gy}/scene[?time=day\|night]` | `TacticalScene` `{tactical_format, origin_gs, scene, tokens}` | 12, 13 |
| `GET /v1/tactical/cell/{gx}/{gy}.png[?ppsq=&grid=]` | PNG | 11 |
| `GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp[?ppsq=]` | 512 px lossless WebP tile | 11, §api-tiles |
| `GET /v1/tactical/window?gsx=&gsy=&w=&h=` | `TacticalBlock` for a window of ≤ 256 × 256 squares | 09 Branches |
| `GET /v1/tactical/window/scene?…` | `TacticalScene` for the window | 12 |
| `POST /v1/tactical/render` | PNG of a posted layout (debug and tooling; body limit) | 11 |
| `GET /v1/settlements[?realm=&tier=&bbox=x0,y0,x1,y1]` | `SettlementList` (records of 08, paged `?cursor=`) | 08 |
| `GET /v1/settlements/{id}` | `SettlementDetail` (record + history of 14 + offices) | 08, 14 |
| `GET /v1/settlements/{id}/plan` | `TownPlan` | 10 |
| `GET /v1/settlements/{id}/buildings` | `[TownBuilding]` with their `BuildingSpec` | 10 |
| `GET /v1/settlements/{id}/npcs[?building=&job=&notable=&cursor=]` | `NpcPage` (`Npc` records; commoners generated on demand) | 13 |
| `GET /v1/npc/{npc_id}` | `Npc` with its sheet | 13 |
| `GET /v1/realms`, `GET /v1/realms/{id}` | `Realm` (+ relations, offices, events) | 08, 14 |
| `GET /v1/roads[?bbox=]` | roads, crossings and passes clipped to the box | 08 |
| `GET /v1/tiles/overlay/{z}/{x}/{y}.png` | 256 px transparent overlay tiles: tier symbols, road classes, realm borders, labels, aligned with the overview pyramid | 08 overlay, goal 41 |
| `GET /v1/tactical/prefetch?gx=&gy=&radius=1` | `202` with the queued cells | §api-prefetch |

### §api-cell-society

When `society/` exists, `/v1/cell` and `/v1/area/.../cells` fill `road` from `roads.bin` (legend `none`, `footpath`, `track`, `road`, `highway`) and `built_by` from `landuse.bin` owners; they add `land_use` (08 §landuse keys) and `realm_id` (from `realms.bin`). The binary `ARDACOLS` layout gains the columns `land_use` (u8), `realm_id` (u32, 0 none) and changes `road` codes to the society raster codes; its layout version becomes 2.

### §api-tactical

A tactical request resolves in this order: validate coordinates → cache lookup (§api-cache) → `refine_block` with town, ways and fields reservations (09 §reservations) → scene (12) with tokens (12 §scene-tokens) → images on demand. The server's `BlockSource` seam returns the whole `RefinedBlock` (layout, rules sidecar, meta), not the layout alone, so scene and image are built from one layout (goal 48). Sea blocks answer `404 no_block`. Images are rendered from a window with a 2-square apron and cropped (11 §seam-art); walking off an edge loads the neighbour, which joins invisibly (goal 67).

### §api-tiles

- Overview: 256 px PNG slippy tiles cut from one cached Atlas render at `base_px = 256 · 2^max_zoom` (default 4096), `z = 0` the whole world (implemented).
- Tactical: lossless WebP (goal 68), 512 px tiles, one pyramid per block or window. Level `max_zoom` is the full render (a block at 128 px per square is 8192 px, so `max_zoom = 4`); each lower level halves with the compositor's integer premultiplied area average, rounding up; edge tiles are padded transparent.
- `?ppsq=` accepts 64, 96 or 128 (default 128, 11 §ppsq).

### §api-cache

- Every generated body is cached in a byte-bounded LRU keyed by `RenderKey = BLAKE3(world identity, society inputs hash, generator versions, library_version, route, coordinates, options)`. World identity is the seed plus the BLAKE3 of `world.json`; generator versions are the crate versions of `arda-refine`, `arda-town`, the ways and fields crates, `arda-scene` and `arda-npc`. This realises goal 67's "cached by (seed, coordinates, catalogue version)" and also invalidates on a generator change.
- `ETag` is the hex of the key (quoted); `If-None-Match` answers `304`. Tactical and overlay bodies carry `Cache-Control: public, max-age=3600` (the overview's existing policy).
- Budgets (defaults, assumed, tunable; all flags): decoded areas 1 GiB, fine windows 512 MiB, overview PNGs and tiles 1 GiB, tactical layouts and scenes 1 GiB, tactical renders and tiles 4 GiB, town plans and populations 1 GiB. At start the worst case of all budgets plus one transient request per worker must fit 16 GiB, or the server refuses to start (goal-prompt §8).
- Only one full-resolution tactical render runs per key at a time; concurrent requests for the same key wait for it.

### §api-prefetch

A tactical cell request enqueues its 8 neighbours at low priority (radius 1; `?radius=` up to 2 on the prefetch route). The queue is bounded (64 cells), deduplicated and drops when full; prefetching never changes any response's bytes. Prefetch stops at sea cells and the world edge (goal 67).

### §api-errors

Every error is `{"error": {"code", "status", "message"}}`; clients match on `code`:

| Status | `code` | When |
|---|---|---|
| 400 | `bad_request` | unparsable path or query value, unknown option |
| 400 | `out_of_range` | a cell, area, square or point outside the world |
| 404 | `not_found` | unknown route, id, tile outside the pyramid, missing layer or society file |
| 404 | `no_block` | a cell with no land square (open sea) |
| 409 | `stale_society` | `history.json` was built from different 08 files (14 State transitions) |
| 413 | `payload_too_large` | a window or posted layout above the limits |
| 422 | `invalid_layout` | a layout or sidecar inconsistent with the library (11, 12) |
| 500 | `resource_limit` | an allocation or budget refused the work |
| 501 | `not_implemented` | a route whose source crate is not merged yet; the body adds `planned_source` |
| 500 | `internal` | anything else |

### §api-bindings

- TypeScript types are generated from the Rust DTOs with ts-rs into `bindings/ts/arda/` at the workspace root, committed, re-exported by `index.ts` together with `API_VERSION` and every family version (goal 66). `cargo test -p arda-server bindings` fails when the committed files are stale; `ARDA_BLESS_BINDINGS=1` regenerates them.
- Domain crates (`arda-npc`, `arda-scene`, `arda-tactical`, `arda-town`) stay free of ts-rs. The server owns DTO mirrors with `From` conversions, and a test per mirror asserts that the mirror's JSON equals the domain type's serde JSON for a fixture (so the game sees exactly the domain schema, goal 69), except where §api-conventions turns u64 into strings.
- NPC sheets (13 §npc-game-schema) and scenes are exported under the names the game uses: `Npc`, `Sheet`, `Scene`, `TacticalBlock`, `TacticalScene`, `Token`.

### §api-gm-path

The GM tool path (goal 70) is a sequence of these routes, exercised by the developer viewer (feat/viewer) and the end-to-end test: `/v1/world` → overview and overlay tiles to browse → `/v1/settlements?bbox=` or `/v1/cell` to pick a place → `/v1/tactical/cell/{gx}/{gy}` and its tiles for the map → `/scene` for rules and tokens → `/v1/npc/{id}` for any token.

## Steps

1. Parse and validate the request; map errors per §api-errors.
2. Compute the `RenderKey`; answer from cache or `304` when possible.
3. Otherwise build the body from the owning crates (routes table), store it, answer.
4. For tactical cells, enqueue prefetch (§api-prefetch).

## Branches

- No `society/`: settlement, NPC and overlay routes answer `404 not_found` naming the file; tactical cells are natural only (09 Branches).
- A layer crate not merged: `501 not_implemented` with `planned_source`.

## Unhappy paths

Listed in §api-errors. A panic in a handler is a bug: it is caught at the task boundary, answered `500 internal`, and never poisons the caches (entries are inserted only after a body is complete).

## State transitions

In memory only: cache entries appear and are evicted; the prefetch queue fills and drains. The server never writes to the world directory.

## Invariants

1. Determinism: two identical requests give identical bytes and `ETag`s, cold and warm [65, 67].
2. Every route answers against a MICRO seed-42 world with `society/`, with a typed error for every invalid input (tests in-process, no mocks of Arda) [65].
3. Committed TS bindings are fresh; each DTO mirror serialises like its domain type [66, 69].
4. Tiles: every tile inside the pyramid decodes (PNG or WebP) to its documented size; tiles outside answer 404 [68].
5. Seams: the east column of tactical tile pixels of block `(gx, gy)` and the west column of block `(gx+1, gy)` at full zoom differ by no more than the WebP lossless round trip (zero) from the same columns of a 2 × 1 window render [67].
6. Budgets: the admitted worst case never exceeds 16 GiB; the server refuses to start rather than trim [goal-prompt §8].
7. Performance (release, warm world): a cached tile in < 20 ms; a cold block layout plus scene in < 500 ms; a cold quarter-block window render at 128 px per square in < 1 s (assumed budgets consistent with goal 50).
8. Tokens: every token of every scene resolves through `/v1/npc/{id}` [57, 69].

## Outcomes & side effects

HTTP responses; stdout request logging at the binary boundary only (standards: no logging framework in library crates). No files are written.

## Dimensions not in play

- Authentication, multi-tenant hosting and rate limiting (open question).
- Writes: the API is read-only; a GM's edits live in the game.
- Streaming or websockets; animation.
