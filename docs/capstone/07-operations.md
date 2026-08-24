---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: [".github/**", "Dockerfile", "Cargo.toml"]
---

> Prescriptive — written from the design interview, not from code.

# Operations

No hosted service — operations = CI, releases, and the local/docker run
story (`architecture-interview.md §D4, §Q6`).

## Processes

| Process | Local command | Container |
|---|---|---|
| Batch generate | `cargo run -p arda-cli -- generate --seed <n> --out <dir>` (release builds for real runs) | `docker run -v $PWD/worlds:/worlds ghcr.io/<owner>/arda generate …` (`mockup/05`) |
| Export | `cargo run -p arda-cli -- export --world <dir> …` | same image, `export` subcommand |

The binary runs to completion and exits; no daemons, no ports.

## Configuration

No environment variables planned. All configuration is CLI flags plus
the optional `--config` file (`mockup/01`; schema at build). Memory
budget default 16 GB, configurable (§Q3, flag shape deferred — §D9).

## Infrastructure

- `Dockerfile` at repo root: multi-arch (amd64/arm64) image, entrypoint `arda`, no exposed ports, volumes for `/worlds` (+ any `--out` mount) — `mockup/05`.
- `.github/workflows/`: CI (push/PR — test, clippy, fmt, golden-hash gate on ubuntu/macos/windows runners) and release (tag → publish workspace crates to crates.io, build+push GHCR image) — §Q6.

## Developer workflow

- Tests: `cargo test --workspace`; benches: `cargo bench` (§Q7 gates).
- Lint/format: `cargo clippy --workspace -- -D warnings`; `cargo fmt --check`.
- Type check: `cargo check --workspace`.
- Migrations: none — no database; world format changes bump `format_version` (`02-models.md` Schema).
- Trunk-based; PRs optional while solo (§Q6, §Q1).
