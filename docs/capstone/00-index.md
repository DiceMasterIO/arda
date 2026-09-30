# Arda — reference index

> Evidence consolidated on September 25: the [study](features/2026-09-23-terrain-corrections/STUDY.md) and [experiment catalogue](features/2026-09-23-terrain-corrections/EXPERIMENTS.md) replace the raw terrain-corrections archive. Historical filenames below identify removed experiments; current evidence links lead to their retained summaries. Original/final worlds and the 32K output remain under `out/terrain-delivery/`.


Deterministic offline tabletop world generation in a Rust 2021 Cargo workspace: continent → 512² 100 m area cells → sampled 64² five-foot tactical blocks. CLI, reusable library and local Docker image; Apache-2.0. Topic chapters describe current code; retained product designs, standards and diagnosis reports are separate companions.

## Module map

| Module | Purpose | Entry points (repository-relative) |
|---|---|---|
| arda-core | Domain types, coordinates, canonical fine-field sampling, keyed RNG, manifest and layer codecs | `crates/arda-core/src/lib.rs` |
| arda-gen | Canonical terrain preparation, shared annual water, immutable area/block composition and disk batch | `crates/arda-gen/src/lib.rs`, `crates/arda-gen/src/orchestrator.rs` |
| arda-render | Classic/Atlas area and overview PNG, block PNG and versioned JSON | `crates/arda-render/src/lib.rs` |
| arda | Public generation, lazy load/query and saved-layer export facade | `crates/arda/src/lib.rs`, `crates/arda/src/world.rs` |
| arda-cli | Generate, Preview and Export process dispatch | `crates/arda-cli/src/main.rs` |
| workspace tests | Cross-platform golden-world host package | `Cargo.toml:56`, `tests/golden_world.rs` |

## Topics

| Topic | File |
|---|---|
| architecture | [01-architecture.md](01-architecture.md) |
| models | [02-models.md](02-models.md) |
| conventions | [03-conventions.md](03-conventions.md) |
| data-flow | [04-data-flow.md](04-data-flow.md) |
| dependencies | [05-dependencies.md](05-dependencies.md) |
| testing | [06-testing.md](06-testing.md) |
| operations | [07-operations.md](07-operations.md) |
| glossary | [08-glossary.md](08-glossary.md) |
| interfaces | Not yet merged: the HTTP service contract is specified in [16-service-api](logic/16-service-api.md) and built on feat/world-server and feat/server-tactical; see the [integration plan](integration-plan.md). |
| logic | [Scenario index](logic/README.md) |
| logic | [01-continent-generation](logic/01-continent-generation.md) — current behavior and retained history |
| logic | [02-area-generation](logic/02-area-generation.md) — current behavior and retained history |
| logic | [03-block-generation](logic/03-block-generation.md) — retained design and dated history |
| logic | [04-export](logic/04-export.md) — current behavior and retained history |
| logic | [05-load-query](logic/05-load-query.md) — current behavior and retained history |
| logic | [06-society-generation](logic/06-society-generation.md) — retained design and dated history |
| logic | [07-preview](logic/07-preview.md) — observed CLI scenario |
| logic | [08-settlements-roads-realms](logic/08-settlements-roads-realms.md) — normative product design (goals 34–39, 41) |
| logic | [09-tactical-refinement](logic/09-tactical-refinement.md) — normative product design (goals 42–43, 46–50) |
| logic | [10-town-layout](logic/10-town-layout.md) — normative product design (goals 36, 44–45, 55–56) |
| logic | [11-tactical-art-compositor](logic/11-tactical-art-compositor.md) — normative product design (goals 49, 58–64) |
| logic | [12-scene-data](logic/12-scene-data.md) — normative product design (goals 48, 57, 65, 67, 69) |
| logic | [13-npc-population](logic/13-npc-population.md) — normative product design (goals 51–57, 69) |
| logic | [14-society-history](logic/14-society-history.md) — normative product design (goals 36, 38, 40, 53, 56) |
| logic | [15-naming](logic/15-naming.md) — normative product design (goals 40, 41, 53) |
| logic | [16-service-api](logic/16-service-api.md) — normative product design (goals 41, 57, 65–70) |

## Companion docs

Current terrain milestone: opt-in recipe-2 source and Atlas delivery completed, including the seed-42 full 32K output; see [status](open-items.md) and [rendered evidence](features/2026-09-23-terrain-corrections/EXPERIMENTS.md#accepted-look-qualification).

| File | What it is |
|---|---|
| [Map legend](../map-legend.md) | Saved offline colour swatches and interpretation for world and detailed area PNGs. |
| `generation-issues.md` (optional, local only) | User-requested diagnosis, ignored by Git; available only in checkouts where it was created. |
| [standards.md](standards.md) | Normative coding rules, migrated from code-prefs.md. |
| [integration-plan.md](integration-plan.md) | Merge order, expected conflicts, cross-crate adapters and the end-to-end acceptance test for the parallel product branches; lists the inconsistencies found between their goal prompts. |
| [implementation.md](implementation.md) | Retained build plan with source-audited completion/partial-completion states; old skeleton ceilings are reconciled with installed code. |
| [open-items.md](open-items.md) | Source-audited status plus a detailed work queue with dependencies, implementation steps and completion gates; covers every earlier open item and the world/area/tactical-map requirements, preserving historical evidence. |
| [World, area and tactical map roadmap](../tactical-map-roadmap.md) | Proposed visual pipeline and per-scale requirements, including richer world rendering and shared layouts; detailed delivery tasks live in open-items.md. |
| [mockup-artifact.md](mockup-artifact.md) | User-provided generator logic artifact. |
| [mockup/README.md](mockup/README.md) | Retained product brief, surface inventory and assumptions. |
| [mockup/01-generate.md](mockup/01-generate.md) | Retained generate surface design. |
| [mockup/02-world-layout.md](mockup/02-world-layout.md) | Retained world-layout surface design. |
| [mockup/03-export.md](mockup/03-export.md) | Current PNG/JSON export surface and deferred tactical asset design. |
| [mockup/04-crate-api.md](mockup/04-crate-api.md) | Retained crate-api surface design plus the implemented styled export API. |
| [mockup/05-docker.md](mockup/05-docker.md) | Retained docker surface design. |
| [mockup/06-serve.md](mockup/06-serve.md) | Retained serve surface design. |
| uiux | Absent: no frontend; CLI/API and exported PNGs do not constitute application views. |
| `features/` (local only) | Active specifications, plans and retained evidence. The geographical-rendering first pass is implemented; the September 23 terrain/shoreline corrections, material/lake-depth rendering, structural relief and saved-wetness tint are integrated through `14a70b9144dd`, including connected overview channel geometry, tapered display widths and fractional bank coverage, with actual small/default worlds, MICRO99, reviewed deterministic baselines and repeated 32K output. `2026-09-23-terrain-corrections` tracks the remaining broad visual objective, opt-in canonical fine-source worlds, bounded fine Atlas exports, recipe-2 ocean-rim/lowland-gain correction and the exact plateau/wetness attribution. Mountain grandeur, believable lowland rendering, multiple-world qualification and the new full 32K delivery remain open. Earlier absent local galleries do not undo their committed implementation and durable ledger. |
| [changelog.md](changelog.md) | Append-only durable run ledger; historical names/keys retained. |
