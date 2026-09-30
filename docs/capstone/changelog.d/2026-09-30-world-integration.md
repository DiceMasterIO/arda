## 2026-09-30 - integrate: world physics (bands, coast, water, atlas) on variant Q
key: integrate/2026-09-30-world

- Merged, in order, `fix/uneroded-bands`, `feat/world-coast`, `feat/world-water` and `feat/atlas-polish` onto `wip/variant-q`. Not signed off: the look awaits the maintainer.
- **Stage order** (`formation::form_world`, logic/02): margins, bathymetry, arc cones and the basin audit on the macro surface; belt-relief level masks (read after the arcs, so cones count as high relief); formation; coast (infill, terraces, shelf prism, shore rework, littoral, canyons); water (deltas, channels, basins); drainage guarantees; the shore survey last.
- **One delta rule:** §world-water's river-led, volume-limited lobe. `coast::deltas` is removed. The coast rule's two contributions carry over: distributary islands (from 1,000 km², a fork into two tidal channels) and its 160/255 coast limit (for rivers of 2,000 km² and more, rising from 100/255 at 1,000 km²). The shore census reads the water deltas, so delta islands keep their cause. Lobes that would reach within 3 km of the domain rim are refused by the radius bisection.
- **Published layers:** the orchestrator writes both `terrain/shore.bin` and `areas/*/water.bin`. The Atlas paints stored shores and follows the data meanders with the tapered B-spline centrelines.
- **Fixes found while verifying:**
  - canyons followed D8 paths as straight trenches across the shelf prism; they now follow a continuous, wandering heading for up to 50 km and fade out;
  - the shelf-profile distance is Euclidean, not a chamfer;
  - arc cones retreat only 30 m in the littoral pass. Before, cliff retreat planed every full-size arc summit to 40–80 m below sea level.
- **MICRO results** (seeds 42, 3, 5 and 7):
  - pits 0, lakes of 3 cells or fewer 0, straight runs 0–3‰, ledger residual 0;
  - belt crest/flank roughness 0.47–0.55, and 1.37 on seed 3, which has no divide band;
  - beaches: more in bays than on headlands on every seed;
  - deltas 1/8/3/1.
- **Full size** (seed 42, 500×1000 km): 46 min and 13.1 GiB peak. Open items are listed in the report: oxbow fragments of 3 cells or fewer, straight channels on large delta plains, and submerged arcs before the cone fix.
- The Classic render is byte-identical (legacy seed 42 MICRO overview and area 1,1).
