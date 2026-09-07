---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: ad821a345f39
paths_covered: [":(top)Cargo.toml", ":(top)crates/*/Cargo.toml", ":(top)crates/*/src/**", ":(top)Dockerfile", ":(top).github/**", ":(top)rust-toolchain.toml", ":(top)deny.toml"]
capstone_version: 6.4
---

# Operations

## Processes

| Process | Local command | Container command |
|---|---|---|
| Generate | `cargo run -p arda-cli --release -- generate --seed 42 --micro --out /tmp/arda-world` | `docker run --rm -v /host/worlds:/worlds arda-local generate --seed 42 --micro --out /worlds/w42` |
| Preview | `cargo run -p arda-cli --release -- preview --seed 42 --out /tmp/arda-preview` | Same image and volume; `preview --seed 42 --out /worlds/preview` |
| Export | `cargo run -p arda-cli --release -- export --world /tmp/arda-world --area 0,0 --format png --out /tmp/arda-export` | Same image; `export --world /worlds/w42 --overview --out /worlds/export` |

Commands resolve to `crates/arda-cli/src/main.rs:23`; all run to completion. Build the local image with `docker build -t arda-local .` (`Dockerfile:1`). Export/preview need existing readable data or a writable output path; generation refuses occupied targets. `serve` is designed but absent from the command enum.

## Configuration

No runtime environment-variable lookup is implemented in `crates/*/src/`. Compile-time `CARGO_PKG_VERSION` labels the manifest and CLI (`crates/arda-gen/src/orchestrator.rs:243`). Configuration is CLI arguments or `GenerateConfig`:

| Name | Default | Consumer | Documentation |
|---|---|---|---|
| --seed | required | generate/preview | `crates/arda-cli/src/main.rs:23` |
| --size | 500x1000 km | generate/preview | `crates/arda-cli/src/main.rs:23` |
| --micro | false; selects MICRO config | generate/preview | `crates/arda-cli/src/main.rs:100` |
| latitude / density | 35–55°N / 15 people per km² | CLI config construction | `crates/arda-cli/src/main.rs:100` |
| --px | 48 per area | preview overview | `crates/arda-cli/src/main.rs:129` |
| --area | 0,0 | export area, unless block/overview | `crates/arda-cli/src/main.rs:157` |
| --format | png; choices png/json | export area/block | `crates/arda-cli/src/main.rs:85` |
| --out / --world | required where applicable | command enum | `crates/arda-cli/src/main.rs:23` |

The planned config-file flag, memory cap and serve port are absent. Library config validation lives in `crates/arda-core/src/config.rs:73`; the deserialized manifest uses serde and exact format-version gating, not a second call to that constructor.

## Infrastructure

`Dockerfile:1` builds with `rust:1-bookworm`, copies the release binary to `debian:bookworm-slim`, runs UID 10001, declares `/worlds`, and uses entrypoint `arda`. It declares no ports, healthcheck or compose services. Multi-architecture image publication and crates.io/GHCR release automation remain designed but absent; `.github/workflows/` contains CI only.

## Developer workflow

CI commands: `cargo test --workspace --all-features`; `cargo test --test golden_world -- --nocapture`; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` (`.github/workflows/ci.yml:15`). `cargo-deny` runs through its action; configuration is `deny.toml`. Stable toolchain includes rustfmt/clippy (`rust-toolchain.toml:1`). `cargo check --workspace` checks workspace types; `cargo bench -p arda-gen` invokes the two manifest-declared benches. The planned latest-stable-minus-two MSRV matrix is absent from CI. No database migration process exists; incompatible world majors are refused (`crates/arda-core/src/formats/manifest.rs:102`).
