---
generated_date: 2026-09-07
scenarios: [load-query]
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 04 — Crate API (Rust): load & query

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

The library surface: game devs embed `arda` (crates.io,
MIT/Apache-2.0 dual) to load a generated world and read it.
Generation is also callable (the CLI is a thin wrapper — assumed). No
runtime/streaming API commitment in v1. All identifiers below are assumed; the data they expose is the artifact's field list verbatim.

## Layout

```text
// interaction transcript, not final API
let world = arda::World::load("worlds/w42")?;          // reads 02's layout
world.seed();  world.size_km();  world.areas();        // manifest facts

let area  = world.area(3, 11)?;                        // 51.2 km tile
let cell  = area.cell(300, 128);                       // 100 m cell
cell.height_m; cell.cover; cell.slope; cell.aspect;
cell.temperature; cell.rainfall; cell.moisture;
cell.watercourse();   // Option<order, width_m>
cell.height_above_river_m; cell.wetness;
cell.road();          // Option<class>
cell.built_by();      // Option<settlement id>

area.settlements();   // name, tier, population, site tags
area.rivers();        // segments: course, order, size, terminus
area.roads(); area.crossings(); area.passes(); area.lakes();

let block = world.block(3, 11, 300, 128)?;             // decompresses tile-IDs
block.square(10, 41);                                  // tile ID, 5 ft square
block.render_png()?;                                   // same renderer as export
```

Element tree: `World` (manifest) → `Area` (cells + objects) → `Cell`
(properties) / `Block` (squares); mirrors 02's directory tiers exactly.

## Elements

| Element | Does | Status / notes |
| --- | --- | --- |
| `World::load(dir)` | Opens a generated world; refuses partial ones (02 States) | "loading" |
| `World::generate(seed, config)` | Library form of 01's batch | "pure script, a library" (assumed) |
| `Area::cell(x, y)` | Every per-cell property from the artifact's "what the finished map knows" | Confirmed design |
| `Area` object accessors | Settlements, river segments, lakes, roads, crossings, passes as typed lists | Confirmed design |
| `World::block(..)` | Lazy read of the materialized tile-IDs; deterministic render | Confirmed design |
| Property schema detail | Field-by-field types "drilled down later" | deferred, logic stage |

## States

- **Success**: all reads are infallible after load except range errors (typed `Result`).
- **Empty/missing world**: `load` returns a typed error naming the manifest problem.
- **Version skew**: world written by a different arda version → typed error; regenerate from seed (assumed).
