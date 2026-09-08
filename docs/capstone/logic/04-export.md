---
generated_date: 2026-09-08
scenario: export
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# 04 — Export

Renders PNG and serializes JSON from saved world data on demand. The current CLI surface is documented in [export](../mockup/03-export.md); loading follows [load and query](05-load-query.md). This chapter describes the implemented consumer behavior as of 2026-09-08. The five-world C06 data/export panel and approved golden pass; remaining visual realism findings are recorded in [open items](../open-items.md).

## Trigger & preconditions

- Trigger: `arda export`, or the public `export_area_with_quality`, `export_overview_with_quality`, `export_area`, `export_area_with_scale`, `export_overview` and `export_block` functions.
- The CLI requires `--world <dir>` and `--out <dir>`. The library receives a loaded `World` and an existing output directory; the CLI creates its output directory after loading the manifest.
- `world.json` must parse, have format major 4 and contain a valid configuration with matching area dimensions. Its presence is the generator's completion stamp, not proof that every lazily read layer is accessible or valid. Other majors, including preserved format-3 worlds, are refused; no migration runs during export.

## Steps

1. Validate image options before loading the world or creating output. `--quality` accepts any integer 512–32768 pixels or an integer k/K suffix, where 1K is 1024 pixels; area and overview PNGs default to 8192 (8K). Reject explicit quality with JSON, blocks or `--detail`. Reject `--detail` with JSON, overview or blocks. Resolve an explicit `--block ax,ay,cx,cy`, otherwise `--overview`, otherwise `--area ax,ay` (default `0,0`). The CLI accepts one `--format png|json`, defaulting to PNG; ordinary JSON exports without quality retain their existing behavior.
2. Read the requested saved data. Area PNG/JSON use owned, uncached `World::read_area` reads of cells and objects. Quality overview export reads owned areas as needed for bounded raster bands and releases each after use; a tile can be reread across bands. The legacy buffered overview path visits every exported area once. A block request uses the separate cache for its area's entire decompressed block archive; it does not load area cells or objects.
3. Render the selected PNG. Quality area output is square, from 512×512 through 32768×32768, defaulting to 8192×8192 over the same 512×512 100 m cells. Quality overview output uses the selected size for its long edge and rounds the short edge to the nearest pixel from the manifest's area-grid aspect ratio. Areas stream reusable rows; overviews stream bounded bands. Channels use saved adjacent centerline edges, physical widths, terminal footprints and incident junction geometry. All area scales measure physical coverage; the 512-pixel mode retains the legacy Preview's separately bounded faint mark for subpixel streams. Larger quality sizes and legacy Detail (4096×4096) show physical coverage alone. Saved sea/lake classifications determine standing-water extent. Area lakes use blue depth shading where supplied surfaces permit it; depth changes color only. Overview uses categorical standing-water fill and the existing coarse discharge bands. Blocks use the unchanged built-in symbolic 24-tile vocabulary, at eight pixels per square: a 64×64 block produces a 512×512 PNG.
4. For area JSON, serialize one combined document containing `cells`, local `rivers` and `lakes`, saved `channel_edges`, and copied `hydrology` context. JSON schema version is 2; the area's hydrology model revision is 2 with `representative_annual_balance` and `mean_annual_discharge` semantics. Global water IDs and whole annual litre amounts use decimal strings; mean discharge retains the `*_milli_cumecs` keys and L/s units. Cell JSON contains height, terrain, cover, slope, aspect, drainage area, discharge, watercourse order/width, height above river and wetness. It omits stored temperature, rainfall, moisture, forest density, road and `built_by` fields. It is not a complete VTT properties payload. Block JSON uses schema 2 and contains the saved square IDs, `relaxed` flag and an ID/name legend for the existing 24 tiles.
5. Quality exports, including default area/overview PNGs, stream to an exclusive temporary sibling file, flush and close it, then rename it to `area_AX_AY.png` or `overview.png`. Quality does not enter the filename. Legacy Detail writes `area_AX_AY_detail.png` from a completed byte buffer; combined `area_AX_AY.json` and `block_AX_AY_CX_CY.png|json` retain buffered writes. Area indices have at least two digits; block cell indices have at least three. The CLI prints `wrote <path>` for the single artifact.

## Branches

- Area: quality PNG (default 8K), legacy PNG Detail, or combined JSON. Request PNG and JSON in separate calls. The library retains `AreaImageScale::Preview` (512), `Detail` (4096) and `Custom(ImageQuality)`; Custom shows physical coverage only, while the quality facade maps 512 to Preview for compatibility.
- Overview: PNG at the selected quality, defaulting to an 8K long edge. `--area` does not change this branch. With no explicit quality, the legacy behavior of ignoring `--format` remains; explicit quality combined with JSON is rejected. If both `--block` and `--overview` are supplied, the block branch takes precedence.
- Exact-size overview: streaming `write_overview_png` supports 1–78 areas per axis and up to 32,768 pixels per axis, including a square 32K image. The buffered `OverviewRaster::new_exact` has the same axis limit but retains its 134,217,728-pixel total cap and at least one output pixel per area per axis. The regular buffered constructor retains 1–512 pixels per area and the 64-million-pixel cap. Uneven partitions cover the whole raster; enlargement repeats saved cells without adding terrain. The older `export_world_16k` example remains available with its manifest aspect ratio and exclusive destination creation; `--overview --quality 16k` now provides that image size through the CLI.
- Block: symbolic PNG or JSON for a materialized cell. Generation currently saves blocks only at a 64-cell stride over land; a valid in-range cell need not have a block.

## Unhappy paths

- Malformed selectors, invalid quality values or image-option combinations, absent/unreadable manifests and incompatible format majors produce CLI failure or the corresponding typed library error.
- Out-of-range area/cell coordinates or an unmaterialized block return a typed range error. CLI block failures add the sampled-land-block explanation.
- Missing, inaccessible, corrupt or truncated saved layers can fail after manifest loading. Area codecs bound cell/object input bytes before allocation and validate copied water geometry against the manifest-derived fine domain.
- Invalid lake/channel geometry, raster resource limits, PNG encoding and area JSON serialization failures propagate through `ExportError::Render`. An unknown symbolic tile ID produces `UnmappedTile`.
- Output creation/write failures propagate. Quality exports replace an existing completed PNG only after successful encoding/flushing; an ordinary failure preserves that PNG and attempts to remove the temporary sibling. Abrupt termination may leave a temporary file, and there is no power-loss durability promise. Legacy Detail/block/JSON exports and legacy buffered library entrypoints still call `std::fs::write` after rendering/serialization; a write failure can truncate their destination.

## State transitions

Saved world layers remain unchanged. Area and overview export reads do not populate retained area caches; a block export can populate its area's block-archive cache. The output artifact is created or overwritten. No terrain or hydrology generation runs during export.

## Invariants

- Repeating the same request over unchanged saved data with the same implementation produces identical PNG/JSON bytes, including after reloading the world.
- Quality and Detail increase image resolution, not the 100 m terrain grid or tactical detail. Narrow channel coverage never becomes an opaque 100 m square merely because the preview has one pixel per cell.
- Annual water fields describe climatological support, not a dated weather snapshot, seasonal minimum or perennial guarantee. Potential spill and supported annual outflow remain distinct saved facts.
- JSON semantics are explicitly versioned. The historical schema-1 additive-only proposal does not describe the current schema-2 contract.
- Tactical generation rules and the 24-tile vocabulary are unchanged by area water/detail export.

## Outcomes & side effects

- Success returns the written path; the CLI prints that path and exits successfully. No artifact-size or render-time promise is part of this surface.
- Failure leaves saved layers unchanged and may leave the output directory. Quality-path failures preserve the previous completed PNG; ordinary failures attempt temporary-file cleanup. Legacy buffered writes can leave a partial destination. This remains separate from generation's manifest-last publication contract.
- Source authorities: `crates/arda-cli/src/main.rs`, `crates/arda/src/{lib,world,export_quality}.rs`, `crates/arda-render/src/{quality,carto,channels,channel_geometry,json,hydrology_json,symbolic,overview}.rs`, and `crates/arda-render/src/overview/streaming.rs`.

## Dimensions not in play

User tileset manifests, sprites/artwork, glyphs, collision/movement attributes, complete per-cell VTT payloads, settlements, roads, road crossings, passes, buildings, population, NPCs and named continent/society exports remain deferred. Saved shared water crossings describe river boundary flux, not road or settlement crossings.

The 2026-08-25 build-gate amendments proposed enriched tile legends (material, traversability, movement cost, cover and hazards), POIs/buildings, NPC sheets, realms and an `arda serve` consumer sharing the serializers/renderers. Those remain deferred designs, as do the original separate `cells.json`/`objects.json` exports and schema-1 additive-only proposal. [Serve](../mockup/06-serve.md) remains a future surface.
