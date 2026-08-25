---
mode: prescriptive
generated_date: 2026-08-25
paths_covered: [".github/**", "Dockerfile", "Cargo.toml"]
generated_at_commit: 8d3c9d9
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
| Preview | `cargo run -p arda-cli --release -- preview --seed <n> --out <dir>` | same image, `preview` subcommand |
| Serve (read-only HTTP) | `cargo run -p arda-cli --release -- serve <dir> --port 8080` | `docker run -p 8080:8080 -v $PWD/worlds:/worlds ghcr.io/<owner>/arda serve /worlds/w42 --port 8080` (`mockup/06`) |

`generate` and `export` run to completion and exit. `serve` (build
§Q1) is the one long-running process: a synchronous, read-only server
over an already-generated world. It never writes, so it holds no lock
and any number of instances may share a world directory.

## Configuration

No environment variables planned. All configuration is CLI flags plus
the optional `--config` file (`mockup/01`; schema at build), and
`serve`'s `--port` (default 8080 — `mockup/06`). Memory
budget default 16 GB, configurable (§Q3, flag shape deferred — §D9).

## Infrastructure

- `Dockerfile` at repo root: multi-arch (amd64/arm64) image, entrypoint `arda`, volumes for `/worlds` (+ any `--out` mount) — `mockup/05`. `EXPOSE 8080` for `serve` (build §Q1); `generate` and `export` ignore it.
- `.github/workflows/`: CI (push/PR — test, clippy, fmt, golden-hash gate on ubuntu/macos/windows runners) and release (tag → publish workspace crates to crates.io, build+push GHCR image) — §Q6.

## Developer workflow

- Tests: `cargo test --workspace`; benches: `cargo bench` (§Q7 gates, plus the 30 s/tile erosion budget).
- Quick look at a seed: `cargo run -p arda-cli --release -- preview --seed 42 --out ./preview` — the default continent in under 20 s, written as one overview PNG.
- Lint/format: `cargo clippy --workspace -- -D warnings`; `cargo fmt --check`.
- Type check: `cargo check --workspace`.
- Migrations: none — no database; world format changes bump `format_version` (`02-models.md` Schema).
- Trunk-based; PRs optional while solo (§Q6, §Q1).
