# Arda — reference index

Deterministic offline tabletop world generator in a Rust 2021 Cargo workspace (23 crates): continent → 100 m world cells → settlements, roads, realms, society and NPCs → on-demand 64 × 64 five-foot tactical maps, dungeons and mid-zoom relief, served read-only over HTTP by `arda-server` (axum) and browsed with a React/Vite/Leaflet viewer. Tactical art comes from an AI art pipeline (`tools/art-gen`) imported into a validated asset library. Apache-2.0; first consumer is the DiceMaster VTT.

## Module map

| Module | Purpose | Entry points |
|---|---|---|
| arda-core | Shared types, keyed PRNG, the only world byte codec | `crates/arda-core/src/lib.rs:1` |
| arda-ids | Shared ids, canonical enums, stable hash for product layers | `crates/arda-ids/src/lib.rs:1` |
| arda-names | Deterministic phonotactic naming languages | `crates/arda-names/src/lib.rs:47` |
| arda-society | Realm politics, economy, factions, history, campaign hooks | `crates/arda-society/src/lib.rs:134` |
| arda-tactical | Asset catalogue, validator, placeholders, `TacticalLayout`, compositor | `crates/arda-tactical/src/lib.rs:26` |
| arda-wfc | Generic deterministic wave-function collapse | `crates/arda-wfc/src/lib.rs:22` |
| arda-gen | Continent, area, formation and hydrology generation | `crates/arda-gen/src/lib.rs:21` |
| arda-render | PNG/JSON export of stored worlds | `crates/arda-render/src/lib.rs:1` |
| arda-npc | NPC populations with jobs, personalities, ranks, SRD 5.1 sheets | `crates/arda-npc/src/lib.rs:215` |
| arda-scene | SRD movement/sight/cover/light scene from a `TacticalLayout` | `crates/arda-scene/src/lib.rs:51` |
| arda | Public world facade: generate, `World` load/query, exports | `crates/arda/src/lib.rs:44`, `crates/arda/src/world.rs:34` |
| arda-art-import | Raw AI images to a valid tactical asset library | `crates/arda-art-import/src/lib.rs:97` |
| arda-dungeon | Dungeon and cave battle maps | `crates/arda-dungeon/src/lib.rs:227` |
| arda-fields | 5-ft land use around settlements | `crates/arda-fields/src/lib.rs:43` |
| arda-ways | 5-ft roads, verges, bridges, fords, ferries, toll houses | `crates/arda-ways/src/lib.rs:101` |
| arda-town | Town street/plot plans and 5-ft building blocks; `arda-town` binary | `crates/arda-town/src/plan/mod.rs:60`, `crates/arda-town/src/main.rs:260` |
| arda-refine | 100 m world cell → 64 × 64 five-foot squares; `arda-refine` binary | `crates/arda-refine/src/block.rs:167`, `crates/arda-refine/src/bin/arda-refine.rs:75` |
| arda-settle | Settlements, roads, realms, names into `<world>/society/`; `arda-settle` binary | `crates/arda-settle/src/lib.rs:160`, `crates/arda-settle/src/main.rs:42` |
| arda-people | Town plans on world terrain, `society.json`, `notables.json` | `crates/arda-people/src/build.rs:45` |
| arda-blocks | World-derived tactical blocks: refine plus ways/fields/town overlays | `crates/arda-blocks/src/pipeline.rs:165` |
| arda-midzoom | On-demand ~10 m relief refinement and tiles; `arda-midzoom` binary | `crates/arda-midzoom/src/lib.rs:58`, `crates/arda-midzoom/src/bin/arda-midzoom.rs:41` |
| arda-cli | The `arda` binary: generate, preview, export, settle, society, tactical, dungeon | `crates/arda-cli/src/main.rs:360` |
| arda-server | Versioned world-query contract and HTTP service under `/v1` | `crates/arda-server/src/main.rs:94`, `crates/arda-server/src/routes.rs:28` |
| apps/viewer | React/Vite/Leaflet developer viewer for the arda-server API | `apps/viewer/src/main.tsx:9`, `apps/viewer/src/App.tsx:22` |
| tools/art-gen | Node driver generating tactical art through Slopify, writing `import.toml` | `tools/art-gen/generate.mjs:607`, `tools/art-gen/build-prompts.mjs:1` |

## Topics

| Topic | File |
|---|---|
| architecture | [01-architecture.md](01-architecture.md) |
| models | [02-models.md](02-models.md) |
| conventions | [03-conventions.md](03-conventions.md) |
| data-flow | [04-data-flow.md](04-data-flow.md) |
| dependencies | [05-dependencies.md](05-dependencies.md) |
| testing | [06-testing.md](06-testing.md) |
| operations | [07-operations.md](07-operations.md) |
| glossary | [08-glossary.md](08-glossary.md) |
| interfaces | [09-interfaces.md](09-interfaces.md) |
| logic | [Scenario index](logic/README.md) |
| logic | [01-continent-generation](logic/01-continent-generation.md) |
| logic | [02-area-generation](logic/02-area-generation.md) |
| logic | [03-block-generation](logic/03-block-generation.md) |
| logic | [04-export](logic/04-export.md) |
| logic | [05-load-query](logic/05-load-query.md) |
| logic | [06-society-generation](logic/06-society-generation.md) |
| logic | [07-preview](logic/07-preview.md) |
| logic | [08-settlements-roads-realms](logic/08-settlements-roads-realms.md) |
| logic | [09-tactical-refinement](logic/09-tactical-refinement.md) |
| logic | [10-town-layout](logic/10-town-layout.md) |
| logic | [11-tactical-art-compositor](logic/11-tactical-art-compositor.md) |
| logic | [12-scene-data](logic/12-scene-data.md) |
| logic | [13-npc-population](logic/13-npc-population.md) |
| logic | [14-society-history](logic/14-society-history.md) |
| logic | [15-naming](logic/15-naming.md) |
| logic | [16-service-api](logic/16-service-api.md) |
| logic | [17-midzoom-relief](logic/17-midzoom-relief.md) |
| logic | [18-dungeons-and-caves](logic/18-dungeons-and-caves.md) |
| logic | [19-art-import](logic/19-art-import.md) |
| logic | [20-art-generation](logic/20-art-generation.md) |
| logic | [21-society-build](logic/21-society-build.md) |
| logic | [22-developer-inspection](logic/22-developer-inspection.md) |

## Companion docs

| File | What it is |
|---|---|
| `contract-dicemaster.md` (local only) | Arda ↔ DiceMaster contract; ignored by Git. |
| [open-items.md](open-items.md) | Open work only: current state, remaining scope, work queue and items OI-32 … OI-53. |
| [versioning.md](versioning.md) | `/v1` versioning and compatibility policy: route stability review, version families, enums, deprecation, output stability, freeze guard. |
| [standards.md](standards.md) | Normative coding rules. |
| [integration-plan.md](integration-plan.md) | Merge order, adapters and end-to-end acceptance test for the product branches. |
| [integration-status.md](integration-status.md) | Integration record for the product branches. |
| [implementation.md](implementation.md) | Retained build plan with completion states. |
| [mockup-artifact.md](mockup-artifact.md) | User-provided generator logic artifact. |
| [mockup/README.md](mockup/README.md) | Retained product brief, surface inventory and assumptions. |
| [mockup/01-generate.md](mockup/01-generate.md) … [06-serve.md](mockup/06-serve.md) | Retained surface designs (generate, world layout, export, crate API, docker, serve). |
| [uiux/01-direction.md](uiux/01-direction.md) | The viewer's observed design direction. |
| [uiux/02-system.md](uiux/02-system.md) | The viewer's visual system as implemented. |
| [uiux/screens/](uiux/screens/01-world.md) | Viewer surfaces: [world](uiux/screens/01-world.md), [area](uiux/screens/02-area.md), [tactical](uiux/screens/03-tactical.md), [cell](uiux/screens/04-cell.md), [editor](uiux/screens/05-editor.md). |
| [changelog.md](changelog.md) | Append-only run ledger. |
| `features/` (local only) | Specifications, plans and retained evidence; ignored by Git. |
