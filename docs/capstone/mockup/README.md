---
generated_date: 2026-08-24
generated_at_commit: 8d3c9d9
---

# Mockup — arda

One seed → one continent (default 500×1000 km) → 190 areas of 512²
100 m cells → materialized 64² tactical blocks; images/JSON on demand.
Surfaces: CLI + Rust crate (no UI), so screens are surface units with
interaction transcripts. Interview: `../mockup-interview.md`; source
artifact: `../mockup-artifact.md`.

| Screen | Scenario(s) | Interview entries |
|---|---|---|
| `01-generate.md` — batch CLI | batch-generate | Q9, Q10, Q18, Q20, Q22 |
| `02-world-layout.md` — output volume | batch-generate, inspect-volume | Q8, Q11, Q17, Q22 |
| `03-export.md` — images/JSON CLI | export-vtt, inspect-volume | Q12, Q16, Q17, Q22 |
| `04-crate-api.md` — load & query | load-query | Q1, Q5, Q11, Q17, Q19 |
| `05-docker.md` — container | all | Q17, Q19 |
| `06-serve.md` — read-only HTTP API | serve-vtt | build-interview §Q1 (added at build gate) |

## Assumed items (invented, awaiting user review)

| Assumption | Where |
|---|---|
| Command names `generate`/`export`; all flag names | 01, 03 |
| Refuse non-empty `--out`; abort = restart (no resume); partial dirs refused by loaders | 01, 02 |
| `--config` file exists; its schema deferred | 01 |
| Every file name/format in the world directory (`world.json`, `.bin`, `.tiles.zst`) | 02 |
| Export resolutions (8 px/square, 1 px/cell); export timing estimates | 03 |
| VTT compatibility = plain images+JSON, no Foundry/Roll20 module format | 03 (Q16→Q17) |
| All Rust identifiers; CLI as thin wrapper over the crate; version-skew error | 04 |
| GHCR registry; volume-warning behavior | 05 |
