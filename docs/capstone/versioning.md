# Versioning and compatibility policy

From 1.0, clients such as DiceMaster code against `/v1` and pin `(seed, arda_version, server_version)`. This page says what is frozen, which version number covers which body, what each kind of release may change, and what a server does with an older or newer world or library. It is normative. The byte-level route reference is [`crates/arda-server/API.md`](../../crates/arda-server/API.md); the rules every route follows are in [logic/16](logic/16-service-api.md).

The rules are held by tests:

- `crates/arda-server/src/schema/tests.rs` `frozen_schemas_keep_their_shape_unless_their_family_is_bumped` compares the served JSON Schemas with the committed baseline `bindings/schema-baseline/v1/` (see [Freeze guard](#freeze-guard)).
- `crates/arda-server/tests/world_api/inventory.rs` checks that every **stable** route in the table below answers on the MICRO seed-42 world with something other than `404` or `405`. It reads this page, so a stable row without a probe fails as well.
- `every_open_enum_exists_and_the_policy_page_lists_it` checks the open-enum table against `OPEN_ENUMS` (`crates/arda-server/src/versions.rs`).
- `crates/arda-server/tests/contract_kit.rs` replays the contract kit (`contract-kit/`): the exact bytes of every stable route but a finished bake's tiles on the canonical MICRO-42 world, for this release and the placeholder library; `every_stable_route_of_the_policy_has_a_case` fails when a stable row has no case. Clients pin the kit with the release and check a server with `contract-kit/check.mjs`.

## Route review (1.0 freeze)

Every route `crates/arda-server/src/routes.rs` and `crates/arda-server/src/tactical/routes.rs` (with `tactical/bake_routes.rs`) register, one row per method and documented path. Paths that share a registered route are listed apart: `/cell/{gx}/{gy}` also serves `{gy}.png`, `/layout/{file}` serves `{name}` and `{name}.png`, and every `tiles/{z}/{x}/{y}` route takes the format as the suffix of `{y}`. Checked by hand against both routers on 2026-10-13: 49 rows, of which **42 stable**, **1 unstable** and **6 debug**. `HEAD` is answered wherever `GET` is, and `OPTIONS` is the CORS preflight; neither has a row.

Stability:

- **stable**: frozen for 1.x under this policy.
- **unstable**: served, documented, outside the freeze. It may change shape in a minor release; the changelog says so.
- **debug**: samples and test seams. It may change or disappear in any release. Clients must not depend on it.

Every route can also answer `500 internal`; the table lists the other codes. "DiceMaster" is whether the contract's "What DiceMaster calls" table lists the route (`contract-dicemaster.md`, local); "planned" means the contract still lists it as planned.

| Route | Stability | Response | Family | Error codes | DiceMaster |
|---|---|---|---|---|---|
| `GET /v1/health` | stable | `Health` | `contract_version` | — | yes |
| `GET /v1/world` | stable | `WorldInfo` | `contract_version` (+ `versions`) | — | yes |
| `GET /v1/cell/{gx}/{gy}` | stable | `CellSample` | `contract_version` | `bad_request`, `out_of_range`, `not_found` | yes |
| `GET /v1/point` | stable | `PointSample` | `contract_version` | `bad_request`, `out_of_range` | yes |
| `GET /v1/area/{ax}/{ay}/cells` | stable | `AreaCells`; `ARDACOLS` with `?format=bin` | `contract_version`, `columnar_layout_version` | `bad_request`, `out_of_range`, `not_found`, `resource_limit` | yes |
| `GET /v1/area/{ax}/{ay}/rivers` | stable | `AreaRivers` | `contract_version` | `bad_request`, `out_of_range`, `not_found` | yes |
| `GET /v1/area/{ax}/{ay}/lakes` | stable | `AreaLakes` | `contract_version` | `bad_request`, `out_of_range`, `not_found` | yes |
| `GET /v1/overview.png` | stable | PNG | — (content) | `bad_request`, `resource_limit` | yes |
| `GET /v1/tiles/overview/{z}/{x}/{y}.webp` | stable | 256 px WebP | — (content) | `bad_request`, `not_found` | yes |
| `GET /v1/tiles/overview/{z}/{x}/{y}.png` | stable | 256 px PNG | — (content) | `bad_request`, `not_found` | yes |
| `GET /v1/tiles/relief/{z}/{x}/{y}.webp` | stable | 256 px WebP | — (content) | `bad_request`, `not_found` | yes |
| `GET /v1/settlements` | stable | `SettlementList` | `settlement_format` | `bad_request`, `not_found` | yes |
| `GET /v1/settlements/{id}` | stable | `SettlementDetail` | `settlement_format` | `bad_request`, `not_found` | yes |
| `GET /v1/settlements/{id}/plan` | stable | `TownPlan` | `settlement_format` | `bad_request`, `not_found` | yes |
| `GET /v1/settlements/{id}/npcs` | stable | `SettlementNpcs` | `settlement_format`, `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | yes |
| `GET /v1/npc/{npc_id}` | stable | `Npc` | `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | yes |
| `GET /v1/npcs` | stable | `NpcPage` | `npc_page_format`, `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | yes |
| `GET /v1/buildings/{building}/residents` | stable | `NpcPage` | `npc_page_format`, `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | yes |
| `GET /v1/buildings/{building}/workers` | stable | `NpcPage` | `npc_page_format`, `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | yes |
| `POST /v1/npc/population` | stable | `Population` (body `PopulationRequest`) | `npc_format` | `bad_request`, `payload_too_large`, `unsupported_media_type`, `sheet_mapping` | yes |
| `POST /v1/npc/generate` | stable | `Npc` (body `GenerateNpcRequest`) | `npc_format` | `bad_request`, `not_found`, `payload_too_large`, `unsupported_media_type`, `unsatisfiable`, `sheet_mapping` | no (planned) |
| `GET /v1/npc/demo` | debug | `NpcDemo` | `npc_format` | `sheet_mapping` | no |
| `GET /v1/npc/demo/{npc_id}` | debug | `Npc` | `npc_format` | `bad_request`, `not_found`, `sheet_mapping` | no |
| `GET /v1/gazetteer` | stable | `Gazetteer` | `gazetteer_format` | `bad_request`, `not_found` | no (planned) |
| `GET /v1/realms` | stable | `RealmList` | `realm_format` | `not_found` | no (planned) |
| `GET /v1/realms/{id}` | stable | `RealmDetail` | `realm_format` | `bad_request`, `not_found` | no (planned) |
| `GET /v1/schema` | stable | `SchemaIndex` | `schema_index_format` | — | yes |
| `GET /v1/schema/{name}.json` | stable | a JSON Schema | the schema's own family | `not_found` | yes |
| `GET /v1/tactical/layouts` | debug | `TacticalLayouts` | `tactical_format` | — | no |
| `GET /v1/tactical/layout/{name}` | debug | `TacticalLayoutDto` | `layout_schema` | `not_found` | no |
| `GET /v1/tactical/layout/{name}.png` | debug | PNG | — (content) | `bad_request`, `not_found`, `payload_too_large`, `resource_limit` | no |
| `GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp` | debug | 512 px WebP | — (content) | `bad_request`, `not_found` | no |
| `POST /v1/tactical/render` | unstable | PNG (body `TacticalLayoutDto`) | `layout_schema` | `bad_request`, `payload_too_large`, `unsupported_media_type`, `invalid_layout`, `resource_limit` | no |
| `GET /v1/tactical/cell/{gx}/{gy}` | stable | `TacticalBlockDto` | `tactical_format`, `layout_schema`, `rules_sidecar_format`, `scene_format` | `bad_request`, `out_of_range`, `no_block`, `resource_limit` | yes |
| `GET /v1/tactical/cell/{gx}/{gy}.png` | stable | PNG | — (content) | `bad_request`, `out_of_range`, `no_block`, `payload_too_large`, `resource_limit` | yes |
| `GET /v1/tactical/cell/{gx}/{gy}/scene` | stable | `TacticalScene` | `tactical_format`, `scene_format` | `bad_request`, `out_of_range`, `no_block` | yes |
| `GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp` | stable | 512 px WebP | — (content) | `bad_request`, `out_of_range`, `no_block`, `not_found`, `payload_too_large` | yes |
| `GET /v1/tactical/window` | stable | `TacticalBlockDto` | `tactical_format`, `layout_schema`, `rules_sidecar_format`, `scene_format` | `bad_request`, `out_of_range`, `payload_too_large`, `resource_limit` | yes |
| `GET /v1/tactical/window.png` | stable | PNG | — (content) | `bad_request`, `out_of_range`, `payload_too_large`, `resource_limit` | yes |
| `GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp` | stable | 512 px WebP | — (content) | `bad_request`, `out_of_range`, `not_found`, `payload_too_large` | no |
| `GET /v1/tactical/library` | stable | `TacticalLibraryDto` | `tactical_format` | — | no |
| `POST /v1/tactical/prefetch` | stable | `PrefetchAccepted` (body `PrefetchRequest`) | `tactical_format` | `bad_request`, `out_of_range`, `unsupported_media_type` | yes |
| `POST /v1/tactical/bake` | stable | `BakeAccepted` (body `BakeRequest`) | `bake_format`, `layout_schema` | `bad_request`, `out_of_range`, `payload_too_large`, `unsupported_media_type`, `invalid_layout`, `not_implemented`, `busy` | no (planned) |
| `GET /v1/tactical/bake/{id}` | stable | `BakeStatus` | `bake_format` | `not_found` | no (planned) |
| `GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp` | stable | 512 px WebP | — (content) | `bad_request`, `not_found` | no (planned) |
| `DELETE /v1/tactical/bake/{id}` | stable | `204`, no body | `bake_format` | `not_found` | no (planned) |
| `GET /v1/tactical/dungeon` | stable | `DungeonDto` | `tactical_format`, `layout_schema`, `rules_sidecar_format` | `bad_request`, `out_of_range` | yes |
| `GET /v1/tactical/dungeon.png` | stable | PNG | — (content) | `bad_request`, `out_of_range`, `payload_too_large`, `resource_limit` | yes |
| `GET /v1/tactical/dungeon/scene` | stable | `SceneDto` | `scene_format` | `bad_request`, `out_of_range` | yes |

Unstable and debug surface inside stable routes:

- `?demo_overlays=0|1` and `?demo_at=GX,GY` on the tactical cell, window and prefetch routes compose the synthetic Thornby samples instead of the world's society layers. **debug**: the options and the `meta.demo_at` key may go in any release.
- `501 not_implemented` with `NotYetError` (`planned_source`) comes only from the `PendingBlocks` block source, a test seam the default server never installs. **debug**. The other `501` (`POST /v1/tactical/bake` with `--bake-workers 0`) is stable and has no `planned_source`.
- `meta` on tactical bodies is provenance: free `string → string` pairs. Its keys are **unstable**.
- Image bytes (PNG, WebP) and the generated content of any body (which squares are forest, who lives where) are not shapes: see [Output stability](#output-stability-across-releases).

`GET /v1/schema` lists each schema's `family` and `stability` (`stable`, `unstable` or `debug`); `NpcDemo`, `TacticalLayouts` and `NotYetError` are `debug`.

## Version families

`GET /v1/world` carries `versions`, the current version of every family, so a client checks once at start; `bindings/ts/arda/index.ts` exports each as a constant (`CONTRACT_VERSION`, `TACTICAL_FORMAT`, …). Body fields named in the table carry the same number in the body itself.

| `versions` key | In bodies as | Current | Covers |
|---|---|---|---|
| `contract_version` | `contract_version` | 3 | `CellSample`, `PointSample`, `AreaCells`, `AreaRivers`, `AreaLakes`, `Health`, `WorldInfo` |
| `tactical_format` | `tactical_format` | 1 | `TacticalBlockDto` and `TacticalScene` envelopes; `DungeonDto`, `TacticalLibraryDto`, `PrefetchRequest`/`PrefetchAccepted`, `TacticalLayouts` |
| `layout_schema` | `layout_schema` | 1 | `TacticalLayoutDto` (block `layout`, bake and render bodies, `DungeonDto.layout`) |
| `scene_format` | `scene.format_version` | 1 | `SceneDto` (`arda-scene` `SCENE_FORMAT_VERSION`) |
| `rules_sidecar_format` | `rules.format_version` | 2 | `RulesSidecarDto` (`arda-scene` `SIDECAR_FORMAT_VERSION`) |
| `bake_format` | `format_version` | 1 | `BakeRequest`, `BakeAccepted`, `BakeStatus` |
| `npc_format` | — | 1 | `Npc`, `Sheet`, `Population`, `PopulationRequest`, `GenerateNpcRequest`, `NpcDemo` |
| `npc_page_format` | `format_version` | 1 | `NpcPage` |
| `settlement_format` | `format_version` (list) | 1 | `SettlementList`, `SettlementDetail`, `Settlement`, `SettlementSociety`, `TownPlan`, `SettlementNpcs` |
| `realm_format` | `format_version` (list) | 1 | `RealmList`, `RealmDetail`, `Realm`, `RealmSociety`, `Relation`, `Event` |
| `gazetteer_format` | `format_version` | 1 | `Gazetteer`, `GazetteerFeature`, `GazetteerPoint` |
| `schema_index_format` | `format_version` | 1 | `SchemaIndex` |
| `columnar_layout_version` | header word at offset 8 | 2 | the `?format=bin` `ARDACOLS` area layout |
| `library_catalogue_format` | `catalog.json` `format_version` | 1 | the tactical library this server loads (not a response) |
| `sheet_mapping_format` | mapping file `version` | 1 | the `--sheet-mapping` file this server reads (not a response) |
| (`api_version`) | `api_version` | `v1` | the route family, `ApiError`, `NotYetError` |

`format_version` therefore has three meanings; it is not renamed, because renaming is breaking:

- On `WorldInfo` it is the **world's on-disk format major** (`world.json`, 4 today), not a body version. See [Worlds](#worlds-on-disk-format_version-and-recipe_version).
- On `SettlementList`, `RealmList`, `NpcPage`, `Gazetteer`, `SchemaIndex` and the bake bodies it is that **body's** version (the family in the table).
- On `SceneDto` and `RulesSidecarDto` it is the **`arda-scene`** scene or sidecar format, and on the library catalogue the **catalogue file** format.

Two build versions sit beside the families:

- `WorldInfo.arda_version` is the generator that **wrote the world** (the manifest). A world generated by 0.8.1 and served by 1.0.0 reports `0.8.1`.
- `WorldInfo.server_version` and `Health.server_version` are the **`arda-server` build serving it**. Tactical maps, scenes, NPCs, dungeons and images are generated on request by this build, so they follow `server_version`, not `arda_version`.

A client that wants identical output pins **both**: `(seed, arda_version, server_version)`, plus the library's `library_version` for tactical images. Same triple, same bytes.

## HTTP rules

- **What bumps what.** Removing or renaming a field, changing a field's type, unit or meaning, making an optional field required, or adding a value to a **closed** enum bumps the family that covers the body. Adding an optional field, a route, a schema or a value to an **open** enum bumps nothing.
- **Unknown fields.** Clients ignore fields they do not know. Arda refuses unknown fields in request bodies it parses strictly (`deny_unknown_fields`: render, bake, prefetch, generate), so a client must not send fields a route does not document.
- **Unknown versions.** A client refuses a body whose family version it does not know, and reports the world as unavailable rather than guessing.
- **Enums.** Every enum is closed unless the table below lists it. A closed enum gains a value only with a family bump. An open enum may gain values in a minor release; a client maps a value it does not know to the listed fallback (`-`: skip the entry or show it untyped). Rules-bearing enums (movement, cover levels, obscurement, walls, terrain kind, cover, road, token kind, bake and prefetch states) are closed, because a client computes rules from them.

| Open enum | Fallback |
|---|---|
| `AssetClassDto` | `prop` |
| `Biome` | `temperate` |
| `BlockerKindDto` | `prop` |
| `BuildingFunction` | `house` |
| `DistrictKind` | `residential` |
| `EntityRef` | `-` |
| `EventKind` | `-` |
| `FocalKind` | `crossroads` |
| `Function` | `-` |
| `GoodRole` | `-` |
| `JobCategory` | `labour` |
| `LandUseDto` | `none` |
| `Preset` | `heartland` |
| `RelationKind` | `friend` |
| `RoomPurposeDto` | `hall` |
| `SettlementFunction` | `-` |
| `Trend` | `stable` |

Free strings that are vocabularies but not schema enums (job keys, gazetteer `kind`, ground keys, asset ids, tags, `meta` keys) may gain values in any release; clients treat an unknown one as opaque text.

- **Error codes.** `{"error": {"code", "status", "message"}}`. The set of codes is closed for 1.x: `bad_request`, `out_of_range`, `not_found`, `no_block`, `payload_too_large`, `unsupported_media_type`, `invalid_layout`, `unsatisfiable`, `resource_limit`, `sheet_mapping`, `not_implemented`, `busy`, `internal`. A route may start returning an existing code in a minor release; a new code needs a family bump of the route's family. `message` is never stable.
- **Breaking changes and `/v2`.** A change that would break a stable route's shape and that a family bump cannot express (a route's path, method or parameters, the error envelope, the identity or coordinate conventions) ships as a new route under `/v2`. `/v1` stays served beside it **until both sides (Arda and DiceMaster) agree to drop it**, and at least for one minor release after `/v2` ships. The changelog announces the deprecation; a deprecated route answers as before.
- **Bumping a family** in 1.x is itself a breaking change for clients that pinned the old number. It ships only in a major release, unless it covers an unstable or debug body, or both sides agree.

## Output stability across releases

Shapes are covered above. Content (which bytes a stable route returns for a given world) follows the release type:

| Release | Shapes | Generated content for an existing world and library |
|---|---|---|
| patch `x.y.Z` | unchanged | unchanged: every route returns the same bytes for the same world, library and request |
| minor `x.Y.0` | additive only | may change: maps, scenes, people, images. Every such change is flagged in `CHANGELOG.md` with the routes it affects |
| major `X.0.0` | may break (with a family bump or `/v2`) | may change |

A world written by an older generator stays servable as long as its format major is supported (next section); the content of its stored layers never changes, but on-demand bodies come from the serving build.

## Worlds: on-disk `format_version` and `recipe_version`

- `world.json` `format_version` is the on-disk format major (`arda_core::FORMAT_VERSION`, 4). A server loads only its own major: an older or newer world is refused at startup with "regenerate the world" (`crates/arda-core/src/formats/manifest.rs`). A major bump happens only in a major release.
- `fine_terrain.recipe_version` is the fine-terrain recipe (7 today). A server serves every recipe up to its own; a feature that needs a newer recipe than the world has is unavailable on that world (relief tiles need recipe 5 or later and answer `404 not_found`). A world with a recipe newer than the server knows is served with the newest behaviour the server has; generate and serve with the same release to avoid it.
- `society/` is written by `arda settle` and `arda society build`; rebuild it with the serving release after an upgrade. A society raster whose size is not the world's cell grid refuses startup; a world without `society/` serves the world routes, and the society routes answer `404 not_found`.

## Library: catalogue `format_version` and `library_version`

- `catalog.json` `format_version` (`arda_tactical::catalog::FORMAT_VERSION`, 1) must equal the server's; any other value refuses startup. It bumps only in a major release.
- `library_version` (a free string, such as `0.1.0+seed58`) is the content version of the art. It is part of every tactical render's cache key and bake job id. Changing it changes images, never shapes; a client pinning images records it with the world triple.

## Binaries and image

- The workspace version (`Cargo.toml` `[workspace.package]`) is the version of `arda`, `arda-server` and the `ghcr.io/dicemasterio/arda` image, and is `server_version` on the wire. They follow semantic versioning with the meaning above: patch for fixes that keep shapes and bytes, minor for additions and content changes, major for breaking changes.
- Image tags: `vX.Y.Z` publishes `X.Y.Z`, `X.Y` and `latest`; pre-releases publish only their exact tag. Release mechanics are in [07-operations](07-operations.md) "Release process".
- CLI flags and exit codes of the binaries follow the same rule as routes: removing or re-meaning a flag is a major change.

## Rust crates

The crates' Rust APIs are **not** covered by this policy. They are internal to the workspace until the crates are published on crates.io (1.0 task 02 decides); if they are, a separate crate policy will apply per crate.

## Freeze guard

`frozen_schemas_keep_their_shape_unless_their_family_is_bumped` (`crates/arda-server/src/schema/tests.rs`, logic in `src/schema/compat.rs`) compares every committed schema with its copy in `bindings/schema-baseline/v1/` (`versions.json` there records each schema's family and every family's version at baseline time). It fails on:

- a removed property, or a property newly in `required`;
- a changed `type`, `format`, `const` or `$ref` (becoming nullable is a type change);
- a removed enum value or union variant, and a value or variant added to a closed enum;
- a removed schema;

unless the schema's family, or the family owning the changed `$defs` entry (the smallest schema that defines it: `MovementDto` belongs to `SceneDto`), has a different version than at baseline. Added optional properties, schemas and `$defs` pass.

After an intended change, re-baseline with `ARDA_BLESS_SCHEMA_BASELINE=1 cargo test -p arda-server --lib frozen_schemas` and commit the baseline in its own commit ([06-testing](06-testing.md)). Re-baselining after an additive change is optional; after a breaking one it must come with the family bump.
