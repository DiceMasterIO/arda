---
generated_at_commit: 342d03e55120
generated_date: 2026-09-22
content_hash: 454d2f2ef505
paths_covered: [":(top)crates/*/src/**"]
absorbed_from: [features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22]
---

# Models

## Entities

| Name | Definition site | Storage | Purpose |
|---|---|---|---|
| Area | `crates/arda/src/world.rs:15` | in-memory | Owned cell and object layers for one requested area |
| World | `crates/arda/src/world.rs:78` | in-memory | Manifest-first directory handle with separate lazy area/block caches |
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
| AreaCells | `crates/arda-core/src/formats/cells.rs:24` | areas/*/cells.bin | 512² row-major cells |
| ValidationStats | `crates/arda-core/src/formats/manifest.rs:15` | world.json.stats | Reported batch counts and land fraction |
| Manifest | `crates/arda-core/src/formats/manifest.rs:31` | world.json | World identity, configuration and counts |
| RiverSegment | `crates/arda-core/src/objects.rs:49` | areas/*/objects.bin | Tile-local channel fragment linked to a stable global reach |
| Lake | `crates/arda-core/src/objects.rs:70` | areas/*/objects.bin | Tile-local cells/depth/outlet of one connected representative lake |
| AreaObjects | `crates/arda-core/src/objects.rs:90` | areas/*/objects.bin | Local fragments, physical channel halo and copied global authority |
| SeedKey | `crates/arda-core/src/rng.rs:50` | in-memory | Subseed domain coordinates |
| TileId | `crates/arda-core/src/tiles.rs:9` | in-memory | u16 vocabulary identifier |
| TileDef | `crates/arda-core/src/tiles.rs:44` | in-memory | Static tile name and adjacency group |
| Hand | `crates/arda-gen/src/area/fields.rs:78` | in-memory; local diagnostic path | Local downstream-channel height and discharge vectors |
| Basin | `crates/arda-gen/src/area/fill.rs:16` | in-memory; local diagnostic path | Local filled depression; not the published global basin identity |
| Filled | `crates/arda-gen/src/area/fill.rs:27` | in-memory; local diagnostic helper | Temporary area routing surface and local basins; not the published shared evolution scratch |
| ReliefGrid | `crates/arda-gen/src/area/relief.rs:12` | in-memory; local diagnostic helper | Area heights |
| WaterGrid | `crates/arda-gen/src/area/water.rs:35` | in-memory; local diagnostic path | Retained local routing/loads; not published shared-flow authority |
| BlockConstraints | `crates/arda-gen/src/block/constraints.rs:8` | in-memory | Allowed tile set and neighbor wetness |
| EnteringRiver | `crates/arda-gen/src/continent/bundles.rs:78` | in-memory; local diagnostic path | Coarse boundary load retained for local diagnostic generation |
| TileBundle | `crates/arda-gen/src/continent/bundles.rs` | in-memory | Raw edge samples, climate and diagnostic water patches; no production erosion boundary authority |
| ContinentClimate | `crates/arda-gen/src/continent/climate.rs:12` | in-memory | 1 km climate vectors |
| ContinentHydrology | `crates/arda-gen/src/continent/hydrology.rs:20` | in-memory | 1 km routing and basin vectors |
| ContinentGrid | `crates/arda-gen/src/continent/mod.rs:34` | in-memory | 1 km relief working grid |
| Continent | `crates/arda-gen/src/continent/mod.rs:84` | in-memory | Accepted relief/climate/hydrology context |
| SimExtent | `crates/arda-gen/src/continent/plates.rs:8` | in-memory | 4 km simulation extent |
| Plate | `crates/arda-gen/src/continent/plates.rs:31` | in-memory | Transient plate crust, site and drift |
| CellOut | `crates/arda-render/src/json.rs:18` | export JSON | Area JSON cell subset |
| RiverOut | `crates/arda-render/src/json.rs:33` | export JSON | Area JSON river |
| LakeOut | `crates/arda-render/src/json.rs:45`; `crates/arda-render/src/hydrology_json.rs:49` | export JSON | Local area lake DTO and module-scoped copied global lake DTO |
| AreaOut | `crates/arda-render/src/json.rs:66` | export JSON | Versioned area JSON payload |
| LegendEntry | `crates/arda-render/src/json.rs:90` | export JSON | Block JSON tile legend |
| BlockOut | `crates/arda-render/src/json.rs:96` | export JSON | Versioned block JSON payload |
| GlobalCell | `crates/arda-core/src/coords.rs:169` | global hydrology and area geometry | Absolute 100 m coordinate across the modeled domain |
| TerminalId | `crates/arda-core/src/hydrology.rs:20` | typed in-memory identity | Stable physical terminal namespace |
| BasinId | `crates/arda-core/src/hydrology.rs:24` | global/area records and child table | Stable physical basin or connected-lake namespace |
| ReachId | `crates/arda-core/src/hydrology.rs:28` | global/area records | Canonical directed start/step or tagged physical point identity |
| JunctionId | `crates/arda-core/src/hydrology.rs:32` | receiving-account records | Packed actual dry junction coordinate; no separate junction table |
| CatchmentId | `crates/arda-core/src/hydrology.rs:103` | global/area records | Immutable contributing-owner namespace |
| Litres | `crates/arda-core/src/hydrology.rs:109` | annual hydrology records | Exact nonnegative whole annual water volume |
| HydrologyDomain | `crates/arda-core/src/hydrology.rs:130` | hydrology/metadata.bin; World | Modeled request/export union and final-area dimensions |
| CrossingId | `crates/arda-core/src/hydrology.rs:144` | global/area crossing records | Canonical undirected cross-area D8 endpoint pair |
| ReceivingAccount | `crates/arda-core/src/hydrology.rs:153` | global/area water records | Exclusive immediate destination of a transfer |
| SpillConnection | `crates/arda-core/src/hydrology.rs:168` | basin/lake/catchment records | Actual neighboring physical spill witness or real rim export |
| BasinNode | `crates/arda-core/src/hydrology.rs:184` | bounded in-memory API view | Logical physical hierarchy node with owned child IDs |
| TableSpan | `crates/arda-core/src/formats/hydrology.rs:343` | hydrology/basins.bin | Record offset/count into the separate child-ID table |
| BasinNodeRow | `crates/arda-core/src/formats/hydrology.rs:365` | hydrology/basins.bin | Fixed hierarchy row used by the production writer |
| AnnualWaterBalance | `crates/arda-core/src/hydrology.rs:202` | hydrology/metadata.bin | Whole-domain annual ledger after internal transfers cancel |
| GlobalLake | `crates/arda-core/src/hydrology.rs:221` | hydrology/lakes.bin; area context | Connected positive-depth representative lake authority |
| AnnualCatchment | `crates/arda-core/src/hydrology.rs:240` | hydrology/catchments.bin; area context | Original contributing cells, physical terminal and annual destination |
| GlobalReach | `crates/arda-core/src/hydrology.rs:259` | hydrology/reaches.bin; area context | Directed annual flow, drainage and downstream identity |
| SharedCrossing | `crates/arda-core/src/hydrology.rs:281` | hydrology/crossings.bin; area context | One canonical directed transfer across an area boundary |
| ChannelEdge | `crates/arda-core/src/hydrology.rs:305` | area objects channel section | Saved adjacent centerline endpoints and physical widths |
| AreaHydrologyReferences | `crates/arda-core/src/hydrology.rs:320` | bounded in-memory API view | Relevant identities for one area; not a separate published table |
| AreaHydrologyContext | `crates/arda-core/src/hydrology.rs:333` | area objects global section | Exact relevant global copies for independent area reads |
| HydrologyMetadata | `crates/arda-core/src/hydrology.rs:359` | hydrology/metadata.bin | Model revision, domain, global row counts and annual ledger |
| TableHeader | `crates/arda-core/src/formats/hydrology.rs:500` | each hydrology/*.bin table | Explicit tag, fixed row size and checked table payload length |
| MarineBoundary | `crates/arda-gen/src/hydrology/types.rs:8` | prepared in-memory/private files | Which cropped tile sides meet the actual modeled rim |
| PreparedExtent | `crates/arda-gen/src/hydrology/types.rs:20` | prepared in-memory/private files | Valid 1–512 rows/columns and real outer boundary sides |
| SharedTerrain | `crates/arda-gen/src/area/prepare.rs` | private in-memory preparation owner | Complete modeled fine surface after shared evolution; released after immutable slices are persisted |
| PreparedTerrain | `crates/arda-gen/src/hydrology/types.rs:31` | in-memory; cropped private prepared files | Immutable slice of evolved physical heights, rain and unlapsed temperature arrays |
| HydrologyLimits | `crates/arda-gen/src/hydrology/types.rs:84` | in-memory generation API | Explicit whole-generation resource and feature capacities |
| SolvedCell | `crates/arda-gen/src/area/shared_compose.rs:15` | in-memory composition handoff | Immutable shared marine/lake, drainage, flow/order and HAND facts |
| ImageQuality | `crates/arda-render/src/quality.rs:7` | in-memory render request | Validated PNG edge length independent of saved terrain resolution |
| MapStyle | `crates/arda/src/lib.rs:28` | in-memory render request | Classic default or Atlas PNG presentation; never serialized in a world |
| AreaImageScale | `crates/arda-render/src/channels.rs:13` | in-memory render request | Preview, detail or custom square area-image resolution |
| AtlasNeighbor | `crates/arda-render/src/atlas.rs:47` | in-memory render context | Fixed eight adjacent-area directions |
| AtlasHalo | `crates/arda-render/src/atlas.rs:77` | in-memory render scratch | 516² optional saved height/class context plus eight direction states |
| AtlasTerrain | `crates/arda-render/src/atlas.rs:159` | in-memory render scratch | 514² derived palette RGB, Q12 lighting and terrain classes |
| OverviewRaster | `crates/arda-render/src/overview.rs:22` | in-memory buffered renderer | Exact or regular-size overview pixels, feature precedence and supplied-area tracking |
| AreaRaster | `crates/arda-render/src/channels.rs:315` | in-memory streaming renderer | Prepared channel geometry, bounded candidate indexes and one reusable area scanline |
| RasterBand | `crates/arda-render/src/overview/streaming.rs:89` | in-memory streaming helper | At most 256 overview rows of RGB and feature classifications |
| ChannelEdgeOut | `crates/arda-render/src/json.rs:81` | export JSON | Global centerline endpoints, widths and mean discharge |
| AccountOut | `crates/arda-render/src/hydrology_json.rs:7` | export JSON | Tagged immediate receiving account with decimal identity payload |
| SpillOut | `crates/arda-render/src/hydrology_json.rs:32` | export JSON | Physical spill geometry and receiving account |
| CatchmentOut | `crates/arda-render/src/hydrology_json.rs:71` | export JSON | Copied immutable catchment and annual lake association |
| ReachOut | `crates/arda-render/src/hydrology_json.rs:93` | export JSON | Copied global reach, point flag and exact annual amount |
| CrossingOut | `crates/arda-render/src/hydrology_json.rs:119` | export JSON | Copied cross-area transfer geometry and exact annual amount |
| HydrologyOut | `crates/arda-render/src/hydrology_json.rs:145` | export JSON | Explicit annual semantics and relevant global records |

Enums used by fields list accepted variants below. Error and CLI dispatch enums remain covered by `03-conventions.md` and `01-architecture.md`; private routing/cache records belong to their stage modules rather than the saved-world entity schema.

## Fields and types

Definition sites are in Entities. Required `no` denotes an optional field or serde default. Option fields in JSON remain keys with null values. `AccountOut` lists its serialized tagged-union fields: the selected identity payload is a String; other variants omit it. `ReceivingAccount` rows describe exclusive enum alternatives. The two module-scoped `LakeOut` types share a heading but have separate tables.

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
| domain | `HydrologyDomain` | yes | Reconstructed from validated manifest configuration and exported dimensions |
| areas | `BTreeMap<AreaCoord, OnceLock<Area>>` | yes | Empty OnceLock slots at load; area() retains only successfully requested areas |
| blocks | `BTreeMap<AreaCoord, OnceLock<BlockArchive>>` | yes | Independent requested-area archive cache; first block query decodes that whole archive |

### Cell

| Field | Type | Required | Notes |
|---|---|---|---|
| height | `HeightMm` | yes | — |
| terrain | `TerrainKind` | yes | accepted: Sea, Land, Lake |
| cover | `Cover` | yes | accepted: Bare, Grass, Scrub, Forest, Marsh, Rock, Ice |
| slope_milli_deg | `u16` | yes | — |
| aspect_deg | `u16` | yes | — |
| temperature | `TempCentiC` | yes | Produced from prepared reference and final signed physical-altitude lapse; marine altitude is zero |
| rainfall | `RainfallMm` | yes | — |
| moisture | `u8` | yes | Generation retains default; producer not built |
| forest_density | `u8` | yes | Generation retains default; producer not built |
| drainage_area_cells | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| watercourse_order | `u8` | yes | — |
| watercourse_width_dm | `u32` | yes | — |
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
| global_id | `ReachId` | yes | Stable identity of the containing global reach |
| id | `u32` | yes | 1-based; 0 reserved for absent feeds on wire |
| order | `u8` | yes | — |
| width_dm | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| feeds | `Option<u32>` | no | Only Junction has one local downstream segment; divergence uses global junction identity |
| ends | `Terminus` | yes | accepted: Junction, Sea, Lake, OffTile, Basin, Divergence |
| course | `CellCoord[]` | yes | — |

### Lake

| Field | Type | Required | Notes |
|---|---|---|---|
| global_id | `BasinId` | yes | Stable identity shared across all local fragments of the connected lake |
| id | `u32` | yes | — |
| surface | `HeightMm` | yes | Exact global representative annual surface, whole millimetres |
| depth_mm | `u32` | yes | Greatest local positive depth |
| outlet | `Option<CellCoord>` | no | Local source of supported annual outflow; absent if dry outflow or outlet belongs to another area |
| cells | `CellCoord[]` | yes | — |

### AreaObjects

| Field | Type | Required | Notes |
|---|---|---|---|
| channel_edges | `ChannelEdge[]` | yes | Neighbor halo included when a physical envelope touches the area |
| global | `AreaHydrologyContext` | yes | Relevant global copies; no recursive neighbor reads |
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
| carried_milli | `u64[]` | yes | — |

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
| discharge | `u64[]` | yes | — |
| order | `u8[]` | yes | — |
| downstream | `Option<u32>[]` | no | — |
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
| north | `i32[]` | yes | Unevolved absolute-coordinate relief samples; retained for the local diagnostic path |
| south | `i32[]` | yes | Unevolved samples at the next area's first row |
| east | `i32[]` | yes | Unevolved samples at the next area's first column |
| west | `i32[]` | yes | Unevolved absolute-coordinate relief samples |
| north_outside | `i32[]` | yes | 512 actual adjacent samples at local y=-1; legacy routing only |
| west_outside | `i32[]` | yes | 512 actual adjacent samples at local x=-1; legacy routing only |
| outside_corners | `[i32; 4]` | yes | Adjacent NW, NE, SW, SE at (-1,-1), (512,-1), (-1,512), (512,512) |
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
| ocean | `bool[]` | yes | D8-connected nonpositive coarse cells touching the domain rim |
| ocean_distance_km | `u16[]` | yes | Cardinal distance to connected ocean, capped at 300 km |

### ContinentHydrology

| Field | Type | Required | Notes |
|---|---|---|---|
| filled | `i32[]` | yes | — |
| basin_surface | `i32[]` | yes | NO_BASIN = i32::MIN outside filled depression; 8-connected components |
| downstream | `Option<u32>[]` | no | — |
| downstream_dir | `u8[]` | yes | NO_DOWNSTREAM = 255 |
| catchment_km2 | `u32[]` | yes | — |
| discharge_l_s | `u64[]` | yes | — |

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
| climate | `ContinentClimate` | yes | — |
| hydrology | `ContinentHydrology` | yes | — |

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

Local area fragment (`crates/arda-render/src/json.rs:45`):

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `u32` | yes | — |
| global_basin_id | `String` | yes | Decimal string preserves exact global integer identity |
| surface_mm | `i32` | yes | — |
| depth_mm | `u32` | yes | — |
| outlet | `Option<[u16; 2]>` | no | — |
| cells | `[u16; 2][]` | yes | — |

Copied global lake (`crates/arda-render/src/hydrology_json.rs:49`):

| Field | Type | Required | Notes |
|---|---|---|---|
| global_basin_id | `String` | yes | Decimal string preserves exact global integer identity |
| surface_mm | `i32` | yes | — |
| deepest_bed_mm | `i32` | yes | — |
| submerged_cells | `u32` | yes | — |
| outlet | `Option<SpillOut>` | no | — |
| annual_outflow_litres | `String` | yes | Exact whole litres encoded as a decimal string |
| mean_outflow_milli_cumecs | `u64` | yes | — |

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

### GlobalCell

| Field | Type | Required | Notes |
|---|---|---|---|
| x | `u32` | yes | — |
| y | `u32` | yes | — |

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
| width_cells | `u32` | yes | — |
| height_cells | `u32` | yes | — |
| exported_areas_wide | `u32` | yes | — |
| exported_areas_high | `u32` | yes | — |

### CrossingId

| Field | Type | Required | Notes |
|---|---|---|---|
| low | `GlobalCell` | yes | Lower endpoint under GlobalCell ordering (x,y), unlike packed terminal ordering (y,x) |
| high | `GlobalCell` | yes | Higher endpoint under GlobalCell ordering (x,y) |

### ReceivingAccount

| Field | Type | Required | Notes |
|---|---|---|---|
| Reach | `ReachId` | no | Exclusive enum alternative; accepted: Reach, Lake, Junction, Sea, DomainExport |
| Lake | `BasinId` | no | Exclusive enum alternative; accepted: Reach, Lake, Junction, Sea, DomainExport |
| Junction | `JunctionId` | no | Exclusive enum alternative; accepted: Reach, Lake, Junction, Sea, DomainExport |
| Sea | `()` | no | Exclusive enum alternative; accepted: Reach, Lake, Junction, Sea, DomainExport |
| DomainExport | `()` | no | Exclusive enum alternative; accepted: Reach, Lake, Junction, Sea, DomainExport |

### SpillConnection

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `GlobalCell` | yes | — |
| to | `Option<GlobalCell>` | no | — |
| sill | `HeightMm` | yes | — |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### BasinNode

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BasinId` | yes | — |
| parent | `Option<BasinId>` | no | — |
| anchor | `GlobalCell` | yes | — |
| floor | `HeightMm` | yes | — |
| children | `BasinId[]` | yes | — |
| spill | `Option<SpillConnection>` | no | — |

### TableSpan

| Field | Type | Required | Notes |
|---|---|---|---|
| offset | `u64` | yes | Record index, not a byte offset; zero for an empty span |
| count | `u64` | yes | Checked against the linked table count |

### BasinNodeRow

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `BasinId` | yes | — |
| parent | `Option<BasinId>` | no | — |
| anchor | `GlobalCell` | yes | — |
| floor | `HeightMm` | yes | — |
| children | `TableSpan` | yes | Span into hydrology/children.bin, not an inline vector |
| spill | `Option<SpillConnection>` | no | — |

### AnnualWaterBalance

| Field | Type | Required | Notes |
|---|---|---|---|
| land_precipitation | `Litres` | yes | — |
| land_loss | `Litres` | yes | — |
| lake_precipitation | `Litres` | yes | — |
| lake_evaporation | `Litres` | yes | — |
| marginal_evaporation | `Litres` | yes | Separately paid unsupported-band adjustment; that marginal band stays geometrically dry |
| sea_outflow | `Litres` | yes | — |
| domain_outflow | `Litres` | yes | — |

### GlobalLake

| Field | Type | Required | Notes |
|---|---|---|---|
| basin | `BasinId` | yes | — |
| surface | `HeightMm` | yes | Positive depth means physical bed < surface |
| deepest_bed | `HeightMm` | yes | — |
| submerged_cells | `u32` | yes | — |
| outlet | `Option<SpillConnection>` | no | Potential witness retained when nonspilling; canonical active witness when annual_outflow > 0 |
| annual_outflow | `Litres` | yes | Whole litres/year; can be positive when mean_outflow rounds to zero |
| mean_outflow | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |

### AnnualCatchment

| Field | Type | Required | Notes |
|---|---|---|---|
| catchment | `CatchmentId` | yes | — |
| terminal | `GlobalCell` | yes | — |
| contributing_cells | `u32` | yes | — |
| basin | `Option<BasinId>` | no | — |
| representative_lake | `Option<BasinId>` | no | — |
| potential_spill | `Option<SpillConnection>` | no | — |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### GlobalReach

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `ReachId` | yes | — |
| from | `GlobalCell` | yes | — |
| to | `GlobalCell` | yes | — |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |
| catchment | `CatchmentId` | yes | — |
| drainage_cells | `u32` | yes | — |
| annual_volume | `Litres` | yes | — |
| mean_discharge | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |

### SharedCrossing

| Field | Type | Required | Notes |
|---|---|---|---|
| id | `CrossingId` | yes | — |
| from | `GlobalCell` | yes | — |
| to | `GlobalCell` | yes | — |
| reach | `ReachId` | yes | — |
| catchment | `CatchmentId` | yes | — |
| drainage_cells | `u32` | yes | — |
| annual_volume | `Litres` | yes | — |
| mean_discharge | `DischargeMilli` | yes | Whole annual litres / 31,536,000, rounded down to L/s |
| receiving | `ReceivingAccount` | yes | accepted: Reach(ReachId), Lake(BasinId), Junction(JunctionId), Sea, DomainExport |

### ChannelEdge

| Field | Type | Required | Notes |
|---|---|---|---|
| from | `GlobalCell` | yes | — |
| to | `GlobalCell` | yes | — |
| from_width_dm | `u32` | yes | — |
| to_width_dm | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |

### AreaHydrologyReferences

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | — |
| lakes | `BasinId[]` | yes | — |
| reaches | `ReachId[]` | yes | — |
| crossings | `CrossingId[]` | yes | — |

### AreaHydrologyContext

| Field | Type | Required | Notes |
|---|---|---|---|
| model_revision | `u32` | yes | 2; representative annual model |
| reaches | `GlobalReach[]` | yes | — |
| lakes | `GlobalLake[]` | yes | — |
| catchments | `AnnualCatchment[]` | yes | — |
| crossings | `SharedCrossing[]` | yes | — |

### HydrologyMetadata

| Field | Type | Required | Notes |
|---|---|---|---|
| model_revision | `u32` | yes | 2; representative annual model |
| domain | `HydrologyDomain` | yes | — |
| basin_count | `u64` | yes | — |
| lake_count | `u64` | yes | — |
| reach_count | `u64` | yes | — |
| crossing_count | `u64` | yes | — |
| catchment_count | `u64` | yes | — |
| budget | `AnnualWaterBalance` | yes | — |

### TableHeader

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `TableKind` | yes | accepted: Metadata=1, Basins=2, Children=3, Lakes=4, Reaches=5, Crossings=6, Channels=11, Catchments=12 |
| record_bytes | `u32` | yes | — |
| count | `u64` | yes | — |
| payload_bytes | `u64` | yes | — |

### MarineBoundary

| Field | Type | Required | Notes |
|---|---|---|---|
| north | `bool` | yes | — |
| east | `bool` | yes | — |
| south | `bool` | yes | — |
| west | `bool` | yes | — |

### PreparedExtent

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u16` | yes | — |
| height | `u16` | yes | — |
| boundary | `MarineBoundary` | yes | — |

### SharedTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `usize` | yes | Modeled union width in 100 m cells |
| height | `usize` | yes | Modeled union height in 100 m cells |
| heights | `i32[]` | yes | Row-major evolved physical millimetres; includes publication areas and modeled fringe |

### PreparedTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| area | `AreaCoord` | yes | — |
| valid | `PreparedExtent` | yes | — |
| heights | `HeightMm[]` | yes | Full 512² container; valid cells copy SharedTerrain exactly, outside-valid zero padding is never serialized |
| annual_rain | `RainfallMm[]` | yes | — |
| temperature_base_centi | `i32[]` | yes | Unclamped reference with coarse lapse removed; not a final persisted temperature |

### HydrologyLimits

| Field | Type | Required | Notes |
|---|---|---|---|
| max_closed_leaves | `u64` | yes | — |
| max_bands | `u64` | yes | — |
| max_feature_records | `u64` | yes | — |
| max_area_references | `u64` | yes | — |
| max_lake_boundary_edges | `u64` | yes | — |
| global_io_operations | `u64` | yes | — |
| ram_bytes | `u64` | yes | Owned-payload admission ceiling, including dense terrain and water-stage reservations; default 16 GiB |
| spatial_scratch_bytes | `u64` | yes | — |
| combined_scratch_bytes | `u64` | yes | — |
| global_event_operations | `u64` | yes | — |
| global_io_bytes | `u128` | yes | — |

### SolvedCell

| Field | Type | Required | Notes |
|---|---|---|---|
| marine | `bool` | yes | — |
| lake | `Option<BasinId>` | no | — |
| drainage_cells | `u32` | yes | — |
| discharge | `DischargeMilli` | yes | — |
| order | `u8` | yes | Zero below 40 L/s channel initiation |
| hand_mm | `u32` | yes | u32::MAX when no downstream channel exists |
| hand_discharge | `DischargeMilli` | yes | — |

### ImageQuality

| Field | Type | Required | Notes |
|---|---|---|---|
| 0 | `u32` | yes | Accepted: 512–32,768 pixels; default 8,192 |

### MapStyle

| Field | Type | Required | Notes |
|---|---|---|---|
| variant | enum | yes | Classic is the default; Atlas is an explicit PNG presentation, with unchanged saved data |

### AreaImageScale

| Field | Type | Required | Notes |
|---|---|---|---|
| variant | enum | yes | accepted: Preview, Detail, Custom(ImageQuality); sides are 512, 4,096, or the validated custom edge |

### AtlasNeighbor

| Field | Type | Required | Notes |
|---|---|---|---|
| variant | enum | yes | North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest (`crates/arda-render/src/atlas.rs:47`) |

### AtlasHalo

| Field | Type | Required | Notes |
|---|---|---|---|
| context | `Option<SourceSample>[]` | yes | Fixed 516² saved height/class samples for target plus two-cell cardinal strips and 2×2 diagonal corners (`crates/arda-render/src/atlas.rs:77`, `crates/arda-render/src/atlas.rs:108`) |
| states | `NeighborState[8]` | yes | Every direction resolved once as Copied or WorldEdge; incomplete or contradictory context is refused (`crates/arda-render/src/atlas.rs:176`) |

### AtlasTerrain

| Field | Type | Required | Notes |
|---|---|---|---|
| palette | `[u8; 3][]` | yes | 514² derived RGB terrain palette, including interpolation ghosts (`crates/arda-render/src/atlas.rs:159`) |
| light | `u16[]` | yes | 514² Q12 relief factors, applied only to land (`crates/arda-render/src/atlas.rs:241`) |
| classes | `TerrainKind[]` | yes | 514² ownership classes for filtered sampling (`crates/arda-render/src/atlas.rs:302`) |

### OverviewRaster

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `u32` | yes | Final image width |
| height | `u32` | yes | Final image height |
| areas_wide | `i32` | yes | Validated 1–78 |
| areas_high | `i32` | yes | Validated 1–78 |
| rgb | `u8[]` | yes | Three bytes per output pixel |
| features | `Feature[]` | yes | Private accepted variants: Sea, Land, River(Light\|Mid\|Dark), Lake |
| supplied | `bool[]` | yes | One duplicate-detection flag per area |

### AreaRaster

| Field | Type | Required | Notes |
|---|---|---|---|
| cells | `&AreaCells` | yes | Immutable saved 100 m cell grid |
| terrain | `Option<&AtlasTerrain>` | no | Atlas-derived palette/light/class grid; absent on Classic (`crates/arda-render/src/channels.rs:321`) |
| lake_colours | `Option<[u8; 3]>[]` | yes | Per-cell validated saved-lake colour, absent outside lakes |
| shapes | `Shape[]` | yes | Prepared saved channel strips, joins, terminals and preview marks |
| starts | `usize[]` | yes | Shape indexes ordered by first raster row |
| ends | `usize[]` | yes | Shape indexes ordered by last raster row |
| next_start | `usize` | yes | Scanline event cursor |
| next_end | `usize` | yes | Scanline event cursor |
| active | `BTreeSet<usize>` | yes | Shape indexes active on the current row |
| pixels | `Vec<usize>[]` | yes | Reused per-column candidate lists |
| pixel_capacity | `usize` | yes | Bounded aggregate candidate allocation |
| base_row | `u8[]` | yes | Reused terrain/lake RGB row |
| base_cell_y | `Option<usize>` | no | Saved-cell row currently represented by base_row |
| rgb | `u8[]` | yes | Reused final RGB row |
| last_y | `Option<usize>` | no | Enforces forward sequential row requests |
| side | `usize` | yes | Selected image edge |
| work | `WorkBudget` | yes | Finite polygon coverage budget scaled from output side |

### RasterBand

| Field | Type | Required | Notes |
|---|---|---|---|
| width | `usize` | yes | Exact output width |
| y0 | `u32` | yes | Inclusive band start row |
| y1 | `u32` | yes | Exclusive band end row |
| rgb | `u8[]` | yes | Reused RGB storage for at most 256 rows |
| features | `Feature[]` | yes | Reused classifications for river widening across seams |

### ChannelEdgeOut

| Field | Type | Required | Notes |
|---|---|---|---|
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `[u32; 2]` | yes | — |
| from_width_dm | `u32` | yes | — |
| to_width_dm | `u32` | yes | — |
| discharge_milli_cumecs | `u64` | yes | — |

### AccountOut

| Field | Type | Required | Notes |
|---|---|---|---|
| kind | `&'static str` | yes | accepted: reach, lake, junction, sea, domain_export; exactly one enum variant |
| global_reach_id | `String` | no | Decimal identity payload; emitted only for kind=reach |
| global_basin_id | `String` | no | Decimal identity payload; emitted only for kind=lake |
| global_junction_id | `String` | no | Decimal identity payload; emitted only for kind=junction |

### SpillOut

| Field | Type | Required | Notes |
|---|---|---|---|
| from_global_cell | `[u32; 2]` | yes | — |
| to_global_cell | `Option<[u32; 2]>` | no | — |
| sill_mm | `i32` | yes | — |
| receiving | `AccountOut` | yes | — |

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

## Relationships

`World` owns its `Manifest`, derived `HydrologyDomain`, and independent `BTreeMap<AreaCoord, OnceLock<...>>` caches. `World::load` fills empty slots only; `area()` caches a requested `Area`, `read_area()` returns owned uncached data, and the first `block()` request decodes that area's complete `BlockArchive`. Each `Area` owns cells and objects; each archive maps `(cy,cx)` to a `Block` (`crates/arda/src/world.rs:78`, `crates/arda/src/world.rs:192`, `crates/arda/src/world.rs:208`, `crates/arda/src/world.rs:274`, `crates/arda-core/src/formats/blocks.rs:67`).

Published generation runs accepted `Continent` → one evolved `SharedTerrain` across the modeled union → immutable `PreparedTerrain` slices → shared fine ocean/routing/MST/hierarchy → representative annual support → signed fine flow and final metrics → immutable per-area composition. Shared terrain is released after slice persistence and before annual-water solving. Preparation/composition are sequential; final workers do not open another area's final files. `TileBundle` supplies climate patches for prepared cells; its raw edge samples and entering loads, local `WaterGrid`, `Basin`, and the local `generate_area` route remain diagnostic data rather than production boundary or published water authority (`crates/arda-gen/src/area/prepare.rs`, `crates/arda-gen/src/orchestrator.rs:416`, `crates/arda-gen/src/orchestrator/shared_solve.rs:385`, `crates/arda-gen/src/hydrology/area_output.rs:251`, `crates/arda-gen/src/area/mod.rs:1`).

Shared preparation uses the normalized radial continent mask and bounded affine regional interpolation plus four aligned area-detail octave lattice spacings, 40/20/10/5 cells. Forty evolution iterations use whole-domain fractional MFD contributing area and a shared regional uplift maximum. Implicit downstream-first incision uses each receiver's updated bed while protecting actual submerged depressions; creep and collapse also cross area cuts. Only the actual modeled outer rim is fixed. Publication-area indices affect slicing, not the physical boundary conditions (`crates/arda-gen/src/continent/coast.rs`, `crates/arda-gen/src/continent/area_detail.rs`, `crates/arda-gen/src/area/evolution.rs`).

`BasinNodeRow.parent` and child-table spans describe physical containment, independently of annual wet occupancy. `GlobalLake.basin` is shared by all its local `Lake.global_id` fragments; `GlobalReach.id` is shared by `RiverSegment.global_id` and crossings. `AnnualCatchment` keeps original contributing ownership separate from final drainage. `ReceivingAccount::Lake` can identify a dry physical basin; `representative_lake` identifies actual positive-depth water. `SharedCrossing` is an actual directed D8 adjacency across areas; its undirected key sorts endpoints by `(x,y)`. A public multi-cell reach can contain an internal crossing. `JunctionId` names an actual dry coordinate without a separate table; tagged point reaches represent physical terminals without an invented step (`crates/arda-core/src/hydrology.rs:54`, `crates/arda-core/src/hydrology.rs:141`, `crates/arda-core/src/hydrology.rs:240`, `crates/arda-core/src/formats/hydrology.rs:789`).

`AreaHydrologyContext` carries the global records needed for that area's water and geometry, including neighboring channel halos. Referenced downstream identities may be outside that bounded copy; reading an area does not recursively fetch them. Local fragment IDs are `u32`; continent river IDs remain continent-local `u16` (`crates/arda-core/src/objects.rs:49`, `crates/arda-core/src/continent.rs:70`, `crates/arda-gen/src/hydrology/area_output.rs:155`, `crates/arda-core/src/formats/hydrology.rs:810`).

`ImageQuality` validates a requested edge and maps an overview's area-grid ratio to an exact long-edge size. `AreaImageScale::Custom` carries that validated quality into the same area renderer used by Preview and Detail. `OverviewRaster` owns a complete buffered image and tracks each supplied area, while `write_overview_png` owns only a 256-row band, one preceding feature row and one loaded `AreaCells`; a saved area may be reloaded for every intersecting band. The facade's quality exports use uncached `World::read_area`, transfer overview cell ownership with `Area::into_cells`, stream PNG rows, and publish through a unique temporary sibling followed by rename (`crates/arda-render/src/quality.rs:9`, `crates/arda-render/src/channels.rs:13`, `crates/arda-render/src/overview.rs:21`, `crates/arda-render/src/overview/streaming.rs:15`, `crates/arda/src/world.rs:54`, `crates/arda/src/export_quality.rs:20`, `crates/arda/src/export_quality.rs:58`, `crates/arda/src/export_quality.rs:81`).

`Plate` remains transient generation state. Designed `Range`, `Region`, `Sea`, `Settlement`, `Road`, society/road `Crossing`, `Pass`, `Realm`, `Building`, `Npc`, and `Poi` still have no current struct/storage producer. Water `SharedCrossing` does not implement the designed road crossing. Named regions/rivers, society partitions, shared building layouts, notable NPC sheets, on-demand commoners and sparse block POIs remain plans in `logic/01-continent-generation.md`, `logic/02-area-generation.md`, `logic/03-block-generation.md` and `logic/06-society-generation.md`.

## Boundaries

The in-memory `OverviewRaster` can use regular per-area scale or explicit image dimensions. Exact dimensions are bounded to 32,768 pixels per axis and 134,217,728 pixels total, with at least one pixel per area per axis; the original constructor retains 1–512 pixels per area and its 64-million-pixel cap. The streaming overview accepts the same exact axis and area bounds without the buffered pixel-total cap because it retains at most 256 output rows. These are rendering allocation contracts, with no saved-format or terrain-resolution change (`crates/arda-render/src/overview.rs:33`, `crates/arda-render/src/overview.rs:54`, `crates/arda-render/src/overview/streaming.rs:15`).

The five-crate boundary remains: `arda-core` owns typed records/codecs; `arda-gen` owns continent, prepared/shared hydrology, immutable area composition and block generation; `arda-render` reads stored PNG/JSON inputs without a generator dependency; `arda` provides generation/load/export APIs; `arda-cli` dispatches Generate/Preview/Export. Golden-world tests are in the root workspace (`crates/arda-core/src/lib.rs:1`, `crates/arda-gen/src/lib.rs:1`, `crates/arda-render/src/lib.rs:1`, `crates/arda/src/lib.rs:1`, `crates/arda-cli/src/main.rs:26`, `tests/golden_world.rs:69`).

Core codecs explicitly write little-endian fields; Rust padding is not persisted. The owned logical `BasinNode` view is distinct from the fixed `BasinNodeRow` plus streamed child table. JSON DTOs convert global IDs and `u128` annual litres to decimal strings; mean discharge keeps the existing `*_milli_cumecs` names and whole L/s units. Account JSON uses a snake_case `kind` discriminator and the corresponding identity field. There is no simulated month/calendar reservoir state (`crates/arda-core/src/formats/hydrology.rs:365`, `crates/arda-render/src/hydrology_json.rs:7`, `crates/arda-render/src/hydrology_json.rs:145`).

`CellOut` still omits temperature, rainfall, moisture, forest_density, road and built_by; area JSON combines cells, objects, physical channel edges and hydrology in one document. `BlockOut` retains tile IDs/names without material/traversal/movement/cover/hazard attributes, POIs or buildings. Loaded continent-object queries and independently framed lazy block decoding remain unimplemented; `World::load` reads the manifest without loading continent or global hydrology tables (`crates/arda-render/src/json.rs:18`, `crates/arda-render/src/json.rs:66`, `crates/arda-render/src/json.rs:96`, `crates/arda/src/world.rs:93`).

Area Preview is 512² pixels with faint marks for subpixel streams; Detail is 4,096² and Custom is any validated 512–32,768² edge, both showing physical channel coverage only. Every scale resamples the same immutable 512² grid of 100 m cells and saved global D8 channel geometry. Atlas adds an ephemeral 516² halo and 514² palette/light/class grids; it creates no new saved fields or world format. Supplied lake surfaces determine the existing blue-to-blue depth ramp; overview retains categorical lake fill and discharge-band river symbols. These are display choices, not changed wet membership or finer terrain (`crates/arda-render/src/atlas.rs:77`, `crates/arda-render/src/atlas.rs:159`, `crates/arda-render/src/channels.rs:11`, `crates/arda-render/src/overview.rs:278`).

## Validation

`GenerateConfig::new` accepts axes 64–4000 km, density 1–200 and ordered latitudes within −80..80. Serde construction alone bypasses that constructor; both generation admission and `World::load` explicitly revalidate it. The loader also requires manifest area dimensions to match configuration before creating caches. Area counts retain integer division by 51 km while physical tiles span 51.2 km; default remains 9×19 = 171 areas. Each modeled axis is `max(requested_km × 10, exported_areas × 512)`, retaining requested fringe and any existing export overshoot (`crates/arda-core/src/config.rs:73`, `crates/arda-core/src/config.rs:133`, `crates/arda-gen/src/hydrology/prepared_domain.rs:40`, `crates/arda/src/world.rs:93`).

The continent acceptance gate remains 250–900 per mille land and at least one sea-reaching major river within five attempts; the designed mountain-range gate remains absent. Published sea membership is the D8-connected nonpositive fine component touching the real modeled rim; enclosed negative land is not automatically marine (`crates/arda-gen/src/orchestrator.rs:369`, `crates/arda-gen/src/hydrology/ocean.rs:108`).

`ImageQuality::new` accepts 512–32,768 pixels; parsing accepts a decimal pixel count or an integer `k`/`K` suffix where 1K is 1,024, and defaults to 8,192. Overview long-edge sizing requires 1–78 areas per axis, preserves the area-grid ratio and rounds the shorter edge to the nearest pixel. Exact overview validation also requires 1–32,768 pixels per axis and at least one pixel per area on each axis; only buffered `OverviewRaster::new_exact` adds the 134,217,728-pixel ceiling. Duplicate or out-of-range buffered area pushes are refused (`crates/arda-render/src/quality.rs:9`, `crates/arda-render/src/quality.rs:41`, `crates/arda-render/src/quality.rs:65`, `crates/arda-render/src/overview.rs:64`, `crates/arda-render/src/overview.rs:104`, `crates/arda-render/src/overview.rs:197`).

CLI `--quality` is valid only for area or overview PNG output, conflicts with block output, and is refused for JSON; `--detail` remains area-PNG-only and conflicts with quality. Preview retains legacy `--px` at 1–512 pixels per area, mutually exclusive with quality; without it, Preview and PNG Export use the 8K quality default (`crates/arda-cli/src/main.rs:53`, `crates/arda-cli/src/main.rs:81`, `crates/arda-cli/src/main.rs:99`, `crates/arda-cli/src/main.rs:105`, `crates/arda-cli/src/main.rs:161`, `crates/arda-cli/src/main.rs:235`).

`MapStyle::Classic` is the default. Explicit CLI `--style classic|atlas` is limited to preview and area/overview PNG, including Atlas area `--detail`; JSON, blocks and preview `--px` reject explicit style. Atlas halo construction requires all eight neighboring directions, accepts absent directions only beyond manifest bounds, and uses widened integer gradients with one-sided differences at the outside world edge. Output axes independently use center-based linear interpolation for partitions at least 512 pixels and half-open box footprints below 512; only samples of the selected terrain class contribute, with palette and light averaged separately (`crates/arda/src/lib.rs:28`, `crates/arda-cli/src/main.rs:60`, `crates/arda-cli/src/main.rs:134`, `crates/arda-render/src/atlas.rs:176`, `crates/arda-render/src/atlas.rs:483`, `crates/arda-render/src/atlas.rs:418`, `crates/arda-render/src/atlas.rs:302`).

Area PNG output uses bounded reusable RGB row buffers. Channel candidates, polygon fragments, fixed-point coordinates and total geometry work are bounded with explicit caps, including a work budget scaled by output size; invalid saved D8 steps, nonpositive terminal widths, out-of-range raster coordinates and polygon-union excesses are typed refusals. Overview streaming requires every saved area and deliberately may reload areas across bands. Facade quality exports write a temporary sibling, flush it and rename only after successful completion, so an ordinary failed export preserves an earlier completed PNG and attempts partial-file cleanup; abrupt termination can leave the temporary file and power-loss durability is not promised (`crates/arda-render/src/carto.rs:152`, `crates/arda-render/src/channel_geometry.rs:169`, `crates/arda-render/src/channel_geometry.rs:250`, `crates/arda-render/src/channel_geometry.rs:307`, `crates/arda-render/src/overview/streaming.rs:15`, `crates/arda/src/export_quality.rs:79`).

For each 100 m cell, annual precipitation `P = rain_mm × 10,000 L`, evaporation `E = sum(monthly_evap_um) × 10 L`, land loss `A = min(P/2,E)`, runoff `R = P−A`, and additional wet-support cost `D = E−A`. Nested support/fill/spill operates on real physical bands and witnesses. Wet membership is strictly `bed < surface`; partially supported bands retain separately accounted marginal evaporation while remaining dry at their bed. This representative annual approximation makes no dated, seasonal-minimum or perennial claim. The former published 300-cell/4 m lake-retention filter and coarse seam-level substitution were replaced on 2026-09-08 (`crates/arda-gen/src/hydrology/annual_aggregation.rs:53`, `crates/arda-gen/src/hydrology/annual.rs:540`, `crates/arda-gen/src/area/shared_compose.rs:79`).

Annual source and fine-flow passes require exact canonical coverage and actual spill ownership; whole-domain outflow is reconciled before composition. Final metrics verify lake membership, use the final directed graph for drainage/Strahler and HAND, and admit resource bounds independently of timing. Supported size validation does not constitute a measured maximum-world runtime guarantee (`crates/arda-gen/src/orchestrator/annual_source.rs:313`, `crates/arda-gen/src/hydrology/fine_flow.rs:447`, `crates/arda-gen/src/hydrology/flow_metrics.rs:155`, `crates/arda-gen/src/orchestrator/generation_limits.rs:161`).

Resource admission includes two `i32` terrain inputs plus the shared kernel's scratch: 53 bytes per fine cell and container headers on the current 64-bit layout, in addition to coarse and water-stage reservations. This conservative sum admits MICRO and default sizes under 16 GiB; maximum 4000×4000 km refuses default RAM before output creation and needs explicitly larger limits. Candidate06 completes the five-world saved-data checks and reduces repeated shallow pond patterns through local-relief detail. Capacity and arithmetic checks do not close remaining drainage and regional-basin realism issues (`crates/arda-gen/src/area/evolution.rs`, `crates/arda-gen/src/orchestrator/generation_limits.rs`, `crates/arda-gen/src/orchestrator/generation_limits_tests.rs`).

Current reads require format major exactly 4; preserved format-3 worlds are refused and no migration is implemented. Fixed codecs reject invalid tags, wrong lengths, noncanonical order, invalid mean/annual pairs, impossible physical spill/crossing geometry and unsupported model revision. Area objects enforce local identity/course, feeds-cycle and disjoint lake-member constraints. `World::read_area` bounds requested cells/objects bytes before allocation and validates copied global geometry against the manifest-derived domain. Block decoding still accepts any `u16` TileId; symbolic rendering reports unknown vocabulary IDs (`crates/arda-core/src/formats/manifest.rs:75`, `crates/arda-core/src/formats/hydrology.rs:486`, `crates/arda-core/src/formats/area_objects_v4.rs:290`, `crates/arda/src/world.rs:208`, `crates/arda/src/world.rs:242`, `crates/arda-render/src/symbolic.rs:30`).

## Schema

No database or migration layer exists. World major 4, hydrology model revision 2 and export schema 2 are separate version authorities. Binary integer widths below come from the explicit codecs, not Rust memory layout; the earlier format-3/JSON-1 layouts are historical as of 2026-09-08 (`crates/arda-core/src/formats/mod.rs:17`, `crates/arda-core/src/formats/hydrology.rs:12`, `crates/arda-render/src/json.rs:15`).

| File | Layout | Codec |
|---|---|---|
| world.json | Manifest in fixed serde field order; river_count defaults to 0 on read; published last by same-directory rename after successful writes/flushes | `crates/arda-core/src/formats/manifest.rs:59`; `crates/arda-gen/src/orchestrator/publication.rs:126` |
| areas/AX_AY/cells.bin | No header; exactly 262,144 rows × 39 bytes = 10,223,616 bytes. Cell field order, including rainfall u16, discharge u64 and width u32 | `crates/arda-core/src/formats/cells.rs:17`; `crates/arda-core/src/formats/cells.rs:135` |
| areas/AX_AY/objects.bin | ARDAOBJ\0, u16 section count; four required increasing tags: rivers=1, lakes=2, channels=3, global=4. Each header is tag:u16, count:u32, payload_bytes:u32. River prefix 34 bytes plus 4 bytes/cell; lake prefix 29 plus 4 bytes/member; ChannelEdge 32 bytes | `crates/arda-core/src/formats/area_objects_v4.rs:17`; `crates/arda-core/src/formats/area_objects_v4.rs:539` |
| area objects global context | ARDACTX4, model_revision:u32, four u32 counts (lakes, catchments, crossings, reaches): 28-byte header, then fixed record rows in that order. Default limit 1,048,576 records; full objects container capped at 128 MiB before count-driven allocation | `crates/arda-core/src/formats/hydrology.rs:814`; `crates/arda-core/src/formats/area_objects_v4.rs:135` |
| continent/overview.bin | ARDAOVR\0, width:u32, height:u32, then 22-byte ContinentCell rows | `crates/arda-core/src/formats/overview.rs:21`; `crates/arda-core/src/formats/overview.rs:30` |
| continent/objects.bin | ARDACOB\0 and tagged continent-river section; widened u64 discharge, retained u16 continent-local IDs/courses | `crates/arda-core/src/formats/objects.rs:69` |
| blocks/AX_AY.tiles.zst | Retained zstd level-3 frame per area: ARDABLK\0, count:u32; each block cx:u16, cy:u16, relaxed:u8, 4,096 u16 tile IDs | `crates/arda-core/src/formats/blocks.rs:100` |

Every published `hydrology/*.bin` below begins with the 32-byte header `ARDAHYD4`, kind:u32, record_bytes:u32, count:u64, payload_bytes:u64. Complete keyed tables require unique ascending keys; child IDs are ordered within each parent span. Optional fields use an explicit presence tag and canonical zero payload when absent. `global_output::write` streams these seven files; channels remain in area objects, with no standalone channels or junctions table (`crates/arda-core/src/formats/hydrology.rs:166`, `crates/arda-core/src/formats/hydrology.rs:383`, `crates/arda-core/src/formats/hydrology.rs:500`, `crates/arda-gen/src/orchestrator/global_output.rs:65`).

| Table | Kind | Fixed row bytes | Row / ordering |
|---|---|---|---|
| hydrology/basins.bin | 2 | 76 | BasinNodeRow; BasinId |
| hydrology/children.bin | 3 | 8 | BasinId; each parent TableSpan |
| hydrology/lakes.bin | 4 | 75 | GlobalLake; BasinId |
| hydrology/reaches.bin | 5 | 69 | GlobalReach; ReachId |
| hydrology/crossings.bin | 6 | 85 | SharedCrossing; (low.x, low.y, high.x, high.y) |
| hydrology/catchments.bin | 12 | 78 | AnnualCatchment; CatchmentId |
| hydrology/metadata.bin | 1 | 172 | One HydrologyMetadata row |

Field sequences and byte widths are defined by `structure!` and `FixedRecord` (`crates/arda-core/src/formats/hydrology.rs:293`, `crates/arda-core/src/formats/hydrology.rs:337`, `crates/arda-core/src/formats/hydrology.rs:379`, `crates/arda-core/src/formats/hydrology.rs:441`). Metadata's seven `Litres` ledger fields are each u128; global identities are u64 except CrossingId's two GlobalCells. Saved rates round annual litres down by 31,536,000 seconds, so positive annual overflow can have a zero whole-L/s mean (`crates/arda-core/src/hydrology.rs:202`, `crates/arda-core/src/hydrology.rs:221`).

Schema-2 area JSON retains existing field names/units where meanings are unchanged and adds global IDs, channel edges and annual hydrology semantics; large identity/volume strings remain lossless in consumers without exact u64/u128 number support. The designed 200+ vocabulary remains 24 TileDef entries with id/name/group; no TileSet wrapper exists (`crates/arda-render/src/json.rs:33`, `crates/arda-render/src/hydrology_json.rs:145`, `crates/arda-core/src/tiles.rs:44`).
