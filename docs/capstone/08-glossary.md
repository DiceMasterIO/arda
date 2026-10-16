---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 290ecb60fc76
paths_covered:
  - ":(top)crates/*/src/**"
  - ":(top)tests/**"
---

# Glossary

## Concepts

Terms are grouped by layer. Each row gives the meaning in this system and its definition site. Hydrology terms are the subset still needed to read the stored-world types; the full water model is in `02-models.md` and `logic/01`–`02` (not repeated here).

### Scales and frames

| Term | Meaning in Arda | Definition site |
|---|---|---|
| Continent | The coarse 1 km world grid produced by plates, tectonics and upsampling before any area exists; one `Tier` of the core RNG key. | `crates/arda-gen/src/continent/mod.rs:119`, `crates/arda-core/src/rng.rs:19` |
| Area (area tile) | A 512 × 512-cell tile, 51.2 km on a side; the unit of storage and of the legacy generation stages. Directory name `<ax>_<ay>`. | `crates/arda-core/src/coords.rs:7`, `crates/arda-core/src/coords.rs:15` |
| Cell | One 100 m square of an area; the `Cell` record holds height, terrain, cover, climate, drainage, road, `built_by` and the rest of the stored per-cell facts. | `crates/arda-core/src/coords.rs:9`, `crates/arda-core/src/cell.rs:103` |
| `CellKey` | A global (whole-world) signed cell coordinate used by the tactical crates; neighbourhoods may step past the world edge and sources clamp. | `crates/arda-refine/src/source.rs:16` |
| Fine terrain / fine lattice | The canonical height field at 39.0625 m spacing, stored at `terrain/fine.bin`; every consumer samples it. Lattice node (0,0) sits at the centre of cell (0,0) (`FINE_FRAME_OFFSET_UM` = 50 m). | `crates/arda-gen/src/formation/mod.rs:73`, `crates/arda-core/src/formats/manifest.rs:43`, `crates/arda-core/src/terrain.rs:29` |
| Legacy vs fine terrain | CLI `--terrain` choice: `legacy` (default) runs the original continent/area stages; `fine` forms the fine lattice with a recipe. | `crates/arda-cli/src/main.rs:51`, `crates/arda-cli/src/main.rs:191` |
| Recipe | A versioned fine-formation rule set (`V5`–`V8`); the number is the manifest's `recipe_version`. New worlds use 7, the newest is 8. | `crates/arda-gen/src/formation/mod.rs:208`, `crates/arda-core/src/formats/manifest.rs:58`, `crates/arda-core/src/formats/manifest.rs:63` |
| MICRO | The small fixed generation config (102 × 204 km, latitude 35–55°) used by tests and examples; "MICRO 42" is that config with seed 42. | `crates/arda-core/src/config.rs:63` |
| Square (tactical) | One 5-ft square of the tactical layer: a 100 m cell is 64 × 64 squares, so a square is exactly 1.5625 m (a binary fraction, exact in floating point). | `crates/arda-refine/src/context.rs:36`, `crates/arda-blocks/src/pipeline.rs:15`, `crates/arda-ways/src/input.rs:10` |
| Square size constant | `SQUARE_SIZE_UM`, 1,562,500 µm: 64 squares (`BLOCK_SQUARES`) make exactly one 100 m cell. Play calls a square five feet (1.524 m), a deliberate 2.5 % horizontal compression (logic/09 §square-frame); the five-foot `SQUARE_SIZE_MM` was deleted (review 3.4-11). | `crates/arda-core/src/coords.rs` |
| Block (refined) | One cell refined to 64 × 64 squares by `arda-refine`; the unit `arda-blocks` caches. | `crates/arda-refine/src/block.rs:35` |
| Window | A rectangle of global squares that a tactical crate renders at once; `arda-blocks` caps it at 256 squares a side. | `crates/arda-blocks/src/pipeline.rs:17`, `crates/arda-town/src/block/mod.rs:43`, `crates/arda-ways/src/plan/mod.rs:22` |
| Tactical | The 5-ft battle-map layer: `TacticalLayout` plus its art library and rules data. "Tactical zoom" is the client view of these maps. | `crates/arda-tactical/src/lib.rs:1` |
| Refine | Turning one world cell into a natural tactical block: contoured elevation, rivers and shores, border corners, WFC ground, Poisson scatter and a rules sidecar, from the seed and global coordinates only. | `crates/arda-refine/src/lib.rs:1`, `crates/arda-refine/src/lib.rs:68` |
| Midzoom | On-demand ~10 m relief between the stored 39 m field and the tactical layer, grown only for tiles on screen; keeps every 39 m cell mean and every saved river and lake. | `crates/arda-midzoom/src/lib.rs:1`, `crates/arda-midzoom/src/lib.rs:60` |
| Origin | A layout's world position of square (0,0), in world squares; the compositor hashes and samples noise at world coordinates so adjacent blocks join. | `crates/arda-tactical/src/layout.rs:37` |

### World generation and storage

| Term | Meaning in Arda | Definition site |
|---|---|---|
| World | A saved directory: manifest, continent layers, shared hydrology, area layers and terrain layers. A pre-1.0 `blocks/` folder is ignored. The facade's `World` loads the manifest first and fills per-area caches lazily. | `crates/arda/src/world.rs:19`, `crates/arda-core/src/formats/manifest.rs:67` |
| `format_version` | The stored-world major, currently 4; loaders refuse any other major. Separate versions exist per document (water layer, render JSON schema, sidecars, scene, server contract). | `crates/arda-core/src/formats/mod.rs:19`, `crates/arda-core/src/formats/water.rs:28`, `crates/arda-render/src/json.rs:12` |
| `SeedKey` / subseed | The core stream key `(tier, stage, x, y, attempt)`; BLAKE3 over seed and key gives a ChaCha8 key. `arda_ids::subseed` is the product layers' named sub-generator seed. | `crates/arda-core/src/rng.rs:50`, `crates/arda-core/src/rng.rs:84`, `crates/arda-ids/src/hash.rs:98` |
| Stable hash | `arda_ids::hash(seed, tag, args)`: the SplitMix64 chain every tactical rule uses for picks that two crates or two blocks must agree on; `tag` is FNV-1a of the rule name. | `crates/arda-ids/src/hash.rs:56`, `crates/arda-ids/src/hash.rs:68` |
| `TileId` | A u16 index into a tile vocabulary, not a coordinate. `arda-refine` has its own corner-tile vocabulary (the 24-tile block vocabulary was retired in 1.0). | `crates/arda-refine/src/tiles.rs:20` |
| `TileBundle` / `EnteringRiver` | Coarse-derived per-area inputs (pinned edge heights, entering rivers, climate patches) for the legacy area path; not the authority for published fine water. | `crates/arda-gen/src/continent/bundles.rs:142`, `crates/arda-gen/src/continent/bundles.rs:116` |
| `PreparedTerrain` | Canonical heights, rainfall and temperature arrays that shared water consumes before final area composition. | `crates/arda-gen/src/hydrology/types.rs:31` |
| Shared evolution | The whole-domain fine erosion that runs 160 iterations before areas are composed. | `crates/arda-gen/src/area/evolution.rs:20` |
| Resource admission | RAM, scratch, work and I/O capacity checks made before generation writes; exceeding a limit is a typed error, never a different result. | `crates/arda-gen/src/hydrology/types.rs:84` |
| Wetness | Saved 0–255 standing-water tendency per cell; distinct from soil `moisture` and from `forest_density` (canopy closure). | `crates/arda-core/src/cell.rs:133`, `crates/arda-core/src/cell.rs:119`, `crates/arda-core/src/cell.rs:121` |
| HAND | Height above the nearest downstream channel, stored per cell in decimetres. | `crates/arda-core/src/cell.rs:131` |
| `built_by` | The settlement that built on a cell; 0 when none. Contract 3 serves it as a settlement id string. | `crates/arda-core/src/cell.rs:137` |
| Golden world | The MICRO fixture whose per-file fingerprints are compared by `tests/golden_world.rs`; `ARDA_BLESS=1` rewrites them. | `tests/golden_world.rs:69`, `tests/golden_world.rs:82` |

### Water

| Term | Meaning in Arda | Definition site |
|---|---|---|
| `Litres` / `DischargeMilli` | Exact u128 annual volume, and u64 whole litres per second (thousandths of m³/s). | `crates/arda-core/src/hydrology.rs:109`, `crates/arda-core/src/fixed.rs:78` |
| P / A / E / R / D cell budget | Per 100 m cell: precipitation, actual land loss, open-water evaporation, runoff and added wet-support cost, computed from rainfall and monthly evaporation. | `crates/arda-gen/src/hydrology/annual_aggregation.rs:40`, `crates/arda-gen/src/hydrology/annual_aggregation.rs:53` |
| `GlobalLake` | One connected representative lake with whole-mm surface; local fragments share its identity and level. | `crates/arda-core/src/hydrology.rs:221` |
| `GlobalReach` / `RiverSegment` | A reach owns annual flow and downstream identity; a segment is its tile-local fragment carrying `global_reach_id`. | `crates/arda-core/src/hydrology.rs:259`, `crates/arda-core/src/objects.rs:49` |
| `ReachId` / point reach | Directed reach identity; bit 63 tags a point reach for a terminal with no directed step. | `crates/arda-core/src/hydrology.rs:28`, `crates/arda-core/src/hydrology.rs:82` |
| `JunctionId` | A dry junction named by its packed global (y,x); no separate junction table. | `crates/arda-core/src/hydrology.rs:32` |
| `BasinId` | Stable identity of a coalesced physical basin, independent of whether its lake is wet, dry or spilling. | `crates/arda-core/src/hydrology.rs:24` |
| Spill connection | The actual adjacent source/target cells and sill of a basin's exit. | `crates/arda-core/src/hydrology.rs:168` |
| `AnnualCatchment` | Terminal ownership and contributing cells of a catchment. | `crates/arda-core/src/hydrology.rs:240` |
| `SharedCrossing` | A directed water transfer across an area boundary, copied into both areas. Unrelated to road crossings. | `crates/arda-core/src/hydrology.rs:281` |
| Area hydrology context | Bounded copies of global water records an area needs to interpret and rasterize itself, including neighbours touching it. | `crates/arda-core/src/hydrology.rs:333` |
| Channel width | One integer helper maps discharge to physical width in decimetres. | `crates/arda-core/src/hydrology.rs:117` |

### Tactical maps and art

| Term | Meaning in Arda | Definition site |
|---|---|---|
| `TacticalLayout` | A W × H battle map: row-major squares (ground key, elevation in feet), edge walls, placements, lights and an optional origin. The shared output of refine, fields, ways, town and dungeon. | `crates/arda-tactical/src/layout.rs:15`, `crates/arda-tactical/src/layout.rs:43` |
| Catalogue / asset / library | The art manifest (`Catalog`) lists assets by class and layer; a `Library` is a catalogue plus its images. | `crates/arda-tactical/src/catalog.rs:15`, `crates/arda-tactical/src/catalog.rs:52`, `crates/arda-tactical/src/library.rs:18` |
| Placeholder library | The code-generated library at 128 px per square that every imported library falls back to. | `crates/arda-tactical/src/placeholders/mod.rs:29`, `crates/arda-tactical/src/placeholders/mod.rs:238` |
| Library stack | `top:…:bottom` list of libraries, leftmost wins; fallback is resolved per thing the compositor picks among, so styles never mix inside one pick. | `crates/arda-tactical/src/library_stack.rs:5` |
| Take (`.altN`) | A variant of one asset, id suffixed `.alt<N>`; a placement draws one take hashed from seed and world position, and an explicit take id pins it. | `crates/arda-tactical/src/library.rs:267`, `crates/arda-tactical/src/layout.rs:141` |
| ppsq | Pixels per square. Clients may request 64, 96 or 128 (default 128); the importer and placeholders use 128. | `crates/arda-server/src/tactical/mod.rs:53`, `crates/arda-art-import/src/lib.rs:60` |
| Wall kit | A named family of wall pieces (`stone`, `timber`, `wattle`, …) a `WallSegment` draws with; run, door, window or gate by `WallRole`. | `crates/arda-tactical/src/layout.rs:80`, `crates/arda-tactical/src/catalog.rs:288` |
| Partition | Edge tag for an interior wall inside one building; drawn with `<kit>_partition` art or the kit squeezed to 50 % thickness. Blocks like any wall. | `crates/arda-tactical/src/layout.rs:109`, `crates/arda-tactical/src/compose/mod.rs:106` |
| Party wall | Edge tag for an outer wall two buildings share; drawn as an exterior wall. `arda-town` classifies every building edge as `Shell`, `Partition` or `Party`. | `crates/arda-tactical/src/layout.rs:112`, `crates/arda-town/src/block/mod.rs:151` |
| World tint | Optional grade that pulls ground and water layers toward the world map's colour at their location before sprites are drawn, from a world-anchored lattice so shared pixels tint identically. | `crates/arda-tactical/src/compose/world_tint.rs:45` |
| WFC | Generic wave-function collapse over a grid of `TileSet` bitsets with adjacency rules, up to 8 attempts; a dependency of `arda-refine` and `arda-town` (`arda-refine` also keeps its own corner-tile `Problem` in `src/wfc.rs`). | `crates/arda-wfc/src/solve.rs:51`, `crates/arda-wfc/src/set.rs:10`, `crates/arda-wfc/src/solve.rs:23` |
| Overlays | The seam by which ways, fields and town layers are composed onto a refined window, in that order. `SocietyOverlays` reads world society data. | `crates/arda-blocks/src/overlays.rs:117`, `crates/arda-blocks/src/society/mod.rs:91` |
| Reservation / `Owner` | Who claims a square, in rising precedence natural < verge < fields < croft < water < ways < town; verge and croft are soft claims of ways and town. | `crates/arda-blocks/src/overlays.rs:28` |
| Made ground | Town footprint over at most 1 ft of standing water (quays, filled shore or marsh fringe); the town outranks water there. | `crates/arda-blocks/src/overlays.rs:21` |
| Composed block | One `TacticalLayout` with world origin plus one rules sidecar, built from the same layout so image and rules agree. | `crates/arda-blocks/src/pipeline.rs:23` |
| Rules sidecar | Per-square SRD rules beside a layout (format 2) plus edge rules for parapets, retaining walls, hedges and field walls; merged into the scene. | `crates/arda-scene/src/sidecar.rs:32`, `crates/arda-scene/src/sidecar.rs:37` |
| Scene | Game-facing data for a layout: per-square movement, climb mask, cover, obscurement, elevation, walls with door state, lights, regions and spawn hints. Identity is seed plus library version. | `crates/arda-scene/src/types.rs:274`, `crates/arda-scene/src/types.rs:35`, `crates/arda-scene/src/types.rs:62` |
| Token | An NPC placed on a tactical block: npc id, square, building, settlement and kind `worker` or `resident`. | `crates/arda-server/src/tactical/tokens.rs:26` |
| Dungeon | An underground battle map: `Kind::Dungeon` (BSP rooms, MST corridors with loops, locked and secret doors) or `Kind::Cave` (cellular-automaton caverns); an ordinary layout plus sidecar. | `crates/arda-dungeon/src/lib.rs:199`, `crates/arda-dungeon/src/lib.rs:54`, `crates/arda-dungeon/src/lib.rs:229` |
| Ways | Roads and crossings drawn at 5-ft scale onto a window: surfaces, verges, ditches, benches, switchbacks, bridges, fords, ferries, toll houses. | `crates/arda-ways/src/lib.rs:1`, `crates/arda-ways/src/lib.rs:101` |
| Fields | Countryside land use at 5-ft scale: strips, enclosed fields, pasture, meadows, orchards, woodland, farmsteads, mills, mines. | `crates/arda-fields/src/lib.rs:1`, `crates/arda-fields/src/window.rs:59` |
| Export overlay (`BandOverlay`) | The hook that paints on finished RGB row groups just before the overview or area exporter encodes them; `None` is exactly the no-overlay path. The society layer supplies it from above the renderer. | `crates/arda-render/src/overlay.rs:18` |
| Works | The worked land of a pixel window apart from relief (parcels, coarse land-use tone, roads, settlement plans), shared by relief tiles (`LandWindow`) and area exports (`AreaLandOverlay`). | `crates/arda-midzoom/src/land/mod.rs:153`, `crates/arda-midzoom/src/land/export.rs:33` |
| Bake | `POST /v1/tactical/bake`: a background, uncached, content-addressed job that renders a client-edited cell into the WebP tile set the client stores; finished sets wait in a byte budget until deleted. | `crates/arda-server/src/tactical/bake.rs:1`, `crates/arda-server/src/tactical/bake_dto.rs:126` |
| Splice (apron) | Pasting the posted cell layout over the centre of the cell's apron block so its edges join unedited neighbours; items go by anchor and appear once. | `crates/arda-server/src/tactical/splice.rs:76` |

### Settlements and towns

| Term | Meaning in Arda | Definition site |
|---|---|---|
| Settlement | One placed settlement in `society/settlements.json`: id, name, tier, population, functions, wealth, culture and more. | `crates/arda-settle/src/model.rs:87` |
| Settlement tier | `Hamlet` (12–80 people), `Village` (a few hundred), `Town` (1,000–8,000), `City` (8,000+). Each consuming crate keeps a mirror enum. | `crates/arda-settle/src/model.rs:12`, `crates/arda-settle/src/model.rs:42` |
| Realm | A polity of settlements in `society/realms.json`, with a generated name and English gloss. | `crates/arda-settle/src/realms.rs:39` |
| Political map | The default look of `arda-settle render` and the `political` overlay of overviews and overview tiles: each realm's own colour (one per realm, never shared) as an inner edge band at its borders, dark dash-dot borders over a pale halo, ivory cased roads, no area fill (logic/16 §political-map). | `crates/arda-settle/src/render/political.rs`, `crates/arda-settle/src/render/palette.rs` |
| World scale | An image coarser than 30 m per pixel (`render::scale::HAMLET_MAX_M`): the offline page, overview exports and overview tiles. It shows cities, towns and villages; hamlets and the tracks built to them are area-scale only (logic/04 §overlay). | `crates/arda-settle/src/render/scale.rs` |
| Crossing / pass | A road crossing water (bridge, ford or ferry) or a mountain pass, as world objects in metres. | `crates/arda-settle/src/crossings.rs:32`, `crates/arda-settle/src/crossings.rs:56` |
| `RoadClass` | Canonical road codes `none`, `track`, `road`, `highway`, `footpath`. | `crates/arda-ids/src/road.rs:13` |
| `LandUse` | Canonical 100 m land-use classes (`built`, `field`, `pasture`, `orchard`, `woodland`, `mill`, `mine`, `meadow`, `fallow`, `farmstead`, …). | `crates/arda-ids/src/land_use.rs:15` |
| `Biome` / culture | Closed settlement biome list shared through `arda-ids`; culture keys come from `arda-npc` content data and `arda-settle`'s `Culture`. | `crates/arda-ids/src/biome.rs:14`, `crates/arda-settle/src/culture.rs:18` |
| `BuildingFunction` / craft | Snake-case building function keys (32); a workshop's craft travels beside it as `craft:<key>`. | `crates/arda-ids/src/function.rs:15`, `crates/arda-ids/src/function.rs:182` |
| `TownSite` | The settlement record a town plan is drawn from. | `crates/arda-town/src/site.rs:79` |
| `TownPlan` | A complete town: streets, bridges, square, plots, districts, wall, castle, buildings, `fabric`, plus non-serialized grid and per-building looks. | `crates/arda-town/src/plan/types.rs:293` |
| Plot | A burgage or toft along a street frontage, with frontage, depth, district and inside-wall flag. | `crates/arda-town/src/plan/types.rs:138` |
| District | Functional quarter of a town: market, residential, craft, waterfront, religious, castle, suburb, farmstead. | `crates/arda-town/src/plan/types.rs:116` |
| Croft | Open ground inside the built-up footprint (paddock, orchard, kitchen garden, small green, work yard). In block composition it is a soft town claim that yields to water and roads. | `crates/arda-town/src/plan/grid.rs:57`, `crates/arda-blocks/src/overlays.rs:36` |
| Ambitus | A one-square eaves-drip gap kept in the last column of some street-front row plots: 25 % in towns, 15 % in cities, none in villages, never on the market square or under a three-square frontage. | `crates/arda-town/src/plan/params.rs:29`, `crates/arda-town/src/plan/place.rs:110` |
| Fabric | The settlement facts behind building materials: wealth, merchant, biome, forest and stone at hand, great fires and golden age from history. Layout-neutral. | `crates/arda-town/src/plan/types.rs:356` |
| Look | One building's shell kit, partition kit and floor, drawn from the fabric table. | `crates/arda-town/src/block/kits.rs:292`, `crates/arda-town/src/block/kits.rs:337` |
| Kit chain | Fallback order from a kit a library lacks to one it has (e.g. `log` → timber, `adobe` → stone). | `crates/arda-town/src/block/fallback.rs:66` |
| Gazetteer feature | A named natural feature of the world map (`sea`, `bay`, `lake`, `island`, `cape`, `forest`, `marsh`, `mountain` range or `river`) with anchor, optional label path, extent, rank 1 to 5 and first zoom; stored in `society/gazetteer.json` and served with settlement points as `Gazetteer` (distinct from the text `Gazetteer` of the society crate). | `crates/arda-settle/src/gazetteer/mod.rs:136`, `crates/arda-settle/src/gazetteer/list.rs:88`, `crates/arda-server/src/schema/mod.rs:161` |
| Range (mountain range) | Summits within `link_m` of each other whose ridge never drops more than `SADDLE_MM` below the lower summit, at least `min_peaks` of them; one `mountain` gazetteer feature. | `crates/arda-settle/src/gazetteer/range.rs:1`, `crates/arda-settle/src/gazetteer/mod.rs:67` |
| Tile labels | Gazetteer feature names (spread along the feature, italic on water) and settlement names placed once per zoom level, greedy by rank, then clipped to the window so neighbouring tiles agree. | `crates/arda-settle/src/render/tilelabels.rs:1` |

### People and society

| Term | Meaning in Arda | Definition site |
|---|---|---|
| NPC / `NpcId` | An inhabitant with name, ancestry, age, home, job, personality, relationships and an SRD 5.1 sheet. Its id is `subseed(settlement, "npc-id", [building, index])`. | `crates/arda-npc/src/npc.rs:292`, `crates/arda-ids/src/ids.rs:116` |
| Household / population | Residents grouped by home; a settlement's population is regenerated on demand from the seed. | `crates/arda-npc/src/population.rs:21`, `crates/arda-npc/src/population.rs:51` |
| Sheet | An NPC's SRD 5.1 character sheet. | `crates/arda-npc/src/sheet/mod.rs:160` |
| Notable | An NPC bound to a society role slot, stored in `society/notables.json`; commoners are not stored. | `crates/arda-npc/src/input.rs:133`, `crates/arda-people/src/files.rs:14` |
| Rank | `SocialRank` from `Dependent` to `Noble`; set by job and tier and raised by any office held; bounds lifestyle. | `crates/arda-npc/src/npc.rs:124`, `crates/arda-npc/src/ranks.rs:17`, `crates/arda-npc/src/ranks.rs:28` |
| Office / role | A notable slot the society layer fixes (`ruler`, `lord`, `reeve`, `captain`, …) with title, building, faction and stat block. Civic offices count toward a per-tier civic cap. | `crates/arda-society/src/roles.rs:27`, `crates/arda-npc/src/ranks.rs:40`, `crates/arda-npc/src/ranks.rs:52` |
| Society | Output of `simulate_society`: realm politics, economy, factions, a simulated history whose outcome is the present, hooks and role slots. | `crates/arda-society/src/lib.rs:112`, `crates/arda-society/src/lib.rs:136` |
| Event | One timeline entry of simulated history (founding, realm formed, accession, war, peace, flood, plague, fire, famine, mine collapse, abandonment, …). | `crates/arda-society/src/history/mod.rs:58` |
| Hook | A short plot hook for the referee, per settlement or realm, at most 5. | `crates/arda-society/src/hooks/mod.rs:32` |
| Faction | A group within one settlement (court, guilds, clergy, watch, criminals, …) with goals, a leader slot, a seat building and pairwise stances. | `crates/arda-society/src/politics/factions.rs:19` |
| Gazetteer | Readable text rendering of a `Society`. | `crates/arda-society/src/gazetteer.rs:1` |
| Language / preset / tongue | A naming language built from a seed and a culture `Preset`; a `Tongue` holds everything needed to rebuild a place's local language. | `crates/arda-names/src/language.rs:58`, `crates/arda-names/src/preset/mod.rs:156`, `crates/arda-names/src/tongue.rs:18` |
| People world | `arda-people`'s view of a world's `society/` directory: plans, notables and populations by settlement. | `crates/arda-people/src/people.rs:43`, `crates/arda-people/src/people.rs:137` |
| Major event (named) | A history event with a proper `name`, a `backstory` and a `cause_kind` from the closed list `CAUSE_KINDS`; names are unique in the world. | `crates/arda-society/src/history/mod.rs:89`, `crates/arda-society/src/history/mod.rs:99` |
| Causal chain | A flood or war that brings a famine within three years, a famine or war camp a plague within six, then ruins; made by moving a planned event after its cause, never by adding events. | `crates/arda-society/src/history/chains.rs:1` |
| Template family | A key of the embedded history tables with the slots its call site fills; `lint` checks every table against the list. | `crates/arda-society/src/families.rs:11`, `crates/arda-society/src/families.rs:274` |
| Synthetic settlement | The reserved settlement id 0 that `POST /v1/npc/generate` lays out so a requested job exists; never a real settlement. | `crates/arda-npc/src/one/mod.rs:30` |

### Service

| Term | Meaning in Arda | Definition site |
|---|---|---|
| `contract_version` | Version of the per-cell query contract, currently 3 (adds `footpath`, string `built_by`, `land_use`, `realm_id`); clients reject unknown versions. | `crates/arda-server/src/contract/mod.rs:31` |
| `CellSample` | The versioned per-cell record every consumer reads. | `crates/arda-server/src/contract/mod.rs:179` |
| API version | Route prefix `v1`. | `crates/arda-server/src/dto.rs:12` |
| Error code | Stable machine code in every error body (`bad_request`, `out_of_range`, `not_found`, `no_block`, `payload_too_large`, `invalid_layout`, `unsatisfiable`, `resource_limit`, `not_implemented`, `busy`, `internal`, …). | `crates/arda-server/src/error.rs:100` |
| Prefetch | Background refinement of a cell's neighbour rings (radius ≤ 2) queued by `POST /v1/tactical/prefetch`. | `crates/arda-server/src/tactical/prefetch.rs:44`, `crates/arda-server/src/tactical/prefetch.rs:105` |
| Version family | One independently versioned part of the `/v1` surface (`contract_version`, `tactical_format`, `bake_format`, `npc_format`, `gazetteer_format`, …); `WorldInfo.versions` lists the current version of each. | `crates/arda-server/src/versions.rs:25` |
| `server_version` | The `arda-server` build's Cargo package version, in `WorldInfo` and `Health`; tactical, NPC and image bodies come from this build, unlike the world's `arda_version`. | `crates/arda-server/src/versions.rs:17`, `crates/arda-server/src/dto.rs:98` |
| Stability | How firmly a route or schema is frozen: `stable` (frozen for 1.x), `unstable` or `debug`. | `crates/arda-server/src/versions.rs:130` |
| Open enum | An enum that may gain values in a minor release without a version bump; a client maps an unknown value to the listed fallback. | `crates/arda-server/src/versions.rs:143` |
