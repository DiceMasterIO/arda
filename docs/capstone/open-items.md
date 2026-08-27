---
generated_date: 2026-08-26
generated_at_commit: 8be0a0a
---

# Open items

Everything known-incomplete, with the evidence and the code that owns it.
Deferred build-order steps are listed last; the defects and limitations above
them are live.

## Blocking realism

| # | Item | Evidence | Owner |
|---|---|---|---|
| ~~1~~ | **Closed by feature 03** (2026-08-27): entering rivers seed each tile from the continent drainage tree. Measured max area-cell catchment at default size 50,070 km² against the old one-tile ceiling of 2,621 km². | Max catchment anywhere = 2,207 km²; hard ceiling is one tile at 2,621 km². A UK-scale major basin is 9,385 km² — **3.6× larger than a whole tile**, so the continent cannot produce even one of the 22–30 systems a UK-sized landmass should have. | `continent/bundles.rs` (bundle has only edge heights), `area/water.rs` |
| 2 | **The WFC tactical layer is salt-and-pepper noise.** Converges, satisfies adjacency, deterministic — and has no spatial structure, because `may_adjoin` lets anything within one "wetness rank" touch, so nearly every tile is compatible with nearly every other and collapse degenerates to uniform random choice. | `export --block 3,7,192,256` on any world. | `arda-core/src/tiles.rs::may_adjoin`, `arda-gen/src/block/wfc.rs` |
| 3 | **The relaxed-fallback ladder is unreachable.** Every tile is adjacency-compatible with itself, so any non-empty constraint set tiles trivially; only an empty set fires the fallback. `logic/03` §Q12 specifies a ladder that cannot trigger in practice. | Test `an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill` has to pass an empty set. | `arda-gen/src/block/wfc.rs` |
| 4 | **Value noise is anisotropic.** Its gradients favour the square lattice's axes, so steepest descent does too and rivers tend to straight runs. | ~17% of flow directions diagonal against an isotropic ~50%, measured on raw relief before any erosion. | `arda-gen/src/noise.rs` |

On (4): isotropic gradient noise is the fix. One attempt produced blocky
rectangular coastlines and drove diagonal flow to 8%; it was reverted rather
than shipped. Domain rotation per octave is the untried alternative.

## Stand-ins awaiting a producer

| # | Item | Current behaviour | Unblocked by |
|---|---|---|---|
| ~~5~~ | **Closed by feature 03** (2026-08-27): `Cell.rainfall` is written from the bundle's 1 km rainfall patch; discharge is `Σupstream rain × 125/788,400` L/s plus entering rivers, and channels initiate at 40 L/s. The artifact's "3 km² ≈ 40 L/s" equivalence is now emergent rather than assumed. | — | — |
| ~~6~~ | **Closed by feature 02** (2026-08-26): `overview.bin` carries real 18 B/cell records (relief, climate, drainage) and `continent/objects.bin` carries rivers, at format 3. | — | — |
| 7 | Blocks are materialised on a **64-cell stride**, not one per land cell. | `mockup/02` specifies per-land-cell. | Build-order step 6 |
| 8 | `World::load` reads **every** area and block eagerly. | `logic/05` specifies lazy access with an O(accessed) cache. | Build-order step 10 |

## Calibration held open

| # | Item | Note |
|---|---|---|
| 9 | **No statistical validation suite.** No Horton, Hack, rank-size, sinuosity, or farmland gate exists. | Erosion and lake constants are calibrated only against the artifact's two stated equilibrium anchors (1 km² → 9% slope, 100 km² → 1%) and against Earth's hypsometric curve — not against network statistics. Build-order step 12. |
| 10 | **Lake thresholds** (100 cells, 2 m) were derived from basin distributions measured on **pre-erosion** relief. | Erosion reshapes that distribution; re-derive when (9) lands. |
| 11 | **Tile-edge taper band.** Area erosion ramps to zero over 32 cells at the pinned rim, so the 35° repose rule is not enforced there (measured tan×1000 of 1,324 inside the band against exactly 700 at full strength). | Structural: per-tile erosion must freeze tile edges for neighbours to agree byte-for-byte. Seamless tiled erosion needs a global pass or a proven-decay overlap scheme. |
| 12 | A basin straddling a tile seam: **materially improved, not exactly closed** (feature 03). Both sides now take the lake surface from the shared continent routing surface instead of independent local spills, and membership is trimmed to cells genuinely under that surface. | Each side still takes the max over *its own* rim cells, so fragments with different contact spans can differ (synthetic case: 723 mm; the spec's pre-approved fallback measured worse). **No natural straddling pair exists in 4,280 surveyed (seed, seam) combinations.** Exact agreement needs continent-tier lake identity — `logic/01` step 6 does not emit lake objects. |

## Groomed but not built

- **Feature 02** — continent climate and hydrology: **built** (2026-08-26,
  commits c2c0d2a..9c48e00; spike S1 discharged by the recorded Horton
  measurements). (1), (5), (12) are now unblocked, not closed.
- **Feature 03** — climate-driven refinement: **built** (2026-08-27,
  commits 78bbff5..8be0a0a). Closed (1) and (5); improved (12); also
  landed the step-10 bundle payload, the step-9 river gate, and
  order-banded river rendering. Five review rounds to dry.

**Next, in the order the evidence suggests:** (4) the value-noise
anisotropy is now the most visible remaining river defect — at area
zoom, streams still run as straight parallel combs along the lattice
axes, which is what "rivers look like fjords" describes at close range.
Cross-tile continuity and hierarchy are fixed; the *shape* of an
individual stream is not. The recorded fix (isotropic gradient noise)
was tried once and reverted for blocky coastlines, so it needs its own
feature rather than an in-place patch.

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
