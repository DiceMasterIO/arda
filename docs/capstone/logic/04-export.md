---
generated_date: 2026-09-08
scenario: export
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# 04 — Export

Renders PNG and serializes JSON from saved world data on demand. The current CLI surface is documented in [export](../mockup/03-export.md); loading follows [load and query](05-load-query.md). This chapter describes the implemented consumer behavior as of 2026-09-08. The five-world C06 data/export panel and approved golden pass; remaining visual realism findings are recorded in [open items](../open-items.md).

## Trigger & preconditions

- Trigger: `arda export`, or the public `export_area`, `export_area_with_scale`, `export_overview` and `export_block` functions.
- The CLI requires `--world <dir>` and `--out <dir>`. The library receives a loaded `World` and an existing output directory; the CLI creates its output directory after loading the manifest.
- `world.json` must parse, have format major 4 and contain a valid configuration with matching area dimensions. Its presence is the generator's completion stamp, not proof that every lazily read layer is accessible or valid. Other majors, including preserved format-3 worlds, are refused; no migration runs during export.

## Steps

1. Reject `--detail` combined with JSON, overview or a block before loading the world or creating output. Resolve an explicit `--block ax,ay,cx,cy`, otherwise `--overview`, otherwise `--area ax,ay` (default `0,0`). The CLI accepts one `--format png|json`, defaulting to PNG.
2. Read the requested saved data. Area PNG/JSON use owned, uncached `World::read_area` reads of cells and objects. Overview visits every exported area through the same owned read and releases each area after adding its cells to the raster. A block request uses the separate cache for its area's entire decompressed block archive; it does not load area cells or objects.
3. Render the selected PNG. Area Preview is 512×512 pixels over 512×512 100 m cells; Detail is 4096×4096 over the same physical bounds and saved terrain. Channels use saved adjacent centerline edges, physical widths, terminal footprints and incident junction geometry. Both scales measure physical coverage; Preview adds a separately bounded faint mark for subpixel streams. Saved sea/lake classifications determine standing-water extent. Area lakes use blue depth shading where supplied surfaces permit it; depth changes color only. Overview uses categorical standing-water fill and the existing coarse discharge bands, at 48 pixels per area in the export CLI. Blocks use the unchanged built-in symbolic 24-tile vocabulary, at eight pixels per square: a 64×64 block produces a 512×512 PNG.
4. For area JSON, serialize one combined document containing `cells`, local `rivers` and `lakes`, saved `channel_edges`, and copied `hydrology` context. JSON schema version is 2; the area's hydrology model revision is 2 with `representative_annual_balance` and `mean_annual_discharge` semantics. Global water IDs and whole annual litre amounts use decimal strings; mean discharge retains the `*_milli_cumecs` keys and L/s units. Cell JSON contains height, terrain, cover, slope, aspect, drainage area, discharge, watercourse order/width, height above river and wetness. It omits stored temperature, rainfall, moisture, forest density, road and `built_by` fields. It is not a complete VTT properties payload. Block JSON uses schema 2 and contains the saved square IDs, `relaxed` flag and an ID/name legend for the existing 24 tiles.
5. Write the completed byte buffer to `area_AX_AY.png`, `area_AX_AY_detail.png`, combined `area_AX_AY.json`, `overview.png`, or `block_AX_AY_CX_CY.png|json`. Area indices have at least two digits; block cell indices have at least three. The CLI prints `wrote <path>` for the single artifact.

## Branches

- Area: PNG Preview, PNG Detail, or combined JSON. Request PNG and JSON in separate calls.
- Overview: always PNG. `--format` and `--area` do not change this branch. If both `--block` and `--overview` are supplied, the block branch takes precedence.
- Exact-size overview: `OverviewRaster::new_exact` supports 1–78 areas per axis, at least one output pixel per area per axis, at most 16,384 pixels per axis and at most 134,217,728 pixels total. Uneven area pixel partitions cover the whole raster; enlargement repeats saved cells without adding terrain. The `export_world_16k` workspace example uses the manifest aspect ratio and refuses existing output via exclusive creation. It is separate from the regular CLI overview command and retains the same palette and feature rules.
- Block: symbolic PNG or JSON for a materialized cell. Generation currently saves blocks only at a 64-cell stride over land; a valid in-range cell need not have a block.

## Unhappy paths

- Malformed selectors, invalid Detail combinations, absent/unreadable manifests and incompatible format majors produce CLI failure or the corresponding typed library error.
- Out-of-range area/cell coordinates or an unmaterialized block return a typed range error. CLI block failures add the sampled-land-block explanation.
- Missing, inaccessible, corrupt or truncated saved layers can fail after manifest loading. Area codecs bound cell/object input bytes before allocation and validate copied water geometry against the manifest-derived fine domain.
- Invalid lake/channel geometry, raster resource limits, PNG encoding and area JSON serialization failures propagate through `ExportError::Render`. An unknown symbolic tile ID produces `UnmappedTile`.
- Output creation/write failures propagate with path context. Existing output files are overwritten without confirmation. Rendering/serialization finishes before `std::fs::write`, but a failed write can leave a truncated or partial artifact; export does not promise atomic replacement.

## State transitions

Saved world layers remain unchanged. Area and overview export reads do not populate retained area caches; a block export can populate its area's block-archive cache. The output artifact is created or overwritten. No terrain or hydrology generation runs during export.

## Invariants

- Repeating the same request over unchanged saved data with the same implementation produces identical PNG/JSON bytes, including after reloading the world.
- Detail increases image resolution, not the 100 m terrain grid or tactical detail. Narrow channel coverage never becomes an opaque 100 m square merely because the preview has one pixel per cell.
- Annual water fields describe climatological support, not a dated weather snapshot, seasonal minimum or perennial guarantee. Potential spill and supported annual outflow remain distinct saved facts.
- JSON semantics are explicitly versioned. The historical schema-1 additive-only proposal does not describe the current schema-2 contract.
- Tactical generation rules and the 24-tile vocabulary are unchanged by area water/detail export.

## Outcomes & side effects

- Success returns the written path; the CLI prints that path and exits successfully. No artifact-size or render-time promise is part of this surface.
- Failure leaves saved layers unchanged but may leave the output directory or a partial artifact. This differs from generation's separate manifest-last publication contract.
- Source authorities: `crates/arda-cli/src/main.rs`, `crates/arda/src/{lib,world}.rs`, and `crates/arda-render/src/{carto,channels,channel_geometry,json,hydrology_json,symbolic,overview}.rs`.

## Dimensions not in play

User tileset manifests, sprites/artwork, glyphs, collision/movement attributes, complete per-cell VTT payloads, settlements, roads, road crossings, passes, buildings, population, NPCs and named continent/society exports remain deferred. Saved shared water crossings describe river boundary flux, not road or settlement crossings.

The 2026-08-25 build-gate amendments proposed enriched tile legends (material, traversability, movement cost, cover and hazards), POIs/buildings, NPC sheets, realms and an `arda serve` consumer sharing the serializers/renderers. Those remain deferred designs, as do the original separate `cells.json`/`objects.json` exports and schema-1 additive-only proposal. [Serve](../mockup/06-serve.md) remains a future surface.
