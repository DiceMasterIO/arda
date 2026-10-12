---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: a4c760303590
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top)Cargo.lock"
  - ":(top)crates/*/Cargo.toml"
  - ":(top)deny.toml"
  - ":(top)rust-toolchain.toml"
  - ":(top)apps/viewer/package.json"
  - ":(top)apps/viewer/package-lock.json"
  - ":(top)tools/art-gen/generate.mjs"
  - ":(top)tools/*.py"
---

# Dependencies

## Runtime dependencies

Shared declarations sit in `[workspace.dependencies]` (`Cargo.toml:11`); the 21 path entries there (`Cargo.toml:12`-`Cargo.toml:32`) are workspace crates at version 0.8.1, not third-party packages. Crates without a workspace entry declare their version in their own manifest (`crates/arda-server/Cargo.toml`, `crates/arda-art-import/Cargo.toml`). Locked versions come from `Cargo.lock`, which holds 227 packages, 24 of them workspace members (23 crates plus the root `arda-workspace-tests` package, `Cargo.toml:76`). Licenses are read from each locked crate's own `Cargo.toml` `license` field. The workspace's own license is Apache-2.0 (`Cargo.toml:8`).

### Rust, workspace-declared

| Package | Declared (site) | Locked | License | Used by / role |
|---|---|---|---|---|
| `ab_glyph` | 0.2 (`Cargo.toml:33`) | 0.2.32 | Apache-2.0 | `arda-settle` label rasterising over embedded Noto Serif fonts (`crates/arda-settle/src/render/font.rs:12`) |
| `zstd` | 0.13 (`Cargo.toml:34`) | 0.13.3 | MIT | Block payload compression in `arda-core` (`crates/arda-core/src/formats/blocks.rs:116`) and settlement outputs in `arda-settle` (`crates/arda-settle/src/output.rs:175`) |
| `png` | 0.17 (`Cargo.toml:35`) | 0.17.16 | MIT OR Apache-2.0 | PNG encode/decode in `arda-render`, `arda-tactical`, `arda-scene`, `arda-settle`, `arda-refine`, `arda-midzoom`, `arda-art-import`, `arda-server` |
| `flate2` | 1 (`Cargo.toml:36`) | 1.1.9 | MIT OR Apache-2.0 | Deflate in the server's tactical PNG encoder (`crates/arda-server/src/tactical/encode.rs:12`) |
| `crc32fast` | 1 (`Cargo.toml:37`) | 1.5.1 | MIT OR Apache-2.0 | PNG chunk CRCs in the same encoder (`crates/arda-server/src/tactical/encode.rs:76`) |
| `serde` (feature `derive`) | 1.0 (`Cargo.toml:38`) | 1.0.229 | MIT OR Apache-2.0 | Derives on manifests, configs, records and DTOs in most crates |
| `serde_json` | 1.0 (`Cargo.toml:39`) | 1.0.151 | MIT OR Apache-2.0 | JSON manifests, exports, sidecars and HTTP bodies |
| `schemars` | 1.2 (`Cargo.toml:40`) | 1.2.2 | MIT | JSON Schema derives behind the optional `schema` feature of `arda-ids`, `arda-names`, `arda-settle`, `arda-society`, `arda-town` (`crates/arda-ids/Cargo.toml`, `crates/arda-settle/Cargo.toml`); unconditional in `arda-server` (`crates/arda-server/Cargo.toml:41`), which enables those features (`crates/arda-server/Cargo.toml`) |
| `rand_chacha` | 0.3 (`Cargo.toml:41`) | 0.3.1 | MIT OR Apache-2.0 | ChaCha behind the keyed RNG; `arda-core`, `arda-gen` |
| `rand_core` | 0.6 (`Cargo.toml:42`) | 0.6.4 | MIT OR Apache-2.0 | RNG traits; `arda-core`, `arda-gen` |
| `rayon` | 1.10 (`Cargo.toml:43`) | 1.12.0 | MIT OR Apache-2.0 | Data parallelism in `arda-gen`, `arda-render`, `arda-tactical`, `arda-fields`, `arda-town`, `arda-people`, `arda-blocks`, `arda-midzoom`, `arda-art-import`, `arda-server` |
| `clap` (feature `derive`) | 4.5 (`Cargo.toml:44`) | 4.6.6 | MIT OR Apache-2.0 | CLI parsing for the six binaries (`arda-cli`, `arda-server`, `arda-settle`, `arda-town`, `arda-refine`, `arda-midzoom`) |
| `blake3` | 1.5 (`Cargo.toml:45`) | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | Runtime subseed derivation (`crates/arda-core/src/rng.rs:84`); stable ids in `arda-ids`, `arda-npc`, `arda-society`; ETags in `arda-server`; golden fingerprints in tests |
| `thiserror` | 1.0 (`Cargo.toml:46`) | 1.0.69 | MIT OR Apache-2.0 | Library error derives in every library crate; a second copy, 2.0.21, is locked as a transitive dependency |
| `anyhow` | 1.0 (`Cargo.toml:47`) | 1.0.104 | MIT OR Apache-2.0 | Error context in `arda-cli` only (`crates/arda-cli/Cargo.toml:23`) |

### Rust, crate-declared

| Package | Declared (site) | Locked | License | Role |
|---|---|---|---|---|
| `axum` (no default features; `http1`, `json`, `query`, `tokio`) | 0.8 (`crates/arda-server/Cargo.toml:31`) | 0.8.9 | MIT | HTTP router for `/v1` (`crates/arda-server/src/routes.rs:61`) |
| `hyper` (`http1`, `server`) | 1 (`crates/arda-server/Cargo.toml:43`) | 1.11.1 | MIT | HTTP/1 connection serving in the custom accept loop (`crates/arda-server/src/serve.rs:9`) |
| `hyper-util` | 0.1 (`crates/arda-server/Cargo.toml:44`) | 0.1.21 | MIT | Tokio IO/timer adapters and tower bridge (`crates/arda-server/src/serve.rs:10`) |
| `tokio` (`macros`, `net`, `rt-multi-thread`, `signal`, `sync`, `time`) | 1 (`crates/arda-server/Cargo.toml:45`) | 1.53.1 | MIT | Async runtime, listener, Ctrl-C shutdown, `spawn_blocking` (`crates/arda-server/src/main.rs:93`) |
| `tower-http` (`cors`) | 0.6 (`crates/arda-server/Cargo.toml:46`) | 0.6.11 | MIT | CORS layer restricted to local origins (`crates/arda-server/src/routes.rs:92`) |
| `ts-rs` | 12 (`crates/arda-server/Cargo.toml:47`) | 12.0.1 | MIT | TypeScript bindings for the viewer (`crates/arda-server/src/bindings.rs:37`) |
| `image-webp` | 0.2 (`crates/arda-server/Cargo.toml:34`) | 0.2.4 | MIT OR Apache-2.0 | WebP encoding of tactical images (`crates/arda-server/src/tactical/images.rs:73`) |
| `jpeg-decoder` (no default features) | 0.3 (`crates/arda-art-import/Cargo.toml:11`) | 0.3.1 | MIT OR Apache-2.0 | Reads raw JPEG art (`crates/arda-art-import/src/image_io.rs:64`) |
| `toml` | 0.8 (`crates/arda-art-import/Cargo.toml:17`) | 0.8.2 | MIT OR Apache-2.0 | Parses `import.toml` manifests (`crates/arda-art-import/src/manifest.rs:163`) |

### Recorded picks reconciled

| Package | Status |
|---|---|
| `tiny_http` 0.12 | Removed from the record. It was the picked-but-not-installed synchronous HTTP server for `arda serve`; it was never added to any manifest (`git log -S tiny_http` touches only docs) and is absent from `Cargo.lock`. HTTP serving shipped as the separate `arda-server` binary on `axum`/`hyper`/`tokio` (`crates/arda-server/Cargo.toml:31`-`crates/arda-server/Cargo.toml:45`). No `serve` subcommand exists in `arda-cli` (`crates/arda-cli/src/main.rs:34`). |

No other recorded pick is outstanding.

### Carried-forward stack rationale

These selection reasons come from the August 2026 stack decisions and are not re-vetted here.

- Handwritten `Display`/`Error` was rejected for `thiserror` derives; `anyhow` stays confined to the CLI binary (`crates/arda-cli/Cargo.toml:23`, no other manifest declares it).
- BLAKE3 was promoted from dev-only hashing to runtime subseed derivation (`crates/arda-core/src/rng.rs:84`).
- Simulation arithmetic, noise, erosion, hydrology and WFC are implemented in-repo to keep bit-exact behavior. The `fixed` crate was rejected for integer newtypes (`crates/arda-core/src/fixed.rs`). WFC is the in-repo crate `arda-wfc`, whose only dependency is `thiserror` (`crates/arda-wfc/Cargo.toml`).
- The adoption bar is permissive licensing, more than one year of maintenance, no platform-variant sim arithmetic and a soft cap of about 40 crates (`docs/capstone/standards.md:23`). The cap is a design constraint; the lockfile holds 203 third-party packages including transitive ones.
- The synchronous no-async-runtime serve design that motivated `tiny_http` no longer holds: `arda-server` runs on `tokio` with a hand-rolled accept loop that adds the header-read timeout and connection cap `axum::serve` lacks (`crates/arda-server/src/serve.rs:1`).

### Non-crate runtime assets

`arda-settle` embeds six Noto Serif TTF subsets under `crates/arda-settle/assets/fonts/`, licensed under the SIL Open Font License (`crates/arda-settle/assets/fonts/OFL.txt:1`).

## Dev and tooling

### Rust dev-dependencies

| Package | Role |
|---|---|
| `criterion` 0.5 (locked 0.5.1, MIT OR Apache-2.0; default features off, `cargo_bench_support` on; `Cargo.toml:48`) | `arda-gen` benches `area_erosion` and `continent_stage` (`crates/arda-gen/Cargo.toml:18`) |
| `jsonschema` 0.58 (locked 0.58.3, MIT; default features off; `crates/arda-server/Cargo.toml:53`) | Validates server responses against the served JSON Schemas in tests; brings in MIT-0 and Zlib transitive licenses (`deny.toml:2`) |
| `http-body-util` 0.1 (locked 0.1.5, MIT; `crates/arda-server/Cargo.toml:52`, `Cargo.toml:98`) | Reads response bodies in router tests |
| `tower` 0.5 `util` (locked 0.5.3, MIT; `crates/arda-server/Cargo.toml:54`, `Cargo.toml:102`) | `oneshot` router calls in tests |
| `tokio` `io-util` / `macros`, `rt-multi-thread` (`crates/arda-server/Cargo.toml:51`, `Cargo.toml:101`) | Async test runtime |
| `axum` 0.8 (`Cargo.toml:96`) | Root workspace tests drive the server router (`tests/e2e_village.rs`) |
| `blake3`, `serde_json`, `png`, `rayon` | Golden fingerprints and fixtures (`Cargo.toml:97`-`Cargo.toml:100`; dev sections of `arda-dungeon`, `arda-tactical`, `arda-gen`, `arda-ids`, `arda-names`) |
| Workspace crates as dev-deps | `arda-society` (`arda-blocks`, `arda-settle`), `arda-npc` (`arda-settle`), `arda-tactical` (`arda-midzoom`), `arda-names` (`arda-server`); root package pulls `arda`, `arda-core`, `arda-midzoom`, `arda-npc`, `arda-people`, `arda-server`, `arda-settle`, `arda-town` (`Cargo.toml:87`) |

### Toolchain and policy

| Tool | Role |
|---|---|
| Rust stable with `rustfmt`, `clippy` | Pinned channel (`rust-toolchain.toml`); CI also checks Rust 1.96.1 (`.github/workflows/ci.yml:22`) |
| Workspace lints | `unsafe_code`, `missing_docs` denied; clippy `all`, `unwrap_used`, `expect_used` denied plus selected pedantic warnings (`Cargo.toml:50`-`Cargo.toml:64`) |
| `cargo-deny` | Run by `EmbarkStudios/cargo-deny-action@v2` (`.github/workflows/ci.yml:39`); policy in `deny.toml` |
| Python 3 standard library | Offline table generators `tools/generate_water_forcing_tables.py` (writes forcing tables consumed by `crates/arda-gen/src/hydrology/forcing_tables.rs`) and `tools/generate_spectral_tables.py` (writes `crates/arda-gen/src/spectral_tables.rs`, `tools/generate_spectral_tables.py:6`); never run by a build |

### License policy (`deny.toml`)

| Setting | Value |
|---|---|
| Allowed licenses | MIT, MIT-0, Apache-2.0, Apache-2.0 WITH LLVM-exception, CC0-1.0, Unicode-3.0, BSD-3-Clause, BSD-2-Clause, ISC, Zlib (`deny.toml`); BSD-2-Clause and ISC were added to match the npm runtime list, no crate needs them today |
| Confidence threshold | 0.9 (`deny.toml:5`) |
| Duplicate versions | `warn` (`deny.toml:8`) |
| Yanked crates | `deny` (`deny.toml:11`) |
| Ignored advisory | RUSTSEC-2026-0192, `ttf-parser` unmaintained (locked 0.25.1 via `ab_glyph` → `owned_ttf_parser`); only embedded trusted fonts are parsed (`deny.toml:12`-`deny.toml:20`) |
| Sources | unknown registries and unknown git sources denied (`deny.toml:23`-`deny.toml:24`) |

### Viewer (`apps/viewer/package.json`)

Private package `arda-viewer` 0.8.1, Apache-2.0, ES modules (`apps/viewer/package.json:2`-`apps/viewer/package.json:7`). Locked versions from `apps/viewer/package-lock.json`.

| Package | Declared | Locked | License | Role |
|---|---|---|---|---|
| `leaflet` | ^1.9.4 | 1.9.4 | BSD-2-Clause | Map rendering (runtime) |
| `react` | ^19.3.0 | 19.3.0 | MIT | UI (runtime) |
| `react-dom` | ^19.3.0 | 19.3.0 | MIT | DOM renderer (runtime) |
| `vite` | ^8.3.1 | 8.3.1 | MIT | Dev server and bundler |
| `@vitejs/plugin-react` | ^6.1.1 | 6.1.1 | MIT | Vite React plugin |
| `typescript` | ~6.0.3 | 6.0.3 | Apache-2.0 | Type checking (`tsc -b`) |
| `vitest` | ^5.0.2 | 5.0.2 | MIT | Unit tests |
| `eslint` | ^10.11.0 | 10.11.0 | MIT | Lint |
| `@eslint/js` | ^10.0.1 | 10.0.1 | MIT | ESLint base config |
| `typescript-eslint` | ^8.71.0 | 8.71.0 | MIT | TS lint rules |
| `eslint-plugin-react-hooks` | ^7.1.1 | 7.1.1 | MIT | Hooks lint rules |
| `eslint-plugin-react-refresh` | ^0.5.7 | 0.5.7 | MIT | Fast-refresh lint rules |
| `globals` | ^17.12.0 | 17.12.0 | MIT | ESLint globals |
| `@types/leaflet` | ^1.9.22 | 1.9.22 | MIT | Types |
| `@types/node` | ^24.19.0 | 24.19.0 | MIT | Types |
| `@types/react` | ^19.3.0 | 19.3.0 | MIT | Types |
| `@types/react-dom` | ^19.3.0 | 19.3.0 | MIT | Types |

The viewer imports the `ts-rs` bindings from `bindings/ts/arda` through the `@arda` alias (`apps/viewer/vite.config.ts:7`). `deny.toml` covers Rust only; npm packages are covered by the policy below.

### npm license policy (`apps/viewer`)

Policy file `apps/viewer/licence-policy.json`; checker `apps/viewer/scripts/check-licences.mjs` (Node standard library only, run as `npm run licences`, and by the CI `viewer` job). It reads `package-lock.json`, so no install is needed, and classifies each package as runtime (no `dev`/`devOptional` flag) or build-time (flagged).

| Scope | Allowed licenses |
|---|---|
| Runtime (bundled into `dist/`) | MIT, ISC, BSD-2-Clause, BSD-3-Clause, Apache-2.0, 0BSD, Zlib, MIT-0, CC0-1.0 |
| Build-time only | the runtime list plus MPL-2.0 (unmodified tool), CC-BY-4.0 (data, not shipped), BlueOak-1.0.0, Python-2.0 |

- Everything else fails: GPL, LGPL, AGPL, SSPL, BUSL, `UNLICENSED`, missing and unknown licenses.
- SPDX `OR` passes if any branch is allowed; `AND` needs every part allowed; `X WITH Y` must be listed whole.
- Per-package exceptions go in the `exceptions` array of the policy file, each with a `package` and a non-empty `reason`; the list is empty today.
- On the current lockfile (204 packages, 4 runtime) there are no violations. Build-time MPL-2.0 (`lightningcss` and its platform packages, via Vite), CC-BY-4.0 (`caniuse-lite`) and BlueOak-1.0.0 (`minimatch`) pass only because they are dev-only.
- Notices: `apps/viewer/scripts/licence-notices.mjs`, a Vite plugin in `vite.config.ts`, writes `THIRD_PARTY_LICENSES.txt` into `dist/` with the licence text of every runtime package (Leaflet's BSD-2-Clause notice, React, React DOM, Scheduler), read from `node_modules/<pkg>/LICENSE*`. The build fails if a runtime package has no licence file. Any later release that ships `dist/` (for example in the Docker image) must ship this file with it.

### Art pipeline (`tools/art-gen`)

No `package.json` and no npm dependencies: the scripts declare Node 26 with the standard library only (`tools/art-gen/generate.mjs:3`) and import only `node:` modules and sibling files (`tools/art-gen/generate.mjs:10`-`tools/art-gen/generate.mjs:16`). Tests run with `node --test tools/art-gen/test/` (`tools/art-gen/README.md:13`).

## External services

No database, broker or remote API is connected from `crates/*/src/`; the Rust runtime reads and writes only the local filesystem and serves HTTP itself (`crates/arda-server/src/main.rs:76`).

| Service | Caller | Connection setup | Config source |
|---|---|---|---|
| Slopify (local image-generation orchestrator, HTTP JSON API under `/api/...`) | `tools/art-gen/generate.mjs` | `fetch(`${server}${path}`)` in `client()` (`tools/art-gen/generate.mjs:433`, `tools/art-gen/generate.mjs:438`); health probe `GET /api/health` (`tools/art-gen/generate.mjs:696`) | `--server URL`, else env `SLOPIFY_URL`, else `http://127.0.0.1:6969` (`tools/art-gen/generate.mjs:21`, `tools/art-gen/generate.mjs:108`) |
| Image providers behind Slopify (`openai-image`, `google-image`, `fal`, `replicate`, `codex-image`) | Slopify, selected by `--provider` (default `openai-image`, model `gpt-image-2`) | Not contacted by art-gen directly; art-gen checks Slopify's `/api/providers` readiness and refuses when the provider has no key configured in Slopify (`tools/art-gen/generate.mjs:44`, `tools/art-gen/generate.mjs:504`-`tools/art-gen/generate.mjs:507`) | Provider keys live in Slopify's settings; art-gen holds none |
| Image reviewer (`claude-code` default, or `codex`) | Slopify, on art-gen's request (`--reviewer`) | Requested through the Slopify project payload | `tools/art-gen/generate.mjs` defaults (`reviewer`, `reviewerModel`) |

Submission is gated: `--dry-run` contacts nothing and `--yes` is required to submit paid generations (`tools/art-gen/generate.mjs:114`-`tools/art-gen/generate.mjs:115`). Reads retry on network drops with delays 2-60 s; writes do not (`tools/art-gen/generate.mjs:431`). GitHub Actions supplies CI (`.github/workflows/ci.yml`). crates.io and GHCR are the recorded release destinations; no publishing workflow exists.

## Forcing data provenance

The checked-in forcing tables use five observed monthly profiles as procedural climate analogs. The offline generator preserves their reported rounding, apportions rainfall with exact integer totals, and supplies temperature anomalies and astronomical daylength; builds and generation never fetch these sources (`tools/generate_water_forcing_tables.py`, `crates/arda-gen/src/hydrology/forcing.rs`). Carried forward from the previous chapter; not re-verified against the source pages.

| Profile | Source and period | Input limitation retained |
|---|---|---|
| Singapore, Changi | [Meteorological Service Singapore, 1991–2020](https://www.weather.gov.sg/climate-climate-of-singapore/) | An equatorial maritime analog, not every tropical climate. |
| Valencia | [AEMET, 1981–2010](https://www.aemet.es/es/serviciosclimaticos/datosclimatologicos/valoresclimatologicos?l=8414A) | Rounded monthly precipitation sums to 459 mm; the separately rounded annual page total is 461 mm. |
| Heathrow | [Met Office, 1991–2020](https://www.metoffice.gov.uk/research/climate/maps-and-data/location-specific-long-term-averages/gcpsvg3nc) | Monthly temperature is the midpoint of reported mean daily maximum/minimum. |
| Ottawa | [ECCC, 1991–2020](https://climate.weather.gc.ca/climate_normals/results_1991_2020_e.html?climate_id=6105976&dispBack=0&lstProvince=ON&searchType=stnProv) | Total monthly precipitation is used; runtime snow storage is not modeled. |
| Yellowknife Hydro | [ECCC, 1971–2000, climate ID 2204200](https://climate.weather.gc.ca/climate_normals/results_e.html?climate_id=2204200) | Older class-C normals with poorer coverage; a dry-cold analog, not current weather. |

The evaporation expression follows Hamon 1961 as used in [USGS Open-File Report 2025-1021, equations 4–5](https://pubs.usgs.gov/publication/ofr20251021/full), which treats it as a lower estimate beside a radiation method. The integer table values are in `crates/arda-gen/src/hydrology/forcing_tables.rs`.
