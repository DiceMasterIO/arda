---
generated_date: 2026-10-10
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# Arda behavior scenarios

The generation, export, loading and preview chapters (01–07) describe current behavior first. Dated observations and deferred designs remain labeled within them. Chapters 08–16 are normative designs for the product layers being built in parallel branches; code cites their rules as `// logic/<nn> §<rule>`, and the [integration plan](../integration-plan.md) gives their merge order, adapters and end-to-end test. Chapter 17 restates the mid-zoom relief design against the merged code, with source sites per rule. Chapters 18–22 are descriptive extractions of current behavior (stamped with `paths_covered`); their rule names label observed behavior.

| Scenario | Reference |
|---|---|
| Continent terrain, climate and shared annual water | [01-continent-generation.md](01-continent-generation.md) |
| Prepared terrain and immutable area composition | [02-area-generation.md](02-area-generation.md) |
| Retired in 1.0: the sampled WFC block archive, superseded by 09 | [03-block-generation.md](03-block-generation.md) |
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
| Mid-zoom relief: on-demand ~10 m refinement, relief tiles and worked land | [17-midzoom-relief.md](17-midzoom-relief.md) |
| Dungeons and caves: `arda dungeon` and `/v1/tactical/dungeon*` | [18-dungeons-and-caves.md](18-dungeons-and-caves.md) |
| Art import: `arda tactical import`, raw AI images to a validated library | [19-art-import.md](19-art-import.md) |
| Art generation: `tools/art-gen` prompt pack and Slopify driver | [20-art-generation.md](20-art-generation.md) |
| Society build: `arda society build` (town plans, society simulation, stored notables) | [21-society-build.md](21-society-build.md) |
| Developer inspection: `arda-settle render`, `arda-town`, `arda-refine stats/survey`, `/v1/tactical/library` | [22-developer-inspection.md](22-developer-inspection.md) |

## Rule index (08–22)

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
| 17 | §frame, §base, §detail, §amplitude, §mantle, §means, §pits, §render, §water, §rivers, §low-relief, §land, §land-fields, §land-roads, §land-towns, §land-tones, §pyramid |
| 18 | §dgn-params, §dgn-site-seed, §dgn-rng, §dgn-zones, §dgn-rooms, §dgn-corridors, §dgn-purposes, §dgn-openings, §dgn-prison, §dgn-caverns, §dgn-mouth, §dgn-water, §dgn-ground, §dgn-dress, §dgn-rules-sidecar, §dgn-serve, §dgn-cli-outputs |
| 19 | §imp-cli, §imp-scan, §imp-naming, §imp-ids, §imp-manifest, §imp-vocab, §imp-metadata, §imp-provenance, §imp-ppsq, §imp-texture, §imp-matte, §imp-shadow, §imp-holes, §imp-cleanup, §imp-fit, §imp-wall, §imp-structured, §imp-grade, §imp-validate, §imp-coverage, §imp-output, §imp-report, §imp-contact, §imp-exit, §imp-determinism |
| 20 | §gen-pack, §gen-keywords, §gen-frame, §gen-tiers, §gen-select, §gen-guard, §gen-estimate, §gen-licence, §gen-preflight, §gen-library-prompts, §gen-batch, §gen-parallel, §gen-review, §gen-state, §gen-collect, §gen-files, §gen-manifest, §gen-retry, §gen-exit |
| 21 | §build-inputs, §build-seed, §build-plan-all, §build-plan-site, §build-simulate, §build-buildings, §build-notables, §build-write, §build-report, §build-read-path |
| 22 | §insp-settle-overlay, §insp-settle-guard, §insp-settle-quality, §insp-town-sites, §insp-town-window, §insp-town-block-render, §insp-town-overview, §insp-town-summary, §insp-town-export, §insp-refine-stats, §insp-refine-survey, §insp-library-summary |
