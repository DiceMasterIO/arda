---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: d6f554562c40
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
  - ":(top)tools/*.sh"
  - ":(top).dockerignore"
  - ":(top)contract-kit/**"
  - ":(top)apps/viewer/scripts/**"
  - ":(top)apps/viewer/licence-policy.json"
  - ":(top)tools/art-gen/safety.mjs"
  - ":(top)tools/art-gen/README.md"
---

# Operations

## Processes

Six Rust binaries plus the viewer dev server and the art tool. All Rust binaries run to completion except `arda-server`, which serves until Ctrl-C or, on Unix, SIGTERM (`shutdown_signal` in `crates/arda-server/src/main.rs`).

| Process | Binary (manifest) | Local command | Container command | Depends on |
|---|---|---|---|---|
| World CLI | `arda` (`crates/arda-cli/Cargo.toml:10`) | `cargo run -p arda-cli --release -- generate --seed 42 --micro --out out/micro42` | `docker run --rm -v /host/worlds:/worlds --entrypoint arda arda-local generate --seed 42 --micro --out /worlds/w42` (`Dockerfile`) | Filesystem only |
| HTTP server | `arda-server` (`crates/arda-server/Cargo.toml:13`) | `cargo run -p arda-server --release -- --world out/micro42` | `docker run --rm -v /host/worlds/w42:/worlds/world:ro -p 8787:8787 arda-local` (default entry `tools/docker-entrypoint.sh`: `--host 0.0.0.0 --port 8787`, world `/worlds/world`) | A generated world directory; tactical library (default `assets/tactical/placeholder`, `crates/arda-server/src/tactical/mod.rs:57`), validated at startup |
| Settlements | `arda-settle` (`crates/arda-settle/Cargo.toml:33`) | `cargo run -p arda-settle --release -- generate --world out/micro42` | None | A generated world |
| Town planner | `arda-town` (`crates/arda-town/Cargo.toml:13`) | `cargo run -p arda-town -- info --site aldermere` | None | Built-in sample sites; tactical library for `block` |
| Tactical refinement | `arda-refine` (`crates/arda-refine/Cargo.toml:10`) | `cargo run -p arda-refine --release -- survey --world out/micro42` | None | A generated world |
| Mid-zoom relief | `arda-midzoom` (`crates/arda-midzoom/Cargo.toml:10`) | `cargo run -p arda-midzoom --release -- --world out/micro42 --z 4 --center 50000,100000 --out /tmp/mz.png` | None | A generated world; optional `society/` |
| Viewer | Vite (`apps/viewer/package.json:9`) | `cd apps/viewer && npm ci && npm run dev` (port 5173, `apps/viewer/vite.config.ts:17`) | None | A running `arda-server`, default base `http://localhost:8787` (`apps/viewer/src/api/client.ts:52`) |
| Art generation | `node tools/art-gen/generate.mjs` | `node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --dry-run` (`tools/art-gen/generate.mjs:5`) | None | A local Slopify for real runs (`tools/art-gen/generate.mjs:22`); output feeds `arda tactical import` |

The world pipeline order is `arda generate` → `arda settle` (writes `<world>/society/`, `crates/arda-cli/src/main.rs:137`) → `arda society build` (writes `society.json`, `notables.json`, `crates/arda-cli/src/society.rs:17`-`crates/arda-cli/src/society.rs:20`). Process stdout carries a one-line banner and summaries; every binary prints errors as `<binary>: <error>` on stderr and exits non-zero (`crates/arda-server/src/main.rs:136`, `crates/arda-settle/src/main.rs:71`, `crates/arda-refine/src/bin/arda-refine.rs:88`).

### `arda` subcommands and flags

Defined in `crates/arda-cli/src/main.rs:34`.

| Subcommand | Flag | Default | Site |
|---|---|---|---|
| `generate` | `--seed` | required | `crates/arda-cli/src/main.rs:39` |
| | `--size WxH` km | `500x1000` | `crates/arda-cli/src/main.rs:41` |
| | `--micro` | off (8-area MICRO continent) | `crates/arda-cli/src/main.rs:45` |
| | `--out` | required | `crates/arda-cli/src/main.rs:48` |
| | `--terrain legacy\|fine` | `fine` (since recipe 9; `legacy` was the default before, review 3.2-15) | `crates/arda-cli/src/main.rs:53` |
| | `--recipe 4..=9` | recipe 9 in fine mode (`--recipe 7` reproduces the v0.4–v0.8 default); fine only | `crates/arda-cli/src/main.rs:60` |
| | `--latitude S,N` | `35,55` | `crates/arda-cli/src/main.rs:65` |
| | `--fine-ram-bytes` / `--fine-file-bytes` | 16 GiB / 4 GiB; rejected with `legacy` (`crates/arda-cli/src/generate.rs:76`) | `crates/arda-cli/src/main.rs:68`-`crates/arda-cli/src/main.rs:71` |
| `preview` | `--seed`, `--size`, `--micro`, `--out` | as `generate`, but always legacy terrain; writes `world/` and `overview.png` | `crates/arda-cli/src/main.rs:77` |
| | `--px 1..=512` | absent; conflicts with `--quality`, `--style` | `crates/arda-cli/src/main.rs:84` |
| | `--quality` | `8k` (512–32768 px or `1k`–`32k`) | `crates/arda-cli/src/main.rs:87` |
| | `--style classic\|atlas\|atlas-oblique` | Classic | `crates/arda-cli/src/main.rs:91`, `crates/arda-cli/src/main.rs:183` |
| | `--society auto\|off` | `auto` (a fresh preview world has no society) | `crates/arda-cli/src/main.rs:95` |
| `export` | `--world`, `--out` | required | `crates/arda-cli/src/main.rs:104`, `crates/arda-cli/src/main.rs:133` |
| | `--area ax,ay` | `0,0` | `crates/arda-cli/src/main.rs:107` |
| | `--format png\|json` | `png` | `crates/arda-cli/src/main.rs:110` |
| | `--overview` | off | `crates/arda-cli/src/main.rs:113` |
| | `--block` (hidden) | removed in 1.0; fails with a pointer to `arda-refine render` and `GET /v1/tactical/cell/{gx}/{gy}.png` | `crates/arda-cli/src/main.rs:115`, `crates/arda-cli/src/main.rs:316` |
| | `--detail` (4096² area PNG) | off; conflicts with overview/quality | `crates/arda-cli/src/main.rs:119` |
| | `--quality`, `--style` | `8k`, Classic; `atlas-oblique` overview only (`crates/arda-cli/src/main.rs:227`) | `crates/arda-cli/src/main.rs:122`, `crates/arda-cli/src/main.rs:125` |
| | `--society auto\|off` | `auto`: land use, roads and settlements on overview and area PNGs when `<world>/society/` exists (logic/04 §overlay); `off` is the plain terrain | `crates/arda-cli/src/main.rs:130` |
| `settle` | `--world` | required | `crates/arda-cli/src/main.rs:140` |
| `society build` | `--world` | required | `crates/arda-cli/src/society.rs:23` |
| `tactical validate` | `<dir>` (library or `top:…:bottom` stack) | required | `crates/arda-cli/src/tactical.rs:16` |
| `tactical import` | `<raw_dir>`, `--out`, `--manifest`, `--base` (`assets/tactical/placeholder`), `--no-base`, `--reference`, `--grade-strength`, `--contact-sheet` | as listed | `crates/arda-cli/src/tactical_import.rs:11` |
| `tactical placeholders` | `--out`, `--seed` | `assets/tactical/placeholder`, 58 (`crates/arda-tactical/src/placeholders/mod.rs:31`) | `crates/arda-cli/src/tactical.rs:23` |
| `tactical render` | `--layout` (built-in name, `all`, or JSON path), `--library`, `--ppsq`, `--out`, `--seed`, `--grid`, `--no-lighting`, `--no-grade` | layout required; `assets/tactical/placeholder`, 128, `out/tactical`, 1 | `crates/arda-cli/src/tactical.rs:32` |
| `dungeon` | `--seed`, `--kind dungeon\|cave`, `--size WxH` (16–160), `--out`, `--rules`, `--meta`, `--scene`, `--render`, `--library`, `--ppsq`, `--grid` | 1, `dungeon`, `48x36`, `out/dungeon/layout.json`, …, `assets/tactical/placeholder`, 128 | `crates/arda-cli/src/dungeon.rs:19` |

### `arda-server` flags

Defined in `crates/arda-server/src/main.rs:16`; routes are nested under `/v1` (`crates/arda-server/src/routes.rs:72`).

| Flag | Default | Site |
|---|---|---|
| `--world` | required unless `--probe` | `crates/arda-server/src/main.rs:19` |
| `--port` | 8787 | `crates/arda-server/src/main.rs:26` |
| `--host` | `127.0.0.1` | `crates/arda-server/src/main.rs:29` |
| `--area-cache-mib` | 256 (`crates/arda-server/src/query.rs:42`; 1024 before review 3.6-8, 384 before the server-startup fix) | `crates/arda-server/src/main.rs:32` |
| `--fine-cache-mib` | 128 (`crates/arda-server/src/query.rs:44`; 512 before review 3.6-8, 256 before the server-startup fix) | `crates/arda-server/src/main.rs:35` |
| `--max-overview-px` | 8192 | `crates/arda-server/src/main.rs:38` |
| `--tile-base-px` | 4096 | `crates/arda-server/src/main.rs:41` |
| `--library` | `assets/tactical/placeholder` | `crates/arda-server/src/main.rs:45` |
| `--header-timeout-s` | 10 | `crates/arda-server/src/main.rs:48` |
| `--max-connections` | 32 (`crates/arda-server/src/serve.rs:32`; 64 before the server-startup fix) | `crates/arda-server/src/main.rs:51` |
| `--prefetch-workers` | 2 (`crates/arda-server/src/tactical/prefetch.rs:37`); 0 disables | `crates/arda-server/src/main.rs:54` |
| `--bake-workers` | 1 (`crates/arda-server/src/tactical/bake.rs:37`); 0 disables `POST /v1/tactical/bake` (501); values above 1 bake one at a time | `crates/arda-server/src/main.rs:58` |
| `--bake-result-mib` | 384 (`crates/arda-server/src/tactical/bake.rs:41`): finished bake tile sets held for download, oldest evicted first; counted in startup admission, and below one worst-case 128-ppsq set (about 362 MiB) startup is refused while baking is on | `crates/arda-server/src/main.rs:62` |
| `--sheet-mapping` | absent; a bad file refuses startup | `crates/arda-server/src/main.rs:66` |
| `--probe` | off; `GET /v1/health` on loopback at `--port`, exit 0 on 200, 1 otherwise, serves nothing and needs no `--world` (the image `HEALTHCHECK`) | `crates/arda-server/src/main.rs:23`, `crates/arda-server/src/probe.rs` |

### Other binaries

| Binary | Subcommand / flags | Defaults | Site |
|---|---|---|---|
| `arda-settle` | `generate --world` | memory budget 16 GiB (`crates/arda-settle/src/grid.rs:15`) | `crates/arda-settle/src/main.rs:20` |
| | `render --world --out [--quality] [--full-frame]` | quality 2048 | `crates/arda-settle/src/main.rs:26` |
| `arda-town` | `render --site --out [--seed] [--px]` | seed 42, px 0 (auto ~1600 px) | `crates/arda-town/src/main.rs:23` |
| | `block --site [--window x,y,w,h \| --focus NAME] [--size] --out [--library] [--seed] [--ppsq] [--grid] [--json]` | size 64, `assets/tactical/placeholder`, seed 42, ppsq 48 | `crates/arda-town/src/main.rs:38` |
| | `info --site [--seed]`; `export --site --out [--seed]` | seed 42; sites `aldermere`, `thornby`, `highcrag`, `saltwick` | `crates/arda-town/src/main.rs:74`, `crates/arda-town/src/main.rs:83` |
| `arda-refine` | `render --world --cell gx,gy [--window WxH] --out [--json] [--scale]` | window 1x1, scale 12 | `crates/arda-refine/src/bin/arda-refine.rs:22` |
| | `stats --world --cell`; `coastwet --world [--stride]` (coast and wetland statistics as JSON); `survey --world` | `--stride` 16 | `crates/arda-refine/src/bin/arda-refine.rs:43`, `crates/arda-refine/src/bin/arda-refine.rs:52`, `crates/arda-refine/src/bin/arda-refine.rs:61` |
| `arda-midzoom` | `--world --z --center x,y [--size] [--base-px] --out [--plain] [--tiles k]` | size 512, base 4096 | `crates/arda-midzoom/src/bin/arda-midzoom.rs:12` |

## Configuration

Runtime configuration is CLI flags only; no binary reads a config file from a fixed or implicit location (files such as `--sheet-mapping` or `--manifest` are passed explicitly). Compile-time `CARGO_PKG_VERSION` labels banners and manifests (`crates/arda-cli/src/main.rs:271`, `crates/arda-server/src/main.rs:96`). `GenerateConfig` validates size 64–4000 km per axis, latitude band, and density 1–200 people/km² (`crates/arda-core/src/config.rs:10`-`crates/arda-core/src/config.rs:13`, `crates/arda-core/src/config.rs:83`-`crates/arda-core/src/config.rs:100`). No variable matching `*_SECRET`, `*_TOKEN`, `*_PASSWORD` or `*_KEY` is read anywhere in the repository.

| Name | Default | Consuming code | Documented where |
|---|---|---|---|
| `ARDA_BLESS` | unset | Rewrites root golden fixtures instead of comparing (`tests/golden_world.rs:82`, `tests/golden_recipes.rs:104`) | Panic message in the same files |
| `ARDA_BLESS_BINDINGS` | unset | Rewrites `bindings/ts/arda` and the served JSON Schemas (`crates/arda-server/src/bindings.rs:240`, `crates/arda-server/src/schema/tests.rs:8`) | `apps/viewer/README.md:78` |
| `ARDA_BLESS_SCHEMA_BASELINE` | unset | Rewrites `bindings/schema-baseline/v1/` (schemas plus `versions.json`) instead of comparing (`crates/arda-server/src/schema/tests.rs:164`) | `docs/capstone/versioning.md` § Freeze guard |
| `ARDA_BLESS_DUNGEON` | unset | Rewrites dungeon golden (`crates/arda-dungeon/tests/levels.rs:230`) | Panic message there |
| `ARDA_BLESS_CONTRACT` | unset | Rewrites the contract kit's goldens, hashes, expected headers, fixture identity and `world.txt` instead of comparing (`crates/arda-server/tests/contract_kit.rs`); `tools/release.sh <version>` sets it | `contract-kit/README.md` |
| `ARDA_TEST_WORLD`, `ARDA_TEST_WORLD_<seed>` | unset → tests build a fixture under `target/` | `crates/arda-blocks/tests/settlements.rs:35`, `crates/arda-people/tests/interiors.rs:39`, `crates/arda-settle/tests/micro.rs:25`, `crates/arda-settle/tests/realms_micro.rs:22` | None beyond code |
| `ARDA_SERVER_TEST_WORLD` | unset → built fixture | `crates/arda-server/tests/world_api/support.rs:35`, `crates/arda-server/tests/world_api/schemas.rs:225` | None beyond code |
| `ARDA_MIDZOOM_TEST_WORLD` | unset → built fixture | `crates/arda-midzoom/tests/world.rs:36`, `crates/arda-midzoom/tests/export.rs:40` | None beyond code |
| `ARDA_ZOOM_WORLD`, `ARDA_ZOOM_ARID_WORLD` | unset → built fixture | `tests/zoom_continuity.rs:164`, `tests/zoom_continuity.rs:235` | None beyond code |
| `ARDA_WORLD` | required by that test | `crates/arda-people/tests/town_wfc.rs:12` | None beyond code |
| `ARDA_WORLD`, `ARDA_LIBRARY` (container only) | `/worlds/world`; the AI library stacked over the placeholder when `/opt/arda/library/ai/catalog.json` exists, else the placeholder | `tools/docker-entrypoint.sh:13`, `tools/docker-entrypoint.sh:22`; read only when the image is started with no arguments | Header comment of the script |
| `ARDA_SMOKE_PORT` | 18787 (`tools/docker-smoke.sh`), 18788 (`tools/smoke-binaries.sh`) | Host port the smoke scripts publish or bind (`tools/docker-smoke.sh:31`, `tools/smoke-binaries.sh:33`) | None beyond code |
| `ARDA_REFINE_WORLD` | `../../out/micro42` | `crates/arda-refine/tests/real_world.rs:17`, `crates/arda-refine/tests/coastwet_world.rs:10` | None beyond code |
| `ARDA_COAST_MEASUREMENT_OUT` | required by that test | `crates/arda-gen/tests/coast_measurement.rs:111` | None beyond code |
| `ARDA_SOCIETY_TILES` | test default | `crates/arda-society/tests/scale.rs:82` | None beyond code |
| `ARDA_TOWN_DEBUG` | unset | Town planner debug output (`crates/arda-town/src/plan/mod.rs:308`) | None beyond code |
| `SINKS`, `LAKES`, `PLAN_ID` | unset | Diagnostic examples (`crates/arda-gen/examples/water_diag.rs:32`, `crates/arda/examples/water_metrics.rs:151`, `crates/arda-server/examples/tactical_timing.rs:35`) | None beyond code |
| `VITE_ARDA_API` | `http://localhost:8787`, after `?api=` and localStorage (`apps/viewer/src/config.ts:10`-`apps/viewer/src/config.ts:19`) | Viewer API base | `apps/viewer/README.md:57` |
| `SLOPIFY_URL` | `http://127.0.0.1:6969`; `--server` overrides | `tools/art-gen/generate.mjs:22` | `tools/art-gen/generate.mjs:106` |
| `ART_GEN_MAX_SPEND` | unset; default for `--max-spend` (USD, worst case incl. reviewer remakes); `--max-images N` caps images per run | `tools/art-gen/generate.mjs:130` (`maxSpend`), `tools/art-gen/safety.mjs` | `tools/art-gen/README.md` (Spending safety) |
| `SLOPIFY_DIR` | `../slopify` | art-gen tests against Slopify's code (`tools/art-gen/test/art-gen.test.mjs:28`) | `tools/art-gen/README.md:262` |

Third-party runtime controls such as `RAYON_NUM_THREADS` are read by `rayon`, not by repository code.

## Infrastructure

| Item | Facts | Site |
|---|---|---|
| Docker image | Multi-stage `Dockerfile`: `rust:1.98.1-bookworm` with `RUSTUP_TOOLCHAIN=1.98.1` builds `cargo build --release --locked -p arda-cli -p arda-server` (BuildKit cache mounts); runtime `debian:bookworm-slim` pinned by digest, user and group `arda` 10001, both binaries in `/usr/local/bin`, `WORKDIR /opt/arda`, `EXPOSE 8787`, `VOLUME /worlds`, `HEALTHCHECK arda-server --probe`, OCI labels (`VERSION`, `REVISION` build args). Entry `arda-entrypoint` (`tools/docker-entrypoint.sh`): no arguments serves `/worlds/world` on `0.0.0.0:8787`; arguments go to `arda-server` verbatim; `--entrypoint arda` runs the CLI. Two variants: default ships the placeholder library only (`/opt/arda/library/placeholder`); `--build-context artlib=<dir>` adds the AI library at `/opt/arda/library/ai`, stacked over the placeholder. Maintainer decision D1: the published GHCR image is the placeholder variant only; the AI library stays an opt-in local build. Mount worlds `:ro` for serving; CLI use needs a directory writable by UID 10001. Checked by `tools/docker-smoke.sh`. | `Dockerfile`, `.dockerignore`, `tools/docker-entrypoint.sh`, `tools/docker-smoke.sh` |
| Compose | None in the repository | — |
| Ports | `arda-server` binds `127.0.0.1:8787` by default (`crates/arda-server/src/main.rs:26`-`crates/arda-server/src/main.rs:29`); viewer dev server 5173 (`apps/viewer/vite.config.ts:17`); art-gen talks to Slopify on 6969 (`tools/art-gen/generate.mjs:22`) | as cited |
| Server health | `GET /v1/health` (`crates/arda-server/src/routes.rs:30`) | — |
| Server limits | Header-read timeout and connection cap in a custom hyper accept loop (`crates/arda-server/src/serve.rs:1`); CORS allows only local origins, methods GET/HEAD/POST/DELETE/OPTIONS (`crates/arda-server/src/routes.rs:103`) | as cited |
| Server caches | Byte-bounded LRUs (`crates/arda-server/src/cache.rs:1`): areas 256 MiB, derived 128 MiB, fine windows 128 MiB (1 GiB, 256 MiB and 512 MiB before review 3.6-8; areas 384 MiB and fine windows 256 MiB before the server-startup fix) (`crates/arda-server/src/query.rs:42`-`crates/arda-server/src/query.rs:44`); overview PNGs/tiles 128 MiB each (256 MiB before the server-startup fix; `crates/arda-server/src/overview.rs:36`); relief tiles 128 MiB (`crates/arda-server/src/relief.rs:23`); tactical renders 768 MiB, pyramid 1 GiB, encoded PNGs, tiles and bodies 160 MiB each (256 MiB before review 3.6-8) (`crates/arda-server/src/tactical/mod.rs:88`-`crates/arda-server/src/tactical/mod.rs:93`). Finished bake tile sets: 384 MiB default (`crates/arda-server/src/tactical/bake.rs:41`). Only area, fine and bake-result budgets have flags. Startup admission also counts the tactical pipeline (2 concurrent compositions, 256 refined blocks, the refine source's 8 kept decoded areas plus as many evicted ones still in use and one fine memo, the town-plan budget's 768 MiB for cached plans and plans being drawn; `crates/arda-server/src/admission.rs`) and, once the world and library are open, the decoded library and textures, the `society/` files, the shore layer (twice) and the largest town plan drawn (plans may exceed their budget by one); it refuses to start above 16 GiB and the startup line prints the admitted total. A town plan is charged its square grid (not serialised; half a gigabyte for a default-size world's largest city) plus twice its JSON (`arda_people::people::plan_bytes`). All in-memory; nothing persisted. | as cited |
| Test fixture cache | Shared worlds under `target/`, built in a sibling dir and renamed into place, stamped `.fixture-stamp` with a fingerprint of library sources, data, manifests and lockfile; stale stamps rebuild (`tests/support/fixture_dir.rs:1`-`tests/support/fixture_dir.rs:27`) | as cited |
| Logs | No logging crate; `println!`/`eprintln!` only. Server: startup banner on stdout (`crates/arda-server/src/main.rs:94`); 5xx detail on stderr with redacted body (`crates/arda-server/src/error.rs:231`); per-request tactical line `arda-server tactical <route> <detail> cache=<hit\|miss> bytes=<n> <ms> ms` (`crates/arda-server/src/tactical/routes.rs:143`) | as cited |
| CI | `.github/workflows/ci.yml`, on push and pull_request (`.github/workflows/ci.yml:2`): `test` matrix ubuntu/macos/windows, `cargo test --workspace --all-features --no-fail-fast` (`.github/workflows/ci.yml:8`-`.github/workflows/ci.yml:17`); `msrv` Rust 1.96.1 `cargo check --workspace --all-targets --all-features --locked` (`.github/workflows/ci.yml:26`-`.github/workflows/ci.yml:27`); `lint` fmt check and clippy `-D warnings` (`.github/workflows/ci.yml:37`-`.github/workflows/ci.yml:38`); `deny` cargo-deny-action v2.1.1 (`.github/workflows/ci.yml:44`). Actions are pinned to full commit SHAs. A `viewer` job (working directory `apps/viewer`, Node 26): `npm ci`, `npm run licences`, `typecheck`, `lint`, `npm test`, `npm run build` (`.github/workflows/ci.yml:46`-`.github/workflows/ci.yml:63`); no art-gen job. `.github/workflows/release.yml` (tag `v*` and manual dry run) verifies, builds four binary targets and the multi-platform GHCR image, and drafts the GitHub release; see Release process. | as cited, `.github/workflows/release.yml` |
| Line endings | `* text=auto eol=lf` so golden fixtures hash identically on Windows; `tools/art-gen/checklist.csv` kept `-text` | `.gitattributes` |
| Build profiles | `[profile.test] opt-level = 2`; `[profile.release] codegen-units = 1` | `Cargo.toml:68`-`Cargo.toml:72` |

## Developer workflow

| Task | Command | Source |
|---|---|---|
| Tests (as CI) | `cargo test --workspace --all-features --no-fail-fast` | `.github/workflows/ci.yml:17` |
| MSRV check | `cargo +1.96.1 check --workspace --all-targets --all-features --locked` | `.github/workflows/ci.yml:26`-`.github/workflows/ci.yml:27` |
| Format | `cargo fmt --all --check` | `.github/workflows/ci.yml:37` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | `.github/workflows/ci.yml:38` |
| Dependency policy | `cargo deny check` (policy `deny.toml`) | `.github/workflows/ci.yml:44` |
| Benches | `cargo bench -p arda-gen` (`area_erosion`, `continent_stage`) | `crates/arda-gen/Cargo.toml` |
| Golden re-bless | `ARDA_BLESS=1 cargo test --release --test golden_world --test golden_recipes` rewrites `tests/golden/micro-42*.txt` | `tests/golden_world.rs:82`, `tests/golden_recipes.rs:104` |
| Bindings/schema re-bless | `ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings` (schemas: same variable, `schema` tests) | `apps/viewer/README.md:78`, `crates/arda-server/src/schema/tests.rs:8` |
| Dungeon re-bless | `ARDA_BLESS_DUNGEON=1 cargo test -p arda-dungeon` | `crates/arda-dungeon/tests/levels.rs:230` |
| Contract kit re-bless | `ARDA_BLESS_CONTRACT=1 cargo test --release -p arda-server --test contract_kit` | `contract-kit/README.md` |
| Contract kit live check | `node contract-kit/check.mjs --base http://127.0.0.1:8787` (exit 0 all match, 1 a difference, 2 unreachable); its tests `node --test contract-kit/test/` | `contract-kit/check.mjs` |
| Viewer | `npm run typecheck`, `npm run lint`, `npm test`, `npm run build` in `apps/viewer` | `apps/viewer/package.json` |
| Viewer licence policy | `npm run licences` in `apps/viewer` (policy `apps/viewer/licence-policy.json`) | `apps/viewer/scripts/check-licences.mjs` |
| Art tool tests | `node --test tools/art-gen/test/` | `tools/art-gen/README.md:13` |
| Table regeneration | `python3 tools/generate_water_forcing_tables.py <out.rs>`; `python3 tools/generate_spectral_tables.py` | `tools/generate_water_forcing_tables.py`, `tools/generate_spectral_tables.py:6` |
| Migrations | None: no database. World format versioning is in the saved manifest (`format_version` in `tests/golden/micro-42.txt`) | — |

### Release process

Releases v0.5.0 to v0.8.1 were hand-made `release: Arda v<version>` commits with tags. Since 1.0 a tag drives the build; the maintainer's credentials and approval stay in GitHub.

**Cutting a release.**

1. Run `tools/release.sh <version>` on a clean branch. It bumps `[workspace.package] version` and the 21 path entries in `[workspace.dependencies]` (`Cargo.toml`), `apps/viewer/package.json` and its lockfile (`npm version --no-git-tag-version`), and every `arda_version` and `server_version` example in `crates/arda-server/API.md` (the `/v1/world` and `/v1/health` bodies; `--check` counts each as a site); runs `cargo update -w`; re-blesses `tests/golden/micro-42*.txt` with `ARDA_BLESS=1 cargo test --release --test golden_world --test golden_recipes` (the saved `world.json` embeds `arda_version`); re-blesses the contract kit (`contract-kit/golden/`, `manifest.json`, `world.txt`) with `ARDA_BLESS_CONTRACT=1 cargo test --release -p arda-server --test contract_kit` and runs it once more without; checks that every version site agrees; and checks that `git diff` touches only those files. It prints the remaining commands.
2. Add a `## <version> — <date>` section to `CHANGELOG.md`. `tools/release.sh --notes <version>` prints what the release page will show. The design ledger (`docs/capstone/changelog.md`, `changelog.d/`) is separate.
3. Run `tools/release-gates.sh --with-benches` on the release machine (see "Release gates" below). CI never runs the gates. Its receipt is `target/release-gates/receipt-<commit>.md`; the script exits 0 when every gate is present and under budget, 1 otherwise (2 on a build error). Attach the receipt to the release notes or paste it under "Recorded measurements".
4. Commit as `release: Arda v<version>`, tag `v<version>`, push the branch and the tag.
5. The tag starts `.github/workflows/release.yml`: `verify` (tag equals the `Cargo.toml` version, `tools/release.sh --check`, a changelog section exists, fmt, clippy, `cargo test --workspace --locked`), then `binaries`, `image`, `release`. Review the **draft** release and publish it by hand.

`tools/release.sh --check [version]` verifies all 54 version sites (workspace version, 21 path entries, 24 lockfile members, viewer package and lockfile, API.md, and the contract kit manifest's `arda_version` and `server_version`) without writing; it also runs in `verify`.

**Artifacts.**

- Binaries `arda` and `arda-server` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (built on Ubuntu 22.04 for older glibc), `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`: `arda-<version>-<target>.tar.gz` (`.zip` on Windows) with LICENSE, NOTICE, README.md and CHANGELOG.md, plus `SHA256SUMS` on the release. Each build is smoke-tested first; Linux builds also generate the seed-42 micro world and answer `/v1/health`.
- Image `ghcr.io/dicemasterio/arda`, `linux/amd64` and `linux/arm64`, built from the root `Dockerfile` with no `artlib` context, so it carries the placeholder tactical library only (maintainer decision D1). Tags: `vX.Y.Z` gives `X.Y.Z`, `X.Y` and `latest`; a pre-release such as `v1.0.0-rc.1` pushes only its exact tag and the GitHub release is marked pre-release. OCI labels carry the version and revision; an SBOM and max-mode provenance attestation are attached to the manifest. The amd64 image is checked by `tools/docker-smoke.sh` before anything is pushed.
- Crates are not published (maintainer decision D5): every member sets `publish = false`. The public interface is the HTTP API, the CLI and the world format, not the Rust crates.
- An AI-library image is not published (D1). If that ever changes: store `arda-art-<library_version>.tar.zst` as a release asset of its own, commit its sha256 in a lock file, download and verify it in the `image` job, and pass it with `--build-context artlib=<dir>`. Uploading the asset is the maintainer's action.

**Dry run.** Start the `release` workflow from the Actions tab (`workflow_dispatch`) on any branch. It uses the `Cargo.toml` version, runs `verify`, `binaries` and `image` (smoke plus a no-push multi-platform build) and skips login, push and the `release` job. Locally: `tools/release.sh --check`, `cargo build --release --locked -p arda-cli -p arda-server`, `tools/smoke-binaries.sh target/release --serve`, `tools/package-release.sh <version> <target> target/release <dir>` and `tools/docker-smoke.sh <world-dir>`.

**Pins.** Every third-party action is pinned to a full commit SHA with the version in a comment. The release toolchain is Rust 1.98.1 (`RUST_TOOLCHAIN` in `release.yml`), matching the Dockerfile's `RUSTUP_TOOLCHAIN`; change both together.

### Release gates

`tools/release-gates.sh` is the one command for every wall-clock budget (1.0 task 09, OI-28). It builds the release test binaries once, runs each `#[ignore = "wall-clock timing; release gate"]` test with `--ignored --nocapture --test-threads=1`, and parses the `GATE <name> <measured_ms> <budget_ms>` line each gate prints before it asserts. A missing line, an over-budget gate or a failed cargo target is a failure. `--with-benches` adds the two `arda-gen` budget benches (about 6 minutes of criterion sampling, so they are opt-in; they also print `GATE` lines); `--from-log FILE` re-parses a saved log. The release hardware is the maintainer's workstation (Ryzen 9 9950X3D, 16 cores / 32 threads, about 91 GiB RAM, CachyOS); the receipt prints `lscpu`, `free`, the kernel, the commit and the load average before and after. Run it on an idle machine: gates take the best of 3 to 5 fresh measurements, but the numbers are only comparable at low load. GitHub-hosted runners are too small and noisy, so the gates stay out of `ci.yml`.

The server gates (`crates/arda-server/tests/world_api/gates.rs`) run on the settled society fixture `target/arda-server-fixture/micro42-society` (the default recipe, 9 since the recipe-9 change), the river cell (518, 822; (529, 812) on recipe 7), in-process through the real router; `e2e_village` keeps its own recipe-6 world.

### Recorded measurements

Dated timing receipts for terrain generation and Atlas export (2026-09-08 through 2026-09-29) are kept in `docs/capstone/features/2026-09-23-terrain-corrections/STUDY.md` and `EXPERIMENTS.md`, which are symlinks to a path outside the repository. They were not re-measured for this chapter and predate the crates added since v0.2.0.

**2026-10-13 gate receipt (task 09).** Commit `2f94e3b39` (tree equal to the gated tree apart from a doc-only edit), run with `tools/release-gates.sh --with-benches`, rustc 1.98.1, release profile, Ryzen 9 9950X3D, 32 threads, 91 GiB, kernel 7.2.9-2-cachyos. The machine was shared with other agents: load average 12.3 / 16.1 / 18.8 before and 14.2 / 16.7 / 18.6 after, so every figure is an upper bound on an idle machine. All gates pass.

| Gate | Measured ms | Budget ms | Result |
|---|---:|---:|---|
| refine_block | 32.675 | 250 | pass |
| refine_quarter_block | 33.477 | 100 | pass |
| tactical_64x64_128ppsq | 719.967 | 2000 | pass |
| server_cached_layout_png | 0.801 | 5 | pass |
| server_cached_cell_tile | 0.808 | 20 | pass |
| server_cold_cell_json_scene | 159.394 | 500 | pass |
| server_cold_window_128 | 501.693 | 1000 | pass |
| server_open | 1194.168 | 2000 | pass |
| world_load | 0.003 | 100 | pass |
| server_area_cells_json | 160.414 | 500 | pass |
| server_area_cells_bin | 108.900 | 250 | pass |
| server_warm_overview_tile | 0.245 | 20 | pass |
| midzoom_warm_tile_z7 | 14 | 150 | pass |
| midzoom_refine_nodes | 10 | 150 | pass |
| npc_city_20k | 78.964 | 1000 | pass |
| names_100k | 281.939 | 1000 | pass |
| e2e_cold_block_png_64 | 375.164 | 500 | pass |
| e2e_cold_quarter_window_64 | 108.623 | 250 | pass |
| gen_area_erosion | 2764.883 | 30000 | pass |
| gen_continent_stage | 2091.886 | 60000 | pass |

`world_load` is `World::load`, which reads the manifest and maps the layers lazily (about 3 µs); the first query pays the cost, which `server_open` and the cold gates include. `server_open` is `AppState::open` on the society fixture (ground textures for every ppsq, town and city plans drawn at open). The process peak RSS of the `world_api` gate binary was about 1.75 GiB (it includes the gates that ran before it); that is a process high-water mark, not the startup admission total, which, since the serving-concurrency fix (review 3.6-8), counts the tactical pipeline and the measured world and library: about 14.1 GiB of default budgets plus 0.15 GiB for the MICRO society world; it is not measured by these gates.

Startup and serving at default size (server-startup fix, 2026-10-16). Before the fix `arda-server` never became ready on a default-size world: the town-plan warm-up drew 32 plans at once across the world while the refine source re-opened its world (re-verifying the 1.25 GB `terrain/fine.bin`, about 0.3 s, under its lock) whenever more than 6 distinct areas were touched, so the readers livelocked on one core; past that, admission would have refused the world (default budgets 15.6 GiB left 0.4 GiB for a default-size world's ~1 GiB of society files, shore layer and library). Now the refine source keeps 8 decoded areas, each opened as a manifest load sharing the fine-terrain reader and shore layer verified once at start (outside its lock), the warm-up draws together only plans that read at most 8 areas between them, plan draws reserve an estimate within the 768 MiB plan budget before drawing, and the default budgets are trimmed (`--max-connections` 32, query areas 256 MiB, fine windows 128 MiB, overview caches 128 MiB each). Measured on the seed-42 recipe-9 500 × 1000 km world, settled and society built (14,672 settlements), default config and the placeholder library, release build, 32 cores: ready on `/v1/health` in 12.8 to 13.0 s, peak RSS 2.6 GiB, admitted 15,854 MiB of 16,384 MiB (configured 14.1 GiB; resident: society 772 MiB, plans 486 MiB (largest city), shore 2 × 43 MiB, library 89 MiB). Sixteen concurrent cold `/v1/tactical/cell/{gx}/{gy}` requests in 16 different areas (8 cities, 4 towns, 2 villages, 2 hamlets): all done in 101 to 134 s over three runs, peak RSS 3.8 to 4.0 GiB; with plan draws unbounded (the startup fix alone) the same load took 76 s at 6.3 GiB, above what admission allows for plans, with identical bodies. City plans (estimate about 750 MB each) are drawn one at a time; across the 80 plans drawn the largest was 81 % of its estimate. MICRO society fixture: ready in 1.9 s (2.0 s before), `server_open` 1853 ms in the serial gate run (1918 ms before). The ignored `world_api::startup_scale` test (`ARDA_WORLD=<settled default-size world>`) holds the open under 60 s.
