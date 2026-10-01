---
generated_date: 2026-09-23
generated_at_commit: 14a70b9144ddd
scenario: export
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22, features/2026-09-23-terrain-corrections@2026-09-23
---

# 04 — Export

> Evidence consolidated on September 25: the [study](../features/2026-09-23-terrain-corrections/STUDY.md) and [experiment catalogue](../features/2026-09-23-terrain-corrections/EXPERIMENTS.md) replace the raw terrain-corrections archive. Historical filenames below identify removed experiments; current evidence links lead to their retained summaries. Original/final worlds and the 32K output remain under `out/terrain-delivery/`.


Renders PNG and serializes JSON from saved world data on demand. The current CLI surface is documented in [export](../mockup/03-export.md); loading follows [load and query](05-load-query.md). Classic remains the default; Atlas is an explicit PNG presentation over unchanged saved geography. The five-world C06 data/export panel and approved golden pass; remaining visual realism findings are recorded in [open items](../open-items.md).

## Trigger & preconditions

- Trigger: `arda export`, or the public `export_area_with_quality_and_style`, `export_overview_with_quality_and_style`, `export_area_with_quality`, `export_overview_with_quality`, `export_area`, `export_area_with_scale`, `export_overview` and `export_block` functions. The older entry points retain Classic behavior (`crates/arda/src/export_quality.rs:20`, `crates/arda/src/export_quality.rs:94`, `crates/arda/src/lib.rs:98`).
- The CLI requires `--world <dir>` and `--out <dir>`. The library receives a loaded `World` and an existing output directory; the CLI creates its output directory after loading the manifest.
- `world.json` must parse, have format major 4 and contain a valid configuration with matching area dimensions. Its presence is the generator's completion stamp, not proof that every lazily read layer is accessible or valid. Other majors, including preserved format-3 worlds, are refused; no migration runs during export.

## Steps

1. Validate image options before loading the world or creating output. `--quality` accepts any integer 512–32768 pixels or an integer k/K suffix, where 1K is 1024 pixels; area and overview PNGs default to 8192 (8K). `--style classic|atlas` applies only to PNG area/overview exports; omission selects Classic. Reject explicit style with JSON or blocks, explicit quality with JSON, blocks or `--detail`, and `--detail` with JSON, overview or blocks. Clap rejects unknown styles. Resolve an explicit `--block ax,ay,cx,cy`, otherwise `--overview`, otherwise `--area ax,ay` (default `0,0`). Ordinary JSON exports without quality retain their existing behavior (`crates/arda-cli/src/main.rs:120`, `crates/arda-cli/src/main.rs:250`).
2. Read the requested saved data. Area PNG/JSON use owned, uncached `World::read_area` reads of cells and objects. Atlas derives a two-cell terrain halo from all eight neighboring areas in north, northeast, east, southeast, south, southwest, west, northwest order, copying saved height, terrain class, optional lake depth, wetness, moisture and canopy (`crates/arda/src/atlas.rs:35`, `crates/arda-render/src/atlas.rs:143`). World Atlas overview also constructs `OverviewChannelContext` for each target from its saved `AreaObjects` and every existing area in its 3×3 neighborhood. Neighbor objects use bounded object-only reads with modeled-domain validation. The context requires all existing neighbors, canonicalizes identical halo records, and refuses conflicting saved fields for the same directed endpoints (`crates/arda/src/export_quality.rs:125`, `crates/arda/src/world.rs:245`, `crates/arda-render/src/overview/channel_overlay.rs:43`, `crates/arda-render/src/overview/channel_overlay.rs:131`). Quality overview export reads owned areas as needed for 256-row output bands with up to 14 halo rows above and below; a target, terrain neighbors and object-only channel neighbors can be reread across bands. The legacy buffered overview path visits every exported area once. A block request uses the separate cache for its area’s entire decompressed block archive (`crates/arda-render/src/overview/streaming.rs:100`, `crates/arda/src/world.rs:208`).
3. Render the selected PNG. Quality area output is square, from 512×512 through 32768×32768, defaulting to 8192×8192 over the same 512×512 100 m cells. Quality overview output uses the selected size for its long edge and rounds the short edge to the nearest pixel from the manifest’s area-grid aspect ratio. Areas stream reusable rows; overviews stream bounded bands. Atlas builds fixed 516×516 saved-cell context and 514×514 palette/light/class/height samples. Saved lake membership and physical area-channel rendering stay unchanged. World Atlas overview uses saved D8 centerline strips, terminal footprints and incident junction hulls. Recipe 4 uses display width `min(3 × saved_width_dm, 4_000 dm)` and 13/16 water opacity over Atlas land; older recipes retain `min(8 × saved_width_dm, 10_000 dm)` and full opacity. The overlay clips shapes in source Q20 area coordinates, projects to possibly unequal output partitions, and unions bounded Q20 pixel coverage. A 64-piece fragmentation threshold triggers recursive quarter-pixel subdivision to depth 6; work is capped at 250 million units per tile construction and row, with typed refusal instead of dropped geometry. Sea and lake feature masks own their pixels. The older discharge-band Mid/Dark chamfer symbols remain in direct legacy Atlas overview APIs, and Classic retains its right/down Dark extension (`crates/arda-render/src/overview/channel_overlay.rs:168`, `crates/arda-render/src/channel_geometry.rs:380`, `crates/arda-render/src/channel_geometry.rs:439`, `crates/arda-render/src/overview.rs:90`). Blocks use the unchanged built-in symbolic 24-tile vocabulary, at eight pixels per square: a 64×64 block produces a 512×512 PNG.
4. For area JSON, serialize one combined document containing `cells`, local `rivers` and `lakes`, saved `channel_edges`, and copied `hydrology` context. JSON schema version is 2; the area's hydrology model revision is 2 with `representative_annual_balance` and `mean_annual_discharge` semantics. Global water IDs and whole annual litre amounts use decimal strings; mean discharge retains the `*_milli_cumecs` keys and L/s units. Cell JSON contains height, terrain, cover, slope, aspect, drainage area, discharge, watercourse order/width, height above river and wetness. It omits stored temperature, rainfall, moisture, forest density, road and `built_by` fields. It is not a complete VTT properties payload. Block JSON uses schema 2 and contains the saved square IDs, `relaxed` flag and an ID/name legend for the existing 24 tiles.
5. Quality exports, including default area/overview PNGs and Atlas `--detail`, stream to an exclusive temporary sibling file, flush and close it, then rename it to `area_AX_AY.png` or `overview.png`. Quality and style do not enter the filename. Classic `--detail` writes `area_AX_AY_detail.png` from a completed byte buffer; combined `area_AX_AY.json` and `block_AX_AY_CX_CY.png|json` retain buffered writes. Area indices have at least two digits; block cell indices have at least three. The CLI prints `wrote <path>` for the single artifact (`crates/arda-cli/src/main.rs:290`, `crates/arda/src/export_quality.rs:143`).

Older saved worlds keep the original land palette and its Q12 wetness tint exactly. Recipe-4 cells additionally carry climate-derived soil moisture and canopy density. Atlas blends dry, lush and canopy tones from those saved fields before exposed rock and altitude/slope snow can override them; sea and lake colours are unchanged. Rock tint still varies continuously from 1–2 km elevation, and snow appearance from 2.85–4.3 km. Local northwest light retains bounded Q12 modulation. Area and overview derive the material in the same `AtlasTerrain` constructor before class-filtered resampling. These are deterministic material and display rules, not a simulated forest species, lithology or snow-storage model (`crates/arda-gen/src/orchestrator/fine_materials.rs`, `crates/arda-render/src/atlas.rs`).

For Linear×Linear output, Atlas reconstructs displayed land/sea ownership from four class-directed signed heights using exact rational pixel centers and i128 bilinear weights. Land samples are at least +1 mm and sea samples at most −1 mm. Positive chooses land, negative sea, and exact zero retains saved ownership. Saved lakes, quads touching lakes, alternating land/sea checkerboards, exact saved-cell centers and guarded one-cell islands/straits retain saved ownership. Any Box axis also retains the existing aggregation/ownership rule. Area colour and channel clipping and both overview paths use this same classifier; an internal AtlasSea feature prevents connected channel overlay and legacy river-trunk widening over reconstructed sea. Classic remains unchanged (`crates/arda-render/src/atlas.rs:412`, `crates/arda-render/src/channels.rs:451`, `crates/arda-render/src/overview.rs:428`, `crates/arda-render/src/overview/streaming.rs:92`).

## Branches

- Area: quality PNG (default 8K), Classic legacy PNG Detail, Atlas `--detail` at 4096 through the standard quality filename, or combined JSON. Request PNG and JSON in separate calls. The library retains `AreaImageScale::Preview` (512), `Detail` (4096) and `Custom(ImageQuality)`; Custom shows physical coverage only, while the quality facade maps 512 to Preview for compatibility.
- Overview: PNG at the selected quality, defaulting to an 8K long edge. `--area` does not change this branch. With no explicit quality, the legacy behavior of ignoring `--format` remains; explicit quality combined with JSON is rejected. If both `--block` and `--overview` are supplied, the block branch takes precedence.
- Exact-size overview: streaming `write_overview_png` and `write_atlas_overview_png_with_channels` support 1–78 areas per axis and up to 32,768 pixels per axis, including square 32K output. The connected API requires an `OverviewChannelContext` for each area; `OverviewRaster::push_atlas_with_channels` provides the buffered equivalent. The legacy `write_atlas_overview_png` and `OverviewRaster::push_atlas` continue to render the older Atlas symbol profile when called directly. Buffered `OverviewRaster::new_exact` retains its 134,217,728-pixel total cap and at least one output pixel per area per axis; its regular constructor retains 1–512 pixels per area and the 64-million-pixel cap (`crates/arda-render/src/overview/streaming.rs:75`, `crates/arda-render/src/overview.rs:338`, `crates/arda-render/src/overview.rs:267`).
- Block: symbolic PNG or JSON for a materialized cell. Generation currently saves blocks only at a 64-cell stride over land; a valid in-range cell need not have a block.

## Unhappy paths

- Malformed selectors, invalid quality values or image-option combinations, absent/unreadable manifests and incompatible format majors produce CLI failure or the corresponding typed library error.
- Out-of-range area/cell coordinates or an unmaterialized block return a typed range error. CLI block failures add the sampled-land-block explanation.
- Missing, inaccessible, corrupt or truncated saved layers can fail after manifest loading. Area codecs bound cell/object input bytes before allocation and validate copied water geometry against the manifest-derived fine domain.
- Atlas also requires valid saved layers for each in-bounds neighboring area. Incomplete, duplicate or contradictory halo directions and absent in-bounds context fail with `RenderError::AtlasContext`; an outer-world boundary is handled explicitly (`crates/arda/src/atlas.rs:43`, `crates/arda-render/src/atlas.rs:176`).
- Invalid lake/channel geometry, incomplete or conflicting Atlas overview channel context, bounded pixel-union work/depth, PNG encoding and area JSON serialization failures propagate through `ExportError::Render`. An unknown symbolic tile ID produces `UnmappedTile` (`crates/arda-render/src/overview/channel_overlay.rs:131`, `crates/arda-render/src/channel_geometry.rs:439`, `crates/arda/src/export_quality.rs:125`).
- Output creation/write failures propagate. Quality exports replace an existing completed PNG only after successful encoding/flushing; an ordinary failure preserves that PNG and attempts to remove the temporary sibling. Abrupt termination may leave a temporary file, and there is no power-loss durability promise. Legacy Detail/block/JSON exports and legacy buffered library entrypoints still call `std::fs::write` after rendering/serialization; a write failure can truncate their destination.

## State transitions

Saved world layers remain unchanged. Area and overview export reads do not populate retained area caches; a block export can populate its area's block-archive cache. The output artifact is created or overwritten. No terrain or hydrology generation runs during export.

## Invariants

- Repeating the same request over unchanged saved data with the same implementation, style and quality produces identical PNG/JSON bytes, including after reloading the world. Buffered and streamed PNG encodings may differ even when their decoded pixels agree.
- Quality and Detail increase image resolution, not the 100 m terrain grid or tactical detail. Narrow channel coverage never becomes an opaque 100 m square merely because the preview has one pixel per cell.
- Annual water fields describe climatological support, not a dated weather snapshot, seasonal minimum or perennial guarantee. Potential spill and supported annual outflow remain distinct saved facts.
- JSON semantics are explicitly versioned. The historical schema-1 additive-only proposal does not describe the current schema-2 contract.
- Tactical generation rules and the 24-tile vocabulary are unchanged by area water/detail export.

## Outcomes & side effects

- Success returns the written path; the CLI prints that path and exits successfully. No artifact-size or render-time promise is part of this surface.
- Failure leaves saved layers unchanged and may leave the output directory. Quality-path failures preserve the previous completed PNG; ordinary failures attempt temporary-file cleanup. Legacy buffered writes can leave a partial destination. This remains separate from generation's manifest-last publication contract.
- Source authorities: `crates/arda-cli/src/main.rs`, `crates/arda/src/{lib,world,atlas,export_quality}.rs`, `crates/arda-render/src/{atlas,quality,carto,channels,channel_geometry,json,hydrology_json,symbolic,overview}.rs`, and `crates/arda-render/src/overview/streaming.rs`.

## Dimensions not in play

User tileset manifests, sprites/artwork, glyphs, collision/movement attributes, complete per-cell VTT payloads, settlements, roads, road crossings, passes, buildings, population, NPCs and named continent/society exports remain deferred. Saved shared water crossings describe river boundary flux, not road or settlement crossings.

The 2026-08-25 build-gate amendments proposed enriched tile legends (material, traversability, movement cost, cover and hazards), POIs/buildings, NPC sheets, realms and an `arda serve` consumer sharing the serializers/renderers. Those remain deferred designs, as do the original separate `cells.json`/`objects.json` exports and schema-1 additive-only proposal. [Serve](../mockup/06-serve.md) remains a future surface.

## Canonical fine-source Atlas exports

A world declaring fine terrain causes Atlas area/overview export to verify its source once per export and read one bounded area window at a time. Missing/corrupt declared terrain fails explicitly. Fine heights and physical gradients determine land material/light at pixel centers; reduced footprints average fine shaded colors. Recipe 4 also reads a separate, globally aligned ~1 km canonical height context with an 8 km halo per area. Eight-direction horizons at 1, 2, 4 and 8 km yield at most 18% broad valley darkening; recipe 2 retains the original light path. Global coordinates and neighbor support prevent local terrain regeneration at area boundaries. Only actual outside-world queries may clamp to source endpoints. Saved 100 m water classes, shoreline contours, lake depths, wetness and connected channel overlays keep their roles. Worlds without fine terrain use the legacy path. The earlier recipe-2 integration and its visual limits are recorded in the [fine Atlas evidence](../features/2026-09-23-terrain-corrections/EXPERIMENTS.md#fine-atlas-integration).

## Recipe-5 formed Atlas shading

Worlds declaring recipe 5 or later use `AtlasTerrain::new_with_fine_formed`.
Recipes 2–4 and Classic output are unchanged. Source:
[formed.rs](../../../crates/arda-render/src/atlas/formed.rs),
[fine.rs](../../../crates/arda-render/src/atlas/fine.rs).

### §atlas-formed recipes

The export reads the manifest recipe and passes it to
`AtlasTerrain::with_recipe`. Recipe-6 worlds get the v0.2 look described in
this section: the v0.2 palette and lake stops, surface mottling and sub-grid
detail, stored shore classes, snow sky light and the highlight shoulder;
curved, relaxed, source-tapered rivers with the log-scaled minimum width and
v0.2 river colours; braid threads where water forms are stored. Recipe-5
worlds render exactly as v0.1 drew them: the frozen shader in
[formed/v5.rs](../../../crates/arda-render/src/atlas/formed/v5.rs), straight
river strips between snapped nodes, the stepped minimum widths and the v0.1
river colours. Constructors default to recipe 6.

### §atlas-formed arid basins

Recipe-7 worlds (logic/02 §world-water arid basins) also pass the arid water
of the area and its neighbours to `AtlasTerrain::with_salt`: saline lake
cells (from each lake's stored saline flag), and salt crust and mudflat
cells (from the stored playa runs). Per fine sub-sample, with bilinear
weights over the four surrounding cells:
- **Saline lakes** use lighter turquoise depth stops instead of the fresh
  lake stops: `#a8e2d6` at 0 m, `#78cfc6` at 1.5 m, `#4fb3b4` at 5 m,
  `#3593a2` at 15 m, `#267890` from 40 m.
- **Salt pans** paint over the shaded land colour: mudflat `#d9cdb0` at
  13/16 of its weight, then crust `#f0ece2` at 7/8, so a little light and
  shade remain.
- **Dry land below sea level** inside the continent (all four cells land)
  is land, not sea: a dried basin floor below sea level used to take the
  sea ramp.

Recipe 6 and earlier pass nothing and draw exactly as before.

### §atlas-formed light

A single north-west light at 42° elevation. The relief term is a weighted sum
(0.30/0.25/0.25/0.20) of Lambert terms from gradients at four baselines: 39 m and
312.5 m from the fine window (halo widened to 450 m), and 1 km and 3 km from the
1 km context. Vertical exaggeration per baseline is 1/1.5/3/5. The sum is
compressed with a Padé tanh, so shadows never reach black. Shadows take a warm
grey-violet tint, lit faces a warm highlight, and concavity plus sky exposure
add ambient occlusion.

### §atlas-formed materials

Colour is evaluated per fine sub-sample from the canonical field. Elevation bands
run olive to ochre to umber, dry to moist using interpolated saved moisture.
Valley floors (156 m concavity ring) are greener, crests more ochre. Rock
appears by slope and altitude. Snow starts at 2,700 m, higher on south-facing
slopes, lower in gullies, and sheds from cliffs. (Superseded for snow: perennial snow
and ice ramp in from −4 °C to −7 °C mean annual temperature at the sample's
height, from saved temperature lapsed at 6.5 °C/km. South aspect counts up to
1.5 °C warmer, gullies up to 2.5 °C colder and convex ridges up to 1.5 °C
warmer, so snow streaks down couloirs and ridges stay rocky.) Where the ring touches the sea,
low gentle ground is sand and steep ground is shore rock.

### §atlas-formed vegetation

Forest canopy weight = moisture (90→170 of 255) × warmth × gentleness:
- **Warmth** is the saved temperature lapsed to the sample's height, ramping from +1 to +4 °C mean annual, so it sets the tree line.
- **Gentleness** thins the canopy on slopes steeper than about 0.6.

Canopy colour is mottled by world-coordinate value noise (200 m and 60 m). South-facing slopes are drier: up to 40% less canopy and a slight ochre shift. The palette is matched to median colours measured from the reference: olive `#707540`, ochre `#908750`, rock `#6e6864`, snow `#d8d0c9`, water `#0a3559`. Canopy thins by up to 55 % on convex crests (156 m ring), so vegetation follows landform, and the fine canopy noise is ±0.2 of moisture. Snow pulls its light 40 % toward flat light, takes a blue-grey shadow tint and half the highlight; every land channel passes a soft shoulder above 190, so bright ground keeps gradation instead of clipping.

### §atlas-formed sea

Each non-lake sub-sample is classified by the fine field itself (h > 0 is land),
and pixel footprints average sub-samples, so coasts follow the 39 m contour,
anti-aliased. Sea colour is an 8-stop depth ramp: a bright band at 0–3 m, teal
shelf, blue slope, navy abyss. Gentle seafloor relief comes from 1 km-baseline
gradients. Lakes keep saved ownership.

### §atlas-formed shore

Worlds with a shore layer (logic/02 §fine-formation shore classes) paint shores from the stored classes, not from a height-and-slope guess. The 100 m class grid only picks the material: each sub-sample takes the bilinear share of each class among its four classified neighbours. The fine height and slope then place it, so the painted band follows the smooth waterline and not the cell grid:
- **sand** `#e2d0a0` and **shingle** `#b4ab98` on the strip below 2–7 m;
- **cliff** rock `#8e8476` on steep faces and half-strength up to 12 m;
- **rocky shore** `#868073`, two-thirds strength up to 12 m;
- **salt marsh** olive `#868f5c` below 1.5–5 m.

On water, **tidal flats** tint water shallower than 0.5–4 m toward wet sand-grey `#a6a283`, and **estuaries** tint two-thirds toward turbid teal `#4f8c86`. Worlds without the layer keep the previous heuristic.

### §atlas-formed rivers

Area channel vertices are snapped to the valley floor inside their saved 100 m
cell: the centroid of fine nodes within 1 m of the cell's lowest node. The offset depends only on the canonical field, so
area, halo and neighbour renders agree. Snapped edges use a general-direction
strip; saved D8 strips keep their strict contract. Each formed edge is drawn as a uniform cubic B-spline over its main-stem neighbours (the largest inflow and outflow of each node; a tributary keeps its own direction into a confluence), after one relaxation pass `(prev + 2p + next) / 4` along main stems, as convex quads with shared normals, about one per two pixels. A source node (no inflow) starts at a quarter of the saved width, so streams taper to a point. Rivers at least 55 m wide on floodplains (312 m slope below 3%, with a defined 6 km down-valley gradient) also get a meander offset. It is a sinusoid across the valley with wavelength 11 × channel width and amplitude up to a quarter wavelength scaled by flatness, with phase set by absolute position along the valley, so every render of a vertex agrees. Worlds with stored water forms (logic/02 §world-water publication) carry their meanders in the saved courses, so this offset is not applied there; the relaxed B-spline centreline then follows the saved meanders.

### §atlas-formed braids

Where an area's stored water forms mark a segment braided, the overview draws two extra threads, each as wide as the channel symbol, across its belt. A node's thread points lie on the normal of that node's own downstream edge, offset by `0.45 × belt × sin(phase)`, where the phase is a sinusoid of the node's absolute position (periods of 700 and 1,000 position units). Consecutive pieces therefore join, the threads cross the main channel regularly, and they converge on it where the belt ends. Worlds without forms render as before.

### §atlas-formed close zoom

When a pixel is smaller than 12 m, height and the 39 m gradient blend toward a uniform cubic B-spline patch of the fine field: fully at or below 4 m per pixel, on a smoothstep between. The B-spline smooths rather than interpolates and stays within the local node range. It removes the piecewise-constant slope facets of bilinear heights, and softens the field's 39 m channel staircases, which would otherwise show at 16K and 32K. Coarser exports keep the crisp central differences, and land/sea ownership still comes from the bilinear field.

### §atlas-formed lakes

For each fine sub-sample, the highest saved lake surface among the surrounding cells is taken. The sub-sample is water where the fine ground lies below that surface, coloured by true depth: a light margin (`#74b8c8` at 0 m, `#4a9cc0` at 1.5 m) deepening quickly (`#1c5690` at 15 m) to `#0f3462` at 120 m, so even shallow glacial lakes read deep blue.

Canopy weight reads saved moisture plus a ±0.2 fine term (900 m world-coordinate noise), so canopy edges follow organic shapes rather than the 4–13 km soil patches.

### §atlas-formed scale

Every per-sample colour input is prefiltered to the drawn scale. The pixel footprint is the linear-kernel pixel size, or the whole box-kernel span for reduced outputs.
- **Material slope and aspect** blend from the 39 m to the 312 m gradient as the footprint grows from 40 m to 250 m.
- **Concavity** uses a ring no smaller than the footprint.
- **Texture terms** fade to their mean before their wavelength drops below a few pixels: canopy mottle (60 and 200 m) fades between 50 and 200 m, fine canopy moisture (900 m) between 60 and 200 m.
- **Visible noise** is value noise averaged over two rotated domains, never raw axis-aligned value noise.

Formed rivers have a minimum on-screen width that grows with log2 discharge, the same in overviews and area renders: none below 0.7 m³/s, 0.5 px at 1 m³/s, about 1.3 px at 5 m³/s, 2.5 px at 50 m³/s, capped at 4 px.

### §atlas-formed detail

Below the field's own resolution the light gets sub-grid relief, never height, ownership or drainage. Four octaves (160, 96, 48 and 24 m across the fall line) each fade in once the wavelength spans three pixels and are full at six, so no export aliases or is supersampled. On slopes the relief is gradient noise stretched 3:1 along the smooth 312 m fall line, blended from four fixed orientations (12°, 57°, 102°, 147°) so it never swims; its slope amplitude rises from turf to bare rock, and only rugged faces are strongly directed. Runnel troughs darken the ambient term by up to a quarter. Turf, canopy and scree also get a 40 m and 20 m tonal texture (±4 % turf, ±7 % rock) under the same fade.
