# arda-settle

Settlements, roads, realms and names for a generated arda world (goals 34–41).
It reads a stored world through `arda::World::load`, derives the society
layers, and writes them to `<world>/society/`. It changes nothing else in the world directory.

```sh
cargo build --release -p arda-cli -p arda-settle
target/release/arda generate --seed 42 --micro --terrain fine --out out/micro42
target/release/arda-settle generate --world out/micro42
target/release/arda-settle render --world out/micro42 --out out/settle/micro42-overlay.png --quality 4096
```

`generate` prints the statistics below and writes them to `society/stats.json`.
`render` draws the overlay over the Atlas overview, which it gets from
`arda::export_overview_with_quality_and_style`, and crops the page to the
land (`--full-frame` keeps the whole overview). See "Overlay" below.

## Pipeline

Each stage reads only the outputs of earlier stages. Random choices come from
per-entity streams seeded by `(seed, stage tag, cell or entity key)`
(`rng.rs`). Generation uses integer or fixed-point maths; floating point
appears only in the reported statistics, the overlay and the synthetic test
fixture.

1. **Grid** (`load.rs`, `grid.rs`). Every area tile is stitched into one
   raster that holds only the fields the stage reads, about 26 bytes a cell.
   Coast is not stored, so it is derived from sea neighbours. Confluences come
   from the saved river segments: a segment fed by two or more tributaries of
   order 2 or more.
2. **Site tags** (`tags.rs`). The stage first finds point features, then
   gives the tag to every land cell within a short walk of one (200–500 m):
   - `ford`: a reach 2–14 m wide, of order 3 or less, gentler than 4°, with
     firm banks (no marsh, 8° or less);
   - `bridge_site`: a wider reach with firm banks;
   - `confluence`;
   - `harbour`: a coastal cell open to the sea through 1–4 of 16 rays of
     4 km, with water at least 3 m deep within 300 m;
   - `estuary`: a river mouth of order 3 or more;
   - `pass`: a saddle whose ground 1 km away is at least 30 m higher on both
     sides along one axis and 30 m lower across it, taking the lowest
     candidate nearby;
   - `defensible`: prominence of 40 m or more above the mean of the
     surrounding kilometre (`hill` from 20 m);
   - `ore`: see "Ore proxy" below;
   - `timber`: 75 % forest over the surrounding kilometre;
   - `fish`: sea, lake or an order-3 river nearby;
   - `salt`: a warm, flat, low coast, or coastal marsh;
   - `spring`: a channel head;
   - descriptive tags: `river`, `lake`, `coast`, `forest`, `marsh`,
     `mountain` and `navigable` (order 4 or more).
3. **Suitability** (`suitability.rs`), following the artifact's "Where people
   settle". Each component is scored per mille and weighted:
   - water within 150 m scores fully, fading to nothing at 750 m (weight 30);
   - the arable share of the surrounding kilometre, full at 60 % (26);
   - flat ground (10), warmth (10) and a view, full at 30 m of prominence (8);
   - a sheltered coast: harbour, then estuary, then open shore (6);
   - a confluence or ford (10).

   **Arable** means land that is not a channel, under grass, scrub or
   clearable forest, 8° or gentler, at least 4 °C, with moisture of 60 or
   more and at least 1 m above the river. **Refused** sites are water,
   channels, marsh, rock, ice, beach (bare coastal ground), slopes over 14°,
   ground colder than 3 °C, and anything less than 1.5 m above the nearest
   watercourse.
4. **Placement** (`place.rs`, `central.rs`). Population is the manifest's
   density times the land area. Three tenths of it lives in towns and 47 %
   in villages; the rest lives in hamlets.
   - Towns follow the rank-size rule, taking as many towns as the rule allows
     while the smallest still has 1,000 people. Town *k* has *P₁/k* people;
     8,000 or more makes a city.
   - **Towns are central places** (Christaller, Lösch). A market town lives
     on the land a farmer can reach, trade in and leave within a day, so a
     town site is scored mostly by its **market catchment**, the arable
     share of the 16 km square around it (55 %), blended with the local
     suitability (45 %). A coastal site loses the half of its catchment
     that is sea. Each kind of node then adds once: a harbour 130 (or an
     estuary 80), a confluence 130 (or a ford or bridge 100), a navigable
     reach 80, a pass 40 and a defensible hill 30. Harbours matter, but a
     fertile inland basin at a river crossing can beat a bare harbour.
   - Towns share the land out. With *n* towns over land *A*, a hexagonal
     market lattice has spacing `d = √(2A / (√3 n))` (27.7 km on MICRO).
     Each placed town tolls every site within `d` by up to 320 in
     proportion to its nearness, and no two towns stand closer than 62 % of
     `d`, relaxed in steps (54, 46, 38, 30 %) only when the land cannot hold
     them all, and never under the artifact's 8 km. The best site is placed
     first and becomes the largest town.
   - Villages and hamlets are placed greedy, best site first, with the
     artifact's spacing: villages 1.5 km from towns and 2 km from each
     other; hamlets 900 m from anything. They add small bonuses for a
     harbour or a river node, and a crowding toll lowers a site's score for
     each settlement of the same or a higher tier nearby: 110 per village
     within 5 km and 70 per hamlet within 2.5 km, so density follows
     farmland.
   - A village has 120 + 450 × (arable share) people and a hamlet has
     12 + 68 × (arable share), each with a little jitter.
5. **Profile** (`profile.rs`). Functions come from the site tags and the
   surrounding land:
   - farming and pastoral from the arable share;
   - fishing and port from fish, harbour, estuary and navigable-river tags;
   - market for towns and villages of 450 or more;
   - mining for the three non-city settlements nearest each mineral
     district's ore, within 1.5 km of it (see "Ore proxy");
   - logging from timber, and crafting for towns;
   - fortress for towns on a hill or pass, villages holding a defensible pass,
     and realm seats;
   - abbey for about 1 village or town in 14 away from harbours;
   - crossing from ford, bridge and confluence tags, and capital for seats.

   Wealth (0–255) comes from the tier, the arable share and the trades.
   Culture is a regional key read from a 6.4 km lattice, where each lattice
   cell summarises the ~19 km around it; the keys match `arda-npc`'s
   `cultures.json`. The profile also records the biome and an estimated
   building mix keyed by `arda-npc` building functions.
6. **Land use** (`landuse.rs`). The built footprint takes cells at 150, 110,
   30 and 20 people per hectare for a city, town, village and hamlet, on
   buildable ground only.
   - Fields cover **0.8 ha a head**. They take the nearest arable cells, open
     ground before forest, within 16, 12, 4 and 2.5 km for a city, town,
     village and hamlet.
   - A tenth of village and town fields on warm, sunny slopes of 3–8° are
     orchards, and about a third of the rest lie fallow (a three-field
     rotation).
   - Villages and hamlets keep hay meadow on floodplain grass (0.2 ha a
     head), below the 1.5 m line where no one builds.
   - Villages and hamlets add 0.4 ha of pasture a head on ground too rough to
     plough, and keep woodland on forest slopes steeper than 12°. Towns keep
     no pasture.
   - Each village or town gets a mill beside the nearest stream of order 2 or
     more, and each mining settlement gets a mine at the nearest ore cell.

   Settlements are processed largest first. Buildings override fields, and
   fields override pasture.
7. **Roads** (`cost.rs`, `route.rs`, `roads.rs`, `crossings.rs`).
   - **Costs.** They follow the artifact's cost table, in metre-equivalents
     where 100 m of flat grass costs 100:
     - fields cost 110, scrub 120, forest 125–200, alpine ground 160–180,
       marsh 450, rock 900 and ice 1,400;
     - slope multiplies the cost by `100 + 3g + 2(g−12)² + 20(g−30)²` per
       cent, where *g* is the grade along the step read against the class's
       sustained grade (`g × 10 / max`, with max 8 % for a highway, 10 % for
       a road, 14 % for a track and 25 % for a footpath), so a highway
       switches back up a slope that a footpath climbs straight;
     - a crossing adds `100·w` for a channel *w* m wide up to 10 m, and
       `1000 + 75(w−10)` above that, which makes 10 m worth a kilometre of
       detour and 50 m worth four;
     - existing road costs 30 % per metre, and open water 400 % per metre plus
       a 40 km-equivalent landing, so a ferry is used only where no land route
       is sensible;
     - water is always crossed orthogonally, and saddles are 15 % cheaper;
     - a two-octave hash texture (1.2 km and 400 m) adds up to 60 %, standing
       in for hedges and holdings, so open-ground routes wander like lanes.
   - **Hierarchy.** The network is built top-down:
     - least-cost routes join each town to its four nearest towns, and their
       minimum spanning tree becomes the **highways**;
     - highways are pushed from the nearest town to the middle of every land
       run of 10 km or more along the map edges;
     - each village takes a **road** to whichever of its three nearest towns
       is cheapest to reach (one A* toward all three), largest village
       first;
     - each hamlet takes a **track** to the cheapest of its three nearest
       villages or towns;
     - a **footpath** joins each village to its nearest village within 4 km,
       but only when it is a short cut (at most twice the straight line);
     - **no runaway detours**: a village or hamlet route that runs more than
       twice its straight line over the network (typically along a trunk
       winding round a hill) is re-routed directly, ignoring the network,
       and the direct route is kept when it is shorter. Trunk routes keep to
       the network, so no highway runs parallel to another;
     - every road records its **terrain** class by the median slope along
       its whole route: open under 5°, hill 5–12°, mountain 12° or more.

     Routes fall onto existing road, and only a route's new cells become a
     road object, so segments are shared and never duplicated. The routing
     is a windowed A* (admissible at road cost).
   - **Crossings.** Every connected stretch of road over a channel or open
     water is one crossing object:
     - streams narrower than 14 m are bridged within 1 km of a village or on
       a highway, and forded elsewhere;
     - rivers of 14–50 m are bridged on highways or near settlements, and
       forded up to 20 m or ferried beyond that;
     - wider rivers are ferried unless a highway comes within 3 km of a town;
     - open water is always a ferry.
   - **Passes.** A route's high point is a pass when it stands at least
     120 m above the lowest point on both sides and lies within 300 m of a
     saddle.
8. **Realms** (`seats.rs`, `market.rs`, `primacy.rs`, `realms.rs`,
   `snap.rs`), following `logic/06` steps 1–3. Seats and primate capitals
   are settled right after placement, before land use, so fields and roads
   serve the final populations.
   - **How many.** *N* is the smallest of: the population over 24,000 (a
     capital city needs a realm three times its size), the habitable land
     (suitability above zero) over 600 km², and a third of the towns; at
     least 2 when there are 2 towns or more.
   - **Which towns** (`seats.rs`). Candidates are the larger half of the
     towns. The largest is the first seat; each further seat maximises
     `d² × population`, `d` being its travel cost to the nearest chosen
     seat over a coarse market lattice (`market.rs`: about 20,000 blocks,
     a step costs its length plus 8 m per metre of climb). The seats then
     move, one at a time, to another town of their own market area when
     that cuts the imbalance of the areas' shares (people ¾, land ¼) by a
     twentieth. If a market area holds fewer than 16,000 people, the land
     holds one realm fewer and the choice runs again.
   - **Primate capitals** (`primacy.rs`). In every market area of 12,000
     people or more, the towns keep their townspeople `U` and follow the
     rank-size rule with the seat first (`h / r`, `h = U / H_n`). The seat
     grows to at least 8,000, a city, and the court lifts the other towns
     by the fourth root of the capital's growth. The people come from the
     area's villages and hamlets, which shrink in proportion within their
     tier bounds. Towns and cities are then re-ranked by population.
   - One multi-source Dijkstra runs over the road and terrain cost surface
     and gives every cell to the seat cheapest to reach. Ties go to the
     higher-ranked seat. Stepping over a river of order 3 or more costs a
     toll of 800, and stepping onto a ridge crest costs 600. A ridge crest
     is a divide cell with 40 m of prominence, thickened by a cell.
   - **Border snapping** (`snap.rs`) is an explicit pass. Every land cell
     within three steps of the raw border is handed out again by a flood
     from the cells just outside that band, which keep their realm. The
     flood pays 1,000 for stepping off a river or ridge line (grown by one
     cell to close the gaps of a patchy crest) onto open ground, so each
     realm fills the band up to the line on its own side and the border
     settles on the line. The pass runs twice. Then any fragment of a realm
     cut off from its seat goes to the neighbour it borders most, unless it
     touches no other realm (an island). Seats never move.
   - Water cells are 0, and every land cell has exactly one realm.
   - A realm keeps at most two cities; any further city becomes a town of
     7,999.
9. **Names** (`naming.rs`, `names.rs`, `features.rs`) come from
   `arda-names`.
   - Each culture has one invented language (`Preset::Heartland` and so on,
     keyed like the culture), and every language of a world shares one
     substrate tongue, so old river names agree across borders.
   - Each culture region spreads its language with its own `DialectMap`, so
     a place is named in the dialect of its region at its own cell.
   - One `NameScope` covers the whole world, so no two places share a
     spelling. Places are named in a fixed order: rivers, peaks,
     settlements by id, passes, realms, regions.
   - Settlements are named with `place_name` from their tier and site tags.
     Rivers of order 4 or more are `River`s (substrate names); smaller ones
     are `Stream`s. Peaks are `Mountain`s and passes `Pass`es. A realm
     derives from its capital's name ("Oakmark") and a region from its
     largest settlement ("Oakfordshire").
   - Every record stores the native form in `name` and the English gloss in
     `name_gloss`.
   - Rivers are traced as main stems from each mouth of order 3 or more,
     with side branches of order 3 or more becoming rivers of their own.
     Mountains are summits 150 m above their surrounding 5 km.
   - The history hook comes from the settlement's site and the nearest
     named river or peak, for example "grew at the ford of the Teyn".

### Ore proxy

The world has no geology, so ore stands in for it in two steps (`ore.rs`):
- **Mineral districts.** One 8 km lattice block in seven, chosen by a
  seeded hash, holds a district: a disc of 2.5 km radius around a seeded
  point in the block. Deposits therefore cluster, as real ore fields do.
- **Deposits.** Inside a district, a cell carries ore when at least 10 % of
  the surrounding kilometre is bare rock and its mean slope is 15° or more.

Only the three settlements nearest each district's ore (within 1.5 km, not
cities) mine it, so mining is a rare, local trade.

## Output formats (`format_version` 1)

All JSON is pretty-printed, and the key order is fixed by the struct
definitions. Following the canonical conventions (`vocabulary.md`), every
id (settlement, realm, road, crossing, pass, river, region) is a u64
written as a JSON **string**, and so is the seed. Positions are metres from the world's north-west corner (`x`
east, `y` south). Cell `(x, y)` covers `[100x, 100x+100)`, and its centre is
`100x + 50`.

### `settlements.json`

```json
{
  "format_version": 1,
  "seed": "42",
  "width_cells": 1024,
  "height_cells": 2048,
  "settlements": [ … ]
}
```

A settlement record has these fields:

| field | type | notes |
|---|---|---|
| `id` | u64 as string | 1-based, in placement order (towns by rank, then villages, then hamlets) |
| `name` | string | |
| `tier` | `hamlet` \| `village` \| `town` \| `city` | |
| `population` | u32 | |
| `functions` | array of `farming`, `pastoral`, `fishing`, `port`, `market`, `mining`, `logging`, `crafting`, `fortress`, `abbey`, `crossing`, `capital` | sorted |
| `wealth` | u8 | 0–255 |
| `culture` | string | `heartland`, `highland`, `sylvan`, `coastal`, `southern` or `borderland` |
| `realm_id` | u64 as string | 1-based |
| `biome` | enum | `temperate`, `warm_temperate`, `temperate_forest`, `boreal_forest`, `highland`, `alpine`, `wetland`, `steppe` or `coastal` (see `model::Biome`) |
| `coastal`, `riverine` | bool | |
| `name_gloss` | string | English gloss of `name` ("Oakford") |
| `x_m`, `y_m` | i64 | centre |
| `cell_x`, `cell_y` | u32 | centre cell |
| `height_m` | i32 | |
| `rank` | u32 | rank among towns and cities; 0 otherwise |
| `site_tags` | array of string | see stage 2 |
| `history` | string | origin hook |
| `buildings` | object | building-function key → count, using the shared vocabulary (`house`, `farmhouse`, `cottage`, `manor`, `inn`, `tavern`, `bakery`, `brewery`, `mill`, `smithy`, `workshop`, `tannery`, `apothecary`, `temple`, `shrine`, `library`, `warehouse`, `stall`, `dock`, `boathouse`, `keep`, `barracks`, `guardhouse`, `stable`, `barn`) plus `market_hall`, `mine`, `lumber_camp` and `school` |

`name` is the native form from `arda-names`. The first eleven fields are exactly `arda-npc`'s `SettlementProfile`, with
the same names and the same snake_case enum values. Ids are strings and
`biome` is a closed enum under the canonical conventions, so `arda-npc`'s
numeric `SettlementId` and free-text `biome` must follow the same
convention at integration. `ancestry_mix` is left out and takes
its serde default. The remaining fields are extras that the profile ignores.

### `roads.json`

The file holds `{format_version, roads, crossings, passes}`.

- A **road** holds `id`, `class` (`track`, `road`, `highway` or `footpath`,
  whose stored codes are 1, 2, 3 and 4, with 0 for none: the world's
  `RoadClass` codes with footpath appended),
  `from` (a settlement id), `to` (a settlement id or null) and `to_edge`
  (`north`, `east`, `south`, `west` or null). It also holds three lengths in
  metres: `length_m`, the whole route including shared stretches;
  `straight_m`, the straight line between its ends; and `new_m`, the new cells
  only. `relief_m` is the height range along the route and `terrain` its
  class (`open`, `hill` or `mountain`). `segments` lists the
  new stretches, each a polyline of `[x_m, y_m]` cell centres with collinear
  points dropped. A segment's first or last point is the existing road cell
  it joins.
- A **crossing** holds `id`, `kind` (`bridge`, `ford` or `ferry`), `water`
  (`river`, `sea` or `lake`), `x_m`, `y_m`, `width_m`, `order`,
  `road_class` (the highest class using it) and `river` (its name, or `""`).
- A **pass** holds `id`, `x_m`, `y_m`, `height_m`, `rise_from_m`,
  `rise_to_m`, `road_id`, `name` and `name_gloss`.

### `realms.json`

The file holds `{format_version, realms}`. A realm holds `id`, `name`,
`name_gloss`, `seat` (a settlement id), `culture`, `settlements` (member ids),
`population`, `land_cells` (in hectares), `neighbours` (realm ids) and
`borders`. `borders` is a list of land-border polylines of `[x_m, y_m]` cell
corners, shared with the neighbour on the other side.

### `names.json`

The file holds `{format_version, rivers, mountains, regions}`.
- A river holds `id`, `name`, `name_gloss`, `mouth_m`, `length_m`, `order`
  and `course_m`, its main stem from the mouth upstream every 400 m.
- A mountain holds `name`, `name_gloss`, `at_m` and `height_m`.
- A region holds `id`, `name`, `name_gloss`, `culture` and `land_ha`.

### `landuse.bin` and `realms.bin`

Both rasters share one layout:

| bytes | content |
|---|---|
| 0–7 | magic: `ARDALND\0` for land use, `ARDARLM\0` for realms |
| 8–11 | `format_version`, u32 LE (1) |
| 12–15 | width in cells, u32 LE |
| 16–19 | height in cells, u32 LE |
| 20– | one zstd frame (level 9) of the row-major payload |

- **`landuse.bin`** has two planes: `width × height` u8 codes, then
  `width × height` u32 LE owner ids (a settlement id, or 0). The codes are
  0 none, 1 built (village, town or city footprint), 2 arable, 3 pasture,
  4 orchard, 5 woodland, 6 mill, 7 mine_quarry, 8 meadow, 9 fallow and
  10 farmstead (a hamlet's footprint); `landuse::code::name` gives the
  class name. Farmland is arable, fallow and orchard.
- **`realms.bin`** has one plane of `width × height` u16 LE realm ids, with
  0 on water.

`output::read_landuse` and `output::read_realm_map` decode them.

### `stats.json`

The plausibility summary, for reading only. Besides counts, populations,
the rank-size fit, farmland per head, road kilometres and crossings, it
holds:
- `inland_town_share`, `town_nni` (the Clark–Evans nearest-neighbour index
  of towns over the land: 1 is random, 2.15 a perfect hexagonal lattice)
  and `town_nn_km`;
- `sinuosity_median` per road class, `sinuosity_all`, `sinuosity_open`
  (open terrain, the artifact's 1.2–1.4 check), `sinuosity_by_terrain`
  (routes, median, p90 and max for `open`, `hill` and `mountain`) and
  `runaway_routes` (over 2.5), all over routes of 1.5 km or more;
- `border_on_river_or_ridge_pm`: of the realm border that has a river
  (order 3 or more) or ridge line within two cells, the per mille that runs
  on it (a cell beside the border edge is on or next to the line), with
  `border_on_river_or_ridge_raw_pm` before snapping,
  `border_natural_of_all_pm` over all border, and
  `land_near_river_or_ridge_pm` as a baseline;
- `mining_settlements`, `mining_share_pm` and `mining_clustered_pm` (the
  per mille of mining settlements with another within 4 km).

## MICRO fixture (seed 42, `--terrain fine`)

The whole stage takes about 4.5 s on MICRO (1,024 × 2,048 cells). Memory
grows linearly with cells, so a 500 × 1000 km world is estimated at about
5 GB (not yet measured). "Before" is the first version of this crate.

| statistic | before | now |
|---|---|---|
| settlements | 1 city, 8 towns, 138 villages, 495 hamlets (90,669 people) | 1 city, 8 towns, 138 villages, 497 hamlets (90,947 people) |
| rank-size slope | −1.000 (R² 1.000) | −1.000 (R² 1.000) |
| towns inland | 2 of 9 | 5 of 9 |
| town nearest-neighbour index | 1.26 (16.3 km) | 1.90 (24.6 km) |
| farmland per head | 0.75 ha | 0.77 ha |
| road km (new cells) | highway 174, road 970, track 1,032, footpath 20 | highway 266, road 1,014, track 974, footpath 14 |
| sinuosity, open terrain (median) | 1.27 | 1.27 (276 routes) |
| sinuosity, hill (median, p90) | 1.54, 2.66 | 1.39, 1.90 (240 routes) |
| sinuosity, mountain (median, p90) | 1.87, 2.79 | 1.63, 2.00 (48 routes) |
| runaway routes (over 2.5) | 50 | 10 |
| crossings | 121 bridges, 252 fords, 5 ferries | 129 bridges, 258 fords, 1 ferry |
| passes | 10 | 6 |
| realms | 3 | 3 |
| border on a river or ridge, where one lies within two cells | 771 ‰ | 931 ‰ |
| mining settlements | 124 (19 %) | 29 (4.5 %), 86 % within 4 km of another |

The hill and mountain figures before are those of the central-place town
pattern before the routing changes; the first version reported only a
relief-based open-ground median.

## Overlay

`render` composites, over the Atlas overview:
- quiet land-use tints (fields, fallow, meadow and built ground);
- a soft tint per realm that deepens into a colour ribbon along its land
  borders, under a dashed border line smoothed off the cell grid;
- roads by class: a red highway and a pale road, each with a dark casing,
  a thin brown track and a dashed footpath;
- symbols: a walled city (a wall ring with eight bastions round a red
  core), a red town disc, village and hamlet dots, a ring round each realm
  seat, crossed picks at mines, `)(` at passes and triangles on the twelve
  highest peaks;
- labels in anti-aliased serif type with paper halos, set most important
  first by a greedy placer that tries eight positions round each symbol
  and never overlaps another label, a symbol or the furniture: realms in
  letter-spaced small capitals with their gloss, cities in capitals and
  towns in roman with their glosses in italic, rivers in italic along their
  courses, peaks with their heights, and villages as room allows;
- a neatline, a title cartouche with a north arrow, a legend and a scale
  bar, in the page corner that covers the least land.

Sizes scale with the overview (1 at 2,048 pixels across). The type is Noto
Serif and Noto Serif Display, embedded from `assets/fonts/` as Latin subsets
with their pair kerning flattened into a `kern` table for `ab_glyph`. Both
are under the SIL Open Font License 1.1 (`assets/fonts/OFL.txt`), which
allows embedding in commercial software.

## Tests

`tests/society.rs` runs the real pipeline on a synthetic 64 × 48 km
landscape (`synthetic.rs`). It checks:
- determinism, byte for byte;
- the refusals;
- 8 km town spacing and the rank-size fit;
- towns as central places: an inland share of 30–85 % and a
  nearest-neighbour index above 1.3;
- 0.8 ha of farmland a head;
- that the network is connected and that every wet road cell is covered by a
  recorded crossing;
- open-terrain sinuosity of 1.2–1.4, hill and mountain medians of 1.2–2.0
  where there are routes, and runaway routes under 5 %;
- the realm partition and closed borders, 900 ‰ or more of snappable
  border on a river or ridge, and no realm fragment cut off from its seat;
- that mining is rare (60 ‰ or less) and clustered;
- that names are unique;
- the `SettlementProfile` fields;
- run time.

`tests/names.rs` checks that names are deterministic, glossed, plain
romanisation and unique across the whole world, and that realms are marks.
`tests/render.rs` draws the overlay twice onto the same page and requires
identical pixels. Unit tests cover the central-place score and spacing, the
ore districts, the switchback behaviour of each road class on a steep
slope, strokes and halos, kerned and tracked text, and the label placer.

## Limits

- Seats are chosen over straight market areas on a coarse lattice, while
  the partition follows the road and terrain cost surface, so a realm's
  final share can differ from its market area's (on seed 7 MICRO the
  smallest realm holds 9.5 % of the land and 13 % of the people).
- With the fixed 8,000 city floor, a MICRO world's capitals carry about
  37–42 % of its people in towns and cities, above the artifact's three
  tenths.
- About 10 routes (under 2 %) still run over 2.5 times their straight
  line, where a fjord or lake lies between a village and its town and no
  direct route exists.
- Mountain routes are more sinuous than 1.4, as the artifact expects; the
  1.2–1.4 check applies to open terrain (median slope under 5°).
- The border share counts a border edge as on a line when a cell beside it
  is within one cell of the line; where a river crosses a border rather
  than running along it, a few edges near the crossing stay off it.
- The overlay's full-canvas masks take about 4 bytes a pixel each, so very
  large renders (over 16,384 pixels) need several gigabytes.
- The building mix is an estimate. Building layout belongs to the
  town-layout stage.
