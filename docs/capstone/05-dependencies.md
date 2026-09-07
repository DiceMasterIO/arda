---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: fecfaee8f7ea
paths_covered: [":(top)Cargo.toml", ":(top)Cargo.lock", ":(top)crates/*/Cargo.toml", ":(top)rust-toolchain.toml", ":(top)deny.toml", ":(top).github/**"]
capstone_version: 6.4
---

# Dependencies

## Runtime dependencies

Installed declarations come from `Cargo.toml:15` and `crates/*/Cargo.toml`; exact resolutions from `Cargo.lock`. License labels and the selection rationale are carried forward from the recorded August 2026 stack decisions, not newly researched or re-vetted here. No paid service was selected.

| Package | Declared range / retained floor | Locked version | Recorded license | Role / status |
|---|---|---|---|---|
| `zstd` | 0.13 | 0.13.3 | MIT | Block compression; arda-core |
| `png` | 0.17 | 0.17.16 | MIT/Apache-2.0 | PNG encoding; arda-render |
| `serde` | 1.0 | 1.0.229 | MIT/Apache-2.0 | Derives for manifest/config and export DTOs |
| `serde_json` | 1.0 | 1.0.151 | MIT/Apache-2.0 | JSON manifest and exports |
| `rand_chacha` | 0.3 | 0.3.1 | MIT/Apache-2.0 | ChaCha8 behind keyed RNG; arda-core |
| `rand_core` | 0.6 | 0.6.4 | MIT/Apache-2.0 | RNG traits; arda-core and arda-gen |
| `rayon` | 1.10 | 1.12.0 | MIT/Apache-2.0 | Area fan-out; arda-gen |
| `clap` | 4.5 | 4.6.6 | MIT/Apache-2.0 | CLI derive parsing |
| `tiny_http` | 0.12 | Not installed | MIT/Apache-2.0 | Picked but not yet installed: planned synchronous serve |
| `thiserror` | 1.0 | 1.0.69 | MIT/Apache-2.0 | Library error derives |
| `anyhow` | 1.0 | 1.0.104 | MIT/Apache-2.0 | CLI context only |
| `blake3` | 1.5 | 1.8.7 | CC0/Apache-2.0 | Runtime subseed derivation and dev golden hashing |

`tiny_http` remains the chosen future HTTP dependency, with the synchronous design avoiding an async runtime (`mockup/06-serve.md`). Handwritten `Display`/`Error` was rejected in favor of thiserror derives; anyhow is restricted to the CLI by the crate manifests. BLAKE3 was promoted from dev-only hashing to runtime subseed derivation (`crates/arda-core/Cargo.toml`, `crates/arda-core/src/rng.rs:84`).

Simulation arithmetic, noise, erosion, hydrology and WFC were selected for implementation under repository control to preserve bit-exact behavior. The `fixed` crate was rejected in favor of integer newtypes; current wrappers store scaled integers rather than generic Q formats (`crates/arda-core/src/fixed.rs`, `crates/arda-gen/src/`). The recorded adoption bar is permissive licensing, more than one year of maintenance, no platform-variant sim arithmetic and a roughly 40-crate soft cap (`standards.md`). That cap is a design constraint, not a claim about the full transitive lockfile count.

## Dev and tooling

| Package | Role |
|---|---|
| criterion 0.5 (locked 0.5.1) | Area erosion and continent benchmarks; default features disabled, cargo_bench_support enabled (`Cargo.toml:29`, `crates/arda-gen/Cargo.toml`). |
| blake3 | Golden file fingerprints as well as runtime RNG keys (`tests/golden_world.rs`, `crates/arda-core/Cargo.toml`). |
| rustfmt / clippy | Stable toolchain components; default formatter and CI warning denial (`rust-toolchain.toml`, `.github/workflows/ci.yml`). |
| cargo-deny | License/advisory check through CI action; policy in `deny.toml`. |

## External services

No runtime API, database or broker connection is implemented in `crates/*/src/`. GitHub Actions supplies CI (`.github/workflows/ci.yml`); crates.io and GHCR remain the selected release destinations, but no release workflow exists. Standard registries were chosen for low exit cost; there is no service-specific data migration in the runtime design (`07-operations.md`).
