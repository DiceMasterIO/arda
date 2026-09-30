---
generated_at_commit: 0fc9666b583b
content_hash: 992a8086d2f5
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/Cargo.toml", ":(top)crates/*/src/**", ":(top).github/**"]
generated_date: 2026-09-23
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-23-terrain-corrections@2026-09-23
---

# Conventions

## Paradigm

Data-oriented functions transform explicit owned or borrowed grids. Canonical preparation and final area composition are sequential; shared physical routing and annual water run between them. Mutation stays with a stage or its concrete file store, and final composition reads completed shared authority (`crates/arda-gen/src/orchestrator.rs:338`, `crates/arda-gen/src/orchestrator/shared_solve.rs:385`). The facade wraps saved data in manifest-first `World` and lazy `Area`/archive caches (`crates/arda/src/world.rs:73`). Normative requirements remain in `standards.md`.

## Typing

Rust 2021 inherits workspace `unsafe_code = "deny"`, `missing_docs = "deny"`, Clippy `all`, `unwrap_used` and `expect_used` denials, plus selected cast/passing/semicolon warnings. CI promotes warnings to errors (`Cargo.toml:7`, `Cargo.toml:31`, `.github/workflows/ci.yml`). Scaled integer wrappers and distinct global/area/local coordinate and hydrology identifier types prevent accidental grid/account mixing (`crates/arda-core/src/fixed.rs:9`, `crates/arda-core/src/coords.rs:169`, `crates/arda-core/src/hydrology.rs:1`).

The September 23 source-text inventory covers 155 Rust files under `crates/`, including test modules, integration tests, benches and examples. Counts use regex matches after stripping `//` line comments: `as` tokens, explicit allow attributes, direct `.unwrap()`/`.expect()` calls, unsafe blocks and trait declarations. They are whole-crates lexical counts, not compiler-parsed or production-only counts; `unwrap_or` variants are excluded.

| Escape hatch / abstraction | Count | Example locations |
|---|---|---|
| as casts | 470 | `crates/arda-core/src/config.rs:135`, `crates/arda-core/src/config.rs:142`, `crates/arda-core/src/continent.rs:112`, `crates/arda-core/src/coords.rs:71` |
| allow attributes | 80 | `crates/arda/src/lib.rs:7`, `crates/arda/tests/area_exports.rs:2`, `crates/arda/tests/round_trip.rs:6`, `crates/arda-cli/tests/cli.rs:5` |
| unwrap/expect calls | 2076 | `crates/arda/src/world.rs:362`, `crates/arda/src/world.rs:378`, `crates/arda/src/world.rs:383`, `crates/arda/src/world.rs:389` |
| unsafe blocks | 0 | None found |
| trait declarations | 5 | `crates/arda-core/src/formats/hydrology.rs:108`, `crates/arda-core/src/formats/hydrology.rs:421`, `crates/arda-gen/src/hydrology/fine_flow.rs:76`, `crates/arda-gen/src/hydrology/hierarchy.rs:134` |

Tests explicitly allow unwrap/expect; strict workspace Clippy passes. Production arithmetic documents bounded casts, widens intermediates, uses checked conversions and typed capacity failures; older fixed-grid modules also retain clamping/`unwrap_or` fallbacks. No unsafe block is present. Storage traits have real memory/file implementations and format traits serve multiple concrete record types (`crates/arda-gen/src/hydrology/routing.rs:210`, `crates/arda-gen/src/hydrology/fine_flow.rs:76`, `crates/arda-gen/src/hydrology/hierarchy.rs:134`, `crates/arda-core/src/formats/hydrology.rs:108`). No ambient random/time source enters simulation; keyed BLAKE3→ChaCha8 remains the RNG path (`crates/arda-core/src/rng.rs:84`).

Tectonic boundary belts use capped exact Euclidean distances on the 4 km grid, retaining Q10 fractional distance through their cubic profiles. The forced ocean rim preserves negative depths with `h.min(-1)`. The 1 km surface then receives 25 nine-point hillslope-creep passes; stream-power incision occurs during the existing 40-step shared 100 m evolution (`crates/arda-gen/src/continent/tectonics.rs:208`, `crates/arda-gen/src/continent/tectonics.rs:220`, `crates/arda-gen/src/continent/mod.rs:209`, `crates/arda-gen/src/continent/erode.rs:19`, `crates/arda-gen/src/area/evolution.rs:189`).

## Error handling

Libraries expose typed `Result` errors. Nested shared-stage, scratch, geometry and codec failures retain original `Error::source` causes through the public generator/facade. CLI adds anyhow context and prints progress; no library logging framework is installed (`crates/arda-gen/src/orchestrator.rs:49`, `crates/arda-gen/src/orchestrator/error_chain_tests.rs`, `crates/arda/src/lib.rs:56`, `crates/arda-cli/src/main.rs:224`).

The earlier swallowed overview-read error, occupied-directory read fallback and empty area-JSON fallback are fixed. Overview now propagates reads, output reservation propagates directory errors, and area JSON returns a typed serializer failure (`crates/arda/src/lib.rs:144`, `crates/arda-gen/src/orchestrator/publication.rs:58`, `crates/arda-render/src/json.rs:130`). Existing block JSON retains its simple infallible-shape wrapper and `unwrap_or_default` fallback (`crates/arda-render/src/json.rs:221`). Invariant-backed indexing can still panic; there is no custom panic-recovery boundary.

## Dependency injection

Plain arguments carry config, seed, grids, limits, concrete stores and output paths. There is no container or mutable global service registry. SeedKey carries tier/stage/coordinates/attempt; deterministic BTreeMaps and explicit canonical sorting govern saved identity order; JSON fields follow struct order (`crates/arda-core/src/rng.rs:50`, `crates/arda-gen/src/hydrology/final_index.rs`, `crates/arda-render/src/json.rs:71`). Geometry/display scales do not change simulation data, and runtime resource capacities admit or refuse work rather than simplify geography.
