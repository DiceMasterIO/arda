---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: e26034835289
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/Cargo.toml", ":(top)crates/*/src/**", ":(top).github/**"]
capstone_version: 6.4
---

# Conventions

## Paradigm

Data-oriented functions transform owned grids; mutable vectors stay within stage execution and accepted continent data is shared by reference across Rayon area workers (`crates/arda-gen/src/orchestrator.rs:96`, `crates/arda-gen/src/area/mod.rs:656`). The facade wraps loaded data in `World`/`Area` with accessors (`crates/arda/src/lib.rs:146`). Normative requirements remain in `standards.md`; this chapter records their implementation.

## Typing

Rust 2021 with inherited workspace lints: `unsafe_code = "deny"`, `missing_docs = "deny"`; Clippy `all`, `unwrap_used`, `expect_used` denied, selected cast/passing/semicolon lints warn (`Cargo.toml:7`, `Cargo.toml:31`). CI promotes warnings to errors (`.github/workflows/ci.yml:25`). `HeightMm`, `TempCentiC`, `RainfallMm`, `DischargeMilli` are scaled integer wrappers, not a general Q-format implementation (`crates/arda-core/src/fixed.rs:9`). Coordinate newtypes and enums constrain grid crossings and closed sets (`crates/arda-core/src/coords.rs:17`, `crates/arda-core/src/cell.rs:11`).

Counts below scan non-comment lines in tracked `crates/*/src/*.rs` and nested modules before their test modules; integration tests/benches are excluded. Locations show the first four occurrences; `rg` over the named source directories locates the remainder.

| Escape hatch / abstraction | Count | Locations |
|---|---|---|
| as casts | 105 | `crates/arda-core/src/config.rs:135`, `crates/arda-core/src/config.rs:142`, `crates/arda-core/src/coords.rs:71`, `crates/arda-core/src/coords.rs:71` (remaining occurrences distributed across simulation/codecs) |
| allow attributes | 17 | `crates/arda-core/src/config.rs:133`, `crates/arda-core/src/config.rs:140`, `crates/arda-core/src/formats/overview.rs:154`, `crates/arda-gen/src/area/erosion.rs:58` (remaining occurrences distributed across simulation/codecs) |
| unwrap/expect calls | 0 | None found in `crates/*/src/` |
| unsafe blocks | 0 | None found in `crates/*/src/` |
| trait declarations | 0 | None found in `crates/*/src/` |

Casts and explicit allow attributes remain in fixed-grid indexing and arithmetic; the selected cast lints are not an absolute ban. Tests opt out of unwrap/expect bans (`crates/arda/src/lib.rs:7`). Integer widening, `try_from`, clamping and `unwrap_or` fallbacks are observed in generation and codecs (`crates/arda-gen/src/orchestrator.rs:208`, `crates/arda-core/src/formats/overview.rs:58`). No ambient random/time source is called by the simulation; keyed BLAKE3 → ChaCha8 is the RNG path (`crates/arda-core/src/rng.rs:84`).

## Error handling

Libraries expose `ConfigError`, `FormatError`, `LoadError`, `GenError`, `RenderError`, and `ExportError` through `Result`; derives use thiserror (`crates/arda-core/src/error.rs`, `crates/arda-gen/src/orchestrator.rs:25`, `crates/arda/src/lib.rs:37`). CLI wraps failures with anyhow context and returns its `Result` from main, printing progress with `println!` (`crates/arda-cli/src/main.rs:100`, `crates/arda-cli/src/main.rs:215`). No library logging framework is installed (`crates/*/Cargo.toml`).

The designed no-swallowed-errors rule has observed exceptions: JSON serializers use `unwrap_or_default`, overview uses `.ok()` inside `filter_map`, and occupied-directory detection defaults a read error to false (`crates/arda-render/src/json.rs:178`, `crates/arda/src/lib.rs:96`, `crates/arda-gen/src/orchestrator.rs:138`). Indexing relies on grid invariants and can panic; no custom panic-to-tile-error boundary is installed in the orchestrator.

## Dependency injection

Plain arguments supply config, seed, grids and output paths; no container or mutable global registry exists (`crates/arda-gen/src/orchestrator.rs:137`). `SeedKey` carries tier, stage, coordinates and attempt; each RNG stream hashes these with the world seed, preserving independence from worker completion order (`crates/arda-core/src/rng.rs:50`). Block archives use BTreeMap coordinate order and JSON uses struct field order for stable encoding (`crates/arda-core/src/formats/blocks.rs:100`, `crates/arda-render/src/json.rs:60`).
