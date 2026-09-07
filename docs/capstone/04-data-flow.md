---
generated_at_commit: 987aeca04c77
generated_date: 2026-09-07
content_hash: b30247c202b0
paths_covered: [":(top)crates/**"]
capstone_version: 6.4
---

# Data flow

## Lifecycles

1. **Generate:** `crates/arda-cli/src/main.rs:100` → `crates/arda/src/lib.rs:22` → `crates/arda-gen/src/orchestrator.rs:137`. Up to five candidate attempts apply the 250–900‰ land gate, compute climate/hydrology/rivers, then require a river with `feeds: None`. The designed ≥1,500 m range gate is absent. An accepted `Continent` is shared across Rayon area workers.
2. **Continent:** `crates/arda-gen/src/continent/mod.rs:115` runs seeded plates, 20 tectonic steps on a 4 km grid, shelf/coast construction, 1 km resampling and erosion. Climate and hydrology follow in the orchestrator, rather than feeding rainfall back into erosion. The original design called for roughly 100 tectonic steps and later human geography/naming (`logic/01-continent-generation.md`).
3. **Area:** `crates/arda-gen/src/orchestrator.rs:96` → `bundle_for` → `crates/arda-gen/src/area/mod.rs:656`: relief, 40 erosion iterations, fill, rainfall sampling, water routing, composition. Erosion tapers to zero over 32 cells at the pinned rim (`crates/arda-gen/src/area/erosion.rs:24`). Water accumulates rainfall-driven discharge plus entering rivers and initiates channels at 40 L/s (`crates/arda-gen/src/area/water.rs:19`).
4. **Lakes and cells:** `crates/arda-gen/src/area/mod.rs:321` clamps/trims basins, retains ≥300 cells and ≥4 m maximum depth, then computes outlets against the final surviving set. Selection has no evaporation or water-balance calculation. Stream incision skips every filled depression before final lake acceptance, while creep still runs (`crates/arda-gen/src/area/erosion.rs:109`). Stored heights remain raw; routing uses filled heights. `compose` writes rainfall and hydrology fields; temperature, moisture, forest density, road and built_by retain defaults (`crates/arda-gen/src/area/mod.rs:185`). Climate/vegetation/settlement/land-use/road stages remain planned.
5. **Blocks:** `crates/arda-gen/src/orchestrator.rs:96` samples land cells every 64 cells in x and y → `constraints_for` → `fill_block` → one compressed archive per area. Constraints choose broad tile groups; river-bearing land excludes Water. Every square starts with the same allowed set, minimum-option ties use row order, a random choice restricts only undecided direct neighbors. There are no fixed shared crossings, count constraints or coherent building layouts (`crates/arda-gen/src/block/constraints.rs:17`, `crates/arda-gen/src/block/wfc.rs:54`). The designed 200+ vocabulary and full land-cell coverage remain pending; current vocabulary is 24.
6. **Load/query:** `crates/arda/src/lib.rs:223` reads manifest, every area's cells and objects, and every block archive eagerly; lookups read BTreeMaps. This differs from the intended lazy O(accessed) cache and per-block decompression (`logic/05-load-query.md`). Continent layers are not read by `World::load`.
7. **Export:** `crates/arda-cli/src/main.rs:157` → load world → create output directory → facade export → renderer/serializer → `std::fs::write`. Area PNG is one pixel per 100 m cell; every channel occupies that pixel irrespective of stored physical width (`crates/arda-render/src/carto.rs:184`). Overview is aggregated from loaded area cells, using discharge cuts 4/20/80 m³/s and a 25% lake-coverage threshold (`crates/arda-render/src/carto.rs:122`). Block PNG uses flat 8×8 pixel color squares (`crates/arda-render/src/symbolic.rs:30`).
8. **Preview:** `crates/arda-cli/src/main.rs:129` creates `out/world`, generates there, eagerly loads it and renders `out/overview.png`. It executes the area/block batch too; it is not a continent-only shortcut. `logic/07-preview.md` owns this composed scenario.

## State

The directory stores completed layers; manifest presence/version is the load gate (`crates/arda-core/src/formats/manifest.rs:75`). Generation owns mutable candidate/area buffers, then writes independent paths. World owns all decoded areas and block archives in memory (`crates/arda/src/lib.rs:209`). No sessions, service state or UI stores exist. The planned 16 GB worker budget has no implemented enforcement in `crates/arda-gen/src/orchestrator.rs:137`.

## Side-effect boundaries

Generation writes directly through `crates/arda-gen/src/orchestrator.rs:84`; codecs encode/decode bytes while manifest helpers also perform IO. Facade and CLI perform export/load filesystem IO (`crates/arda/src/lib.rs:59`, `crates/arda-cli/src/main.rs:157`). No network side effects are implemented.

## Failure paths

Generation rejects a nonempty output directory; failed IO can leave partial files, and retrying that same directory is refused. Manifest is written last but is not an atomic rename, so interruption during its write can leave malformed JSON; load reports it unreadable (`crates/arda-core/src/formats/manifest.rs:59`). No resume, rollback, output lock, cancellation handler or custom tile-panic recovery exists in the orchestrator. Concurrent writers are not coordinated.

WFC tries attempts 0–7, then fills the block with its first allowed tile (ID 0 if empty) and marks it relaxed (`crates/arda-gen/src/block/wfc.rs:19`). Export load/render failures occur before writing the artifact, but a filesystem write can truncate an existing file or leave partial bytes; the earlier whole-file atomicity promise is not implemented (`crates/arda/src/lib.rs:59`). Overview drops area lookup errors via `filter_map` (`crates/arda/src/lib.rs:93`). JSON serializers return an empty string on serde failure (`crates/arda-render/src/json.rs:178`). CLI propagates `anyhow::Result` from main; no retries or timeout policy exists (`crates/arda-cli/src/main.rs:215`).
