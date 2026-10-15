---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: a48fef3bb6dc
paths_covered:
  - ":(top)crates/arda-midzoom/**"
  - ":(top)crates/arda-server/src/relief.rs"
  - ":(top)crates/arda-server/src/routes.rs"
  - ":(top)crates/arda-server/src/lib.rs"
  - ":(top)crates/arda-server/tests/world_api/relief.rs"
  - ":(top)tests/zoom_land.rs"
  - ":(top)tests/zoom_continuity.rs"
  - ":(top)tests/zoom_terrain.rs"
  - ":(top)crates/arda-server/src/overview.rs"
  - ":(top)crates/arda-settle/src/render/tilelabels.rs"
---

# 17 — Mid-zoom relief: on-demand ~10 m refinement between the world map and tactical maps

> The world stores one height every 39.0625 m. Zoomed past the overview's native scale, each sample spans many pixels and the map is smooth by construction. Storing a 4× finer level everywhere breaks the 16 GiB budget, so, like the tactical layer (09), detail is grown on demand for the tiles on screen only, from the stored fine terrain and area data (`crates/arda-midzoom/src/lib.rs:1-21`). The layer reads only public APIs (`World::fine_terrain`, `World::read_area`, `arda::area_atlas_terrain`), so it composes with any world that stores recipe-5 fine terrain. The rules were written as a normative design and merged with the crate; this file restates them against the code at commit 022ce7cb5707, each rule with its source sites. It owns the route `GET /v1/tiles/relief/{z}/{x}/{y}.webp` (`crates/arda-server/src/routes.rs:39`, `:409-427`) and the `arda-midzoom` binary (`crates/arda-midzoom/src/bin/arda-midzoom.rs`).

Code cites rules as `// logic/17 §<rule>`.

## Trigger & preconditions

| Entry point | Inputs | Site |
|---|---|---|
| `GET /v1/tiles/relief/{z}/{x}/{y}.webp[?labels=0\|1]` | `z`, `x`, `y` as `u32`; the last segment must end in `.webp`; `labels` is `0` (names off) or `1`/absent (names on) | `crates/arda-server/src/routes.rs:409-427`, `labels_flag` `:355-366` |
| `arda-midzoom --world <dir> --z <z> --center x,y --out <png> [--size 512] [--base-px 4096] [--plain] [--tiles k]` | crop centre in world metres; side in pixels; overview base; `--plain` ignores `society/`; `--tiles k` times a k × k block of 256 px tiles instead | `crates/arda-midzoom/src/bin/arda-midzoom.rs:10-39` |
| `arda_midzoom::render_tile`, `render_window`, `refine_tile`, `refine_nodes(_for)` | library calls | `crates/arda-midzoom/src/tile.rs:41`, `:66`, `crates/arda-midzoom/src/lib.rs:60-88`, `crates/arda-midzoom/src/refine.rs:40-75` |

- Authority: none; the route shares the server's unauthenticated loopback binding (16 §api-conventions); the binary is a local process.
- Preconditions: a loadable world whose manifest carries fine terrain with recipe ≥ 5. The server enables relief only then (`crates/arda-server/src/relief.rs:82-91`); `ReliefWorld::new` refuses a world without fine terrain with `MidzoomError::NoFineTerrain` (`crates/arda-midzoom/src/world.rs:49-58`, `crates/arda-midzoom/src/source.rs:88-92`). Worked land (§land) additionally needs `<world>/society/settlements.json` (`crates/arda-midzoom/src/land/mod.rs:77-86`); the server builds the landscape from the same society overlays as its tactical blocks (`crates/arda-server/src/lib.rs:233-243`).
- The pyramid's native zoom is the overview's `max_zoom` (`crates/arda-server/src/lib.rs:247-253`); the binary derives it as `log2(base_px / 256)` (`crates/arda-midzoom/src/bin/arda-midzoom.rs:45-49`).

## Rules

### §frame

- Refined node `(i, j)` of subdivision `n ∈ {1, 2, 4, 8}` sits at fine-lattice micrometres `(i·s, j·s)`, `s = 39,062,500 / n` (n = 4: 9.765625 m). The fine lattice is the world frame minus `FINE_FRAME_OFFSET_UM` (node 0 is the centre of cell 0, vocabulary I1).
- Every refined height is a fixed integer stencil of global stored nodes and saved cells. A window never changes a node's value; tiles therefore join pixel-exactly.

Sites: `crates/arda-midzoom/src/source.rs:14`, `crates/arda-midzoom/src/refine.rs:57-68`, `crates/arda-midzoom/src/tile.rs:73-86`.

### §base
The stored nodes `H` are sharpened into near-interpolating cubic B-spline coefficients by three rounds of `C ← C + (H − S·C)` (`S` = [1 4 1]/6 per axis, the spline's node filter), then clamped to the 3 × 3 stored range so the surface neither blurs crests nor rings into new pits. The base `B` is the cubic B-spline of `C` evaluated exactly (weights scaled by 6n³).

Sites: `crates/arda-midzoom/src/base.rs:17`, `crates/arda-midzoom/src/base.rs:153-181`, `crates/arda-midzoom/src/base.rs:183-232`.

### §detail
Drainage-aligned detail: per octave (72, 44, 30 m; domains rotated 37°, −23°, 61°), feature points jittered anywhere in their cells carry stripe kernels `w·a·cos(2π·(p − f)·d)` where `d` is the unit vector across the local fall line, `w = (1 − r²)²` within one cell and `a ∈ [0.4, 1.6]` a per-feature depth. Crests and troughs therefore run downslope. Each octave reads the gradient left by the coarser ones, so finer gullies turn down the walls of coarser ones and join them (branching, converging drainage). The cross-section is `2√((1+v)/2) − 1` (narrow V gullies, broad spurs) on soil and its mirror (sharp ribs, broad chutes) on steep rock (slope ≥ 31°–48°). Octaves need ≥ 3 lattice steps per wave and fade below four screen pixels.

Sites: `crates/arda-midzoom/src/detail.rs:26-62`, `crates/arda-midzoom/src/detail.rs:87`, `crates/arda-midzoom/src/detail.rs:143-154`, `crates/arda-midzoom/src/detail.rs:155`.

### §amplitude
Coarsest-octave amplitude = 2 × stored roughness (mean |H − mean of 8 neighbours| over 3 × 3, the field's own short-wave energy and the maturity proxy) × steepness (7°–26°) × relief (40–200 m over 156 m) × cover (rock 1, scrub 0.9, grass 0.8, forest 0.75, marsh 0.2) × slope position (gullies head below divides, deepen downslope, die out on valley floors) × patchiness (rotated noise at 260 m, 0.3–1.7) × (1 − watercourse) × (1 − standing water). Wall slopes are capped at 0.8 × the fall-line slope, so detail never reverses drainage. Low, gentle ground stays gentle.

Sites: `crates/arda-midzoom/src/refine.rs:132-191`, `crates/arda-midzoom/src/masks.rs:27-35`, `crates/arda-midzoom/src/detail.rs:62`.

### §mantle
Gentle, non-flat ground (slope ≥ 3°) gets turf hummocks: isotropic rotated noise at 56 and 32 m, ≤ 0.8 m, never steeper than half the fall-line slope.

Sites: `crates/arda-midzoom/src/detail.rs:211`, `crates/arda-midzoom/src/refine.rs:26`.

### §means
Two rounds restore every stored node's 39 m cell mean (trapezoid over its refined nodes): residuals are sharpened like §base and added back as a spline.

Sites: `crates/arda-midzoom/src/refine.rs:23`, `crates/arda-midzoom/src/refine.rs:192`.

### §pits
Two passes lift every land node lower than its eight neighbours to 1 mm above the lowest.

Sites: `crates/arda-midzoom/src/refine.rs:24`, `crates/arda-midzoom/src/refine.rs:258`.

### §render
Relief tile pixels are shaded by the formed Atlas shader (logic/04 §atlas-formed) through `AtlasTerrain::relief_surface_colour`: the finest light, height and a 20 m concavity (weight 2) come from the refined surface (Catmull-Rom, C1); broader light, sky, materials and climate come from the stored field and saved cells exactly as in the overview, including the stored shore classes (`shore.bin`, logic/04 §atlas-formed shore) on land and in the shallows. On recipe-9 worlds the shader's canopy and snow use the shared tree and snow lines (`arda_core::biome::Lines`) at each row's latitude, and its sunny side is the one facing the equator (`AtlasTerrain::with_frame`); older recipes keep their pinned look, which took the north-facing side for the sunny one. Which surface a pixel shows (land, lake or sea) comes from §water, not from the stored field; lake and sea depth tints read the stored field wherever it holds water there, and §water's depth at the margins only §water calls wet. Below 9 m/px land gets a ±5 % ground-cover tone at 24 and 11 m.

Sites: `crates/arda-midzoom/src/tile.rs:186`, `crates/arda-midzoom/src/tile.rs:235-243`.

### §water
One water geometry serves every zoom (goal 49): relief tiles draw rivers, lakes, coasts and marsh pools from `arda_refine::region::WaterRegion`, which gathers the same global cells, channel edges and fine lattice nodes a refined block reads (`Ctx::gather_region`, in the I1 frame of logic/09 §square-frame) and answers with the blocks' own rules (logic/09 §linear-features: `standing_water` for shores, the centreline pieces for channels, pools). At a square's centre the region's water is the block's water, so at z9 (one pixel per square on MICRO) relief and tactical water agree square for square. A pixel is sampled at its centre in global square units (world µm / 1,562,500); pools show at ≤ 3.2 m/px.

Sites: `crates/arda-midzoom/src/water.rs:16-18`, `crates/arda-midzoom/src/water.rs:36-145`, `crates/arda-refine/src/region.rs:69`, `crates/arda-refine/src/region.rs:91`.

### §rivers
Channels are the tactical layer's centreline pieces (logic/09 §linear-features: crossings on the line between the cells' formed thalwegs on recipe-9 worlds, hashed elsewhere; the cell's thalweg or low node; diagonal steps through their via cell; cubic Hermite with a bow), drawn over land with coverage `clamp(0.5 − d / pixel, 0, 1)`, where `d` is the signed distance to the drawn banks, so coverage passes one half exactly at the banks. Drawn half-width is `max(half · gain, 0.6 px)` with `gain = clamp(pixel / 6.25 m, 1, 4)`: the overview's ×4 symbol at 25 m/px, the true banks at ≤ 6.25 m/px. Colour is the formed overview's discharge band of the piece's saved edge. Channels are not drawn over lakes or the sea, which the blocks mark as standing water.

Sites: `crates/arda-midzoom/src/water.rs:25-26`, `crates/arda-midzoom/src/water.rs:84-86`, `crates/arda-midzoom/src/water.rs:127`.

### §low-relief
On low ground the stored drainage relief is made readable without adding any (`lowrelief.rs`). From the stored 39 m nodes around the window: a neighbourhood mean (three box passes of radius 5 nodes, ≈ Gaussian σ 214 m) and the local relief (mean absolute deviation from it, two box passes of radius 8). Both terms fade in below 10 m of local relief and are full below 3 m:
- **position**: `t = (h − mean) / max(relief, 0.7 m)`, clamped to ±1.2, tints valley floors cooler and greener (−4 % red, −3.5 % blue per unit) and interfluves warmer and paler (+3 %, +2.5 %, +1 %);
- **slope light**: the refined surface's Lambert term (light from the north-west at 42°, logic/04) is added again with gain `clamp(6 m / max(relief, 2.4 m), 1, 2.5) − 1`, so terrace risers and bluffs read as lines while flat treads stay flat.
Integer arithmetic over global nodes with a margin wider than the kernels: tiles join exactly. Only worlds with society data take it (§land), so others stay byte-identical.

Sites: `crates/arda-midzoom/src/lowrelief.rs:24-32`, `crates/arda-midzoom/src/lowrelief.rs:83-173`.

### §land
Relief tiles of a world with `society/` (`arda settle`, `arda society build`) draw worked land from the geometry the tactical overlays compose (arda-blocks `SocietyOverlays`, shared with the server's blocks), so parcels, roads and buildings stay in place across the relief → tactical hand-off as water does (§water). Order per land pixel: shader colour → §low-relief → land use and canopy → channels (§rivers) → roads → settlements. Detail by pixel size: > 45 m the 100 m land-use raster as a bilinear tone; ≤ 45 m parcels, hedgerows and roads; ≤ 20 m settlement plans; ≤ 6 m strips, furrows, orchard rows, crowns, roof pitches and shadows. A world without `society/` renders byte-identically to before.

The worked land is the `Works` struct (grid, parcels, coarse tone, roads, settlement plans), gathered by `Works::gather(land, grid, bbox)`; `LandWindow` holds `Works` plus the low-relief and forest fields that only relief tiles use. Area PNG exports reuse `Works` without the relief beneath (logic/04 §overlay): `AreaLandOverlay` (a `BandOverlay`) cuts each group of exported rows into chunks about 3,200 m wide, gathers `Works` per chunk in parallel and paints land-use tone, parcels (tone, walls, orchard trees), then roads and settlements, with the same pixel-size thresholds and geometry, no low-relief drainage or canopy; row groups hold at most 4 Mi pixels (16 to 1,024 rows). A pixel depends only on its world position, so a settlement across an area border paints the same pixels from both areas. `arda_midzoom::export::{export_overview, export_area}` choose the layer by `SocietyMode` (`Off`, `Auto` when `society/settlements.json` exists, `On` errors without it) and refuse the oblique look with an overlay.

Sites: `crates/arda-midzoom/src/land/mod.rs:12-13`, `:43`, `:77-86`, `Works` `:153-161`, `Works::gather` `:251-328`, `Works::paint` `:330-343`, `LandWindow::gather` `:356-410`, `LandWindow::ground` `:421-486`, `crates/arda-midzoom/src/land/export.rs:1-15` (constants `:26-30`), `AreaLandOverlay` `:33-105`, `Works::pixel` `:107-143`, `BandOverlay::paint` `:145-186`, `crates/arda-midzoom/src/export.rs:15-115`, `crates/arda-server/src/lib.rs:233-243`.

### §land-fields
Parcels are the arda-fields partition itself (`arda_fields::partition`), built once per window from the same inputs the tactical fields layer reads: the land-use raster (crofts as arable), heights and river pieces of the surroundings, the road segments as the ways lay them (`arda_ways::curve::centreline` of settle's segments, review 3.5-10; both layers read `SocietyOverlays::field_roads`) and the settlements within `INPUT_REACH_M` (3.2 km) of the window, and the fields layer's culture and wealth (`FIELDS_CULTURE`, `FIELDS_WEALTH` of arda-blocks). The partition is a fixed global hierarchy:

- **Land blocks.** A brick lattice of 500 m tiles paired into blocks two tiles wide, alternate rows offset by a tile (every block corner a T-junction); vertices displaced by a smooth 2 km noise field (±0.6 tile, so the brickwork turns and stretches from place to place) and jittered ±0.17 tile by their own hash. Everything is drawn in a warped frame: rotated value noise at 150 and 60 squares, up to 4 squares in old enclosure and 1.4 in planned.
- **Subdivision.** Each block is split recursively until a piece is the size its ground asks for. A road (track and above; its quadratic B-spline, as arda-ways draws it) crossing a piece still to be split nearly straight is the cut, so fields line the roads. Otherwise the cut runs across the piece (or lengthwise, for long narrow closes near the houses) square to its longest straight side, so new hedges meet old ones near right angles; on slopes above 0.035 it follows the contour where a side allows. Old enclosure jitters cut positions (0.3–0.7) and angles (±10°, a piece over 20 ha takes its own grain ±34°) and kinks 60% of cuts by 4–17°; planned enclosure (a 4 km noise field, culture-biased) cuts near the middle along one surveyed bearing per 6 km. Cuts that would leave a wedge (a child filling less than 0.64 of its principal-axis box) are retried gentler. No cut but a road may meet the piece's boundary (its clipped outline, the block outline and the true lines of the cuts above, a road's end rays included) at under 38°, nor run within 12 squares of a boundary edge at under 38° (a sliver whose tip another edge blunts); the last resorts, square to the frontage and then along it, obey the same rule, and a piece no cut can split stays one field, so a would-be wedge stays part of its neighbour. The partition tests hold every field corner in five scenarios to 35° or more, away from roads and rivers; on MICRO 42 the rule cut corners under 35° from 98 to 4 in six 4 km squares. Where broad cover (farmed, wood, waste) changes within a piece, a straight cut that separates it clearly better wins, and mixed pieces split down to 1.2 ha.
- **Sizes.** Enclosed ploughland 0.4 ha by the settlement's built edge rising smoothly to 3.4 ha 1.4 km out (pasture ×1.25); furlongs 6 ha within a village's open-field disc; meadow 1.4, orchard 0.7, farmstead closes 0.45, woods 7, waste 9 ha; × wealth (0.8–1.2), culture (dwarf 0.75, elf 0.85, halfling 0.6) and planned enclosure (up to ×1.25); each piece's target × 0.4–2.2 (log-uniform). No field is longer than 180 squares or smaller than 640.
- **Use.** A field's class is the majority of its land-use samples, read through a smooth displacement (up to 70 m) so farmland outlines wander with the field edges; `fields::kind_of` then decides (strips inside open-field discs), except that farmed fields within 70 m of a river bank on slopes below 0.05 are floodplain meadow and pasture on slopes above 0.14 (or, 30%, beyond 900 m from a settlement) is open common.

A point's parcel is its leaf (`Partition::locate`). Since every part of a cut outside its own piece lies outside every leaf beneath it, the distance to a leaf's edge is the least distance to the cuts above it and the block outline (`locate_edge`), and the parcel across is the leaf just over that nearest point: tones blend across the edge box-filtered, and a hedge (2.6 m, swelling and gapping with rotated noise at 16 m) or a drystone wall (1 m, on fields steeper than 0.12, logic/09's mixed-region kit) is drawn on edges where either parcel is enclosed. A pixel at or below 2 squares decides at its square's centre, as the tactical plan does, coarser pixels at their own centre. Tones are multiplicative tints against the shader's lowland grass (§land-tones), with the per-parcel jitter on enclosed fields and furlongs only (woods and commons run on without seams); strength is 1 at ≤ 3 m/px, 0.7 at 8, 0.55 at 15 and 0.42 from 30 m out (the coarse raster tone uses 0.42 too). Strips (`strips::locate`, per-strip crop, one-square balks at ≤ 3.2 m/px, a 4.5 m grass headland round each furlong), furrows (2.6 m, ±7 %, faded before aliasing) and orchard rows (trees every 5 × 4 squares along the parcel) appear once pixels resolve them. Unworked valley floors (§low-relief position < 0) take a floodplain-meadow tint.

Sites: `crates/arda-midzoom/src/land/fields.rs:87-206`, `crates/arda-midzoom/src/land/fields.rs:142`, `crates/arda-midzoom/src/land/fields.rs:167-168`.

### §land-roads
Road centrelines are `arda_ways::plan::build` over the window (B-spline pieces, switchbacks, junction snapping) with every 0.5 m station; a pixel's coverage is the box-filtered share of the class band (highway 5, road 4, track 2, footpath 1 squares, logic/08 §roads), so at one pixel per square it covers exactly the squares whose centres lie within half the width. Coarser pixels keep a minimum drawn half-width (0.70, 0.55, 0.38, 0.30 px from highway to footpath) with opacity `(true / drawn)^0.6`; footpaths stop above 8 m/px, tracks above 24, roads above 64. Squares a town's streets replace (arda-blocks `Streets::replace`: inside a town's reach, where the plan lays public street ground, 09 §reservations) are skipped. Crossings are not re-planned: where the tactical layer straightens a bridge approach the two may differ by a few metres.

Sites: `crates/arda-midzoom/src/land/roads.rs:40`, `crates/arda-midzoom/src/land/roads.rs:94-206`.

### §land-towns
Settlements whose plan window (`plan_radius_for`, which grows with a city's people, OI-50) reaches the window are drawn from their `arda-town` plans square by square (`grid.kind`, `grid.building`), plans claiming squares in id order as in arda-blocks; a pixel averages up to 4 × 4 squares. A dock's squares, water in the plan grid, are claimed and painted as a planked deck, as the tactical town layer composes them (`Towns::claim`, `land::town::dock_tests`). Roofs: thatch, red or brown tile, slate or shingle by tier, wealth and a per-building hash, ±6 % weathering; at ≤ 4 m/px pitches lit from the north-west, a darker ridge along the longer side and ground shadows two-thirds of the building's height toward the south-east. Streets (cobbles where paved), squares, walls, yards, gardens, greens, churchyards and crofts (which give way to roads) have their own paint.

Sites: `crates/arda-midzoom/src/land/town.rs:60-318`.

### §land-tones
Grassland reference (111, 119, 62); targets: ploughed (120, 108, 74), stubble (136, 132, 86), fallow (120, 124, 74), hay (116, 132, 66), mown (134, 142, 80), grazed (104, 126, 62), orchard (94, 114, 58), woodland (72, 92, 48), floodplain (98, 126, 66); per-parcel jitter ±4 % brightness, ±2 % warmth; within-field soil noise at 55 m, ±3.5 %. Canopy (woodland parcels; forest cover read bilinearly between cell centres): clumps at 40 and 18 m (±10 %) fading out by 30 m/px, then crowns on a jittered 6.5 m lattice lit from the north-west below 6 m/px.

Sites: `crates/arda-midzoom/src/land/tone.rs:55-172`.

### §labels
Type and symbol scale (`crates/arda-settle/src/render/scale.rs`): a level's names are laid out at `level_unit(z, base_zoom)`: 1.6 at the finest overview level (4), 1.3 one level finer, the page size 1.0 from two levels finer (so relief tiles at close zoom are unchanged), and +0.2 per coarser level up to 2.2; halos and the settlement symbols' clearance scale with it, and the overview society overlay draws symbols at `symbol_scale(width)` (logic/04 §overlay), so a name sits clear of its ring at every level. `Labeler::level_for_width` lays out a whole `/v1/overview.png?labels=1` image from the level nearest its size.

Relief and overview tiles carry the gazetteer's names (logic/15 §name-what-is-named), drawn after the relief and its worked land (`crates/arda-midzoom/src/labels.rs`, `crates/arda-settle/src/render/tilelabels.rs`). A level's layout is made once, for the whole level, from the gazetteer, the settlements and the zoom, then clipped to each tile, so labels never differ between neighbours and a name crossing a tile edge is cut, not re-placed; glyph coverage depends only on global pixels, so tiles join exactly. Items show from their `min_zoom` (features: rank minus 1; city 1, town 2, village 4, hamlet 6). Placement is greedy, rank first: settlement symbols are reserved, a settlement name takes the first free of eight positions beside its symbol, an area name is letter-spaced to fill about 70% of its path (curved text) or of its extent (centred on the anchor, nudged up or down), and a name that does not fit is left out. Water names are pale italic on a deep-blue halo, ranges small capitals, forests green italic, other names dark on a paper halo; type sizes are constant in pixels (12.5 to 27). Tiles take `?labels=0` to leave them off (default on; cache keys include the flag, `TileKey = (z, x, y, labelled)`); a tile is labelled only when the flag is on and the server enabled labels (`crates/arda-server/src/relief.rs:32-33`, `:143-144`); worlds without `society/gazetteer.json` and the oblique pyramid carry none.

Sites: `crates/arda-settle/src/render/tilelabels.rs`, `crates/arda-midzoom/src/labels.rs`, `crates/arda-server/src/relief.rs:116-131` (`enable_labels`, once per server), `:143-152` (draw per tile), `crates/arda-server/src/lib.rs:257-263`, `crates/arda-server/src/overview.rs`.

### §pyramid
Relief levels continue the overview pyramid: level `z ∈ (max_zoom, max_zoom + 8]` has `max(areas) × 51.2 km / (256 · 2^z)` metres per pixel; pixel `p` covers world `[p·m, (p+1)·m)`. Subdivision: n = 4 at ≤ 16 m/px, n = 2 at ≤ 32 m/px, else 1. Pixels outside the world are transparent. The viewer switches to these tiles past native zoom and offers the tactical map (logic/09) at ≤ 2 m/px.

Sites: `crates/arda-midzoom/src/pyramid.rs:10-13`, `crates/arda-midzoom/src/pyramid.rs:41-83`, `crates/arda-midzoom/src/tile.rs:102`.

## Steps

1. Route: parse `?labels` (`labels_flag`), strip `.webp`, parse `z`, `x`, `y`, run the rest on the blocking pool (`crates/arda-server/src/routes.rs:416-426`).
2. Refuse when the world has no relief (`crates/arda-server/src/relief.rs:139-141`) or the address is outside the relief levels or grid (§pyramid; `crates/arda-midzoom/src/pyramid.rs:77-83`).
3. Answer from the byte-bounded tile cache keyed `(z, x, y, labelled)` (128 MiB default) when present (`crates/arda-server/src/relief.rs:20-26`, `:143-147`).
4. Otherwise `render_tile` → `render_window` for the 256 px window (`crates/arda-midzoom/src/tile.rs:41-59`): pick `n` (§pyramid), refine the nodes with a margin, fading waves shorter than four pixels (§base → §detail/§amplitude → §mantle → §means → §pits; `crates/arda-midzoom/src/tile.rs:73-86`), gather area contexts, water (§water) and, with society, land (§land); shade each pixel (§render), skipping pixels outside the world (`crates/arda-midzoom/src/tile.rs:87-102`); when labelled, draw the gazetteer's names over the render (`crates/arda-midzoom/src/labels.rs:53-59`, `crates/arda-server/src/relief.rs:150-152`).
5. Encode lossless WebP, insert into the cache (`crates/arda-server/src/relief.rs:153-158`), answer with `ETag` (BLAKE3 of the bytes), `Cache-Control: no-cache` and `304` on a matching `If-None-Match` (`crates/arda-server/src/routes.rs:161`, `:169-195`).
6. Binary: load the world, attach the landscape unless `--plain`, render the crop twice (cold, warm) and refuse a mismatch as "non-deterministic render", write an RGBA PNG and print timings; with `--tiles k`, render one warm-up tile then k × k tiles and print mean, median and max (`crates/arda-midzoom/src/bin/arda-midzoom.rs:41-132`).

## Branches

- World without recipe-5 fine terrain: the server serves no relief levels and reports `relief_max_zoom` equal to the overview's `max_zoom` (`crates/arda-server/src/relief.rs:82-91`, `:106-114`, `crates/arda-server/src/dto.rs:54-56`).
- World without `society/`: terrain and water only, byte-identical to a plain render (§land, §low-relief).
- Subdivision by pixel size (§pyramid).
- Pixel size thresholds within §render, §water, §rivers, §land, §land-roads and §land-tones decide which detail is drawn.

## Unhappy paths

| Failure | Response | Site |
|---|---|---|
| `labels` other than `0` or `1` | `400 bad_request` "labels must be 0 or 1" (checked before the path) | `crates/arda-server/src/routes.rs:355-366`, `:416` |
| last segment without `.webp` | `404 not_found` | `crates/arda-server/src/routes.rs:418-420` |
| non-integer `z`/`x`/`y` | `400 bad_request` | `crates/arda-server/src/routes.rs:421-425` |
| world has no formed fine terrain | `404 not_found` | `crates/arda-server/src/relief.rs:139-141` |
| `z` not a relief level, or `x`/`y` ≥ 2^z | `404 not_found` (`OutOfPyramid`) | `crates/arda-midzoom/src/pyramid.rs:77-83`, `crates/arda-server/src/relief.rs:52-58` |
| allocation refused | `500 resource_limit` | `crates/arda-server/src/relief.rs:55` |
| read, shading, water or society failure | `500 internal` "relief: …" | `crates/arda-server/src/relief.rs:56`, `crates/arda-midzoom/src/error.rs:6-48` |
| refinement window empty or above 2,048 nodes a side, or `n ∉ {1, 2, 4, 8}` | `MidzoomError::Window` | `crates/arda-midzoom/src/refine.rs:22`, `:63-68` |
| world or fine layer unloadable at server start | server refuses to start | `crates/arda-server/src/relief.rs:69-98` |
| binary: malformed `--center`, unloadable world, unreadable society, write error | error, nonzero exit | `crates/arda-midzoom/src/bin/arda-midzoom.rs:41-86` |

- Concurrency: two misses for the same tile both render; the cache lock is released during rendering and the second insert replaces the first with identical bytes (`crates/arda-server/src/relief.rs:145-158`). No single-flight.
- Retry: none. A failed render is not cached; the next request renders again.

## State transitions

None persisted: tiles are derived on demand and held only in the in-memory LRU, evicted by byte budget (`crates/arda-server/src/relief.rs:39`, `:95`). `ReliefWorld` caches up to 12 area Atlas contexts (`crates/arda-midzoom/src/world.rs:14`, `:101`) and `WorldTerrain` up to 512 fine chunks and 16 areas (`crates/arda-midzoom/src/source.rs:51-53`). The world on disk is never written.

## Invariants

Checked by tests:

1. Determinism and seams: refinement is deterministic and windows join exactly (`crates/arda-midzoom/tests/checks.rs:65`); adjacent tiles join pixel-exactly on a stored world (`crates/arda-midzoom/tests/world.rs:79`); parcels join exactly across windows (`crates/arda-midzoom/tests/land.rs:92`); the server serves identical bytes twice (`crates/arda-server/tests/world_api/relief.rs:15`).
2. 39 m means kept: mean error ≤ 0.3 m, worst ≤ 2.5 m on a synthetic massif (`crates/arda-midzoom/tests/checks.rs:95`).
3. No pits deeper than 0.5 m (`crates/arda-midzoom/tests/checks.rs:171`).
4. Drainage: ≥ 90 % of refined channels draining ≥ 0.3 km² lie within 60 m of stored D8 channels (`crates/arda-midzoom/tests/checks.rs:237`).
5. Isotropy: detail energy within 10 % over twelve azimuths (`crates/arda-midzoom/tests/checks.rs:272`); gentle low ground stays smooth (`:309`).
6. Pyramid: tiles outside the relief levels are refused; off-world pixels are transparent and land is opaque (`crates/arda-midzoom/tests/world.rs:118`, `:135`); saved rivers draw blue-teal (`:146`).
7. Budget: a warm 256 px tile at ~10 m renders in < 150 ms (`crates/arda-midzoom/tests/world.rs:167`); a tile of nodes refines within budget (`crates/arda-midzoom/tests/checks.rs:336`).
8. Land agreement with the tactical layer (release gate, `#[ignore]`, needs a settled MICRO seed 42 world): relief z9 against composed tactical blocks on villages, town centres, roads and farmland; road IoU ≥ 0.85, building IoU ≥ 0.95, walls between two fields on a relief hedge line ≥ 0.95; land tiles pixel-exact across windows at z7, z8 and z10, and plain worlds unchanged (`tests/zoom_land.rs:180-218`, a module of the `zoom_continuity` target (`tests/zoom_land.rs:1-5`)). Synthetic: every tactical field is one relief parcel and every wall between fields lies on a parcel edge; road squares IoU ≥ 0.97 against `arda_ways::apply_ways` (`crates/arda-midzoom/tests/land.rs:29`, `:134`, `:186`).
9. Water agreement (release gate): relief z9 and tactical water squares on MICRO 42 rivers, an inlet and coasts, IoU ≥ 0.9 per cell, and on an arid world (`tests/zoom_continuity.rs:163`, `:231`); `region_water_matches_the_refined_blocks` on the synthetic world, lakes included (`crates/arda-refine/src/region.rs:244`).
10. Terrain agreement (release gate, review 3.6-10): over 48 natural cells of MICRO 42 spread from lowland to 45° slopes, relief z9 ground height against the composed tactical elevation square by square: median |dz| ≤ 1.5 m (recipe 9: 0.66), p90 ≤ 3 m (2.59), median bias ≤ 0.8 m (+0.12), steep cells (above 30°) within 3.5 m (+2.65 at worst: the tactical map cuts ledges into steep slopes); and world forest against tactical woods, agreeing on ≥ 80 % of forest, grass and scrub cells (44 of 45) (`tests/zoom_terrain.rs`, a module of the `zoom_continuity` target). The gates run on the power-of-two-aligned MICRO world only (open-items).
11. Labels: relief tiles join across a label and hide it below its zoom (`crates/arda-midzoom/tests/labels.rs:60`). Society exports: a settlement on an area border paints the same pixels from both sides; `off` is the plain export, `on` is deterministic and differs (`crates/arda-midzoom/tests/export.rs:86`, `:136`).
12. Every value is integer and a pure function of the seed and the global position (`crates/arda-midzoom/src/lib.rs:20-21`).

## Outcomes & side effects

- Route: a 256 × 256 lossless WebP (`image/webp`) with `ETag` and `Cache-Control: no-cache`, or `304` (`crates/arda-server/src/routes.rs:169-195`). `/v1/world` advertises `tiles.relief_max_zoom` (`crates/arda-server/src/dto.rs:54-56`).
- Binary: a PNG at `--out` and one timing line, or timing lines only with `--tiles` (`crates/arda-midzoom/src/bin/arda-midzoom.rs:82-97`, `:103-131`).
- The relief world is shared with the tactical world grade as its tint source (`crates/arda-server/src/lib.rs:254-256`, `crates/arda-server/src/relief.rs:100-104`).
- No files written by the server, no notifications, no external calls.

## Dimensions not in play

- D1 Authority: no roles; loopback server and local binary.
- D5 Money: nothing is charged or credited.
- D7 Time: no clock input to rendering; the binary's `Instant` timings are printed only (`crates/arda-midzoom/src/bin/arda-midzoom.rs:73-78`).
- D9 Lifecycle: no entity changes state (State transitions).
- D11 Termination: one synchronous render per request; nothing half-done persists.
- D12 Visibility: a tile shows the whole stored world at that place; nothing is hidden per viewer.
- D13 Notification: none.
- D14 Effects on others: the tile cache is shared by all clients; no other state changes.
- D15 Record and audit: none; tiles are reproducible from the world.
