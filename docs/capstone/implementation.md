---
generated_date: 2026-09-22
generated_at_commit: 342d03e55120
capstone_version: 6.4
---

# Implementation plan — arda

Backend only (no frontend — design skipped, no UI). Every step follows
`standards.md` and code-craft (TDD, YAGNI); citations name the doc
that decided each shape. Verification is per-step; a step is done when
its verification passes.

**Build state updated 2026-09-22 at source `342d03e55120`.** Steps 0–3 were completed
in the initial build. Substantial portions of steps 4, 5, 9 and 10 are also
implemented: continental climate/drainage, shared physical terrain and annual
water, produced area climate/water fields, simple ground cover, format-4
storage, lazy loading, Classic/Atlas PNG exports through 32K with per-pixel
Atlas palette/lighting interpolation, JSON, CLI,
library and Docker. Step 6 retains its sampled tactical prototype; step 12
has physical/boundary/determinism tests but not the full proposed calibration
suite. The table below records those partial completions explicitly.

The original task descriptions and acceptance targets remain the plan; a
remaining subtask does not make an entire stage unimplemented. Current source
evidence, historical successful checks, the latest Windows CI line-ending
failure and unresolved visual acceptance are recorded in
[implementation status](open-items.md).

The [detailed work queue](open-items.md#detailed-work-queue--2026-09-22) now decomposes the remaining work into ordered steps and completion gates, including the confirmed richer-world, area-outline and full tactical-map requirements. Its coverage tables map every original stage and historical open item to the relevant tasks.

The workspace and seam sketches below are retained design/history, not an
exact inventory of current modules. Current structure is in
[architecture](01-architecture.md) and [data flow](04-data-flow.md).

## Workspace layout — retained original plan

```text
arda/
├── Cargo.toml                  # [workspace] members = crates/*
├── Dockerfile                  # multi-arch, entrypoint arda (07-operations)
├──.github/workflows/{ci,release}.yml
├── deny.toml                   # cargo-deny (standards.md § Tooling)
├── crates/
│   ├── arda-core/src/
│   │   ├── lib.rs
│   │   ├── coords.rs           # AreaCoord, CellCoord, SquareCoord newtypes (standards.md § Typing)
│   │   ├── fixed.rs            # Q-format i32/i64 newtypes: HeightMm, Discharge, …
│   │   ├── rng.rs              # subseed derivation → ChaCha8
│   │   ├── cell.rs             # per-cell fields (artifact "finished map knows")
│   │   ├── objects.rs          # RiverSegment, Lake, Settlement, Road, Crossing, Pass, Region…
│   │   ├── tiles.rs            # TileId(u16) + the tile vocabulary + adjacency
│   │   ├── config.rs           # GenerateConfig: size, latitude band, density, memory budget
│   │   ├── error.rs            # per-crate error enums (standards.md § Error handling)
│   │   └── formats/            # ONLY byte codec in the workspace (01-architecture)
│   │       ├── manifest.rs     # world.json, format_version gate (logic/05)
│   │       ├── cells.rs        # cells.bin fixed little-endian layout
│   │       ├── objects.rs      # objects.bin
│   │       └── blocks.rs       # tiles.zst archive, 2 B/square + relaxed marks
│   ├── arda-gen/src/
│   │   ├── lib.rs              # World::generate orchestrator + rayon fan-out
│   │   ├── continent/
│   │   │   ├── plates.rs       # Voronoi seeding, drift, boundary typing (logic/01)
│   │   │   ├── tectonics.rs    # 100-step coupled loop (logic/01)
│   │   │   ├── erosion.rs      # coarse stream-power + drainage (shared w/ area via generics later ONLY if identical — earn it)
│   │   │   ├── climate.rs      # latitude + advection (logic/01)
│   │   │   ├── hydrology.rs    # drainage tree → ContinentRiver
│   │   │   ├── people.rs       # density map + trunk corridors
│   │   │   ├── naming.rs       # regions/ranges/rivers/seas
│   │   │   ├── validate.rs     # bounds + subseed reroll ≤5
│   │   │   └── bundles.rs      # per-tile input bundles
│   │   ├── area/
│   │   │   ├── relief.rs       # conditioned erosion, pinned edges (logic/02 amendments)
│   │   │   ├── water.rs        # drainage, Strahler, floodplain, wetness (artifact)
│   │   │   ├── climate.rs      # lapse, aspect, advection (artifact)
│   │   │   ├── vegetation.rs   # scored cover + override order (artifact)
│   │   │   ├── settlement.rs   # scoring, refusals, tiers, greedy placement, names (artifact)
│   │   │   ├── landuse.rs      # fields/pasture/footprints (artifact)
│   │   │   └── roads.rs        # cost surface, top-down network, crossings, passes (artifact)
│   │   └── block/
│   │       ├── constraints.rs  # cell+8-neighbors → WFC constraints (logic/03)
│   │       ├── edges.rs        # shared-edge entry/exit points from coarse+seed (artifact)
│   │       └── wfc.rs          # most-constrained fill, ≤8 retries, relaxed fallback
│   ├── arda-render/src/
│   │   ├── lib.rs
│   │   ├── symbolic.rs         # built-in block style (logic/04)
│   │   ├── tileset.rs          # tileset-manifest renderer (deferred until custom-art user — deferred; stub error)
│   │   ├── carto.rs            # area/continent cartographic maps
│   │   └── json.rs             # versioned export schema
│   ├── arda/src/lib.rs         # facade: World::{generate,load}, Area, Cell, Block views (mockup/04)
│   └── arda-cli/src/main.rs    # clap: generate | export (mockup/01, 03)
└── tests/                      # workspace golden-world hashes (06-testing)
```

## Load-bearing seams (sketches)

**Subseeding** (`arda-core/src/rng.rs`) — the determinism spine
; every random draw everywhere goes through this:

```rust
pub struct SeedKey { pub tier: Tier, pub stage: Stage, pub x: i32, pub y: i32, pub attempt: u8 }
pub fn rng(world_seed: u64, k: SeedKey) -> ChaCha8Rng {
    // blake3(domain ‖ world_seed ‖ k) → 32-byte key; ChaCha8 stream is portable
    ChaCha8Rng::from_seed(derive(world_seed, k))
}
```

**Correction (built).** This makes `blake3` a *runtime* dependency of
`arda-core`, not the dev/tooling-only crate `05-dependencies.md`
originally listed; that chapter now says so. The derivation is prefixed
with a domain separator (`arda-subseed-v1`) and hashes every field
little-endian in a fixed order, so it is endianness-independent. A
reference-vector test pins the byte layout — changing it changes every
world ever generated, so it may only move with a `format_version`
major.

**Stage purity** (`arda-gen`) — the causal rule (logic/02) as
signatures; each stage takes only prior outputs by value/ref:

```rust
pub fn relief(bundle: &TileBundle, coarse: &CoarseTile, seed: u64) -> ReliefGrid;
pub fn water(relief: &ReliefGrid, bundle: &TileBundle) -> WaterGrid;
// … orchestrator is the only caller; rayon over tiles:
tiles.par_iter().map(|t| generate_area(seed, t, &bundles[t])).collect()  // keyed outputs, order-free
```

**Pinned edges** (`area/relief.rs`, logic/02 amendment 3): edge heights
and river entry/exit computed by a pure function of
(coarse data, seed) reused verbatim by both neighbors — same function
the block edge fixer uses at its scale (`block/edges.rs`).

**Correction (built).** "Reused verbatim" is not enough on its own: the
function must take *absolute* cell coordinates, and the shared edge must
be the same absolute line for both tiles. A tile's east edge therefore
samples local offset `AREA_CELLS` (the neighbour's first column), not
`AREA_CELLS - 1` — those are adjacent cells, not the same cell, and
sampling the latter yields edges that are merely close instead of
equal. With absolute coordinates the whole relief field is edge-safe by
construction, so pinning stops being a repair step and becomes the
definition.

**Continent validation reroll** (`arda-gen/src/orchestrator.rs`,
logic/01) — **missing from this plan until it was built**. The
continent stage is not "generate once": a continent failing the step-9
gate is regenerated from a derived subseed, up to 5 attempts, then a
hard error naming the failed check. `attempt` must reach the plate
seeding `SeedKey` and every noise seed downstream of it, or the reroll
returns the same continent:

```rust
pub fn generate_continent_attempt(seed: u64, cfg: GenerateConfig, attempt: u8) -> ContinentGrid;

for attempt in 0..CONTINENT_ATTEMPTS {           // 5
    let c = generate_continent_attempt(seed, cfg, attempt);
    if LAND_FRACTION_GATE.contains(&c.land_fraction_permille()) { break }
}
```

This is not hypothetical: at the micro size, seed 42 passes at attempt
0 but seed 43 produces 197‰ land and needs the ladder. A plan without
it hard-errors on ordinary seeds.

**Formats** (`arda-core/formats/cells.rs`): fixed-layout row = one
`#[repr(C)]`-documented struct serialized field-by-field little-endian
(no `unsafe` transmute — standards.md § Typing); round-trip property tests.

**Facade** (`crates/arda/src/lib.rs`): thin re-exports; `World::load`
eager manifest, `OnceLock`-cached lazy areas, lazy block decompression
(logic/05).

## Build order

Each step: failing test first (standards.md § Testing), then minimum code,
then its verification. Conventional commit per step.

| # | Step | Verification | State |
| --- | --- | --- | --- |
| 0 | Workspace scaffold: five crates, lints (`deny(unsafe_code)`, missing_docs), CI (3-OS test+clippy+fmt+deny), Dockerfile stub | `cargo test --workspace` green in CI on 3 OSes | done |
| 1 | `arda-core`: coords, fixed-point, rng, error enums, config | unit tests incl. rng reference vectors (same key → same stream) | done |
| 2 | `arda-core::formats`: manifest + cells + objects + blocks codecs | round-trip property tests; version-skew refusal test | done |
| 3 | **Walking skeleton**: micro-continent config (8 tiles) through minimal-but-real stages — kinematic-lite tectonics pass, relief+water only area, ~24-tile WFC subset, symbolic PNG + JSON export, load round-trip | skeleton runs end to end; blake3 stage hashes identical on 3 OSes in CI — **`status: formalized` lands here** | done |
| 4 | Spike S1 then continent full: plates → coupled tectonics/erosion → coast → climate → hydrology → people → naming → validate+reroll → bundles (logic/01 steps 1–10) | S1 eyeball + Horton on fixtures; unit tests per rule; continent stage ≤30 min release-mode | partial: physical terrain, climate, drainage/rivers, validation/rerolls, bundles and overview persistence implemented; people/naming and remaining full-fidelity acceptance open |
| 5 | Area full: seven stages at artifact fidelity, pinned edges, cross-tile agreement | per-rule unit tests from logic/02; two adjacent fixture tiles agree on shared edge byte-for-byte | partial: shared terrain evolution replaces internal edge pinning; annual water, climate fields, basic cover and cross-area water records implemented; full vegetation/soil moisture, settlement, land use and roads open |
| 6 | Spike S2 then blocks full: 200+ tile vocabulary, constraints, WFC ≤8 + relaxed marks | S2 convergence stats on hard cells; continuity test: river/road entry=exit across fixture edges | sampled 24-tile prototype implemented; full constraints/vocabulary/layout/coverage pending |
| 7 | Society stage (`logic/06`): realms (cost-distance allegiance + border snap), building objects from the block layout function, NPC notables with SRD 5.1-style sheets + on-demand commoners; NOTICE file (CC-BY-4.0 attribution) | partition invariants (every land cell in exactly one realm); every notable housed; sheet determinism test | pending |
| 8 | Tile attributes + POI layer: vocabulary attribute table, sparse per-block POIs | legend JSON carries attrs; POI determinism test | pending |
| 9 | Export full: carto renders, block symbolic render, versioned JSON incl. realms/buildings/npcs/poi | byte-identical re-export test; refusal cases (logic/04); export benches ≤60 s area / ≤5 s block | partial: Classic/Atlas cartographic/overview PNGs through 32K, smooth display interpolation over saved 100 m terrain, symbolic blocks and versioned terrain/water/block JSON implemented; society/POI export waits on its producers; original full-scope performance acceptance remains distinct |
| 10 | Facade + CLI + docker + release workflow | `mockup/01`/`03` transcripts reproduced; `World::load` ≤100 ms bench; image runs `generate` via volume | partial: facade, lazy loading, generate/preview/export CLI, Docker and CI implemented; dedicated release workflow and remaining full-scope acceptance pending |
| 11 | `arda serve` (`mockup/06`): tiny_http sync server, read-only endpoints reusing export renderers; docker EXPOSE | endpoint tests against a fixture world; ETag byte-identity; refuses partial worlds | pending |
| 12 | Statistical validation suite as CI gate (Horton/Hack/rank-size/sinuosity/farmland) + full-batch measurement | gates green on fixture continents; default-continent batch time recorded vs ≤12 h target | partial: terrain/water/boundary/resource/determinism tests and batch measurements exist; full combined calibration suite pending; latest CI is not fully green |

New crate paths for the additions: `arda-gen/src/society/{realms,buildings,npcs}.rs`,
`arda-core/src/tiles.rs` gains the attribute table, `arda-cli` gains
`serve.rs` (tiny_http, MIT — synchronous by design; now recorded in
`05-dependencies.md`).

## What building steps 0–3 taught the later steps

Constraints discovered by construction, binding on the steps named:

- **Step 6 (WFC at full vocabulary).** Most-constrained-first must not
  rescan the whole grid each collapse. Recomputing options for all
  4,096 squares per step made one block take seconds; caching options
  per square and refreshing only the four neighbours of the square just
  collapsed cut a test from 27 s to 0.4 s. At 200+ tiles and one block
  per land cell rather than a 64-cell stride, the naive form is not
  merely slow — it does not finish. Budget an entropy heap here, not
  a linear scan.
- **Step 6 (relaxed fallback).** The fallback is unreachable through
  constraint sets alone: every tile is adjacency-compatible with
  itself, so any non-empty allowed set tiles trivially. The retry
  ladder only fires on a genuinely empty option set. Test it that way;
  a test that feeds "incompatible" tiles passes for the wrong reason.
- **Step 5 (area full).** Sim arithmetic overflows `i32` sooner than it
  looks: noise amplitude times a full-swing sample already exceeds it.
  Widen to `i64` for every intermediate product and truncate once at
  the end.
- **Steps 4 and 5 (sinks).** Relief built from noise has many interior
  pits, so D8 alone yields short disconnected channels rather than
  networks reaching the sea. Depression filling is not optional at
  artifact fidelity — it is the prerequisite for the Horton and Hack
  gates at step 12, and step 5 should carry it explicitly.

## Current disposition of the original skeleton ceilings

- **Still present:** blocks are materialized at a 64-cell stride, not at every land cell. `crates/arda-gen/src/orchestrator.rs:300`.
- **Resolved:** `World::load` reads the manifest and uses lazy requested-area/archive caches; owned area reads support exports. The planned `OnceLock` layer exists. `crates/arda/src/world.rs:93`, `crates/arda/src/world.rs:192`.
- **Resolved:** `continent/overview.bin` contains real relief, climate and drainage records; continent river objects are stored too. `crates/arda-gen/src/orchestrator.rs:490`.
- **Still present:** block access decompresses/caches the area's complete archive. Per-block frames and an offset index remain future work. `crates/arda/src/world.rs:274`.
- **Resolved:** final discharge comes from shared annual water processing, with rainfall/runoff/evaporation support, not only upstream cell count. `crates/arda-gen/src/orchestrator/shared_solve.rs:382`, `crates/arda-gen/src/area/shared_compose.rs:203`.
- **Resource handling advanced:** admission and stage budgets are implemented; describing memory control as only an areas-in-flight cap is obsolete. `crates/arda-gen/src/orchestrator.rs:338`.

Detailed custom-art rendering, a region-file front door and a mutable runtime
game API remain outside the implemented surface. The existing read/query Rust
API is implemented; it is not an absent feature. See `crates/arda/src/lib.rs`
and [export behavior](mockup/03-export.md).
