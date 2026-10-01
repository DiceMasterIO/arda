## 2026-10-01 - feat: recipe 7, arid terminal and salt lakes
key: feat/2026-10-01-arid-basins

- **Recipe 7 is the default** (approved by the maintainer on 2026-10-01; `FINE_TERRAIN_RECIPE_VERSION` 7). `--recipe 6` reproduces v0.2–v0.3 worlds byte for byte (`tests/golden_recipes.rs`); recipe 7 is pinned from its first release.
- **Climate in formation** (logic/02 §fine-formation climate runoff): the continent's annual balance, `R = P − min(P/2, E)` and `D = E − min(P/2, E)` with Hamon `E`, weights contributing area by `R / 500 mm` for channel initiation, incision and channel sizing (network, oxbows, delta distributaries).
- **Subtropical highs** (logic/01 §Q6): recipe-7 rainfall is scaled by the Hadley belt, 30% between 15° and 26°, full poleward of 38°. `arda generate --latitude SOUTH,NORTH` sets the band (default 35,55).
- **Arid basins** (logic/02 §world-water arid basins, goal 12): an audited tectonic basin whose catchment runoff is below 90% of the evaporation its depression would sustain at the spill is terminal. Formation fills its floor into a playa (three times the equilibrium lake), and the shared annual solve finds the terminal lake at equilibrium on it. Ledger closure with evaporation is tested on arid and humid bowls (goal 21).
- **water.bin layout 2** (recipe 7): lake origin `arid_terminal`, a saline flag, and playa runs (salt crust, mudflat). Recipe 6 still writes layout 1.
- **Atlas** (logic/04 §atlas-formed arid basins): saline lakes in lighter turquoise, salt pans as white crust with pale mudflat margins; dry land below sea level inside the continent is land.
- **Tactical** (logic/09): playa squares take the new ground keys `salt_crust` and `mudflat` (difficult), with no scatter; generated placeholder art for both (211 assets), fallback chains in arda-fields, arda-ways and arda-town; `docs/goal-prompts/vocabulary.md`.
- MICRO seed 74 at 15–35°: a 482 km² terminal saline lake on 697 km² of dry pan (land rain 382 mm); recipe 6 at the same band fills the basin into a 1,278 km² fresh lake. Renders: `out/v4-arid/`.
