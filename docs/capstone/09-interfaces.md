---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: eb418a4a3677
paths_covered:
  - ":(top)crates/arda-server/src/**"
  - ":(top)bindings/**"
  - ":(top)crates/arda-town/src/plan/types.rs"
  - ":(top)crates/arda-dungeon/src/lib.rs"
  - ":(top)contract-kit/**"
  - ":(top)Dockerfile"
  - ":(top)tools/docker-entrypoint.sh"
  - ":(top)crates/arda-server/tests/contract_kit.rs"
known_as:
  - "127.0.0.1:8787"
edges:
  produces:
    - { kind: http, name: "GET /v1/health", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/world", site: crates/arda-server/src/routes.rs, schema: WorldInfo }
    - { kind: http, name: "GET /v1/cell/{gx}/{gy}", site: crates/arda-server/src/routes.rs, schema: CellSample }
    - { kind: http, name: "GET /v1/area/{ax}/{ay}/cells", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/area/{ax}/{ay}/rivers", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/area/{ax}/{ay}/lakes", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/point", site: crates/arda-server/src/routes.rs, schema: PointSample }
    - { kind: http, name: "GET /v1/overview.png", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/tiles/overview/{z}/{x}/{y}.webp", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/tiles/overview/{z}/{x}/{y}.png", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/tiles/relief/{z}/{x}/{y}.webp", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "POST /v1/npc/population", site: crates/arda-server/src/routes.rs, schema: Population }
    - { kind: http, name: "POST /v1/npc/generate", site: crates/arda-server/src/npc.rs, schema: Npc }
    - { kind: http, name: "GET /v1/npc/demo", site: crates/arda-server/src/routes.rs }
    - { kind: http, name: "GET /v1/npc/demo/{npc_id}", site: crates/arda-server/src/routes.rs, schema: Npc }
    - { kind: http, name: "GET /v1/npc/{npc_id}", site: crates/arda-server/src/npcs/mod.rs, schema: Npc }
    - { kind: http, name: "GET /v1/npcs", site: crates/arda-server/src/npcs/mod.rs, schema: NpcPage }
    - { kind: http, name: "GET /v1/buildings/{building}/residents", site: crates/arda-server/src/npcs/mod.rs, schema: NpcPage }
    - { kind: http, name: "GET /v1/buildings/{building}/workers", site: crates/arda-server/src/npcs/mod.rs, schema: NpcPage }
    - { kind: http, name: "GET /v1/settlements", site: crates/arda-server/src/people.rs, schema: SettlementList }
    - { kind: http, name: "GET /v1/settlements/{id}", site: crates/arda-server/src/people.rs }
    - { kind: http, name: "GET /v1/settlements/{id}/plan", site: crates/arda-server/src/people.rs }
    - { kind: http, name: "GET /v1/settlements/{id}/npcs", site: crates/arda-server/src/people.rs, schema: SettlementNpcs }
    - { kind: http, name: "GET /v1/gazetteer", site: crates/arda-server/src/gazetteer.rs, schema: Gazetteer }
    - { kind: http, name: "GET /v1/realms", site: crates/arda-server/src/realms.rs, schema: RealmList }
    - { kind: http, name: "GET /v1/realms/{id}", site: crates/arda-server/src/realms.rs, schema: RealmDetail }
    - { kind: http, name: "GET /v1/schema", site: crates/arda-server/src/schema/routes.rs, schema: SchemaIndex }
    - { kind: http, name: "GET /v1/schema/{Name}.json", site: crates/arda-server/src/schema/routes.rs }
    - { kind: http, name: "GET /v1/tactical/layouts", site: crates/arda-server/src/tactical/routes.rs }
    - { kind: http, name: "GET /v1/tactical/layout/{name}", site: crates/arda-server/src/tactical/routes.rs }
    - { kind: http, name: "GET /v1/tactical/layout/{name}.png", site: crates/arda-server/src/tactical/routes.rs }
    - { kind: http, name: "GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp", site: crates/arda-server/src/tactical/routes.rs }
    - { kind: http, name: "POST /v1/tactical/render", site: crates/arda-server/src/tactical/routes.rs }
    - { kind: http, name: "GET /v1/tactical/cell/{gx}/{gy}", site: crates/arda-server/src/tactical/world_routes.rs, schema: TacticalBlockDto }
    - { kind: http, name: "GET /v1/tactical/cell/{gx}/{gy}.png", site: crates/arda-server/src/tactical/world_routes.rs }
    - { kind: http, name: "GET /v1/tactical/cell/{gx}/{gy}/scene", site: crates/arda-server/src/tactical/scene.rs, schema: TacticalScene }
    - { kind: http, name: "GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp", site: crates/arda-server/src/tactical/world_routes.rs }
    - { kind: http, name: "GET /v1/tactical/window", site: crates/arda-server/src/tactical/world_routes.rs, schema: TacticalBlockDto }
    - { kind: http, name: "GET /v1/tactical/window.png", site: crates/arda-server/src/tactical/world_routes.rs }
    - { kind: http, name: "GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp", site: crates/arda-server/src/tactical/world_routes.rs }
    - { kind: http, name: "GET /v1/tactical/library", site: crates/arda-server/src/tactical/world_routes.rs }
    - { kind: http, name: "POST /v1/tactical/prefetch", site: crates/arda-server/src/tactical/routes.rs, schema: PrefetchAccepted }
    - { kind: http, name: "POST /v1/tactical/bake", site: crates/arda-server/src/tactical/bake_routes.rs, schema: BakeAccepted }
    - { kind: http, name: "GET /v1/tactical/bake/{id}", site: crates/arda-server/src/tactical/bake_routes.rs, schema: BakeStatus }
    - { kind: http, name: "GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp", site: crates/arda-server/src/tactical/bake_routes.rs }
    - { kind: http, name: "DELETE /v1/tactical/bake/{id}", site: crates/arda-server/src/tactical/bake_routes.rs }
    - { kind: http, name: "GET /v1/tactical/dungeon", site: crates/arda-server/src/tactical/dungeon.rs, schema: DungeonDto }
    - { kind: http, name: "GET /v1/tactical/dungeon.png", site: crates/arda-server/src/tactical/dungeon.rs }
    - { kind: http, name: "GET /v1/tactical/dungeon/scene", site: crates/arda-server/src/tactical/dungeon.rs, schema: SceneDto }
    - { kind: schema, name: bindings/ts/arda, site: crates/arda-server/src/bindings.rs }
    - { kind: schema, name: bindings/schema, site: crates/arda-server/src/schema/mod.rs }
    - { kind: schema, name: contract-kit, site: contract-kit/manifest.json }
  consumes: []
---

# Interfaces

The only cross-repo surface is `arda-server`'s HTTP API under `/v1` (consumed by the DiceMaster VTT, a sibling repo) and the two committed schema packages generated from the same Rust types. Every route is registered in `router` (`crates/arda-server/src/routes.rs:28-76`) and the nested tactical router (`crates/arda-server/src/tactical/routes.rs:37-63`) (the bake routes merged from `crates/arda-server/src/tactical/bake_routes.rs:34-42`). `apps/viewer` calls the same API from inside this repo (`apps/viewer/src/api/client.ts:52`), so it is not an edge.

## Produces

| Kind | Name | To | Site |
|---|---|---|---|
| http | GET /v1/health | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/world | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/cell/{gx}/{gy} | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/area/{ax}/{ay}/cells | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/area/{ax}/{ay}/rivers | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/area/{ax}/{ay}/lakes | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/point | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/overview.png | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/tiles/overview/{z}/{x}/{y}.webp | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/tiles/overview/{z}/{x}/{y}.png | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/tiles/relief/{z}/{x}/{y}.webp | - | `crates/arda-server/src/routes.rs` |
| http | POST /v1/npc/population | - | `crates/arda-server/src/routes.rs` |
| http | POST /v1/npc/generate | - | `crates/arda-server/src/npc.rs` |
| http | GET /v1/npc/demo | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/npc/demo/{npc_id} | - | `crates/arda-server/src/routes.rs` |
| http | GET /v1/npc/{npc_id} | - | `crates/arda-server/src/npcs/mod.rs` |
| http | GET /v1/npcs | - | `crates/arda-server/src/npcs/mod.rs` |
| http | GET /v1/buildings/{building}/residents | - | `crates/arda-server/src/npcs/mod.rs` |
| http | GET /v1/buildings/{building}/workers | - | `crates/arda-server/src/npcs/mod.rs` |
| http | GET /v1/settlements | - | `crates/arda-server/src/people.rs` |
| http | GET /v1/settlements/{id} | - | `crates/arda-server/src/people.rs` |
| http | GET /v1/settlements/{id}/plan | - | `crates/arda-server/src/people.rs` |
| http | GET /v1/settlements/{id}/npcs | - | `crates/arda-server/src/people.rs` |
| http | GET /v1/gazetteer | - | `crates/arda-server/src/gazetteer.rs` |
| http | GET /v1/realms | - | `crates/arda-server/src/realms.rs` |
| http | GET /v1/realms/{id} | - | `crates/arda-server/src/realms.rs` |
| http | GET /v1/schema | - | `crates/arda-server/src/schema/routes.rs` |
| http | GET /v1/schema/{Name}.json | - | `crates/arda-server/src/schema/routes.rs` |
| http | GET /v1/tactical/layouts | - | `crates/arda-server/src/tactical/routes.rs` |
| http | GET /v1/tactical/layout/{name} | - | `crates/arda-server/src/tactical/routes.rs` |
| http | GET /v1/tactical/layout/{name}.png | - | `crates/arda-server/src/tactical/routes.rs` |
| http | GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp | - | `crates/arda-server/src/tactical/routes.rs` |
| http | POST /v1/tactical/render | - | `crates/arda-server/src/tactical/routes.rs` |
| http | GET /v1/tactical/cell/{gx}/{gy} | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/cell/{gx}/{gy}.png | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/cell/{gx}/{gy}/scene | - | `crates/arda-server/src/tactical/scene.rs` |
| http | GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/window | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/window.png | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | GET /v1/tactical/library | - | `crates/arda-server/src/tactical/world_routes.rs` |
| http | POST /v1/tactical/prefetch | - | `crates/arda-server/src/tactical/routes.rs` |
| http | POST /v1/tactical/bake | - | `crates/arda-server/src/tactical/bake_routes.rs` |
| http | GET /v1/tactical/bake/{id} | - | `crates/arda-server/src/tactical/bake_routes.rs` |
| http | GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp | - | `crates/arda-server/src/tactical/bake_routes.rs` |
| http | DELETE /v1/tactical/bake/{id} | - | `crates/arda-server/src/tactical/bake_routes.rs` |
| http | GET /v1/tactical/dungeon | - | `crates/arda-server/src/tactical/dungeon.rs` |
| http | GET /v1/tactical/dungeon.png | - | `crates/arda-server/src/tactical/dungeon.rs` |
| http | GET /v1/tactical/dungeon/scene | - | `crates/arda-server/src/tactical/dungeon.rs` |
| schema | bindings/ts/arda | - | `crates/arda-server/src/bindings.rs` |
| schema | bindings/schema | - | `crates/arda-server/src/schema/mod.rs` |
| schema | contract-kit | - | `contract-kit/manifest.json` |

Contract facts shared by every row:

| Fact | Value | Pointer |
|---|---|---|
| Cell contract version | `CONTRACT_VERSION: u32 = 3`; "Clients must reject a version they do not know"; bumped when a field changes meaning, unit or presence | `crates/arda-server/src/contract/mod.rs:4-5`, `crates/arda-server/src/contract/mod.rs:24-31` |
| Contract 3 change | `road` gains `footpath`; `built_by` is a settlement id string; `land_use` and `realm_id` added (null without `society/`) | `crates/arda-server/src/contract/mod.rs:26-30` |
| `contract_version` emitted in | `Health` (`routes.rs:202`), `WorldInfo` (`dto.rs:112`), `CellSample` (`contract/convert.rs:201`), `PointSample` (`query.rs:337`), `AreaCells` (`columnar.rs:271`), `AreaRivers` (`routes.rs:282`), `AreaLakes` (`routes.rs:298`) | `crates/arda-server/src/` |
| Versions in TS | `export const API_VERSION` and one constant per version family (`CONTRACT_VERSION`, `TACTICAL_FORMAT`, …) written into `index.ts` | `crates/arda-server/src/bindings.rs:76-84` |
| Version families | `WorldInfo.versions` (`VersionsDto`) lists every family; `WorldInfo.server_version` and `Health.server_version` name the serving build, `WorldInfo.arda_version` the generator that wrote the world; policy, stability per route and the schema freeze guard in [versioning.md](versioning.md) | `crates/arda-server/src/versions.rs` |
| API version | `API_VERSION = "v1"`; every route nests under `/v1` | `crates/arda-server/src/dto.rs:12`, `crates/arda-server/src/routes.rs:72` |
| Tactical envelope versions | `TACTICAL_FORMAT = 1`, `LAYOUT_SCHEMA = 1`, `SCENE_FORMAT = TACTICAL_FORMAT` | `crates/arda-server/src/tactical/dto.rs:90-92`, `crates/arda-server/src/tactical/scene.rs:24` |
| Unknown path | 404 `not_found` "every endpoint lives under /v1" | `crates/arda-server/src/routes.rs:195-197` |
| Error body | every 4xx/5xx is `ApiError { error: ApiErrorBody }`; 501 from unready tactical sources is `NotYetError` (adds `planned_source`) | `crates/arda-server/src/error.rs:80-100` |
| Error codes | `ServerError::classify` maps variants to status and code | `crates/arda-server/src/error.rs:121-154` |
| CORS | only `http(s)://localhost`, `127.0.0.1`, `[::1]` origins; methods GET/HEAD/POST/DELETE/OPTIONS; exposes `ETag`, `X-Arda-Cache`, `X-Arda-Sheet-Mapping` | `crates/arda-server/src/routes.rs:80-125` |
| Image caching | strong `ETag` (BLAKE3 of bytes), `304` on `If-None-Match`, `Cache-Control: no-cache` | `crates/arda-server/src/routes.rs:157-193`, `crates/arda-server/src/tactical/routes.rs:147-182` |
| Sheet mapping | with `--sheet-mapping`, every NPC-bearing body is reshaped and carries `X-Arda-Sheet-Mapping`; the `Npc` schemas then no longer apply | `crates/arda-server/src/sheet_map/mod.rs:263-277`, `crates/arda-server/src/schema/mod.rs:176` |
| Bind address | `--host` default `127.0.0.1`, `--port` default `8787` | `crates/arda-server/src/main.rs:24-29` |
| Deploy config | `Dockerfile` builds `arda` and `arda-server` (`cargo build --release --locked -p arda-cli -p arda-server`), runs `ENTRYPOINT ["arda-entrypoint"]` (`tools/docker-entrypoint.sh`: with no arguments it serves `/worlds/world` on `0.0.0.0:8787`), `EXPOSE 8787` and a `HEALTHCHECK` running `arda-server --probe`; no compose, Kubernetes or ingress files name the server | `Dockerfile:21`, `Dockerfile:34-43`, `tools/docker-entrypoint.sh:9-22` |
| Authentication | none; no auth layer in the router | `crates/arda-server/src/routes.rs:28-76` |

### GET /v1/health

Type `Health` (`crates/arda-server/src/dto.rs:144`).

| Field | Type | Required |
|---|---|---|
| status | String | yes |
| contract_version | u32 | yes |
| seed | String | yes |
| server_version | String | yes |

### GET /v1/world

Model: WorldInfo

### GET /v1/cell/{gx}/{gy}

Model: CellSample

### GET /v1/area/{ax}/{ay}/cells

Type `AreaCells` (`crates/arda-server/src/columnar.rs:237`) as `application/json` (default, `?format=json`); `?format=bin` returns the same body as `application/octet-stream` in the `ARDACOLS` v2 layout (`crates/arda-server/src/columnar.rs:283-287`, `crates/arda-server/src/routes.rs:209-242`).

| Field | Type | Required |
|---|---|---|
| contract_version | u32 | yes |
| ax | i32 | yes |
| ay | i32 | yes |
| gx0 | u32 | yes |
| gy0 | u32 | yes |
| width | u32 | yes |
| height | u32 | yes |
| legend | Legend | yes |
| columns | AreaColumns | yes |

### GET /v1/area/{ax}/{ay}/rivers

Type `AreaRivers` (`crates/arda-server/src/dto.rs:225`).

| Field | Type | Required |
|---|---|---|
| contract_version | u32 | yes |
| ax | i32 | yes |
| ay | i32 | yes |
| rivers | Vec<RiverDto> | yes |

### GET /v1/area/{ax}/{ay}/lakes

Type `AreaLakes` (`crates/arda-server/src/dto.rs:238`).

| Field | Type | Required |
|---|---|---|
| contract_version | u32 | yes |
| ax | i32 | yes |
| ay | i32 | yes |
| lakes | Vec<LakeDto> | yes |

### GET /v1/point

Model: PointSample

### GET /v1/overview.png

Response `image/png` (`crates/arda-server/src/routes.rs:321-339`). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| quality | u32 | no | long edge in px, 512 doubling up to `--max-overview-px` (`crates/arda-server/src/overview.rs:168-188`) |
| style | String | no | `atlas` (default), `classic`, `atlas-oblique` (`crates/arda-server/src/overview.rs:42-64`) |
| overlay | String | no | `society` or `none`; default `society` when the world has `society/` (never with `atlas-oblique`); `society` without society files or with oblique is 400 |

### GET /v1/tiles/overview/{z}/{x}/{y}.webp

Response `image/webp`; `z` above 31 is 404 (`crates/arda-server/src/routes.rs:369-407`). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| oblique | 0 or 1 | no | serves the oblique pyramid |
| overlay | String | no | `society` or `none`; as on `/v1/overview.png` (`overlay_flag`, `crates/arda-server/src/routes.rs:344-354`) |
| labels | 0 or 1 | no | default 1; `0` leaves the gazetteer's names off the tile; other values are 400 (`labels_flag`, `crates/arda-server/src/routes.rs:357-366`) |

### GET /v1/tiles/overview/{z}/{x}/{y}.png

Response `image/png`; same handler and parameters as the `.webp` form (`crates/arda-server/src/routes.rs:369-407`).

| Field | Type | Required | Notes |
|---|---|---|---|
| oblique | 0 or 1 | no | serves the oblique pyramid |
| overlay | String | no | `society` or `none`; as on `/v1/overview.png` |
| labels | 0 or 1 | no | as on the `.webp` form |

### GET /v1/tiles/relief/{z}/{x}/{y}.webp

Response `image/webp`, mid-zoom relief past the overview's native zoom (`crates/arda-server/src/routes.rs:410-428`).

| Field | Type | Required | Notes |
|---|---|---|---|
| z, x, y | u32 path segments | yes | `{y}` must end in `.webp` |
| labels | 0 or 1 | no | default 1; `0` omits the gazetteer's names |

### POST /v1/npc/population

Model: Population

Request body: `PopulationRequest` (`crates/arda-server/src/npc_dto/mod.rs:327`), limited to `MAX_BODY_BYTES` (`crates/arda-server/src/routes.rs:40-43`, `crates/arda-server/src/routes.rs:430-449`).

### POST /v1/npc/generate

Model: Npc

Request body: `GenerateNpcRequest` (`crates/arda-server/src/npc_dto/mod.rs:340`; wire struct `crates/arda-server/src/npc.rs:39`), every field optional, unknown fields refused, at most `MAX_GENERATE_BODY_BYTES` = 16 KiB (`crates/arda-server/src/npc.rs:33`), `Content-Type: application/json` required (415). Handler `generate_route` (`crates/arda-server/src/npc.rs:217`) converts it to `arda_npc::OneRequest` (`settlement_id` resolved through `arda_people::World::profile`) and calls `arda_npc::generate_one` with the served world seed; the response goes through `AppState::npc_response(.., NpcAt::Root)`, so `--sheet-mapping` applies. Nothing is stored; `id`, `home_building` and `household` are synthetic (settlement 0) and never resolve.

| Field | Type | Required | Notes |
|---|---|---|---|
| seed | String | no | decimal u64 salt, default `"0"` |
| job | String | no | `occupations.json` job key |
| office | String | no | `occupations.json` office kind |
| rank | SocialRank | no | |
| ancestry | String | no | SRD race key |
| sex | Sex | no | |
| tier | Tier | no | default: borrowed, else village, else smallest tier with the job |
| culture | String | no | `cultures.json` key |
| settlement_id | String | no | decimal id; 404 unknown or without `society/` |
| notable | bool | no | |

400 `bad_request` (malformed seed or id, unknown field or key, a rank or tier the job lacks), 404 `not_found`, 413, 415, 422 `unsatisfiable`.

### GET /v1/npc/demo

Type `NpcDemo` (`crates/arda-server/src/npc_dto/mod.rs:386`; wire struct `crates/arda-server/src/npc.rs:86`).

| Field | Type | Required |
|---|---|---|
| world_seed | String | yes |
| settlement | SettlementProfile | yes |
| buildings | Vec<BuildingSpec> | yes |
| population | Population | yes |

### GET /v1/npc/demo/{npc_id}

Model: Npc

### GET /v1/npc/{npc_id}

Model: Npc

Optional `?settlement=`; `{npc_id}` is an id or a `<settlement>.<building>.<index>` reference (`crates/arda-server/src/npcs/mod.rs:177-231`).

### GET /v1/npcs

Model: NpcPage

Query: `settlement`, `building`, `job`, `realm`, `notable`, `limit`, `cursor` (`crates/arda-server/src/npcs/mod.rs:72-116`).

### GET /v1/buildings/{building}/residents

Model: NpcPage

### GET /v1/buildings/{building}/workers

Model: NpcPage

### GET /v1/settlements

Model: SettlementList

Built as ad-hoc JSON `{ "format_version": 1, "settlements": [...] }` filtered by `?tier=` and `?realm=`, at most `MAX_LISTED = 20_000` records (`crates/arda-server/src/people.rs:28`, `crates/arda-server/src/people.rs:51-69`).

### GET /v1/settlements/{id}

Schema type `SettlementDetail` (`crates/arda-server/src/schema/bodies.rs:20`); the handler builds it as ad-hoc JSON (`crates/arda-server/src/people.rs:72-86`).

| Field | Type | Required | Notes |
|---|---|---|---|
| record | arda_settle::model::Settlement | yes | |
| society | Option<arda_society::SettlementSociety> | no | `null` when the society layer has none |

### GET /v1/settlements/{id}/plan

Type `arda_town::TownPlan` (`crates/arda-town/src/plan/types.rs:293`), serialised as is (`crates/arda-server/src/people.rs:89-103`); 404 when the settlement has no plan.

| Field | Type | Required | Notes |
|---|---|---|---|
| site | SettlementId | yes | |
| name | String | yes | |
| tier | Tier | yes | |
| culture | String | yes | |
| seed | u64 | yes | JSON string |
| base_height_m | f64 | yes | |
| water_level_m | f64 | yes | |
| focal | Focal | yes | |
| streets | Vec<Street> | yes | |
| bridges | Vec<Bridge> | no | serde default |
| square | Option<MarketSquare> | no | |
| plots | Vec<Plot> | yes | |
| districts | Vec<(DistrictKind, Vec<PlotId>)> | yes | |
| wall | Option<TownWall> | no | |
| castle | Option<CastleWard> | no | |
| buildings | Vec<Building> | yes | |
| notes | Vec<String> | yes | |
| fabric | Fabric | no | serde default |

### GET /v1/settlements/{id}/npcs

Model: SettlementNpcs

### GET /v1/gazetteer

Model: Gazetteer

Handler `get` (`crates/arda-server/src/gazetteer.rs:68`), filters in `filter` (`crates/arda-server/src/gazetteer.rs:22-66`). Typed `GazetteerList { format_version: 1, features: [GazetteerFeature | GazetteerPoint] }`: the natural features of `society/gazetteer.json` in file order, then the settlements by id as points (`kind` `hamlet`..`city`, `at_m`, `rank` 5..2). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | String | no | comma list (`sea`, `lake`, `range`, `city`, ...); unknown kind is 400 |
| bbox | String | no | `x0,y0,x1,y1` in metres with `x0 <= x1` and `y0 <= y1`; else 400 |
| min_rank | u32 | no | 1 to 5, keeps rank at most this |

404 `not_found` without `society/` or `gazetteer.json`.

### GET /v1/realms

Model: RealmList

Typed `RealmList { format_version: 1, realms: RealmSummary[] }` in id order, at most `MAX_LISTED` entries (`crates/arda-server/src/realms.rs`). Summaries omit `borders`; `style`, `rank`, `government_label` and `ruler` are absent when `society.json` does not exist. 404 `not_found` without `society/`.

### GET /v1/realms/{id}

Model: RealmDetail

| Field | Type | Required | Notes |
|---|---|---|---|
| record | arda_settle::realms::Realm | yes | with `borders` |
| politics | Option<arda_society::RealmSociety> | no | `null` before `arda society build` |
| relations | Vec<arda_society::politics::relations::Relation> | yes | those involving the id |
| events | Vec<arda_society::history::Event> | yes | those listing the id in `realms` |
| reigns | Vec<arda_society::history::Reign> | yes | of this realm |

404 `not_found` for an unknown id, 400 `bad_request` for a non-numeric one.

### GET /v1/schema

Model: SchemaIndex

Response `application/json`, the committed `bindings/schema/index.json` (`crates/arda-server/src/schema/routes.rs:34-39`); type `SchemaIndex` with `SchemaIndexEntry` items (`crates/arda-server/src/schema/mod.rs`, `index()` builds it).

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | u32 | yes | `INDEX_FORMAT = 1` |
| api_version | String | yes | `"v1"` |
| dialect | String | yes | JSON Schema draft 2020-12 |
| note | String | yes | sheet-mapping caveat |
| schemas | Vec<SchemaIndexEntry> `{ name, url, routes }` | yes | one per catalog entry, `SchemaIndex` itself included |

### GET /v1/schema/{Name}.json

Response `application/schema+json`, `Cache-Control: public, max-age=3600`, one committed file from `bindings/schema/` byte for byte; 404 for a name without `.json` or not in the catalog (`crates/arda-server/src/schema/routes.rs:11-51`).

| Field | Type | Required | Notes |
|---|---|---|---|
| Name | path segment | yes | a `catalog!` name (`crates/arda-server/src/schema/mod.rs:115-166`) |

### GET /v1/tactical/layouts

Type `TacticalLayouts` (`crates/arda-server/src/tactical/dto.rs:14`).

| Field | Type | Required |
|---|---|---|
| world_seed | String | yes |
| library_version | String | yes |
| ppsq_options | Vec<u32> | yes |
| default_ppsq | u32 | yes |
| layouts | Vec<TacticalLayoutSummary> | yes |

### GET /v1/tactical/layout/{name}

A built-in `arda_tactical::TacticalLayout`, described by `TacticalLayoutDto` (`crates/arda-server/src/tactical/dto.rs:97`); `Cache-Control: no-cache` (`crates/arda-server/src/tactical/routes.rs:193-222`).

| Field | Type | Required | Notes |
|---|---|---|---|
| name | String | yes | |
| width | u32 | yes | |
| height | u32 | yes | |
| squares | Vec<SquareDto> | yes | row-major |
| walls | Vec<WallSegmentDto> | no | serde default |
| placements | Vec<PlacementDto> | no | serde default |
| lights | Vec<LightSourceDto> | no | serde default |
| origin | Option<[i64; 2]> | no | omitted when none |

### GET /v1/tactical/layout/{name}.png

Response `image/png` with `ETag`, `X-Arda-Cache`, `Server-Timing` (`crates/arda-server/src/tactical/routes.rs:193-222`). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| ppsq | u32 | no | 64, 96 or 128 (default 128) (`crates/arda-server/src/tactical/mod.rs:53-55`) |
| grid | 0 or 1 | no | |

### GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp

Response `image/webp` (`crates/arda-server/src/tactical/routes.rs:243-279`). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| ppsq | u32 | no | 64, 96 or 128 |
| grid | 0 or 1 | no | |

### POST /v1/tactical/render

Request: a `TacticalLayoutDto` JSON body with `Content-Type: application/json` (else 415), bounded by `max_body_bytes`; response `image/png` (`crates/arda-server/src/tactical/routes.rs:281-337`). Query parameters:

| Field | Type | Required | Notes |
|---|---|---|---|
| ppsq | u32 | no | 64, 96 or 128 |
| grid | 0 or 1 | no | |
| origin | String | no | `X,Y` in world squares; only this route accepts it |

### GET /v1/tactical/cell/{gx}/{gy}

Model: TacticalBlockDto

Query: `ppsq` (64/96/128 or 16/32), `grid`, `demo_overlays`, `demo_at`, `world_grade` (`crates/arda-server/src/tactical/world_routes.rs:67-100`, `crates/arda-server/src/tactical/world_routes.rs:148-187`).

### GET /v1/tactical/cell/{gx}/{gy}.png

Response `image/png`, same handler as the JSON form (`crates/arda-server/src/tactical/world_routes.rs:148-187`).

| Field | Type | Required | Notes |
|---|---|---|---|
| ppsq | u32 | no | 64, 96, 128, 16 or 32 (default 128) |
| grid | 0 or 1 | no | |
| demo_overlays | 0 or 1 | no | |
| demo_at | String | no | `GX,GY` |
| world_grade | 0 or 1 | no | |

### GET /v1/tactical/cell/{gx}/{gy}/scene

Model: TacticalScene

Query: `time` = `day` (default) or `night`, plus the cell options (`crates/arda-server/src/tactical/scene.rs:93-115`).

### GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp

Response `image/webp` (`crates/arda-server/src/tactical/world_routes.rs:189-227`).

| Field | Type | Required | Notes |
|---|---|---|---|
| ppsq | u32 | no | 64, 96, 128, 16 or 32 (default 128) |
| grid | 0 or 1 | no | |
| world_grade | 0 or 1 | no | |

### GET /v1/tactical/window

Model: TacticalBlockDto

Query: `gx0,gy0,w,h` in cells (`w`, `h` 1..=3) or `gsx,gsy,w,h` in squares (1..=192); `ppsq` default 32 (`crates/arda-server/src/tactical/world_routes.rs:37-39`, `window_request`, `window`). `tiles` describes the pyramid served by `GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp` with the same query.

### GET /v1/tactical/window.png

Response `image/png` (`crates/arda-server/src/tactical/world_routes.rs:341-368`).

| Field | Type | Required | Notes |
|---|---|---|---|
| gx0, gy0 | u32 | no | cell window origin (with `w`, `h` 1..=3) |
| gsx, gsy | u32 | no | square window origin (with `w`, `h` 1..=192) |
| w, h | u32 | yes | |
| ppsq | u32 | no | default 32 |
| grid | 0 or 1 | no | |
| world_grade | 0 or 1 | no | |

### GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp

Response `image/webp`: 512 px lossless tiles cut from the `/window.png` render of the same query, `z = max_zoom` the full render; 404 outside the pyramid or for a tile not named `{y}.webp` (`window_tile` in `crates/arda-server/src/tactical/world_routes.rs`). Query as `GET /v1/tactical/window.png`; the `tiles` block of `GET /v1/tactical/window` describes the pyramid.

| Field | Type | Required | Notes |
|---|---|---|---|
| z, x, y | u32 | yes | path segments; `y` carries the `.webp` suffix; `z = max_zoom` is the full render |
| gx0, gy0 | u32 | no | cell window origin (with `w`, `h` 1..=3) |
| gsx, gsy | u32 | no | square window origin (with `w`, `h` 1..=192) |
| w, h | u32 | yes | |
| ppsq | u32 | no | default 32 |
| grid | 0 or 1 | no | |
| world_grade | 0 or 1 | no | |

### GET /v1/tactical/library

Type `TacticalLibraryDto` (`crates/arda-server/src/tactical/library_dto.rs:12`).

| Field | Type | Required |
|---|---|---|
| library | String | yes |
| library_version | String | yes |
| pixels_per_square | u32 | yes |
| grounds | Vec<GroundKeyDto> | yes |
| wall_kits | Vec<WallKitDto> | yes |
| assets | Vec<LibraryAssetDto> | yes |
| effects | Vec<EffectDto> | yes |

`EffectDto` is `{ id, kind, footprint, tags, variants }`: one entry per effect set (`effect.frost`, `kind` `texture` or `decal`, 2 x 2 squares), whose images are `{id}.{n}.png` for `n < variants`. Added with 1.0 task 05; additive, so no version bump (logic/16 §api-versioning).

### GET /v1/tactical/library/asset/{id}.png

Response `image/png`: the image of any asset id in the loaded stacked library (`library_asset::asset_png`, `crates/arda-server/src/tactical/library_asset.rs`), RGB when fully opaque and RGBA otherwise. Strong `ETag`, `Cache-Control: no-cache`, `304` on `If-None-Match`. 400 `bad_request` for a malformed id, 404 `not_found` for an unknown one.

### POST /v1/tactical/prefetch

Model: PrefetchAccepted

Request body `PrefetchRequest` (`crates/arda-server/src/tactical/prefetch.rs:44`); answers `202 Accepted` at once (`crates/arda-server/src/tactical/routes.rs:66-92`).

### POST /v1/tactical/bake

Model: BakeAccepted

Request body `BakeRequest` (`crates/arda-server/src/tactical/bake_dto.rs:15`; handler `submit`, `crates/arda-server/src/tactical/bake_routes.rs:54-96`), `application/json` only (415), at most `max_body_bytes`, unknown fields refused; checked by `Tactical::check_bake` (`crates/arda-server/src/tactical/bake_render.rs:24`) and queued by `Baker::submit` (`crates/arda-server/src/tactical/bake.rs`). Answers `202 Accepted` (`200` once the job is `done` or `failed`) with `Location: status_url`; 503 `busy` when 8 jobs wait; 501 with `--bake-workers 0`.

| Field | Type | Required |
|---|---|---|
| format_version | `u32` | yes |
| job_id | `String` (32 hex digits) | yes |
| state | BakeState (`queued`, `running`, `done`, `failed`) | yes |
| status_url | `String` | yes |

Request body fields (`BakeRequest`, `crates/arda-server/src/tactical/bake_dto.rs:15-35`):

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | u32 | no | `1` when present |
| gx, gy | u32 | yes | cell column and row |
| ppsq | u32 | no | 64, 96 or 128 (default 128) |
| world_grade | bool | no | as `?world_grade=1` on the cell route (default false) |
| layout | TacticalLayoutDto | yes | exactly 64 x 64 squares with `origin` equal to `[gx * 64, gy * 64]` (422 `invalid_layout` otherwise) |

### GET /v1/tactical/bake/{id}

Model: BakeStatus

`Cache-Control: no-store` (`BAKE_CACHE_CONTROL`, `crates/arda-server/src/tactical/bake_routes.rs:31`, handler `status` at `crates/arda-server/src/tactical/bake_routes.rs:98-108`); 404 `not_found` for unknown, deleted or evicted jobs (`Baker::status`, `crates/arda-server/src/tactical/bake.rs:249`). The tiles and the delete have their own sections below.

| Field | Type | Required |
|---|---|---|
| format_version | `u32` | yes |
| job_id | `String` | yes |
| state | BakeState | yes |
| gx, gy | `u32` | yes |
| origin_gs | `[i64; 2]` | yes |
| world_grade | `bool` | yes |
| tiles | Option<TacticalTilesDto> | yes (null until done) |
| tile_count | `u32` | yes |
| total_bytes | `u32` | yes |
| error | Option<ApiErrorBody> | yes (null unless failed) |

### GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp

Response `image/webp`, one 512 px lossless tile of the pyramid `BakeStatus.tiles` describes, with a strong `ETag`, `If-None-Match` answered `304` and `Cache-Control: no-store` (handler `tile`, `crates/arda-server/src/tactical/bake_routes.rs:118-153`). 404 `not_found` for an unknown id, a job not yet `done`, a tile outside the pyramid, or a last segment not ending in `.webp` (`Baker::tile`, `crates/arda-server/src/tactical/bake.rs:262-279`).

| Field | Type | Required | Notes |
|---|---|---|---|
| id | String | yes | path segment, the `job_id` of `POST /v1/tactical/bake` |
| z, x, y | u32 | yes | path segments; `y` carries the `.webp` suffix; no query parameters |

### DELETE /v1/tactical/bake/{id}

Answers `204 No Content` with no body and frees the job and its tiles; a queued job is dropped and a running job's result is discarded when it finishes (handler `delete`, `crates/arda-server/src/tactical/bake_routes.rs:110-116`; `Baker::delete`, `crates/arda-server/src/tactical/bake.rs:284-296`). 404 `not_found` for an unknown, already deleted or evicted id. CORS allows `DELETE` from localhost origins (`crates/arda-server/src/routes.rs:108-114`).

| Field | Type | Required | Notes |
|---|---|---|---|
| id | String | yes | path segment, the `job_id` of `POST /v1/tactical/bake` |

### GET /v1/tactical/dungeon

Model: DungeonDto

The handler serialises `arda_dungeon::Dungeon` (`crates/arda-dungeon/src/lib.rs`), described by the mirror `DungeonDto` (`crates/arda-server/src/tactical/dungeon_dto.rs`); level chosen by `?seed=` or `?gx=&gy=`, `kind` = `dungeon`|`cave`, `w`/`h` default 48 × 36 (`crates/arda-server/src/tactical/dungeon.rs:32-89`). `params.seed` is a decimal string (`arda_scene::serde_str`).

| Field | Type | Required |
|---|---|---|
| params | DungeonParamsDto `{ seed: String, kind, width, height }` | yes |
| layout | TacticalLayoutDto | yes |
| rules | RulesSidecarDto | yes |
| rooms | Vec<DungeonRoomDto> | yes |
| doors | Vec<DungeonDoorDto> | yes |
| exits | Vec<DungeonExitDto> | yes |

### GET /v1/tactical/dungeon.png

Response `image/png` (`crates/arda-server/src/tactical/dungeon.rs:110-161`).

| Field | Type | Required | Notes |
|---|---|---|---|
| seed | u64 | no | or `gx` + `gy` |
| gx, gy | u32 | no | world cell |
| kind | String | no | `dungeon` or `cave` |
| w, h | u32 | no | default 48 × 36 |
| ppsq | u32 | no | 64, 96 or 128 |
| grid | 0 or 1 | no | |

### GET /v1/tactical/dungeon/scene

Model: SceneDto

An `arda_scene::Scene`, described by `SceneDto` (`crates/arda-server/src/tactical/scene_dto.rs:227`); same level parameters as `/dungeon` (`crates/arda-server/src/tactical/dungeon.rs:91-107`).

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | u32 | yes | |
| name | String | yes | |
| width | u32 | yes | |
| height | u32 | yes | |
| seed | String | yes | |
| library | String | yes | |
| library_version | String | yes | |
| movement | Runs<MovementDto> | yes | |
| climb | Runs<u8> | yes | |
| cover | Runs<CoverLevelDto> | yes | |
| obscured | Runs<ObscurementDto> | yes | |
| elevation_ft | Runs<i16> | yes | |
| water_depth_ft | Runs<u8> | yes | |
| walls | Vec<SceneWallDto> | yes | |
| vision_blockers | Vec<VisionBlockerDto> | yes | |
| lights | Vec<SceneLightDto> | yes | |
| regions | Vec<RegionDto> | yes | |
| spawn_hints | SpawnHintsDto | yes | |
| tokens | Vec<TokenSlotDto> | yes | |
| origin_gs | Option<[i64; 2]> | no | omitted when none |

### bindings/ts/arda

ts-rs TypeScript declarations of every wire type, committed under `bindings/ts/arda/` (`BINDINGS_DIR`, `crates/arda-server/src/bindings.rs:47`); `cargo test -p arda-server bindings` fails when stale, `ARDA_BLESS_BINDINGS=1` rewrites them (`crates/arda-server/src/bindings.rs:1-5`). No `package.json`; consumers import the files directly.

| Field | Type | Required | Notes |
|---|---|---|---|
| CONTRACT_VERSION | number const | yes | `index.ts`, the first constant of `VersionsDto::current().entries()` (`crates/arda-server/src/bindings.rs:77-84`) |
| API_VERSION | string const | yes | `index.ts` (`crates/arda-server/src/bindings.rs:76`) |
| `export type { <Name> }` | type re-export | yes | one per exported type in `exported!` (`crates/arda-server/src/bindings.rs:63-94`) |

### bindings/schema

JSON Schema (draft 2020-12) files, one `<Name>.json` per body plus `index.json`, generated with schemars from the same Rust types and served by `GET /v1/schema` (`crates/arda-server/src/schema/mod.rs:1-11`, `crates/arda-server/src/schema/mod.rs:25`); `bindings/schema-baseline/v1/` holds the frozen copy the freeze guard compares against (`versioning.md`); a schema's name equals the TypeScript type name.

| Field | Type | Required | Notes |
|---|---|---|---|
| `<Name>.json` | JSON Schema document | yes | one per `catalog!` entry, 45 entries (`crates/arda-server/src/schema/mod.rs:115-166`) |
| index.json | object | yes | see `GET /v1/schema` |

### contract-kit

Golden responses of the canonical world (seed 42, MICRO, fine terrain, society) for the stable routes, plus a live checker; `contract-kit/manifest.json` carries `kit_format` 1, the fixture identity (`arda_version`, `server_version`, BLAKE3 of `world.json`, `library_version`, `contract_version`) and one entry per case (`contract-kit/manifest.json:1-12`, `contract-kit/README.md:1-24`). `contract-kit/check.mjs` (Node 22, standard library only) replays the cases against a running server; `crates/arda-server/tests/contract_kit.rs` replays them in-process and fails when a stable route of [versioning.md](versioning.md) has no case (`ARDA_BLESS_CONTRACT=1` re-records).

| Field | Type | Required | Notes |
|---|---|---|---|
| manifest.json | JSON | yes | kit format, fixture identity, cases |
| golden/ | JSON bodies | yes | responses byte for byte as served; large bodies and images are stored as SHA-256 and size |
| requests/ | JSON bodies | yes | bodies the `POST` cases send (`npc-generate`, `npc-population`, `tactical-bake`, `tactical-prefetch`) |
| world.txt | text | yes | `<path>  <blake3>` of every file of the canonical world |

## Consumes

| Kind | Name | From | Site |
|---|---|---|---|

None found. No Rust crate depends on an HTTP client (`reqwest`, `ureq` absent from `crates/*/Cargo.toml`); the only outbound network call is `tools/art-gen/generate.mjs:438`, which talks to an external image-generation vendor (covered in `05-dependencies.md`), not a sibling repo.
