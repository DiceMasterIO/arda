---
generated_date: 2026-09-08
scenarios: [export-vtt, inspect-volume]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# 03 — `arda export` (CLI): images + JSON on demand

Current implemented CLI surface as of 2026-09-08. Export reads a saved format-4 world and creates one PNG or JSON artifact per call. Repeating the request over unchanged saved data produces identical bytes. Area Detail adds display resolution over the existing 100 m terrain; it adds no tactical geometry. The corresponding loading, format and write rules are in [export logic](../logic/04-export.md).

## Layout

```text
$ arda export --world ./worlds/micro-42 --area 1,1 --format png --out ./maps
wrote ./maps/area_01_01.png

$ arda export --world ./worlds/micro-42 --area 1,1 --detail --out ./maps
wrote ./maps/area_01_01_detail.png

$ arda export --world ./worlds/micro-42 --area 1,1 --format json --out ./maps
wrote ./maps/area_01_01.json

$ arda export --world ./worlds/micro-42 --overview --out ./maps
wrote ./maps/overview.png

$ arda export --world ./worlds/micro-42 --block 1,1,64,128 --format png --out ./maps
wrote ./maps/block_01_01_064_128.png

$ arda export --world ./worlds/micro-42 --block 1,1,64,128 --format json --out ./maps
wrote ./maps/block_01_01_064_128.json
```

The examples assume a generated MICRO seed-42 world at `./worlds/micro-42`. Cell `(64,128)` in area `(1,1)` is sampled land in the inspected 2026-09-08 candidate, so it has a stored block. Other worlds may lack a block at that coordinate; in-range coordinates alone do not establish materialization.

Element tree: command → required saved-world/output paths → target (area / overview / block) → one format and optional area Detail → written-path line.

## Elements

| Element | Exact form | Does | Status / notes |
| --- | --- | --- | --- |
| `export` | `arda export` | Reads saved data and exports one artifact | Implemented; performs no generation |
| World | `--world <dir>`; required | Loads the completion manifest, then requested layers | Exact format-major 4 reader |
| Output | `--out <dir>`; required | Creates the directory and writes the artifact | Existing artifact overwritten; no atomic-write guarantee |
| Area | `--area <ax,ay>`; default `0,0` | Selects a 51.2 km, 512×512-cell tile | Comma-separated indices; ignored by overview/block branches |
| Format | `--format png` or `--format json`; default `png` | Selects one representation for an area or block | Separate calls for both; overview always writes PNG |
| Detail | `--detail` | Produces a native 4096×4096 area PNG | Preview without this flag is 512×512; incompatible with JSON, overview and blocks |
| Overview | `--overview` | Writes `overview.png` at 48 pixels per exported area | Explicit selector; no continent JSON branch |
| Block | `--block <ax,ay,cx,cy>` | Exports a saved 64×64 tactical block | Takes precedence over overview when both are supplied; only sampled land cells have blocks |
| Result | `wrote <path>` | Identifies the one written artifact | No artifact-size or duration footer |

Area PNGs show elevation colors, saved standing-water extent and connected channels at physical width. Preview adds a faint readability mark for streams below one pixel; Detail shows physical coverage alone. Area lakes use blue depth shading from saved surfaces, with deeper water darker. The overview retains categorical water styling. Block PNGs retain the built-in symbolic renderer: eight pixels per square and the existing 24-tile vocabulary, with no change to tactical rules.

Area JSON is one combined schema-2 document: cell fields, local rivers/lakes, physical channel edges and copied model-2 annual hydrology. It is not separate `cells.json` and `objects.json`. Water identities and whole annual litres are decimal strings; mean flow fields retain L/s units. The cell list includes height, terrain, cover, slope, aspect, drainage, discharge, channel order/width, height above river and wetness; it omits temperature, rainfall, moisture, forest density, road and `built_by`. Block JSON contains square tile IDs, the relaxed-fill flag and an ID/name legend. Neither is a complete VTT collision, movement or society payload.

## States

- **Success**: one written-path line; repeating the same request overwrites identical bytes without rewriting saved world layers.
- **Missing/incompatible world**: missing or unreadable manifest, invalid configuration/dimensions, or any format major other than 4 causes failure. Preserved format-3 worlds are refused without migration.
- **Reading/rendering**: the command runs synchronously. Area reads are owned and released; overview reads one area at a time; the first block query decompresses its area's whole archive. This surface promises no fixed time, output size or browser parse cost.
- **Invalid mode**: Detail with JSON, overview or a block is rejected before world loading/output creation. Malformed selectors, out-of-range coordinates and absent materialized blocks fail with context.
- **Data/render error**: unreadable, corrupt or truncated requested layers, invalid saved water geometry, raster resource limits and unknown symbolic tile IDs produce failure; the manifest alone does not prevalidate those layers.
- **Write error**: rendering/serialization completes before file writing, but I/O failure can leave a partial artifact. The CLI may already have created the output directory.
- **Deferred design**: custom tilesets/artwork, glyphs, collision/movement attributes, society/settlement/road/NPC output and `arda serve` remain future surfaces. The 2026-09-07 mockup's `--cell`, `--format png,json`, implicit overview, default output directory, separate JSON files, invented sizes/timings and atomic-write assumption were proposals, superseded by this implemented layout. The C06 five-world data/export checks and separately approved golden pass; remaining visual realism findings stay open. This surface update is not an overall feature completion claim.
