# 2026-10-07 — Towns build over the shallow shore fringe

- `crates/arda-blocks/src/overlays.rs`: in its own footprint the town layer (buildings and crofts) may claim base squares holding at most 1 ft of standing water (`MADE_GROUND_FT`), as quays and made ground. Deeper water and the town plan's own water stay water. Fields and ways still never take water.
- Why: the biome pass lets the sea reach into sea-level coast cells. In a port at sea level (MICRO 42, Dilrou 309,1096) that 1-ft fringe covered 42% of the town's open ground, because a layer could only take a wet base square by drawing water itself. Refine cannot avoid this, since world cells carry no settlement data until `arda settle`.
