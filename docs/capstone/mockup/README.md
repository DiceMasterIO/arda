---
generated_date: 2026-09-07
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# Mockup — arda

One seed → one continent (default 500×1000 km) → 190 areas of 512²
100 m cells → materialized 64² tactical blocks; images/JSON on demand.
Surfaces: CLI + Rust crate (no UI), so screens are surface units with
interaction transcripts. Source artifact: `../mockup-artifact.md`. These are retained product designs; implemented command forms and behavior are in `../01-architecture.md` and `../07-operations.md`.

| Screen | Scenario(s) | Status / notes |
| --- | --- | --- |
| `01-generate.md` — batch CLI | batch-generate | Confirmed design |
| `02-world-layout.md` — output volume | batch-generate, inspect-volume | Confirmed design |
| `03-export.md` — images/JSON CLI | export-vtt, inspect-volume | Confirmed design |
| `04-crate-api.md` — load & query | load-query | Confirmed design |
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
