---
generated_date: 2026-08-24
scenario: block-generation
traces: [Q12, Q13]
artifact: ../mockup-artifact.md
---

# 03 — Block generation

Third batch stage: materializes the tactical layer as tile-IDs (mockup
Q22). Mechanism is the artifact's `Inside a cell: the D&D grid` section,
confirmed verbatim; this file adds the §Q12 parameters. A block is
64×64 five-ft squares (320 ft; the cell's "100 m" is nominal 97.5 m —
§Q12). An ordinary battle map is a quarter block.

## Trigger & preconditions

- Trigger: batch step [3/3]; one block per land/coast cell, any order (blocks are independent).
- Preconditions: the block's cell and its 8 neighbors exist in `areas/` (the only inputs — a block never sees anything larger, per the artifact).

## Steps

1. **Constraints from the cell** (artifact, binding): ground kind from cover; tree count from forest density (thinning across the block when neighbor density differs); contour steps from slope/aspect (hillside = terraces, valley floor = flat); pools/reeds/soft ground from height-above-river; water band of the cell's width/depth, hop-vs-swim by order; road band width by class; crossing record → bridge/ford/ferry-landing tiles; pass → road on highest ground; settlement footprint → buildings at tier density, church + inn + market at the centre cell, road becomes street; field hedges/walls on cell boundaries.
2. **Edge fixing** (artifact, binding): every linear feature's entry/exit square is computed from the two cells' coordinates + seed alone; both sides of an edge compute the same point. Ground-cover changes straddle the boundary as transition zones.
3. **WFC fill** (§Q12): tile set of 200+ tiles (semantic kinds plus variants — species, wear, furniture; transitions are explicit tiles, WFC cannot blend). Most-constrained-square-first with propagation; fixed entry/exit tiles; required paths connecting them per river/road; count constraints for trees and buildings; adjacency rules for the rest.
4. **Contradiction handling** (§Q12): refill with border kept, subseeded per attempt `(seed, cell, attempt)`, ≤8 attempts; then a greedy constraint-relaxed fill that cannot fail, and the block is marked `relaxed` in metadata. The batch can never die inside WFC.
5. **Write**: 2 bytes/square tile-IDs into the area's block archive, zstd-compressed (~0.4–0.8 GB per area compressed; ~80–160 GB continent default — the doubling over the mockup's 1-byte estimate was accepted at §Q12).

## Branches

- Cell kind selects the constraint bundle (step 1); sea cells get no block (mockup 02).
- Attempt count selects WFC vs relaxed fill (step 4).

## Unhappy paths

- WFC contradiction: bounded, deterministic, self-healing (step 4); `relaxed` marks make imperfect blocks findable.
- Missing neighbor (map corner/edge tiles): the continent guarantees an ocean rim, so boundary blocks border sea cells — constraints exist for all 8 neighbors always (01 invariants).
- Interrupt: restart; identical output (determinism).

## State transitions

None — appends blocks to `blocks/<ax>_<ay>.tiles.zst`; never mutates cell or object data.

## Invariants

- Same (seed, cell coords) → same block, every time it is drawn (artifact).
- Linear features are continuous across every shared edge; a road or river followed 20 km never breaks (artifact).
- A block reads only coarse data — never a neighbor's finished squares (artifact).
- Tile count and adjacency constraints hold except in `relaxed` blocks, which are explicitly marked (§Q12).

## Outcomes & side effects

- Success: complete block archives per area; batch summary reports relaxed-block count (assumed reporting); world is complete (mockup 02 "Complete" state).
- Failure: none in-scenario beyond process death; restart regenerates.

## Amendments (build gate, 2026-08-25)

- **Tile attributes** (`../build-interview.md` §Q5): every tile kind in the vocabulary carries static attributes — material, traversable (yes/difficult/no), movement cost, cover, hazard tags (deep_water, thin_ice, bog, cliff, …). Squares remain 2-byte tile-IDs; attributes live once in the vocabulary and export via the JSON legend. Dynamic states (burning, frozen-now) are the consuming game's, not arda's.
- **POI/items layer** (§Q5): a sparse seeded per-block object list (containers, campsites, shrines, hazard sources) generated with the block, stored as objects, exported in block JSON.
- **Building layout** (§Q3): for settlement cells, the building-layout function is the single source of both `Building` objects (type, footprint squares, occupants — `06-society-generation.md`) and the block's building tiles.
