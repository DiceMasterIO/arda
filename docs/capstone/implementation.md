---
generated_date: 2026-08-25
---

# Implementation plan — arda

Backend only (no frontend — design skipped, no UI). Every step follows
`code-prefs.md` and code-craft (TDD, YAGNI); citations name the doc
that decided each shape. Verification is per-step; a step is done when
its verification passes.

## Workspace layout (to create)

```text
arda/
├── Cargo.toml                  # [workspace] members = crates/*
├── Dockerfile                  # multi-arch, entrypoint arda (07-operations)
├── .github/workflows/{ci,release}.yml
├── deny.toml                   # cargo-deny (code-prefs §Q7)
├── crates/
│   ├── arda-core/src/
│   │   ├── lib.rs
│   │   ├── coords.rs           # AreaCoord, CellCoord, SquareCoord newtypes (code-prefs §Q1)
│   │   ├── fixed.rs            # Q-format i32/i64 newtypes: HeightMm, Discharge, … (stack §Q7)
│   │   ├── rng.rs              # subseed derivation → ChaCha8 (arch §Q4)
│   │   ├── cell.rs             # per-cell fields (artifact "finished map knows")
│   │   ├── objects.rs          # RiverSegment, Lake, Settlement, Road, Crossing, Pass, Region…
│   │   ├── tiles.rs            # TileId(u16) + the tile vocabulary + adjacency (logic §Q12)
│   │   ├── config.rs           # GenerateConfig: size, latitude band, density, memory budget
│   │   ├── error.rs            # per-crate error enums (code-prefs §Q4)
│   │   └── formats/            # ONLY byte codec in the workspace (01-architecture)
│   │       ├── manifest.rs     # world.json, format_version gate (logic/05)
│   │       ├── cells.rs        # cells.bin fixed little-endian layout (arch §Q5)
│   │       ├── objects.rs      # objects.bin
│   │       └── blocks.rs       # tiles.zst archive, 2 B/square + relaxed marks
│   ├── arda-gen/src/
│   │   ├── lib.rs              # World::generate orchestrator + rayon fan-out (arch §Q3)
│   │   ├── continent/
│   │   │   ├── plates.rs       # Voronoi seeding, drift, boundary typing (logic/01 §Q5)
│   │   │   ├── tectonics.rs    # 100-step coupled loop (logic/01 §Q3–Q4)
│   │   │   ├── erosion.rs      # coarse stream-power + drainage (shared w/ area via generics later ONLY if identical — earn it)
│   │   │   ├── climate.rs      # latitude + advection (logic/01 §Q6)
│   │   │   ├── hydrology.rs    # drainage tree → ContinentRiver (§Q7)
│   │   │   ├── people.rs       # density map + trunk corridors (§Q7)
│   │   │   ├── naming.rs       # regions/ranges/rivers/seas (§Q8)
│   │   │   ├── validate.rs     # bounds + subseed reroll ≤5 (§Q9)
│   │   │   └── bundles.rs      # per-tile input bundles (§Q7–Q8)
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
│   │       └── wfc.rs          # most-constrained fill, ≤8 retries, relaxed fallback (§Q12)
│   ├── arda-render/src/
│   │   ├── lib.rs
│   │   ├── symbolic.rs         # built-in block style (logic/04 §Q14)
│   │   ├── tileset.rs          # tileset-manifest renderer (deferred until custom-art user — D9; stub error)
│   │   ├── carto.rs            # area/continent cartographic maps
│   │   └── json.rs             # versioned export schema (§Q14)
│   ├── arda/src/lib.rs         # facade: World::{generate,load}, Area, Cell, Block views (mockup/04)
│   └── arda-cli/src/main.rs    # clap: generate | export (mockup/01, 03)
└── tests/                      # workspace golden-world hashes (06-testing)
```

## Load-bearing seams (sketches)

**Subseeding** (`arda-core/src/rng.rs`) — the determinism spine
(arch §Q4); every random draw everywhere goes through this:

```rust
pub struct SeedKey { pub tier: Tier, pub stage: Stage, pub x: i32, pub y: i32, pub attempt: u8 }
pub fn rng(world_seed: u64, k: SeedKey) -> ChaCha8Rng {
    // blake3(world_seed ‖ k) → 32-byte key; ChaCha8 stream is portable
    ChaCha8Rng::from_seed(derive(world_seed, k))
}
```

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

**Formats** (`arda-core/formats/cells.rs`): fixed-layout row = one
`#[repr(C)]`-documented struct serialized field-by-field little-endian
(no `unsafe` transmute — code-prefs §Q1); round-trip property tests.

**Facade** (`crates/arda/src/lib.rs`): thin re-exports; `World::load`
eager manifest, `OnceLock`-cached lazy areas, lazy block decompression
(logic/05).

## Build order

Each step: failing test first (code-prefs §Q6), then minimum code,
then its verification. Conventional commit per step.

| # | Step | Verification |
|---|---|---|
| 0 | Workspace scaffold: five crates, lints (`deny(unsafe_code)`, missing_docs), CI (3-OS test+clippy+fmt+deny), Dockerfile stub | `cargo test --workspace` green in CI on 3 OSes |
| 1 | `arda-core`: coords, fixed-point, rng, error enums, config | unit tests incl. rng reference vectors (same key → same stream) |
| 2 | `arda-core::formats`: manifest + cells + objects + blocks codecs | round-trip property tests; version-skew refusal test |
| 3 | **Walking skeleton** (arch §Q8): micro-continent config (8 tiles) through minimal-but-real stages — kinematic-lite tectonics pass, relief+water only area, ~24-tile WFC subset, symbolic PNG + JSON export, load round-trip | skeleton runs end to end; blake3 stage hashes identical on 3 OSes in CI — **`status: formalized` lands here** |
| 4 | Spike S1 then continent full: plates → coupled tectonics/erosion → coast → climate → hydrology → people → naming → validate+reroll → bundles (logic/01 steps 1–10) | S1 eyeball + Horton on fixtures; unit tests per rule; continent stage ≤30 min release-mode (arch §Q7) |
| 5 | Area full: seven stages at artifact fidelity, pinned edges, cross-tile agreement | per-rule unit tests from logic/02; two adjacent fixture tiles agree on shared edge byte-for-byte |
| 6 | Spike S2 then blocks full: 200+ tile vocabulary, constraints, WFC ≤8 + relaxed marks | S2 convergence stats on hard cells; continuity test: river/road entry=exit across fixture edges |
| 7 | Society stage (`logic/06`, build §Q2–Q4): realms (cost-distance allegiance + border snap), building objects from the block layout function, NPC notables with SRD 5.1-style sheets + on-demand commoners; NOTICE file (CC-BY-4.0 attribution) | partition invariants (every land cell in exactly one realm); every notable housed; sheet determinism test |
| 8 | Tile attributes + POI layer (build §Q5): vocabulary attribute table, sparse per-block POIs | legend JSON carries attrs; POI determinism test |
| 9 | Export full: carto renders, block symbolic render, versioned JSON incl. realms/buildings/npcs/poi | byte-identical re-export test; refusal cases (logic/04); export benches ≤60 s area / ≤5 s block |
| 10 | Facade + CLI + docker + release workflow | `mockup/01`/`03` transcripts reproduced; `World::load` ≤100 ms bench; image runs `generate` via volume |
| 11 | `arda serve` (`mockup/06`, build §Q1): tiny_http sync server, read-only endpoints reusing export renderers; docker EXPOSE | endpoint tests against a fixture world; ETag byte-identity; refuses partial worlds |
| 12 | Statistical validation suite as CI gate (Horton/Hack/rank-size/sinuosity/farmland) + full-batch measurement | gates green on fixture continents; default-continent batch time recorded vs ≤12 h target |

New crate paths for the additions: `arda-gen/src/society/{realms,buildings,npcs}.rs`,
`arda-core/src/tiles.rs` gains the attribute table, `arda-cli` gains
`serve.rs` (tiny_http, MIT — sync per arch §Q3).

Deferred (D9, not planned): tileset-manifest renderer beyond a stub,
`--memory` tuning beyond a simple areas-in-flight cap, region-file
front door, runtime in-process game API.
