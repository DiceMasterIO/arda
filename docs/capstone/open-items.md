---
generated_date: 2026-09-07
generated_at_commit: b0ce261
capstone_version: 6.4
---

# Open items

Everything known-incomplete, with the evidence and the code that owns it.
Deferred build-order steps are listed last; the defects and limitations above
them are live.

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
