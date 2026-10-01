# arda-scene

Game-facing scene data for a tactical map (goals 48 and 65–69). The VTT draws
nothing itself: it gets a painted image from `arda-tactical` and this scene
JSON, and both come from the same `TacticalLayout`, library and seed, so they
always agree.

```rust
let scene = arda_scene::build_scene(&layout, &library, seed, sidecar.as_ref())?;
let json = scene.to_json()?;                 // compact
let png = arda_scene::scene_debug_png(&scene, 48)?;
```

`seed` must be the seed the image is rendered with. Tag queries in the layout
resolve exactly as `arda_tactical::render` resolves them.

`cargo run -p arda-scene --example debug` writes a debug PNG and a JSON file
for each `arda_tactical::layouts` layout to `out/scene/`, plus
`riverside_sidecar`, which shows a rules sidecar in use.

## Conventions

- Units are 5-ft **squares**. The origin is the map's top-left corner and y
  grows south, as in `TacticalLayout`.
- A square `(x, y)` spans `x..x+1 × y..y+1`. Grid vertices are integer
  `[x, y]` pairs, and a square reference `Sq` is written `[x, y]` too.
- Per-square layers are row-major, `width × height` long, and **run-length
  encoded** as `[[count, value], ...]`. A 64 × 64 all-normal layer is
  `[[4096, "normal"]]`.
- 64-bit seeds and ids are JSON **strings** (conventions I5, I17). Input
  also accepts a plain number for the seed.
- Enums are `snake_case` strings. Map edges are written `N`, `E`, `S` and `W`.
- `to_json` writes minified JSON. `to_json_pretty` writes the same document
  indented. `Scene::from_json` also checks that every layer has
  `width × height` squares.

## Scene (format_version 1)

| field | type | meaning |
|---|---|---|
| `format_version` | `u32` | `1` |
| `name` | string | layout name |
| `width`, `height` | `u32` | size in squares |
| `seed` | string | u64 seed the asset queries were resolved with, as a decimal string (convention I17) |
| `library`, `library_version` | string | art library identity; the cache key with the seed and coordinates |
| `movement` | `Grid<Movement>` | per-square movement |
| `climb` | `Grid<u8>` | per-square climb mask (see below) |
| `cover` | `Grid<CoverLevel>` | cover granted by what stands in the square |
| `obscured` | `Grid<Obscurement>` | vision obscurement of the square |
| `elevation_ft` | `Grid<i16>` | elevation in feet |
| `water_depth_ft` | `Grid<u8>` | water depth in feet, after the sidecar |
| `walls` | `Wall[]` | wall, door, window, gate and secret-door polylines |
| `vision_blockers` | `VisionBlocker[]` | canopy and prop polygons |
| `lights` | `SceneLight[]` | light sources |
| `regions` | `Region[]` | merged difficult-terrain and water areas |
| `spawn_hints` | `SpawnHints` | token placement hints |
| `tokens` | `TokenSlot[]` | NPC token slots (goal 48); empty by default, may be omitted on input |

### Enums

- `Movement`
  - `normal`
  - `difficult`
  - `wade`: water 1–4 ft deep, treated as difficult terrain
  - `swim`: water 5 ft or deeper
  - `impassable`: a prop that blocks movement
- `CoverLevel`: `none`, `half` (+2 AC and Dex saves), `three_quarters` (+5),
  `total` (can't be targeted directly). Input also accepts `full` as an alias
  for `total`.
- `Obscurement`
  - `clear`
  - `light`: lightly obscured, as by moderate foliage or a tree canopy
  - `heavy`: blocks vision; this is what stops line of sight
- `WallKind`: `wall`, `door`, `window`, `gate`, `secret`.

### climb

Bit `d` is set when stepping to the neighbour in direction `d` crosses an
elevation step of 10 ft or more. The bit order is 0 N, 1 NE, 2 E, 3 SE, 4 S,
5 SW, 6 W, 7 NW. Both squares of a step carry the flag, so it applies going
up and going down.

### Wall

```json
{"kind":"door","open":false,"points":[[10,3],[10,4]],
 "blocks_sight":true,"blocks_movement":true,"blocks_light":true,
 "cover":"total","kit":"timber"}
```

- `points` are grid vertices, at least two. Plain walls and windows are
  merged into maximal collinear runs of identical attributes. Doors, gates
  and secret doors are one edge each, so each can be toggled.
- `open` is present only on doors, gates and secret doors. Layouts carry no
  door state, so everything starts closed.
- The flags describe the current state:
  - a closed door, gate or secret door blocks sight, movement and light, and
    gives `total` cover;
  - an open one blocks nothing;
  - walls and windows take their flags from the kit's catalogue piece
    (`blocks_sight`, `blocks_movement`, `cover`), and `blocks_light` is set
    to `blocks_sight`;
  - the placeholder window blocks movement, not sight, and gives
    `three_quarters` cover to a creature behind it.
- `secret`: the layout schema has no secret role yet. The doors of a kit
  whose door piece carries the free tag `secret` become secret doors.
- `Scene::set_open(index, open)` toggles a door. Rebuild any `SceneIndex`
  afterwards.

### VisionBlocker

```json
{"kind":"canopy","asset":"veg.tree_oak","obscurement":"light",
 "polygon":[[0.17,10.1],[1.43,10.1], ...]}
```

- `kind` is `canopy` or `prop`.
- The polygon is closed, in squares, and clockwise in y-down coordinates.
  - A canopy is an octagon inscribed in the tree's footprint. It is present
    for every canopy-layer asset.
  - A prop is its rotated footprint rectangle. It is present only for props
    that block sight.
- `obscurement` is `heavy` when the asset blocks sight and `light`
  otherwise.

### SceneLight

```json
{"x":6.5,"y":10.6,"bright_ft":20,"dim_ft":40,"colour":[255,176,88],"asset":"prop.brazier"}
```

- `dim_ft` is the outer radius of the dim light. As with the SRD's torch,
  lantern and candle, dim light extends beyond the bright radius by the
  bright radius again, so `dim_ft = 2 × bright_ft`. The SRD lamp (15 ft
  bright, 30 ft more dim) does not follow that rule; there is no lamp asset,
  and one would need a light with two radii.
- `asset` is missing for free layout lights.
- Free lights come first, then emissive assets in placement order.

### Region

```json
{"kind":"shallow_water","rings":[[[5,0],[6,0],[6,2],[5,2]]]}
```

- `kind` is `difficult`, `shallow_water` or `deep_water`. It mirrors the
  `movement` layer, so squares under a floor asset such as a bridge or dock
  are not water.
- Rings are closed; the first point is not repeated. Only turning points are
  kept.
- Fill rings with the even-odd rule. Outer rings run clockwise in y-down
  coordinates and holes run anticlockwise. Squares that touch only at a
  corner form separate rings.

### SpawnHints

- `open`: squares, row-major, that are normal, have no cover and are clear,
  and whose eight neighbours are the same and reachable in one step.
- `entrances`: `{ "wall": <index into walls>, "squares": [[x,y], ...] }`, one
  for each door or gate, listing the enterable squares on both sides. Secret
  doors are left out: they look like wall until found.
- `exits`: `{ "edge": "N", "from": [x,y], "to": [x,y] }`, runs of enterable
  border squares, inclusive, with no movement-blocking wall on the map edge.
  They are listed in N, E, S, W order and are meant for seamless travel to the
  neighbouring map.

### TokenSlot

```json
{"npc_id":"9007199254740993","x":3.5,"y":7.5,"facing":90}
```

- `npc_id` is the NPC's u64 id as a decimal string. The game fetches the
  sheet by it (goal 69).
- `x`, `y` are the token centre in squares; `facing` is degrees clockwise
  from north (default 0).
- `build_scene` leaves `tokens` empty. The NPC stage fills it.

## How per-square rules are derived

1. The layout gives `elevation_ft` and `water_depth_ft`.
2. Each placement is resolved to its asset, and its footprint is rotated and
   mirrored exactly as the compositor does it. A square is covered when its
   centre lies in the footprint.
   - A **canopy** asset gives its whole footprint `light` obscurement, or
     `heavy` if it blocks sight. Only its trunk square (the anchor's square)
     takes its movement block and its cover.
   - A **floor** asset (dock, bridge) marks its squares as bridged. They are
     walked, not waded or swum.
   - Any other asset applies `blocks_movement` (impassable),
     `difficult_terrain`, `cover` (the strongest wins) and `blocks_sight`
     (`heavy`) to every covered square.
3. The sidecar is merged in (see below). It can also set or clear the
   movement block and the deck.
4. `movement` is decided in this order: impassable, then swim (depth of 5 ft
   or more), then wade (1–4 ft), then difficult, then normal.

## RulesSidecar (format_version 2)

`Square` is `deny_unknown_fields`, so per-square rules from the terrain
generator travel beside the layout in their own JSON. The sidecar has the same
width and height as the layout and its cells are row-major
(`docs/goal-prompts/vocabulary.md`). This crate owns the one per-square
rules schema (convention I9); other crates mirror its field names. Every cell
field is optional. A missing
field keeps the value derived from the layout and its assets.

```json
{"format_version":2,"width":2,"height":1,"squares":[
  {}, {"difficult":true,"water_depth_ft":6,"cover":"half","blocks_sight":true,"blocks_movement":false,"deck":true}]}
```

| cell field | type | merge |
|---|---|---|
| `difficult` | bool | overrides: `true` sets difficult terrain (undergrowth, scree, mud); `false` clears what props implied |
| `water_depth_ft` | u8 | overrides the layout's depth |
| `cover` | CoverLevel | takes the stronger of the sidecar and prop cover, since only the most protective degree applies |
| `blocks_sight` | bool | overrides: `true` makes the square `heavy` (dense canopy); `false` clears asset-derived `heavy` to `clear` (canopy `light` stays) |
| `blocks_movement` | bool | overrides: `true` makes the square `impassable` (cliff face, thicket); `false` clears prop blocking |
| `deck` | bool | overrides: `true` means a walkable bridge deck or jetty spans the square, so it is walked, not waded or swum; `false` removes a layout floor asset's deck |
| `lightly_obscured` | bool | `true` raises a `clear` square to `light` (moderate foliage, reeds); never lowers `heavy` |
| `ext` | object | layer-specific extras the scene passes through uninterpreted: arda-ways writes `feature`, `road_class` and `deck_elevation_ft`; arda-fields `crop`, `field` and `furrow`; arda-town `building` |

Top level, `edges` (optional, omitted when empty) lists rules for low edge
features that are not wall-kit walls: `{x, y, axis, role, blocks_movement,
blocks_sight, cover}` with `role` one of `parapet`, `retaining`, `building`,
`boundary` (hedges, drystone walls) or `gate`. An edge rule overrides the
blocking and cover of the layout wall on the same unit edge, unless that
wall opens (doors and gates keep their closed state). A bridge parapet
therefore blocks movement but not sight.

Several owners' sidecars combine with `RulesSidecar::overlay(upper, owned)`
in rising reservation precedence (logic/09 §reservations): on owned squares
each field the upper sidecar states wins, `cover` takes the stronger,
`difficult` is true if either sets it, and `ext` maps are unioned.

Water depth follows convention I15: under 5 ft is wading (difficult
terrain), 5 ft or more is swimming. Format 1 is refused.

A sidecar whose size or version doesn't match is refused with
`SceneError::Sidecar`. An all-empty sidecar changes nothing.

## Queries (server-side validation)

`scene.queries()` builds a `SceneIndex`. Reuse it for many queries. `Scene`
also has one-shot wrappers with the same names. The game may reimplement all
of these from the JSON.

- `line_of_sight(a, b)` runs from centre to centre, with exact integer
  geometry at 8 units per square.
  - Crossing a wall edge that blocks sight blocks the line.
  - Entering a `heavy` square that is not the target blocks the line.
  - A line that passes exactly through a grid vertex is blocked only when
    sight blockers lie on **both** sides of it. The blockers counted are
    wall arms at that vertex and `heavy` squares beside the line.
  - So a line may graze a wall end or the outside of a corner, but it cannot
    slip through a closed corner or between two diagonally touching opaque
    squares. This vertex rule is a house rule: SRD 5.1 has no grid
    line-of-sight procedure.
- `visible_squares(from, radius_ft)` returns the squares within
  `distance_ft` of `from` that `from` can see, row-major, `from` included.
- `path_cost(a, b)` and `shortest_path(a, b)` apply the SRD 5.1 movement
  rules:
  - Each square entered costs 5 ft, and a diagonal step counts as one
    square: every diagonal costs 5 ft. The 5/10 ft alternating diagonal is
    a DMG variant, not SRD 5.1 content, so it is deliberately not offered.
  - Entering `difficult`, `wade` or `swim` squares costs 1 extra foot per
    foot.
  - A flagged climb step costs 1 extra foot per foot. The extras come from
    separate rules, so they add up.
  - Impassable squares and movement-blocking edges stop a step.
  - A diagonal may not cut a vertex that any movement-blocking wall touches,
    and may not squeeze between two impassable squares. Both are house
    rules; SRD 5.1 does not cover grid corners.
  - `distance_ft(a, b)` measures range the same way: 5 ft × the larger of
    the two axis differences.
- `cover_between(attacker, target)` applies the SRD degrees of cover, read
  off the grid with the DMG's optional four-corner procedure, used here as a
  house rule (SRD 5.1 gives no grid procedure).
  - The attacker uses the corner of its square that sees the target best.
    Lines run from that corner to the target square's four corners, each
    inset by 1/8 square.
  - All four lines blocked gives `total`, three gives `three_quarters`, and
    one or two gives `half`.
  - Obstacles along the open lines add their own cover. These are the
    `cover` of squares strictly between the two creatures and walls that
    don't block sight, such as windows.
  - Only the most protective degree applies. A creature's own square gives
    it no cover.

## Debug view

`scene_debug_png(&scene, ppsq)` returns PNG bytes. `scene_debug_image`
returns the raster. The legend is in `src/debug/mod.rs`:

- square fill shows movement;
- red pips show cover;
- green or grey tints show obscurement;
- red edge bars show climbs;
- coloured wall lines show the wall kind;
- yellow circles show bright light and dashed orange circles show dim light;
- teal marks show entrances and exits.

## Notes for the TypeScript side

Every DTO derives `serde`. Most types map directly to TS. The exceptions are:

- `Grid<T>`, which serialises as `Array<[number, T]>` and needs a hand-written
  ts-rs override (`#[ts(type = "Array<[number, T]>")]`);
- `Sq`, which is `[number, number]`;
- `seed`, which is a `string` on the wire (`#[ts(type = "string")]`).

Rust is the source of truth. Bump `SCENE_FORMAT_VERSION` on breaking changes.
