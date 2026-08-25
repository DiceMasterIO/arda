# arda — design index

Deterministic procedural worldgen for tabletop: one seed → one
continent (default 500×1000 km) → 51.2 km areas of 512² 100 m cells →
64² five-ft tactical blocks. Rust workspace (crate + CLI + docker),
OSS MIT/Apache-2.0, offline batch only. Reference is prescriptive
(from the design interview); the walking-skeleton slice is now code —
chapters have not yet been resynced against it. Pipeline: mockup ✓
logic ✓ design (skipped, no UI) ✓ architecture ✓ code-prefs ✓ stack ✓
build ✓ (build-order steps 0–3 of 13; see `docs/capstone/changelog.md`).

## Topics

| # | File | Covers | Date |
|---|---|---|---|
| 01 | `docs/capstone/01-architecture.md` | Workspace crates, boundaries, entry points, composition | 2026-08-25 |
| 02 | `docs/capstone/02-models.md` | Entities, binary layers, format_version, validation | 2026-08-24 |
| 03 | `docs/capstone/03-conventions.md` | Determinism rules, error posture (rest → code-prefs) | 2026-08-24 |
| 04 | `docs/capstone/04-data-flow.md` | generate/export/load/serve lifecycles, state, failure paths | 2026-08-25 |
| 05 | `docs/capstone/05-dependencies.md` | Chosen stack: picks, floors, licenses | 2026-08-25 |
| 06 | `docs/capstone/06-testing.md` | Unit + statistical gates + golden worlds | 2026-08-24 |
| 07 | `docs/capstone/07-operations.md` | CI/CD, releases, docker, serve, dev workflow | 2026-08-25 |
| 08 | `docs/capstone/08-glossary.md` | Domain vocabulary | 2026-08-24 |

## Companion docs

| File | What it is | Date |
|---|---|---|
| `docs/capstone/mockup-artifact.md` | The traceable product mockup the surface units decompose | 2026-08-24 |
| `docs/capstone/mockup/` | Product mockup: 6 surface units + README index | 2026-08-25 |
| `docs/capstone/logic/` | Business logic: 6 scenario files (continent, area, block, export, load-query, society) | 2026-08-25 |
| `docs/capstone/implementation.md` | Approved build plan: 13 steps, layout, seams; steps 0–3 built | 2026-08-25 |
| `docs/capstone/code-prefs.md` | Normative code preferences (9 domains) | 2026-08-24 |
| `docs/capstone/changelog.md` | Append-only ledger of capstone runs | 2026-08-24 |
