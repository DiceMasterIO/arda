---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: e9cc03d1397a
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/src/**", ":(top)crates/*/tests/**", ":(top)crates/*/benches/**", ":(top)tests/**", ":(top).github/**"]
capstone_version: 6.4
---

# Testing

## Layout

| Suite | Evidence | Coverage |
|---|---|---|
| Inline unit tests | `crates/*/src/` | Coordinates, arithmetic, RNG, codecs, stage rules, renderer choices and refusals. |
| Area drainage | `crates/arda-gen/tests/drainage_invariants.rs` | Termination, lake thresholds/outlets, zeroed non-land flow, channel partition, Strahler continuity, determinism and seams. |
| Continent hydrology | `crates/arda-gen/tests/continent_hydrology.rs:30` | Persistence, ocean-reaching paths, courses, catchment/discharge, determinism. |
| Cross-tile | `crates/arda-gen/tests/cross_tile.rs:355` | Independent inflow derivation, seam alignment, inflow effect, composed climate rules, synthetic seam lakes. |
| Measurements | `crates/arda-gen/tests/continent_measures.rs`, `crates/arda-gen/tests/anisotropy_probe.rs` | Ignored exploratory rainfall/Horton and directional probes; not default statistical gates. |
| Facade/CLI | `crates/arda/tests/round_trip.rs`, `crates/arda-cli/tests/cli.rs` | World round trips, exports and CLI refusal paths. |
| Golden world | `tests/golden_world.rs:69` | Fixed MICRO fingerprint and repeat-generation equality. |
| Benchmarks | `crates/arda-gen/benches/area_erosion.rs`, `crates/arda-gen/benches/continent_stage.rs` | Release area erosion budget 30 s; default continent stage budget 60 s. |

CI runs workspace tests and the golden gate on Linux/macOS/Windows (`.github/workflows/ci.yml:7`). `[profile.test] opt-level = 2` speeds simulation tests (`Cargo.toml:49`). This documentation run did not execute the Rust suite or re-bless hashes.

## Doubles

Pure stage fixtures and temporary directories are used; no mocking framework is declared in `crates/*/Cargo.toml`. Cross-tile tests independently derive expected inputs rather than using only the producer under test (`crates/arda-gen/tests/cross_tile.rs:151`).

## Coverage shape

The implemented gates emphasize structural drainage correctness, format refusal and determinism. The design's Horton/Hack/settlement rank-size/road sinuosity/farmland statistical acceptance suite, load/export latency benches, and full tactical continuity/layout checks are not implemented in `crates/*/tests/` and `crates/arda-gen/benches/`; the 200+ tile WFC and society stages also remain unbuilt. Recorded earlier pass counts and timings were historical runs, not validation of this checkout. Golden equality verifies reproducibility, not geographic or tactical realism (`tests/golden_world.rs:69`).
