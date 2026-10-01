---
generated_date: 2026-09-30
scenario: midzoom-relief
status: normative design; implementation on feat/midzoom-refine (crate arda-midzoom, route /v1/tiles/relief)
goals: 1, 2, 9, 22, 27, 32, 33, 49
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

### §pyramid
Relief levels continue the overview pyramid: level `z ∈ (max_zoom, max_zoom + 8]` has `max(areas) × 51.2 km / (256 · 2^z)` metres per pixel; pixel `p` covers world `[p·m, (p+1)·m)`. Subdivision: n = 4 at ≤ 16 m/px, n = 2 at ≤ 32 m/px, else 1. Pixels outside the world are transparent. The viewer switches to these tiles past native zoom and offers the tactical map (logic/09) at ≤ 2 m/px.

## Checks
Water agreement (`tests/zoom_continuity.rs`, release gate: relief z9 and tactical water squares on MICRO 42 rivers, an inlet and coasts, IoU ≥ 0.9 per cell; `region_water_matches_the_refined_blocks` on the synthetic world, lakes included). Seams (windows and adjacent tiles pixel-exact), 39 m means (mean error ≤ 0.3 m, worst ≤ 2.5 m on a synthetic massif), pits (none deeper than 0.5 m), drainage (≥ 90 % of refined channels draining ≥ 0.3 km² lie within 60 m of stored D8 channels), isotropy (detail energy within 10 % over twelve azimuths), gentle plains stay smooth, and a warm 256 px tile at 9.77 m renders in < 150 ms.
