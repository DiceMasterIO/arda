---
generated_date: 2026-09-30
scenario: tactical-refinement
artifact: ../mockup-artifact.md
status: normative design; implementation on feat/tactical-terrain (crate arda-refine), with feat/tactical-ways and feat/tactical-fields filling reserved features
goals: 42, 43, 46, 47, 48, 49, 50
---

# 09 — Tactical refinement: 5-ft terrain, WFC, edge rules and seams

> Normative design for turning one 100 m world cell (or a window of cells) into a 64 × 64 grid of 5-ft squares. The mechanism is the artifact's section "Inside a cell: the D&D grid", which is binding: a block is a faithful rendering of its cell and its 8 neighbours, linear features cross block edges at points fixed from coarse data alone, and WFC fills the rest. This file supersedes the storage-oriented parts of [03 — Block generation](03-block-generation.md) for new tactical output: blocks are generated on demand and never written to `blocks/` archives, which stay as they are for legacy worlds. The output feeds the [art compositor](11-tactical-art-compositor.md) and [scene data](12-scene-data.md); roads, fields and towns are reserved into the block by the rules here and filled by [08](08-settlements-roads-realms.md)-derived layers and [10 — Town layout](10-town-layout.md).

Code cites rules as `// logic/09 §<rule>`.

## Trigger & preconditions

- Trigger: `refine_block(world, society, GlobalCell)` or `refine_window(world, society, gsx0, gsy0, w, h)` (window in global squares), called by the service on a cache miss (16-service-api §api-cache) or by `arda-refine render` for debugging.
- Preconditions: the cell and its 8 neighbours are inside the world; the world's cells, rivers and lakes are readable; `terrain/fine.bin` exists (recipe ≥ 2; worlds without it fall back to bilinear cell heights, see Branches). `society/` is optional: without it, no roads, fields or buildings are reserved and the block is purely natural.
- A block reads only coarse data: world cells, river and lake records, fine terrain, `society/` files and the settlement's town plan (10-town-layout), all of which are global functions. It never reads a neighbour block's finished squares (artifact; logic/03). The artifact's input is the cell and its 8 neighbours; an implementation may read further out (feat/tactical-terrain reads a 7 × 7 cell neighbourhood for smoothing) provided every derived quantity remains a function of global position.

## Rules

### §square-frame

- Square pitch for **sampling the world** is exactly `100 m / 64 = 1.5625 m` (1,562,500 µm). Square pitch for **play** is 5 ft. The block covers its cell's footprint exactly (08 §world-frame), so horizontal distances inside a block are compressed by 0.97536 relative to the world. This is the artifact's and logic/03's "nominal 100 m / 97.5 m" choice made explicit; vertical values are true (feet of real height).
- Global square `(gsx, gsy) = (64·gx + sx, 64·gy + sy)` for square `(sx, sy)` of block `(gx, gy)`, with `0 ≤ sx, sy < 64`, x east, y south.
- Square centre in world micrometres: `x_um = 1,562,500·gsx + 781,250 − 50,000,000` (likewise y). This is exact integer arithmetic and is the `TerrainPoint` passed to fine-terrain sampling.
- The fine lattice spacing 39.0625 m is exactly 25 square pitches, so lattice lines fall on square boundaries every 25 squares.
- A layout's local square `(x, y)` maps to global `(gsx0 + x, gsy0 + y)`, where `(gsx0, gsy0)` is the layout origin recorded in `RefineMeta.origin_gs`.

Source: `SQUARE_SIZE_MM = 1524` and `BLOCK_SQUARES = 64` in `arda-core`; artifact "a cell is a whole number of squares … 64 squares (320 ft, 97.5 m) is the natural choice"; logic/03 "the cell's 100 m is nominal 97.5 m".

### §elevation

Elevation at any square is a function of its global square coordinate only:

1. `E0(p)`: bicubic (Catmull-Rom, integer fixed point, weights in 1/2¹⁶) interpolation of the fine terrain lattice at the square centre. On worlds without fine terrain, bilinear interpolation of the four surrounding cell-node heights.
2. `D(p)`: detail, the sum of three octaves of value noise at wavelengths 8, 16 and 32 squares, each sampled through a distinct integer rotation matrix (the Pythagorean pairs (3,4,5), (5,12,13), (8,15,17)), so no octave is axis-aligned. Wavelengths stay ≥ 5 squares (goal-prompt §7 lesson). Amplitude `a = a0 · k_slope · k_cover`, with `a0 = 0.3 m`, `k_slope` rising linearly from 1 at 0° to 6 at ≥ 30° cell slope, `k_cover` 1.5 for rock and scrub, 1 otherwise, 0.3 for marsh and fields (all assumed, tunable). The slope and cover inputs are bilinear blends of the four surrounding cell nodes, so amplitude is continuous across block edges.
3. `E(p) = E0(p) + D(p)` in millimetres, except inside reserved water, road and building squares, where the owning rule sets the ground (below) and `D` is faded to 0 over 2 squares.
4. Play elevation: `elevation_ft = 5 · floor(E_mm / 1524)` (5-ft contour steps; 1,524 mm is exactly 5 ft). Clamped to the `i16` range; a clamp sets `meta.flags.elevation_clamped`.

`elevation_ft` is **absolute**: feet above sea level. Water squares store the bed elevation; the water surface is `elevation_ft + water_depth_ft`.

### §linear-features

Rivers, streams, lake and sea shores, roads, field boundaries and walls are placed before WFC (goal 46). Every linear feature that crosses the shared edge of cells A and B does so at a point both sides compute from `(seed, kind, A, B)` alone:

- **Which edges.** A river enters its cell from each upstream cell in its course and leaves toward its downstream cell (artifact). A road enters from the previous road cell and leaves toward the next (from `roads.json` routes). A D8 **diagonal** step from A to D is routed through the one of the two orthogonal cells B, C adjacent to both that has the lower cell height (rivers) or lower road cost (roads), ties by lower `(gy, gx)`; the feature then makes two orthogonal crossings. The via-cell is computable by every block that sees A or D.
- **Where on the edge.** `offset = 32 + (H(seed, kind, min(A,B), max(A,B)) mod 33) − 16` squares along the edge (so between 16 and 48, keeping clear of corners), where `H` is the canonical hash (§hash) and cells are ordered by `(gy, gx)`. Rivers and roads use distinct `kind` tags, so a road never lands on the same point as a river by rule.
- **Width at the edge.** `(w_A + w_B) / 2` for rivers (cell watercourse widths); the class band width of 08 §roads for roads. The width is a symmetric function of A and B, so both sides agree.
- **Inside the block.** The centreline is a cubic Hermite curve from entry to exit, tangent to the edge normal at both ends, with a lateral midpoint offset of up to ±12 squares keyed by `(seed, kind, gx, gy)` and scaled by `max(0, 1 − slope/5°)` (lowland meanders, goal 9 at 5-ft scale). Confluences inside a block join tributary curves to the trunk curve at its nearest centreline point. Width varies linearly along the curve between entry and exit widths. Sub-square channels (width < 1.5625 m) are one square wide, `water_depth_ft = 1`.
- **Cross-section.** Thalweg depth `d = clamp(0.1·w + 0.2, 0.2, 6)` m (assumed, tunable; the artifact gives only "depth a little slower than width"); depth at lateral offset `u` is `d·(1 − (2u/w)²)`; `water_depth_ft = round(depth_m / 0.3048)`, at least 1 on wet squares. Bank squares (dry, within 1 square of water) get `mud` or `gravel` (order ≥ 3 or slope ≥ 3°), or `reed_bed` where wetness ≥ 0.7 and slope < 2° (assumed).
- **Shores.** A square is sea when `E(p) < 0` and at least one of its four surrounding cell nodes is sea; lake when `E(p) < surface_m` and one of the four nodes is that lake's cell (lake records). Depth is `surface − E` (sea surface 0). A dry square touching sea or lake is `sand` where the square-scale slope is < 8° and the node cover is not rock, else `rock` (artifact beach/cliff rule at 5-ft scale; slope assumed).
- **Cliffs.** A square whose `elevation_ft` exceeds a 4-neighbour's by ≥ 10 ft gets ground `cliff` (the upper square), matching the climb threshold of [12](12-scene-data.md) §scene-climb.
- **Contours** need no edge rule: they are level sets of the global `E`.
- **Relief.** The shore, channel and pool rules are point functions of global position, so `arda_refine::region::WaterRegion` answers them for any window; mid-zoom relief draws its water from it ([17](17-midzoom-relief.md) §water), and a region's water at a square centre is the block's.

Roads (bands, bridges, fords, ferry landings, switchbacks) are reserved by feat/tactical-ways with these same rules; farmland parcels, hedges and field walls by feat/tactical-fields (§reservations). A ford is a road band over water whose depth is capped at 2 ft; a bridge is a `floor`-layer deck placement plus `planks` squares with `water_depth_ft` unchanged beneath (goal 43).

### §reservations

Before WFC, every square is either **free** or **reserved** by exactly one owner, in this precedence (higher wins):

| Precedence | Owner | Squares | Rule source |
|---|---|---|---|
| 1 | town (10-town-layout) | building footprints, yards, streets, plazas, town walls | 10 §town-clip |
| 2 | ways | road bands, bridges, fords, ferry landings | §linear-features, 08 §roads |
| 3 | water (refine) | river, lake and sea squares, banks | §linear-features |
| 4 | fields | farmland, pasture and orchard parcels; hedges and drystone walls on cell boundaries | 08 §landuse, artifact "field hedges or walls along the cell boundaries" |
| 5 | cliffs (refine) | cliff squares | §linear-features |
| — | free | everything else, filled by WFC | §wfc |

A road over water is a bridge or ford (ways wins over water); a building never stands on water (town refuses such plots, 10 §town-plots). Reservation functions read only coarse data, so they are seam-safe by construction. Each owner also sets its squares' ground key, `water_depth_ft`, placements and rules-sidecar cells.

Two claims are soft, ranked between the rows above (arda-blocks `Owner`):

- **Crofts** (town, between water and fields): the open ground of a settlement's built-up footprint (10 §town-crofts), which fields run up to but never enter, gives way to water and roads.
- **Verges** (ways, below fields): a verge or shoulder that kept the natural ground; fields may take it, and over forest it is cleared to grass.

Where a world road comes within a town plan's reach of its market (the length of its longest main street, less 2 squares), the town's streets replace the road; ways' river channels and banks only guide crossings, since the refined river is the real one. Fields claim only worked ground: a field site with no land use (kind `wild`) stays natural, so its ecology, rocks, copses and trails come from the refiner rather than a heath-and-scrub paint. Fields also mark a noise-shaped **fringe** of 1.5–6 squares of rough grass and scrub on wild ground beside worked land, applied only over wooded natural ground, so forest edges are organic.

### §ground-field

`G(p)`: the deterministic natural ground at a global square, used for the fixed border frame and as the relaxed fill:

- Weight vector per cell node from cover, forest density, wetness, slope and aspect (table below); the square's weights are the bilinear blend of its four surrounding nodes (so density gradients thin across the block and never stop at its edge, artifact).
- Two octaves of rotated value noise (wavelengths 6 and 20 squares) perturb the weights; the maximum weight picks the base key.
- A deterministic transition pass picks the tile for each square from its base key and its 4 neighbours' base keys, so `G` is a legal tiling by construction (Invariant 5).

| Node cover | Base keys and weights | Modifiers |
|---|---|---|
| forest | `forest_floor` 60, `leaf_litter` 20 (broadleaf) or `moss` 20 (conifer), `scrub` 10, `grass` 10 | forest density scales forest keys |
| grass | `grass` 60, `meadow` 30, `dirt` 10 | wetness ≥ 0.7 → `marsh` 20 |
| scrub | `scrub` 50, `heath` 30, `grass` 20 | slope ≥ 25° → `scree` 20 |
| marsh | `marsh` 50, `reed_bed` 25, `mud` 15, `water_shallow` 10 | |
| rock | `rock` 50, `scree` 35, `moss` 15 | north-facing and < 2 °C → `snow` 20 |
| ice | `snow` 70, `ice` 30 | |
| bare | `dirt` 40, `gravel` 30, `sand` 30 | coast → `sand` 60 |

Landform modifies the blend (`prior.rs` `landform`): knolls and ridges gain rock, heath and thin-soil dirt and lose grass and meadow; hollows gain meadow, moss, and marsh and mud where the cells are wet; talus gains scree. In cold country (cell mean below 5 °C) snow lies in proportion to shade (north-facing steep ground) and hollowness, so patches sit in shaded hollows on north slopes. The patch noise was halved (amplitude 1.0 in log-weight) so the landform, not noise blobs, places the ground.

All weights assumed, tunable; ground keys are the canonical ones in `docs/goal-prompts/vocabulary.md` (untracked; copied into 11-tactical-art-compositor §vocabulary).

**Arid basins** (recipe 7, logic/02 §world-water arid basins). A free land square whose stored playa cell (`water.bin` pan runs) is salt crust or mudflat takes the ground key `salt_crust` or `mudflat` instead of its WFC class; saline lake squares stay water. The cell is looked up after a smooth jitter of up to 20 squares (two-octave noise, 48-square wavelength) of the square's position, so crust and mudflat edges wander instead of following the 100 m grid, and neighbouring blocks agree. Natural scatter (trees, bushes, rocks) is removed from playa squares. `mudflat` is difficult terrain (soft clay); `salt_crust` is not.

### §wfc

- **Tiles.** At least 60 natural tiles: every base key above plus transition tiles for each legal pair of adjacent base keys and bank/shallow transitions. A tile is `(u16 id, base key, corner or edge classes)`; feat/tactical-terrain uses corner-labelled (Wang-corner) tiles over 19 ground classes and 70 legal class pairs, which satisfies this rule. Adjacency is legal when the facing corners or edges match. The exported `Square.ground` is the tile's base key; the tile id is kept in `RefineMeta.tiles` for review.
- **Fixed cells.** The 1-square border frame (`sx` or `sy` in {0, 63}) and every reserved square are fixed before WFC: the frame from `G`, reserved squares from their owner. Seams are therefore guaranteed without looking at neighbours.
- **Fill.** Most-constrained square first (fewest remaining tiles; ties by lowest global row-major index), weighted choice from `G`'s weight vector, arc-consistency propagation after each choice.
- **Contradiction.** Restart with fixed cells kept, subseeded by `(seed, "wfc", gx, gy, attempt)`, at most 8 attempts (logic/03). Then the relaxed fill: every free square takes `G(p)`, which is legal by construction, and the cell is listed in `RefineMeta.relaxed` (goal 47). WFC never fails a request.

### §scatter

- Trees, bushes, boulders, rocks, fallen logs, stumps, ferns, reeds and cattails are placed by Poisson-disk sampling in **global** 16 × 16-square scatter tiles: each tile draws candidates from `(seed, "scatter", kind, tile)`, and a candidate survives when no higher-priority candidate (by hash) within the minimum spacing exists in the tile or its 8 neighbours. Every block that contains a tile computes the same survivors.
- Density field: canopy share target = node forest density, blended bilinearly; free squares only; reserved squares take no natural scatter.
- Minimum spacing and footprint (assumed, tunable): large tree 2.5 squares, small tree 1.5, bush 1, boulder 2, log 2, reeds 0.7.
- Species (artifact "conifer where cold, broadleaf where warm, mixed in between"; thresholds assumed, tunable): mean temperature < 6 °C → `veg.tree_pine` / `veg.tree_spruce`; 6–9 °C mixed; > 9 °C → `veg.tree_oak` / `veg.tree_elm` / `veg.tree_birch`; wetness ≥ 0.7 → `veg.tree_willow`; forest edges favour `veg.tree_birch`; `veg.tree_dead` 2 % in conifer stands.
- **Ecology (goals 19, 20; `ecology.rs`).** Each kept point reads the cells' temperature, moisture, wetness, forest density and cover, bilinearly blended, corrected locally: effective temperature is the cell's plus up to ±1.8 °C for sun or shade (`-north × steepness`); growth fades from 1 to 0 between +3.5 °C and -1.5 °C (the world's canopy limit runs from -4 °C to +5 °C); wetness rises in hollows and within six squares of water and falls on knolls. Species follow: near the tree line dead and stunted trees (`veg.tree_stunted`, krummholz) take over; within about four squares of water or in wet hollows `veg.tree_willow` (above 8.5 °C) or `veg.tree_alder`; conifer share `smoothstep(9.5, 3.5, T)` plus a ridge bonus, with `veg.tree_spruce` on moist, shaded ground and `veg.tree_pine` on dry, sunny knolls; broadleaves `veg.tree_oak` on warm, drained ground, `veg.tree_elm` on moist lowland, `veg.tree_birch` as the small tree of edges and cool ground. Low plants: ferns in shade, heather on cold, dry knolls and scrub, tall grass in damp swales, flowers on sunny meadow, mushroom rings now and then; shrubs add `veg.juniper` on cold open ground. Thresholds assumed, tunable.
- **Landform (`shape.rs`).** Curvature of the land surface over rings of 2 and 5 squares gives a hollow index (`+` swale, `-` knoll) and a talus index (the foot of ground steeper than 32° within three squares). Both read only the physical grid, which now reaches 8 squares past the block so every point a block asks about is exact.
- **Structure (`density.rs`).** Woods grow in groves: a slow noise field (26 squares, 2 octaves) is cut at the level that leaves a share of the ground equal to the forest density wooded (logistic approximation of the normal quantile), so sparse cells get copses, dense ones glades, and the expected canopy stays proportional to density; grassland adds 7 % copse cover. Broken tree lines follow streams and lake edges within 4.5 squares of water. Young trees and undergrowth gather on grove rims. Rock outcrops (`veg.rock_outcrop`, 3 × 3, total cover, blocks movement) stand on steep knolls; boulder fields of boulders, small rocks and stones collect on talus; loose rocks clump; shrubs and grass tufts fill crevices in rocky hollows. Fallen logs and stumps follow the canopy, with more stumps near settlements. Low plants and shrubs gather in drifts of a 11-square noise field.
- **Trails (`trails.rs`).** A sparse network of game trails and footpaths (ground key `trail`, not difficult). Whether a trail crosses the edge between cells A and B, and where (12–51 squares along it, its own hash tag), is a function of A, B and their four-neighbours alone: an edge score of a hash plus bonuses for water (watercourse cells), settlement (built or road cells, which make it a footpath) and passes (saddle cells), minus slope and large-river penalties, against a threshold. Inside a cell every crossing joins the cell's hub (its river node, where the trail fords, or a hashed clearing) by the cheapest eight-connected route over the smooth terrain, with cost `1 + 40·grade² + 1.2·noise + 12·bank`; diagonal steps keep a corner square so the trail is four-connected. Routes stay inside their cell, so a block computes its own and its eight neighbours' trails, and blocks agree square for square. Trail squares on water are fords; overlays (roads, fields, towns) take precedence. No scatter stands on a trail, and no trunk, rock or log beside one.
- Placements use `AssetRef::Id` with the canonical `veg.*` ids; a query is used only for purely decorative variety (`{"class": "vegetation", "tags": ["free:rock"]}`). Specific art is the library's concern (11 §validator, rule `vocabulary_coverage`).

### §rules-sidecar

Per-square SRD rules are written into the `arda-scene` `RulesSidecar` (format 2, [12](12-scene-data.md) §scene-sidecar), never into `Square` (which denies unknown fields):

| Field | Refine sets it when | Source |
|---|---|---|
| `difficult` | ground is `scrub`, `heath` (undergrowth), `scree`, `mud`, `mudflat`, `marsh`, `reed_bed`, `snow` or `ice`; or a bush or fallen log stands in the square | SRD 5.1 difficult terrain examples (undergrowth, snow, shallow bogs, rubble); the key mapping is assumed |
| `water_depth_ft` | always on water squares (equal to the layout's) | §linear-features |
| `cover` | trunk of a large tree or a large boulder → `three_quarters`; small tree, bush or small rock → `half` | SRD 5.1 cover ("a narrow tree trunk" half, "a thick tree trunk" three-quarters); boulder mapping assumed |
| `blocks_sight` | canopy covers ≥ 7 of the 9 squares of its 3 × 3 neighbourhood (dense foliage, heavily obscured) | SRD 5.1 obscurement; threshold assumed |
| `lightly_obscured` | canopy covers 4–6 of 9, or `reed_bed` | SRD 5.1 "moderate foliage"; threshold assumed |

Wading and swimming are derived by the scene from depth (12 §scene-movement), not set here.

### §hash

One canonical integer hash serves every tactical rule whose result two crates or two blocks must agree on (edge offsets, via-cells, scatter tiles, value-noise lattices, variant picks): the SplitMix64-finaliser chain `hash(seed, tag, args…)` of feat/tactical-terrain (`hash2`, `hash3`), with `tag` a fixed 64-bit constant per rule kind and arguments as i64. After merge it moves to one shared function (proposed: `arda_core::tactical_hash`), and its outputs are frozen by a golden test; changing it is a format change. `arda-npc` keeps its own BLAKE3 streams (13 §npc-seed), which no other crate needs to reproduce.

Floating point: feat/tactical-terrain and feat/tactical-ways compute in `f64`. That is acceptable only single-threaded per block, with `+ − × ÷ sqrt floor` and no platform-variant functions (`sin`, `exp`, `powf` must be table- or series-based under repo control); integer or fixed point stays preferred (goal-prompt §8).

## Steps

1. Validate the request (cell or window inside the world; window ≤ 4 × 4 blocks, 65,536 squares, matching the scene grid bound).
2. Read the cell and its 8 neighbours (for a window, the covered cells plus a 1-cell ring), their rivers and lakes, the fine terrain window, and the relevant `society/` records.
3. Compute `E` and `elevation_ft` for every square (§elevation).
4. Reservations in precedence order (§reservations, §linear-features).
5. Fixed frame from `G`; WFC over free squares; relaxed fill on failure (§ground-field, §wfc).
6. Scatter over free squares (§scatter); owners add their own placements (town dressing, 11 §dress).
7. Rules sidecar (§rules-sidecar) merged with the owners' cells.
8. Assemble `RefinedBlock` (Outcomes).

## Branches

- Sea cell with no land square in the block: `NoBlock` (the service answers 404, 16-service-api).
- World without fine terrain: bilinear cell heights for `E0`; `meta.flags.coarse_elevation = true`.
- No `society/`: no town, ways or fields reservations; `meta.flags.natural_only = true`.
- Window: the same functions over a larger rectangle; WFC runs per block (each block's frame fixed from `G`) so a window equals the concatenation of its blocks square for square (Invariant 3).

## Unhappy paths

- Out-of-world cell or oversized window: a typed request error (400 or 413 at the service).
- Unreadable world layer: the stored-layer error with its path; nothing is cached.
- WFC contradiction: never an error (§wfc).
- Elevation outside `i16` feet: clamped and flagged, not an error.

## State transitions

None. Refinement is a pure function; results are cached by the service (16-service-api §api-cache), never written into the world directory.

## Invariants

1. Determinism: the same world, society and request give byte-identical `RefinedBlock` JSON; generating cells in any order, or in parallel, gives the same bytes [42].
2. Seams: for every edge inside a 3 × 3 set of blocks generated independently, the tile pairs across the edge are legal; `E` is continuous (it is one global function); and every river and road that leaves one block enters the next at the same edge offset, with the same width and water depth, square for square [42, 46].
3. Window equality: `refine_window` over whole blocks equals the concatenation of `refine_block` outputs [42].
4. World agreement [42, 43]: the block's mean `E` is within `max(1 m, 0.1 · (fine.max − fine.min))` of the cell's stored height; the median square-scale slope is within ±30 % or ±2° of the cell's slope; water squares exist iff the cell or a neighbour is a watercourse, lake or sea whose geometry reaches the block; the canopy share of a forest cell is within ±0.1 of its forest density. Tolerances assumed, tunable.
5. Legality: after a normal fill every pair of adjacent tiles is legal; `G` over any 256 × 256-square window is legal; on a forced contradiction the relaxed fill is used and the cell is in `meta.relaxed` [47].
6. No grid artefacts: the orientation histogram of ground-class borders and of `D`'s gradient, in 16 bins over 180°, has no bin above 1.5× the mean (statistical, over MICRO seed 42 forest and hill cells) [49; goal-prompt §7].
7. Reservation exclusivity: every square has exactly one owner or is free; no natural scatter on a reserved square.
8. Frame: `x_um` of every square centre follows §square-frame exactly; `elevation_ft` is a multiple of 5.
9. Performance (release, one thread): one block < 250 ms; a quarter-block window (32 × 32) < 100 ms [50].

## Outcomes & side effects

`RefinedBlock`, serialised as three JSON documents (the service may send them together, 16-service-api):

| Part | Schema | Notes |
|---|---|---|
| `layout` | `arda-tactical` `TacticalLayout` | `name` = `cell_<gx>_<gy>` or `window_<gsx0>_<gsy0>_<w>x<h>`; ground keys, `elevation_ft`, `water_depth_ft`, walls (from town and fields), placements, lights |
| `rules` | `arda-scene` `RulesSidecar`, format 2 (12 §scene-sidecar) | same width and height, row-major; merged from refine, ways, fields and town |
| `meta` | `RefineMeta`, format 1 | `{format_version, generator: "arda-refine", generator_version, seed (decimal string), contract_version (world cell contract), origin_gs: [gsx0, gsy0], cells: [[gx, gy]], relaxed: [[gx, gy]], attempts: [[gx, gy, n]], flags: {coarse_elevation, natural_only, elevation_clamped}, tiles: run-length [[count, id]]}` |

`meta.origin_gs` is what the compositor needs for globally seeded, seamless art (11 §seam-art). Until the layout schema carries an `origin` field, the service passes it beside the layout.

Debug: `arda-refine render --world <dir> --cell gx,gy [--window 3x3] --out <png>` draws flat colours per ground key, 5-ft contour lines, water and tree dots. It is for checking only.

## Dimensions not in play

- Interiors, buildings and streets: owned by 10-town-layout; refine only honours their reservations.
- Painted art: owned by 11-tactical-art-compositor.
- Dynamic state (weather, burning, frozen-now, time of day): the game's, not Arda's (logic/03 amendment).
- Legacy `blocks/*.tiles.zst` archives and the 24-tile prototype: unchanged and still loadable.
