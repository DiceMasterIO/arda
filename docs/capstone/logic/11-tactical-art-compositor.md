---
generated_date: 2026-09-30
scenario: tactical-art-compositor
status: normative design; catalogue, validator, placeholders and compositor implemented on feat/tactical-catalogue (crate arda-tactical); library growth and dressing on feat/tactical-library
goals: 49, 58, 59, 60, 61, 62, 63, 64
---

# 11 — Tactical art: catalogue, validator, compositor and lighting

> Normative design for drawing a `TacticalLayout` as a painted top-down battle map in the style of `assets/reference/tactical-target/`. The art library itself is produced outside Arda; this scenario covers the code that consumes it. The byte-level catalogue format is documented in `crates/arda-tactical/README.md` on feat/tactical-catalogue, which is the data contract with the art library; this file states the rules that format must satisfy and the rules the compositor follows. Layouts come from [09](09-tactical-refinement.md) and [10](10-town-layout.md); the [scene data](12-scene-data.md) is derived from the same layout; the [service](16-service-api.md) caches and tiles the images.

Code cites rules as `// logic/11 §<rule>`.

## Trigger & preconditions

- Trigger: `render(layout, library, seed, RenderOptions)`; `arda tactical validate <dir>`; `arda tactical placeholders --out <dir>`; the service's tactical image routes.
- Preconditions: a library directory with `catalog.json` that passes validation (§validator); a layout that passes `TacticalLayout::check` against that library.

## Rules

### §catalogue

A library is a directory with `catalog.json` plus PNG images named by each asset's `image` path (relative, inside the directory). The catalogue carries `format_version` (1), `library`, `library_version` (part of every render's identity), `pixels_per_square` (source art resolution) and a controlled `vocabulary` for `biome`, `culture`, `wealth` and `function` tags. Every asset record carries (goal 58): `id`; `class` (`ground`, `wall`, `prop`, `vegetation`, `water`, goal 59); `footprint` in squares; `anchor`; allowed `rotations` (subset of 0, 90, 180, 270) and `mirror`; `tags` (the four controlled lists plus `free`); `placement` rules (`on_water`, `against_wall`, `near_road`, `clearance_squares`, allowed `ground` keys); `blocks_sight`, `blocks_movement`, `difficult_terrain`, `cover` (`none`, `half`, `three_quarters`, `full`); `light` (`radius_ft`, `colour`); render `layer` and `z`; `casts_shadow` and `height_ft`; `tileable`; `ground` key for textures; `wall` kit and role for wall pieces; `provenance` and `licence` (both required, non-empty). Unknown fields are rejected everywhere. The art can be swapped by pointing at another directory; no rebuild (goal 60).

### §vocabulary

The canonical tactical keys (copied from `docs/goal-prompts/vocabulary.md`, which is untracked; this list is normative once merged, and keys are only ever added, never renamed):

- **Ground keys** (`Square.ground`): `grass`, `dirt`, `cobbles`, `mud`, `sand`, `gravel`, `stone_floor`, `planks`, `water_shallow`, `water_deep`, `meadow`, `forest_floor`, `leaf_litter`, `heath`, `scrub`, `moss`, `scree`, `rock`, `cliff`, `snow`, `ice`, `marsh`, `reed_bed`, `farmland`, `pasture`, `packed_earth`, `flagstone`, `rug`.
- **Wall kits** (`WallSegment.kit`): `stone`, `timber`, `wattle`, `palisade`, `hedge`, `drystone`, `city_wall`.
- **Building functions** (`function:` tags): see [10](10-town-layout.md) §town-function-keys, including the three additions `mine`, `lumber_camp`, `school`.
- **Prop ids** (`prop.<name>`): barrel, crate, sacks, chest, table, bench, bed, cart, rowboat, market_stall, tent, fence, dock_planks, bridge_deck, crane, well, brazier, woodpile, chair, stool, cupboard, shelf, bookshelf, hearth, oven, bar_counter, cask_rack, anvil, forge, workbench, loom, grindstone, weapon_rack, armour_stand, altar, pew, candle_stand, statue, hay_bale, trough, millstone, bucket, wheelbarrow, ladder, throne, banner, rug_small, lantern, signpost, grave, haycart.
- **Vegetation ids** (`veg.<name>`): tree_oak, tree_elm, tree_birch, tree_fruit, bush, bush_flowering, reeds, boulder, stones, tree_pine, tree_spruce, tree_willow, tree_dead, fallen_log, stump, fern, mushroom_ring, heather, flower_patch, tall_grass, cattail, lily_pads, rock_small, rock_large, scree_patch.

### §tag-query

`AssetRef::Query { class, tags }` matches assets of the class carrying every listed tag. A **namespaced** tag `ns:value` with `ns` in {`biome`, `culture`, `wealth`, `function`, `free`} matches only that list; a bare tag matches any list. Any other `ns:` prefix is a bare tag containing a colon and matches only a `free` tag spelled identically. (The namespaced form is what `vocabulary.md` prescribes; `Library::query` on feat/tactical-catalogue matches bare values only, and feat/tactical-library adds the namespace parse. Tags such as `tree:broadleaf:large` from the terrain goal text are therefore not a query form; terrain uses `veg.*` ids, 09 §scatter.)

### §validator

`arda tactical validate` reports every issue as `<asset>: [<rule>] <detail>` and fails when any exists (goal 60). Rules and thresholds (all from feat/tactical-catalogue):

| Rule | Fails when |
|---|---|
| `format_version` | ≠ 1, or `pixels_per_square` = 0 |
| `duplicate_id` | an id repeats |
| `unknown_class` | not one of the five classes |
| `unknown_tag` | a controlled tag is not in `vocabulary` |
| `licence`, `provenance` | empty |
| `rotation` | empty, duplicated, or not in {0, 90, 180, 270} |
| `class_fields` | a texture without `ground`, a wall without `wall`, a non-wall with `wall` |
| `wall_kit` | a kit lacks `run`, `corner`, `tee`, `cross` or `end` |
| `image_missing` | unreadable, or the path leaves the library |
| `image_size` | not `footprint × pixels_per_square` |
| `alpha_opaque` | a cut-out with no transparent pixel |
| `alpha_fringe` | > 0.5 % of pixels have alpha 1–239 and lie > 2 px from an opaque pixel |
| `texture_alpha` | a texture with any alpha < 255 |
| `tile_seam` | for tileable textures, mean RGB difference across the wrap seams > 2 × the mean between interior neighbours |

Added by this spec (owner feat/tactical-library):

| Rule | Fails when |
|---|---|
| `vocabulary_coverage` | (with `--require-vocabulary`) a canonical ground key, wall kit, `prop.*` or `veg.*` id in §vocabulary has no asset. The production library must pass it; the placeholder library must pass it for the vertical-slice subset listed in §placeholders |
| `commercial_licence` | `licence` is not in the allow-list (`Apache-2.0`, `MIT`, `CC0-1.0`, `CC-BY-4.0`, `Apache-2.0 (generated placeholder)`, or a maintainer-owned licence string beginning `Proprietary (maintainer)`) — the game is commercial (goal-prompt §8) |

### §placeholders

The placeholder library is generated by code from a seed (default 58), marked `licence: "Apache-2.0 (generated placeholder)"`, committed under `assets/tactical/placeholder/`, and passes the validator (goal 61). It must cover the riverside-village vertical slice: every ground key used by 09 and 10 for a temperate lowland village, the `stone`, `timber`, `wattle`, `hedge` and `drystone` kits, the props of the inn, smithy, temple, house, warehouse, dock and market dressing sets (§dress), and the temperate `veg.*` ids. No baked cast shadows. Only feat/tactical-library regenerates it; other branches never commit placeholder images.

### §compose-order

Draw order is ground → water → floor → prop → wall → canopy, then lighting, then the optional grid (goal 62). Within a layer, ascending `z`, then placement order. Rotation is clockwise, mirror is applied before rotation.

### §ground-blend

Each texture key has one or more variants; a square's variant is chosen by the canonical hash of its **global** square coordinate and the render seed, and neighbouring variants cross-fade. Borders between ground keys are soft and noise-shaped: each pixel's position is domain-warped by value noise sampled through integer rotation matrices (never axis-aligned), the warped position interpolates the per-square ground weights, per-key noise roughens them, and a sharpening curve sets the border width. Borders never cross a wall. Water uses the same warp at lower amplitude and a 1.5 px threshold, so its edge stays clean; depth (capped at 8 ft) blends `water_shallow` to `water_deep` (about 2 ft reads shallow, ≥ 7 ft deep; values from feat/tactical-catalogue).

### §walls-assembly

Walls sit on square edges. Edge pieces (`run`, `door`, `window`, `gate`) span one edge; joint pieces sit on vertices and are chosen from the vertex's arm mask: `corner` (E, S), `tee` (E, S, W), `cross` (all), `end` (E), optional `post` (E, W), rotated clockwise until the canonical arms match. Where two segments share an edge, the later one wins (a door punched into a run). Multiple assets per role are chosen per edge or vertex by hash.

### §seam-art

Neighbouring blocks must join invisibly (goals 46, 49, 67):

1. Every hash and noise input is a **global** coordinate: the layout origin `origin_gs` (09 §square-frame) is added to local square and pixel coordinates before hashing. Pixel coordinates are `gs · ppsq + pixel`.
2. A block is rendered from a layout that includes an **apron** of 2 squares on every side taken from the neighbouring blocks (a window, 09 Branches), then cropped to the block. Soft blends, shadows and canopies that cross the edge are therefore drawn identically on both sides.
3. The render seed of a world block is `H(world_seed, "render", 0, 0)` (one per world, 09 §hash), never a per-block value, and never the constant used for built-in test layouts.

This requires `TacticalLayout` to gain an optional `origin: {gsx, gsy}` field (default `0, 0`, so existing layouts and test fixtures keep their bytes); that schema addition is owned by feat/tactical-library. Until then the service passes the origin beside the layout (16-service-api).

### §lighting

One light direction for the whole product: from the top left (north-west, azimuth 315°), matching the world-map relief light (logic/04 §atlas-formed light; goal 22). Drop shadows are cast from a height buffer (elevation plus `height_ft` of shadow-casting assets) with length about 0.055 squares per foot of height and softened edges; ambient occlusion darkens wall bases and the ground under props; water darkens along banks; `light` assets and free layout lights draw warm pools (goal 63). The art never carries painted shadows; the validator cannot detect them, so the library review checklist does.

### §dress

Dressing places props by building function, room and context (goal 64). It is run by the owner of the squares (town for buildings and streets, ways for roads and bridges, refine for natural ground), from data sets, not code:

- A dressing set is `{context, required: [asset ref + count range], optional: [asset ref + weight], density per 10 squares}`, keyed by `function:<key>` and room tag (10 §town-interiors). Examples (assumed, tunable): inn common room → `prop.table` 1 per 12 squares with 2–4 `prop.chair` or `prop.bench` each, `prop.bar_counter` 1, `prop.hearth` 1, `prop.cask_rack` 0–2; warehouse → `prop.crate`, `prop.sacks`, `prop.barrel` at 1 per 3 floor squares against walls; smithy forge → `prop.anvil`, `prop.forge`, `prop.grindstone`, `prop.weapon_rack`; temple nave → `prop.pew` rows, `prop.altar` 1, `prop.candle_stand` 2–4; dock → `prop.rowboat` on adjacent water squares, `prop.crane` 0–1; river banks → `veg.reeds`, `veg.cattail` on `reed_bed` and bank squares.
- Placement honours each asset's `placement` rules (`on_water`, `against_wall`, `near_road`, `clearance_squares`, allowed ground) and never blocks the path from a door to the room's other doors (the doorway squares and one square beyond are kept clear).
- **Anti-repetition:** within any 5 × 5-square window, no two placements share `(asset id, rotation, mirror)` unless the set requires more of that asset than there are alternatives; the choice among alternatives is by hash (§ground-blend). Tested statistically.
- Biome: the catalogue `biome:` tag is chosen from the settlement or cell biome (08 settlement `biome`, or the cell's cover and temperature for natural ground) by the table `alpine, highland → biome:highland`; `wetland → biome:wetland`; `steppe → biome:steppe`; `boreal_forest → biome:boreal`; `coastal → biome:coastal`; everything else `→ biome:temperate` (tag values to be added to the catalogue vocabulary). Missing biome art falls back to `biome:temperate`.

### §ppsq

Output pixels per square is a render option: 128 by default, and the service offers 64, 96 and 128 (the goal-prompt range "100–140 typical" is met by the default; the open question stays open for the real library). Art is rescaled from the library's `pixels_per_square` with premultiplied area averaging when shrinking and bilinear interpolation when growing.

### §render-identity

A render is identified by `(layout JSON bytes, origin, library, library_version, render seed, ppsq, grid, lighting)`. Output is byte-identical for identical identity (goal 62): single-threaded, integer maths plus IEEE `f32` `+ − × ÷ sqrt floor` only (no transcendental functions, no fused multiply-add assumptions).

## Steps

1. Load and validate the library (once per process; the service holds it).
2. Check the layout against the library.
3. Resolve every placement (`Id` or `Query`, by hash of seed and placement index).
4. Paint ground and water (§ground-blend), floors, props, walls (§walls-assembly) and canopies in order.
5. Lighting (§lighting); optional grid.
6. Return RGBA; the service encodes PNG or tiled WebP (16-service-api).

## Branches

- `lighting: false` skips step 5 (debug).
- A query that matches no asset is a check failure, not a silent skip.
- Missing water keys when a square has depth: check failure.

## Unhappy paths

- Invalid library: the validator's report; the service refuses to start with the report in its log.
- Layout inconsistent with the library: `TacticalError::Layout` naming the layout and the first inconsistency (the service answers 422).
- Image allocation beyond the service's pixel budget: refused before rendering (413 at the service).

## State transitions

None for rendering. `arda tactical placeholders` writes the placeholder directory (replacing it only with `--out` pointing at it).

## Invariants

1. Determinism: rendering twice gives the same hash; the same identity on two machines gives the same bytes [62].
2. Blends are not grid-aligned: the orientation histogram of the ground-border mask shows no bias toward 0° or 90° (statistical test on a two-key checkerboard layout) [62; goal-prompt §7].
3. Wall-kit selection: every arm mask of the 16 maps to the right role and rotation, or to nothing for an isolated vertex [62].
4. Placeholders pass the validator, including `vocabulary_coverage` for the vertical-slice subset [61].
5. Seams: a 2 × 1 block window rendered whole equals, pixel for pixel, the two blocks rendered separately with aprons and cropped [46, 49].
6. Every validator rule has one failing fixture that trips exactly that rule [60].
7. Anti-repetition: in dressed layouts, the 5 × 5 repetition rule holds [64].
8. The same layout rendered with a different `library_version` gives different bytes only through art and seed, never through geometry: wall and placement positions are identical [58, 62].

## Outcomes & side effects

An RGBA image of `width·ppsq × height·ppsq` pixels. `arda tactical render` writes PNGs; the service writes nothing to disk.

## Dimensions not in play

- Animation (water, fire) and day/night relighting.
- Roofs over interiors (10 Dimensions not in play).
- AI image generation at map-generation time: never (goal-prompt §6 decision).
