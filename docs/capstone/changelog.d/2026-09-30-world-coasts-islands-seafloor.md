## 2026-09-30 - feat: recipe-5 coasts, islands, seafloor and plains
key: feat/2026-09-30-world-coast

- Tectonic margin context: the recipe-5 plate drift is replayed without uplift to mark collision, arc, island-arc and rift proximity (logic/02 §fine-formation margins).
- Bathymetry (goal 18):
  - shelf width follows tectonic margin activity as well as coastal relief;
  - passive shelves now shallow toward the profile (a sediment prism);
  - submarine canyons are cut down the slope off rivers of 150 km² or more (§bathymetry, §canyons).
- Islands (goal 17):
  - volcanic arc cones on deep ocean along the final-step subduction axes;
  - barrier islands and spits on low, sandy, exposed coasts;
  - delta distributaries that split fans into islands; deltas now start at 500 km²;
  - drowned-ridge continental islands, as before.
- Coasts (goals 15 and 16): rock- and convexity-driven headland retreat into cliffs over wave-cut platforms, bay-head berms, and stored shore classes: sand, shingle, cliff, rocky, marsh, tidal flat and estuary (§littoral, §shore classes).
- Plains (goal 5): lowland valley sides get a terrace staircase and bluffs, and interfluves are compressed (§terraces).
- Basin audit (goal 6): every closed macro basin and plateau gets a tectonic cause (rift, foreland, arc, orogenic, rift shoulder); unexplained basins are filled before formation (§basin audit).
- New optional world layer `terrain/shore.bin` (`arda_core::ShoreLayer`): shore classes on the 100 m grid, an island census with causes, and audited landforms, including glacial troughs. It is published with the fine terrain and read by `World::shore`. The Atlas paints shores from it (logic/04 §atlas-formed shore).
- Earlier recipes, Classic output and the manifest are unchanged. Recipe-5 terrain changes; it has no golden hash.
- `world_metrics` gains `coast`, `landforms` and `plains` objects.
