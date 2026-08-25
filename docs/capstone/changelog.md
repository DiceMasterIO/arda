---
generated_date: 2026-08-24
---

# Changelog

## 2026-08-25 — build: code@Q5
key: build/code@Q5

- `crates/arda-core/**` — coords/fixed/rng/error/config/cell/objects/tiles plus `formats::{manifest,cells,objects,blocks}`, the workspace's only byte codec. `cells.bin` row pinned at 33 bytes carrying the full artifact field list (skeleton fills relief+water only, rest default); `objects.bin` is a tagged section container whose unknown kinds are skipped, so step 5's settlements/roads/crossings/passes are additive within format major 1. Rejected: `unsafe` transmute for row layout (code-prefs §Q1), a dev-dependency temp-dir crate (hand-rolled instead).
- `crates/arda-gen/**` — hand-rolled integer value noise; continent stage (Voronoi plates, kinematic-lite 20-step coupled tectonics, isostasy, coast, 4km→1km upsample); tile bundles; area relief+water; 24-tile WFC block fill; orchestrator with rayon fan-out. Deferred to step 4: climate, hydrology objects, human geography, naming, and the full 100-step tectonics loop behind spike S1. Deferred to step 5: the five remaining area stages. Deferred to step 6: the 200+ tile vocabulary behind spike S2.
- `crates/arda-render/**`, `crates/arda/**`, `crates/arda-cli/**` — symbolic block PNG, cartographic area PNG, versioned JSON (schema_version 1); `World`/`Area` facade; `generate`/`export` subcommands.
- `tests/golden_world.rs` + `tests/golden/micro-42.txt` — 26-file blake3 fingerprint, wired into the 3-OS CI matrix as the §Q4 gate. Not yet observed green on macOS/Windows — only Linux has run.
- Divergences from `05-dependencies.md`, each needing a chapter update: `blake3` promoted from dev/tooling to an `arda-core` runtime dependency (the subseed derivation needs it); `thiserror` added workspace-wide (code-prefs §Q4 mandates thiserror-style derives, the chapter names no error crate); `anyhow` added to `arda-cli` only (code-prefs §Q4 permits it there).
- Divergence from `implementation.md`: its orchestrator sketch omitted `logic/01` §Q9's validation reroll. Added — seed 43 fails the land-fraction gate at attempt 0 and is rerolled deterministically, ≤5 attempts.
- Known-weak, left open: area drainage does no depression filling, so channels emerge as short disconnected fragments rather than networks reaching the sea. The Horton/Hack/rank-size gates that would catch this are build-order step 12. Blocks are sampled on a 64-cell stride, not one per land cell. Areas and blocks load eagerly, not lazily as `logic/05` specifies. All three marked in-code.

