---
generated_date: 2026-08-24
---

## 2026-10-14 - The tactical rules contract
- **arda-scene**: the rules sidecar now adds to what the placements imply and never erases it. A cell's `false` is no opinion: every asset whose catalogue entry blocks movement or sight is impassable or opaque in the built scene, whichever layer owns the square; flags are or-ed, cover is the stronger, only `water_depth_ft` and `deck` replace. New `RulesCell::flag` and `RulesCell::stated_cover` (a layer states a flag only when it holds), `squares::fold_placements` and `squares::resolve_known`. A step of 5 ft or more touching `cliff` ground is a climb (logic/12 §scene-climb).
- **arda-refine**: refine states ground difficulty, water, canopy obscurement and trunk and rock cover only; it no longer draws its own 3 × 3 outcrop walls (outcrops, boulders and large rocks block by their catalogue footprints). Sea depth is `clamp(level − land, 0.3, 40)` m per square in the block and region paths, so deep water runs on across block edges.
- **arda-town, arda-ways, arda-fields**: their sidecar cells state flags only when they hold and no `none` cover, so tables, counters, stalls, wells and statues keep their catalogue blocking.
- **arda-tactical, arda-art-import, art-gen**: small scattered rocks are difficult terrain, not obstacles (maintainer decision): `veg.stones` (no cover), `veg.rock_small` and its alpine variant `veg.lichen_rock` (half cover) no longer block movement or sight. Changed in the placeholder specs and `catalog.json` (images byte-identical; `library_version` stays `0.3.0+seed58`, which versions the art, versioning.md), the importer's vegetation defaults (`stones`, `pebble*`, `scree*`, `rock_small`, `lichen_rock`) and the art-gen metadata (`prompts.json` regenerated). Boulders, large rocks, outcrops, sea and snow rocks keep blocking.
- **arda-server API.md**: the tactical response example's `rules` follows the contract.
- **arda-ways**: a ford keeps water 5 ft or deeper (its squares are swum, no gravel shelf or bar there); shallower water is capped at 1–2 ft as before.
- **arda-blocks**: a verge (the soft claim that keeps the natural ground) merges its rules onto the natural cell, less its cover; the pipeline folds every placement's catalogue blocking, sight blocking, difficulty and cover into the composed sidecar (world seed, as `/scene`), so token placement and the tactical `rules` see the scene's blockers. Overlay unit tests move to `overlays_tests.rs`.
- **Docs**: logic/09 §linear-features, §reservations and §rules-sidecar; logic/11 §catalogue; logic/12 §scene-movement, §scene-climb, §scene-sight and §scene-sidecar (the rules contract).
- **Why**: review 3.4-1, 3.5-1 (Critical), 3.5-5, 3.4-3 (High), 3.5-7, 3.4-4 (Medium-High). Layers stated every field, and the scene applied a stated `false` as an override, so the catalogue's blocking was erased almost everywhere.
- **After (MICRO 42, `out/micro42`, `tests/rules_contract.rs` sample)**: blocking placements left walkable 392/426 refined-only (boulders and large rocks 147/147) and 3126/3288 composed (town props 2903/2903), now 0; sight blockers see-through 225/228 and 120/121, now 0; verge squares that lost refine's `difficult` 4817/8390 and `blocks_sight` 2279/8390, now 0; stepped dry edges touching cliff ground that are climbs 15508/44500, now 44500/44500. Synthetic sea: worst deep-water step across a block edge 4 ft east and a whole cell's depth south, now 1 and 2 ft (as inside a block). Ford: 6 ft water under a ford was filled to 1–2 ft, now kept (no ford square on ≥ 5 ft water in the MICRO sample).
- **Re-blessed**: contract kit, 5 of 55 cases (`tactical-cell`, `tactical-scene-day`, `tactical-scene-night`, `tactical-window`, `relief-tile`), bytes only; then for the small rocks `tactical-library` and the same four tactical cases. Response shapes unchanged. No tactical, dungeon or bindings golden changed.
- **Tests**: arda-scene `tests/rules.rs` (`a_sidecar_false_never_clears_catalogue_blocking`, `a_step_onto_cliff_ground_is_a_climb`, `small_scattered_rocks_are_difficult_and_passable`), `tests/derive.rs`; arda-art-import `small_scattered_rocks_default_to_difficult_terrain`; arda-refine `tests/sea_depth.rs`, `tests/schema.rs`; arda-ways `tests/wet.rs` (`a_ford_keeps_swimming_depth`), `tests/rules_sidecar.rs`; arda-blocks `a_verge_merges_with_the_natural_rules_it_keeps`, `tests/rules_contract.rs` (real world), `tests/compose.rs`; arda-fields `tests/rules.rs`.

## 2026-10-14 - Kitchen-garden art and dressing
- **Request**: 1.0 task 17, OI-33. Croft kitchen gardens and the farmstead plot drew as bare furrowed `farmland`.
- **Placeholder library 0.3.0**: seven new pieces (`veg.vegetable_bed`, `veg.herb_bed`, `veg.bean_poles`, `prop.cold_frame`, `prop.compost_heap`, `prop.rain_barrel`, `prop.garden_tools`, free tag `garden`), 292 assets (was 285); existing images byte-identical. `library_version` 0.2.0 to 0.3.0 changes render seeds: dungeon render hashes, contract-kit goldens and three tactical test pins were re-blessed.
- **Dressing**: `block/garden.rs` lays rows of beds in kitchen parcels (both fills); WFC kitchen palette gains clutter with per-chunk caps for compost and cold frame; the farmstead plot gets a laid-out garden. No new ground key. Town goldens: none pinned a croft kitchen.
- **art-gen / importer**: tier 7 (27 images), vocabulary additions. Generated 27 images (Codex gpt-6-astra, thinking high, $0 counted; `veg.bean_poles` and `prop.rain_barrel.alt2` were retried after a Slopify collect error) and imported into a scratch library (27 imported, 0 rejected, 5 flagged for a possible baked shadow); a village croft and the farmstead plot render with them over the placeholder. The art is not committed; the redo list is in the task report.
- **Closed**: OI-33 (code and prompt side; curation of the redo ids follows).

## 2026-10-13 - Tier-5 partition art prepared
- **Request**: 1.0 task 06, OI-32. Interior walls inside one building are tagged `partition`; until `timber_partition`, `wattle_partition` and `adobe_partition` art exists the compositor squeezes the kit's own piece to half thickness.
- **Importer**: `WALL_KITS` gains `log`, `adobe` and the three partition kits, so `wall.<kit>_partition.<role>` files are no longer flagged "not in the vocabulary"; coverage slots grow by five. A partition kit with missing roles is reported, not rejected (test `partition_kits_are_in_the_vocabulary`).
- **Compositor**: test that a library holding thin `timber_partition` pieces draws them as painted (thinner than the exterior piece) and that without them the half-thickness fallback stays.
- **art-gen**: `tiers.about` and the `expansion.mjs` header now describe tier 5 (57 images); plan.md has the tier-5 dry-run command and the thin-band redo rule. The images are not generated yet; the fallback stays until they are.
- **Closed**: OI-32 (code and prompt side; generation and import follow with the maintainer's approval).

## 2026-10-13 - Effect and damage art tier
- **Request**: 1.0 task 05, OI-45. DiceMaster draws spell effects and lasting damage over Arda's tiles itself; Arda now lists and serves matching art.
- **Catalogue**: new `AssetClass::Effect` (`effect.<name>.<n>`, free tag `effect:texture` or `effect:decal`, layer `floor`). Effects may be semi-transparent but not empty or opaque (`effect_alpha`); tileable ones must wrap in colour and opacity. `FORMAT_VERSION` stays 1. Layouts never place effects and the compositor ignores the class (test: layouts render byte-identical with the effects removed). Stacked libraries replace an effect set by name.
- **Placeholders**: 27 procedural effect images (frost, fire, acid, ash, web, grease, fog, water, scorch mark) and three damage props (`prop.rubble_heap`, `prop.broken_deck`, `prop.charred_floor`, tag `damage`), appended; every earlier PNG is byte-identical. `library_version` 0.1.0+seed58 to 0.2.0+seed58, which is part of the render seed, so placeholder renders changed once (pinned open-ground hash re-pinned).
- **API** (additive, no version bump): `GET /v1/tactical/library` gains `effects` (`id`, `kind`, `footprint`, `tags`, `variants`); new `GET /v1/tactical/library/asset/{id}.png` serves any library asset (ETag, `no-cache`, 404 unknown, 400 malformed). TS bindings and schema regenerated.
- **Importer**: `effect.*` stems keep alpha (no matte, shadow strip or flattening), textures wrap with an alpha-aware blend, decals fade before the border, and `alpha = "luminance"` (or a black border) derives alpha from brightness for light-on-black art.
- **art-gen**: Library prompt `arda-effect` and tier 6 (37 images: nine effect sets and the three damage props). Not generated yet; the real images wait for the maintainer's spend approval, and the placeholder effects stand in.
- **Closed**: OI-45.

## 2026-10-13 - map: all
key: map/all@b6cae7350f6b

- **Refreshed (covered paths changed)**: 01-architecture.md, 02-models.md, 03-conventions.md, 04-data-flow.md, 05-dependencies.md, 06-testing.md, 07-operations.md, 08-glossary.md, 09-interfaces.md, open-items.md, logic/07, 17, 18, 19, 20, 21, 22, uiux/01-direction.md, uiux/02-system.md.
- **Drift fixed**: moved `file:line` pointers remapped; 1.0 content added (overlays on exports, gazetteer and tile labels, bake, `npc/generate`, realms, contract kit, versions, release automation, container image, art spend caps, viewer licence policy, `arda-refine coastwet`); retired block archive removed.
- **09-interfaces**: bake tile and DELETE payload sections, `BakeRequest` and gazetteer tables, `contract-kit` edge; no `to`/`from` written.
- **paths_covered**: globs added for `contract-kit/**`, `tools/*.sh`, release workflow, Dockerfile, viewer licence files and the new gazetteer, realms, coastwet and label sources.
- **Open**: 02-models has no entity rows for a few scratch and error types of the blocks, wfc and dungeon crates (predates this run); 03-conventions lexical-count rows (`as`, `#[allow]`, HashMap) not recounted.
- **Skipped as current**: logic/01-06, 08-16, uiux/screens/*, mockup/*, standards.md, implementation.md.

## 2026-10-13 - Housekeeping before 1.0
- **tests**: `tests/zoom_land.rs` gains a module-header note: it is the `zoom_land` module of the `zoom_continuity` target (`mod zoom_land;`, `tests/zoom_continuity.rs`), not a target of its own, and is run with `cargo test --release --test zoom_continuity -- --ignored`. It stays out of the root `[[test]]` list; moving it was ruled out because 06-testing, logic/17 and open-items cite the current path.
- **docs**: `09-interfaces.md` gives `GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp` the path and query table it lacked (same fields as `/window.png`).
- **docs**: the 2026-09-25 terrain-study fragment is folded into `changelog.md` as a dated entry, trimmed (no `codex/` branch name, no agent or model names; commit `52eab74` kept), and the fragment removed.
- **capstone schema**: `logic/README.md` is checked as a scenario by `~/.agents/skills/core/references/schema.txt` and reports eight missing headings. The upstream fix is a `logic-readme` record above `logic-scenario` (`match logic/README.md`, `keys! generated_date`); the installed skill is left unedited and the patch goes to `GentBajko/capstone`. In a scratch copy it removes the row (32 findings to 24 before the refresh).
- **docs**: `versioning.md` gains `generated_date` frontmatter (the schema pass required it).
- **Why**: `map-check` on main reported stale and false-positive findings that every 1.0 branch would meet again; the test-registration and fragment questions kept coming back from audits.
- **Tests**: `cargo test --test zoom_continuity -- --list` still lists the `zoom_land::` tests; `cargo fmt`, `clippy`, `cargo test --workspace` green.

## 2026-10-12 - Contract kit: golden responses of the canonical world and a live checker
- **contract-kit** (new, repository root): `manifest.json` (kit format 1; fixture identity: seed 42, MICRO, fine recipe 7, society, `arda_version`, `server_version`, BLAKE3 of `world.json`, `library_version`, `contract_version`; 54 cases with method, URI, request body, status, content type, `etag`/`cache-control`/`location`, schema name and a golden file or SHA-256 plus size), `golden/` (38 JSON bodies as served), `requests/`, `world.txt` (the canonical world's file fingerprint), `README.md` (world recipe, serving, checking, manifest reference, the DiceMaster side). 41 of the 42 stable routes have a case, plus `no_block`, `out_of_range`, `not_found`, `bad_request` and two `304` revalidations; a finished bake's tiles are left out (their existence depends on when the background job ends). About 2.8 MB.
- **contract-kit** `check.mjs`: Node 22+ standard library only; replays every case against a running server, compares status, content type, headers and bytes or hash, names the first differing JSON path, exits 0, 1 (a difference) or 2 (unreachable). Tests in `contract-kit/test/` against a `node:http` stub.
- **arda-server** (tests): `tests/contract_kit.rs` replays the kit in-process on the stamped society fixture with the CLI's default limits and the placeholder library, validates every JSON golden against its served schema, checks the fixture identity and `world.txt`, and fails when a stable route of `versioning.md` has no case. `ARDA_BLESS_CONTRACT=1` re-records (never adds or removes cases). Dev-dependency `sha2` (MIT/Apache-2.0) for the SHA-256 the Node checker can compute.
- **tools**: `tools/release.sh <version>` re-blesses the kit after a bump and allows its files in the release diff; `--check` counts the manifest's `arda_version` and `server_version` as version sites (54 sites). `.gitattributes` keeps kit goldens and requests byte-exact.
- **docs**: 06-testing § Contract kit and the bless and live-check commands; 07-operations `ARDA_BLESS_CONTRACT`, the commands and the release step; logic/16 §api-schema and versioning.md point at the kit.
- **Why**: DiceMaster plans a replay test over recorded Arda fixtures and a live contract test against the published image. Arda only checked its own side (fresh schemas, live bodies valid); nothing pinned which bodies a client may rely on, and DiceMaster's docs named a legacy-terrain world without society. One versioned kit gives both sides the same canonical world, bodies and checker.
- **Tests**: `contract_kit.rs` (5 tests; failed first with an empty `golden/` on "missing golden", and with one altered golden byte naming the case), `node --test contract-kit/test/` (5). Live: the release CLI's canonical world was byte-identical to the test fixture (47 files), and `check.mjs` passed 54 of 54 cases against `arda-server` on it, cold (about 9 s) and warm (0.4 s).

## 2026-10-13 - Settlements, roads and land use on overview and area exports
- **arda-render**: `BandOverlay` hook (`paint(y0, rows, width)`) called on each streamed overview band and area row group before encoding; `write_overview_png_with_overlay`, `write_atlas_overview_png_with_channels_overlay`, `OverviewRaster::finish_with_overlay`, `render_area_png_to_with_overlay`; `RenderError::Overlay`. `None` keeps the exact bytes (goldens untouched).
- **arda-settle**: `render::band::SocietyBandOverlay` (land-use tone, roads by class, tier symbols, painted in 128-column strips from whole-image geometry); `layers.rs` split into band-paintable pieces (`road_lines`/`paint_roads`, `symbol_list`/`paint_symbols`, `land_tone`) with `arda-settle render` byte-identical.
- **arda-midzoom**: `land::Works` (parcels, tone, roads, plans) factored out of `LandWindow`; `land::export::AreaLandOverlay` for area PNGs; `export` module (`SocietyMode`, `export_overview`, `export_area`).
- **arda**: `export_overview_with_overlay`, `export_area_with_overlay`.
- **arda-cli**: `--society auto|off` on `preview` and `export`.
- **arda-server**: `?overlay=society|none` on `/v1/overview.png` and overview tiles; default `society` for worlds that have it; cache keys include it.
- **Why**: a GM looking at the world map, and DiceMaster's static base map, need the human layer, not bare terrain (OI-12, OI-14).
- **Tests**: arda-render hook (call order, `None` identity, row buffer independent of height); arda-settle band, strip and window seam equivalence; arda-midzoom border-settlement equality and Off/On export; arda-server overlay routes.

## 2026-10-12 - API freeze: versioning policy, version families and the schema freeze guard
- **arda-server**: `GET /v1/world` gains `server_version` (the serving build; `arda_version` stays the generator that wrote the world) and `versions`, the current version of every family (`VersionsDto`: contract, tactical, layout, scene, rules sidecar, bake, NPC, NPC page, settlement, realm, gazetteer, schema index, columnar layout, library catalogue, sheet mapping); `GET /v1/health` gains `server_version`. `index.ts` exports one constant per family. The schema index names each schema's `family` and `stability`. New constants `NPC_FORMAT`, `SETTLEMENT_FORMAT`, `REALM_FORMAT`; `SCENE_FORMAT` is now an alias of `TACTICAL_FORMAT`. All additive: no family bumps.
- **arda-server**: schema freeze guard (`src/schema/compat.rs`) against the committed baseline `bindings/schema-baseline/v1/`; re-baseline with `ARDA_BLESS_SCHEMA_BASELINE=1`.
- **docs**: `docs/capstone/versioning.md`, the route-by-route stability review (42 stable, 1 unstable, 6 debug) and the versioning and compatibility policy (families, open and closed enums, deprecation until both sides agree, output stability per release type, worlds, libraries, binaries). logic/16 brought up to date with the code (status, routes, cache budgets, errors without the never-emitted `stale_society`); API.md marks the unstable and debug surface and explains the three meanings of `format_version`.
- **Why**: from 1.0 DiceMaster codes against `/v1` and pins its output; a version it cannot see, a misleading `arda_version` or an unmarked debug route would cost a breaking change to fix later.
- **Tests**: `world_api/endpoints.rs` (`server_version`, `versions`), `bindings.rs` (`index.ts` constants), `schema::compat` (one synthetic pair per rule, family bumps, `$defs` ownership), `schema/tests.rs` (baseline guard, families, open enums listed in the policy), `world_api/inventory.rs` (every stable route of the policy answers). Golden responses recorded by 1.0 task 10 must include the new `WorldInfo` and `Health` fields.

## 2026-10-12 - Performance budgets as one release gate
- **tools**: `tools/release-gates.sh` builds the release tests once, runs every wall-clock gate with `--ignored --nocapture --test-threads=1`, parses one `GATE <name> <measured_ms> <budget_ms>` line per gate and writes a Markdown receipt (commit, date, `lscpu`/`free`/kernel, load average, table, result) to stdout and `target/release-gates/receipt-<commit>.md`. Exit 0 on pass, 1 on an over-budget, missing or failed gate, 2 on a build error. `--with-benches` adds the `arda-gen` benches; `--from-log` re-parses a log.
- **arda-server**: new `tests/world_api/gates.rs` with eight ignored gates (cached cell WebP tile, cold cell JSON plus scene, cold 32×32 window at 128 ppsq, `AppState::open`, `World::load`, area cells JSON and bin, warm overview tile), best of 3 through the real router on the settled society fixture. They print the `X-Arda-Cache` state.
- **arda-refine, arda-tactical, arda-midzoom, arda-npc, arda-names, arda, arda-gen**: every existing timing gate prints its `GATE` line before it asserts; `timing.rs` and `e2e_village` now use the `"wall-clock timing; release gate"` ignore reason. Names, budgets and response bytes are unchanged.
- **Budgets**: ratified as stated, none loosened. New: `AppState::open` < 2 s, `World::load` < 100 ms, area cells < 500 ms JSON and < 250 ms bin, warm overview tile < 20 ms. Measured 2026-10-13 at load 12-14: cached tile 0.8 ms, cell plus scene 159 ms, 128 ppsq window 502 ms, open 1.19 s, area 160 / 109 ms, overview 0.25 ms. `World::load` reads the manifest lazily (3 µs).
- **Docs**: logic/16 Invariant 7, OI-28 (browser and DiceMaster budgets stay open), 06-testing, 07-operations (release gates and the dated receipt), `API.md` timing table.
- **Why**: the budgets were scattered `#[ignore]` tests with their own commands, none recorded against a declared machine, and logic/16 promised server budgets nothing checked. DiceMaster sizes its prefetch and loading UX on these numbers.
- **Tests**: the gates are the tests. Each new gate was first run with a 0.001 ms budget and failed with its measured value; the parser fails a log with a missing `GATE` line (exit 1); `cargo test -p arda-server --test world_api` runs none of them.

## 2026-10-13 - Retire the sampled block archive
- **arda-gen**: `arda generate` no longer writes `blocks/<ax>_<ay>.tiles.zst`. Removed `SKELETON_BLOCK_STRIDE`, the `block` module (24-tile WFC filler, constraints) and the `seed` argument of `write_area`.
- **arda-core**: deleted `formats/blocks.rs` (`Block`, `BlockArchive`, `encode_blocks`, `decode_blocks`, `ZSTD_LEVEL`), `tiles.rs` (the 24-tile vocabulary), `SquareCoord` and `BLOCK_SQUARES`, and the `zstd` dependency.
- **arda**: removed `World::block`, its archive cache and `export_block`; `arda::Block` and `arda::TileId` re-exports are gone.
- **arda-render**: removed `symbolic.rs` (`render_block_png`, `SQUARE_PX`), `block_json` and `RenderError::UnmappedTile`.
- **arda-cli**: `arda export --block` is hidden and fails with: `--block was removed in 1.0; use arda-refine render --world <dir> --cell <gx>,<gy> or GET /v1/tactical/cell/{gx}/{gy}.png`.
- **World format**: `format_version` is unchanged. `world.json` never listed `blocks/` and `World::load` never required it, so older worlds that still contain the folder load and export unchanged; the folder is ignored.
- **Docs**: logic/03 marked retired; logic/04, 05, 09, README, 01, 02, 04, 05-dependencies, 06, 07, 08 and open-items updated; OI-43 closed.
- **Why**: nothing that serves maps read the archive. `arda-server` and the tactical pipeline build battle maps on demand from `arda-refine`/`arda-blocks`. The archive cost generation time, disk, a public API, a CLI flag and a tile vocabulary that contradicted the real one. 1.0 ships one tactical path.
- **Measurements** (MICRO seed 42, `--terrain fine`, release build, shared machine): the archive was 8 files, 396,310 bytes of a 143,296,267 byte world (0.28 percent); the world is 142,899,957 bytes without it. Wall time was too noisy to resolve (with: 76, 127, 111 s; without: 78, 57, 55 s, other jobs running); the filler is a few hundred WFC blocks per world, small next to terrain. Default-size estimate: about 0.3 percent of bytes, since the stride is fixed at 64 cells and scales with land area like everything else.
- **Tests**: the golden fingerprints lost only their `blocks/` lines (every other line, including the PNG hashes, is byte-identical); a fresh MICRO world has no `blocks/`; `--block` exits non-zero with the hint; an old world with a dummy `blocks/00_00.tiles.zst` still loads and exports an area. Tests of deleted code were removed with it.

## 2026-10-13 - Gazetteer: named seas, lakes, forests and ranges, and map labels
- **arda-names**: `PlaceKind::Sea` (keys `sea`, `gulf`), added last so every other kind keeps the number it seeds names with; bays, capes, islands, lakes and marshes are sometimes named for a founder, which gives them room for many names. Tests for natural-feature kinds (unique, deterministic, per preset), the seed numbers and the new key; README line.
- **arda-settle**: `gazetteer` module finds seas, bays, lakes, islands, capes, forests, marshes and mountain ranges on a 400 m coarse raster (components, exact distance transform, pole-of-inaccessibility anchors, centre-line label paths of at most 24 points), names them after regions so no earlier name changes, and writes `society/gazetteer.json`. `render::tilelabels` lays labels out once per zoom level and clips them to windows; the offline PNG now sets the same feature labels.
- **arda-people**: `SocietyFiles::gazetteer`.
- **arda-server**: `GET /v1/gazetteer[?kind=&bbox=&min_rank=]`; overview and relief tiles carry the labels by default, `?labels=0` leaves them off, cache keys include the flag; the oblique pyramid carries none. Schemas `Gazetteer`, `GazetteerFeature`, `GazetteerPoint`.
- **arda-midzoom**: `labels` module (labels on relief tiles).
- **Why**: a map reader and a game master expect "the Grey Sea" and "the Thornspine Mountains", and DiceMaster draws labels from Arda's JSON; closes the main part of OI-34.
- **Tests**: arda-names feature kinds; arda-settle made-world detection (two lakes, a bay, an island, a peninsula, a ridge), anchors and paths inside features, `names.json` bytes pinned to their pre-gazetteer values, tile-label seam and zoom tests; arda-midzoom seam test; arda-server `world_api/gazetteer.rs`.

## 2026-10-13 - POST /v1/tactical/bake
- **arda-server**: `POST /v1/tactical/bake` takes an edited cell layout (`BakeRequest {gx, gy, ppsq?, world_grade?, layout}`, exactly 64 × 64 at the cell's origin) and answers `202 BakeAccepted` with a content-addressed `job_id` (BLAKE3 of world seed, `library_version` and the canonical request; a re-post returns the same job, `200` once finished). `GET /bake/{id}` gives `BakeStatus`, `GET /bake/{id}/tiles/{z}/{x}/{y}.webp` the tiles (strong `ETag`, `Cache-Control: no-store`), `DELETE /bake/{id}` frees them (`204`). CORS allows `DELETE`.
- **arda-server**: `tactical/splice.rs` pastes the posted cell into the cell's apron block (squares inside from the post; walls by square, placements and lights by position strictly inside the cell from the post; the rest from the apron; item order kept), so edges join unbaked neighbours and an unedited layout bakes to the cell route's tiles byte for byte.
- **arda-server**: the bake lane (`tactical/bake.rs`, `bake_store.rs`, `bake_render.rs`, `bake_routes.rs`): at most 8 waiting jobs (503 `busy`, new `ServerError::Busy`), one worker (`--bake-workers`, default 1; 0 answers 501 via new `ServerError::Disabled`), its own render lock in `raw.rs`, no shared-cache reads or writes, tile encodes through the shared encode slot one tile at a time. Finished sets are held within `--bake-result-mib` (default 384), oldest evicted first. Admission adds one transient render with its pyramid, queued bodies and the result budget (default config 13.86 → 15.94 GiB of the 16 GiB ceiling) and refuses a budget below one worst-case set.
- **arda-server**: `API.md` § `POST /render` limits corrected to the code (41 616 squares, 96 Mi pixels). Bindings and schemas for `BakeRequest`, `BakeState`, `BakeAccepted`, `BakeStatus`; `ApiErrorBody` lists `busy`.
- **Docs**: `API.md` bake section and errors, logic/16 (routes, §api-tactical bake, §api-cache, §api-prefetch bake lane, §api-errors `busy`), 09-interfaces, 02-models, 07-operations; OI-44 removed.
- **Measured**: MICRO seed 42, river cell (529, 812) at 128 ppsq with a 16 × 16 ground edit: 341 tiles, 150 673 194 bytes, baked in 3.6 s on a release build.
- **Why**: DiceMaster bakes lasting campaign changes into real art once and serves them from its own storage; Arda stays stateless and keeps nothing.
- **Tests**: `tests/world_api/bake.rs` (unedited bake equals the cell tiles, caches untouched, two fresh servers bake identical bytes; an edited 4 × 4 patch changes its tile but not the edge tiles; a cell request completes while a bake holds its lane; idempotent ids, delete, 404, 503 `busy`, 415, 422 size/origin/asset, 404 `no_block`, 501 when off; the cell layout splices into its apron unchanged on three cells); unit tests for `splice`, store eviction order, job ids, tile counts and admission.

## 2026-10-12 - release automation
- **Workflow**: `.github/workflows/release.yml` runs on `v*` tags and manual dispatch. Jobs: `verify` (tag equals `Cargo.toml` version, `tools/release.sh --check`, a changelog section, fmt, clippy, `cargo test --workspace --locked`), `binaries` (four targets, smoke-tested, archived, checksummed), `image` (amd64 smoke through `tools/docker-smoke.sh`, then a `linux/amd64,linux/arm64` build with SBOM and max provenance; pushed to `ghcr.io/dicemasterio/arda` only on a real tag), `release` (a draft GitHub release with `SHA256SUMS` and notes from the changelog). Manual dispatch is a dry run that never pushes or releases.
- **Tools**: `tools/release.sh <version>` bumps every version site, runs `cargo update -w`, re-blesses the micro-42 goldens and checks the diff; `--check` verifies 50 version sites; `--notes` extracts a changelog section. `tools/package-release.sh` and `tools/smoke-binaries.sh` are the packaging and smoke steps the workflow calls.
- **Metadata**: workspace `repository` is now `https://github.com/DiceMasterIO/arda`; every crate sets `publish = false` (D5: no crates.io for 1.0).
- **Pins**: every action in `ci.yml` and `release.yml` is pinned to a full commit SHA; the release toolchain is Rust 1.98.1, matching the Dockerfile.
- **Image content**: the published image carries the placeholder tactical library only (D1). The AI library path (a pinned release asset with a committed sha256) is recorded in 07-operations but not built.
- **Docs**: 07-operations Release process and CI row, 06-testing CI matrix, 05-dependencies, OI-30, README install section.
- **Why**: DiceMaster pins `(seed, arda_version)` and pulls `ghcr.io/dicemasterio/arda:<arda_version>`, so 1.0.0 needs a repeatable path from tag to checksummed binaries and a verified image, with publication left to the maintainer.
- **Tests**: `actionlint` clean on both workflows; `shellcheck` clean on the three scripts; `tools/release.sh 9.9.9` in a throwaway clone leaves a consistent tree; `--check` passes on the real tree and fails after a hand edit; release-mode build and `tools/smoke-binaries.sh --serve` pass locally. A real tag run is untested.

## 2026-10-13 - POST /v1/npc/generate
- **arda-npc**: `generate_one(world_seed, &OneRequest) -> Npc` in `src/one/` builds a 40-person synthetic settlement with the reserved id 0, laid out so the requested job exists (dwellings, two copies of the job's workplace, or the land-job table narrowed to that job), and returns one inhabitant matching every filter (job, office, rank, ancestry, sex, notable), up to 16 keyed attempts. Relationships, employer and workplace are cleared and the id is re-keyed from the request. New `NpcError::UnknownKey`, `Unreachable` and `Unsatisfiable`. The only plan change is the additive `Plan::build_with(.., land_job)`; real settlements pass `None` and their populations are byte-identical.
- **arda-server**: `POST /v1/npc/generate` (`src/npc.rs`): `GenerateNpcRequest` with Arda's own optional inputs (`seed`, `job`, `office`, `rank`, `ancestry`, `sex`, `tier`, `culture`, `settlement_id`, `notable`), `settlement_id` resolved through `society/` (404 when unknown or absent), body ≤ 16 KiB, JSON only (415), response through `npc_response(.., NpcAt::Root)` so `--sheet-mapping` applies. New error code 422 `unsatisfiable`. TS mirror and schema `GenerateNpcRequest`; `Npc` lists the route.
- **Docs**: `API.md` section with body table, errors, synthetic ids and the body decision; logic/13 §npc-generate-one; logic/16 routes and errors; 09-interfaces; 02-models; OI-40 closed.

**Why**: a game master needs "a dwarf smith" or "a gentry priest" on the spot, and DiceMaster wants NPCs that match Arda's people without describing a settlement and its buildings.

**Tests**: `crates/arda-npc/tests/generate_one.rs` (every job and ancestry, every office and sex, rank per tier, filters, identical bytes, salts differ, refusals), `allow_list.rs` scans generated office holders, existing populations unchanged (determinism, regeneration, golden hash of the sample settlements before and after); `tests/world_api/npc_generate.rs` (library equality, filters, borrowed settlement, 404s, 400/413/415/422, synthetic id 404, identity sheet mapping) and `generated_npc_bodies_validate` in `schemas.rs`; the mirror round-trip in `npc_dto/tests.rs`.

## 2026-10-13 - Terrain findings re-measured on recipes 7 and 8
- **Request**: close or reopen OI-02 to OI-05 with measurements of the old findings (parallel ravines, angular coasts, rectangular basins) on the current stream-power formation. Measurement only; no generation code changed, golden tests untouched.
- **Tool**: `crates/arda/examples/terrain_findings` (with `run.sh`) prints deterministic JSON for one saved world: steepest-descent and reach direction histograms with a 17° rotated control, exact straight channel runs and incision troughs, 10 km straight-reach probes, 2D-FFT valley spacing over highland windows, sea and lake boundary chord and turning histograms with a rotated control, stair-step runs, box-counting dimension, and lake shape (oriented-rectangle fill, perimeter ratio, area-line share, level versus rim). Synthetic controls (lattice, rectangle lake, axis and 45° coasts flagged; white and fractal noise, circle lake, fractal coast not) are unit tests.
- **Panel**: seed 42 at 500×1000 km (recipe 7 and 8), seed 42 and 436342 at 200×300 km (recipes 7, 8), seed 42 dry latitudes 15 to 35 at 200×300 km (recipe 7), MICRO seeds 42 and 99 (recipes 7, 8). Seed 436342 at the default size was skipped (about 80 minutes per world). Regenerating micro42-r7 and micro99-r8 reproduced whole-tree hashes exactly.
- **Result**: OI-02 closed (baseline delivered). OI-04 closed: coast axis+45 chord ratio 0.90 to 1.10 at 5 km (threshold 1.4), rotated control agrees; 100 m stairs remain visible only in plain area views. OI-05 closed: the default seed-42 world has no lake above 14 km²; no panel world has one of 500 km². OI-03 narrowed and kept open: no lattice, no exact run of 20 km (longest 76 cells), no grid-bound descent field, but 10 km near-straight reaches lean to the axes and diagonals (excess 1.2 to 1.7 at 600 m tolerance) with the cause not isolated.
- **Not measured**: tectonic belt edges, bay-spacing regularity, a routing control for near-straight reaches. The 2026-09-25 numbers (trough 203.8 to 83.1 m, 30,709.15 km² lake) could not be recomputed because their worlds and scripts were removed.
- **Evidence**: local only, `docs/capstone/features/2026-10-terrain-recheck/` (gitignored) and `out/recheck/`.

## 2026-10-12 - Coast and wetland calibration
- **Request**: calibrate marsh, pools, beaches and the sea-level strip of tactical ground against measured world statistics (1.0 task 18, OI-41, OI-16).
- **Tool**: `arda-refine coastwet --world <dir> [--stride N]` prints deterministic JSON: signal shares by climate group, prior ground shares, pools per km2 of wet cells, and coast strip width per saved shore class. Baseline and retuned numbers for MICRO seed 42 and the same seed at latitudes 15-35 are in `logic/09-tactical-refinement.md` §biome-calibration.
- **Signals**: marsh wetness band 0.8-0.98 is now 0.5-0.7 (lowland marshy share 1.9 % to 4.5 %), and marsh cover obeys the 12 degree slope limit. In a world with `terrain/shore.bin` the coast signal reads the distance to the sea (4-cell reach) and the saved shore class: sand beaches are 3 cells deep over firm ground (89-92 % sand on beach cells, was 0 %), shingle and salt marsh 2 cells with their own ground, cliffs and rocky shores bare rock and no sand. Worlds without the layer keep the old height rule and weights.
- **Source**: `Source::shore` and `Source::has_shore` are new required methods; `GridSource::set_shore` builds fixtures.
- **Fixtures**: `tests/biome.rs` damp-meadow wetness 150 became 110 (150 is now partly marsh by design); the synthetic world's river row (wetness 200) now reads as marshy.
- **Not changed**: shore formation and cover marsh (recipe-gated). The sand beach share of the coast (5-6 % against about a third) is a recipe-9 candidate, recorded in OI-41. No golden changed.

## 2026-10-12 - Light pools stop at walls
- **Request**: OI-39, torchlight and lamp glow shone through walls onto rock and neighbouring rooms in the composited PNG.
- **Change**: `crates/arda-tactical/src/compose/light_los.rs` computes a per-pool visibility mask in layout coordinates (8 samples per square, integer segment-versus-edge test, scene vertex rule, two blurs) and `add_pool_row` scales the pool by it. Pools that reach no blocking edge are byte-identical to before.
- **Shared rule**: `WallSegment::blocks_light(lib)` is used by the compositor and by `arda-scene`; windows pass light, closed doors, gates and secret doors block. Walls only; props and canopies do not occlude.
- **Render identity**: lit maps with walls near lights render differently; no cache key changed (server caches are in memory).
- **Docs**: logic/11 §lighting; OI-39 removed; the torchlight limit removed from `crates/arda-dungeon/README.md`.
- **Tests**: `compose/tests_light.rs` (wall, window, door and gate, light on an edge, open-ground golden, determinism, clip invariance) and `crates/arda-scene/tests/light_parity.rs`.

## 2026-10-12 - Richer world history
- **Request**: give generated history more templates per event kind, named events with backstories and causal chains (1.0 task 16, OI-35).
- **Templates**: every event kind has at least 4 text templates and every titled kind at least 3; lore families have at least 4; 12 war-name patterns, 18 plague names, 7 ruin kinds per tier. A linter in `Tables::check` refuses unknown slots, empty families and unused keys.
- **Events**: new optional `name`, `backstory` and `cause_kind` fields on `Event` (additive, `format_version` unchanged). Plagues, wars, famines, severe floods and fires and dynasty changes are named uniquely; plagues, famines, floods, fires, mine collapses and wars say where they came from.
- **Causes**: planned famines and plagues move to just after a flood, war or famine of their realm (flood or war -> famine -> plague -> abandonment); borders and present realms are unchanged. Settlement stories, region hooks and ruin lore use the names and backstories.
- **Bytes**: output differs from before in event text, titles and the new fields, and in the years of famines and plagues that were moved into a chain.

## 2026-10-12 - arda-server container image
- **Docker**: the `Dockerfile` builds `arda` and `arda-server` from a pinned `rust:1.98.1-bookworm` with `RUSTUP_TOOLCHAIN` set and BuildKit cache mounts, into a digest-pinned `debian:bookworm-slim` as user/group 10001. It adds `EXPOSE 8787`, a `HEALTHCHECK`, OCI labels, `WORKDIR /opt/arda` and a `.dockerignore` (the build context drops from several GB to a few MB). Entry `arda-entrypoint` (`tools/docker-entrypoint.sh`) serves `/worlds/world` on `0.0.0.0:8787`; `--entrypoint arda` runs the CLI.
- **Library variants**: the placeholder library ships always; the AI library enters only through the optional BuildKit context `artlib` and is stacked over the placeholder. Maintainer decision D1: the published image is the placeholder variant only.
- **arda-server**: SIGTERM now shuts the server down like Ctrl-C (Unix); new `--probe` flag backed by a std-only `probe` module for the healthcheck.
- **Tooling**: `tools/docker-smoke.sh` builds either variant and checks health, world, library, a tactical PNG, uid 10001, the CLI entry and a sub-3 s `docker stop`.
- **Docs**: README Docker section (the stale `arda serve` line is gone), 07-operations Processes, flags and Infrastructure rows, API.md, OI-30.
- **Why**: nobody could deploy the HTTP service from the image, and `docker stop` hung ten seconds on SIGTERM.
- **Tests**: probe unit tests (200, non-200, refused, silent close); `tests/serve.rs` shutdown-future test; the smoke script for both variants.
- **Open**: the `commercial_licence` validator rule from logic/11 is not implemented, so `LicenseRef-AI-generated` is not gated.

## 2026-10-12 - GET /v1/realms
- **arda-settle**: `realms::Realm` derives `JsonSchema` under the `schema` feature (id fields as strings).
- **arda-society**: schema derives on `RealmState`, `RealmSociety`, `Ruler`, `Vassal`, `Relation`, `Stance`, `Event`, `EventKind` and `Reign`; attribute lines only.
- **arda-people**: `SocietyFiles.realms: Option<RealmsFile>` (`None` when `realms.json` is absent) and `SocietyFiles::realm(id)`.
- **arda-server**: `GET /v1/realms` (summaries without borders, plus style, rank, government and ruler when `society.json` exists) and `GET /v1/realms/{id}` (record with borders, politics, relations, events, reigns), in `crates/arda-server/src/realms.rs`; schemas `Realm`, `RealmList`, `RealmDetail`, `RealmSociety`, `Relation`, `Event`.
- **Docs**: `API.md` Realms subsection, logic/16 routes, 09-interfaces, 02-models; OI-46 closed.

**Why**: DiceMaster saw only bare `realm_id` strings and could not label settlements or show who rules a realm.

**Tests**: `tests/world_api/realms.rs` (list vs `realms.json`, politics vs realms, detail of every realm, 404 and 400, identical bytes, no-society 404), `realm_bodies_validate` in `schemas.rs`, `arda-people` files unit tests.

## 2026-10-12 - Viewer npm licence policy, CI job and bundle notices
- **Viewer**: `apps/viewer/licence-policy.json` (runtime and build-only allow-lists, reasoned exceptions), `scripts/check-licences.mjs` (stdlib only, reads the lockfile, deterministic sorted output) and `npm run licences`. The Vite plugin `scripts/licence-notices.mjs` writes `THIRD_PARTY_LICENSES.txt` into `dist/` with Leaflet's BSD-2-Clause notice and the MIT texts for React, React DOM and Scheduler; the build fails if a runtime package has no licence file. `npm test` now also runs `node --test scripts/`.
- **CI**: new `viewer` job in `.github/workflows/ci.yml` (Node 26, SHA-pinned `actions/setup-node`): `npm ci`, licences, typecheck, lint, test, build. `actionlint` clean.
- **deny.toml**: BSD-2-Clause and ISC added to the Rust allow-list so one rule covers the repo (maintainer decision D7); no crate uses them today.
- **Docs**: 05-dependencies "npm license policy", standards vetting bar, 06-testing CI matrix, 07-operations developer workflow, viewer README scripts.
- **Why**: `deny.toml` covered Rust only and the viewer's BSD-2-Clause dependency requires its notice in redistributed bundles; Arda ships in a commercial product.
- **Tests**: six `node --test` cases on fixture lockfiles (runtime GPL fails, dev MPL passes, runtime MPL fails, missing licence fails, `OR`/`AND`, exception, sorting); real `npm ci`, `licences`, `typecheck`, `lint`, `test` (86 Vitest) and `build` green. `cargo deny` was not run (not installed).

## 2026-10-12 - Art generation spend caps and no double billing
- **Caps**: `tools/art-gen/generate.mjs` takes `--max-images N` and `--max-spend USD` (default `$ART_GEN_MAX_SPEND`). A run whose worst case (images x price x (1 + reviewer remakes)) exceeds `--max-spend` is refused with exit 2 before any request, also under `--dry-run`. `--yes` with an unknown price needs `--max-images` or `--price-per-image`. The summary prints the worst case.
- **Run-time guard**: every submission (new project, re-sent batch, re-posted failed or cancelled item) counts; the run stops cleanly at the cap with state and `import.toml` written and exit 3.
- **Dropped POST**: parallel mode writes a `posting` intent before each `POST /api/projects`; on a network error it reads `GET /api/projects` and adopts a project created meanwhile instead of posting again, retries once only when the list shows none, and exits 1 when the list is unreadable. Leftover intents are reconciled at the next start.
- **Code**: new `tools/art-gen/safety.mjs`; the stand-in server moved to `tools/art-gen/test/fake-slopify.mjs` with project listing and connection-drop switches; `tools/art-gen/test/safety.test.mjs` adds the cap and drop cases.
- **Docs**: `logic/20-art-generation.md` (guard, estimate, parallel, state, exit, unhappy paths, invariants), `tools/art-gen/README.md` (Spending safety), `tools/art-gen/plan.md`.

## 2026-10-10 - Server schema gaps: dungeon and index schemas, string dungeon seed, window tiles, `{building}`
- **arda-dungeon:** `Params::seed` serialises as a decimal string through `arda_scene::serde_str` and reads a string or a number (I5, I17; 16 §api-conventions). Affects `GET /v1/tactical/dungeon` `params.seed` and the CLI's `--meta` file; layouts, rules, scenes and renders are unchanged.
- **arda-server, dungeon:** new wire mirror `DungeonDto` (`tactical/dungeon_dto.rs`: `DungeonParamsDto`, `DungeonKindDto`, `RoomPurposeDto`, `DungeonRoomDto`, `DungeonDoorDto`, `DungeonExitKindDto`, `DungeonExitDto`; layout and rules reuse `TacticalLayoutDto` and `RulesSidecarDto`), exported to `bindings/ts/arda/` and published as `bindings/schema/DungeonDto.json` for `GET /v1/tactical/dungeon`. The catalogue now lists `GET /v1/tactical/dungeon/scene` under `SceneDto`, and `DungeonDto.layout` / `DungeonDto.rules` under their schemas. The handler still serialises the domain `Dungeon`.
- **arda-server, schema index:** `GET /v1/schema` has a type of its own, `SchemaIndex` with `SchemaIndexEntry` items (TS binding and `SchemaIndex.json`, listed for `/v1/schema`). `index()` builds the body from it; `index.json` gains only the new entries.
- **arda-server, window tiles:** new route `GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp?<window query>`, the pyramid the window body's `tiles` already described but nothing served. It reuses `window_request` and `world_tile_look`, so tiles are cut from the same render key as `/window.png` (default `ppsq` 32, same limits). Additive: no `tactical_format` bump (16 §api-versioning).
- **arda-server, building routes:** the axum placeholder of `/v1/buildings/{…}/residents|workers` is renamed from `{id}` to `{building}`, matching API.md and the schema index; handlers read the segment by position, so the wire is unchanged.
- **Docs:** `crates/arda-server/API.md` (dungeon `params`, window tiles, schema table and index type), logic/16 (routes, §api-tiles, §api-schema), logic/18 §dgn-serve (seed finding resolved), 09-interfaces (`{building}` in edges, table and headings; `schema:` names for `/v1/schema`, `/dungeon`, `/dungeon/scene`; window tile route), 02-models (the ten new DTOs), 01-architecture and 04-data-flow (`{building}`).
- **Why:** found while documenting the API for a consumer: the dungeon body had no schema or binding and broke the "every u64 is a decimal string" rule, the schema index was the one body without a schema, the window body advertised a tile pyramid no route served, and the building path parameter had two names.
- **Tests:** `dungeons_and_caves_round_trip_through_the_mirror` (domain JSON equals mirror JSON for both kinds and three seeds, including `u64::MAX - 7`); `params_seed_is_a_decimal_string` (arda-dungeon); `the_index_reads_as_schema_index_and_lists_itself`, `the_dungeon_seed_is_a_string` (schema); world tests validate `/v1/schema` against `SchemaIndex`, two dungeon bodies against `DungeonDto`, `/dungeon/scene` against `SceneDto`, a window body against `TacticalBlockDto`, assert `params.seed == "9"`, and `a_window_serves_the_tile_pyramid_its_body_describes` (z 0 and the deepest corner tile 200, outside 404, `.png` 404, bad query 400). Bindings and schemas re-blessed.

## 2026-10-10 - open-items: open work only
key: open-items/prune@0b02525

- `open-items.md`: removed finished items OI-01, OI-07–OI-11, OI-13, OI-15, OI-17–OI-27, the milestone log, implemented-work table, verification receipts, delivery-order table, coverage table and historical inventories (1,247 → ~350 lines).
- Kept: current state table, remaining scope, OI-02–06, 12, 14, 16, 28–31, OI-32–46 and scope boundaries.
- OI-21, 22, 23, 26 and 27 removed as done on Arda's side; their remainders are OI-39, OI-42, OI-44 and OI-45 or belong to DiceMaster.
- `00-index.md`: open-items row describes the open-only scope.

## 2026-10-10 - open-items: DiceMaster map changes
key: open-items/dicemaster-map-changes@59edbb1

- `open-items.md`: OI-44 (background bake route returning tiles, uncached), OI-45 (effect textures, damage decals and props), OI-46 (realm names over HTTP).
- Decision: Arda's map is the same for every campaign; campaign changes are DiceMaster records merged over Arda's layers and drawn by DiceMaster.
- Decision: lasting changes are baked once in the background and stored by the client; `POST /v1/tactical/render` is not used for per-campaign maps.

## 2026-10-10 - map: all
key: map/all@022ce7cb5707

- `01-architecture.md`: regenerated (covered paths changed); all 23 crates in tiers 0–7, 37 server routes, viewer and art-gen.
- `02-models.md`: regenerated (covered paths changed); 624 entities across every crate, server DTOs and persistence formats.
- `03-conventions.md`: regenerated (covered paths changed); determinism/hashing schemes, rule citations, data files, fixtures.
- `04-data-flow.md`: regenerated (covered paths changed); full pipeline generate → settle → society build → server request flows; persisted vs computed table.
- `05-dependencies.md`: regenerated (covered paths changed); records `ab_glyph`, `crc32fast`, `flate2`, `schemars`, the axum server stack and viewer npm deps; `tiny_http` recorded as never installed.
- `06-testing.md`: regenerated (covered paths changed); per-crate test inventory, fixtures, ignored release gates, bindings checks.
- `07-operations.md`: regenerated (covered paths changed); six binaries, viewer, art-gen, env vars, CI, release.
- `08-glossary.md`: regenerated (covered paths changed); ~110 terms.
- `09-interfaces.md`: created (newly applicable topic: arda-server publishes an HTTP API consumed by DiceMaster); 40 http rows and 2 schema rows, no `to`/`from`.
- `open-items.md`: refreshed in place (covered paths changed); OI-01..31 audited at 022ce7cb5707, OI-32..43 added.
- `logic/07-preview.md`: regenerated (covered paths changed).
- `logic/17-midzoom-relief.md`: rewritten to the scenario template (template drift).
- `logic/18-dungeons-and-caves.md`, `logic/19-art-import.md`, `logic/20-art-generation.md`, `logic/21-society-build.md`, `logic/22-developer-inspection.md`: created (logic coverage gap).
- `logic/README.md`: index rows for 17–22.
- `logic/14-society-history.md`: trigger names `arda society build` (was the non-existent `arda-society generate`).
- `logic/16-service-api.md`: `contract_version` row records contract 3.
- `uiux/01-direction.md`, `uiux/02-system.md`, `uiux/screens/01-world.md` … `05-editor.md`: created (design coverage; `apps/viewer` is a frontend).
- `contract-dicemaster.md` (local only, gitignored): created; Arda ↔ DiceMaster semantics, statuses (implemented/planned/open), route map from DiceMaster's assumed paths.
- Interfaces: quarry is installed but not initialised in this repo (`quarry docs index` fails); edges left without `to`/`from`.
- Skipped as current: `logic/03`, `logic/06`, `standards.md`, `implementation.md`, `mockup-artifact.md`, `mockup/*`.
- Left unfolded: `changelog.d/2026-09-25-terrain-study-consolidation.md` (untracked, not this run's).
- Index refreshed.

## 2026-10-09 - Terraced houses read as separate houses: party walls, materials from history, ambitus gaps
- `crates/arda-tactical`: new edge tags `partition` and `party` (`layout::TAG_PARTITION`, `TAG_PARTY`, `WallSegment::is_partition`). The compositor draws a `partition` edge with the library's `<kit>_partition` piece when it has one, else the kit's own piece squeezed to half thickness (`compose::PARTITION_PCT`); a vertex where a partition meets an outer or party wall takes its joint from the outer arms only, so the shell runs on unbroken; all-partition vertices get a thin joint. `party` walls draw as outer walls. Rules are unchanged: every wall blocks. `Rgba::inset` added.
- `crates/arda-town` blocks: an edge on a footprint boundary is a building's shell; one that two buildings claim is tagged `party`; an edge inside one footprint is tagged `partition` (`block::WallClass`, `block::on_boundary`). logic/10 §party-walls.
- `crates/arda-town` materials: `block::kits` is now a data table driven by the settlement (logic/10 §building-materials): culture and biome give the vernacular (timber, wattle, log in sylvan woodland and the boreal forest, adobe on the steppe and in the warm south, rubble and stone in the uplands); district rank plus wealth, tier and building stone at hand, a golden age (core) and great fires (inside the wall) pick stone, a stone/vernacular mix, the middling or the poor vernacular; a house's standing relative to its own settlement moves it one step. Rich merchant towns and cities (wealth ≥ 220, market or port) are stone throughout, with rubble for low-standing houses. New kits used: `log`, `adobe`, `drystone` for rubble houses (`fallback::kit_chain` maps `log` → timber and `adobe` → stone for libraries without them). Floors follow material and wealth, and homes sharing a wall take different floors where the palette allows (greedy in id order). `TownPlan` gains `fabric` (serialised; the plan schema changes) and a cached `looks`.
- `crates/arda-people`: `World::plan` reads great fires and the prosperity history from `society.json` into the plan's fabric (`town::read_history`), after planning, so the layout and society's buildings are unchanged.
- `crates/arda-town` plans: a deterministic share of street-front row plots keeps a one-square ambitus (eaves-drip gap) in its last column: 25 % in towns, 15 % in cities, none on the market square (`Params::ambitus`, hashed from the plot's first front square; the random stream, front line and depth are untouched, so only those houses lose one column).
- `tools/art-gen`: tier-5 prompt entries for `timber_partition`, `wattle_partition` and `adobe_partition` (57 images with alts; not generated).
- Why: in towns and cities houses are terraced rows sharing party walls; every house used the same kit, partitions looked exactly like party walls and floors gave no cue, so a row read as one big building with many rooms.
- After (MICRO 42): Eayeyil (city, 254, port and market) is stone throughout; Dilrou (heartland town, 183) has a stone market and temple close, mixed craft and waterfront, timber residential streets, wattle suburbs; Vefleth (sylvan town) builds logs; Fetarmedhe (southern, two great fires) builds stone; Tudeeyude (poor sylvan forest village) builds wattle with log houses for the better off. Renders change wherever a town or village shows: wall kits, floors, thin partitions, and about a quarter of town row houses one square narrower.
- Tests: `arda-tactical` (joint selection with tagged partitions, partition thickness, dedicated partition art), `arda-town` (`block::kits` rules, `tests/blocks.rs` party/partition classification, `tests/plan.rs` ambitus share), `arda-blocks/tests/settlements.rs` (tags agree with the scene's building ids on seed 42, fires read from society, rich merchant towns all stone, others split). Schema `TownPlan` re-blessed. Docs: logic/10 §town-footprints, §town-walls, new §party-walls and §building-materials, §town-interiors, invariants 10–12; logic/11 §vocabulary, §walls-assembly, invariant 3.

## 2026-10-09 - Notable ranks by tier and office; one civic figure per hamlet
- `crates/arda-npc/data/content/occupations.json`: jobs carry `rank_by_tier`. Workshop, inn and boathouse owners are tradesfolk in hamlets and villages and masters in towns and cities; tavern and shrine keepers, headmen, elders and reeves are tradesfolk; a village lord of the manor is gentry, a lord mayor gentry. New `offices` table ranks society offices (`elder` and `reeve` 2, `commons_voice`, `fisher_head`, `forester`, `miner_captain` 1, `lord` 4–5, `ruler` 5, …), marks the civic ones and lists the tier officials each covers. New `rank_lifestyles` keeps a working NPC's wealth inside its rank's lifestyle band.
- `crates/arda-npc`: rank = job rank in the tier raised to the held office's rank (`ranks` module, public). Society slots now bind before officials are appointed; an office stands in for the official it covers (no second "Reeve" beside "Reeve of X"), officials and promotions stop at the tier's `civic_cap` (hamlet 1, village 3, town 8, city 16), office holders are never demoted, and the hamlet promotion list keeps the head's own land job (`land`). "Master …" titles are kept only at master rank (a village "Smith").
- `crates/arda-society`: no `harbourmaster` office in hamlets; the hamlet commons faction is led by the elder (`leader_hamlet`); an office without a building of its kind sits in a dwelling instead of the first building (a "Chief Woodward" was bound to the harbourmaster or a stallholder). `society.json` changes accordingly (hamlet factions and roles, fallback role buildings).
- `crates/arda-people`: `compute` returns a build without writing it (`build` = `compute` + write).
- Why: on MICRO 42, 70 % of the 3,362 notables were `master` (every workplace owner and official was rank 3), 30 % were government, every hamlet had 3–4 civic figures (society elder and voice plus a promoted headman and elder), and office holders kept their job's rank, so "Elder of X" who farmed was a labourer while a plain "Reeve" was a master.
- After (MICRO 42, 2,533 notables): tradesfolk 53 %, master 25 %, labourer 17 %, gentry 4 %, noble 1 %; government 7 %; every hamlet has exactly one civic figure; hamlet notables are 93 % land, water and woodland folk.
- Tests: `arda-npc/tests/ranks.rs` (job/office → rank, lifestyle band, "Master" titles, hamlet civic cap, office covers official), `arda-people/tests/notable_mix.rs` (MICRO distributions and the rank invariant). Docs: logic/13 §npc-jobs, §npc-notables, new §npc-ranks; logic/14 §soc-offices.

## 2026-10-07 - Towns build over the shallow shore fringe
- `crates/arda-blocks/src/overlays.rs`: in its own footprint the town layer (buildings and crofts) may claim base squares holding at most 1 ft of standing water (`MADE_GROUND_FT`), as quays and made ground. Deeper water and the town plan's own water stay water. Fields and ways still never take water.
- Why: the biome pass lets the sea reach into sea-level coast cells. In a port at sea level (MICRO 42, Dilrou 309,1096) that 1-ft fringe covered 42% of the town's open ground, because a layer could only take a wet base square by drawing water itself. Refine cannot avoid this, since world cells carry no settlement data until `arda settle`.

## 2026-10-07 - Tactical biomes: marsh, coast, alpine, crags and steppe from the cell fields; no texture checker
- **Problem:** on MICRO seed 42 (cover is `grass` on 99.7 % of land) every refined cell looked like the same meadow: a wetland (wetness 1.0) was grass and a stream, a coast cell (0 m, sea next door) was grass with no beach or sea, a snowbound peak (3,150 m) was snow strewn with flowers and bushes, a 35° mountainside was a flat scree sheet, and steppe (400 mm at 12.8 °C) was lush green. AI ground textures showed a blocky checker where variants met.
- **Biome signals (arda-refine `biome.rs`):** marshy, arid, coastal, alpine and craggy, each from one cell's wetness, rainfall against warmth, height, temperature and slope (logic/09 §ground-field). They drive the ground prior, marsh pools, the standing-water shoreline (sea-level land is half sea) and the scatter (logic/09 §scatter): reeds, sedge and marsh flowers; driftwood, dune grass, tide pools and sea rocks; no plants on snow, krummholz, alpine flowers and lichen rock; dry grass, tussocks and sagebrush. Craggy cells are terraced into 7 m strata (logic/09 §elevation), so mountains show benches, cliff bands and outcrops.
- **Forests:** a forest cell at density 0.84 (legacy MICRO 42, cell 462,765, after the ecology pass) drew about 200 trees spaced like parkland, 13 % crown cover. Trees now stand on a 1.6-square lattice with overlapping crowns, so crown cover tracks the forest density (0.80 there; 0.52 at density 0.45 with copses), with undergrowth in the gaps. Aridity ramps from 0.75 to 0.35 of the water need so temperate 570 mm forest country is not read as steppe.
- **Compositor (arda-tactical):** natural texture variants are flattened to one tone (no checker), one variant per 3 × 3 cells; an optional `Square.dryness` (refined layouts carry aridity) tints turf toward dry grass, seam-exact (logic/11 §ground-blend). `bindings/` gain `SquareDto.dryness`.
- **Crisp parcel edges:** farmland, stubble and fallow borders (against headlands, hedges, grass) are drawn as clean lines (12 % warp, `w¹²` sharpening) instead of half-square blobs; natural borders stay organic (logic/11 §ground-blend).
- **Placeholder library:** 13 new vegetation ids with existing looks; existing assets unchanged.
- **Compatibility:** built-in layouts carry no dryness, but every placeholder render changes slightly through the flattened variants (intended; mean per-channel difference 2–10 levels). Refined blocks change wherever the new signals are non-zero. No golden fingerprints changed.
- **Renders:** before/after sheet of seven MICRO 42 cells and two legacy MICRO 42 woods (forest 462,765; woodland 470,645) at 64 px per square (wetland 344,680; coast 56,1592; peak 632,1328; mountain 680,1400; steppe pasture 848,1328; steppe river 872,1184; wet river 80,1280).

## 2026-10-07 - Legacy terrain gets ground ecology: forests on the default world
- **Symptom:** in MICRO seed 42 generated with the CLI default (`--terrain legacy`), the cell contract reported `cover = "grass"` on 99.8% of land, marsh on the rest, and `forest_density = 0` and `moisture = 0` on every land cell. Settlements classified as steppe/wetland and tactical maps never placed trees.
- **Not the climate:** the same world is temperate and wet. Land rainfall median 853 mm (58.6% above 600 mm), temperature median 10.7 °C; 49.8% of land is both above 5 °C and above 600 mm. MICRO seeds 1, 7, 99 and 2024 matched it: rainfall medians 583–1,278 mm, 34–52% of land temperate and above 600 mm, and still zero canopy and moisture everywhere. The fine path on the same seeds is forested (fine 42: 67% forest cover, mean density 0.66).
- **Cause:** the ground-material pass (`arda-gen/src/orchestrator/fine_materials.rs`, logic/02 §Ground material) ran only for fine recipe ≥ 4. Legacy composition sets grass or marsh and leaves the ecology fields at their zero defaults, which logic/02 described as "richer ecology deferred". The pass reads only saved temperature, rainfall, slope, aspect and drainage, all of which legacy cells already carry, so nothing tied it to the terrain source.
- **Fix:** run the pass for legacy terrain too. Fine recipes 1–3 still replay their published bytes. Fine recipes 4–8 are unchanged: `tests/golden_recipes.rs` stays byte-identical.
- **Result (legacy MICRO, before → after):** forest cover 0% → 61% (seed 42), 56% (1), 58% (7), 41% (99), 53% (2024). Mean forest density 0 → 0.55–0.64. Where land is above 5 °C and above 600 mm, 64–82% of it is forest. Climate fields are byte-identical; only moisture, wetness, canopy and cover change.
- **Golden:** `tests/golden/micro-42.txt` re-blessed on purpose. The six land-bearing `areas/*/cells.bin` change because of the new ecology fields. The two all-sea areas, blocks, continent and hydrology are unchanged. `world.json` changes only because the fixture still carried the v0.6.0 `arda_version`: the v0.7.0 release did not re-bless it.
- **Tactical:** in the new seed-42 world, cell (462, 765) has 9.9 °C, 673 mm, forest density 0.84 and cover forest. It renders 198 trees (birch, oak, elm) with ferns, logs and stumps through `arda-server --library out/art-gen/lib`.

## 2026-10-07 - Dungeons and caves: arda-dungeon, `arda dungeon`, `/v1/tactical/dungeon`
- **Gap:** Arda made towns, fields and roads but no underground maps.
- **New crate `arda-dungeon`:** deterministic levels as ordinary `TacticalLayout`s plus a format-2 rules sidecar and a description (rooms with purposes, doors with `locked`/`secret`, exits). `dungeon`: BSP rooms, MST corridors plus loops, doors where corridors enter rooms, purposes (entrance, crypt, barracks, storage, shrine, prison with locked cells, treasure behind locked doors, hall) with dressing, stairs up and down, torch sconces and braziers. `cave`: 4-5 cellular-automaton caverns with every pocket tunnelled to the main cave, a mouth on the map edge, pools (wade and swim) and a stream, scree, moss, stalagmites, rocks, rubble, glowing mushrooms and a descent. Rock is impassable and opaque through the sidecar; a connectivity guard keeps every walkable square reachable. `site_seed` gives a stable seed per world cell. README: `crates/arda-dungeon/README.md`.
- **Schema:** `WallSegment` gains optional free `tags` (`locked`, `secret`; omitted when empty, older layouts unchanged). A `secret` door is drawn as a plain run; arda-scene reports it as `kind: secret`, and scene walls gain `locked` (omitted when false). Server DTOs, JSON schemas and TS bindings follow.
- **Access:** `arda dungeon --seed N --kind dungeon|cave --size WxH --out layout.json [--rules --meta --scene --render --library --ppsq --grid]`; `GET /v1/tactical/dungeon?seed=|gx=&gy=&kind=&w=&h=` (layout, rules, rooms, doors, exits), `/dungeon.png` and `/dungeon/scene` (`crates/arda-server/API.md`).
- **Vocabulary and art:** placeholder art (242 assets, was 217) for new `ground.cave_floor`, `ground.bedrock`, wall kit `cave`, `prop.stairs_down`, `prop.torch_sconce`, `veg.stalagmite`, and for `prop.tomb`, `prop.coffin`, `prop.bone_pile`, `prop.skeleton`, `prop.cage`, `prop.gaol_cot`, `prop.rubble_pile`, `prop.chest_treasure`, `prop.stairs` (already in the AI library). They carry only free tags no town or field tag query uses, so no query picks them; the one visible effect elsewhere is that town blocks resolved against the placeholder library alone now draw `prop.stairs` instead of its ladder fallback. Every existing test passes unchanged. Prompt pack tier 4 (34 images, not generated): the two ground keys, the `cave` kit and three takes each of the three new props.
- **Tests:** determinism, every walkable square reachable with doors opened, no floating doors, exits present, sane scene rules (rock impassable and heavy, closed doors solid, locked and secret doors, wade, swim, difficult), refused sizes, pinned renders, and the three server routes.
- **Follow-ups:** choosing world sites for entrances (crypt entrances near settlements, cave mouths on steep slopes); mixed kinds (mines, ruins); light pools that respect walls in the compositor.

## 2026-10-02 - Plains: measured flat; recipe 8 adds discharge-scaled floodplains (goal 5)
- **Question:** goal 5 was 🟡 because the lowland (land below 200 m) slope median was 4–5° and rose from 4.6° to 5.2° on seed 42 after the bands fix. Real plains are well under 2°.
- **Measured first** (`plains_metrics`, recipe 7, 100 m area cells): the 4–5° figure is MICRO seed 42, where 88% of land and 60% of lowland lie in Hammond "mountain" windows (over 300 m of relief within ~10 km). There, 98% of land below 200 m has at least 50 m of relief within 2.1 km: hill country near the sea, not plain. Genuine plains (under 50 m of relief) are flat on every seed: median 0.2–1.0° on MICRO 42/3/5/7. On the full-size seed-42 world, land below 200 m has a median slope of 0.95°, plains cover 34% of the land at 0.54° (interfluves 0.75°, valley floors 0.09°), Hammond plains 16% at 0.36°, and 57% of lowland valley sides are terrace treads.
- **What was wrong:** floodplains did not scale with discharge. On full-size seed 42, rivers in plain settings had floors (ground within 2 m of the river) with median widths of 705 m at 2–20 km², 500 m at 20–100 km², 423 m at 100–1,000 km² and 705 m above 1,000 km².
- **Recipe 8 (opt-in, `--recipe 8`):** alluvial valley floors before the terraces (logic/02 §fine-formation plains). Half-width `60 m × A^0.4`; built where the bankfull specific stream power is low (Nanson & Croke), the channel under 3% and belt relief under 800 m; floor at the channel plus 0.4‰ (aggradation below, lateral planation above) up to a `4 D` bluff; no floor in confined valleys (openness under 35%) or narrower than 100 m a side. Interfluves are not smoothed: they are already flat.
- **Result, full-size seed 42 (recipe 7 → 8):** plain-river floor medians 705 → 705, 500 → 564, 423 → 600 and 705 → 987 m (now growing with discharge from 20 km² up); all-river floor medians 200 → 282 m below 1,000 km². Lowland 0.95° → 0.92°, plains 0.54° → 0.53°, low hill country 2.87° → 2.82° (unchanged character). Pits 0 → 0, lakes ≤ 3 cells 0 → 0, straight runs 2‰ → 1‰, lakes 356 → 394 (oxbow 110 → 137, karst 216 → 227, all explained). MICRO seeds: hill country unchanged, 0 pits, 0 speckle lakes.
- **Compatibility:** recipes 5, 6 and 7 are byte-identical (`tests/golden_recipes.rs`); recipe 8 is pinned from its first release (`tests/golden/micro-42-recipe8.txt`). The default stays 7.
- **Renders:** `out/v6-plains/` (overview crops at 100% and mid-zoom relief, recipe 7 against 8).

## 2026-10-02 - v0.6 integration: plains, worked land and field systems on one branch
- **What:** `integrate/v0.6` merges feat/v6-plains, then feat/v6-field-patterns (which contains feat/v6-midzoom-landuse), onto main's 0.5.0 (`c63a4b1`), with no conflicts. The default recipe stays 7; recipe 8 is opt-in.
- **Wedge-shaped fields (logic/17 §land-fields):** a cut other than a road may not meet its piece's boundary at under 38°, nor run within 12 squares of a boundary edge at such an angle. The check reads the clipped outline, the block outline and the true lines of the cuts above. The last-resort cuts obey it too, and a piece no cut can split stays one field. On MICRO 42, corners under 35° away from roads and rivers drop from 98 to 4 over six 4 km squares.
- **Verification:** `arda-fields/tests/partition.rs` `no_field_corner_is_a_wedge` (five synthetic scenarios, every corner 35° or more away from roads and rivers; it fails without the rule); the full gate and the server smoke test (integration-status §v0.6). Renders in `out/v6-int/`.
- **Skipped:** hedges and field walls drawn along the true diagonal instead of square-edge staircases. The reason is in integration-status §v0.6.

## 2026-10-02 - Mid-zoom relief: worked land and readable plains (goals 5, 39, 41, 49)
- **What:** at 15–30 m/px the full-size world's plains rendered as featureless green blotches; relief tiles drew no land use, roads or settlements, although the society data and the tactical layers had them.
- **Land (logic/17 §land):** relief tiles of a world with `society/` draw field parcels with hedgerows and drystone walls, pasture, arable, orchards and woodland, roads and lanes at their class widths, and settlements' buildings and streets, from the tactical layers' own geometry: the arda-fields partition, the arda-ways centrelines and the arda-town plan grids. Detail follows the pixel (tone from afar; strips, furrows, orchard rows, crowns, lit roof pitches and shadows close up).
- **Low relief (logic/17 §low-relief):** on low ground, topographic position against the ~200 m mean tints valley floors and interfluves, and the slope light is amplified as local relief shrinks below 6 m, so terraces and bluffs read; nothing is added to the heights.
- **Defaults:** worlds without `society/` render byte-identically; the tactical world grade is unchanged. `arda-blocks` shares its land-use raster and roads; `arda-fields` samples its tensor field in parallel (same values).
- **Verification:** `tests/zoom_land.rs` in the zoom-continuity gate (MICRO 42, z9 against composed blocks: road IoU 1.0 over 1,342 squares, building IoU 1.0 over 1,793, all 1,148 walls between fields on relief hedge lines; land tiles pixel-exact); `arda-midzoom/tests/land.rs` (synthetic: fields one-to-one with parcels, road IoU 1.0). Renders in `out/v6-mzland/`.

## 2026-10-02 - Historical field patterns (goals 39, 43, 49)
- **What:** the field partition was a jittered lattice of sites under a contour-stretched metric; from above (2–15 m/px) its parcels read as Voronoi cells of one size, a board-game look.
- **Field systems (logic/17 §land-fields):** `arda_fields::partition` is now a fixed global hierarchy: brick-laid land blocks with drifting vertices, recursively split along roads, across long axes square to the longest side, along contours, at jittered and slightly kinked lines (old enclosure) or on a surveyed bearing (planned enclosure), until each piece is the size its ground asks for. Closes by the houses grow into larger, more varied fields further out; villages keep furlongs of strips with headlands; floodplains are meadow and poor grazing is open common (`FieldKind::Common`).
- **Contracts kept:** `kind_of`, crops, hedges, walls, gates (every enclosed field gated), farmsteads and the sidecar rules are unchanged; the relief tiles read the same partition (`Partition::locate_edge` gives the exact distance to a field's edge and the field across it), so field walls stay on relief hedge lines.
- **Inputs:** `FieldInputs` gains `rivers`; `LandUseMap` is `Sync`; arda-blocks reads fields' surroundings and settlements over the partition's reach (`FIELDS_MARGIN_M` 3.8 km) and shares `field_roads`, `fields_rivers`, `FIELDS_CULTURE` and `FIELDS_WEALTH` with the relief tiles.
- **Verification:** `arda-fields/tests/partition.rs` (determinism and exact seams; shape on synthetic country: four-sided fields 5–7 % → 89–90 %, T-junction share 0.22–0.50 → 0.98–0.99, median elongation 1.38–1.53 → 1.51–1.70, p90 1.99–2.30 → 2.8–3.8; enclosed field size 0.47 ± 0.28 ha by the hamlet against 0.78 ± 0.40 ha further out, where the old partition was flat at 1.05 ± 0.29); e2e village (cold block 340 ms, quarter window 99 ms); 1,082 walls between fields all on relief hedge lines. Renders in `out/v6-fields/`.

## 2026-10-01 - Zoom continuity: one water geometry, soft snow (goal 49)
- **What:** at the relief z9 to tactical hand-off rivers and shores sat in different places (cell 465,1166 about 40 m off; at 519,1377 the relief drew a sea inlet over most of the cell, the tactical map only a corner). Relief tiles drew rivers along the overview's centrelines and lakes and coasts from the stored field; the tactical blocks build their own channels and shores.
- **Source of truth:** the tactical layer's water (logic/09 §linear-features), which towns, ways and crossings already use. `arda_refine::region::WaterRegion` gathers the cells, channel edges and fine lattice a block reads for any window (`Ctx::gather_region`) and answers shores (`terrain::standing_water`, now one shared function), channels and pools at any global square position. Relief tiles (`arda-midzoom` `water.rs`) sample it at pixel centres; widths ease from the overview's ×4 symbol at 25 m/px to the true banks at ≤ 6.25 m/px. Depth tints keep the stored field where it holds water. Logic/17 §water, §rivers, §render.
- **I1:** both layers now reach the fine lattice through refine's single conversion (`FINE_ORIGIN_SQ`); relief no longer classifies water in its own frame.
- **Snow (logic/11 §snow-cover):** snow near a border is a cover over the ground beneath: widely warped, kernel-softened weight broken by rotated noise inside the band, a speckled dusting ahead of the patch, grey-blue thin cover; seams stay pixel-exact.
- **Verification:** `region_water_matches_the_refined_blocks` (synthetic rivers, lake, sea, marsh: square for square); `tests/zoom_continuity.rs` (release gate, MICRO 42: IoU 1.0 over 5,102 water squares in 11 cells); snow seam and feathering tests; overview bytes and tactical layouts unchanged. Renders in `out/v4-geo/{before,after}/`.

## 2026-10-01 - v0.5 integration: hardening, schemas and art import on one branch
- **What:** `integrate/v0.5` merges fix/v4-hardening, feat/v4-schemas and feat/v4-art-import onto `integrate/v0.4` (`c28e2bf`, recipe 7 the default). The JSON Schemas and TypeScript bindings are regenerated for cell contract 3, so `CellSample`, `PointSample` and `AreaCells` describe `land_use`, `realm_id` and the `footpath` road.
- **Art import (goal 60):** the new crate `arda-art-import` and `arda tactical import` turn raw AI-generated images into a valid tactical library (cut-out, shadow stripping, fit, tileable textures, colour grade, catalogue with inherited metadata and provenance, report and contact sheet). `Library::load_stack` reads `top:…:bottom` library stacks with per-slot fallback; the CLI, the server, `arda-town block` and the arda-scene `debug` and arda-ways `crossings` examples accept them.
- **Footprint cap and stacks:** import manifests refuse footprints over 16 squares a side before fitting, and a stack refuses to rescale a lower layer's asset past 2,048 px a side (a top layer with a large `pixels_per_square`).
- **Verification:** the server's schema tests validate real responses on a recipe-7 fixture (the shared fixture is reused only when its recipe is the default), including a village cell's contract-3 fields alone and in its area; the shared test-fixture builder takes a lock, closing a race where a late builder moved a complete fixture aside.

## 2026-10-01 - JSON Schemas, typed scenes and the sheet mapping (goals 66, 69; A12)
- **What:** every public body now has a draft 2020-12 JSON Schema generated by `schemars` from the same Rust types as the TypeScript bindings, committed in `bindings/schema/` and served at `GET /v1/schema` and `/v1/schema/{Name}.json` (logic/16 §api-schema). Settlement, society and town-plan schemas come from the domain types through a new optional `schema` feature in arda-ids, arda-names, arda-settle, arda-society and arda-town; serialized output is unchanged.
- **A12:** the scene is typed end to end. `SceneDto` mirrors `arda_scene::Scene` (run-length `Runs<T>` grids, `[x, y]` squares, string seed), `TacticalScene`, `Token` and `TokenKind` type the scene route, and `TacticalBlockDto.scene` is `SceneDto | null` instead of `unknown`. The viewer reads scenes through the generated types and shows each square's movement, cover, obscurement and climb.
- **Goal 69:** `--sheet-mapping game.json` reshapes every served NPC into a game's own schema with a declarative file (pointers, nesting, constants, templates, enum tables, unit scaling, case and format conversions, drops), checked and dry-run at startup (logic/16 §api-sheet-mapping). Examples: `identity.json` and `5e-srd-monster.json` (the open 5e SRD JSON monster layout).
- **Verification:** schema staleness and determinism tests; scene mirror round trips; `tests/world_api/schemas.rs` validates every route's real MICRO seed-42 response with the `jsonschema` crate; mapping unit tests (identity, an inverse round trip, the SRD mapping over about 80 probe NPCs, refusals) and `tests/world_api/mapping.rs` (mapped routes, byte-identical unmapped routes, refused startup).

## 2026-10-01 - feat: opt-in oblique overview and tactical world grade
key: feat/2026-10-01-v4-look-options

- **Oblique overview (goal 24, opt-in).** `arda export --overview --style atlas-oblique`, `GET /v1/overview.png?style=atlas-oblique` and `GET /v1/tiles/overview/{z}/{x}/{y}.webp?oblique=1`: the finished Atlas rows are warped north by 0.5 × a smooth 400 m surface (sea and lakes at 0, so coasts never move; south flanks stretch, north flanks compress, Catmull-Rom across rows), plus aerial perspective (≤ 6 % in deep valleys, ≤ 4 % on low ground), valley occlusion (≤ 9 %), crest lift and sky fill. Streams with a bounded row window. Recipe-5+ Atlas only.
- **Tactical world grade (goal 49, opt-in).** `?world_grade=1` on world cell/window images and tiles: ground and water are pulled toward the formed relief colour at their world position (12.5 m world-anchored lattice, `arda_midzoom::world_lattice`), relative to a 7 × 7-square regional texture mean so detail survives; streets and floors take a quarter, farmland half. Seam invariant holds.
- **Viewer:** World view toggles `oblique` (hides relief levels while on) and `world grade` (carried into "Open tactical map here"); Cell view toggles `world grade` and `world hand-off` (relief at the switch-over zoom, a cross-fade and the tactical map side by side).
- Defaults are byte-identical: the recipe-5/6 4K Atlas goldens pass; `world_grade=0`/`oblique=0` equal the plain requests. Comparisons: `out/v4-look/`.

## 2026-10-01 - v0.4 hardening: cell contract 3 and the open review items
- **Cell contract 3 (adapter A11, logic/16 §api-cell-society):** with a `society/` directory, `/v1/cell`, `/v1/point` and `/v1/area/.../cells` read `road` from the new `society/roads.bin` (gaining `footpath`), `built_by` from the `landuse.bin` owners (now a settlement id string), and the new `land_use` and `realm_id` from `landuse.bin` and `realms.bin`. `ARDACOLS` layout 2 adds the `land_use` and `realm_id` columns. `arda settle` writes `roads.bin` (logic/08 output table). Worlds without `society/` keep the stored `road` and `built_by`; re-run `arda settle` to get `roads.bin` for an existing world.
- **`arda_refine::Source::pan` is required:** a wrapping source must forward the playa query or fail to compile.
- **Server:** error bodies shorten absolute paths to the file name; overview qualities are a short ladder (512 doubling to the limit); images revalidate by `ETag` with `Cache-Control: no-cache` instead of `public, max-age=3600`; tactical PNG encodes and pyramid builds run one at a time and are admitted; an empty world no longer underflows the out-of-range message.
- **Society** indexes its history, traffic and relations (8,000 settlements: 1.0 s, was 5.1 s; output byte-identical). **Settle** encodes every file before writing, and its rank-size fit uses a platform-independent logarithm (`stats.json` moves in the last digit).
- **Ways:** the layout's lakes and sea are never paved; crossings are planned wherever their synthetic channel reaches; a straightened way is reindexed at once; only roads that can shape a window are planned (byte-identical); ids are u64. **Fields:** mines move off roads; open fields can be `complete`; land-use rasters are bounded; furrows run across window edges. **Scene:** secret doors are no spawn entrances, cleared canopies stay lightly obscured, `derive` checks its inputs, and the grid rules are attributed as house rules. **Names:** roots are never blocked or repeated. **NPC:** weighted draws over huge totals and resistance de-duplication are fixed. **Tactical:** catalogue footprints are capped at 16 squares a side. **Viewer:** late responses of aborted requests are ignored.

## 2026-10-01 - fix: v0.3 integration follow-ups
key: fix/2026-10-01-v0.3-integration

- The five v0.3 branches (recipe 6, realms, API, tactical, town WFC) are merged on `integrate/v0.3` (integration-status §v0.3 integration).
- **Varied interiors feed the town WFC.** A home's partition, household trade, beds, fire and clutter pools (goal 64) become its WFC programme, keyed by the interior salt; the rule programmes stay the relaxed fill and the `TownFill::Rules` strategy. Seed-42 city: 1,665 distinct WFC home interiors of 1,673, no adjacent pair alike.
- **Ordered furniture stands in rows** (logic/10 §town-wfc): pews either side of a clear central aisle, dormitory beds and library bookshelves along the long walls, warehouse racks and market-hall tables in free-standing rows, laid out by rule before the WFC fills around them.
- **v0.2.0 worlds** (recipe 5 recorded, recipe-6 terrain) load as recipe 6 when `terrain/shore.bin` is present (logic/02 §fine-formation recipes).
- **Relaxed town fills reach the tactical meta:** cell and window `meta.relaxed` and `meta.review_squares` count the town WFC's relaxed interiors and chunks.
- **`prop.drain` is art-free:** placed by the WFC, kept out of layouts until a library draws it.
- **Fix:** the parallel PNG encoder could lose a strip's sync flush and serve a corrupt PNG (a 2048 × 2432 px window on MICRO seed 42); strips now end on their flush marker.
- MICRO seed 42 relaxed WFC fills: 43 of 44,819 (0.10 %). Renders: `out/v3-integrate/{before,after}/`.

## 2026-10-01 - Open country at tactical scale (goals 19, 20, 43)
- **What:** open ground was flat meadow with blotchy heath, gravel and dirt. Two causes: the fields overlay claimed every field site without a land use and painted it with heath/scrub noise, and the refiner's own ground prior was dominated by patch noise while its scatter was even.
- **Fields:** sites with no land use stay natural (`arda-fields` `window.rs`, `fringe.rs`).
- **Ecology (`arda-refine` `ecology.rs`):** species from temperature corrected for aspect (±1.8 °C), growth to the tree line, wetness from hollows and water, and landform: conifers on cold and shaded ground, broadleaves low, willow and alder by water, stunted and dead trees at the tree line, juniper on cold open ground; low plants by shade, wetness and sun.
- **Landform (`shape.rs`, `prior.rs`):** a curvature index and a talus index from the refined surface. Ridges carry rock and heath, hollows meadow, moss and marsh; snow lies in shaded hollows on north slopes in cold country. Patch noise halved.
- **Structure (`density.rs`):** groves cut from a slow noise field at the forest-density quantile (copses in open land, glades in forest), stream tree lines, grove rims with young trees and undergrowth, rock outcrops on steep knolls, boulder fields on talus, clumped loose rocks, crevice plants, logs and stumps, drifts of flowers, tall grass, heather and ferns.
- **Trails (`trails.rs`):** game trails and footpaths decided per cell edge from the two cells (water, settlement and pass bonuses), routed inside each cell by the cheapest path to its hub (the river node, a ford), continuous across blocks; ground key `trail`.
- **Art:** placeholders for `trail`, `veg.tree_alder`, `veg.tree_stunted`, `veg.juniper`, `veg.rock_outcrop` (library 207 → 213 assets).
- **Verification:** unit and integration tests for species versus altitude, aspect and water, snow on north faces, feature-count bounds, scatter and trail agreement across seams, trail continuity across block edges, determinism. Renders before and after in `out/v4-country/`.

## 2026-10-01 - feat: recipe 7, arid terminal and salt lakes
key: feat/2026-10-01-arid-basins

- **Recipe 7 is the default** (approved by the maintainer on 2026-10-01; `FINE_TERRAIN_RECIPE_VERSION` 7). `--recipe 6` reproduces v0.2–v0.3 worlds byte for byte (`tests/golden_recipes.rs`); recipe 7 is pinned from its first release.
- **Climate in formation** (logic/02 §fine-formation climate runoff): the continent's annual balance, `R = P − min(P/2, E)` and `D = E − min(P/2, E)` with Hamon `E`, weights contributing area by `R / 500 mm` for channel initiation, incision and channel sizing (network, oxbows, delta distributaries).
- **Subtropical highs** (logic/01 §Q6): recipe-7 rainfall is scaled by the Hadley belt, 30% between 15° and 26°, full poleward of 38°. `arda generate --latitude SOUTH,NORTH` sets the band (default 35,55).
- **Arid basins** (logic/02 §world-water arid basins, goal 12): an audited tectonic basin whose catchment runoff is below 90% of the evaporation its depression would sustain at the spill is terminal. Formation fills its floor into a playa (three times the equilibrium lake), and the shared annual solve finds the terminal lake at equilibrium on it. Ledger closure with evaporation is tested on arid and humid bowls (goal 21).
- **water.bin layout 2** (recipe 7): lake origin `arid_terminal`, a saline flag, and playa runs (salt crust, mudflat). Recipe 6 still writes layout 1.
- **Atlas** (logic/04 §atlas-formed arid basins): saline lakes in lighter turquoise, salt pans as white crust with pale mudflat margins; dry land below sea level inside the continent is land.
- **Tactical** (logic/09): playa squares take the new ground keys `salt_crust` and `mudflat` (difficult), with no scatter; generated placeholder art for both (211 assets), fallback chains in arda-fields, arda-ways and arda-town; `docs/goal-prompts/vocabulary.md`.
- MICRO seed 74 at 15–35°: a 482 km² terminal saline lake on 697 km² of dry pan (land rain 382 mm); recipe 6 at the same band fills the basin into a 1,278 km² fresh lake. Renders: `out/v4-arid/`.

## 2026-09-30 - feat: world rivers and lakes (goals 8-13)
key: feat/2026-09-30-world-water

- New `formation::water` stage (logic/02 §world-water) on the drained recipe-5 lattice:
  - **Braids:** braided belts above the Leopold–Wolman slope, where bed load is plentiful (piedmont or proglacial) and the valley opens.
  - **Meanders:** Kinoshita meanders on alluvial floors below that slope, with sinuosity rising as slope falls, planed meander belts and oxbow cutoff crescents.
  - **Karst:** poljes on a documented carbonate proxy, with an inlet and a downstream spill; dolines are recorded but not carved.
  - **Deltas:** these now build. The old rule's 2,000 km² threshold excluded every MICRO river, and its fan sat inside the drowned ria head. The new deltas are sediment-volume lobes after the shore rework; steep coasts keep their rias.
- `arda_core::water`: bankfull depth `0.3 m × Q^0.4` beside width `4 m × Q^0.5`, the braiding threshold, and stored forms. The optional `areas/<ax>_<ay>/water.bin` holds, per river segment, the pattern, bankfull width and depth, belt, sinuosity and slope. Per lake it holds the origin and terminal (saline) state. It also lists deltas and dolines. Load it with `Area::water()`.
- Atlas overviews draw braided threads, and they drop the synthetic meander where the saved courses already meander.
- New `water_metrics` and `water_diag` examples.
- MICRO results, before → after (the `water_metrics` and `world_metrics` examples):
  - **Hydraulic geometry**, all seeds: width exponent 0.511 and depth exponent 0.412 against discharge. Depth never falls downstream at any confluence (21,123 on seed 42).
  - **Sinuosity** of rivers ≥ 1 m³/s:
    - below 0.3‰ slope: seed 42 1.17 → 1.19, seed 3 1.12 → 1.25, seed 5 1.14 → 1.49;
    - 0.3-1‰: seed 42 1.15 → 1.23, seed 3 1.08 → 1.27, seed 5 1.09 → 1.29.
  - **Lakes:**
    - seed 3: 11 → 17 (6 karst; 15 with an inlet, 17 with an outlet);
    - seed 5: 0 → 2 oxbow lakes;
    - seed 42: 0 → 0 (V-valleys only).
    - No lake of three cells or fewer anywhere.
  - **Deltas:** 0 → 1 (seed 42) and 0 → 3 (seeds 3 and 5).
  - **Other checks:** straight-run share 14 → 12‰ (seed 42), bed pits 0, water-ledger residual 0 L. Seed-42 generation takes 52 → 55 s.

## 2026-09-30 - integrate: world physics (bands, coast, water, atlas) on variant Q
key: integrate/2026-09-30-world

- Merged, in order, `fix/uneroded-bands`, `feat/world-coast`, `feat/world-water` and `feat/atlas-polish` onto `wip/variant-q`. Not signed off: the look awaits the maintainer.
- **Stage order** (`formation::form_world`, logic/02): margins, bathymetry, arc cones and the basin audit on the macro surface; belt-relief level masks (read after the arcs, so cones count as high relief); formation; coast (infill, terraces, shelf prism, shore rework, littoral, canyons); water (deltas, channels, basins); drainage guarantees; the shore survey last.
- **One delta rule:** §world-water's river-led, volume-limited lobe. `coast::deltas` is removed. The coast rule's two contributions carry over: distributary islands (from 1,000 km², a fork into two tidal channels) and its 160/255 coast limit (for rivers of 2,000 km² and more, rising from 100/255 at 1,000 km²). The shore census reads the water deltas, so delta islands keep their cause. Lobes that would reach within 3 km of the domain rim are refused by the radius bisection.
- **Published layers:** the orchestrator writes both `terrain/shore.bin` and `areas/*/water.bin`. The Atlas paints stored shores and follows the data meanders with the tapered B-spline centrelines.
- **Fixes found while verifying:**
  - canyons followed D8 paths as straight trenches across the shelf prism; they now follow a continuous, wandering heading for up to 50 km and fade out;
  - the shelf-profile distance is Euclidean, not a chamfer;
  - arc cones retreat only 30 m in the littoral pass. Before, cliff retreat planed every full-size arc summit to 40–80 m below sea level.
- **MICRO results** (seeds 42, 3, 5 and 7):
  - pits 0, lakes of 3 cells or fewer 0, straight runs 0–3‰, ledger residual 0;
  - belt crest/flank roughness 0.47–0.55, and 1.37 on seed 3, which has no divide band;
  - beaches: more in bays than on headlands on every seed;
  - deltas 1/8/3/1.
- **Full size** (seed 42, 500×1000 km): 46 min and 13.1 GiB peak. Open items are listed in the report: oxbow fragments of 3 cells or fewer, straight channels on large delta plains, and submerged arcs before the cone fix.
- The Classic render is byte-identical (legacy seed 42 MICRO overview and area 1,1).

## 2026-09-30 - fix: world physics that only showed at full size
key: integrate/2026-09-30-world-full-size

Found by the seed-42 full-size run (500 × 1000 km) and reproduced on a 250 × 500 km world, which runs in 10 minutes. Not signed off: the look awaits the maintainer.

- **Deep closed basins** (logic/02 §fine-formation basins). Only rim-connected water below the −120 m lowstand is fixed ocean. An enclosed macro depression deeper than that used to be fixed as sea floor and then lifted 150–200 m by the final fills. That produced the 60 km "delta plain" at 63 m, dead flat, with straight channels and chains of tiny oxbows. It now forms as land, and on seed 42 it floods as a long sound.
- **Fill flats** (§fine-formation flats). Flats are regraded as cost-weighted geodesics, at 1–5 mm per step from rotated noise, so rivers wander and converge across them instead of following grid geodesics.
- **Deltas** (§world-water deltas).
  - The lobe's reach varies with bearing, giving about five sub-lobes, so the front has no noise holes.
  - The plain is concave.
  - A tree of distributaries bifurcates up to three times. The channels bend 28–36° at eleven widths and wander at fan scale.
  - Levees stand 1.4–0.4 m above interdistributary basins.
  - The tidal reach is cut to −1.5 m, so the distal plain splits into delta islands by open water. Sheltered delta shores are marsh.
- **Oxbows** (§world-water oxbows). Each crescent is protected whole and carved with a flat bottom.
  - A loop survives only if it is one 8-connected 100 m lake of at least eight samples that holds at least 0.3 m of water before it spills.
  - At most one loop per two wavelengths.
- **Beaches** (§shore classes). A low headland keeps a beach only on a built sand body or within 1.5 km of a mouth of 20 km². The survey's sea is 8-connected, as in the hydrology.
- **Sampled drainage** (§sampled drainage). The sea is 8-connected, so a lagoon joined across a diagonal inlet is no longer filled into a 100 m grid texture (seed-5 MICRO). Pockets are levelled whole, and the fine fill and sampled pass always run twice.
- **Terrace risers** (§terraces). Risers are read at a noise-shifted height, so they scallop instead of drawing straight lines along planar valley sides (seed-3 MICRO).
- **Channel initiation** (§hillslopes).
  - Maturity scales the initiation threshold 1–2× (it was 1–4×).
  - Low relief raises it up to 24×, cubically.
  - Strong relief lowers it to 0.35×.
  - Slopes over 22° divide it by (S/0.4)².
- **Arc volcanoes** (§margins). A straight subaerial cone reaches the shore at 0.35 R. The concave cone from a 1–4 km floor stood above sea only within 0.4 km of its summit, so every full-size arc ended under water, even after the littoral fix.
- **Memory.** The finest level no longer allocates a zero `u64` area vector (2.6 GB at full size) for roughening.
- **world_metrics** reads lake origins from `water.bin`: 540 → 0 unexplained lakes at full size. It reports barrier and delta islands apart from bays and headlands, and gives class breakdowns.

## 2026-09-30 - feat: recipe-5 coasts, islands, seafloor and plains
key: feat/2026-09-30-world-coast

- Tectonic margin context: the recipe-5 plate drift is replayed without uplift to mark collision, arc, island-arc and rift proximity (logic/02 §fine-formation margins).
- Bathymetry (goal 18):
  - shelf width follows tectonic margin activity as well as coastal relief;
  - passive shelves now shallow toward the profile (a sediment prism);
  - submarine canyons are cut down the slope off rivers of 150 km² or more (§bathymetry, §canyons).
- Islands (goal 17):
  - volcanic arc cones on deep ocean along the final-step subduction axes;
  - barrier islands and spits on low, sandy, exposed coasts;
  - delta distributaries that split fans into islands; deltas now start at 500 km²;
  - drowned-ridge continental islands, as before.
- Coasts (goals 15 and 16): rock- and convexity-driven headland retreat into cliffs over wave-cut platforms, bay-head berms, and stored shore classes: sand, shingle, cliff, rocky, marsh, tidal flat and estuary (§littoral, §shore classes).
- Plains (goal 5): lowland valley sides get a terrace staircase and bluffs, and interfluves are compressed (§terraces).
- Basin audit (goal 6): every closed macro basin and plateau gets a tectonic cause (rift, foreland, arc, orogenic, rift shoulder); unexplained basins are filled before formation (§basin audit).
- New optional world layer `terrain/shore.bin` (`arda_core::ShoreLayer`): shore classes on the 100 m grid, an island census with causes, and audited landforms, including glacial troughs. It is published with the fine terrain and read by `World::shore`. The Atlas paints shores from it (logic/04 §atlas-formed shore).
- Earlier recipes, Classic output and the manifest are unchanged. Recipe-5 terrain changes; it has no golden hash.
- `world_metrics` gains `coast`, `landforms` and `plains` objects.

## 2026-09-30 - fix: uneroded straight crest bands in recipe-5 formation
key: fix/2026-09-30-uneroded-bands

- Level masks read **belt relief** (logic/02 §fine-formation masks): the highest macro relief within a 12 km disc, softened. Local standard deviation dips along every broad crest, so the seed-42 MICRO main divide (~70% of the width) was classed as low hills, with up to 7× the channel-initiation area, and stayed a smooth band ~4 km wide. Crest/flank roughness 0.24 → 0.56.
- Macro heights are sampled Catmull-Rom clamped to the enclosing nodes (logic/02 §fine-formation macro warp). The cubic undershoot beside the east-coast cliff had become a straight line of fixed sea: a long, straight trench and ridge at ~97% of the width.
- Seed-42 MICRO metrics: straight runs 14‰ → 0‰; pits, lakes and speckle lakes stay 0.
- Tests: seed-42 divide dissection regression; belt relief crest; bounded cliff sampling.

## 2026-09-30 - feat: town fabric by WFC (goal 44)
key: feat/2026-09-30-town-wfc

- New crate `arda-wfc`: a generic deterministic WFC (tile sets, symmetric adjacency per direction and edge kind, integer weights, anchors, limits, a global check, local repair, at most 8 subseeded attempts). `arda-refine` now takes its hashing and work lists from it; its block output is byte-identical.
- `arda-town` fills the town fabric inside the plan by WFC (logic/10 §town-wfc) behind `TownFill` (WFC default, rule programmes kept):
  - interiors: rectangular rooms by position-class tiles, zones by a programme search, then doorways, open circulation and typed furniture per function, every square reachable from the doors;
  - shell windows and curtain arrow loops as kit pieces chosen by edge constraints;
  - streets (clear carriageway, kerbs, drains, clutter), the market square (paved edge, goods at pitches), fronts, yards, gardens, churchyards, greens, crofts, per global 32-square chunk with seam-safe borders.
- Relaxed fills are listed in `TownBlock.relaxed` (goal 47). MICRO seed 42, every plan: 45 of 47,247 problems relaxed (0.10 %; checked by the e2e release gate); a city block costs about 10-30 ms more (about 90 ms in all).
- Vocabulary: 306 tiles; one new prop id, `prop.drain`.
- Before/after renders of an inn, smithy, temple, market square and city block: `out/v3-wfc/`.

## 2026-09-30 - perf/feat: cold tactical images under half a second, varied furnishing (goals 50, 64)
key: feat/2026-09-30-v3-tactical

- **Goal 50, cold tactical images.** Release, MICRO seed 42, 64 px per square, 32 threads:
  - a cold block PNG, before → after: city centre 2.0 s → ~0.34 s; village centre, on a freshly opened server, 0.93 s → ~0.42 s; a block next to one already served ~0.52 s → ~0.28 s;
  - a quarter-block window (32 × 32 squares), 0.20 s → ~0.10 s;
  - output is unchanged pixel for pixel wherever the furnishing did not change (checked against v0.2.0 on road, field and river cells, a 2 × 2 window and a WebP tile); the seam tests still pass.
- Where the time went and what changed:
  - **Compositor** (render 340 → ~200 ms for a 76 × 76-square apron block): sprites are stamped in parallel bands of rows, each replaying every sprite in draw order (byte-identical); the drop-shadow sweep runs per diagonal in parallel; ground textures are scaled once per ppsq and cached in the `Library`; `render_region` paints, shades and grades only the block's own pixels while heights, sprites, shadows and blurs still cover the apron.
  - **PNG encode** (90 → ~35 ms): strips of 64 rows are filtered and deflated in parallel and joined into one zlib stream; opaque maps are written as RGB.
  - **Composition** (neighbour block 85 → ~47 ms): the ways, fields and town layers run at once and are composed in the fixed order; arda-fields samples terrain and assigns field sites in parallel.
  - **Town plans** (city plan 1.2 s → ~0.24 s): arda-town samples its plan grid, measures river crossings, finds the nearest channel and wobbles the croft footprint in parallel; plans near a window are drawn at once.
  - **Startup**: the server scales textures for every ppsq and draws the town and city plans (about 0.5 s on MICRO), so no request waits for them.
- `GET /v1/tactical/window[.png]?gsx=&gsy=&w=&h=` takes a window in squares (logic/16), for example a quarter block.
- New example `tactical_timing` (arda-server) prints the stages of a cold block, a quarter window and a neighbour. `tests/e2e_village.rs` (release gate) holds a cold block under 500 ms and a quarter window under 250 ms, best of three fresh servers (measured: 333 ms and 88 ms for the village Ylkin at cell 570,702).
- **Goal 64, furnishing without repetition.** Homes are furnished by wealth, size, storeys and the building's own stream: a town house picks one of five partitions (open hall, back chambers, side chamber, front workroom, cross passage) and sometimes a household trade (weaver, cooper, carpenter, fisher, scribe, chandler) that brings its own pieces; hearth or fire pit, table and seats, beds with chests, storage and clutter (barrels, sacks, crates, rugs, chests, woodpiles by the fire) stand on randomly chosen walls, corners and spots. Cottages, farmhouses (byre side, gate and stock vary) and manors vary too. Each building's interior has a salt chosen once per plan so that no two adjacent buildings share an interior; the choice depends only on the plan, so seams are unaffected.
- Yards and crofts: a home's yard draws 0–6 props from a pool by function and wealth along its back and sides, one or two squares out; each croft parcel has its own density and a secondary use (hay in a meadow, tools in a kitchen garden, favoured stacks in a work yard).
- Diversity on the seed-42 city (Ofylyel, 1,868 homes), before → after: 400 → 1,868 distinct interiors, 91.6 % → 0 % of homes sharing an interior with another, 20 → 0 adjacent pairs alike (`crates/arda-people/tests/interiors.rs`; threshold 5 %).

## 2026-09-30 - integrate: phase E and release 0.2.0
key: integrate/2026-09-30-release-0.2.0

- `integrate/world` (variant Q, bands, coasts, water, atlas polish, full-size fixes) merged into `integrate/product`; the one conflict (`arda-render` `atlas/fine.rs`) kept midzoom's `land_inputs`/`sea_colour` helpers and moved the stored shore painting into them, so relief tiles shade coasts like the overview.
- Relief tiles take the overview's river geometry (main-stem relaxation, B-spline centreline, tapered sources) and the atlas-polish palette; `arda::area_atlas_terrain` takes the shore layer, which `ReliefWorld` reads once (logic/17 §render, §rivers).
- `out/micro42` regenerated on variant Q: 629 settlements, 648 roads, 3 realms; 91,966 people and 3,384 stored notables. The e2e village is now Ylkin (id 24, 524 people, cell 570,702).
- Re-baselined: only the two `world_api` river cells (recipe-5 fixture). The Classic golden world is byte-identical except `world.json`'s `arda_version`.
- Version 0.2.0 for the workspace, its path dependencies and the viewer; release notes in `CHANGELOG.md`.

## 2026-09-30 - fix: restore recipe compatibility with recipe 6
key: fix/2026-09-30-recipe-6

- v0.2.0 changed recipe-5 output in place (variant Q, the bands fix, coasts, water). Recipe 6 is now the default and is exactly v0.2.0's behaviour; recipe 5 again replays v0.1 (commit 7f32695) byte for byte (logic/02 §fine-formation recipes).
- Every v0.2 formation rule is gated on recipe 6: margins, arcs and basin audit, Euclidean bathymetry and shelf fill, clamped macro sampling and ocean mask, belt-relief masks, maturity and relief scaling, roughness, the 0.2 km² channel area, one seam pass, coast stages, water forms, the eight-connected sampled drainage. Recipe 5 keeps the v0.1 bathymetry, deltas and drainage. `terrain/shore.bin` and `areas/*/water.bin` are written only for recipe 6.
- Rendering follows the manifest recipe (logic/04 §atlas-formed recipes): recipe-5 worlds keep the v0.1 formed shader, lake stops, straight river strips, minimum widths and river colours.
- `arda generate --terrain fine --recipe <4|5|6>` (default 6), `arda::generate_from_fine_recipe`, `FineRecipe::FormationV5`; `FINE_TERRAIN_RECIPE_VERSION` is 6.
- Proof, MICRO seed 42: recipe 5 matches a 7f32695 build on every terrain, area, block, continent and hydrology file and on the Atlas 4K overview, area Atlas and Classic exports; recipe 6 matches v0.2.0 main on all of them. Only `world.json` differs (`arda_version`; recipe 6 records 6). Pinned by `tests/golden_recipes.rs`; the Classic golden world is unchanged.
- `formation/mod.rs` split into `macro_view`, `levels`, `surface`, `finish` (recipe 6) and `recipe5`.

## 2026-09-30 - feat: spread realm seats and primate capitals (goals 35, 38)
key: feat/2026-09-30-realm-seats

- New `seats.rs` and `market.rs` (logic/08 §realm-seats). The realm count follows people (24,000 a realm), habitable land (600 km² a realm) and towns (3 a realm). Seats are picked among the larger half of the towns by a size-weighted farthest-point pick over a terrain-aware market lattice. Each seat then moves within its own market area while that balances the areas' shares of people and land. A market area under 16,000 people has no core, so the land holds one realm fewer.
- New `primacy.rs`: within each realm the towns follow rank-size with the seat first. The seat grows into a primate city of at least 8,000 people, and the other towns rise by the fourth root of its growth. The extra people come from the realm's villages and hamlets. This runs before land use, so fields and roads serve the final populations. The city cap never demotes a seat.
- `stats.json` has a new `realm` block: cities per realm, seat spacing against the hexagonal lattice spacing, people-weighted distance to a seat, and realm land and people shares.
- New test `tests/realms_micro.rs`, run on seed 42 by default and on seeds 3 and 7 with `--ignored`. `tests/micro.rs` now checks that society history ends at the present partition and seats.
- MICRO results, before → after:
  - **Cities per realm:**
    - seed 42: [1, 0, 0] → [1, 1, 1];
    - seed 3: [1, 0, 0, 0] → [1, 1, 1, 1];
    - seed 7: [1, 1, 0, 0, 0] → [1, 1, 1, 1, 1].
  - **Nearest-seat distance, min / mean km:** seed 42 21.5 / 25.0 → 48.2 / 50.1; seed 3 42.8 / 45.9 → 42.8 / 45.9 (the same seats); seed 7 23.1 / 25.4 → 25.6 / 28.1.
  - **People-weighted p90 distance to a seat, km:** seed 42 54.4 → 32.4; seed 3 26.5 → 26.0; seed 7 57.5 → 42.2.
  - **Largest land share / largest-to-smallest ratio:** seed 42 0.75 / 5.98 → 0.45 / 2.26; seed 3 0.38 / 3.32 → 0.38 / 3.38; seed 7 0.42 / 7.94 → 0.36 / 3.79.
  - **Largest-to-smallest people ratio:** seed 42 2.35 → 2.06; seed 3 1.32 → 1.31; seed 7 3.18 → 1.90.
  - **Rank-size slope (R²):** seed 42 −1.00 (1.00) → −1.01 (0.85); seed 3 −1.00 → −0.98 (0.84); seed 7 −1.00 → −0.87 (0.83).
  - The urban share rises from 30 % to 37–42 %, because the city floor is fixed at 8,000 on worlds of 90,000–195,000 people.

## 2026-09-30 - feat: on-demand mid-zoom relief (arda-midzoom, /v1/tiles/relief)
key: feat/2026-09-30-midzoom-relief

- New crate `arda-midzoom` (logic/17): refines the stored 39.0625 m field to 9.765625 m on demand, for the tiles on screen only — sharpened spline base, drainage-aligned branching gullies and rock ribs scaled by roughness, slope, relief and cover, 39 m means restored, no new pits, stored rivers and lakes unchanged. Deterministic integer maths; tiles join pixel-exactly.
- `arda-render`: `AtlasTerrain::relief_colour`, `river_vertex_um` and `formed_river_rgb`; `arda::area_atlas_terrain`. Overview output is byte-identical.
- `arda-server`: `GET /v1/tiles/relief/{z}/{x}/{y}.webp` for eight levels past the overview's native zoom; `tiles.relief_max_zoom` in `/v1/world`.
- Viewer: World view switches to relief tiles past native zoom, shows m/px, and offers "Open tactical map here" at ≤ 2 m/px.

## 2026-09-30 - feat: people queries, WebP overview tiles and tactical prefetch (goals 51, 57, 67, 68)
key: feat/2026-09-30-api-people-prefetch-webp

- **Commoners are addressable (goal 51).** `GET /v1/npc/{id}` accepts the reference `<settlement>.<building>.<index>` (logic/13 §npc-id) for anyone, commoners included, regenerated from the world seed, the settlement, the home building and the resident index. The u64 id still resolves stored notables, and resolves a commoner with `?settlement=`. A notable resolves to its stored copy by either name.
- **People are queryable (goal 57).** `GET /v1/npcs?settlement=&building=&job=&realm=&notable=&limit=&cursor=` returns `NpcPage` pages (at most 200 people and 16 settlement skeletons a page, cursor-resumed, in settlement then skeleton order). `GET /v1/buildings/{settlement}.{building}/residents` and `/workers` list a building's people. `arda-npc` gains lazy skeleton access (`Generator::resident`, `residents_of`, `npc_at`), so a page expands only the people it returns and nothing is materialised world-wide (goal 56).
- **WebP overview tiles (goal 68).** `/v1/tiles/overview/{z}/{x}/{y}.webp` serves the overview pyramid as lossless WebP with the PNG tile's pixels; `.png` stays. The viewer's world map uses WebP.
- **Prefetch (goal 67).** `POST /v1/tactical/prefetch {gx, gy, radius ≤ 2, ppsq}` answers 202 and queues the rings around the cell (bounded at 64, deduplicated). `--prefetch-workers` threads (default 2) warm each cell's block and JSON body and, where the caches have room beside the open cell, its image pyramid. Renders now run in a request lane and a prefetch lane, so warming never holds a request back, and a key never renders twice at once; admission counts the second lane's transient render. The render cache key is confirmed to carry the world seed, the cell's world-square origin and the catalogue `library_version`.
- **Viewer.** The Cell tab prefetches the open block's 8 neighbours and has edge arrows that open the seamless neighbour.
- New ts-rs bindings: `NpcPage`, `NpcEntry`, `PrefetchRequest`, `PrefetchAccepted`, `PrefetchCell`, `PrefetchStatus`.

## 2026-09-29 - feat: recipe-5 stream-power terrain formation and formed Atlas shading
key: feat/2026-09-29-recipe5-formation

- New `arda-gen::formation` stage (logic/02 §fine-formation): six-level integer stream-power formation from the macro surface to the canonical 39.0625 m field.
  - **Erosion:** warped upsampling between levels; stochastic slope-weighted receivers; relative uplift against a spatially varying talus cap (rock strength); coarse-level diffusion plus lowland soil creep; soft-edged alluvial floodplains.
  - **Coasts:** glacial-lowstand drowned coasts with estuarine infill, river-mouth deltas and wave-reworked shores.
  - **Basins and lakes:** tectonic basins preserved as closed depressions; glacial trough lakes and fjords.
  - **Drainage guarantees:** a final fine-lattice fill plus a 100 m sampled-drainage pass; local depression fills between exact steps.
- `--terrain fine` now produces recipe 5. Recipe 4 remains reproducible via `generate_world_with_fine_recipe(.., FineRecipe::Valleys)`. `FINE_TERRAIN_RECIPE_VERSION` is 5.
- Atlas (logic/04 §atlas-formed), recipe ≥ 5 only:
  - four-baseline relief light with soft warm shadows and a palette matched to measured reference colours;
  - landform-driven materials, beaches and shore rock;
  - snow from saved temperature lapsed to fine height;
  - fine-contour anti-aliased coasts and lakes, and an 8-stop depth sea palette;
  - thalweg-snapped river vertices in area and overview renders, and a recipe-5 overview river symbol;
  - parallel overview base sampling, byte-identical to serial.

  Recipes 2–4 and Classic are unchanged, apart from a 20-byte Atlas halo sample that now carries saved temperature.
- New diagnostics: the `world_metrics` example (lakes, straight-run river share, pits, hypsometry, slopes) and the `form_fine_terrain` example.
- Seed-42 measurements:
  - MICRO formation 25 s; complete world about 55 s; lakes 1,055 → 0 artefact lakes.
  - 500×1000 km worlds complete within **12.8 GB peak RSS** (under the 16 GiB ceiling).
  - Against the previous best recipe-4 world: grid-straight river runs 265‰ → 0‰, bed pits 447 → 1.
- Tests: formation determinism (repeat and single-thread); no fine or sampled pits; local fill; admission refusal; ria infill; basin closure; glacial basins; order and area conservation; integer sqrt, Catmull-Rom and blur; formed shader light, snow, sea, shore and lake colours.

## 2026-09-25 - Terrain study and requested evidence cleanup
- **Request**: replace the feature's accumulated working evidence with a readable study, a catalogue and representative images; delete redundant feature files.
- **Retained study**: the terrain-corrections feature folder keeps a `STUDY.md` that gives the chronological account from first Atlas rendering through the accepted procedural source and full 32K delivery, and an `EXPERIMENTS.md` that accounts for all 194 evidence groups, including rejected trials, numerical-only work and final outcome corrections. Eight existing images are retained unchanged.
- **Preservation**: the original 523-file world, the final 524-file world and the final 32K PNG moved to `out/terrain-delivery/`; all file hashes were verified before and after relocation. `receipt.json` preserves hashes, stage timings, resource admission and cleanup counts.
- **Deletion**: removed 22,094 redundant files totaling 32,673,485,699 bytes. The feature folder now holds only two Markdown files and eight images. Earlier September 22 feature folders and historical changelog entries remain intact.
- **Reference refresh**: current chapter evidence links resolve to the study/catalogue or the relocated deliverable. Historical inline filenames are marked as provenance for removed experiments.
- **Review**: a review pass synthesized the experiment groups; the final read-only check corrected wording that could imply an unrun lateral-retreat hypothesis had been tested. Catalogue coverage, local links, image hashes and retained-world relocation were checked.
- **Code**: implementation commit `52eab74` was pushed to a feature branch before consolidation. This cleanup changes documentation and local artifacts only; no generation or rendering code changed.

## 2026-09-25 - Preferred source: macro-geography composition
Added shared integer local-relief gating and checked signed composition. The working diagnostic reproduces inland/coastal fields exactly across fresh processes and against an independent integer oracle. Actual Arda comparisons place rugged relief within an existing mountain belt while retaining quiet lowlands; local coastal geometry remains connected to the crop boundary. Eighteen targeted tests and workspace checks pass.

Normal world generation remains unchanged. Whole-world admission, seed/coverage, fine persistence and drainage/evolved-height authority require integration before final 32K delivery. Original world preserved. Evidence: `features/2026-09-23-terrain-corrections/evidence/spectral-macro-composition/`.

## 2026-09-25 - First complete world using canonical fine terrain
Added whole-domain source admission and streaming composition; registered 1 km and 100 m samplers; an opt-in complete-world generator consuming the same finalized file; optional manifest recipe metadata and an explicitly budgeted validated loader. Legacy worlds remain compatible and normal generation retains its existing path.

Seed-42 102×204 km attempt 0 generated eight areas with climate and shared drainage. All five 64 km candidates failed the existing river gate. The unchanged 160-step evolution was rejected after an exact comparison recreated smooth faces and repeated incision. Fine-resolution production Atlas export, a broad flat region in the first complete-world render, multi-world validation and full 32K delivery remain open. See `features/2026-09-23-terrain-corrections/evidence/fine-world-integration/` and `spectral-evolution-handoff/`.

## 2026-09-25 - Fine terrain storage and full world seed identity
Added bounded streaming fine-height persistence with versioned geometry, metadata/payload checksums and canonical integer queries. Both source-composition cases reopen and reproduce their four Arda renders byte-for-byte. Added the existing full 64-bit world seed/attempt RNG path for spectral generation while retaining the native 32-bit reference/calibration path. Core and focused generation checks pass.

Normal world admission, terrain authority, climate/drainage, manifest/export integration and final 32K delivery remain open. Evidence: `features/2026-09-23-terrain-corrections/evidence/fine-terrain-persistence/` and `world-spectral-seed/`.

## 2026-09-25 - Fine terrain in Atlas and source-boundary correction
- **What**: normal Atlas exports consume optional saved canonical fine terrain with bounded windows and global physical sampling.
- **Source**: one fixed smooth local-range gain removes the old 400 m dead zone; an existing-rim-derived taper keeps outer source edges ocean. Source recipe 2 is explicit provenance; recipe 1 remains loadable.
- **Verification**: 250 core/render/facade tests, 15 focused gen checks, all-target workspace Clippy, formatting and diff checks pass; corrected source and 35 complete small-world files replay exactly, saved heights match, and annual water closes.
- **Preservation**: all 523 original-world files and exact file set unchanged.
- **Visual finding**: fine detail survives normal exports; broad uniform lowland and sharp relief/material front persist. Exact replay attributes the source plateau to the inherited tectonic subsidence clamp; saved wetness saturates there.
- **Open**: reference-quality appearance, default source adoption, multiple worlds, full-size runtime/resource qualification and final seed-42 32K delivery. No completion marker.
- **References**: current status, architecture, models, testing, export scenario and feature plan updated; evidence in fine-atlas-integration, source-lowland-boundary and macro-stage-attribution.
- **Next defect isolated**: saved wetness computes a different expression from its documented log-ratio index, with a demonstrated ordering reversal; no wetness or tectonic variant was implemented.

## 2026-09-25 - Accepted fine terrain: production access and full 32K delivery
- **What**: preserve the user-approved recipe-2 terrain appearance and deliver a complete seed-42 500×1000 km world with normal Atlas exports.
- **Production**: `generate --terrain fine` and `generate_from_fine_source` orchestrate admitted source/world phases, five deterministic candidate gates and private staging. Legacy remains the default; the production MICRO command reproduces all 35 accepted files exactly.
- **Full output**: 15,522×32,768 PNG, 259,955,341 bytes, SHA-256 `b350076951b0148fa10a09be902ac909f94d001bf1f0c553317b782dc906a583`; actual overview and mountain close-up inspected.
- **Timing**: source 406.630 s, world 1,223.970 s, verification 2.574 s, render 193.293 s; sum 1,826.467 s (30m 26s). Single local release run; full peak RSS was not measured.
- **Verification**: all 44,826,624 exported heights and 500,000 continent heights equal the canonical field; receiving topology and saved tables validate; annual sources/sinks both 159,795,615,670,000 L. Additional small seeds 43/44 pass. Existing exact repeats, seam tests and scoped suites remain preserved.
- **Preservation**: all 523 original-world files, exact file set and 1,943,353,639 bytes unchanged.
- **Review**: final independent GPT-6 Sol delivery review finds no blocker; documented minor edge is an error returned for private-stage cleanup failure after successful world publication. No such error occurred in the recorded runs.
- **Limits**: broad uniform plains, straight river runs, dense small highland lakes, 100 m categorical water boundaries and rectangular rim segments remain. User acceptance ends further visual experiments; exact reference-image parity is not claimed. Complete-area export retains its existing partial-fringe omission.
- **Reference refresh**: current status, architecture, testing, operations, index and final feature checklist record the delivered state. Reproducible commands, checksums, runtimes, inspection images and audit are retained in `features/2026-09-23-terrain-corrections/evidence/accepted-look-qualification/`.

## 2026-09-24 - Exact native-mesh raster transfer diagnostic
- Verified all original pinned TIN faces and conditional builder edge insertion; the sole complete-mesh graph difference is an omitted fixed-hull edge, with no interior differences. Independently checked barycentric sample heights and retained original natural-neighbor field/image parity.
- On unchanged evolved heights and 100 m center locations, triangle-linear interpolation reduces central geometric sampled basin capacity4.180306→2.015581 km³ and maximum depth180.269→139.916 m. Native capacity0.020302 km³ / maximum6.399 m remains much smaller. These are not canonical annual-water results.
- A valid native minimax outlet route reaches183.79183 m maximum, but its contiguous pixel chain samples heights up to367.885 m. Actual Atlas changes local faceting without resolving dense ridge walls; no production adoption. Full reference-quality landforms remain open.
- Evidence: `features/2026-09-23-terrain-corrections/evidence/triangle-linear-transfer-control/`.

## 2026-09-24 - TIN temporal refinement control
A source-only synthetic TIN comparison held the dense cosine-fold source, n=1 λ=0 erosion, lowest-pass routing, uniform material and D=0.011 m²/year diffusion fixed for 5 Myr, comparing 100 × 50 kyr against 400 × 12.5 kyr. The coarse arm replays 12 retained diffusion fields and the actual Atlas exactly; independent source, bed, hull and geometric-basin checks pass. The refined field differs materially (native median absolute 16.87 m, p95 245.47 m) while actual Atlas still shows repeated rounded mountain bands. Native and raster geometric basin capacities both rise. This establishes timestep sensitivity, not convergence or reference-quality terrain. No production code changed or result adopted. Evidence: [time-control assessment](../features/2026-09-23-terrain-corrections/evidence/tin-time-refinement-control/root-assessment.md).

## 2026-09-24 - Threshold-operator causal control
- **What:** the isolated dense-source operator ablation completed before the preceding status-only turn, which made no implementation progress. The current continuation revalidated and retained that result before selecting a separate n=2 diagnostic.
- **Frozen scope:** same 148,992-site TIN, fold uplift, initial heights, lowest-pass routing, static area-weighted K and talus angle, and 100 × 50 kyr evolution. Only the extra erosion term above the talus threshold is removed; stream-power erosion remains.
- **Verification:** the slope-limited baseline matches prior uniform-material retained fields and actual Atlas exactly. Local lambda-zero regression, focused tests, release-smoke repeat, all-node identities, 100 step ledgers, and per-arm material metadata pass. Both arms complete in 9.779 seconds. Native/raster geometry and source/image hashes are retained under features/2026-09-23-terrain-corrections/evidence/slope-operator-control/.
- **Measured effect:** central native area within 1° of the 41.4874° threshold falls from 52.33% to 2.94%; above 1 km it falls from 71.86% to 3.38%. Native central geometric capacity rises from 0.1122 to 0.1288 km³, and 100 m raster capacity from 1.0675 to 4.3973 km³.
- **Actual visual decision:** Atlas shows more irregular crags, but much steeper terrain and narrow valleys. Reject production adoption. Geometric capacities are not canonical annual-water volumes; no water or full-world run was made.
- **Next bounded question:** a separate n=1 versus n=2 stream-power slope-exponent pair with fixed K, lambda=0, source and routing has since completed and is recorded in its own changelog/evidence. This was not a coefficient/angle search or a proposed final hillslope model.
- **Status:** production stays at 14a70b9; reference-quality landforms remain open. The active plan and latest open-items paragraph now reflect this result.

## 2026-09-24 - Terrain research before further variants
- Recorded the user's request to stop repeated local repairs and compare complete published approaches first.
- Audited previous experiments against the methods they cited; several were partial or modified pipelines, so their failures do not reject the full methods.
- Compared structural generation and multiscale amplification. The inspected multiscale code omits the paper's peak restoration and breaching stages; another unchanged preset run would not be a complete reproduction.
- Updated current status and plan with these gaps and the completed independent audit of the failed homotopy path.
- No new terrain generation, renderer change or production adoption. The reference appearance remains unmet.

## terrain-corrections/terrain-model-reassessment@2026-09-24

- Record user rejection of the latest experimental mountains as folded paper and stop the source-variant sequence; the remaining-7-km removal is not selected.
- Re-inspect existing same-renderer measured/generated and pre/post-production-erosion images. The renderer can express richer natural structure; production erosion damages it. The different experimental TIN's causal defect remains unresolved.
- Correct the stale status that the coarse-only comparison remains to be run. Require visual terrain-form acceptance before full-world adoption; numerical checks alone are insufficient.
- No new simulation, production change or saved-world replacement. Evidence: features/2026-09-23-terrain-corrections/evidence/terrain-model-reassessment/README.md. The reference target remains unmet.

## 2026-09-24 - tangent-trust-flow
- Replayed retained late flow proposals exactly and attributed almost all rejected-step model error to one interior cell's water-depth collapse and drag response. Independent baseline residual maps show distributed remaining error.
- Implemented and reviewed full log-depth/velocity tangent trust geometry with exact-water trial retraction. Three synthetic tests, branch checks and independent local replay pass.
- Fresh original-initial measured run reaches 60 accepted steps in 88 trials/119.17 s. Momentum scaled L2 is 90.90, 140× below the prior metric at the same step budget, but still fails the unchanged convergence target.
- Independently checked 61 saved states and all trial decisions, water conservation, final residuals and old-solver water agreement. No terrain update, new rendered mountain result, production adoption or original-world change.
- A single old momentum-map check still requires up to 20.43 m/s local velocity change; 452 cells miss its stability threshold. This localizes the next investigation to interior pockets despite the improved global residual.

## terrain-corrections/synthetic-source-controls@2026-09-24

- **What:** completed isolated synthetic source and fixed-lighting comparisons toward the reference-quality terrain goal; no production source changed.
- **Lighting:** a fixed 30° NW light improves sharp measured/detail faces but strengthens repeated gullies in the adopted world. Exact baseline PNG parity passes; adoption held.
- **Prior sources:** a read-only audit found no earlier synthetic candidate rejected solely for depressions; each also had visible morphology defects.
- **Synthetic control:** pinned fastlem generated a 76.8 km square with 30,000 interior sites and default uniform parameters. All five numerical outputs repeat exactly; literal heights reach 8.644 km and central median slope is 57.7°. Actual Atlas shows excessively steep, snow-dominated terrain; no adoption.
- **Slope control:** one 30° receiver-edge cap leaves sites and all other parameters unchanged. All numerical outputs repeat exactly. The central maximum falls to 5.572 km, but Atlas shows smooth uniform faces and repeated ridges. No adoption.
- **Audit:** the uniform cap is mathematically equivalent to local capped slope accumulation, but not under arbitrary heterogeneous parameters. Its 3 km² drainage-area crossover and 443 m average site spacing do not reproduce the cited three-regime hillslope model.
- **Verification scope:** root independently checked raw fields, integer conversion, repeat hashes and actual images. Neither candidate ran Arda canonical water or whole-world validation. Library convergence is unknown for these two controls.
- **Status:** production remains at `14a70b9`; reference-quality landforms remain open. The finer three-regime source is also complete and rejected for adoption after actual rendering.
- **Records:** `features/2026-09-23-terrain-corrections/evidence/{sharp-terrain-lighting,saleve-source-control,saleve-hillslope-control}/`, `synthetic-source-rejection-audit.md`, and `saleve-hillslope-model-audit.md`; current status and active plan refreshed.
- **Finer source:** separate continuous hillslope/colluvial/river geometry on 2,365,440 sites. The first run stopped before terrain on a strict boundary-count check; root traced 152 dropped sites to one-ulp fixed-axis interpolation overshoots. A minimal constant-axis repair restores all nodes and retains exact original small-control output.
- **Bounded result:** repaired run ends nonconverged at the soft runtime budget after 614 iterations, with maximum last height change 96.775 m. Root independently verifies all node-height recurrences and contributing-area ledgers. Atlas displays distributed branching hills but repeated wedge faces, narrow valleys and insufficient major landform variety. No production adoption or full-run repeat.
- **Further records:** `features/2026-09-23-terrain-corrections/evidence/saleve-three-regime/` retains frozen design, source/patch, independent preflight review, failed attempt, raw fields, actual images and root checks. No new fixed-integer or whole-world water claim follows from this float prototype.

## terrain-corrections/strict-fold-water-validation@2026-09-24

- **What:** completed the retained strict fold source’s first paired canonical local-water solve and actual lake-aware Atlas comparison. No production source or saved-world changes.
- **Harness:** minimally adapts the prior private harness so both arms use a common control-derived collar outside their exact 768² patches. Unlike the earlier exemplar, input rims differ; the candidate has an explicit interface step up to 67.241 m. Shared water code, one common climate realization, default limits and source heights remain unchanged.
- **Verification:** final independent review and retained hashes pass. Root decodes prepared height/rain/temperature, routing bed, solved lake fields, report arrays and annual ledgers. Both arms have identical raw forcing and exactly balanced 11,347,791,030,000-litre annual sources/sinks. The older exemplar’s lower land/lake ledger is explained by rainfall on its sea cells; raw climate forcing is byte-identical.
- **Result:** the single paired solve finishes in 47.93 s. Central candidate lake coverage is 7.5813% (19,874 cells), maximum depth 454.872 m and geometric water volume 10.58829343 km³, versus 2.3006%, 105.959 m and 0.95463054 km³ control. Candidate has fewer but larger central lakes: 1,518 touching IDs versus 2,313.
- **Boundary limits:** all central physical owners are inside the patch and central lake IDs have no collar cells. Nevertheless full-domain collar-touching lake outlet witnesses number 234 candidate versus 7 control. This is not full-world or boundary-independent validation.
- **Visual verdict:** actual unchanged Atlas preserves the improved rocky ridges but reveals scattered lake fragments and flooded inter-ridge hollows, with parallel bands/benches still apparent. Dry render controls match prior terrain-only outputs pixel-for-pixel. The exact prototype is not adopted; numerical water balance does not establish reference-quality terrain.
- **Next:** retain the useful source direction and attribute native-mesh versus raster depression capacity before choosing a corrective mechanism. No rainfall tuning, further source-coefficient sweep or full-world rerun follows this result.
- **Evidence:** `features/2026-09-23-terrain-corrections/evidence/fold-strict-water-validation/` includes reviewed source/binary, raw canonical outputs, extraction/check scripts, matched renders and archive hashes. Status and active plan are refreshed. Full mounds/hills/rocks/cliffs/mountains objective remains open.

### Completed native/raster attribution

An exact original-graph minimax flood finds no central native depression capacity in the smooth control. The strict candidate already has 263 central minima, 6.1516% of central Voronoi area below spill, 10.1808 km³ of capacity and maximum depth 451.676 m. Root validates every node’s spill level with an independent ascending-height connectivity algorithm. Native-site and raster area measures/outlet positions differ; no exact decomposition is claimed. The evidence separates intrinsic candidate basins from additional grid-conversion sinks. Both native valley formation and drainage-preserving raster transfer remain required; changing interpolation alone is insufficient. No further source or water run was performed.

## terrain-corrections/stratified-source-diagnostic@2026-09-24

- **What:** completed a controlled depth-dependent rock-layer source experiment toward the reference-quality terrain goal; no production source changes.
- **Contrast:** identical 148,992-node geometry, uplift, initial materials, outlets and 5 Myr duration; control freezes each node’s initial material, candidate changes material as erosion exposes prescribed folded layers. This differs from earlier static erodibility noise and homogeneous cover tests.
- **Method:** scratch float TIN source uses explicit uplift splitting and exact local erosion between material/talus events. It is a simplified graphical model, not the cited paper’s complete tectonic simulation, calibrated geology or the fixed-integer production generator.
- **Preflight:** corrected cancellation in event timing and added the missing integrated crossing gate; three independent high-precision oracles, five focused tests, no-crossing byte parity and a forced 128-node crossing case pass. Independent review found no blocking defect at frozen source9d5f23a3.
- **Verification:** both main arms complete100steps. Root checks all node bed identities, duration, material labels, local contributing-area balances and local-site-area-weighted volume ledgers; all26 main artifact hashes and6 frozen preflight hashes pass. Removed material is exported by assumption; no sediment transport is claimed.
- **Observed change:** candidate crosses86,431 contacts;53,779 nodes change active material. Crop slope>40° coverage rises6.79%→9.77%, median slope falls16.82°→12.95°. Eleven final uphill receiver edges remain, up to8.60m; no canonical water acceptance follows.
- **Visual verdict:** actual unchanged Atlas shows broader faces and sharper stepped slopes but retains broad smooth crests and repeated radial/parallel gullies. Reject adoption; no follow-up parameter sweep or full-world generation.
- **Records:** current status and local active plan refreshed; source, frozen design, numerical arrays, actual renders, independent reviews and root assessment retained under `features/2026-09-23-terrain-corrections/evidence/stratified-source-probe/`. The complete reference-quality objective remains open.

## terrain-corrections/spatial-refinement-control@2026-09-24

- **What:** completed one same-source actual mesh comparison,148,992→592,896 sites, toward the natural terrain-reference goal; no production adoption.
- **Controls:** exact saved continuous fold segments, same uplift formula/K/talus/lambda0/n1/D and400×12.5 kyr. Fine initial heights sample the saved coarse TIN, already height-gained once; recomputed clipped Voronoi weights pass graph and independent cotangent checks. Source quadrature differs+0.0927%; the unstructured meshes are not nested.
- **Repair:** first fine attempt stops at a scalar asymptotic zero-slope roundoff event. A narrow analytic-limit branch passes the captured regression,21 tests and independent80-digit oracle. Failed and repaired sources/binaries remain separate; no general tolerance change.
- **Replay:** repaired original-mesh run exactly reproduces24 producer-compared deterministic files and400 physics rows; root independently checks17 selected files and400 rows.
- **Fine result:**400 steps complete in186.76 s. Final source/bed errors≤2.48e−10 m; fixed hull exact; all ledger checks and independent final diffusion residual1.548e−7 m pass.
- **Visual result:** matched actual Atlas and native crops add sharp local cuts but worsen steep cellular ridge bands and do not supply broad connected valleys or distinct mountain masses. Reject adoption; no further resolution sweep.
- **Geometry:** central raster median slope38.694°→52.657°; native geometric basin capacity0.020302→0.474240 km³ and raster4.180306→11.337142 km³. More sampled summits do not prove fidelity. Basin geometry is not canonical annual water.
- **Scope:** original/wider physical presentations use identical cameras/render settings within each pair; coarse PNG parity and zero displayed-interior halo checks pass. No production, saved-world, convergence or full-reference completion claim.
- **Records:** active plan and current status refreshed; reproducible source/commands/receipts and root assessment retained in `features/2026-09-23-terrain-corrections/evidence/spatial-refinement-control/`.
- **Follow-through:** a read-only physical-area census locates steepening mainly below0.1 km² contributing area; previous gate experiments used other models. Its final-height slopes follow pre-final-step routes, so attribution remains descriptive. No threshold or further evolution selected.

## terrain-corrections/spatial-direction-control@2026-09-24

- **What:** compare original versus spatially rotated compression directions with identical primal-MST/complementary-dual construction, fine TIN, integrated uplift and colluvial evolution law.
- **Checks:** 23 scratch-runner tests and independent source/graph review pass. The new external-source input path exactly replays retained deterministic terrain files and all400 physical ledger rows before the two new400-step runs. Independent source, bed, fixed-hull, ledger and final diffusion checks pass for both.
- **Actual result:** matched2048 Atlas, fixed60-degree projection and native crop show varied ridge directions but persistent repeated cliff benches and smooth faces. Global peak2329.55→2320.45m; ≥150m prominent summits88→88. Neither arm is a production replacement or reference completion.
- **Next evidence:** only1.20%/0.31% of normalized uplift mass changes between7/24km blocks; the broad belt remains dominant. Mountain-mass allocation and face shaping remain unresolved, rather than calling for another angle/seed sweep.
- **Scope:** isolated synthetic float experiment. No production code, saved-world or canonical-water change. Full reference goal remains active.
- **Evidence:** features/2026-09-23-terrain-corrections/evidence/spatial-direction-source-control/, including frozen source, source/binary/input hashes, exact replay, actual renders and independent metrics.

## 2026-09-24 - simultaneous-flow-preflight
- Reviewed nonlinear soil-transport applicability and prior measured finite-cover failures; no new hillside law selected without a supported mechanism.
- Derived exact positive-flow one-cell and nodal two-cell controls; the previous relaxed iteration is stable there, so lagged depth alone does not explain the measured failure.
- Implemented an isolated full coupled water/momentum Newton fixture with five passing tests, including mixed interior flow reversals and failure checkpoint retention.
- Ran one unchanged measured 128×128 fixed-bed case. Line search fails at iteration 23 after 24.48 s, reducing the scaled residual only 2.815%; no qualified flow or terrain result.
- Independently reproduced final residuals and checked frozen sources/inputs and global accounting. No production change, original-world modification or new visual acceptance. Periodic actual-render updates remain a required part of progress reporting.

## 2026-09-24 - Rock-surface light-detail control
- **What:** close one isolated opt-in renderer comparison on identical saved terrain/water. Two deterministic smooth world-coordinate fields at nominal 400/800 m scales perturb only the land light normal, tapered by the original rock weight; palette, terrain, water, sun and existing constructors remain unchanged.
- **Source verification:** final isolated atlas.rs SHA-256 4cd42bc6a40f0214871c58bcf482098788cb59bfa01dc9e5dac66ad71a4f5790. An integer vector-cap boundary bug was corrected using squared magnitude and ceil square root. Thirty Atlas tests, strict renderer-library Clippy and independent source review pass. The detail-off neutral 2048-pixel control and detail-on repeat are byte-identical to their baselines.
- **Saved-world verification:** final saved-area runner SHA-256 682ad000c5a7f3b0d81b91b7cda0a1027baa5893cc0fa95b6d9aab7fb969b134. Its four area (5,4) 512/2048 detail-on/off PNG hashes reproduce the earlier source snapshot exactly. At 512 pixels, about 14.8% of saved-area output pixels change by at most two RGB levels; all 26,237 lake pixels are exact. This area has no sea pixels, so saved-crop sea immunity is not claimed.
- **Actual visual decision:** synthetic and saved-world/native crop inspection finds the fixed effect too subtle to establish convincing rock faces or better landforms. Reject production adoption. Display-normal detail cannot alter real cliffs, silhouettes, broad ridge bands, valley widths or basins. Smooth cubic fields contain harmonics; no band-limited or alias-free guarantee is claimed.
- **Scope:** one fixed setting only, with no amplitude sweep, full-world adoption, new source geometry or saved-world mutation. Evidence, binary/source snapshots, PNGs and hashes are retained under features/2026-09-23-terrain-corrections/evidence/rock-surface-detail-control/. Production stays at 14a70b9 and the reference-quality objective remains open.

## terrain-corrections/rock-albedo-control@2026-09-24

- **What:** completed one isolated exposed-rock colour comparison on saved production area (5,4), using a world-coordinate200 m cubic field, seed42, bounded±16 RGB levels and4×4 pixel-footprint quadrature. Physical terrain, normal/light, masks and water geometry stay fixed.
- **Implementation:** opt-in scratch Atlas exposure mask and colour-before-light sampler; existing saved-area channel path supplies actual pixel coordinates. A reviewed preliminary river-alpha suppression branch was removed so partial water coverage retains its original blend over detailed ground. No production source change.
- **Checks:**114 renderer tests, strict Clippy, independent code review,79 exact-rational field/footprint cases and enabled halo/interior RGB at512/2048 pass. Old/off PNGs and repeated candidates match exactly. Root confirms all523 saved-world files and26,237/419,792 saved lake pixels remain unchanged. The selected area has no sea; sea behavior has fixture coverage only.
- **Visual result:** actual native and fitted comparisons add neutral mottling, leaving smooth mountain faces, repeated incisions and broad soft crests. Reject this fixed material setting for adoption; no claim that all texture methods fail. The reference-quality objective remains open.
- **Evidence:** `features/2026-09-23-terrain-corrections/evidence/rock-albedo-control/root-assessment.md`, frozen source/binary, render receipts and root comparison metrics. No new full-world export or generator adoption.

## terrain-corrections/ridge-tree-preflight@2026-09-24

- **What:** graph-only source connectivity preflight on the retained dense7/3/1 km Poisson/Delaunay sites, faces and alignment energies. Selected-first Kruskal constructs full primal ridge trees; omitted edges form complementary dual trees with the outer face explicit.
- **Checks:** independent Prim selects exactly the same edges. All-face incidence, triangle nondegeneracy, primal/dual connectivity and zero cycle ranks pass. The three primal trees contain193/870/6863 edges; complementary duals contain375/1723/13703 edges.
- **Result:** exact-coordinate overlay still shows long near-parallel strands. The result does not establish varied mountain hierarchy, source-field validity or terrain appearance. No fine uplift field, terrain evolution, saved-world change or production adoption; the full reference objective remains open.
- **Evidence:** `features/2026-09-23-terrain-corrections/evidence/ridge-tree-source-control/`, including retained source, graph plots, receipts and independent root checks. Future source work must distinguish orientation/mass hierarchy from connectivity alone.

## 2026-09-24 - reduced-trust-flow
- Implemented and reviewed an isolated exact-water dogleg solver for the unchanged fixed-bed transport equations, including the full implicit depth response and true adjoint.
- Four small-field tests and independent first-step replay pass. Oversized trials are rejected before state mutation; the first accepted bounded step reduces squared momentum error by about 46%.
- The measured run takes 60 accepted steps in 94 trials/90.67 s, then fails its declared cap. Momentum error decreases 301-fold within the run but remains far above tolerance; late steps cycle between accepted small moves and rejected larger ones.
- Independently verified 61 saved states and all trial decisions, positive water balance, final residual field and agreement with the old water solver. No flow acceptance, terrain evolution, production change or new visual improvement.

## 2026-09-24 - Quadratic stream-power slope control
- **What:** close one predeclared n=1 versus n=2 stream-power slope-exponent pair after revalidating the previously completed threshold-operator control. The preceding status-only turn itself made no implementation progress.
- **Frozen scope:** identical dense fold uplift, 148,992-site TIN, lowest-pass routing, initial bed, static uniform K/talus, lambda=0 and 100 × 50 kyr schedule. The candidate changes only slope exponent 1→2; the same K makes rates equal at the declared dimensionless slope S=1. No exponent, K or angle search.
- **Verification:** six routing-library and 12 runner/scalar tests, exact repeated release smoke, analytic local n=2 checks and an independent source review pass. Both full arms complete in 12.129 seconds. The n=1 fields and Atlas image exactly replay the prior stream-power-only control. Root verifies all-node site/area/uplift/bed identities, material metadata and each step volume ledger.
- **Measured effect:** central raster slope p90/p99 falls 61.736°/71.986°→40.978°/52.413°. Native central geometric depression area rises 0.0677%→15.0366% and capacity 0.128764→47.499096 km³; raster capacity rises 4.397320→46.633620 km³. The candidate has 776 central native local minima.
- **Actual visual decision:** unchanged Atlas shows moderated extreme cliffs but smoother continuous ridge bands, reduced irregular crags and still-narrow valleys. Reject production adoption. Geometric capacity is not a canonical annual-water volume; no water or full-world run was made.
- **Retention and scope:** frozen source/patch, executable, full raw fields, step ledgers, actual renders, checks, archives and hashes are retained under features/2026-09-23-terrain-corrections/evidence/quadratic-slope-control/. The model still lacks hillslope transport and is not a calibrated generator. No further exponent/coefficient sweep follows. Production remains 14a70b9 and the reference-quality landform objective remains open.

## 2026-09-24 - Heightfield projection and zero-background uplift controls
- Completed an isolated true-depth 90°/60° presentation of retained 100 m terrain at physical 1× relief. Both 90° images match their neutral Atlas inputs pixel-for-pixel; independent analytic, depth and replay checks pass. The predeclared 1280² interiors have no clamped halo texture. The 60° view reveals raised faces but emphasizes repeated long walls and smooth platforms, so it is held from production.
- Completed one equal-total-uplift zero-background source pair on the same 148,992-node model for 100 × 50 kyr. All 12 baseline fields and the Atlas image replay exactly; independent source, bed and hull checks pass. The candidate worsens repeated high walls and raises central native geometric basin capacity from 0.305058 to 64.420222 km³ and raster capacity from 1.398812 to 80.854159 km³. Reject production adoption. No canonical water result is inferred.
- Both are isolated diagnostics. Production terrain and renderer remain unchanged; reference-quality mountains, cliffs, hills, rocks and mounds remain open. Evidence: `features/2026-09-23-terrain-corrections/evidence/heightfield-projection-control/` and `features/2026-09-23-terrain-corrections/evidence/zero-background-uplift-control/`.

## 2026-09-24 - Process-switch timing diagnostic
- Reconstructed common-bed rates and domains on retained terrain; tested tiny perturbations do not support widespread floating-point-noise classification chatter.
- Implemented one paired macro with fixed drainage, geometry and coefficients; isolated domain refresh frequency. All50 release tests pass; frozen original-step replay and paired first-substep fields are exact.
- Independently verified all28 substep domains, operators, bed/volume identities and final diffusion. Resolved one borderline seed mismatch by matching coordinate arithmetic in the independent verifier, without tolerance relaxation or raw output edits.
- Central mean loss rises20.989→25.433m with domain refresh; actual Atlas retains similar broad forms. Frequent switching and temporal convergence remain unresolved; no production adoption or reference-quality claim.
- Updated status and active-plan evidence. Original world and production code remain unchanged; procedural realism, mountain grandeur and final world-scale validation remain open.

## 2026-09-24 - process-interface-control
- Separated common-bed hillside potential and masked channel replacement at three saved states; independently checked all native edge rates, mask partitions, shape-rate integrals and prior rate means.
- Added a local analytic stencil showing a finite erosion-rate switch across a tiny above-roundoff crease change, without claiming widespread measured noise sensitivity.
- Selected one bounded maximum-process morphology intervention: less instantaneous sharpening competes with greater detachment. Scratch implementation and 56 tests pass review; original benchmark replays exactly and the new benchmark passes independent equation, hull, material and diffusion checks.
- Completed all 40 matched measured-input steps in 251.48 s; independently verified retained operators, fixed hull, cumulative material identities and all macro budgets. Actual unchanged Atlas shows fewer repeated incisions but broad smooth lobes and reduced mountain mass (central peak 2246 m versus binary 2652 m). Reject this candidate for production and mountain-grandeur acceptance. Production and saved world remain unchanged.

## 2026-09-24 - Preferred procedural source qualification
- Reproduced the user-preferred procedural input and two additional fixed seeds in fresh processes, then rendered all three with unchanged Arda Atlas.
- Verified that independent resolution changes move the terrain; a canonical world field is needed for consistent zoom and region queries.
- Measured and rendered loss of fine detail through 100 m storage; retained fine elevation remains required for the preferred close view.
- Audited production terrain, climate, river and publication handoffs; recorded unresolved basin/coast semantics without adopting an erosion or filling recipe.
- Preserved all 523 saved-world files and closed particle/hydraulic evidence. No production source change or final 32K delivery claim.

## 2026-09-24 - Prefer procedural input; preview stronger relief
Record the user's preference for the procedural input over the corrected particle erosion result, and their request for more daunting mountains. Preserve that source as the next visual baseline. Render one controlled comparison with 50% greater vertical relief, identical horizontal scale and unchanged Arda renderer. Both original millimetre render inputs reconstruct byte-exactly; source bytes are preserved. Taller render also changes height-driven snow coverage. Comparison awaits user judgement; no production adoption or full-world completion claim.

## 2026-09-24 - Preferred terrain: preserve physical scale in larger domains
Added a fixed-reference calibration and signed relief filter alongside the native-compatible integer source. Five 20/40/80 km cases repeat exactly and match the independent oracle within 1 mm. Actual Arda renders retain rugged detail that per-domain normalization flattened. Thirteen focused tests, Clippy and workspace checks pass. Original saved world and earlier evidence remain unchanged.

Normal world generation is not switched over. Macro/coast composition, drainage, fine persistence, full-world validation and final 32K delivery remain open. See `features/2026-09-23-terrain-corrections/evidence/physical-spectral-scale/README.md`.

## 2026-09-24 - Physical-scale renderer diagnostic
- Measured oversteep native geometry in the height-gained probe: above2 km median slope68.483°,89.10% of area steeper than45°. Along-crest uplift already varies substantially, so no generic crest modulation follows.
- Added a scratch-only explicit-spacing Atlas constructor. Default100 m image is byte-identical; the one483.4857455 m interpretation recalculates both slope material and light with unchanged heights/palette/sun.109 renderer tests and independent review pass.
- Actual nadir/60° comparisons look less wall-like at the wider physical extent, but mountain faces remain soft and reference quality is not reached. Correct physical camera scaling, prior projection parity, nadir image identity and unclamped shown interiors are verified.
- Full source crop51.2→247.545 km is a physical reinterpretation, not the same geography, new resolution or an evolution run. Production and saved world remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/physical-scale-render-control/`.

## 2026-09-24 - Hydraulic-reference and corrected particle diagnostic
- Verified the unchanged hydraulic demo creates negative sediment inventory at its first exchange, before slippage; stopped before a full landscape run.
- Reviewed Erodr and its cited thesis, demonstrated the downhill velocity-sign inconsistency and disclosed shared-bed races and lifetime sediment discard.
- Built an isolated serial velocity correction with native coefficients and explicit seed; retained raw output and separate boundary/lifetime sediment ledgers.
- Passed flat/slope preflights and fresh-process equality for a fixed procedural 512² input, then rendered actual matched Arda before/after full and central detail views.
- Root and independent visual review reject smoothing of inherited mounds and repeated directional cuts as the requested rugged mountain result. No coefficient, lifetime or source sweep and no production adoption.
- Updated current status and plan; preserved the preferred completed baseline and original saved world. The terrain and final 32K goal remain open.

## 2026-09-24 - Native mesh smooth-normal comparison
- Completed a scratch-only vertex-normal lighting comparison on the original frozen native TIN, with identical heights, material, camera and occlusion.
- Independently verified all rendered vertex gradients and signed normal numerators; prior sampled/flat-face output bytes and halo masks replay exactly.
- Vertex normals remove flat-face triangles but soften terrain and leave dense branching landforms unchanged. Reject production adoption; the reference-quality terrain goal remains open.
- Production code and saved-world fields remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/native-smooth-light-control/root-assessment.md`.

## 2026-09-24 - Native-face lighting comparison
- Completed a scratch-only comparison using identical native geometry, unlit material, physical scale and cameras; only sampled versus flat-face light changes.
- Independently verified all 296,446 face gradients and exact production light values, full nadir baseline reconstruction, image encodings and unclamped displayed interiors.
- Actual native-face lighting looks sharply triangulated across hills and mountains and does not resolve narrow valleys. Reject production adoption; the reference-quality landform goal remains open.
- Production code, saved world and full-world outputs remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/native-face-light-control/root-assessment.md`.

## terrain-corrections/mountain-mass-control@2026-09-24

- **What:** one equal-total source-strength comparison using the existing24km field on the frozen rotated ridge graph; same592,896-site mesh, initial bed, physical law and400-step schedule.
- **Checks:** independent all-node field/formula/budget/hull and final input hashes pass. All160 prior artifacts remain exact, permitting control reuse. The new run passes independent source duration, bed identity, fixed hull, all400 ledgers and final diffusion checks.
- **Actual result:** stronger mountain-group versus low-hill contrast, but taller repeated cliff benches and smooth faces. Central native area above2km0.421→70.91km²; raster slopep9956.62→69.60degrees and geometric basin capacity1.972→2.781km³.
- **Decision:** retain mass-allocation evidence, do not adopt this exact candidate. No production or saved-world change; no canonical-water or complete-reference claim.
- **Evidence:** features/2026-09-23-terrain-corrections/evidence/mountain-mass-source-control/, including matched Atlas/projection and two native windows, source/binary/input receipts, independent checks and assessment.

## terrain-corrections/mountain-grandeur@2026-09-24

- Record the user's requirement that mountains communicate grandeur: visibly taller peaks, substantial mass, deep valleys and a hierarchy through lower ridges and foothills.
- Require actual delivered-view evidence; numeric height, snow colour or taller folded walls alone cannot establish acceptance.
- Add features/2026-09-23-terrain-corrections/terrain-acceptance.md and an unchecked plan criterion; the full goal remains unmet.

## 2026-09-24 - Measured-input corrected-TIN diagnostic
Completed the isolated zero-uplift measured-terrain survival control with transfer verification, five snapshots, numerical checks, actual Atlas comparisons and independent Sol review. Repeated cuts and smooth steep faces develop without procedural uplift forcing. The process census motivates revisiting hillslope/channel treatment; it selects no new source or parameter variant. Production and the original saved world are unchanged. Mountain grandeur and full reference fidelity remain unmet. See features/2026-09-23-terrain-corrections/evidence/measured-tin-survival/root-assessment.md.

## 2026-09-24 - Matched material-resistance experiment
The prior corrected-scale output had appealing mountain relief but repetitive incision. A source audit and fixed-window saved-field comparison weakened coarse sampling as the main broad-shape cause. Feature-primitive research did not establish an automatic procedural replacement. The amplification paper supplied a specific, separately testable erosion-resistance mechanism.

Implemented one isolated 1024-stage comparison: original coefficient, a uniform resistance matched to initial total removal, and a fixed procedural spatial resistance. Retained kernels, source terrain and render settings are unchanged. All cases completed and original reference E/T/D outputs replay exactly. Independent material-map replay, frozen inputs, finite fields and rim checks pass. Actual Arda images show a modest interruption of grooves relative to the matched uniform case; broad folded faces remain. The user-liked full 2048 baseline is preserved.

Completed one unchanged 2048 continuation of both resistance arms, with the saved material field bilinearly transferred and no tuning sweep. Frozen-file, finite/rim and reference-render checks pass; all 523 original world files retain their hashes. Actual final images retain a modest interruption of grooves and mountain mass, but broad folded faces remain. A separate fixed 100 m storage test shows that much of the fine gully detail would be lost in the current saved terrain representation. No production code or original saved-world data changed. Reference-quality geometry, integration, seams, drainage, multiple seeds, runtime and final full 32K remain open.

Evidence: [material control](../features/2026-09-23-terrain-corrections/evidence/material-incision-control/README.md), [formation review](../features/2026-09-23-terrain-corrections/evidence/formation-structure-review/formation-construction-audit.md).

Final decision after independent review: the subtle complete-stage difference does not justify selecting this frozen resistance field. Close the candidate without a tuning sweep and retain the user's earlier baseline.

## 2026-09-24 - Material-band attribution and controlled terrain comparisons
- **What:** investigate repeated cliff bands in the improved dense lowest-pass terrain; the full reference-quality objective remains open.
- **Association check:** true 250 m contact crossings contain 63.72% of steep central TIN edges, while a fixed 125 m shifted contour contains 69.86%. Contact phase includes final height, so this association does not demonstrate a specific contact effect.
- **Frozen-initial control:** one selector change invokes the existing static material law. Both arms use identical dense uplift and lowest-pass routing. The pair completes in 13.235 s with exact layered baseline field/Atlas parity, zero static contacts, unchanged active IDs and passing ledgers. Actual Atlas worsens into broad smooth cliff walls; reject adoption.
- **Uniform control:** one static material uses initial non-outlet Voronoi-area-weighted K/talus means and unchanged lambda. It matches initial coefficient means, not nonlinear erosion or time-integrated exposure. Distinct ID2 and per-arm initial IDs prevent confusing uniform rock with hard/weak layers.
- **Verification:** existing six routing and nine scalar/runner tests pass. Repeated smoke fields match exactly. The uniform main pair completes in 12.853 s; root confirms mean formulas, baseline parity, zero contacts, constant active IDs, area/height/volume identities and unchanged Atlas baseline.
- **Uniform visual outcome:** repeated ridge walls and smooth faces persist despite one central native minimum and 0.06465% geometric fill area. Reject adoption. Fresh original-graph slopes place 52.33% of central area and 71.86% above 1 km within 1° of the 41.4874° talus threshold. This supports investigating slope saturation, not claiming its sole causality.
- **Decision:** close further material, angle, layer-thickness and coefficient sweeps. Keep layered lowest-pass terrain as the research baseline; investigate slope and ridge/valley geometry without repeating prior uniform slope-cap controls.
- **Scope:** production remains `14a70b9`; no original-world changes, full-world generation or new canonical water solve. Source archives, binaries, raw fields, actual renders, checks and hashes are retained under `features/2026-09-23-terrain-corrections/evidence/{material-band-attribution,fixed-material-source,uniform-material-source}`.
- **Reference refreshed:** `open-items.md` and the active feature plan record completed controls and visual rejection.

## 2026-09-24 - Lowest-pass terrain-routing diagnosis and controlled comparisons
- **What:** isolated source experiments identify and correct overly high basin reconnections; reference-quality terrain remains unfinished.
- **Diagnosis:** old routes exceed true minimum pass for 24,466 central strict-source sites (977.026 km²), median positive excess 317.622 m and maximum 1908.449 m. The only raw-versus-model mesh difference connects fixed hull outlets and changes no spill level.
- **Approach:** deterministic basin-pass spanning tree with original receiver-path reversal, immutable bed during routing, unchanged strict downhill ties and erosion law. This is a declared integrated-drainage landscape-evolution approximation, not annual physical flux.
- **Verification:** six small routing fixtures, nine runner tests, repeated smoke and independent all 148,992 node minimax/local-edge/root/acyclic checks pass. Both 100-step paired runs reproduce retained old source fields and Atlas PNGs exactly; root area/material/height ledgers pass.
- **Strict source:** native central geometric fill 6.1516% → 0.3184%, minima 263 → 14; raster fill 7.5813% → 2.7061%. Paired runtime 15.984 s.
- **Dense source:** unchanged previous source rechecked because the same routing defect affected it; native central fill 9.6284% → 0.3810%, minima 371 → 27; raster fill 11.0409% → 2.7287%. Paired runtime 15.840 s.
- **Visual outcome:** valleys connect more coherently, but repeated cliff/bench bands and narrow incisions remain. Neither source is adopted or meets the reference. Material-contact attribution and drainage-preserving raster transfer remain open.
- **Scope:** production remains at `14a70b9`; no saved-world changes, source coefficient sweeps, full-world regeneration or new canonical-water solve. Source, binaries, raw fields, actual Atlas images, scripts, reviews and hashes retained under `features/2026-09-23-terrain-corrections/evidence/{native-basin-route-audit,lowest-pass-source,lowest-pass-dense-source}`.
- **Reference refreshed:** `open-items.md`; active feature plan records completed checks and visual limits.

## 2026-09-24 - Linear fold-profile source control
Completed one isolated 100-step comparison replacing the dense source’s cosine top-to-bottom profile with a linear distance profile at the same area-weighted uplift. The baseline exactly replays retained n=1 plus diffusion fields; source, bed and solver checks pass. Native and raster geometric basin capacity fall, but actual Atlas and fixed projection still show broad rounded faces and repeated mountain bands. Hold production adoption; retain the cosine baseline. Evidence: [linear-fold-profile-control](../features/2026-09-23-terrain-corrections/evidence/linear-fold-profile-control/root-assessment.md).

## 2026-09-24 - Bank-only substep sensitivity
- Compared one versus eight bank-only passes inside the unchanged400 main erosion steps; lateral law and source remain exact. Independent review and31 tests pass.
- Replayed all18 prior deterministic one-pass files and400 physical/bank ledger rows exactly; all3200 candidate substep aggregates and field budgets pass.
- Smaller bank steps reduce unused virtual attack but increase expiry, remove15.4% less bed and give mixed interpolation-derived valley widths. Geometric basin capacity increases; actual render retains repetitive ridges and soft faces.
- Withhold adoption; no coupled convergence or reference-quality claim. Production and saved world unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/lateral-substep-control/root-assessment.md`.

## 2026-09-24 - Lateral bank erosion control
- Implemented one isolated undercutting–slump comparison using the retained native source and fixed published Kl/Kv ratio. Common-neighbor bank selection preserves a downstream connection at collapse; actual bed export is separate from virtual attack capacity.
- Exact baseline replay,29 focused/inherited tests and independent400-step bed checks pass. Native and sampled geometric basin capacities decline; fixed interpolation-derived near-channel width proxies increase.
- Actual rendered candidate opens some valley corridors but retains repeated ridge walls and soft mountain faces. Substantial summits survive with changed heights. Retain partial mechanism evidence; withhold production adoption.
- One-collapse-per-step treatment discards25.70% of virtual attack and reaches8.30 attack/threshold. Event-time sensitivity, resolved valley widths, whole-network valley length, full-world integration and reference fidelity remain unproved.
- Production and saved world remain unchanged. Evidence: `features/2026-09-23-terrain-corrections/evidence/lateral-bank-control/root-assessment.md`.

## 2026-09-24 - Deterministic preferred terrain source
- Ported the selected procedural source into repository-owned MT19937 phase generation, Q60 cosine tables and integer Fourier filtering; added reproducible Decimal table generation and an executable diagnostic consumer.
- Matched native square sources across three seeds and through 1024² within 1 mm, with byte-identical repeated Rust outputs and actual Arda render comparisons.
- Checked a declared rectangular physical-frequency extension, explicit resource admission and invalid input handling; passed nine targeted tests, generation all-target Clippy and workspace formatting/compilation.
- Normal world integration, physical-domain/coast composition, 64-bit world-seed mapping, fine-field persistence, drainage and final 32K delivery remain open. The saved original world is preserved.

## 2026-09-24 - Conservative hillslope diffusion control
- **What:** close one isolated native-TIN hillslope redistribution comparison while keeping the dense fold source, lowest-pass routing and n=1 lambda-zero stream-power erosion fixed. Candidate adds implicit signed finite-volume diffusion at D=0.011 m²/year after each stream-power update, with original hull heights fixed.
- **Preparation:** exact clipped Voronoi face conductances are retained and independently checked against graph membership and linear/cotangent cases. The solver uses positive symmetric edge weights, site-area masses, explicit boundary exchange and bounded PCG; six module and 18 runner tests pass.
- **Numerical correction:** the first full candidate stopped after step 1 when the true local equation residual reached 1.03378×10⁻⁶ m, just above its 1×10⁻⁶ m gate. It is retained as a failed numerical attempt, not a terrain result. Joint global/local PCG stopping with a 0.5×10⁻⁶ m margin then completed the pair without changing physical coefficients, conductance, timestep, iteration cap or final acceptance gate.
- **Verified run:** the corrected pair completes 100 steps/5 Myr in 14.588 seconds. The baseline replays nine raw fields and the actual Atlas PNG exactly. Root independently verifies all 100 volume ledgers, last-step matrix residual and routing-area recursion. Maximum per-step mass closure error is 0.07690 m³; cumulative signed diffusion storage −11,795,581,628.06932 m³ plus boundary export 11,795,581,629.193224 m³ leaves 1.12390 m³ numerical closure, within the declared gate. Last true nodal equation residual is 2.516×10⁻⁷ m.
- **Morphology:** central raster median/p99 slope falls 38.324°/71.986°→28.652°/63.583°. Raster geometric depression area/capacity falls 3.9093%/4.39732 km³→2.0351%/1.39881 km³, while native area/capacity rises 0.0677%/0.128764 km³→0.4128%/0.305058 km³ and native minima 1→109.
- **Actual visual decision:** Atlas shows rounder foothills, but smooth repeated mountain bands/walls and narrow incisions persist. Reject production adoption; retain the solver as diagnostic infrastructure only. Bed spill measures are not canonical annual lake water, and the single-surface model lacks separate sediment cover and physical channel–hillslope coupling. No D, exponent or source-coefficient sweep follows.
- **Retention/status:** source, patch, conductance weights, binary, corrected raw fields and ledgers, actual renders, failed-attempt receipt, independent reviews, archives and hashes are under features/2026-09-23-terrain-corrections/evidence/hillslope-diffusion-control/. Production remains 14a70b9 and the reference-quality terrain objective remains open.

## 2026-09-24 - hillside-shape-control
- Verified exact constant-speed hillside solutions and decreasing cone/bowl errors from the frozen kernel on three regular mesh spacings; independently checked raw fields and boundary reach.
- Evaluated measured-raster disk minima with explicit interpolation bounds and independent all-cell offset reductions; rendered unchanged Atlas.
- Completed a reviewed 40-step pure-hillside isolation on the actual irregular TIN in 188.62 s, preserving geometry, Ch, timestep and hull. Independent saved-endpoint/material checks and exact initial raster transfer pass.
- Actual Atlas confirms broad smooth mountain faces without channels or diffusion. Coupling adds 319.8 m average lowering, mostly through changed hillside response; it remains a combined channel/diffusion intervention.
- No production adoption or grandeur acceptance. Original saved world remains untouched; replacement mechanisms must address this demonstrated shape behavior and account for previous failed finite-cover/diffusion controls.

## 2026-09-24 - Hierarchy and multiscale diagnostic rejected
- Completed one frozen procedural hierarchy, multiscale erosion, height-restoration and drainage-repair diagnostic. No measured terrain was used.
- Independent checks confirm 70 restored raster controls and strict downhill paths after millimetre export. The successful export preserves the failed attempt's pre-export terrain exactly.
- Actual Arda stage renders fail the appearance gate: smooth rolled faces remain and breaching adds trenches. The last breach cuts up to 89.78 m.
- Retained coarse-interpolation and per-stage comparisons locate detail loss before the last fine-resolution erosion pass. The completed source audit identifies a physical-scale mismatch. A frozen one-step cone/pit test confirms it: central thermal transfer is about 41 times larger (corrected after verifying the author’s radius-based box constructor) at this experiment’s starting spacing. This does not establish a visual fix.
- Updated current status and plan. No production adoption or new full-world 32K result; the reference goal remains open.

Evidence: `features/2026-09-23-terrain-corrections/evidence/hierarchical-multiscale-pipeline/README.md`.

## terrain-corrections/fold-skeleton-source@2026-09-24

- **What:** completed one frozen multiscale ridge-forming uplift comparison through actual Atlas. This is an isolated source experiment; production and saved worlds remain unchanged.
- **Control:** identical 148,992-node mesh, initial heights, material contacts, erosion law, boundaries and 5 Myr duration. Spatial fold allocation at nominal 7/3/1 km scales is compared with a smooth source having equal total area-weighted uplift.
- **Preflight:** source geometry, scalar oracles, all-node rate formulas, exact rebuilt sites/areas, retained source/binary hashes, and unchanged material smoke are verified. Final independent runner review clears the exact selected source. The literal selected-edge rule creates dense polygonal networks; that limitation is declared before running.
- **Run:** both arms finish 100 steps in 17.08 seconds combined. Root all-node bed identities, material ids, contributing areas and ledgers pass. Complete fields and actual Atlas outputs are retained with hashes.
- **Visual result:** meaningful improvement from broad smooth principal crests and comb gullies to irregular broken ridges and rocky faces. Central conditional saddle-defined summits with at least 150 m prominence increase from 33 to 105; median slope stays near 23.8°. Counts do not establish visual acceptance.
- **Remaining defect:** cellular hollows and angular benches. A fresh exact TIN audit finds 587 candidate local minima versus zero control; a declared-boundary raster flood finds central geometric spill-fill coverage of 11.04% versus 2.18%, with maxima of 509.11 m versus 104.36 m. These describe geometric capacity, not available water or canonical lakes.
- **Decision:** retain the fold-source direction as real visual progress, withhold adoption of this dense variant, and diagnose ridge-graph connectivity. No equilibrium, fixed-integer determinism, Arda hydrology compatibility or full-world/reference completion is claimed.
- **Evidence:** `features/2026-09-23-terrain-corrections/evidence/fold-skeleton-source/`, especially `root-assessment.md`, `atlas-pair.jpg`, `root-field-check.json`, `root-shape-check.json`, `sink-audit.md`, and the frozen manifests. Current status and active plan are updated; the full objective remains open.

### Stricter edge-selection follow-up

The read-only source-graph audit reproduces frozen geometry and identifies 5,888 independent cycles at 1 km. Requiring lower energy than all four dual chords leaves 146 cycles. This explicitly alternative interpretation is tested in isolation, not presented as a publication correction. The reviewed implementation changes one predicate, preserving the runner and original mesh/base-uplift bytes. Independent geometry/field checks and all 34 preflight hashes pass.

The strict pair finishes 100 steps / 5 Myr in 11.51 seconds. Unchanged Atlas retains irregular rocky ridges with a more open pattern, but parallel bands, benches and hollows persist. Central saddle-defined peaks at least 150 m rise from 29 to 113 relative to its own smooth control. Geometric spill-fill coverage is 7.5813% versus 2.3006%, maximum depth 454.872 m versus 105.959 m. Its total uplift differs from the dense experiment, so direct cross-variant differences are contextual rather than a pure selection-rule effect.

Retain the improved source direction, hold production adoption, and stop graph-rule/parameter sweeps. Canonical drainage/water compatibility and lake-aware appearance remain unverified. Full-world integration and the full reference-quality objective remain open. Evidence is under `features/2026-09-23-terrain-corrections/evidence/fold-strict-source/`; current status and active plan include both outcomes.

## 2026-09-24 - Localized broad-face diagnosis
Mapped independently marked face boxes and cliff polylines to existing mesh constraints using the actual Atlas sample-center transform. Most marked lines track original ridges more closely than rivers; this is localization, not proof of a defective ridge model.

An exact original-mesh replay recovered pre-smoothed refined river heights omitted from earlier diagnostic metadata. The source uses these for initial Poisson interpolation while fixing river vertices to smoothed heights. The algebraic discrepancy has a crop median 0.488 m and exceeds 100 m at only 4 of 6,548 interior samples. It is not selected as the main broad-fold fix; corrected smoothing summaries remain close to previous values. No modified terrain, production code, or saved-world write occurred.

The user prefers the rugged preserved baseline over the smoother recipe. Status and plan now preserve that preference and distinguish the completed diagnostic from appearance acceptance. [Evidence](../features/2026-09-23-terrain-corrections/evidence/fold-control-localization/README.md).

## 2026-09-24 - flow-stagnation-witness
- Traced the retained measured Newton failure to one interior upwind donor reversal just below the smallest permitted trial; before crossing, the accurately solved Newton direction descends as predicted.
- Validated both limiting face derivatives independently. Both one-sided Newton directions depart into the opposite side, so no simple donor flip or average is adopted.
- Independently projected saved velocities to exact positive water balance and checked the reduced coupled derivative. The single full velocity trial is extreme and fails its gate; no reduced trajectory follows.
- Production and original world remain unchanged. No new terrain render or mountain-grandeur acceptance is claimed; current-status and feature-plan documentation record the specific numerical limitation.

## 2026-09-24 - Flow pockets and a nonlinear velocity-error control
Diagnosed opposing cell velocities that nearly cancel at shared faces, along with strongly unequal common/antisymmetric water and momentum sensitivity. Implemented an isolated implicit nonlinear-map objective with unchanged physical equations and depth-aware trust geometry; three small tests, measured derivative/local-step checks and independent review pass.

The fresh 128×128 control stops at 60 accepted steps/83 trials in 178.61 s without convergence. Maximum velocity-map error improves from the prior endpoint's 20.43 to 1.407 m/s, but raw scaled momentum error worsens from 90.90 to 1924.12 and errors become more widespread. All 61 saved states and 83 decisions independently reconcile. No terrain update, new render, production change or appearance acceptance.

Updated current status and feature plan. [Evidence](../features/2026-09-23-terrain-corrections/evidence/flow-pocket-diagnosis/root-assessment.md) retains the failed run, limitations and the next connection to an actual terrain test. The full reference-quality objective remains open.

## 2026-09-24 - Flow plateau and analytical-start preflight
Verified the terminal branch-aware experiment: 50 accepted steps, 53 trials,
345.02 seconds, stopped at its declared double plateau. Temporary face
constraints release, but both physical acceptance gates still fail. Retained
linear and pocket diagnostics distinguish this from a permanently frozen face
or a single closed-basin explanation.

Derived and evaluated an explicit zero-transport equilibrium and continuation
tangent for a proposed numerical path to the unchanged full equations. The
start agrees with the original initial velocity to roundoff. Three synthetic
fixtures verify the isolated transport-fraction grid, including exact original
residual/Jacobian parity at the target. A strict relative
tangent-precision probe exposes cancellation; backward error and the existing
linear-defect criterion pass. No positive-parameter step, path, erosion or new
render has run. The reference-quality terrain goal remains open.

All 523 original-world files and prior frozen experiment manifests verify
unchanged; production has no diff. Updated current status and the feature plan.
[Terminal evidence](../features/2026-09-23-terrain-corrections/evidence/flow-branch-aware/root-assessment.md),
[analytical preflight](../features/2026-09-23-terrain-corrections/evidence/flow-homotopy-preflight/root-assessment.md).

## 2026-09-24 - Analytical-start first flow step
- The first predeclared positive-transport step converges in three Newton corrections/2.61 seconds at lambda3.29776421748822e-5.
- Independent face/local-law checks verify all four retained states, the exact predictor, accepted steps and scaled water accounting. Final scaled residual2.054e-12 passes the unchanged1.28e-8 intermediate target.
- Implemented the exact continuation tangent; three small-field tests pass. Implemented and reviewed the bounded path to full transport; nine path/tangent tests pass. The declared run has finished unsuccessfully.
- No physical endpoint, terrain evolution, new render or production adoption is claimed. Updated current status and feature plan.

- The bounded continuation failed at its per-root retry cap after78accepted stages/93attempts/398.37seconds, at transport fraction0.012977557. The completed independent audit verifies all93attempts and422retained Newton states. Full flow and terrain remain unqualified; the branch is closed.

## 2026-09-24 - Flow continuation and switching-face diagnosis
Completed and independently verified the bounded dogleg continuation: 500
additional accepted steps, 614 trials, 1463.77 seconds, still unqualified.
A tested projected-CG step improves a matched local comparison, but its bounded
60-step control also fails convergence after stalling at a reversing upwind
face. All retained fields and decisions in both runs verify independently.

Fixed-state directional checks show that the collapsed radius is not proof of
stationarity. A single local face-tangent comparison prevents that reversal but
still fails actual acceptance while another face switches. No erosion, new
terrain render or production adoption follows. Updated current status and the
feature plan to replace the stale running checkpoint with terminal evidence.

[Continuation result](../features/2026-09-23-terrain-corrections/evidence/flow-map-continuation/root-assessment.md)
and [Krylov result and limits](../features/2026-09-23-terrain-corrections/evidence/flow-krylov-control/root-assessment.md).

## 2026-09-24 - facet-shape-attribution
- Added a read-only original-TIN shape decomposition at four retained measured-input snapshots, with plane, V-valley, orientation and linearity oracles.
- Independently reproduced height/gradient/jump identities, corrected positive-decile selection, weighted contributions and flattening projections. Hillside and channel effects interact; accounting does not establish a causal replacement.
- Inspected actual Atlas with component maps, a fixed cross-section and registered mesh-crease markers. No new evolved terrain, production adoption or mountain-grandeur acceptance.
- Corrected stale current-status statements about the procedural-only requirement and the adaptive transport branch's ten accepted updates before failure. Full terrain goal remains open.

## 2026-09-24 - Facet-retreat diagnostic
- Implemented and tested an isolated two-process harness: geometric channel creases and feasible two-dimensional hillslope retreat, with matched 28-substep control and exact old-mode replay.
- Completed both 40-step measured-input runs. Independent retained-operator, mask, boundary, diffusion and all-macro budget checks pass; 48 offline release tests pass.
- Inspected actual unchanged-Atlas images and signed changes. Candidate suppresses repeated cuts but smooths and lowers ridges more; central peak 2652 m versus control 2814 m, from 3844 m initially.
- Recorded severe process-domain turnover: median 329352 of 592896 sites change channel eligibility per macro step after the first. Correct mask implementation does not establish a physically stable domain rule.
- Rejected production adoption, updated current status and completed this bounded plan item. Natural mountain form, visible grandeur, procedural validation and final full-world acceptance remain open. Production and the original saved world are unchanged.

## 2026-09-24 - Evolved height and diffusion-ratio control
- Completed one isolated paired evolution with scaled initial terrain/uplift, fixed anchor K and 400×12.5 kyr. Only hillslope D differs. Exact source/hull/material checks and independent bed/matrix checks pass.
- The baseline reproduces the prior height-matched source through evolution to maximum native difference 7.584e−6 m, with identical final pre-update receivers. Stronger D restores broad repeated walls; actual Atlas, native crops and fixed 60-degree presentation reject adoption.
- Native geometric basin capacity rises 0.020302→0.248723 km³; raster capacity falls 4.180306→1.753432 km³. The large transfer discrepancy remains unresolved; these are not canonical water results.
- Production and the saved world remain unchanged. Reference-quality mounds, hills, rocky faces, cliffs and mountains remain open. Evidence: `features/2026-09-23-terrain-corrections/evidence/height-scale-diffusion-control/`.

## 2026-09-24 - Temporal refinement of process switching
- Traced two direct crease-feedback oscillators and one propagated-label oscillator on saved terrain; reconstructed their finite contrast changes from neighboring erosion, distinct from roundoff ambiguity.
- Implemented isolated28/56/112 refreshed-substep schedules with unchanged physical laws, routing, geometry and total duration.52 release tests pass;214 physical files replay exactly at28 steps.
- Independently verified every mask/operator/budget across196 substeps and all final diffusion solves. Average height differences decrease under refinement, while worst local differences remain nonmonotone.
- Bounded outlier audit links the largest local discrepancy to neighboring process-switch timing; only0.00569% of central area exceeds10m between56 and112 steps. This does not establish uniform convergence.
- Inspected actual matched Atlas, native crops and diagnostic plots. No meaningful mountain-form or grandeur improvement is accepted; no production adoption or new world generation.
- Updated current status and active-plan evidence. The physical formation question, procedural visual acceptance and final world-scale validation remain open.

## 2026-09-24 - Dendry native source qualification
- Tested one exact released third-teaser fixture after auditing its branching and local terrain-primitive construction; no measured terrain or parameter search.
- Mechanically extracted the original constructor and compiled unchanged core sources, retaining the released bool-valued control-range behavior. Independent adapter review passes.
- Generated two byte-identical 1024² double fields in 9.33/9.41 seconds and rendered fixed full/detail views through unchanged Arda Atlas.
- Independent visual review rejects repeated wedge-shaped masses and broad smooth faces as the final rugged mountain appearance. No added erosion, tuning or production adoption.
- Audited native slopes and perimeter-only spill; qualified the absence of ocean/outlet semantics. Fixed grid-boundary differences shrink with coordinate separation and reveal no material finite jump in that screen.
- Updated current status and the active plan. Preserved the preferred rugged baseline and original saved world; the complete terrain and 32K delivery goal remains open.

## terrain-corrections/coupled-incision-control@2026-09-24

- **Defect:** old uplift-then-exponential incision gives a timestep-dependent fixed-route equilibrium. Implement an isolated opt-in receiver-first backward-Euler correction with unchanged source, mesh, material, diffusion and timestep.
- **Checks:**32 offline scratch tests reported,7 independent exact-rational scalar cases, separate Sol review, exact old-mode400-step replay, and one completed candidate with saved-field source/bed/hull/ledger/final-diffusion checks. Actual control renders reproduce the prior files exactly.
- **Result:** native geometric depressions shrink substantially, but actual Atlas retains taller folded faces and repeated gullies. Peak4856→7672m. Retain the numerical correction for research; reject this terrain's production appearance adoption.
- **Scope:** no production code or original saved-world change, no new source/parameter sweep and no full-world completion claim. Evidence: features/2026-09-23-terrain-corrections/evidence/coupled-incision-control/. Full reference objective remains open.

## 2026-09-24 - Complete contour method qualification
Completed a fixed source-audited Figure 2 contour→erosion→thermal→deposition sequence through 1024, using original author GPU classes in an isolated headless executable. A small preflight diagnosed and declared an invalid thermal coordinate upload; its correction changes the noise origin compared with literal execution of the broken release. The source checkout and production code remain unchanged.

Two fresh processes completed in about 17.3 seconds, with exact retained terrain replay, byte-identical pre-erosion controls, finite fields, bilinear handoffs and range restoration verified. Actual Arda comparisons show substantially better branching valleys and rougher faces. Root and independent review still reject production adoption because of the central cliff/shelf ring, remaining repeated gullies and coastal cuts. This is evidence for the complete method's effect, not a finished world or a parameter-sweep invitation.

The original saved world retains all 523 hashes. Preserve the corrected-scale render that the user liked. Status and feature-plan completion markers now distinguish numerical qualification from unmet appearance/integration acceptance.

Evidence: [result and actual images](../features/2026-09-23-terrain-corrections/evidence/iso-contour-complete-pipeline/README.md).

## 2026-09-24 - Contour construction qualification
Built a headless adapter for a pinned, non-DEM author example from the 2026 iso-contour terrain paper. Source review corrected missing GUI height normalization and exterior fade before execution. An initial sampler abort was traced to signed-overflow optimization in the author RNG; an isolated `-fwrapv` build preserves the intended sequence, with full 100,000-value parity against the unoptimized probe.

Two fresh-process terrain runs complete in about 2.217 seconds each and reproduce graph, contour, mask and height data byte-for-byte. A separate explicit-mask Atlas runner passes original all-land PNG parity and renders fixed whole/central views. Root and independent visual review find broad polygonal terraces and a central mound, failing the declared broad mountain-form gate. Close this fixture without an amplification or parameter sweep; preserve the user's previous baseline.

No production code or original saved-world data changed. The reference-quality goal remains open. [Evidence and actual images](../features/2026-09-23-terrain-corrections/evidence/iso-contour-author-control/README.md).

## 2026-09-24 - Same-source complete erosion transfer
Applied the qualified complete author erosion/thermal/deposition recipe to the exact preserved 20 km mountain input through 2048, in a separate experimental driver. Two fresh runs complete in about 41 seconds with exact retained-field replay; input, handoffs, final range, frozen artifacts and unchanged original-world hashes pass verification. Matched actual Arda images reproduce the old source and baseline renders exactly.

The new result substantially reduces parallel comb gullies while retaining the mountain mass. Root and independent review keep this as a stronger processing candidate, but broad sheet-like faces and long cliff edges remain below final appearance acceptance. The comparison changes a whole processing recipe, not one isolated coefficient; coordinate origin, API correction and range restoration are disclosed. No production integration or original-world write occurred.

[Result and images](../features/2026-09-23-terrain-corrections/evidence/complete-erosion-transfer/README.md).


Subsequent user feedback supersedes the candidate ranking above: the preserved baseline looks “really cool,” while the new recipe looks “a bit like plastic.” Root withdraws the stronger-candidate selection and keeps the rugged baseline as the preferred visual direction. Numerical repeatability and fewer grooves do not justify replacing an appearance the user prefers. Neither image is a final world delivery.

## terrain-corrections/colluvial-domain-control@2026-09-24

- **What:** completed one isolated, paired low-area celerity test on the frozen592,896-site synthetic fine TIN. Below1 km² drainage area only, the candidate uses continuous `K√A0(A/A0)^0.24`; above that anchor, the original `K√A` law is unchanged. This transfers Steer2021 colluvial-valley geometry without its critical-hillslope branch, incision threshold or calibrated mass-wasting physics.
- **Controls:** same saved source, initial bed, mesh, hull, K, talus/lambda0, D, lowest-pass routing and400×12.5 kyr. Candidate changes only the local erosion coefficient while retaining actual drainage areas in routing and mass budgets. The frozen source passes22 tests including one-ulp anchor and independent closed-form scalar checks.
- **Replay and independent checks:** both arms finish400 steps under the retained guards; the baseline matches the prior fine run byte-for-byte in24 common deterministic files and400 physical ledger rows. Independent all-node source/bed/fixed-hull and final diffusion-matrix checks pass. [Independent paired result](../features/2026-09-23-terrain-corrections/evidence/colluvial-domain-control/independent-post-run.md).
- **Terrain result:** independent matched render shows some broader, greener connected valleys and fewer repetitive dark crags, but regular ridge benches and sharply reduced mountain mass remain. Central area above2 km drops154.27→1.108 km² and native ≥150 m summits1,052→93. Raster median slope falls52.66°→25.54°, and geometric raster basin capacity11.34→2.169 km³. These are geometry checks, not canonical annual-water results.
- **Decision:** partial valley improvement only; no production adoption, coefficient sweep, or full-reference completion claim. The original saved world remains unchanged. Root inspected the matched original-spacing Atlas and wider-spacing native crop and confirms the partial improvement and remaining repeated ridge benches; see the retained root assessment.

## 2026-09-24 - Coastal surface interpolation comparison
- Compared fixed geometric cubic against linear interpolation on the retained coastal constrained mesh, with the same anchors, coast blend,100m crop and actual neutral Arda renderer.
- Linear output matches the retained initial field byte-for-byte. Independent checks preserve connectivity and anchor heights; all frozen inputs verify.
- Actual Atlas reduces literal facets but leaves rounded mountain masses. Central depression area grows17.75→60.81km²; local height overshoot reaches97.09m. Reject adoption and skip erosion of this candidate.
- Original523-file world remains unchanged. Updated current status and feature plan; reference-quality terrain is not complete.

## terrain-corrections/coarse-source-control@2026-09-24

- **What:** remove persistent 3 km and 1 km source terms on the exact mountain-mass control, compensating total uplift with a derived scalar. Fine mesh, broad mass envelope, 7 km source, physical law and renderer stay fixed.
- **Checks:** all-site extended-precision formula/budget/hull checks and source identity fixtures pass; completed control artifacts and rendering are byte-exact. One 400-step run passes saved-field source/bed/hull/ledger/final-diffusion checks. Root completes work inline after Sol agents reach their usage limit.
- **Actual result:** large cliff bands persist, with longer and smoother faces. High-terrain area and slope distributions remain similar; basin measures are mixed. No production adoption or reference-completion claim.
- **Next evidence:** remaining 7 km forcing supplies 19.1093% of integrated uplift; no further source field or run is included here.
- **Evidence:** features/2026-09-23-terrain-corrections/evidence/coarse-source-control/. Production and original saved world are unchanged; the full reference target remains open.

## 2026-09-24 - Channel-domain and slope preflight
- Implemented an isolated full-TIN channel classifier and verified its outputs independently; actual inclined planes disprove its use as an erosion-domain rule.
- Added a facet-geometry alternative preflight that passes both planes, with remaining channel-initiation and endpoint limitations recorded.
- Implemented a standalone feasible downhill facet-gradient kernel with geometric stability guards; local tests and185 independent directional-oracle cases pass.
- Inspected fixed-input cast shadows, corrected a missing upstream crop halo and rejected this pass as a mountain-grandiosity solution.
- Updated current status and active-plan evidence. No production or saved-world changes; reference-quality terrain and final32K validation remain open.

## 2026-09-24 - Closed-cell minimum transfer diagnostic
- Implemented one exact triangle-square clipping transfer on hash-bound original native heights/faces; six fixtures, independent source review, independent262-cell intersection values and exact replay pass.
- Full sampled maximum geometric hollow depth falls139.916→3.546 m, below original native6.398876 m plus1 mm. Central capacity falls2.015581→0.00672595 km³; the former worst sampled hollow drains. This isolates narrow-outlet sampling loss, not canonical annual-water behavior or preservation of individual basins.
- Height semantics change substantially: full-grid median lowering30.281 m, central median61.04 m, maximum435.498 m; highest sampled peak falls261.299 m. Exact cell minima can erase true basin separations and volume. Actual Atlas retains dense walls and narrow valleys; reject a drop-in physical terrain replacement.
- Production and the saved world remain unchanged; full reference-quality landforms stay open. Evidence: `features/2026-09-23-terrain-corrections/evidence/cell-minimum-transfer-control/`.

## 2026-09-24 - Canonical fine terrain sampling
- Added immutable `TerrainField` and typed absolute `TerrainPoint` to arda-core, with validated coverage and integer bilinear interpolation retaining fine millimetre heights.
- Added a raw-input diagnostic consumer and verified exact shared-point consistency across nested resolutions, overlapping regions and repeated processes on three procedural sources.
- Preserved the preferred actual Arda appearance: all three full views remain pixel-identical, while close-view height differences stay within 1 mm.
- Passed 100 core tests, all-target core Clippy, workspace formatting and workspace compilation; independent review found no blocking defect.
- World source generation, fine-field persistence, production rendering, drainage and final 32K delivery remain incomplete. No saved-world migration or rewrite.

## 2026-09-24 - Direction-consistent flow proposal
Implemented an isolated derivative-only donor selector and temporary
face-tangent constraints to address the measured flow-reversal cycle. Physical
residuals remain unchanged and constraints are reselected for each proposal.
Nine candidate tests pass, including signed donor limits, release and exact
ordinary-CG parity where branch handling is unnecessary.

The bounded local check reproduces the prior rejected candidate exactly, then
accepts a smaller step through the existing trust-radius update. Independent
replay passes. Map error decreases slightly while raw momentum increases
slightly; neither terminal flow gate passes. A separately bounded continuation
completed from the exact accepted state and next radius, stopping at its
fifty-step double plateau after 53 trials/345.02 seconds. Independent checks
cover all 51 states and 53 decisions. The reversing-face constraint releases,
but raw momentum and map gates still fail.

No terrain update, new render or production adoption is claimed. Current status
and the feature plan record the remaining validation.
[Evidence](../features/2026-09-23-terrain-corrections/evidence/flow-branch-aware/CONTROL-DESIGN.md).

## 2026-09-24 - Corrected author extent and direct mesh control
The author constructor Box2(center,10000) means radius10km and width20km. Earlier audit prose incorrectly used width10km. Correct initial sample spacing is78.431m; the coarse64km diagnostic is6.425times coarser, with41.283times the thermal transfer per neighbor. The original39m analytic fixture remains preserved and must not be labelled the author scale.

A new20km control samples the retained procedural mesh directly around its highest-prominence peak. It preserves original heights and faces, then applies the unchanged four preset triples at256/512/1024/2048. Finite stages and rendered appearance are assessed separately. Original world and production code are unchanged.

The run completed all four levels in1016.185seconds of stage time and failed the visual gate. Actual overview/detail comparisons retain folded faces and comb-like gullies. No postprocessing or integration followed. Bounded released-shader comparisons check CPU arithmetic without establishing visual acceptance or long-run parity.

Research identifies a published optional hardness control absent from the released shader. A fixed read-only orientation check finds the incision marks mostly follow broad source slopes and show no strong excess of grid-axis alignment over existing detail. No hardness-noise variation or further terrain run was selected.

## 2026-09-24 - Historical channel-slope anchor K diagnostic
- Ran one isolated 400-step/5 Myr dense-fold TIN pair, changing only uniform stream-power K from the initial hard/weak mean 5.251946814e−7/year to 5.555555556e−6/year. The candidate derives from the historical 1 km²/9% channel-slope anchor at an explicitly hypothetical constant 0.5 mm/year uplift; it is not an observed terrain calibration or coefficient sweep.
- All 13 deterministic baseline outputs and 400 physical ledger rows match the retained time-refined control exactly. Both arms pass source, bed, fixed-hull, material and duration checks. Actual unchanged Atlas loses the distinctive rocky mountain range while making hills gentler; maximum height falls 4,345.801→898.848 m.
- A single post-evolution height-matched display check (factor 4.834857455317221 from the two native maxima) shows more irregular small crests, but still steep repeated walls, terraces and unresolved broad valleys. It is not a physical evolution or water result.
- Reject this fixed K as a full production replacement. Native geometric basin capacity falls 0.911870→0.004199 km³ while coverage rises; raster capacity falls 1.653344→0.864616 km³ while coverage rises. These are not canonical annual-water results. No production source, saved world or renderer changed, and the full landform-reference goal remains open. Evidence: `features/2026-09-23-terrain-corrections/evidence/anchor-k-control/`.

## 2026-09-23 — Operator isolation and transport replacement prototype

key: map/transport-replacement-diagnostic@14a70b9144ddd

- Replay the measured-terrain control exactly and test each single-operator removal; retain numerical and actual Atlas evidence. None resolves the coupled visual defects.
- Investigate a distinct momentum/transport model from primary2026 research, separating its equations from the current public implementations.
- Implement a dependency-free CPU characteristic solver and independently verify36 scalar constant-flow/decay cases, all conservation ledgers and exact repeatability. Five focused tests cover rectangular cells and stagnation. Independently reviewed analytic cell-average expectations yield0.0352% pooled relativeL2 at256samples/cell; subsequent component checks are recorded below; nonlinear terrain coupling remains open. Transport-only checks do not constitute terrain or rendering acceptance.
- Refresh current status and local plan. Production source, saved worlds and completion markers remain unchanged.
- Extend the independent solver to signed, spatially varying fields and affine flow: 52 oracle cases pass, with 0.0487% pooled relative L2 at 256 samples/cell and exact replay of all 36 frozen scalar cases.
- Verify explicit face-inlet flux and an SI channel component chain with conserved erosion/deposition/export, a serialized bed update, zero-erosion control and frozen signed momentum. Independent review is dry within this scope; nonlinear feedback and terrain appearance remain unfinished.
- Add a separately tested bilinear face inlet and close the water/velocity loop on a fixed-bed SI channel. Both predeclared grids converge against an independent conservative-law ODE; fine-grid maximum velocity/depth errors are 0.04277%/0.05967%, water flux is exact and a complete field/history repeat is byte-identical. Bed evolution and actual terrain appearance remain open.
- Complete ten annual changing-bed channel steps with conservative sediment feedback, zero-erosion control and exact bed/history repeat. Independent review is dry, with fixed-face response explicitly separated from terrain morphology.
- Freeze and render actual 128² terrain inputs. The first measured-terrain water solve exceeds the declared 300-second limit before any bed update; retain the failure and investigate accounted numerical termination separately from physical parameters.
- Diagnose and fix exact-zero-time cell-face ownership in an isolated research solver, retaining explicit bounded-tail accounting. Twelve focused tests and all88prior-field/ledger regressions pass; both128²initial terrain-water solves now complete without numerical/cap loss. Natural-terrain feedback remains a separate experiment.
- Test one frozen crest-segment source under matched coarse forcing. Added summits survive, but the actual Atlas comparison retains the unwanted repeated gullies; reject adoption despite improved summit topology.
- Execute the full first2Dcoupling trial: generated flow converges but its initial bed step fails the fixed5m limit; measured control fails runtime preflight. Retain all per-iteration evidence and unchanged beds; independent review verifies373 generated-arm hashes. Numerical timestep/cost and the original appearance goal remain open.
- Verify a separate conservative finite-volume transport method and run rate-selected terrain timesteps. Generated accepts 10 bed updates over 2,722.6 years, then fails flow convergence; measured flow diverges before any bed update. Independent review checks 4,170 solve-file hashes. Actual Atlas comparison shows no useful interior morphology change (core p90 0.169 m); reject production adoption and retain the failed evidence.
- Test implicit quadratic drag with independent local/channel/face-flux checks. The momentum equation converges internally, but the measured coupled hydro iteration still fails at its 100-iteration cap. Reject adoption and close this bounded numerical experiment without another terrain run.

## 2026-09-23 — Refresh terrain and render references
key: map/all@757b2ab

- **Trigger:** covered terrain/render source changed; implemented behavior and current verification replaced stale active claims. Historical candidate results remain dated.
- **Architecture:** `01-architecture.md` records Euclidean belts, coarse creep/fine incision, height-backed Atlas shore reconstruction and bounded context.
- **Models:** `02-models.md` adds AtlasTerrain heights and internal AtlasSea semantics; persisted schema remains unchanged.
- **Conventions:** `03-conventions.md` refreshes whole-crates lexical counts and current integer algorithms.
- **Data flow:** `04-data-flow.md` records the four fixed Atlas arrays (2,641,960 bytes), shared shoreline decision and corrected terrain stages.
- **Testing:** `06-testing.md` records correction controls, combined broad/focused/golden gates, two dry review rounds and actual-world evidence with explicit limits.
- **Operations:** `07-operations.md` records shoreline behavior, updated scratch shape and measured generation/32K costs.
- **Glossary:** `08-glossary.md` distinguishes coarse creep from fine incision and defines Atlas shore reconstruction.
- **Logic:** `logic/01-continent-generation.md`, `logic/02-area-generation.md`, `logic/04-export.md` and `logic/07-preview.md` absorb the corrected source behavior without rewriting historical designs.
- **Status/index:** `open-items.md` and `00-index.md` distinguish verified implementation from remaining landscape-naturalness acceptance; no completion marker is invented.
- **Dependencies:** `05-dependencies.md` reconciles the workspace Apache-2.0 license and deduplicated CI golden invocation; installed dependencies and lockfile are unchanged.
- **Absent:** no frontend or cross-repository interface became applicable.

## 2026-09-23 — Terrain and Atlas shoreline correction verification
key: fix/2026-09-23-terrain-corrections@757b2ab

- **Implemented:** exact Euclidean tectonic belts with fractional Q10 distance; preserved negative ocean-rim depth; 25 coarse hillslope-creep passes with incision left to shared 100 m evolution; guarded Atlas land/sea contours shared by colour, channel clipping and overview paths. Classic and stored formats remain compatible.
- **Integration:** persisted MICRO fields/rivers are tied to accepted attempt 2; actual convergent plates verify uplift beyond the immediate diffusion ring. Golden comparison and independent repeat generation pass after the authorized physical baseline update (32 changed fingerprints, two unchanged, no files added/removed).
- **Checks:** broad non-golden run 659 passed/8 ignored; final targeted tectonic/continent tests 18/5 passed; strict golden/repeat two passed; format, strict workspace Clippy, Rust 1.96.1 and cached dependency policy pass. Four existing dependency-policy warnings remain. No fresh remote or Windows CI pass is claimed.
- **Review:** round 1 raised three test suggestions, two accepted/fixed and one refuted; fresh rounds 2 and 3 found zero new actionable defects across contract, quality and verification lenses.
- **Actual outputs:** fixed seed42 small/default and MICRO99 worlds close annual budgets. The default has 118 lakes, 799,834 reaches and 2,123 crossings; audited crossing fields match canonical reaches. The 15,522×32,768 PNG repeats byte-identically in 65.65 s at 57,240 KiB peak RSS, with 523 saved-world files unchanged.
- **Visual boundary:** removing coarse incision reduces measured straight cuts; shared shores round land/sea steps. Broad smooth mountains, parallel valleys and the large physically supported default basin remain. No renderer texture, lake deletion, seed reroll or arbitrary terrain clipping hides them. The overall reference-quality objective remains open.
- **Retained evidence:** local ignored `features/2026-09-23-terrain-corrections/` contains plan/review state, world directories, commands, water audits and actual 8K/32K images; original worlds remain immutable baselines.

## 2026-09-23 — Approved structural relief integration and reference refresh

key: map/all@4ea2271cd809

- Source `4ea2271` installs the explicitly approved eight-file terrain/test change: globally sampled 8/4 km structural relief, 160 shared evolution steps, 640N initial logical work and reviewed deterministic fixture. Legacy tile evolution stays 40 steps; saved formats and the 53N terrain memory reservation remain unchanged.
- Verify exact installed source hashes against the independently reviewed proposal. Both root release golden tests pass (strict fingerprint and independent repeated generation; 192.10 s), with formatting and whitespace checks passing. The fixture changes 23 of 34 fingerprints with no added or removed paths. Retain the earlier 674-test, two-review and complete-world evidence without claiming a new broad root/remote CI run.
- Refresh `01-architecture.md`: structural initial bed, cached/direct sampling and shared duration.
- Refresh `02-models.md`: initial physical surface, absence of new saved fields and unchanged memory reservation.
- Refresh `04-data-flow.md`: structural stage, sign/coast behavior and 160 shared iterations. Correct an older halo-memory figure to the tested 16-byte optional sample layout (4,260,096 bytes for 516² samples).
- Refresh `06-testing.md`: physical gate/cache controls, full-world annual/seam checks, reviewed fixture, exact root integration and repeated 32K export evidence.
- Refresh `07-operations.md`: measured default logical work 4,685,132,212,727; separate current 54 min 16 s generation / 71.31 s export from prior 40-step measurements.
- Refresh `08-glossary.md`: structural relief, shared/legacy evolution separation and legacy-only edge taper.
- Absorb installed behavior into `logic/02-area-generation.md`: integer structural field, gate offsets/thresholds, permitted shallow-land crossings, direct/outside sampling and resource formula.
- Refresh `open-items.md` and `00-index.md`: explicit approval blocker resolved, source adoption complete, prior work retained, broader visual target open. Existing module map and absent frontend/interfaces remain unchanged.
- Retain the exact highland gate diagnosis and rejected isolated 10 km radius experiment. Weaker summit modulation is measured, but widening alone retains broad regular crests and adds ring-shaped forms and physical basins; no further generator change is adopted.
- Preserve all historical worlds and the user's unrelated deleted plan. Documentation remains unstaged in the continuing implementation run; no full-feature implemented marker, push or PR is made.

## 2026-09-23 — Natural Atlas river presentation

key: map/natural-rivers@92689f8a8142

- Source `92689f8` replaces hard Mid/Dark overview strokes with bounded saved-width variation, fractional bank coverage and a muted edge-to-core water-colour profile. Source routes, physical widths and water ownership remain unchanged; display shading is not measured channel depth. Maximum 14-row halos keep 256-row streaming bounded.
- Reject the width/palette-only intermediate candidate after native-pixel inspection: it still resembles an opaque strip. Accept the final water-profile treatment as a partial improvement in actual small, fitted 32K and native-pixel comparisons. Mountains and reference-quality acceptance remain open.
- Correct the zero-width synthetic discharge fallback to saturate at maximum display size when physical width exceeds u32; preserve Classic bytes.
- Verify103 renderer and 12 public export tests, strict renderer Clippy,formatting and diff checking. Two fresh final independent Sol reviews are dry.
- Verify two identical 15,522×32,768 outputs (326,768,717bytes; 80.75/80.68s), unchanged 523 saved files and byte-identicalClassic 2K. Exact commands,source hashes and render hashes remain in local evidence/natural-river-pass.
- Refresh rendering architecture, models, flow, operations, testing, export behavior, current status and index. Preserve prior implemented work; no whole-feature completion marker, push or PR.

## terrain-corrections/mountain-source-validation@2026-09-23

- Completed the single hillslope incision-area diagnostic: exact control replay and material balances pass; regular side-gullies remain. Rejected for adoption.
- Completed the narrow faceted-crest source diagnostic: added summits render as a regular row of pyramids. Rejected for adoption.
- Checked terrain-cast shadows on the sharper measured-detail prototype with analytic controls: only12 of262,144 central cells are occluded within the inspected northwest radius at the existing Atlas light. No shadow renderer selected.
- Corrected the measured-detail filter description: sigma800m is not a1km wavelength cutoff; the ideal high-pass half-amplitude wavelength is4.27km. Input/output terrain bytes are unchanged.
- Completed paired canonical basin and annual-water validation of the unchanged source prototype with explicit local boundaries and common climate forcing. Both annual ledgers close exactly; candidate central wet area is4.04%. Actual lake-aware Atlas rendering preserves dry-control pixels exactly. The candidate remains visually incomplete and is not adopted; full-world validity is not claimed.
- Updated the active feature checklist and current status; production remains at14a70b9 and the reference-quality goal stays open.

## 2026-09-23 — Measured terrain shape and erosion controls

key: map/measured-terrain-control@14a70b9144ddd

- Compare public measured Alpine heights with the generated crop through unchanged Atlas at100m with neutral metadata and real halos. Retain attribution and independent byte-identical resampling verification.
- Quantify63 saddle-defined prominent measured summits versus none generated; distinguish source geometry from renderer presentation.
- Apply unchanged160-step erosion with zero uplift as a declared control. Record exact rim, positive heights, hashes and actual visual loss of craggy detail with repeated grooves. Source structure and evolution both remain unresolved.
- Reject both frozen finite-cover controls on the measured input after actual four-way Atlas inspection: peak retention improves but repeated exaggerated gullies remain.
- Refresh current status and local plan; no production code, saved-world, fixture or completion-marker changes.

## 2026-09-23 — Landform source diagnostics

key: map/landform-source-diagnostics@92689f8a8142

- Reproduce the current continent stages and all 500,000 saved 1 km heights in an isolated collision-source control. Reject the single warp-Jacobian concentration variant: principal crest widths remain 7/23 km, with one summit above the declared 150 m prominence threshold. No fine run or production adoption follows.
- Audit saved lowland/highland samples independently. Structural relief is provably disabled at at least 54.2% of sampled coastal 1 km nodes and 28.8% of mixed-river nodes. These bounds describe the sampled nodes, not full map areas. Fine relief also vanishes on locally flat regional input.
- Record separate hill and cliff limitations: steep-face relaxation has no bedrock/resistance distinction, and the rock material can only display slopes already present. Neutral hillshade and actual Atlas views are both required for visual acceptance.
- Inspect the authors' MIT peak/saddle terrain reconstruction source at `c85d6ca393339f04e09097d6cc151c896763d271`. Record its omitted erosion stage, floating-point geometry and dense reconstruction matrices as constraints on direct reuse. No external implementation is installed in Arda.
- Update current status and the active correction plan. Preserve all installed terrain/river improvements and immutable saved worlds. The full reference-quality objective remains open.
- Reject a second controlled source-envelope trial with separated anchors and reduced inter-anchor uplift. It does not meet the predeclared multiple-summit gate; no fine/full-world run or production change follows. Source weighting does not implement the paper's explicit dual ridge/river construction.
- Verify the independent summit-prominence diagnostic against 2,000 random fields and eight targeted cases. The actual saved 100 m highland crop also retains one interior summit over the declared 150 m threshold; finite-crop prominence remains conditional on missing outside paths.

## 2026-09-23 — Threshold-incision landform diagnostic

key: map/incision-threshold-diagnostic@14a70b9144ddd

- Test one predeclared slope-and-area incision threshold,900 mm per model step, in the isolated finite-cover/bedrock experiment. Retain source/input/binary hashes and exact zero-threshold height/rock/cover/ledger replay.
- Verify224 independent rational scalar cases and all160 per-step material budgets. Preserve physical receiver selection, material transfers and fixed cells.
- Inspect actual Atlas outputs on the frozen graph and on the accepted structural-relief initial field, using neutral identical metadata and real height halos. Separate the material-only control from the threshold change.
- Reject adoption: cover retention improves but broad smooth surfaces and repetitive steep channel clusters remain; accepted-source summit topology remains one broad crest. No threshold retuning, production terrain change, saved-world change or fixture update follows.
- Update current status and the local feature plan. Retain both successful numerical checks and failed visual acceptance; the full landform objective remains open.

## 2026-09-23 — Connected Atlas overview rivers

key: map/connected-atlas-rivers@14a70b9144dd

- Integrate saved channel strips and junction hulls into Atlas world-overview exports as `14a70b9144dd`. Complete 3×3 saved context, source clipping and exact rectangular output spans keep geometry connected across publication boundaries.
- Use 8× physical display widths capped 10,000 dm only in the new overview path. Preserve physical area coverage, Classic, direct legacy Atlas APIs and saved formats.
- Fix dense low-quality union work with a 64-piece subdivision trigger, maximum depth 6 and unchanged 250M work budget; omit no channels. Retain failed then passing 512 export evidence.
- Verify 108 renderer/12 public export tests, formatting, strict scoped Clippy, analytic rectangle and dense GEOS fixtures, plus two independent final dry reviews.
- Repeat full 32K output byte-for-byte in 93.19/92.62 s. Preserve all 523 saved files and selected Classic/physical-area controls.
- Refresh architecture, models, data-flow, testing, operations, export behavior, index and current status. Root accepts a partial river-geometry improvement; D8 angles, faint fitted tributaries and mountain morphology remain open.

## 2026-09-23 — Whole-region coastal ridge reconstruction diagnostic

key: map/coastal-ridge-diagnostic@92689f8a8142

- Reject the prior fixed-crop outlet fit after independent verification: source/saddle caps and retained high support exits conflict by up to1,299.989m. No outlet-fit raster was produced.
- Recover the saved27-peak topology and millimetre raster with sparse river adjacency, removing quadratic matrices from the isolated author-code adaptation. Floating mesh coordinates differ by at most7.11e−15km; production code is unchanged.
- Measure the actual seed-42 mainland and distinguish exterior ocean from internal nonpositive pockets. Use complete500×1000km coarse support for one new926-peak procedural reconstruction with83 coastal graph terminals.
- Complete the sparse full-domain mesh in14.855s at1,404,820KiB peak RSS. Preserve original nonpositive bed and other-island heights in the1km output; verify zero uphill graph edges. Do not infer raster drainage or annual water validity from graph checks.
- Run the already byte-controlled unchanged160-step erosion kernel on the new100m highland crop. Retain29 conditional prominent interior summits and reduce the prior graph-insertion's severe crop depressions to a5.529m maximum residual depth. These are prototype comparisons, not production acceptance or a lake quota.
- Inspect actual neutral hillshades and record persistent smooth shoulders, repeated gullies, slope bands and unverified lowland/cliff quality. Keep the full natural-atlas objective open and production source/worlds unchanged.
- Refresh current status and the continuing local plan with completed failed/successful diagnostics. Preserve the user's existing unrelated plan deletion.
- Render both evolved highland fields through the current Atlas renderer with real height halos and identical neutral cell metadata. Record visible segmented peaks/passes, remaining straight terraces and repeated side gullies; do not present the diagnostic as a generated physical world.
- Reproduce the steep-face audit:677 initial coastal-graph cells above40° become zero after160 steps. Trace the blanket creep/talus rule and record a bounded finite-cover/bedrock hypothesis, with conservation and inherited-mesh-artifact gates; no variant or production material state is installed.

## 2026-09-23 — Material and constrained-mesh diagnostics

key: map/bedrock-and-mesh-diagnostics@92689f8a8142

- Complete one frozen-input finite-cover/bedrock experiment, eight focused tests, exact standard control replay and independent 160-step material-budget reconciliation. Reject adoption after actual Atlas inspection: new steep faces amplify terrace/gully repetition and 93.6146% of central cells lose their cover.
- Correct the earlier prototype description: the original meshing call did not enforce prescribed segments. Independently reproduce ignored and incompatible constraint cases; preserve prior evidence.
- Enable constrained meshing, use interpolated vertex heights, preserve subdivided constraint vertices and remove perturbation-induced graph crossings. Repeat the full reconstruction identically; all 48,622 segment interpolation checks pass.
- Evolve the corrected crop with the unchanged 160-step kernel (25.848776 s). Retain 30 conditional prominent summits, with 9.45 km² depressed central terrain and 6.480 m maximum spill depth. Inspect actual neutral-metadata Atlas output; reject adoption because smooth shoulders, terraces and repeated gullies remain.
- Update the local plan and current status. Production source, deterministic fixture and saved worlds remain unchanged. Full landform and natural-river acceptance remain open.
- Diagnose the remaining broad shelf: prototype valley and free-fill heights lower the northern shoulder hundreds of metres. Erosion removes exact mesh faceting but leaves 468 km² of connected low-slope terrain. Correct the scope claim: interior mainland macro heights are not preserved. Record a bounded graph-profile feasibility check before further rasterization.
- Complete the graph minimax feasibility diagnostic: exact minimum maximum bed deviation is 2,018.222 m. Root independently checks 157,948 path inequalities and every returned profile bound; graph orientation and saddle constraints force the discrepancy. Reject further height-formula-only variants on this graph; no raster or erosion follows.
- Complete instrumentation-only replay of the rejected material diagnostic with exact output and ledger hashes. Quantify broad low-area incision while distinguishing rock cutting from direct cover removal; retain the causal and unit limitations. No channel-initiation threshold or new terrain variant is selected.

## 2026-09-23 — Saved-wetness Atlas material and reference refresh

key: map/all@3f95fcca59a2

- Source `3f95fcc` uses saved drainage/slope wetness to tint Atlas land with Q12 weight `wetness×2048/(wetness+12)` toward RGB `[81,126,73]`, before existing rock/snow blending. Target and eight-neighbor halo samples carry the existing byte; the optional sample remains 16 bytes. Geometry, saved worlds, Classic and public options remain unchanged.
- Verify 98 renderer library tests, 12 public area-export tests, the final focused mixed-axis test, formatting and strict renderer Clippy. Two fresh independent GPT-6 Sol review rounds find no actionable defect; retain exact source hashes and commands in the local component receipt. These are scoped checks, not a new full-generation or remote CI run.
- Accept matched actual-world material comparisons as a partial improvement. Retain the invalid initial old-world comparison separately; adoption uses the corrected structural-relief world. No ecological cover or soil-moisture producer is claimed.
- Refresh `01-architecture.md`, `02-models.md`, `04-data-flow.md`, `07-operations.md` and `08-glossary.md` for saved-field flow, integer material ordering, fixed context size and display semantics.
- Refresh `06-testing.md` for exact tint, zero-wetness compatibility, cardinal/diagonal context, water isolation and mixed-axis buffered/streamed coverage. Absorb the shared area/overview behavior into `logic/04-export.md`.
- Refresh `open-items.md` and `00-index.md` to distinguish committed improvements from the open reference-quality objective. A replayed coarse-field diagnostic identifies persistent collision-source uplift as a contributor to smooth crests; its bounded convergence-magnitude prototype is rejected and is not a production feature. A subsequent adapted released multiscale-erosion preset is likewise rejected on the fixed crop; retain its source provenance, numerical controls and limitations without a production claim.
- Leave documentation unstaged during the continuing implementation run; no broad feature completion marker, push or PR is made.
- Verify actual repeated 15,522 × 32,768 output: 327,031,502 bytes, 76.16/76.05 s and 66,516/66,284 KiB child RSS. Both PNGs match exactly, all 523 world files remain unchanged, and root inspects the downsampled completed-map preview.

## 2026-09-23 - authorization: terrain and shoreline corrections
key: scope/2026-09-23-atlas-terrain-corrections@approved

- The user explicitly answered “yes include” to extending the Atlas continuation to terrain-generator and shoreline-geometry corrections. The former scope blocker is resolved; GPT-6 Sol execution continues.
- The first-pass restrictions on generation and categorical display ownership are superseded for the authorized corrections. Existing saved worlds remain immutable comparison baselines; storage/API compatibility, deterministic simulation, water authority, annual accounting, shared boundaries and bounded resources remain requirements.
- Implementation sequence: Euclidean tectonic belt distance; preservation of existing seafloor at the forced-ocean rim; evidence-backed erosion routing correction; consistent shoreline/mouth geometry; regenerated comparison worlds, physical tests, justified fingerprints and visual acceptance.
- No completed-feature or final visual acceptance claim is made by this authorization record.

## 2026-09-23 — Atlas overview river readability

key: map/render-readability@a9ce4fefd566

- Source `a9ce4fe` corrects Atlas overview river visibility at large export sizes using centered scale-dependent Mid/Dark symbols and a clearer blue palette. A bounded 3/4 chamfer field protects sea/lake/style ownership and shares buffered/streamed behavior; Classic retains its previous right/down symbol. Physical channel widths and saved worlds are unchanged.
- Verify 101 renderer and 12 public area-export tests, strict renderer Clippy, formatting and whitespace checks. Two fresh GPT-6 Sol reviews are dry; retain independent chamfer and band-seam controls. These are scoped checks, not new full-generation or remote CI claims.
- Verify repeated 15,522 × 32,768 exports: 325,652,273 bytes, 78.91/79.36 seconds, identical SHA-256. All 523 saved world files remain unchanged and the Classic 2K control is byte-identical. Inspect the actual fitted 32K preview and accept improved river readability.
- Reject the bounded mountain presentation probes: broad lighting adds little and 3× normal exaggeration with lower sun overemphasizes grooves. Retain their saved-height provenance and offline relighting limitations; install no mountain change.
- Refresh affected rendering models, architecture, flow, operations, testing and export behavior; update current status and index. Preserve earlier implemented work and the open mountain/reference-quality objective. No full-feature completion marker, push or PR.

## 2026-09-23 — Atlas land material readability

key: feat/atlas-land-materials@3c06909

- Added slope-derived exposed-rock colour, continuous rock-altitude blending, slope-weighted snow appearance and stronger bounded integer relief to Atlas land. Shared saved-world sampling, Classic, water ownership, formats and fixed rendering buffers remain unchanged.
- Verified 89 renderer tests, 56 facade/CLI tests, formatting and strict workspace Clippy. Two fresh independent review rounds found no actionable defects. A 32K export of the same default seed-42 world completed in 66.92 s; actual overview and matched regional renders were inspected.
- Refreshed architecture, models, data flow, tests, glossary, export behavior, map legend and current status. Coverage stamps reflect the committed renderer source; unchanged conventions/operations retain their existing contracts.
- The wider visual objective remains open: clearer materials do not resolve broad smooth mountains, parallel drainage, the large regional basin or missing ecological layers. Controlled terrain experiments remain isolated evidence, not production generator changes or completed visual acceptance.

## 2026-09-23 — Atlas lake depth across map scales

key: feat/atlas-lake-depth@0fc9666

- Added validated saved-surface-minus-bed lake colour to Atlas area and overview maps using the shared Atlas water-depth ramp. Membership, river precedence, world data, Classic output and existing constructor fallbacks are preserved; lake-aware constructors are additive.
- Kept context bounded: each halo sample is 16 bytes on the verified target, with a transient 2 MiB lake-depth lookup per area/neighbor. No output-sized depth buffer or new saved field is introduced.
- Verified 94 renderer tests, 12 facade area-export tests, 14 CLI tests, scoped strict Clippy, formatting and two independent dry code reviews. Added corner and buffered/streamed mixed-axis coverage after review. Actual matched clean-baseline images were inspected; Classic lake-margin output is byte-identical. Current 32K output completed in 69.34 s at 76,904 KiB peak child RSS.
- Refreshed architecture, models, data flow, tests, operations, glossary, export/preview behavior, map legend and current status. No full workspace or remote CI run is claimed for this rendering continuation.
- Full visual acceptance remains open. Controlled terrain experiments are isolated evidence; ordinary broad relief plus longer evolution shows a partial branching improvement on one crop but has no accepted production integration yet.

## 2026-09-23 — Structural candidate verification and resource documentation

key: docs/structural-candidate-progress@2026-09-23

- Update current status with isolated small42/MICRO99 terrain comparisons, named saved-water audits, 674 passing non-golden tests, two dry component reviews and all-file repeat equivalence. Default generation and golden verification subsequently completed as recorded below; source adoption and reference-quality acceptance remain pending.
- Record the changed lake floors/outlets and limits of that interpretation. Climate tint, cast-shadow diagnosis and doubled structural amplitude did not establish another useful production change.
- Correct current-production default logical work from 1,881,932,212,727 to 1,894,732,212,727. The old value omitted the earlier 256N regional-detail sampling increase. Derive this from the directly measured isolated candidate admission and its exact additional terrain work; all other admission source is unchanged.
- Update open-items.md and 07-operations.md. This adopts no candidate code and changes no resource limit, memory reservation, saved world or completed prior feature.
- Record passing independent release repeat, Rust 1.96.1, cached offline dependency policy and all-package Clippy. Keep the reviewed 23/34-file fingerprint change pending explicit adoption authorization; the stored fixture is unchanged.
- Retain the controlled single-flow accumulation rejection: verified control bytes and mass flow, but worse straight channels and terraces in actual terrain images. No production routing change.
- Record the inspected 1,600-step duration control: exact earlier checkpoints, substantial broad-relief change, but unsuitable angular morphology. No duration increase beyond the isolated 160-step candidate.
- Record completed default candidate generation (54 min 16 s), exact annual balance, 12,356 wet seam pairs and inspected actual overview/area panels. Production adoption and fingerprint update await explicit authorization after automatic approval review rejected source mutation; no candidate source is represented as installed.
- Verify the candidate full 32K overview and recovered repeat: byte-identical 15,522 × 32,768 PNGs, 71.31/71.40 s, all 523 saved world files unchanged. Preserve pending adoption and the open visual target.

## 2026-09-22 - plan: 2026-09-22-geographical-rendering-first-pass
key: plan/2026-09-22-geographical-rendering-first-pass@Q5

- `features/2026-09-22-geographical-rendering-first-pass/plan.md`: amend Tasks1–3 for exact per-pixel interpolation, eight-neighbor context and unchanged water masks.
- `features/2026-09-22-geographical-rendering-first-pass/plan-review.md`: record resolved API, test-fixture, owner-weight and independent Classic-baseline findings.
- User accepted interpolation as an addition to the concrete first-pass plan; proceed on the recorded codex branch with source-only task commits.
- Use GPT-6 Sol for subsequent agents as newly requested.

## 2026-09-22 - map: Atlas rendering reference refresh
key: map/all@342d03e55120-atlas-rendering

- Trigger: source changed across the renderer, facade and CLI; source stamp `342d03e55120`.
- `01-architecture.md`: styled export entry points, renderer/facade ownership, sequential neighbor reads and bounded sampling/streaming.
- `02-models.md`: in-memory MapStyle, AtlasNeighbor, AtlasHalo and AtlasTerrain, exact sampling and validation; persisted schemas remain unchanged.
- `04-data-flow.md`: style refusal, halo derivation, per-pixel interpolation, publication and failure paths.
- `06-testing.md`: focused tests, current Linux gates, independent Classic baseline and real-map evidence; preserves historical records.
- `07-operations.md`: CLI selection and legacy compatibility, actual 32K timings/RSS and bounded neighboring context.
- `logic/04-export.md`: absorbs style branches, eight-neighbor loading, class filtering, water authority and publication.
- `logic/07-preview.md`: absorbs Atlas preview and explicit-style conflicts with legacy `--px`.
- `mockup/03-export.md`: current user-visible choices, filenames and errors.
- `mockup/04-crate-api.md`: additive implemented styled APIs alongside explicitly retained future design.
- `logic/README.md`, `mockup/README.md`, `00-index.md`: companion/index descriptions reflect the delivered first pass.
- `open-items.md`, `implementation.md`, `README.md`, `../tactical-map-roadmap.md`: preserve delivered generation work and earlier queue; mark only the Atlas slice delivered, with ecology/human/tactical scope still open.
- `../map-legend.md`: separates Classic legend from Atlas relief/depth interpretation; neither elevation palette implies forest or simulated snow.
- Divergences: Atlas `--detail` intentionally uses the streamed 4096 standard filename; Classic retains the legacy `_detail` filename. The concept is a visual direction, not exact generated detail.
- Unchanged source topics and retained generation scenarios were left untouched; no frontend/UI chapter or cross-repository contract was introduced. This is a targeted reference refresh, not a claim that every unrelated reference stamp is current.
- Verification: source-citation spot checks, 147 resolved local links, 40 previously documented absent historical feature links, no unexpected missing links and a clean whitespace check; a separate Sol documentation review found three stale detail-filename descriptions, corrected before completion.

## 2026-09-22 - implement: geographical rendering first pass
key: implement/2026-09-22-geographical-rendering-first-pass@Q5

- What: add selectable natural shaded Atlas world/area PNGs for users of saved Arda worlds, retaining Classic as the default.
- Approach: derive fixed integer palette/light/class grids from saved heights and two-cell eight-neighbor context; class-filter and interpolate palette/light separately at output-pixel centers; reuse physical water validation, rows/bands and temporary-file publication.
- Rejected: resolution-only output adds no geographical information.
- Rejected: palette-only refresh does not supply the selected relief direction.
- Rejected: illustrated-fantasy direction was not selected.
- Rejected: generated artwork as final map cannot guarantee agreement with saved geography.
- Rejected: stored slope/aspect alone lacks complete boundary gradients.
- Rejected: fading shade at area borders risks artificial grid bands.
- Out of scope: terrain/erosion/coast/basin generation changes and saved codecs/golden hashes.
- Out of scope: forest/soil-moisture producers, farms, settlements, extents, roads, buildings and society.
- Out of scope: tactical vocabulary/WFC changes, assets, interiors, gameplay and browser/VTT delivery.
- Out of scope: seasonal water, exact invented concept detail, unrelated Windows line-ending repair and changing the default style.
- Task 1: deterministic Atlas halo, terrain derivation and exact sampler; commit `4dbebd149f6b`.
- Task 2: area/overview integration, water authority and bounded streaming; commit `f851ecac3aed`.
- Task 3: MapStyle and bounded style-aware facade exports; commit `238e7550ae21`.
- Task 4: CLI style selection, early refusals, preview compatibility and detail routing; commit `342d03e55120`.
- Task 5: retained real-map panel, native crops, independent Classic comparisons, repeat 32K exports, protected-path checks and reference absorption.
- Diff: `crates/arda-render/src/atlas.rs`, `atlas/tests.rs`, `carto.rs`, `channels.rs`, `channels/tests.rs`, `lib.rs`, `overview.rs`, `overview/streaming.rs`.
- Diff: `crates/arda/src/atlas.rs`, `export_quality.rs`, `lib.rs`, `crates/arda/tests/area_exports.rs`.
- Diff: `crates/arda-cli/src/main.rs`, `crates/arda-cli/tests/cli.rs`.
- Chapters refreshed: `01-architecture.md` gains Atlas composition and entry points.
- Chapters refreshed: `02-models.md` gains in-memory display/context records and validation.
- Chapters refreshed: `04-data-flow.md` gains per-output-pixel sampling and failure/publication flow.
- Chapters refreshed: `06-testing.md` gains dated tests, actual output and evidence boundaries.
- Chapters refreshed: `07-operations.md` gains style options, compatibility and observed resource cost.
- Scenarios absorbed: `logic/04-export.md` gains style/load/water/sampling/publication rules.
- Scenarios absorbed: `logic/07-preview.md` gains Atlas selection and explicit-style/legacy-pixel refusal.
- Surfaces absorbed: `mockup/03-export.md` gains current choices/errors; `mockup/04-crate-api.md` gains additive actual APIs without replacing future designs.
- Index/status: `00-index.md`, folder indexes, `open-items.md`, `implementation.md`, `README.md`, map legend and tactical roadmap reflect this delivered slice and preserve the larger queue.
- Verification: 648 workspace tests passed, 8 ignored, across 21 suites on Linux; formatting, strict Clippy, Rust 1.96.1 and dependency policy passed. Existing Windows LF/CRLF CI failure remains historical/open.
- Evidence: one retained base-generated seed-42 200×300 km world, matched 8K overview/five-area pairs, 512 Atlas area, repeated 32K area/overview, native relief/water/edge/corner/band crops and exact hashes.
- Compatibility: three independent pre-change executable comparisons preserve Classic PNG bytes; all 55 saved files and five selected JSON exports remain identical.
- Resources: repeated 32768² area PNGs took 31.06/31.35 s, peak 44312/43956 KiB; repeated 19661×32768 overviews took 29.57/29.81 s, peak 66604/66204 KiB, measured by Linux child wait4 rusage. Both image pairs were byte-identical; these are host observations.
- Visual verdict: clearer relief, earthy palette and smooth within-cell transitions accepted; saved 100 m shoreline steps and existing straight/parallel drainage forms remain. The broader multi-seed/terrain-realism and inhabited-map milestones stay open.
- Review loop: two consecutive full dry rounds, three fresh GPT-6 Sol lenses per round; zero findings confirmed/fixed/refuted, no source changes between rounds. Separate documentation review corrections were verified before wrap.
- Retention: local ignored `features/2026-09-22-geographical-rendering-first-pass/` keeps the spec, approved plan, review ledger, acceptance report and generated evidence. No push, PR, merge or release performed.

## 2026-09-22 - groom: 2026-09-22-geographical-rendering-first-pass
key: groom/2026-09-22-geographical-rendering-first-pass@Q5

- `features/2026-09-22-geographical-rendering-first-pass/spec.md`: add per-output-pixel palette and lighting interpolation with separate class-filtered sampling.
- Read two-cell edge/corner context from eight saved neighbors; preserve physical water masks, Classic behavior and bounded row/band rendering.
- Use bilinear pixel-center weights on enlarged axes and existing source footprints on reduced axes, including mixed scaling.
- Interpolation improves presentation only; the saved geographical source remains 100 m.

## 2026-09-22 - progress: Atlas visual fidelity
key: progress/2026-09-22-atlas-visual-fidelity@9a16883

- User objective remains active: finish the whole rendering feature until it meets the selected atlas reference; the completed first pass does not establish final visual acceptance.
- `crates/arda-render/src/atlas.rs`: clearer elevation colours and a piecewise saved-depth sea ramp replace the compressed linear sea palette; lighting, masks, Classic and APIs remain unchanged.
- `crates/arda-render/src/atlas/tests.rs`: verifies depth interpolation/joins/extremes and updates literal RGB records; preserves the adjacent-row corner regression oracle with a locally shifted planar fixture.
- Source increment: `9a16883`, 80 renderer tests passed, formatting and strict renderer Clippy passed; an independent Sol reviewer found no actionable issue.
- Real evidence: four isolated actual-CLI variants separate palette from lighting; production area/overview match the palette-only prototype, Classic matches its pre-Atlas hash, and all 55 input files remain unchanged.
- Rejected for production: stronger display exaggeration/lighting, because it accentuates the saved straight gullies without supplying reference-quality terrain.
- Diagnosis: the visible outer ocean strip matches the generator's two-coarse-cell forced ocean rim at approximately -1,050 m; the cause of straight terrestrial incision remains unproven.
- A fixed default seed-42 500×1000 km world generation was started to broaden evidence; no seed reroll, terrain code change or completed-world claim.
- `features/2026-09-22-atlas-visual-fidelity/goal-audit.md`: records requirement-by-requirement incompleteness, evidence and next actions; local probes, commands, sources, hashes and receipts are retained.
- Broader goal remains incomplete: coast/terrain fidelity, distracting boundary patterns, representative-world acceptance and user visual acceptance still require work. No implement-completion marker is written.

## 2026-09-22 - progress: Atlas terrain and shoreline diagnosis
key: progress/2026-09-22-atlas-visual-fidelity@terrain-stage-diagnosis

- `open-items.md`: updates OI-03 and OI-04 from direct saved-data comparisons; both remain open.
- Seed 42, 200×300 km: regenerated continent attempt 0 matches all 60,000 saved regional heights. Aligned valleys in area (2,0) are present before area evolution; final evolution deepens them. A second scratch probe captures the pre-erosion field and localizes appearance to the 25 continent-erosion iterations; 11 consecutive northward receivers explain one reproduced straight incision segment.
- The ocean contour diagnostic changes 4,894 of 1,048,576 pixels in a 6.4 km crop. It demonstrates smoother inferred geometry and the change to categorical land/water ownership; it is not a production implementation or topology guarantee.
- A diagnostic inventory measures every area in the retained world and selects representative altitude, slope, lake, river and mixed-coast views by recorded rules. Land count and maximum height are cross-checked against the independent JSON export for area (2,0).
- Production source and saved generation remain unchanged in this diagnostic increment. The broader visual objective remains active; no completion marker is written.

## 2026-09-22 - status: Atlas visual goal blocked on scope
key: status/2026-09-22-atlas-visual-fidelity@scope-block

- The requested visual goal remains unmet. The previous turn completed the larger-world gallery and diagnostics; it was progress, not a stalled or failed generation attempt.
- The unanswered scope decision has persisted through three consecutive goal turns. The approved first-pass R2/R6/G1 preserves terrain ownership and generation rules; the proposed corrections change those contracts.
- Generation and gallery jobs have completed successfully. The worktree was clean, all independent verification was complete, and no fresh user answer was present when revalidated. No generation restart or repeated test run was needed.
- The goal tool now reports `blocked`. This records a need for the user's scope decision, not completion, rejection of implemented work, or a request to pause.
- `open-items.md`: reflects the blocked continuation while preserving the completed first pass, committed palette refinement, test results, real galleries and unresolved visual findings.

## 2026-09-22 - progress: Atlas prototype results and workspace verification
key: progress/2026-09-22-atlas-visual-fidelity@probe-results

- Production palette source `9a16883`: `rtk cargo test --workspace --offline` passes 649 tests, 8 ignored across 21 suites; command wrapper elapsed 395.39 seconds. Production source remains unchanged by these experiments.
- Scratch ocean contour renderer: actual 2K/8K exports of saved area (2,1), matching coordinates and channel clipping; 81 scratch renderer tests pass. The 8K crop changes 3,292 of 1,048,576 pixels. Zero-height source samples force categorical fallback on 6,144 pixels under mixed quads. No production geometry or topology acceptance is claimed.
- Scratch continent routing experiment: identical pre-erosion field and unchanged erosion constants, but multiple-flow accumulation alone reduces measured local valley relief from 83.0 m to 16.2 m. Rejected as a correction because it loses the required valley shape.
- `open-items.md`: records the production verification, confirmed erosion-stage source of aligned gullies, rejected alternative, shoreline limitations and pending scope decision.
- Fixed seed-42 500×1000 km generation remains live; a separate follower waits for its terminal receipt, then inventories all areas and exports representative 8K Atlas views. It never restarts generation. Completion and visual inspection remain pending.
- The broader objective remains active. No implementation-completion marker is written.

## 2026-09-22 - progress: default-world Atlas inspection
key: progress/2026-09-22-atlas-visual-fidelity@default-world-evidence

- Existing seed-42 configured 500×1000 km generation completed once: 171 areas, exit 0, 1,510.31 seconds, peak RSS 1,848,684 KiB. No restart, reroll or generator modification.
- Retained world: 523 files, 1,913,421,396 bytes. All 171 area cell hashes match the pre-render inventory after gallery export. This is not a before/after claim for every other file.
- Production renderer source `9a16883`: real 3880×8192 overview and six 8192×8192 area views. Selection uses altitude, slope, lake/river cell counts and balanced coastline; a mixed lake-margin view supplements the retained all-lake maximum.
- World maximum 3,801.251 m establishes coverage of palette ranges missing from the earlier small fixture. Independent Sol inspection and root inspection find repeated grooves, stepped shores, thin rivers and a large angular interior lake. No final visual acceptance is claimed.
- The alpine image contains no channel at 255; broad pale relief is not evidence of RGB clipping. Across internal default-world coast edges, 9,679 of 45,792 touch a zero-height sea sample; inter-area edges are excluded from this diagnostic.
- A second scratch shoreline prototype uses categorical land/sea samples and rounds corners without zero-height fallback. Its small-world 8K crop changes 920 pixels; broad steps remain. Production source is unchanged by either shore prototype.
- A globally saturated deep-water palette was tested against fresh production renders and rejected: it reduces rim contrast but leaves straight terraces and loses deep-water colour variation. The current palette remains unchanged.
- `open-items.md`: records the completed default-world gallery and updates OI-02, OI-05 and OI-12; terrain/shoreline scope remains a pending user decision. The broader visual objective remains incomplete.

## 2026-09-22 - groom: geographical rendering first pass
key: groom/2026-09-22-geographical-rendering-first-pass@Q4

- `features/2026-09-22-geographical-rendering-first-pass/spec.md`: formalizes eight requirements, twelve behavior rules and six constraints for an additive natural shaded atlas over saved geography.
- `features/2026-09-22-geographical-rendering-first-pass/references/natural-atlas-concept.png`: retains the selected AI-generated style reference, explicitly distinct from real Arda evidence.
- User selected the first natural-atlas concept and accepted the render-first scope; delegated work uses GPT-5.6 Sol.
- Rejected illustrated-fantasy, resolution-only, palette-only and generated-artwork export approaches; saved edge slope alone cannot ensure seam continuity.
- Proposed implementation preserves Classic compatibility and uses cardinal saved-height borders, existing water authority and bounded streaming.
- New generation, ecology, settlement/road outlines, society, tactical scenes and unrelated Windows line-ending repair remain outside this pass.
- Actual code, real-map visual acceptance and concrete implementation-plan approval remain pending.

## 2026-09-22 - map: rendering feature prerequisite refresh
key: map/all@23dfb0999672-rendering-prerequisite

- `01-architecture.md`: source-drift refresh for quality exports, complete CLI branches, renderer payloads, bounded streaming and temporary-file publication; preserves delivered physical generation.
- `02-models.md`: source-drift refresh adds image-quality and streaming records, corrects exact overview limits and updates loading/render validation without changing format-4 definitions.
- `04-data-flow.md`: source-drift refresh replaces obsolete fixed-resolution/buffer-only export flow with quality bands/rows, uncached reads and explicit failure behavior.
- `06-testing.md`: source-drift refresh adds quality/streaming and publication test coverage while preserving dated verification sections unchanged; no tests rerun.
- `07-operations.md`: covered-source audit retains current quality/export contracts and records single-invocation workspace CI wiring and the dated Windows failure.
- `logic/07-preview.md`: source-drift refresh records rereads across 256-row bands and the cells-only overview handoff; existing quality/default/legacy behavior is preserved.
- `00-index.md`: records the new local rendering-feature working state; earlier feature-local galleries remain absent, not undone.
- User selected Sol subagents for the feature chain; GPT-6 Sol was rejected by the account, so three GPT-5.6 Sol agents supplied read-only audits and root composed the reference changes.
- Export logic and the export surface already describe current quality behavior; unrelated conventions/dependencies/glossary drift is outside this prerequisite refresh. No frontend or runtime interface was added.
- No production code, saved-world data, generation, rendering, approved golden hashes or runtime tests changed; the feature remains in grooming with its visual approach awaiting the user.

## 2026-09-22 - docs: detailed remaining-work queue
key: docs/open-items-detail@23dfb0999672-2026-09-22

- `docs/capstone/open-items.md`: adds 31 work items with 203 ordered steps, dependencies, code targets and completion gates; maps all 12 historical inventory items, original build stages and 18 companion roadmap steps.
- `docs/tactical-map-roadmap.md`: links the detailed queue and aligns delivery order with shared-layout finalization before publishing settlement outlines.
- `docs/capstone/implementation.md`: points the retained build stages to the detailed queue without discarding delivered work or historical plans.
- `docs/capstone/00-index.md`: describes the queue and the world/area/tactical roadmap as complementary references.
- Confirmed scope: richer geographical world rendering plus settlement outlines/main roads; area building outlines/local paths; detailed tactical scenes sharing the same geometry and identities.
- World/area output must be available before tactical interiors or furnishings; WFC remains one assembly stage and society binds to the shared layout.
- Current code already includes bounded terrain interpolation, shared annual water, lazy loading and 32K streaming exports; retained tasks build on those foundations.
- CI rechecked September 22: exact-HEAD run 34260968213 remains attempt 1 with the recorded Windows golden-text line-ending failure; no newer run or retry was found.
- Verification: three domain reviews, 31 task-structure checks, 63 local links/anchors, inventory/roadmap coverage and unchanged historical-appendix hash; documentation diff passes whitespace checks.
- No production changes, generation, rendering or runtime tests were performed; terrain fixes, content implementation and visual acceptance remain future work.

## 2026-09-21 - map: implementation status audit
key: map/status@23dfb0999672

- `README.md`: separates implemented physical-world/data/export/tooling capabilities from remaining content and model boundaries; records exact-commit CI outcome.
- `docs/capstone/open-items.md`: source-audited capability inventory at `23dfb0999672`; distinguishes basic cover from full vegetation, recorded successful checks from live CI, and historical visual findings from fresh observations.
- `docs/capstone/implementation.md`: credits delivered parts of steps 4, 5, 6, 9, 10 and 12; retires obsolete eager-loading, empty-overview, upstream-count-only discharge and simple-memory-cap claims.
- `docs/capstone/06-testing.md`: points to the current Windows golden-text line-ending failure without relabeling earlier successful evidence or restamping the historical inventory.
- `docs/capstone/00-index.md`: describes the audited status and reconciled plan; indexes the tactical roadmap and records the local feature evidence folder's absence in this checkout.
- `docs/tactical-map-roadmap.md`: expands the implemented baseline so tactical work builds on delivered terrain, climate, shared water, basic cover, storage, loading and 32K exports.
- CI run `34260968213`, attempt 1 at exact HEAD, passed Linux/macOS, lint, MSRV and dependency jobs; Windows golden text differed only by LF/CRLF in 34 otherwise matching fingerprint entries. No CI rerun or code fix was performed.
- Prior C06 and later export verification counts remain dated records; no generation, rendering or test suite was rerun. Missing local image/report files do not reverse committed implementation.
- Overall terrain visual acceptance, richer vegetation/human geography, complete tactical scenes and serving remain open. The static annual model's absent seasonal processes are scope boundaries.

## 2026-09-08 - snapshot: C06 correction and feature-code cleanup
key: snapshot/2026-09-07-area-water-terrain-realism@c06-committed-cleanup

- `ed4875d`: committed the local-relief terrain correction, resource admission and regressions under the Regional detail correction rule in `logic/02-area-generation.md`.
- `tests/golden/micro-42.txt`: user separately approved the exact C06 candidate; 24 fingerprints change, 10 remain, none are added/removed. Fresh comparison passes in 44.50 s; combined broad/focused/golden evidence covers 590 passing tests.
- `b0f93f2`: preserved the 16K capability in canonical `arda-render::OverviewRaster::new_exact` and `crates/arda/examples/export_world_16k.rs`, removing the need for a copied renderer under feature docs.
- Exact overview keeps the former adapter's 16,384-axis/134,217,728-total-pixel bounds; regular overview limits remain unchanged. The migrated example reproduces the C05 7,761×16,384 PNG byte for byte in 3.718 s.
- The exporter move passes 49 renderer tests (four new), two dry code reviews, workspace formatting, strict all-target/all-feature Clippy and the official Rust 1.96.1 check. It changes no generation rule or dependency.
- `docs/map-legend.html` and `docs/map-legend.md`: saved the accepted colour swatches as an offline standalone reference and documented elevation, depth and discharge semantics; linked from the reference index and operations.
- User explicitly authorized removal of feature-local Rust and then Python. Dependency audit found no production/workspace/CI source dependency on that folder; the offline forcing-table generator already lives under `tools/`.
- Removed 419 Rust source files, 70 Python scripts, 24 source patches, spike manifests, native helpers, bytecode and 16 Cargo build caches; removed 6,449,946,322 bytes. No Rust/Python source files remain under the feature folder.
- Preserved the feature folder, all 860 PNGs and non-code evidence. The local cleanup receipt records removed paths/sizes/source hashes; the feature README marks old replay commands as historical. Unique ignored spike sources were not Git-backed.
- Refreshed factual source stamps against `b0f93f22b969`, accepted-baseline status and the area/export scenarios; retained the earlier verification entries unchanged.
- Parallel drainage, angular shorelines and large rectangular regional basins remain open. Tactical assets and NPCs remain deferred; this snapshot does not mark the overall feature complete.

## 2026-09-08 - repeated lake-district correction verification
key: fix/2026-09-07-area-water-terrain-realism@lake-district-c06-verification

- Replaced absolute-altitude fine-detail amplitude with surrounding regional height differences, capped at 90 m; sea detail, noise octaves, annual water rules and rendering remain unchanged. Implements logic/02 Regional detail correction.
- Reused the admitted regional/uplift cache with canonical outside sampling; increased initial logical-work admission from 256N to 512N without another dense allocation.
- Added translation, flat/slope/bowl, continuity, numeric-bound, cache-equivalence and pre-output admission controls; retained real boundary fixtures and fixed statistical gates while replacing a diagnostic snapshot with independent exact inflow conservation.
- Two production-source review rounds and a subsequent independent fixture review are dry. The broad run plus focused repairs cover 589 passing tests; formatting, strict Clippy, official Rust 1.96.1 and the erosion budget pass.
- Completed five fixed worlds, 38 selected JSON checks and 238 exports; repeated exports match, saved worlds remain unchanged, all annual balances close exactly and all 20,712 both-wet adjacent pairs share IDs/surfaces.
- Reported seed436342 changes from 2,653 to 186 lakes and from 5,028.39 to 2,558.01 km² wet area; fixed 50 km squares with at least 50 anchors change from 18 to zero. These observations are not lake-count quotas.
- Preserved native 7,761×16,384 world and four 4096² area PNGs; default generation took 38–44 minutes, the reported 16K export 3.576 seconds and detailed areas 0.301–0.385 seconds each.
- Refreshed architecture, models, data flow, testing, operations, open items and area-generation logic. Retained all feature state, old candidates, images and evidence by explicit user instruction.
- Parallel drainage, angular shorelines, large rectangular regional basins and unmodeled geological/groundwater processes remain open. This is corrective verification, not the overall feature completion marker.
- The separate C06 MICRO42 candidate changes 24 fingerprints, leaves 10 unchanged and adds/removes no files; exact SHA256 3e86ce4c247244b65a6aa8bb6682204b41fa91208ad18f3cd7da70f7cc04e2eb. Explicit approval, final golden test and commit remain pending; the approved C05 golden is untouched.

## 2026-09-08 - snapshot: area-water-terrain implementation
key: snapshot/2026-09-07-area-water-terrain-realism@candidate05-approved-golden

- Committed the coordinated shared terrain, annual hydrology, format-4 storage, lazy readers and physical PNG/JSON exports as `2e79ca6`; kept the toolchain CI gate and reference documentation separate.
- `tests/golden/micro-42.txt`: user explicitly approved the verified candidate05 baseline, adding seven records and changing 26; three independent saved worlds match, and the fresh golden comparison passes in 43.82 s.
- `06-testing.md`: recorded 583 passing workspace checks plus the separately passing approved golden; retained ignored controls, verification scope and open visual acceptance.
- `07-operations.md`: recorded the world-only 7761×16384 PNG, 12.39 MB, 3.670 s, and 556,592 KiB peak RSS from the isolated native renderer; production render limits and all source-world hashes remain unchanged.
- `tools/generate_water_forcing_tables.py`: offline generation reproduces checked-in Rust constants; Python is not a production build or runtime dependency.
- Folded accumulated ledger fragments on main and refreshed factual source stamps against the committed implementation.
- Documentation check reports zero source-file drift and zero unfolded fragments; retained nine missing-version metadata warnings and eight scenario-heading false positives for the logic index, with the raw report preserved locally.
- Preserved the feature folder, rendered maps, review records and prior candidate evidence locally under the existing ignore rules.
- Natural visual acceptance remains open for repetitive ravines and regional lake shapes; tactical assets and NPC generation remain deferred. This commit records current work without marking the feature complete.

## 2026-09-08 - plan: 2026-09-07-area-water-terrain-realism
key: plan/2026-09-07-area-water-terrain-realism@Q15

- `features/2026-09-07-area-water-terrain-realism/plan.md`: user approved the reviewed26-task static terrain/water implementation plan with “ok then start implementing”.
- File map: core formats and identities; terrain/shared hydrology and generation; persisted global/area data; lazy facade loading; PNG/JSON renderer and CLI; existing regression tests and required evidence tools.
- Approved specification checksum: `126d6cc76437bb6590d84d095d7ec823b5c64328`; coverage includes12 requirements and13 behavior rules.
- Task1: Install one format-4 core and annual public schema.
- Task2: Install static hydrology types and annual support kernel.
- Task3: Correct regional interpolation and coastline signs.
- Task4: Correct incision and conserve hillslope contributions.
- Task5: Supply valid canonical annual climate inputs.
- Task6: Prepare and page the complete physical domain.
- Task7: Resolve fine ocean, flats, owners and actual saddles.
- Task8: Build and sort the witnessed physical MST.
- Task9: Persist the coalesced physical hierarchy.
- Task10: Bind spills to real source membership and receiving cells.
- Task11: Aggregate canonical cells into annual physical bands.
- Task12: Normalize supported physical transfers and global lake records.
- Task13: Route exact annual balances on the fine physical tree.
- Task14: Derive catchment, channel order, drainage and HAND.
- Task15: Extract real reaches, divergent endpoints and crossings.
- Task16: Build bounded global and exported-area record indices.
- Task17: Stream the canonical prepared annual source.
- Task18: Connect the actual shared solve.
- Task19: Compose one area from immutable shared state.
- Task20: Build standalone saved area contexts.
- Task21: Stream global tables and publish only completed output.
- Task22: Admit resources and wire the real public generator.
- Task23: Load saved layers only when requested.
- Task24: Render saved physical channels and expose explicit detail exports.
- Task25: Verify the integrated candidate and frozen natural panel.
- Task26: Absorb verified contracts and retain the feature evidence.
- Preserve original main checkout and pinned geography, deterministic integer results, read-only saved exports and clean-rerun failures.
- Preserve the entire feature folder, research, prototypes and baseline permanently; no feature cleanup deletion.
- Tactical assets/WFC/gameplay schemas remain separate; no seasonal simulation or groundwater programme is introduced.
- Required integrated natural/world/project checks remain pending; golden expectations have no new modification authorization.
- Execute locally with focused independent subagents where dependencies permit; retain usage checkpoints and all evidence.

## 2026-09-08 - fix verification: area water and terrain realism
key: fix/2026-09-07-area-water-terrain-realism@terrain-correction-verification

- Installed shared rectangular terrain evolution, implicit downstream-first incision, four resolved detail octaves and a radial coast mask; retained exact controlled regressions and all previous feature evidence.
- Added conservative terrain work to generation admission; current source has two consecutive dry review rounds and 567 passing workspace tests with the protected golden expectation excluded. Golden hashes remain unchanged.
- Completed five fixed worlds, 38 selected area JSON checks and 238 saved export commands; repeat exports match and saved worlds remain unchanged. The final binary independently reproduces candidate04 MICRO42's 34 saved-file fingerprints.
- Original reported area's wet rim falls from 50.10% to 4.89%, against 4.95% current interior; 87 wet border pairs share lake IDs and levels. The continuous water-square artifact is absent in the inspected current image.
- Default global lake totals change from 11,523/8,829/13,782 to 1,298/825/2,249 for seeds 42/7/436342; all three annual budget residuals are zero. These numerical observations do not establish natural abundance or morphology.
- Updated testing, operations and open-items references; retained current world/two-area/three-tactical gallery and PNG/JSON evidence under the preserved feature folder.
- Current default driver times are 45–50 minutes under three-world concurrency, with approximately 1.762 GiB peak RSS per process. Saved default area previews/details/JSON complete below one second each; future tactical assets and browser/network delivery are not measured.
- Feature remains incomplete: repetitive drainage, rectangular coarse basins, protected golden approval and final wrap remain open. This fragment is verification history, not an implement completion marker.

## 2026-09-08 - candidate05 correction verification and detailed renders
key: fix/2026-09-07-area-water-terrain-realism@candidate05-verification

- Corrected tectonic boundary classification using widened moved-center-normal motion and incident-kind union; preserved unrelated forcing, belt profiles and the documented warped-interface approximation.
- Corrected the legacy area water helper to read actual north/west outside neighbors and all four diagonal corners while preserving canonical edge identities; fixed three-context survey retains original thresholds and measures 11/13 large matches, median mismatch two cells.
- Verified primary shared prepared data and all 34 saved MICRO42 fingerprints are unchanged by the legacy-neighbor repair; added bundle samples fit existing admitted memory/work bounds.
- Completed 583 passing workspace tests with zero failures, eight ignored controls and only the protected golden expectation filtered. Formatting, strict Clippy, isolated official Rust 1.96.1 and both existing benchmark workloads pass; two current-source review rounds are dry.
- Completed all five frozen worlds, 38 selected area JSONs and 243 generation/export commands; repeated exports agree and source worlds remain unchanged. All three default annual budgets balance exactly.
- Audited all 513 published default areas; all 21,772 adjacent pairs wet on both sides match lake ID and surface. Reported area 3,10 now has 6.51% wet rim versus 6.45% interior, compared with the prior 50.10% rim concentration.
- Recorded default driver generation times of 36m52s, 43m07s and 47m26s under concurrent load, around 1.762 GiB RSS per process. Separate terrain/water/area/export stages are retained; the host clock discrepancy is disclosed.
- Produced a detailed 4608×9728 world PNG in 2.134s through the existing renderer, verifying all 523 saved-world file hashes unchanged. Delivery contains four 4096² area renders and excludes tactical images per the latest user instruction; earlier tactical artifacts remain preserved.
- Refreshed models, testing, operations, logic and open-items references. Preserved the complete feature folder, all prior candidates, failures and rejected experiments.
- Natural visual acceptance remains open for repetitive ravines, regional rectangular lakes and tiny-stream readability; the original area 6,13 is ocean in this candidate. Goldens remain unchanged and implemented remains false. This is a verification record, not a completed-feature marker.

## 2026-09-07 - groom: 2026-09-07-area-water-terrain-realism
key: groom/2026-09-07-area-water-terrain-realism@Q12

- `docs/capstone/features/2026-09-07-area-water-terrain-realism/spec.md`: approved twelve requirements for physical channel rendering, terrain/erosion diagnosis and correction, connected lake/river accounting, deterministic persistence and measured verification.
- Approved architecture amendment: prepare detailed terrain, resolve shared hydrology, then compose independent final areas; replaces the coarse-only single-pass input contract while retaining pinned terrain and deterministic outputs.
- Approved area output choice: add 4096×4096 detail with physical channel widths and retain 512×512 previews with separately measured readability marks.
- Physical-model scope: seasonal and subsurface comparisons are required planning investigations; the production model follows their demonstrated effect and supported inputs, without preselecting added simulation complexity.
- Fixed comparison panel: default seeds 436342, 42 and 7 plus MICRO 42 and 99, with 38 area cases frozen before tuning and identical-scale overview/area evidence.
- Rejected: renderer/threshold-only completion, coarse masks as proof of fine-scale continuity, regional warping or blanket breaching as assumed remedies, and universal lake-fraction/diagonal-flow targets.
- Verification requirements: physical units, connected basin membership and mass balance, actual tile rims/corners, deterministic order independence, required project checks, and measured shared-stage scaling before plan approval.
- Out of scope: tactical and society generation, unrelated rewrites, new checkout/branch; preserve the local ignored generation-issues.md and baseline artifacts.
- Compatibility follows the concrete persistence design; incompatible changes require format major 4. Explain intended golden-output changes before updating expectations; implementation still requires plan approval.

## 2026-09-07 - groom: 2026-09-07-area-water-terrain-realism
key: groom/2026-09-07-area-water-terrain-realism@Q14

- `features/2026-09-07-area-water-terrain-realism/spec.md`: replace the earlier scientific investigation obligations with the user's clarified static VTT terrain/water scope; retain fine topology, shared identity/flux, physical exports, determinism and required project checks.
- `features/2026-09-07-area-water-terrain-realism/history/spec-scientific-scope-history.md`: preserve the original approved specification as superseded history.
- `features/2026-09-07-area-water-terrain-realism/history/PAUSED.md` and `planning-progress.md`: record explicit resume and preserve the previous research checkpoint.
- Select representative annual water semantics; seasonal persistence, dated snapshots and periodic-state accuracy are not claims of the current feature.
- Exclude scientific seasonal calendars, dynamic groundwater reservoirs, litre-level return proofs and billion-cell convergence research from required completion.
- Keep tactical WFC/assets, gameplay collision/NPC schemas and society implementation for subsequent features.
- Reuse tested terrain, topology, climate, format, loader and renderer payloads where they support the revised scope.
- Retain Capstone task-based execution and usage checks, with a review point around 50% total account usage from the observed 33% starting point.
- Implementation-plan approval remains outstanding; production code and golden expectations remain unchanged.

## 2026-09-07 - feature: 2026-09-07-area-water-terrain-realism
key: feature/2026-09-07-area-water-terrain-realism@Q15

- `features/2026-09-07-area-water-terrain-realism/`: preserve the entire feature folder and all contents after completion, as explicitly required by the user.
- Override Capstone implement Phase D step 6 and completed-folder cleanup; completion does not authorize deleting or moving this directory.

## 2026-09-07 - map: local report policy
key: map/local-report-policy@987aeca04c77

- `docs/capstone/.gitignore`: ignore only the local generation-issues.md report at the user’s request; retain the file on disk.
- `docs/capstone/00-index.md`: mark the report optional and local only, without a link implying it ships in Git.
- User authorized committing the documentation refresh and version metadata, excluding the issue report; no push requested.

## 2026-09-07 - map: version metadata
key: map/version-metadata@987aeca04c77-6.4

- `docs/capstone/01-architecture.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/02-models.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/03-conventions.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/04-data-flow.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/05-dependencies.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/06-testing.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/07-operations.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/08-glossary.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/standards.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/implementation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/open-items.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup-artifact.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/generation-issues.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/01-continent-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/02-area-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/03-block-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/04-export.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/05-load-query.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/06-society-generation.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/logic/07-preview.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/01-generate.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/02-world-layout.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/03-export.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/04-crate-api.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/05-docker.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/06-serve.md`: stamped capstone_version 6.4, supplied by the user.
- `docs/capstone/mockup/README.md`: stamped capstone_version 6.4, supplied by the user.
- User supplied the installed version, resolving the prior missing-version warning without changing the skill installation.

## 2026-09-07 - map: all
key: map/all@987aeca04c77

- `docs/capstone/01-architecture.md`: Prescriptive-to-observed refresh: crate boundaries, complete command inventory and actual communication; planned serve retained.
- `docs/capstone/02-models.md`: Prescriptive-to-observed refresh: domain/working-state/DTO field tables, formats and validation; absent society types and unimplemented fields retained explicitly.
- `docs/capstone/03-conventions.md`: Prescriptive-to-observed refresh: enforced lints, counted casts/allow attributes, deterministic inputs and observed error fallbacks.
- `docs/capstone/04-data-flow.md`: Prescriptive-to-observed refresh: full batch, eager load and rendering paths; lake/WFC behavior and planned-vs-built divergences.
- `docs/capstone/05-dependencies.md`: Prescriptive-to-observed refresh: manifest/lockfile resolutions; tiny_http kept as picked but not installed, with original rationale.
- `docs/capstone/06-testing.md`: Prescriptive-to-observed refresh: structural/golden suites and ignored probes; planned statistical acceptance gates distinguished.
- `docs/capstone/07-operations.md`: Prescriptive-to-observed refresh: actual CLI, container, config and CI; serve/release automation retained as plans.
- `docs/capstone/08-glossary.md`: Prescriptive-to-observed refresh: current scales, hydrology, block sampling, vocabulary and format semantics.
- `docs/capstone/standards.md`: Migrated code-prefs.md and removed working-record provenance; preserved normative rules and left undecided domains explicit.
- `docs/capstone/implementation.md`: Repointed renamed standard and removed working-record provenance; retained build plan, status history, assumptions and constraints.
- `docs/capstone/open-items.md`: Removed working-record provenance only; existing diagnosis and deferred items were not re-reviewed.
- `docs/capstone/mockup-artifact.md`: Removed provenance from the generated wrapper; user-provided artifact body retained verbatim.
- `docs/capstone/logic/07-preview.md`: Filled the only missing implemented scenario: preview generation, eager load, overview rendering and partial-failure behavior.
- `docs/capstone/generation-issues.md`: User-requested separate diagnosis record: source behavior, two saved seed-436342 samples, earlier anisotropy measurements, unknown causes and planned society.
- `docs/capstone/.gitignore`: Initializer removed obsolete project-config ignore and added current UI preview/raster ignore rules.
- `docs/capstone/00-index.md`: Migrated root DESIGN.md, rebuilt module/topic/companion tables, indexed every final output and removed freshness columns.
- `docs/capstone/logic/01-continent-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/02-area-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/03-block-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/04-export.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/05-load-query.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/logic/06-society-generation.md`: Removed working-record provenance; retained chosen design and dated implementation history, with current-code pointers and unspecified exclusion metadata.
- `docs/capstone/mockup/01-generate.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/02-world-layout.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/03-export.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/04-crate-api.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/05-docker.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/06-serve.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- `docs/capstone/mockup/README.md`: Removed working-record provenance while retaining designed surfaces, rationale and assumptions; distinguished them from current CLI/API behavior.
- Refresh triggers: all eight topic chapters were prescriptive with tracked source present; final-output provenance and the index/standards shape also required migration. No topic chapter skipped as current.
- Absent: interfaces (no implemented sibling-repository contract) and uiux extraction (no frontend); CLI/API retained mockups remain indexed.
- Preserved decisions: synchronous future HTTP, handmade deterministic simulation, lazy future loading, full tactical continuity, future society and researched dependency choices.
- Unknown: installed skill manifest unavailable, so capstone_version omitted rather than guessed; project config personal keys left unchanged, ignored for expertise resolution.
- Validation: documentation schema/link/source-pointer checks only; no world generation, Rust-suite run, statistical sweep, source change or golden re-blessing.
- Cleanup: reused five verified draft chapters from the stopped rejected copy, then removed only /home/gent/.codex/worktrees/0a95/arda; saved checkout remains on main.

# Changelog

## 2026-08-27 — fix: open-items #12 and #4 investigation
key: fix/open-items-12-and-4@2026-08-27

- `crates/arda-gen/src/continent/hydrology.rs`, `continent/bundles.rs`, `area/mod.rs` — **open-items #12** gets the mechanism it needed: the continent tier now emits lake identity (`ContinentHydrology.basin_surface` — every 1 km cell inside a filled depression carries that depression's single surface, `NO_BASIN` outside), the bundle carries it as a `basin_km` patch, and a near-rim area basin takes its lake surface from a **nearest-cell** lookup of that constant (never interpolated: the field is piecewise constant, and bilinear blurring across depression edges would reintroduce the span dependence being removed). Two fragments of one depression now agree **exactly** — measured 0 mm against the real `compose` path, where the previous rule measured 723 mm. Components are grouped 8-connected, matching `erode::fill`'s own spillover, so grouping can never split a plateau the flood treated as one water body. Nothing is persisted: `FORMAT_VERSION` stays 3 and the golden fixture is byte-identical (MICRO has no near-rim basin over a continent depression).
- Recorded rather than overclaimed, after review: two residues remain, both unobserved. No fixture has yet produced a straddling basin whose cells see a continent depression at all (0 in a 4,280 seed/seam sweep), so the exact path is proven on constructed input rather than natural data; and a fragment whose rim abuts two *different* depressions takes a max of two constants, which is span-dependent again. Where the continent tier sees no depression — sub-kilometre pits invisible at 1 km — the older bilinear rule still applies.
- **open-items #4 (value-noise anisotropy): investigated, diagnosis corrected, no fix shipped.** The recorded "~17% diagonal flow" did not reproduce: measured 23.2% post-erosion / 38.1% pre-erosion. Zeroing the value-noise detail term entirely barely moves it (21.3%), which proves the detail noise is **not** the dominant source — `coarse_height`'s own separable smoothstep-bilinear sample of the 1 km grid is. Candidates measured: per-octave domain rotation (the previously "untried alternative") 23.9%, rotation + per-octave offset 23.8%, warping the detail term alone 23.4–24.0%, warping `coarse_height`'s sample position too 28.6%. The last one visibly reduces the combs at area zoom **but was rejected on the render**: at default size it speckles the coastline, scatters noise-like micro-lakes through the interior, and weakens the trunk hierarchy — the same class of regression that killed the first attempt, so it was reverted rather than shipped. Evidence now points at terrain shape, not noise isotropy: the combs are worst on near-planar mountain flanks where no drainage convergence can form. A real fix needs the erosion budget or the relief construction revisited — a feature with a spike, not a constant to tune.
- Docs: `open-items.md` (#12 restated precisely, #4's diagnosis corrected with the full candidate table), `02-models.md` (continent lake identity), `logic/02-area-generation.md` (the Invariants amendment now describes the shared-surface rule and its residues).

## 2026-08-27 (6b65f7c6a0c0..f56314f58bb2)

Architecture-level delta for feature 03, base = the head of the previous
ranged entry.

- **New module surface:** none — feature 03 added no crate and no module. `crates/arda-gen/src/continent/mod.rs` gained the `Continent { grid, climate, hydrology }` context and `build_continent`, which replaced `&ContinentGrid` in `bundle_for` and `generate_area`. That is the one architectural change: the continent tier's outputs now flow to the area tier instead of being computed and discarded at the write site.
- **Data contracts:** `TileBundle` reached its full `logic/01` step-10 shape (entering rivers, 53×53 rainfall/regime/routing-surface patches, wind octant, west-edge moisture). `ValidationStats` gained `river_count` (`#[serde(default)]`, additive). `fill::Basin` lost its `outlet` field and `spill_cell` was deleted once outlets moved to a tile-wide second pass. `FORMAT_VERSION` stays **3** — no binary layout changed anywhere (33 B cell record, 18 B overview record untouched), and the export JSON schema is unmodified.
- **Generation pipeline:** the continent tier's reroll ladder gained a second gate (≥1 river reaching the sea), ordered after the cheap land-fraction gate. The area tier's water stage changed rule, not shape: channel initiation moved from a 300-cell catchment to 40 L/s discharge, discharge became rainfall-driven, and entering rivers seed boundary cells with upstream catchment, discharge, and a Strahler floor.
- **Rendering:** `arda-render/src/carto.rs` gained order bands (three colours, 1/2/3 px widths, floor at order 3) and a widening pass; `arda-cli`'s generate footer reports the river count. No new dependency, no dependency-direction change.
- **Test surface:** `crates/arda-gen/tests/cross_tile.rs` (new) holds the cross-tile invariants; `drainage_invariants.rs` gained a lake-outlet sweep. The golden fixture was re-blessed twice on the branch — once for the feature's cell changes, once for a defect the review loop caught before merge.

Stale-topic check: `03-conventions.md`, `05-dependencies.md`,
`07-operations.md`, `08-glossary.md` still carry the 8d3c9d9 stamp. A `sync`
run would settle them; 08 in particular now lacks entries for entering river,
climate regime at the area tier, and the discharge-based watercourse rule.

## 2026-08-27 — implement: 03-climate-driven-refinement@Q9
key: implement/03-climate-driven-refinement@Q9

- Executed all nine plan tasks on `feat/climate-driven-refinement` (base 78bbff5, head 8be0a0a, 19 commits), subagent-driven with a per-task review gate: (1) expose the continent moisture store, (2) thread a `Continent { grid, climate, hydrology }` context to bundles and areas — a byte-inert refactor gated by the golden fixture staying green, (3) derive the step-10 bundle payload and entering rivers, (4) drive area discharge and channels from rainfall and inflow, (5) clamp seam lakes to the continent spill level, (6) gate generation on a sea-reaching river, (7) band river rendering by order, (8) the cross-tile invariant suite, (9) the golden re-bless. Source touched: `crates/arda-gen/src/{continent/{climate,mod,bundles}.rs, area/{water,mod,fill}.rs, orchestrator.rs}`, `crates/arda-core/src/formats/manifest.rs`, `crates/arda-render/src/carto.rs`, `crates/arda-cli/src/main.rs`, `crates/arda-gen/tests/{cross_tile,drainage_invariants}.rs`, `tests/golden/micro-42.txt`. 228 workspace tests green; clippy `-D warnings` and fmt clean.
- **Measured outcome:** max area-cell catchment at the default 500×1000 km size rose from 2,207 km² (capped by one tile's 2,621 km² ceiling) to **50,070 km²** — 19× the old ceiling and 5× the UK-scale major-basin reference open-items #1 cited. 3,335 entering rivers seed 171 tiles; the world yields 4 major river systems at seed 42 and 8 at seed 7.
- Review loop: **5 rounds to dry** (rounds 4 and 5 consecutive dry at 8be0a0a). 12 findings confirmed — 0 Critical, 4 Important, 8 Minor — all fixed (fa95cf4, f6f113a, 8b4d1ed, 8be0a0a) or accepted-and-recorded; 0 refuted. Two Important findings were defects in the loop's *own* earlier fixes, which is the case for requiring two dry rounds. The headline catch: discharge-driven initiation made channel-less land a designed regime, exposing that `hand()` conflated "is a channel" with "no channel below", so every land cell on an arid tile persisted as **Marsh** — the opposite of desert. 18,508 MICRO land cells were wrong; `hand` now returns a `u32::MAX` sentinel, restoring what feature 01's R9 already prescribed.
- Deviations from the approved plan, each measured before overruling: the plan's seam-lake BFS membership expansion flooded 20–88% of a tile, so membership stays local per §Q5's "minimal blast radius", plus a trim of cells at/above the clamped surface (down-clamps measured on 27 of 63 near-rim basins, 26 of them producing depth-0 drops); the plan's render floor of order 4 kept only 5.58% of channel cells, so cutoffs were calibrated to 3/5/7 (17.94%) against a real default-world histogram; invariant (b) ships as "EXISTS a tile with a whole-tile increase AND EVERY seeded path increases" because the literal single-tile form is false where an interior watershed dominates; `CHANNEL_THRESHOLD_CELLS` was deleted rather than retained once it became vestigial.
- **Claim corrected, not overstated:** open-items #12 (seam-straddling basins) is **materially improved, not exactly closed**. Both sides now derive the lake level from shared continent data instead of independent local spills, but each takes the maximum over its own rim cells, so fragments with different contact spans can still differ (synthetic case 723 mm; the spec's pre-approved fallback measured *worse* at 901 mm and was not adopted). A sweep of 4,280 (seed, seam) combinations found **zero** natural straddling pairs. Exact agreement needs continent-tier lake identity, which feature 02 §Q1 placed out of scope.
- Chapters refreshed to 8be0a0a: `01-architecture.md` (the `Continent` context closes feature 02's computes-and-discards seam), `02-models.md` (`river_count`, `Cell.rainfall`'s producer, the `u16::MAX` HAND sentinel, the full `TileBundle` payload with three fields documented as awaiting the vegetation stage), `04-data-flow.md` (continent tier ends at the step-9 gate; cross-tile inflow exists), `06-testing.md` (228 tests, measured calibration, recorded fixture limits), `open-items.md` (#1 and #5 closed, #12 reclassified, #4 named as the next target).
- Absorbed per the spec's Reference impact: `logic/01-continent-generation.md` — step 10 built (crossing geometry against the exact 100 m boundary line, the merge rule taking max-of-parts order, `g(catchment)` continuity) and step 9's river gate, with the loop-back double-count and the 53–94% seam alignment recorded as approximations; `logic/02-area-generation.md` — the water stage's rainfall/discharge/40 L/s initiation, entering seeds, seam-lake clamp and trim, and the Dry-not-Marsh correction, plus an amendment noting the straddling-lake case is improved but not exact; `mockup/01-generate.md` — the validation footer's river count. All stamped `absorbed_from`.
- Left open: open-items #4 (value-noise anisotropy) is now the most visible remaining river defect — at area zoom streams still run as straight lattice-aligned combs, which is what "rivers look like fjords" describes at close range; cross-tile continuity and hierarchy are fixed, individual stream *shape* is not. Also still open: `regime_km`/`wind`/`west_moisture` ride in the bundle awaiting the vegetation stage; naming (step 8) keeps `named_river_count` at 0; statistical gates remain step 12.

## 2026-08-27 — plan: 03-climate-driven-refinement@Q9
key: plan/03-climate-driven-refinement@Q9

- `docs/capstone/features/03-climate-driven-refinement/plan.md` — nine TDD tasks: (1) `ContinentClimate.moisture` output, (2) the `Continent { grid, climate, hydrology }` context threaded through `bundle_for`/`generate_area`/orchestrator as a byte-inert refactor gated by the golden fixture staying green, (3) the step-10 bundle payload with the complete entering-river derivation (100 m boundary-line geometry, corner tie to the horizontal line, all-sea window drop, BTreeMap merge, `entering_order` floor-log map) plus an independent-derivation sum test, (4) area rainfall sampling, drainage/discharge/order seeding, u64 rain accumulation, 40 L/s initiation, compose rewiring, (5) seam-lake clamp with the pre-approved fallback path, (6) step-9 river gate + serde-defaulted `river_count`, (7) order-banded rendering with a widening pass, (8) `tests/cross_tile.rs` R12 invariants with skeleton + verbatim (b)/(e) bodies, (9) the single authorized golden re-bless with a continent-layer-must-not-drift checklist. File map spans arda-gen, one arda-core field, carto, the CLI footer, both benches.
- Review passes fixed five drafting defects before the gate: a junk assertion and an elided predicate in the crossing task (now complete code), an all-sea-window false-failure in the seam sum test, `compose`'s signature changing twice across tasks, a u16 constant-eval overflow in the skeleton, and a missing coverage row for the seed-drains-back unhappy path.
- Constraints pinned: golden green through Task 3 and red exactly Tasks 4–8; `FORMAT_VERSION` stays 3; `named_river_count` stays 0; no RNG/float/hash-order in the new sim paths (BTreeMap, integer arithmetic); exact constants (≥3 km² crossings, g ratio 4 clamp 12, Σrain×125/788,400, 40 L/s, 53×53 patches, bands 4–5/6–7/≥8).

## 2026-08-26 — groom: 03-climate-driven-refinement@Q9
key: groom/03-climate-driven-refinement@Q9

- `docs/capstone/features/03-climate-driven-refinement/spec.md` — thirteen requirements (R1–R13) making the area tier consume feature 02: a `Continent { grid, climate, hydrology }` context threaded once through `bundle_for`/`generate_area`; `TileBundle` reaching its full `logic/01` step-10 shape (entering rivers from ≥3 km² continent-tree crossings defined against the exact 100 m boundary line so both seam sides derive identical sets, seeded at the lowest edge cell with catchment×100 cells + discharge and an order floor `g(catchment) = 1 + ilog2(c/3)/2`; 53×53 rainfall/regime/filled patches; wind octant; west-edge moisture via a new `ContinentClimate.moisture` output); `Cell.rainfall` bilinear producer; discharge `Σrain × 125/788,400 + entering` L/s; channel initiation at 40 L/s replacing the 300-cell rule; edge-touching basins clamped to the shared continent filled surface; order-banded river rendering with the overview floor raised 3→4; the step-9 ≥1-sea-river reroll gate with a `river_count` stat. Closes open-items #1, #5, #12 when built. One golden re-bless authorized (§Q9); FORMAT_VERSION stays 3.
- User chose maximal scope at §Q1 (all four offered satellites in, including bundle climate fields against the YAGNI recommendation and the step-9 gate) — recorded as a deliberate override mirroring feature-01 §Q8. Rejected along the way: self-contained bundles and per-tile climate recomputation; seeding all crossings; persisted continent Strahler (1 km confluence counts misalign with 100 m orders); fbm rainfall detail; dual initiation thresholds; routing-surface max-blend; whole-tile moisture patch; log-discharge render scaling; stricter reroll gate.
- Recorded risks/approximations: a river looping out and back into its own tile double-counts upstream (accepted); seam-lake byte-agreement depends on matching boundary-contact sets — invariant (d) verifies at implement time with a pre-approved fallback rule (clamp at the basin's lowest boundary cell) requiring no re-gate.
- Out of scope as decisions: naming (step 8, `named_river_count` stays 0), human geography (step 7), vegetation-stage consumers of the new bundle fields, area temperature/moisture/forest fields, statistical gates (step 12), export JSON schema changes.

## 2026-08-26 (8d3c9d930157..6b65f7c6a0c0)

Architecture-level delta across 86 files (the drainage/continent rebuild
plus feature 02), base = oldest topic stamp since no prior ranged entry
exists.

- **New modules:** `arda-gen/src/continent/{climate,hydrology}.rs` (logic/01 steps 5–6: integer advection-diffusion climate; continent drainage tree + river extraction) and `arda-core/src/{continent.rs, formats/overview.rs}` (continent-tier types; `continent/overview.bin` codec). `arda-gen/src/continent/{plates,tectonics,coast,erode}.rs` were rebuilt in place (belt-based uplift, warped Voronoi, shelf-gradient coast, global 1 km stream-power erosion).
- **World format major 2 → 3:** `continent/overview.bin` carries real 18 B/cell records and `continent/objects.bin` a rivers section; the manifest gate became exact-match (older worlds now refused, `formats/manifest.rs`). New error variant `FormatError::DimensionsOverflow`; decoders refuse crafted headers via fully checked size arithmetic.
- **New commands/surfaces:** `arda-cli` gained `preview` and `export --overview` / `--block` (whole-world PNG, WFC block layer). No new external dependencies; no dependency-direction changes (`arda-render` still never depends on `arda-gen`).
- **New test/bench surfaces:** `arda-gen/tests/{drainage_invariants,continent_hydrology,continent_measures}.rs`, `benches/{area_erosion,continent_stage}.rs` (30 s/tile and 60 s/stage release gates); golden fixture re-blessed twice (area drainage rewrite at format 2; continent layer at format 3).
- **Area tier rewritten** (`arda-gen/src/area/**`): four-process erosion, priority-flood fill with lake emission, D8 by steepest descent, Strahler orders, network-link reaches, HAND/TWI fields — per-cell layouts changed with format 2.

Stale-topic check: `03-conventions.md`, `05-dependencies.md`,
`07-operations.md`, `08-glossary.md` still carry the 8d3c9d9 stamp; a
`sync` run would settle them (03's no-libm consequence and 08's new
continent-tier terms are the visible gaps).

## 2026-08-26 — implement: 02-continent-climate-hydrology@Q8
key: implement/02-continent-climate-hydrology@Q8

- Executed all ten plan tasks on `feat/drainage-correctness` (base c2c0d2a, head 9c48e00, 22 commits), subagent-driven with a per-task review gate: (1) continent types + overview codec, (2) continent objects codec, (3) exact-match format gate, (4) temperature/regime/sea-distance, (5) advection-diffusion rainfall, (6) drainage tree with dual loads, (7) river extraction, (8) orchestrator persistence + invariant suite, (9) 60 s stage bench, (10) FORMAT_VERSION 3 + C_NORM calibration (256→153, seed-42 land mean 799 mm/yr) + the single authorized golden re-bless. Source touched: `crates/arda-core/{continent.rs, coords.rs, error.rs, lib.rs, formats/{mod,manifest,objects,overview}.rs}`, `crates/arda-gen/{Cargo.toml, src/continent/{climate,hydrology,erode,mod}.rs, src/orchestrator.rs, tests/{continent_hydrology,continent_measures}.rs, benches/continent_stage.rs}`, `tests/golden/micro-42.txt`. 194 workspace tests green; clippy `-D warnings` and fmt clean.
- Review loop (ledger, not indexed): **7 rounds to dry** (rounds 6–7 consecutive dry at 9c48e00). 19 branch-level findings confirmed — 1 Critical (EOF-guard add-wrap at the exact `usize` window, proven reachable at dims 238,795,480 × 4,291,618,565), 5 Important (unchecked dims multiply; fabricated `expected: usize::MAX`; `width as i32` wrap decoding u32::MAX to −1; duplicated EOF guard; half-tested dims guard), 13 Minor — all fixed across five waves (97e227a, 9a21416, 6405ad9, 9e33d52, 9c48e00) or recorded/accepted (6: diffuse-step floor decomposition, row-major tie-break, climb/lapse elevation floors → absorbed as shipped rules; objects.rs ~500-line soft cap and course-helper duplication → accepted). 0 refuted; 10 task-level minors triaged accept-as-recorded. A new honest `FormatError::DimensionsOverflow` variant and a shared pub `NO_DOWNSTREAM` sentinel came out of the loop.
- Chapters refreshed to 9c48e00: `01-architecture.md` (climate/hydrology modules, formats::overview), `02-models.md` (format 3, 18 B ContinentCell, continent objects, decoder refusal posture), `04-data-flow.md` (continent stage through step 6 + persistence; the computes-and-discards seam Feature 03 must budget for), `06-testing.md` (194 tests, invariant suite, probes, bench, re-bless scope), `open-items.md` (#6 closed; #1/#5/#12 marked unblocked-by-02/owned-by-03; build step 4 partially done).
- Absorbed per the spec's Reference impact: `logic/01-continent-generation.md` — new Observed section for steps 5–6 recording the shipped rules including the four deviations (lapse and climb floors, diffuse decomposition, row-major main-stem ties) and the threshold rule `max(300, land_km2/30)` amending §Q7's assumed fixed ~3,000 km²; `mockup/02-world-layout.md` — overview.bin/objects.bin rows real since format 3. Both stamped `absorbed_from`.
- Also repaired: `docs/capstone/.gitignore` carried the legacy `changelog.md` ignore line, silently untracking this ledger since 435a6fe; line removed so the ledger follows `docs_in_git: commit` again.
- Left open for Feature 03 (not groomed): bundle entering rivers + climate fields, area `Cell.rainfall`, cross-seam basin fill, rainfall-driven discharge and channel initiation; plus the recorded seam note that `generate_world` discards the climate/hydrology structs after persisting.

## 2026-08-26 — plan: 02-continent-climate-hydrology@Q8
key: plan/02-continent-climate-hydrology@Q8

- `docs/capstone/features/02-continent-climate-hydrology/plan.md` — ten TDD tasks with full test and implementation code: (1) continent types + `overview.bin` codec, (2) `continent/objects.bin` rivers codec with its own `ARDACOB\0` magic, (3) exact-match format gate replacing the newer-only check at `manifest.rs:94`, (4) temperature/regime/BFS distance-to-sea, (5) advection-diffusion rainfall, (6) drainage tree reusing `erode.rs`'s `fill`/`accumulate` made `pub(crate)` with dual catchment/discharge loads and the routing surface exposed for feature 03, (7) river extraction with the scaled threshold and largest-catchment main stems, (8) orchestrator persistence + the R7 invariant suite, (9) 60 s continent-stage bench mirroring `area_erosion.rs`, (10) `FORMAT_VERSION` 3, `C_NORM` calibration, R8 measurement probes into `spike-report.md`, and the single authorized golden re-bless. File map: 7 arda-core paths, 8 arda-gen paths, the golden fixture, the spike report. Coverage table maps all 11 requirements, every behavior rule, all 7 unhappy paths, and the global constraints to named tasks.
- Review passes fixed three drafting defects before the gate: rainfall conversion pass-count-normalized so one calibration serves 10-wide unit grids, MICRO, and the default world (spec's Behavior formula amended accordingly, §Q3/§Q4 targets unchanged); `as usize` casts replaced with `usize::try_from` in the orchestrator snippet; the Horton probe's stream-count array sized from the observed maximum order instead of a fixed cap.
- Constraints pinned: golden fixture touched only in task 10 (§Q6 sign-off); golden expected red between tasks 8–10 with per-crate test commands named; `stats.named_river_count` stays 0 (§Q1 deferral); no RNG anywhere in the new sim paths; `TileBundle`/area/render files absent from the file map by design (R11).

## 2026-08-26 — groom: 02-continent-climate-hydrology@Q8
key: groom/02-continent-climate-hydrology@Q8

- `docs/capstone/features/02-continent-climate-hydrology/spec.md` — eleven requirements (R1–R11) building `logic/01` steps 5–6 plus real persistence: uniform-westerly advection-diffusion rainfall (fixed 1.5×width passes, 12.5% diffusion, recharge over water, base + orographic release), band-linear temperature with 6.5 °C/km lapse and 300 km continentality, regime by band position + elevation, one final fill+route on the post-erosion 1 km surface with dual catchment/discharge accumulation, river objects above a land-scaled threshold `max(300, land_km2/30)`, an 18 B/cell `overview.bin` codec plus an `objects.bin` rivers section at `FORMAT_VERSION` 3, five gated structural invariants, measured-not-gated climate stats, Horton on seeds {1,7,42,99} discharging spike S1, a ≤60 s continent-stage bench, and a single end-of-feature golden re-bless with sign-off recorded at §Q6.
- Rejected and why: rainfall-coupled erosion (reorders logic/01's step sequence, risks measured hypsometry); 4 km routing (courses quantize, disagrees with the 1 km relief areas sample); single upwind sweep and graded/warped wind fields (user chose the softer scheme; extra calibration axes, noise in a sim path); Köppen-lite regimes (amends logic/01 §Q6's recorded rule); heightless overview.bin and unversioned sidecar (readers re-run the stage; version skew moves into code); fixed 3,000 km² threshold (MICRO fixtures get zero rivers); gating rainfall bounds now (fits thresholds to current output).
- Out of scope as decisions: steps 7–8, step-9 gate wiring, every `TileBundle` change, render/overlay work, continent-tier lakes, area `Cell.rainfall` — all owned by Feature 03 or build-order step 4's remainder. Open-items #6 closes here; #1, #5, #12 are unblocked, not closed.
- Groom's staleness pass also stamp-refreshed `02-models.md` and `04-data-flow.md` (separate sync entry below).

## 2026-08-26 — sync: 02-models.md+04-data-flow.md
key: sync/02-models+04-data-flow@c2c0d2a

- `docs/capstone/02-models.md`, `docs/capstone/04-data-flow.md` — stamp refresh only, performed by groom's staleness pass for feature 02. Both chapters' Observed sections were written during the drainage/continent rebuild but their `generated_at_commit` stamps still read 8d3c9d9 (pointer drift). Content verified against HEAD: `FORMAT_VERSION` is 2 (`formats/mod.rs:13`), the D8/fill/outlet behaviour matches `area/water.rs`, the bundle carries edge heights only (`continent/bundles.rs`), and cross-tile inflow has no producer. Stamps bumped to c2c0d2a; no body text changed.

## 2026-08-26 — docs: open-items
key: docs/open-items@2026-08-26

- `docs/capstone/open-items.md` — single index of everything known-incomplete, each with its evidence and owning code: four blocking-realism defects (cross-tile rivers, WFC noise, unreachable relaxed fallback, value-noise anisotropy), four stand-ins awaiting a producer, four calibration items held open, the two ungroomed successor features, and the unbuilt build-order steps. Also records two findings as *not* defects so they are not re-investigated.

## 2026-08-26 — build: code@Q5 (drainage and continent)
key: build/code@Q5-drainage

- `crates/arda-gen/src/area/**` — rewrote the area stage against the artifact's own Relief and Water sections, which `logic/02` designates normative and which had not been read when the stage was first built. Four coupled erosion processes (uplift, stream-power incision, hillslope creep, 35-degree talus collapse) replace one; priority-flood filling with lake emission; D8 by steepest descent rather than steepest drop; true Strahler order; reaches as network links carrying `feeds` and `ends`; HAND, TWI, slope, aspect, and floodplain-driven marsh cover. Constants taken from the artifact rather than invented: 300-cell channel threshold (its 3 km² ≈ 40 L/s), `w = 4·sqrt(Q)` channel width, the 1 m / 2.5 m / 2–15 m floodplain bands.
- `crates/arda-gen/src/continent/**` — rebuilt. Plate count scales with domain area; crust follows position; Voronoi lookup is domain-warped; uplift spreads across orogenic belts and plates drift so belts migrate; uplift is normalised to a target relief and subsidence floored; the crust field is blurred into a shelf and blended with a centred mask; noise applies on both sides of sea level; and coarse erosion now runs globally on the 1 km grid.
- `crates/arda-render/**`, `crates/arda-cli/**` — hypsometric palette replacing a linear ramp that saturated to white above 2,040 m and hid a 118 m plateau; whole-world overview render; `preview` subcommand; `export --overview` and `export --block`, the latter exposing the WFC tile layer which had no command at all.
- `crates/arda-gen/tests/drainage_invariants.rs` — eight global invariants, the class of assertion whose absence let a stage with 93% of catchment in interior pits pass 113 tests. `crates/arda-gen/benches/area_erosion.rs` gates erosion at 30 s/tile; measured 2.17 s.
- Rejected along the way, with reasons: isotropic gradient noise (blocky coastlines, reverted — the lattice anisotropy it would fix is recorded as a known limitation, ~17% diagonal flow against 50%); smoothstep belt profiles (flat-topped, made plateaus not ranges); heavier hillslope creep and lower uplift as lake remedies (both made lakes worse by reducing the relief that drives incision).
- Left open: rivers still cannot cross tile boundaries, so no catchment can exceed one tile's 2,621 km² — a UK-scale major basin is 9,385 km², 3.6x larger, and the continent cannot yet produce one. `Cell.rainfall` still has no producer, so discharge and channel initiation remain catchment-driven stand-ins. The tactical WFC layer converges but produces salt-and-pepper noise: its adjacency rule is too permissive for structure to emerge.
- `format_version` is 2: `objects.bin` record layouts changed for `feeds`/`ends` and lake depth/outlet. Golden fixture re-blessed.

## 2026-08-25 — groom: 01-area-drainage-correctness@Q15
key: groom/01-area-drainage-correctness@Q15

- `docs/capstone/features/01-area-drainage-correctness/spec.md` — twelve requirements (R1–R12) closing ten of the thirteen findings from the river pipeline audit, plus the process finding that no global drainage invariant existed. Chosen approach: priority-flood depressions to a *routing* surface with stored heights left raw; lakes emitted above 100 cells and 2,000 mm (~20/tile, measured from 859 basins/tile with median 2 cells and 208 mm); true Strahler order; D8 by drop÷distance via integer cross-multiplication; reaches as network links partitioning channel cells exactly once; outflow routed against the bundle's real neighbour heights; two-pass erosion with stream-power carving and magnitude-capped droplets detailing; HAND plus a fixed-point TWI with no libm in any sim path.
- Rejected and why: breach/carve depressions (mutates stored relief, ~2,048 gorges/tile, breaks the pinned-edge guarantee at seams); fill without lakes (leaves `Lake` producer-less); Shreve magnitude (Horton's ratio is defined on Strahler orders); droplet-only erosion (permitted by §Q4 with explicit ordering, but its network statistics are not guaranteed to hit the Horton/Hack/sinuosity bands, and it would run a different erosion model from the continent tier); diffusion-only smoothing (does not concentrate incision); deduplicating head-to-terminus courses (coverage would depend on iteration order); unconditional edge outlets and off-tile-as-sea-level (water leaves uphill, or every edge becomes a fabricated cliff); bespoke banded wetness (not a recognised index); bringing step 12's statistical gates forward (their thresholds would be fitted to current output).
- Scope reopened at §Q8 after the gate: "no out of scope" superseded §Q1's deferrals. Erosion and the derived river fields moved **into** this feature — both are computable from relief and drainage, and shelving them was an error. Three items stayed out with verified data dependencies rather than preferences: cross-tile inflow, rainfall-driven discharge, and climate-driven channel initiation all require producers that do not exist (`Cell.rainfall` has no writer anywhere in `arda-gen`; the continent stage stops after step 4). They become **feature 02** (continent climate and hydrology, `logic/01` steps 5–6, with spike S1) and **feature 03** (climate-driven refinement plus inflow), neither yet groomed.
- Two decisions were recorded late, at §Q12 and §Q13: the D8 rule change and sea-cell hygiene had been carried in the gate summary and asserted by a test, but never given decision entries, so no requirement could trace to them. Two further decisions were surfaced only by a coverage check: §Q14 caps both erosion passes at 30 s/tile enforced by a `criterion` bench (rejected: 2 min/tile, which would breach step 12's 12 h batch ceiling once blocks are per-cell), and §Q15 authorises re-blessing `tests/golden/micro-42.txt` once as the final task, which `code-prefs.md` §Q9 requires be explicit.
- Left open: the lake thresholds and every erosion constant were measured or chosen on pre-erosion noise relief and are re-derived when the statistical gates land at step 12; a basin straddling a tile seam still fills independently on each side and may reach different spill levels until feature 02 supplies a continent drainage tree.

## 2026-08-25 — doctor: ledger catch-up
key: doctor/torn-writes@2026-08-25

Seven stages carried a `formalized` or `plan_approved` marker with no
matching changelog key — torn writes under core.md's ledger rule. The
entries below are reconstructed from each interview's recorded
decisions; no stage was re-run. They were written late, so read them as
a summary of the decision record rather than a contemporaneous receipt.

## 2026-08-25 — build: code@Q5
key: build/code@Q5

- `crates/arda-core/**` — coords/fixed/rng/error/config/cell/objects/tiles plus `formats::{manifest,cells,objects,blocks}`, the workspace's only byte codec. `cells.bin` row pinned at 33 bytes carrying the full artifact field list (skeleton fills relief+water only, rest default); `objects.bin` is a tagged section container whose unknown kinds are skipped, so step 5's settlements/roads/crossings/passes are additive within format major 1. Rejected: `unsafe` transmute for row layout (code-prefs §Q1), a dev-dependency temp-dir crate (hand-rolled instead).
- `crates/arda-gen/**` — hand-rolled integer value noise; continent stage (Voronoi plates, kinematic-lite 20-step coupled tectonics, isostasy, coast, 4km→1km upsample); tile bundles; area relief+water; 24-tile WFC block fill; orchestrator with rayon fan-out. Deferred to step 4: climate, hydrology objects, human geography, naming, and the full 100-step tectonics loop behind spike S1. Deferred to step 5: the five remaining area stages. Deferred to step 6: the 200+ tile vocabulary behind spike S2.
- `crates/arda-render/**`, `crates/arda/**`, `crates/arda-cli/**` — symbolic block PNG, cartographic area PNG, versioned JSON (schema_version 1); `World`/`Area` facade; `generate`/`export` subcommands.
- `tests/golden_world.rs` + `tests/golden/micro-42.txt` — 26-file blake3 fingerprint, wired into the 3-OS CI matrix as the §Q4 gate. Not yet observed green on macOS/Windows — only Linux has run.
- Divergences from `05-dependencies.md`, each needing a chapter update: `blake3` promoted from dev/tooling to an `arda-core` runtime dependency (the subseed derivation needs it); `thiserror` added workspace-wide (code-prefs §Q4 mandates thiserror-style derives, the chapter names no error crate); `anyhow` added to `arda-cli` only (code-prefs §Q4 permits it there).
- Divergence from `implementation.md`: its orchestrator sketch omitted `logic/01` §Q9's validation reroll. Added — seed 43 fails the land-fraction gate at attempt 0 and is rerolled deterministically, ≤5 attempts.
- Known-weak, left open: area drainage does no depression filling, so channels emerge as short disconnected fragments rather than networks reaching the sea. The Horton/Hack/rank-size gates that would catch this are build-order step 12. Blocks are sampled on a 64-cell stride, not one per land cell. Areas and blocks load eagerly, not lazily as `logic/05` specifies. All three marked in-code.

## 2026-08-25 — build: plan@Q5
key: build/plan@Q5

- `docs/capstone/implementation.md` — approved build plan: workspace layout, three load-bearing seam sketches (subseeding, stage purity, pinned edges), and a 13-step build order with per-step verification. Five late decisions folded in from this stage's own interview: `serve` (Q1), cost-distance realms (Q2), buildings (Q3), SRD 5.1-style NPCs (Q4), tile attributes plus a POI layer (Q5). Deferred as D9 and explicitly not planned: tileset-manifest renderer beyond a stub, `--memory` tuning past an areas-in-flight cap, region-file front door, runtime in-process game API.

## 2026-08-25 — stack: all@Q8
key: stack/all@Q8

- `docs/capstone/05-dependencies.md` — eight capabilities picked: `zstd`, `png`, `serde`/`serde_json`, `rand_chacha` (ChaCha8), `rayon`, `clap` v4, hand-rolled Q-format fixed-point, and hand-rolled noise/erosion/hydrology/WFC. Dev/tooling: `criterion`, `blake3`, clippy/rustfmt, `cargo-deny`. Rejected: the `fixed` crate, in favour of hand-rolled Q-format newtypes; any third-party crate for sim arithmetic (code-prefs §Q2 forbids it). Zero paid services.

## 2026-08-25 — code-prefs: all@Q9
key: code-prefs/all@Q9

- `docs/capstone/code-prefs.md` — nine normative domains: `deny(unsafe_code)`/`deny(missing_docs)` with unwrap banned outside tests; commodity adopted but determinism-critical code hand-rolled; data-oriented functional core with traits only when a second implementation exists; per-crate thiserror-style enums with `anyhow` confined to the CLI; package-by-stage layout; test-first with no mocks ever; rustfmt defaults and clippy `-D warnings`; conventional commits; and agent rules forbidding ambient randomness, wall-clock time, and hash-iteration-order dependence in sim paths.

## 2026-08-25 — architecture: all@Q8
key: architecture/all@Q8

- `docs/capstone/01-architecture.md`, `02-models.md`, `03-conventions.md`, `04-data-flow.md`, `06-testing.md`, `07-operations.md`, `08-glossary.md` — five-crate workspace with a one-way dependency direction and `arda-render` barred from depending on `arda-gen`. The defining decision is §Q4: cross-platform bit-exact determinism, a one-way door binding every later stage to integer/fixed-point sim arithmetic and a counter-based PRNG keyed `(seed, tier, stage, coords, attempt)`. Rejected: same-binary-only determinism, statistical-only determinism, FlatBuffers/Cap'n Proto/Arrow, SQLite, a plugin architecture. Risks logged R1–R4, with spikes S1 (tectonics plausibility) and S2 (WFC convergence) gating build-order steps 4 and 6.

## 2026-08-25 — design: skipped
key: design/all@skipped

- No `docs/capstone/design/` written. The product has no human-facing UI — surfaces are `[api, cli]` — so the stage was formalized as `skipped: no-ui` rather than producing empty chapters. Revisit only if a UI surface is added.

## 2026-08-25 — logic: all@Q16
key: logic/all@Q16

- `docs/capstone/logic/01-continent-generation.md` … `06-society-generation.md` — six scenarios pinned trigger by trigger: continent (plates, coupled tectonics, coast, climate, hydrology, human geography, naming, validation with ≤5 subseeded rerolls, tile bundles), area, block (WFC ≤8 retries then a marked relaxed fill), export, load-query, and society. The load-bearing invariant across all six: a tile bundle depends on coarse data and the seed only, never on any area's fine output, which is what lets areas generate in any order. Society was added after the other five, which is why the interview's scenario checklist needed today's catch-up.

## 2026-08-25 — mockup: all@Q22
key: mockup/all@Q22

- `docs/capstone/mockup-artifact.md` plus `docs/capstone/mockup/01-generate.md` … `06-serve.md` — the product defined across six surfaces: the `generate` batch, the world directory layout, `export`, the crate API, docker, and (added at the build gate) `serve`. Scope decisions recorded: offline batch preparation only, no runtime or streaming API in v1, worlds disposable and regenerable from a seed, and ~61 GB at the default continent size accepted.
