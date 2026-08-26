---
mode: prescriptive
generated_date: 2026-08-26
paths_covered: ["crates/**", "Cargo.toml"]
generated_at_commit: 9c48e00
---

> Prescriptive — written from the design interview, not from code.

# Architecture

## Layers

Cargo workspace, one repo (`architecture-interview.md §Q2`). Crates and
allowed dependency direction (enforced by the crate graph — a crate not
in another's `Cargo.toml` cannot be imported):

| Crate | Planned path | Contains | May depend on |
|---|---|---|---|
| `arda-core` | `crates/arda-core/` | Shared types (coords, cells, objects, tile IDs), the two-grid coordinate system, seed/subseed derivation (counter-based PRNG keyed (seed, tier, stage, coords, attempt) — §Q4), world formats read+write, manifest | nothing internal |
| `arda-gen` | `crates/arda-gen/` | The three generation stages: continent (`logic/01`), area (`logic/02`), block (`logic/03`); the batch orchestrator with the rayon-style pool (§Q3) | `arda-core` |
| `arda-render` | `crates/arda-render/` | Built-in symbolic style, tileset-manifest rendering, cartographic area/continent maps, JSON serialization (`logic/04` §Q14) | `arda-core` |
| `arda` | `crates/arda/` | Facade: re-exports the public API (`World::load`, `World::generate`, query types, export calls). The crate consumers depend on | `arda-core`, `arda-gen`, `arda-render` |
| `arda-cli` | `crates/arda-cli/` | The `arda` binary: `generate`/`export`/`preview`/`serve` subcommands; docker entrypoint. `serve` (build §Q1) is a synchronous read-only HTTP layer over the export renderers — it adds no generation path and no write path | `arda` |

`arda-render` never depends on `arda-gen` — rendering reads stored
worlds only (§Q2).

## Module boundaries

- Public surface = the `arda` facade crate; `arda-core`/`-gen`/`-render` are published but semver-internal (0.x, §Q6) — consumers are documented to use `arda` only.
- Inside `arda-gen`, stage modules mirror the causal pipeline (`continent`, `area::{relief,water,climate,vegetation,settlement,landuse,roads}`, `block`); a stage module may read only prior stages' output types (`logic/02` invariant), enforced by review, not tooling (single-crate interior).
- World formats live only in `arda-core::formats`; no other crate encodes/decodes bytes.
- Observed (2026-08-26, feature 02): the continent stage grew `continent/{climate,hydrology}.rs` (steps 5–6 of `logic/01`, pure functions, no RNG) and `arda-core` grew `formats::overview` plus continent types in `continent.rs`; `erode.rs`'s `fill`/`accumulate` are `pub(crate)`, reused by hydrology so the tier has one routing rule.

## Entry points

- `crates/arda-cli/src/main.rs` — the only process; subcommands `generate`, `export`, `preview`, `serve` (`mockup/01`, `mockup/03`, `mockup/06`). `preview` is generate-plus-overview in one step; `export` gained `--overview` (whole world) and `--block` (one tactical block).
- Library entry: `arda::World` (`mockup/04`).

## Communication

No queues, no IPC: all communication is in-process calls plus the world
directory on disk (`mockup/02`). The CLI↔library seam is plain function
calls — the CLI is a thin wrapper (§D6, mockup 04).

One network surface exists, added at the build gate: `serve`'s
read-only HTTP endpoints (build §Q1, `mockup/06`). It lives entirely in
`arda-cli` — no library crate opens a socket — and is synchronous, so
architecture §Q3's no-async-runtime decision still holds. Generation
remains fully offline.

## Composition

No DI container. `main()` parses args, builds a `Config`, calls facade
functions; the batch orchestrator wires stages sequentially per tier
and fans areas out over the thread pool (§Q3). Determinism forbids
wiring that depends on execution order (§Q4).

## Frontend

No human-facing UI — surfaces are `[api, cli]`; design stage formalized
`skipped: no-ui` (`design-interview.md`, `architecture-interview.md §D7`).
