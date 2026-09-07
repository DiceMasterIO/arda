---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: 0056a4cd273f
paths_covered: [":(top)crates/*/src/**"]
capstone_version: 6.4
---

# Models

## Entities

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| Area | `crates/arda/src/lib.rs:146` | in-memory | Loaded cell and object layers |
| World | `crates/arda/src/lib.rs:209` | in-memory | Loaded directory, manifest and eager caches |
| Cell | `crates/arda-core/src/cell.rs:103` | areas/*/cells.bin | 100 m terrain and hydrology facts |
| SizeKm | `crates/arda-core/src/config.rs:17` | world.json.config | Requested extent |
| LatitudeBand | `crates/arda-core/src/config.rs:34` | world.json.config | Latitude interval |
| GenerateConfig | `crates/arda-core/src/config.rs:54` | world.json.config | Validated generation request |
| ContinentCell | `crates/arda-core/src/continent.rs:38` | continent/overview.bin | Persisted 1 km climate and flow facts |
| ContinentOverview | `crates/arda-core/src/continent.rs:58` | continent/overview.bin | Row-major persisted continent grid |
| ContinentRiver | `crates/arda-core/src/continent.rs:70` | continent/objects.bin | 1 km source-to-mouth course |
| ContinentObjects | `crates/arda-core/src/continent.rs:87` | continent/objects.bin | Persisted continent rivers |
| AreaCoord | `crates/arda-core/src/coords.rs:17` | in-memory | Area index |
| CellCoord | `crates/arda-core/src/coords.rs:40` | in-memory | Bounds-checked area-cell index |
| SquareCoord | `crates/arda-core/src/coords.rs:77` | in-memory | Bounds-checked block-square index |
| KmCoord | `crates/arda-core/src/coords.rs:119` | in-memory | 1 km grid index |
| HeightMm | `crates/arda-core/src/fixed.rs:9` | in-memory | Millimetre elevation wrapper |
| TempCentiC | `crates/arda-core/src/fixed.rs:36` | in-memory | Centi-degree temperature wrapper |
| RainfallMm | `crates/arda-core/src/fixed.rs:60` | in-memory | Millimetre rainfall wrapper |
| DischargeMilli | `crates/arda-core/src/fixed.rs:78` | in-memory | Litre-per-second discharge wrapper |
| Block | `crates/arda-core/src/formats/blocks.rs:24` | blocks/*.tiles.zst | 64² tile grid and relaxed flag |
| BlockArchive | `crates/arda-core/src/formats/blocks.rs:67` | blocks/*.tiles.zst | Area block map in (y,x) order |
| AreaCells | `crates/arda-core/src/formats/cells.rs:21` | areas/*/cells.bin | 512² row-major cells |
| ValidationStats | `crates/arda-core/src/formats/manifest.rs:15` | world.json.stats | Reported batch counts and land fraction |
| Manifest | `crates/arda-core/src/formats/manifest.rs:31` | world.json | World identity, configuration and counts |
| RiverSegment | `crates/arda-core/src/objects.rs:42` | areas/*/objects.bin | Channel reach and downstream link |
| Lake | `crates/arda-core/src/objects.rs:61` | areas/*/objects.bin | Retained basin surface, depth and cells |
| AreaObjects | `crates/arda-core/src/objects.rs:77` | areas/*/objects.bin | Area river/lake collections |
| SeedKey | `crates/arda-core/src/rng.rs:50` | in-memory | Subseed domain coordinates |
| TileId | `crates/arda-core/src/tiles.rs:9` | in-memory | u16 vocabulary identifier |
| TileDef | `crates/arda-core/src/tiles.rs:44` | in-memory | Static tile name and adjacency group |
| Hand | `crates/arda-gen/src/area/fields.rs:78` | in-memory | Height and discharge of downstream channel |
| Basin | `crates/arda-gen/src/area/fill.rs:16` | in-memory | Transient depression before lake filtering |
| Filled | `crates/arda-gen/src/area/fill.rs:27` | in-memory | Area routing surface and basins |
| ReliefGrid | `crates/arda-gen/src/area/relief.rs:12` | in-memory | Area heights |
| WaterGrid | `crates/arda-gen/src/area/water.rs:35` | in-memory | Area routing and loads |
| BlockConstraints | `crates/arda-gen/src/block/constraints.rs:8` | in-memory | Allowed tile set and neighbor wetness |
| EnteringRiver | `crates/arda-gen/src/continent/bundles.rs:101` | in-memory | Cross-boundary upstream load |
| TileBundle | `crates/arda-gen/src/continent/bundles.rs:127` | in-memory | In-process area boundary and climate inputs |
| ContinentClimate | `crates/arda-gen/src/continent/climate.rs:12` | in-memory | 1 km climate vectors |
| ContinentHydrology | `crates/arda-gen/src/continent/hydrology.rs:20` | in-memory | 1 km routing and basin vectors |
| ContinentGrid | `crates/arda-gen/src/continent/mod.rs:31` | in-memory | 1 km relief working grid |
| Continent | `crates/arda-gen/src/continent/mod.rs:81` | in-memory | Accepted relief/climate/hydrology context |
| SimExtent | `crates/arda-gen/src/continent/plates.rs:8` | in-memory | 4 km simulation extent |
| Plate | `crates/arda-gen/src/continent/plates.rs:31` | in-memory | Transient plate crust, site and drift |
| CellOut | `crates/arda-render/src/json.rs:16` | export JSON | Area JSON cell subset |
| RiverOut | `crates/arda-render/src/json.rs:31` | export JSON | Area JSON river |
| LakeOut | `crates/arda-render/src/json.rs:42` | export JSON | Area JSON lake |
| AreaOut | `crates/arda-render/src/json.rs:60` | export JSON | Versioned area JSON payload |
| LegendEntry | `crates/arda-render/src/json.rs:73` | export JSON | Block JSON tile legend |
| BlockOut | `crates/arda-render/src/json.rs:79` | export JSON | Versioned block JSON payload |

Enums used by these fields are documented with accepted values below; error and CLI dispatch enums are covered in `03-conventions.md` and `01-architecture.md`.

## Fields and types

Each section follows its definition site in Entities. `Required: no` means an Option value or a serde-defaulted input; an Option field in exported JSON is still emitted as a key, with null when absent. Private fields are included because they determine storage and handoffs.

### Area

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `AreaCells` | yes | — |
| objects | `AreaObjects` | yes | — |

### World

| Field | Type | Required | Notes |
|---|---|---|---|
| dir | `PathBuf` | yes | — |
| manifest | `Manifest` | yes | — |
| areas | `BTreeMap<(i32, i32), Area>` | yes | — |
| blocks | `BTreeMap<(i32, i32), arda_core::BlockArchive>` | yes | — |

### Cell

| Field | Type | Required | Notes |
|---|---|---|---|
| height | `HeightMm` | yes | — |
| terrain | `TerrainKind` | yes | accepted: Sea, Land, Lake |
| cover | `Cover` | yes | accepted: Bare, Grass, Scrub, Forest, Marsh, Rock, Ice |
| slope_milli_deg | `u16` | yes | — |
| aspect_deg | `u16` | yes | — |
| temperature | `TempCentiC` | yes | Generation retains default; producer not built |
| rainfall | `RainfallMm` | yes | — |
| moisture | `u8` | yes | Generation retains default; producer not built |
| forest_density | `u8` | yes | Generation retains default; producer not built |
| drainage_area_cells | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| watercourse_order | `u8` | yes | — |
| watercourse_width_dm | `u16` | yes | — |
| height_above_river_dm | `u16` | yes | u16::MAX where no downstream channel |
| wetness | `u8` | yes | — |
| road | `RoadClass` | yes | accepted: None, Track, Road, Highway; Generation retains default; producer not built |
| built_by | `u16` | yes | Generation retains default; producer not built |

### SizeKm

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | — |
| height | `u32` | yes | — |

### LatitudeBand

| Field | Type | Required | Notes |
|---|---|---|---|
| south_deg | `i16` | yes | — |
| north_deg | `i16` | yes | — |

### GenerateConfig

| Field | Type | Required | Notes |
|---|---|---|---|
| size_km | `SizeKm` | yes | — |
| latitude_band | `LatitudeBand` | yes | — |
| mean_density_per_km2 | `u16` | yes | — |

### ContinentCell

| Field | Type | Required | Notes |
|---|---|---|---|
| height | `HeightMm` | yes | — |
| temperature | `TempCentiC` | yes | — |
| rainfall | `RainfallMm` | yes | — |
| regime | `ClimateRegime` | yes | accepted: Temperate, Mediterranean, Boreal, Tropical |
| downstream | `Option<u8>` | no | accepted: None or neighbor index 0–7; encoded None = 255 |
| catchment_km2 | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |

### ContinentOverview

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | — |
| height | `i32` | yes | — |
| cells | `ContinentCell[]` | yes | — |

### ContinentRiver

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | 1-based; 0 reserved for absent feeds on wire |
| catchment_km2 | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| feeds | `Option<u16>` | no | — |
| course | `KmCoord[]` | yes | — |

### ContinentObjects

| Field | Type | Required | Notes |
|---|---|---|---|
| rivers | `ContinentRiver[]` | yes | — |

### AreaCoord

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `i32` | yes | — |
| y | `i32` | yes | — |

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
| x | `u16` | yes | — |
| y | `u16` | yes | — |

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
| 0 | `u32` | yes | — |

### Block

| Field | Type | Required | Notes |
|---|---|---|---|
| tiles | `TileId[]` | yes | — |
| relaxed | `bool` | yes | — |

### BlockArchive

| Field | Type | Required | Notes |
|---|---|---|---|
| blocks | `BTreeMap<(u16, u16), Block>` | yes | — |

### AreaCells

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `Cell[]` | yes | — |

### ValidationStats

| Field | Type | Required | Notes |
|---|---|---|---|
| land_fraction_permille | `u16` | yes | — |
| area_count | `u32` | yes | — |
| settlement_count | `u32` | yes | — |
| named_river_count | `u32` | yes | — |
| river_count | `u32` | no | serde default 0 for older same-major manifests |

### Manifest

| Field | Type | Required | Notes |
|---|---|---|---|
| format_version | `u32` | yes | — |
| arda_version | `String` | yes | — |
| seed | `u64` | yes | — |
| config | `GenerateConfig` | yes | — |
| areas_wide | `i32` | yes | — |
| areas_high | `i32` | yes | — |
| stats | `ValidationStats` | yes | — |

### RiverSegment

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | 1-based; 0 reserved for absent feeds on wire |
| order | `u8` | yes | — |
| width_dm | `u16` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| feeds | `Option<u16>` | no | — |
| ends | `Terminus` | yes | accepted: Junction, Sea, Lake, OffTile |
| course | `CellCoord[]` | yes | — |

### Lake

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | — |
| surface | `HeightMm` | yes | — |
| depth_mm | `u32` | yes | — |
| outlet | `Option<CellCoord>` | no | — |
| cells | `CellCoord[]` | yes | — |

### AreaObjects

| Field | Type | Required | Notes |
|---|---|---|---|
| rivers | `RiverSegment[]` | yes | — |
| lakes | `Lake[]` | yes | — |

### SeedKey

| Field | Type | Required | Notes |
|---|---|---|---|
| tier | `Tier` | yes | accepted: Continent, Area, Block |
| stage | `Stage` | yes | accepted: Plates, Tectonics, Upsample, Relief, Water, Blocks, Bundles, Erosion |
| x | `i32` | yes | — |
| y | `i32` | yes | — |
| attempt | `u8` | yes | — |

### TileId

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u16` | yes | — |

### TileDef

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `TileId` | yes | — |
| name | `&'static str` | yes | — |
| group | `TileGroup` | yes | accepted: Water, Shore, Ground, Vegetation, Rock, Structure |

### Hand

| Field | Type | Required | Notes |
|---|---|---|---|
| above_mm | `u32[]` | yes | — |
| carried_milli | `u32[]` | yes | — |

### Basin

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `CellCoord[]` | yes | — |
| surface_mm | `i32` | yes | — |
| depth_mm | `u32` | yes | — |

### Filled

| Field | Type | Required | Notes |
|---|---|---|---|
| surface | `i32[]` | yes | — |
| basins | `Basin[]` | yes | — |

### ReliefGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| heights | `i32[]` | yes | — |

### WaterGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| drainage | `u32[]` | yes | — |
| discharge | `u32[]` | yes | — |
| order | `u8[]` | yes | — |
| downstream | `Vec<Option<u32>>` | yes | — |
| outlets | `bool[]` | yes | — |

### BlockConstraints

| Field | Type | Required | Notes |
|---|---|---|---|
| allowed | `TileId[]` | yes | — |
| wet_fraction | `u8` | yes | — |

### EnteringRiver

| Field | Type | Required | Notes |
|---|---|---|---|
| cell | `CellCoord` | yes | — |
| catchment_km2 | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| order | `u8` | yes | — |

### TileBundle

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | — |
| north | `i32[]` | yes | — |
| south | `i32[]` | yes | — |
| east | `i32[]` | yes | — |
| west | `i32[]` | yes | — |
| mean_height_mm | `i32` | yes | — |
| entering | `EnteringRiver[]` | yes | — |
| rainfall_km | `u16[]` | yes | — |
| regime_km | `ClimateRegime[]` | yes | — |
| filled_km | `i32[]` | yes | — |
| basin_km | `i32[]` | yes | — |
| wind | `CompassOctant` | yes | accepted: West |
| west_moisture | `u16[]` | yes | — |

### ContinentClimate

| Field | Type | Required | Notes |
|---|---|---|---|
| temperature | `i16[]` | yes | — |
| rainfall | `u16[]` | yes | — |
| regime | `ClimateRegime[]` | yes | — |
| moisture | `u16[]` | yes | — |

### ContinentHydrology

| Field | Type | Required | Notes |
|---|---|---|---|
| filled | `i32[]` | yes | — |
| basin_surface | `i32[]` | yes | NO_BASIN = i32::MIN outside filled depression; 8-connected components |
| downstream | `Vec<Option<u32>>` | yes | — |
| downstream_dir | `u8[]` | yes | NO_DOWNSTREAM = 255 |
| catchment_km2 | `u32[]` | yes | — |
| discharge_l_s | `u32[]` | yes | — |

### ContinentGrid

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | — |
| height | `i32` | yes | — |
| height_mm | `i32[]` | yes | — |

### Continent

| Field | Type | Required | Notes |
|---|---|---|---|
| grid | `ContinentGrid` | yes | — |
| climate | `climate::ContinentClimate` | yes | — |
| hydrology | `hydrology::ContinentHydrology` | yes | — |

### SimExtent

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `i32` | yes | — |
| height | `i32` | yes | — |

### Plate

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u8` | yes | — |
| centre_x | `i32` | yes | — |
| centre_y | `i32` | yes | — |
| crust | `CrustType` | yes | accepted: Continental, Oceanic |
| drift_x | `i32` | yes | — |
| drift_y | `i32` | yes | — |

### CellOut

| Field | Type | Required | Notes |
|---|---|---|---|
| height_mm | `i32` | yes | — |
| terrain | `&'static str` | yes | — |
| cover | `&'static str` | yes | — |
| slope_milli_deg | `u16` | yes | — |
| aspect_deg | `u16` | yes | — |
| drainage_area_cells | `u32` | yes | — |
| discharge_milli_cumecs | `u32` | yes | — |
| watercourse_order | `u8` | yes | — |
| watercourse_width_dm | `u16` | yes | — |
| height_above_river_dm | `u16` | yes | — |
| wetness | `u8` | yes | — |

### RiverOut

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | — |
| order | `u8` | yes | — |
| width_dm | `u16` | yes | — |
| discharge_milli_cumecs | `u32` | yes | — |
| feeds | `Option<u16>` | no | — |
| ends | `&'static str` | yes | — |
| course | `Vec<[u16; 2]>` | yes | — |

### LakeOut

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | — |
| surface_mm | `i32` | yes | — |
| depth_mm | `u32` | yes | — |
| outlet | `Option<[u16; 2]>` | no | — |
| cells | `Vec<[u16; 2]>` | yes | — |

### AreaOut

| Field | Type | Required | Notes |
|---|---|---|---|
| schema_version | `u32` | yes | — |
| format_version | `u32` | yes | — |
| seed | `u64` | yes | — |
| area_x | `i32` | yes | — |
| area_y | `i32` | yes | — |
| cells_per_side | `u16` | yes | — |
| cells | `CellOut[]` | yes | — |
| rivers | `RiverOut[]` | yes | — |
| lakes | `LakeOut[]` | yes | — |

### LegendEntry

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u16` | yes | — |
| name | `&'static str` | yes | — |

### BlockOut

| Field | Type | Required | Notes |
|---|---|---|---|
| schema_version | `u32` | yes | — |
| format_version | `u32` | yes | — |
| seed | `u64` | yes | — |
| squares_per_side | `u8` | yes | — |
| relaxed | `bool` | yes | — |
| legend | `LegendEntry[]` | yes | — |
| squares | `u16[]` | yes | — |

## Relationships

`World` owns `Manifest`, `(ax,ay) → Area`, and `(ax,ay) → BlockArchive`; each archive maps `(cy,cx) → Block` (`crates/arda/src/lib.rs:209`, `crates/arda-core/src/formats/blocks.rs:67`). `AreaObjects` contains only rivers and lakes. Segment ids are tile-local; continent river ids are continent-local. Thus the original no-synthetic-IDs design is not the current object representation (`crates/arda-core/src/objects.rs:42`, `crates/arda-core/src/continent.rs:70`).

`Continent → bundle_for → TileBundle → generate_area` is an in-process handoff; bundles and basin identity are not persisted, and areas never read neighboring area outputs (`crates/arda-gen/src/orchestrator.rs:96`). `Plate` exists only during generation, not in continent objects. The designed `Range`, `Region`, `Sea`, `Settlement`, `Road`, `Crossing`, `Pass`, `Realm`, `Building`, `Npc`, and `Poi` have no current struct/storage producer. Their decisions remain in `logic/01-continent-generation.md`, `logic/02-area-generation.md`, `logic/03-block-generation.md`, and `logic/06-society-generation.md`: named regions/rivers, society partitions, shared building layouts, notable NPC sheets, on-demand commoners, and sparse block POIs are retained plans.

## Boundaries

Codecs in `arda-core::formats` convert typed layers to little-endian bytes; manifest is serde JSON. Renderer DTOs explicitly convert core records to exports (`crates/arda-render/src/json.rs:118`). `CellOut` omits temperature, rainfall, moisture, forest_density, road and built_by, so the designed complete-cell JSON contract is not implemented. Area cells/objects share one AreaOut file, rather than the planned separate cells.json/objects.json. BlockOut carries ids/names, without the planned material/traversal/movement/cover/hazard attributes, POIs or buildings (`crates/arda-render/src/json.rs:16`, `crates/arda-render/src/json.rs:79`).

Climate, hydrology and basin patches are simulation state; `World::load` reads area/block layers but never reads the persisted continent layers (`crates/arda/src/lib.rs:223`). The design's loaded continent queries and lazy per-block frames remain planned. The chosen binary layout avoids a database and keeps deterministic bytes; world-format major is independent of crate semver (`crates/arda-core/src/formats/mod.rs:15`).

## Validation

`GenerateConfig::new` accepts axes 64–4000 km inclusive, density 1–200 inclusive and ordered latitudes within −80..80; serde loading does not rerun that constructor (`crates/arda-core/src/config.rs:73`, `crates/arda-core/src/formats/manifest.rs:75`). Area counts divide each axis by the integer 51 km constant, truncating; spatial area coordinates still use 512×100 m = 51.2 km (`crates/arda-core/src/config.rs:8`, `crates/arda-core/src/config.rs:133`, `crates/arda-core/src/coords.rs:7`). Default config therefore gives 9×19 = 171 areas, whereas earlier mockups estimated 190.

The batch accepts land fraction 250–900 per mille and at least one sea-reaching major river within five attempts; the designed mountain-range gate and per-area statistical acceptance suite are absent (`crates/arda-gen/src/orchestrator.rs:61`). Lake retention is ≥300 cells and ≥4,000 mm maximum depth after trimming, without rainfall/evaporation balance (`crates/arda-gen/src/area/mod.rs:321`). Area basin grouping is 4-connected; continent basin identity uses the eight routing neighbors (`crates/arda-gen/src/area/fill.rs:123`, `crates/arda-gen/src/continent/hydrology.rs:145`). The latter type's leading doc comment still says four, but the implementation iterates all eight.

Manifest load requires format major exactly 3, refusing older and newer worlds. Codecs check magic, truncated records, closed discriminants and coordinate ranges; overview dimensions use checked arithmetic and reject values beyond i32 (`crates/arda-core/src/formats/overview.rs:58`, `crates/arda-core/src/formats/objects.rs:136`). Block decoding stores any u16 TileId; symbolic rendering reports unknown IDs rather than validating vocabulary during load (`crates/arda-core/src/formats/blocks.rs:133`, `crates/arda-render/src/symbolic.rs:30`).

## Schema

No database or migrations exist. All integer fields below are little-endian; format major 3 is checked in world.json (`crates/arda-core/src/formats/mod.rs:15`).

| File | Layout | Codec |
|---|---|---|
| world.json | Manifest with fixed serde field order, written last; river_count defaults to 0 on read | `crates/arda-core/src/formats/manifest.rs:59` |
| areas/AX_AY/cells.bin | No header: exactly 262,144 rows of 33 bytes in Cell field order | `crates/arda-core/src/formats/cells.rs:132` |
| areas/AX_AY/objects.bin | ARDAOBJ\0, tagged length-delimited river/lake sections; coordinates and courses explicit | `crates/arda-core/src/formats/objects.rs:76` |
| continent/overview.bin | ARDAOVR\0, u32 width/height, 18-byte ContinentCell rows | `crates/arda-core/src/formats/overview.rs:27` |
| continent/objects.bin | ARDACOB\0, tagged continent-river section | `crates/arda-core/src/formats/objects.rs:260` |
| blocks/AX_AY.tiles.zst | One zstd level-3 frame per area: ARDABLK\0, u32 count, each block cx:u16, cy:u16, relaxed:u8, 4,096 u16 tile ids | `crates/arda-core/src/formats/blocks.rs:100` |

Export schema_version is 1, distinct from format_version 3; additive evolution remains the selected contract (`crates/arda-render/src/json.rs:13`). The designed 200+ vocabulary is presently 24 TileDef entries, with only id/name/group; a TileSet wrapper is not defined (`crates/arda-core/src/tiles.rs:44`).
