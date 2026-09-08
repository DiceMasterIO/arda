# Arda — reference index

Deterministic offline tabletop world generation in a Rust 2021 Cargo workspace: continent → 512² 100 m area cells → sampled 64² five-foot tactical blocks. CLI, reusable library and local Docker image; MIT/Apache-2.0. Topic chapters describe current code; retained product designs, standards and diagnosis reports are separate companions.

## Module map

| Module | Purpose | Entry points (repository-relative) |
|---|---|---|
| arda-core | Domain types, coordinates, keyed RNG, manifest and layer codecs | `crates/arda-core/src/lib.rs` |
| arda-gen | Canonical terrain preparation, shared annual water, immutable area/block composition and disk batch | `crates/arda-gen/src/lib.rs`, `crates/arda-gen/src/orchestrator.rs` |
| arda-render | Area/overview/block PNG and versioned JSON | `crates/arda-render/src/lib.rs` |
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
| interfaces | Absent: no implemented sibling-repository service/schema contract in the entry points or deployment configuration. |
| logic | [Scenario index](logic/README.md) |
| logic | [01-continent-generation](logic/01-continent-generation.md) — current behavior and retained history |
| logic | [02-area-generation](logic/02-area-generation.md) — current behavior and retained history |
| logic | [03-block-generation](logic/03-block-generation.md) — retained design and dated history |
| logic | [04-export](logic/04-export.md) — current behavior and retained history |
| logic | [05-load-query](logic/05-load-query.md) — current behavior and retained history |
| logic | [06-society-generation](logic/06-society-generation.md) — retained design and dated history |
| logic | [07-preview](logic/07-preview.md) — observed CLI scenario |

## Companion docs

| File | What it is |
|---|---|
| [Map legend](../map-legend.md) | Saved offline colour swatches and interpretation for world and detailed area PNGs. |
| `generation-issues.md` (optional, local only) | User-requested diagnosis, ignored by Git; available only in checkouts where it was created. |
| [standards.md](standards.md) | Normative coding rules, migrated from code-prefs.md. |
| [implementation.md](implementation.md) | Approved build plan and dated implementation history; outstanding steps remain plans. |
| [open-items.md](open-items.md) | Current implementation status and remaining limits, with the older issue/calibration history retained. |
| [mockup-artifact.md](mockup-artifact.md) | User-provided generator logic artifact. |
| [mockup/README.md](mockup/README.md) | Retained product brief, surface inventory and assumptions. |
| [mockup/01-generate.md](mockup/01-generate.md) | Retained generate surface design. |
| [mockup/02-world-layout.md](mockup/02-world-layout.md) | Retained world-layout surface design. |
| [mockup/03-export.md](mockup/03-export.md) | Current PNG/JSON export surface and deferred tactical asset design. |
| [mockup/04-crate-api.md](mockup/04-crate-api.md) | Retained crate-api surface design. |
| [mockup/05-docker.md](mockup/05-docker.md) | Retained docker surface design. |
| [mockup/06-serve.md](mockup/06-serve.md) | Retained serve surface design. |
| uiux | Absent: no frontend; CLI/API and exported PNGs do not constitute application views. |
| [features/](features/) | Local feature working state, including the revised static-VTT area-water-terrain-realism specification; this feature’s folder and evidence remain by explicit user instruction. Completion requires its done marker and ledger entry. |
| [changelog.md](changelog.md) | Append-only durable run ledger; historical names/keys retained. |
