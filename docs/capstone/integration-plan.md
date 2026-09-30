---
generated_date: 2026-09-30
generated_at_commit: 7f32695
status: plan; nothing merged yet
---

# Integration plan — product layers

> How the eighteen parallel branches become one workspace: the inconsistencies between them (first, because they decide the adapters), the merge order, the conflicts to expect, the adapters to write, and the end-to-end acceptance test. The normative rules are the product-layer logic specs [08](logic/08-settlements-roads-realms.md)–[16](logic/16-service-api.md), indexed in the [scenario index](logic/README.md). Branch state below was read on 2026-09-30 from each worktree, read-only; several branches were still being written, so re-check each one's `git log` before its merge.

## 1. Inconsistencies between the goal prompts and the branches

Sources: the goal prompts in `docs/goal-prompts/` (01–05 and `vocabulary.md`) and `docs/goal-prompt.md`, which are untracked in the main checkout and so cannot be linked from here; the committed branches; and the uncommitted work in the other worktrees. "Resolved by" names the spec rule that decides it; the adapter numbers refer to §5.

| # | Inconsistency | Where | Resolved by |
|---|---|---|---|
| I1 | **Cell position frame.** The server contract puts global cell `(gx, gy)` at the node `(100·gx, 100·gy)` m with the footprint centred on it; `arda-settle` writes settlements, crossings and passes at `100·gx + 50` and calls road vertices "cell centres" and border vertices "cell corners" in that shifted frame. Every position differs by 50 m on both axes. | `crates/arda-server/API.md` "Coordinates"; `arda-settle` `place.rs`, `crossings.rs`, `roads.rs`, `realms.rs` | 08 §world-frame; A1 |
| I2 | **Square size vs cell size.** Goal 05 cites `SQUARE_SIZE_MM = 1524` and 64 squares per block; 64 × 1.524 m = 97.536 m, but a cell is 100 m and the server contract says so. The mapping from squares to world metres is unstated. `arda-refine` uses `100/64` m. | goal 05; `arda-core` `coords.rs`; logic/03 | 09 §square-frame |
| I3 | **Road classes.** `arda-core` `RoadClass` is none/track/road/highway (codes 0–3) and the `CellSample` legend follows it; goal 04 asks for highway/road/track/footpath; `arda-settle` and `arda-ways` use footpath 1, track 2, road 3, highway 4; `arda-fields` has its own enum without codes. The same code 1 means track in the world and footpath in society. | `cell.rs`; `API.md`; goal 04 step 5; `arda-settle` `roads.rs`; `arda-ways` `input.rs`; `arda-fields` `input.rs` | 08 §roads; 16 §api-versioning (contract 2); A6 |
| I4 | **Road widths.** `arda-ways`: highway 5 squares, road 3 (4 when wealth ≥ 160), track 2, footpath 1. `arda-fields`: half-widths 2.0/1.5/1.05/0.55 squares (4/3/2.1/1.1). A road would change width where fields meet ways. | `arda-ways` `ClassSpec`; `arda-fields` `RoadClass::half_width_sq` | 08 §roads table; A6 |
| I5 | **Id and code widths.** `Cell::built_by` is u16, the server's binary column u32, `arda-settle` settlement ids u64 with a u32 land-use owner raster; realm ids are u32 but the realm raster is u16; `arda-town` `BuildingId` is u32 while `arda-npc` `BuildingId` is u64; `arda-town` defines its own `SettlementId`. | `cell.rs`; `API.md` ARDACOLS; `arda-settle` `landuse.rs`, `realms.rs`; `arda-town` `plan/types.rs`; `arda-npc` `input.rs` | 08 Outcomes (u64 ids; documented raster limits); 16 §api-conventions (strings on the wire); A4, A6 |
| I6 | **Building function vocabularies.** `vocabulary.md` function tags use `market` and add `farm`, `street`, `barn`, but lack `market_hall`, `mine`, `lumber_camp`, `school`. `arda-npc` keys use `market_hall`, `mine`, `lumber_camp`, `school` and `workshop(craft)`. `arda-settle`'s building mix uses the npc keys; `arda-town` plans with the vocabulary keys and reads that mix, so `market_hall`, `mine`, `lumber_camp`, `school` go unmapped. The placeholder catalogue's vocabulary lists only 7 functions, so the validator rejects the rest. | `vocabulary.md`; `arda-npc` `BuildingFunction`; `arda-settle` `profile.rs`; `arda-town` `function.rs`; `assets/tactical/placeholder/catalog.json` | 10 §town-function-keys; 11 §vocabulary; A3 |
| I7 | **Building function serde form.** `arda-npc` serialises `BuildingFunction` adjacently tagged (`{"kind": "inn"}`, `{"kind": "workshop", "craft": "weaving"}`); `arda-settle` writes plain string keys; `arda-society` mirrors the tagged form. | `arda-npc` `input.rs`; `arda-settle` `model.rs`; `arda-society` `input.rs` | 13 §npc-inputs; A4 |
| I8 | **Tag-query syntax.** `vocabulary.md` prescribes namespaced tags (`function:*`, `biome:*`); `Library::query` on feat/tactical-catalogue matches bare values only, so a namespaced query matches nothing (feat/tactical-library is fixing it, uncommitted). Goal 05's example scatter tag `tree:broadleaf:large` matches no asset under either reading. | `vocabulary.md`; `arda-tactical` `library.rs`; goal 05 step 5 | 11 §tag-query; 09 §scatter; A15 |
| I9 | **Where per-square rules go.** Goal 05 says "a documented extension object" in the layout; `vocabulary.md` and `deny_unknown_fields` on `Square` require a sidecar. `arda-scene` defines `RulesSidecar` format 1 (difficult, water depth, cover, blocks sight, lightly obscured); `arda-ways` defines a different `Sidecar`, also "version 1" (feature, road class, deck, deck elevation, difficult, edge rules). | goal 05 step 7; `vocabulary.md`; `arda-scene` `sidecar.rs`; `arda-ways` `sidecar.rs` | 12 §scene-sidecar (format 2); A8 |
| I10 | **Seamless art and cache identity.** `TacticalLayout` has no origin; the compositor hashes local square and pixel coordinates; `arda-server` (tactical) renders with a constant `RENDER_SEED = 1` and keys renders by layout hash, ppsq, grid and `library_version`. Neighbouring blocks cannot join invisibly, and the key omits the world seed and coordinates that goal 67 names. | `arda-tactical` `layout.rs`, `compose/`; `arda-server` `tactical/mod.rs` (feat/server-tactical, uncommitted) | 11 §seam-art; 16 §api-cache; A10 |
| I11 | **Block seam carries only a layout.** The server's `BlockSource::block` returns a `TacticalLayout`; goal 48 requires scene data from the same layout, which needs the rules sidecar, meta and origin too. | `arda-server` `tactical/block.rs` | 16 §api-tactical; A9 |
| I12 | **No NPC tokens.** Goal 48 lists NPC tokens in the scene data; `arda-scene` format 1 has none, and `arda-npc` has no notion of where a person stands. | goal-prompt goal 48; `arda-scene` `types.rs` | 12 §scene-tokens; A13 |
| I13 | **Tier population ranges.** `arda-npc` documents hamlet 20–100 and village 100–1,000; `arda-settle` uses the artifact's hamlet 12–80; towns 1,000–8,000 in both. | `arda-npc` `input.rs`; `arda-settle` `model.rs`; artifact | 08 §settle-tiers; A16 |
| I14 | **City notable count.** Goal 03 cites "city about 60–200" to logic/06 step 5, which gives only hamlet, village and town. | goal 03 "Notables"; [logic/06](logic/06-society-generation.md) | 13 §npc-notables (city marked assumed) |
| I15 | **Water depth semantics.** The compositor reads about 2 ft as shallow and ≥ 7 ft as deep (capped at 8); `arda-scene` wades at 1–4 ft and swims at ≥ 5 ft; goal 05 says "shallow means wading; deep means swimming" with no numbers. | `arda-tactical` README; `arda-scene` `squares.rs`; goal 05 step 6 | 12 §scene-movement (the rules threshold is the one source; the look is presentation) |
| I16 | **Cover naming.** The catalogue says `full`; the scene says `total` and accepts `full` as an alias. | `catalog.rs`; `arda-scene` `types.rs` | 12 §scene-cover |
| I17 | **u64 on the wire.** The server writes u64 as decimal strings; the scene writes `seed` as a JSON number (unsafe above 2⁵³ in JS). | `API.md`; `arda-scene` `types.rs` | 16 §api-conventions; A12 |
| I18 | **Three name generators.** Goal 03 and goal 04 each require their own original syllable generator (`arda-npc` `names.rs` + `names.json`; `arda-settle` `phonology.rs` + `names.rs`); feat/names builds a third. `arda-names` keys are u64 (`PlaceSpec::new(kind, key)`). | goal 03 "Names"; goal 04 step 7; `arda-names` `types.rs` | 15 §name-key, §name-inventories; A5 |
| I19 | **Stored vs derived buildings.** logic/06 step 4 writes `Building` objects into area layers and logic/03 writes block archives; goal 56 requires storage O(settlements), and goals 42 and 67 require on-demand blocks. | [logic/03](logic/03-block-generation.md); [logic/06](logic/06-society-generation.md) | 10 (plans are derived); 09 (no archives) |
| I20 | **Elevation reference.** `Square.elevation_ft` (i16) does not say whether it is absolute or block-relative. | `layout.rs`; goal 05 step 1 | 09 §elevation (absolute, 5-ft steps) |
| I21 | **One hash per crate.** `arda-tactical`, `arda-refine`, `arda-town`, `arda-settle` and `arda-npc` each hash differently. Quantities two crates must agree on (edge offsets, scatter, variants) need one function. | each crate's `rng.rs`/`hash.rs` | 09 §hash |
| I22 | **Biome strings.** `arda-settle` writes free text with spaces (`"boreal forest"`); the catalogue's controlled `biome` vocabulary holds `temperate` only. | `arda-settle` `profile.rs`; `catalog.json` | 08 settlement record (snake_case); 11 §dress (biome mapping) |
| I23 | **Land-use classes.** `arda-settle` stores 8 codes; `arda-fields` expects 12 classes (adds wild, meadow, fallow, quarry, farmstead). | `arda-settle` `landuse.rs`; `arda-fields` `input.rs` | 08 §landuse derivation; A17 |
| I24 | **Society rewrites the present.** feat/society's timeline includes "wars and border shifts", while realms (logic/06 steps 1–3) are produced by `arda-settle`, whose partition is authoritative. | `arda-society` `history/mod.rs`; goal 04 step 6 | 14 §soc-present |
| I25 | **World branches on an unsigned snapshot.** feat/world-water, feat/world-coast and feat/atlas-polish start from `7ad0ab5`, the variant-Q snapshot that is not signed off, and the main checkout holds the same work uncommitted. feat/world-water adds a new `arda-core` water format, which may change the world format for every `World::load` consumer. | branch bases; worktree status | §3 phase E |
| I26 | **Sea-cell snowline** is computed from the seafloor height. | JOURNAL entry "Tasks 1 and 2 reviewed" | 16 §api-versioning (contract 2) |
| I27 | **Diagonal rule source.** `arda-scene` calls the 5-ft diagonal "the SRD default"; not verified against the SRD 5.1 text. | `arda-scene` `types.rs` | 12 §scene-diagonal (assumed until checked) |
| I28 | **Block neighbourhood.** The artifact and logic/03 say a block reads its cell and 8 neighbours; `arda-refine` reads a 7 × 7 neighbourhood. | artifact; `arda-refine` `context.rs` | 09 Preconditions (allowed while every quantity stays global) |
| I29 | **Tile formats.** Goal 68 asks for tiled WebP; overview tiles are 256 px PNG, tactical tiles 512 px WebP. | `API.md`; `pyramid.rs` | 16 §api-tiles (both kept; the overview stays PNG) |
| I30 | **Dependency declarations.** `arda-town`, `arda-fields` and `arda-scene` depend on `arda-tactical` by path; `arda-ways` by `workspace = true`. | the crates' `Cargo.toml` | A6 (normalise to workspace deps) |
| I31 | **NPC types for TS.** Goal 69 wants NPC sheets in the game's schema with generated TS types; `arda-npc` has no ts-rs (correctly, as a pure library). | goal 66, 69; `arda-npc` `Cargo.toml` | 16 §api-bindings; A12 |
| I32 | **Capacity table.** The only resident-capacity and workplace-slot table is `arda-npc`'s `sample.rs` stand-in; `arda-settle`'s building-mix estimate uses its own implicit ratios. | `arda-npc` `sample.rs`; `arda-settle` `profile.rs` | 10 §town-capacity; A4 |
| I33 | **Floating point.** `arda-refine`, `arda-ways` and `arda-fields` compute in `f64` (including `ln`); goal-prompt §8 asks for integer or fixed-point generation. | `arda-refine` `math.rs`; `arda-ways` `input.rs`; `arda-fields` `input.rs` | 09 §hash (floating-point clause) |
| I34 | **Root journal file.** feat/tactical-catalogue commits `JOURNAL-tactical.md` at the repository root, while every goal prompt reserves journalling to the main session's `JOURNAL.md`. | feat/tactical-catalogue | §4 (maintainer decides to fold or drop it) |
| I35 | **Society's building input.** `arda-society` mirrors `BuildingSpec` with only `id`, `settlement_id`, `function`, but buildings exist only in town plans, which `arda-society` does not read. | `arda-society` `input.rs` | 14 §soc-offices; A14 |

## 2. Branch inventory

| Branch | Worktree | Base | Owns | State on 2026-09-30 |
|---|---|---|---|---|
| docs/product-specs | `../arda-specs` | `7f32695` | `docs/capstone/` (this plan, logic/08–16) | this branch |
| feat/tactical-catalogue | `../arda-tactical` | `7f32695` | `crates/arda-tactical`, `assets/tactical/placeholder`, `arda tactical` CLI | 12 commits, reviewed |
| feat/world-server | `../arda-server` | `7f32695` | `crates/arda-server`, `bindings/ts/arda` | 7 commits, reviewed |
| feat/npc-population | `../arda-npc` | `7f32695` | `crates/arda-npc`, `NOTICE` | 11 commits, reviewed |
| feat/settlements-roads | `../arda-settle` | `7f32695` | `crates/arda-settle` | uncommitted crate |
| feat/tactical-terrain | `../arda-refine` | `7f32695` | `crates/arda-refine` | uncommitted crate |
| feat/town-layouts | `../arda-town` | `77d9626` | `crates/arda-town` | uncommitted crate |
| feat/tactical-library | `../arda-tactical-lib` | `77d9626` | `crates/arda-tactical` changes, placeholder art | uncommitted edits |
| feat/tactical-scene | `../arda-scene` | `77d9626` | `crates/arda-scene` | uncommitted crate |
| feat/server-tactical | `../arda-server-tac` | `724c528` (merge of world-server into catalogue) | `crates/arda-server/src/tactical` | uncommitted edits |
| feat/viewer | `../arda-viewer` | `db0efca` | `apps/viewer` (TS/React) | uncommitted app |
| feat/world-water | `../arda-water` | `7ad0ab5` | `arda-core` water format, `arda-gen/src/formation/water` | uncommitted |
| feat/world-coast | `../arda-coast` | `7ad0ab5` | `arda-gen` continent margins, bathymetry | uncommitted |
| feat/atlas-polish | `../arda-atlas` | `7ad0ab5` | `arda-render` atlas | uncommitted |
| feat/names | `../arda-names` | `7f32695` | `crates/arda-names` | uncommitted crate |
| feat/society | `../arda-society` | `7f32695` | `crates/arda-society` | uncommitted crate |
| feat/tactical-ways | `../arda-ways` | `77d9626` | `crates/arda-ways` | uncommitted crate |
| feat/tactical-fields | `../arda-fields` | `77d9626` | `crates/arda-fields` | uncommitted crate |
| wip/variant-q | `../arda-q` | `7f32695` | `arda-gen/src/formation`, `arda-render/src/atlas/formed.rs` | `7ad0ab5`, not signed off |

## 3. Merge order

Integrate on a fresh branch, `integrate/product`, from `7f32695` in its own worktree; never in the main checkout, which holds uncommitted terrain work. Each step is a `--no-ff` merge followed by the adapter commits listed for it and the per-merge gate (§6). Phases A–D add crates and never change a world byte; phase E changes the world and is one deliberate re-baseline.

**Phase A: specs and foundations**

1. docs/product-specs — docs only; lands first so every later commit can cite `logic/08`–`16`.
2. feat/tactical-catalogue — the base of five tactical branches.
3. feat/world-server.
4. feat/server-tactical — it already contains 2 and 3, so this merge adds only the tactical routes.
5. feat/npc-population.
6. feat/names — an independent crate; the replacement of local generators waits for step 8 (A5).

**Phase B: world-scale society**

7. feat/settlements-roads, with A1 (frame), A2 (profile test), A6 (shared record types) and A16.
8. Adapter commit A5: `arda-settle` and `arda-npc` switch to `arda-names`.

**Phase C: tactical layers**

9. feat/tactical-library, with A15 and the `origin` field of A10.
10. feat/tactical-scene, with A8 (sidecar format 2).
11. feat/tactical-terrain, with A7 (use the real `TacticalLayout` and `RulesSidecar` types).
12. feat/tactical-ways, with A6 (road table) and A8.
13. feat/tactical-fields, with A6, A8 and A17.
14. feat/town-layouts, with A3 and A4.
15. feat/society, with A14.
16. Adapter commits A9, A10 (server side), A11, A12 and A13: the server composes world blocks, scenes and tokens.

**Phase D: client**

17. feat/viewer — regenerate bindings first; switch `apps/viewer/src/api/tactical.ts` from its normalising parser to the generated types.

**Phase E: world track (only after the maintainer signs off variant Q or its successor)**

18. wip/variant-q, or its signed-off successor (the main checkout's uncommitted work must first be committed there or discarded by the maintainer).
19. feat/world-water, then 20. feat/world-coast, then 21. feat/atlas-polish.
22. Regenerate the MICRO and default fixtures, re-run the statistical suites, and re-baseline golden hashes only with the maintainer's explicit approval (standards "Agent rules").

If variant Q is signed off before integration starts, phase E may move to the front instead, so the product crates are verified once against the final world; either way there is exactly one re-baseline.

## 4. Expected conflicts

| Files | Branches | Resolution |
|---|---|---|
| `Cargo.lock` | every crate branch | never hand-merge: take either side, then `cargo check --workspace` regenerates it; commit the result |
| `Cargo.toml` `[workspace.dependencies]` | feat/tactical-catalogue adds `arda-tactical`; later crates may add their own entries | union of entries, versions unchanged; normalise path dependencies to `workspace = true` (A6) |
| `NOTICE` | feat/npc-population (SRD 5.1 attribution); feat/tactical-library if it adds art attributions | union; keep the SRD paragraph verbatim |
| `crates/arda-cli/src/main.rs`, `crates/arda-cli/src/tactical.rs` | feat/tactical-catalogue; feat/tactical-library if it adds subcommands | union of `Commands` variants; each subcommand in its own module |
| `crates/arda-tactical/src/**` | feat/tactical-library (`library.rs`, `noise.rs`, placeholder modules renamed and deleted); branches based on `77d9626` compile against the old API | merge library first (step 9); rebuild and re-test town, scene, ways and fields against it before merging each |
| `assets/tactical/placeholder/**` (catalogue and PNGs) | feat/tactical-library regenerates; any other branch that commits placeholder output | binary conflicts: take feat/tactical-library's, then regenerate with `arda tactical placeholders` and validate; only that branch owns this output (11 §placeholders) |
| `bindings/ts/arda/**` | feat/world-server, feat/server-tactical, later DTO additions | regenerate with `ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings`; never hand-merge generated files |
| `crates/arda-server/src/{routes,error,lib}.rs` | feat/server-tactical, adapter commits of step 16 | sequential edits on the integration branch; no parallel branch touches them |
| `crates/arda-gen/src/formation/mod.rs` | wip/variant-q, feat/world-water, feat/world-coast | merge in phase E order; the water and coast stage registrations go in the order water then coast |
| `crates/arda-gen/src/formation/{incision,bathymetry}.rs`, `crates/arda-gen/src/continent/{mod,tectonics}.rs` | variant Q, feat/world-coast | coast rebases onto the signed-off terrain |
| `crates/arda-core/src/{lib.rs,formats/mod.rs}` | feat/world-water (new water format) | if the world format major changes, every `World::load` consumer (settle, refine, server) needs the new major and fixtures regenerate (phase E) |
| `crates/arda-render/src/atlas*.rs`, `crates/arda-render/src/lib.rs`, `overview/channel_overlay.rs` | variant Q (`atlas/formed.rs`), feat/atlas-polish | atlas rebases onto the signed-off `formed.rs` |
| `docs/capstone/00-index.md`, `docs/capstone/logic/README.md` | this branch | other branches must not edit them; add rows on the integration branch |
| `JOURNAL-tactical.md` | feat/tactical-catalogue | the maintainer folds it into `JOURNAL.md` or drops it before merge (I34) |

## 5. Adapters

| # | Adapter | From → to | Owner (step) | Test |
|---|---|---|---|---|
| A1 | Node frame | `arda-settle` positions → 08 §world-frame (`x_m = 100·cell_x`; polylines through nodes; borders on footprint edges at ±50 m) | settle (7) | 08 invariant 13 |
| A2 | Settle record → `SettlementProfile` | `society/settlements.json` → `arda_npc::SettlementProfile` by serde; `biome` becomes snake_case | settle (7) | deserialise every MICRO record into `SettlementProfile`; fields equal |
| A3 | Function keys | one table (10 §town-function-keys) in `arda-town`; `vocabulary.md` and the catalogue vocabulary gain `mine`, `lumber_camp`, `school` and the full function list | town (14), library (9) | every settle mix key maps; placeholder validates with the full vocabulary |
| A4 | Town building → `BuildingSpec` | `TownBuilding` → `arda_npc::BuildingSpec` (10 §town-building-spec): `BuildingId` widened to u64, capacity and slots from 10 §town-capacity, `Workshop(craft)` by hash | town (14) | 10 invariant 2; 13 invariant 11 |
| A5 | `arda-names` replaces local names | `arda-settle` `phonology.rs`/`names.rs` and `arda-npc` `names.rs`/`names.json` → `arda-names` with 15 §name-key keys; existing inventories migrate as presets | integration commit (8) | 15 invariants; NPC and settlement names change once, so the example outputs are re-read by the maintainer |
| A6 | Shared society record types | the five copies of `Settlement`, `Road`, `Crossing`, `Pass`, `Realm`, `Tier`, `Function`, `RoadClass` and land-use codes (in `arda-settle`, `arda-society`, `arda-town`, `arda-ways`, `arda-fields`) → one serde-only crate, proposed `arda-society-model`, written by settle and read by the rest; road widths from 08 §roads | settle (7), then each consumer at its merge | round-trip MICRO `society/` files through the shared types byte-identically |
| A7 | Refine's layout mirror | `arda-refine`'s own `TacticalLayout` JSON → `arda_tactical::TacticalLayout` | refine (11) | JSON equality on the six fixture cells before and after the switch |
| A8 | Sidecar format 2 | `arda-ways` `Sidecar` (and any fields or town sidecar) → `arda_scene::RulesSidecar` format 2 (12 §scene-sidecar); merge by reservation precedence | scene (10), ways (12), fields (13) | a bridge square is `normal` movement at deck elevation; a parapet edge blocks movement, not sight |
| A9 | `BlockSource` returns a `RefinedBlock` | `Fn(gx, gy) -> TacticalLayout` → layout + rules + meta + origin; the implementation composes refine, ways, fields and town reservations (09 §reservations) | server (16) | `/v1/tactical/cell` against MICRO returns 200 for a land cell |
| A10 | Global art seeding | `TacticalLayout.origin` (optional, additive); compositor hashes global coordinates; server renders world blocks from a window with a 2-square apron and a per-world render seed; `RENDER_SEED = 1` stays for built-in layouts only | library (9), server (16) | 11 invariant 5; 16 invariant 5 |
| A11 | Cell contract 2 | `CellSample` `road`/`built_by` from `society/`; `land_use`, `realm_id`; `ARDACOLS` layout 2; sea snowline from sea level | server (16) | 16 §api-cell-society; bindings fresh |
| A12 | TS mirrors | `Npc`, `Sheet`, `Scene`, `TacticalBlock`, `TacticalScene`, `Token`, `TownPlan`, settlement and realm records → ts-rs DTOs in `arda-server`, u64 as strings | server (16) | per-mirror JSON equality with the domain type; bindings staleness test |
| A13 | Tokens | town plan + population + scene → `tokens` (12 §scene-tokens) | server (16) | 12 invariant 7 |
| A14 | Offices and houses | `arda-society` offices → town-plan buildings; the ruling house's family name reaches the ruler's household through an optional `family_name` on `BuildingSpec` (an additive `arda-npc` change) | society (15), npc (15) | 14 invariant 5 |
| A15 | Tag namespaces and coverage | `Library::query` namespaced tags; `vocabulary_coverage` and `commercial_licence` validator rules; biome tag values | library (9) | 11 invariants 4, 6 |
| A16 | Tier ranges | `arda-npc` tier docs and any range checks → 08 §settle-tiers | settle (7) | 08 invariant 6 |
| A17 | Fields land-use classes | `landuse.bin` codes → `arda-fields` classes by 08 §landuse derivation | fields (13) | fields render of a MICRO village ring shows fields, fallow and pasture |

## 6. Per-merge gate

After every merge and adapter commit on `integrate/product`:

- `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`, counting successes with `grep "test result: ok. [1-9]"`, never the absence of FAILED.
- The statistical and golden-world suites pass unchanged (phase E excepted, with approval).
- `arda tactical validate assets/tactical/placeholder` passes; TS bindings are fresh.
- No `unwrap`/`expect` outside tests, no `unsafe`, files ≤ ~500 lines (goal-prompt §8).
- Every new rule comment cites a `logic/` rule (standards "Documentation").

## 7. End-to-end acceptance test

`tests/e2e_village.rs` in the workspace test host package (next to `tests/golden_world.rs`), marked `#[ignore]` and run in the release gate with `cargo test --release --test e2e_village -- --ignored`. It proves goal-prompt roadmap step 3 and goal 70: **seed → world → settlements → one tactical village map with NPCs served over HTTP.**

1. **World.** Generate MICRO seed 42 with `--terrain fine` into a temp directory through the library API (`arda::generate`).
2. **Settlements.** Run `arda-settle` on it, then `arda-society`. Assert that `society/` holds `settlements.json`, `roads.json`, `realms.json`, `features.json`, `landuse.bin`, `roads.bin`, `realms.bin` and `history.json`, each at format 1, and that 08 invariants 2, 3, 9 and 11 hold.
3. **Pick the village.** The `riverine` village with the largest population, ties by lowest id. Fail if none exists (a MICRO world without a river village is itself a finding).
4. **Serve.** Build the `arda-server` router in-process (tower `oneshot`, no sockets, no mocks of Arda) over the world, `society/` and the placeholder library.
5. **World routes.** `GET /v1/world` lists every version family (16 §api-versioning); `GET /v1/cell/{cell_x}/{cell_y}` has `contract_version` 2, `built_by` equal to the village id and a `road` class.
6. **Settlement routes.** `GET /v1/settlements?tier=village` contains the village; `GET /v1/settlements/{id}/plan` returns a plan whose capacity ≥ population (10 invariant 2) and whose every building outline is closed with a street-facing door (10 invariant 4).
7. **Tactical map.** `GET /v1/tactical/cell/{cell_x}/{cell_y}` returns 200 with a 64 × 64 layout, a format-2 sidecar and meta; the layout contains at least one building's wall loop and, because the village is riverine, water squares within 3 cells when the block is the river cell (otherwise the block of the nearest river cell is fetched too); any `relaxed` cells are reported, not failed.
8. **Scene and NPCs.** `GET …/scene` returns tokens. For every token, `GET /v1/npc/{npc_id}` returns 200; the NPC's `home_building` or `workplace_building` equals the token's `building_id`, and that building is in the plan (12 invariant 7, 13 invariant 11). Every notable of the village has a sheet that passes the `arda-npc` recomputation checks (13 invariant 3).
9. **Images.** `GET /v1/tactical/cell/{…}.png` decodes to `64·128` px square; tile `z = max_zoom, x = 0, y = 0` decodes as 512 × 512 WebP; a tile outside the pyramid is 404.
10. **Seams.** Fetch the east neighbour; the entry offsets of every river and road on the shared edge match (09 invariant 2), the exits of 12 §scene-spawn agree, and the edge pixel columns equal those of the `2 × 1` window render (16 invariant 5).
11. **Determinism.** Repeat every request: identical bytes and ETags; rebuild the router from scratch and repeat: identical again.
12. **Performance** (release only): the cold village block layout plus scene in < 500 ms and a cached tile in < 20 ms (16 invariant 7).
13. **Artefacts for review.** Write the PNG, layout, scene and a roster of the village's NPCs to `out/e2e/`; the maintainer looks at them (goal-prompt §7: judge by looking).

The test passes only when every assertion holds; it is the definition of "integrated" for this plan.
