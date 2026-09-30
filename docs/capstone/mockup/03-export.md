---
generated_date: 2026-09-22
scenarios: [export-vtt, inspect-volume]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# 03 — `arda export` (CLI): images + JSON on demand

Current implemented CLI surface as of 2026-09-22. Export reads a saved format-4 world and creates one PNG or JSON artifact per call. Repeating the request over unchanged saved data, style and quality produces identical bytes. Area and overview PNGs default to 8K and accept `--quality` up to 32K. PNGs default to Classic; explicit `--style atlas` adds natural relief and color over the same 100 m terrain. Quality and style add no tactical geometry. The corresponding loading, format and write rules are in [export logic](../logic/04-export.md).

## Layout

```text
$ arda export --world ./worlds/micro-42 --area 1,1 --format png --out ./maps
wrote ./maps/area_01_01.png

$ arda export --world ./worlds/micro-42 --area 1,1 --quality 32K --out ./maps
wrote ./maps/area_01_01.png

$ arda export --world ./worlds/micro-42 --area 1,1 --style atlas --out ./atlas-maps
wrote ./atlas-maps/area_01_01.png

$ arda export --world ./worlds/micro-42 --area 1,1 --style atlas --detail --out ./atlas-maps
wrote ./atlas-maps/area_01_01.png

$ arda export --world ./worlds/micro-42 --area 1,1 --detail --out ./maps
wrote ./maps/area_01_01_detail.png

$ arda export --world ./worlds/micro-42 --area 1,1 --format json --out ./maps
wrote ./maps/area_01_01.json

$ arda export --world ./worlds/micro-42 --overview --out ./maps
wrote ./maps/overview.png

$ arda export --world ./worlds/micro-42 --overview --quality 16k --out ./maps
wrote ./maps/overview.png

$ arda export --world ./worlds/micro-42 --overview --quality 16k --style atlas --out ./atlas-maps
wrote ./atlas-maps/overview.png

$ arda export --world ./worlds/micro-42 --block 1,1,64,128 --format png --out ./maps
wrote ./maps/block_01_01_064_128.png

$ arda export --world ./worlds/micro-42 --block 1,1,64,128 --format json --out ./maps
wrote ./maps/block_01_01_064_128.json
```

The examples assume a generated MICRO seed-42 world at `./worlds/micro-42`. Cell `(64,128)` in area `(1,1)` is sampled land in the inspected 2026-09-08 candidate, so it has a stored block. Other worlds may lack a block at that coordinate; in-range coordinates alone do not establish materialization.

Element tree: command → required saved-world/output paths → target (area / overview / block) → one format and optional PNG quality, style or area Detail → written-path line.

## Elements

| Element | Exact form | Does | Status / notes |
| --- | --- | --- | --- |
| `export` | `arda export` | Reads saved data and exports one artifact | Implemented; performs no generation |
| World | `--world <dir>`; required | Loads the completion manifest, then requested layers | Exact format-major 4 reader |
| Output | `--out <dir>`; required | Creates the directory and writes the artifact | Quality PNGs replace the destination after a completed temporary-file write; legacy Detail/block/JSON retain buffered writes |
| Area | `--area <ax,ay>`; default `0,0` | Selects a 51.2 km, 512×512-cell tile | Comma-separated indices; ignored by overview/block branches |
| Format | `--format png` or `--format json`; default `png` | Selects one representation for an area or block | Separate calls for both; overview always writes PNG |
| Quality | `--quality <pixels>` or `--quality <integer>k/K`; default `8k` | Sets area side or overview long edge to any integer 512–32768 pixels | 1K = 1024; incompatible with JSON, blocks and Detail; filenames are unchanged |
| Style | `--style classic|atlas`; omitted = Classic | Selects existing Classic or shaded Atlas PNG presentation | Area/overview PNG only; explicit style with JSON or block fails before loading the world (`crates/arda-cli/src/main.rs:120`) |
| Detail | `--detail` | Produces a 4096×4096 area PNG | Classic writes `_detail` filename; Atlas writes standard `area_XX_YY.png` through quality export. Incompatible with JSON, overview, blocks and quality (`crates/arda-cli/src/main.rs:290`) |
| Overview | `--overview` | Writes `overview.png` with an 8192-pixel long edge by default | Uses quality and the saved area-grid aspect ratio; short edge rounds to the nearest pixel; no continent JSON branch |
| Block | `--block <ax,ay,cx,cy>` | Exports a saved 64×64 tactical block | Takes precedence over overview when both are supplied; only sampled land cells have blocks |
| Result | `wrote <path>` | Identifies the one written artifact | No artifact-size or duration footer |

Area PNGs are square: the default is 8192×8192 and `--quality 32K` produces 32768×32768. Classic keeps its existing elevation colours and water presentation. Atlas adds earthy elevation color, saved sea-depth color and deterministic directional relief, interpolated at output pixels from saved cells and their neighbors (`crates/arda-render/src/atlas.rs:171`). It does not add finer terrain, ecology, roads or settlements. Both styles preserve saved standing-water extent and connected channels at physical width. The 512-pixel quality mode retains the legacy Preview faint mark for streams below one pixel; larger qualities and Detail show physical coverage alone. Area lakes use validated blue depth shading from saved surfaces, with deeper water darker (`crates/arda-render/src/channels.rs:278`). The overview retains categorical lake fill and discharge-band rivers rather than lake-depth shading or physical channel widths (`crates/arda-render/src/overview.rs:239`). Block PNGs retain the built-in symbolic renderer: eight pixels per square and the existing 24-tile vocabulary, with no change to tactical rules.

Area JSON is one combined schema-2 document: cell fields, local rivers/lakes, physical channel edges and copied model-2 annual hydrology. It is not separate `cells.json` and `objects.json`. Water identities and whole annual litres are decimal strings; mean flow fields retain L/s units. The cell list includes height, terrain, cover, slope, aspect, drainage, discharge, channel order/width, height above river and wetness; it omits temperature, rainfall, moisture, forest density, road and `built_by`. Block JSON contains square tile IDs, the relaxed-fill flag and an ID/name legend. Neither is a complete VTT collision, movement or society payload.

## States

- **Success**: one written-path line; repeating the same request overwrites identical bytes without rewriting saved world layers.
- **Missing/incompatible world**: missing or unreadable manifest, invalid configuration/dimensions, or any format major other than 4 causes failure. Preserved format-3 worlds are refused without migration.
- **Reading/rendering**: the command runs synchronously. Area reads are owned and released; overview reads one area at a time; the first block query decompresses its area's whole archive. This surface promises no fixed time, output size or browser parse cost.
- **Invalid mode**: unknown styles, out-of-range quality values, explicit style with JSON/blocks, explicit quality with JSON/blocks/Detail, or Detail with JSON/overview/blocks are rejected before world loading/output creation. Ordinary JSON exports without quality are unchanged; overview still ignores format when quality is omitted and no style is explicit. Malformed selectors, out-of-range coordinates and absent materialized blocks fail with context.
- **Data/render error**: unreadable, corrupt or truncated requested layers, invalid saved water geometry, raster resource limits and unknown symbolic tile IDs produce failure; the manifest alone does not prevalidate those layers. Atlas also requires every in-bounds neighbor needed for relief; missing or corrupt neighbors and invalid halo topology fail the export (`crates/arda/src/atlas.rs:35`).
- **Write error**: default and explicit-quality area/overview PNGs, including Atlas Detail, stream to a temporary sibling and rename it only after successful encoding/flushing. An ordinary failure preserves any previous completed PNG and attempts temporary-file cleanup. Abrupt termination can leave the temporary file; there is no power-loss durability guarantee. Classic Detail/block/JSON still write completed buffers, so I/O failure can leave a partial destination. The CLI may already have created the output directory.
- **Deferred design**: custom tilesets/artwork, glyphs, collision/movement attributes, society/settlement/road/NPC output and `arda serve` remain future surfaces. The 2026-09-07 mockup's `--cell`, `--format png,json`, implicit overview, default output directory, separate JSON files, invented sizes/timings and blanket atomic-write assumption were proposals, superseded by this implemented layout and its specific quality-PNG publication path. The C06 five-world data/export checks and separately approved golden pass; remaining visual realism findings stay open. This surface update is not an overall feature completion claim.
