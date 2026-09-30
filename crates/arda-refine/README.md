# arda-refine

Tactical terrain refinement for arda worlds. Any 100 m world cell becomes a
64 × 64 block of 5-ft squares with natural ground, water, rocks and
vegetation. The same code turns any window of squares into one map that
crosses block edges (goals 42, 43, 46, 47 and 50). Settlements and roads
are out of scope.

```text
cargo build --release -p arda-refine
target/release/arda-refine render --world out/micro42 --cell 530,810 --out river.png --json
target/release/arda-refine render --world out/micro42 --cell 529,809 --window 3x3 --out window.png --json
target/release/arda-refine stats  --world out/micro42 --cell 530,810   # WFC and timing
target/release/arda-refine survey --world out/micro42                  # exemplar cells by type
```

The library API:

- `refine_block(src, cell) -> Map` builds one block. `Map` holds `layout` (the `TacticalLayout`), `rules` (the SRD sidecar) and `meta`. Use `layout_json()`, `rules_json()` or `meta_json()` to serialise them.
- `refine_window(src, x0, y0, w, h) -> Map` builds a window in global squares. The blocks it covers run on parallel threads. Each block is a pure function of its cell, so the result is the same byte for byte.
- `refine(src, cell) -> Block` returns the full internal record: corner classes, fixed decisions, the halo, items and rules.
- `WorldSource::new(&arda::World)` reads a stored world. `GridSource` holds the same data in memory and serves synthetic fixtures.

## Coordinates and conventions

This crate follows the canonical conventions in `docs/goal-prompts/vocabulary.md`:

- Cell `(gx, gy)` covers `[100·gx, 100·gx + 100)` m, and its centre is at `100·gx + 50`.
- A square is 100/64 m (1.5625 m) in world units and counts as 5 ft for play.
- The global square `(64·gx + sx, 64·gy + sy)` is square `(sx, sy)` of that cell's block. All geometry is done in global squares.
- **Fine terrain registration.** The fine layer stores each cell's height at world metre `100·gx`, which is the cell's corner under the convention above. So the refiner reads fine sample `k` at world metre `39.0625·k + 50`. With this half-cell shift, fine and coarse data line up: rivers stay in the fine valleys and block heights match cell heights. Remove the shift once the fine layer adopts the convention.
- `elevation_ft` is absolute feet above sea level, in 5-ft steps.
- Water under 5 ft is `water_shallow`: wading, and difficult terrain. Water of 5 ft or more is `water_deep`: swimming.

## Pipeline

Every step below is a function of the world seed and global position only.
A block never reads another block's output, so neighbouring blocks agree
without talking to each other, and the order in which blocks are built
never matters.

1. **Gather** (`context.rs`). The block reads a 5 × 5 cell neighbourhood, the channel edges of the 3 × 3 cells around it, and a window of the fine terrain lattice. The design names the 3 × 3 neighbours. The extra ring is needed for two things: the C1 Catmull-Rom interpolation of cell fields at the block's three-square halo, and the far ends of the neighbours' channel edges. The fine window covers every point any channel piece of those cells can reach, so no block ever reads a clamped sample that its neighbour reads unclamped.
2. **Continuous elevation** (`terrain.rs`). Catmull-Rom interpolation of the fine lattice gives the base; worlds without a fine layer fall back to cell heights. The base is sampled through a gentle rotated domain warp of 4 squares, so the iso-lines of separable interpolation never run along the grid axes. On top goes detail: three octaves of gradient noise, each rotated by a different fixed angle and scaled by slope and cover (rock is rough, marsh is smooth). The result is quantised to 5-ft contour steps.
3. **Linear features first** (`rivers.rs`, `terrain.rs`, `fixed.rs`).
   - **Rivers.** A saved channel edge `A → B` crosses the shared cell boundary at a point hashed from the pair's coordinates and the seed, within 8 squares of the edge midpoint (a diagonal step crosses at the shared corner). Inside a cell, each channel runs from its entry crossing to the cell's node and on to its exit as a cubic Hermite curve. The node is the valley floor within about 10 squares of the centre. At every crossing the curve's tangent is the direction between the two cell centres, so the channel is C1 across block edges. A bow with zero slope at both ends adds a gentle meander.
   - **Channel shape.** Width comes from the edge's `from_width_dm` and `to_width_dm`, with at least one square drawn. Depth is parabolic across the channel, with the thalweg at the centreline. The surface follows the smooth terrain under the centreline, 0.35 m below it. The banks are ramped at about 18° and blended out over 20 squares.
   - **Lakes and sea.** A Catmull-Rom field of ±1 over cells marks standing water. It is blended with the terrain's water-level contour, warped, and bent by noise, so shores never follow cell edges.
   - **Shores and cliffs.** Lake and sea shores become beach (sand), shingle (gravel) or rock, chosen by slope and rocky cover. Dry squares steeper than 48° become cliff. Marsh cells open shallow pools.
4. **Fixed borders** (`borders.rs`). The corners on a block edge are shared with the neighbouring block, so they are decided before the WFC from data that both blocks see identically: corner masks, prior scores, and the masks one corner either side of the edge.
   - Each block corner takes its best feasible class.
   - Each edge is then solved as the cheapest legal class sequence, in a Viterbi pass that always runs in increasing coordinate order.
   - Every pair of squares across a seam therefore shares its two edge corners exactly.
5. **WFC ground** (`tiles.rs`, `wfc.rs`, `prior.rs`).
   - **Weights.** The class weights come from the cell's cover, forest density, wetness, moisture, height above the river and temperature. They are interpolated between cell centres, so a density change between cells thins across the block rather than stopping at its edge. The local slope (scree, rock, cliff) and aspect (moss and snow on north faces, heath on south faces) shape them further, and a rotated noise field per class breaks them into patches.
   - **Solving.** The solver collapses the most constrained corner first, with ties broken by a hashed priority to avoid scan-order bias. It picks by weight and propagates to arc consistency.
   - **Transitions.** A corner that has lost its preferred class picks the transition class that leads back to it, rather than spreading an unwanted class.
   - **Contradictions.** A contradiction triggers a local repair of radius 4. After 48 repairs the attempt is abandoned. After 6 attempts the block takes a deterministic relaxed fill: every free corner gets its best allowed class. Any square whose corners then form no tile is flagged `review` in `meta.json` (goal 47). The solver never fails.
6. **Scatter** (`scatter.rs`). Each layer is a Matérn type-II hard-core process on a global jittered lattice: one candidate per lattice cell with a hashed mark, kept when no other candidate within the layer's spacing has a smaller mark. This gives Poisson-disk spacing that depends only on global position, so layers continue across blocks. Density fields then thin the kept points independently, which keeps the minimum spacing and makes the tree count proportional to `forest_density`. The layers are:
   - large and small trees (species by temperature and nearness to water: oak, elm, birch, willow, pine, spruce, dead);
   - undergrowth;
   - boulders and rocks;
   - fallen logs and stumps;
   - reeds and cattails;
   - lily pads;
   - low plants.
7. **Rules** (`output.rs`, `rules.rs`). See the SRD sidecar below.

## Tile vocabulary

There are 19 ground classes: water, grass, meadow, forest floor, leaf litter, heath, scrub, moss, scree, rock, cliff, sand, gravel, mud, marsh, reed bed, snow, ice and dirt. The classes label square **corners**, so a tile is fixed by its four corner classes. This is a corner (Wang) tile set.

| Tiles | Count |
| --- | --- |
| Pure: one class on all four corners | 19 |
| Pair transitions: the 14 two-class corner patterns (straight edges, outer and inner corners, diagonals) for each of the 72 natural pairs in `classes::PAIRS`, e.g. water–sand (beach), water–mud (bank), forest floor–leaf litter (forest edge), scree–cliff, marsh–reed bed | 1,008 |
| Junctions: the 36 patterns using all three classes of each of the 95 mutually compatible triples, where three kinds of ground meet | 3,420 |
| **Total** | **4,447** |

Two tiles may sit side by side when the two corners on their shared edge
match. A grass-to-water transition is therefore a tile of its own, with
water corners on one side and grass corners on the other. Water squares
may carry bank classes on their corners (the shallows transitions). Land
squares next to water take bank classes on their water side (the bank
transitions).

A square's ground key comes from its tile:

- a water square takes `water_shallow` or `water_deep` by depth;
- a cliff square takes `cliff`, and a shore square takes its shore class;
- an open square takes its dominant corner class. The most common non-water corner wins, and a tie goes to the more tactically significant class (`Class::rank`).

Every class maps to a vocabulary ground key (`Class::ground_key`).

## Edge rules

- Squares are decided by global functions of position: elevation, water, depth and fixed features. The block also stores a one-square halo, so tests can confirm that each neighbour computes the same values.
- Border corners are fixed by the shared-edge Viterbi pass before the WFC.
- River crossings depend only on the two cells' coordinates and the seed. Tangents at crossings are the centre-to-centre direction.
- Scatter comes from global hard-core processes, and every item belongs to the block that contains its anchor. Canopy and cross-layer spacing checks read a two-square halo.

## Output schema

Each map is written as three files.

- **`*.layout.json`** is exactly `arda-tactical`'s `TacticalLayout` (`crates/arda-tactical/src/layout.rs`, branch `feat/tactical-catalogue`), mirrored in `layout.rs` without depending on that crate. `Square` holds only `ground`, `elevation_ft` and `water_depth_ft`, from the vocabulary. Placements use exact vocabulary ids (`veg.tree_oak`, `veg.fallen_log`, `veg.reeds` and so on) with rotation 0, 90, 180 or 270. Positions are fixed-point multiples of 1/64 square. `walls` and `lights` are empty.
- The layout is `arda-tactical`'s `TacticalLayout` itself, with `origin` set to the world square of its top-left square, so the compositor hashes world coordinates and neighbouring blocks' art joins (I10).
- **`*.rules.json`** is `arda-scene`'s `RulesSidecar` itself, format version 2 with the canonical field names. It has the same width and height as the layout, row-major, one cell per square, and every field is stated explicitly:
  - `difficult`: undergrowth, scrub, scree, mud, marsh, reed bed, snow, ice, cliff (climbing), logs, or wading water;
  - `water_depth_ft`;
  - `cover`: `half` for a slender trunk or large rock, `three_quarters` for a thick trunk or boulder;
  - `blocks_sight`: four or more large trunks within 2.4 squares, which counts as dense canopy;
  - `blocks_movement`: always false for natural ground;
  - `deck`: never set.
- **`*.meta.json`** is this crate's own review record (format `arda-refine.meta/2`): the name, the size, the relaxed flag, the `review` square indices, and per-block WFC attempts and repairs. It also holds `placements`, running parallel to the layout's placements, each with its semantic tag (`tree:broadleaf:large`, `rock:boulder`, `water_plant:reeds` and so on), cover and difficulty.

## Tests

- `determinism`: identical bytes on repeated runs; generation order and window use change nothing.
- `seams`: in 3 × 3 windows over the river, the lake, the coast and the hill, the elevation, water depth and corner lines agree exactly with independently generated neighbours, and the river crosses on both sides.
- `agreement`: block mean height and slope follow the cells; water appears where the cells say and nowhere else; the tree count is proportional to forest density.
- `wfc`: every pairing is a legal tile after a normal fill; a forced contradiction is relaxed, flagged and deterministic.
- `bias`: the gradient-orientation histograms and directional variograms of the elevation detail, and the orientation of ground borders, show no axis preference.
- `perf`: a block under 250 ms and a quarter block under 100 ms. In release they take about 30 to 40 ms.
- `schema`: the output parses into verbatim copies of the consumer structs.
- `world_adapter`: a world stored on disk refines exactly like the same data held in memory.
- `real_world` (ignored; needs a generated world in `ARDA_REFINE_WORLD`): agreement and seams on the fixture.

The synthetic fixture in `tests/common` is a 16 × 16 cell world. It has a forest with a density gradient, a river with a tributary, a lake, a sea coast, a marsh and a rocky hill, all over an analytic fine lattice.

## Determinism

The output is a pure function of the seed and global coordinates. All
randomness is hashed from them (`hash.rs`). Floating-point work uses only
IEEE-754 `+ - * /`, `sqrt` and `floor`, with no platform `libm`: `exp`,
`ln` and `atan` are implemented in-crate. The output is therefore
bit-identical across platforms. Stored placement coordinates are
fixed-point.

## Known limits

- The fine layer from recipe 5 keeps sub-cell hollows and glacial troughs that the coarse cell height, a point sample, does not show. In seed 42, cell 752,798 has a trough reaching −58 m next to a cell at +17 m. Blocks follow the fine terrain, so a few cells (4 of 162 sampled) sit outside the height tolerance, and channels may bend around such troughs.
- Standing-water shores follow the coarse water cells, bent by noise and by the terrain's water-level contour. A land cell next to the sea is therefore mostly land with a beach at its edge, as its cell says.
- The debug render is flat colour for checking only. The art comes from `arda-tactical`.
