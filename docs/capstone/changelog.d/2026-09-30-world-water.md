## 2026-09-30 - feat: world rivers and lakes (goals 8-13)
key: feat/2026-09-30-world-water

- New `formation::water` stage (logic/02 §world-water) on the drained recipe-5 lattice:
  - **Braids:** braided belts above the Leopold–Wolman slope, where bed load is plentiful (piedmont or proglacial) and the valley opens.
  - **Meanders:** Kinoshita meanders on alluvial floors below that slope, with sinuosity rising as slope falls, planed meander belts and oxbow cutoff crescents.
  - **Karst:** poljes on a documented carbonate proxy, with an inlet and a downstream spill; dolines are recorded but not carved.
  - **Deltas:** these now build. The old rule's 2,000 km² threshold excluded every MICRO river, and its fan sat inside the drowned ria head. The new deltas are sediment-volume lobes after the shore rework; steep coasts keep their rias.
- `arda_core::water`: bankfull depth `0.3 m × Q^0.4` beside width `4 m × Q^0.5`, the braiding threshold, and stored forms. The optional `areas/<ax>_<ay>/water.bin` holds, per river segment, the pattern, bankfull width and depth, belt, sinuosity and slope. Per lake it holds the origin and terminal (saline) state. It also lists deltas and dolines. Load it with `Area::water()`.
- Atlas overviews draw braided threads, and they drop the synthetic meander where the saved courses already meander.
- New `water_metrics` and `water_diag` examples.
- MICRO results, before → after (the `water_metrics` and `world_metrics` examples):
  - **Hydraulic geometry**, all seeds: width exponent 0.511 and depth exponent 0.412 against discharge. Depth never falls downstream at any confluence (21,123 on seed 42).
  - **Sinuosity** of rivers ≥ 1 m³/s:
    - below 0.3‰ slope: seed 42 1.17 → 1.19, seed 3 1.12 → 1.25, seed 5 1.14 → 1.49;
    - 0.3-1‰: seed 42 1.15 → 1.23, seed 3 1.08 → 1.27, seed 5 1.09 → 1.29.
  - **Lakes:**
    - seed 3: 11 → 17 (6 karst; 15 with an inlet, 17 with an outlet);
    - seed 5: 0 → 2 oxbow lakes;
    - seed 42: 0 → 0 (V-valleys only).
    - No lake of three cells or fewer anywhere.
  - **Deltas:** 0 → 1 (seed 42) and 0 → 3 (seeds 3 and 5).
  - **Other checks:** straight-run share 14 → 12‰ (seed 42), bed pits 0, water-ledger residual 0 L. Seed-42 generation takes 52 → 55 s.
