## 2026-09-23 — Refresh terrain and render references
key: map/all@757b2ab

- **Trigger:** covered terrain/render source changed; implemented behavior and current verification replaced stale active claims. Historical candidate results remain dated.
- **Architecture:** `01-architecture.md` records Euclidean belts, coarse creep/fine incision, height-backed Atlas shore reconstruction and bounded context.
- **Models:** `02-models.md` adds AtlasTerrain heights and internal AtlasSea semantics; persisted schema remains unchanged.
- **Conventions:** `03-conventions.md` refreshes whole-crates lexical counts and current integer algorithms.
- **Data flow:** `04-data-flow.md` records the four fixed Atlas arrays (2,641,960 bytes), shared shoreline decision and corrected terrain stages.
- **Testing:** `06-testing.md` records correction controls, combined broad/focused/golden gates, two dry review rounds and actual-world evidence with explicit limits.
- **Operations:** `07-operations.md` records shoreline behavior, updated scratch shape and measured generation/32K costs.
- **Glossary:** `08-glossary.md` distinguishes coarse creep from fine incision and defines Atlas shore reconstruction.
- **Logic:** `logic/01-continent-generation.md`, `logic/02-area-generation.md`, `logic/04-export.md` and `logic/07-preview.md` absorb the corrected source behavior without rewriting historical designs.
- **Status/index:** `open-items.md` and `00-index.md` distinguish verified implementation from remaining landscape-naturalness acceptance; no completion marker is invented.
- **Dependencies:** `05-dependencies.md` reconciles the workspace Apache-2.0 license and deduplicated CI golden invocation; installed dependencies and lockfile are unchanged.
- **Absent:** no frontend or cross-repository interface became applicable.
