# Shared tactical vocabulary (canonical keys for every agent)

Every crate that emits or consumes `TacticalLayout` uses these keys. The schema is in `/home/gent/code/arda-tactical/crates/arda-tactical/src/layout.rs` on branch `feat/tactical-catalogue`. Add keys only by extending this file's lists, in a comment in your report. Never rename an existing key.

## Ground keys (`Square.ground`)
Ground keys that already exist in the placeholder library:
- `grass`, `dirt`, `cobbles`, `mud`, `sand`, `gravel`
- `stone_floor`, `planks`
- `water_shallow`, `water_deep`

New ground keys:
- **Natural:** `meadow`, `forest_floor`, `leaf_litter`, `heath`, `scrub`, `moss`, `scree`, `rock`, `cliff`, `snow`, `ice`, `marsh`, `reed_bed`
- **Farmed:** `farmland` (ploughed), `pasture`, `stubble`, `fallow` (added by arda-fields)
- **Floors and yards:** `packed_earth` (floor), `flagstone` (yard or plaza), `rug` (interior accent)
- **Arid basins** (recipe 7, added by arda-refine): `salt_crust` (white evaporite crust on a dry playa), `mudflat` (pale clay flats on the playa margin; difficult terrain)

## Wall kits (`WallSegment.kit`)
- Existing: `stone`, `timber`.
- New: `wattle`, `palisade`, `hedge`, `drystone` (field wall), `city_wall`.

## Building functions (tags `function`)
- **Homes:** `house`, `farmhouse`, `cottage`, `manor`
- **Food and drink:** `inn`, `tavern`, `bakery`, `brewery`, `mill`
- **Crafts:** `smithy`, `workshop`, `tannery`, `apothecary`
- **Faith and learning:** `temple`, `shrine`, `library`
- **Trade and storage:** `warehouse`, `market`, `stall`, `dock`, `boathouse`
- **Military:** `keep`, `barracks`, `guardhouse`
- **Farm buildings:** `stable`, `barn`
- **Travel:** `toll_house`, `waystation`
- **Outdoor areas:** `street`, `farm`

## Prop IDs and tags (`prop.<name>`)
Props that already exist:
- barrel, crate, sacks, chest, table, bench, bed, cart, rowboat
- market_stall, tent, fence, dock_planks, bridge_deck, crane, well, brazier, woodpile

New props:
- **Seating and storage:** chair, stool, cupboard, shelf, bookshelf
- **Hearth and kitchen:** hearth, oven, bar_counter, cask_rack
- **Workshop:** anvil, forge, workbench, loom, grindstone, weapon_rack, armour_stand
- **Religious:** altar, pew, candle_stand, statue
- **Farm and yard:** hay_bale, trough, millstone, bucket, wheelbarrow, ladder
- **Wealth and decoration:** throne, banner, rug_small, lantern
- **Outdoor:** signpost, grave, haycart
- **Farm animals and works (added by arda-fields):** waterwheel, sheep, cow, hen. Tags: `livestock:sheep|cattle|poultry`, `hay`
- **Buildings (added by arda-town):** stairs (1×2)
- **Ways (added by arda-ways):** milestone, marker_post, ferry_rope, ferry_boat, bridge_deck_stone (a one-square paving tile)

## Vegetation IDs (`veg.<name>`)
Vegetation that already exists:
- tree_oak, tree_elm, tree_birch, tree_fruit
- bush, bush_flowering, reeds, boulder, stones

New vegetation:
- **Trees:** tree_pine, tree_spruce, tree_willow, tree_dead
- **Forest floor:** fallen_log, stump, fern, mushroom_ring
- **Low plants:** heather, flower_patch, tall_grass
- **Water plants:** cattail, lily_pads
- **Rocks:** rock_small, rock_large, scree_patch

## Placement queries
- `AssetRef::Query { class, tags }` or `AssetRef::Id("prop.anvil")`, in the serde `snake_case` form.
- Tags are drawn from the catalogue vocabulary: `function:*`, `biome:*`, `wealth:*`, plus free tags.
- `Square` and the layout use `deny_unknown_fields`. Extra per-square data, such as SRD rules, goes in a **sidecar** JSON (same width and height, row-major), never inside `Square`.

## Canonical conventions (binding; decided 2026-09-30 from integration-plan inconsistencies)
- **Cell position (I1):** cell `(gx, gy)` covers `[100·gx, 100·gx+100)` m. Its **centre** is `100·gx + 50`. Every crate uses exactly this; the server's point-at-`100·gx` convention is to be fixed at integration.
- **Squares to world (I2):** a cell is exactly 64 × 64 squares, so one square is **100/64 m (1.5625 m)** in world units. For play it counts as 5 ft. Use world metres for all geometry.
- **Road classes (I3):** use the world's stored codes, with footpath appended: `none = 0, track = 1, road = 2, highway = 3, footpath = 4`. The serde names are `"none" | "track" | "road" | "highway" | "footpath"`.
- **Road widths in squares (I4):** highway 5, road 4, track 2, footpath 1.
- **Building functions (I6, I7):** use the vocabulary list above, plus `market_hall`, `mine`, `lumber_camp`, `school`. `workshop` carries its craft as a free tag (`craft:weaver`). The serde form is a plain snake_case string, never a tagged object.
- **Tag queries (I8):** tags are `key:value` strings (`function:inn`, `biome:temperate`), and matching is exact on the full string.
- **Ids (I5):** settlement, realm, building and NPC ids are **u64**. Serialise them as JSON **strings** (JS-safe), as the server already does for large integers. Raster stores may use u32 local indices with a lookup table.
- **Water depth (I15):** shallow is under 5 ft (wade, difficult terrain); deep is 5 ft or more (swim). The renderer's `water_shallow` / `water_deep` must follow the same threshold.
- **Cover (I16):** `none | half | three_quarters | total`, the SRD terms. `full` is accepted as an input alias only.
- **Seeds (I17):** 64-bit seeds are serialised as JSON strings.
- **elevation_ft (I20):** absolute feet above sea level, rounded to 5 ft steps.
- **Per-square rules (I9):** one sidecar schema, `RulesSidecar` format 2, owned by arda-scene. Other crates mirror its field names until integration: `difficult`, `water_depth_ft`, `cover`, `blocks_sight`, `blocks_movement`, `deck`.
- **World-anchored art (I10):** layouts may carry an optional `origin` in world squares (`[i64; 2]`). The compositor hashes world square coordinates when `origin` is present, so neighbouring blocks' art joins.

## Additions recorded 2026-09-30 (from finished agents)
- **Functions:** market_hall, mine, lumber_camp, school, toll_house, waystation.
- **Biomes:** boreal, alpine, wetland.
- **Free tag:** `structured` (tiles laid in aligned 4×4-square patterns, e.g. cobbles and flagstones).

## Additions recorded 2026-10-01 (v0.4)
- **Ground:** `trail` (game trails and footpaths in open country, added by arda-refine); `salt_crust`, `mudflat` (recipe-7 arid basins, listed above)
- **Vegetation:** `veg.tree_alder`, `veg.tree_stunted`, `veg.juniper`, `veg.rock_outcrop` (3×3, total cover, blocks movement)
- **Props:** `prop.drain` (in vocabulary; kept out of layouts until art exists)
