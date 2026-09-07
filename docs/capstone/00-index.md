# Arda — reference index

Deterministic offline tabletop world generation in a Rust 2021 Cargo workspace: continent → 512² 100 m area cells → sampled 64² five-foot tactical blocks. CLI, reusable library and local Docker image; MIT/Apache-2.0. Topic chapters describe current code; retained product designs, standards and diagnosis reports are separate companions.

## Module map

| Module | Purpose | Entry points (repository-relative) |
|---|---|---|
| arda-core | Domain types, coordinates, keyed RNG, manifest and layer codecs | `crates/arda-core/src/lib.rs:8` |
| arda-gen | Continent/area/block generation, independent area fan-out and disk batch | `crates/arda-gen/src/lib.rs:9`, `crates/arda-gen/src/orchestrator.rs:137` |
| arda-render | Area/overview/block PNG and versioned JSON | `crates/arda-render/src/lib.rs:6` |
| arda | Public generate/load/query/export facade | `crates/arda/src/lib.rs:22`, `crates/arda/src/lib.rs:223` |
| arda-cli | Generate, Preview and Export process dispatch | `crates/arda-cli/src/main.rs:215` |
| workspace tests | Cross-platform golden-world host package | `Cargo.toml:56`, `tests/golden_world.rs:69` |

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
| interfaces | Absent: no implemented sibling-repository service/schema contract in the entry points or deployment configuration. |
| logic | [01-continent-generation](logic/01-continent-generation.md) — retained design and dated history |
| logic | [02-area-generation](logic/02-area-generation.md) — retained design and dated history |
| logic | [03-block-generation](logic/03-block-generation.md) — retained design and dated history |
| logic | [04-export](logic/04-export.md) — retained design and dated history |
| logic | [05-load-query](logic/05-load-query.md) — retained design and dated history |
| logic | [06-society-generation](logic/06-society-generation.md) — retained design and dated history |
| logic | [07-preview](logic/07-preview.md) — observed CLI scenario |

## Companion docs

| File | What it is |
|---|---|
| `generation-issues.md` (optional, local only) | User-requested diagnosis, ignored by Git; available only in checkouts where it was created. |
| [standards.md](standards.md) | Normative coding rules, migrated from code-prefs.md. |
| [implementation.md](implementation.md) | Approved build plan and dated implementation history; outstanding steps remain plans. |
| [open-items.md](open-items.md) | Existing issue/calibration history; retained without a new review. |
| [mockup-artifact.md](mockup-artifact.md) | User-provided generator logic artifact. |
| [mockup/README.md](mockup/README.md) | Retained product brief, surface inventory and assumptions. |
| [mockup/01-generate.md](mockup/01-generate.md) | Retained generate surface design. |
| [mockup/02-world-layout.md](mockup/02-world-layout.md) | Retained world-layout surface design. |
| [mockup/03-export.md](mockup/03-export.md) | Retained export surface design. |
| [mockup/04-crate-api.md](mockup/04-crate-api.md) | Retained crate-api surface design. |
| [mockup/05-docker.md](mockup/05-docker.md) | Retained docker surface design. |
| [mockup/06-serve.md](mockup/06-serve.md) | Retained serve surface design. |
| uiux | Absent: no frontend; CLI/API and exported PNGs do not constitute application views. |
| [features/](features/) | Local feature working state; retained folders are not lifecycle authority. Shipped history is in the ledger. |
| [changelog.md](changelog.md) | Append-only durable run ledger; historical names/keys retained. |
