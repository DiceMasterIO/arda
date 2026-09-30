---
generated_date: 2026-09-30
scenario: settlements-roads-realms
artifact: ../mockup-artifact.md
status: normative design; implementation in progress on feat/settlements-roads (crate arda-settle)
goals: 34, 35, 36, 37, 38, 39, 41 (40 is in 15-naming)
---

# 08 — Settlements, roads and realms

> Normative design for the `arda-settle` crate. It turns the stored physical world into settlements, land use, roads, crossings, passes and realms. The mechanism is the artifact's sections "Where people settle", "Land use around settlements", "Roads" and "How plausibility is checked", which stand verbatim. This file fixes the parameters, the world-scale frame, the stored formats and the contracts that [town layout](10-town-layout.md), the [NPC population](13-npc-population.md), [society and history](14-society-history.md), [naming](15-naming.md) and the [service API](16-service-api.md) read. It extends [06 — Society generation](06-society-generation.md) steps 1–3 and supersedes the artifact's "borders … not produced" non-goal. Merge-order and adapter notes are in the [integration plan](../integration-plan.md).

Code cites rules as `// logic/08 §<rule>`, for example `// logic/08 §settle-refusals`.

## Trigger & preconditions

- Trigger: `arda-settle generate --world <dir>` (or the library call `settle(world, seed)`), after a world batch has completed. It is a post-pass: it never runs inside `arda generate` and never rewrites a world layer.
- Preconditions: a loadable world (`World::load`, format major 4) with every area's `cells.bin` and `objects.bin` readable; the world seed from the manifest. `<world>/society/` must be absent or hold only files this stage wrote with the same `format_version` (they are replaced atomically, see State transitions).
- Resources: the whole-world grid is admitted before any output is written, against the 16 GiB ceiling (goal-prompt §8). Admission counts 128 bytes per 100 m cell (assumed, tunable; the stitched grid plus cost surface and route scratch). A default 500 × 1000 km world has 9 × 19 = 171 areas of 512² cells, 44,826,624 cells, so about 5.7 GB; the admitted figure is computed from the manifest, never estimated. A refusal names the requirement and writes nothing.

## Rules

### §world-frame

The frame every product layer uses at world scale. It is the `arda-server` cell contract frame (`crates/arda-server/API.md`, "Coordinates"), restated here because it is normative for every crate:

- World metres: x east, y south, origin at the modeled domain's north-west corner.
- Global cell `(gx, gy)` is the 100 m lattice **node** at `(100·gx, 100·gy)` m. Its footprint is the 100 m square centred on the node: `[100·gx − 50, 100·gx + 50) × [100·gy − 50, 100·gy + 50)`.
- A settlement, crossing or pass that sits "at a cell" is stored with `cell_x = gx`, `cell_y = gy` and `x_m = 100·gx`, `y_m = 100·gy`. Positions that are not cell-snapped (polyline vertices) are integer metres in the same frame.
- Road, realm-border and river polylines are lists of `[x_m, y_m]` integer pairs. A polyline through cells passes through their nodes; a border between cells runs on footprint edges (`100·g ± 50`).

Source: the generator samples fine terrain at the nodes (`logic/02`), and the server contract states the node convention. The current `arda-settle` code writes `x_m = 100·gx + 50` (a cell-corner convention); that is one of the inconsistencies listed in the integration plan and must be changed to this rule before merge.

### §settle-suitability

Every land cell gets a site score in per-mille (0–1000), from its surroundings (artifact, "Where people settle"):

| Term | Rule | Source |
|---|---|---|
| Water | full within 150 m of a river, lake or sea; linear fade to 0 at 750 m | artifact |
| Arable land | share of arable cells within 1 km; arable = slope ≤ 14°, temperature above the crop limit, moisture index ≥ 0.3, height above river ≥ 2.5 m, cover grass/scrub/meadow or clearable forest | artifact; crop limit and moisture floor assumed, tunable |
| Flat building ground | slope of the cell itself, full at 0°, zero at 14° | artifact (refusal slope) |
| Warmth | mean temperature, full at ≥ 10 °C (assumed, tunable) | artifact names the term only |
| Prominence | height above the surrounding 1 km, full marks at 30 m | artifact |
| Shelter | bay or river mouth rather than open shore or headland tip | artifact |
| Travel node | confluence of two rivers of order ≥ 2, or a ford site | artifact; order threshold assumed, tunable |

Weights: water and arable dominate (water 0.30, arable 0.25, the other five share 0.45; assumed, tunable). Scores carry a deterministic jitter of ±40 ‰ keyed by `(seed, "suit", gx, gy)` so the pattern is not mechanical (artifact "a little randomness"; magnitude assumed, tunable).

### §settle-refusals

A cell is refused outright (score 0, never a settlement centre) when any of these holds (artifact, binding):

1. terrain is sea or lake, or cover is marsh, rock or ice; or cover-derived class is snow, cliff or beach;
2. slope > 14°;
3. too cold to farm (mean temperature below the crop limit);
4. height above the nearest watercourse < 1.5 m.

### §settle-sites

Site tags are bit flags per cell (goal 34), written as lowercase strings on the settlement record: `river`, `lake`, `coast`, `harbour` (sheltered bay with depth), `estuary`, `navigable` (order ≥ 4, artifact), `confluence`, `ford` (stream 2–14 m wide with gentle banks, artifact), `bridge` (a crossing site too wide to ford), `pass` (a saddle between basins on a road profile), `defensible` (prominence ≥ 30 m), `hill`, `forest`, `marsh` (adjacent), `mountain`, `ore` (rock cover plus relief: a documented proxy, no geology layer exists), `timber`, `fish`, `salt`, `spring` (channel head within 300 m, assumed). The tags drive functions, history hooks and names (15-naming §name-site-suffix).

### §settle-tiers

- Population `P = density × land area`, density from the world config (default 15 people/km², `logic/01`).
- 30 % live in towns and cities, 47 % in villages, the rest in hamlets (artifact: "three tenths", "just under half"; 47 % assumed within that phrase).
- Town sizes follow rank-size: the town of rank r has `P_urban / (r · H_n)` people, with as many towns as keep the smallest ≥ `TOWN_MIN` (artifact rule; `H_n` is the harmonic number that makes the list sum to `P_urban`).
- Tier bounds (canonical for every crate):

| Tier | Population | Source |
|---|---|---|
| hamlet | 12–99 | artifact "a dozen to eighty"; the upper bound is widened to 99 so tiers are contiguous (assumed) |
| village | 100–999 | artifact "a few hundred"; bounds assumed |
| town | 1,000–7,999 | assumed, tunable (`TOWN_MIN`) |
| city | ≥ 8,000 | assumed, tunable (`CITY_MIN`) |

- At most two cities per realm (goal 35). A third would-be city in a realm is demoted to a town with population `CITY_MIN − 1`; the surplus goes to that realm's villages proportionally. This runs after §realm-seats.

### §settle-placement

Greedy, best site first, within each tier, in the order towns (by rank), villages, hamlets (artifact):

| Spacing | Value | Source |
|---|---|---|
| town to town | ≥ 8,000 m | artifact |
| village to town | ≥ 1,500 m | artifact |
| village to village | ≥ 2,000 m | artifact |
| hamlet to anything | ≥ 900 m | artifact "just under a kilometre" |

- Towns weigh `harbour` and `navigable` sites far above villages do (×3 for towns vs ×1; assumed, tunable).
- A better site gets a larger settlement within its tier (rank order follows score order).
- Distances are exact integer Euclidean distances between nodes.
- `SettlementId` is 1-based in placement order. Ties in score break by the lower `(gy, gx)`.

### §settle-functions

Each settlement gets a sorted, non-empty set of functions from its tags, tier and surroundings (goal 36). The closed set, shared with `arda-npc` `SettlementFunction`, serialised `snake_case`: `farming`, `pastoral`, `fishing`, `port`, `market`, `mining`, `logging`, `crafting`, `fortress`, `abbey`, `crossing`, `capital`. Rules: `farming` where arable share ≥ 15 %; `pastoral` where arable share < 35 % (both may hold); `port` for non-hamlets with `harbour`/`estuary`, or towns on `navigable`; `market` for towns, cities and villages ≥ 450; `mining` with `ore` within 3 km (not cities); `logging` with `timber`; `crafting` for towns and cities; `fortress` for `defensible` towns or any urban `pass`; `abbey` for about 1 in 14 villages and towns without a harbour, keyed by `(seed, "abbey", id)`; `crossing` with `ford`, `bridge` or `confluence`; `capital` added by §realm-seats. If nothing applies, `pastoral`. Thresholds other than the artifact's are assumed, tunable.

Wealth 0–255: base by tier (45, 85, 135, 180), plus arable share, plus function bonuses (port 25, market 10, mining 15, crafting 10, abbey 5), minus 10 on poor land, plus a ±10 jitter; capital +20. All assumed, tunable.

### §settle-building-mix

Each record carries `buildings`: an estimated count per canonical **society building key** (the `arda-npc` `BuildingFunction::key` strings: `house`, `farmhouse`, `cottage`, `inn`, `tavern`, `smithy`, `temple`, `shrine`, `mill`, `market_hall`, `stall`, `warehouse`, `dock`, `boathouse`, `keep`, `barracks`, `guardhouse`, `manor`, `workshop`, `mine`, `lumber_camp`, `stable`, `bakery`, `brewery`, `tannery`, `apothecary`, `library`, `school`). The mix is an estimate for the overview and for town planning; the realised building list is [10 — Town layout](10-town-layout.md)'s and may differ. Invariant: the dwelling estimate houses the population (see Invariants).

### §landuse

Fields at 0.8 ha per person (artifact), taken nearest arable cell first, open ground before woodland; villages and hamlets add pasture on ground too steep or rough to plough; towns keep none (artifact). Built-up footprint from population at urban and village densities (artifact; densities assumed, tunable: 100 people per built cell in towns, 40 in villages, 15 in hamlets). Precedence: built > fields > pasture > prior cover (artifact). Mills sit on a river cell (order ≥ 2) within 1 km of a farming village or town; mines on the nearest `ore` cell of a mining settlement (goal 39). Orchards take 10 % of fields on sun-facing slopes of 2–8° (assumed, tunable). Woodland is kept on slopes > 14° inside the field radius (assumed).

Land-use codes (u8, canonical; `fields` and `town` read them):

| Code | Key | Meaning |
|---|---|---|
| 0 | none | untouched; prior cover stands |
| 1 | built | built-up footprint |
| 2 | field | ploughed fields |
| 3 | pasture | pasture |
| 4 | orchard | orchard (counts as farmland) |
| 5 | woodland | managed woodland kept on slopes |
| 6 | mill | a water mill site |
| 7 | mine | a mine or quarry head |

The raster keeps these eight codes. Finer classes used at 5-ft scale by feat/tactical-fields (`meadow`, `fallow`, `quarry`, `farmstead`, and `wild` for code 0) are derived inside the fields layer from these codes plus local terrain and a hash; they are never written to `landuse.bin`. The mapping is: 0 → wild; 1 → built (farmstead for a hamlet's built cells); 2 → arable, or fallow for one field in three by `H(seed, "fallow", gx, gy)`, or meadow on field cells with wetness ≥ 0.6; 3 → pasture; 4 → orchard; 5 → woodland; 6 → mill; 7 → mine, or quarry on rock cover (thresholds assumed, tunable).

### §roads

Least-effort routes over a cost surface (artifact, "Roads"):

| Ground | Relative cost | Source |
|---|---|---|
| flat grassland | 1 (baseline) | artifact |
| fields, scrub, open woodland | "a little more": 1.2 | artifact; value assumed, tunable |
| dense forest, alpine | "more again": 2 | artifact; value assumed |
| marsh | "several times": 5 | artifact; value assumed |
| bare rock, snow | "many times": 12 | artifact; value assumed |
| cliff | "almost prohibitive": 200 | artifact; value assumed |
| slope | multiplier `1 + (g/0.3)²` below a 30 % grade, `4·(g/0.3)⁴` above | artifact shape; exponents assumed |
| watercourse crossing | fording 10 m ≈ 1 km of detour, 50 m ≈ 4 km, linear between and beyond | artifact |
| lake, sea | impassable, except a ferry edge (below) | artifact; goal 04 |
| existing road | 30 % of the cost of its ground | artifact rule ("cheap"); value assumed, tunable |
| ferry crossing of open water | 25 % per cell plus a 20 km landing charge | assumed, tunable |

Build order, top down (artifact): trunk (highway) = minimum spanning set of least-cost town-to-town routes; trunks pushed from the nearest town to each map edge where land leaves the map; each village to its nearest town (road); each hamlet to its nearest village (track); footpaths between neighbouring villages within 4 km not already joined (assumed). Routes are 8-connected A* over integer costs with ties broken by `(gy, gx)`. Only the cells a route adds become a new road object; a route falling onto an existing road reuses it (no parallel duplicates).

Road classes (canonical for every crate):

| Class | Society raster code | Cell `RoadClass` | Tactical surface width | Verge each side | Max sustained grade | Bridge deck | Source |
|---|---|---|---|---|---|---|---|
| none | 0 | `None` (0) | — | — | — | — | |
| footpath | 1 | `Track` (1) | 1 square | 0 | 25 % | 2 squares | goal 37; widths from feat/tactical-ways `ClassSpec`, assumed, tunable |
| track | 2 | `Track` (1) | 2 squares | 0.5 | 14 % | 2 | same |
| road | 3 | `Road` (2) | 3 squares, 4 where settlement wealth ≥ 160 | 1 | 10 % | 3 | same |
| highway | 4 | `Highway` (3) | 5 squares, with side ditches | 1 | 8 % | 4 | same |

Every crate reads widths from this table; feat/tactical-fields' own half-widths (2.0, 1.5, 1.05, 0.55 squares) must be replaced by it. Road surface ground keys: highway `cobbles`, road `gravel`, track and footpath `dirt` (from feat/tactical-fields; assumed). Above the maximum sustained grade the tactical road switchbacks (goal 37).

The stored world `Cell::road` has no footpath value and is never rewritten by this stage (world layers are immutable). Consumers read the road class from `society/`, never from `Cell::road`. The contract mapping in the table is for display only.

### §crossings

Every place a road meets a watercourse, lake or sea is a crossing object (artifact). Classification (artifact rule, widths assumed, tunable):

| Width | Near a settlement (≤ 1 km) or on a highway | Elsewhere |
|---|---|---|
| < 14 m (small) | bridge | ford |
| 14–50 m (middling) | bridge | ford if ≤ 20 m, else ferry |
| > 50 m (wide) | bridge only on a highway within 1 km of a town | ferry |
| lake or sea | ferry | ferry |

A crossing's position is the node of the first water cell the route enters. Its `order` and `width_m` are that cell's.

### §passes

Where a route climbs over a ridge, the high point of its profile is a pass object with its height and the rise from the lowest point on each side, when both rises are ≥ 100 m (assumed, tunable).

### §realm-seats

Realms follow [06 — Society generation](06-society-generation.md) steps 1–3:

1. Seat count `N = clamp(P / 8,000, 2, towns / 3)` when there are ≥ 2 towns, else 1 (logic/06 for `P/8,000` and the minimum 2; the `towns/3` cap is assumed, tunable). Seats are the N largest towns or cities; each gains `capital`.
2. Allegiance: every settlement swears to the seat cheapest to reach over the road-and-terrain cost surface; ties break by the seat's rank (logic/06).
3. Territory: every land cell joins the realm of its cheapest seat (a multi-source least-cost flood over the same surface, ties by seat rank). Borders snap to a river (order ≥ 3) or a ridge line when one lies within 2 cells (logic/06), by reassigning the cells between the raw border and the feature.
4. Realm id is 1-based in seat rank order. The realm raster stores `u16` per cell (0 on water), so a world may hold at most 65,535 realms.

## Steps

1. Admit resources (Trigger). Stitch every area into one grid by reading areas uncached, one at a time.
2. Suitability and refusals (§settle-suitability, §settle-refusals); site tags (§settle-sites).
3. Culture regions: a 6.4 km lattice (64 cells) where each lattice node reads its 3 × 3 lattice neighbourhood (about 19 km) and takes one of the canonical culture keys `heartland`, `highland`, `sylvan`, `coastal`, `southern`, `borderland` (the `arda-npc` `cultures.json` keys; lattice size assumed, tunable). A settlement's culture is its node's.
4. Tiers and placement (§settle-tiers, §settle-placement), then functions, wealth and building mix (§settle-functions, §settle-building-mix).
5. Land use (§landuse).
6. Roads, crossings and passes (§roads, §crossings, §passes).
7. Realms (§realm-seats), then the city cap (§settle-tiers) and capital refresh.
8. Names: every settlement, river, peak, pass, region and realm is named through [15 — Naming](15-naming.md). Until `arda-names` merges, the crate's local generator stands in and must use the same keys.
9. Write `society/` (Outcomes). Render the overlay on request (goal 41; Outcomes).

## Branches

- Fewer than 2 towns: one realm covering all land; its seat is the largest settlement (logic/06).
- A world with no refusal-free land: zero settlements, empty roads, one realm with seat `0`; the files are still written (an empty world is valid output).
- A settlement no route can reach (an island without a ferry edge): recorded in `roads.json` `unreachable`, and it fails the connectivity invariant only when it is on the mainland component.

## Unhappy paths

- Resource admission fails: typed refusal before `society/` is touched.
- A world layer is unreadable: the stored-layer error propagates with its file path; nothing is written.
- An existing `society/` from another `format_version`: refused with both versions named; the user deletes it or passes `--replace`.
- Interrupt: the staging directory is abandoned; the previous `society/` (if any) is intact.

## State transitions

`absent society/` → `society.tmp/` staged → atomic rename to `society/`. The world's own layers (`world.json`, `areas/`, `terrain/`, `continent/`) are never written. `--replace` renames the old directory aside before the swap and deletes it only after the swap succeeds.

## Invariants

Each is a test (goal numbers in brackets):

1. Determinism: the same world gives byte-identical `society/` files, independent of thread count [goal-prompt §8].
2. Refusals: no settlement centre on sea, lake, marsh, rock, ice, snow, cliff or beach, on slope > 14°, or less than 1.5 m above a watercourse [34].
3. Spacing: all pairwise distances meet §settle-placement [34].
4. Rank-size: for towns and cities, the least-squares slope of log(population) against log(rank) is in [−1.2, −0.8] when there are ≥ 5 towns [35].
5. At most two cities per realm [35].
6. Tier bounds: every settlement's population lies in its tier's range [35].
7. Carrying capacity: world-wide field area per inhabitant is 0.8 ha ± 10 %, and farmland (fields + orchards + pasture) per inhabitant is within [0.8, 1.5] ha [39; artifact "close to a hectare"].
8. Housing: for every settlement, `capacity_estimate(buildings) ≥ population` with the per-key capacities of [10 — Town layout](10-town-layout.md) §town-capacity [55].
9. Roads: every mainland settlement is connected to every other through the network; no road cell is lake or sea unless it is part of a ferry crossing; every crossing lies on a watercourse, lake or sea cell; the median sinuosity (route length / straight distance) of village-to-town roads on ground with median slope < 5° is in [1.2, 1.4] [37; artifact].
10. No parallel duplicates: no two road objects share a cell except at their junction cell [37].
11. Realms: every land cell has exactly one realm id; each realm's cells form one 8-connected component or a component per island; there are ≥ 2 realms when there are ≥ 2 towns; each realm's seat lies inside it [38; logic/06].
12. Ids: settlement ids are 1..n without gaps; road, crossing, pass and realm ids likewise.
13. Frame: every `x_m, y_m` of a cell-snapped object equals `100·cell_x, 100·cell_y` (§world-frame).
14. Budget: MICRO seed 42 runs in under 10 s in release; the full default world stays within 16 GiB (measured once) [goal-prompt §8].

## Outcomes & side effects

`<world>/society/`, each JSON file with `"format_version": 1` and the world `seed` as a decimal string:

| File | Content | Consumers |
|---|---|---|
| `settlements.json` | `{format_version, seed, width_cells, height_cells, settlements: [Settlement]}` | town, npc, society, names, server |
| `roads.json` | `{format_version, roads: [Road], crossings: [Crossing], passes: [Pass], unreachable: [id]}` | ways, server |
| `realms.json` | `{format_version, realms: [Realm]}` | society, server |
| `features.json` | named rivers, peaks and regions (`{id, name, kind, anchor_m, …}`) | names, server |
| `landuse.bin` | magic `ARDALND\0`, u32 version, u32 width, u32 height, then zstd(level 9) of `width·height` u8 codes followed by `width·height` u32 LE owner settlement ids (0 = none) | fields, town, server |
| `roads.bin` | magic `ARDARDS\0`, same header, zstd of u8 society raster codes (§roads) | ways, refine, server |
| `realms.bin` | magic `ARDARLM\0`, same header, zstd of u16 LE realm ids | society, server |
| `overlay.png` (on request) | Atlas overview with tier symbols, road classes, realm borders and labels | maintainer, goal 41 |

**Settlement record** (the `arda-npc` `SettlementProfile` fields first; extra fields are ignored by `SettlementProfile`, which does not deny unknown fields):

| Field | Type | Meaning |
|---|---|---|
| `id` | u64 | `SettlementId`, §settle-placement |
| `name` | string | 15-naming |
| `tier` | `hamlet`/`village`/`town`/`city` | §settle-tiers |
| `population` | u32 | within the tier range |
| `functions` | sorted list | §settle-functions |
| `wealth` | u8 | §settle-functions |
| `culture` | culture key | Steps 3 |
| `realm_id` | u32 | §realm-seats |
| `biome` | string | informational; one of `alpine`, `highland`, `wetland`, `steppe`, `boreal_forest`, `temperate_forest`, `coastal`, `warm_temperate`, `temperate` (snake_case; the catalogue `biome:` tags map from it in [11](11-tactical-art-compositor.md) §dress) |
| `coastal`, `riverine` | bool | tags `coast`; `river` or `navigable` |
| `cell_x`, `cell_y` | u32 | centre global cell |
| `x_m`, `y_m` | i64 | `100·cell_x`, `100·cell_y` |
| `height_m` | i32 | stored cell height, rounded |
| `rank` | u32 | rank among towns and cities, 0 otherwise |
| `site_tags` | list | §settle-sites |
| `history` | string | one-line origin hook; replaced by the richer record of [14](14-society-history.md) |
| `buildings` | map key → u32 | §settle-building-mix |

The CellSample fields `road` and `built_by` are not updated by this stage; the server fills them from `roads.bin` and `landuse.bin` ([16 — Service API](16-service-api.md) §api-cell-society).

## Dimensions not in play

- Time: a single present-day snapshot; history is narrative (14-society-history), not a simulation of growth.
- Money and trade flows: wealth is a scalar, not an economy.
- Authority: no users or permissions; a local CLI stage.
- Legacy worlds: format-3 worlds are refused by `World::load`; no society is generated for them.
