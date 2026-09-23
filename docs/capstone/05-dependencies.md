---
generated_at_commit: 757b2ab5418b
generated_date: 2026-09-23
content_hash: 1692736c95fb
paths_covered: [":(top)Cargo.toml", ":(top)Cargo.lock", ":(top)crates/*/Cargo.toml", ":(top)rust-toolchain.toml", ":(top)deny.toml", ":(top).github/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# Dependencies

## Runtime dependencies

Installed declarations come from `Cargo.toml:15` and `crates/*/Cargo.toml`; exact resolutions from `Cargo.lock`. License labels and the selection rationale are carried forward from the recorded August 2026 stack decisions, not newly researched or re-vetted here. No paid service was selected. The workspace package license is Apache-2.0 (`Cargo.toml:8`); dependency license labels below are separate.

| Package | Declared range / retained floor | Locked version | Recorded license | Role / status |
|---|---|---|---|---|
| `zstd` | 0.13 | 0.13.3 | MIT | Block compression; arda-core |
| `png` | 0.17 | 0.17.16 | MIT/Apache-2.0 | PNG encoding; arda-render |
| `serde` | 1.0 | 1.0.229 | MIT/Apache-2.0 | Derives for manifest/config and export DTOs |
| `serde_json` | 1.0 | 1.0.151 | MIT/Apache-2.0 | JSON manifest and exports |
| `rand_chacha` | 0.3 | 0.3.1 | MIT/Apache-2.0 | ChaCha8 behind keyed RNG; arda-core |
| `rand_core` | 0.6 | 0.6.4 | MIT/Apache-2.0 | RNG traits; arda-core and arda-gen |
| `rayon` | 1.10 | 1.12.0 | MIT/Apache-2.0 | Retained generator dependency; the shared world pipeline currently prepares/composes sequentially |
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
| serde_json | Existing workspace dependency newly used by generator dev fixtures; no runtime dependency added (`crates/arda-gen/Cargo.toml`). |
| blake3 | Golden file fingerprints as well as runtime RNG keys (`tests/golden_world.rs`, `crates/arda-core/Cargo.toml`). |
| rustfmt / clippy | Stable toolchain components; default formatter and CI warning denial (`rust-toolchain.toml`, `.github/workflows/ci.yml`). |
| cargo-deny | License/advisory check through CI action; policy in `deny.toml`. |

The forcing lookup tables and MFD numerical data are checked in under generator modules; `tools/generate_water_forcing_tables.py` is an offline standard-library maintenance tool, not a runtime service. Rust1.96.1 now supplies the explicit latest-stable-minus-two CI check (`.github/workflows/ci.yml:18`). No dependency version or new runtime package was introduced by the area-water or Atlas/terrain corrections. CI runs the golden/repeatability tests within its single workspace invocation rather than a second redundant determinism step (`.github/workflows/ci.yml:14`).

## External services

No runtime API, database or broker connection is implemented in `crates/*/src/`. GitHub Actions supplies CI (`.github/workflows/ci.yml`); crates.io and GHCR remain the selected release destinations, but no release workflow exists. Standard registries were chosen for low exit cost; there is no service-specific data migration in the runtime design (`07-operations.md`).

## Forcing data provenance

The checked-in forcing tables use five observed monthly profiles as procedural climate analogs. The offline generator preserves their reported rounding, apportions rainfall with exact integer totals, and supplies temperature anomalies and astronomical daylength; normal builds and generation do not fetch these sources. These inputs support the chosen static annual convention, not a claim of global calibration. The runtime annual rule and its omitted seasons/snow/groundwater are documented in the area-generation scenario.

| Profile | Source and period | Input limitation retained |
|---|---|---|
| Singapore, Changi | [Meteorological Service Singapore, 1991–2020](https://www.weather.gov.sg/climate-climate-of-singapore/) | An equatorial maritime analog, not every tropical climate. |
| Valencia | [AEMET, 1981–2010](https://www.aemet.es/es/serviciosclimaticos/datosclimatologicos/valoresclimatologicos?l=8414A) | Rounded monthly precipitation sums to 459 mm; the separately rounded annual page total is 461 mm. |
| Heathrow | [Met Office, 1991–2020](https://www.metoffice.gov.uk/research/climate/maps-and-data/location-specific-long-term-averages/gcpsvg3nc) | Monthly temperature is the midpoint of reported mean daily maximum/minimum. |
| Ottawa | [ECCC, 1991–2020](https://climate.weather.gc.ca/climate_normals/results_1991_2020_e.html?climate_id=6105976&dispBack=0&lstProvince=ON&searchType=stnProv) | Total monthly precipitation is used; runtime snow storage is not modeled. |
| Yellowknife Hydro | [ECCC, 1971–2000, climate ID 2204200](https://climate.weather.gc.ca/climate_normals/results_e.html?climate_id=2204200) | Older class-C normals with poorer coverage; a dry-cold analog, not current weather. |

The evaporation expression follows Hamon 1961 as used in [USGS Open-File Report 2025-1021, equations 4–5](https://pubs.usgs.gov/publication/ofr20251021/full). That report uses it as a lower estimate alongside a radiation method; it does not establish universal accuracy. The astronomical daylength formula is explicit in `tools/generate_water_forcing_tables.py`; consumers and integer table values are in `crates/arda-gen/src/hydrology/{forcing.rs,forcing_tables.rs}`. Scientific alternatives investigated before the final static scope do not add runtime snow, leakage or subsurface stores.
