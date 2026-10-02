# arda-fields

Tactical land use around settlements (goals 39 and 43). This crate turns the 100 m land-use raster around a settlement into 5-ft battle-map detail:

- open-field strips near villages;
- enclosed fields behind hedgerows, drystone walls or hurdle fences, each with a gate;
- pasture with sheep and cattle, hay meadows, fallow and orchards;
- woodland with a scrub fringe;
- farmsteads with a lane to the road;
- watermills with a leat and wheel;
- mines and quarries with spoil heaps.

The output is an `arda_tactical::TacticalLayout` and an SRD rules sidecar.

```rust
use arda_fields::{fields_window, generate, FieldInputs};

let layout = fields_window(&inputs, [x_m, y_m], 64, 64, seed)?;   // layout only
let win = generate(&inputs, [x_m, y_m], 64, 64, seed)?;           // layout + sidecar
std::fs::write("rules.json", serde_json::to_string(&win.rules)?)?;  // arda-scene RulesSidecar format 2
```

Run the example to render the synthetic scenarios:

```sh
cargo run --release -p arda-fields --example countryside [-- --ppsq 16]
```

It writes `out/fields/{village_strips,hedge_country,orchard_farmstead,watermill,quarry}.png`, with `.json` sidecars. It also writes three close-ups (`*_close.png`), `hedge_country_bare.png` (the bare placeholder library, where fallbacks are visible) and `fallbacks.json`. A debug build works too, but it is several times slower.

## Inputs (`FieldInputs`)

| field | meaning |
|---|---|
| `landuse: &dyn LandUseMap` | `class_at(cx, cy) -> Option<LandUse>` per 100 m cell. Cell `(cx, cy)` covers `[100·cx, 100·cx+100)` m. The classes are the canonical `arable, pasture, orchard, woodland, meadow, fallow, mill, mine_quarry, farmstead`. `None` means land with no use, which becomes rough grazing. `LandUseGrid` is a dense implementation. |
| `terrain: &dyn Terrain` | `sample(x_m, y_m) -> TerrainSample { height_m, water_depth_m }`. Any `Fn(f64, f64) -> TerrainSample` qualifies. |
| `culture: &str` | `dwarf`/`dwarven`/`highland` always wall in stone. `elf`/`elven`/`halfling` always keep hedges. Every other culture follows the region. |
| `region: Region` | `lowland` uses hedges, `upland` uses drystone, and `mixed` uses drystone where the slope is over 12% and hedges elsewhere. |
| `wealth: u8` | Selects building kits (stone at 150 or more), yard walls, the farmhouse floor, a stable at 100 or more, and hurdle-fenced paddocks when wealth is under 120. |
| `settlements: &[Settlement]` | Village and larger settlements get open-field strips within the disc that holds 0.8 ha per inhabitant. Every settlement's built-up core (60 people/ha for villages, 150 for towns) is left as grass for the town generator. |
| `roads: &[Road]` | Polylines in world metres, with the canonical classes `none=0, track=1, road=2, highway=3, footpath=4`. The widths are 2, 4, 5 and 1 squares. |

World coordinates are metres with y pointing south. One square is exactly 100/64 m (1.5625 m), so a cell is a 64 × 64 block. `window_origin_m` is snapped down to the square lattice. Windows can be up to 2048 squares on a side.

## How it works

Everything is a function of global square coordinates and the seed. A window is computed over itself plus a 290-square margin (`plan::MARGIN`), so every field that touches it is seen whole. As a result, gates, field ids and hedgerow trees are the same in every window that shows them. `tests/seams.rs` checks this exactly, both for overlapping windows and for four blocks against their parent.

1. **Cover** (`plan.rs`). Each square gets one cover, in this order of precedence:
   - water, from the terrain;
   - a compound;
   - a road carriageway;
   - a farm lane;
   - an apron kept clear around unwalled compounds;
   - a settlement core;
   - a field.
2. **Partition** (`partition/`). Fields come from a fixed global hierarchy (logic/17 §land-fields), so every window and the relief tiles agree:
   - land blocks are a brick lattice of 500 m tiles paired two wide, alternate rows offset, every corner a T-junction; the vertices drift with a smooth 2 km noise field, so the brickwork turns from place to place (`lattice.rs`);
   - each block is split recursively (`split.rs`): along a road that crosses a piece nearly straight, else across the piece square to its longest straight side (lengthwise for long narrow closes by the houses, along the contour on slopes), at jittered and often slightly kinked lines in old enclosure and on one surveyed bearing in planned enclosure; cuts that would leave a wedge are retried gentler;
   - a piece stops splitting at the size its ground asks for (`context.rs`): closes of 0.4 ha by the houses growing to 3.4 ha 1.4 km out, 6 ha furlongs in a village's open fields, small orchards and farmstead closes, large woods and wastes, scaled by wealth, culture and enclosure style;
   - where farmland meets wood or waste the cut that separates them wins, and the land-use raster is read through a smooth 70 m displacement, so outlines follow field edges, not 100 m cells;
   - everything is drawn in a gently warped frame, so hedges wander a few squares;
   - lanes and roads still cut parcels; connected parcels under 320 squares (0.08 ha) or thinner than 4.5 squares on average become scrubby corners.
3. **Kinds and crops** (`fields.rs`, `partition/context.rs`). A field takes the majority land use of its ground:
   - arable fields inside a village's open-field disc become strips, and the rest are enclosed;
   - farmed fields within 70 m of a river on the flat are floodplain meadow;
   - pasture on steep or remote poor ground is open common;
   - crops are ploughed 45%, stubble 35% and fallow 20%;
   - meadows are standing hay or mown, in equal shares;
   - farmstead cells split between pasture and arable;
   - mill cells become meadow;
   - mine and quarry cells become rough ground.
4. **Strips** (`strips.rs`). Each furlong is divided into strips 7–11 squares (11–17 m) wide, with a reverse-S curve and a 1-square grass balk between strips. Strips run down the slope where there is one, else along the furlong (30% of broad furlongs across it). A strip usually takes its furlong's crop, and 28% of strips take their own. Headlands are grass.
5. **Boundaries** (`boundary.rs`). Every edge of an enclosed field that faces another cover gets that field's kit, except for edges facing water or a walled compound. Where two fields meet, the field with the lower id sets the kit. Each enclosed field has one gate:
   - the gate faces a lane or road if possible;
   - otherwise it faces the nearest farm gate or settlement within 800 m;
   - it is two edges wide where the boundary runs straight.
6. **Dressing** (`dress/`). Placements come from global jittered lattices:
   - orchards are planted in rows of fruit trees, 5 squares apart;
   - woodland is a closed canopy in stands, with glades and a 3-square fringe of scrub, birch and bushes;
   - pasture has loose flocks, found by the tag queries `livestock:sheep` or `livestock:cattle`, and a trough inside each gate;
   - hay meadows have tall grass and flowers, or rows of bales;
   - stubble has stooks;
   - hedges have standard oaks and elms, at most one per 14-square cell.
7. **Compounds** (`compounds/`). Each compound is built from its own cell in a local frame, then turned and mirrored onto the lattice.
   - **Farmstead.** A 26 × 20 yard behind a wall with a double gate facing the nearest road. Inside are:
     - a furnished farmhouse with a door, windows and a hearth light;
     - a barn with hay, a cart and a ladder;
     - a stable (or a pig pen) and a fenced kitchen garden;
     - a well, trough, woodpile and poultry in the yard.
   - **Watermill.** Built on the dry bank square nearest the cell centre:
     - the stone mill house holds a millstone and sacks, with a yard and lane to the north;
     - a leat runs along its river wall, fed by an intake upstream (upstream is found from the water-surface slope);
     - the wheel sits in the leat, with a tailrace back to the river.
   - **Mine or quarry.** A cell is worked by an adit if its slope is 10% or more, and by an open quarry otherwise.
     - The quarry is a pit with a cliffed face uphill, a gravel floor, a ramp, a crane and rock props.
     - The mine is a timber-framed adit with a door in a rock face, an apron, an ore cart, pit props and a lantern.
     - Both have scree spoil heaps tipped downhill.
     - Roads win: a mine or quarry whose works would touch a road (its carriageway or edge square) moves 16 squares to the first clear site of eight around its own; if none is clear it keeps its site and gives up the squares, props and walls on the road.
   - **Lanes.** Every compound gets a 2-square lane to the nearest road within 1.5 km. `CompoundRecord.lane` is `false` if no road is in reach.
8. **Layout** (`window.rs`).
   - Ground keys come from the canonical vocabulary.
   - `elevation_ft` is absolute and rounded to 5 ft, plus earthworks.
   - Water depths come from the terrain and the leats.
   - Placements with anchors inside the window are included, along with the compound lights.

## Rules (`arda-scene` `RulesSidecar` format 2)

`generate` returns `rules`, the product's one per-square rules schema (I9), and `owned`, the row-major mask of squares this layer reserves: fields, compounds, lanes and aprons (logic/09 §reservations, precedence 4). Roads, water, built cores and wild land are left to their owners, so their cells state nothing. On owned squares the shared fields are stated in full and `crop`, `furrow` and `field` ride in each cell's `ext`. Hedges, drystone walls and fences become `edges` with role `boundary` (or `gate` where they open), carrying the SRD reading below. The layout's `origin` is the window's world square (I10).

## Sidecar (`sidecar.rs`, format 1)

The sidecar holds `name`, `seed` (as a string), `width`, `height`, `origin_square` (global) and `square_m`, plus the following.

- **`squares`**: row-major entries with these fields:
  - `ground` is the canonical key, before any fallback;
  - `difficult`, `water_depth_ft`, `cover`, `blocks_sight`, `blocks_movement` and `deck` use the `RulesSidecar` names;
  - `crop`, `furrow` (a unit vector) and `field` (the id as a string) are optional.
- **`edges`**: one entry per wall segment, with `x`, `y`, `axis`, `kind`, `kit`, `blocks_sight`, `blocks_movement`, `cover`, `climb` and `openable`.
- **`fields`**: `id`, `kind`, `crop`, `kit`, `hectares`, `enclosed`, `gates` and `complete`. `complete` means the field lies wholly inside the window (an enclosed field by its boundary, an open one by its squares).
- **`compounds`**: `kind`, `cell` and `lane`.

Cover values are `none | half | three_quarters | total`. Each kit reads as follows under the SRD:

| kit | run | gate |
|---|---|---|
| `hedge` | blocks sight and movement, ¾ cover | open: no block, half cover |
| `drystone`, `wattle` | half cover, climbable (`climb`) | open: no block, half cover |
| `stone`, `timber` (buildings) | block both, total cover; windows ¾ and block movement | doors and barn gates block until opened (`openable`) |

The following count as difficult terrain:

- ploughland, scrub, scree, rock, cliff, mud and marsh;
- water under 5 ft;
- bushes and small rocks.

Water of 5 ft or more blocks movement (you swim).

## Fallbacks and the placeholder supplement

The generator always emits canonical keys. `degrade::adapt(layout, lib)` rewrites a copy of the layout so that everything in it exists in `lib`. It walks nearest-substitute chains, for example:

- `farmland → dirt`, `stubble → farmland → sand`;
- `hedge → wattle → timber`, `drystone → stone`;
- `prop.hay_bale → prop.sacks`, `prop.waterwheel → prop.millstone → prop.crane`.

Tag queries drop their last tags, and anything with no match is dropped. Every substitution comes back as a `Fallback { kind, wanted, used, count }`.

`supplement` adds temporary generated art only where the base library lacks it: 13 grounds (recoloured placeholder textures, so they still tile), the `hedge`, `drystone` and `wattle` kits, and hay bales, troughs, millstone, waterwheel, hay cart, hearth, sheep, cow, hen and tall grass. It never replaces an asset the library already has, so a real library takes over without code changes. Everything in it is Apache-2.0.

`overlay::paint_furrows` draws plough rows across the render, using the sidecar's per-square furrow direction and global coordinates. Ground textures cannot rotate, so this is how ploughed land shows its orientation.

The example's previews render `earthworks_ft` instead of the absolute elevation. The compositor lights every 5-ft step as a cliff, which would stripe natural slopes. This substitution is recorded as a `render` fallback.

## Vocabulary extensions used (proposed for `vocabulary.md`)

- **Ground:** `stubble` and `fallow`, the crop states of ploughland.
- **Props:** `prop.waterwheel`, `prop.sheep`, `prop.cow` and `prop.hen`.
- **Vegetation:** `veg.tall_grass` is already listed.
- **Free tags:** `livestock:sheep`, `livestock:cattle`, `livestock:poultry` and `hay`.
