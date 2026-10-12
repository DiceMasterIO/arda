---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: aa5464971e03
paths_covered:
  - ":(top)Cargo.toml"
  - ":(top)crates/*/Cargo.toml"
  - ":(top)Dockerfile"
  - ":(top).github/**"
  - ":(top)rust-toolchain.toml"
  - ":(top)CHANGELOG.md"
  - ":(top).gitattributes"
  - ":(top)crates/arda-cli/src/**"
  - ":(top)crates/arda-server/src/**"
  - ":(top)crates/arda-settle/src/main.rs"
  - ":(top)crates/arda-town/src/main.rs"
  - ":(top)crates/arda-town/src/plan/mod.rs"
  - ":(top)crates/arda-refine/src/bin/**"
  - ":(top)crates/arda-midzoom/src/bin/**"
  - ":(top)crates/*/tests/**"
  - ":(top)crates/*/examples/**"
  - ":(top)tests/**"
  - ":(top)apps/viewer/package.json"
  - ":(top)apps/viewer/vite.config.ts"
  - ":(top)apps/viewer/src/config.ts"
  - ":(top)apps/viewer/src/api/client.ts"
  - ":(top)tools/art-gen/generate.mjs"
  - ":(top)tools/art-gen/test/**"
---

# Operations

## Processes

Six Rust binaries plus the viewer dev server and the art tool. All Rust binaries run to completion except `arda-server`, which serves until Ctrl-C (`crates/arda-server/src/main.rs:86`-`crates/arda-server/src/main.rs:88`).

| Process | Binary (manifest) | Local command | Container command | Depends on |
|---|---|---|---|---|
| World CLI | `arda` (`crates/arda-cli/Cargo.toml:9`) | `cargo run -p arda-cli --release -- generate --seed 42 --micro --out out/micro42` | `docker run --rm -v /host/worlds:/worlds arda-local generate --seed 42 --micro --out /worlds/w42` (`Dockerfile:9`-`Dockerfile:11`) | Filesystem only |
| HTTP server | `arda-server` (`crates/arda-server/Cargo.toml:12`) | `cargo run -p arda-server --release -- --world out/micro42` | None: the image ships only `arda` (`Dockerfile:4`, `Dockerfile:8`) | A generated world directory; tactical library (default `assets/tactical/placeholder`, `crates/arda-server/src/tactical/mod.rs:50`), validated at startup |
| Settlements | `arda-settle` (`crates/arda-settle/Cargo.toml:31`) | `cargo run -p arda-settle --release -- generate --world out/micro42` | None | A generated world |
| Town planner | `arda-town` (`crates/arda-town/Cargo.toml:12`) | `cargo run -p arda-town -- info --site aldermere` | None | Built-in sample sites; tactical library for `block` |
| Tactical refinement | `arda-refine` (`crates/arda-refine/Cargo.toml:9`) | `cargo run -p arda-refine --release -- survey --world out/micro42` | None | A generated world |
| Mid-zoom relief | `arda-midzoom` (`crates/arda-midzoom/Cargo.toml:9`) | `cargo run -p arda-midzoom --release -- --world out/micro42 --z 4 --center 50000,100000 --out /tmp/mz.png` | None | A generated world; optional `society/` |
| Viewer | Vite (`apps/viewer/package.json:9`) | `cd apps/viewer && npm ci && npm run dev` (port 5173, `apps/viewer/vite.config.ts:16`) | None | A running `arda-server`, default base `http://localhost:8787` (`apps/viewer/src/api/client.ts:52`) |
| Art generation | `node tools/art-gen/generate.mjs` | `node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --dry-run` (`tools/art-gen/generate.mjs:5`) | None | A local Slopify for real runs (`tools/art-gen/generate.mjs:21`); output feeds `arda tactical import` |

The world pipeline order is `arda generate` → `arda settle` (writes `<world>/society/`, `crates/arda-cli/src/main.rs:129`) → `arda society build` (writes `society.json`, `notables.json`, `crates/arda-cli/src/society.rs:17`-`crates/arda-cli/src/society.rs:20`). Process stdout carries a one-line banner and summaries; every binary prints errors as `<binary>: <error>` on stderr and exits non-zero (`crates/arda-server/src/main.rs:98`, `crates/arda-settle/src/main.rs:71`, `crates/arda-refine/src/bin/arda-refine.rs:79`).

### `arda` subcommands and flags

Defined in `crates/arda-cli/src/main.rs:34`.

| Subcommand | Flag | Default | Site |
|---|---|---|---|
| `generate` | `--seed` | required | `crates/arda-cli/src/main.rs:39` |
| | `--size WxH` km | `500x1000` | `crates/arda-cli/src/main.rs:41` |
| | `--micro` | off (8-area MICRO continent) | `crates/arda-cli/src/main.rs:45` |
| | `--out` | required | `crates/arda-cli/src/main.rs:48` |
| | `--terrain legacy\|fine` | `legacy` | `crates/arda-cli/src/main.rs:51` |
| | `--recipe 4..=8` | recipe 7 in fine mode; fine only | `crates/arda-cli/src/main.rs:57` |
| | `--latitude S,N` | `35,55` | `crates/arda-cli/src/main.rs:61` |
| | `--fine-ram-bytes` / `--fine-file-bytes` | 16 GiB / 4 GiB; rejected with `legacy` (`crates/arda-cli/src/generate.rs:76`) | `crates/arda-cli/src/main.rs:64`-`crates/arda-cli/src/main.rs:68` |
| `preview` | `--seed`, `--size`, `--micro`, `--out` | as `generate`; writes `world/` and `overview.png` | `crates/arda-cli/src/main.rs:73` |
| | `--px 1..=512` | absent; conflicts with `--quality`, `--style` | `crates/arda-cli/src/main.rs:84` |
| | `--quality` | `8k` (512–32768 px or `1k`–`32k`) | `crates/arda-cli/src/main.rs:87` |
| | `--style classic\|atlas\|atlas-oblique` | Classic | `crates/arda-cli/src/main.rs:91`, `crates/arda-cli/src/main.rs:157` |
| `export` | `--world`, `--out` | required | `crates/arda-cli/src/main.rs:100`, `crates/arda-cli/src/main.rs:125` |
| | `--area ax,ay` | `0,0` | `crates/arda-cli/src/main.rs:103` |
| | `--format png\|json` | `png` | `crates/arda-cli/src/main.rs:106` |
| | `--overview` | off | `crates/arda-cli/src/main.rs:109` |
| | `--block ax,ay,cx,cy` | absent | `crates/arda-cli/src/main.rs:113` |
| | `--detail` (4096² area PNG) | off; conflicts with overview/block/quality | `crates/arda-cli/src/main.rs:116` |
| | `--quality`, `--style` | `8k`, Classic; `atlas-oblique` overview only (`crates/arda-cli/src/main.rs:200`) | `crates/arda-cli/src/main.rs:119`, `crates/arda-cli/src/main.rs:122` |
| `settle` | `--world` | required | `crates/arda-cli/src/main.rs:132` |
| `society build` | `--world` | required | `crates/arda-cli/src/society.rs:23` |
| `tactical validate` | `<dir>` (library or `top:…:bottom` stack) | required | `crates/arda-cli/src/tactical.rs:16` |
| `tactical import` | `<raw_dir>`, `--out`, `--manifest`, `--base` (`assets/tactical/placeholder`), `--no-base`, `--reference`, `--grade-strength`, `--contact-sheet` | as listed | `crates/arda-cli/src/tactical_import.rs:11` |
| `tactical placeholders` | `--out`, `--seed` | `assets/tactical/placeholder`, 58 (`crates/arda-tactical/src/placeholders/mod.rs:31`) | `crates/arda-cli/src/tactical.rs:23` |
| `tactical render` | `--layout` (built-in name, `all`, or JSON path), `--library`, `--ppsq`, `--out`, `--seed`, `--grid`, `--no-lighting`, `--no-grade` | layout required; `assets/tactical/placeholder`, 128, `out/tactical`, 1 | `crates/arda-cli/src/tactical.rs:32` |
| `dungeon` | `--seed`, `--kind dungeon\|cave`, `--size WxH` (16–160), `--out`, `--rules`, `--meta`, `--scene`, `--render`, `--library`, `--ppsq`, `--grid` | 1, `dungeon`, `48x36`, `out/dungeon/layout.json`, …, `assets/tactical/placeholder`, 128 | `crates/arda-cli/src/dungeon.rs:19` |

### `arda-server` flags

Defined in `crates/arda-server/src/main.rs:16`; routes are nested under `/v1` (`crates/arda-server/src/routes.rs:61`).

| Flag | Default | Site |
|---|---|---|
| `--world` | required | `crates/arda-server/src/main.rs:19` |
| `--port` | 8787 | `crates/arda-server/src/main.rs:22` |
| `--host` | `127.0.0.1` | `crates/arda-server/src/main.rs:25` |
| `--area-cache-mib` | 1024 | `crates/arda-server/src/main.rs:28` |
| `--fine-cache-mib` | 512 | `crates/arda-server/src/main.rs:31` |
| `--max-overview-px` | 8192 | `crates/arda-server/src/main.rs:34` |
| `--tile-base-px` | 4096 | `crates/arda-server/src/main.rs:37` |
| `--library` | `assets/tactical/placeholder` | `crates/arda-server/src/main.rs:41` |
| `--header-timeout-s` | 10 | `crates/arda-server/src/main.rs:44` |
| `--max-connections` | 64 | `crates/arda-server/src/main.rs:47` |
| `--prefetch-workers` | 2 (`crates/arda-server/src/tactical/prefetch.rs:37`); 0 disables | `crates/arda-server/src/main.rs:50` |
| `--sheet-mapping` | absent; a bad file refuses startup | `crates/arda-server/src/main.rs:54` |

### Other binaries

| Binary | Subcommand / flags | Defaults | Site |
|---|---|---|---|
| `arda-settle` | `generate --world` | memory budget 16 GiB (`crates/arda-settle/src/grid.rs:15`) | `crates/arda-settle/src/main.rs:20` |
| | `render --world --out [--quality] [--full-frame]` | quality 2048 | `crates/arda-settle/src/main.rs:26` |
| `arda-town` | `render --site --out [--seed] [--px]` | seed 42, px 0 (auto ~1600 px) | `crates/arda-town/src/main.rs:23` |
| | `block --site [--window x,y,w,h \| --focus NAME] [--size] --out [--library] [--seed] [--ppsq] [--grid] [--json]` | size 64, `assets/tactical/placeholder`, seed 42, ppsq 48 | `crates/arda-town/src/main.rs:38` |
| | `info --site [--seed]`; `export --site --out [--seed]` | seed 42; sites `aldermere`, `thornby`, `highcrag`, `saltwick` | `crates/arda-town/src/main.rs:74`, `crates/arda-town/src/main.rs:83` |
| `arda-refine` | `render --world --cell gx,gy [--window WxH] --out [--json] [--scale]` | window 1x1, scale 12 | `crates/arda-refine/src/bin/arda-refine.rs:22` |
| | `stats --world --cell`; `survey --world` | none | `crates/arda-refine/src/bin/arda-refine.rs:43`, `crates/arda-refine/src/bin/arda-refine.rs:52` |
| `arda-midzoom` | `--world --z --center x,y [--size] [--base-px] --out [--plain] [--tiles k]` | size 512, base 4096 | `crates/arda-midzoom/src/bin/arda-midzoom.rs:12` |

## Configuration

Runtime configuration is CLI flags only; no binary reads a config file from a fixed or implicit location (files such as `--sheet-mapping` or `--manifest` are passed explicitly). Compile-time `CARGO_PKG_VERSION` labels banners and manifests (`crates/arda-cli/src/main.rs:240`, `crates/arda-server/src/main.rs:81`). `GenerateConfig` validates size 64–4000 km per axis, latitude band, and density 1–200 people/km² (`crates/arda-core/src/config.rs:10`-`crates/arda-core/src/config.rs:13`, `crates/arda-core/src/config.rs:83`-`crates/arda-core/src/config.rs:100`). No variable matching `*_SECRET`, `*_TOKEN`, `*_PASSWORD` or `*_KEY` is read anywhere in the repository.

| Name | Default | Consuming code | Documented where |
|---|---|---|---|
| `ARDA_BLESS` | unset | Rewrites root golden fixtures instead of comparing (`tests/golden_world.rs:82`, `tests/golden_recipes.rs:102`) | Panic message in the same files |
| `ARDA_BLESS_BINDINGS` | unset | Rewrites `bindings/ts/arda` and the served JSON Schemas (`crates/arda-server/src/bindings.rs:208`, `crates/arda-server/src/schema/tests.rs:7`) | `apps/viewer/README.md:77` |
| `ARDA_BLESS_DUNGEON` | unset | Rewrites dungeon golden (`crates/arda-dungeon/tests/levels.rs:215`) | Panic message there |
| `ARDA_TEST_WORLD`, `ARDA_TEST_WORLD_<seed>` | unset → tests build a fixture under `target/` | `crates/arda-blocks/tests/settlements.rs:35`, `crates/arda-people/tests/interiors.rs:39`, `crates/arda-settle/tests/micro.rs:25`, `crates/arda-settle/tests/realms_micro.rs:22` | None beyond code |
| `ARDA_SERVER_TEST_WORLD` | unset → built fixture | `crates/arda-server/tests/world_api/support.rs:35`, `crates/arda-server/tests/world_api/schemas.rs:199` | None beyond code |
| `ARDA_MIDZOOM_TEST_WORLD` | unset → built fixture | `crates/arda-midzoom/tests/world.rs:36` | None beyond code |
| `ARDA_ZOOM_WORLD`, `ARDA_ZOOM_ARID_WORLD` | unset → built fixture | `tests/zoom_continuity.rs:164`, `tests/zoom_continuity.rs:235` | None beyond code |
| `ARDA_WORLD` | required by that test | `crates/arda-people/tests/town_wfc.rs:12` | None beyond code |
| `ARDA_REFINE_WORLD` | `../../out/micro42` | `crates/arda-refine/tests/real_world.rs:17` | None beyond code |
| `ARDA_COAST_MEASUREMENT_OUT` | required by that test | `crates/arda-gen/tests/coast_measurement.rs:111` | None beyond code |
| `ARDA_SOCIETY_TILES` | test default | `crates/arda-society/tests/scale.rs:82` | None beyond code |
| `ARDA_TOWN_DEBUG` | unset | Town planner debug output (`crates/arda-town/src/plan/mod.rs:308`) | None beyond code |
| `SINKS`, `LAKES`, `PLAN_ID` | unset | Diagnostic examples (`crates/arda-gen/examples/water_diag.rs:32`, `crates/arda/examples/water_metrics.rs:151`, `crates/arda-server/examples/tactical_timing.rs:35`) | None beyond code |
| `VITE_ARDA_API` | `http://localhost:8787`, after `?api=` and localStorage (`apps/viewer/src/config.ts:10`-`apps/viewer/src/config.ts:19`) | Viewer API base | `apps/viewer/README.md:57` |
| `SLOPIFY_URL` | `http://127.0.0.1:6969`; `--server` overrides | `tools/art-gen/generate.mjs:21` | `tools/art-gen/generate.mjs:108` |
| `ART_GEN_MAX_SPEND` | unset; default for `--max-spend` (USD, worst case incl. reviewer remakes); `--max-images N` caps images per run | `tools/art-gen/generate.mjs` (`maxSpend`), `tools/art-gen/safety.mjs` | `tools/art-gen/README.md` (Spending safety) |
| `SLOPIFY_DIR` | `../slopify` | art-gen tests against Slopify's code (`tools/art-gen/test/art-gen.test.mjs:28`) | `tools/art-gen/README.md:249` |

Third-party runtime controls such as `RAYON_NUM_THREADS` are read by `rayon`, not by repository code.

## Infrastructure

| Item | Facts | Site |
|---|---|---|
| Docker image | Two stages: `rust:1-bookworm` builds `cargo build --release -p arda-cli`; runtime `debian:bookworm-slim`, user `arda` UID 10001, `VOLUME /worlds`, `ENTRYPOINT ["arda"]`. No `EXPOSE`, no healthcheck, no server binary. Build with `docker build -t arda-local .`; mounted destinations must be writable by UID 10001. | `Dockerfile:1`-`Dockerfile:11` |
| Compose | None in the repository | — |
| Ports | `arda-server` binds `127.0.0.1:8787` by default (`crates/arda-server/src/main.rs:22`-`crates/arda-server/src/main.rs:25`); viewer dev server 5173 (`apps/viewer/vite.config.ts:16`); art-gen talks to Slopify on 6969 (`tools/art-gen/generate.mjs:21`) | as cited |
| Server health | `GET /v1/health` (`crates/arda-server/src/routes.rs:30`) | — |
| Server limits | Header-read timeout and connection cap in a custom hyper accept loop (`crates/arda-server/src/serve.rs:1`); CORS allows only local origins, methods GET/HEAD/POST/OPTIONS (`crates/arda-server/src/routes.rs:92`) | as cited |
| Server caches | Byte-bounded LRUs (`crates/arda-server/src/cache.rs:1`): areas 1 GiB, derived 256 MiB, fine windows 512 MiB (`crates/arda-server/src/query.rs:36`-`crates/arda-server/src/query.rs:38`); overview PNGs/tiles 256 MiB each (`crates/arda-server/src/overview.rs:32`); relief tiles 128 MiB (`crates/arda-server/src/relief.rs:22`); tactical renders 768 MiB, pyramid 1 GiB, encoded 256 MiB (`crates/arda-server/src/tactical/mod.rs:80`-`crates/arda-server/src/tactical/mod.rs:82`). Only area and fine budgets have flags. All in-memory; nothing persisted. | as cited |
| Test fixture cache | Shared worlds under `target/`, built in a sibling dir and renamed into place, stamped `.fixture-stamp` with a fingerprint of library sources, data, manifests and lockfile; stale stamps rebuild (`tests/support/fixture_dir.rs:1`-`tests/support/fixture_dir.rs:27`) | as cited |
| Logs | No logging crate; `println!`/`eprintln!` only. Server: startup banner on stdout (`crates/arda-server/src/main.rs:79`); 5xx detail on stderr with redacted body (`crates/arda-server/src/error.rs:217`); per-request tactical line `arda-server tactical <route> <detail> cache=<hit\|miss> bytes=<n> <ms> ms` (`crates/arda-server/src/tactical/routes.rs:138`) | as cited |
| CI | `.github/workflows/ci.yml`, on push and pull_request (`.github/workflows/ci.yml:2`): `test` matrix ubuntu/macos/windows, `cargo test --workspace --all-features --no-fail-fast` (`.github/workflows/ci.yml:9`-`.github/workflows/ci.yml:15`); `msrv` Rust 1.96.1 `cargo check --workspace --all-targets --all-features --locked` (`.github/workflows/ci.yml:22`-`.github/workflows/ci.yml:23`); `lint` fmt check and clippy `-D warnings` (`.github/workflows/ci.yml:32`-`.github/workflows/ci.yml:33`); `deny` cargo-deny-action v2 (`.github/workflows/ci.yml:39`). No viewer or art-gen job; no release or publish workflow. | as cited |
| Line endings | `* text=auto eol=lf` so golden fixtures hash identically on Windows; `tools/art-gen/checklist.csv` kept `-text` | `.gitattributes` |
| Build profiles | `[profile.test] opt-level = 2`; `[profile.release] codegen-units = 1` | `Cargo.toml:68`-`Cargo.toml:72` |

## Developer workflow

| Task | Command | Source |
|---|---|---|
| Tests (as CI) | `cargo test --workspace --all-features --no-fail-fast` | `.github/workflows/ci.yml:15` |
| MSRV check | `cargo +1.96.1 check --workspace --all-targets --all-features --locked` | `.github/workflows/ci.yml:22`-`.github/workflows/ci.yml:23` |
| Format | `cargo fmt --all --check` | `.github/workflows/ci.yml:32` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | `.github/workflows/ci.yml:33` |
| Dependency policy | `cargo deny check` (policy `deny.toml`) | `.github/workflows/ci.yml:39` |
| Benches | `cargo bench -p arda-gen` (`area_erosion`, `continent_stage`) | `crates/arda-gen/Cargo.toml` |
| Golden re-bless | `ARDA_BLESS=1 cargo test --release --test golden_world --test golden_recipes` rewrites `tests/golden/micro-42*.txt` | `tests/golden_world.rs:82`, `tests/golden_recipes.rs:102` |
| Bindings/schema re-bless | `ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings` (schemas: same variable, `schema` tests) | `apps/viewer/README.md:77`, `crates/arda-server/src/schema/tests.rs:7` |
| Dungeon re-bless | `ARDA_BLESS_DUNGEON=1 cargo test -p arda-dungeon` | `crates/arda-dungeon/tests/levels.rs:215` |
| Viewer | `npm run typecheck`, `npm run lint`, `npm test`, `npm run build` in `apps/viewer` | `apps/viewer/package.json` |
| Viewer licence policy | `npm run licences` in `apps/viewer` (policy `apps/viewer/licence-policy.json`) | `apps/viewer/scripts/check-licences.mjs` |
| Art tool tests | `node --test tools/art-gen/test/` | `tools/art-gen/README.md:13` |
| Table regeneration | `python3 tools/generate_water_forcing_tables.py <out.rs>`; `python3 tools/generate_spectral_tables.py` | `tools/generate_water_forcing_tables.py`, `tools/generate_spectral_tables.py:6` |
| Migrations | None: no database. World format versioning is in the saved manifest (`format_version` in `tests/golden/micro-42.txt`) | — |

### Release process

Observed from the `release: Arda v0.8.1` commit (`022ce7c`) and tags `v0.5.0`…`v0.8.1`:

1. Add a `## <version> — <date>` section to `CHANGELOG.md` (`CHANGELOG.md:6`); the design ledger lives in `docs/capstone/changelog.md` and `docs/capstone/changelog.d/` (`CHANGELOG.md:3`-`CHANGELOG.md:4`).
2. Bump `[workspace.package] version` and every path entry's `version` in `[workspace.dependencies]` (`Cargo.toml:6`, `Cargo.toml:12`-`Cargo.toml:32`), which updates `Cargo.lock`; bump `apps/viewer/package.json:4` and its lockfile; update the version in `crates/arda-server/API.md`.
3. Re-bless `tests/golden/micro-42.txt` with `ARDA_BLESS=1`: the saved world manifest embeds `arda_version`, so `world.json`'s fingerprint changes with every version bump.
4. Commit as `release: Arda v<version>` and tag `v<version>`. No workflow builds or publishes artifacts; crates are not published to crates.io and no image is pushed.

### Recorded measurements

Dated timing receipts for terrain generation and Atlas export (2026-09-08 through 2026-09-29) are kept in `docs/capstone/features/2026-09-23-terrain-corrections/STUDY.md` and `EXPERIMENTS.md`, which are symlinks to a path outside the repository. They were not re-measured for this chapter and predate the crates added since v0.2.0.
