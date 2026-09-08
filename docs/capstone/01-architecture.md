---
generated_at_commit: 311829e4e5c9
content_hash: efac7c44ca05
paths_covered: [":(top)Cargo.toml", ":(top)crates/**", ":(top)Dockerfile"]
generated_date: 2026-09-08
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# Architecture

## Layers

| Crate | Directory | Dependencies and responsibility |
|---|---|---|
| arda-core | `crates/arda-core/` | Domain identities, deterministic PRNG, explicit binary codecs and manifest; no internal dependency. |
| arda-gen | `crates/arda-gen/` | Continent, canonical physical preparation, shared annual water, immutable area composition and sampled blocks; depends on core. |
| arda-render | `crates/arda-render/` | Stored cells/objects/blocks to PNG or JSON; depends on core, never the generator. |
| arda | `crates/arda/` | Public generation, manifest-first loading, lazy queries and exports; composes the three libraries. |
| arda-cli | `crates/arda-cli/` | clap argument dispatch through the public facade. |

The manifests enforce this dependency direction. The root package hosts golden-world integration tests (`Cargo.toml:56`). There is no runtime HTTP service or frontend.

## Module boundaries

`arda-core::formats` owns format4 record encoding and validation; `arda-core::hydrology` owns shared identities, physical geometry and representative annual records. Private simulation scratch formats live with their generator stages, separately from published world formats (`crates/arda-core/src/formats/mod.rs:17`, `crates/arda-gen/src/hydrology/private_rows.rs:1`).

`arda-gen::continent` produces coarse relief/climate/bundles. Its normalized radial coast mask avoids the former square contour construction, and area detail uses four aligned integer octave lattice spacings of 40, 20, 10 and 5 fine cells. `area::prepare::SharedTerrain` initializes and evolves one complete modeled fine rectangle; `area::evolution` owns the shared erosion kernel. Prepared areas and partial fringe tiles are immutable slices of that completed surface with sampled forcing. `hydrology` implements fine ocean/receiver ownership, witnessed saddles/MST/hierarchy, annual support, signed flow, metrics and extraction. Concrete file stores and complete resource admission live under `orchestrator/`. Final area composition consumes completed shared authority (`crates/arda-gen/src/continent/coast.rs`, `crates/arda-gen/src/continent/area_detail.rs`, `crates/arda-gen/src/area/prepare.rs`, `crates/arda-gen/src/area/evolution.rs`, `crates/arda-gen/src/orchestrator/shared_solve.rs`, `crates/arda-gen/src/hydrology/area_output.rs`).

The public tile-only `area::generate_area` helper remains diagnostic and returns `LocalAreaObjects`, which have no format4 encoder; its historical coarse-inflow/lake-threshold path does not publish world water (`crates/arda-gen/src/area/local_objects.rs:1`). The block module retains its24-tile WFC. Society, roads, naming, coherent buildings and NPC generation remain deferred designs (`crates/arda-gen/src/block/mod.rs:1`, `logic/06-society-generation.md`).

## Entry points

| Entry | Definition / dispatch | Behavior |
|---|---|---|
| CLI generate | `crates/arda-cli/src/main.rs:102` | Validate config, generate and save a world, print counts. |
| CLI preview | `crates/arda-cli/src/main.rs:129` | Generate `out/world`, load it and export an overview. |
| CLI export | `crates/arda-cli/src/main.rs:157` | Saved block, explicit overview, or area; PNG/JSON, with optional area-only detail. |
| Library generation | `crates/arda/src/lib.rs:24`, `:33` | Generate with default or explicit `HydrologyLimits`. |
| Library load/query | `crates/arda/src/world.rs:88`, `:187`, `:203` | Manifest load; cached area or owned uncached area read; independent block cache. |
| Library exports | `crates/arda/src/lib.rs:78`, `:93`, `:144`, `:168` | Area preview/detail, overview and block exports returning a path or typed error. |

The complete command enum remains Generate, Preview, Export; `serve` remains unimplemented (`crates/arda-cli/src/main.rs:23`).

## Communication

All calls are in-process; there is no queue, event bus or outbound client (`crates/arda-cli/src/main.rs:224`).

| Caller → receiver | Required request payload | Return payload |
|---|---|---|
| facade → generate_world_with_limits | seed:u64, GenerateConfig, output:&Path, HydrologyLimits | Manifest or GenError (`crates/arda-gen/src/orchestrator.rs:338`) |
| orchestrator → SharedTerrain::build | seed, Continent, PreparedDomain | Complete evolved SharedTerrain or HydrologyError (`crates/arda-gen/src/area/prepare.rs`) |
| orchestrator → prepare_area_terrain | SharedTerrain, Continent, TileBundle, PreparedExtent | Immutable sliced PreparedTerrain or HydrologyError (`crates/arda-gen/src/area/prepare.rs`) |
| orchestrator → shared_solve::solve | PreparedReader, continent grid/climate, config, scratch path, SharedLimits | SharedArtifacts or SharedError (`crates/arda-gen/src/orchestrator/shared_solve.rs:385`) |
| shared index → area composition | PreparedTile, completed routing/flow, FinalIndex, GlobalLake records, AreaLimits | AreaCells/AreaObjects or AreaError (`crates/arda-gen/src/hydrology/area_output.rs:1`) |
| facade → render/serialize | saved cells/objects plus global origin/scale, or Block; JSON also Manifest | PNG bytes/RenderError or versioned JSON (`crates/arda/src/lib.rs:93`, `:168`) |
| facade → stored codecs | requested layer path and bounded bytes | typed layer or LoadError (`crates/arda/src/world.rs:203`) |

Payload fields and wire layouts are in `02-models.md`. Published disk handoffs are manifest JSON, explicit little-endian layers and zstd block archives. Scratch remains a private transaction detail; it is not a resume or interchange API.

## Composition

`main` dispatches through the facade. The generator admits the full request before reserving a fresh output, evolves the complete modeled terrain once, persists immutable tile slices, releases the dense terrain, runs one shared annual-water solve, indexes its final records, then composes and writes each exported area. Global/coarse layers precede final manifest publication (`crates/arda-cli/src/main.rs:224`, `crates/arda-gen/src/orchestrator.rs:338`).

Preparation and composition currently execute sequentially. Uplift, fractional MFD contributing area, creep and collapse cross publication-area boundaries; only the real modeled outer rim remains fixed. Incision is solved implicitly downstream first against updated receiver beds. This bounded implementation does not use Rayon area fan-out; the dependency remains installed. No final area reads another final area file. Physical neighbor data comes from the shared prepared domain, so exact shared crossings do not require output-order dependence. Dependencies are plain arguments and real memory/file storage traits, with no DI container.

Admission includes the dense terrain inputs and scratch, 53 bytes per modeled fine cell plus container headers on the current 64-bit layout, conservatively summed with water-stage reservations. Default and MICRO requests fit the default 16 GiB allowance; a 4000×4000 km request requires explicitly larger limits and is refused before output creation under the defaults. Visual acceptance of the current terrain correction remains open while its full candidate validation runs; earlier candidate02 results describe the preceding implementation (`crates/arda-gen/src/area/evolution.rs`, `crates/arda-gen/src/orchestrator/generation_limits.rs`, `crates/arda-gen/src/orchestrator/generation_limits_tests.rs`).

## Frontend

No human-facing UI: CLI and Rust API are the chosen current surfaces; PNGs are export artifacts. A consuming VTT, asset loading and browser chunking are outside this implementation.
