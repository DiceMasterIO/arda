---
generated_date: 2026-08-26
generated_at_commit: 9c48e00
---

# Open items

Everything known-incomplete, with the evidence and the code that owns it.
Deferred build-order steps are listed last; the defects and limitations above
them are live.

## Blocking realism

| # | Item | Evidence | Owner |
|---|---|---|---|
| 1 | **Rivers cannot cross tile boundaries.** `water` routes per tile; outflow works, inflow does not, because `logic/01` step 10's entering rivers have no producer. **Feature 02 built the continent drainage tree this needs** (`continent/hydrology.rs`, persisted in `overview.bin`); closure is Feature 03's bundle entering-rivers. | Max catchment anywhere = 2,207 km²; hard ceiling is one tile at 2,621 km². A UK-scale major basin is 9,385 km² — **3.6× larger than a whole tile**, so the continent cannot produce even one of the 22–30 systems a UK-sized landmass should have. | `continent/bundles.rs` (bundle has only edge heights), `area/water.rs` |
| 2 | **The WFC tactical layer is salt-and-pepper noise.** Converges, satisfies adjacency, deterministic — and has no spatial structure, because `may_adjoin` lets anything within one "wetness rank" touch, so nearly every tile is compatible with nearly every other and collapse degenerates to uniform random choice. | `export --block 3,7,192,256` on any world. | `arda-core/src/tiles.rs::may_adjoin`, `arda-gen/src/block/wfc.rs` |
| 3 | **The relaxed-fallback ladder is unreachable.** Every tile is adjacency-compatible with itself, so any non-empty constraint set tiles trivially; only an empty set fires the fallback. `logic/03` §Q12 specifies a ladder that cannot trigger in practice. | Test `an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill` has to pass an empty set. | `arda-gen/src/block/wfc.rs` |
| 4 | **Value noise is anisotropic.** Its gradients favour the square lattice's axes, so steepest descent does too and rivers tend to straight runs. | ~17% of flow directions diagonal against an isotropic ~50%, measured on raw relief before any erosion. | `arda-gen/src/noise.rs` |

On (4): isotropic gradient noise is the fix. One attempt produced blocky
rectangular coastlines and drove diagonal flow to 8%; it was reverted rather
than shipped. Domain rotation per octave is the untried alternative.

## Stand-ins awaiting a producer

| # | Item | Current behaviour | Unblocked by |
|---|---|---|---|
| 5 | `Cell.rainfall` (area tier) has **no writer**; continent-tier rainfall now exists per 1 km cell (feature 02) awaiting the area handoff. | Discharge is `catchment × 40 / 300`, anchored to the artifact's own "3 km² ≈ 40 L/s" equivalence rather than to rain. Channel initiation uses the 300-cell catchment form of the same rule. The artifact's real rule is "the rain that fell upstream, less the roughly half that evaporates or soaks in". | Continent climate ✓ (feature 02); area handoff = Feature 03 |
| ~~6~~ | **Closed by feature 02** (2026-08-26): `overview.bin` carries real 18 B/cell records (relief, climate, drainage) and `continent/objects.bin` carries rivers, at format 3. | — | — |
| 7 | Blocks are materialised on a **64-cell stride**, not one per land cell. | `mockup/02` specifies per-land-cell. | Build-order step 6 |
| 8 | `World::load` reads **every** area and block eagerly. | `logic/05` specifies lazy access with an O(accessed) cache. | Build-order step 10 |

## Calibration held open

| # | Item | Note |
|---|---|---|
| 9 | **No statistical validation suite.** No Horton, Hack, rank-size, sinuosity, or farmland gate exists. | Erosion and lake constants are calibrated only against the artifact's two stated equilibrium anchors (1 km² → 9% slope, 100 km² → 1%) and against Earth's hypsometric curve — not against network statistics. Build-order step 12. |
| 10 | **Lake thresholds** (100 cells, 2 m) were derived from basin distributions measured on **pre-erosion** relief. | Erosion reshapes that distribution; re-derive when (9) lands. |
| 11 | **Tile-edge taper band.** Area erosion ramps to zero over 32 cells at the pinned rim, so the 35° repose rule is not enforced there (measured tan×1000 of 1,324 inside the band against exactly 700 at full strength). | Structural: per-tile erosion must freeze tile edges for neighbours to agree byte-for-byte. Seamless tiled erosion needs a global pass or a proven-decay overlap scheme. |
| 12 | A basin straddling a tile seam is filled **independently by each side** and may reach different spill levels. | The continent drainage tree now exists and exposes its routing surface (feature 02); wiring it into per-tile fill is Feature 03's, same as (1). |

## Groomed but not built

- **Feature 02** — continent climate and hydrology: **built** (2026-08-26,
  commits c2c0d2a..9c48e00; spike S1 discharged by the recorded Horton
  measurements). (1), (5), (12) are now unblocked, not closed.
- **Feature 03** — rainfall-driven discharge, climate-driven channel
  initiation, cross-tile inflow via bundle entering-rivers. Not yet
  groomed. Its planner should read feature 02's spec §Reference impact
  and `04-data-flow.md`'s seam note: `generate_world` currently computes
  and discards the climate/hydrology structs after persisting.

## Unbuilt build-order steps

| Step | Scope |
|---|---|
| 4 | Continent full — **climate, hydrology objects, and S1 done** (feature 02); remainder: human geography, naming, validation stats. |
| 5 | Area stages 3–7: climate, vegetation, settlement, land use, roads. Also `Cell.temperature`, `rainfall`, `moisture`, `forest_density`, `road`, `built_by`, all still at `Default`. |
| 6 | Blocks full: 200+ tile vocabulary. Spike S2 first — note S2 was scoped to *convergence*, and (2) above shows convergence was never the risk. |
| 7 | Society: realms, buildings, NPC sheets. |
| 8 | Tile attribute table + POI layer. |
| 9–12 | Export full, facade/CLI/docker polish, `arda serve`, statistical gates. |

## Not defects

Recorded so they are not re-investigated: the pinned tile seam is **smoother**
than the interior (p90 2,009 mm at the seam against 7,400 mm ten cells in,
land-to-land), and two all-ocean tiles legitimately share identical
`objects.bin` and block-archive hashes.
