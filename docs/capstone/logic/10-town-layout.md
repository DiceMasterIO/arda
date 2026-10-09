---
generated_date: 2026-09-30
scenario: town-layout
artifact: ../mockup-artifact.md
status: normative design; implementation on feat/town-layouts (crate arda-town)
goals: 36, 44, 45, 55, 56
---

# 10 — Town layout: streets, plots, buildings and interiors

> Normative design for turning one settlement record ([08](08-settlements-roads-realms.md)) into a town plan: streets, plazas, walls and gates, plots, buildings with typed footprints, and interiors with rooms and doors. The plan is the **single source** of both the building objects that the [NPC population](13-npc-population.md) lives and works in and the building squares of every tactical block that the plan touches ([09](09-tactical-refinement.md) §reservations). This is the building-layout function that [03 — Block generation](03-block-generation.md) (amendment "Building layout") and [06 — Society generation](06-society-generation.md) step 4 call for, so objects and drawings cannot disagree (goal 45).

Code cites rules as `// logic/10 §<rule>`.

## Trigger & preconditions

- Trigger: `town_plan(world, society, SettlementId)`, called on demand by block refinement (09 §reservations), by the NPC adapter (§town-building-spec) and by the service (16-service-api). The plan is derived, not stored: storage stays O(settlements) (goal 56).
- Preconditions: `society/settlements.json`, `landuse.bin` and `roads.json` exist (08); the world's cells, rivers and fine terrain are readable; the settlement id exists.

## Rules

### §town-frame

A plan is computed once per settlement in **global square coordinates** (09 §square-frame) over the bounding box of its built cells plus a 2-cell margin. A block never re-plans a town: it clips the plan (§town-clip). Plan geometry is integer squares; walls lie on square edges, as in `TacticalLayout`.

### §town-footprint

The built footprint is the set of cells with land-use code `built` owned by the settlement (08 §landuse). The artifact's "centre cell" is the settlement's `cell_x, cell_y`; it receives the temple (or shrine), the inn and the market (artifact, "Inside a cell"). Squares whose `E` gives a square-scale slope above 20° or that are water, cliff or road-ford squares are unbuildable (slope assumed, tunable).

### §town-streets

1. Every road of `roads.json` that crosses the footprint becomes a street, keeping its class band width (08 §roads): the highest class through the centre cell is the **main street**.
2. Market settlements get a plaza of `flagstone` (towns, cities) or `packed_earth` (villages) at the centre cell: 12 × 12 squares for villages, 20 × 20 for towns, 28 × 28 for cities (assumed, tunable).
3. Lanes (2 squares, `dirt` or `cobbles` by wealth ≥ 128) branch off streets at deterministic intervals of 14–20 squares keyed by `(seed, "lane", settlement, street, index)`, run perpendicular to the street on ground with slope < 8°, and follow the contour otherwise (switchback rule of 08 §roads at plan scale). Lanes stop at the footprint edge, at water, or at another street.
4. Hamlets and villages without a market get no plaza and no lanes: farmsteads string along the track (a street village) or cluster around a green of 10 × 10 `grass` squares when three or more tracks meet (assumed).

### §town-rivers

A plan is drawn on the refined rivers, the same centreline pieces and water squares arda-refine draws (09; `arda_refine::water::cell_channels`, `RiverWater`), not on the settlement stage's straight centre-to-centre channels: a square is river water in the plan exactly when the tactical block shows river water there.

1. A main street follows its road, but where its carriageway would come within one square of a river it is moved onto the nearer bank; where the road passes from one bank to the other the street keeps to the first bank up to one point and goes straight across, square to the flow, where the widest channel of that stretch is narrowest (a confluence is never the narrowest). No main street runs along or in a river.
2. Every stretch where a street's centreline runs over water, between land on both sides, is a bridge (`TownPlan.bridges`): rows of deck squares, along x or along y, whichever deck is smaller; each row covers the whole crossing and is carried on to its own banks (or to a wall standing in the river). Main streets bridge up to 64 squares; lanes cross only where the gap between dry points is at most 18 m and they cross at more than 45° to the flow (a footbridge of at most 10 squares). A street that would end in water or cross a longer stretch is cut at the banks.
3. Plots and buildings never stand on river water (docks and jetties are decks over it by design); plots with water within four squares form the waterfront district, where mills, warehouses and tanneries prefer to stand (§town-functions).

### §town-plots

- Plots front a street or lane. Frontage and depth in squares (assumed, tunable; medieval burgage plots are narrow and deep):

| Tier | Frontage | Depth | Back yard |
|---|---|---|---|
| hamlet | 10–16 | 12–18 | farmyard, kitchen garden |
| village | 6–10 | 10–16 | garden, yard |
| town | 4–6 | 10–14 | yard |
| city | 3–5 | 8–12 | yard or none |

- Plots never overlap water, cliffs, streets or another plot, and never cross the footprint edge. Plot order: by distance of the plot centroid from the centre node, then global row-major index. `BuildingId` is 1-based in plot order, then in function-assignment order within a plot (a farmhouse plot holds the farmhouse, then its barn). Ids are stable for a given world and society.

### §town-crofts

After zoning, the open ground the plots enclose becomes crofts, so a settlement is continuous from its core to its rim and the countryside begins at its edge (arda-town `plan::croft`). The built-up footprint is a morphological closing of the plots, buildings, squares, greens, churchyards, baileys and walls: every open square within a noisy reach `R₁` of them (±4 squares of value noise), eroded by `R₂`, so gaps up to `2·R₁` fill and the outline runs `R₁ − R₂` squares beyond the outermost plots.

| Tier | `R₁` | `R₂` |
|---|---|---|
| hamlet | 12 | 7 |
| village | 20 | 14 |
| town, city | 18 | 13 |

Crofts are cut into oblong parcels (bands of 12 rows, runs of 17 columns at a per-band offset) used as paddock (30 %), orchard (22 %), kitchen garden (20 %), small meadow (18 %) or work yard (10 %). Hedges divide parcels of different use; wattle fences close each plot's back onto the crofts and, in hamlets and villages, part neighbouring gardens; hedges close plot backs onto open country; 7 % of fence edges stay open as gaps. The footprint's outer one to three squares are rough grass and scrub with bushes and young birch. Crofts are soft claims (09 §reservations): they yield to water and roads. Crofts never move a building, street or plot, so building ids are unchanged.

### §town-functions

Buildings are assigned to plots from the settlement's building mix (08 §settle-building-mix), in this order, each taking the nearest free plot that satisfies its siting rule (goal 36):

| Function | Siting | Source |
|---|---|---|
| temple or shrine, inn, market_hall | centre cell, fronting the plaza or main street | artifact |
| keep | highest defensible plot within the footprint (prominence), or the edge nearest a `pass` | goal 36; assumed |
| dock, warehouse, boathouse | fronting water (river order ≥ 3, lake or sea) | goal 44; assumed |
| mill | on a river square edge, at the land-use `mill` cell if inside the plan | 08 §landuse |
| smithy, stable | on the main street near the entry road | assumed |
| tannery | the downstream edge of the footprint | historical practice; assumed |
| mine, lumber_camp | at the land-use `mine` cell or the nearest forest edge, outside the footprint | 08 §landuse |
| everything else | remaining plots, most central first for higher wealth | assumed |
| dwellings | all remaining plots, then fringe plots until §town-capacity holds | goal 55 |

### §town-footprints

Every building is a rectangle or an L of squares inside its plot, set back 0–1 square from the frontage (assumed). Town and city row houses (house, cottage, workshop, bakery, apothecary, tavern, brewery) fill their frontage, so a street front is a terrace of party walls, except for an **ambitus**: a deterministic share of street-front row plots (a quarter in towns, `Params::ambitus = 0.25`; about one in seven in cities' denser cores, `0.15`; none on the market square, none on plots under four squares wide or with a side passage) keeps its last frontage column free as a one-square eaves-drip gap to the yard. The draw is a hash of the plot's first front square, not the plan's random stream; the front line and depth are measured over the whole frontage, so the gap only takes that column away and no other building moves. Villages and hamlets keep their side gaps instead (assumed, tunable). Typed base sizes in squares (w × h, before rotation to face the street; assumed, tunable; ±1 square jitter keyed by building id):

| Key | Size | Key | Size | Key | Size |
|---|---|---|---|---|---|
| house | 4 × 6 | cottage | 3 × 4 | farmhouse | 5 × 8 (+ barn 4 × 6) |
| manor | 8 × 12 | inn | 8 × 10 | tavern | 6 × 8 |
| smithy | 5 × 6 (+ yard) | workshop | 4 × 6 | tannery | 5 × 8 |
| bakery | 4 × 6 | brewery | 6 × 8 | apothecary | 4 × 5 |
| temple | 8 × 14 | shrine | 3 × 4 | library | 6 × 8 |
| school | 5 × 7 | market_hall | 8 × 12 | stall | 2 × 2 |
| warehouse | 6 × 10 | dock | 4 × 12 deck over water | boathouse | 4 × 7 at the water edge |
| keep | 12 × 12 (+ bailey) | barracks | 8 × 12 | guardhouse | 4 × 5 |
| stable | 5 × 8 | mill | 5 × 7 | mine | 3 × 3 adit |
| lumber_camp | 5 × 6 + log yard | | | | |

### §town-walls

- Building wall kits (canonical `WallSegment.kit` keys) come from the settlement's data by §building-materials: `stone`, `drystone` (rubble stone), `timber`, `wattle`, `log` and `adobe`. Field walls `drystone` and `hedge` belong to fields (09 §reservations); `palisade` encloses a fortress village or a town without a wall; `city_wall` encloses cities and fortress towns, following the footprint hull at a 2-square offset, with a `gate` edge piece where every street crosses it (all assumed, tunable).
- Every building's outline is a closed loop of wall segments. Doors: at least one `door` segment on the side facing its street or plaza; windows: a `window` on every exterior run of ≥ 4 squares in towns (assumed).

### §party-walls

A terrace must read as a row of houses, not one building with many rooms (`arda_town::block`):

- An edge on a building's footprint boundary is its **shell**. When a second building's shell claims the same edge (a terraced neighbour), the edge is a **party wall**, tagged `party`; the stronger piece still wins the edge (doors over runs over windows, then the lower id). An edge strictly inside one footprint is an interior **partition**, tagged `partition` (11 §walls-assembly draws it thin). Curtain walls, fences and hedges carry no tag.
- Rules are unchanged: party walls and partitions block movement and sight like any wall (12 scene walls read the edge, not the tag).
- Neighbours sharing a wall take different floors where their palettes allow (§building-materials), so the boundary also shows in the floor.

### §building-materials

Materials come from the settlement, never per house at random (`arda_town::block::kits`, a data table; deterministic). The plan carries a `Fabric`: the record's wealth, `merchant` (market or port), biome, `forest` (forest biomes, `forest` or `timber` site tags, logging) and `stone` (highland or alpine biome, `hill`, `mountain` or `quarry` tags, mining); and, read from `society.json` after planning by `arda-people` (layout-neutral), `fires` (great-fire events naming the settlement) and `golden_age` (wealth ≥ 160 in at least half of four or more prosperity samples).

1. **Vernacular** `(poor, middling)` by biome and culture tradition (heartland and coastal build timber, highland and southern stone, sylvan and borderland wattle):

   | Biome / tradition | poor | middling |
   |---|---|---|
   | steppe (any) | adobe | adobe |
   | boreal forest (any) | log | log |
   | highland, alpine (any) | rubble | stone |
   | warm temperate, stone tradition | adobe | stone |
   | other, stone tradition | rubble | stone |
   | wattle tradition with woodland at hand | wattle | log |
   | other timber or wattle tradition | wattle | timber |

2. **Always stone:** temple, keep, library, school, guardhouse, barracks, market hall, manor. **Farm buildings** (barn, stable, boathouse) take the vernacular: poor at low standing, else middling.
3. **A rich merchant town or city** (town or city, merchant, wealth ≥ 220) is stone throughout: ashlar `stone`, rubble `drystone` for houses of low standing (below); neighbours still differ by finish and floor.
4. **Otherwise rank by district:** castle 4; market and religious 3; waterfront, craft and farmstead 2; residential 1; suburb 0. Add the settlement: wealth < 100 −1, 170–229 +1, ≥ 230 +2; city +1, hamlet −1; building stone at hand +1. Add history: a golden age +1 in the core (castle, market, religious); any great fire +1 inside the wall outside suburbs and farmsteads (rebuilt in stone under the town's ordinances). Add the house's **standing** within its own settlement: +1 when its wealth is at least 45 above the settlement's (manors, the best market houses), −1 when at least 40 below (cottages, suburb houses, poor barns), else 0. Standing is relative, so a rich town's ordinary houses do not count as wealthy twice.
5. **Rank to material:** ≥ 4 stone (rubble for a house of low standing); 3 mixed, stone or the middling vernacular by a hash of the building id (half each); 2 middling; ≤ 1 poor.
6. **Partitions:** adobe in adobe houses, wattle in wattle houses and in houses of the poor wealth band, timber otherwise.
7. **Floors:** working and civic buildings keep a fixed floor (temple, shrine, keep, library, market hall `flagstone`; warehouse, barracks, guardhouse, mill, bakery `stone_floor`; smithy, stable, barn, tannery, brewery `packed_earth`). Homes take a palette by material and wealth (wealthy stone `flagstone`, `planks`, `stone_floor`; modest stone `stone_floor`, `planks`, `flagstone`; poor stone `stone_floor`, `packed_earth`; wealthy timber or log `planks`, `flagstone`; modest timber or log `planks`, `packed_earth`; wealthy adobe `flagstone`, `stone_floor`; other adobe `packed_earth`, `stone_floor`; other poor `packed_earth`, `planks`). In building-id order each home takes the first floor of its palette, starting one step down for a hashed quarter of homes, that no lower-id neighbour sharing a wall already has. Room floors (`rug` accents, kitchens) still override per room.

On MICRO seed 42: the city Eayeyil (wealth 254, port and market) is stone throughout; the heartland town Dilrou (183) has a stone market and temple close, mixed stone and timber craft and waterfront streets, timber residential streets and wattle suburbs; the sylvan town Vefleth (196) builds logs where Dilrou builds timber; the southern town Fetarmedhe (two great fires) builds stone; the poor sylvan forest village Tudeeyude (90) builds wattle with log houses for the better off.

### §town-interiors

- v1 depth is the **ground floor only** (the goal-prompt open question "interior depth" is decided here for v1 as ground floor; upper storeys are Dimensions not in play).
- Every non-trivial building (not stall, dock, mine or barn) has an interior: interior walls on square edges from a per-function room template (for example inn: common room ≥ 50 % of the floor, kitchen, store, private room; smithy: forge room open to the yard; temple: nave and sanctum; house: hall and one or two rooms). Room templates are data, not code.
- Floor ground keys follow §building-materials (`planks`, `stone_floor`, `flagstone`, `packed_earth`), with `rug` accents in wealthy rooms (vocabulary.md keys).
- Each room has a typed `room` tag (`common`, `kitchen`, `store`, `bedroom`, `forge`, `nave`, `sanctum`, `office`, `cell`, …) for dressing (11 §dress).
- Furniture and props are placed by dressing (11 §dress), not by the plan.

### §town-wfc

The town fabric inside the plan is filled by WFC at 5-ft scale (goal 44; arda-town `block::wfc`, solver crate `arda-wfc`). The plan fixes footprints, shells, exterior doors, streets, plots and building ids; the WFC never moves them. `TownFill::Wfc` is the default; `TownFill::Rules` keeps the rule programmes, which are also the interiors' relaxed fill.

1. **Rooms (pass A).** A tile is a position class (room corner, edge or middle, 9 classes); pairwise rules make every room a rectangle of at least 2 × 2 squares that meets its neighbours along partitions. North-west corners are capped at the programme's room slots, and its extra rooms are anchored as corners on the west or north shell, behind the entry zone when that zone keeps to the front. Zones are then assigned by a deterministic search for the best total fit (front and back zones by depth, larger zones in larger rooms) that keeps required zones present, room sizes and shortest sides, the entry zone's floor share, the front door in an entry zone, and a doorway tree from the entrance over the programme's door pairs.
2. **Doorways and furniture (pass B).** Typed tile sets per function (`programmes/`): each piece has needs on its sides (wall behind, open floor in front, table for seats, partition for doorways, plain ground for graves and arrow loops), and edge kinds (open, partition, shell, exterior door) meet them. Doorways are anchored first, one per tree edge; each room's spine (the middle line along its long axis; in a nave, a central aisle from the front wall to the chancel) and the paths from its entrances stay open floor; required pieces are anchored (fewest options first) where the room is large enough; caps and a density limit (35 % of the floor by default, 60 % in a nave) bound the rest. The check: every walkable square reachable from the exterior doors, and every piece touching a reachable square.
   - **Ordered furniture** (`block::wfc::rows`) is laid out by rule before pass B and the WFC fills around it: pews in rows across the nave either side of the clear central aisle with a row of legroom between rows (a nave that must also seat an altar keeps its back rows free); dormitory beds and reading-room bookshelves along both long walls, short of the corners; warehouse racks and market-hall tables in free-standing rows with a walk before each row and passages kept open at both ends (rows across the room preferred, turned rows only when none fit). Placed squares are fixed in the pass-B domains, the piece leaves the rest of the room, the anchors that asked for it count the placed pieces and the caps and density limit make room for them.
   - **Households** (`programmes/household`): a home draws its partition (open hall, back chambers, side chamber, front workroom, cross passage), household trade, beds, fire (an open fire pit in some poor homes) and clutter pools by wealth from its own stream keyed by the interior salt, the same choices the rule programmes make (goal 64), and the programme turns them into zones, required pieces and caps. A trade works in its own back room except in an open hall; houses under 20 squares stay one room and under 24 squares take no trade; the front-workroom partition becomes a hall with the workroom behind it; clutter draws under low-weight `clutter.*` pieces capped at two. The WFC problem keys carry the salt. Seed-42 city: 1,665 distinct WFC home interiors of 1,673, no adjacent pair alike.
3. **Wall kit pieces.** Shell edges take a run or a window by a 1-D WFC per side: doors and gates fixed, corners, party walls and walls with furniture against them stay runs, windows never touch each other or a door. The curtain wall's faces take arrow loops (`window` pieces of `city_wall`) and gate passages braziers, inside the outdoor problem.
4. **Streets, squares and yards.** Per global chunk of 32 × 32 squares: classes from the plan (carriageway kept clear; kerb and drains only along streets at least 3 squares wide, paved kerbs as `flagstone`; the square's paved edge; market pitches beside stalls; fronts, yards, gardens, churchyards, greens, croft uses, the rim, wall and gate). Only tiles with no need on a side may stand on a chunk seam, so chunks join legally without seeing each other and blocks stay exact at every seam (09 Invariant 2). The market well and braziers, bridges, docks, reeds and open-country trees stay plan-fixed (`exterior::fixed`).
5. **Retries and relaxed fill (goal 47).** At most 8 attempts per problem, subseeded by `(seed, key, attempt)`, each with up to 24 local repairs (cells within 2 squares reset). Then interiors fall back to the rule programme and chunks to plain ground; both are listed in `TownBlock.relaxed`, and arda-blocks marks their squares for review, so a block's or window's `meta.relaxed` and `meta.review_squares` count them (logic/16). Relaxed-fill rate on MICRO seed 42 (every plan): 45 of 47,247 problems (0.10 %) at v3-town-wfc; 43 of 44,819 (0.10 %) after the v0.3 integration (realm primacy changes the plans).

Vocabulary: 306 tiles in all: 9 room classes, 224 indoor tiles (44 pieces in every allowed orientation and square; each function's set is 18 to 105 of them), 3 shell pieces and 70 outdoor tiles (26 pieces). One vocabulary addition, `prop.drain` (a street drain grate). No library draws it yet, so it is **art-free** (`outdoor_vocab::ART_FREE`): the WFC still places it along paved kerbs, but its prop stays out of town layouts, and validation, fallbacks and renders stay clean; remove it from `ART_FREE` once libraries carry it. The v0.3 integration replaces the warehouse `goods` query piece with a free-standing `rack` (drawn as `prop.cask_rack`) and adds the household `clutter.*` pieces, so the counts above are v3-town-wfc's.

### §town-capacity

Resident capacity and workplace slots per society building key (canonical; values from the `arda-npc` sample generator, which this table replaces; assumed, tunable):

| Key | Capacity | Slots | Key | Capacity | Slots |
|---|---|---|---|---|---|
| house | 6 | 0 | farmhouse | 7 | 0 |
| cottage | 4 | 0 | inn | 8 | 5 |
| manor | 8 | 5 | tavern | 4 | 4 |
| smithy | 5 | 3 | temple | 4 | 4 |
| shrine | 3 | 3 | mill | 5 | 2 |
| market_hall | 0 | 4 | stall | 0 | 2 |
| warehouse | 0 | 5 | dock | 0 | 5 |
| boathouse | 3 | 3 | keep | 14 | 7 |
| barracks | 20 | 12 | guardhouse | 4 | 4 |
| workshop | 4 | 3 | mine | 6 | 8 |
| lumber_camp | 6 | 5 | stable | 3 | 3 |
| bakery | 4 | 2 | brewery | 4 | 3 |
| tannery | 4 | 2 | apothecary | 4 | 2 |
| library | 3 | 4 | school | 4 | 2 |

Town capacity (a ground-floor-only plan in a town or city may count dwellings as two storeys, capacity × 1.5, rounded down; assumed) must reach `population × 1.08` (the `arda-npc` sample slack). If the planned plots fall short, fringe plots are added at the footprint edge, then on adjacent `field` cells, until it holds; population is never trimmed.

### §town-building-spec

The adapter from a plan building to `arda-npc` `BuildingSpec` (13-npc-population):

| `BuildingSpec` field | From |
|---|---|
| `id` | `BuildingId(plan building id)` |
| `settlement_id` | `SettlementId(settlement id)` |
| `function` | society key → `BuildingFunction`; `workshop` → `Workshop(craft)` with the craft drawn by `(seed, "craft", settlement, building)` from the 12 SRD-neutral crafts, weighted by culture (assumed) |
| `capacity`, `workplace_slots` | §town-capacity (with the storey factor) |
| `wealth` | settlement wealth ± 40, higher toward the centre and the plaza (assumed) |

Barns, yards, plazas, streets and walls are plan features, not `BuildingSpec`s.

### §town-function-keys

One mapping table for the three vocabularies (goal 45). Society keys (the `arda-npc` keys) are canonical in data; tactical `function` tags are for dressing and catalogue queries:

| Society key | `BuildingFunction` | Tactical tag (`function:`) |
|---|---|---|
| house, farmhouse, cottage, manor, inn, tavern, bakery, brewery, mill, smithy, tannery, apothecary, temple, shrine, library, warehouse, stall, dock, boathouse, keep, barracks, guardhouse, stable | same name | same name |
| workshop | `Workshop(craft)` | `workshop` |
| market_hall | `MarketHall` | `market` |
| mine | `Mine` | `mine` (to be added to vocabulary.md) |
| lumber_camp | `LumberCamp` | `lumber_camp` (to be added) |
| school | `School` | `school` (to be added) |
| — (barn) | — | `barn` |
| — (street, plaza) | — | `street` |
| — (fields, farmyard) | — | `farm` |

### §town-clip

`town_reservation(plan, gx, gy)` returns the squares of block `(gx, gy)` that the plan owns, with their ground keys, wall segments (on the block's edges they are split so a segment on the shared edge appears in exactly one block: the block whose square lies south or east of it, matching the `TacticalLayout` north/west-edge convention), placements whose anchor lies in the block, and lights. A building that straddles block edges appears partly in each block and whole in a window.

## Steps

1. Load the settlement, its built cells, the roads through its footprint and the local `E` field (09 §elevation).
2. Streets, plaza and lanes (§town-streets).
3. Plots (§town-plots), functions (§town-functions), footprints (§town-footprints).
4. Capacity check and fringe growth (§town-capacity).
5. Walls, doors, windows and gates (§town-walls); interiors (§town-interiors).
6. Return the `TownPlan`.

## Branches

- Hamlets: a street village or green (§town-streets 4); farmhouses with barns and yards.
- Cities and fortress towns: `city_wall` with gates.
- A settlement whose footprint has no buildable square (steep site): the footprint grows onto the nearest buildable cells within 1 km; if none, the plan is empty and `plan.flags.no_site` is set; the NPC adapter then refuses the settlement (13 Unhappy paths).

## Unhappy paths

- Unknown settlement id: `NotFound`.
- Missing `society/` files: `SocietyMissing`, naming the file.
- Capacity cannot be reached within 1 km of fringe growth: `TownError::Capacity { needed, planned }`; never a silent trim.

## State transitions

None. The plan is a pure function cached by the service (16-service-api §api-cache).

## Invariants

1. Determinism: the same world, society and settlement give a byte-identical `TownPlan` [45].
2. Capacity: `Σ capacity ≥ population` for every settlement of MICRO seed 42 [55].
3. Every building lies on buildable squares; no two buildings, and no building and street, overlap [44].
4. Every building outline is closed; every building has ≥ 1 exterior door on its street-facing side, and every room is reachable from an exterior door through doors [44].
5. Every building is inside exactly one plan; `BuildingId`s are 1..n per settlement without gaps [45].
6. The function mix realised equals the settlement's estimate for every non-dwelling key, except where a siting rule has no site (a dock without water), which is recorded in `plan.unsited` [36].
7. Clip consistency: the union of `town_reservation` over all blocks of the plan equals the plan, and each wall segment appears in exactly one block [42, 45].
8. No building footprint but a dock's covers river water; every street square over river water is a bridge deck; every deck row reaches a bank (or a wall in the river) at both ends (`arda_town::plan::check::water`, tested on the synthetic sites and on MICRO seed 42's riverine plans).
9. Every NPC's `home_building` and `workplace_building` is a building of the plan (tested through 13-npc-population) [45, 55].
10. Party walls and partitions: an edge tagged `partition` has one building on both sides and lies inside its footprint; an edge tagged `party` lies on the footprints of two different buildings; no other edge carries either tag (`arda-town/tests/blocks.rs`, `arda-blocks/tests/settlements.rs` against the scene's building ids).
11. Materials: a rich merchant town or city is stone throughout; other towns split by district; history only upgrades towards stone; materials never change the layout (`block::kits` tests, `arda-blocks/tests/settlements.rs`).
12. Ambitus: in towns and cities 10–40 % of street-front row houses leave a one-square gap and full frontages stay the majority; the gap is plot ground with no building in it (`arda-town/tests/plan.rs`).

## Outcomes & side effects

`TownPlan` (serde, format 1): `{format_version, settlement_id, origin_gs, size_squares, streets: [{class, squares-polyline}], plazas, walls: [WallSegment in global squares], buildings: [TownBuilding], unsited: [key], flags}`. `TownBuilding`: `{id, key, footprint: [[gsx, gsy] ring], rotation, wall_kit, doors: [WallSegment], rooms: [{tag, squares}], floor, wealth, capacity, workplace_slots}`. No files are written.

## Dimensions not in play

- Upper storeys, cellars and roofs (a later roof layer may hide interiors when viewed from outside).
- Growth over time and ruins (14-society-history carries narrative history only).
- Land ownership and rents.
