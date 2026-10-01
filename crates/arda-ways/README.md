# arda-ways

Tactical (5-ft) ways for arda. The crate turns the world's road polylines
and crossing records into square-level detail on an `arda_tactical`
`TacticalLayout` window: road surfaces, verges, ditches, cut benches and
switchbacks, bridges, fords, ferries and toll houses. It covers goals 37,
43 and 46 of `docs/goal-prompt.md`. The rules come from the "Roads" and
"Inside a cell: the D&D grid" sections of `docs/capstone/mockup-artifact.md`.

```rust
let out = arda_ways::apply_ways(&mut layout, [x_m, y_m], &roads, &crossings, &terrain, seed)?;
out.sidecar.to_json()?; // per-square scene semantics, same size as the layout
out.rules;              // the same as arda-scene's RulesSidecar format 2
arda_ways::fallback::degrade(&mut layout, &library); // before rendering
```

## Inputs

| Type | Source | Notes |
|---|---|---|
| `Road` | `arda-settle` `society/roads.json` | `id`, `class`, `segments` (polylines of `[x_m, y_m]`, x east and y south). Other fields are ignored. It adds an optional `wealth` field (0–255, default 128), which the caller fills from the settlements it serves. |
| `Crossing` | `arda-settle` crossings | `id`, `kind` (`bridge`, `ford` or `ferry`), `x_m`, `y_m`, `width_m`, `order` and `road_class`. |
| `RiverChannel` | terrain or water stage | A centreline polyline in metres, `width_m` and `depth_m`. Squares whose centres lie within half the width of the smoothed centreline are water. |
| `Terrain` | trait | `height_m(x, y)`, `slope(x, y)` (defaults to a central difference), `channels()`, and optionally `rivers_rasterised()`/`river_water(gx, gy)` and `wet_ground(gx, gy)`. `FnTerrain` wraps a closure. |

Ids are read from either a JSON number or a string and written as strings
(vocabulary I5). Road classes use the canonical codes `none = 0`,
`track = 1`, `road = 2`, `highway = 3` and `footpath = 4` (I3). Use
`RoadClass::hierarchy()` to compare importance, not the code.

## Geometry and seams

- One square is exactly `100/64 m` (`SQUARE_M`), so a 100 m cell is a 64 × 64
  block (I2). The window origin must lie on this lattice, otherwise
  `WaysError::UnalignedOrigin` is returned. Square centres are binary
  fractions, so two windows compute bit-identical world positions for a
  shared square.
- Each polyline becomes quadratic B-spline pieces: a straight lead-in to the
  first edge midpoint, one quadratic per interior vertex and a straight
  lead-out. Each piece depends only on three vertices, and it is sampled at
  a fixed step derived from its own geometry. The curve is therefore the
  same in every window, and a road leaves one block exactly where it enters
  the next (goal 46).
- The whole plan (ways, junction snapping, switchbacks, crossing spans and
  toll-house sites) is computed in world space from world data alone. The
  plan is then rasterised on a grid with a three-square apron, so decisions
  that depend on neighbouring squares, such as retaining walls, agree across
  the border. Props anchored up to three squares outside the window are
  kept, so a sprite that straddles the border is drawn on both sides.
- A window plans only the roads that can shape it (`plan::relevant::relevant_roads`):
  roads whose vertices come within 500 m of it (the 400 m densify margin,
  the widest switchback and a snap), then, to a fixed point, every road
  passing within 64 m of a kept road's endpoint, since it may snap that
  endpoint. The result is byte-identical to planning every road, so a
  window's cost follows its neighbourhood, not the world's network. The
  report's `switchbacks`, `over_grade` and `junctions` count only ways and
  junctions near the window.
- The tests check this directly. A 128-square window equals its two
  64-square halves exactly: squares, sidecar, walls and placements. This
  holds both east–west (across a bridge) and north–south (along
  switchbacks).

## Rules

**Widths (I4).** The travelled surface is highway 5, road 4, track 2 and
footpath 1 squares. The test is half-open, `-W/2 < d ≤ W/2` from the
centreline, so a straight road is exactly W squares across wherever it
falls on the lattice.

**Bands**, from the centreline outward:

| Band | Width | Highway | Road | Track | Footpath |
|---|---|---|---|---|---|
| surface | as above | cobbles when wealth ≥ 96, else gravel; worn gravel edges | cobbles when wealth ≥ 170, gravel when ≥ 60, else dirt with mud | dirt with mud wheel ruts in 4 m stretches; mud near water | dirt, mud near water |
| verge | 1 / 1 / ½ / 0 squares | worn grass (35 % dirt) | same | same | none |
| ditch | 1 square | mud at −1 ft, with 1-ft pools only on wet ground (`Terrain::wet_ground`); difficult terrain | none | none | none |
| shoulder | 2 squares | elevation graded from the road down to the ground; a cut of 4 ft or more becomes `scree` (difficult terrain) | same | same | same |

Where two ways overlap, the band with the higher priority wins (surface,
then verge, ditch and shoulder), and ties go to the higher class. A track
therefore runs through a highway's ditch at a junction. Trees and rocks
from earlier stages are removed from every way square.

**Profile and switchbacks.** Off switchbacks, the road follows the ground at
its centreline. A piece steeper than its class allows (highway 8 %, road
10 %, track 14 %, footpath 25 %) gets a rounded triangle wave across the
slope. The search takes the fewest legs first, which keeps the legs as far
apart as possible, and then the narrowest amplitude (at most 36 m) that
brings the sustained grade under the limit. Heights along the wave rise
linearly in arc length, so the grade is constant. Across the road the
surface is level at the centreline height, which cuts a bench into the
hillside. Any drop of 4 ft or more off the surface or verge gets a
`drystone` retaining wall.

**Elevation (I20).** Every `elevation_ft` and `deck_elevation_ft` this crate
writes is absolute feet above sea level, rounded to 5-ft steps. Adjacent
road squares differ by 0 or 5 ft.

**Junctions and furniture.** When a way ends on another way's interior, its
end is snapped onto that way's curve and a `prop.signpost` is placed
opposite the branch. Highways and roads get a `prop.milestone` at every
kilometre of arc length, beside the verge.

**Rasterised rivers.** A `Terrain` whose `rivers_rasterised()` is true
supplies its own river water square by square (`river_water(gx, gy)`;
arda-blocks passes arda-refine's refined channels). Its channels then only
guide crossings (flow direction, width, depth) and paint no water. Every
stretch where a way's centreline runs over that water gets a crossing
(`plan::wet`): the nearest river crossing record within 120 m lends its id
and kind, otherwise highways and roads bridge and tracks and footpaths
ford (ids with bit 62 set; no toll house). The span may move up to three
rows across its axis to the narrowest water, covers all water in its rows
(dry gaps up to five squares included), and a road never paints over the
raster's water elsewhere. Crossings of open water (ferries) are laid from
their records as below.

**Standing water.** Squares the input layout already holds as water (a
lake or the sea from arda-refine, any `water_depth_ft > 0`) keep their
depth, ground and elevation: no road band claims them, no channel repaints
them, and a crossing whose foot lies in them gets no synthetic river (its
synthetic channel only orients it). Squares outside the window are dry for
this rule, so it depends only on each square's own water and seams agree.

**Crossings.** Each crossing is matched to its nearest way (the same class
is preferred) and its nearest channel. When no channel is given, a
straight synthetic channel is made perpendicular to the road. The span is
axis-aligned, on the axis nearest the river's normal, so that parapets lie
on square edges. The way is bent onto the span by Hermite blends. Arc
lengths outside the blend do not change, so no rut or milestone moves.

- **Bridge.** The deck is four squares for a highway, three for a road and
  two for a track or footpath. It covers every water square in its rows,
  plus two abutment squares on each bank. Highways, and roads with wealth
  of 80 or more, get stone bridges: a `flagstone` abutment, a
  `prop.bridge_deck_stone` tile on each square over water and `stone`
  parapets. Other bridges are timber: `planks` abutments, two-square
  `prop.bridge_deck` strips and `timber` parapets. The deck stands at the
  higher bank plus 0.3–0.6 m. The water stays in `Square.water_depth_ft`,
  and the sidecar marks the square `deck` with a `deck_elevation_ft`.
- **Ford.** Across the road width, the water becomes 1–2 ft deep (2 ft in
  the centre rows) over `gravel`. The squares are difficult terrain. Up to
  three squares either side, near the banks, 70 % of squares become dry
  gravel bars, and the squares next to the ford are at most 3 ft deep. A
  `prop.marker_post` stands every third square along both edges, and the
  track approaches through mud.
- **Ferry.** Each bank has a landing stage of up to 3 × 3 `prop.dock_planks`
  (marked `deck` in the sidecar). A `prop.ferry_rope` line crosses the
  river one square beside the landings, with a `prop.marker_post` on each
  bank, and a `prop.ferry_boat` is moored off the first landing.
- **Toll house or waystation.** These stand at major bridges: any highway
  bridge, or any bridge over a river 18 m or wider. The building is 6 × 5
  squares on a bank, beside the road beyond the verge and ditch, with a
  three-square yard. It has walls in the bridge's kit, a door facing the
  road, windows, a floor (`flagstone` or `planks`) and furniture: a desk
  (`table`), a `bench`, a `bed`, a `chest`, a `hearth`, a `cupboard`,
  stores, and a lit `prop.lantern` by the door. The site is the first
  clear one among four candidates (either bank, either side), in a
  seed-hashed order. Clear means dry and away from every road band.

## Sidecar (`Sidecar`, format 1)

```json
{ "format_version": 1, "layout": "…", "width": 64, "height": 48,
  "squares": [ { "difficult": false, "water_depth_ft": 9, "cover": "none",
                 "blocks_sight": false, "blocks_movement": false, "deck": true,
                 "deck_elevation_ft": 70, "feature": "bridge", "road_class": "highway" }, … ],
  "edges": [ { "x": 33, "y": 22, "axis": "horizontal", "role": "parapet",
               "blocks_movement": true, "blocks_sight": false }, … ] }
```

- `squares` is row-major and matches the layout's size.
- The first six fields mirror `arda-scene`'s `RulesSidecar` names (I9).
  `difficult` covers fords, ditches and cut faces, and any undecked water
  under 5 ft, which is wading. Water of 5 ft or more is swimming (I15).
- `deck_elevation_ft`, `feature` and `road_class` are specific to this
  crate and are omitted when empty. The features are `road`, `ruts`,
  `verge`, `ditch`, `shoulder`, `cut_face`, `bank`, `bridge`, `abutment`,
  `ford`, `gravel_bar`, `landing`, `ferry_rope` and `building`.
- `edges` holds the rules for each wall segment this stage added:
  - parapets and retaining walls block movement but not sight;
  - building runs block both;
  - windows block movement only;
  - doors block neither.

## Fallbacks

Layouts use the canonical vocabulary keys. `fallback::degrade` rewrites any
key the library lacks to the nearest key it has, fixes the rotation and
mirroring to what the substitute allows, and returns a log. The substitute
chains are:

- **ground:** `flagstone` → `stone_floor`, `scree` → `gravel`, `scrub` →
  `grass`;
- **kits:** `drystone` → `stone`;
- **props:** `bridge_deck_stone` → `dock_planks`, `signpost` and
  `marker_post` → `fence`, `ferry_rope` → `fence`, `ferry_boat` →
  `rowboat`, `milestone` → `veg.stones`, `hearth` and `lantern` →
  `brazier`, `cupboard` → `chest`, `chair` → `bench`;
- **vegetation:** `tree_pine` → `tree_birch`, `rock_large` → `boulder`,
  `rock_small` → `stones`.

Unknown ground falls back to `dirt`. A placement with no substitute is
dropped and logged.

Keys this crate adds to the vocabulary: `prop.milestone`,
`prop.marker_post`, `prop.ferry_rope`, `prop.ferry_boat`,
`prop.bridge_deck_stone` (a one-square paving tile), and the functions
`toll_house` and `waystation`.

## Example

```sh
cargo run -p arda-ways --example crossings --release
```

The example builds four synthetic windows: `highway_stone_bridge` (with a
track junction and a toll house), `track_ford` (with a footpath),
`river_ferry` and `mountain_switchbacks`. It degrades them to
`assets/tactical/placeholder` and renders them at 64 px per square with the
grid. For each window it writes `out/ways/<name>.png`, `.layout.json`
(before degrading) and `.sidecar.json`. The fallback log goes to
`out/ways/report.json`.

## Open

- Bridges are single-level decks. Piers, arches seen from below, and
  bridges crossing other ways are not modelled.
- The native sidecar above is this crate's own format 1, kept for its
  debug output. `apply_ways` also returns `out.rules`, the same rules as
  `arda-scene`'s `RulesSidecar` format 2 (`Sidecar::to_rules`): squares
  no way touches state nothing, the shared fields map one to one,
  `feature`, `road_class` and `deck_elevation_ft` go in each cell's `ext`,
  and parapets and retaining walls become `edges` that block movement,
  not sight, with half cover. `apply_ways` sets the layout's `origin` to
  the window's world square.
