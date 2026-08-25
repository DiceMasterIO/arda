---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/**"]
generated_at_commit: 8d3c9d9
---

> Prescriptive — written from the design interview, not from code.

# Glossary

## Concepts

| Term | Meaning in arda | Defined |
|---|---|---|
| World | Everything one (seed, config) produces: one continent, its areas, their blocks; a directory on disk | `mockup/02` |
| Continent | The whole generated landmass, default 500×1000 km, flat map, ocean at every edge | mockup Q22 |
| Area / tile | One 51.2 km × 51.2 km square of the continent: 512×512 cells; the unit of parallel generation | artifact "Two grids"; `logic/02` |
| Cell | 100 m (nominal; 97.5 m actual) square, one hectare; the unit of area-level reasoning | artifact; §Q12 (64 squares) |
| Block | The 64×64 grid of 5-ft squares inside one cell; a battle map is a quarter block | artifact; `logic/03` |
| Square | One 5-ft D&D square, carrying a 2-byte tile ID | `logic/03` |
| Tile (ID) | An entry in the 200+ tile vocabulary the WFC places; transitions are explicit tiles | `logic-interview.md §Q12` |
| Tile bundle | A tile's generation inputs from the continent: edge heights, entering rivers, regime, wind, density, road exits | `logic/01` step 10 |
| Entering river | A watercourse crossing into a tile with catchment/discharge already accumulated upstream | artifact; `logic/01` §Q7 |
| Strahler order | River importance measure: two order-n streams meet → order n+1 | artifact "Water" |
| Regime | Climate preset of a place: mediterranean / temperate / boreal / tropical | artifact; `logic/01` §Q6 |
| Relaxed block | A block whose WFC hit the retry bound and was filled greedily with constraints relaxed; marked in metadata | `logic-interview.md §Q12` |
| Trunk corridor | Continent-level least-cost route between density basins; area roads must connect where it crosses their edges | `logic/01` §Q7 |
| Subseed | PRNG key derived as (seed, tier, stage, coords, attempt); the only randomness source | `architecture-interview.md §Q4` |
| format_version | The world directory's compatibility major, independent of crate semver; newer majors are refused at load | §Q5, §Q6; `logic/05` |
| Golden world | Tiny fixed-seed world whose per-stage hashes CI compares across OSes to enforce bit-exactness | §Q5 |
| Walking skeleton | The first build slice: micro-continent → one area → blocks → export → load, in CI | §Q8 |

## Terms from the drainage and continent work (2026-08-26)

| Term | Meaning here |
|---|---|
| Priority-flood | Depression-filling sweep that raises every interior pit to its spill elevation, producing the surface flow is routed over. Stored heights keep the real terrain; only routing sees the filled field. |
| Routing surface | The filled field. Distinct from stored relief, which the artifact keeps as real terrain that rivers pass through. |
| Basin / lake | A connected group of cells under the filled surface. Recorded as a `Lake` when it covers at least 100 cells and is at least 2 m deep; smaller ones fill silently as noise. |
| Strahler order | Stream rank: heads are 1, and a cell takes the maximum incoming order plus one when two or more inflows share it. Not a magnitude band. |
| Reach / segment | One network link: source or confluence, downstream to the next confluence, the sea, a lake, or the tile edge. Channel cells belong to exactly one. |
| HAND | Height Above Nearest Drainage — elevation above the first watercourse down the flow path. Decides the floodplain bands (marsh under 1 m, floods under 2.5 m, terrace 2-15 m). |
| TWI | Topographic wetness index, drainage area over slope, in fixed point. No `ln` or `tan` reaches a sim path. |
| Orogenic belt | The band either side of a convergent plate boundary that uplift is spread across — 9 sim cells for a collision, 6 for an arc. Raising the boundary line alone gives a plateau, not a range. |
| Shelf gradient | Blurred crust field that makes the continent meet the ocean over tens of kilometres rather than one 4 km cell. Without it the coastline followed the grid as rectangular steps. |
| Continental mask | Centred weighting that places the landmass in the middle of the domain, so a continent exists at any size. |
| Talus collapse | The artifact's rule that slopes past about 35 degrees fail until they are not. Enforced where erosion runs at full strength. |
| Edge taper | Ramp that fades area-tier erosion to zero at the pinned tile rim, so the frozen edge leaves a gradient rather than a lip. |
