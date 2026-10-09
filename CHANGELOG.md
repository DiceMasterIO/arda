# Changelog

Release notes for Arda. The design ledger (decisions, stage records and per-change detail)
lives in `docs/capstone/changelog.md` and `docs/capstone/changelog.d/`.

## 0.8.0 — 2026-10-09

- **Forests on default worlds.** Worlds generated with the default `--terrain legacy` never ran the ground-ecology pass, so every land cell had forest density and moisture 0 and was grass. Legacy worlds now get the same ecology as the fine path: MICRO seeds 1, 7, 42, 99 and 2024 go from 0% to 41–61% forest cover. Saved worlds change for land cells; climate, hydrology and blocks do not.
- **Biome-correct tactical maps.** Refined cells read marsh, aridity, coast, alpine and crag signals from the cell fields. Wetlands get marsh ground, pools, reeds and cattails. Coasts get the sea and a beach with driftwood and tide pools. Snowbound peaks get no plants. Steep ground is terraced into ledges and cliff bands. Steppe turf is tinted dry. Forests close their canopy (crown cover tracks forest density). Texture variants are flattened to one tone, removing the checker pattern, and worked parcels have crisp edges.
- **Dungeons and caves (new `arda-dungeon`).** Deterministic dungeon levels (rooms, corridors with loops, locked and secret doors, themed rooms, stairs, lights) and natural caves (connected caverns, a mouth, pools and streams), as ordinary tactical layouts with rules sidecars. `arda dungeon` CLI and `/v1/tactical/dungeon` endpoints. `WallSegment` gains optional `tags`; scene walls gain `locked`. Placeholder art and 34 tier-4 prompts for underground ground, cave walls, stairs down, torch sconces and stalagmites.
- **Towns.** Farmstead compounds are no longer placed on top of town crofts (no crossing walls); yards, gardens, croft parcels and fields have one ground each; each walled room has one floor finish; yards are dressed sparsely for their building's trade with clear paths to doors. Towns build over at most 1 ft of standing water in their own footprint, so sea-level ports are no longer flooded.
- **Notables.** Rank follows job and tier and is raised by any office held; lifestyle and wealth stay within the rank's band; "Master" titles only at master rank. One civic figure per hamlet and a civic cap per tier; offices replace the tier officials they cover. On MICRO 42, masters drop from 70% to 25% of notables and government jobs from 30% to 7%.
- **CI and tests.** Green on Linux, macOS and Windows: Rust 1.99 lints, a documented `ttf-parser` advisory ignore, wall-clock budgets as ignored release gates, Windows drive letters in library stacks, LF checkouts, `--no-fail-fast`. Shared test fixtures carry a source fingerprint and rebuild when stale.
- **Docs.** Only `docs/capstone/` is tracked; the README shows an example sheet of procedural tactical maps.

## 0.7.0 — 2026-10-06

- **AI art pipeline (`tools/art-gen`).** A Node driver turns a 1,425-asset prompt pack into a tactical art library through a local Slopify instance. Assets are ground and water tiles, wall kits, props and vegetation, each with variant takes. `--parallel N` (default 5) keeps N assets generating at once. Every image is checked by an image reviewer (Claude Code or Codex, `--reviewer*` flags) against the brief and Arda's rules, and failed images are remade up to twice. Runs are filed into a named Slopify channel, recover from dropped connections and Slopify updates, and `--resume` / `--overwrite` pick up where a run left off. The prompt pack fixes several systematic misreadings: doors and gates sit in a gap along the wall line, wall posts are the post alone, fences are a thin line from above, vegetation has uneven outlines and varied sizes, and palisades are flat log ends.
- **Variant takes in the compositor.** A library may hold several takes of one asset (`<id>.altN`). Each placement draws one, hashed from the seed and its world position, so neighbouring windows agree. Wall pieces pick takes per world edge or vertex. Tag queries pick a family, then a take. An explicit `.altN` id stays pinned.
- **One take per floor structure.** Bridges, docks and other floor-layer props draw a single take per connected structure instead of a patchwork. Separate structures can still differ. `compose::resolve_all` resolves a whole layout in one pass.
- **Vegetation pose.** Square vegetation is drawn at one of seven sizes (85–115%) and is sometimes flipped across the diagonal, which keeps light from the top-left. `fixed_pose` opts out; the placeholder library uses it, so placeholder renders are unchanged. The scale is visual only, and rules data follows the footprint.
- **Importer.** Enclosed backdrop pockets are cleared (holes inside foliage and props). Floor-layer tiles fill their footprint edge to edge. Vegetation keeps its drawn size relative to the frame and flags tiny objects. New manifest keys: `holes`, `rot_free`, `fixed_pose`.
- **Riverside layout.** Rails along both sides of the bridge.

No change to world generation, terrain, rendering of world maps, or saved formats.

## 0.6.0 — 2026-10-02

- **Worked land on relief tiles.** With a `society/` directory, close-zoom relief tiles draw field parcels with hedgerows and drystone walls, pasture, arable, orchards and woodland, roads and lanes, and settlements' buildings and streets, from the same geometry as the tactical maps. On low ground, valley floors and interfluves are tinted and slope light is amplified as local relief shrinks, so terraces and bluffs read. Worlds without `society/` render as before.
- **Historical field systems.** The field partition is now a global hierarchy of land blocks split along roads, square to their longest side and along contours, into small closes by the houses, larger fields further out, furlongs of strips by villages, floodplain meadow and open commons. It replaces the Voronoi-like parcels, and the tactical maps and relief tiles share it. Cuts that would leave a wedge-shaped field (a corner under about 35°, other than at a road) are refused, and the would-be wedge stays part of its neighbour.
- **Recipe 8 (opt-in, `arda generate --terrain fine --recipe 8`).** Alluvial valley floors scaled to discharge: where a river's specific stream power is low, the valley bottom is aggraded and planed to a flat floor (`60 m × A^0.4` half-width) bounded by bluffs, before the terraces are cut. Confined and steep valleys and hill country keep their shape. Recipes 5–7 are unchanged; 7 stays the default.
- **Plains diagnostics.** The `plains_metrics` example separates plains from low hill country and measures floodplain width by river size. On the full-size seed-42 world, recipe-7 plains already have a median slope of 0.5°; the steeper MICRO lowland is hill country near the sea.

## 0.5.0 — 2026-10-01

- **Cell contract 3.** With a `society/` directory, `/v1/cell`, `/v1/point` and `/v1/area/.../cells` take `road` from the new `society/roads.bin` (which adds `footpath`) and `built_by` from the land-use owners (now a settlement id string), and add `land_use` and `realm_id`. `ARDACOLS` layout 2 carries the new columns. `arda settle` writes `roads.bin`; re-run it to get the raster for an existing world. Worlds without `society/` keep their stored `road` and `built_by`.
- **JSON Schemas.** Every public body has a draft 2020-12 schema, generated from the same Rust types as the TypeScript bindings, committed in `bindings/schema/` and served at `GET /v1/schema` and `/v1/schema/{Name}.json`. Real responses of every route validate against them on a recipe-7 world, including the contract-3 cell fields.
- **Typed scenes.** `SceneDto`, `TacticalScene` and `Token` are generated TypeScript types, and the viewer reads scenes through them.
- **Sheet mappings.** `arda-server --sheet-mapping game.json` reshapes every served NPC into a game's own schema with a declarative file, checked at startup. `identity.json` and `5e-srd-monster.json` are included.
- **Importing AI art.** The new `arda tactical import` (crate `arda-art-import`) turns a folder of raw generated images into a valid tactical library: background removal, baked-shadow stripping, fitting, tileable textures, an optional colour grade, a report and a contact sheet.
- **Library stacks.** Wherever a library is named (`arda tactical render` and `validate`, `arda-server --library`, `arda-town block --library`, the scene and crossings examples), a `top:…:bottom` stack works too: the leftmost library wins and the others fill in what it lacks.
- **Hardening.** `Source::pan` is a required method. Images revalidate by `ETag`. Error bodies no longer show absolute paths. Tactical PNG encodes are admitted one at a time. Catalogue footprints are capped at 16 squares a side, and import manifests and stacks respect the cap. Society lookups are indexed (8,000 settlements in 1.0 s, was 5.1 s). Ways, fields, scene, names and NPC fixes from the review rounds. A test-fixture race is fixed.
- `deny.toml` allows MIT-0 and Zlib for the dev-only schema validator.

## 0.4.0 — 2026-10-01

- **Recipe 7 is the new default: climate.** Rainfall now has a subtropical dry belt. Runoff weights channel initiation and incision, so wet uplands are more dissected and dry land less. Arid tectonic basins keep terminal saline lakes on salt pans, with mudflat margins. Atlas draws saline lakes and salt pans, and tactical maps get `salt_crust` and `mudflat` ground. Set the band with `arda generate --latitude S,N`. `--recipe 6` reproduces v0.2–v0.3 worlds byte for byte, and recipes 5, 6 and 7 are all golden-pinned. `water.bin` gains layout 2.
- **Zoom continuity.** Close-zoom relief and tactical maps share one water geometry (`WaterRegion`), so rivers, lakes and coasts sit on the same squares at every zoom.
- **Open country.** Species follow altitude, aspect, wetness and the tree line. There are groves and glades, rock outcrops, boulder fields, game trails and footpaths continuous across blocks, and soft snow cover. A fields bug that painted noise over unused land is fixed.
- **Opt-in looks.** An oblique overview (`--style atlas-oblique`, `?oblique=1`) and a tactical world grade (`?world_grade=1`). Defaults are unchanged.
- **Code health.** Every source file over about 550 lines is split, with byte-identical outputs. A flaky fixture race is fixed.

## 0.3.0 — 2026-10-01

Closes the gaps found in the post-0.2 goal audit.

- **Recipe versioning restored.** Recipe 6 is the default and reproduces 0.2.0 byte for byte. Recipe 5 reproduces 0.1 again. Both are pinned by golden tests. Worlds generated by 0.2.0 (recipe 5 with `terrain/shore.bin`) load as recipe 6. Use `arda generate --recipe 4|5|6`.
- **Realms.** Seats are spread out, and every realm has a primate capital city. Borders still follow rivers and ridges.
- **Towns built with WFC.** A new generic `arda-wfc` solver, shared with tactical terrain, fills interiors, streets, squares, yards and wall pieces inside each town plan. The vocabulary has 306 tiles. Pews, beds, shelves, racks and hall tables are laid out in rows. The relaxed-fill rate is about 0.1%, and fallbacks are reported in the block metadata.
- **Interiors vary.** No two neighbouring homes share a layout. In the seed-42 city, 1,665 of 1,673 homes are distinct.
- **Faster tactical maps.** A cold battle-map block renders in about 0.3 s and a quarter window in under 0.1 s, enforced by the end-to-end gate. A PNG strip-encoding bug was fixed.
- **API.**
  - Commoners can be fetched by reference (`<settlement>.<building>.<index>`).
  - `GET /v1/npcs` filters by settlement, building, job and realm, with paging.
  - New `/v1/buildings/{s}.{b}/residents` and `/workers` routes.
  - Overview tiles are also served as WebP.
  - `POST /v1/tactical/prefetch` warms neighbouring cells in the background.
- **Viewer.** It prefetches neighbouring cells, and edge arrows walk to the next cell seamlessly.

## 0.2.0 — 2026-09-30

Arda grows from a world generator into a product: a physically formed world, the people who
live in it, tactical battle maps of any 100 m cell, an HTTP API and a browser viewer. The same
seed still gives the same bytes. Legacy (Classic) worlds are unchanged but for the version
string in `world.json`.

### World physics (recipe-5 "fine" terrain)

- **Variant Q mountains:** stream-power formation with drifting plates, convergent routing,
  glacial troughs and talus seams gives ranges with real crests, valleys and flanks. The
  uneroded-bands fix removes the flat strips that used to cross belts.
- **Water** (`areas/*/water.bin`): hydraulic geometry for every reach, irregular meanders
  with oxbow lakes that hold water, braided reaches, and river-led deltas with levees, forking
  distributaries and delta islands. Closed basins drain or become proper lakes.
- **Coasts** (`terrain/shore.bin`): beaches in bays and at river mouths, shingle, cliffs,
  rocky shores, marsh and tidal flats; islands, volcanic arcs standing above the sea, a
  continental shelf with wandering submarine canyons, and coastal plains.
- **Atlas look:** sub-grid relief detail, tapered smooth river centrelines, a warmer formed
  palette and stored shores painted along the coast.
- A full-size world (500 × 1000 km, seed 42) generates within 13.4 GiB peak memory.

### Settlements, names and society

- `arda settle`: settlements of every tier sited on the terrain, roads, crossings, passes,
  realms, land use and names (`<world>/society/`).
- `arda-names`: languages and dialects per culture, used for places, houses and people.
- `arda society build`: a town plan for every settlement (streets, plots, buildings, walls,
  crofts, river bridges and docks), politics, trade, economy and history, and the stored
  notables. MICRO seed 42 holds 629 settlements and about 92,000 people.

### People

- `arda-npc`: households, jobs, personalities and relationships for every settlement, with
  notables bound to the offices and houses of the society layer.
- Every NPC has an SRD 5.1 stat sheet (see `NOTICE` for the attribution).

### Tactical maps

- 5 ft battle maps of any 100 m cell: WFC-driven terrain refined from the world, towns from
  the settlement plans, roads, bridges and fords on the refined rivers, fields, pastures and
  crofts around villages, and organic forest edges and pools.
- Scene rules (movement, cover, sight, decks, exits) in a format-2 sidecar, and NPC tokens
  placed in their homes and workplaces.
- The art compositor renders layouts to PNG with global seeding, so neighbouring cells meet
  seamlessly; a placeholder art library ships in `assets/tactical/placeholder`
  (`arda tactical validate` checks it).

### Mid-zoom relief

- `arda-midzoom` refines the stored 39 m field to about 10 m with drainage-aligned gullies
  and ribs. Relief tiles shade with the overview's own shader, palette, shores and rivers, so
  zooming past the overview is continuous.

### Server and viewer

- `arda-server`: the HTTP API (`/v1/world`, cells, points, overview and relief tiles,
  settlements and plans, tactical cells, windows, scenes and PNGs, `/v1/npc/{id}`), with
  TypeScript bindings generated from the Rust types in `bindings/ts/`.
- `apps/viewer`: a browser viewer that pans and zooms the world, switches to relief tiles past
  the overview, and opens the tactical map of a cell with its tokens.

### Compatibility

- World format 4 is unchanged; recipe-5 worlds gain the optional `shore.bin` and `water.bin`
  layers. Worlds generated with 0.1.0 fine terrain should be regenerated to get the new terrain,
  water and coasts.
