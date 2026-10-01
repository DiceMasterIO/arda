# arda-town

Town layouts for Arda (goals 36, 44, 45 and 64). The crate turns a settlement record and its terrain into a **town plan** in world metres, then cuts any window of that plan into **tactical blocks** of 5-ft squares, with furnished interiors, that render through `arda_tactical::render`.

Output is deterministic for a given world seed and settlement id.

```sh
arda-town render --site aldermere --out out/town/aldermere-plan.png        # plan overview
arda-town block  --site aldermere --window 385,322,64,64 --out block.png   # plan-relative squares
arda-town block  --site aldermere --focus inn --size 32 --ppsq 96 --out inn.png --json inn.json
arda-town info   --site highcrag                                           # summary, mix, notes
arda-town export --site thornby --out out/town                             # plan JSON + BuildingSpecs
```

The synthetic sites are `aldermere`, a riverside market town of 2,600 at a bridge; `thornby`, a crossroads village of 320; `highcrag`, a walled hilltop town of 1,800 under a castle; and `saltwick`, a fishing hamlet of 60 on a cove.

`block` takes these options:

- `--library`: the library directory, or a `top:…:bottom` stack such as `out/art:assets/tactical/placeholder` whose lower libraries fill in what the upper ones lack (see the arda-tactical README); `assets/tactical/placeholder` by default;
- `--ppsq`: output pixels per square, 48 by default;
- `--grid`: draw the square grid.

`--focus` accepts `market`, `inn`, `river` (the bridge or landing), `temple`, `gate`, `smithy` or `building:<id>`. `block` prints every fallback it used.

## Inputs

### `TownSite`

`TownSite` mirrors one record of `society/settlements.json` from `arda-settle` (goal 04, step 8), using the same field names and spellings. Unknown fields are ignored.

| field | type | notes |
|---|---|---|
| `id` | `SettlementId` (u64) | written as a JSON string; read from a string or a number |
| `name`, `culture`, `biome` | string | the culture selects a building tradition (below) |
| `tier` | `hamlet` \| `village` \| `town` \| `city` | towns and cities are walled |
| `population` | u32 | sets the core radius by density: 14, 45, 170 and 230 people per ha |
| `functions` | `farming`, `market`, `fortress`, … | `fortress` and `capital` build a castle ward |
| `wealth` | u8 | the base for each building's wealth |
| `x_m`, `y_m` | i64 | centre in metres: east of the west edge, south of the north edge |
| `site_tags` | strings | `ford`, `bridge_site`, `harbour`, `defensible`, … |
| `buildings` | map key → count | the estimated building mix; `market` is an alias of `market_hall`, and `mine` and `lumber_camp` are noted as off-plan |

### `TerrainInput`

`TerrainInput` holds callbacks over world metres:

- `height(p)`: metres above sea level;
- `slope(p)`: degrees;
- `water(p)`: a mask for river, lake and sea;
- river centrelines with widths;
- entering road polylines with a `RoadClass` (`none`, `track`, `road`, `highway`, `footpath`).

`TerrainInput::from_height` derives the slope from the height field. Roads may be coarse (100 m cell centres): they are smoothed before use.

## Plan (`plan::generate` → `TownPlan`)

The pipeline runs in order, and each step reads only earlier outputs:

1. **Focal point.** In order of precedence:
   - a castle on the highest ground (fortress sites);
   - a harbour at the shore (coastal fishing and port sites);
   - a river crossing, with the market 70–140 m up the road on the town side;
   - a road junction;
   - the nearest road.
2. **Main streets.** Each entering road is split at its closest point to the market, and each half becomes an arm running outwards. Widths follow convention I4: highway 5, road 4, track 2, footpath 1 squares. Arms get a low-frequency wobble that fades to nothing at the market, and near-duplicate arms are merged.
3. **Market square.** Towns get a spindle, a widened street along the through road or the castle approach. Villages get a green between the arms.
4. **Castle ward.** A bailey with a two-square curtain wall and a gate facing the market.
5. **Wall ring.** An ellipse stretched about 1.35:1 along the main road and roughened. On hills it follows the contour at the core radius. It always encloses the square and the keep.
6. **Lanes.** A radio-concentric web:
   - ring segments between neighbouring arms, one plot depth apart, bowed and noisy;
   - secondary radials in wide wedges;
   - an intramural lane inside the wall;
   - a strand lane along the shore for harbours.
7. **Rasterisation** onto the square grid:
   - squares are `SQUARE_M` = 100/64 m, so a 64 × 64 block is one 100 m cell;
   - the grid origin is a multiple of 64 squares;
   - main streets become straight, axis-aligned bridges over water;
   - wall crossings become gatehouses: a three-square passage between two-square towers;
   - rivers get water gates.
8. **Off-plot buildings.** The keep and barracks go in the bailey, the market hall and stall rows in the square, and docks and boathouses on the shore.
9. **Burgage plots.** Each street side is walked in half-square steps. The outward normal snaps to a grid axis, each column finds its front cell beside the street, and the runs are cut into plots:

   | tier | frontage (squares) | frontage (m) |
   |---|---|---|
   | city | 3–5 | 4.7–7.8 |
   | town | 4–6 | 6.3–9.4 |
   | village | 8–13 | 12.5–20 |
   | hamlet | 9–15 | 14–23 |

   Plots are claimed nearest the market first. Every column reaches back to a common rear line, and the plot stops at other claims and at a one-square gap before the wall. Curved streets produce staggered fronts, as in real towns, while every plot stays on the tactical lattice.
10. **Function assignment** (goal 36):
    - temples and the first inn go on the square;
    - later inns, smithies, guardhouses and stables go by the gates;
    - warehouses and mills go on the water, and tanneries downstream and outside the walls;
    - manors take deep plots a little way out;
    - homes take the nearest remaining plots: farmhouses the largest, cottages the outermost.

    Wide functions merge neighbouring plots until the frontage fits.
11. **Footprints.** Town houses fill their frontage, giving a continuous street front. Village buildings stand detached. Farmhouses get a barn at the back of the plot (marked `ancillary`) with a side passage. The first door faces the street; a back door opens onto the yard.
12. **Zoning.** Unbuilt plot squares become one of `front`, `yard`, `garden` or `churchyard`. Plots and districts are assigned: `market`, `residential`, `craft`, `waterfront`, `religious`, `castle`, `suburb` and `farmstead`.

If the plots cannot hold the mix, the core radius grows by 20 % and the plan is rebuilt, up to six attempts. A building that does not fit its plot moves to the next free plot. The last attempt keeps what fits and records the rest in `notes`.

`TownPlan` serialises to JSON (`to_json`) without the grid:

- `streets` (with class and paved flag), `square`, `plots` (with frontage, depth, district and front side), `districts`, `wall` (ring and gates), `castle`;
- `buildings`: `id`, `function`, `plot`, `footprint` polygon in metres, `rect` in global squares, `storeys`, `wealth`, `wealth_level`, `front`, `doors`, `ancillary`, `tags` (`craft:*` on workshops);
- `base_height_m`, `water_level_m`, `seed` (a string) and `notes`.

Building ids are 1-based u64 values serialised as strings, like `arda-npc`'s `BuildingId`. `plan::spec::specs` emits one `BuildingSpec` per building: `id`, `settlement_id`, `function`, `capacity`, `workplace_slots`, `wealth` and `tags`. Capacity and slots mirror the `arda-npc` tables.

`plan::check` holds the invariants the tests use:

- `overlaps`;
- `unreachable`: a door that cannot reach public ground over open land or its own plot;
- `escapes`: a flood fill from the market, with gates open or shut;
- `frontages` and `built_mix`.

## Tactical blocks (`block::generate` → `TownBlock`)

A `Window` is a rectangle of global squares, up to 1024 squares on a side. The block holds:

- `layout`: a `TacticalLayout` using the canonical vocabulary keys;
- `buildings`: the sidecar, giving each plan building touching the window with its id, function, whole footprint, the part inside this block, its doors, and its rooms (`common_room`, `kitchen`, `nave`, `forge`, …);
- `rules`: per-square rules with `arda-scene` `RulesSidecar` field names (`difficult`, `water_depth_ft`, `cover`, `blocks_sight`, `blocks_movement`, `deck`);
- `origin`: the world square of layout square (0, 0), for convention I10; the layout's own `origin` carries the same value, so the compositor hashes world coordinates;
- `owned`: which squares the town reserves (every plan kind but open country and water, logic/09 §reservations precedence 1).

`TownBlock::to_rules()` gives the same rules as `arda-scene`'s `RulesSidecar` format 2 (I9): owned squares state them in full (the water under a bridge deck is kept, with `deck` set), squares inside a plan building carry `ext.building`, the building id as a string, and all other squares state nothing.

What goes into a block:

- **Ground:**
  - streets are `cobbles` where paved, otherwise `dirt` and `mud`;
  - the square is `cobbles` (`flagstone` in cities) and the green is `grass`;
  - plot fronts are `packed_earth` or `gravel` in towns and `grass` in villages;
  - yards are `packed_earth`, and gardens are `grass` with `farmland` beds;
  - open country is `meadow` or `pasture`;
  - water bed is `mud`, with a depth of 2 ft per square from the bank, so shallow means under 5 ft (I15).

  Elevation is absolute and in 5-ft steps (I20). Floors, squares, greens and baileys are levelled, and water sits at one level. The wall walk stands 20 ft above the ground.
- **Walls:**
  - shells use the kit for their culture and wealth: `stone` for civic buildings, manors and the wealthy; `timber` or `wattle` by tradition (heartland and coastal are timber, highland and southern stone, sylvan and borderland wattle); partitions are `timber`, or `wattle` for the poor;
  - the curtain and castle walls use `city_wall`, with `gate` pieces across passages and water gates;
  - bridges have `stone` parapets;
  - a shared party wall keeps the stronger piece (door over run over window), and the curtain always wins.
- **Interiors.** Each programme is authored with the street front to the north, then rotated onto the footprint:
  - inn: common room, bar and casks, hearth, tables, kitchen with oven, stair hall;
  - tavern;
  - smithy: forge area with anvil, trough and weapon rack;
  - temple: nave with pews, aisle, chancel with altar, statue and candles, vestry;
  - shrine; house (hall, bedchamber, store); cottage; farmhouse (long house with byre); manor (hall, solar, kitchen); keep (great hall with throne, armoury, chamber); barracks; guardhouse;
  - bakery, brewery, mill; workshops by craft (loom, workbench, kiln); tannery; apothecary; library; school;
  - warehouse: goods as `function:warehouse` + `container` tag queries, and an office;
  - market hall (arcade), stall, dock, boathouse, stable (stalls), barn.
- **Dressing:**
  - fruit trees and flowering bushes in gardens, graves and elms in churchyards, reeds on banks, trees and bushes in the fields;
  - barrels, benches and a lantern at inn doors;
  - crates at warehouses and woodpiles at smithies and in yards;
  - goods beside stalls, a well and braziers in the square;
  - cargo and a boat at docks;
  - braziers at gates;
  - plank decks on bridges.
- **Lights.** Hearths, ovens, forges, candles and lanterns give explicit lights, each tied to its prop.

**Seams (goal 46).** Everything is a pure function of global squares and plan objects:

- ground and curtain edges come from the grid;
- interiors come from the building id;
- dressing belongs to a feature (a square, a building, a stall) and uses only cells that feature owns;
- edges on a window's border appear in both neighbours;
- props that overlap the border appear in both, offset, and the compositor clips them.

The tests check that two 64-square windows equal one 128-square window, square by square, edge by edge and prop by prop.

## Library fallbacks (`block::fallback::resolve`)

`resolve` maps a canonical block onto a concrete library and returns the resolved layout plus a sorted list of `Fallback { kind, wanted, used, count }`. For each key the library lacks, it uses the nearest available relative along a fixed chain:

- **Ground:**
  - `meadow`, `pasture` → `grass`;
  - `packed_earth`, `farmland` → `dirt`;
  - `flagstone` → `stone_floor`;
  - `rug` is dropped into the surrounding floor.
- **Kits:** `wattle`, `palisade`, `hedge` → `timber`; `city_wall`, `drystone` → `stone`.
- **Props:** by use and footprint:
  - `chair` → `stool` → `barrel`;
  - `cupboard` → `shelf` → `chest`;
  - `hearth` → `oven` → `brazier`;
  - `bar_counter`, `altar`, `workbench` → `table`;
  - `cask_rack` → `barrel`; `pew`, `trough` → `bench`;
  - `hay_bale` → `sacks`; `millstone` → `well`; `statue` → `veg.boulder`; `grave` → `veg.stones`;
  - `stairs` → `ladder` → `bridge_deck`;
  - `banner` and `rug_small` are left out, since they are purely decorative.

Tag queries use `key:value` strings (I8) and are resolved by hashing the world square, so a query picks the same asset in every block. When nothing matches, tags are relaxed from the end. Rotations fall back to the nearest allowed quarter turn.

## Conventions applied

From `docs/goal-prompts/vocabulary.md`, "Canonical conventions":

- I1: cell centres at 100·gx + 50;
- I2: 100/64 m squares;
- I3 and I4: road codes and widths;
- I5 and I17: ids and seeds as strings;
- I6 and I7: plain snake_case functions, and `craft:*` tags on workshops;
- I8: `key:value` queries;
- I9: rules sidecar field names;
- I15: water depth;
- I16: cover;
- I20: absolute elevation in 5-ft steps.

One vocabulary addition is proposed: `prop.stairs` (1 × 2), for stair halls in inns, manors, keeps and two-storey houses.

## Limits

- Upper storeys are not laid out; stairs mark where they start.
- Walls are square-edge staircases where the ring runs diagonally, which is the tactical grid's limit. Gatehouses are squared off.
