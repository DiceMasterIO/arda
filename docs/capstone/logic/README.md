---
generated_date: 2026-09-22
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# Arda behavior scenarios

The generation, export, loading and preview chapters (01–07) describe current behavior first. Dated observations and deferred designs remain labeled within them. Chapters 08–16 are normative designs for the product layers being built in parallel branches; code cites their rules as `// logic/<nn> §<rule>`, and the [integration plan](../integration-plan.md) gives their merge order, adapters and end-to-end test.

| Scenario | Reference |
|---|---|
| Continent terrain, climate and shared annual water | [01-continent-generation.md](01-continent-generation.md) |
| Prepared terrain and immutable area composition | [02-area-generation.md](02-area-generation.md) |
| Tactical generation and deferred richer tile rules (superseded for new output by 09) | [03-block-generation.md](03-block-generation.md) |
| Saved PNG and JSON export, including Classic/Atlas PNG style | [04-export.md](04-export.md) |
| Lazy world loading and layer queries | [05-load-query.md](05-load-query.md) |
| Deferred society generation (refined by 08, 13 and 14) | [06-society-generation.md](06-society-generation.md) |
| Generate a world and preview its Classic/Atlas overview | [07-preview.md](07-preview.md) |
| Settlements, land use, roads, crossings, passes and realms | [08-settlements-roads-realms.md](08-settlements-roads-realms.md) |
| Tactical refinement: 5-ft terrain, WFC, edge rules and seams | [09-tactical-refinement.md](09-tactical-refinement.md) |
| Town layout: streets, plots, buildings and interiors | [10-town-layout.md](10-town-layout.md) |
| Tactical art: catalogue, validator, compositor and lighting | [11-tactical-art-compositor.md](11-tactical-art-compositor.md) |
| Scene data: SRD movement, cover, line of sight and lights | [12-scene-data.md](12-scene-data.md) |
| NPC population: jobs, personality, SRD sheets and regeneration | [13-npc-population.md](13-npc-population.md) |
| Society and history: polities, offices and a consistent past | [14-society-history.md](14-society-history.md) |
| Naming: deterministic, culture-consistent, original names | [15-naming.md](15-naming.md) |
| Service API: HTTP routes, versioning, caching and TS bindings | [16-service-api.md](16-service-api.md) |

## Rule index (08–16)

| Spec | Rules |
|---|---|
| 08 | §world-frame, §settle-suitability, §settle-refusals, §settle-sites, §settle-tiers, §settle-placement, §settle-functions, §settle-building-mix, §landuse, §roads, §crossings, §passes, §realm-seats |
| 09 | §square-frame, §elevation, §linear-features, §reservations, §ground-field, §wfc, §scatter, §rules-sidecar, §hash |
| 10 | §town-frame, §town-footprint, §town-streets, §town-plots, §town-functions, §town-footprints, §town-walls, §town-interiors, §town-wfc, §town-capacity, §town-building-spec, §town-function-keys, §town-clip |
| 11 | §catalogue, §vocabulary, §tag-query, §validator, §placeholders, §compose-order, §ground-blend, §walls-assembly, §seam-art, §lighting, §dress, §ppsq, §render-identity |
| 12 | §scene-frame, §scene-movement, §scene-climb, §scene-diagonal, §scene-cover, §scene-sight, §scene-lights, §scene-regions, §scene-spawn, §scene-sidecar, §scene-tokens |
| 13 | §npc-inputs, §npc-id, §npc-seed, §npc-regeneration, §npc-storage, §npc-households, §npc-jobs, §npc-personality, §npc-relationships, §npc-sheet, §npc-notables, §npc-queries, §npc-game-schema |
| 14 | §soc-calendar, §soc-founding, §soc-realms, §soc-relations, §soc-events, §soc-offices, §soc-present, §soc-regions |
| 15 | §name-key, §name-inventories, §name-site-suffix, §name-form, §name-unique, §name-what-is-named |
| 16 | §api-conventions, §api-versioning, §api-routes, §api-cell-society, §api-tactical, §api-tiles, §api-cache, §api-prefetch, §api-errors, §api-bindings, §api-gm-path |
