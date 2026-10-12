---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: f1e231ae1505
paths_covered:
  - ":(top)crates/*/src/**"
---

# Models

Coverage. Every type that derives `Serialize`, `Deserialize`, `TS` or `JsonSchema` in the 18 product crates is listed, plus every `pub` type of `arda-blocks`, `arda-dungeon`, `arda-ids`, `arda-people` and `arda-wfc`, a curated set of `arda-refine` and `arda-midzoom` types, and the saved-world records of the original five crates. Shallow, not inventoried: private algorithm scratch types of `arda-gen` (233 `pub` types, only the saved-world and preparation records below), `arda-refine` (`Ctx`, `Eco`, `Woods`, `Prior`, `Shape`, `Phys`, `Trails` and similar), `arda-midzoom` (`Coarse`, `Detail`, `Style`, `Lattice`, `LowRelief`, `CellMasks` and similar), `arda-render` scratch rasters beyond those listed, `arda-tactical` compositor internals, and every error enum (covered by `03-conventions.md`).

Headings. A type name that occurs in more than one crate is written bare for one owner and as `<Name> (<crate>)` for each other copy; the bare heading is the TypeScript-exported server DTO where one exists (`Npc`, `Population`, `Sheet` and the rest of `npc_dto`), so `09-interfaces.md` resolves wire bodies to the wire type. `### <Name> (npc.rs runtime)` marks the two untyped-mirror runtime bodies of `crates/arda-server/src/npc.rs`.

## Entities

Storage values: `HTTP JSON; ts-rs bindings/ts/arda/<Name>.ts` means the type is in the ts-rs export list (`crates/arda-server/src/bindings.rs:81`) and has a committed generated TypeScript file; `society/...` paths are under a world directory; "read view" means a crate-local deserialisation of another crate's file.

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| TerrainKind | `crates/arda-core/src/cell.rs:11` | `areas/AX_AY/cells.bin` (u8 code) | Whether a cell is sea, dry land, or lake surface. |
| Cover | `crates/arda-core/src/cell.rs:37` | `areas/AX_AY/cells.bin` (u8 code) | Dominant ground cover (`logic/02` vegetation). |
| RoadClass | `crates/arda-core/src/cell.rs:75` | `areas/AX_AY/cells.bin` (u8 code) | Road class crossing a cell (`logic/02` roads). |
| Cell | `crates/arda-core/src/cell.rs:103` | areas/*/cells.bin | Everything the finished map knows about one 100 m cell. |
| SizeKm | `crates/arda-core/src/config.rs:17` | `world.json` | Continent extent in kilometres (`mockup/01` `--size`). |
| LatitudeBand | `crates/arda-core/src/config.rs:34` | `world.json` | The latitude belt the continent sits in (`logic/01` §Q6). |
| GenerateConfig | `crates/arda-core/src/config.rs:54` | `world.json` | A validated generation request. |
| ClimateRegime | `crates/arda-core/src/continent.rs:10` | `continent/overview.bin` | Climate regime of a 1 km cell (`logic/01` §Q6: band position and elevation). |
| ContinentCell | `crates/arda-core/src/continent.rs:38` | continent/overview.bin | One 1 km cell of the persisted continent grid (feature 02 §Q6). |
| ContinentOverview | `crates/arda-core/src/continent.rs:58` | continent/overview.bin | The whole persisted 1 km grid, row-major. |
| ContinentRiver | `crates/arda-core/src/continent.rs:70` | continent/objects.bin | A continent-scale river (`logic/01` §Q7): catchment above the feature-02 threshold, whole course at 1 km resolution. |
| ContinentObjects | `crates/arda-core/src/continent.rs:87` | continent/objects.bin | Every object stored in `continent/objects.bin`. |
| AreaCoord | `crates/arda-core/src/coords.rs:17` | in-memory | Index of a 51.2 km area tile within the continent. |
| CellCoord | `crates/arda-core/src/coords.rs:40` | in-memory | Index of a 100 m cell within one area tile. |
| SquareCoord | `crates/arda-core/src/coords.rs:77` | in-memory | Index of a five-foot square within one block. |
| KmCoord | `crates/arda-core/src/coords.rs:119` | in-memory | Index of a 1 km cell within the continent working grid. |
| GlobalCell | `crates/arda-core/src/coords.rs:169` | global hydrology and area geometry | Absolute coordinate on the shared 100 m terrain grid. |
| HeightMm | `crates/arda-core/src/fixed.rs:9` | in-memory | Elevation in millimetres above sea level; negative below. |
| TempCentiC | `crates/arda-core/src/fixed.rs:36` | in-memory | Temperature in hundredths of a degree Celsius. |
| RainfallMm | `crates/arda-core/src/fixed.rs:60` | in-memory | Annual rainfall in millimetres. |
| DischargeMilli | `crates/arda-core/src/fixed.rs:78` | in-memory | River discharge in thousandths of a cubic metre per second. |
| Block | `crates/arda-core/src/formats/blocks.rs:24` | blocks/*.tiles.zst | One 64x64 tactical block. |
| BlockArchive | `crates/arda-core/src/formats/blocks.rs:67` | blocks/*.tiles.zst | Every block generated for one area tile. |
| AreaCells (arda-core) | `crates/arda-core/src/formats/cells.rs:24` | areas/*/cells.bin | One area tile's full cell grid, row-major. |
| TableSpan | `crates/arda-core/src/formats/hydrology.rs:350` | hydrology/basins.bin | Range measured in records, not byte offsets. |
| BasinNodeRow | `crates/arda-core/src/formats/hydrology.rs:372` | hydrology/basins.bin | Fixed basin row; its children remain in a separately streamed ID table. |
| TableKind | `crates/arda-core/src/formats/hydrology/table.rs:10` | each `hydrology/*.bin` header | Stable format-4 table tags. |
| TableHeader | `crates/arda-core/src/formats/hydrology/table.rs:127` | each hydrology/*.bin table | The explicit 32-byte global table header. |
| ValidationStats | `crates/arda-core/src/formats/manifest.rs:16` | `world.json` | Plausibility statistics printed by `generate` and stored for `load` (`logic/01` step 9, `mockup/01` validation footer). |
| FineTerrainDescriptor | `crates/arda-core/src/formats/manifest.rs:35` | `world.json` | Identity of an optional canonical fine terrain layer. |
| Manifest | `crates/arda-core/src/formats/manifest.rs:67` | `world.json` | Everything needed to identify, verify, or regenerate a world. |
| ShoreClass | `crates/arda-core/src/formats/shore.rs:29` | `terrain/shore.bin` | The kind of shore a 100 m cell belongs to (goals 15 and 16). |
| IslandCause | `crates/arda-core/src/formats/shore.rs:76` | `terrain/shore.bin` | Why an island exists (goal 17). |
| LandformKind | `crates/arda-core/src/formats/shore.rs:104` | `terrain/shore.bin` | A closed basin, a plateau or a glacial trough of the landform audit. |
| LandformCause | `crates/arda-core/src/formats/shore.rs:129` | `terrain/shore.bin` | The geological reason for a landform (goal 6). |
| Landform | `crates/arda-core/src/formats/shore.rs:165` | `terrain/shore.bin` | One audited landform. |
| Island | `crates/arda-core/src/formats/shore.rs:182` | `terrain/shore.bin` | One island of the census. |
| ShoreLayer | `crates/arda-core/src/formats/shore.rs:197` | `terrain/shore.bin` | Shore classes on a regular grid plus the island census. |
| TerminalId | `crates/arda-core/src/hydrology.rs:20` | typed in-memory identity | Stable identity of a physical routing terminal. |
| BasinId | `crates/arda-core/src/hydrology.rs:24` | global/area records and child table | Stable identity of a coalesced physical basin node. |
| ReachId | `crates/arda-core/src/hydrology.rs:28` | global/area records | Directed reach identity, with bit 63 reserved for physical terminals without a step. |
| JunctionId | `crates/arda-core/src/hydrology.rs:32` | receiving-account records | Physical dry junction identity: packed global (y,x), without a separate table. |
| CatchmentId | `crates/arda-core/src/hydrology.rs:103` | global/area records | Stable identity of an immutable contributing catchment. |
| Litres | `crates/arda-core/src/hydrology.rs:109` | annual hydrology records | Exact nonnegative water volume; one millimetre over one 100 m cell is 10,000 L. |
| HydrologyDomain | `crates/arda-core/src/hydrology.rs:130` | hydrology/metadata.bin; World | Domain union includes requested fringe and any existing exported overshoot. |
| CrossingId | `crates/arda-core/src/hydrology.rs:144` | global/area crossing records | Canonical crossing pair uses GlobalCell derived Ord (x, then y). |
| ReceivingAccount | `crates/arda-core/src/hydrology.rs:153` | global/area water records | Immediate downstream destination of a directed water transfer. |
| SpillConnection | `crates/arda-core/src/hydrology.rs:168` | basin/lake/catchment records | Actual D8 neighboring cells witnessing a physical spill, oriented from the spilling source. |
| BasinNode | `crates/arda-core/src/hydrology.rs:184` | bounded in-memory API view | Zero-capacity equal-height join chains have already been coalesced. |
| AnnualWaterBalance | `crates/arda-core/src/hydrology.rs:202` | hydrology/metadata.bin | Whole-domain annual balance after every internal transfer cancels. |
| GlobalLake | `crates/arda-core/src/hydrology.rs:221` | hydrology/lakes.bin; area context | One actual connected positive-depth component in the annual representative. |
| AnnualCatchment | `crates/arda-core/src/hydrology.rs:240` | hydrology/catchments.bin; area context | Compact immutable contributing-owner identity, with no simulated reservoir state. |
| GlobalReach | `crates/arda-core/src/hydrology.rs:259` | hydrology/reaches.bin; area context | One annual discharge authority shared by every fragment of a course. |
| SharedCrossing | `crates/arda-core/src/hydrology.rs:281` | hydrology/crossings.bin; area context | Both neighboring areas reference/copy this canonical directed record. |
| ChannelEdge | `crates/arda-core/src/hydrology.rs:305` | area objects channel section | Saved render geometry derived exclusively from solved global courses. |
| AreaHydrologyReferences | `crates/arda-core/src/hydrology.rs:320` | bounded in-memory API view | Per-area lookup table, with full records stored once in global tables. |
| AreaHydrologyContext | `crates/arda-core/src/hydrology.rs:333` | area objects global section | Exact relevant copies let an area explain saved water without neighbor reads. |
| HydrologyMetadata | `crates/arda-core/src/hydrology.rs:359` | hydrology/metadata.bin | Persisted root metadata; large tables stream in separate checked sections. |
| Terminus | `crates/arda-core/src/objects.rs:14` | `areas/AX_AY/objects.bin` | How a river segment stops. |
| RiverSegment | `crates/arda-core/src/objects.rs:49` | areas/*/objects.bin | One watercourse reach: source or junction, downstream to the next junction, the sea, a lake, or the tile edge (artifact, Water). |
| Lake | `crates/arda-core/src/objects.rs:70` | areas/*/objects.bin | A body of inland standing water (artifact, Relief and Water). |
| AreaObjects | `crates/arda-core/src/objects.rs:90` | areas/*/objects.bin | Every object stored alongside one area's cells. |
| PanKind | `crates/arda-core/src/playa.rs:11` | `areas/AX_AY/water.bin` | Surface of a dry arid-basin floor (recipe 7, logic/02 §world-water arid basins): the playa around a terminal lake, or the whole floor of a basin whose lake has dried. |
| PanRun | `crates/arda-core/src/playa.rs:33` | `areas/AX_AY/water.bin` | A run of playa cells along one area row: cells `x0..x0 + len` of row `y`. |
| SeedKey | `crates/arda-core/src/rng.rs:50` | in-memory | The full key for one random stream. |
| TerrainPoint | `crates/arda-core/src/terrain.rs:11` | in-memory; canonical fine-field diagnostic | A physical position in absolute micrometres. |
| TerrainField | `crates/arda-core/src/terrain.rs:73` | in-memory; canonical fine-field diagnostic | Owned lattice heights with closed first-to-last coverage on each axis. |
| TileId | `crates/arda-core/src/tiles.rs:9` | in-memory | A tile identifier, two bytes on disk (`02-models.md`). |
| TileGroup | `crates/arda-core/src/tiles.rs:27` | in-memory | Broad tile family; adjacency is decided between groups. |
| TileDef | `crates/arda-core/src/tiles.rs:44` | in-memory | One entry in the vocabulary. |
| ChannelPattern | `crates/arda-core/src/water.rs:275` | `areas/AX_AY/water.bin` | Planform of a river segment. |
| LakeOrigin | `crates/arda-core/src/water.rs:301` | `areas/AX_AY/water.bin` | Geological origin of a lake basin. |
| SegmentForm | `crates/arda-core/src/water.rs:337` | `areas/AX_AY/water.bin` | Stored form of one area river segment, in the order of the area's rivers. |
| LakeForm | `crates/arda-core/src/water.rs:355` | `areas/AX_AY/water.bin` | Stored form of one area lake fragment, in the order of the area's lakes. |
| DeltaForm | `crates/arda-core/src/water.rs:368` | `areas/AX_AY/water.bin` | A river-mouth delta whose apex lies in this area. |
| Doline | `crates/arda-core/src/water.rs:379` | `areas/AX_AY/water.bin` | A karst doline (closed solution hollow) below the 100 m cell scale. |
| AreaWater | `crates/arda-core/src/water.rs:390` | `areas/AX_AY/water.bin` | Every stored water form of one area (`areas/<ax>_<ay>/water.bin`). |
| Hand | `crates/arda-gen/src/area/fields.rs:78` | in-memory; local diagnostic path | Height above the nearest downstream watercourse, plus how much water that watercourse carries. |
| Basin | `crates/arda-gen/src/area/fill.rs:16` | in-memory; local diagnostic path | A filled depression: the cells under water and how deep. |
| Filled | `crates/arda-gen/src/area/fill.rs:27` | in-memory; local diagnostic helper | The routing surface plus the basins that produced it. |
| SharedTerrain | `crates/arda-gen/src/area/prepare.rs:12` | private in-memory preparation owner | Physical heights shared by publication areas and modeled fringe cells. |
| ReliefGrid | `crates/arda-gen/src/area/relief.rs:12` | in-memory; local diagnostic helper | One tile's elevation field, in millimetres. |
| SolvedCell | `crates/arda-gen/src/area/shared_compose.rs:15` | in-memory composition handoff | Immutable final flow/occupancy fields for one 100 m cell. |
| WaterGrid | `crates/arda-gen/src/area/water.rs:35` | in-memory; local diagnostic path | Drainage directions, accumulation, discharge, and Strahler order for one tile. |
| BlockConstraints | `crates/arda-gen/src/block/constraints.rs:8` | in-memory | What a block may be built from, derived from its cell and the eight neighbours around it. |
| EnteringRiver | `crates/arda-gen/src/continent/bundles.rs:116` | in-memory; local diagnostic path | One watercourse crossing into the tile (logic/01 step 10). |
| TileBundle | `crates/arda-gen/src/continent/bundles.rs:142` | in-memory | One area tile's inputs, computed from coarse data and the seed only. |
| ContinentClimate | `crates/arda-gen/src/continent/climate.rs:12` | in-memory | Per-cell climate fields, row-major over the 1 km grid. |
| ContinentHydrology | `crates/arda-gen/src/continent/hydrology.rs:20` | in-memory | The continent drainage tree and its per-cell loads. |
| ContinentGrid | `crates/arda-gen/src/continent/mod.rs:38` | in-memory | The 1 km continent working grid (`logic/01` step 4). |
| Continent | `crates/arda-gen/src/continent/mod.rs:119` | in-memory | Everything the continent stage knows, threaded to bundles and areas (feature 03 §Q2 — closes the computes-and-discards seam). |
| SimExtent | `crates/arda-gen/src/continent/plates.rs:8` | in-memory | The 4 km simulation domain, roughly twice the visible continent. |
| Plate | `crates/arda-gen/src/continent/plates.rs:31` | in-memory | One tectonic plate. |
| MarineBoundary | `crates/arda-gen/src/hydrology/types.rs:8` | prepared in-memory/private files | Actual domain sides which can seed marine connectivity. |
| PreparedExtent | `crates/arda-gen/src/hydrology/types.rs:20` | prepared in-memory/private files | Valid rows and columns of one prepared tile. |
| PreparedTerrain | `crates/arda-gen/src/hydrology/types.rs:31` | in-memory; cropped private prepared files | The pure preparation function retains full 512² canonical arrays; the private writer crops valid rows only, preserving existing terrain on partial fringes. |
| HydrologyLimits | `crates/arda-gen/src/hydrology/types.rs:84` | in-memory generation API | Every reservation is checked before allocation/writing. |
| AtlasTerrain | `crates/arda-render/src/atlas.rs:58` | in-memory render scratch | Atlas palette, Q12 light, and terrain ownership for one saved area. |
| AtlasNeighbor | `crates/arda-render/src/atlas/halo.rs:19` | in-memory render context | One of the eight saved areas adjacent to the target area. |
| AtlasHalo | `crates/arda-render/src/atlas/halo.rs:49` | in-memory render scratch | Nearest two saved-cell rows or columns from each adjacent area. |
| AreaImageScale | `crates/arda-render/src/channels.rs:18` | in-memory render request | Resolution of an area image; every scale uses the same 100 m terrain. |
| AreaRaster | `crates/arda-render/src/channels.rs:164` | in-memory streaming renderer | Prepared geometry and a single reusable scanline. |
| AccountOut | `crates/arda-render/src/hydrology_json.rs:7` | export JSON | Tagged immediate receiving account with decimal identity payload |
| SpillOut | `crates/arda-render/src/hydrology_json.rs:32` | export JSON | Physical spill geometry and receiving account |
| LakeOut (hydrology_json) | `crates/arda-render/src/hydrology_json.rs:49` | export JSON | Local area lake DTO and module-scoped copied global lake DTO |
| CatchmentOut | `crates/arda-render/src/hydrology_json.rs:71` | export JSON | Copied immutable catchment and annual lake association |
| ReachOut | `crates/arda-render/src/hydrology_json.rs:93` | export JSON | Copied global reach, point flag and exact annual amount |
| CrossingOut | `crates/arda-render/src/hydrology_json.rs:119` | export JSON | Copied cross-area transfer geometry and exact annual amount |
| HydrologyOut | `crates/arda-render/src/hydrology_json.rs:145` | export JSON | Explicit annual semantics and relevant global records |
| CellOut | `crates/arda-render/src/json.rs:18` | export JSON | Area JSON cell subset |
| RiverOut | `crates/arda-render/src/json.rs:33` | export JSON | Area JSON river |
| LakeOut | `crates/arda-render/src/json.rs:45` | export JSON | Local area lake DTO and module-scoped copied global lake DTO |
| AreaOut | `crates/arda-render/src/json.rs:66` | export JSON | Versioned area JSON payload |
| ChannelEdgeOut | `crates/arda-render/src/json.rs:81` | export JSON | Global centerline endpoints, widths and mean discharge |
| LegendEntry | `crates/arda-render/src/json.rs:90` | export JSON | Block JSON tile legend |
| BlockOut | `crates/arda-render/src/json.rs:96` | export JSON | Versioned block JSON payload |
| OverviewRaster | `crates/arda-render/src/overview.rs:44` | in-memory buffered renderer | Overview-sized buffer that never retains an area's cell grid. |
| RasterBand | `crates/arda-render/src/overview/streaming.rs:343` | in-memory streaming helper | 256 output rows plus at most 14 halo rows on each side for RGB and feature classifications |
| ImageQuality | `crates/arda-render/src/quality.rs:7` | in-memory render request | PNG edge length: an area's side or an overview's longest edge. |
| OverviewLook | `crates/arda/src/export_quality.rs:138` | in-memory | Opt-in looks of an overview render. |
| MapStyle | `crates/arda/src/lib.rs:32` | in-memory render request | Cartographic presentation applied to PNG exports. |
| ExportFormat | `crates/arda/src/lib.rs:100` | in-memory | Output format for `export` (`mockup/03`). |
| World | `crates/arda/src/world.rs:19` | in-memory | An immutable stored world; layer caches fill only when queried. |
| Area | `crates/arda/src/world/area.rs:6` | in-memory | One loaded area tile. |
| Biome (arda-ids) | `crates/arda-ids/src/biome.rs:14` | wire/file JSON value | The biome a settlement reports, serialised snake_case. |
| Cover (arda-ids) | `crates/arda-ids/src/cover.rs:12` | wire/file JSON value | SRD 5.1 cover. |
| BuildingFunction (arda-ids) | `crates/arda-ids/src/function.rs:15` | wire/file JSON value | What a building is for: the vocabulary list plus `market_hall`, `mine`, `lumber_camp` and `school`. |
| Craft | `crates/arda-ids/src/function.rs:182` | wire/file JSON value | The trade practised in a `BuildingFunction::Workshop`. |
| SettlementId | `crates/arda-ids/src/ids.rs:94` | wire/file JSON value | A settlement, unique within a world. |
| RealmId | `crates/arda-ids/src/ids.rs:98` | wire/file JSON value | A realm, unique within a world. |
| BuildingId | `crates/arda-ids/src/ids.rs:102` | wire/file JSON value | A building, unique within its settlement. |
| NpcId (arda-ids) | `crates/arda-ids/src/ids.rs:106` | wire/file JSON value | A person, unique within a world. |
| LandUse | `crates/arda-ids/src/land_use.rs:15` | wire/file JSON value | The canonical land-use class stored in `society/landuse.bin` as a u8. |
| RoadClass (arda-ids) | `crates/arda-ids/src/road.rs:13` | wire/file JSON value | The canonical road class. |
| SoundChange | `crates/arda-names/src/change.rs:13` | in-memory; embedded in records (serde) | One regular sound change. |
| Isogloss | `crates/arda-names/src/dialect.rs:42` | in-memory; embedded in records (serde) | One sound change and the line it stops at. |
| DialectMap | `crates/arda-names/src/dialect.rs:66` | in-memory; embedded in records (serde) | A language spread over a realm, with dialect isoglosses. |
| Blocklist | `crates/arda-names/src/filter.rs:15` | in-memory; embedded in records (serde) | Words and fragments no generated name may use. |
| Grammar | `crates/arda-names/src/language.rs:30` | in-memory; embedded in records (serde) | Naming customs of a language. |
| Language | `crates/arda-names/src/language.rs:58` | in-memory; embedded in records (serde) | A complete naming language, deterministic in `(seed, preset)`. |
| Class | `crates/arda-names/src/meaning.rs:12` | in-memory; embedded in records (serde) | What a meaning is used for. |
| Meaning | `crates/arda-names/src/meaning.rs:48` | in-memory; embedded in records (serde) | A meaning with a morpheme in every language. |
| Ortho | `crates/arda-names/src/ortho.rs:13` | in-memory; embedded in records (serde) | A language's spelling rules. |
| Sex (arda-names) | `crates/arda-names/src/person.rs:24` | in-memory; embedded in records (serde) | Sex, for given-name endings and patronymics. |
| FamilyCtx | `crates/arda-names/src/person.rs:34` | in-memory; embedded in records (serde) | What the family and circumstances of a person supply. |
| PersonName | `crates/arda-names/src/person.rs:67` | in-memory; embedded in records (serde) | A full personal name. |
| Ph | `crates/arda-names/src/phoneme.rs:11` | in-memory; embedded in records (serde) | A phoneme: an index into `TABLE`. |
| Phonology | `crates/arda-names/src/phonology.rs:26` | in-memory; embedded in records (serde) | Inventory and phonotactic rules. |
| Stress | `crates/arda-names/src/preset/mod.rs:17` | in-memory; embedded in records (serde) | Where the main stress falls. |
| FamilyStyle | `crates/arda-names/src/preset/mod.rs:29` | in-memory; embedded in records (serde) | How family names are formed; weights live in `Spec::family`. |
| Preset | `crates/arda-names/src/preset/mod.rs:156` | in-memory; embedded in records (serde) | A culture or ancestry preset. |
| NameScope | `crates/arda-names/src/scope.rs:21` | in-memory; embedded in records (serde) | The set of names already used in a scope. |
| Tongue (arda-names) | `crates/arda-names/src/tongue.rs:18` | in-memory; embedded in records (serde) | Everything needed to rebuild a place's local language. |
| PlaceKind | `crates/arda-names/src/types.rs:12` | in-memory; embedded in records (serde) | What kind of place is being named. |
| SiteTag | `crates/arda-names/src/types.rs:107` | in-memory; embedded in records (serde) | A site tag, as `arda-settle` records them for each settlement. |
| PlaceSpec | `crates/arda-names/src/types.rs:174` | in-memory; embedded in records (serde) | What to name. |
| Part | `crates/arda-names/src/types.rs:224` | in-memory; embedded in records (serde) | One morpheme of a name, kept so names can be derived from names. |
| Name (arda-names) | `crates/arda-names/src/types.rs:235` | in-memory; embedded in records (serde) | A generated name. |
| CrossingKind | `crates/arda-settle/src/crossings.rs:21` | `society/roads.json` | How a road gets over water. |
| Crossing | `crates/arda-settle/src/crossings.rs:32` | `society/roads.json` | One crossing object. |
| Pass | `crates/arda-settle/src/crossings.rs:56` | `society/roads.json` | One pass object. |
| Tier (arda-settle) | `crates/arda-settle/src/model.rs:12` | `society/settlements.json` | Settlement size class (`arda-npc` `Tier`). |
| Function | `crates/arda-settle/src/model.rs:50` | `society/settlements.json` | What a settlement lives on (`arda-npc` `SettlementFunction`). |
| Settlement | `crates/arda-settle/src/model.rs:87` | `society/settlements.json` | One settlement, as written to `society/settlements.json`. |
| SettlementsFile | `crates/arda-settle/src/output.rs:28` | `society/settlements.json` | `settlements.json`. |
| RoadsFile | `crates/arda-settle/src/output.rs:45` | `society/roads.json` | `roads.json`. |
| RealmsFile | `crates/arda-settle/src/output.rs:58` | `society/realms.json` | `realms.json`. |
| NamedRiver | `crates/arda-settle/src/output.rs:67` | `society/names.json` | A named river. |
| NamedPeak | `crates/arda-settle/src/output.rs:87` | `society/names.json` | A named mountain. |
| NamedRegion | `crates/arda-settle/src/output.rs:100` | `society/names.json` | A named culture region. |
| NamesFile | `crates/arda-settle/src/output.rs:116` | `society/names.json` | `names.json`. |
| RealmStats | `crates/arda-settle/src/realm_stats.rs:15` | `society/stats.json` | The realm summary written into `stats.json`. |
| Realm | `crates/arda-settle/src/realms.rs:38` | `society/realms.json` | One realm, as written to `society/realms.json`. |
| Road | `crates/arda-settle/src/roads.rs:27` | `society/roads.json` | One road object: the new cells a route added to the network. |
| Terrain | `crates/arda-settle/src/roads.rs:58` | `society/roads.json` | Terrain class of a route, by the median slope along it. |
| Spread | `crates/arda-settle/src/stats.rs:19` | `society/stats.json` | Distribution of one measure over a set of routes. |
| Stats | `crates/arda-settle/src/stats.rs:52` | `society/stats.json` | The summary written to `stats.json` and printed by the CLI. |
| BuildingRef | `crates/arda-society/src/buildings.rs:15` | in-memory | One building as society sees it. |
| GoodRole | `crates/arda-society/src/economy/mod.rs:16` | `society/society.json` | How a key good moves through a market. |
| KeyGood | `crates/arda-society/src/economy/mod.rs:28` | `society/society.json` | One of a market's key trade goods. |
| SettlementEconomy | `crates/arda-society/src/economy/mod.rs:40` | `society/society.json` | Economy of one settlement. |
| RoadTraffic | `crates/arda-society/src/economy/mod.rs:53` | `society/society.json` | Traffic on one road. |
| GoodTotals | `crates/arda-society/src/economy/mod.rs:65` | `society/society.json` | World totals for one good. |
| Economy | `crates/arda-society/src/economy/mod.rs:86` | `society/society.json` | World economy. |
| TradeFlow | `crates/arda-society/src/economy/trade.rs:30` | `society/society.json` | One settlement-to-settlement shipment of one good per year. |
| Ledger | `crates/arda-society/src/economy/trade.rs:56` | `society/society.json` | Where each load of one settlement went. |
| HistoryHook | `crates/arda-society/src/history/lore.rs:17` | `society/society.json` | One line of settlement history. |
| EventKind | `crates/arda-society/src/history/mod.rs:23` | `society/society.json` | What happened. |
| Event | `crates/arda-society/src/history/mod.rs:54` | `society/society.json` | One timeline entry. |
| Dynasty | `crates/arda-society/src/history/mod.rs:85` | `society/society.json` | A ruling house. |
| Reign | `crates/arda-society/src/history/mod.rs:103` | `society/society.json` | One reign. |
| BorderShift | `crates/arda-society/src/history/mod.rs:127` | `society/society.json` | A settlement's change of realm. |
| War | `crates/arda-society/src/history/mod.rs:145` | `society/society.json` | A war between two realms. |
| Ruin | `crates/arda-society/src/history/mod.rs:164` | `society/society.json` | A ruin of an abandoned settlement. |
| Allegiance | `crates/arda-society/src/history/mod.rs:200` | `society/society.json` | The realm a settlement first belonged to. |
| History | `crates/arda-society/src/history/mod.rs:211` | `society/society.json` | The whole history. |
| Trend | `crates/arda-society/src/history/prosperity.rs:22` | `society/society.json` | Direction of a settlement's fortunes now. |
| Prosperity | `crates/arda-society/src/history/prosperity.rs:36` | `society/society.json` | Prosperity history of one settlement. |
| Hook | `crates/arda-society/src/hooks/mod.rs:32` | `society/society.json` | One plot hook. |
| Tier (arda-society) | `crates/arda-society/src/input.rs:12` | read view of `society/*.json` | Settlement size class. |
| Function (arda-society) | `crates/arda-society/src/input.rs:45` | read view of `society/*.json` | What a settlement lives on. |
| Settlement (arda-society) | `crates/arda-society/src/input.rs:95` | read view of `society/*.json` | One settlement record (`settlements.json`). |
| RoadClass (arda-society) | `crates/arda-society/src/input.rs:166` | read view of `society/*.json` | Road class (vocabulary.md I3): the world's stored codes with footpath appended. |
| Road (arda-society) | `crates/arda-society/src/input.rs:207` | read view of `society/*.json` | One road object (`roads.json`). |
| Realm (arda-society) | `crates/arda-society/src/input.rs:237` | read view of `society/*.json` | One realm (`realms.json`, `logic/06` "State transitions"). |
| BuildingSpec (arda-society) | `crates/arda-society/src/input.rs:256` | read view of `society/*.json` | An explicit building (vocabulary.md I5–I7): `function` is a plain snake_case key; a workshop's craft is the free tag `craft:<name>`. |
| WorldSettlements | `crates/arda-society/src/input.rs:272` | read view of `society/*.json` | Everything society is built from. |
| SettlementSociety | `crates/arda-society/src/lib.rs:61` | `society/society.json` | Society of one settlement. |
| RealmSociety | `crates/arda-society/src/lib.rs:100` | `society/society.json` | A realm today with its region-level plot hooks. |
| Society | `crates/arda-society/src/lib.rs:110` | `society/society.json` | The whole society layer of a world. |
| Faction | `crates/arda-society/src/politics/factions.rs:19` | `society/society.json` | One faction. |
| FactionStance | `crates/arda-society/src/politics/factions.rs:46` | `society/society.json` | How two factions stand. |
| FactionRelation | `crates/arda-society/src/politics/factions.rs:73` | `society/society.json` | Stance between factions `a < b` (by id) of one settlement. |
| Ruler | `crates/arda-society/src/politics/realm.rs:20` | `society/society.json` | The present ruler of a realm. |
| Vassal | `crates/arda-society/src/politics/realm.rs:41` | `society/society.json` | A vassal lord. |
| RealmState | `crates/arda-society/src/politics/realm.rs:65` | `society/society.json` | One realm today. |
| Stance | `crates/arda-society/src/politics/relations.rs:16` | `society/society.json` | How two realms stand. |
| Relation | `crates/arda-society/src/politics/relations.rs:45` | `society/society.json` | Relation between realms `a < b`. |
| SettlementsFile (arda-society) | `crates/arda-society/src/read.rs:23` | read view of `society/*.json` | — |
| RoadsFile (arda-society) | `crates/arda-society/src/read.rs:29` | read view of `society/*.json` | — |
| RealmsFile (arda-society) | `crates/arda-society/src/read.rs:35` | read view of `society/*.json` | — |
| EntityRef | `crates/arda-society/src/refs.rs:10` | `society/society.json` | A reference to one entity of the input world or of the society. |
| ClassLevel | `crates/arda-society/src/roles.rs:17` | `society/society.json` | An SRD 5.1 class and level. |
| NpcRole | `crates/arda-society/src/roles.rs:27` | `society/society.json` | One notable NPC slot. |
| Yield | `crates/arda-society/src/tables.rs:12` | `crates/arda-society/data/*.json` (compiled in) | `[good key, amount]`. |
| Good | `crates/arda-society/src/tables.rs:16` | `crates/arda-society/data/*.json` (compiled in) | One trade good. |
| Input | `crates/arda-society/src/tables.rs:40` | `crates/arda-society/data/*.json` (compiled in) | A processing input: `per` loads of `good` per 10 loads of output. |
| BiomeMod | `crates/arda-society/src/tables.rs:49` | `crates/arda-society/data/*.json` (compiled in) | Biome effect on land yields. |
| GoodsTable | `crates/arda-society/src/tables.rs:60` | `crates/arda-society/data/*.json` (compiled in) | `goods.json`. |
| Government | `crates/arda-society/src/tables.rs:85` | `crates/arda-society/data/*.json` (compiled in) | A government type. |
| PoliticsTable | `crates/arda-society/src/tables.rs:107` | `crates/arda-society/data/*.json` (compiled in) | `politics.json`. |
| FactionKind | `crates/arda-society/src/tables.rs:120` | `crates/arda-society/data/*.json` (compiled in) | One kind of faction. |
| Affinity | `crates/arda-society/src/tables.rs:154` | `crates/arda-society/data/*.json` (compiled in) | `[kind a, kind b, affinity]`, affinity −2 (hostile) to +2 (allied). |
| FactionTable | `crates/arda-society/src/tables.rs:158` | `crates/arda-society/data/*.json` (compiled in) | `factions.json`. |
| Suggestion | `crates/arda-society/src/tables.rs:175` | `crates/arda-society/data/*.json` (compiled in) | SRD 5.1 stat block or class suggestion. |
| RoleDef | `crates/arda-society/src/tables.rs:186` | `crates/arda-society/data/*.json` (compiled in) | One notable role kind. |
| RoleTable | `crates/arda-society/src/tables.rs:199` | `crates/arda-society/data/*.json` (compiled in) | `roles.json`. |
| HistoryTable | `crates/arda-society/src/tables.rs:206` | `crates/arda-society/data/*.json` (compiled in) | `history.json`. |
| HookDef | `crates/arda-society/src/tables.rs:238` | `crates/arda-society/data/*.json` (compiled in) | One campaign-hook template set: `[title, text]` pairs, so a title always matches its text. |
| HookTable | `crates/arda-society/src/tables.rs:245` | `crates/arda-society/data/*.json` (compiled in) | `hooks.json`. |
| CultureNames | `crates/arda-society/src/tables.rs:254` | `crates/arda-society/data/*.json` (compiled in) | Syllables for one culture. |
| NameTable | `crates/arda-society/src/tables.rs:273` | `crates/arda-society/data/*.json` (compiled in) | `names.json`. |
| TraitEntry | `crates/arda-npc/src/data/content.rs:11` | `crates/arda-npc/data/content/*.json` (compiled in) | A personality trait on one axis. |
| IdealEntry | `crates/arda-npc/src/data/content.rs:19` | `crates/arda-npc/data/content/*.json` (compiled in) | An ideal with its alignment lean and coherence tags. |
| BondEntry | `crates/arda-npc/src/data/content.rs:29` | `crates/arda-npc/data/content/*.json` (compiled in) | A bond template. |
| FlawEntry | `crates/arda-npc/src/data/content.rs:36` | `crates/arda-npc/data/content/*.json` (compiled in) | A flaw. |
| PersonalityTables | `crates/arda-npc/src/data/content.rs:43` | `crates/arda-npc/data/content/*.json` (compiled in) | `personality.json`. |
| JobData | `crates/arda-npc/src/data/content.rs:55` | `crates/arda-npc/data/content/*.json` (compiled in) | One occupation. |
| HouseholdStyle | `crates/arda-npc/src/data/content.rs:75` | `crates/arda-npc/data/content/*.json` (compiled in) | How a building's residents form households. |
| WorkplaceData | `crates/arda-npc/src/data/content.rs:86` | `crates/arda-npc/data/content/*.json` (compiled in) | Staffing of a building function. |
| CraftData | `crates/arda-npc/src/data/content.rs:98` | `crates/arda-npc/data/content/*.json` (compiled in) | A workshop craft. |
| LandJob | `crates/arda-npc/src/data/content.rs:105` | `crates/arda-npc/data/content/*.json` (compiled in) | A land job and its weights. |
| OfficeData | `crates/arda-npc/src/data/content.rs:116` | `crates/arda-npc/data/content/*.json` (compiled in) | A society office (a notable slot's `kind`, logic/14 §soc-offices). |
| Occupations | `crates/arda-npc/src/data/content.rs:149` | `crates/arda-npc/data/content/*.json` (compiled in) | `occupations.json`. |
| Culture | `crates/arda-npc/src/data/content.rs:163` | `crates/arda-npc/data/content/*.json` (compiled in) | A culture's ancestry weights (its tongue comes from `arda-names`). |
| Cultures | `crates/arda-npc/src/data/content.rs:169` | `crates/arda-npc/data/content/*.json` (compiled in) | `cultures.json`. |
| TierData | `crates/arda-npc/src/data/content.rs:180` | `crates/arda-npc/data/content/*.json` (compiled in) | Per-tier parameters. |
| LevelException | `crates/arda-npc/src/data/content.rs:195` | `crates/arda-npc/data/content/*.json` (compiled in) | Rare high-level exceptions among notables. |
| Tiers | `crates/arda-npc/src/data/content.rs:203` | `crates/arda-npc/data/content/*.json` (compiled in) | `tiers.json`. |
| BackgroundData | `crates/arda-npc/src/data/content.rs:212` | `crates/arda-npc/data/content/*.json` (compiled in) | A flavour background. |
| Backgrounds | `crates/arda-npc/src/data/content.rs:220` | `crates/arda-npc/data/content/*.json` (compiled in) | `backgrounds.json`. |
| RaceData | `crates/arda-npc/src/data/srd.rs:11` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD race (with its SRD subrace folded in). |
| SubclassData | `crates/arda-npc/src/data/srd.rs:36` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD subclass. |
| Loadout | `crates/arda-npc/src/data/srd.rs:46` | `crates/arda-npc/data/srd/*.json` (compiled in) | Default equipment for a class, by wealth band (poor, modest, rich). |
| ClassData | `crates/arda-npc/src/data/srd.rs:54` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD class. |
| PactRow | `crates/arda-npc/src/data/srd.rs:76` | `crates/arda-npc/data/srd/*.json` (compiled in) | One row of the Pact Magic table. |
| SlotTables | `crates/arda-npc/src/data/srd.rs:83` | `crates/arda-npc/data/srd/*.json` (compiled in) | SRD spell-slot tables. |
| SpellData | `crates/arda-npc/src/data/srd.rs:90` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD spell, by name and level only. |
| ExtraDamage | `crates/arda-npc/src/data/srd.rs:98` | `crates/arda-npc/data/srd/*.json` (compiled in) | Additional damage on an SRD stat-block attack. |
| ActionData | `crates/arda-npc/src/data/srd.rs:106` | `crates/arda-npc/data/srd/*.json` (compiled in) | An attack action of an SRD stat block. |
| BlockSpellcasting | `crates/arda-npc/src/data/srd.rs:116` | `crates/arda-npc/data/srd/*.json` (compiled in) | Spellcasting of an SRD stat block. |
| StatBlockData | `crates/arda-npc/src/data/srd.rs:126` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD NPC stat block. |
| ArmorData | `crates/arda-npc/src/data/srd.rs:151` | `crates/arda-npc/data/srd/*.json` (compiled in) | SRD armour. |
| WeaponData | `crates/arda-npc/src/data/srd.rs:162` | `crates/arda-npc/data/srd/*.json` (compiled in) | An SRD weapon. |
| EquipmentData | `crates/arda-npc/src/data/srd.rs:182` | `crates/arda-npc/data/srd/*.json` (compiled in) | SRD equipment lists. |
| Languages | `crates/arda-npc/src/data/srd.rs:191` | `crates/arda-npc/data/srd/*.json` (compiled in) | SRD languages. |
| Tier (arda-npc) | `crates/arda-npc/src/input.rs:17` | in-memory input; HTTP request body | Settlement size class (`logic/06` step 4). |
| SettlementFunction (arda-npc) | `crates/arda-npc/src/input.rs:44` | in-memory input; HTTP request body | What a settlement lives on. |
| SettlementProfile (arda-npc) | `crates/arda-npc/src/input.rs:94` | in-memory input; HTTP request body | One settlement to populate. |
| NotableSlot | `crates/arda-npc/src/input.rs:133` | in-memory input; HTTP request body | A notable slot fixed by the society layer (`arda-society` `NpcRole`, logic/14 §soc-offices; adapter A14): a role in a building, and the name and sex history gave the holder, if any. |
| BuildingSpec (arda-npc) | `crates/arda-npc/src/input.rs:168` | in-memory input; HTTP request body | One building of a settlement. |
| HouseholdId (arda-npc) | `crates/arda-npc/src/npc.rs:16` | `society/notables.json` (notables); regenerated otherwise | Identity of a household: its home building and index inside it. |
| Sex (arda-npc) | `crates/arda-npc/src/npc.rs:26` | `society/notables.json` (notables); regenerated otherwise | Biological sex, used for names and gendered titles. |
| Name (arda-npc) | `crates/arda-npc/src/npc.rs:35` | `society/notables.json` (notables); regenerated otherwise | A person's name. |
| JobCategory (arda-npc) | `crates/arda-npc/src/npc.rs:58` | `society/notables.json` (notables); regenerated otherwise | Broad kind of work. |
| Job (arda-npc) | `crates/arda-npc/src/npc.rs:112` | `society/notables.json` (notables); regenerated otherwise | A person's occupation. |
| SocialRank (arda-npc) | `crates/arda-npc/src/npc.rs:124` | `society/notables.json` (notables); regenerated otherwise | Standing in local society. |
| Lifestyle (arda-npc) | `crates/arda-npc/src/npc.rs:157` | `society/notables.json` (notables); regenerated otherwise | SRD 5.1 lifestyle, derived from wealth. |
| Ideal (arda-npc) | `crates/arda-npc/src/npc.rs:206` | `society/notables.json` (notables); regenerated otherwise | An ideal and the alignment it leans toward. |
| Personality (arda-npc) | `crates/arda-npc/src/npc.rs:219` | `society/notables.json` (notables); regenerated otherwise | Personality from Arda's original tables. |
| RelationKind (arda-npc) | `crates/arda-npc/src/npc.rs:238` | `society/notables.json` (notables); regenerated otherwise | How two people are related. |
| Relationship (arda-npc) | `crates/arda-npc/src/npc.rs:283` | `society/notables.json` (notables); regenerated otherwise | One relationship: `other` is this person's `kind` (e.g. |
| Npc (arda-npc) | `crates/arda-npc/src/npc.rs:292` | `society/notables.json` (notables); regenerated otherwise | One inhabitant. |
| HouseholdRelation (arda-npc) | `crates/arda-npc/src/population.rs:10` | `society/notables.json` (notables); regenerated otherwise | One family tie inside a household: `to` is `from`'s `kind`. |
| Household (arda-npc) | `crates/arda-npc/src/population.rs:21` | `society/notables.json` (notables); regenerated otherwise | People sharing a home. |
| RosterEntry (arda-npc) | `crates/arda-npc/src/population.rs:37` | `society/notables.json` (notables); regenerated otherwise | Compact index entry for one inhabitant (about 40 bytes), so queries work without expanding anyone. |
| Population (arda-npc) | `crates/arda-npc/src/population.rs:51` | `society/notables.json` (notables); regenerated otherwise | The population of one settlement. |
| Ability (arda-npc) | `crates/arda-npc/src/rules.rs:11` | `society/notables.json` (notables); regenerated otherwise | One of the six ability scores, in SRD order. |
| CasterKind | `crates/arda-npc/src/rules.rs:101` | in-memory | How a class casts spells. |
| SheetKind (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:23` | `society/notables.json` (notables); regenerated otherwise | What the sheet is built from. |
| Background (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:48` | `society/notables.json` (notables); regenerated otherwise | Original flavour background of a class build. |
| SaveBonus (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:59` | `society/notables.json` (notables); regenerated otherwise | A saving throw. |
| SkillBonus (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:70` | `society/notables.json` (notables); regenerated otherwise | A proficient skill. |
| AttackKind (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:84` | `society/notables.json` (notables); regenerated otherwise | Melee or ranged. |
| Attack (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:93` | `society/notables.json` (notables); regenerated otherwise | One attack line. |
| Item (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:114` | `society/notables.json` (notables); regenerated otherwise | An item and how many. |
| Coins (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:123` | `society/notables.json` (notables); regenerated otherwise | Coins carried. |
| SpellRef (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:134` | `society/notables.json` (notables); regenerated otherwise | A spell known or prepared. |
| Spellcasting (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:143` | `society/notables.json` (notables); regenerated otherwise | Spellcasting block. |
| Sheet (arda-npc) | `crates/arda-npc/src/sheet/mod.rs:160` | `society/notables.json` (notables); regenerated otherwise | A full SRD 5.1 sheet. |
| BuildReport | `crates/arda-people/src/build.rs:18` | in-memory | What a build produced. |
| StoredNotable | `crates/arda-people/src/files.rs:22` | `society/notables.json` | One settlement's stored notables (goal 56: only notables are stored; commoners are regenerated). |
| NotablesFile | `crates/arda-people/src/files.rs:34` | `society/notables.json` | `notables.json`. |
| SocietyFiles | `crates/arda-people/src/files.rs:45` | in-memory | Everything read from `<world>/society/`. |
| BuildingSource | `crates/arda-people/src/people.rs:22` | in-memory | Where a settlement's buildings come from. |
| World (arda-people) | `crates/arda-people/src/people.rs:43` | in-memory | A world with its `society/` directory. |
| Surroundings | `crates/arda-people/src/terrain.rs:21` | in-memory | World cells around a point, with the river channels among them. |
| FallbackKind | `crates/arda-town/src/block/fallback.rs:23` | in-memory (block JSON when serialised) | What kind of key was replaced. |
| Fallback | `crates/arda-town/src/block/fallback.rs:36` | in-memory (block JSON when serialised) | One recorded substitution. |
| Window | `crates/arda-town/src/block/mod.rs:43` | in-memory (block JSON when serialised) | A window of global squares. |
| BlockBuilding | `crates/arda-town/src/block/mod.rs:56` | in-memory (block JSON when serialised) | A plan building as seen by one block. |
| SquareRules | `crates/arda-town/src/block/mod.rs:73` | in-memory (block JSON when serialised) | Per-square rules, mirroring `arda-scene` `RulesSidecar` format 2 names. |
| Cover (arda-town) | `crates/arda-town/src/block/mod.rs:91` | in-memory (block JSON when serialised) | SRD cover levels (I16). |
| TownBlock | `crates/arda-town/src/block/mod.rs:104` | in-memory (block JSON when serialised) | A tactical block and its sidecar. |
| TownFill | `crates/arda-town/src/block/wfc/mod.rs:31` | in-memory (block JSON when serialised) | How the town fabric inside the plan is filled. |
| Relaxed | `crates/arda-town/src/block/wfc/mod.rs:42` | in-memory (block JSON when serialised) | A problem that fell back to the relaxed fill, for review (goal 47). |
| Report | `crates/arda-town/src/block/wfc/mod.rs:60` | in-memory (block JSON when serialised) | Counts of WFC problems and relaxed fills, for review and the gates. |
| BuildingFunction (arda-town) | `crates/arda-town/src/function.rs:14` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A building function key (vocabulary `function:*`). |
| Vec2 | `crates/arda-town/src/geom.rs:12` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A point or vector in world metres. |
| Side | `crates/arda-town/src/plan/grid.rs:272` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A side of a rectangle or the direction a front faces. |
| SquareRect | `crates/arda-town/src/plan/grid.rs:337` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A half-open rectangle of global squares. |
| BuildingSpec (arda-town) | `crates/arda-town/src/plan/spec.rs:14` | in-memory; `GET /v1/settlements/{id}/plan` JSON | One building as `arda-npc` reads it. |
| StreetId | `crates/arda-town/src/plan/types.rs:14` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Street index within a plan. |
| PlotId | `crates/arda-town/src/plan/types.rs:20` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Plot index within a plan. |
| BuildingId (arda-town) | `crates/arda-town/src/plan/types.rs:28` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Building id, unique within the settlement and 1-based like `arda-npc`'s `BuildingId`; stable for a given seed and site, and the key NPC homes and workplaces refer to (goal 45). |
| StreetClass | `crates/arda-town/src/plan/types.rs:46` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Street hierarchy. |
| Street | `crates/arda-town/src/plan/types.rs:60` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A street centreline. |
| FocalKind | `crates/arda-town/src/plan/types.rs:77` | in-memory; `GET /v1/settlements/{id}/plan` JSON | What the town grew around. |
| Focal | `crates/arda-town/src/plan/types.rs:93` | in-memory; `GET /v1/settlements/{id}/plan` JSON | The focal point and where the market sits. |
| MarketSquare | `crates/arda-town/src/plan/types.rs:105` | in-memory; `GET /v1/settlements/{id}/plan` JSON | The market square or village green. |
| DistrictKind | `crates/arda-town/src/plan/types.rs:116` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Functional districts (goal 36: layout reflects function). |
| Plot | `crates/arda-town/src/plan/types.rs:138` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A plot of land along a street frontage (burgage or toft). |
| Gate | `crates/arda-town/src/plan/types.rs:160` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A gate in the town wall. |
| Bridge | `crates/arda-town/src/plan/types.rs:171` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A bridge deck where a street crosses a river: rows of deck squares, each spanning the water from bank to bank. |
| TownWall | `crates/arda-town/src/plan/types.rs:184` | in-memory; `GET /v1/settlements/{id}/plan` JSON | The town wall. |
| CastleWard | `crates/arda-town/src/plan/types.rs:196` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A castle ward with its own curtain wall. |
| WealthLevel | `crates/arda-town/src/plan/types.rs:207` | in-memory; `GET /v1/settlements/{id}/plan` JSON | Coarse wealth band for kits and dressing (vocabulary `wealth:*`). |
| Door | `crates/arda-town/src/plan/types.rs:241` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A doorway: on edge `side` of global square `(x, y)` inside the building. |
| Building | `crates/arda-town/src/plan/types.rs:262` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A building on the plan. |
| TownPlan | `crates/arda-town/src/plan/types.rs:293` | in-memory; `GET /v1/settlements/{id}/plan` JSON | A complete town plan. |
| Fabric | `crates/arda-town/src/plan/types.rs:356` | in-memory; `GET /v1/settlements/{id}/plan` JSON | The settlement facts behind its building materials (logic/10 §building-materials): the record's wealth, trade, biome and site, and the society's history (great fires, a long golden age) when it… |
| SettlementId (arda-town) | `crates/arda-town/src/site.rs:18` | read view of `society/settlements.json` | Settlement id, as in `settlements.json` (1-based). |
| Tier (arda-town) | `crates/arda-town/src/site.rs:28` | read view of `society/settlements.json` | Settlement size class (`arda-settle` `Tier`). |
| SettlementFunction (arda-town) | `crates/arda-town/src/site.rs:50` | read view of `society/settlements.json` | What a settlement lives on (`arda-settle` `Function`). |
| TownSite | `crates/arda-town/src/site.rs:79` | read view of `society/settlements.json` | One settlement record. |
| RoadClass (arda-town) | `crates/arda-town/src/site.rs:177` | read view of `society/settlements.json` | Road class with the world's stored codes (vocabulary.md I3): `none = 0, track = 1, road = 2, highway = 3, footpath = 4`. |
| EnteringRoad | `crates/arda-town/src/site.rs:230` | read view of `society/settlements.json` | A road polyline passing through or ending at the settlement. |
| RiverLine | `crates/arda-town/src/site.rs:239` | read view of `society/settlements.json` | A river centreline in world metres. |
| Dir | `crates/arda-wfc/src/rules.rs:12` | in-memory | A grid direction; y grows south. |
| Rules | `crates/arda-wfc/src/rules.rs:75` | in-memory | Symmetric adjacency rules over `tiles` tiles and `kinds` edge kinds. |
| TileSet | `crates/arda-wfc/src/set.rs:10` | in-memory | A set of tile ids `0..MAX_TILES`. |
| Anchor (arda-wfc) | `crates/arda-wfc/src/solve.rs:27` | in-memory | Before the free fill, make at least `count` of these options hold. |
| Limit | `crates/arda-wfc/src/solve.rs:37` | in-memory | At most `max` cells of `cells` (every cell when empty) take a tile of `tiles`. |
| Problem | `crates/arda-wfc/src/solve.rs:51` | in-memory | One WFC problem over a `w × h` grid. |
| Solution | `crates/arda-wfc/src/solve.rs:77` | in-memory | A solved grid. |
| Failure | `crates/arda-wfc/src/solve.rs:86` | in-memory | Every attempt failed; the caller applies its relaxed fill. |
| Block (arda-refine) | `crates/arda-refine/src/block.rs:35` | in-memory | One refined 64 × 64 block. |
| Class (arda-refine) | `crates/arda-refine/src/classes.rs:8` | in-memory | A natural ground class, assigned to square corners by the WFC. |
| Map | `crates/arda-refine/src/output.rs:102` | `layout.json` + `meta.json` (arda-refine CLI) | A layout plus its rules sidecar and metadata. |
| WaterRegion | `crates/arda-refine/src/region.rs:69` | in-memory | Water geometry over a region of global squares. |
| PlacementMeta | `crates/arda-refine/src/rules.rs:23` | `meta.json` / sidecar JSON | Metadata for one placement, parallel to the layout's placements. |
| BlockMeta | `crates/arda-refine/src/rules.rs:34` | `meta.json` / sidecar JSON | How one block was generated. |
| MapMeta | `crates/arda-refine/src/rules.rs:47` | `meta.json` / sidecar JSON | The refiner's metadata beside a layout. |
| SquareRules (arda-refine) | `crates/arda-refine/src/rules.rs:68` | `meta.json` / sidecar JSON | Full per-square rules as the refiner computes them. |
| Kind | `crates/arda-refine/src/scatter.rs:22` | in-memory | What a placement stands for, independent of art. |
| Item (arda-refine) | `crates/arda-refine/src/scatter.rs:47` | in-memory | One placed item in global square units. |
| CellKey | `crates/arda-refine/src/source.rs:15` | in-memory | A global 100 m cell coordinate. |
| Edge (arda-refine) | `crates/arda-refine/src/source.rs:38` | in-memory | One saved channel centreline step between neighbouring cells. |
| LakeInfo | `crates/arda-refine/src/source.rs:53` | in-memory | A lake as seen from one of its cells. |
| Water | `crates/arda-refine/src/terrain.rs:15` | in-memory | Which water, if any, covers a square. |
| TileId (arda-refine) | `crates/arda-refine/src/tiles.rs:20` | in-memory | Index of a tile in `vocabulary`. |
| WorldSource | `crates/arda-refine/src/world_source.rs:48` | in-memory | A stored world as a refinement source. |
| FallbackKind (arda-ways) | `crates/arda-ways/src/fallback.rs:14` | in-memory; sidecar JSON when serialised | What kind of key was substituted. |
| Fallback (arda-ways) | `crates/arda-ways/src/fallback.rs:25` | in-memory; sidecar JSON when serialised | One recorded substitution. |
| RoadClass (arda-ways) | `crates/arda-ways/src/input.rs:21` | in-memory input | Road class with the canonical stored codes (vocabulary I3): `none = 0, track = 1, road = 2, highway = 3, footpath = 4`. |
| Raw | `crates/arda-ways/src/input.rs:117` | in-memory input | — |
| Road (arda-ways) | `crates/arda-ways/src/input.rs:150` | in-memory input | One road object (`arda-settle` `Road`). |
| CrossingKind (arda-ways) | `crates/arda-ways/src/input.rs:167` | in-memory input | How a road gets over water (`arda-settle` `CrossingKind`). |
| Crossing (arda-ways) | `crates/arda-ways/src/input.rs:178` | in-memory input | One crossing object (`arda-settle` `Crossing`). |
| RiverChannel | `crates/arda-ways/src/input.rs:207` | in-memory input | A river channel at 5-ft resolution: centreline, bank-to-bank width and thalweg depth. |
| CrossingReport | `crates/arda-ways/src/lib.rs:36` | in-memory; sidecar JSON when serialised | A resolved crossing, for reports. |
| WaysReport | `crates/arda-ways/src/lib.rs:58` | in-memory; sidecar JSON when serialised | What `apply_ways` did, beyond the sidecar. |
| Feature | `crates/arda-ways/src/sidecar.rs:23` | in-memory; sidecar JSON when serialised | What a square is, as far as ways are concerned. |
| Cover (arda-ways) | `crates/arda-ways/src/sidecar.rs:60` | in-memory; sidecar JSON when serialised | SRD cover a square grants (vocabulary I16). |
| SquareRules (arda-ways) | `crates/arda-ways/src/sidecar.rs:78` | in-memory; sidecar JSON when serialised | Per-square rules. |
| EdgeRole (arda-ways) | `crates/arda-ways/src/sidecar.rs:114` | in-memory; sidecar JSON when serialised | Why an edge exists. |
| EdgeRule (arda-ways) | `crates/arda-ways/src/sidecar.rs:126` | in-memory; sidecar JSON when serialised | Movement and sight rules for one wall segment of the layout. |
| Sidecar | `crates/arda-ways/src/sidecar.rs:144` | in-memory; sidecar JSON when serialised | The sidecar document. |
| CompoundKind | `crates/arda-fields/src/compounds/mod.rs:59` | in-memory; sidecar JSON when serialised | What a compound is. |
| Fallback (arda-fields) | `crates/arda-fields/src/degrade.rs:18` | in-memory; sidecar JSON when serialised | One substitution, counted over the layout. |
| FieldKind | `crates/arda-fields/src/fields.rs:15` | in-memory; sidecar JSON when serialised | What a field is used for. |
| Crop | `crates/arda-fields/src/fields.rs:50` | in-memory; sidecar JSON when serialised | What a field (or strip) looks like this season. |
| Field | `crates/arda-fields/src/fields.rs:96` | in-memory; sidecar JSON when serialised | One field: a connected parcel of squares with one use. |
| LandUse (arda-fields) | `crates/arda-fields/src/input.rs:11` | in-memory input | Land use of one 100 m cell (goal 39; `04-settlements-roads.md` step 4), exactly the canonical classes of `vocabulary.md`. |
| LandUseGrid | `crates/arda-fields/src/input.rs:41` | in-memory input | A dense land-use raster. |
| Region (arda-fields) | `crates/arda-fields/src/input.rs:137` | in-memory input | The landscape region, which picks the default field boundary. |
| Tier (arda-fields) | `crates/arda-fields/src/input.rs:149` | in-memory input | Settlement tiers, as in `04-settlements-roads.md` step 3. |
| Settlement (arda-fields) | `crates/arda-fields/src/input.rs:162` | in-memory input | A neighbouring settlement. |
| RoadClass (arda-fields) | `crates/arda-fields/src/input.rs:178` | in-memory input | Road classes with the world's stored codes (`vocabulary.md`, I3): `none = 0, track = 1, road = 2, highway = 3, footpath = 4`. |
| Road (arda-fields) | `crates/arda-fields/src/input.rs:242` | in-memory input | A road polyline in world metres. |
| RiverLine (arda-fields) | `crates/arda-fields/src/input.rs:252` | in-memory input | A straight river channel piece in world metres (between two cell centres, as the world stores them). |
| SrdCover | `crates/arda-fields/src/sidecar.rs:19` | in-memory; sidecar JSON when serialised | SRD cover levels in the canonical spelling (`vocabulary.md`, I16). |
| Sidecar (arda-fields) | `crates/arda-fields/src/sidecar.rs:47` | in-memory; sidecar JSON when serialised | SRD rules and records for one window. |
| SquareRules (arda-fields) | `crates/arda-fields/src/sidecar.rs:74` | in-memory; sidecar JSON when serialised | Rules of one square, with the `RulesSidecar` field names (I9). |
| EdgeRules | `crates/arda-fields/src/sidecar.rs:103` | in-memory; sidecar JSON when serialised | Rules of one edge segment. |
| FieldRecord | `crates/arda-fields/src/sidecar.rs:128` | in-memory; sidecar JSON when serialised | A field record. |
| CompoundRecord | `crates/arda-fields/src/sidecar.rs:149` | in-memory; sidecar JSON when serialised | A compound record. |
| DemoOverlays | `crates/arda-blocks/src/demo/mod.rs:33` | in-memory | Overlays built from the synthetic samples, anchored at a world cell. |
| Owner | `crates/arda-blocks/src/overlays.rs:28` | in-memory | Who reserves a square, in rising precedence (logic/09 §reservations: town 1 > ways 2 > water 3 > fields 4 > natural ground). |
| OverlayCtx | `crates/arda-blocks/src/overlays.rs:75` | in-memory | What an overlay provider sees: the window in world squares, the world seed, the refined base layout and the library the result must resolve against. |
| OverlayLayer | `crates/arda-blocks/src/overlays.rs:94` | in-memory | One overlay's contribution to a window. |
| Composed | `crates/arda-blocks/src/pipeline.rs:23` | in-memory | A composed window: what the compositor draws and what the scene reads. |
| Pipeline | `crates/arda-blocks/src/pipeline.rs:70` | in-memory | Refines and composes tactical windows of one world. |
| LandUseRaster | `crates/arda-blocks/src/society/layers.rs:84` | in-memory | The world's land-use raster, one code per 100 m cell. |
| SocietyOverlays | `crates/arda-blocks/src/society/mod.rs:91` | in-memory | The world's society as the three overlay layers. |
| Catalog | `crates/arda-tactical/src/catalog.rs:15` | library `catalog.json` | A whole asset library: metadata plus every asset record. |
| TagVocabulary | `crates/arda-tactical/src/catalog.rs:34` | library `catalog.json` | The controlled vocabularies that asset tags are checked against. |
| Asset | `crates/arda-tactical/src/catalog.rs:52` | library `catalog.json` | One asset record. |
| AssetClass | `crates/arda-tactical/src/catalog.rs:141` | library `catalog.json` | The asset classes of goal 59. |
| Footprint | `crates/arda-tactical/src/catalog.rs:168` | library `catalog.json` | Footprint in squares. |
| Anchor | `crates/arda-tactical/src/catalog.rs:178` | library `catalog.json` | A pivot point, in squares from the image's top-left corner. |
| Tags | `crates/arda-tactical/src/catalog.rs:189` | library `catalog.json` | Classification tags. |
| PlacementRules | `crates/arda-tactical/src/catalog.rs:210` | library `catalog.json` | Placement rules for dressing (goal 64 consumes these). |
| Cover (arda-tactical) | `crates/arda-tactical/src/catalog.rs:231` | library `catalog.json` | SRD cover levels. |
| Light | `crates/arda-tactical/src/catalog.rs:248` | library `catalog.json` | A light source. |
| Layer | `crates/arda-tactical/src/catalog.rs:258` | library `catalog.json` | Render layers, drawn in declaration order (goal 62). |
| WallPiece | `crates/arda-tactical/src/catalog.rs:276` | library `catalog.json` | Membership of a wall kit. |
| WallRole | `crates/arda-tactical/src/catalog.rs:288` | library `catalog.json` | Wall-kit piece roles. |
| TacticalLayout | `crates/arda-tactical/src/layout.rs:15` | layout JSON (`*.layout.json`, `layout.json`) | A W × H battle map. |
| Square | `crates/arda-tactical/src/layout.rs:43` | layout JSON (`*.layout.json`, `layout.json`) | One 5-ft square. |
| EdgeAxis | `crates/arda-tactical/src/layout.rs:69` | layout JSON (`*.layout.json`, `layout.json`) | Which edge of a square a segment lies on. |
| WallSegment | `crates/arda-tactical/src/layout.rs:80` | layout JSON (`*.layout.json`, `layout.json`) | A wall-kit edge feature. |
| AssetRef | `crates/arda-tactical/src/layout.rs:141` | layout JSON (`*.layout.json`, `layout.json`) | An asset chosen by id or by a deterministic tag query. |
| Placement | `crates/arda-tactical/src/layout.rs:161` | layout JSON (`*.layout.json`, `layout.json`) | A prop or vegetation placement. |
| LightSource | `crates/arda-tactical/src/layout.rs:179` | layout JSON (`*.layout.json`, `layout.json`) | A light source not attached to an asset. |
| RulesSidecar | `crates/arda-scene/src/sidecar.rs:37` | rules sidecar JSON (format 2) | Per-square rules produced beside a layout. |
| EdgeRole | `crates/arda-scene/src/sidecar.rs:56` | rules sidecar JSON (format 2) | Why a rules edge exists. |
| EdgeRule | `crates/arda-scene/src/sidecar.rs:73` | rules sidecar JSON (format 2) | Movement and sight rules for one unit edge, in layout edge coordinates (as `WallSegment`: `x`, `y` and `axis`). |
| RulesCell | `crates/arda-scene/src/sidecar.rs:94` | rules sidecar JSON (format 2) | One square's rules. |
| Sq | `crates/arda-scene/src/types.rs:16` | scene JSON (format 1) | A square, as `[x, y]` in JSON. |
| Movement | `crates/arda-scene/src/types.rs:35` | scene JSON (format 1) | How a creature moves through a square (SRD 5.1, "Movement"). |
| CoverLevel | `crates/arda-scene/src/types.rs:62` | scene JSON (format 1) | SRD 5.1 degrees of cover, ordered from least to most protective. |
| Obscurement | `crates/arda-scene/src/types.rs:92` | scene JSON (format 1) | SRD 5.1 vision obscurement of a square. |
| WallKind | `crates/arda-scene/src/types.rs:105` | scene JSON (format 1) | The type of a wall-edge feature. |
| Wall | `crates/arda-scene/src/types.rs:129` | scene JSON (format 1) | A wall polyline on grid lines. |
| BlockerKind | `crates/arda-scene/src/types.rs:158` | scene JSON (format 1) | Where a vision blocker comes from. |
| VisionBlocker | `crates/arda-scene/src/types.rs:168` | scene JSON (format 1) | A polygon that obscures vision. |
| SceneLight | `crates/arda-scene/src/types.rs:182` | scene JSON (format 1) | A light source with SRD bright and dim radii. |
| RegionKind | `crates/arda-scene/src/types.rs:202` | scene JSON (format 1) | What a region marks. |
| Region | `crates/arda-scene/src/types.rs:214` | scene JSON (format 1) | Merged squares of one kind, as closed rings on grid vertices. |
| Edge | `crates/arda-scene/src/types.rs:224` | scene JSON (format 1) | A map edge. |
| EdgeExit | `crates/arda-scene/src/types.rs:239` | scene JSON (format 1) | A run of enterable squares along one map edge, for travel to the neighbouring map. |
| Entrance | `crates/arda-scene/src/types.rs:251` | scene JSON (format 1) | A square beside a door or gate. |
| SpawnHints | `crates/arda-scene/src/types.rs:261` | scene JSON (format 1) | Where tokens may be put. |
| Scene | `crates/arda-scene/src/types.rs:274` | scene JSON (format 1) | Game-facing scene data for one tactical layout. |
| TokenSlot | `crates/arda-scene/src/types.rs:322` | scene JSON (format 1) | Where an NPC's token stands on the map. |
| Kind (arda-dungeon) | `crates/arda-dungeon/src/lib.rs:54` | in-memory | What to generate. |
| Params | `crates/arda-dungeon/src/lib.rs:88` | in-memory | Generation parameters. |
| Purpose | `crates/arda-dungeon/src/lib.rs:119` | in-memory | The purpose of a built room, which decides its dressing. |
| Room | `crates/arda-dungeon/src/lib.rs:142` | in-memory | A room: its purpose and its floor rectangle in squares. |
| Door (arda-dungeon) | `crates/arda-dungeon/src/lib.rs:159` | in-memory | A door on a square edge, in layout edge terms (as `WallSegment`). |
| ExitKind | `crates/arda-dungeon/src/lib.rs:175` | in-memory | A way in or out of the level. |
| Exit | `crates/arda-dungeon/src/lib.rs:186` | in-memory | An exit: its kind and the square a token arrives on. |
| Dungeon | `crates/arda-dungeon/src/lib.rs:197` | in-memory; `out/dungeon/layout.json` via CLI | A generated level. |
| HeightTile | `crates/arda-midzoom/src/height.rs:11` | in-memory | Refined heights on a lattice `n` times finer than the stored one. |
| Landscape | `crates/arda-midzoom/src/land/mod.rs:54` | in-memory | A world's society data as relief tiles read it. |
| LandWindow | `crates/arda-midzoom/src/land/mod.rs:142` | in-memory | Everything one window's land layer reads. |
| Tone | `crates/arda-midzoom/src/land/tone.rs:15` | in-memory | What a piece of worked land shows from above. |
| Pyramid | `crates/arda-midzoom/src/pyramid.rs:17` | in-memory | Pyramid geometry of one world. |
| CellInfo | `crates/arda-midzoom/src/source.rs:20` | in-memory | The saved facts refinement reads from one 100 m cell. |
| AreaFacts | `crates/arda-midzoom/src/source.rs:57` | in-memory | The saved facts of one area that refinement and rivers read (≈ 2 MB). |
| WorldTerrain | `crates/arda-midzoom/src/source.rs:67` | in-memory | A stored `World` as a `Terrain`: the verified fine layer read in 128-node chunks and cached (pure reads only, so caching never changes an answer), saved cells through the world's own area cache. |
| Rgba | `crates/arda-midzoom/src/tile.rs:28` | in-memory | A rendered RGBA window. |
| WindowWater | `crates/arda-midzoom/src/water.rs:36` | in-memory | The water of one relief window. |
| ReliefWorld | `crates/arda-midzoom/src/world.rs:22` | in-memory | A world ready for relief rendering. |
| Manifest (arda-art-import) | `crates/arda-art-import/src/manifest.rs:36` | `import.toml` | The whole manifest. |
| LibrarySection | `crates/arda-art-import/src/manifest.rs:51` | `import.toml` | `library`. |
| GradeSection | `crates/arda-art-import/src/manifest.rs:71` | `import.toml` | `grade`. |
| AssetEntry | `crates/arda-art-import/src/manifest.rs:85` | `import.toml` | `[asset]`: matches a raw file by `file` name, or every file whose parsed id is `id` when `file` is absent. |
| HoleMode | `crates/arda-art-import/src/manifest.rs:138` | `import.toml` | What to do with backdrop-coloured pockets the border flood cannot reach. |
| ShadowMode | `crates/arda-art-import/src/manifest.rs:148` | `import.toml` | What to do with a detected baked shadow. |
| Status | `crates/arda-art-import/src/report.rs:11` | `report.json` | Whether an asset made it into the catalogue. |
| AssetReport | `crates/arda-art-import/src/report.rs:21` | `report.json` | One processed raw file. |
| Skipped | `crates/arda-art-import/src/report.rs:38` | `report.json` | A raw file that could not be mapped to an asset. |
| Coverage | `crates/arda-art-import/src/report.rs:47` | `report.json` | Which vocabulary slots the import covers. |
| Report (arda-art-import) | `crates/arda-art-import/src/report.rs:58` | `report.json` | The whole report. |
| Legend | `crates/arda-server/src/columnar.rs:25` | HTTP JSON; ts-rs `bindings/ts/arda/Legend.ts` | Enum codes: a column value `i` means the `i`-th entry. |
| AreaColumns | `crates/arda-server/src/columnar.rs:70` | HTTP JSON; ts-rs `bindings/ts/arda/AreaColumns.ts` | One array per `CellSample` field, each 512 × 512 long. |
| AreaCells | `crates/arda-server/src/columnar.rs:237` | HTTP JSON; ts-rs `bindings/ts/arda/AreaCells.ts` | `/v1/area/{ax}/{ay}/cells` JSON body. |
| TerrainKindDto | `crates/arda-server/src/contract/mod.rs:39` | HTTP JSON; ts-rs `bindings/ts/arda/TerrainKindDto.ts` | Sea, dry land, or lake surface (source: `Cell::terrain`). |
| CoverDto | `crates/arda-server/src/contract/mod.rs:51` | HTTP JSON; ts-rs `bindings/ts/arda/CoverDto.ts` | Dominant ground cover (source: `Cell::cover`, `logic/02` vegetation). |
| RoadDto | `crates/arda-server/src/contract/mod.rs:71` | HTTP JSON; ts-rs `bindings/ts/arda/RoadDto.ts` | Road class crossing the cell (source: `Cell::road`). |
| LandUseDto | `crates/arda-server/src/contract/mod.rs:87` | HTTP JSON; ts-rs `bindings/ts/arda/LandUseDto.ts` | Land-use class of a cell (`society/landuse.bin`, logic/08 §landuse keys). |
| CoastSample | `crates/arda-server/src/contract/mod.rs:114` | HTTP JSON; ts-rs `bindings/ts/arda/CoastSample.ts` | Derived coast facts. |
| SnowSample | `crates/arda-server/src/contract/mod.rs:124` | HTTP JSON; ts-rs `bindings/ts/arda/SnowSample.ts` | Derived snow proxy. |
| RiverMembership | `crates/arda-server/src/contract/mod.rs:138` | HTTP JSON; ts-rs `bindings/ts/arda/RiverMembership.ts` | The watercourse reach that owns this cell, when a saved river course passes through it. |
| LakeMembership | `crates/arda-server/src/contract/mod.rs:153` | HTTP JSON; ts-rs `bindings/ts/arda/LakeMembership.ts` | The lake covering this cell, when a saved lake lists it. |
| FineHeights | `crates/arda-server/src/contract/mod.rs:168` | HTTP JSON; ts-rs `bindings/ts/arda/FineHeights.ts` | Canonical fine terrain (`terrain/fine.bin`, 39.0625 m lattice) over the cell. |
| CellSample | `crates/arda-server/src/contract/mod.rs:179` | HTTP JSON; ts-rs `bindings/ts/arda/CellSample.ts` | Everything the world knows about one 100 m global cell, in plain units. |
| HeightSource | `crates/arda-server/src/contract/mod.rs:257` | HTTP JSON; ts-rs `bindings/ts/arda/HeightSource.ts` | Where a `PointSample` height came from. |
| PointSample | `crates/arda-server/src/contract/mod.rs:266` | HTTP JSON; ts-rs `bindings/ts/arda/PointSample.ts` | A height query at arbitrary world metres, plus the nearest cell's sample. |
| SizeKmDto | `crates/arda-server/src/dto.rs:16` | HTTP JSON; ts-rs `bindings/ts/arda/SizeKmDto.ts` | Continent extent in kilometres (manifest `config.size_km`). |
| LatitudeDto | `crates/arda-server/src/dto.rs:25` | HTTP JSON; ts-rs `bindings/ts/arda/LatitudeDto.ts` | Latitude belt the continent sits in, degrees north. |
| FineTerrainDto | `crates/arda-server/src/dto.rs:34` | HTTP JSON; ts-rs `bindings/ts/arda/FineTerrainDto.ts` | Canonical fine terrain descriptor. |
| TilePyramidDto | `crates/arda-server/src/dto.rs:43` | HTTP JSON; ts-rs `bindings/ts/arda/TilePyramidDto.ts` | Slippy overview tile pyramid geometry. |
| WorldInfo | `crates/arda-server/src/dto.rs:61` | HTTP JSON; ts-rs `bindings/ts/arda/WorldInfo.ts` | `/v1/world`: identity, size and grids of the served world. |
| Health | `crates/arda-server/src/dto.rs:133` | HTTP JSON; ts-rs `bindings/ts/arda/Health.ts` | `/v1/health`. |
| TerminusDto | `crates/arda-server/src/dto.rs:145` | HTTP JSON; ts-rs `bindings/ts/arda/TerminusDto.ts` | How a river segment ends. |
| RiverDto | `crates/arda-server/src/dto.rs:173` | HTTP JSON; ts-rs `bindings/ts/arda/RiverDto.ts` | One saved watercourse reach fragment inside an area. |
| LakeDto | `crates/arda-server/src/dto.rs:194` | HTTP JSON; ts-rs `bindings/ts/arda/LakeDto.ts` | One saved lake fragment inside an area. |
| AreaRivers | `crates/arda-server/src/dto.rs:211` | HTTP JSON; ts-rs `bindings/ts/arda/AreaRivers.ts` | `/v1/area/{ax}/{ay}/rivers`. |
| AreaLakes | `crates/arda-server/src/dto.rs:224` | HTTP JSON; ts-rs `bindings/ts/arda/AreaLakes.ts` | `/v1/area/{ax}/{ay}/lakes`. |
| ApiError | `crates/arda-server/src/error.rs:72` | HTTP JSON; ts-rs `bindings/ts/arda/ApiError.ts` | JSON body of every error response. |
| NotYetError | `crates/arda-server/src/error.rs:79` | HTTP JSON; ts-rs `bindings/ts/arda/NotYetError.ts` | The 501 body of a feature whose source crate does not exist yet. |
| ApiErrorBody | `crates/arda-server/src/error.rs:88` | HTTP JSON; ts-rs `bindings/ts/arda/ApiErrorBody.ts` | Machine-readable code, HTTP status and human message. |
| PopulationRequest (npc.rs runtime) | `crates/arda-server/src/npc.rs:23` | HTTP JSON (runtime body) | Body of `POST /v1/npc/population` (TS: `PopulationRequest`). |
| NpcDemo (npc.rs runtime) | `crates/arda-server/src/npc.rs:34` | HTTP JSON (runtime body) | Body of `GET /v1/npc/demo` (TS: `NpcDemo`). |
| Tier | `crates/arda-server/src/npc_dto/input.rs:11` | HTTP JSON; ts-rs `bindings/ts/arda/Tier.ts` | Settlement size class. |
| SettlementFunction | `crates/arda-server/src/npc_dto/input.rs:25` | HTTP JSON; ts-rs `bindings/ts/arda/SettlementFunction.ts` | What a settlement lives on. |
| Biome | `crates/arda-server/src/npc_dto/input.rs:55` | HTTP JSON; ts-rs `bindings/ts/arda/Biome.ts` | The closed settlement biome list. |
| SettlementProfile | `crates/arda-server/src/npc_dto/input.rs:79` | HTTP JSON; ts-rs `bindings/ts/arda/SettlementProfile.ts` | One settlement to populate. |
| Tongue | `crates/arda-server/src/npc_dto/input.rs:116` | HTTP JSON; ts-rs `bindings/ts/arda/Tongue.ts` | Recipe of a place's local speech (`arda_names::Tongue`): seeds are decimal strings. |
| BuildingFunction | `crates/arda-server/src/npc_dto/input.rs:142` | HTTP JSON; ts-rs `bindings/ts/arda/BuildingFunction.ts` | What a building is for: a plain snake_case string (vocabulary I7). |
| BuildingSpec | `crates/arda-server/src/npc_dto/input.rs:212` | HTTP JSON; ts-rs `bindings/ts/arda/BuildingSpec.ts` | One building of a settlement. |
| NpcId | `crates/arda-server/src/npc_dto/mod.rs:27` | HTTP JSON; ts-rs `bindings/ts/arda/NpcId.ts` | An inhabitant's id: the `arda-ids` u64 as a decimal string. |
| HouseholdId | `crates/arda-server/src/npc_dto/mod.rs:32` | HTTP JSON; ts-rs `bindings/ts/arda/HouseholdId.ts` | A household: its home building and index inside it. |
| Sex | `crates/arda-server/src/npc_dto/mod.rs:42` | HTTP JSON; ts-rs `bindings/ts/arda/Sex.ts` | Biological sex. |
| Name | `crates/arda-server/src/npc_dto/mod.rs:52` | HTTP JSON; ts-rs `bindings/ts/arda/Name.ts` | A person's name. |
| JobCategory | `crates/arda-server/src/npc_dto/mod.rs:64` | HTTP JSON; ts-rs `bindings/ts/arda/JobCategory.ts` | Broad kind of work. |
| Job | `crates/arda-server/src/npc_dto/mod.rs:96` | HTTP JSON; ts-rs `bindings/ts/arda/Job.ts` | A person's occupation. |
| SocialRank | `crates/arda-server/src/npc_dto/mod.rs:108` | HTTP JSON; ts-rs `bindings/ts/arda/SocialRank.ts` | Standing in local society. |
| Lifestyle | `crates/arda-server/src/npc_dto/mod.rs:126` | HTTP JSON; ts-rs `bindings/ts/arda/Lifestyle.ts` | SRD 5.1 lifestyle. |
| Ideal | `crates/arda-server/src/npc_dto/mod.rs:146` | HTTP JSON; ts-rs `bindings/ts/arda/Ideal.ts` | An ideal and its alignment lean. |
| Personality | `crates/arda-server/src/npc_dto/mod.rs:160` | HTTP JSON; ts-rs `bindings/ts/arda/Personality.ts` | Personality. |
| RelationKind | `crates/arda-server/src/npc_dto/mod.rs:178` | HTTP JSON; ts-rs `bindings/ts/arda/RelationKind.ts` | How two people are related. |
| Relationship | `crates/arda-server/src/npc_dto/mod.rs:204` | HTTP JSON; ts-rs `bindings/ts/arda/Relationship.ts` | One relationship: `other` is this person's `kind`. |
| Npc | `crates/arda-server/src/npc_dto/mod.rs:214` | HTTP JSON; ts-rs `bindings/ts/arda/Npc.ts` | One inhabitant with a full SRD 5.1 sheet (`logic/13` §npc-game-schema). |
| HouseholdRelation | `crates/arda-server/src/npc_dto/mod.rs:258` | HTTP JSON; ts-rs `bindings/ts/arda/HouseholdRelation.ts` | One family tie inside a household: `to` is `from`'s `kind`. |
| Household | `crates/arda-server/src/npc_dto/mod.rs:270` | HTTP JSON; ts-rs `bindings/ts/arda/Household.ts` | People sharing a home. |
| RosterEntry | `crates/arda-server/src/npc_dto/mod.rs:286` | HTTP JSON; ts-rs `bindings/ts/arda/RosterEntry.ts` | Compact index entry for one inhabitant. |
| Population | `crates/arda-server/src/npc_dto/mod.rs:300` | HTTP JSON; ts-rs `bindings/ts/arda/Population.ts` | The population of one settlement; notables stored in full. |
| PopulationRequest | `crates/arda-server/src/npc_dto/mod.rs:322` | HTTP JSON; ts-rs `bindings/ts/arda/PopulationRequest.ts` | Body of `POST /v1/npc/population`. |
| NpcDemo | `crates/arda-server/src/npc_dto/mod.rs:334` | HTTP JSON; ts-rs `bindings/ts/arda/NpcDemo.ts` | Body of `GET /v1/npc/demo`: the market-town example and its population. |
| Ability | `crates/arda-server/src/npc_dto/sheet.rs:9` | HTTP JSON; ts-rs `bindings/ts/arda/Ability.ts` | One of the six abilities, in SRD order. |
| SheetKind | `crates/arda-server/src/npc_dto/sheet.rs:33` | HTTP JSON; ts-rs `bindings/ts/arda/SheetKind.ts` | What the sheet is built from. |
| Background | `crates/arda-server/src/npc_dto/sheet.rs:59` | HTTP JSON; ts-rs `bindings/ts/arda/Background.ts` | Flavour background of a class build. |
| SaveBonus | `crates/arda-server/src/npc_dto/sheet.rs:71` | HTTP JSON; ts-rs `bindings/ts/arda/SaveBonus.ts` | A saving throw. |
| SkillBonus | `crates/arda-server/src/npc_dto/sheet.rs:83` | HTTP JSON; ts-rs `bindings/ts/arda/SkillBonus.ts` | A proficient skill. |
| AttackKind | `crates/arda-server/src/npc_dto/sheet.rs:97` | HTTP JSON; ts-rs `bindings/ts/arda/AttackKind.ts` | Melee or ranged. |
| Attack | `crates/arda-server/src/npc_dto/sheet.rs:107` | HTTP JSON; ts-rs `bindings/ts/arda/Attack.ts` | One attack line. |
| Item | `crates/arda-server/src/npc_dto/sheet.rs:129` | HTTP JSON; ts-rs `bindings/ts/arda/Item.ts` | An item and how many. |
| Coins | `crates/arda-server/src/npc_dto/sheet.rs:139` | HTTP JSON; ts-rs `bindings/ts/arda/Coins.ts` | Coins carried. |
| SpellRef | `crates/arda-server/src/npc_dto/sheet.rs:151` | HTTP JSON; ts-rs `bindings/ts/arda/SpellRef.ts` | A spell known or prepared. |
| Spellcasting | `crates/arda-server/src/npc_dto/sheet.rs:161` | HTTP JSON; ts-rs `bindings/ts/arda/Spellcasting.ts` | Spellcasting block. |
| Sheet | `crates/arda-server/src/npc_dto/sheet.rs:179` | HTTP JSON; ts-rs `bindings/ts/arda/Sheet.ts` | A full SRD 5.1 sheet. |
| NpcEntry | `crates/arda-server/src/npcs/dto.rs:17` | HTTP JSON; ts-rs `bindings/ts/arda/NpcEntry.ts` | One person of a query page. |
| NpcPage | `crates/arda-server/src/npcs/dto.rs:32` | HTTP JSON; ts-rs `bindings/ts/arda/NpcPage.ts` | A page of people (TS `NpcPage`). |
| SettlementList | `crates/arda-server/src/schema/bodies.rs:11` | HTTP JSON (schemars only) | `GET /v1/settlements[?tier=&realm=]`. |
| SettlementDetail | `crates/arda-server/src/schema/bodies.rs:20` | HTTP JSON (schemars only) | `GET /v1/settlements/{id}`. |
| SettlementNpcs | `crates/arda-server/src/schema/bodies.rs:29` | HTTP JSON (schemars only) | `GET /v1/settlements/{id}/npcs`: the stored notables. |
| RealmRuler | `crates/arda-server/src/schema/bodies.rs` | HTTP JSON | Name and title of a realm's ruler. |
| RealmSummary | `crates/arda-server/src/schema/bodies.rs` | HTTP JSON | One entry of `GET /v1/realms`. |
| RealmList | `crates/arda-server/src/schema/bodies.rs` | HTTP JSON (schemars only) | `GET /v1/realms`. |
| RealmDetail | `crates/arda-server/src/schema/bodies.rs` | HTTP JSON (schemars only) | `GET /v1/realms/{id}`. |
| MappingFile | `crates/arda-server/src/sheet_map/spec.rs:16` | `--sheet-mapping` JSON file | A whole mapping file. |
| Base | `crates/arda-server/src/sheet_map/spec.rs:43` | `--sheet-mapping` JSON file | The starting output. |
| Rule | `crates/arda-server/src/sheet_map/spec.rs:54` | `--sheet-mapping` JSON file | One rule: write one output location. |
| Unknown | `crates/arda-server/src/sheet_map/spec.rs:94` | `--sheet-mapping` JSON file | What a `table` or `keys` conversion does with a value the table lacks. |
| Case | `crates/arda-server/src/sheet_map/spec.rs:107` | `--sheet-mapping` JSON file | Letter case and word-separator conversions. |
| Op | `crates/arda-server/src/sheet_map/spec.rs:123` | `--sheet-mapping` JSON file | A value conversion. |
| Block (arda-server) | `crates/arda-server/src/tactical/block.rs:20` | in-memory server cache | One world-derived tactical block or window. |
| TacticalLayouts | `crates/arda-server/src/tactical/dto.rs:14` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalLayouts.ts` | `GET /v1/tactical/layouts`: the built-in test layouts. |
| TacticalLayoutSummary | `crates/arda-server/src/tactical/dto.rs:29` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalLayoutSummary.ts` | One built-in layout. |
| TacticalTilesDto | `crates/arda-server/src/tactical/dto.rs:42` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalTilesDto.ts` | Tile pyramid geometry of one render. |
| TacticalBlockDto | `crates/arda-server/src/tactical/dto.rs:60` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalBlockDto.ts` | `GET /v1/tactical/cell/{gx}/{gy}` and `GET /v1/tactical/window`: one world-derived block or window (logic/16 §api-tactical). |
| TacticalLayoutDto | `crates/arda-server/src/tactical/dto.rs:97` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalLayoutDto.ts` | Mirror of `arda_tactical::TacticalLayout`: a W × H battle map. |
| SquareDto | `crates/arda-server/src/tactical/dto.rs:125` | HTTP JSON; ts-rs `bindings/ts/arda/SquareDto.ts` | One 5-ft square. |
| EdgeAxisDto | `crates/arda-server/src/tactical/dto.rs:150` | HTTP JSON; ts-rs `bindings/ts/arda/EdgeAxisDto.ts` | Which edge of a square a segment lies on. |
| WallRoleDto | `crates/arda-server/src/tactical/dto.rs:160` | HTTP JSON; ts-rs `bindings/ts/arda/WallRoleDto.ts` | Wall-kit piece roles; layouts use the edge roles `run`, `door`, `window`, `gate`. |
| AssetClassDto | `crates/arda-server/src/tactical/dto.rs:184` | HTTP JSON; ts-rs `bindings/ts/arda/AssetClassDto.ts` | Catalogue asset classes. |
| WallSegmentDto | `crates/arda-server/src/tactical/dto.rs:200` | HTTP JSON; ts-rs `bindings/ts/arda/WallSegmentDto.ts` | A wall-kit edge feature. |
| AssetRefDto | `crates/arda-server/src/tactical/dto.rs:220` | HTTP JSON; ts-rs `bindings/ts/arda/AssetRefDto.ts` | An asset chosen by id or by a deterministic tag query. |
| PlacementDto | `crates/arda-server/src/tactical/dto.rs:236` | HTTP JSON; ts-rs `bindings/ts/arda/PlacementDto.ts` | A prop or vegetation placement. |
| LightSourceDto | `crates/arda-server/src/tactical/dto.rs:254` | HTTP JSON; ts-rs `bindings/ts/arda/LightSourceDto.ts` | A light source not attached to an asset. |
| TacticalLibraryDto | `crates/arda-server/src/tactical/library_dto.rs:12` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalLibraryDto.ts` | The loaded asset library, summarised. |
| GroundKeyDto | `crates/arda-server/src/tactical/library_dto.rs:29` | HTTP JSON; ts-rs `bindings/ts/arda/GroundKeyDto.ts` | One ground key and how many texture variants back it. |
| WallKitDto | `crates/arda-server/src/tactical/library_dto.rs:40` | HTTP JSON; ts-rs `bindings/ts/arda/WallKitDto.ts` | One wall kit and the roles it has pieces for. |
| LibraryAssetDto | `crates/arda-server/src/tactical/library_dto.rs:49` | HTTP JSON; ts-rs `bindings/ts/arda/LibraryAssetDto.ts` | One placeable asset. |
| PrefetchRequest | `crates/arda-server/src/tactical/prefetch.rs:44` | HTTP JSON; ts-rs `bindings/ts/arda/PrefetchRequest.ts` | Body of `POST /v1/tactical/prefetch` (TS `PrefetchRequest`). |
| PrefetchStatus | `crates/arda-server/src/tactical/prefetch.rs:67` | HTTP JSON; ts-rs `bindings/ts/arda/PrefetchStatus.ts` | What happened to one neighbour. |
| PrefetchCell | `crates/arda-server/src/tactical/prefetch.rs:78` | HTTP JSON; ts-rs `bindings/ts/arda/PrefetchCell.ts` | One neighbour of a prefetch. |
| PrefetchAccepted | `crates/arda-server/src/tactical/prefetch.rs:91` | HTTP JSON; ts-rs `bindings/ts/arda/PrefetchAccepted.ts` | `202` body of `POST /v1/tactical/prefetch` (TS `PrefetchAccepted`). |
| CoverLevelDto | `crates/arda-server/src/tactical/rules_dto.rs:15` | HTTP JSON; ts-rs `bindings/ts/arda/CoverLevelDto.ts` | SRD 5.1 degrees of cover. |
| EdgeRoleDto | `crates/arda-server/src/tactical/rules_dto.rs:29` | HTTP JSON; ts-rs `bindings/ts/arda/EdgeRoleDto.ts` | Why a rules edge exists. |
| RulesCellDto | `crates/arda-server/src/tactical/rules_dto.rs:45` | HTTP JSON; ts-rs `bindings/ts/arda/RulesCellDto.ts` | One square's rules; an absent field means "no opinion". |
| EdgeRuleDto | `crates/arda-server/src/tactical/rules_dto.rs:84` | HTTP JSON; ts-rs `bindings/ts/arda/EdgeRuleDto.ts` | Movement and sight rules of one unit edge. |
| RulesSidecarDto | `crates/arda-server/src/tactical/rules_dto.rs:109` | HTTP JSON; ts-rs `bindings/ts/arda/RulesSidecarDto.ts` | The per-square rules sidecar, format 2. |
| SceneTime | `crates/arda-server/src/tactical/scene.rs:27` | HTTP JSON; ts-rs `bindings/ts/arda/SceneTime.ts` | Time of day a scene's tokens are placed for. |
| TacticalScene | `crates/arda-server/src/tactical/scene.rs:37` | HTTP JSON; ts-rs `bindings/ts/arda/TacticalScene.ts` | `GET /v1/tactical/cell/{gx}/{gy}/scene`: the scene of one block and the NPCs on it. |
| Runs | `crates/arda-server/src/tactical/scene_dto.rs:19` | HTTP JSON; ts-rs `bindings/ts/arda/Runs.ts` | A row-major per-square layer as runs `[[count, value], …]`, expanding to `width × height` values (`arda_scene::Grid`). |
| SqDto | `crates/arda-server/src/tactical/scene_dto.rs:23` | HTTP JSON; ts-rs `bindings/ts/arda/SqDto.ts` | A square `[x, y]` (column, row) in the scene's own squares. |
| MovementDto | `crates/arda-server/src/tactical/scene_dto.rs:28` | HTTP JSON; ts-rs `bindings/ts/arda/MovementDto.ts` | How a creature moves through a square (SRD 5.1 "Movement"). |
| ObscurementDto | `crates/arda-server/src/tactical/scene_dto.rs:44` | HTTP JSON; ts-rs `bindings/ts/arda/ObscurementDto.ts` | SRD 5.1 vision obscurement of a square. |
| WallKindDto | `crates/arda-server/src/tactical/scene_dto.rs:56` | HTTP JSON; ts-rs `bindings/ts/arda/WallKindDto.ts` | The type of a wall-edge feature. |
| SceneWallDto | `crates/arda-server/src/tactical/scene_dto.rs:72` | HTTP JSON; ts-rs `bindings/ts/arda/SceneWallDto.ts` | A wall polyline on grid vertices. |
| BlockerKindDto | `crates/arda-server/src/tactical/scene_dto.rs:100` | HTTP JSON; ts-rs `bindings/ts/arda/BlockerKindDto.ts` | Where a vision blocker comes from. |
| VisionBlockerDto | `crates/arda-server/src/tactical/scene_dto.rs:110` | HTTP JSON; ts-rs `bindings/ts/arda/VisionBlockerDto.ts` | A polygon that obscures vision. |
| SceneLightDto | `crates/arda-server/src/tactical/scene_dto.rs:124` | HTTP JSON; ts-rs `bindings/ts/arda/SceneLightDto.ts` | A light source with SRD bright and dim radii. |
| RegionKindDto | `crates/arda-server/src/tactical/scene_dto.rs:144` | HTTP JSON; ts-rs `bindings/ts/arda/RegionKindDto.ts` | What a region marks. |
| RegionDto | `crates/arda-server/src/tactical/scene_dto.rs:156` | HTTP JSON; ts-rs `bindings/ts/arda/RegionDto.ts` | Merged squares of one kind as closed rings on grid vertices (even-odd fill). |
| EdgeDto | `crates/arda-server/src/tactical/scene_dto.rs:165` | HTTP JSON; ts-rs `bindings/ts/arda/EdgeDto.ts` | A map edge. |
| EdgeExitDto | `crates/arda-server/src/tactical/scene_dto.rs:179` | HTTP JSON; ts-rs `bindings/ts/arda/EdgeExitDto.ts` | A run of enterable squares along one map edge. |
| EntranceDto | `crates/arda-server/src/tactical/scene_dto.rs:191` | HTTP JSON; ts-rs `bindings/ts/arda/EntranceDto.ts` | Squares beside a door or gate. |
| SpawnHintsDto | `crates/arda-server/src/tactical/scene_dto.rs:201` | HTTP JSON; ts-rs `bindings/ts/arda/SpawnHintsDto.ts` | Where tokens may be put. |
| TokenSlotDto | `crates/arda-server/src/tactical/scene_dto.rs:213` | HTTP JSON; ts-rs `bindings/ts/arda/TokenSlotDto.ts` | Where an NPC's token stands (scene token slot). |
| SceneDto | `crates/arda-server/src/tactical/scene_dto.rs:227` | HTTP JSON; ts-rs `bindings/ts/arda/SceneDto.ts` | `arda-scene` `Scene` format 1: everything the game needs to run a map. |
| TokenKind | `crates/arda-server/src/tactical/tokens.rs:16` | HTTP JSON; ts-rs `bindings/ts/arda/TokenKind.ts` | Why a token stands where it does. |
| Token | `crates/arda-server/src/tactical/tokens.rs:26` | HTTP JSON; ts-rs `bindings/ts/arda/Token.ts` | One token on the scene. |
| DungeonDto | `crates/arda-server/src/tactical/dungeon_dto.rs:19` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonDto.ts` | `GET /v1/tactical/dungeon`: mirror of `arda_dungeon::Dungeon`. |
| DungeonParamsDto | `crates/arda-server/src/tactical/dungeon_dto.rs:37` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonParamsDto.ts` | Mirror of `arda_dungeon::Params`, `seed` a decimal string. |
| DungeonKindDto | `crates/arda-server/src/tactical/dungeon_dto.rs:51` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonKindDto.ts` | Dungeon or cave. |
| RoomPurposeDto | `crates/arda-server/src/tactical/dungeon_dto.rs:61` | HTTP JSON; ts-rs `bindings/ts/arda/RoomPurposeDto.ts` | The purpose of a dungeon room. |
| DungeonRoomDto | `crates/arda-server/src/tactical/dungeon_dto.rs:85` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonRoomDto.ts` | A room: its purpose and its floor rectangle in squares. |
| DungeonDoorDto | `crates/arda-server/src/tactical/dungeon_dto.rs:103` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonDoorDto.ts` | A door on a square edge, in layout edge terms. |
| DungeonExitKindDto | `crates/arda-server/src/tactical/dungeon_dto.rs:119` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonExitKindDto.ts` | A way in or out of the level. |
| DungeonExitDto | `crates/arda-server/src/tactical/dungeon_dto.rs:131` | HTTP JSON; ts-rs `bindings/ts/arda/DungeonExitDto.ts` | An exit: its kind and the floor square a token arrives on. |
| SchemaIndex | `crates/arda-server/src/schema/mod.rs:153` | HTTP JSON; ts-rs `bindings/ts/arda/SchemaIndex.ts` | `GET /v1/schema`: the catalogue of published schemas (`index.json`). |
| SchemaIndexEntry | `crates/arda-server/src/schema/mod.rs:169` | HTTP JSON; ts-rs `bindings/ts/arda/SchemaIndexEntry.ts` | One schema in `SchemaIndex`: name, URL and routes. |

## Fields and types

Required `no` marks an `Option` field, a serde default, a field omitted when empty, or a field never serialized. Types are written as the code spells them, with `Vec<T>` shown as `T[]`. Enum sections hold one `value` row listing accepted wire values, or one row per variant for data-carrying enums. Notes come from the field doc comments; notes on the original five crates' records are carried over from the previous chapter where the field is unchanged.

### TerrainKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Sea=0, Land=1, Lake=2 |

### Cover

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Bare=0, Grass=1, Scrub=2, Forest=3, Marsh=4, Rock=5, Ice=6 |

### RoadClass

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: None=0, Track=1, Road=2, Highway=3 |

### Cell

| Field | Type | Required | Notes |
|---|---|---|---|
| height | `HeightMm` | yes | Elevation. |
| terrain | `TerrainKind` | yes | accepted: Sea, Land, Lake |
| cover | `Cover` | yes | accepted: Bare, Grass, Scrub, Forest, Marsh, Rock, Ice |
| slope_milli_deg | `u16` | yes | Slope in thousandths of a degree. |
| aspect_deg | `u16` | yes | Downslope compass bearing, 0-359; 0 on flat ground. |
| temperature | `TempCentiC` | yes | Produced from prepared reference and final signed physical-altitude lapse; marine altitude is zero |
| rainfall | `RainfallMm` | yes | Mean annual rainfall. |
| moisture | `u8` | yes | Recipe 4 estimates soil moisture from rainfall, continuous centi-degree temperature and broad absolute-coordinate soil variation; older producers retain default |
| forest_density | `u8` | yes | Recipe 4 estimates potential canopy from growth limits and regional patchiness; older producers retain default |
| drainage_area_cells | `u32` | yes | Upstream cells draining through here. |
| discharge | `DischargeMilli` | yes | Mean discharge. |
| watercourse_order | `u8` | yes | Strahler order; 0 when no watercourse. |
| watercourse_width_dm | `u32` | yes | Channel width in decimetres; 0 when no watercourse. |
| height_above_river_dm | `u16` | yes | u16::MAX where no downstream channel |
| wetness | `u8` | yes | Drainage/slope index; recipe 4 uses a logarithmic version with flat-ground headroom, while older producers retain the prior index |
| road | `RoadClass` | yes | accepted: None, Track, Road, Highway; Generation retains default; producer not built |
| built_by | `u16` | yes | Generation retains default; producer not built |

### SizeKm

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | East-west extent. |
| height | `u32` | yes | North-south extent. |

### LatitudeBand

| Field | Type | Required | Notes |
|---|---|---|---|
| south_deg | `i16` | yes | Southern edge, degrees north of the equator. |
| north_deg | `i16` | yes | Northern edge, degrees north of the equator. |

### GenerateConfig

| Field | Type | Required | Notes |
|---|---|---|---|
| size_km | `SizeKm` | yes | — |
| latitude_band | `LatitudeBand` | yes | — |
| mean_density_per_km2 | `u16` | yes | — |

### ClimateRegime

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Temperate=0, Mediterranean=1, Boreal=2, Tropical=3 |

### ContinentCell

| Field | Type | Required | Notes |
|---|---|---|---|
| height | `HeightMm` | yes | Terrain elevation. |
| temperature | `TempCentiC` | yes | Mean annual temperature. |
| rainfall | `RainfallMm` | yes | Mean annual rainfall. |
| regime | `ClimateRegime` | yes | accepted: Temperate, Mediterranean, Boreal, Tropical |
| downstream | `Option<u8>` | no | accepted: None or neighbor index 0–7; encoded None = 255 |
| catchment_km2 | `u32` | yes | Drainage area above this cell; one cell is one km². |
| discharge | `DischargeMilli` | yes | Accumulated discharge (thousandths of a m³/s, i.e. |

### ContinentOverview

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | Grid width in 1 km cells. |
| height | `i32` | yes | Grid height in 1 km cells. |
| cells | `ContinentCell[]` | yes | `width × height` cells, row-major. |

### ContinentRiver

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | 1-based; 0 reserved for absent feeds on wire |
| catchment_km2 | `u32` | yes | Catchment at the mouth, km². |
| discharge | `DischargeMilli` | yes | Discharge at the mouth. |
| feeds | `Option<u16>` | no | River this one joins; `None` when it reaches the sea. |
| course | `KmCoord[]` | yes | Course cells, source to mouth. |

### ContinentObjects

| Field | Type | Required | Notes |
|---|---|---|---|
| rivers | `ContinentRiver[]` | yes | Continent-scale rivers. |

### AreaCoord

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `i32` | yes | Tile column. |
| y | `i32` | yes | Tile row. |

### CellCoord

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u16` | yes | — |
| y | `u16` | yes | — |

### SquareCoord

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u8` | yes | — |
| y | `u8` | yes | — |

### KmCoord

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u16` | yes | Column, kilometres from the west edge. |
| y | `u16` | yes | Row, kilometres from the north edge. |

### GlobalCell

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Column from the modeled domain's west edge. |
| y | `u32` | yes | Row from the modeled domain's north edge. |

### HeightMm

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `i32` | yes | — |

### TempCentiC

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `i16` | yes | — |

### RainfallMm

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u16` | yes | — |

### DischargeMilli

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | Whole L/s, equivalent to thousandths of a cubic metre per second |

### Block

| Field | Type | Required | Notes |
|---|---|---|---|
| tiles | `TileId[]` | yes | — |
| relaxed | `bool` | yes | — |

### BlockArchive

| Field | Type | Required | Notes |
|---|---|---|---|
| blocks | `BTreeMap<(u16, u16), Block>` | yes | — |

### AreaCells (arda-core)

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `Cell[]` | yes | — |

### TableSpan

| Field | Type | Required | Notes |
|---|---|---|---|
| offset | `u64` | yes | Record index, not a byte offset; zero for an empty span |
| count | `u64` | yes | Checked against the linked table count |

### BasinNodeRow

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BasinId` | yes | Canonical physical basin identity. |
| parent | `Option<BasinId>` | no | Canonical parent identity, absent only at the physical root. |
| anchor | `GlobalCell` | yes | Absolute physical cell anchoring this basin. |
| floor | `HeightMm` | yes | Lowest physical bed elevation in millimetres. |
| children | `TableSpan` | yes | Span into hydrology/children.bin, not an inline vector |
| spill | `Option<SpillConnection>` | no | Physical neighboring spill witness, when this node has an outlet. |

### TableKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Metadata=1, Basins=2, Children=3, Lakes=4, Reaches=5, Crossings=6, Channels=11, Catchments=12 |

### TableHeader

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `TableKind` | yes | accepted: Metadata=1, Basins=2, Children=3, Lakes=4, Reaches=5, Crossings=6, Channels=11, Catchments=12 |
| record_bytes | `u32` | yes | Exact encoded byte width of every record. |
| count | `u64` | yes | Number of records, checked before reading or allocation. |
| payload_bytes | `u64` | yes | Checked record count multiplied by record width. |

### ValidationStats

| Field | Type | Required | Notes |
|---|---|---|---|
| land_fraction_permille | `u16` | yes | Land cells per thousand continent cells. |
| area_count | `u32` | yes | Area tiles written. |
| settlement_count | `u32` | yes | Settlements placed across the continent. |
| named_river_count | `u32` | yes | Named rivers reaching the sea. |
| river_count | `u32` | no | serde default 0 for older same-major manifests; serde default |

### FineTerrainDescriptor

| Field | Type | Required | Notes |
|---|---|---|---|
| recipe_version | `u16` | yes | Version of the source recipe that generated the layer. |
| attempt | `u8` | yes | Deterministic continent/source validation attempt. |

### Manifest

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | World-format major (`logic/05` version gate). |
| arda_version | `String` | yes | The arda build that wrote this world. |
| seed | `u64` | yes | The sole source of nondeterminism (`mockup/01` `--seed`). |
| config | `GenerateConfig` | yes | Config echo — enough to regenerate. |
| areas_wide | `i32` | yes | Area tiles across. |
| areas_high | `i32` | yes | Area tiles down. |
| stats | `ValidationStats` | yes | Validation statistics. |
| fine_terrain | `Option<FineTerrainDescriptor>` | no | Present only for worlds that published a canonical fine terrain layer.; serde default; omitted when empty/none |

### ShoreClass

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: None=0, SandBeach=1, ShingleBeach=2, Cliff=3, RockyShore=4, Marsh=5, TidalFlat=6, Estuary=7 |

### IslandCause

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Continental=0, Barrier=1, Volcanic=2, Delta=3 |

### LandformKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Basin=0, Plateau=1, GlacialTrough=2 |

### LandformCause

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Rift=0, Foreland=1, Arc=2, Orogenic=3, RiftShoulder=4, Glacial=5, Unexplained=6 |

### Landform

| Field | Type | Required | Notes |
|---|---|---|---|
| x_um | `i64` | yes | Centroid, micrometres from the terrain origin. |
| y_um | `i64` | yes | Centroid, micrometres from the terrain origin. |
| area_km2 | `u32` | yes | Area, km². |
| relief_m | `i16` | yes | Basin depth below spill or plateau mean height, metres. |
| kind | `LandformKind` | yes | What it is. |
| cause | `LandformCause` | yes | Why it exists. |

### Island

| Field | Type | Required | Notes |
|---|---|---|---|
| x_um | `i64` | yes | Centroid, micrometres from the terrain origin. |
| y_um | `i64` | yes | Centroid, micrometres from the terrain origin. |
| cells | `u32` | yes | Land cells (100 m). |
| top_m | `i16` | yes | Highest point, metres. |
| cause | `IslandCause` | yes | Cause. |

### ShoreLayer

| Field | Type | Required | Notes |
|---|---|---|---|
| spacing_um | `u32` | yes | Grid spacing, micrometres (100 m for the area cell grid). |
| width | `u32` | yes | Cells per row. |
| height | `u32` | yes | Rows. |
| classes | `ShoreClass[]` | yes | Row-major classes, `width * height` long. |
| islands | `Island[]` | yes | Islands, in centroid row-major order. |
| landforms | `Landform[]` | yes | Audited basins, plateaus and glacial troughs. |

### TerminalId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | Type-specific namespace; physical terminal/leaf anchors use packed (y,x) coordinates |

### BasinId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | Type-specific namespace; physical terminal/leaf anchors use packed (y,x) coordinates |

### ReachId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | Directed start/first D8 step; bit 63 distinguishes physical point records |

### JunctionId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | (y << 32) \| x; separate namespace from reaches; no junction table |

### CatchmentId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | Type-specific namespace; physical terminal/leaf anchors use packed (y,x) coordinates |

### Litres

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u128` | yes | Exact nonnegative whole volume; 1 mm over a 100 m cell = 10,000 L |

### HydrologyDomain

| Field | Type | Required | Notes |
|---|---|---|---|
| width_cells | `u32` | yes | Modeled width in 100 m cells, including hydrology-only fringe. |
| height_cells | `u32` | yes | Modeled height in 100 m cells, including hydrology-only fringe. |
| exported_areas_wide | `u32` | yes | Number of complete 512-cell area columns with final output files. |
| exported_areas_high | `u32` | yes | Number of complete 512-cell area rows with final output files. |

### CrossingId

| Field | Type | Required | Notes |
|---|---|---|---|
| low | `GlobalCell` | yes | Lower endpoint under GlobalCell ordering (x,y), unlike packed terminal ordering (y,x) |
| high | `GlobalCell` | yes | Higher endpoint under GlobalCell ordering (x,y) |

### ReceivingAccount

| Field | Type | Required | Notes |
|---|---|---|---|
| Reach | `(ReachId)` | no | variant (externally tagged); The next modeled river reach. |
| Lake | `(BasinId)` | no | variant (externally tagged); The receiving physical basin or connected lake component. |
| Junction | `(JunctionId)` | no | variant (externally tagged); An actual dry junction; outgoing reaches may be absent in potential topology. |
| Sea | `unit` | no | variant (externally tagged); Water entering the connected modeled ocean. |
| DomainExport | `unit` | no | variant (externally tagged); Water leaving the actual outer modeled boundary. |

### SpillConnection

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `GlobalCell` | yes | Actual modeled cell on the source side of the physical spill. |
| to | `Option<GlobalCell>` | no | None only for DomainExport at an actual modeled outer boundary. |
| sill | `HeightMm` | yes | Physical spill elevation in millimetres. |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### BasinNode

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BasinId` | yes | Stable identity of this coalesced node. |
| parent | `Option<BasinId>` | no | Containing basin, absent at a hierarchy root. |
| anchor | `GlobalCell` | yes | Canonical physical minimum or canonical descendant anchor. |
| floor | `HeightMm` | yes | Lowest physical bed elevation represented by this node. |
| children | `BasinId[]` | yes | Canonical immediate child identities in this bounded logical view. |
| spill | `Option<SpillConnection>` | no | Potential outward spill, independent of whether water currently reaches it. |

### AnnualWaterBalance

| Field | Type | Required | Notes |
|---|---|---|---|
| land_precipitation | `Litres` | yes | Precipitation assigned to cells remaining dry land. |
| land_loss | `Litres` | yes | Effective dry-land loss, bounded by land precipitation. |
| lake_precipitation | `Litres` | yes | Direct precipitation replacing runoff on strictly submerged cells. |
| lake_evaporation | `Litres` | yes | Open-water evaporation on strictly submerged cells. |
| marginal_evaporation | `Litres` | yes | Separately paid unsupported-band adjustment; that marginal band stays geometrically dry |
| sea_outflow | `Litres` | yes | Annual discharge entering the connected modeled ocean. |
| domain_outflow | `Litres` | yes | Annual discharge leaving the modeled outer boundary. |

### GlobalLake

| Field | Type | Required | Notes |
|---|---|---|---|
| basin | `BasinId` | yes | Stable identity shared by all fragments of this connected component. |
| surface | `HeightMm` | yes | Positive depth means physical bed < surface |
| deepest_bed | `HeightMm` | yes | Lowest submerged physical bed elevation. |
| submerged_cells | `u32` | yes | Number of cells with strictly positive depth. |
| outlet | `Option<SpillConnection>` | no | Potential witness retained when nonspilling; canonical active witness when annual_outflow > 0 |
| annual_outflow | `Litres` | yes | Whole litres/year; can be positive when mean_outflow rounds to zero |
| mean_outflow | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |

### AnnualCatchment

| Field | Type | Required | Notes |
|---|---|---|---|
| catchment | `CatchmentId` | yes | Stable contributing-owner identity used by reaches and crossings. |
| terminal | `GlobalCell` | yes | Actual physical routing terminal of this catchment. |
| contributing_cells | `u32` | yes | Number of distinct physical source cells assigned to this owner. |
| basin | `Option<BasinId>` | no | Immutable physical depression leaf, absent for direct open destinations. |
| representative_lake | `Option<BasinId>` | no | Actual positive-depth lake component containing the terminal, if wet. |
| potential_spill | `Option<SpillConnection>` | no | Potential physical spill of a depression, absent for direct open destinations. |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### GlobalReach

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `ReachId` | yes | Canonical identity shared by all local fragments of this reach. |
| from | `GlobalCell` | yes | First actual modeled cell of the directed reach. |
| to | `GlobalCell` | yes | Last actual modeled cell; equal to `from` exactly for a point reach. |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |
| catchment | `CatchmentId` | yes | Canonical drainage ownership associated with this reach. |
| drainage_cells | `u32` | yes | Number of distinct upstream contributing cells. |
| annual_volume | `Litres` | yes | Whole litres transported per representative year. |
| mean_discharge | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |

### SharedCrossing

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `CrossingId` | yes | Canonical undirected endpoint identity of this shared adjacency. |
| from | `GlobalCell` | yes | Actual upstream cell of this directed crossing. |
| to | `GlobalCell` | yes | Actual downstream D8 neighbor across the area boundary. |
| reach | `ReachId` | yes | Canonical reach containing this crossing. |
| catchment | `CatchmentId` | yes | Canonical contributing ownership at the crossing. |
| drainage_cells | `u32` | yes | Number of distinct upstream contributing cells at the crossing. |
| annual_volume | `Litres` | yes | Whole litres transported across this adjacency per representative year. |
| mean_discharge | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### ChannelEdge

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `GlobalCell` | yes | Upstream global centerline endpoint. |
| to | `GlobalCell` | yes | Downstream global centerline endpoint. |
| from_width_dm | `u32` | yes | Full channel width at the upstream endpoint in decimetres. |
| to_width_dm | `u32` | yes | Full channel width at the downstream endpoint in decimetres. |
| discharge | `DischargeMilli` | yes | Authoritative annual mean discharge used for physical width. |

### AreaHydrologyReferences

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | Exported area addressed by this lookup row. |
| lakes | `BasinId[]` | yes | Canonical identities of relevant annual lake components. |
| reaches | `ReachId[]` | yes | Canonical identities of relevant reaches, including rendering context. |
| crossings | `CrossingId[]` | yes | Canonical identities of crossings relevant to the area. |

### AreaHydrologyContext

| Field | Type | Required | Notes |
|---|---|---|---|
| model_revision | `u32` | yes | 2; representative annual model |
| reaches | `GlobalReach[]` | yes | Relevant global reach authorities. |
| lakes | `GlobalLake[]` | yes | Relevant connected annual lake authorities. |
| catchments | `AnnualCatchment[]` | yes | Relevant immutable catchment identities and destinations. |
| crossings | `SharedCrossing[]` | yes | Canonical cross-area transfers. |

### HydrologyMetadata

| Field | Type | Required | Notes |
|---|---|---|---|
| model_revision | `u32` | yes | 2; representative annual model |
| domain | `HydrologyDomain` | yes | Exact modeled rectangle and exported area dimensions. |
| basin_count | `u64` | yes | Number of coalesced physical basin nodes. |
| lake_count | `u64` | yes | Number of actual connected positive-depth components. |
| reach_count | `u64` | yes | Number of canonical directed reaches. |
| crossing_count | `u64` | yes | Number of canonical directed area crossings. |
| catchment_count | `u64` | yes | Number of immutable contributing-owner summaries. |
| budget | `AnnualWaterBalance` | yes | Whole-domain annual ledger after internal transfers cancel. |

### Terminus

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Junction=0, Sea=1, Lake=2, OffTile=3, Basin=4, Divergence=5 |

### RiverSegment

| Field | Type | Required | Notes |
|---|---|---|---|
| global_id | `ReachId` | yes | Stable identity of the containing global reach |
| id | `u32` | yes | 1-based; 0 reserved for absent feeds on wire |
| order | `u8` | yes | Strahler order. |
| width_dm | `u32` | yes | Channel width in decimetres, from `4 * sqrt(discharge)`. |
| discharge | `DischargeMilli` | yes | Mean discharge. |
| feeds | `Option<u32>` | no | Only Junction has one local downstream segment; divergence uses global junction identity |
| ends | `Terminus` | yes | accepted: Junction, Sea, Lake, OffTile, Basin, Divergence |
| course | `CellCoord[]` | yes | Cells the channel runs through, upstream to downstream. |

### Lake

| Field | Type | Required | Notes |
|---|---|---|---|
| global_id | `BasinId` | yes | Stable identity shared across all local fragments of the connected lake |
| id | `u32` | yes | Tile-local identifier, 1-based. |
| surface | `HeightMm` | yes | Exact global representative annual surface, whole millimetres |
| depth_mm | `u32` | yes | Greatest local positive depth |
| outlet | `Option<CellCoord>` | no | Local source of supported annual outflow; absent if dry outflow or outlet belongs to another area |
| cells | `CellCoord[]` | yes | Cells covered by the lake. |

### AreaObjects

| Field | Type | Required | Notes |
|---|---|---|---|
| channel_edges | `ChannelEdge[]` | yes | Neighbor halo included when a physical envelope touches the area |
| global | `AreaHydrologyContext` | yes | Relevant global copies; no recursive neighbor reads |
| rivers | `RiverSegment[]` | yes | Watercourse reaches. |
| lakes | `Lake[]` | yes | Lakes. |

### PanKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: SaltCrust=0, Mudflat=1 |

### PanRun

| Field | Type | Required | Notes |
|---|---|---|---|
| y | `u16` | yes | Row. |
| x0 | `u16` | yes | First column. |
| len | `u16` | yes | Number of cells. |
| kind | `PanKind` | yes | Surface. |

### SeedKey

| Field | Type | Required | Notes |
|---|---|---|---|
| tier | `Tier` | yes | accepted: Continent, Area, Block |
| stage | `Stage` | yes | accepted: Plates, Tectonics, Upsample, Relief, Water, Blocks, Bundles, Erosion |
| x | `i32` | yes | — |
| y | `i32` | yes | — |
| attempt | `u8` | yes | — |

### TerrainPoint

| Field | Type | Required | Notes |
|---|---|---|---|
| x_um | `i64` | yes | Eastward coordinate. |
| y_um | `i64` | yes | Southward coordinate. |

### TerrainField

| Field | Type | Required | Notes |
|---|---|---|---|
| origin | `TerrainPoint` | yes | — |
| spacing_um | `u32` | yes | — |
| width | `u32` | yes | — |
| height | `u32` | yes | — |
| heights | `HeightMm[]` | yes | — |

### TileId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u16` | yes | — |

### TileGroup

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Water, Shore, Ground, Vegetation, Rock, Structure |

### TileDef

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `TileId` | yes | Stable identifier. |
| name | `&'static str` | yes | Stable snake_case name, used by the export legend (`logic/04`). |
| group | `TileGroup` | yes | accepted: Water, Shore, Ground, Vegetation, Rock, Structure |

### ChannelPattern

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Straight=0, Meandering=1, Braided=2 |

### LakeOrigin

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Unclassified=0, Tectonic=1, Glacial=2, Oxbow=3, Karst=4, AridTerminal=5 |

### SegmentForm

| Field | Type | Required | Notes |
|---|---|---|---|
| pattern | `ChannelPattern` | yes | Planform. |
| bankfull_width_dm | `u32` | yes | Single-thread bankfull width, decimetres (`4 m × Q^0.5`). |
| bankfull_depth_cm | `u32` | yes | Bankfull depth, centimetres (`0.3 m × Q^0.4`). |
| belt_width_dm | `u32` | yes | Width of the active belt: the braidplain for braided reaches, the bankfull channel otherwise; decimetres. |
| sinuosity_permille | `u16` | yes | Course length over straight-line length, thousandths. |
| slope_ppm | `u32` | yes | Channel bed slope along the course, parts per million. |

### LakeForm

| Field | Type | Required | Notes |
|---|---|---|---|
| origin | `LakeOrigin` | yes | Geological origin. |
| terminal | `bool` | yes | No surface outflow: a terminal, evaporation-balanced lake. |
| saline | `bool` | yes | Saline: solutes concentrate because the lake loses water only by evaporation. |

### DeltaForm

| Field | Type | Required | Notes |
|---|---|---|---|
| apex | `CellCoord` | yes | Apex cell. |
| radius_m | `u32` | yes | Fan radius, metres. |
| catchment_km2 | `u32` | yes | Catchment feeding it, km². |

### Doline

| Field | Type | Required | Notes |
|---|---|---|---|
| at | `CellCoord` | yes | Cell containing its centre. |
| radius_dm | `u16` | yes | Rim radius, decimetres. |
| depth_dm | `u16` | yes | Depth below the rim, decimetres. |

### AreaWater

| Field | Type | Required | Notes |
|---|---|---|---|
| segments | `SegmentForm[]` | yes | One form per river segment, same order as the area's rivers. |
| lakes | `LakeForm[]` | yes | One form per lake fragment, same order as the area's lakes. |
| deltas | `DeltaForm[]` | yes | Deltas with their apex in this area. |
| dolines | `Doline[]` | yes | Karst dolines in this area. |
| pans | `PanRun[]` | yes | Dry arid-basin floors (recipe 7): row runs sorted by `(y, x0)`. |

### Hand

| Field | Type | Required | Notes |
|---|---|---|---|
| above_mm | `u32[]` | yes | Millimetres above the channel this cell drains to; `u32::MAX` where the path leaves the tile before meeting one. |
| carried_milli | `u64[]` | yes | Discharge of that channel in thousandth-cumecs, 0 where none was reached. |

### Basin

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `CellCoord[]` | yes | Submerged cells, ascending row-major. |
| surface_mm | `i32` | yes | Spill elevation, millimetres — the water surface. |
| depth_mm | `u32` | yes | Surface minus the lowest submerged cell, millimetres. |

### Filled

| Field | Type | Required | Notes |
|---|---|---|---|
| surface | `i32[]` | yes | — |
| basins | `Basin[]` | yes | Basins found, ordered by their first cell. |

### SharedTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `usize` | yes | Modeled union width in 100 m cells |
| height | `usize` | yes | Modeled union height in 100 m cells |
| heights | `i32[]` | yes | Row-major evolved physical millimetres; includes publication areas and modeled fringe |

### ReliefGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| heights | `i32[]` | yes | — |

### SolvedCell

| Field | Type | Required | Notes |
|---|---|---|---|
| marine | `bool` | yes | Actual rim-connected marine membership. |
| lake | `Option<BasinId>` | no | Connected representative lake, present only on positive-depth wet cells. |
| drainage_cells | `u32` | yes | Distinct upstream contributing cells on the final dry flow graph. |
| discharge | `DischargeMilli` | yes | Representative mean annual discharge on a dry cell. |
| order | `u8` | yes | Zero below 40 L/s channel initiation |
| hand_mm | `u32` | yes | u32::MAX when no downstream channel exists |
| hand_discharge | `DischargeMilli` | yes | Mean discharge of that downstream channel; zero on the channel itself. |

### WaterGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| drainage | `u32[]` | yes | — |
| discharge | `u64[]` | yes | — |
| order | `u8[]` | yes | — |
| downstream | `Option<u32>[]` | yes | — |
| outlets | `bool[]` | yes | — |

### BlockConstraints

| Field | Type | Required | Notes |
|---|---|---|---|
| allowed | `TileId[]` | yes | Tiles permitted in this block. |
| wet_fraction | `u8` | yes | How wet the surroundings are, 0-255; drives the water-to-land mix. |

### EnteringRiver

| Field | Type | Required | Notes |
|---|---|---|---|
| cell | `CellCoord` | yes | Boundary cell the river is seeded at, the point of lowest relief in its crossing window. |
| catchment_km2 | `u32` | yes | Catchment feeding this entry point, summed over every continent cell that crosses in at the same seed cell. |
| discharge | `DischargeMilli` | yes | Discharge feeding this entry point, summed the same way. |
| order | `u8` | yes | Strahler floor for this entry point. |

### TileBundle

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | Which tile this bundle feeds. |
| north | `i32[]` | yes | Unevolved absolute-coordinate relief samples; retained for the local diagnostic path |
| south | `i32[]` | yes | Unevolved samples at the next area's first row |
| east | `i32[]` | yes | Unevolved samples at the next area's first column |
| west | `i32[]` | yes | Unevolved absolute-coordinate relief samples |
| north_outside | `i32[]` | yes | 512 actual adjacent samples at local y=-1; legacy routing only |
| west_outside | `i32[]` | yes | 512 actual adjacent samples at local x=-1; legacy routing only |
| outside_corners | `[i32; 4]` | yes | Adjacent NW, NE, SW, SE at (-1,-1), (512,-1), (-1,512), (512,512) |
| mean_height_mm | `i32` | yes | Mean elevation across the sampled edges, for cheap summaries. |
| entering | `EnteringRiver[]` | yes | Watercourses crossing into the tile from the continent drainage tree (`logic/01` step 10). |
| rainfall_km | `u16[]` | yes | Rainfall patch: the tile's 52 km plus the +1 bilinear row/column, row-major (`PATCH_KM` per side). |
| regime_km | `ClimateRegime[]` | yes | Climate regime patch, same footprint as `TileBundle::rainfall_km`. |
| filled_km | `i32[]` | yes | Routing-surface patch, same footprint as `TileBundle::rainfall_km`. |
| basin_km | `i32[]` | yes | Continent-tier lake-identity patch (feature 02 §Q1 deferred this; added to close open-items #12 exactly — see `continent::hydrology::ContinentHydrology::basin_surface`), same footprint as… |
| wind | `CompassOctant` | yes | accepted: West |
| west_moisture | `u16[]` | yes | Final advection moisture store along the tile's western patch edge, `PATCH_KM` long. |

### ContinentClimate

| Field | Type | Required | Notes |
|---|---|---|---|
| temperature | `i16[]` | yes | Mean annual temperature, centi-°C. |
| rainfall | `u16[]` | yes | Mean annual rainfall, mm. |
| regime | `ClimateRegime[]` | yes | Climate regime. |
| moisture | `u16[]` | yes | Final advection moisture store, 0..=M_SAT (feature 03 §Q6). |
| ocean | `bool[]` | yes | D8-connected nonpositive coarse cells touching the domain rim |
| ocean_distance_km | `u16[]` | yes | Cardinal distance to connected ocean, capped at 300 km |

### ContinentHydrology

| Field | Type | Required | Notes |
|---|---|---|---|
| filled | `i32[]` | yes | Priority-flood routing surface (basin spill levels for 03). |
| basin_surface | `i32[]` | yes | NO_BASIN = i32::MIN outside filled depression; 8-connected components |
| downstream | `Option<u32>[]` | yes | Row-major downstream index per cell. |
| downstream_dir | `u8[]` | yes | NO_DOWNSTREAM = 255 |
| catchment_km2 | `u32[]` | yes | Drainage area, km²; zero on sea cells (§D9 hygiene). |
| discharge_l_s | `u64[]` | yes | Discharge, L/s; zero on sea cells. |

### ContinentGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | — |
| height | `i32` | yes | — |
| height_mm | `i32[]` | yes | — |

### Continent

| Field | Type | Required | Notes |
|---|---|---|---|
| grid | `ContinentGrid` | yes | The accepted 1 km continent grid. |
| climate | `climate::ContinentClimate` | yes | Per-cell climate fields for `Continent::grid`. |
| hydrology | `hydrology::ContinentHydrology` | yes | The drainage tree and per-cell loads routed over `Continent::grid`. |

### SimExtent

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | Domain width in 4 km cells. |
| height | `i32` | yes | Domain height in 4 km cells. |

### Plate

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u8` | yes | Plate identifier, used as the Voronoi label. |
| centre_x | `i32` | yes | Voronoi site column on the 4 km grid. |
| centre_y | `i32` | yes | Voronoi site row on the 4 km grid. |
| crust | `CrustType` | yes | accepted: Continental, Oceanic |
| drift_x | `i32` | yes | Drift per step, columns. |
| drift_y | `i32` | yes | Drift per step, rows. |

### MarineBoundary

| Field | Type | Required | Notes |
|---|---|---|---|
| north | `bool` | yes | The tile touches the modeled northern boundary. |
| east | `bool` | yes | The tile touches the modeled eastern boundary. |
| south | `bool` | yes | The tile touches the modeled southern boundary. |
| west | `bool` | yes | The tile touches the modeled western boundary. |

### PreparedExtent

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u16` | yes | Number of valid tile columns, from one through 512. |
| height | `u16` | yes | Number of valid tile rows, from one through 512. |
| boundary | `MarineBoundary` | yes | Which valid tile sides are actual domain boundaries. |

### PreparedTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | Canonical tile coordinate within the modeled domain. |
| valid | `PreparedExtent` | yes | Cropped physical extent and actual domain boundaries. |
| heights | `HeightMm[]` | yes | Full 512² container; valid cells copy SharedTerrain exactly, outside-valid zero padding is never serialized |
| annual_rain | `RainfallMm[]` | yes | Full 512 by 512 annual rainfall sampled before water classification. |
| temperature_base_centi | `i32[]` | yes | Unclamped reference with coarse lapse removed; not a final persisted temperature |

### HydrologyLimits

| Field | Type | Required | Notes |
|---|---|---|---|
| max_closed_leaves | `u64` | yes | Maximum actual closed physical terminal count; larger inputs fail without truncation. |
| max_bands | `u64` | yes | Maximum grouped annual support bands, bounded by physical cells. |
| max_feature_records | `u64` | yes | Maximum saved reach/crossing/catchment records in the final index. |
| max_area_references | `u64` | yes | Maximum saved area-to-reach references including required endpoint closure and halo. |
| max_lake_boundary_edges | `u64` | yes | Maximum actual outward lake-boundary edges used by metrics, including small flows. |
| global_io_operations | `u64` | yes | Maximum counted filesystem operations across the complete declared transaction. |
| ram_bytes | `u64` | yes | Owned-payload admission ceiling, including dense terrain and water-stage reservations; default 16 GiB |
| spatial_scratch_bytes | `u64` | yes | Maximum simultaneous spatial scratch bytes. |
| combined_scratch_bytes | `u64` | yes | Maximum simultaneous bytes across all live scratch stages. |
| global_event_operations | `u64` | yes | Maximum summed declared logical work across admitted generation stages. |
| global_io_bytes | `u128` | yes | Maximum summed declared prepared, solve and final-file payload I/O bytes. |

### AtlasTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| palette | `[u8; 3][]` | yes | 514² derived RGB terrain palette, including interpolation ghosts; older worlds tint land from saved wetness, recipe-4 worlds from climate moisture and canopy, before rock/snow blending |
| light | `u16[]` | yes | 514² Q12 relief factors, applied only to land |
| classes | `TerrainKind[]` | yes | 514² ownership classes for filtered sampling |
| heights | `i32[]` | yes | 514² saved millimetre heights, including independently clamped outer ghosts; supports integer bilinear land/sea reconstruction |
| wetness | `u8[]` | yes | — |
| moisture | `u8[]` | yes | — |
| forest_density | `u8[]` | yes | — |
| temperature | `i16[]` | yes | — |
| lake_surface | `i32[]` | yes | Saved lake water surface per sample (`i32::MIN` off lakes), for recipe-5 shoreline contouring. |
| channel_width | `u32[]` | yes | Saved channel width per sample, for recipe-5 meanders. |
| shore | `u8[]` | yes | Stored shore class per sample (0 = none or no shore layer). |
| salt | `u8[]` | yes | Arid water per sample (recipe 7; empty before): `SALT_NONE`, `SALT_SALINE_LAKE`, `SALT_CRUST` or `SALT_MUDFLAT`. |
| has_lake_depths | `bool` | yes | Target has supplied lake memberships; enables Atlas depth sampling while preserving older caller fallback |
| data_meanders | `bool` | yes | The saved courses already meander (stored water forms): the synthetic floodplain meander is not added on top. |
| fine | `Option<FineAtlas>` | no | — |

### AtlasNeighbor

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest |

### AtlasHalo

| Field | Type | Required | Notes |
|---|---|---|---|
| context | `Option<SourceSample>[]` | yes | Fixed 516² saved height/class/optional-lake-depth/wetness samples for target plus two-cell cardinal strips and 2×2 diagonal corners |
| states | `[NeighborState; 8]` | yes | Every direction resolved once as Copied or WorldEdge; incomplete or contradictory context is refused |

### AreaImageScale

| Field | Type | Required | Notes |
|---|---|---|---|
| Preview | `unit` | no | variant (externally tagged); 512 square pixels with a faint mark for subpixel streams. |
| Detail | `unit` | no | variant (externally tagged); 4096 square pixels, showing physical channel coverage only. |
| Custom | `(ImageQuality)` | no | variant (externally tagged); An exact image side, showing physical channel coverage only. |

### AreaRaster

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `&'a AreaCells` | yes | Immutable saved 100 m cell grid |
| terrain | `Option<&'a AtlasTerrain>` | no | Atlas-derived palette/light/class grid; absent on Classic |
| lake_colours | `Option<[u8; 3]>[]` | yes | Per-cell validated saved-lake colour, absent outside lakes |
| shapes | `Shape[]` | yes | Prepared saved channel strips, joins, terminals and preview marks |
| starts | `usize[]` | yes | Shape indexes ordered by first raster row |
| ends | `usize[]` | yes | Shape indexes ordered by last raster row |
| next_start | `usize` | yes | Scanline event cursor |
| next_end | `usize` | yes | Scanline event cursor |
| active | `BTreeSet<usize>` | yes | Shape indexes active on the current row |
| pixels | `usize[][]` | yes | Reused per-column candidate lists |
| pixel_capacity | `usize` | yes | Bounded aggregate candidate allocation |
| base_row | `u8[]` | yes | Reused terrain/lake RGB row |
| base_cell_y | `Option<usize>` | no | Saved-cell row currently represented by base_row |
| rgb | `u8[]` | yes | Reused final RGB row |
| last_y | `Option<usize>` | no | Enforces forward sequential row requests |
| side | `usize` | yes | Selected image edge |
| work | `WorkBudget` | yes | Finite polygon coverage budget scaled from output side |

### AccountOut

| Field | Type | Required | Notes |
|---|---|---|---|
| reach | `{global_reach_id: String}` | no | variant (internally tagged `kind`) |
| lake | `{global_basin_id: String}` | no | variant (internally tagged `kind`) |
| junction | `{global_junction_id: String}` | no | variant (internally tagged `kind`) |
| sea | `unit` | no | variant (internally tagged `kind`) |
| domain_export | `unit` | no | variant (internally tagged `kind`) |

### SpillOut

| Field | Type | Required | Notes |
|---|---|---|---|
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `Option<[u32; 2]>` | no | — |
| sill_mm | `i32` | yes | — |
| receiving | `AccountOut` | yes | — |

### LakeOut (hydrology_json)

| Field | Type | Required | Notes |
|---|---|---|---|
| global_basin_id | `String` | yes | Decimal string preserves exact global integer identity |
| surface_mm | `i32` | yes | — |
| deepest_bed_mm | `i32` | yes | — |
| submerged_cells | `u32` | yes | — |
| outlet | `Option<SpillOut>` | no | — |
| annual_outflow_litres | `String` | yes | Exact whole litres encoded as a decimal string |
| mean_outflow_milli_cumecs | `u64` | yes | — |

### CatchmentOut

| Field | Type | Required | Notes |
|---|---|---|---|
| catchment_id | `String` | yes | Decimal string preserves exact global integer identity |
| terminal_global_cell | `[u32; 2]` | yes | — |
| contributing_cells | `u32` | yes | — |
| physical_basin_id | `Option<String>` | no | Decimal string preserves exact global integer identity |
| representative_lake_id | `Option<String>` | no | Decimal string preserves exact global integer identity |
| potential_spill | `Option<SpillOut>` | no | — |
| receiving | `AccountOut` | yes | — |

### ReachOut

| Field | Type | Required | Notes |
|---|---|---|---|
| global_reach_id | `String` | yes | Decimal string preserves exact global integer identity |
| is_point | `bool` | yes | Physical terminal with equal endpoints; no invented directed step |
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `[u32; 2]` | yes | — |
| receiving | `AccountOut` | yes | — |
| catchment_id | `String` | yes | Decimal string preserves exact global integer identity |
| drainage_cells | `u32` | yes | — |
| annual_volume_litres | `String` | yes | Exact whole litres encoded as a decimal string |
| mean_discharge_milli_cumecs | `u64` | yes | — |

### CrossingOut

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `[[u32; 2]; 2]` | yes | — |
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `[u32; 2]` | yes | — |
| global_reach_id | `String` | yes | Decimal string preserves exact global integer identity |
| catchment_id | `String` | yes | Decimal string preserves exact global integer identity |
| drainage_cells | `u32` | yes | — |
| annual_volume_litres | `String` | yes | Exact whole litres encoded as a decimal string |
| mean_discharge_milli_cumecs | `u64` | yes | — |
| receiving | `AccountOut` | yes | — |

### HydrologyOut

| Field | Type | Required | Notes |
|---|---|---|---|
| model_revision | `u32` | yes | 2; representative annual model |
| representative | `&'static str` | yes | accepted: representative_annual_balance |
| flow_semantics | `&'static str` | yes | accepted: mean_annual_discharge |
| annual_seconds | `u32` | yes | 31,536,000 seconds per representative 365-day year |
| lake_membership | `&'static str` | yes | accepted: physical_bed_mm < surface_mm |
| lakes | `LakeOut[]` | yes | — |
| reaches | `ReachOut[]` | yes | — |
| catchments | `CatchmentOut[]` | yes | — |
| crossings | `CrossingOut[]` | yes | — |

### CellOut

| Field | Type | Required | Notes |
|---|---|---|---|
| height_mm | `i32` | yes | — |
| terrain | `&'static str` | yes | accepted: sea, land, lake |
| cover | `&'static str` | yes | accepted: bare, grass, scrub, forest, marsh, rock, ice |
| slope_milli_deg | `u16` | yes | — |
| aspect_deg | `u16` | yes | — |
| drainage_area_cells | `u32` | yes | — |
| discharge_milli_cumecs | `u64` | yes | — |
| watercourse_order | `u8` | yes | — |
| watercourse_width_dm | `u32` | yes | — |
| height_above_river_dm | `u16` | yes | — |
| wetness | `u8` | yes | — |

### RiverOut

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | — |
| global_reach_id | `String` | yes | Decimal string preserves exact global integer identity |
| order | `u8` | yes | — |
| width_dm | `u32` | yes | — |
| discharge_milli_cumecs | `u64` | yes | — |
| feeds | `Option<u32>` | no | — |
| ends | `&'static str` | yes | accepted: junction, sea, lake, off_tile, basin, divergence |
| course | `[u16; 2][]` | yes | — |

### LakeOut

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | — |
| global_basin_id | `String` | yes | Decimal string preserves exact global integer identity |
| surface_mm | `i32` | yes | — |
| depth_mm | `u32` | yes | — |
| outlet | `Option<[u16; 2]>` | no | — |
| cells | `[u16; 2][]` | yes | — |

### AreaOut

| Field | Type | Required | Notes |
|---|---|---|---|
| schema_version | `u32` | yes | 2; separate from stored-world format major |
| format_version | `u32` | yes | 4 for current generated worlds |
| seed | `u64` | yes | — |
| area_x | `i32` | yes | — |
| area_y | `i32` | yes | — |
| cells_per_side | `u16` | yes | — |
| cells | `CellOut[]` | yes | — |
| rivers | `RiverOut[]` | yes | — |
| lakes | `LakeOut[]` | yes | — |
| channel_edges | `ChannelEdgeOut[]` | yes | — |
| hydrology | `HydrologyOut` | yes | — |

### ChannelEdgeOut

| Field | Type | Required | Notes |
|---|---|---|---|
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `[u32; 2]` | yes | — |
| from_width_dm | `u32` | yes | — |
| to_width_dm | `u32` | yes | — |
| discharge_milli_cumecs | `u64` | yes | — |

### LegendEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | — |
| name | `&'static str` | yes | — |

### BlockOut

| Field | Type | Required | Notes |
|---|---|---|---|
| schema_version | `u32` | yes | 2; block keys and tactical semantics are retained |
| format_version | `u32` | yes | — |
| seed | `u64` | yes | — |
| squares_per_side | `u8` | yes | — |
| relaxed | `bool` | yes | — |
| legend | `LegendEntry[]` | yes | — |
| squares | `u16[]` | yes | — |

### OverviewRaster

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | Final image width |
| height | `u32` | yes | Final image height |
| areas_wide | `i32` | yes | Validated 1–78 |
| areas_high | `i32` | yes | Validated 1–78 |
| rgb | `u8[]` | yes | Three bytes per output pixel |
| features | `Feature[]` | yes | Private accepted variants: Sea, AtlasSea, Land, River(Light\|Mid\|Dark), Lake; AtlasSea blocks trunk widening |
| supplied | `bool[]` | yes | One duplicate-detection flag per area |
| channel_mode | `Option<bool>` | no | — |

### RasterBand

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `usize` | yes | Exact output width |
| height | `u32` | yes | — |
| y0 | `u32` | yes | Inclusive band start row |
| y1 | `u32` | yes | Exclusive band end row |
| rgb | `u8[]` | yes | Reused RGB storage for 256 output rows plus up to 14 halo rows above and below |
| features | `Feature[]` | yes | Saved lake/sea/land ownership masks connected world Atlas channels; legacy Atlas APIs also reuse these classifications for Mid/Dark symbol distance fields |

### ImageQuality

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u32` | yes | Accepted: 512–32,768 pixels; default 8,192 |

### OverviewLook

| Field | Type | Required | Notes |
|---|---|---|---|
| oblique | `bool` | yes | Goal 24: a slightly oblique, soft 3-D view (recipe-5 Atlas only): ground shifts north with its height (coasts stay put) and takes aerial perspective, valley occlusion and sky light. |

### MapStyle

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Classic, Atlas |

### ExportFormat

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Png, Json |

### World

| Field | Type | Required | Notes |
|---|---|---|---|
| dir | `PathBuf` | yes | — |
| manifest | `Manifest` | yes | — |
| domain | `arda_core::hydrology::HydrologyDomain` | yes | Reconstructed from validated manifest configuration and exported dimensions |
| areas | `BTreeMap<AreaCoord, OnceLock<Area>>` | yes | Empty OnceLock slots at load; area() retains only successfully requested areas |
| blocks | `BTreeMap<AreaCoord, OnceLock<arda_core::BlockArchive>>` | yes | Independent requested-area archive cache; first block query decodes that whole archive |

### Area

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `AreaCells` | yes | — |
| objects | `AreaObjects` | yes | — |
| water | `Option<arda_core::water::AreaWater>` | no | — |

### Biome (arda-ids)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: temperate, warm_temperate, temperate_forest, boreal_forest, highland, alpine, wetland, steppe, coastal |

### Cover (arda-ids)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total (alias full) |

### BuildingFunction (arda-ids)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: house, farmhouse, cottage, manor, inn, tavern, bakery, brewery, mill, smithy, workshop, tannery, apothecary, temple, shrine, library, warehouse, market, stall, dock, boathouse, keep, barracks, guardhouse, stable, barn, street, farm, market_hall, mine, lumber_camp, school |

### Craft

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: carpentry, weaving, pottery, cobbling, masonry, jewellery, glassblowing, leatherwork, woodcarving, tinkering, painting, cartography |

### SettlementId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | decimal string |

### RealmId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | decimal string |

### BuildingId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | decimal string |

### NpcId (arda-ids)

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | decimal string |

### LandUse

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none=0, built=1, field=2 (alias arable), pasture=3, orchard=4, woodland=5, mill=6, mine=7 (alias mine_quarry), meadow=8, fallow=9, farmstead=10 |

### RoadClass (arda-ids)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none=0, track=1, road=2, highway=3, footpath=4 |

### SoundChange

| Field | Type | Required | Notes |
|---|---|---|---|
| lenition | `unit` | no | variant (externally tagged); Voiceless stops voice between vowels (t → d). |
| spirantisation | `unit` | no | variant (externally tagged); Voiced stops become fricatives between vowels (d → dh). |
| final_devoicing | `unit` | no | variant (externally tagged); Voiced obstruents devoice at the end of a word (d → t). |
| palatalisation | `unit` | no | variant (externally tagged); /k g/ become /ch j/ before front vowels. |
| shift | `{from: Ph, to: Ph}` | no | variant (externally tagged); One sound becomes another everywhere (vowel shifts, mergers). |
| apocope | `unit` | no | variant (externally tagged); A final vowel is lost in words of two or more syllables. |
| h_dropping | `unit` | no | variant (externally tagged); /h/ is lost. |
| rhotacism | `unit` | no | variant (externally tagged); /s z/ become /r/ between vowels. |
| sibilant_shift | `unit` | no | variant (externally tagged); /s/ becomes /sh/ before a consonant. |
| monophthongisation | `unit` | no | variant (externally tagged); Diphthongs become long vowels (ai → e:). |
| initial_voicing | `unit` | no | variant (externally tagged); Word-initial fricatives voice (f → v). |
| final_nasal_loss | `unit` | no | variant (externally tagged); A word-final /n/ after a vowel is lost. |
| diphthongisation | `unit` | no | variant (externally tagged); Long vowels become diphthongs (i: → ai). |

### Isogloss

| Field | Type | Required | Notes |
|---|---|---|---|
| change | `SoundChange` | yes | The change. |
| at | `(i64, i64)` | yes | A point on the line. |
| normal | `(i64, i64)` | yes | Normal pointing into the side where the change holds (×1000). |

### DialectMap

| Field | Type | Required | Notes |
|---|---|---|---|
| base | `Language` | yes | The standard language. |
| isoglosses | `Isogloss[]` | yes | Isoglosses in historical order. |

### Blocklist

| Field | Type | Required | Notes |
|---|---|---|---|
| words | `BTreeSet<String>` | yes | — |
| fragments | `BTreeSet<String>` | yes | — |

### Grammar

| Field | Type | Required | Notes |
|---|---|---|---|
| head_first | `bool` | yes | Compounds put the head first ("Ford-of-Oaks") rather than last. |
| linker | `bool` | yes | Head-first compounds join nouns with a genitive linker. |
| family | `[u32; 5]` | yes | Family-name style weights (see `crate::preset::FAMILY_STYLES`). |
| byname | `u32` | yes | Per mille of adults with a byname. |

### Language

| Field | Type | Required | Notes |
|---|---|---|---|
| preset | `Preset` | yes | Preset it was built from. |
| seed | `u64` | yes | Seed it was built from.; serde with `crate::ids` |
| phonology | `Phonology` | yes | Sounds and phonotactics. |
| ortho | `Ortho` | yes | Spelling. |
| grammar | `Grammar` | yes | Customs. |
| lexicon | `Ph[][]` | yes | One root per meaning, in `crate::meaning::ALL` order. |
| name_roots | `Ph[][]` | yes | Meaningless roots for personal names. |
| endings | `[Vec<Vec<Ph>>; 2]` | yes | Feminine and masculine given-name endings. |
| changes | `SoundChange[]` | yes | Dialect sound changes applied when a word is spoken, in order. |
| substrate | `Option<Box<Language>>` | no | The older tongue under this one (rivers, some mountains). |
| blocklist | `Blocklist` | yes | Words no name may contain. |

### Class

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: head, adj, noun, being, affix, trade |

### Meaning

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: the lexicon keys of the `meanings!` table (`crates/arda-names/src/meaning.rs:61`), one variant per row, serialized by its key string |

### Ortho

| Field | Type | Required | Notes |
|---|---|---|---|
| spell | `String[]` | yes | Spelling per phoneme (index = phoneme). |
| final_spell | `(Ph, String)[]` | yes | Word-final spellings. |
| k_front | `Option<String>` | no | Spelling of /k/ before a front vowel, when it differs. |

### Sex (arda-names)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: female, male |

### FamilyCtx

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable identity of the person (an NPC id).; serde with `crate::ids` |
| father | `Option<Name>` | no | The father's given name, for patronymics. |
| family | `Option<Name>` | no | An inherited family name, used as is. |
| trade | `Option<M>` | no | The person's or household's trade, for occupational names. |
| home | `Option<Name>` | no | Home settlement, for toponymic names. |
| adult | `bool` | yes | Adults may carry bynames. |

### PersonName

| Field | Type | Required | Notes |
|---|---|---|---|
| given | `Name` | yes | Given name. |
| family | `Option<Name>` | no | Family name, if the culture gives one. |
| style | `Option<FamilyStyle>` | no | Family-name custom used (`None` when inherited). |
| byname | `Option<Name>` | no | Byname ("the Tall"), native form. |
| full | `String` | yes | Given and family name as written ("Tessa da Velora"). |
| gloss | `String` | yes | English reading ("Tessa, of Oakford, the Tall"). |

### Ph

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u8` | yes | — |

### Phonology

| Field | Type | Required | Notes |
|---|---|---|---|
| consonants | `(Ph, u32)[]` | yes | Consonants and weights. |
| vowels | `(Ph, u32)[]` | yes | Vowels and weights. |
| diphthongs | `(Ph, Ph)[]` | yes | Allowed diphthongs. |
| clusters | `Ph[][]` | yes | Allowed onset clusters. |
| codas | `(Ph, u32)[]` | yes | Single codas and weights. |
| coda_clusters | `Ph[][]` | yes | Allowed coda clusters. |
| no_initial | `Ph[]` | yes | Never word-initial. |
| initial_onset | `u32` | yes | Per mille: word starts with a consonant. |
| cluster_rate | `u32` | yes | Per mille: onset is a cluster. |
| medial_coda | `u32` | yes | Per mille: non-final syllable is closed. |
| final_coda | `u32` | yes | Per mille: word ends in a consonant. |
| diph_rate | `u32` | yes | Per mille: nucleus is a diphthong. |
| hiatus | `bool` | yes | Vowels may meet across syllables. |
| harmony | `bool` | yes | Vowel harmony. |
| geminates | `bool` | yes | Geminates allowed. |
| max_medial | `usize` | yes | Most consonants in a row inside a word. |
| roots | `[u32; 3]` | yes | Root length weights (1, 2, 3 syllables). |
| stress | `Stress` | yes | Stress rule. |
| epenthetic | `Ph` | yes | Vowel used to break clusters. |

### Stress

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: initial, penult, final |

### FamilyStyle

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: patronymic, occupational, toponymic, compound, clan |

### Preset

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: heartland, highland, sylvan, coastal, southern, borderland, dwarvish, elvish, halfling, gnomish, orcish, draconic, infernal, ancient |

### NameScope

| Field | Type | Required | Notes |
|---|---|---|---|
| used | `BTreeSet<String>` | yes | — |

### Tongue (arda-names)

| Field | Type | Required | Notes |
|---|---|---|---|
| preset | `Preset` | yes | The culture's preset. |
| language_seed | `u64` | yes | Seed of the culture's standard language.; serde with `crate::ids` |
| substrate_seed | `u64` | yes | Seed of the world's shared substrate tongue.; serde with `crate::ids` |
| dialect_seed | `u64` | yes | Seed of the dialect map of the culture region.; serde with `crate::ids` |
| width | `i64` | yes | Width of the dialect map, in the caller's units (cells). |
| height | `i64` | yes | Height of the dialect map. |
| x | `i64` | yes | Where the place lies on the map. |
| y | `i64` | yes | Where the place lies on the map. |

### PlaceKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city, fort, abbey, port, mine, river, stream, lake, mountain, hill, forest, marsh, pass, bay, island, cape, vale, region, realm |

### SiteTag

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ford, bridge_site (alias bridge), confluence, harbour (alias harbor), estuary, pass, defensible, ore, timber, fish, salt, spring, river, lake, coast, hill, forest, marsh, mountain, navigable, arable |

### PlaceSpec

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `PlaceKind` | yes | Place kind. |
| features | `Meaning[]` | yes | Descriptive features the name should mention (`Oak`, `White`), in order of preference; empty lets the site and kind choose. |
| site_tags | `SiteTag[]` | yes | Site tags of the place. |
| key | `u64` | yes | Stable identity.; serde with `crate::ids` |
| from | `Option<Name>` | no | For regions and realms: the capital (or region) to derive from. |

### Part

| Field | Type | Required | Notes |
|---|---|---|---|
| meaning | `Option<Meaning>` | no | What it means (`None` for a meaningless personal-name root). |
| gloss | `String` | yes | English form for glosses. |
| ph | `Ph[]` | yes | Underlying phonemes, before dialect sound changes. |

### Name (arda-names)

| Field | Type | Required | Notes |
|---|---|---|---|
| native | `String` | yes | The name in its own language ("Dervanford" becomes e.g. |
| gloss | `String` | yes | English rendering in English place-name style ("Oakford"). |
| literal | `String` | yes | Plain literal meaning ("ford of the oaks"). |
| pronunciation | `String` | yes | Respelling with the stressed syllable in capitals ("TAR-na-gel"). |
| word | `Ph[]` | yes | Underlying phonemes of the whole name, before dialect changes. |
| parts | `Part[]` | yes | Its morphemes, first to last. |

### CrossingKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: bridge, ford, ferry |

### Crossing

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id.; serde with `crate::ids::string` |
| kind | `CrossingKind` | yes | Bridge, ford or ferry. |
| water | `String` | yes | What is crossed: `river`, `sea` or `lake`. |
| x_m | `i64` | yes | Position, metres. |
| y_m | `i64` | yes | Position, metres. |
| width_m | `u32` | yes | Channel width in metres (0 for open water). |
| order | `u8` | yes | Strahler order (0 for open water). |
| road_class | `RoadClass` | yes | Highest road class using the crossing. |
| river | `String` | yes | Name of the river, filled by the naming stage. |

### Pass

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id.; serde with `crate::ids::string` |
| x_m | `i64` | yes | Position, metres. |
| y_m | `i64` | yes | Position, metres. |
| height_m | `i32` | yes | Height of the high point, metres. |
| rise_from_m | `i32` | yes | Climb from the lowest point on the route's start side, metres. |
| rise_to_m | `i32` | yes | Climb from the lowest point on the route's end side, metres. |
| road_id | `u64` | yes | Road whose profile peaks here.; serde with `crate::ids::string` |
| name | `String` | yes | Name, filled by the naming stage. |
| name_gloss | `String` | yes | English gloss of the name. |
| cell | `usize` | no | Grid cell (not serialised).; not serialized |

### Tier (arda-settle)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### Function

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |

### Settlement

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `SettlementId` | yes | Stable identifier, 1-based in placement order (a JSON string, I5). |
| name | `String` | yes | Display name. |
| tier | `Tier` | yes | Size class. |
| population | `u32` | yes | Inhabitants. |
| functions | `Function[]` | yes | Economic functions, sorted. |
| wealth | `u8` | yes | Overall wealth, 0–255. |
| culture | `String` | yes | Culture key (`arda-npc` `data/content/cultures.json`). |
| realm_id | `RealmId` | yes | Owning realm, 1-based (a JSON string, I5). |
| biome | `Biome` | yes | Biome, from a closed list. |
| coastal | `bool` | yes | On a sea coast. |
| riverine | `bool` | yes | On a river. |
| name_gloss | `String` | yes | English gloss of the name ("Oakford"); `name` is the native form. |
| x_m | `i64` | yes | Centre, metres east of the world's west edge. |
| y_m | `i64` | yes | Centre, metres south of the world's north edge. |
| cell_x | `u32` | yes | Centre cell column. |
| cell_y | `u32` | yes | Centre cell row. |
| height_m | `i32` | yes | Ground height at the centre, metres. |
| rank | `u32` | yes | Rank among towns and cities (1 = largest); 0 for villages and hamlets. |
| site_tags | `String[]` | yes | Descriptive site tags (see the README). |
| history | `String` | yes | A one-line origin hook for the referee. |
| buildings | `BTreeMap<BuildingFunction, u32>` | yes | Estimated building mix: building function → count. |
| tongue | `Option<arda_names::Tongue>` | no | The local speech the name was drawn in, for naming the settlement's people in the same dialect (A5).; serde default; omitted when empty/none |
| tag_bits | `u32` | no | Tag bits of the centre cell (not serialised).; not serialized |
| score | `u32` | no | Placement score (not serialised).; not serialized |

### SettlementsFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Format version. |
| seed | `u64` | yes | World seed.; serde with `crate::ids::string` |
| width_cells | `u32` | yes | Grid cells across (100 m each). |
| height_cells | `u32` | yes | Grid cells down. |
| settlements | `Settlement[]` | yes | Settlements by id. |

### RoadsFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Format version. |
| roads | `Road[]` | yes | Road objects. |
| crossings | `Crossing[]` | yes | Crossing objects. |
| passes | `Pass[]` | yes | Pass objects. |

### RealmsFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Format version. |
| realms | `Realm[]` | yes | Realms by id. |

### NamedRiver

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id.; serde with `crate::ids::string` |
| name | `String` | yes | Native name. |
| name_gloss | `String` | yes | English gloss of the name. |
| mouth_m | `[i64; 2]` | yes | Mouth, metres. |
| length_m | `u64` | yes | Traced main-stem length, metres. |
| order | `u8` | yes | Highest Strahler order. |
| course_m | `[i64; 2][]` | yes | Main-stem course from the mouth upstream, every 400 m, metres. |

### NamedPeak

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Native name. |
| name_gloss | `String` | yes | English gloss of the name. |
| at_m | `[i64; 2]` | yes | Summit, metres. |
| height_m | `i32` | yes | Summit height, metres. |

### NamedRegion

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id.; serde with `crate::ids::string` |
| name | `String` | yes | Native name. |
| name_gloss | `String` | yes | English gloss of the name. |
| culture | `String` | yes | Culture key. |
| land_ha | `u64` | yes | Approximate land area, hectares. |

### NamesFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Format version. |
| rivers | `NamedRiver[]` | yes | Rivers. |
| mountains | `NamedPeak[]` | yes | Mountains. |
| regions | `NamedRegion[]` | yes | Regions. |

### RealmStats

| Field | Type | Required | Notes |
|---|---|---|---|
| cities_per_realm | `u64[]` | yes | Cities in each realm, in realm id order. |
| seat_lattice_km | `f64` | yes | Hexagonal-lattice spacing of `N` seats over the land, `√(2A / (√3 N))`, km: the distance evenly spread seats would keep. |
| seat_nn_min_km | `f64` | yes | Smallest distance from a seat to its nearest other seat, km. |
| seat_nn_mean_km | `f64` | yes | Mean of those distances, km. |
| people_to_seat_median_km | `f64` | yes | Median distance from a person to the nearest seat, km. |
| people_to_seat_p90_km | `f64` | yes | 90th percentile of that distance, km. |
| land_share_max_pm | `u32` | yes | Largest realm share of the land, per mille. |
| land_share_min_pm | `u32` | yes | Smallest realm share of the land, per mille. |
| people_share_max_pm | `u32` | yes | Largest realm share of the people, per mille. |
| people_share_min_pm | `u32` | yes | Smallest realm share of the people, per mille. |

### Realm

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id.; serde with `crate::ids::string` |
| name | `String` | yes | Name, filled by the naming stage. |
| name_gloss | `String` | yes | English gloss of the name ("Oakmark"). |
| seat | `u64` | yes | Seat settlement id.; serde with `crate::ids::string` |
| culture | `String` | yes | Culture of the seat. |
| settlements | `u64[]` | yes | Member settlement ids.; serde with `crate::ids::vec_string` |
| population | `u64` | yes | Total population of members. |
| land_cells | `u64` | yes | Land cells (hectares). |
| neighbours | `u64[]` | yes | Realms sharing a land border.; serde with `crate::ids::vec_string` |
| borders | `[i64; 2][][]` | yes | Land borders as polylines of `[x_m, y_m]` cell corners. |

### Road

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id in build order.; serde with `crate::ids::string` |
| class | `RoadClass` | yes | Class. |
| from | `u64` | yes | Settlement the route starts at.; serde with `crate::ids::string` |
| to | `Option<u64>` | no | Settlement the route ends at, if any.; serde with `crate::ids::opt_string` |
| to_edge | `Option<String>` | no | Map edge the route leaves through (`north`, `east`, `south`, `west`). |
| length_m | `u64` | yes | Length of the whole route, shared stretches included, metres. |
| straight_m | `u64` | yes | Straight-line distance between the ends, metres. |
| new_m | `u64` | yes | Length of the new cells this road added, metres. |
| relief_m | `i32` | yes | Height range along the whole route, metres (open ground is < 150). |
| terrain | `Terrain` | yes | Ground the whole route crosses, by its median slope. |
| segments | `[i64; 2][][]` | yes | New stretches as polylines of `[x_m, y_m]` cell centres. |

### Terrain

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: open, hill, mountain |

### Spread

| Field | Type | Required | Notes |
|---|---|---|---|
| routes | `u64` | yes | Routes counted. |
| median | `f64` | yes | Median. |
| p90 | `f64` | yes | 90th percentile. |
| max | `f64` | yes | Largest. |

### Stats

| Field | Type | Required | Notes |
|---|---|---|---|
| target_population | `u64` | yes | Population the land supports (density × land). |
| population | `u64` | yes | Population placed. |
| count_by_tier | `BTreeMap<String, u64>` | yes | Settlements per tier. |
| population_by_tier | `BTreeMap<String, u64>` | yes | People per tier. |
| rank_size_slope | `f64` | yes | Least-squares slope of ln(population) on ln(rank) over towns and cities (the rank-size rule is −1). |
| rank_size_r2 | `f64` | yes | R² of that fit. |
| farmland_ha_per_person | `f64` | yes | Farmland (fields and orchards) per inhabitant, hectares. |
| road_km | `BTreeMap<String, f64>` | yes | Road kilometres per class (new cells only, so shared stretches count once). |
| sinuosity_median | `BTreeMap<String, f64>` | yes | Median route sinuosity (route length ÷ straight line) per class. |
| sinuosity_all | `f64` | yes | Median sinuosity over every route of 1.5 km or more. |
| sinuosity_open | `f64` | yes | Median sinuosity over open-ground routes (median slope under 5°), the artifact's 1.2–1.4 check; hill and mountain routes rise above it. |
| open_routes | `u64` | yes | Routes counted in `sinuosity_open`. |
| sinuosity_by_terrain | `BTreeMap<String, Spread>` | yes | Sinuosity per terrain class (`open`, `hill`, `mountain`). |
| runaway_routes | `u64` | yes | Routes more than `RUNAWAY` times their straight line. |
| crossings | `BTreeMap<String, u64>` | yes | Crossings per kind. |
| passes | `u64` | yes | Passes found. |
| realms | `u64` | yes | Realms. |
| border_on_river_or_ridge_pm | `u32` | yes | Per mille of the realm border that has a river or ridge line within three cells and runs on it, after snapping (target ≥ 900). |
| border_on_river_or_ridge_raw_pm | `u32` | yes | The same share before the snapping pass. |
| border_natural_of_all_pm | `u32` | yes | Per mille of all realm border on a river or ridge line. |
| land_near_river_or_ridge_pm | `u32` | yes | Baseline: per-mille of all land within two cells of one. |
| unreachable | `u64` | yes | Settlements no route reached. |
| inland_town_share | `f64` | yes | Share of towns and cities away from the coast (central places at inland crossings and basins). |
| town_nni | `f64` | yes | Clark–Evans nearest-neighbour index of towns and cities over the land area: 1 is random, above 1 regular, 2.15 a perfect hexagonal lattice. |
| town_nn_km | `f64` | yes | Mean distance from a town to its nearest town, km. |
| mining_settlements | `u64` | yes | Settlements with the `mining` function. |
| mining_share_pm | `u32` | yes | Their share of all settlements, per mille. |
| mining_clustered_pm | `u32` | yes | Per mille of mining settlements with another within `MINING_CLUSTER_M` (1000 means every mine has a neighbour). |
| realm | `crate::realm_stats::RealmStats` | no | Cities per realm, seat spread and realm balance.; serde default |

### BuildingRef

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Building id, unique within the settlement.; serde with `crate::ids::str` |
| function | `String` | yes | Building-function key (plain snake_case). |
| tags | `String[]` | no | Free tags; a workshop carries `craft:<name>`.; serde default; omitted when empty/none |

### GoodRole

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: export, import, transit |

### KeyGood

| Field | Type | Required | Notes |
|---|---|---|---|
| good | `String` | yes | Good key. |
| role | `GoodRole` | yes | Dominant direction. |
| value_sp | `u64` | yes | Value handled per year, silver pieces. |

### SettlementEconomy

| Field | Type | Required | Notes |
|---|---|---|---|
| ledger | `Ledger` | yes | Where every load went. |
| trade_balance_sp | `i64` | yes | Export value minus import value, silver pieces per year. |
| market | `bool` | yes | Whether it is a market (market or port function). |
| key_goods | `KeyGood[]` | yes | Key trade goods (markets only), most valuable first. |

### RoadTraffic

| Field | Type | Required | Notes |
|---|---|---|---|
| road | `u64` | yes | Road id.; serde with `crate::ids::str` |
| value_sp | `u64` | yes | Value carried per year, silver pieces. |
| goods | `String[]` | yes | Most valuable goods carried, up to three. |

### GoodTotals

| Field | Type | Required | Notes |
|---|---|---|---|
| good | `String` | yes | Good key. |
| label | `String` | yes | Readable name. |
| value_sp | `u32` | yes | Value of one load, silver pieces. |
| produced | `u64` | yes | Loads produced. |
| local_use | `u64` | yes | Loads used where made. |
| traded | `u64` | yes | Loads traded. |
| stored | `u64` | yes | Loads stored unsold. |
| shortfall | `u64` | yes | Loads wanted but missing. |

### Economy

| Field | Type | Required | Notes |
|---|---|---|---|
| goods | `GoodTotals[]` | yes | Totals per good, table order. |
| flows | `TradeFlow[]` | yes | All flows. |
| road_traffic | `RoadTraffic[]` | yes | Traffic per road, busiest first. |

### TradeFlow

| Field | Type | Required | Notes |
|---|---|---|---|
| good | `String` | yes | Good key. |
| from | `u64` | yes | Exporting settlement.; serde with `crate::ids::str` |
| to | `u64` | yes | Importing settlement.; serde with `crate::ids::str` |
| amount | `u64` | yes | Loads per year. |
| value_sp | `u64` | yes | Value per year, silver pieces. |
| distance_m | `u64` | yes | Road distance, metres. |
| path | `u64[]` | yes | Settlements on the way, both ends included.; serde with `crate::ids::vec` |
| roads | `u64[]` | yes | Road ids on the way, in travel order.; serde with `crate::ids::vec` |

### Ledger

| Field | Type | Required | Notes |
|---|---|---|---|
| production | `Basket` | yes | Loads produced. |
| demand | `Basket` | yes | Loads wanted. |
| local_use | `Basket` | yes | Own production used at home. |
| exports | `Basket` | yes | Loads sent out. |
| imports | `Basket` | yes | Loads brought in. |
| stored | `Basket` | yes | Surplus nobody could reach. |
| shortfall | `Basket` | yes | Demand left unmet. |

### HistoryHook

| Field | Type | Required | Notes |
|---|---|---|---|
| text | `String` | yes | The line. |
| year | `Option<i32>` | no | Year it refers to, if any. |
| refs | `EntityRef[]` | yes | Entities it mentions. |

### EventKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: founding, realm_formed, dynasty_founded, accession, war, peace, border_shift, flood, plague, fire, famine, mine_collapse, abandonment |

### Event

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | Stable id, in timeline order. |
| year | `i32` | yes | Year it began. |
| end_year | `Option<i32>` | no | Year it ended, for lasting events; `None` if instantaneous or ongoing. |
| ongoing | `bool` | yes | Whether it is still going on at present. |
| kind | `EventKind` | yes | Kind. |
| title | `String` | yes | Short title. |
| text | `String` | yes | One or two sentences of original text. |
| settlements | `u64[]` | yes | Settlements involved.; serde with `crate::ids::vec` |
| realms | `u64[]` | yes | Realms involved.; serde with `crate::ids::vec` |
| refs | `EntityRef[]` | yes | Further references. |
| cause | `Option<u32>` | no | Event that caused this one. |
| severity | `u8` | yes | 1 (minor) to 3 (catastrophic). |

### Dynasty

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable id (`d<realm>.<n>`). |
| realm_id | `u64` | yes | Realm ruled.; serde with `crate::ids::str` |
| name | `String` | yes | House name. |
| from | `i32` | yes | First year in power. |
| to | `Option<i32>` | no | Last year in power; `None` if ruling now. |
| origin | `String` | yes | How it came to power (`founding`, `usurpation`, `succession`, `election`, `conquest`). |

### Reign

| Field | Type | Required | Notes |
|---|---|---|---|
| realm_id | `u64` | yes | Realm ruled.; serde with `crate::ids::str` |
| dynasty_id | `String` | yes | Ruling house. |
| given | `String` | yes | Given name. |
| regnal | `String` | yes | Regnal style, e.g. |
| female | `bool` | yes | Female ruler. |
| title | `String` | yes | Title held. |
| from | `i32` | yes | First year. |
| to | `Option<i32>` | no | Last year; `None` if reigning now. |
| end | `Option<String>` | no | How the reign ended (`died`, `fell`, `deposed`, `plague`, `abdicated`, `heirless`). |

### BorderShift

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement | `u64` | yes | Settlement.; serde with `crate::ids::str` |
| from_realm | `u64` | yes | Realm it belonged to before.; serde with `crate::ids::str` |
| to_realm | `u64` | yes | Realm it belongs to now.; serde with `crate::ids::str` |
| year | `i32` | yes | Year it changed hands. |
| war_event | `u32` | yes | War that moved it. |

### War

| Field | Type | Required | Notes |
|---|---|---|---|
| event | `u32` | yes | War event id. |
| name | `String` | yes | Name. |
| realms | `[u64; 2]` | yes | The two realms, lower id first.; serde with `crate::ids::pair` |
| from | `i32` | yes | First year. |
| to | `Option<i32>` | no | Last year; `None` if still fought. |
| winner | `Option<u64>` | no | Winner, once over.; serde default; serde with `crate::ids::opt` |

### Ruin

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | Stable id, 1-based. |
| name | `String` | yes | Name it bore. |
| kind | `String` | yes | What it was (`hamlet`, `village`, `watchtower`, …). |
| former_tier | `Tier` | yes | Size it reached. |
| x_m | `i64` | yes | Position, metres. |
| y_m | `i64` | yes | Position, metres. |
| near | `u64` | yes | Nearest living settlement.; serde with `crate::ids::str` |
| road | `Option<u64>` | no | Road it lies beside, if any.; serde default; serde with `crate::ids::opt` |
| realm_id | `u64` | yes | Realm whose land it lies in.; serde with `crate::ids::str` |
| founded | `i32` | yes | Year founded. |
| abandoned | `i32` | yes | Year abandoned. |
| cause | `String` | yes | Why (`plague`, `flood`, `war`, `famine`, `fire`, `mine_collapse`, `well_failed`, `raiders`, `enclosure`). |
| cause_event | `Option<u32>` | no | Event that emptied it, if any. |
| event | `u32` | yes | Abandonment event id. |

### Allegiance

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement | `u64` | yes | Settlement.; serde with `crate::ids::str` |
| first_realm | `u64` | yes | Realm it swore to at founding (or at its realm's formation).; serde with `crate::ids::str` |

### History

| Field | Type | Required | Notes |
|---|---|---|---|
| era | `String` | yes | Era name. |
| era_abbrev | `String` | yes | Era abbreviation for dates. |
| start_year | `i32` | yes | First year. |
| present_year | `i32` | yes | Present year. |
| events | `Event[]` | yes | Timeline, in id (= chronological) order. |
| dynasties | `Dynasty[]` | yes | Ruling houses. |
| reigns | `Reign[]` | yes | Reigns, per realm in order. |
| wars | `War[]` | yes | Wars. |
| border_shifts | `BorderShift[]` | yes | Border shifts. |
| ruins | `Ruin[]` | yes | Ruins. |
| allegiances | `Allegiance[]` | yes | First allegiance of every settlement; replaying `border_shifts` on it gives the present partition (`History::replay_partition`). |

### Trend

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: rising, stable, declining, recovering |

### Prosperity

| Field | Type | Required | Notes |
|---|---|---|---|
| present | `u8` | yes | Present wealth, 0–255 (the input value). |
| trend | `Trend` | yes | Current direction. |
| samples | `[i32; 2][]` | yes | `[year, wealth]` samples from founding to present. |

### Hook

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable id (`h<settlement>.<kind>` or `hr<realm>.<kind>`). |
| kind | `String` | yes | Kind key. |
| title | `String` | yes | Title. |
| text | `String` | yes | Two or three sentences for the referee. |
| tension | `u8` | yes | How hot the tension is, 0–100. |
| refs | `EntityRef[]` | yes | Entities involved. |

### Tier (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### Function (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |

### Settlement (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable identifier.; serde with `crate::ids::str` |
| name | `String` | yes | Display name. |
| tier | `Tier` | yes | Size class. |
| population | `u32` | yes | Inhabitants. |
| functions | `Function[]` | yes | Economic functions. |
| wealth | `u8` | yes | Overall wealth, 0–255. |
| culture | `String` | yes | Culture key. |
| realm_id | `u64` | yes | Owning realm.; serde with `crate::ids::str` |
| biome | `String` | yes | Biome key, snake_case as arda-settle writes it (`temperate_forest`); it must be a key of `data/goods.json` `biomes`. |
| coastal | `bool` | yes | On a sea coast. |
| riverine | `bool` | yes | On a river. |
| x_m | `i64` | yes | Centre, metres east of the world's west edge. |
| y_m | `i64` | yes | Centre, metres south of the world's north edge. |
| cell_x | `u32` | no | Centre cell column.; serde default |
| cell_y | `u32` | no | Centre cell row.; serde default |
| height_m | `i32` | no | Ground height at the centre, metres.; serde default |
| rank | `u32` | no | Rank among towns and cities (1 = largest); 0 otherwise.; serde default |
| site_tags | `String[]` | no | Site tags (`ford`, `harbour`, `ore`, `river`, …).; serde default |
| history | `String` | no | One-line site history hook from the settlement stage.; serde default |
| buildings | `BTreeMap<String, u32>` | no | Estimated building mix: building-function key → count.; serde default |

### RoadClass (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, track, road, highway, footpath |

### Road (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable id (a u64 as a JSON string, I5; numbers are accepted).; serde with `crate::ids::str` |
| class | `RoadClass` | yes | Class. |
| from | `u64` | yes | Settlement the route starts at.; serde with `crate::ids::str` |
| to | `Option<u64>` | no | Settlement the route ends at, if any.; serde default; serde with `crate::ids::opt` |
| to_edge | `Option<String>` | no | Map edge the route leaves through, if any.; serde default |
| length_m | `u64` | yes | Route length, metres. |
| straight_m | `u64` | no | Straight-line distance between the ends, metres.; serde default |
| new_m | `u64` | no | Length of the new cells this road added, metres.; serde default |
| segments | `[i64; 2][][]` | no | New stretches as polylines of `[x_m, y_m]` points.; serde default |

### Realm (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable id (settlements' `realm_id`).; serde with `crate::ids::str` |
| name | `String` | yes | Realm name. |
| seat | `u64` | yes | Seat settlement id.; serde with `crate::ids::str` |
| members | `u64[]` | no | Member settlement ids (optional; derived from `realm_id` when empty).; serde default; serde with `crate::ids::vec` |

### BuildingSpec (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Building id, unique within the settlement.; serde with `crate::ids::str` |
| settlement_id | `u64` | yes | Owning settlement.; serde with `crate::ids::str` |
| function | `String` | yes | Building-function key (`inn`, `keep`, `workshop`, …). |
| tags | `String[]` | no | Free tags, e.g.; serde default |

### WorldSettlements

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | no | Format version of the settlement stage's files.; serde default |
| settlements | `Settlement[]` | yes | All settlements. |
| roads | `Road[]` | no | All roads.; serde default |
| realms | `Realm[]` | no | All realms.; serde default |
| buildings | `BuildingSpec[]` | no | Explicit buildings; settlements without any get buildings derived from their mix (see `buildings.rs`).; serde default |

### SettlementSociety

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Settlement id.; serde with `crate::ids::str` |
| name | `String` | yes | Name. |
| realm_id | `u64` | yes | Realm today.; serde with `crate::ids::str` |
| founded | `i32` | yes | Year founded. |
| founded_from | `Option<u64>` | no | Settlement whose settlers founded it.; serde default; serde with `crate::ids::opt` |
| founding_event | `u32` | yes | Founding event id. |
| buildings | `BuildingRef[]` | yes | Buildings with ids. |
| economy | `SettlementEconomy` | yes | Economy. |
| prosperity | `Prosperity` | yes | Prosperity over time. |
| factions | `Faction[]` | yes | Factions. |
| faction_relations | `FactionRelation[]` | yes | Stances between its factions. |
| roles | `NpcRole[]` | yes | Notable NPC slots. |
| history_hooks | `HistoryHook[]` | yes | 2–4 lines of history. |
| hooks | `Hook[]` | yes | 3–5 plot hooks. |

### RealmSociety

| Field | Type | Required | Notes |
|---|---|---|---|
| state | `RealmState` | yes | State.; flattened into parent |
| hooks | `Hook[]` | yes | 3–5 plot hooks. |

### Society

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Output format version. |
| seed | `u64` | yes | World seed.; serde with `crate::ids::str` |
| realms | `RealmSociety[]` | yes | Realms, by id. |
| relations | `Relation[]` | yes | Relations, one per realm pair. |
| settlements | `SettlementSociety[]` | yes | Settlements, in input order. |
| economy | `Economy` | yes | World economy. |
| history | `History` | yes | History. |

### Faction

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable id (`f<settlement>.<kind>`). |
| settlement | `u64` | yes | Settlement.; serde with `crate::ids::str` |
| kind | `String` | yes | Kind key. |
| kind_label | `String` | yes | Readable kind. |
| name | `String` | yes | Name. |
| goals | `String[]` | yes | What it wants. |
| leader_role | `String` | yes | Leader NPC slot. |
| seat_building | `Option<u64>` | no | Building it meets in. |
| influence | `u8` | yes | Influence, 0–100. |

### FactionStance

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: allied, friendly, rival, hostile |

### FactionRelation

| Field | Type | Required | Notes |
|---|---|---|---|
| a | `String` | yes | Lower faction id. |
| b | `String` | yes | Higher faction id. |
| stance | `FactionStance` | yes | Stance. |
| reason | `String` | yes | Why. |

### Ruler

| Field | Type | Required | Notes |
|---|---|---|---|
| role | `String` | yes | NPC slot at the seat. |
| given | `String` | yes | Given name. |
| regnal | `String` | yes | Regnal style. |
| title | `String` | yes | Title. |
| female | `bool` | yes | Female ruler. |
| dynasty | `String` | yes | Ruling house id. |
| house | `String` | yes | House name. |
| since | `i32` | yes | Reigning since. |

### Vassal

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement | `u64` | yes | Settlement held.; serde with `crate::ids::str` |
| role | `String` | yes | NPC slot of the lord. |
| title | `String` | yes | Title. |
| given | `String` | yes | Suggested given name. |
| female | `bool` | yes | Female lord. |
| house | `String` | yes | House name. |
| loyalty | `u8` | yes | Loyalty to the ruler, 0–100. |
| grievances | `String[]` | yes | Grievances against the crown. |
| claimant | `bool` | yes | Whether the house once held the throne and could claim it again. |

### RealmState

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Realm id.; serde with `crate::ids::str` |
| name | `String` | yes | Realm name. |
| style | `String` | yes | Formal style, e.g. |
| government | `String` | yes | Government key. |
| government_label | `String` | yes | Readable government. |
| rank | `String` | yes | `kingdom`, `duchy` or `county`. |
| seat | `u64` | yes | Seat settlement.; serde with `crate::ids::str` |
| founded | `i32` | yes | Year formed. |
| members | `u64[]` | yes | Member settlements.; serde with `crate::ids::vec` |
| population | `u64` | yes | Total population. |
| culture | `String` | yes | Culture of the seat. |
| tax_pct | `u32` | yes | Levy on trade, percent. |
| levy_sp | `u64` | yes | Yearly levy on the realm's trade, silver pieces. |
| ruler | `Ruler` | yes | Present ruler. |
| vassals | `Vassal[]` | yes | Vassal lords. |

### Stance

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: alliance, trade_pact, neutral, rivalry, war |

### Relation

| Field | Type | Required | Notes |
|---|---|---|---|
| a | `u64` | yes | Lower realm id.; serde with `crate::ids::str` |
| b | `u64` | yes | Higher realm id.; serde with `crate::ids::str` |
| stance | `Stance` | yes | Stance. |
| score | `i32` | yes | Underlying score (positive friendly). |
| border_roads | `u64[]` | yes | Roads crossing their shared border.; serde with `crate::ids::vec` |
| trade_sp | `u64` | yes | Yearly trade between them, silver pieces. |
| wars | `u32[]` | yes | War event ids between them. |
| marriage | `bool` | yes | Whether their houses are joined by marriage. |
| reasons | `String[]` | yes | Why they stand as they do. |

### SettlementsFile (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | — |
| settlements | `Settlement[]` | yes | — |

### RoadsFile (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | — |
| roads | `Road[]` | yes | — |

### RealmsFile (arda-society)

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | — |
| realms | `Realm[]` | yes | — |

### EntityRef

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement | `{id: u64}` | no | variant (internally tagged `kind`); An input settlement. |
| realm | `{id: u64}` | no | variant (internally tagged `kind`); An input realm. |
| road | `{id: u64}` | no | variant (internally tagged `kind`); An input road. |
| building | `{settlement: u64, id: u64}` | no | variant (internally tagged `kind`); A building of a settlement. |
| faction | `{id: String}` | no | variant (internally tagged `kind`); A faction. |
| role | `{id: String}` | no | variant (internally tagged `kind`); A notable NPC slot. |
| ruin | `{id: u32}` | no | variant (internally tagged `kind`); A ruin of an abandoned settlement. |
| event | `{id: u32}` | no | variant (internally tagged `kind`); A timeline event. |
| dynasty | `{id: String}` | no | variant (internally tagged `kind`); A ruling house. |
| good | `{key: String}` | no | variant (internally tagged `kind`); A trade good. |

### ClassLevel

| Field | Type | Required | Notes |
|---|---|---|---|
| class | `String` | yes | SRD class name. |
| level | `u8` | yes | Level. |

### NpcRole

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable id. |
| settlement | `u64` | yes | Settlement.; serde with `crate::ids::str` |
| kind | `String` | yes | Role kind key (`ruler`, `captain`, `high_priest`, …). |
| title | `String` | yes | Title as the NPC would be addressed. |
| building | `Option<u64>` | no | Workplace building.; serde default; serde with `crate::ids::opt` |
| faction | `Option<String>` | no | Faction led, if any. |
| stat_block | `Option<String>` | no | Suggested SRD 5.1 NPC stat block. |
| class | `Option<ClassLevel>` | no | Suggested SRD 5.1 class and level. |
| given_name | `Option<String>` | no | Given name fixed by history (rulers, vassals). |
| family_name | `Option<String>` | no | Family name fixed by history. |
| female | `Option<bool>` | no | Sex fixed by history. |
| duties | `String` | yes | What the role does. |

### Yield

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `String` | yes | — |
| 1 | `u32` | yes | — |

### Good

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | Stable key. |
| label | `String` | yes | Readable name. |
| value_sp | `u32` | yes | Value of one load, silver pieces. |
| range_km | `u32` | yes | How far a load is worth carrying, km. |
| demand_per_100 | `u32` | yes | Loads consumed per 100 people a year. |
| urban_only | `bool` | no | Only towns and cities want it.; serde default |
| staple | `bool` | no | A staple even small places need (drives village shortage hooks).; serde default |
| wealth_scaled | `bool` | no | Demand scales with wealth (128 = ×1).; serde default |

### Input

| Field | Type | Required | Notes |
|---|---|---|---|
| good | `String` | yes | Input good. |
| per | `u32` | yes | Loads of input per 10 loads of output. |

### BiomeMod

| Field | Type | Required | Notes |
|---|---|---|---|
| mult_pct | `BTreeMap<String, u32>` | no | Percent multipliers per good (100 = unchanged).; serde default |
| extra | `Yield[]` | no | Extra yields per 100 farming workers.; serde default |

### GoodsTable

| Field | Type | Required | Notes |
|---|---|---|---|
| goods | `Good[]` | yes | All goods. |
| land_share_pct | `BTreeMap<String, u32>` | yes | Percent of people working the land, per tier key. |
| land | `BTreeMap<String, Vec<Yield>>` | yes | Land yields per 100 workers, per function key. |
| function_weight | `BTreeMap<String, u32>` | yes | Share of the land workforce each land function draws. |
| buildings | `BTreeMap<String, Vec<Yield>>` | yes | Yields per building, per building-function key. |
| crafts | `BTreeMap<String, Vec<Yield>>` | yes | Yields per workshop, per craft key. |
| workshop_crafts | `String[]` | yes | Craft cycle for workshops derived from a building mix. |
| inputs | `BTreeMap<String, Input>` | yes | Processing inputs per output good. |
| biomes | `BTreeMap<String, BiomeMod>` | yes | Biome modifiers by biome name. |
| sites | `BTreeMap<String, Vec<Yield>>` | yes | Yields per 100 people from site tags. |

### Government

| Field | Type | Required | Notes |
|---|---|---|---|
| label | `String` | yes | Readable name. |
| hereditary | `bool` | yes | Whether rule passes by blood. |
| tax_pct | `u32` | yes | Levy on trade, percent. |
| titles | `BTreeMap<String, [String; 2]>` | yes | Ruler titles `[male, female]` per realm rank. |
| style | `BTreeMap<String, String>` | yes | Realm style per rank, with `{name}`. |
| vassal | `[String; 2]` | yes | Vassal title `[male, female]`. |
| blurb | `String` | yes | One-line description. |
| ruler | `Option<Suggestion>` | no | Suggested stat build for the ruler of a town or city seat.; serde default |

### PoliticsTable

| Field | Type | Required | Notes |
|---|---|---|---|
| governments | `BTreeMap<String, Government>` | yes | Government types by key. |
| heads | `BTreeMap<String, [String; 2]>` | yes | Special head titles `[male, female]` (`village`, `hamlet`, `fortress`, `border`). |
| grievances | `BTreeMap<String, String>` | yes | Vassal grievance templates per key. |
| reasons | `BTreeMap<String, String>` | yes | Relation reason templates per key. |

### FactionKind

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | Stable key. |
| label | `String` | yes | Readable kind. |
| buildings_any | `String[]` | no | Present when the settlement has any of these buildings.; serde default |
| functions_any | `String[]` | no | Present when the settlement has any of these functions.; serde default |
| min_tier | `Tier` | yes | Smallest tier where it forms. |
| names | `String[]` | yes | Name templates. |
| goals | `String[]` | yes | Goal templates. |
| goals_rural | `String[]` | no | Goal templates for villages and hamlets, when they differ.; serde default |
| leader | `String` | yes | Role kind of its leader. |
| leader_hamlet | `Option<String>` | no | Role kind of its leader in a hamlet, where one figure speaks for everyone (`head` = the settlement's elder); `leader` when absent.; serde default |
| power | `u32` | yes | Base influence. |
| seat | `String[]` | yes | Preferred seat buildings, first match wins. |

### Affinity

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `String` | yes | — |
| 1 | `String` | yes | — |
| 2 | `i32` | yes | — |

### FactionTable

| Field | Type | Required | Notes |
|---|---|---|---|
| kinds | `FactionKind[]` | yes | Faction kinds. |
| affinity | `Affinity[]` | yes | Pairwise base affinities. |
| reasons | `BTreeMap<String, Vec<String>>` | yes | Reason templates per stance key. |
| patrons | `String[]` | yes | Original patron names for temples. |
| pair_reasons | `BTreeMap<String, Vec<String>>` | no | Reasons for a specific kind pair `"a\|b"` (keys sorted), used for rival and hostile stances.; serde default |

### Suggestion

| Field | Type | Required | Notes |
|---|---|---|---|
| block | `Option<String>` | no | SRD NPC stat block name.; serde default |
| class | `Option<(String, u8)>` | no | SRD class and level.; serde default |

### RoleDef

| Field | Type | Required | Notes |
|---|---|---|---|
| title | `String` | yes | Title template. |
| buildings | `String[]` | yes | Preferred buildings, first match wins. |
| by_tier | `BTreeMap<String, Suggestion>` | yes | Stat suggestion per tier key. |
| duties | `String` | yes | What the role does. |

### RoleTable

| Field | Type | Required | Notes |
|---|---|---|---|
| roles | `BTreeMap<String, RoleDef>` | yes | Role kinds. |

### HistoryTable

| Field | Type | Required | Notes |
|---|---|---|---|
| era | `String` | yes | Era name for dates. |
| era_abbrev | `String` | yes | Era abbreviation. |
| titles | `BTreeMap<String, Vec<String>>` | yes | Event titles per event kind. |
| events | `BTreeMap<String, Vec<String>>` | yes | Event text per event kind. |
| lore | `BTreeMap<String, Vec<String>>` | yes | History-hook text per hook kind. |
| war_names | `String[]` | yes | War name patterns. |
| plague_names | `String[]` | yes | Plague names. |
| reign_ends | `BTreeMap<String, String>` | yes | How a reign ends, per cause key. |
| epithets | `String[]` | yes | Ruler epithets. |
| ruin_kinds | `BTreeMap<String, Vec<String>>` | yes | Ruin kinds per former tier. |
| sites | `BTreeMap<String, String>` | yes | Site phrase per site tag, in priority order of `site_order`. |
| site_order | `String[]` | yes | Site tags in the order they are preferred for prose. |
| site_default | `String` | yes | Phrase when no tag applies. |

### HookDef

| Field | Type | Required | Notes |
|---|---|---|---|
| variants | `[String; 2][]` | yes | `[title, text]` template pairs. |

### HookTable

| Field | Type | Required | Notes |
|---|---|---|---|
| hooks | `BTreeMap<String, HookDef>` | yes | Hook kinds. |
| hideouts | `String[]` | yes | Bandit hideouts when no ruin lies on the road. |

### CultureNames

| Field | Type | Required | Notes |
|---|---|---|---|
| onsets | `String[]` | yes | Opening consonant clusters. |
| nuclei | `String[]` | yes | Vowels. |
| codas | `String[]` | yes | Closing consonants. |
| given_m | `String[]` | yes | Male given-name endings. |
| given_f | `String[]` | yes | Female given-name endings. |
| house | `String[]` | yes | House-name endings. |
| place | `String[]` | yes | Place-name endings. |

### NameTable

| Field | Type | Required | Notes |
|---|---|---|---|
| default | `String` | yes | Fallback culture key. |
| cultures | `BTreeMap<String, CultureNames>` | yes | Syllables per culture key. |

### TraitEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| axis | `String` | yes | — |
| short | `String` | yes | — |
| text | `String` | yes | — |

### IdealEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| alignment | `String` | yes | — |
| tags | `String[]` | yes | — |
| short | `String` | yes | — |
| text | `String` | yes | — |

### BondEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| needs | `String` | yes | — |
| text | `String` | yes | — |

### FlawEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| short | `String` | yes | — |
| text | `String` | yes | — |

### PersonalityTables

| Field | Type | Required | Notes |
|---|---|---|---|
| traits | `TraitEntry[]` | yes | — |
| ideals | `IdealEntry[]` | yes | — |
| bonds | `BondEntry[]` | yes | — |
| flaws | `FlawEntry[]` | yes | — |
| mannerisms | `String[]` | yes | — |
| exclusions | `BTreeMap<String, Vec<String>>` | yes | — |
| affinities | `BTreeMap<String, Vec<String>>` | yes | — |

### JobData

| Field | Type | Required | Notes |
|---|---|---|---|
| title | `String` | yes | — |
| title_female | `Option<String>` | no | — |
| category | `String` | yes | — |
| rank | `u8` | yes | Social rank 0–5 (`SocialRank::from_level`). |
| rank_by_tier | `BTreeMap<String, u8>` | no | Rank where the tier differs from `rank` (a village smith is tradesfolk, a guild master in a town is a master).; serde default |
| stat_block | `String` | yes | — |
| classes | `BTreeMap<String, u32>` | yes | — |
| tools | `String[]` | yes | — |
| gear | `String[]` | yes | — |
| leader | `bool` | yes | — |

### HouseholdStyle

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: family, singles, family_with_staff |

### WorkplaceData

| Field | Type | Required | Notes |
|---|---|---|---|
| master | `Option<String>` | no | — |
| workers | `String[]` | yes | — |
| notable | `bool` | yes | — |
| importance | `u8` | yes | — |
| household | `HouseholdStyle` | yes | — |
| family_job | `Option<String>` | no | serde default |

### CraftData

| Field | Type | Required | Notes |
|---|---|---|---|
| noun | `String` | yes | — |
| tool | `String` | yes | — |

### LandJob

| Field | Type | Required | Notes |
|---|---|---|---|
| job | `String` | yes | — |
| scale | `String` | yes | — |
| base | `u32` | yes | — |
| functions | `BTreeMap<String, u32>` | yes | — |
| needs | `Option<String>` | no | serde default |

### OfficeData

| Field | Type | Required | Notes |
|---|---|---|---|
| rank | `u8` | yes | Lowest social rank of the holder, 0–5. |
| rank_by_tier | `BTreeMap<String, u8>` | no | Rank where the tier differs from `rank`.; serde default |
| civic | `bool` | no | Whether the office is civic (counts toward the tier's civic cap).; serde default |
| covers | `String[]` | no | Tier officials the office stands in for: they are not appointed again while the office is held.; serde default |

### Occupations

| Field | Type | Required | Notes |
|---|---|---|---|
| jobs | `BTreeMap<String, JobData>` | yes | — |
| workplaces | `BTreeMap<String, WorkplaceData>` | yes | — |
| crafts | `BTreeMap<String, CraftData>` | yes | — |
| land_jobs | `LandJob[]` | yes | — |
| offices | `BTreeMap<String, OfficeData>` | yes | Society offices by slot kind. |
| rank_lifestyles | `BTreeMap<SocialRank, [Lifestyle; 2]>` | yes | Lifestyle band `[lowest, highest]` per working social rank; dependants are not banded. |

### Culture

| Field | Type | Required | Notes |
|---|---|---|---|
| ancestry | `BTreeMap<String, u32>` | yes | — |

### Cultures

| Field | Type | Required | Notes |
|---|---|---|---|
| default | `String` | yes | — |
| cultures | `BTreeMap<String, Culture>` | yes | — |

### TierData

| Field | Type | Required | Notes |
|---|---|---|---|
| notables | `[u32; 2]` | yes | — |
| levels | `[u8; 2]` | yes | — |
| leader_levels | `[u8; 2]` | yes | — |
| rural_pm | `u32` | yes | — |
| urban_pm | `u32` | yes | — |
| officials | `String[]` | yes | — |
| promote | `String[]` | yes | — |
| civic_cap | `u32` | yes | Most civic figures (government jobs and civic offices) the tier appoints or promotes; society offices always bind. |

### LevelException

| Field | Type | Required | Notes |
|---|---|---|---|
| chance_per_mille | `u32` | yes | — |
| bonus_levels | `u8` | yes | — |
| max_level | `u8` | yes | — |

### Tiers

| Field | Type | Required | Notes |
|---|---|---|---|
| tiers | `BTreeMap<String, TierData>` | yes | — |
| capital_officials | `String[]` | yes | — |
| exception | `LevelException` | yes | — |
| household_sizes | `[u32; 2][]` | yes | — |

### BackgroundData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| text | `String` | yes | — |
| skills | `String[]` | yes | — |

### Backgrounds

| Field | Type | Required | Notes |
|---|---|---|---|
| backgrounds | `BTreeMap<String, BackgroundData>` | yes | — |

### RaceData

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | — |
| name | `String` | yes | — |
| subrace | `Option<String>` | no | — |
| size | `String` | yes | — |
| speed | `u8` | yes | — |
| darkvision | `u16` | yes | — |
| ability_bonuses | `[i8; 6]` | yes | — |
| flexible_bonuses | `u8` | yes | — |
| languages | `String[]` | yes | — |
| extra_languages | `u8` | yes | — |
| traits | `String[]` | yes | — |
| hp_per_level | `u8` | yes | — |
| skills | `String[]` | yes | — |
| skill_choices | `u8` | yes | — |
| resistances | `String[]` | yes | — |
| resistance_choices | `String[]` | yes | — |
| adult_age | `u16` | yes | — |
| max_age | `u16` | yes | — |
| working_age | `u16` | yes | — |
| elder_age | `u16` | yes | — |

### SubclassData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| level | `u8` | yes | — |
| features | `BTreeMap<u8, Vec<String>>` | yes | — |
| hp_per_level | `u8` | yes | — |
| unarmored_base | `Option<u8>` | no | — |

### Loadout

| Field | Type | Required | Notes |
|---|---|---|---|
| armor | `[Option<String>; 3]` | yes | — |
| shield | `bool` | yes | — |
| weapons | `String[]` | yes | — |

### ClassData

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | — |
| name | `String` | yes | — |
| hit_die | `u8` | yes | — |
| priority | `Ability[]` | yes | — |
| saves | `Ability[]` | yes | — |
| armor_training | `String[]` | yes | — |
| weapon_training | `String[]` | yes | — |
| skill_count | `u8` | yes | — |
| skills | `String[]` | yes | — |
| subclass | `SubclassData` | yes | — |
| features | `BTreeMap<u8, Vec<String>>` | yes | — |
| caster | `CasterKind` | yes | — |
| spell_ability | `Option<Ability>` | no | — |
| cantrips_known | `[u8; 3]` | yes | — |
| unarmored_bonus | `Option<Ability>` | no | — |
| asi_levels | `u8[]` | yes | — |
| loadout | `Loadout` | yes | — |

### PactRow

| Field | Type | Required | Notes |
|---|---|---|---|
| slots | `u8` | yes | — |
| level | `u8` | yes | — |

### SlotTables

| Field | Type | Required | Notes |
|---|---|---|---|
| full_caster_slots | `u8[][]` | yes | — |
| pact_slots | `PactRow[]` | yes | — |

### SpellData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| level | `u8` | yes | — |
| classes | `String[]` | yes | — |

### ExtraDamage

| Field | Type | Required | Notes |
|---|---|---|---|
| dice | `String` | yes | — |
| type | `String` | yes | Rust field `damage_type` |

### ActionData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| attack_bonus | `i8` | yes | — |
| damage | `String` | yes | — |
| damage_type | `String` | yes | — |
| extra_damage | `ExtraDamage[]` | yes | — |

### BlockSpellcasting

| Field | Type | Required | Notes |
|---|---|---|---|
| ability | `Ability` | yes | — |
| save_dc | `u8` | yes | — |
| attack_bonus | `i8` | yes | — |
| slots | `u8[]` | yes | — |
| spells | `(String, u8)[]` | yes | — |

### StatBlockData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| armor_class | `u8` | yes | — |
| armor | `String[]` | yes | — |
| hit_points | `u16` | yes | — |
| hit_dice | `String` | yes | — |
| speed | `u8` | yes | — |
| abilities | `[u8; 6]` | yes | — |
| saves | `BTreeMap<Ability, i8>` | yes | — |
| skills | `BTreeMap<String, i8>` | yes | — |
| passive_perception | `u8` | yes | — |
| languages | `String` | yes | — |
| challenge_rating | `String` | yes | — |
| xp | `u32` | yes | — |
| damage_resistances | `String[]` | yes | — |
| traits | `String[]` | yes | — |
| reactions | `String[]` | yes | — |
| multiattack | `bool` | yes | — |
| actions | `ActionData[]` | yes | — |
| spellcasting | `Option<BlockSpellcasting>` | no | — |
| equipment | `String[]` | yes | — |

### ArmorData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| category | `String` | yes | — |
| base_ac | `u8` | yes | — |
| dex_bonus | `bool` | yes | — |
| max_dex | `Option<u8>` | no | — |
| str_minimum | `u8` | yes | — |

### WeaponData

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | — |
| category | `String` | yes | — |
| range_kind | `String` | yes | — |
| damage_dice | `String` | yes | — |
| damage_type | `String` | yes | — |
| range | `Option<(u16, Option<u16>)>` | no | — |
| properties | `String[]` | yes | — |

### EquipmentData

| Field | Type | Required | Notes |
|---|---|---|---|
| armor | `ArmorData[]` | yes | — |
| weapons | `WeaponData[]` | yes | — |
| gear | `String[]` | yes | — |
| tools | `String[]` | yes | — |

### Languages

| Field | Type | Required | Notes |
|---|---|---|---|
| standard | `String[]` | yes | — |
| secret | `String[]` | yes | — |

### Tier (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### SettlementFunction (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |

### SettlementProfile (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `SettlementId` | yes | Settlement identifier; part of every seed key. |
| name | `String` | yes | Display name, used in personality bonds. |
| tier | `Tier` | yes | Size class. |
| population | `u32` | yes | Number of inhabitants to generate. |
| functions | `SettlementFunction[]` | yes | Economic functions; drive the land-job mix. |
| wealth | `u8` | yes | Overall wealth, 0 (destitute) to 255 (opulent). |
| culture | `String` | yes | Culture key for names and the ancestry mix (see `data/content/cultures.json`). |
| realm_id | `RealmId` | yes | Realm the settlement belongs to. |
| biome | `Biome` | yes | Biome, from the closed settlement list; informational. |
| coastal | `bool` | yes | On a sea coast. |
| riverine | `bool` | yes | On a river. |
| ancestry_mix | `Option<BTreeMap<String, u32>>` | no | Optional per-settlement ancestry weights (ancestry key → weight) that replace the culture's table.; serde default |
| tongue | `Option<arda_names::Tongue>` | no | The local speech `arda-settle` named the settlement in; its people are named in it too (A5).; serde default; omitted when empty/none |

### NotableSlot

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable role id (`r<settlement>.<kind>`). |
| kind | `String` | yes | Role kind (`ruler`, `lord`, `reeve`, `captain`, …). |
| title | `String` | yes | Title the holder is addressed by; replaces the job title. |
| building | `Option<BuildingId>` | no | Workplace building, if the role has one.; serde default |
| given_name | `Option<String>` | no | Given name fixed by history.; serde default |
| family_name | `Option<String>` | no | Family name fixed by history; the holder's household shares it.; serde default |
| female | `Option<bool>` | no | Sex fixed by history.; serde default |

### BuildingSpec (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BuildingId` | yes | Building identifier, unique within the settlement. |
| settlement_id | `SettlementId` | yes | Owning settlement; must equal the profile's id. |
| function | `BuildingFunction` | yes | What the building is for. |
| tags | `String[]` | no | Free tags such as `craft:weaving`.; serde default; omitted when empty/none |
| capacity | `u16` | yes | How many people can live here. |
| workplace_slots | `u16` | yes | How many people work here, master included. |
| wealth | `u8` | yes | Building wealth, 0–255. |

### HouseholdId (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| building | `BuildingId` | yes | Home building. |
| index | `u32` | yes | Index among the building's households. |

### Sex (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: female, male |

### Name (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| given | `String` | yes | Given name. |
| family | `String` | yes | Family name, shared within a family household. |
| byname | `Option<String>` | no | Optional nickname or epithet. |

### JobCategory (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: agriculture, craft, trade, service, religion, military, government, maritime, extraction, scholar, labour, nobility, dependent |

### Job (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | Occupation key in `occupations.json`, e.g. |
| title | `String` | yes | Display title, e.g. |
| category | `JobCategory` | yes | Broad category. |

### SocialRank (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: dependent, labourer, tradesfolk, master, gentry, noble |

### Lifestyle (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: wretched, squalid, poor, modest, comfortable, wealthy, aristocratic |

### Ideal (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Short name, e.g. |
| text | `String` | yes | The ideal in the person's words. |
| alignment | `String` | yes | Alignment lean, e.g. |
| tags | `String[]` | yes | Coherence tags such as `faith` or `wealth`. |

### Personality (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| traits | `String[]` | yes | Two traits on different axes. |
| ideal | `Ideal` | yes | One ideal, compatible with the job. |
| bond | `String` | yes | One bond, filled in with real people and places. |
| flaw | `String` | yes | One flaw. |
| mannerism | `String` | yes | A voice or mannerism hook. |
| summary | `String` | yes | One-line summary. |

### RelationKind (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: spouse, parent, child, sibling, landlord, lodger, employer, employee, friend, rival |

### Relationship (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `RelationKind` | yes | What the other person is to this one. |
| other | `NpcId` | yes | The other person. |

### Npc (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `NpcId` | yes | Stable id. |
| name | `Name` | yes | Name. |
| ancestry | `String` | yes | SRD race key, e.g. |
| ancestry_name | `String` | yes | SRD race display name. |
| subrace | `Option<String>` | no | SRD subrace, if the race has one. |
| age | `u16` | yes | Age in years. |
| sex | `Sex` | yes | Sex. |
| job | `Job` | yes | Occupation. |
| workplace_building | `Option<BuildingId>` | no | Where the person works, if at a building. |
| home_building | `BuildingId` | yes | Where the person lives. |
| household | `HouseholdId` | yes | Household. |
| employer | `Option<NpcId>` | no | The master of the workplace, for employees. |
| social_rank | `SocialRank` | yes | Social standing. |
| wealth | `u8` | yes | Wealth, 0–255. |
| lifestyle | `Lifestyle` | yes | SRD lifestyle matching the wealth. |
| personality | `Personality` | yes | Personality. |
| relationships | `Relationship[]` | yes | Family, work, friend and rival ties, sorted. |
| sheet | `Sheet` | yes | SRD 5.1 character sheet. |
| notable | `bool` | yes | Whether the person is a stored notable. |

### HouseholdRelation (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `NpcId` | yes | The person the relation belongs to. |
| kind | `RelationKind` | yes | What `to` is to `from`. |
| to | `NpcId` | yes | The other person. |

### Household (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `HouseholdId` | yes | Household id. |
| home | `BuildingId` | yes | Home building. |
| family_name | `String` | yes | Family name of the head. |
| members | `NpcId[]` | yes | Members, head first. |
| relations | `HouseholdRelation[]` | yes | Spouse, parent/child, sibling and lodger ties, both directions. |

### RosterEntry (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `NpcId` | yes | Person. |
| job | `u16` | yes | Index into `Population::job_keys`. |
| workplace | `Option<BuildingId>` | no | Workplace, if any. |
| notable | `bool` | yes | Whether the person is a stored notable. |

### Population (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement_id | `SettlementId` | yes | Settlement. |
| population | `u32` | yes | Total inhabitants. |
| households | `Household[]` | yes | Every household. |
| npcs | `Npc[]` | yes | Stored notables, full records. |
| commoner_count | `u32` | yes | Inhabitants who are not stored. |
| job_keys | `String[]` | yes | Job keys referenced by the roster. |
| job_categories | `JobCategory[]` | yes | Category of each job key, same order. |
| roster | `RosterEntry[]` | yes | One compact entry per inhabitant, in skeleton order (home building id, then resident index). |

### Ability (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: STR, DEX, CON, INT, WIS, CHA |

### CasterKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, full, half, pact |

### SheetKind (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| stat_block | `{name: String, challenge_rating: String, xp: u32}` | no | variant (internally tagged `type`); An SRD NPC stat block, used as printed. |
| class | `{class: String, subclass: Option<String>, level: u8, background: Background}` | no | variant (internally tagged `type`); A character built with SRD class levels. |

### Background (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Name, e.g. |
| text | `String` | yes | One-line description. |
| skills | `String[]` | yes | The two skills it grants. |

### SaveBonus (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| ability | `Ability` | yes | Ability. |
| bonus | `i8` | yes | Total bonus. |
| proficient | `bool` | yes | Whether proficiency applies. |

### SkillBonus (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD skill name. |
| ability | `Ability` | yes | Ability used. |
| bonus | `i8` | yes | Total bonus. |
| expertise | `bool` | yes | Whether proficiency is doubled. |

### AttackKind (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: melee, ranged |

### Attack (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Weapon or attack name. |
| kind | `AttackKind` | yes | Melee or ranged. |
| to_hit | `i8` | yes | Attack bonus. |
| damage | `String` | yes | Damage dice, e.g. |
| average | `i16` | yes | Average damage. |
| damage_type | `String` | yes | Damage type, e.g. |
| range | `String` | yes | Reach or range, e.g. |
| extra | `String[]` | yes | Extra effects, e.g. |

### Item (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD item name. |
| quantity | `u16` | yes | Quantity. |

### Coins (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| gp | `u32` | yes | Gold pieces. |
| sp | `u32` | yes | Silver pieces. |
| cp | `u32` | yes | Copper pieces. |

### SpellRef (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD spell name. |
| level | `u8` | yes | Spell level, 0 for cantrips. |

### Spellcasting (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| ability | `Ability` | yes | Casting ability. |
| save_dc | `u8` | yes | Spell save DC. |
| attack_bonus | `i8` | yes | Spell attack bonus. |
| slots | `u8[]` | yes | Slots per spell level (index 0 = 1st level). |
| pact_magic | `bool` | yes | Whether the slots are Pact Magic slots. |
| spells | `SpellRef[]` | yes | Cantrips and spells. |

### Sheet (arda-npc)

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `SheetKind` | yes | Stat block or class build. |
| size | `String` | yes | Size category. |
| abilities | `[u8; 6]` | yes | STR, DEX, CON, INT, WIS, CHA. |
| modifiers | `[i8; 6]` | yes | Their modifiers. |
| proficiency_bonus | `i8` | yes | Proficiency bonus. |
| saves | `SaveBonus[]` | yes | All six saving throws. |
| skills | `SkillBonus[]` | yes | Proficient skills. |
| hit_points | `u16` | yes | Hit points. |
| hit_dice | `String` | yes | Hit dice, e.g. |
| armor_class | `u8` | yes | Armour class. |
| armor | `String[]` | yes | Worn armour and shield. |
| speed | `u8` | yes | Walking speed in feet. |
| senses | `String[]` | yes | Senses, e.g. |
| passive_perception | `u8` | yes | Passive Wisdom (Perception). |
| languages | `String[]` | yes | Languages. |
| damage_resistances | `String[]` | yes | Damage resistances. |
| attacks | `Attack[]` | yes | Attacks. |
| features | `String[]` | yes | Class, race and stat-block features by name. |
| equipment | `Item[]` | yes | Equipment carried. |
| coins | `Coins` | yes | Coins carried. |
| spellcasting | `Option<Spellcasting>` | no | Spellcasting, for casters. |

### BuildReport

| Field | Type | Required | Notes |
|---|---|---|---|
| settlements | `usize` | yes | Settlements. |
| planned | `usize` | yes | Settlements with a town plan. |
| notables | `usize` | yes | Stored notables. |
| population | `u64` | yes | Inhabitants (stored and regenerated). |

### StoredNotable

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement_id | `arda_ids::SettlementId` | yes | The settlement. |
| buildings | `String` | yes | Where its buildings came from: `plan` (arda-town) or `mix` (the settlement's estimated building mix). |
| npcs | `Npc[]` | yes | The notables, in id order. |

### NotablesFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Format version. |
| seed | `String` | yes | World seed, a decimal string. |
| settlements | `StoredNotable[]` | yes | Per settlement, in settlement order. |

### SocietyFiles

| Field | Type | Required | Notes |
|---|---|---|---|
| dir | `PathBuf` | yes | The directory. |
| settlements | `SettlementsFile` | yes | `settlements.json`. |
| records | `serde_json::Value[]` | yes | The same records as raw JSON, for the crates that read their own view of a record (`arda-town` `TownSite`, `arda-npc` `SettlementProfile`; adapter A2). |
| roads | `RoadsFile` | yes | `roads.json`. |
| index | `BTreeMap<u64, usize>` | yes | Record index by settlement id. |

### BuildingSource

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Plan, Mix |

### World (arda-people)

| Field | Type | Required | Notes |
|---|---|---|---|
| src | `SharedSource` | yes | The world's cells, shared with the tactical pipeline. |
| files | `SocietyFiles` | yes | The settlement stage's files. |
| society | `Option<Society>` | no | `society.json`, when `arda society build` has run. |
| notables | `Option<NotablesFile>` | no | `notables.json`, when `arda society build` has run. |
| notable_index | `BTreeMap<NpcId, (usize, usize)>` | yes | — |
| society_index | `BTreeMap<u64, usize>` | yes | — |
| plans | `Mutex<(u64, PlanCache)>` | yes | — |

### Surroundings

| Field | Type | Required | Notes |
|---|---|---|---|
| origin | `(i64, i64)` | yes | First cell column and row. |
| size | `(i64, i64)` | yes | Cells across and down. |
| height_m | `f64[]` | yes | Ground or water-surface height per cell, metres. |
| open_water | `bool[]` | yes | Open water (sea or lake) per cell. |
| edges | `Edge[]` | yes | River channels touching the cells, sorted and deduplicated. |
| pieces | `RiverPiece[]` | yes | The channels as straight pieces between cell centres. |
| bins | `u32[][]` | yes | Per cell, the pieces whose channel (half its width, at least 1 m) reaches into it. |

### FallbackKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ground, kit, prop, query |

### Fallback

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `FallbackKind` | yes | Kind. |
| wanted | `String` | yes | Key the layout asked for. |
| used | `String` | yes | Key used instead (`-` when omitted). |
| count | `u32` | yes | How often. |

### Window

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `i64` | yes | First column (global square). |
| y | `i64` | yes | First row (global square). |
| w | `i64` | yes | Width in squares. |
| h | `i64` | yes | Height in squares. |

### BlockBuilding

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BuildingId` | yes | Plan id (the same object as in the plan and `BuildingSpec`). |
| function | `BuildingFunction` | yes | Function. |
| rect | `SquareRect` | yes | Whole footprint, global squares. |
| local | `[i64; 4]` | yes | The part inside this block, local squares `[x0, y0, x1, y1)`. |
| doors | `Door[]` | yes | Doors, global squares. |
| rooms | `(String, SquareRect)[]` | yes | Rooms `(kind, global rectangle)`. |

### SquareRules

| Field | Type | Required | Notes |
|---|---|---|---|
| difficult | `bool` | yes | Difficult terrain (shallow water, mud). |
| water_depth_ft | `u8` | yes | Water depth, feet. |
| cover | `Cover` | yes | Cover level (`none` here; walls are edges). |
| blocks_sight | `bool` | yes | Blocks sight. |
| blocks_movement | `bool` | yes | Blocks movement. |
| deck | `bool` | yes | A deck over water (bridge or dock). |

### Cover (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total |

### TownBlock

| Field | Type | Required | Notes |
|---|---|---|---|
| site | `SettlementId` | yes | Settlement id. |
| window | `Window` | yes | The window, global squares. |
| origin | `[i64; 2]` | yes | World-square origin of layout square (0, 0) (convention I10). |
| layout | `TacticalLayout` | yes | The layout with canonical vocabulary keys. |
| buildings | `BlockBuilding[]` | yes | Buildings touching the window. |
| rules | `SquareRules[]` | yes | Per-square rules, row-major. |
| owned | `bool[]` | yes | Row-major: whether the town reserves the square (every plan kind but open country and water; logic/09 §reservations precedence 1). |
| light_owner | `Option<usize>[]` | no | For each light, the placement that emits it (dropped when the resolved asset carries its own light).; not serialized |
| relaxed | `Relaxed[]` | no | WFC problems in or around the window that used the relaxed fill (goal 47: marked for review).; serde default; omitted when empty/none |

### TownFill

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: wfc, rules |

### Relaxed

| Field | Type | Required | Notes |
|---|---|---|---|
| interior | `{building: u64}` | no | variant (internally tagged `kind`); A building interior (plan id). |
| outdoor | `{cx: i64, cy: i64}` | no | variant (internally tagged `kind`); An outdoor chunk (global chunk coordinates). |

### Report

| Field | Type | Required | Notes |
|---|---|---|---|
| interiors | `usize` | yes | Interiors solved by WFC. |
| interiors_relaxed | `usize` | yes | Interiors that used the relaxed fill. |
| relaxed_by_function | `std::collections::BTreeMap<String, usize>` | yes | Relaxed interiors by building function key. |
| chunks | `usize` | yes | Outdoor chunks solved by WFC. |
| chunks_relaxed | `usize` | yes | Outdoor chunks that used the relaxed fill. |

### BuildingFunction (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: house, farmhouse, cottage, manor, inn, tavern, bakery, brewery, mill, smithy, workshop, tannery, apothecary, temple, shrine, library, school, warehouse, market_hall, stall, dock, boathouse, keep, barracks, guardhouse, stable, barn, mine, lumber_camp |

### Vec2

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f64` | yes | Metres east. |
| y | `f64` | yes | Metres south. |

### Side

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: north, east, south, west |

### SquareRect

| Field | Type | Required | Notes |
|---|---|---|---|
| x0 | `i64` | yes | First column. |
| y0 | `i64` | yes | First row. |
| x1 | `i64` | yes | One past the last column. |
| y1 | `i64` | yes | One past the last row. |

### BuildingSpec (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BuildingId` | yes | Building id (same as the plan and the tactical sidecar). |
| settlement_id | `SettlementId` | yes | Owning settlement. |
| function | `F` | yes | Function, a plain snake_case key. |
| capacity | `u16` | yes | How many people can live here. |
| workplace_slots | `u16` | yes | How many people work here, master included. |
| wealth | `u8` | yes | Building wealth, 0–255. |
| tags | `String[]` | no | Free `key:value` tags (`craft:*` on workshops).; serde default; omitted when empty/none |

### StreetId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u16` | yes | — |

### PlotId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u32` | yes | — |

### BuildingId (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | — |

### StreetClass

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: main, lane, alley, intramural |

### Street

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `StreetId` | yes | Index. |
| class | `StreetClass` | yes | Class. |
| points | `Vec2[]` | yes | Centreline in world metres, starting at its junction end. |
| width_m | `f64` | yes | Carriageway width in metres. |
| paved | `bool` | yes | Cobbled rather than dirt. |

### FocalKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: crossing, crossroads, harbour, castle, roadside |

### Focal

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `FocalKind` | yes | Kind. |
| feature | `Vec2` | yes | The feature itself (bridge, junction, quay or keep). |
| market | `Vec2` | yes | The market square centre. |

### MarketSquare

| Field | Type | Required | Notes |
|---|---|---|---|
| polygon | `Vec2[]` | yes | Outline in world metres. |
| green | `bool` | yes | A grass green rather than a paved square. |

### DistrictKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: market, residential, craft, waterfront, religious, castle, suburb, farmstead |

### Plot

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `PlotId` | yes | Index. |
| street | `Option<StreetId>` | no | The street it fronts; `None` for the market square. |
| front | `Side` | yes | Direction the frontage faces (towards the street). |
| polygon | `Vec2[]` | yes | Outline in world metres (rectilinear). |
| frontage_m | `f64` | yes | Frontage width in metres. |
| depth_m | `f64` | yes | Mean depth in metres. |
| district | `DistrictKind` | yes | District. |
| inside_wall | `bool` | yes | Inside the town wall (always true for unwalled settlements). |

### Gate

| Field | Type | Required | Notes |
|---|---|---|---|
| point | `Vec2` | yes | Centre in world metres. |
| street | `Option<StreetId>` | no | The street it carries, or `None` for a water gate. |

### Bridge

| Field | Type | Required | Notes |
|---|---|---|---|
| street | `StreetId` | yes | The street it carries. |
| along_x | `bool` | yes | Rows run west–east (`true`) or north–south. |
| rows | `(i64, i64, i64)[]` | yes | Rows as `(across, from, to)` in global squares, `from..=to` along the row: a row is square row `across` when `along_x`, else column. |

### TownWall

| Field | Type | Required | Notes |
|---|---|---|---|
| ring | `Vec2[]` | yes | Closed centreline ring in world metres. |
| gates | `Gate[]` | yes | Gates, including water gates. |
| thickness_m | `f64` | yes | Thickness in metres. |

### CastleWard

| Field | Type | Required | Notes |
|---|---|---|---|
| bailey | `SquareRect` | yes | Inner bailey, global squares. |
| gate | `Side` | yes | Side of the ward gate. |

### WealthLevel

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: poor, modest, wealthy |

### Door

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `i64` | yes | Square column inside the building. |
| y | `i64` | yes | Square row inside the building. |
| side | `Side` | yes | Which edge of that square. |

### Building

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BuildingId` | yes | Index. |
| function | `BuildingFunction` | yes | Function (vocabulary key). |
| plot | `Option<PlotId>` | no | The plot it stands on, if any. |
| footprint | `Vec2[]` | yes | Footprint polygon in world metres. |
| rect | `SquareRect` | yes | Footprint in global squares (the same rectangle). |
| storeys | `u8` | yes | Storeys above ground. |
| wealth | `u8` | yes | Wealth 0–255. |
| wealth_level | `WealthLevel` | yes | Wealth band. |
| front | `Side` | yes | Side facing the street. |
| doors | `Door[]` | yes | Doors; the first faces the street. |
| ancillary | `bool` | yes | Not in the building mix (barns behind farmhouses, jetties). |
| tags | `String[]` | no | Free `key:value` tags, e.g.; serde default; omitted when empty/none |

### TownPlan

| Field | Type | Required | Notes |
|---|---|---|---|
| site | `SettlementId` | yes | Settlement id. |
| name | `String` | yes | Settlement name. |
| tier | `Tier` | yes | Tier. |
| culture | `String` | yes | Culture key. |
| seed | `u64` | yes | Plan seed (world seed mixed with the site id), as a JSON string.; serde with `crate::ids` |
| base_height_m | `f64` | yes | Ground height at the market, metres above sea level; grid heights are relative to it. |
| water_level_m | `f64` | yes | Water surface level within the plan, metres above sea level. |
| focal | `Focal` | yes | Focal point. |
| streets | `Street[]` | yes | Streets. |
| bridges | `Bridge[]` | no | Bridges carrying streets over rivers.; serde default |
| square | `Option<MarketSquare>` | no | Market square or green. |
| plots | `Plot[]` | yes | Plots. |
| districts | `(DistrictKind, Vec<PlotId>)[]` | yes | Districts and their plots. |
| wall | `Option<TownWall>` | no | Town wall, for walled tiers. |
| castle | `Option<CastleWard>` | no | Castle ward, for fortress sites. |
| buildings | `Building[]` | yes | Buildings. |
| notes | `String[]` | yes | Planner notes: aliases used, off-plan and unplaced mix entries. |
| grid | `PlanGrid` | no | The square-resolution index (not serialised).; not serialized |
| interior_salts | `std::sync::OnceLock<Vec<u8>>` | no | Interior salts by building position, drawn on first use (`crate::block::interior::variety::salts`; not serialised).; not serialized |
| fabric | `Fabric` | no | What the settlement builds with: the facts the wall-kit and floor table reads (logic/10 §building-materials).; serde default |
| looks | `std::sync::OnceLock<Vec<crate::block::kits::Look>>` | no | Each building's wall kits and floor, by position, drawn on first use (`crate::block::kits::looks`; not serialised).; not serialized |

### Fabric

| Field | Type | Required | Notes |
|---|---|---|---|
| wealth | `u8` | yes | Settlement wealth, 0–255. |
| merchant | `bool` | yes | A market or port: merchants set the tone. |
| biome | `String` | yes | Biome key. |
| forest | `bool` | yes | Woodland at hand (`forest` or `timber` site tags, logging, forest biomes). |
| stone | `bool` | yes | Building stone at hand (`hill`, `mountain` or `quarry` site tags, mining, highland and alpine biomes). |
| fires | `u8` | no | Great fires in the settlement's history (`arda-society`).; serde default |
| golden_age | `bool` | no | Prosperous through most of its history (`arda-society`).; serde default |

### SettlementId (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u64` | yes | — |

### Tier (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### SettlementFunction (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |

### TownSite

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `SettlementId` | yes | Stable identifier. |
| name | `String` | yes | Display name. |
| tier | `Tier` | yes | Size class. |
| population | `u32` | yes | Inhabitants. |
| functions | `SettlementFunction[]` | yes | Economic functions. |
| wealth | `u8` | yes | Overall wealth, 0–255. |
| culture | `String` | yes | Culture key (`heartland`, `highland`, `sylvan`, `coastal`, `southern`, `borderland`; others fall back by hash). |
| realm_id | `u64` | no | Owning realm; a JSON string as `arda-settle` writes it (I5), numbers are also read.; serde default; serde with `crate::ids` |
| biome | `String` | no | Biome name.; serde default |
| coastal | `bool` | no | On a sea coast.; serde default |
| riverine | `bool` | no | On a river.; serde default |
| x_m | `i64` | yes | Centre, metres east of the world's west edge. |
| y_m | `i64` | yes | Centre, metres south of the world's north edge. |
| site_tags | `String[]` | no | Descriptive site tags (`ford`, `bridge_site`, `harbour`, `defensible`, `hill`, …).; serde default |
| buildings | `BTreeMap<String, u32>` | no | Estimated building mix: building-function key → count.; serde default |

### RoadClass (arda-town)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, track, road, highway, footpath |

### EnteringRoad

| Field | Type | Required | Notes |
|---|---|---|---|
| class | `RoadClass` | yes | Road class. |
| points | `Vec2[]` | yes | Polyline in world metres; it may be coarse (100 m cell centres). |

### RiverLine

| Field | Type | Required | Notes |
|---|---|---|---|
| points | `Vec2[]` | yes | Centreline, upstream first. |
| width_m | `f64` | yes | Channel width in metres. |

### Dir

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: N, E, S, W |

### Rules

| Field | Type | Required | Notes |
|---|---|---|---|
| tiles | `usize` | yes | — |
| kinds | `usize` | yes | — |
| allow | `TileSet[]` | yes | — |
| edge_ok | `TileSet[]` | yes | — |

### TileSet

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `[u64; WORDS]` | yes | — |

### Anchor (arda-wfc)

| Field | Type | Required | Notes |
|---|---|---|---|
| options | `(usize, TileSet)[]` | yes | `(cell, tiles)` pairs; a chosen cell takes one of its tiles. |
| count | `u32` | yes | How many options must hold. |

### Limit

| Field | Type | Required | Notes |
|---|---|---|---|
| tiles | `TileSet` | yes | The tile group. |
| cells | `usize[]` | yes | The scope; empty means the whole grid. |
| max | `u32` | yes | The cap. |

### Problem

| Field | Type | Required | Notes |
|---|---|---|---|
| rules | `&'a Rules` | yes | The adjacency rules. |
| w | `usize` | yes | Width. |
| h | `usize` | yes | Height. |
| domains | `TileSet[]` | yes | Initial options per cell, row-major; a single tile fixes a cell. |
| edges | `[u8; 4][]` | yes | Edge kind per cell and side (`Dir::index`), including the grid's outer sides. |
| weight | `Weight<'a>` | yes | Choice weights. |
| anchors | `Anchor[]` | yes | Anchors, placed in order. |
| limits | `Limit[]` | yes | Caps. |
| seed | `u64` | yes | World seed. |
| key | `[i64; 3]` | yes | Stable key of the problem (e.g. |

### Solution

| Field | Type | Required | Notes |
|---|---|---|---|
| tiles | `u16[]` | yes | Tile per cell, row-major. |
| attempts | `u8` | yes | Attempts used (1-based). |

### Failure

| Field | Type | Required | Notes |
|---|---|---|---|
| attempts | `u8` | yes | Attempts made. |
| rejected | `u8` | yes | Attempts that finished but failed the caller's check (the rest met a contradiction). |

### Block (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| cell | `CellKey` | yes | The block's cell. |
| origin | `(i64, i64)` | yes | Global square of the block's top-left square. |
| ground | `&'static str[]` | yes | Ground key per square, row-major. |
| elevation_ft | `i16[]` | yes | Elevation in feet on 5-ft steps. |
| depth_ft | `u8[]` | yes | Water depth in feet. |
| dryness | `u8[]` | yes | Climate dryness per square, `0..=255` (`crate::biome::arid` interpolated between cell centres): the compositor's dry-grass tint. |
| fixed | `Fixed[]` | yes | Pre-WFC decision per square. |
| corners | `Class[]` | yes | Corner classes, 65 × 65 row-major. |
| items | `Item[]` | yes | Placements anchored inside the block, global coordinates. |
| halo | `Grid<(i16, u8)>` | yes | Elevation and depth for the one-square ring around the block (squares -1..=64), as the neighbours will compute them. |
| rules | `SquareRules[]` | yes | Rules per square. |
| relaxed | `bool` | yes | Whether the WFC or a border fell back to the relaxed fill. |
| attempts | `u8` | yes | WFC attempts used. |
| repairs | `u32` | yes | Local WFC repairs. |

### Class (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Water=0, Grass=1, Meadow=2, ForestFloor=3, LeafLitter=4, Heath=5, Scrub=6, Moss=7, Scree=8, Rock=9, Cliff=10, Sand=11, Gravel=12, Mud=13, Marsh=14, ReedBed=15, Snow=16, Ice=17, Dirt=18 |

### Map

| Field | Type | Required | Notes |
|---|---|---|---|
| layout | `TacticalLayout` | yes | The layout in the `arda-tactical` schema. |
| rules | `RulesSidecar` | yes | The SRD rules sidecar (`arda-scene` field names). |
| meta | `MapMeta` | yes | Origin, review flags, block records and placement tags. |

### WaterRegion

| Field | Type | Required | Notes |
|---|---|---|---|
| ctx | `Ctx` | yes | — |
| pieces | `Piece[]` | yes | — |
| bins | `HashMap<(i64, i64), Vec<(u32, u32)>>` | yes | — |
| style | `RiverStyle` | yes | — |

### PlacementMeta

| Field | Type | Required | Notes |
|---|---|---|---|
| tag | `String` | yes | Semantic tag such as `tree:broadleaf:large`. |
| cover | `CoverRule` | yes | Cover the object grants. |
| difficult | `bool` | yes | Whether it makes its square difficult terrain. |

### BlockMeta

| Field | Type | Required | Notes |
|---|---|---|---|
| cell | `[i64; 2]` | yes | Global cell. |
| attempts | `u8` | yes | WFC attempts used. |
| repairs | `u32` | yes | Local WFC repairs. |
| relaxed | `bool` | yes | Whether the relaxed fill was used (goal 47: review this block). |

### MapMeta

| Field | Type | Required | Notes |
|---|---|---|---|
| format | `String` | yes | Always `META_FORMAT`. |
| name | `String` | yes | Matches the layout name. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| relaxed | `bool` | yes | Whether any block needs review. |
| review | `u32[]` | yes | Row-major indices of squares whose tile came from a relaxed fill. |
| blocks | `BlockMeta[]` | yes | Per-block generation records. |
| placements | `PlacementMeta[]` | yes | Placement metadata, parallel to the layout's placements. |

### SquareRules (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| difficult | `bool` | yes | Difficult terrain. |
| water_depth_ft | `u8` | yes | Water depth, feet. |
| cover | `CoverRule` | yes | Cover. |
| blocks_sight | `bool` | yes | Heavily obscured by canopy. |
| lightly_obscured | `bool` | yes | Lightly obscured: a thinner canopy or reeds. |
| blocks_movement | `bool` | yes | Impassable. |
| review | `bool` | yes | Relaxed-fill square needing review. |

### Kind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: TreeLarge, TreeSmall, Undergrowth, Boulder, Rock, Log, Reeds, Lilies, Low, Outcrop |

### Item (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f64` | yes | Global x, squares. |
| y | `f64` | yes | Global y, squares. |
| kind | `Kind` | yes | What it is. |
| asset | `&'static str` | yes | Vocabulary asset id, e.g. |
| tag | `&'static str` | yes | Semantic tag, e.g. |
| rotation | `u16` | yes | Rotation, degrees clockwise. |
| mirror | `bool` | yes | Mirror flag. |

### CellKey

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `i64` | yes | Column. |
| y | `i64` | yes | Row. |

### Edge (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `CellKey` | yes | Upstream cell. |
| to | `CellKey` | yes | Downstream cell. |
| from_width_dm | `u32` | yes | Channel width at the upstream end, decimetres. |
| to_width_dm | `u32` | yes | Channel width at the downstream end, decimetres. |
| discharge_milli | `u64` | yes | Mean discharge, thousandths of a cubic metre per second. |

### LakeInfo

| Field | Type | Required | Notes |
|---|---|---|---|
| surface_mm | `i32` | yes | Water surface, millimetres above sea level. |
| depth_mm | `u32` | yes | Greatest depth, millimetres. |

### Water

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Dry, River, Lake, Sea, Pool |

### TileId (arda-refine)

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u16` | yes | — |

### WorldSource

| Field | Type | Required | Notes |
|---|---|---|---|
| world | `WorldRef<'w>` | yes | — |
| wide | `i64` | yes | — |
| high | `i64` | yes | — |
| index | `Mutex<BTreeMap<(i32, i32), Arc<AreaIndex>>>` | yes | — |
| fine | `Mutex<Option<Fine>>` | yes | — |

### FallbackKind (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ground, kit, asset |

### Fallback (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `FallbackKind` | yes | Kind of key. |
| wanted | `String` | yes | The key the layout asked for. |
| used | `String` | yes | The key drawn instead, or `(dropped)`. |
| count | `u32` | yes | How many squares, segments or placements were affected. |

### RoadClass (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none=0, track=1, road=2, highway=3, footpath=4 |

### Raw

| Field | Type | Required | Notes |
|---|---|---|---|
| Num | `(u64)` | no | variant (untagged) |
| Str | `(String)` | no | variant (untagged) |

### Road (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id in build order (u64, as `arda-settle` writes it).; serde with `id_serde` |
| class | `RoadClass` | yes | Class. |
| segments | `[i64; 2][][]` | yes | Stretches as polylines of `[x_m, y_m]` world metres (x east, y south). |
| wealth | `u8` | no | Wealth of the land the road serves, 0–255 (not in `roads.json`; the caller fills it from the settlements, default 128).; serde default |

### CrossingKind (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: bridge, ford, ferry |

### Crossing (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | 1-based id (u64, as `arda-settle` writes it).; serde with `id_serde` |
| kind | `CrossingKind` | yes | Bridge, ford or ferry. |
| water | `String` | no | What is crossed: `river`, `sea` or `lake`.; serde default |
| x_m | `i64` | yes | Position, metres east. |
| y_m | `i64` | yes | Position, metres south. |
| width_m | `u32` | yes | Channel width in metres (0 for open water). |
| order | `u8` | no | Strahler order (0 for open water).; serde default |
| road_class | `RoadClass` | yes | Highest road class using the crossing. |
| river | `String` | no | Name of the river.; serde default |

### RiverChannel

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable id (for reports). |
| centreline | `[f64; 2][]` | yes | Centreline in world metres, upstream first. |
| width_m | `f64` | yes | Bank-to-bank width, metres. |
| depth_m | `f64` | yes | Depth at the centreline, metres. |

### CrossingReport

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Crossing id (a JSON string).; serde with `input::id_serde` |
| kind | `CrossingKind` | yes | Kind. |
| stone | `bool` | yes | Stone arch (bridges only). |
| axis | `&'static str` | yes | Span axis: `east_west` or `north_south`. |
| water_squares | `i64` | yes | Squares of water crossed along the axis. |
| span_squares | `i64` | yes | Squares of the whole structure along the axis. |
| width_squares | `i64` | yes | Width across, in squares. |
| deck_ft | `i16` | yes | Deck level in feet (bridges). |

### WaysReport

| Field | Type | Required | Notes |
|---|---|---|---|
| ways | `u32` | yes | Ways (road segments) that reach the window. |
| crossings | `CrossingReport[]` | yes | Crossings resolved near the window. |
| orphans | `String[]` | yes | Crossing ids near the window that met no way or no water. |
| houses | `(String, &'static str)[]` | yes | Toll houses and waystations `(crossing id, function)`. |
| switchbacks | `u32` | yes | Pieces given switchbacks, on ways that reach the window. |
| over_grade | `u32` | yes | Pieces still over their class grade, on ways that reach the window. |
| junctions | `u32` | yes | Junctions near the window. |

### Feature

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, road, ruts, verge, ditch, shoulder, cut_face, bank, bridge, abutment, ford, gravel_bar, landing, ferry_rope, building |

### Cover (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total (alias full) |

### SquareRules (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| difficult | `bool` | yes | SRD difficult terrain: fords, ditches, cut faces and any undecked water shallower than `crate::input::SWIM_FT` (wading). |
| water_depth_ft | `u8` | yes | Water depth in feet (under a deck, the water below it). |
| cover | `Cover` | yes | Cover granted by the square itself. |
| blocks_sight | `bool` | yes | The square blocks line of sight. |
| blocks_movement | `bool` | yes | The square blocks movement. |
| deck | `bool` | yes | A walkable deck over the square's water (bridges, landing stages): creatures stand at `deck_elevation_ft`, not in the water below. |
| deck_elevation_ft | `Option<i16>` | no | Deck surface elevation in feet (absolute, 5-ft steps), when `deck`.; serde default; omitted when empty/none |
| feature | `Feature` | no | The square's role for this stage.; serde default; omitted when empty/none |
| road_class | `Option<RoadClass>` | no | Road class of the surface, verge or crossing.; serde default; omitted when empty/none |

### EdgeRole (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: parapet, retaining, building |

### EdgeRule (arda-ways)

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column (as in `WallSegment`). |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxis` | yes | Which edge. |
| role | `EdgeRole` | yes | Why it is there. |
| blocks_movement | `bool` | yes | Blocks movement across the edge. |
| blocks_sight | `bool` | yes | Blocks line of sight across the edge. |

### Sidecar

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | `SIDECAR_VERSION`. |
| layout | `String` | yes | Layout name. |
| width | `u32` | yes | Width in squares (equals the layout's). |
| height | `u32` | yes | Height in squares (equals the layout's). |
| squares | `SquareRules[]` | yes | Row-major rules, `width × height`. |
| edges | `EdgeRule[]` | yes | Rules for the wall segments this stage added. |

### CompoundKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farmstead, mill, mine, quarry |

### Fallback (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `&'static str` | yes | `ground`, `kit`, `role` or `asset`. |
| wanted | `String` | yes | What the layout asked for. |
| used | `Option<String>` | no | What was used instead; `None` when the item was dropped. |
| count | `usize` | yes | How many squares, segments or placements it affected. |

### FieldKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: strips, arable, pasture, meadow, fallow, orchard, woodland, wild, common |

### Crop

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ploughed, stubble, fallow, hay, mown, grazed, none |

### Field

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u64` | yes | Stable id from the site and the parcel's first square. |
| kind | `FieldKind` | yes | Use. |
| crop | `Crop` | yes | Crop (per strip for `FieldKind::Strips`). |
| kit | `&'static str` | yes | Boundary wall kit, for enclosed fields. |
| squares | `u32` | yes | Area in squares. |
| first | `Sq` | no | First square in row-major order.; not serialized |
| sum | `[f64; 2]` | no | Sum of square centres, for the centroid.; not serialized |
| site | `usize` | no | Index of the generating site.; not serialized |

### LandUse (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: arable, pasture, orchard, woodland, meadow, fallow, mill, mine_quarry, farmstead |

### LandUseGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| origin | `(i64, i64)` | yes | Cell coordinates of the raster's north-west cell. |
| width | `u32` | yes | Width in cells. |
| height | `u32` | yes | Height in cells. |
| cells | `Option<LandUse>[]` | yes | Row-major classes. |
| outside | `Option<LandUse>` | no | Class reported outside the raster. |

### Region (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: lowland, upland, mixed |

### Tier (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### Settlement (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| x_m | `f64` | yes | Centre, world metres east. |
| y_m | `f64` | yes | Centre, world metres south. |
| tier | `Tier` | yes | Size tier. |
| population | `u32` | yes | Inhabitants. |

### RoadClass (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none=0, track=1, road=2, highway=3, footpath=4 |

### Road (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| class | `RoadClass` | yes | Class. |
| points | `[f64; 2][]` | yes | Vertices in world metres. |

### RiverLine (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| a | `[f64; 2]` | yes | One end. |
| b | `[f64; 2]` | yes | The other end. |
| width_m | `f64` | yes | Channel width, metres. |

### SrdCover

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total |

### Sidecar (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Schema version, `SIDECAR_VERSION`. |
| name | `String` | yes | Layout name. |
| seed | `String` | yes | Generation seed, as a string (I17). |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| origin_square | `[i64; 2]` | yes | Global square of the window's north-west corner. |
| square_m | `f64` | yes | Metres per square. |
| squares | `SquareRules[]` | yes | Row-major square rules. |
| edges | `EdgeRules[]` | yes | Wall, fence, hedge and gate rules, in layout edge coordinates. |
| fields | `FieldRecord[]` | yes | Fields touching the window. |
| compounds | `CompoundRecord[]` | yes | Compounds touching the window. |

### SquareRules (arda-fields)

| Field | Type | Required | Notes |
|---|---|---|---|
| ground | `String` | yes | Canonical ground key (before any library fallback). |
| difficult | `bool` | yes | Difficult terrain (ploughland, scrub, scree, mud, shallow water, bushes). |
| water_depth_ft | `u8` | yes | Water depth in feet; under 5 is shallow (wade), 5 or more deep (swim). |
| cover | `SrdCover` | yes | Cover from what stands on the square. |
| blocks_sight | `bool` | yes | Something on the square blocks sight. |
| blocks_movement | `bool` | yes | Something on the square blocks movement. |
| deck | `bool` | yes | A walkable deck over water (bridges, docks); none in the countryside. |
| crop | `Option<Crop>` | no | Crop, where farmed.; omitted when empty/none |
| furrow | `Option<[f32; 2]>` | no | Furrow direction (unit vector, y south), where ploughed.; omitted when empty/none |
| field | `Option<String>` | no | Field id (a u64 as a string), where in a field.; omitted when empty/none |

### EdgeRules

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column (layout coordinates). |
| y | `u32` | yes | Square row (layout coordinates). |
| axis | `EdgeAxis` | yes | Which edge. |
| kind | `WallRole` | yes | Role. |
| kit | `String` | yes | Kit. |
| blocks_sight | `bool` | yes | Blocks line of sight. |
| blocks_movement | `bool` | yes | Blocks movement. |
| cover | `SrdCover` | yes | Cover to a creature behind it. |
| climb | `bool` | yes | Crossing it costs climbing movement (low walls, hurdles). |
| openable | `bool` | yes | A door or gate that can be opened. |

### FieldRecord

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Stable id (a u64 as a string). |
| kind | `FieldKind` | yes | Use. |
| crop | `Crop` | yes | Crop. |
| kit | `String` | yes | Boundary kit (empty for open land). |
| hectares | `f64` | yes | Area in hectares. |
| enclosed | `bool` | yes | Whether it is enclosed. |
| gates | `usize` | yes | Number of gate edges. |
| complete | `bool` | yes | Whether it lies wholly inside the window. |

### CompoundRecord

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `CompoundKind` | yes | Kind. |
| cell | `[i64; 2]` | yes | Land-use cell. |
| lane | `bool` | yes | Whether a lane joins it to a road. |

### DemoOverlays

| Field | Type | Required | Notes |
|---|---|---|---|
| anchor | `[i64; 2]` | yes | — |
| town | `Mutex<Option<(u64, Arc<TownPlan>)>>` | yes | — |

### Owner

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Natural, Verge, Fields, Croft, Water, Ways, Town |

### OverlayCtx

| Field | Type | Required | Notes |
|---|---|---|---|
| gsx0 | `i64` | yes | First square column (world squares, 64 per cell). |
| gsy0 | `i64` | yes | First square row. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| seed | `u64` | yes | World seed. |
| base | `&'a TacticalLayout` | yes | The refined layout of the window, before any overlay. |
| library | `&'a Library` | yes | The asset library; layers degrade missing keys against it. |

### OverlayLayer

| Field | Type | Required | Notes |
|---|---|---|---|
| layout | `TacticalLayout` | yes | The layer's layout of the whole window (same size as the base). |
| rules | `RulesSidecar` | yes | Its rules, format 2, same size. |
| owned | `bool[]` | yes | Row-major: the squares it reserves. |
| soft | `bool[]` | yes | Row-major: owned squares claimed only at the layer's soft precedence (`Owner::soft`); empty for none. |
| elevation | `bool` | yes | Whether claimed squares take the layer's elevation (a layer built on the world's terrain) or keep the base's (a layer built on synthetic terrain, such as the demo samples). |
| review | `bool[]` | yes | Row-major: squares filled by a relaxed fill and marked for review (goal 47; the town WFC's `TownBlock::relaxed`); empty for none. |

### Composed

| Field | Type | Required | Notes |
|---|---|---|---|
| layout | `TacticalLayout` | yes | The layout, with `origin` set to the window's world square (I10). |
| rules | `RulesSidecar` | yes | The merged rules, `arda-scene` format 2 (I9). |
| meta | `MapMeta` | yes | arda-refine's review record: relaxed fills, WFC attempts, tags. |
| owners | `Owner[]` | yes | Row-major owner of every square after composition. |
| overlays | `&'static str[]` | yes | Overlay layers that claimed at least one square, in order. |

### Pipeline

| Field | Type | Required | Notes |
|---|---|---|---|
| src | `Box<dyn Source + Send + Sync>` | yes | — |
| cache | `Mutex<BlockCache>` | yes | — |

### LandUseRaster

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `usize` | yes | — |
| height | `usize` | yes | — |
| codes | `u8[]` | yes | — |
| crofts | `bool[]` | yes | Built cells of hamlets and villages: their tofts and crofts are farmed wherever the town plan leaves the ground free. |

### SocietyOverlays

| Field | Type | Required | Notes |
|---|---|---|---|
| world | `Arc<World>` | yes | — |
| ways_roads | `(Bbox, arda_ways::Road)[]` | yes | — |
| crossings | `arda_ways::Crossing[]` | yes | — |
| field_roads | `(Bbox, arda_fields::Road)[]` | yes | — |
| landuse | `LandUseRaster` | yes | — |
| channels | `rivers::ChannelCache` | yes | — |

### Catalog

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Schema version; must equal `FORMAT_VERSION`. |
| library | `String` | yes | Short library name, e.g. |
| library_version | `String` | yes | Version of the art itself; part of the render's identity. |
| pixels_per_square | `u32` | yes | Source art resolution: pixels per 5-ft square. |
| vocabulary | `TagVocabulary` | no | Allowed values for the controlled tag lists.; serde default |
| assets | `Asset[]` | yes | Every asset in the library. |

### TagVocabulary

| Field | Type | Required | Notes |
|---|---|---|---|
| biome | `String[]` | no | Biomes, e.g.; serde default |
| culture | `String[]` | no | Cultures, e.g.; serde default |
| wealth | `String[]` | no | Wealth levels, e.g.; serde default |
| function | `String[]` | no | Building functions, e.g.; serde default |

### Asset

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Unique, stable identifier, e.g. |
| class | `AssetClass` | yes | Asset class. |
| image | `String` | yes | Image path relative to the library directory (PNG, RGBA). |
| footprint | `Footprint` | yes | Size in 5-ft squares, before rotation. |
| anchor | `Option<Anchor>` | no | Pivot in squares from the image's top-left corner; defaults to the centre.; serde default |
| rotations | `u16[]` | no | Allowed clockwise rotations in degrees: any of 0, 90, 180, 270.; serde default |
| mirror | `bool` | no | Whether a horizontal mirror may be applied.; serde default |
| tags | `Tags` | no | Classification tags.; serde default |
| placement | `PlacementRules` | no | Where the asset may be placed.; serde default |
| blocks_sight | `bool` | no | Blocks line of sight.; serde default |
| blocks_movement | `bool` | no | Blocks movement through the squares it covers.; serde default |
| difficult_terrain | `bool` | no | Its squares are difficult terrain.; serde default |
| cover | `Cover` | no | Cover granted to a creature behind it.; serde default |
| light | `Option<Light>` | no | Light emitted, if any.; serde default |
| layer | `Layer` | yes | Render layer. |
| z | `i16` | no | Order within the layer; higher draws later.; serde default |
| casts_shadow | `bool` | no | Whether the lighting pass casts a drop shadow for it.; serde default |
| height_ft | `u16` | no | Height hint in feet; sets drop-shadow length.; serde default |
| tileable | `bool` | no | Texture tiles seamlessly (ground and water); checked by the seam rule.; serde default |
| ground | `Option<String>` | no | Ground or water type key this texture paints, e.g.; serde default |
| wall | `Option<WallPiece>` | no | Wall-kit membership, for `wall` assets.; serde default |
| provenance | `String` | yes | Where the art came from (tool, prompt, author). |
| licence | `String` | yes | Licence under which the art may be used. |

### AssetClass

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ground, wall, prop, vegetation, water, unknown |

### Footprint

| Field | Type | Required | Notes |
|---|---|---|---|
| w | `u32` | yes | Width in squares. |
| h | `u32` | yes | Height in squares. |

### Anchor

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f32` | yes | Squares right of the image's left edge. |
| y | `f32` | yes | Squares below the image's top edge. |

### Tags

| Field | Type | Required | Notes |
|---|---|---|---|
| biome | `String[]` | no | Biomes the asset suits.; serde default |
| culture | `String[]` | no | Cultures the asset suits.; serde default |
| wealth | `String[]` | no | Wealth levels the asset suits.; serde default |
| function | `String[]` | no | Building functions the asset suits.; serde default |
| free | `String[]` | no | Free-form tags, e.g.; serde default |

### PlacementRules

| Field | Type | Required | Notes |
|---|---|---|---|
| on_water | `bool` | no | Must stand on water (boats).; serde default |
| against_wall | `bool` | no | Must touch a wall edge.; serde default |
| near_road | `bool` | no | Prefers squares next to a road.; serde default |
| clearance_squares | `u8` | no | Empty squares required around the footprint.; serde default |
| ground | `String[]` | no | Ground types it may stand on; empty means any.; serde default |

### Cover (arda-tactical)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total (alias full) |

### Light

| Field | Type | Required | Notes |
|---|---|---|---|
| radius_ft | `u16` | yes | Bright-light radius in feet. |
| colour | `[u8; 3]` | yes | RGB colour. |

### Layer

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ground, water, floor, prop, wall, canopy |

### WallPiece

| Field | Type | Required | Notes |
|---|---|---|---|
| kit | `String` | yes | Kit name, e.g. |
| role | `WallRole` | yes | The piece's role in the kit. |

### WallRole

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: run, door, window, gate, corner, tee, cross, end, post |

### TacticalLayout

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Layout name. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| squares | `Square[]` | yes | Row-major squares, `width × height` of them. |
| walls | `WallSegment[]` | no | Wall, door, window and gate segments on square edges.; serde default |
| placements | `Placement[]` | no | Props and vegetation.; serde default |
| lights | `LightSource[]` | no | Free-standing light sources.; serde default |
| origin | `Option<[i64; 2]>` | no | World position of square `(0, 0)`, in world squares.; serde default; omitted when empty/none |

### Square

| Field | Type | Required | Notes |
|---|---|---|---|
| ground | `String` | yes | Ground type key, matching a texture's `ground` field. |
| elevation_ft | `i16` | no | Elevation in feet.; serde default |
| water_depth_ft | `u8` | no | Water depth in feet; zero is dry land.; serde default |
| dryness | `u8` | no | Climate dryness, `0` (lush, the default) to `255` (parched steppe): the compositor tints vegetated ground toward dry grass by it, interpolated between square centres.; serde default; omitted when empty/none |

### EdgeAxis

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: horizontal, vertical |

### WallSegment

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxis` | yes | Which edge. |
| kind | `WallRole` | yes | Edge role: run, door, window or gate. |
| kit | `String` | yes | Wall kit name. |
| tags | `String[]` | no | Free tags on the edge (`docs/goal-prompts/vocabulary.md`): on a door, `locked` (needs a key or a check to open) and `secret` (drawn as a plain run of its kit; the scene marks it a secret door); on…; serde default; omitted when empty/none |

### AssetRef

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `(String)` | no | variant (externally tagged); An asset and its `.alt<N>` takes; the compositor draws one take per placement, picked by a hash of the seed and world position. |
| query | `{class: Option<AssetClass>, tags: Vec<String>}` | no | variant (externally tagged); Any asset of the class carrying every tag; the compositor picks a family by hashing the seed and placement index, then a take of it as for `AssetRef::Id`. |

### Placement

| Field | Type | Required | Notes |
|---|---|---|---|
| asset | `AssetRef` | yes | What to place. |
| x | `f32` | yes | Where the asset's anchor lands, in squares (`3.5` is a square centre). |
| y | `f32` | yes | Where the asset's anchor lands, in squares. |
| rotation | `u16` | no | Clockwise rotation in degrees; must be allowed by the asset.; serde default |
| mirror | `bool` | no | Horizontal mirror, applied before rotation.; serde default |

### LightSource

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f32` | yes | Position in squares. |
| y | `f32` | yes | Position in squares. |
| radius_ft | `u16` | yes | Bright-light radius in feet. |
| colour | `[u8; 3]` | yes | RGB colour. |

### RulesSidecar

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Schema version; must equal `SIDECAR_FORMAT_VERSION`. |
| width | `u32` | yes | Width in squares; must equal the layout's. |
| height | `u32` | yes | Height in squares; must equal the layout's. |
| squares | `RulesCell[]` | yes | Row-major cells, `width × height` of them. |
| edges | `EdgeRule[]` | no | Rules for low edge features that are not wall-kit walls (logic/12 §scene-sidecar): parapets, retaining walls, hedges and field walls.; serde default; omitted when empty/none |

### EdgeRole

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: parapet, retaining, building, boundary, gate |

### EdgeRule

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxis` | yes | Which edge of the square. |
| role | `EdgeRole` | yes | Why it is there. |
| blocks_movement | `bool` | yes | Blocks movement across the edge. |
| blocks_sight | `bool` | yes | Blocks line of sight across the edge. |
| cover | `CoverLevel` | no | Cover it grants to a creature behind it.; serde default |

### RulesCell

| Field | Type | Required | Notes |
|---|---|---|---|
| difficult | `Option<bool>` | no | Overrides difficult terrain: `true` sets it (undergrowth, scree, mud), `false` clears what props implied.; serde default; omitted when empty/none |
| water_depth_ft | `Option<u8>` | no | Overrides the layout's water depth in feet (0 is dry, 1–4 wading, 5 or more swimming).; serde default; omitted when empty/none |
| cover | `Option<CoverLevel>` | no | Cover from terrain (trees, boulders).; serde default; omitted when empty/none |
| blocks_sight | `Option<bool>` | no | Overrides whether the square blocks sight (dense canopy clusters).; serde default; omitted when empty/none |
| lightly_obscured | `Option<bool>` | no | Lightly obscured (moderate foliage, reeds), logic/09 §rules-sidecar.; serde default; omitted when empty/none |
| blocks_movement | `Option<bool>` | no | Overrides whether the square blocks movement: `true` makes it impassable (cliff face, thicket), `false` clears prop blocking.; serde default; omitted when empty/none |
| deck | `Option<bool>` | no | Overrides whether a walkable deck (bridge, jetty) spans the square: `true` means it is walked, not waded or swum, whatever the depth.; serde default; omitted when empty/none |
| ext | `Option<RulesExt>` | no | Layer-specific extras the scene does not interpret.; serde default; omitted when empty/none |

### Sq

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u32` | yes | — |
| 1 | `u32` | yes | — |

### Movement

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: normal, difficult, wade, swim, impassable |

### CoverLevel

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total (alias full) |

### Obscurement

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: clear, light, heavy |

### WallKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: wall, door, window, gate, secret |

### Wall

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `WallKind` | yes | Feature type. |
| open | `Option<bool>` | no | `Some(open)` for doors, gates and secret doors; `None` otherwise.; serde default; omitted when empty/none |
| points | `[u32; 2][]` | yes | Grid vertices `[x, y]`, at least two; consecutive points are collinear runs merged from unit edges. |
| blocks_sight | `bool` | yes | Blocks line of sight in its current state. |
| blocks_movement | `bool` | yes | Blocks movement in its current state. |
| blocks_light | `bool` | yes | Blocks light in its current state. |
| cover | `CoverLevel` | no | Cover it grants to a creature behind it when it does not block sight.; serde default |
| kit | `String` | yes | Wall kit name. |
| locked | `bool` | no | A locked door, gate or secret door (layout edge tag `locked`): it needs a key or a check to open.; serde default; omitted when empty/none |

### BlockerKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: canopy, prop |

### VisionBlocker

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `BlockerKind` | yes | Source class. |
| asset | `String` | yes | Asset id. |
| obscurement | `Obscurement` | yes | How much it obscures: `light` for foliage, `heavy` for opaque. |
| polygon | `[f32; 2][]` | yes | Closed polygon in squares, clockwise in y-down coordinates. |

### SceneLight

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f32` | yes | Position in squares. |
| y | `f32` | yes | Position in squares. |
| bright_ft | `u16` | yes | Bright-light radius in feet. |
| dim_ft | `u16` | yes | Outer edge of dim light in feet: dim light extends an additional `bright_ft` beyond the bright radius (the SRD torch pattern). |
| colour | `[u8; 3]` | yes | RGB colour. |
| asset | `Option<String>` | no | Emitting asset id, or `None` for a free layout light.; serde default; omitted when empty/none |

### RegionKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: difficult, shallow_water, deep_water |

### Region

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `RegionKind` | yes | What the region marks. |
| rings | `[u32; 2][][]` | yes | Closed rings (first point not repeated). |

### Edge

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: N, E, S, W |

### EdgeExit

| Field | Type | Required | Notes |
|---|---|---|---|
| edge | `Edge` | yes | Which edge. |
| from | `Sq` | yes | First square of the run. |
| to | `Sq` | yes | Last square of the run (inclusive). |

### Entrance

| Field | Type | Required | Notes |
|---|---|---|---|
| wall | `usize` | yes | Index into `Scene::walls`. |
| squares | `Sq[]` | yes | Enterable squares on either side of it. |

### SpawnHints

| Field | Type | Required | Notes |
|---|---|---|---|
| open | `Sq[]` | yes | Normal-movement squares with no cover whose 8 neighbours are the same: room for a token and some elbow room, row-major. |
| entrances | `Entrance[]` | yes | Squares by doors and gates. |
| exits | `EdgeExit[]` | yes | Enterable runs along the map edges, N, E, S, W order. |

### Scene

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Schema version, `SCENE_FORMAT_VERSION`. |
| name | `String` | yes | Layout name. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| seed | `u64` | yes | Seed the layout's asset queries were resolved with; a JSON string.; serde with `crate::serde_str` |
| library | `String` | yes | Library name. |
| library_version | `String` | yes | Library art version; with the seed, the scene's identity. |
| movement | `Grid<Movement>` | yes | Per-square movement. |
| climb | `Grid<u8>` | yes | Per-square climb mask: bit `d` set means stepping to the neighbour in direction `d` (0 N, 1 NE, 2 E, 3 SE, 4 S, 5 SW, 6 W, 7 NW) crosses an elevation step of 10 ft or more and needs climbing. |
| cover | `Grid<CoverLevel>` | yes | Per-square cover granted by what stands in the square. |
| obscured | `Grid<Obscurement>` | yes | Per-square obscurement. |
| elevation_ft | `Grid<i16>` | yes | Per-square elevation in feet. |
| water_depth_ft | `Grid<u8>` | yes | Per-square water depth in feet (after the sidecar). |
| walls | `Wall[]` | yes | Wall, door, window, gate and secret-door polylines. |
| vision_blockers | `VisionBlocker[]` | yes | Canopy and prop polygons that obscure vision. |
| lights | `SceneLight[]` | yes | Light sources. |
| regions | `Region[]` | yes | Difficult-terrain and water regions. |
| spawn_hints | `SpawnHints` | yes | Token placement hints. |
| tokens | `TokenSlot[]` | no | NPC token slots (goal 48); empty until the NPC stage fills them.; serde default |

### TokenSlot

| Field | Type | Required | Notes |
|---|---|---|---|
| npc_id | `String` | yes | NPC id: a u64 written as a decimal string (canonical convention I5). |
| x | `f32` | yes | Token centre in squares (`3.5` is a square centre). |
| y | `f32` | yes | Token centre in squares. |
| facing | `u16` | no | Facing in degrees clockwise from north, `0..360`.; serde default |

### Kind (arda-dungeon)

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: dungeon, cave |

### Params

| Field | Type | Required | Notes |
|---|---|---|---|
| seed | `u64` | yes | Seed; a decimal string on the wire (`arda_scene::serde_str`), a number also read. |
| kind | `Kind` | yes | Dungeon or cave. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |

### Purpose

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: entrance, crypt, barracks, storage, shrine, prison, treasure, hall, cavern |

### Room

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `usize` | yes | Index in `Dungeon::rooms`. |
| purpose | `Purpose` | yes | What it is for. |
| x | `u32` | yes | Left column. |
| y | `u32` | yes | Top row. |
| w | `u32` | yes | Width in squares. |
| h | `u32` | yes | Height in squares. |

### Door (arda-dungeon)

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxis` | yes | Which edge. |
| locked | `bool` | yes | Needs a key or a check. |
| secret | `bool` | yes | Looks like wall until found. |

### ExitKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: stairs_up, stairs_down, mouth |

### Exit

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `ExitKind` | yes | Kind. |
| x | `u32` | yes | Column. |
| y | `u32` | yes | Row. |

### Dungeon

| Field | Type | Required | Notes |
|---|---|---|---|
| params | `Params` | yes | The parameters it was made from. |
| layout | `TacticalLayout` | yes | The battle map. |
| rules | `RulesSidecar` | yes | Per-square rules: rock impassable and opaque, scree difficult. |
| rooms | `Room[]` | yes | Rooms (one `cavern` spanning the map for caves). |
| doors | `Door[]` | yes | Every door, including locked and secret ones. |
| exits | `Exit[]` | yes | Stairs and mouths. |

### HeightTile

| Field | Type | Required | Notes |
|---|---|---|---|
| i0 | `i64` | yes | First node column. |
| j0 | `i64` | yes | First node row. |
| width | `usize` | yes | Columns. |
| height | `usize` | yes | Rows. |
| n | `i64` | yes | Subdivisions of the stored 39.0625 m spacing (1, 2, 4 or 8). |
| heights_mm | `i32[]` | yes | Heights, row-major, millimetres. |

### Landscape

| Field | Type | Required | Notes |
|---|---|---|---|
| overlays | `Arc<SocietyOverlays>` | yes | — |

### LandWindow

| Field | Type | Required | Notes |
|---|---|---|---|
| grid | `PixelGrid` | yes | — |
| seed | `u64` | yes | — |
| low | `LowRelief` | yes | — |
| parcels | `Option<Parcels>` | no | — |
| parcel_strength | `i64` | yes | — |
| cells | `Option<CellTones>` | no | — |
| forest | `CellField` | yes | — |
| roads | `Option<RoadRaster>` | no | — |
| towns | `Option<Towns>` | no | — |

### Tone

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: Ploughed, Stubble, Fallow, Hay, Mown, Grazed, Orchard, Woodland, Floodplain |

### Pyramid

| Field | Type | Required | Notes |
|---|---|---|---|
| max_zoom | `u32` | yes | Native (overview) deepest zoom. |
| areas_wide | `i32` | yes | Areas across. |
| areas_high | `i32` | yes | Areas down. |

### CellInfo

| Field | Type | Required | Notes |
|---|---|---|---|
| terrain | `TerrainKind` | yes | Sea, land or lake. |
| cover | `Cover` | yes | Dominant ground cover. |
| river_width_dm | `u32` | yes | Channel width in decimetres; 0 without a watercourse. |

### AreaFacts

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `CellInfo[]` | yes | Cells, row-major, 512 × 512. |
| edges | `ChannelEdge[]` | yes | Saved channel centreline edges touching the area. |

### WorldTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| world | `Arc<World>` | yes | — |
| reader | `Mutex<TerrainFileReader<File>>` | yes | — |
| wide | `i64` | yes | — |
| high | `i64` | yes | — |
| chunks | `Mutex<ChunkCache>` | yes | — |
| areas | `Mutex<AreaCache>` | yes | — |

### Rgba

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | Columns. |
| height | `u32` | yes | Rows. |
| pixels | `u8[]` | yes | Row-major RGBA. |

### WindowWater

| Field | Type | Required | Notes |
|---|---|---|---|
| region | `WaterRegion` | yes | — |
| pixel_sq | `f64` | yes | — |
| pools | `bool` | yes | — |

### ReliefWorld

| Field | Type | Required | Notes |
|---|---|---|---|
| terrain | `WorldTerrain` | yes | — |
| water | `arda_refine::WorldSource<'static>` | yes | The tactical layer's view of the world, which relief water is drawn from (logic/17 §water). |
| contexts | `Mutex<Contexts>` | yes | — |
| build | `Mutex<Shore>` | yes | Serialises context builds so peak memory stays bounded; it also holds the shore layer, which every context paints from. |
| land | `Option<crate::land::Landscape>` | no | The world's society (land use, roads, town plans), when relief tiles draw worked land (logic/17 §land). |

### Manifest (arda-art-import)

| Field | Type | Required | Notes |
|---|---|---|---|
| library | `LibrarySection` | no | Library identity and provenance defaults.; serde default |
| grade | `GradeSection` | no | The optional global colour grade.; serde default |
| asset | `AssetEntry[]` | no | Per-file or per-id overrides.; serde default; Rust field `assets` |

### LibrarySection

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `Option<String>` | no | Library name (default `imported`). |
| version | `Option<String>` | no | Library version (default `0.1.0`). |
| pixels_per_square | `Option<u32>` | no | Pixels per square of the output (default: the base library's, else 128). |
| licence | `Option<String>` | no | Default licence for every asset. |
| tool | `Option<String>` | no | Default generator tool, e.g. |
| model | `Option<String>` | no | Default model, e.g. |
| author | `Option<String>` | no | Who curated the art. |

### GradeSection

| Field | Type | Required | Notes |
|---|---|---|---|
| reference | `Option<String>` | no | A reference image (PNG or JPEG) whose colour statistics to match. |
| palette | `String[]` | no | Target palette as `#rrggbb` strings (used when no reference).; serde default |
| strength | `Option<f32>` | no | 0 = no grade, 1 = full statistics transfer (default 0.5). |

### AssetEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| file | `Option<String>` | no | Raw file name (with extension) inside the raw directory. |
| id | `Option<String>` | no | Asset spec, e.g. |
| prompt | `Option<String>` | no | Generator prompt. |
| seed | `Option<u64>` | no | Generator seed. |
| tool | `Option<String>` | no | Generator tool (overrides `library.tool`). |
| model | `Option<String>` | no | Model (overrides `library.model`). |
| licence | `Option<String>` | no | Licence (overrides `library.licence`). |
| footprint | `Option<[u32; 2]>` | no | Footprint `[w, h]` in squares. |
| height_ft | `Option<u16>` | no | Height in feet. |
| cover | `Option<Cover>` | no | Cover level. |
| layer | `Option<Layer>` | no | Render layer. |
| blocks_sight | `Option<bool>` | no | Blocks line of sight. |
| blocks_movement | `Option<bool>` | no | Blocks movement. |
| difficult_terrain | `Option<bool>` | no | Difficult terrain. |
| tags | `Option<Tags>` | no | Tags; replace the inherited tags when given. |
| structured | `Option<bool>` | no | Marks a texture as `structured` (aligned variants). |
| rot_free | `Option<bool>` | no | Adds (or removes) the free tag `rot_free`: the compositor may turn and mirror the cut-out by hash. |
| fixed_pose | `Option<bool>` | no | Adds (or removes) the free tag `fixed_pose`: the compositor draws the vegetation cut-out at its catalogued size and orientation, without the hashed scale and diagonal transpose. |
| shadow | `Option<ShadowMode>` | no | Shadow handling: `strip` (default) or `keep` (never strip, still flag). |
| holes | `Option<HoleMode>` | no | Enclosed backdrop-coloured pockets: `clear` (any size) or `keep` (never). |

### HoleMode

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: keep, clear |

### ShadowMode

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: strip, keep |

### Status

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: imported, rejected |

### AssetReport

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Catalogue id. |
| file | `String` | yes | Raw file name. |
| class | `String` | yes | Asset class. |
| status | `Status` | yes | Imported or rejected. |
| fixes | `String[]` | yes | What the importer did. |
| flags | `String[]` | yes | What a human should look at. |

### Skipped

| Field | Type | Required | Notes |
|---|---|---|---|
| file | `String` | yes | Raw file name. |
| reason | `String` | yes | Why. |

### Coverage

| Field | Type | Required | Notes |
|---|---|---|---|
| imported | `String[]` | yes | Slots the imported library provides. |
| fallback | `String[]` | yes | Slots that fall back to the base library. |
| missing | `String[]` | yes | Slots no library provides. |

### Report (arda-art-import)

| Field | Type | Required | Notes |
|---|---|---|---|
| library | `String` | yes | Library name and version. |
| assets | `AssetReport[]` | yes | Every processed file, in catalogue order. |
| skipped | `Skipped[]` | yes | Files that could not be mapped. |
| grade | `Option<String>` | no | The colour grade, if any. |
| validation | `String[]` | yes | Validator issues of the imported library on its own (excluding kits that are completed by the base). |
| partial_kits | `String[]` | yes | Kits the import only partly provides; the base fills the rest. |
| stack_validation | `Option<Vec<String>>` | no | Validator issues of the imported library stacked over the base. |
| coverage | `Coverage` | yes | Vocabulary coverage. |

### Legend

| Field | Type | Required | Notes |
|---|---|---|---|
| terrain | `TerrainKindDto[]` | yes | Codes of the `terrain` column. |
| cover | `CoverDto[]` | yes | Codes of the `cover` column. |
| road | `RoadDto[]` | yes | Codes of the `road` column. |
| land_use | `LandUseDto[]` | yes | Codes of the `land_use` column (0 also when the world has no `society/`). |

### AreaColumns

| Field | Type | Required | Notes |
|---|---|---|---|
| $( #[doc = $doc] pub $name | `Vec<$ty>, )*` | yes | — |

### AreaCells

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | Cell contract version. |
| ax | `i32` | yes | Area column. |
| ay | `i32` | yes | Area row. |
| gx0 | `u32` | yes | Global column of local x = 0. |
| gy0 | `u32` | yes | Global row of local y = 0. |
| width | `u32` | yes | Columns per row (512). |
| height | `u32` | yes | Rows (512). |
| legend | `Legend` | yes | Enum code tables. |
| columns | `AreaColumns` | yes | Values. |

### TerrainKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: sea, land, lake |

### CoverDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: bare, grass, scrub, forest, marsh, rock, ice |

### RoadDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, track, road, highway, footpath |

### LandUseDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, built, field, pasture, orchard, woodland, mill, mine, meadow, fallow, farmstead |

### CoastSample

| Field | Type | Required | Notes |
|---|---|---|---|
| is_coast | `bool` | yes | A land cell with at least one sea cell among its 8 neighbours. |
| distance_m | `Option<f64>` | no | Euclidean centre-to-centre distance to the nearest coast cell, metres; 0 on the coast, `null` when none lies within `coast::MAX_DISTANCE_M`. |

### SnowSample

| Field | Type | Required | Notes |
|---|---|---|---|
| fraction | `f64` | yes | Perennial snow cover of the cell, 0–1 (see `snow`). |
| peak_fraction | `Option<f64>` | no | The same rule at the footprint's highest fine-terrain point; `null` without fine terrain. |
| perennial | `bool` | yes | `fraction >= 0.5`, or permanent-ice cover. |
| snowline_m | `f64` | yes | Altitude, metres, where the lapse-adjusted mean annual temperature crosses the snow midpoint (−5.5 °C). |

### RiverMembership

| Field | Type | Required | Notes |
|---|---|---|---|
| segment_id | `u32` | yes | Tile-local segment id (`RiverSegment::id`), unique within the area. |
| global_reach_id | `String` | yes | Stable global reach id, decimal string (u64 does not fit a JS number). |
| order | `u8` | yes | Strahler order of the segment. |
| width_m | `f64` | yes | Segment channel width, metres. |
| discharge_m3s | `f64` | yes | Segment mean discharge, m³/s. |

### LakeMembership

| Field | Type | Required | Notes |
|---|---|---|---|
| lake_id | `u32` | yes | Tile-local lake id (`Lake::id`), unique within the area. |
| global_basin_id | `String` | yes | Stable global basin id, decimal string. |
| surface_m | `f64` | yes | Representative water surface, metres above sea level. |
| depth_m | `f64` | yes | Water depth in this cell, metres: surface minus cell height, never negative. |
| max_depth_m | `f64` | yes | Greatest local depth of the lake fragment, metres. |

### FineHeights

| Field | Type | Required | Notes |
|---|---|---|---|
| centre_m | `f64` | yes | Bilinear height at the cell centre, metres. |
| min_m | `f64` | yes | Exact minimum of the bilinear surface over the footprint, metres. |
| max_m | `f64` | yes | Exact maximum of the bilinear surface over the footprint, metres. |

### CellSample

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | `CONTRACT_VERSION` this sample was produced under. |
| gx | `u32` | yes | Global cell column. |
| gy | `u32` | yes | Global cell row. |
| ax | `i32` | yes | Area tile column containing the cell. |
| ay | `i32` | yes | Area tile row containing the cell. |
| cx | `u16` | yes | Column within the area, 0–511. |
| cy | `u16` | yes | Row within the area, 0–511. |
| x_m | `f64` | yes | West edge of the cell, metres east of the world origin (`gx·100`). |
| y_m | `f64` | yes | North edge of the cell, metres south of the world origin (`gy·100`). |
| centre_x_m | `f64` | yes | Cell centre, metres east of the world origin (`gx·100 + 50`). |
| centre_y_m | `f64` | yes | Cell centre, metres south of the world origin (`gy·100 + 50`). |
| height_m | `f64` | yes | Elevation above sea level, metres (`Cell::height`, mm). |
| terrain | `TerrainKindDto` | yes | Sea, land or lake (`Cell::terrain`). |
| cover | `CoverDto` | yes | Dominant cover (`Cell::cover`). |
| slope_deg | `f64` | yes | Slope, degrees (`Cell::slope_milli_deg`). |
| aspect_deg | `f64` | yes | Downslope compass bearing, degrees 0–359; 0 on flat ground (`Cell::aspect_deg`). |
| temperature_c | `f64` | yes | Mean annual temperature, °C (`Cell::temperature`, centi-°C). |
| rainfall_mm | `f64` | yes | Mean annual rainfall, mm (`Cell::rainfall`). |
| moisture | `f64` | yes | Soil moisture, 0–1 (`Cell::moisture` / 255). |
| wetness | `f64` | yes | Standing-water tendency, 0–1 (`Cell::wetness` / 255). |
| forest_density | `f64` | yes | Canopy closure, 0–1 (`Cell::forest_density` / 255). |
| drainage_area_km2 | `f64` | yes | Upstream drainage area, km² (`Cell::drainage_area_cells` × 0.01 km²). |
| discharge_m3s | `f64` | yes | Mean discharge, m³/s (`Cell::discharge`, L/s). |
| watercourse_order | `u8` | yes | Strahler order; 0 when no watercourse (`Cell::watercourse_order`). |
| watercourse_width_m | `f64` | yes | Channel width, metres; 0 when no watercourse (`Cell::watercourse_width_dm`). |
| height_above_river_m | `f64` | yes | Height above the nearest downstream channel, metres (`Cell::height_above_river_dm`). |
| road | `RoadDto` | yes | Road class: `society/roads.bin` when present, else `Cell::road`. |
| built_by | `Option<String>` | no | Settlement id (decimal string) that owns this cell: `society/landuse.bin` owners when present, else `Cell::built_by`; `null` when none. |
| land_use | `Option<LandUseDto>` | no | Land use (`society/landuse.bin`); `null` without a `society/`. |
| realm_id | `Option<String>` | no | Realm id (decimal string, `society/realms.bin`); `null` when none or without a `society/`. |
| coast | `CoastSample` | yes | Derived coast facts. |
| snow | `SnowSample` | yes | Derived snow proxy. |
| river | `Option<RiverMembership>` | no | Saved river course through this cell, if any. |
| lake | `Option<LakeMembership>` | no | Saved lake covering this cell, if any. |
| fine | `Option<FineHeights>` | no | Fine-terrain heights; `null` for worlds without `terrain/fine.bin`. |

### HeightSource

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: fine, cells |

### PointSample

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | `CONTRACT_VERSION`. |
| x_m | `f64` | yes | Query position east of the world origin, metres. |
| y_m | `f64` | yes | Query position south of the world origin, metres. |
| height_m | `f64` | yes | Interpolated elevation, metres. |
| height_source | `HeightSource` | yes | Which surface was interpolated. |
| cell | `CellSample` | yes | Sample of the cell containing the point. |

### SizeKmDto

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | East–west extent. |
| height | `u32` | yes | North–south extent. |

### LatitudeDto

| Field | Type | Required | Notes |
|---|---|---|---|
| south_deg | `i16` | yes | Southern edge. |
| north_deg | `i16` | yes | Northern edge. |

### FineTerrainDto

| Field | Type | Required | Notes |
|---|---|---|---|
| recipe_version | `u16` | yes | Recipe that produced `terrain/fine.bin`. |
| spacing_m | `f64` | yes | Lattice spacing, metres (39.0625). |

### TilePyramidDto

| Field | Type | Required | Notes |
|---|---|---|---|
| tile_px | `u32` | yes | Tile edge, pixels. |
| max_zoom | `u32` | yes | Deepest zoom; zoom 0 is the whole world in one tile. |
| base_px | `u32` | yes | Edge of the square pyramid at `max_zoom`, pixels (`tile_px << max_zoom`). |
| image_width_px | `u32` | yes | Overview image width inside the base square, pixels. |
| image_height_px | `u32` | yes | Overview image height inside the base square, pixels. |
| relief_max_zoom | `u32` | yes | Deepest zoom of `/v1/tiles/relief` (levels `max_zoom + 1..=` this); equal to `max_zoom` when the world has no relief levels. |

### WorldInfo

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | Cell contract version. |
| api_version | `String` | yes | HTTP API version. |
| seed | `String` | yes | World seed as a decimal string (u64 does not fit a JS number). |
| arda_version | `String` | yes | Generator version that wrote the world. |
| format_version | `u32` | yes | Stored format major. |
| size_km | `SizeKmDto` | yes | Nominal continent extent. |
| latitude | `LatitudeDto` | yes | Latitude belt. |
| areas_wide | `i32` | yes | Area tiles per row. |
| areas_high | `i32` | yes | Area tile rows. |
| area_cells | `u32` | yes | Cells along an area edge (512). |
| cell_size_m | `u32` | yes | Cell spacing, metres (100). |
| cells_wide | `u32` | yes | Global cell columns (`areas_wide × 512`). |
| cells_high | `u32` | yes | Global cell rows (`areas_high × 512`). |
| fine_terrain | `Option<FineTerrainDto>` | no | Fine terrain, when the world has it. |
| tiles | `TilePyramidDto` | yes | Overview tile pyramid. |

### Health

| Field | Type | Required | Notes |
|---|---|---|---|
| status | `String` | yes | Always `"ok"` when the service answers. |
| contract_version | `u32` | yes | Cell contract version. |
| seed | `String` | yes | Seed of the served world, decimal string. |

### TerminusDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: junction, sea, lake, off_tile, basin, divergence |

### RiverDto

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | Tile-local id, 1-based. |
| global_id | `String` | yes | Stable global reach id, decimal string. |
| order | `u8` | yes | Strahler order. |
| width_m | `f64` | yes | Channel width, metres. |
| discharge_m3s | `f64` | yes | Mean discharge, m³/s. |
| feeds | `Option<u32>` | no | Local downstream segment at a confluence, if any. |
| ends | `TerminusDto` | yes | How the segment ends. |
| course | `[u32; 2][]` | yes | Global cells `[gx, gy]`, upstream to downstream. |

### LakeDto

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | Tile-local id, 1-based. |
| global_id | `String` | yes | Stable global basin id, decimal string. |
| surface_m | `f64` | yes | Representative water surface, metres. |
| max_depth_m | `f64` | yes | Greatest local depth, metres. |
| outlet | `Option<[u32; 2]>` | no | Global cell of supported surface outflow, if in this area. |
| cells | `[u32; 2][]` | yes | Covered global cells `[gx, gy]`. |

### AreaRivers

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | Cell contract version. |
| ax | `i32` | yes | Area column. |
| ay | `i32` | yes | Area row. |
| rivers | `RiverDto[]` | yes | Segments in stored order. |

### AreaLakes

| Field | Type | Required | Notes |
|---|---|---|---|
| contract_version | `u32` | yes | Cell contract version. |
| ax | `i32` | yes | Area column. |
| ay | `i32` | yes | Area row. |
| lakes | `LakeDto[]` | yes | Lakes in stored order. |

### ApiError

| Field | Type | Required | Notes |
|---|---|---|---|
| error | `ApiErrorBody` | yes | The error details. |

### NotYetError

| Field | Type | Required | Notes |
|---|---|---|---|
| error | `ApiErrorBody` | yes | The error details; `code` is `not_implemented`. |
| planned_source | `String` | yes | Crate planned to provide the feature, for example `arda-refine`. |

### ApiErrorBody

| Field | Type | Required | Notes |
|---|---|---|---|
| code | `String` | yes | Stable code: `bad_request`, `out_of_range`, `not_found`, `no_block`, `payload_too_large`, `unsupported_media_type`, `invalid_layout`, `resource_limit`, `not_implemented` or `internal`. |
| status | `u16` | yes | HTTP status repeated in the body. |
| message | `String` | yes | Human-readable detail; not stable. |

### PopulationRequest (npc.rs runtime)

| Field | Type | Required | Notes |
|---|---|---|---|
| world_seed | `String` | yes | World seed as a decimal string (u64 does not fit a JS number). |
| settlement | `SettlementProfile` | yes | The settlement to populate. |
| buildings | `BuildingSpec[]` | yes | Its buildings. |

### NpcDemo (npc.rs runtime)

| Field | Type | Required | Notes |
|---|---|---|---|
| world_seed | `String` | yes | World seed as a decimal string. |
| settlement | `SettlementProfile` | yes | The example settlement. |
| buildings | `BuildingSpec[]` | yes | Its buildings. |
| population | `Population` | yes | Its population. |

### Tier

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: hamlet, village, town, city |

### SettlementFunction

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |

### Biome

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: temperate, warm_temperate, temperate_forest, boreal_forest, highland, alpine, wetland, steppe, coastal |

### SettlementProfile

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Settlement id. |
| name | `String` | yes | Display name. |
| tier | `Tier` | yes | Size class. |
| population | `u32` | yes | Number of inhabitants. |
| functions | `SettlementFunction[]` | yes | Economic functions. |
| wealth | `u8` | yes | Wealth, 0–255. |
| culture | `String` | yes | Culture key. |
| realm_id | `String` | yes | Realm id. |
| biome | `Biome` | yes | Biome. |
| coastal | `bool` | yes | On a sea coast. |
| riverine | `bool` | yes | On a river. |
| ancestry_mix | `Option<BTreeMap<String, u32>>` | no | Optional ancestry weights replacing the culture's table.; serde default |
| tongue | `Option<Tongue>` | no | The local speech the settlement was named in (`arda_names::Tongue`).; serde default; omitted when empty/none |

### Tongue

| Field | Type | Required | Notes |
|---|---|---|---|
| preset | `String` | yes | Culture preset key (`heartland`, `sylvan`, …). |
| language_seed | `String` | yes | Seed of the standard language. |
| substrate_seed | `String` | yes | Seed of the shared substrate tongue. |
| dialect_seed | `String` | yes | Seed of the region's dialect map. |
| width | `i64` | yes | Dialect map width, cells. |
| height | `i64` | yes | Dialect map height, cells. |
| x | `i64` | yes | Place column, cells. |
| y | `i64` | yes | Place row, cells. |

### BuildingFunction

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: house, farmhouse, cottage, manor, inn, tavern, bakery, brewery, mill, smithy, workshop, tannery, apothecary, temple, shrine, library, warehouse, market, stall, dock, boathouse, keep, barracks, guardhouse, stable, barn, street, farm, market_hall, mine, lumber_camp, school |

### BuildingSpec

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Building id, unique within the settlement. |
| settlement_id | `String` | yes | Owning settlement id. |
| function | `BuildingFunction` | yes | What the building is for. |
| tags | `String[]` | no | Free tags such as `craft:weaving`; omitted when empty.; serde default; omitted when empty/none |
| capacity | `u16` | yes | Resident capacity. |
| workplace_slots | `u16` | yes | Workplace slots, master included. |
| wealth | `u8` | yes | Building wealth, 0–255. |

### NpcId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `String` | yes | — |

### HouseholdId

| Field | Type | Required | Notes |
|---|---|---|---|
| building | `String` | yes | Home building id. |
| index | `u32` | yes | Index among the building's households. |

### Sex

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: female, male |

### Name

| Field | Type | Required | Notes |
|---|---|---|---|
| given | `String` | yes | Given name. |
| family | `String` | yes | Family name. |
| byname | `Option<String>` | no | Optional nickname or epithet. |

### JobCategory

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: agriculture, craft, trade, service, religion, military, government, maritime, extraction, scholar, labour, nobility, dependent |

### Job

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | Occupation key. |
| title | `String` | yes | Display title. |
| category | `JobCategory` | yes | Broad category. |

### SocialRank

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: dependent, labourer, tradesfolk, master, gentry, noble |

### Lifestyle

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: wretched, squalid, poor, modest, comfortable, wealthy, aristocratic |

### Ideal

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Short name. |
| text | `String` | yes | The ideal in the person's words. |
| alignment | `String` | yes | Alignment lean. |
| tags | `String[]` | yes | Coherence tags. |

### Personality

| Field | Type | Required | Notes |
|---|---|---|---|
| traits | `String[]` | yes | Two traits. |
| ideal | `Ideal` | yes | One ideal. |
| bond | `String` | yes | One bond. |
| flaw | `String` | yes | One flaw. |
| mannerism | `String` | yes | A voice or mannerism hook. |
| summary | `String` | yes | One-line summary. |

### RelationKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: spouse, parent, child, sibling, landlord, lodger, employer, employee, friend, rival |

### Relationship

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `RelationKind` | yes | What the other person is to this one. |
| other | `NpcId` | yes | The other person. |

### Npc

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `NpcId` | yes | Stable id. |
| name | `Name` | yes | Name. |
| ancestry | `String` | yes | SRD race key. |
| ancestry_name | `String` | yes | SRD race display name. |
| subrace | `Option<String>` | no | SRD subrace. |
| age | `u16` | yes | Age in years. |
| sex | `Sex` | yes | Sex. |
| job | `Job` | yes | Occupation. |
| workplace_building | `Option<String>` | no | Workplace building id. |
| home_building | `String` | yes | Home building id. |
| household | `HouseholdId` | yes | Household. |
| employer | `Option<NpcId>` | no | Master of the workplace, for employees. |
| social_rank | `SocialRank` | yes | Social standing. |
| wealth | `u8` | yes | Wealth, 0–255. |
| lifestyle | `Lifestyle` | yes | SRD lifestyle. |
| personality | `Personality` | yes | Personality. |
| relationships | `Relationship[]` | yes | Ties, sorted. |
| sheet | `Sheet` | yes | SRD 5.1 character sheet. |
| notable | `bool` | yes | Whether the person is a stored notable. |

### HouseholdRelation

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `NpcId` | yes | The person the relation belongs to. |
| kind | `RelationKind` | yes | What `to` is to `from`. |
| to | `NpcId` | yes | The other person. |

### Household

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `HouseholdId` | yes | Household id. |
| home | `String` | yes | Home building id. |
| family_name | `String` | yes | Family name of the head. |
| members | `NpcId[]` | yes | Members, head first. |
| relations | `HouseholdRelation[]` | yes | Family and lodger ties, both directions. |

### RosterEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `NpcId` | yes | Person. |
| job | `u16` | yes | Index into `Population.job_keys`. |
| workplace | `Option<String>` | no | Workplace building id. |
| notable | `bool` | yes | Whether the person is a stored notable. |

### Population

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement_id | `String` | yes | Settlement id. |
| population | `u32` | yes | Total inhabitants. |
| households | `Household[]` | yes | Every household. |
| npcs | `Npc[]` | yes | Stored notables. |
| commoner_count | `u32` | yes | Inhabitants who are not stored. |
| job_keys | `String[]` | yes | Job keys referenced by the roster. |
| job_categories | `JobCategory[]` | yes | Category of each job key. |
| roster | `RosterEntry[]` | yes | One entry per inhabitant, in id order. |

### PopulationRequest

| Field | Type | Required | Notes |
|---|---|---|---|
| world_seed | `String` | yes | World seed as a decimal string. |
| settlement | `SettlementProfile` | yes | The settlement to populate. |
| buildings | `BuildingSpec[]` | yes | Its buildings. |

### NpcDemo

| Field | Type | Required | Notes |
|---|---|---|---|
| world_seed | `String` | yes | World seed as a decimal string. |
| settlement | `SettlementProfile` | yes | The example settlement. |
| buildings | `BuildingSpec[]` | yes | Its buildings. |
| population | `Population` | yes | Its population. |

### Ability

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: STR, DEX, CON, INT, WIS, CHA |

### SheetKind

| Field | Type | Required | Notes |
|---|---|---|---|
| stat_block | `{name: String, challenge_rating: String, xp: u32}` | no | variant (internally tagged `type`); An SRD NPC stat block. |
| class | `{class: String, subclass: Option<String>, level: u8, background: Background}` | no | variant (internally tagged `type`); A class build. |

### Background

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Name. |
| text | `String` | yes | Description. |
| skills | `String[]` | yes | Granted skills. |

### SaveBonus

| Field | Type | Required | Notes |
|---|---|---|---|
| ability | `Ability` | yes | Ability. |
| bonus | `i8` | yes | Total bonus. |
| proficient | `bool` | yes | Whether proficiency applies. |

### SkillBonus

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD skill name. |
| ability | `Ability` | yes | Ability used. |
| bonus | `i8` | yes | Total bonus. |
| expertise | `bool` | yes | Whether proficiency is doubled. |

### AttackKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: melee, ranged |

### Attack

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Name. |
| kind | `AttackKind` | yes | Melee or ranged. |
| to_hit | `i8` | yes | Attack bonus. |
| damage | `String` | yes | Damage dice. |
| average | `i16` | yes | Average damage. |
| damage_type | `String` | yes | Damage type. |
| range | `String` | yes | Reach or range. |
| extra | `String[]` | yes | Extra effects. |

### Item

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD item name. |
| quantity | `u16` | yes | Quantity. |

### Coins

| Field | Type | Required | Notes |
|---|---|---|---|
| gp | `u32` | yes | Gold. |
| sp | `u32` | yes | Silver. |
| cp | `u32` | yes | Copper. |

### SpellRef

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | SRD spell name. |
| level | `u8` | yes | Spell level, 0 for cantrips. |

### Spellcasting

| Field | Type | Required | Notes |
|---|---|---|---|
| ability | `Ability` | yes | Casting ability. |
| save_dc | `u8` | yes | Spell save DC. |
| attack_bonus | `i8` | yes | Spell attack bonus. |
| slots | `u8[]` | yes | Slots per spell level (index 0 = 1st level). |
| pact_magic | `bool` | yes | Pact Magic slots. |
| spells | `SpellRef[]` | yes | Cantrips and spells. |

### Sheet

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `SheetKind` | yes | Stat block or class build. |
| size | `String` | yes | Size category. |
| abilities | `[u8; 6]` | yes | STR, DEX, CON, INT, WIS, CHA. |
| modifiers | `[i8; 6]` | yes | Their modifiers. |
| proficiency_bonus | `i8` | yes | Proficiency bonus. |
| saves | `SaveBonus[]` | yes | All six saving throws. |
| skills | `SkillBonus[]` | yes | Proficient skills. |
| hit_points | `u16` | yes | Hit points. |
| hit_dice | `String` | yes | Hit dice. |
| armor_class | `u8` | yes | Armour class. |
| armor | `String[]` | yes | Worn armour and shield. |
| speed | `u8` | yes | Walking speed, feet. |
| senses | `String[]` | yes | Senses. |
| passive_perception | `u8` | yes | Passive Wisdom (Perception). |
| languages | `String[]` | yes | Languages. |
| damage_resistances | `String[]` | yes | Damage resistances. |
| attacks | `Attack[]` | yes | Attacks. |
| features | `String[]` | yes | Features by name. |
| equipment | `Item[]` | yes | Equipment. |
| coins | `Coins` | yes | Coins. |
| spellcasting | `Option<Spellcasting>` | no | Spellcasting, for casters. |

### NpcEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| npc_ref | `String` | yes | `"<settlement>.<building>.<index>"`: resolves with `GET /v1/npc/{ref}`, commoners included. |
| settlement_id | `String` | yes | The settlement, a decimal string. |
| notable | `bool` | yes | Whether the person is a stored notable. |
| npc | `npc_dto::Npc` | yes | The person with their SRD 5.1 sheet. |

### NpcPage

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | `NPC_PAGE_FORMAT`. |
| npcs | `NpcEntry[]` | yes | The matches, in settlement-id then skeleton order (home building id, then resident index). |
| next_cursor | `Option<String>` | no | Pass as `?cursor=` for the next page; `null` on the last one. |
| scanned_settlements | `u32` | yes | Settlements whose skeletons this page scanned (bounded per page). |

### SettlementList

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Version of this body (1). |
| settlements | `arda_settle::model::Settlement[]` | yes | The `arda-settle` records, filtered. |

### SettlementDetail

| Field | Type | Required | Notes |
|---|---|---|---|
| record | `arda_settle::model::Settlement` | yes | The `arda-settle` record. |
| society | `Option<arda_society::SettlementSociety>` | no | Its `arda-society` record, or `null` when the society layer has none. |

### SettlementNpcs

| Field | Type | Required | Notes |
|---|---|---|---|
| settlement_id | `String` | yes | The settlement, a decimal string. |
| npcs | `Npc[]` | yes | Its stored notables with their sheets. |

### RealmList

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Version of this body (1). |
| realms | `RealmSummary[]` | yes | Realms in id order, without borders. |

### RealmSummary

| Field | Type | Required | Notes |
|---|---|---|---|
| id, seat | `String` | yes | Decimal strings. |
| name, name_gloss, culture | `String` | yes | |
| settlements, neighbours | `String[]` | yes | Decimal ids. |
| population, land_cells | `u64` | yes | |
| style, rank, government_label | `Option<String>` | no | Absent before `arda society build`. |
| ruler | `Option<RealmRuler>` | no | `{name, title}`; absent before `arda society build`. |

### RealmDetail

| Field | Type | Required | Notes |
|---|---|---|---|
| record | `arda_settle::realms::Realm` | yes | With borders. |
| politics | `Option<arda_society::RealmSociety>` | no | `null` before `arda society build`. |
| relations | `Relation[]` | yes | Those involving the realm. |
| events | `Event[]` | yes | History events listing the realm. |
| reigns | `Reign[]` | yes | The realm's reigns. |

### MappingFile

| Field | Type | Required | Notes |
|---|---|---|---|
| format | `String` | yes | Must be `FORMAT`. |
| version | `u32` | yes | Must be `VERSION`. |
| name | `String` | yes | Short name, sent back in the `X-Arda-Sheet-Mapping` header (`[A-Za-z0-9._-]`, 1–64 characters). |
| description | `String` | no | Free text for humans.; serde default |
| base | `Base` | yes | What the output starts from: `copy` (the Arda NPC) or `empty` (`{}`). |
| tables | `BTreeMap<String, BTreeMap<String, Value>>` | no | Named lookup tables for `table` and `keys` conversions.; serde default |
| rules | `Rule[]` | no | Rules, applied in order; a later rule overwrites an earlier one.; serde default |
| drop | `String[]` | no | Output pointers removed after the rules (missing ones are ignored).; serde default |

### Base

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: copy, empty |

### Rule

| Field | Type | Required | Notes |
|---|---|---|---|
| to | `String` | yes | JSON Pointer (RFC 6901) into the output. |
| from | `Option<String>` | no | JSON Pointer into the source NPC (`""` is the whole source).; serde default |
| const | `Option<Value>` | no | A constant value.; serde default; Rust field `constant` |
| template | `Option<String>` | no | A string with `{/pointer}` placeholders into the source.; serde default |
| fields | `Option<Vec<Rule>>` | no | Sub-rules applied to the `from` value: once to an object, or to every element of an array (giving an array).; serde default |
| where | `Option<BTreeMap<String, Value>>` | no | With `fields` over an array: keep only elements whose pointers equal these values.; serde default; Rust field `filter` |
| append | `bool` | no | Append the array to an array already at `to` instead of replacing it.; serde default |
| convert | `Op[]` | no | Conversions applied in order to the value before it is written.; serde default |
| optional | `bool` | no | A missing or `null` `from` skips the rule (or writes `default`) instead of failing.; serde default |
| default | `Option<Value>` | no | With `optional`: the value written when `from` is missing or `null`.; serde default |

### Unknown

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: error, keep, null |

### Case

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: lower, upper, title, kebab, snake |

### Op

| Field | Type | Required | Notes |
|---|---|---|---|
| table | `{table: String, unknown: Unknown}` | no | variant (internally tagged `op`); Enum conversion: a string (or number, by its decimal text) looked up in `tablestable`. |
| keys | `{table: String, unknown: Unknown}` | no | variant (internally tagged `op`); Renames the keys of an object through `tablestable` (the table's values must be strings). |
| scale | `{factor: f64, offset: f64, round: Option<u8>}` | no | variant (internally tagged `op`); Unit conversion of a number: `value × factor + offset`, then rounded to `round` decimals (0 gives an integer) when given. |
| format | `{pattern: String}` | no | variant (internally tagged `op`); Formats a scalar into `pattern`, replacing each `{}`. |
| case | `{to: Case}` | no | variant (internally tagged `op`); Changes the case of a string. |
| replace | `{find: String, with: String}` | no | variant (internally tagged `op`); Replaces every occurrence of `find` in a string. |
| join | `{sep: String}` | no | variant (internally tagged `op`); Joins an array of scalars into one string. |
| split | `{sep: String}` | no | variant (internally tagged `op`); Splits a string into an array of strings (empty input gives `[]`). |
| to_string | `unit` | no | variant (internally tagged `op`); A scalar as its JSON text (strings unchanged). |
| to_number | `unit` | no | variant (internally tagged `op`); A numeric string as a number (numbers unchanged). |
| each | `{convert: Vec<Op>}` | no | variant (internally tagged `op`); Applies `convert` to every element of an array. |

### Block (arda-server)

| Field | Type | Required | Notes |
|---|---|---|---|
| layout | `TacticalLayout` | yes | What the compositor draws; its `origin` is the world square of its top-left square (I10). |
| rules | `Option<RulesSidecar>` | no | Per-square rules, `arda-scene` format 2, the layout's size. |
| origin | `[i64; 2]` | yes | The layout's top-left square in world squares (64 per cell, I2). |
| meta | `BTreeMap<String, String>` | yes | Provenance, for example `source`, `relaxed` and `overlays`. |

### TacticalLayouts

| Field | Type | Required | Notes |
|---|---|---|---|
| world_seed | `String` | yes | Seed of the served world (a decimal string); part of every render seed and cache key. |
| library_version | `String` | yes | `library_version` of the loaded catalogue; part of every render cache key. |
| ppsq_options | `u32[]` | yes | Output pixels per square clients may request. |
| default_ppsq | `u32` | yes | Default output pixels per square. |
| layouts | `TacticalLayoutSummary[]` | yes | Layouts in name order. |

### TacticalLayoutSummary

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Name used in `/v1/tactical/layout/{name}`. |
| width | `u32` | yes | Width in 5-ft squares. |
| height | `u32` | yes | Height in 5-ft squares. |
| tiles | `TacticalTilesDto` | yes | Tile pyramid geometry at the default ppsq. |

### TacticalTilesDto

| Field | Type | Required | Notes |
|---|---|---|---|
| tile_px | `u32` | yes | Tile edge in pixels (512). |
| ppsq | `u32` | yes | Output pixels per square of the render the pyramid is cut from. |
| image_width_px | `u32` | yes | Full-resolution render width, pixels. |
| image_height_px | `u32` | yes | Full-resolution render height, pixels. |
| max_zoom | `u32` | yes | Deepest zoom; it is the render at full resolution. |
| format | `String` | yes | Tile file extension: `webp`. |

### TacticalBlockDto

| Field | Type | Required | Notes |
|---|---|---|---|
| tactical_format | `u32` | yes | Envelope version of this body (`TACTICAL_FORMAT`). |
| layout_schema | `u32` | yes | `TacticalLayout` schema version (`LAYOUT_SCHEMA`). |
| origin | `[i64; 2]` | yes | Top-left square in world squares (64 per cell); equals `layout.origin`. |
| size | `[u32; 2]` | yes | Width and height in squares. |
| render_seed | `String` | yes | Compositor seed of the images (a decimal string); the scene resolves asset queries with the same seed. |
| layout | `TacticalLayoutDto` | yes | What the compositor draws. |
| rules | `Option<super::rules_dto::RulesSidecarDto>` | no | Per-square rules, `arda-scene` `RulesSidecar` format 2, or `null`. |
| meta | `std::collections::BTreeMap<String, String>` | yes | Provenance: `source`, `relaxed`, `overlays`, `demo_at`, …. |
| scene | `Option<serde_json::Value>` | no | Scene data from the same layout (`arda-scene` `Scene`, format 1: movement, climb, cover, obscurement, walls, lights, regions, spawn hints), or `null` when not requested. |
| tiles | `TacticalTilesDto` | yes | Tile pyramid geometry at the requested ppsq. |

### TacticalLayoutDto

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | Layout name. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| squares | `SquareDto[]` | yes | Row-major squares, `width × height` of them. |
| walls | `WallSegmentDto[]` | no | Wall, door, window and gate segments on square edges.; serde default |
| placements | `PlacementDto[]` | no | Props and vegetation.; serde default |
| lights | `LightSourceDto[]` | no | Free-standing light sources.; serde default |
| origin | `Option<[i64; 2]>` | no | World square of square `(0, 0)` (I10); absent for layouts with no world position.; serde default; omitted when empty/none |

### SquareDto

| Field | Type | Required | Notes |
|---|---|---|---|
| ground | `String` | yes | Ground type key, matching a texture's `ground` field. |
| elevation_ft | `i16` | no | Elevation in feet.; serde default |
| water_depth_ft | `u8` | no | Water depth in feet; zero is dry land.; serde default |
| dryness | `u8` | no | Climate dryness, `0` (lush) to `255` (parched steppe); omitted when zero.; serde default; omitted when empty/none |

### EdgeAxisDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: horizontal, vertical |

### WallRoleDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: run, door, window, gate, corner, tee, cross, end, post |

### AssetClassDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: ground, wall, prop, vegetation, water |

### WallSegmentDto

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxisDto` | yes | Which edge. |
| kind | `WallRoleDto` | yes | Edge role. |
| kit | `String` | yes | Wall kit name. |
| tags | `Option<Vec<String>>` | no | Free edge tags: `locked` and `secret` on doors.; serde default; omitted when empty/none |

### AssetRefDto

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `(String)` | no | variant (externally tagged); A specific asset id. |
| query | `{class: Option<AssetClassDto>, tags: Vec<String>}` | no | variant (externally tagged); Any asset of the class carrying every tag. |

### PlacementDto

| Field | Type | Required | Notes |
|---|---|---|---|
| asset | `AssetRefDto` | yes | What to place. |
| x | `f32` | yes | Anchor position in squares (`3.5` is a square centre). |
| y | `f32` | yes | Anchor position in squares. |
| rotation | `u16` | no | Clockwise rotation in degrees.; serde default |
| mirror | `bool` | no | Horizontal mirror, applied before rotation.; serde default |

### LightSourceDto

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f32` | yes | Position in squares. |
| y | `f32` | yes | Position in squares. |
| radius_ft | `u16` | yes | Bright-light radius in feet. |
| colour | `[u8; 3]` | yes | RGB colour. |

### TacticalLibraryDto

| Field | Type | Required | Notes |
|---|---|---|---|
| library | `String` | yes | Library name. |
| library_version | `String` | yes | `library_version`; part of every render cache key. |
| pixels_per_square | `u32` | yes | Art resolution in pixels per square. |
| grounds | `GroundKeyDto[]` | yes | Ground keys a `Square.ground` may use, sorted. |
| wall_kits | `WallKitDto[]` | yes | Wall kits a `WallSegment.kit` may use, sorted. |
| assets | `LibraryAssetDto[]` | yes | Placeable assets (props and vegetation), sorted by id. |

### GroundKeyDto

| Field | Type | Required | Notes |
|---|---|---|---|
| key | `String` | yes | The key, e.g. |
| variants | `u32` | yes | Texture variants. |
| water | `bool` | yes | Whether the texture is a water texture. |

### WallKitDto

| Field | Type | Required | Notes |
|---|---|---|---|
| kit | `String` | yes | Kit name, e.g. |
| roles | `String[]` | yes | Piece roles (`run`, `door`, `window`, `gate`, joints), sorted. |

### LibraryAssetDto

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `String` | yes | Asset id for `AssetRef::Id`. |
| class | `String` | yes | `prop` or `vegetation`. |
| layer | `String` | yes | Draw layer (`floor`, `prop`, `canopy`, …). |
| footprint | `[u32; 2]` | yes | Footprint in squares, `[w, h]`. |
| rotations | `u16[]` | yes | Allowed rotations in degrees. |
| tags | `String[]` | yes | Tags as `key:value` strings for tag queries (I8), sorted. |
| cover | `String` | yes | SRD cover it grants. |
| blocks_sight | `bool` | yes | Blocks sight. |
| blocks_movement | `bool` | yes | Blocks movement. |
| difficult_terrain | `bool` | yes | Makes its squares difficult terrain. |

### PrefetchRequest

| Field | Type | Required | Notes |
|---|---|---|---|
| gx | `u32` | yes | Centre cell column. |
| gy | `u32` | yes | Centre cell row. |
| radius | `Option<u32>` | no | Rings of neighbours to warm, 1–2 (default 1: the 8 neighbours).; serde default |
| ppsq | `Option<u32>` | no | Pixels per square of the images to warm (the cell route's options; default 128).; serde default |
| demo_overlays | `Option<bool>` | no | Warm the demo-overlay blocks instead (`?demo_overlays=1`).; serde default |

### PrefetchStatus

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: queued, pending, dropped |

### PrefetchCell

| Field | Type | Required | Notes |
|---|---|---|---|
| gx | `u32` | yes | Cell column. |
| gy | `u32` | yes | Cell row. |
| status | `PrefetchStatus` | yes | Queue outcome. |
| image | `bool` | yes | Whether its image pyramid warms too (else JSON and blocks only). |

### PrefetchAccepted

| Field | Type | Required | Notes |
|---|---|---|---|
| radius | `u32` | yes | The radius used. |
| ppsq | `u32` | yes | The ppsq warmed. |
| cells | `PrefetchCell[]` | yes | The neighbours inside the world, nearest ring first, row-major. |
| queue_len | `u32` | yes | Cells waiting after this request. |

### CoverLevelDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: none, half, three_quarters, total |

### EdgeRoleDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: parapet, retaining, building, boundary, gate |

### RulesCellDto

| Field | Type | Required | Notes |
|---|---|---|---|
| difficult | `Option<bool>` | no | Difficult terrain.; serde default; omitted when empty/none |
| water_depth_ft | `Option<u8>` | no | Water depth in feet: under 5 wades, 5 or more swims (I15).; serde default; omitted when empty/none |
| cover | `Option<CoverLevelDto>` | no | Cover the square's contents grant.; serde default; omitted when empty/none |
| blocks_sight | `Option<bool>` | no | Heavily obscured.; serde default; omitted when empty/none |
| lightly_obscured | `Option<bool>` | no | Lightly obscured.; serde default; omitted when empty/none |
| blocks_movement | `Option<bool>` | no | Impassable.; serde default; omitted when empty/none |
| deck | `Option<bool>` | no | A walkable deck (bridge, jetty) over the square.; serde default; omitted when empty/none |
| ext | `Option<BTreeMap<String, serde_json::Value>>` | no | Layer extras: `feature`, `road_class`, `deck_elevation_ft` (ways), `crop`, `field`, `furrow` (fields), `building` (town).; serde default; omitted when empty/none |

### EdgeRuleDto

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxisDto` | yes | Which edge of the square. |
| role | `EdgeRoleDto` | yes | Why it is there. |
| blocks_movement | `bool` | yes | Blocks movement across the edge. |
| blocks_sight | `bool` | yes | Blocks sight across the edge. |
| cover | `CoverLevelDto` | no | Cover it grants.; serde default |

### RulesSidecarDto

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Always 2. |
| width | `u32` | yes | Width in squares (the layout's). |
| height | `u32` | yes | Height in squares. |
| squares | `RulesCellDto[]` | yes | Row-major cells. |
| edges | `EdgeRuleDto[]` | no | Low edge features that are not wall-kit walls (always present on the wire; the domain JSON omits an empty list).; serde default |

### SceneTime

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: day, night |

### TacticalScene

| Field | Type | Required | Notes |
|---|---|---|---|
| tactical_format | `u32` | yes | Envelope version (`SCENE_FORMAT`). |
| origin_gs | `[i64; 2]` | yes | World square of the block's top-left square. |
| time | `SceneTime` | yes | `day` or `night`. |
| scene | `arda_scene::Scene` | yes | The `arda-scene` scene (format 1), described by `SceneDto`. |
| tokens | `Token[]` | yes | NPC tokens. |

### Runs

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `(u32, T)[]` | yes | — |

### SqDto

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u32` | yes | — |
| 1 | `u32` | yes | — |

### MovementDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: normal, difficult, wade, swim, impassable |

### ObscurementDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: clear, light, heavy |

### WallKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: wall, door, window, gate, secret |

### SceneWallDto

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `WallKindDto` | yes | Feature type. |
| open | `Option<bool>` | no | Open state of doors, gates and secret doors; absent otherwise.; serde default; omitted when empty/none |
| points | `[u32; 2][]` | yes | Grid vertices `[x, y]`, at least two. |
| blocks_sight | `bool` | yes | Blocks line of sight in its current state. |
| blocks_movement | `bool` | yes | Blocks movement in its current state. |
| blocks_light | `bool` | yes | Blocks light in its current state. |
| cover | `CoverLevelDto` | yes | Cover it grants when it does not block sight. |
| kit | `String` | yes | Wall kit name. |
| locked | `Option<bool>` | no | `true` on a locked door, gate or secret door; absent otherwise.; serde default; omitted when empty/none |

### BlockerKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: canopy, prop |

### VisionBlockerDto

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `BlockerKindDto` | yes | Source class. |
| asset | `String` | yes | Asset id. |
| obscurement | `ObscurementDto` | yes | `light` for foliage, `heavy` for opaque. |
| polygon | `[f32; 2][]` | yes | Closed polygon in squares, clockwise in y-down coordinates. |

### SceneLightDto

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `f32` | yes | Position in squares. |
| y | `f32` | yes | Position in squares. |
| bright_ft | `u16` | yes | Bright-light radius in feet. |
| dim_ft | `u16` | yes | Outer edge of dim light in feet. |
| colour | `[u8; 3]` | yes | RGB colour. |
| asset | `Option<String>` | no | Emitting asset id; absent for a free layout light.; serde default; omitted when empty/none |

### RegionKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: difficult, shallow_water, deep_water |

### RegionDto

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `RegionKindDto` | yes | What the region marks. |
| rings | `[u32; 2][][]` | yes | Closed rings of `[x, y]` vertices (first point not repeated). |

### EdgeDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: N, E, S, W |

### EdgeExitDto

| Field | Type | Required | Notes |
|---|---|---|---|
| edge | `EdgeDto` | yes | Which edge. |
| from | `SqDto` | yes | First square of the run. |
| to | `SqDto` | yes | Last square of the run (inclusive). |

### EntranceDto

| Field | Type | Required | Notes |
|---|---|---|---|
| wall | `u32` | yes | Index into `walls`. |
| squares | `SqDto[]` | yes | Enterable squares on either side of it. |

### SpawnHintsDto

| Field | Type | Required | Notes |
|---|---|---|---|
| open | `SqDto[]` | yes | Open normal-movement squares, row-major. |
| entrances | `EntranceDto[]` | yes | Squares by doors and gates. |
| exits | `EdgeExitDto[]` | yes | Enterable runs along the map edges, N, E, S, W order. |

### TokenSlotDto

| Field | Type | Required | Notes |
|---|---|---|---|
| npc_id | `String` | yes | NPC id, a decimal string. |
| x | `f32` | yes | Token centre in squares (`3.5` is a square centre). |
| y | `f32` | yes | Token centre in squares. |
| facing | `u16` | yes | Facing in degrees clockwise from north, `0..360`. |

### SceneDto

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Schema version (1). |
| name | `String` | yes | Layout name. |
| width | `u32` | yes | Width in squares. |
| height | `u32` | yes | Height in squares. |
| seed | `String` | yes | Seed the asset queries were resolved with, a decimal string. |
| library | `String` | yes | Library name. |
| library_version | `String` | yes | Library art version. |
| movement | `Runs<MovementDto>` | yes | Per-square movement. |
| climb | `Runs<u8>` | yes | Per-square climb mask: bit `d` (0 N, 1 NE, … 7 NW) set means the step to that neighbour crosses 10 ft or more and needs climbing. |
| cover | `Runs<CoverLevelDto>` | yes | Per-square cover granted by what stands in the square. |
| obscured | `Runs<ObscurementDto>` | yes | Per-square obscurement. |
| elevation_ft | `Runs<i16>` | yes | Per-square elevation in feet. |
| water_depth_ft | `Runs<u8>` | yes | Per-square water depth in feet. |
| walls | `SceneWallDto[]` | yes | Wall, door, window, gate and secret-door polylines. |
| vision_blockers | `VisionBlockerDto[]` | yes | Canopy and prop polygons that obscure vision. |
| lights | `SceneLightDto[]` | yes | Light sources. |
| regions | `RegionDto[]` | yes | Difficult-terrain and water regions. |
| spawn_hints | `SpawnHintsDto` | yes | Token placement hints. |
| tokens | `TokenSlotDto[]` | yes | NPC token slots. |
| origin_gs | `Option<[i64; 2]>` | no | World square of square `(0, 0)` (I17): present in `TacticalBlockDto.scene`, absent in `TacticalScene.scene`, whose envelope carries it.; serde default; omitted when empty/none |

### TokenKind

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: worker, resident |

### Token

| Field | Type | Required | Notes |
|---|---|---|---|
| npc_id | `String` | yes | The NPC, a decimal string (resolve it with `GET /v1/npc/{id}`). |
| name | `String` | yes | Display name. |
| x | `u32` | yes | Square column within the block. |
| y | `u32` | yes | Square row within the block. |
| building_id | `String` | yes | The building it stands in, a decimal string. |
| settlement_id | `String` | yes | Its settlement, a decimal string. |
| kind | `TokenKind` | yes | `worker` (at its workplace) or `resident` (at home). |

### DungeonDto

| Field | Type | Required | Notes |
|---|---|---|---|
| params | `DungeonParamsDto` | yes | The parameters it was made from. |
| layout | `TacticalLayoutDto` | yes | The battle map; `POST /render` takes it as it is. |
| rules | `RulesSidecarDto` | yes | Per-square rules: rock impassable and opaque, scree difficult. |
| rooms | `DungeonRoomDto[]` | yes | Rooms (one `cavern` spanning the map for caves). |
| doors | `DungeonDoorDto[]` | yes | Every door, including locked and secret ones. |
| exits | `DungeonExitDto[]` | yes | Stairs and mouths. |

### DungeonParamsDto

| Field | Type | Required | Notes |
|---|---|---|---|
| seed | `String` | yes | Seed, a decimal string. |
| kind | `DungeonKindDto` | yes | Dungeon or cave. |
| width | `u32` | yes | Width in squares (16–160). |
| height | `u32` | yes | Height in squares (16–160). |

### DungeonKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: dungeon, cave |

### RoomPurposeDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: entrance, crypt, barracks, storage, shrine, prison, treasure, hall, cavern |

### DungeonRoomDto

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | Index in `rooms`. |
| purpose | `RoomPurposeDto` | yes | What it is for. |
| x | `u32` | yes | Left column. |
| y | `u32` | yes | Top row. |
| w | `u32` | yes | Width in squares. |
| h | `u32` | yes | Height in squares. |

### DungeonDoorDto

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | Square column. |
| y | `u32` | yes | Square row. |
| axis | `EdgeAxisDto` | yes | Which edge. |
| locked | `bool` | yes | Needs a key or a check. |
| secret | `bool` | yes | Looks like wall until found. |

### DungeonExitKindDto

| Field | Type | Required | Notes |
|---|---|---|---|
| value | enum | yes | accepted: stairs_up, stairs_down, mouth |

### DungeonExitDto

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `DungeonExitKindDto` | yes | Kind. |
| x | `u32` | yes | Column. |
| y | `u32` | yes | Row. |

### SchemaIndex

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | Version of this body (`INDEX_FORMAT` = 1). |
| api_version | `String` | yes | The API version (`v1`). |
| dialect | `String` | yes | The JSON Schema dialect every file declares (draft 2020-12). |
| note | `String` | yes | How `--sheet-mapping` affects the `Npc` schemas. |
| schemas | `SchemaIndexEntry[]` | yes | Every schema, in name order. |

### SchemaIndexEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| name | `String` | yes | File stem and TypeScript type name. |
| url | `String` | yes | Where it is served: `/v1/schema/{name}.json`. |
| routes | `String[]` | yes | The routes (or body parts, such as `TacticalBlockDto.scene`) it describes. |


## Relationships

### Saved world (arda-core, arda-gen, arda-render, arda)

`World` (`crates/arda/src/world.rs:19`) owns its `Manifest`, a derived `HydrologyDomain` and two independent `BTreeMap<AreaCoord, OnceLock<…>>` caches. `World::load` (`crates/arda/src/world.rs:34`) fills empty slots only; `area()` caches a requested `Area` (`crates/arda/src/world.rs:210`), `read_area()` returns owned uncached data (`crates/arda/src/world.rs:226`), and the first `block()` request decodes that area's whole `BlockArchive` (`crates/arda/src/world.rs:364`). Each `Area` owns cells, objects and the optional `AreaWater` forms (`crates/arda/src/world/area.rs:6`); `World::shore` reads the optional `ShoreLayer` (`crates/arda/src/world.rs:183`) and `World::fine_terrain` the canonical fine lattice (`crates/arda/src/world.rs:139`).

`AreaWater.segments` and `.lakes` are positional: one `SegmentForm` per `RiverSegment` and one `LakeForm` per `Lake`, in the area's object order (`crates/arda-core/src/water.rs:390`).

`BasinNodeRow.parent` and child-table spans describe physical containment, independently of annual wet occupancy. `GlobalLake.basin` is shared by every local `Lake.global_id` fragment; `GlobalReach.id` is shared by `RiverSegment.global_id` and by `SharedCrossing.reach`. `AnnualCatchment` keeps original contributing ownership separate from final drainage. `ReceivingAccount::Lake` can name a dry physical basin; `representative_lake` names actual positive-depth water. `SharedCrossing` is a directed D8 adjacency across areas whose undirected `CrossingId` sorts endpoints by `(x,y)`. `JunctionId` names a dry coordinate without a table (`crates/arda-core/src/hydrology.rs:32`); tagged point reaches represent physical terminals (`crates/arda-core/src/hydrology.rs:28`). `AreaHydrologyContext` carries the global records one area needs, including neighbouring channel halos; reading an area does not fetch referenced downstream identities outside it (`crates/arda-core/src/hydrology.rs:333`). Local fragment IDs are `u32`; continent river IDs stay continent-local `u16` (`crates/arda-core/src/objects.rs:49`, `crates/arda-core/src/continent.rs:70`).

Published generation runs accepted `Continent` → one evolved `SharedTerrain` → immutable `PreparedTerrain` slices → shared routing and annual support → per-area composition through `SolvedCell` (`crates/arda-gen/src/area/prepare.rs:12`, `crates/arda-gen/src/hydrology/types.rs:31`, `crates/arda-gen/src/area/shared_compose.rs:15`). `TileBundle`, `WaterGrid`, `Basin` and `Filled` remain local diagnostic data, not published water authority. `Plate` is transient generation state.

`Cell.road` and `Cell.built_by` keep their defaults in generation; no generator stage writes them (`crates/arda-core/src/cell.rs:137`). Roads, land-use ownership and realms live in separate `society/` rasters that the server overlays onto each `CellSample` (`crates/arda-server/src/contract/convert.rs:169`).

### Settlements, roads, realms, names (arda-settle)

`Settlement.realm_id` → `Realm.id`; `Realm.seat` and `Realm.settlements[]` → `Settlement.id`; `Realm.neighbours[]` → `Realm.id` (`crates/arda-settle/src/model.rs:87`, `crates/arda-settle/src/realms.rs:38`). `Road.from` and `Road.to` → `Settlement.id`, with `Road.to_edge` naming a map edge when `to` is absent (`crates/arda-settle/src/roads.rs:27`). `Pass.road_id` → `Road.id` (`crates/arda-settle/src/crossings.rs:56`). `Crossing` carries its road class and river name but no road id (`crates/arda-settle/src/crossings.rs:32`). `Settlement.tongue` is the `arda-names` `Tongue` its name was drawn in (`crates/arda-settle/src/model.rs:133`). Every id is an `arda-ids` newtype (`SettlementId`, `RealmId`), 1-based in placement order.

The rasters `landuse.bin` (code + owning settlement id per cell), `realms.bin` (realm id per cell, 0 on water) and `roads.bin` (road code per cell) are aligned to the 100 m world cell grid stitched by `Grid` (`crates/arda-settle/src/grid.rs:27`, `crates/arda-settle/src/output.rs:245`, `crates/arda-settle/src/output.rs:294`, `crates/arda-settle/src/output.rs:340`).

### Society, history, offices, factions (arda-society)

`Society` (`crates/arda-society/src/lib.rs:110`) composes `RealmSociety[]`, realm-pair `Relation[]`, `SettlementSociety[]`, `Economy` and `History`. Each `SettlementSociety` references its settlement and realm by id and owns its `BuildingRef[]`, `Faction[]`, `FactionRelation[]`, `NpcRole[]` (offices: notable slots), `HistoryHook[]` and `Hook[]` (`crates/arda-society/src/lib.rs:61`). `NpcRole.building` → `BuildingRef.id`, `NpcRole.faction` → `Faction.id`, `Faction.leader_role` → `NpcRole.id` (`crates/arda-society/src/roles.rs:27`, `crates/arda-society/src/politics/factions.rs:35`). `Event.cause` → `Event.id` (`crates/arda-society/src/history/mod.rs:78`). Cross-entity references in hooks and events use the internally tagged `EntityRef` (settlement, realm, road, building, faction, role, ruin, event, dynasty, good) (`crates/arda-society/src/refs.rs:10`). Input records (`WorldSettlements`, `crates/arda-society/src/input.rs:272`) mirror `arda-settle`'s files field for field.

### People: NPCs, households, sheets, ranks (arda-npc, arda-people, arda-ids)

`Population` (`crates/arda-npc/src/population.rs:51`) owns `Household[]`, the stored notable `Npc[]` and a `RosterEntry[]` with one entry per inhabitant; `RosterEntry.job` indexes `Population.job_keys`. `Npc.household` → `Household.id`; `Household.members[]` → `Npc.id`; `Npc.home_building` and `Npc.workplace_building` → `BuildingSpec.id`; `Npc.employer` → `Npc.id`; `Relationship.other` → `Npc.id`; `Npc.sheet` is an owned `Sheet`; `Npc.social_rank` is the `SocialRank` enum (`crates/arda-npc/src/npc.rs:292`). `NpcId::from_parts(settlement, building, index)` derives a world-unique id from the `arda-ids` subseed (`crates/arda-ids/src/ids.rs:116`).

The society's `NpcRole` becomes an `arda-npc` `NotableSlot` in `arda_people::slot` (`crates/arda-people/src/people.rs:281`), and `Generator::with_notables` binds the slots to inhabitants (`crates/arda-npc/src/lib.rs:74`). `StoredNotable` groups one settlement's notables with the source of its buildings (`plan` or `mix`) (`crates/arda-people/src/files.rs:22`). Commoners are never stored: `World::population` regenerates them from the seed, profile, buildings and slots (`crates/arda-people/src/people.rs:253`).

### Town plans (arda-town)

`TownPlan` (`crates/arda-town/src/plan/types.rs:293`) owns `Street[]`, `Bridge[]`, an optional `MarketSquare`, `Plot[]`, districts as `(DistrictKind, PlotId[])`, optional `TownWall` and `CastleWard`, `Building[]` and `Fabric`. `Plot.street` → `Street.id`; `Building.plot` → `Plot.id`; `Building.doors[]` are owned `Door` values. `TownPlan.site` is the settlement id. Plan buildings become `arda-npc` `BuildingSpec`s in `plan_specs` (`crates/arda-people/src/town.rs:175`) and `arda-society` `BuildingSpec`s in `society_buildings` (`crates/arda-people/src/town.rs:220`); town tactical squares carry `ext.building` with the plan building id as a string (`crates/arda-town/src/block/mod.rs:526`).

### Tactical layout, blocks, scene, tokens, lights

`TacticalLayout` (`crates/arda-tactical/src/layout.rs:15`) owns `Square[]` (row-major, `width × height`), `WallSegment[]`, `Placement[]`, `LightSource[]` and an optional world `origin`. `Placement.asset` is an `AssetRef`: an `Asset.id` of the loaded `Catalog` or a deterministic class/tag query (`crates/arda-tactical/src/layout.rs:141`); `Square.ground` names a catalogue ground key. `RulesSidecar` (`crates/arda-scene/src/sidecar.rs:37`) is parallel to the layout: `squares[]` in the same row-major order plus edge rules.

`arda-refine` produces one `Block` per 100 m cell (64 × 64 squares) and a `Map` of `TacticalLayout` + `RulesSidecar` + `MapMeta` (`crates/arda-refine/src/block.rs:35`, `crates/arda-refine/src/output.rs:102`). `arda-blocks` `Pipeline` caches refined blocks and composes overlay layers (ways, then fields, then town) by `Owner` precedence into `Composed` (`crates/arda-blocks/src/pipeline.rs:23`, `crates/arda-blocks/src/overlays.rs:28`, `crates/arda-blocks/src/overlays.rs:296`). `SocietyOverlays` fills those layers from `society/` (`crates/arda-blocks/src/society/mod.rs:91`).

`build_scene` turns a layout, its library and an optional sidecar into a `Scene` (`crates/arda-scene/src/lib.rs:51`); `Scene.lights` are derived from the layout and its placed assets (`crates/arda-scene/src/lib.rs:89`); `Scene.tokens` are `TokenSlot`s. The server wraps one scene and its `Token[]` in `TacticalScene` (`crates/arda-server/src/tactical/scene.rs:37`); each `Token` names a stored notable (`npc_id`), its settlement and plan building, placed at the workplace by day and at home by night (`crates/arda-server/src/tactical/tokens.rs:79`). The server's internal `Block` (`crates/arda-server/src/tactical/block.rs:20`) is the source of `TacticalBlockDto`.

### Dungeon, midzoom, art import

`Dungeon` owns an ordinary `TacticalLayout`, a `RulesSidecar`, `Room[]`, `Door[]` and `Exit[]`, all a function of `Params` (`crates/arda-dungeon/src/lib.rs:197`, `crates/arda-dungeon/src/lib.rs:227`). `ReliefWorld` combines `WorldTerrain`, an `arda-refine` `WorldSource` for water geometry and an optional `Landscape` that wraps `arda-blocks` `SocietyOverlays` (`crates/arda-midzoom/src/world.rs:22`, `crates/arda-midzoom/src/land/mod.rs:54`). `arda-art-import` reads `Manifest` (`import.toml`) and writes an `arda-tactical` `Catalog` plus a `Report` (`crates/arda-art-import/src/manifest.rs:36`, `crates/arda-art-import/src/report.rs:58`).

### Wire DTOs (arda-server)

`TacticalBlockDto` composes `TacticalLayoutDto`, optional `RulesSidecarDto`, a scene value described by `SceneDto` and `TacticalTilesDto` (`crates/arda-server/src/tactical/dto.rs:60`). `NpcPage` owns `NpcEntry[]`, each wrapping an `Npc` (`crates/arda-server/src/npcs/dto.rs:32`). `SettlementList.settlements[]` and `SettlementDetail.record` are `arda-settle` `Settlement`; `SettlementDetail.society` is `SettlementSociety`; `SettlementNpcs.npcs[]` is `Npc` (`crates/arda-server/src/schema/bodies.rs:11`). `ApiError` wraps `ApiErrorBody`; `NotYetError` adds `planned_source` (`crates/arda-server/src/error.rs:72`, `crates/arda-server/src/error.rs:88`). `PrefetchAccepted.cells[]` are `PrefetchCell`s with a `PrefetchStatus` (`crates/arda-server/src/tactical/prefetch.rs:78`). `DungeonDto` mirrors `arda_dungeon::Dungeon` with `TacticalLayoutDto` and `RulesSidecarDto` parts (`crates/arda-server/src/tactical/dungeon_dto.rs:19`). `SchemaIndex` owns `SchemaIndexEntry[]` and is built by `index()` (`crates/arda-server/src/schema/mod.rs:153`).

## Boundaries

| Representation | Produced by | Consumed by | Conversion site |
|---|---|---|---|
| Binary world (`world.json`, `areas/`, `continent/`, `hydrology/`, `blocks/`, `terrain/`) | `arda-gen` writers | `arda` `World` | codecs in `crates/arda-core/src/formats/`; `World::load` `crates/arda/src/world.rs:34` |
| `CellSample` / `PointSample` (contract 3) | `arda-server` | HTTP clients, viewer | `cell_sample` `crates/arda-server/src/contract/convert.rs:187`; society overlay `crates/arda-server/src/contract/convert.rs:169`, rasters `crates/arda-server/src/society_cells.rs:47` |
| `AreaCells` JSON / `ARDACOLS` v2 binary | `arda-server` | viewer | `build` `crates/arda-server/src/columnar.rs:262`, `encode` `crates/arda-server/src/columnar.rs:287` |
| Export JSON (`AreaOut` schema 2, `BlockOut`) | `arda-render` | `arda export` files | `crates/arda-render/src/json.rs:66`, `crates/arda-render/src/hydrology_json.rs:145` |
| `society/settlements.json`, `roads.json`, `realms.json`, `names.json`, `stats.json`, three rasters | `arda-settle` domain types | readers below | `encode` `crates/arda-settle/src/lib.rs:61`, `write` `crates/arda-settle/src/lib.rs:145` |
| `arda-society` input view of `society/*.json` | `WorldSettlements::read_dir` | `simulate_society` | `crates/arda-society/src/read.rs:75`, `crates/arda-society/src/lib.rs:134` |
| `SocietyFiles` (typed + raw JSON records) | `arda-people` | town, NPC, server routes | `crates/arda-people/src/files.rs:45`; raw record → `TownSite` `crates/arda-people/src/town.rs:33`; raw record → `SettlementProfile` `crates/arda-people/src/people.rs:201` |
| `society/society.json`, `society/notables.json` | `arda society build` | `arda-people` `World` | written `crates/arda-people/src/build.rs:48`; read `crates/arda-people/src/people.rs:85` |
| settle roads/land use → `arda-ways` / `arda-fields` inputs | `arda-blocks` | overlay layers | `crates/arda-blocks/src/society/layers.rs:26`, `crates/arda-blocks/src/society/layers.rs:99`, `crates/arda-blocks/src/society/layers.rs:282` |
| `arda-npc` domain JSON ↔ `npc_dto` mirrors | handlers serialise domain types | ts-rs / schemars | mirror rule `crates/arda-server/src/npc_dto/mod.rs:1`; page wire form `crates/arda-server/src/npcs/dto.rs:1` |
| `arda-tactical` `TacticalLayout` ↔ `TacticalLayoutDto` | handlers | ts-rs / schemars | `crates/arda-server/src/tactical/dto.rs:1` |
| `arda-scene` `Scene` ↔ `SceneDto`; `RulesSidecar` ↔ `RulesSidecarDto` | handlers | ts-rs / schemars | `crates/arda-server/src/tactical/scene_dto.rs:1`, `crates/arda-server/src/tactical/rules_dto.rs:1` |
| `arda-dungeon` `Dungeon` ↔ `DungeonDto` | handlers | ts-rs / schemars | `crates/arda-server/src/tactical/dungeon_dto.rs:1` |
| Arda NPC JSON → mapped sheet JSON | `--sheet-mapping` `MappingFile` | HTTP clients | `SheetMapping::reshape` `crates/arda-server/src/sheet_map/mod.rs:224` |
| Rust DTOs → TypeScript | ts-rs | `apps/viewer` | `crates/arda-server/src/bindings.rs:81` |
| Rust DTOs → JSON Schema draft 2020-12 | schemars | `GET /v1/schema` | `crates/arda-server/src/schema/mod.rs:98` |

TypeScript bindings. `bindings/ts/arda/` holds one generated `<Name>.ts` per type in the `exported!` list plus `index.ts`, which also exports `CONTRACT_VERSION` and `API_VERSION` (`crates/arda-server/src/bindings.rs:69`, `crates/arda-server/src/bindings.rs:81`). The test `committed_bindings_are_fresh` fails when the committed files differ from ts-rs output; `ARDA_BLESS_BINDINGS=1` rewrites them (`crates/arda-server/src/bindings.rs:205`). `u64_ids_are_strings_not_bigints` asserts no generated file uses `bigint` (`crates/arda-server/src/bindings.rs:238`). Only `arda-server` depends on ts-rs; domain crates stay free of it, which is why the server keeps wire mirrors (`crates/arda-server/src/npc_dto/mod.rs:3`). JSON Schemas in `bindings/schema/` come from the same Rust types through schemars, including the domain types `Settlement`, `SettlementSociety` and `TownPlan` behind their `schema` features (`crates/arda-server/src/schema/mod.rs:98`).

Mirror exceptions. `TacticalBlockDto.scene` is typed `Option<serde_json::Value>` and documented as `SceneDto` in the schema catalog (`crates/arda-server/src/tactical/dto.rs:84`); `TacticalScene.scene` is the domain `arda_scene::Scene` (`crates/arda-server/src/tactical/scene.rs:37`). Settlement routes assemble bodies as `serde_json::Value` from the stored raw records; `SettlementList`, `SettlementDetail` and `SettlementNpcs` exist for schemars only (`crates/arda-server/src/schema/bodies.rs:1`, `crates/arda-server/src/people.rs:51`). The runtime `PopulationRequest (npc.rs runtime)` and `NpcDemo (npc.rs runtime)` embed domain types; their TypeScript form comes from the `npc_dto` mirrors (`crates/arda-server/src/npc.rs:4`).

Ids. Every `arda-ids` id serialises as a decimal JSON string and deserialises from a string or a non-negative integer (`crates/arda-ids/src/ids.rs:1`). `arda-society` writes its u64 fields through `crate::ids::str`; `arda-town` `TownPlan.seed` through `crate::ids` (field notes above).

Core codecs write explicit little-endian fields; Rust padding is never persisted. The owned logical `BasinNode` is distinct from the fixed `BasinNodeRow` plus streamed child table. Export JSON converts global IDs and `u128` litres to decimal strings and keeps `*_milli_cumecs` names in whole L/s; account JSON uses a snake_case `kind` discriminator (`crates/arda-render/src/hydrology_json.rs:7`). `CellOut` omits temperature, rainfall, moisture, forest density, road and built_by (`crates/arda-render/src/json.rs:18`); `CellSample` carries all of them (`crates/arda-server/src/contract/mod.rs:179`).

## Validation

| Data | Rule | Site |
|---|---|---|
| `GenerateConfig` | axes 64–4000 km, density 1–200, ordered latitudes within −80..80; serde alone bypasses it, so `World::load` re-runs the constructor | `crates/arda-core/src/config.rs:73`, `crates/arda/src/world.rs:37` |
| `Manifest` | format major exactly 4; area dimensions must match the validated config; fine recipe 1–8; a recipe-5 manifest beside `terrain/shore.bin` reads as recipe 6 | `crates/arda-core/src/formats/manifest.rs:139`, `crates/arda/src/world.rs:46`, `crates/arda/src/world.rs:53`, `crates/arda-core/src/formats/manifest.rs:156` |
| Hydrology rows | each `structure!` row carries its invariant (canonical crossing pair, spill consistency, annual/mean pairing, model revision 2) | `crates/arda-core/src/formats/hydrology.rs:248`–`crates/arda-core/src/formats/hydrology.rs:386` |
| Continent acceptance | land fraction 250–900 per mille | `crates/arda-gen/src/orchestrator.rs:71` |
| `ImageQuality`, overview sizes | 512–32,768 px; `k`/`K` suffix; exact overview axes 1–32,768 and ≤ 134,217,728 px buffered | `crates/arda-render/src/quality.rs:21`, `crates/arda-render/src/quality.rs:71`, `crates/arda-render/src/overview.rs:87`, `crates/arda-render/src/overview.rs:294` |
| Block archive | decodes any `u16` `TileId`; symbolic rendering refuses unknown vocabulary ids | `crates/arda-core/src/formats/blocks.rs:133`, `crates/arda-render/src/symbolic.rs:11` |
| `society/*.bin` rasters | magic, format version 1, `0 < w·h ≤ 2^28` before allocation, exact decompressed length | `crates/arda-settle/src/output.rs:189` |
| `society/*.json` for society | `format_version` must equal 1 | `crates/arda-society/src/read.rs:56` |
| Server rasters | `landuse.bin`, `realms.bin`, `roads.bin` must match the world's cell dimensions | `crates/arda-server/src/society_cells.rs:47` |
| NPC content tables | compiled-in JSON parsed, then cross-file references (job stat blocks and others) must resolve | `crates/arda-npc/src/data/mod.rs:88` |
| `PopulationRequest` | population ≤ 50,000, buildings ≤ 20,000, body ≤ 2 MiB; unknown fields refused | `crates/arda-server/src/npc.rs:14`, `crates/arda-server/src/npc.rs:97`, `crates/arda-server/src/npc.rs:22` |
| `PrefetchRequest` | radius 1–2 (default 1), queue ≤ 64 cells, workers ≤ 8 | `crates/arda-server/src/tactical/prefetch.rs:31`, `crates/arda-server/src/tactical/prefetch.rs:204` |
| Wire mirrors | `#[serde(deny_unknown_fields)]` on `npc_dto`, `NpcPage`, `Token` and others; tests re-serialise domain JSON through each mirror | `crates/arda-server/src/npc_dto/mod.rs:1` |
| Asset catalogue | validator over catalogue and images; library directory check at load | `crates/arda-tactical/src/validate.rs:121`, `crates/arda-tactical/src/library.rs:89` |
| `TacticalLayout` | sizes, ground keys, wall kits and roles, placements and rotations checked against the library (422 `invalid_layout`) | `crates/arda-tactical/src/layout.rs:264`, `crates/arda-server/src/error.rs:5` |
| `RulesSidecar` | dimensions match the layout | `crates/arda-scene/src/sidecar.rs:236` |
| Art-import manifest | `deny_unknown_fields` on every TOML section | `crates/arda-art-import/src/manifest.rs:36` |
| `--sheet-mapping` file | parsed, checked, then dry-run against probe NPCs | `crates/arda-server/src/sheet_map/mod.rs:158` |

No validation library is used beyond serde attributes; every rule above is hand-written.

## Schema

No database or migration layer exists. Version authorities are separate: world format major 4 (`crates/arda-core/src/formats/mod.rs:20`), hydrology model revision 2 (`crates/arda-core/src/formats/hydrology.rs:19`), export JSON schema 2 (`crates/arda-render/src/json.rs:15`), canonical fine terrain file 1.0 (`crates/arda-core/src/formats/terrain.rs:20`), world contract 3 and API `v1` (`crates/arda-server/src/contract/mod.rs:31`, `crates/arda-server/src/dto.rs:12`), tactical envelope 1 and layout schema 1 (`crates/arda-server/src/tactical/dto.rs:90`), scene format 1 (`crates/arda-scene/src/types.rs:12`), rules sidecar format 2 (`crates/arda-scene/src/sidecar.rs:32`), society files 1 (`crates/arda-settle/src/output.rs:15`, `crates/arda-society/src/lib.rs:56`, `crates/arda-people/src/files.rs:13`), catalogue format 1 (`crates/arda-tactical/src/catalog.rs:10`), refine metadata `arda-refine.meta/2` (`crates/arda-refine/src/rules.rs:13`).

### World directory

| File | Layout | Codec |
|---|---|---|
| world.json | `Manifest` in serde field order; `river_count` defaults to 0 and `fine_terrain` is omitted when absent | `crates/arda-core/src/formats/manifest.rs:67`, `crates/arda-core/src/formats/manifest.rs:98` |
| areas/AX_AY/cells.bin | no header; 262,144 rows × 39 bytes | `crates/arda-core/src/formats/cells.rs:17` |
| areas/AX_AY/objects.bin | `ARDAOBJ\0`, u16 section count; tags rivers=1, lakes=2, channels=3, global=4; river prefix 34 B + 4 B/cell, lake prefix 29 B + 4 B/member | `crates/arda-core/src/formats/area_objects_v4.rs:17`, `crates/arda-core/src/formats/area_objects_v4.rs:24` |
| area objects global context | `ARDACTX4`, model revision, four u32 counts; 28-byte header | `crates/arda-core/src/formats/hydrology.rs:23`, `crates/arda-core/src/formats/hydrology/context.rs:68` |
| areas/AX_AY/water.bin (optional) | `ARDAWTR\0`; v1: four counts, segment 19 B, lake 2 B, delta 12 B, doline 8 B; v2 (recipe 7): five counts, lake 3 B, pan 7 B | `crates/arda-core/src/formats/water.rs:1` |
| continent/overview.bin | `ARDAOVR\0`, width, height, 22-byte `ContinentCell` rows | `crates/arda-core/src/formats/overview.rs:18` |
| continent/objects.bin | `ARDACOB\0` plus tagged continent-river section | `crates/arda-core/src/formats/objects.rs:34` |
| blocks/AX_AY.tiles.zst | zstd frame: `ARDABLK\0`, count; per block cx, cy u16, relaxed u8, 4,096 u16 tile IDs | `crates/arda-core/src/formats/blocks.rs:14` |
| terrain/fine.bin (optional) | 88-byte header (`ARDATERR`, major/minor, finalized, origin µm, spacing/width/height, payload length, BLAKE3) then row-major i32 mm | `crates/arda-core/src/formats/terrain.rs:1` |
| terrain/shore.bin (optional) | `ARDASHOR`, major/minor, spacing, width, height, run/island/landform counts; runs 11 B, islands 23 B, landforms 24 B; trailing BLAKE3 | `crates/arda-core/src/formats/shore.rs:1` |

Every `hydrology/*.bin` begins with the 32-byte header `ARDAHYD4`, kind u32, record_bytes u32, count u64, payload_bytes u64; keyed tables require unique ascending keys (`crates/arda-core/src/formats/hydrology.rs:22`, `crates/arda-core/src/formats/hydrology.rs:25`, `crates/arda-core/src/formats/hydrology/table.rs:127`). Field sequences come from `structure!` (`crates/arda-core/src/formats/hydrology.rs:316`–`crates/arda-core/src/formats/hydrology.rs:386`); files are streamed by `global_output::write` (`crates/arda-gen/src/orchestrator/global_output.rs:66`); row widths are asserted in `crates/arda-core/src/formats/hydrology/tests.rs:108`.

| Table | Kind | Row bytes | Row / ordering |
|---|---|---|---|
| hydrology/basins.bin | 2 | 76 | `BasinNodeRow`; `BasinId` |
| hydrology/children.bin | 3 | 8 | `BasinId`; within each parent `TableSpan` |
| hydrology/lakes.bin | 4 | 75 | `GlobalLake`; `BasinId` |
| hydrology/reaches.bin | 5 | 69 | `GlobalReach`; `ReachId` |
| hydrology/crossings.bin | 6 | 85 | `SharedCrossing`; (low.x, low.y, high.x, high.y) |
| hydrology/catchments.bin | 12 | 78 | `AnnualCatchment`; `CatchmentId` |
| hydrology/metadata.bin | 1 | 172 | one `HydrologyMetadata` |

### society/ directory

Written by `arda settle` except the last two rows, which `arda society build` adds. JSON files are pretty-printed serde output of the types named (`crates/arda-settle/src/lib.rs:61`, `crates/arda-people/src/files.rs:79`).

| File | Content | Codec |
|---|---|---|
| settlements.json | `SettlementsFile` { format_version, seed (decimal string), width_cells, height_cells, `Settlement[]` } | `crates/arda-settle/src/output.rs:28` |
| roads.json | `RoadsFile` { format_version, `Road[]`, `Crossing[]`, `Pass[]` } | `crates/arda-settle/src/output.rs:45` |
| realms.json | `RealmsFile` { format_version, `Realm[]` } | `crates/arda-settle/src/output.rs:58` |
| names.json | `NamesFile` { format_version, `NamedRiver[]`, `NamedPeak[]`, `NamedRegion[]` } | `crates/arda-settle/src/output.rs:116` |
| stats.json | `Stats` with nested `RealmStats` | `crates/arda-settle/src/stats.rs:52` |
| landuse.bin | `ARDALND\0`, version u32 = 1, width u32, height u32, then zstd level 9 of u8 code per cell followed by u32 LE owner per cell | `crates/arda-settle/src/output.rs:17`, `crates/arda-settle/src/output.rs:245` |
| realms.bin | `ARDARLM\0`, same 20-byte header, zstd of u16 LE realm id per cell (0 on water) | `crates/arda-settle/src/output.rs:19`, `crates/arda-settle/src/output.rs:294` |
| roads.bin | `ARDARDS\0`, same header, zstd of u8 road code per cell | `crates/arda-settle/src/output.rs:21`, `crates/arda-settle/src/output.rs:340` |
| society.json | `Society` | `crates/arda-people/src/build.rs:48` |
| notables.json | `NotablesFile` { format_version 1, seed string, `StoredNotable[]` } | `crates/arda-people/src/files.rs:34`, `crates/arda-people/src/build.rs:49` |

### Other files

| File | Content | Site |
|---|---|---|
| library `catalog.json` | `Catalog` beside the asset images | `crates/arda-tactical/src/library.rs:14` |
| `import.toml` / `report.json` | art-import `Manifest` in; `Report` out | `crates/arda-art-import/src/manifest.rs:36`, `crates/arda-art-import/src/output.rs:287` |
| `layout.json` / `meta.json` | `arda-refine` CLI: `TacticalLayout` and `MapMeta` | `crates/arda-refine/src/bin/arda-refine.rs:113`, `crates/arda-refine/src/bin/arda-refine.rs:121` |
| `out/dungeon/layout.json` | dungeon `TacticalLayout` from `arda dungeon` | `crates/arda-cli/src/dungeon.rs:31` |
| compiled-in tables | `crates/arda-npc/data/{srd,content}/*.json`, `crates/arda-society/data/*.json` via `include_str!` | `crates/arda-npc/src/data/mod.rs:45`, `crates/arda-society/src/tables.rs:311` |
| `ARDACOLS` v2 (HTTP only) | 48-byte header (magic, layout version, contract version, ax, ay, gx0, gy0, width, height, column count, data offset), column table, 4-byte aligned column data | `crates/arda-server/src/columnar.rs:16`, `crates/arda-server/src/columnar.rs:287` |

Server caches (tactical blocks, JSON bodies, image pyramids, town plans) are in-memory (`crates/arda-blocks/src/pipeline.rs:70`, `crates/arda-people/src/people.rs:137`); the only server file writes outside tests are overview renders in a per-request scratch directory under the system temp directory, removed on drop (`crates/arda-server/src/overview.rs:263`, `crates/arda-server/src/overview.rs:296`).
