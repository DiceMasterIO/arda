---
generated_date: 2026-09-08
generated_at_commit: b0f93f22b969
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# Open items

Current implementation status comes first. The older issue inventory below is retained as dated evidence; its former runtime claims are not current contracts.

## Current status — 2026-09-08

The area-water-terrain implementation is installed. Candidate06 corrects the
altitude-dependent detail amplitude that repeatedly manufactured shallow basins
on gentle high terrain. Detail now follows surrounding regional height differences,
using the existing bounded sampler and noise. The annual water rules, saved format
and renderer are unchanged.

The reported seed436342 changes from 2,653 to 186 lakes; fixed 50 km squares with
at least 50 lake anchors fall from 18 to zero. Its former densest square changes
from 158 to 3. These are measurements, not production quotas. The 16K comparison
visibly removes the repeated pond patches. Total wet area changes from 5,028.39 to
2,558.01 km²; substantial regional lakes remain.

All five frozen worlds, 38 selected JSONs and 238 exports pass, including repeated
bytes, unchanged saved worlds, exact annual balances and shared wet-boundary
identity/surface checks. The combined workspace, focused repairs and fresh
approved C06 golden comparison cover 590 passing tests. The terrain correction
and its deterministic baseline are committed as `ed4875d`. Parallel ravines, angular shorelines and large
rectangular regional basins remain open; the overall feature is not marked done.
Evidence: [C06 comparison](features/2026-09-07-area-water-terrain-realism/verification/lake-district-correction/data-comparison/REPORT.md)
and [maps](features/2026-09-07-area-water-terrain-realism/verification/lake-district-correction/gallery-c06.md).

| Item | Current evidence and disposition |
|---|---|
| Terrain shape and incision (old #4) | Candidate04 evolves one physical rectangle, applies implicit downstream-first incision, removes the unresolved two-cell detail octave and uses a radial coast mask. A frozen-input replay reproduced the numerical pit defect exactly and the implicit update removed it while controlled physical bowls survived. The radial mask removes a verified planar coarse flank; current default and MICRO images still show regular parallel drainage. The large rectangular seed42 lake follows an existing coarse basin. No target direction percentage or arbitrary river-count reduction is used; overall visual acceptance remains open. |
| Lazy loading (old #8) | Implemented: manifest-only `World::load`, requested area/archive caches, uncached owned area reads for exports. Actual missing/corrupt/sparse-file fixtures verify later I/O failures and admission checks. |
| Lake thresholds (old #10) | Canonical generation uses fine-grid depressions and an annual water-support calculation, including positive-depth physical storage, rainfall, runoff and evaporation. The historical 100-cell/2 m rule no longer controls final world lakes. |
| Cross-area lake authority (old #12) | Shared fine topology, global IDs, physical surfaces and copied feature records replace nearest-coarse-basin/max-surface reconciliation. Constructed boundary/corner controls and all 38 selected natural record comparisons pass. |
| Tactical noise and fallback (old #2, #3, #7) | Remain open. Existing 24-tile WFC, permissive adjacency and 64-cell sampling stride are unchanged. Detailed assets, movement/collision geometry, NPCs and server/browser transport require later work. |
| Erosion rim (old #11) | Candidate02 seed436342 area3,10 had 50.1% wet rim cells versus 6.2% wet interior. Production now evolves terrain across publication boundaries without pins or taper. Candidate04 gives 4.89% wet rim versus 4.95% interior; candidate05 gives 6.51% versus 6.45%. Across all 513 candidate05 default areas, every one of 21,772 adjacent pairs wet on both sides has matching IDs/surfaces; candidate06 checks 20,712 such pairs with no mismatch. The continuous water square is absent in the current preview. Only the true modeled outer rim remains fixed. See `features/2026-09-07-area-water-terrain-realism/verification/terrain-correction/visual-default436342-c05.md`. |
| Statistical calibration (old #9) | Existing drainage, cross-tile, terrain and determinism suites run. This work adds controlled physical/annual/resource/geometry checks and a frozen natural panel; it does not supply the full proposed Horton/Hack/rank-size/sinuosity/farmland calibration suite. |
| Cell producers | Temperature, rainfall, wetness, water and terrain metrics now have producers. Cell.moisture, full vegetation, human geography, roads, buildings and society remain deferred. |

Accepted physical/rendering limits are the 100 m terrain lattice, quantized pond footprints, thin channels rendered with physical area coverage, coarse angular shorelines, sequential world preparation/composition, and a representative static annual balance. Snow storage, groundwater, seasons and dynamic floods are absent. Full area JSON is intentionally explicit and can be tens of megabytes; no browser parse or network-loading budget has been demonstrated. See the current [models](02-models.md), [testing](06-testing.md), [operations](07-operations.md) and [export behavior](logic/04-export.md).

## Historical inventory — through 2026-08-27

The entries below preserve the diagnosis and measurements that motivated later work. The current table above supersedes their implementation-status claims.

## Blocking realism

| # | Item | Evidence | Owner |
| --- | --- | --- | --- |
| ~~1~~ | **Closed by feature 03** (2026-08-27): entering rivers seed each tile from the continent drainage tree. Measured max area-cell catchment at default size 50,070 km² against the old one-tile ceiling of 2,621 km². | Max catchment anywhere = 2,207 km²; hard ceiling is one tile at 2,621 km². A UK-scale major basin is 9,385 km² — **3.6× larger than a whole tile**, so the continent cannot produce even one of the 22–30 systems a UK-sized landmass should have. | `continent/bundles.rs` (bundle has only edge heights), `area/water.rs` |
| 2 | **The WFC tactical layer is salt-and-pepper noise.** Converges, satisfies adjacency, deterministic — and has no spatial structure, because `may_adjoin` lets anything within one "wetness rank" touch, so nearly every tile is compatible with nearly every other and collapse degenerates to uniform random choice. | `export --block 3,7,192,256` on any world. | `arda-core/src/tiles.rs::may_adjoin`, `arda-gen/src/block/wfc.rs` |
| 3 | **The relaxed-fallback ladder is unreachable.** Every tile is adjacency-compatible with itself, so any non-empty constraint set tiles trivially; only an empty set fires the fallback. `logic/03` specifies a ladder that cannot trigger in practice. | Test `an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill` has to pass an empty set. | `arda-gen/src/block/wfc.rs` |
| 4 | **Value noise is anisotropic.** Its gradients favour the square lattice's axes, so steepest descent does too and rivers tend to straight runs. | ~17% of flow directions diagonal against an isotropic ~50%, measured on raw relief before any erosion. | `arda-gen/src/noise.rs` |

On (4): **the recorded diagnosis was wrong, and is corrected here.** A
measured sweep (2026-08-27) found the diagonal-flow share is 23.2%
post-erosion / 38.1% pre-erosion, not the "~17%" recorded — and zeroing the
value-noise detail term entirely barely moves it (21.3%), which proves the
detail noise is **not** the dominant source. The axis bias comes from
`coarse_height`'s own separable smoothstep-bilinear sample of the 1 km grid,
which is what every area cell's regional trend is built on.

Attempts, all measured: per-octave domain rotation (the previously "untried
alternative") moves the share to 23.9% — the floor-then-lattice-lookup
reconstructs an axis-aligned staircase, defeating the rotation; rotation plus
per-octave offset, 23.8%; domain-warping the detail term alone, 23.4–24.0%.
Warping `coarse_height`'s sample position as well reaches 28.6% and visibly
reduces the combs at area zoom — **but it was rejected on the render**: at
default size it speckles the coastline, scatters noise-like micro-lakes
through the interior, and weakens the trunk hierarchy. That is the same class
of regression that killed the first attempt, so it was reverted rather than
shipped.

What the evidence now points at: the combs are strongest on smooth mountain
flanks, where a near-planar slope sends every cell the same way and no
convergence forms. That is a *terrain-shape* problem (too little fine-scale
valley structure for erosion to organise), not purely a noise-isotropy one.
A real fix likely needs the erosion budget or the relief construction
revisited, which is a feature with a spike, not a constant to tune.

## Stand-ins awaiting a producer

| # | Item | Current behaviour | Unblocked by |
| --- | --- | --- | --- |
| ~~5~~ | **Closed by feature 03** (2026-08-27): `Cell.rainfall` is written from the bundle's 1 km rainfall patch; discharge is `Σupstream rain × 125/788,400` L/s plus entering rivers, and channels initiate at 40 L/s. The artifact's "3 km² ≈ 40 L/s" equivalence is now emergent rather than assumed. | — | — |
| ~~6~~ | **Closed by feature 02** (2026-08-26): `overview.bin` carries real 18 B/cell records (relief, climate, drainage) and `continent/objects.bin` carries rivers, at format 3. | — | — |
| 7 | Blocks are materialised on a **64-cell stride**, not one per land cell. | `mockup/02` specifies per-land-cell. | Build-order step 6 |
| 8 | `World::load` reads **every** area and block eagerly. | `logic/05` specifies lazy access with an O(accessed) cache. | Build-order step 10 |

## Calibration held open

| # | Item | Note |
| --- | --- | --- |
| 9 | **No statistical validation suite.** No Horton, Hack, rank-size, sinuosity, or farmland gate exists. | Erosion and lake constants are calibrated only against the artifact's two stated equilibrium anchors (1 km² → 9% slope, 100 km² → 1%) and against Earth's hypsometric curve — not against network statistics. Build-order step 12. |
| 10 | **Lake thresholds** (100 cells, 2 m) were derived from basin distributions measured on **pre-erosion** relief. | Erosion reshapes that distribution; re-derive when (9) lands. |
| 11 | **Tile-edge taper band.** Area erosion ramps to zero over 32 cells at the pinned rim, so the 35° repose rule is not enforced there (measured tan×1000 of 1,324 inside the band against exactly 700 at full strength). | Structural: per-tile erosion must freeze tile edges for neighbours to agree byte-for-byte. Seamless tiled erosion needs a global pass or a proven-decay overlap scheme. |
| 12 | A basin straddling a tile seam: **mechanism now exact, precondition unobserved** (2026-08-27). The continent tier emits lake identity — `ContinentHydrology.basin_surface` gives every 1 km cell inside a filled depression that depression's single surface — and a near-rim area basin takes its lake surface from a nearest-cell (never interpolated) lookup of that constant. Two fragments of one depression therefore agree **exactly**, verified at 0 mm on a constructed two-tile case against the real `compose` path. | Residues, both unobserved: no fixture has yet produced a straddling basin whose cells see a continent depression at all (0 in a 4,280 seed/seam sweep), so the exact path is proven on constructed input rather than natural data; and a fragment whose rim abuts two *different* depressions takes a max of two constants, which is span-dependent again. Where the continent tier sees no depression (sub-km pits invisible at 1 km) the old bilinear rule still applies, pinned at 723 mm on the synthetic case. |

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
| --- | --- |
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
