---
generated_date: 2026-09-22
generated_at_commit: 342d03e55120
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# Mockup — arda

Original target: one seed → one continent (default 500×1000 km) → 190 areas of 512²
100 m cells → materialized 64² tactical blocks. Current default exports 9×19 = 171 areas and samples blocks on a 64-cell stride; images/JSON are exported on demand.
Surfaces: CLI + Rust crate (no UI), so screens are surface units with
interaction transcripts. Source artifact: `../mockup-artifact.md`. These are retained product designs; implemented command forms and behavior are in `../01-architecture.md` and `../07-operations.md`.

| Screen | Scenario(s) | Status / notes |
| --- | --- | --- |
| `01-generate.md` — batch CLI | batch-generate | Confirmed design |
| `02-world-layout.md` — output volume | batch-generate, inspect-volume | Confirmed design |
| [03-export.md](03-export.md) — images/JSON CLI | export-vtt, inspect-volume | Current format-4/schema-2 and Classic/Atlas PNG exports; asset-rich tactical design deferred |
| [04-crate-api.md](04-crate-api.md) — load & query | load-query | Retained future accessor design, with separate current Atlas PNG facade section |
| `05-docker.md` — container | all | Confirmed design |
| `06-serve.md` — read-only HTTP API | serve-vtt | (added at build gate) |

## Assumed items (invented, awaiting user review)

| Assumption | Where |
| --- | --- |
| Command names `generate`/`export`; all flag names | 01, 03 |
| Refuse non-empty `--out`; abort = restart (no resume); partial dirs refused by loaders | 01, 02 |
| `--config` file exists; its schema deferred | 01 |
| Every file name/format in the world directory (`world.json`, `.bin`, `.tiles.zst`) | 02 |
| Export resolutions (8 px/square, 1 px/cell); export timing estimates | 03 |
| VTT compatibility = plain images+JSON, no Foundry/Roll20 module format | 03 |
| All Rust identifiers; CLI as thin wrapper over the crate; version-skew error | 04 |
| GHCR registry; volume-warning behavior | 05 |
