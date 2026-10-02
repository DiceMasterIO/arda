---
generated_date: 2026-09-30
scenario: midzoom-relief
status: normative design; implementation on feat/midzoom-refine (crate arda-midzoom, route /v1/tiles/relief)
goals: 1, 2, 5, 9, 22, 27, 32, 33, 39, 41, 49
---

# 17 — Mid-zoom relief: on-demand ~10 m refinement between the world map and tactical maps

> The world stores one height every 39.0625 m. Zoomed past the overview's native scale, each sample spans many pixels and the map is smooth by construction ("as if I forgot my glasses"). Storing a 4× finer level everywhere breaks the 16 GiB budget, so — like the tactical layer (09) — detail is grown on demand, for the tiles on screen only, from the stored fine terrain and area data. The layer reads only public APIs (`World::fine_terrain`, `World::read_area`, `arda::area_atlas_terrain`), so it composes with any world that stores recipe-5 fine terrain.

Code cites rules as `// logic/17 §<rule>`.

## Frame

- Refined node `(i, j)` of subdivision `n ∈ {1, 2, 4, 8}` sits at fine-lattice micrometres `(i·s, j·s)`, `s = 39,062,500 / n` (n = 4: 9.765625 m). The fine lattice is the world frame minus `FINE_FRAME_OFFSET_UM` (node 0 is the centre of cell 0, vocabulary I1).
- Every refined height is a fixed integer stencil of global stored nodes and saved cells. A window never changes a node's value; tiles therefore join pixel-exactly.

## Rules

### §base
The stored nodes `H` are sharpened into near-interpolating cubic B-spline coefficients by three rounds of `C ← C + (H − S·C)` (`S` = [1 4 1]/6 per axis, the spline's node filter), then clamped to the 3 × 3 stored range so the surface neither blurs crests nor rings into new pits. The base `B` is the cubic B-spline of `C` evaluated exactly (weights scaled by 6n³).

### §detail
Drainage-aligned detail: per octave (72, 44, 30 m; domains rotated 37°, −23°, 61°), feature points jittered anywhere in their cells carry stripe kernels `w·a·cos(2π·(p − f)·d)` where `d` is the unit vector across the local fall line, `w = (1 − r²)²` within one cell and `a ∈ [0.4, 1.6]` a per-feature depth. Crests and troughs therefore run downslope. Each octave reads the gradient left by the coarser ones, so finer gullies turn down the walls of coarser ones and join them (branching, converging drainage). The cross-section is `2√((1+v)/2) − 1` (narrow V gullies, broad spurs) on soil and its mirror (sharp ribs, broad chutes) on steep rock (slope ≥ 31°–48°). Octaves need ≥ 3 lattice steps per wave and fade below four screen pixels.

### §amplitude
Coarsest-octave amplitude = 2 × stored roughness (mean |H − mean of 8 neighbours| over 3 × 3, the field's own short-wave energy and the maturity proxy) × steepness (7°–26°) × relief (40–200 m over 156 m) × cover (rock 1, scrub 0.9, grass 0.8, forest 0.75, marsh 0.2) × slope position (gullies head below divides, deepen downslope, die out on valley floors) × patchiness (rotated noise at 260 m, 0.3–1.7) × (1 − watercourse) × (1 − standing water). Wall slopes are capped at 0.8 × the fall-line slope, so detail never reverses drainage. Low, gentle ground stays gentle.

### §mantle
Gentle, non-flat ground (slope ≥ 3°) gets turf hummocks: isotropic rotated noise at 56 and 32 m, ≤ 0.8 m, never steeper than half the fall-line slope.

### §means
Two rounds restore every stored node's 39 m cell mean (trapezoid over its refined nodes): residuals are sharpened like §base and added back as a spline.

### §pits
Two passes lift every land node lower than its eight neighbours to 1 mm above the lowest.

### §render
Relief tile pixels are shaded by the formed Atlas shader (logic/04 §atlas-formed) through `AtlasTerrain::relief_surface_colour`: the finest light, height and a 20 m concavity (weight 2) come from the refined surface (Catmull-Rom, C1); broader light, sky, materials and climate come from the stored field and saved cells exactly as in the overview, including the stored shore classes (`shore.bin`, logic/04 §atlas-formed shore) on land and in the shallows. Which surface a pixel shows (land, lake or sea) comes from §water, not from the stored field; lake and sea depth tints read the stored field wherever it holds water there, and §water's depth at the margins only §water calls wet. Below 9 m/px land gets a ±5 % ground-cover tone at 24 and 11 m.

### §water
One water geometry serves every zoom (goal 49): relief tiles draw rivers, lakes, coasts and marsh pools from `arda_refine::region::WaterRegion`, which gathers the same global cells, channel edges and fine lattice nodes a refined block reads (`Ctx::gather_region`, in the I1 frame of logic/09 §square-frame) and answers with the blocks' own rules (logic/09 §linear-features: `standing_water` for shores, the centreline pieces for channels, pools). At a square's centre the region's water is the block's water, so at z9 (one pixel per square on MICRO) relief and tactical water agree square for square. A pixel is sampled at its centre in global square units (world µm / 1,562,500); pools show at ≤ 3.2 m/px.

### §rivers
Channels are the tactical layer's centreline pieces (logic/09 §linear-features: hashed edge crossings, the cell's low node, cubic Hermite with a bow), drawn over land with coverage `clamp(0.5 − d / pixel, 0, 1)`, where `d` is the signed distance to the drawn banks, so coverage passes one half exactly at the banks. Drawn half-width is `max(half · gain, 0.6 px)` with `gain = clamp(pixel / 6.25 m, 1, 4)`: the overview's ×4 symbol at 25 m/px, the true banks at ≤ 6.25 m/px. Colour is the formed overview's discharge band of the piece's saved edge. Channels are not drawn over lakes or the sea, which the blocks mark as standing water.

### §low-relief
On low ground the stored drainage relief is made readable without adding any (`lowrelief.rs`). From the stored 39 m nodes around the window: a neighbourhood mean (three box passes of radius 5 nodes, ≈ Gaussian σ 214 m) and the local relief (mean absolute deviation from it, two box passes of radius 8). Both terms fade in below 10 m of local relief and are full below 3 m:
- **position**: `t = (h − mean) / max(relief, 0.7 m)`, clamped to ±1.2, tints valley floors cooler and greener (−4 % red, −3.5 % blue per unit) and interfluves warmer and paler (+3 %, +2.5 %, +1 %);
- **slope light**: the refined surface's Lambert term (light from the north-west at 42°, logic/04) is added again with gain `clamp(6 m / max(relief, 2.4 m), 1, 2.5) − 1`, so terrace risers and bluffs read as lines while flat treads stay flat.
Integer arithmetic over global nodes with a margin wider than the kernels: tiles join exactly. Only worlds with society data take it (§land), so others stay byte-identical.

### §land
Relief tiles of a world with `society/` (`arda settle`, `arda society build`) draw worked land from the geometry the tactical overlays compose (arda-blocks `SocietyOverlays`, shared with the server's blocks), so parcels, roads and buildings stay in place across the relief → tactical hand-off as water does (§water). Order per land pixel: shader colour → §low-relief → land use and canopy → channels (§rivers) → roads → settlements. Detail by pixel size: > 45 m the 100 m land-use raster as a bilinear tone; ≤ 45 m parcels, hedgerows and roads; ≤ 20 m settlement plans; ≤ 6 m strips, furrows, orchard rows, crowns, roof pitches and shadows. A world without `society/` renders byte-identically to before.

### §land-fields
Parcels are the arda-fields partition itself (`arda_fields::partition`), built once per window from the same inputs the tactical fields layer reads: the land-use raster (crofts as arable), heights and river pieces of the surroundings, settle's road segments and the settlements within `INPUT_REACH_M` (3.2 km) of the window, and the fields layer's culture and wealth (`FIELDS_CULTURE`, `FIELDS_WEALTH` of arda-blocks). The partition is a fixed global hierarchy:

- **Land blocks.** A brick lattice of 500 m tiles paired into blocks two tiles wide, alternate rows offset by a tile (every block corner a T-junction); vertices displaced by a smooth 2 km noise field (±0.6 tile, so the brickwork turns and stretches from place to place) and jittered ±0.17 tile by their own hash. Everything is drawn in a warped frame: rotated value noise at 150 and 60 squares, up to 4 squares in old enclosure and 1.4 in planned.
- **Subdivision.** Each block is split recursively until a piece is the size its ground asks for. A road (track and above; its quadratic B-spline, as arda-ways draws it) crossing a piece still to be split nearly straight is the cut, so fields line the roads. Otherwise the cut runs across the piece (or lengthwise, for long narrow closes near the houses) square to its longest straight side, so new hedges meet old ones near right angles; on slopes above 0.035 it follows the contour where a side allows. Old enclosure jitters cut positions (0.3–0.7) and angles (±10°, a piece over 20 ha takes its own grain ±34°) and kinks 60% of cuts by 4–17°; planned enclosure (a 4 km noise field, culture-biased) cuts near the middle along one surveyed bearing per 6 km. Cuts that would leave a wedge (a child filling less than 0.64 of its principal-axis box) are retried gentler. No cut but a road may meet the piece's boundary (its clipped outline, the block outline and the true lines of the cuts above, a road's end rays included) at under 38°, nor run within 12 squares of a boundary edge at under 38° (a sliver whose tip another edge blunts); the last resorts, square to the frontage and then along it, obey the same rule, and a piece no cut can split stays one field, so a would-be wedge stays part of its neighbour. The partition tests hold every field corner in five scenarios to 35° or more, away from roads and rivers; on MICRO 42 the rule cut corners under 35° from 98 to 4 in six 4 km squares. Where broad cover (farmed, wood, waste) changes within a piece, a straight cut that separates it clearly better wins, and mixed pieces split down to 1.2 ha.
- **Sizes.** Enclosed ploughland 0.4 ha by the settlement's built edge rising smoothly to 3.4 ha 1.4 km out (pasture ×1.25); furlongs 6 ha within a village's open-field disc; meadow 1.4, orchard 0.7, farmstead closes 0.45, woods 7, waste 9 ha; × wealth (0.8–1.2), culture (dwarf 0.75, elf 0.85, halfling 0.6) and planned enclosure (up to ×1.25); each piece's target × 0.4–2.2 (log-uniform). No field is longer than 180 squares or smaller than 640.
- **Use.** A field's class is the majority of its land-use samples, read through a smooth displacement (up to 70 m) so farmland outlines wander with the field edges; `fields::kind_of` then decides (strips inside open-field discs), except that farmed fields within 70 m of a river bank on slopes below 0.05 are floodplain meadow and pasture on slopes above 0.14 (or, 30%, beyond 900 m from a settlement) is open common.

A point's parcel is its leaf (`Partition::locate`). Since every part of a cut outside its own piece lies outside every leaf beneath it, the distance to a leaf's edge is the least distance to the cuts above it and the block outline (`locate_edge`), and the parcel across is the leaf just over that nearest point: tones blend across the edge box-filtered, and a hedge (2.6 m, swelling and gapping with rotated noise at 16 m) or a drystone wall (1 m, on fields steeper than 0.12, logic/09's mixed-region kit) is drawn on edges where either parcel is enclosed. A pixel at or below 2 squares decides at its square's centre, as the tactical plan does, coarser pixels at their own centre. Tones are multiplicative tints against the shader's lowland grass (§land-tones), with the per-parcel jitter on enclosed fields and furlongs only (woods and commons run on without seams); strength is 1 at ≤ 3 m/px, 0.7 at 8, 0.55 at 15 and 0.42 from 30 m out (the coarse raster tone uses 0.42 too). Strips (`strips::locate`, per-strip crop, one-square balks at ≤ 3.2 m/px, a 4.5 m grass headland round each furlong), furrows (2.6 m, ±7 %, faded before aliasing) and orchard rows (trees every 5 × 4 squares along the parcel) appear once pixels resolve them. Unworked valley floors (§low-relief position < 0) take a floodplain-meadow tint.

### §land-roads
Road centrelines are `arda_ways::plan::build` over the window (B-spline pieces, switchbacks, junction snapping) with every 0.5 m station; a pixel's coverage is the box-filtered share of the class band (highway 5, road 4, track 2, footpath 1 squares, logic/08 §roads), so at one pixel per square it covers exactly the squares whose centres lie within half the width. Coarser pixels keep a minimum drawn half-width (0.70, 0.55, 0.38, 0.30 px from highway to footpath) with opacity `(true / drawn)^0.6`; footpaths stop above 8 m/px, tracks above 24, roads above 64. Squares a town's streets replace (arda-blocks `Streets::replace`) are skipped. Crossings are not re-planned: where the tactical layer straightens a bridge approach the two may differ by a few metres.

### §land-towns
Settlements whose plan radius reaches the window are drawn from their `arda-town` plans square by square (`grid.kind`, `grid.building`), plans claiming squares in id order as in arda-blocks; a pixel averages up to 4 × 4 squares. Roofs: thatch, red or brown tile, slate or shingle by tier, wealth and a per-building hash, ±6 % weathering; at ≤ 4 m/px pitches lit from the north-west, a darker ridge along the longer side and ground shadows two-thirds of the building's height toward the south-east. Streets (cobbles where paved), squares, walls, yards, gardens, greens, churchyards and crofts (which give way to roads) have their own paint.

### §land-tones
Grassland reference (111, 119, 62); targets: ploughed (120, 108, 74), stubble (136, 132, 86), fallow (120, 124, 74), hay (116, 132, 66), mown (134, 142, 80), grazed (104, 126, 62), orchard (94, 114, 58), woodland (72, 92, 48), floodplain (98, 126, 66); per-parcel jitter ±4 % brightness, ±2 % warmth; within-field soil noise at 55 m, ±3.5 %. Canopy (woodland parcels; forest cover read bilinearly between cell centres): clumps at 40 and 18 m (±10 %) fading out by 30 m/px, then crowns on a jittered 6.5 m lattice lit from the north-west below 6 m/px.

### §pyramid
Relief levels continue the overview pyramid: level `z ∈ (max_zoom, max_zoom + 8]` has `max(areas) × 51.2 km / (256 · 2^z)` metres per pixel; pixel `p` covers world `[p·m, (p+1)·m)`. Subdivision: n = 4 at ≤ 16 m/px, n = 2 at ≤ 32 m/px, else 1. Pixels outside the world are transparent. The viewer switches to these tiles past native zoom and offers the tactical map (logic/09) at ≤ 2 m/px.

## Checks
Land agreement (`tests/zoom_land.rs` in the zoom-continuity gate: relief z9 against composed tactical blocks on MICRO 42 villages, town centres, roads and farmland; road IoU ≥ 0.85, building IoU ≥ 0.95, walls between two fields on a relief hedge line ≥ 0.95 — all 1.0 at merge; land tiles pixel-exact across windows at z7, z8 and z10). Synthetic (`arda-midzoom/tests/land.rs`): every tactical field is one relief parcel and every wall between fields lies on a parcel edge; road squares IoU ≥ 0.97 against `arda_ways::apply_ways` (1.0). Water agreement (`tests/zoom_continuity.rs`, release gate: relief z9 and tactical water squares on MICRO 42 rivers, an inlet and coasts, IoU ≥ 0.9 per cell; `region_water_matches_the_refined_blocks` on the synthetic world, lakes included). Seams (windows and adjacent tiles pixel-exact), 39 m means (mean error ≤ 0.3 m, worst ≤ 2.5 m on a synthetic massif), pits (none deeper than 0.5 m), drainage (≥ 90 % of refined channels draining ≥ 0.3 km² lie within 60 m of stored D8 channels), isotropy (detail energy within 10 % over twelve azimuths), gentle plains stay smooth, and a warm 256 px tile at 9.77 m renders in < 150 ms.
