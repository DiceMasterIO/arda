---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: d0bdce330c8c
paths_covered:
  - ":(top)crates/arda-settle/src/main.rs"
  - ":(top)crates/arda-settle/src/render/**"
  - ":(top)crates/arda-settle/src/output.rs"
  - ":(top)crates/arda-settle/src/error.rs"
  - ":(top)crates/arda-settle/tests/render.rs"
  - ":(top)crates/arda-render/src/quality.rs"
  - ":(top)crates/arda-town/src/main.rs"
  - ":(top)crates/arda-town/src/samples.rs"
  - ":(top)crates/arda-town/src/error.rs"
  - ":(top)crates/arda-town/src/render.rs"
  - ":(top)crates/arda-town/src/block/mod.rs"
  - ":(top)crates/arda-town/src/block/fallback.rs"
  - ":(top)crates/arda-town/src/plan/check.rs"
  - ":(top)crates/arda-town/src/plan/spec.rs"
  - ":(top)crates/arda-town/src/plan/types.rs"
  - ":(top)crates/arda-town/tests/plan.rs"
  - ":(top)crates/arda-town/tests/blocks.rs"
  - ":(top)crates/arda-tactical/src/compose/mod.rs"
  - ":(top)crates/arda-tactical/src/raster.rs"
  - ":(top)crates/arda-tactical/src/library_stack.rs"
  - ":(top)crates/arda-refine/src/bin/arda-refine.rs"
  - ":(top)crates/arda-refine/src/world_source.rs"
  - ":(top)crates/arda-refine/src/source.rs"
  - ":(top)crates/arda-server/src/tactical/world_routes.rs"
  - ":(top)crates/arda-server/src/tactical/library_dto.rs"
  - ":(top)crates/arda-server/src/tactical/routes.rs"
  - ":(top)crates/arda-server/tests/world_api/cells.rs"
  - ":(top)crates/arda-refine/src/coastwet.rs"
  - ":(top)crates/arda-refine/tests/coastwet*.rs"
  - ":(top)crates/arda-tactical/src/compose/light_los.rs"
---

# 22 — Developer inspection: debug renders, statistics and summaries

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. It owns the developer and debug entry points whose generation rules belong to other scenarios: `arda-settle render`, `arda-town render|block|info|export`, `arda-refine stats`, `arda-refine survey` and `arda-refine coastwet`, and `GET /v1/tactical/library`. Settlement generation is [08](08-settlements-roads-realms.md), tactical refinement and `arda-refine render` are [09](09-tactical-refinement.md), town planning and blocks are [10](10-town-layout.md), library validation and composition are [11](11-tactical-art-compositor.md), the service is [16](16-service-api.md).

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/22 §<rule>`.

## Trigger & preconditions

| Entry point | Defined at | Arguments (default) | Reads | Writes |
|---|---|---|---|---|
| `arda-settle render --world <dir> --out <png> [--quality N] [--full-frame]` | `crates/arda-settle/src/main.rs:26-39`, handler `crates/arda-settle/src/render/mod.rs:147` | `quality` 2048 (`main.rs:34`), `full_frame` false (`main.rs:37-38`) | world via `arda::World::load` (`render/mod.rs:148`); `society/landuse.bin`, `realms.bin`, `settlements.json`, `roads.json`, `realms.json`, `names.json`, and `gazetteer.json` when the file exists (`render/mod.rs:114-138`, gazetteer `:129-133`) | one RGB PNG at `--out`; a scratch directory under the system temp dir, removed (`render/mod.rs:150-154`) |
| `arda-town render --site <name> --out <png> [--seed N] [--px N]` | `crates/arda-town/src/main.rs:23-36`, handler `main.rs:163-184` | `seed` 42, `px` 0 = auto (`main.rs:31-35`) | built-in sample site (`main.rs:96-99`) | plan overview PNG |
| `arda-town block --site <name> [--window x,y,w,h] [--focus what] [--size N] --out <png> [--library spec] [--seed N] [--ppsq N] [--grid] [--json path]` | `main.rs:38-72`, handler `main.rs:185-235` | `size` 64, `library` `assets/tactical/placeholder`, `seed` 42, `ppsq` 48, `grid` false (`main.rs:50-68`) | sample site; tactical library or `top:…:bottom` stack (`main.rs:204`) | tactical block PNG; optional canonical block JSON (`main.rs:231-233`) |
| `arda-town info --site <name> [--seed N]` | `main.rs:74-81`, handler `main.rs:236-248` | `seed` 42 | sample site | stdout only |
| `arda-town export --site <name> --out <dir> [--seed N]` | `main.rs:83-93`, handler `main.rs:249-255` | `seed` 42 | sample site | `<dir>/<site>-plan.json`, `<dir>/<site>-buildings.json` (`main.rs:251-253`) |
| `arda-refine stats --world <dir> --cell gx,gy` | `crates/arda-refine/src/bin/arda-refine.rs:43-50`, handler `arda-refine.rs:143-195` | none | world via `arda::World::load` and `WorldSource::new` (`arda-refine.rs:144-145`) | stdout only |
| `arda-refine coastwet --world <dir> [--stride N]` | `crates/arda-refine/src/bin/arda-refine.rs:51-59`, handler `arda-refine.rs:196-201` | `stride` 16 (`arda-refine.rs:56-57`) | world via `arda::World::load` and `WorldSource::new` (`arda-refine.rs:197-198`) | stdout only (pretty JSON) |
| `arda-refine survey --world <dir>` | `arda-refine.rs:61-65`, handler `arda-refine.rs:202-206`, `arda_refine_survey` `arda-refine.rs:211-292` | none | world | stdout only |
| `GET /v1/tactical/library` | route `crates/arda-server/src/tactical/routes.rs:58`, handler `crates/arda-server/src/tactical/world_routes.rs:371-376` | none (no query parameters read) | the `Library` loaded at server start (`crates/arda-server/src/tactical/mod.rs:196`, accessor `mod.rs:255`) | JSON response; one stderr log line |

Not claimed here: `arda-settle generate` (08), `arda-refine render` (09 names it as its debug tool, `09-tactical-refinement.md:214`).

- D1 Authority: every CLI entry point runs with the invoking user's filesystem rights; no role or grant check exists in `crates/arda-settle/src/main.rs`, `crates/arda-town/src/main.rs` or `crates/arda-refine/src/bin/arda-refine.rs`. `GET /v1/tactical/library` has no authentication, as for every v1 route (16, `16-service-api.md:27`).
- D2 Preconditions:
  - `arda-settle render` requires a loadable world and an existing `society/` from `arda-settle generate` (08). Society presence is not checked before the overview is rendered: `Society::read` runs after the export (`render/mod.rs:152-156`).
  - `arda-town` requires no input files; sites are compiled in (`crates/arda-town/src/samples.rs:17-29`). `block` also requires a library that loads and validates (11 §validator) through `Library::load_stack` (`crates/arda-tactical/src/library_stack.rs:95-111`).
  - `arda-refine stats|survey|coastwet` require a loadable world. `society/` is not read: `WorldSource::wrap` reads the manifest, the optional fine terrain layer and the optional shore layer (`World::shore`, `terrain/shore.bin`) only (`crates/arda-refine/src/world_source.rs:84-95`).
  - `GET /v1/tactical/library` requires a running server, whose start already required a validated library (16 Trigger & preconditions).

## Rules

### §insp-settle-overlay

`arda-settle render` composes the 08 society files over the world's Atlas-style overview.

- Order: load world (`render/mod.rs:148`) → validate quality (`render/mod.rs:149`) → create scratch dir (`render/mod.rs:150`) → `arda::export_overview_with_quality_and_style(.., MapStyle::Atlas)` into it (`render/mod.rs:151-152`) → decode the PNG to RGB (`render/mod.rs:153`, `decode` `render/mod.rs:259-284`, grey and grey-alpha expanded to RGB at `render/mod.rs:274-277`) → remove the scratch dir, result ignored (`render/mod.rs:154`) → `Society::read` (`render/mod.rs:156`) → `draw` (`render/mod.rs:157`) → create `--out`'s parent (`render/mod.rs:158-160`) → `encode` as 8-bit RGB (`render/mod.rs:286-298`).
- `draw` runs `guard::check` first (`render/mod.rs:207`), then crops to the page unless `--full-frame` (`render/mod.rs:210-225`), then paints the layers in a fixed order: land use, realm tints, realm borders, roads (`layers::roads`, re-exported from `roadlayer.rs`, `render/mod.rs:241`), page furniture placement, settlements, features, labels, natural-feature labels from `gazetteer.json` (`text::features`, `render/mod.rs:253`; absent file: nothing painted), title furniture (`render/mod.rs:237-255`). `text::features` lays out every gazetteer item that is neither a river nor a settlement with `tilelabels::Labeler` around the boxes the earlier labels took, and paints halo then text (`crates/arda-settle/src/render/text.rs:185-224`).
- Scale: `s = (fw / 2048).clamp(0.4, 8.0)` where `fw` is the full overview width (`render/mod.rs:235`). Metres map to page pixels by `m · fw / (gw · 100) − ox` (`render/mod.rs:67-72`); a 100 m cell is the unit (`gw`, `gh` in cells).
- Page crop (`page`, `render/mod.rs:166-197`): the bounding box of cells with `realm_map != 0`, plus a margin of `60 · s` px, widened by `470 · s` px toward the side with more sea for the legend (`render/mod.rs:181-188`). No realm cell → the whole overview (`render/mod.rs:175-177`).
- Title: subtitle from realm names ("An unclaimed land", "The realm of X", "The realms of A, B and C"), a note with the seed, the size in km and settlement counts by tier (`crates/arda-settle/src/render/text.rs:227-247`).
- Shallow: the drawing internals of `layers.rs` (land-use tones in `land_tone`, `layers.rs:45`), `roadlayer.rs` (road styles and painting, split out of `layers.rs`), `labels.rs` (greedy label placer), `furniture.rs`, `font.rs`, `mask.rs` and `text.rs` beyond `title` and `features` are not inventoried. `band.rs` (`SocietyBandOverlay`, strip-wise painting for the exports) and `tilelabels.rs` (label layout shared with the tile pyramid, also used by `text::features`) serve the exports and the server; their rules are logic/17; their behavior is summarised only by the module doc `render/mod.rs:1-12`.

### §insp-settle-guard

Inputs `draw` would index or scale are refused before drawing (`crates/arda-settle/src/render/guard.rs:23-50`):

| Check | Error reason |
|---|---|
| `gw · gh` overflows `usize` | `grid size` (`guard.rs:25`) |
| grid empty, or `codes`/`realm_map` length ≠ `gw · gh` | `rasters are empty or of different sizes` (`guard.rs:26-28`) |
| any settlement position, road segment point, crossing or pass outside `[0, gw·100] × [0, gh·100]` metres | `a position lies off the world` (`guard.rs:29-48`) |

`Society::read` separately refuses a `realms.bin` whose size differs from `landuse.bin` (`render/mod.rs:118-123`). Rasters are bounded to `MAX_RASTER_CELLS = 2^28` cells and checked for magic, `FORMAT_VERSION` 1 and exact payload length (`crates/arda-settle/src/output.rs:186-225`). JSON files are deserialised by serde only (`output.rs:154-160`): their `format_version` field is read but not compared. No check compares the society grid size with the loaded world's dimensions (`render/mod.rs:147-162`).

The scratch directory is `arda-settle-render-<pid>-<k>` for `k` in `0..64`, created with `create_dir` so an existing path or planted symlink is skipped (`guard.rs:58-75`).

### §insp-settle-quality

`--quality` is a clap `u32` (`main.rs:34-35`), so a `K` suffix or a negative number fails argument parsing. The value is validated by `arda::ImageQuality::new`: accepted range `512..=32768` (`crates/arda-render/src/quality.rs:10-27`); outside it the error is mapped to `SettleError::Png` (`render/mod.rs:149`). The overview long edge equals the quality; the short edge is rounded to the nearest pixel (`quality.rs:41-56`, area grid limited to 1–78 areas per side at `quality.rs:46`).

### §insp-town-sites

`arda-town` plans only the four compiled-in sample sites: `aldermere` (Town, `samples.rs:153`), `thornby` (Village, `samples.rs:219`), `highcrag` (Town, `samples.rs:257`), `saltwick` (Hamlet, `samples.rs:299`); names in `samples::NAMES` (`samples.rs:17`). Lookup is an exact, case-sensitive match (`samples.rs:21-29`); any other name is `TownError::UnknownSite` (`main.rs:97`). Every subcommand plans through `plan_for` → `arda_town::generate(&site, &terrain, seed)` (`main.rs:96-99`); the planning rules are 10's.

### §insp-town-window

`arda-town block` picks the window in this precedence (`main.rs:198-202`):

1. `--window x,y,w,h` (plan-relative squares) wins whenever present, `--focus` is then ignored. Parsing splits on `,`, trims, and silently drops components that fail to parse as `i64` (`main.rs:115`); exactly four surviving numbers are required, else `TownError::Window("`…` is not x,y,w,h")` (`main.rs:116-118`). The plan origin is added to `x`, `y` (`main.rs:119-125`, origin `crates/arda-town/src/plan/types.rs:408-410`).
2. `--focus` centres a `size × size` window (`x − size/2`, integer division) on a point of interest (`main.rs:129-159`):

| `--focus` | Point |
|---|---|
| `market` | square of `plan.focal.market` (`main.rs:139`) |
| `river` | square of `plan.focal.feature` (`main.rs:140`) |
| `inn` | first `Inn`, else first `Tavern`, rect midpoint (`main.rs:141`) |
| `temple` | first `Temple`, else first `Shrine` (`main.rs:142-144`) |
| `smithy` | first `Smithy` (`main.rs:145`) |
| `gate` | first gate of the wall (`main.rs:146-150`) |
| `building:<n>` | building id `n` (`main.rs:151-155`; lookup by index `n − 1`, `types.rs:380-383`) |
| anything else, or no match | `TownError::Window("nothing to focus on for `…`")` (`main.rs:157-158`) |

3. Neither flag → `focus market` (`main.rs:201`).

`block::generate` refuses `w` or `h` outside `1..=MAX_WINDOW` (1024) (`crates/arda-town/src/block/mod.rs:39`, `block/mod.rs:198-203`); so `--size` ≤ 0 or > 1024 fails there. The window's position is not checked against the plan extent in `main.rs` or `generate_with` (`block/mod.rs:197-203`).

### §insp-town-block-render

After the window: `block::generate` (10) (`main.rs:203`) → `Library::load_stack(--library)` (`main.rs:204`) → `fallback::resolve(&blk, &lib, plan.seed)` replaces every ground key, wall kit, prop and tag query the library lacks by its nearest relative and records each substitution, sorted (`crates/arda-town/src/block/fallback.rs:1-9`, `fallback.rs:148-153`, `fallback.rs:178`) → `arda_tactical::render` with `lighting: true` and the plan seed (`main.rs:206-211`; composition is 11's). `render` refuses `ppsq` outside `8..=1024` (`crates/arda-tactical/src/compose/mod.rs:331-335`) and canvases over the memory ceiling (`compose/mod.rs:457-469`).

Printed (`main.rs:220-230`): `block X,Y WxH (plan-relative x,y): B buildings, W walls, P props, L lights`, counted on the canonical block before fallback; then one `fallback <kind> <wanted> -> <used> ×<count>` line per substitution. `--json` writes `blk.to_json()`, the canonical block and sidecar before fallback, not the resolved layout that was rendered (`main.rs:231-233`, `block/mod.rs:537-539`).

### §insp-town-overview

`arda-town render` prints `check::summary` (`main.rs:170`), then draws `render::plan_png(&plan, px)` (`main.rs:182`). `px = 0` selects `auto_px(&plan, 1600)`: `ceil(1600 / long edge of the built extent with a 48-square margin)`, clamped to `2..=12` (`crates/arda-town/src/render.rs:197-201`). An explicit `--px` has no upper bound; `plan_png` uses `px.max(1)` (`render.rs:208`) and allocates `w · h · 4` bytes through `Rgba::new` without a ceiling (`render.rs:210-211`, `crates/arda-tactical/src/raster.rs:48-53`). Shallow: the colouring inside `plan_png` (`render.rs:206` onward) is not inventoried.

### §insp-town-summary

`check::summary` (`crates/arda-town/src/plan/check.rs:130-144`) is one line: `<name>: S streets, P plots (median frontage F m), B buildings, G gates, O overlaps, U unreachable`. Median frontage is element `len/2` of `frontages` (0.0 when empty) (`check.rs:131-132`). `overlaps` (`check.rs:12`) and `unreachable` (`check.rs:69`) are counted and printed only; a non-zero count does not change the exit status.

`arda-town info` adds (`main.rs:238-247`): `grid origin X,Y size WxH squares` (`types.rs:408-416`), one `  <function key> <n>` line per function from `built_mix`, which counts non-ancillary buildings in `BuildingFunction` order (`check.rs:120-126`), and one `note: …` line per `plan.notes` entry.

### §insp-town-export

`arda-town export` writes `<out>/<site>-plan.json` from `TownPlan::to_json` (pretty JSON, grid excluded per its doc, `types.rs:385-391`), then `<out>/<site>-buildings.json` from `spec::specs` (`crates/arda-town/src/plan/spec.rs:77`; field mapping is 10 §town-building-spec). `<site>` in the file name is always one of the four sample names (§insp-town-sites). Writes create parent directories and use plain `std::fs::write` (`main.rs:101-112`).

### §insp-refine-stats

`arda-refine stats` (`arda-refine.rs:143-195`):

1. `--cell` parsed by `parse_pair(.., ',')`: split at the first separator, both halves trimmed and parsed as `i64`; failure is a `String` error (`arda-refine.rs:68-75`).
2. Times three refinements with `std::time::Instant`: `refine` cold (loads caches), `refine` warm, and `refine_window(gx·64+16, gy·64+16, 32, 32)` (a centred quarter block) (`arda-refine.rs:147-155`). Refinement itself is 09.
3. Prints, in order: the three timings in ms (`arda-refine.rs:156-158`); `cell gx,gy: relaxed R attempts A repairs N items I <warm ms>` (`arda-refine.rs:163-169`); a `BTreeMap` of ground key → square count (`arda-refine.rs:159-162,170`); a `BTreeMap` of `(asset, Debug(kind))` → item count (`arda-refine.rs:171-177`); an 11-bin histogram of shape `hollow` over `-1..1` by 0.2, bin `round((hollow + 1) · 5)` clamped to `0..=10`, and the count of squares with `talus > 0.3`, computed from a fresh `Ctx::gather` → `physical` over an 80-square window at `(gx·64 − 8, gy·64 − 8)` → `slopes` → `shapes` over the 64-square block (`arda-refine.rs:178-192`); the `Debug` form of the world cell (`arda-refine.rs:193`); `tiles <TILE_COUNT>` (`arda-refine.rs:194`).

### §insp-refine-coastwet

`arda-refine coastwet` prints `serde_json::to_string_pretty(&arda_refine::coastwet::measure(&src, stride)?)` (`arda-refine.rs:196-201`; `crates/arda-refine/src/coastwet.rs:282`). `stride` is clamped to at least 1 (`coastwet.rs:283`). The report is a `Report` (`coastwet.rs:90-106`): `seed`, `cells`, `has_shore_layer` (`Source::has_shore`), `stride`, `groups`, `coast_class_pct`, `strip`.

- Land sampling: one cell in every `stride` columns and rows, land cells only (`coastwet.rs:286-313`). Each cell counts into `all`, one climate band by mean annual temperature (`cold` < 5 °C, `cool` < 11 °C, `warm`) and `temperate_lowland` (5–18 °C and height under 300 m) (`coastwet.rs:28-34,189-201`). Per group: share with `marshy`, coast strip, `arid`, `alpine` above 0.5, share at or above each wetness level 0.5–0.9, the strongest prior ground, and the mean share of the prior's weight per ground (`Group`, `coastwet.rs:37-58`).
- Pools: on cells where `(x / stride + y / stride)` is even, the block's pool sites and `pools::pool` probes on an 8-square lattice (`POOL_STEP`) are tallied (`sample_block`, `coastwet.rs:203-242`); reported as pools per km² of wet cells and the share of probes inside a pool for wet cells and for cells that are not marsh (`PoolStats`, `coastwet.rs:60-74`).
- Coast cells: a sea-touching land cell is sampled when `hash2(seed, 0xC0A5, x, y)` is a multiple of `stride` (about one in `stride`), classed by the saved shore class (`none`, `sand_beach`, `shingle_beach`, `cliff`, `rocky_shore`, `salt_marsh`, `tidal_flat`, `estuary`) or `unclassified` when the world has no shore layer (`class_name`, `coastwet.rs:175-186`; loop `coastwet.rs:319-362`). `strip` gives per class the cell count, mean strip width in cells (walk inland up to `WALK_MAX` = 8 while the coast strip stays above 0.5 and the class stays the same, `strip_width`, `coastwet.rs:244-280`), the width histogram and the share whose strongest prior ground is sand.
- Determinism: a fixed stride over cell coordinates and a seed-keyed hash, no clock; one world and one stride give the same bytes (module doc `coastwet.rs:7-8`). Percentages are rounded to three decimals (`pct`, `coastwet.rs:126-132`). Asserted by `crates/arda-refine/tests/coastwet.rs` and `coastwet_world.rs` (not read in detail here).

### §insp-refine-survey

`arda-refine survey` (`arda-refine.rs:211-292`) lists candidate cells for nine terrain types: `river`, `forest_edge`, `lake_shore`, `sea_coast`, `scree`, `marsh`, `high_meadow`, `pasture`, `stream_valley` (`arda-refine.rs:214-224`).

- Scan: every second cell, `x` in `2..w−2`, `y` in `2..h−2` with step 2, row-major (`arda-refine.rs:226-227`), reading the cell and its 8 neighbours (`arda-refine.rs:229-237`). Slope is `slope_milli_deg / 1000` degrees; temperature `raw / 100` °C (`arda-refine.rs:239-240`).
- Predicates (`arda-refine.rs:252-269`), with `fd` = `forest_density`, `lake`/`sea` = neighbours of that terrain:

| Type | Predicate |
|---|---|
| `river` | land, `watercourse_width_dm ≥ 60`, slope < 6, cover ≠ Marsh, no lake or sea neighbour |
| `forest_edge` | land, slope < 12, and either (a neighbour with fd > 160 and a land neighbour with fd < 40 and own fd in `50..200`) or (cover Forest, fd > 120, 2–5 grass neighbours with fd < 60) |
| `lake_shore` | 1–7 lake neighbours, slope < 15 |
| `sea_coast` | 1–7 sea neighbours, slope < 15 |
| `scree` | land, cover Rock, slope in `28..42` |
| `marsh` | land, cover Marsh, ≥ 5 marsh neighbours |
| `high_meadow` | land, cover Grass, temperature < 5, fd < 60 |
| `pasture` | land, cover Grass, temperature > 8, slope < 4, fd < 40 |
| `stream_valley` | land, `watercourse_order` 1–2, slope > 6 |

- At most 40 candidates per type, first found in scan order (`arda-refine.rs:270-273`).
- Report: per type, each candidate is refined (09) and printed as `(x,y) wet W% trees T`, where `W = squares with depth_ft > 0 · 100 / 4096` (integer division) and `T` = items of kind `TreeLarge` (`arda-refine.rs:277-290`). One line per type, `name: ` followed by `; `-joined entries; a type with no candidate prints an empty list.

### §insp-library-summary

`GET /v1/tactical/library` returns `summarise(state.tactical.library())` as JSON (`world_routes.rs:371-376`; `crates/arda-server/src/tactical/library_dto.rs:81-146`). DTO `TacticalLibraryDto` (`library_dto.rs:11-25`):

| Field | Source |
|---|---|
| `library`, `library_version`, `pixels_per_square` | catalogue header (`library_dto.rs:125-127`) |
| `grounds[]` `{key, variants, water}` | one entry per distinct `asset.ground` key, `BTreeMap` order; `variants` = number of catalogue assets carrying that key; `water` = any of them has class `water` (`library_dto.rs:88-92,128-135`) |
| `wall_kits[]` `{kit, roles}` | one entry per `asset.wall.kit`, `BTreeMap` order; roles sorted and de-duplicated (`library_dto.rs:93-95,136-143`) |
| `assets[]` | assets whose class is `prop` or `vegetation` only (`library_dto.rs:96`); `tags` = `key:value` strings from `biome`, `culture`, `wealth`, `function`, `free`, sorted (`library_dto.rs:97-108`); sorted by `id` with a stable sort (`library_dto.rs:123`) |

Enum fields (`class`, `layer`, `cover`, wall `role`) are rendered through `word`, which serialises to JSON and takes the string, or `""` if the value does not serialise to a string (`library_dto.rs:72-77`). The DTO has TS and JSON-schema bindings (`crates/arda-server/src/bindings.rs:24`).

## Steps

`arda-settle render`: §insp-settle-quality → §insp-settle-overlay (load, export, decode) → §insp-settle-guard → draw → encode → print `wrote <out>` (`main.rs:64-66`).

`arda-town <cmd>`: parse → §insp-town-sites (plan) → per subcommand:
- `render`: print summary → create parent dir (`main.rs:171-176`) → §insp-town-overview → write PNG → print `wrote <out>` (`main.rs:183`).
- `block`: §insp-town-window → block → library → fallback → render → create parent dir (`main.rs:212-217`) → write PNG (`main.rs:218`) → print block line and fallbacks → optional JSON → print `wrote <out>` (`main.rs:234`).
- `info`: §insp-town-summary.
- `export`: §insp-town-export → print `wrote <dir>` (`main.rs:254`).

`arda-refine stats`: load world (`arda-refine.rs:144`) → `WorldSource::new` (`arda-refine.rs:145`) → parse cell (`arda-refine.rs:146`) → §insp-refine-stats.

`arda-refine survey`: load world → `WorldSource::new` → §insp-refine-survey (`arda-refine.rs:203-205`).

`arda-refine coastwet`: load world → `WorldSource::new` → `coastwet::measure` → print JSON (§insp-refine-coastwet; `arda-refine.rs:196-201`).

`GET /v1/tactical/library`: route match → `summarise` on the async handler (not through `blocking`) → `log("library", "-", Hit::Hit, 0, start)` writes `arda-server tactical library - cache=hit bytes=0 <ms> ms` to stderr (`world_routes.rs:374`, `crates/arda-server/src/tactical/routes.rs:137-145`) → `Json(body).into_response()` (`world_routes.rs:375`).

## Branches

- `arda-settle render --full-frame`: page = whole overview, no crop (`render/mod.rs:210-211`); otherwise §insp-settle-overlay page crop, which also degrades to the whole overview when no cell has a realm (`render/mod.rs:175-177`).
- `arda-town block` window source: `--window` > `--focus` > `market` (§insp-town-window).
- `arda-town block --library`: one directory → `Library::load`; a `:`-separated stack of several → each layer loaded with every rule except `wall_kit`, then the merged stack validated (`library_stack.rs:95-111,119-125`); an empty spec → `TacticalError::Options("empty library stack …")` (`library_stack.rs:98-101`).
- `arda-town block --grid`: draws the square grid through `RenderOptions.grid` (`main.rs:206-210`).
- `arda-town block --json`: writes the JSON after the PNG (`main.rs:218,231-233`).
- `arda-town render --px 0` vs explicit: §insp-town-overview.
- `arda-refine stats|survey`: out-of-world coordinates are not a branch; `Source::clamp` clamps cell reads to `[0, w−1] × [0, h−1]` (`crates/arda-refine/src/source.rs:140-143`, used by `split` `world_source.rs:112-126`), so an off-world `--cell` refines from edge-cell data without an error.
- `arda-refine survey` on a world with fewer than 5 cells on a side: the scan ranges are empty (`i64` arithmetic, `world_source.rs:87-90,186-188`) and every type prints an empty list.
- D6 limits at the boundary: `--quality 512` and `32768` accepted, 511 and 32769 refused (`quality.rs:22`); `arda-town block` window edge 1 and 1024 accepted, 0 and 1025 refused (`block/mod.rs:198`); `--ppsq 8` and `1024` accepted (`compose/mod.rs:331`); survey keeps the 40th candidate and drops the 41st (`arda-refine.rs:271`).

## Unhappy paths

| Entry point | Failure | Handling | Pointer |
|---|---|---|---|
| all three CLIs | unknown subcommand, missing required flag, non-integer value for an integer flag | clap usage error on stderr, exit 2 (clap_builder `USAGE_CODE`) | `main.rs:43` (settle), `main.rs:261` (town), `arda-refine.rs:95` |
| `arda-settle render` | world fails to load | `SettleError::Load`, exit 1 | `render/mod.rs:148`, `main.rs:68-74` |
| | quality out of range | `SettleError::Png`, exit 1 | `render/mod.rs:149` |
| | 64 scratch names all taken | `SettleError::Io` | `guard.rs:58-75` |
| | overview export fails | `SettleError::Export`; scratch dir still removed | `render/mod.rs:152-154` |
| | `society/` missing, truncated, wrong magic or version, oversized raster, malformed JSON | `SettleError::Io`/`Format`/`Json` naming the file, after the overview has been rendered | `render/mod.rs:156`, `output.rs:154-225` |
| | inconsistent or off-world society data | `SettleError::Format` with path `society overlay` | `guard.rs:10-15,23-50` |
| | embedded fonts fail to parse | `SettleError::Png` | `render/mod.rs:204-205,242` |
| | `--out` unwritable, PNG encode fails | `SettleError::Io`/`Png`; the file was created by `File::create` before encoding, so a partial PNG can remain; no cleanup | `render/mod.rs:287-297` |
| `arda-town` | unknown site | `TownError::UnknownSite`, exit 1 | `main.rs:97`, `error.rs:28-30` |
| | planner fails a hard rule or rejects the site | `TownError::Plan`/`Site` (10) | `error.rs:9-24` |
| | bad `--window`, `--focus` with no target, window edge out of range | `TownError::Window` | `main.rs:116-118,157-158`, `block/mod.rs:198-203` |
| | library fails to load or validate; `ppsq` out of range; canvas over ceiling; layout inconsistent with library | `TownError::Tactical` | `error.rs:42-44`, `compose/mod.rs:331-342,457-469` |
| | directory create or file write fails | `TownError::Io` naming the path | `main.rs:101-112,171-176,212-217` |
| | `--json` write fails after the PNG was written | exit 1; the PNG stays | `main.rs:218,231-233` |
| | `export` fails on the second file | exit 1; `<site>-plan.json` stays | `main.rs:251-253` |
| | huge explicit `--px` | no bound; allocation failure is not handled | `render.rs:208-211` |
| `arda-refine stats\|survey\|coastwet` | world fails to load, fine terrain declared but unreadable, shore layer present but unreadable (`world_source.rs:95`) | error printed as `arda-refine: <e>`, exit 1 | `arda-refine.rs:84-92,144-145`, `world_source.rs:77-92` |
| | malformed `--cell` | `String` error, exit 1 | `arda-refine.rs:68-75` |
| | `gx · 64` overflows `i64` | unchecked multiplication (`arda-refine.rs:154,181,183`); `[profile.release]` sets no `overflow-checks` (`Cargo.toml:71-72`) | |
| | a cell read or refinement fails mid-survey | survey aborts; lines for types already printed stay on stdout | `arda-refine.rs:229,234,280,289` |
| `GET /v1/tactical/library` | none in the handler: `summarise` is infallible, enum serialisation failure degrades to `""` | always `200` | `world_routes.rs:371-376`, `library_dto.rs:72-77` |

- D10 retry and fallback: no retry anywhere in these entry points. The only fallback is `arda-town block`'s asset substitution (§insp-town-block-render), which is recorded and printed, never an error.
- D8 concurrency: concurrent `arda-settle render` processes get distinct scratch names by pid (`guard.rs:59-62`). Two CLI runs with the same `--out` race on `File::create`/`std::fs::write` with no lock; the last writer wins. `GET /v1/tactical/library` reads an immutable `Library` behind `Arc<AppState>` (`world_routes.rs:35`); concurrent requests do not interact.
- D3 input normalisation absent: `--site` is not lower-cased or trimmed (`samples.rs:22`); `--window` ignores non-numeric components (`main.rs:115`); `--focus` is case-sensitive (`main.rs:138`).

## State transitions

No persistent entity changes state. Every CLI entry point writes only the output files named in the Trigger table; none touches `society/`, the world directory or the library. The server route mutates nothing (D9 not in play).

## Invariants

1. Read-only inputs: no entry point here writes into `--world`, `society/`, or `--library` (writes only at `render/mod.rs:158-161`, `main.rs:101-112,182,218`; `arda-refine` stats and survey write nothing; the route returns a body only).
2. Determinism of the drawing: `render::draw` produces identical RGB for the same society, seed and canvas (`crates/arda-settle/tests/render.rs:9-44`, which also checks the crop stays within the canvas). The overview export and PNG encoding are not covered by that test.
3. Determinism of town output: plans for every sample site are identical for the same seed (`crates/arda-town/tests/plan.rs:17-26`); blocks for the same window are identical (`crates/arda-town/tests/blocks.rs:63-69`). Fallback resolution hashes the placement's world square, so a tag query picks the same asset in every block (`fallback.rs:7-9`). No test invokes the `arda-town` binary.
4. Non-determinism by design: `arda-refine stats` prints wall-clock timings (`arda-refine.rs:147-158`); its counts and histogram depend only on refinement output (09 §hash). `arda-refine render` (09) also prints a timing (`arda-refine.rs:115-117`).
5. Library summary ordering: grounds and kits in `BTreeMap` key order, roles sorted and de-duplicated, assets sorted by id, tags sorted (`library_dto.rs:108,123,128-143`); the body is fixed for the server's lifetime because the library is loaded once (`tactical/mod.rs:196`). Tested for content, not bytes (`crates/arda-server/tests/world_api/cells.rs:195-211`).
6. Off-world positions never reach the overlay arithmetic (`guard.rs:29-48`).
7. No test invokes the `arda-settle`, `arda-town` or `arda-refine` binaries: no `CARGO_BIN_EXE` use in those crates.

## Outcomes & side effects

| Entry point | Success stdout | Files | Exit |
|---|---|---|---|
| `arda-settle render` | `wrote <out>` | RGB 8-bit PNG; scratch dir created and removed under `std::env::temp_dir()` | 0; errors `arda-settle: <e>` on stderr, 1 (`main.rs:68-74`) |
| `arda-town render` | summary line, `wrote <out>` | RGBA PNG | 0; errors `arda-town: <e>`, 1 (`main.rs:260-268`) |
| `arda-town block` | block line, fallback lines, `wrote <out>` | RGBA PNG (`raster.rs:285-297`); optional block JSON | 0 / 1 |
| `arda-town info` | summary, grid line, function mix, notes | none | 0 / 1 |
| `arda-town export` | `wrote <dir>` | two pretty JSON files | 0 / 1 |
| `arda-refine stats` | §insp-refine-stats lines | none | 0; errors `arda-refine: <e>`, 1 (`arda-refine.rs:84-92`) |
| `arda-refine survey` | nine type lines | none | 0 / 1 |
| `arda-refine coastwet` | one pretty JSON `Report` (§insp-refine-coastwet) | none | 0 / 1 |
| `GET /v1/tactical/library` | — | none | HTTP 200, `application/json`, no `ETag` and no `Cache-Control` header (`world_routes.rs:375`; other tactical bodies carry `TACTICAL_CACHE_CONTROL`, `routes.rs:30-32`) |

- D11 termination: every CLI runs to completion or the first error; there is no cancellation handling, and an interrupted run leaves whatever output file was partly written (non-atomic writes, `render/mod.rs:287`, `main.rs:108`, `raster.rs:293`). An interrupted `arda-settle render` leaves its scratch directory under the temp dir (removal only at `render/mod.rs:154`).
- D12 visibility: `arda-town block` prints every library substitution; `--json` exposes the canonical (pre-fallback) block. `arda-settle render` prints the world seed on the page (`text.rs:243-244`). The library summary omits ground textures' images, wall pieces' geometry and every asset that is not a prop or vegetation (`library_dto.rs:96`).
- D13 notification: none; the server writes one stderr log line per library request, with `cache=hit` and `bytes=0` fixed (`world_routes.rs:374`).
- D15 record: no record is kept beyond the output files and that log line.

## Dimensions not in play

- D1 Authority: single local actor for every CLI; the server route is unauthenticated like all v1 routes (`16-service-api.md:27`).
- D5 Money: nothing is charged, credited or balanced.
- D7 Time: no deadline, expiry or calendar; the only clock is `Instant` for printed timings (`arda-refine.rs:115,147-155`, `world_routes.rs:372`), which affects no output file.
- D9 Lifecycle: no entity with states is created or changed (State transitions).
- D13 Notification: no channel exists beyond stdout, stderr and the HTTP response (Outcomes).
- D14 Effects on others: outputs are files at caller-chosen paths and an HTTP body; no shared state, cache or other actor is touched (`GET /v1/tactical/library` bypasses the render caches entirely, `world_routes.rs:371-376`).
- D15 Record and audit: nothing is persisted for later reconstruction beyond the requested output files.
