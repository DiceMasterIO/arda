## 2026-09-29 - feat: recipe-5 stream-power terrain formation and formed Atlas shading
key: feat/2026-09-29-recipe5-formation

- New `arda-gen::formation` stage (logic/02 §fine-formation): six-level integer stream-power formation from the macro surface to the canonical 39.0625 m field.
  - **Erosion:** warped upsampling between levels; stochastic slope-weighted receivers; relative uplift against a spatially varying talus cap (rock strength); coarse-level diffusion plus lowland soil creep; soft-edged alluvial floodplains.
  - **Coasts:** glacial-lowstand drowned coasts with estuarine infill, river-mouth deltas and wave-reworked shores.
  - **Basins and lakes:** tectonic basins preserved as closed depressions; glacial trough lakes and fjords.
  - **Drainage guarantees:** a final fine-lattice fill plus a 100 m sampled-drainage pass; local depression fills between exact steps.
- `--terrain fine` now produces recipe 5. Recipe 4 remains reproducible via `generate_world_with_fine_recipe(.., FineRecipe::Valleys)`. `FINE_TERRAIN_RECIPE_VERSION` is 5.
- Atlas (logic/04 §atlas-formed), recipe ≥ 5 only:
  - four-baseline relief light with soft warm shadows and a palette matched to measured reference colours;
  - landform-driven materials, beaches and shore rock;
  - snow from saved temperature lapsed to fine height;
  - fine-contour anti-aliased coasts and lakes, and an 8-stop depth sea palette;
  - thalweg-snapped river vertices in area and overview renders, and a recipe-5 overview river symbol;
  - parallel overview base sampling, byte-identical to serial.

  Recipes 2–4 and Classic are unchanged, apart from a 20-byte Atlas halo sample that now carries saved temperature.
- New diagnostics: the `world_metrics` example (lakes, straight-run river share, pits, hypsometry, slopes) and the `form_fine_terrain` example.
- Seed-42 measurements:
  - MICRO formation 25 s; complete world about 55 s; lakes 1,055 → 0 artefact lakes.
  - 500×1000 km worlds complete within **12.8 GB peak RSS** (under the 16 GiB ceiling).
  - Against the previous best recipe-4 world: grid-straight river runs 265‰ → 0‰, bed pits 447 → 1.
- Tests: formation determinism (repeat and single-thread); no fine or sampled pits; local fill; admission refusal; ria infill; basin closure; glacial basins; order and area conservation; integer sqrt, Catmull-Rom and blur; formed shader light, snow, sea, shore and lake colours.
