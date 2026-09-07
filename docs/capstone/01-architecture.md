---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: e87cc34b6692
paths_covered: [":(top)Cargo.toml", ":(top)crates/**", ":(top)Dockerfile"]
capstone_version: 6.4
---

# Architecture

## Layers

| Crate | Directory | Dependencies and responsibility |
|---|---|---|
| arda-core | `crates/arda-core/` | Types, deterministic PRNG, binary codecs and manifest; no internal dependencies. |
| arda-gen | `crates/arda-gen/` | Continent → area → block generation; depends on arda-core. |
| arda-render | `crates/arda-render/` | Stored cells/blocks → PNG or JSON; depends on arda-core, never arda-gen. |
| arda | `crates/arda/` | Public generation, loading, query and export facade; depends on all three libraries. |
| arda-cli | `crates/arda-cli/` | clap argument dispatch; depends on arda. |

Dependency direction is enforced by `crates/*/Cargo.toml`. The root package hosts golden tests only (`Cargo.toml:56`). The designed five-crate split is implemented; this keeps rendering independent of simulation.

## Module boundaries

`arda-core::formats` owns byte encoding/decoding; filesystem writes also occur in the orchestrator and facade (`crates/arda-gen/src/orchestrator.rs:84`, `crates/arda/src/lib.rs:59`). `arda-gen::continent` owns coarse relief, climate, hydrology and bundles; `area` owns relief, erosion, fill, rainfall and water composition; `block` owns constraints and WFC. Interior stage ordering is observed in `crates/arda-gen/src/area/mod.rs:656`, not enforced by separate crates.

The design's settlement, land-use, road, naming and society modules do not exist in `crates/arda-gen/src/`; their rules remain in `logic/`. `TileBundle` has climate and river inputs but no settlement-density or road-exit fields (`crates/arda-gen/src/continent/bundles.rs:127`).

## Entry points

| Entry | Dispatch / definition | Behavior |
|---|---|---|
| CLI generate | `crates/arda-cli/src/main.rs:100` | Config → generate → counts printed. |
| CLI preview | `crates/arda-cli/src/main.rs:129` | Generate `out/world`, load it, export overview PNG. |
| CLI export | `crates/arda-cli/src/main.rs:157` | Block first, otherwise overview, otherwise area; PNG or JSON for area/block. |
| Library generation | `crates/arda/src/lib.rs:22` | Returns `Result<Manifest, GenError>`. |
| Library loading and queries | `crates/arda/src/lib.rs:223` | Eager load into `World`; area/cell/block lookups. |
| Library exports | `crates/arda/src/lib.rs:59`, `crates/arda/src/lib.rs:93`, `crates/arda/src/lib.rs:117` | Return written `PathBuf` or `ExportError`. |

The complete command enum has Generate, Preview, Export; the designed synchronous `serve` command remains unimplemented (`crates/arda-cli/src/main.rs:23`).

## Communication

No HTTP listener, queue, event bus or outbound client is registered in `crates/arda-cli/src/main.rs:215`. CLI and libraries communicate through in-process calls:

| Caller → receiver | Request payload | Return payload |
|---|---|---|
| CLI/facade → generate_world | seed: u64, config: GenerateConfig, out: &Path; all required | Manifest or GenError (`crates/arda-gen/src/orchestrator.rs:137`) |
| orchestrator → bundle_for → generate_area | seed, Continent, AreaCoord → TileBundle; all required | (AreaCells, AreaObjects) (`crates/arda-gen/src/orchestrator.rs:96`) |
| area/block data → renderer | AreaCells or Block; JSON also Manifest and area coordinates | PNG bytes / RenderError or JSON String (`crates/arda/src/lib.rs:59`, `crates/arda/src/lib.rs:117`) |
| facade → codecs | path: &str, bytes: &[u8]; required | typed layer or FormatError (`crates/arda-core/src/formats/`) |

Payload field tables are in `02-models.md`. Disk handoffs are manifest JSON, little-endian layers, and zstd block archives; no sibling-repository contract is identified in these entry points.

## Composition

`crates/arda-cli/src/main.rs:215` parses clap and dispatches. `crates/arda-gen/src/orchestrator.rs:137` accepts a continent candidate, shares one immutable `Continent` among Rayon area workers, writes continent layers and stamps the manifest last. Dependencies are plain arguments, with no DI container. Areas need no neighboring area's output, preserving order-independent generation.

## Frontend

No human-facing UI: the chosen surfaces are CLI and Rust API; generated PNGs are export artifacts, not application views (`crates/arda-cli/src/main.rs:23`).
