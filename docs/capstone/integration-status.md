---
generated_date: 2026-10-01
branch: integrate/v0.5 (earlier sections: fix/v4-hardening, feat/v4-schemas, integrate/v0.4, integrate/v0.3, integrate/product)
base: c28e2bf (integrate/v0.4, released as 0.4.0)
status: v0.5 integration — hardening, schemas and art import merged; version still 0.4.0; all gates green; not merged to main, not pushed, not tagged
---

# Integration status: phases A to E, release 0.2.0

What `integrate/product` holds after integration phases A, B (world-scale society) and D
(tactical and client, via integrate/tactical): the merges, the adapter and fix
commits, which adapters of the [integration plan](integration-plan.md) §5 are done or
pending, the conflicts met, and what is left open. The binding conventions at the end of
`docs/goal-prompts/vocabulary.md` (untracked in the main checkout) override the plan where
they differ.

## Merged branches (phase A)

Each merge is `git merge --no-ff` onto `7f32695`, in this order:

| # | Branch | Merge commit | Textual conflicts | Gate after merge |
|---|---|---|---|---|
| 1 | docs/product-specs | `0eac1c8` | none | green: fmt, clippy, 12 suites ok |
| 2 | feat/tactical-catalogue | `5fac354` | none | green: 14 suites ok; `arda tactical validate` ok (68 assets) |
| 3 | feat/world-server | `9c8e3d9` | none | green: 16 suites ok |
| 4 | feat/npc-population | `466137c` | none | green: 26 suites ok |

Phases B and D merged the rest (see below), except the world track (wip/variant-q,
feat/world-water, feat/world-coast, feat/atlas-polish, fix/uneroded-bands), which waits for
phase E. The review branches are not merged; their fixes were cherry-picked.

"Suites ok" counts `test result: ok. N` lines with N ≥ 1 in `cargo test --workspace`, never
the absence of FAILED.

## Conflicts met

- **No textual conflicts** in any of the four merges; NOTICE, the workspace manifest and
  Cargo.lock all merged automatically.
- **Cargo.lock, silently stale twice.** After the world-server and npc-population merges,
  the merged lock kept a bare `"thiserror"` dependency for `arda-tactical` and `arda-npc`
  while world-server's tree brought in thiserror 2, so cargo rewrote it to
  `"thiserror 1.0.69"` on the first build. Committed as `cacadc0` and `d2db12c` (plan §4:
  never hand-merge the lock; let cargo regenerate it).
- **NOTICE:** the SRD 5.1 paragraph from feat/npc-population merged verbatim.
- `JOURNAL-tactical.md` came in with feat/tactical-catalogue (I34). It is left for the
  maintainer to fold into `JOURNAL.md` or drop.

## Commits after the merges

| Commit | What |
|---|---|
| `ff32481` | `crates/arda-ids`: shared hash/subseed, u64 id newtypes, canonical enums (A6, I21) |
| `04a664d` | arda-npc on arda-ids: string ids, `NpcId` u64, `Biome`, plain-string `BuildingFunction` with `craft:` tags (I5, I7, I22) |
| `cbc96ae` | arda-npc README for the above |
| `2a098cc` | TypeScript types for the arda-npc DTOs via server-owned mirrors (I31, A12) |
| `9c1667d` | `POST /v1/npc/population`, `GET /v1/npc/demo`, `GET /v1/npc/demo/{npc_id}` |
| `9ddd9fc` | sea-cell snowline measured from sea level (I26) |
| `7d0fa4e` | cell contract 2: the I1 cell frame, `centre_x_m`/`centre_y_m` |
| `dfa76b0` | optional `TacticalLayout.origin` (I10) |

### arda-ids

`crates/arda-ids` is serde-only (plus `blake3`):

- `hash(seed, tag, &[i64])`: the SplitMix64 chain of logic/09 §hash, equal to
  feat/tactical-terrain's `hash2`/`hash3` for two and three arguments. `tag(name)` is a
  compile-time FNV-1a rule tag.
- `digest(parts)` and `digest_words`: BLAKE3 over the concatenation, which is arda-npc's
  stream scheme. `subseed(seed, domain, args)`: BLAKE3 derive-key `"arda-ids subseed v1"`.
- Outputs are frozen by golden tests.
- Ids: `SettlementId`, `RealmId`, `BuildingId`, `NpcId` (u64, JSON strings, numbers also read).
  `NpcId::from_parts(settlement, building, index)` is a subseed.
- Enums:
  - `RoadClass`: none=0, track=1, road=2, highway=3, footpath=4, with widths 0/2/4/5/1.
  - `BuildingFunction`: the vocabulary list plus `market_hall`, `mine`, `lumber_camp` and
    `school`, as plain strings. `Craft` and its `craft:<key>` tag travel beside it.
  - `Cover`: none/half/three_quarters/total, with `full` as an alias.
  - `LandUse`: logic/08 codes 0–7.
  - `Biome`: arda-settle's nine values.

### arda-npc changes and their output effect

- The streams feed the byte-identical BLAKE3 input through `arda_ids::digest_words`, so every
  draw is unchanged.
- The market-town example's population was compared before and after with ids normalised
  (old composite `NpcId` and new u64 both mapped to roster position; numeric ids to strings).
  **No difference.** People, jobs, names, personalities, sheets and the relationship order
  are identical. Relationships are sorted by kind, then skeleton order, not by hashed id.
- What changed on the wire:
  - ids are strings;
  - `NpcId` values are hashed u64s;
  - `realm_id` is a string;
  - `biome` is the closed list;
  - `BuildingSpec` writes `"function": "workshop", "tags": ["craft:weaving"]`.
- arda-npc has no golden hashes. Its suites were updated for the new ids.
- New tests: `tests/settle_record.rs` (a feat/settlements-roads record deserialises into a
  `SettlementProfile`) and `tests/building_spec.rs`.
- The roster keeps skeleton order (home building id, then resident index). `job_of` and
  `notable` became linear scans.

### Server

- NPC routes:
  - Limits: a 2 MiB body, a population of 50,000 and 20,000 buildings, each refused with
    413 `payload_too_large` (a new error code).
  - `/v1/npc/demo/{npc_id}` regenerates from scratch. Tests prove it equals every notable's
    population entry and matches sampled commoners' roster entries.
- TS types: 37 new files in `bindings/ts/arda/`. The staleness test is extended with
  `npc_types_are_exported_with_string_ids`. Mirror tests prove that the domain JSON
  round-trips unchanged.
- Contract 2 (I1):
  - Cell `(gx, gy)` covers `[100·gx, 100·gx+100)`, and its centre is `+50`.
  - `x_m`/`y_m` are the north-west corner.
  - `fine.centre_m` is taken at the centre, and the extremes over the whole footprint.
  - `/v1/point` uses floor.
  - Caveat, documented in API.md: the generator still samples the stored cell values at the
    corner. Changing that is a world-format change (phase E).
  - Relief water (goal 49): relief tiles no longer read lakes, coasts or rivers from the
    stored field in their own frame. They query `arda_refine::region::WaterRegion` at pixel
    centres in global square units, so the one I1 conversion (`FINE_ORIGIN_SQ`, 32 squares)
    serves both layers and z9 water agrees square for square with the tactical blocks.

## Phases B and D

### Merges

| # | Branch | Merge commit | Textual conflicts | Gate |
|---|---|---|---|---|
| 5 | integrate/tactical (library, scene, refine, ways, fields, town, arda-blocks, server tactical, viewer-tactical; already green) | `d7c7196` | `Cargo.toml`, server `Cargo.toml`, `lib.rs`, `error.rs`, `routes.rs`, `bindings.rs`, `contract/{mod,convert,snow}.rs`, `API.md`, three TS bindings, `Cargo.lock` | green: 55 suites ok |
| 6 | feat/settlements-v2 (contains feat/settlements-roads and feat/names) | `bafc458` | `Cargo.toml`, `Cargo.lock` | settle and names green |
| 7 | feat/society | `71cc5c5` | none (lock refreshed in `cff38c1`) | society green |

Resolutions of the tactical merge: the `TacticalLayout.origin` field was added by both sides
in the same shape and merged cleanly (one field, one test). Server: NPC and tactical routes,
error codes and `AppState` fields unioned; phase A's contract 2 (I1 frame, sea snowline) kept
and the tactical branch's duplicate sea-snowline test dropped; `API.md` keeps contract 2 and
gains the tactical sections; bindings regenerated with `ARDA_BLESS_BINDINGS=1`; the lock
regenerated by cargo.

### Review fixes

- Round 1: integrate/tactical already carried `03c3bb6`, `6841f11`, `1042801`, `9dcd768`
  (ported as `4cc966a`) and `19f2890`; `d1b33d0` and `d9486c9` came with feat/tactical-library
  (`482ee8e`, `728efe4`); `7234e5d` is superseded by the library compositor's own 5 ft water
  step (`compose/water.rs`, `DEEP_FT`). Cherry-picked here: `2a8b0e5` as `3fbb22f`. `3fd3206`
  skipped (superseded by `04a664d`).
- Round 2 (settle, society, names), cherry-picked with `-x`: `1d085d7`→`8f59efa`,
  `a4d1ff1`→`6370f53`, `51e76e8`→`07a59e7`, `d08c3f6`→`5231033`, `4b1cd38`→`036b9a6`,
  `8bd50ad`→`eb02766`, `8bca217`→`b2883a7`, `7d34226`→`15ad5f2`. `8bca217` touched the old
  single-file `render.rs`, which settlements-v2 split into `render/`; its guards and test
  were ported to `render/guard.rs`. The same commit carries settle's adaptation to `4b1cd38`
  (`NameScope::place_name` now returns `Result`; `SettleError::Names`).
- Round 2 open items closed here: #26 (names enums snake_case, `04fd57f`), #27 (realm
  `members`/`settlements`, `a43d608`), #28 (trade reach, `fbf5ef8`).

### Commits after the merges

| Commit | What |
|---|---|
| `8cdc0d4` | arda-settle on arda-ids: `SettlementId`/`RealmId`, `RoadClass` (compare by `rank()`), `Biome`, building mix keyed by `BuildingFunction`, land-use codes from `LandUse` (which gains settle's stored meadow 8, fallow 9, farmstead 10). A2 test on real MICRO output. |
| `04fd57f` | arda-names: every enum serialises as its snake_case key; `SiteTag::Bridge` as settle's `bridge_site`. |
| `a43d608` | arda-society: `WorldSettlements::read_dir` (the settle → society adapter), realm `settlements` alias with a membership cross-check; a MICRO settle → society test. |
| `fbf5ef8` | arda-society: trade uses each supplier's 48 nearest markets (`Graph::reach_nearest`); a 5,000-settlement, 98,500 km road-lattice test. |
| `797b9ce` | arda-npc: names from arda-names in the settlement's `tongue` (I18/A5); `Generator::with_notables` binds society role slots (A14). Settle writes `tongue` per settlement. |
| `8fee3ae` | I1: one fine-lattice frame (`arda_core::FINE_FRAME_OFFSET_UM`) for server and refine. |
| `a6e497a` | New crate arda-people; `arda settle`, `arda society build`. |
| `2ae6342` | arda-blocks `SocietyOverlays` (A9, A17), server settlement/NPC routes and scene tokens (A13), `tests/e2e_village.rs`. |
| `f1b2688` | Viewer: the cell tab draws the block's NPC tokens. |

### NPCs wired to real settlements (A2, A4, A13, A14, I18)

- `arda society build` plans every settlement with arda-town (a `TownSite` from the record,
  terrain from the world's cells in the I1 frame), passes the plan buildings to arda-society as
  explicit buildings, so every role slot names a plan building, and writes `society.json`.
- Each settlement's buildings are its plan's (`BuildingSpec` with plan capacity and slots, A4);
  without a plan, society's mix-derived buildings sized by arda-town's capacity table. A plan
  or mix that houses too few gets outlying cottages (tagged `off_plan`) after its last id.
- Society's role slots (with ruler and vassal names) become `NotableSlot`s: each binds to the
  building's master, else a worker or resident, else an existing notable or head; the holder
  takes the slot's title, given name, family name and sex, and a household it heads shares the
  family name.
- Names: arda-names replaces arda-npc's syllable generator and `names.json`. The language is
  the settlement's culture with the dialect at its position (`arda_names::Tongue`, recorded by
  settle so both stages speak alike); non-human ancestries use their own preset. NPC and
  settlement names change once (A5): the maintainer should re-read the example outputs.
- Only notables are stored (`society/notables.json`, goal 56); commoners are regenerated.
  MICRO seed 42: 644 settlements (all planned), 90,947 people, 3,446 stored notables, 6 s.

### I1, properly

Stored cell values are the fine field's point samples at lattice `(100·gx, 100·gy)` m, and I1
puts a cell's values at its centre, so the lattice's node `(0, 0)` lies at world `(50, 50)` m.
`arda-core` names that frame; the server's `/v1/point`, fine windows and footprints read the
lattice through it (so `fine.centre_m` equals the stored cell height, checked by a test), and
arda-refine's private `TODO(I1)` shift is replaced by `FINE_ORIGIN_SQ` derived from the same
constant (blocks unchanged, seam tests green). **No world byte changed.** Moving the
generator's cell sampling to the cell centre, which would let the constant become 0, is a
world re-baseline and belongs to phase E with the maintainer's approval.

### Real overlays (A9, A17) and the e2e result

- Ways: settle's roads and crossings through arda-ways, pre-filtered to the window plus
  600 m; river pieces between cell centres as channels.
- Fields: `landuse.bin` through arda-fields (A17: field→arable, mine→mine_quarry, meadow,
  fallow, farmstead, orchard, woodland, mill, pasture; built cells of hamlets and villages are
  farmed as crofts where the plan leaves them free). Culture, region and wealth are fixed so the
  layer is a pure function of global inputs.
- Town: every settlement whose plan can reach the window, in id order; town squares carry
  `ext.building` and `ext.settlement`.
- Demo overlays stay behind `?demo_overlays=1`.
- `tests/e2e_village.rs` (`#[ignore]`, release gate) passes in 61 s: MICRO seed 42 → settle →
  society build → Kinkonin (id 118, riverine village, 551 people, cell 618,689) → its block
  over HTTP with its buildings and walls, `fields` in the 3 × 3 window, 4 NPC tokens that each resolve
  through `/v1/npc/{id}` to an NPC whose home or workplace is the token's plan building, a
  8192 px PNG, and byte-identical repeats on a fresh router. Artefacts: `out/e2e/`.
- Looked at: `out/e2e/village_3x3.png` (3 × 3 cells at 32 px/sq): the main streets run through
  and off the window, buildings stand on land and none in the river; the viewer screenshot
  `out/e2e/viewer_cell_618_689_tokens.png` shows the four tokens inside their buildings.

### Gate on the final commit

`cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo test --workspace --no-fail-fast`: 82 binaries report `test result: ok. N` with N ≥ 1
(1,305 tests, 0 failed); `cargo test --release --test e2e_village -- --ignored` passes; in
`apps/viewer`, `npm run build`, `npm run lint` and `npm test` (69 tests in 7 files) pass.

## Adapter status (plan §5)

| # | Adapter | Status |
|---|---|---|
| A1 | Node frame | **Done.** Server contract 2 (phase A); settle already writes centres `100·gx + 50`; the fine lattice is read in the same frame (`8fee3ae`). |
| A2 | Settle record → `SettlementProfile` | **Done**, tested on every MICRO record (`crates/arda-settle/tests/micro.rs`). |
| A3 | Function keys | **Done** for the code paths: arda-ids holds the full list, settle's mix and arda-town convert by key. The catalogue vocabulary still lacks the four additions (library). |
| A4 | Town building → `BuildingSpec` | **Done** (`arda_people::town::plan_specs`). |
| A5 | arda-names | **Done**: settle (feat/settlements-v2) and arda-npc (`797b9ce`). |
| A6 | Shared record types | **Mostly done**: settle uses arda-ids ids and enums; ways, fields and town still keep local mirrors (converted by serde or by key in arda-blocks/arda-people). |
| A7 | Refine's layout mirror | Done on integrate/tactical. |
| A8 | Sidecar format 2 | Done on integrate/tactical. |
| A9 | `BlockSource` → `RefinedBlock` | **Done** with real society overlays (`SocietyOverlays`). |
| A10 | Global art seeding | Done on integrate/tactical. |
| A11 | Cell contract 2 | **Done** on fix/v4-hardening as cell contract 3 (`e9f0a8c`): `road`/`built_by`/`land_use`/`realm_id` from `society/`, ARDACOLS layout 2. |
| A12 | TS mirrors | **Done for scenes** (feat/v4-schemas): `SceneDto` and its parts, `TacticalScene`, `Token`; the viewer uses them. Settlement and plan bodies have JSON Schemas (from the domain types) but no TS types yet. |
| A13 | Tokens | **Done** (stored notables; commoners get no tokens). |
| A14 | Offices and houses | **Done** (explicit plan buildings to society; slots with house names to arda-npc). |
| A15 | Tag namespaces | Done on integrate/tactical. |
| A16 | Tier ranges | Not re-checked here: settle's hamlet range (12–80) is what arda-npc receives. |
| A17 | Fields land-use classes | **Done** (`arda_blocks::society::fields_class`). |

## Village polish

What changed in the look of world villages on the tactical maps (Kinkonin, a hamlet and the
city Velmuth compared before and after in `out/polish/`; logic/09 §reservations, logic/10
§town-crofts):

- **Crofts fill the footprint** (arda-town): the open ground the plots enclose is a closing of
  plots, buildings, squares and walls; it becomes paddocks, orchards, kitchen gardens, small
  meadows and work yards in oblong parcels, with hedges between parcels, wattle behind plots
  and a rough scrub rim. Buildings, streets and plots are unchanged, so building ids, society
  and notables stay valid.
- **Fields to the footprint** (arda-fields `FieldInputs::cores`): arda-blocks passes the plans'
  own squares instead of the density disc, so farmland runs up to the village edge.
  `FieldInputs::barrier_water` limits the hard water barrier to open water: settle's straight
  centre-to-centre rivers left a strip of forest beside the real, meandering river.
- **Soft claims** (arda-blocks `Owner::Croft`, `Owner::Verge`): crofts yield to water and roads;
  a road verge that kept the natural ground yields to fields and is cleared to grass in forest.
  Ways' channel banks are no longer claimed (they followed settle's straight rivers).
- **Roads inside towns**: within a plan's reach (its longest main street) the town's streets
  replace the world road, so no gravel strip runs beside a street; outside it the road
  continues from the street's end.
- **Organic forest edges**: a noise-shaped fringe (1.5–6 squares of grass, meadow and scrub
  with birch and bushes) on wild ground beside worked land, applied over natural forest only;
  arda-fields woodland margins vary from 1 to 4 squares instead of an even 3.
- **Standing water** (arda-refine `pools`): pools are whole patches around sites on a
  32-square lattice, in marshes, on wet floodplains or in closed hollows of the smooth
  terrain, at least 3 squares in radius; the wetness-driven noise speckle is gone.
- The e2e test now asks the centre block for streets rather than the `ways` overlay: within
  Kinkonin's reach its streets replace the road, and its centre block carries `fields,town`.
  It passes (60 s), with the same 4 tokens each inside its NPC's plan building.
- Seams: a cell composed alone equals its crop of the 3 × 3 window square for square, rule for
  rule, placement for placement and wall for wall (checked on Kinkonin, two hamlets, the city
  and a marsh window). A 3 × 3 society window now composes in about 1.2 s (0.6 s before) on
  first request, mostly the fields' footprint lookups.

## Crossings on the refined river

- **Ways crossings** (arda-ways `plan::wet`, arda-blocks `society::rivers`): the ways layer
  now reads the refined rivers themselves. arda-refine exposes each cell's channel pieces
  (`water::cell_channels`) and the exact square test its blocks rasterise water with
  (`water::RiverWater`, sharing `rivers::bank_distance`); arda-blocks caches the pieces per
  cell and gives them to arda-ways as its water raster plus guide channels. Every stretch
  where a way's centreline runs over that water becomes a bridge or ford spanning all of it,
  up to three rows aside where the water is narrowest; settle's crossing record nearest a
  stretch (within 120 m) gives id and kind, other stretches are bridged on roads and forded on
  tracks. Roads never paint over river water; ways over refined pools or marsh water are drained
  (built up). Tested in `crates/arda-ways/tests/wet.rs` (every water square under a road is a
  deck or ford, none on dry land, the bridge spans the whole river, neighbouring windows
  agree), `arda-refine` `water` (the raster equals the blocks' water) and in the e2e test
  (six rural crossing windows of seed 42: decks and fords in water, no road surface in water).
- **Street mud**: only poor towns (mean building wealth poor) keep mud on unpaved streets, in
  scattered squares; ordinary streets are dirt. **Ditch pools** stand only on wet ground (a
  marsh or low floodplain cell, arda-refine's pool rule), `Terrain::wet_ground`.
- Renders (seed 42, 3 × 3 windows, before and after): `out/crossings/{before,after}_c222.png`
  (road bridge, cell 482,1258), `_c170` (road ford, 587,1147), `_c115` (highway bridge plus a
  brook bridge the record never saw, 467,1044), `_kinkonin` (617,688) and `_town_c66`.

## Town plans on the refined rivers

- **Plan water** (arda-people `rivers`, `town::terrain`): a plan's `TerrainInput` now carries
  arda-refine's river pieces (`water::cell_channels` of every cell within the plan radius plus
  300 m) as its `rivers` (centrelines in metres, each as wide as its widest point, main river
  first) and their exact square test (`RiverWater`, now binned by 16-square tiles and with a
  point query `contains`) as its water mask, instead of settle's straight centre-to-centre
  lines. arda-blocks reads plans through `World::plan`, so blocks, `arda society build` and
  the NPC building lists share one plan per settlement.
- **Streets** (arda-town `plan::wet`, `plan::bridge`, logic/10 §town-rivers): main streets are
  moved onto the nearer bank, a square clear of the water, where their road runs in or beside
  a river, and cross straight, square to the flow, at the narrowest point of the widest channel
  where the road changes bank. Every street stretch over water between land is a deck
  (`TownPlan.bridges`: rows along x or y, whichever is smaller, each carried to its banks);
  lanes cross on footbridges only over gaps ≤ 18 m at more than 45° to the flow; streets that
  would end in water or cross too far are cut at the banks. Plots, buildings, waterfront
  districts, mills and docks follow the real water through the existing rules.
- **Tests**: `plan::check::water` (no footprint but a dock's on river water, every street
  square over water a deck, every deck row reaching a bank or a wall standing in the river),
  run on the four synthetic sites (`crates/arda-town/tests/rivers.rs`, with determinism) and on
  MICRO seed 42's riverine towns, city and largest villages against arda-refine's own water
  (`crates/arda-people/tests/rivers.rs`; all 115 riverine plans were checked once). The e2e
  test gains step 12: the first riverine town's first deck is deck over water in its window (Velmuth: 18 deck squares); it passes in 75 s, Kinkonin's 4 tokens each in its
  NPC's plan building.
- **Society rebuilt** for `out/micro42` (`arda settle` output byte-identical; `arda society
  build`: 644 plans, 90,947 people, 3,454 stored notables (3,446 before), 13 s). Building ids and plans
  changed; no fixture pinned them.
- Renders (seed 42, 3 × 3 windows, before and after): `out/town-rivers/{before,after}_ede`
  (cell 560,868: the long deck beside the river is gone, one crossing at the road's bend),
  `_atail`, `_mivafleth`, `_velmuth` (city).
- Still open: a town wall ring that follows a river runs in the water with repeated water
  gates (Atail); bridges at a confluence are ragged (rows reach into the side channel); mills
  prefer waterfront plots but stand at the plot front, not always at the water.

## Mid-zoom relief (feat/midzoom-refine)

feat/midzoom-refine was cut from integrate/product at `9738874` and merged back with
`git merge --no-ff` as `cd81889`, on top of the village polish, the crossings and the town
plans on the refined rivers (`9d4f401`). It brings:

- the `arda-midzoom` crate: the stored 39.0625 m fine field refined on demand to ~10 m with
  drainage-aligned gullies and ribs (logic/17);
- relief shading entry points in arda-render (`atlas/relief.rs`, `atlas/fine/relief.rs`);
- `GET /v1/tiles/relief/{z}/{x}/{y}.webp` and `tiles.relief_max_zoom` in `/v1/world`
  (`TilePyramidDto`; 12 on MICRO, equal to `max_zoom` for worlds without recipe-5 terrain);
- the viewer's World view switching to relief tiles past the overview's native zoom, with a
  hand-off to the tactical cell.

**Conflicts: none.** The only file both sides touched was `crates/arda-server/API.md`
(different sections), which merged cleanly; routes, `AppState`, the viewer and Cargo.lock
merged automatically. `ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings` left the
committed bindings unchanged (the branch already carried `TilePyramidDto.relief_max_zoom`).

Gate on the merge commit: `cargo fmt --check` and `cargo clippy --workspace --all-targets
-- -D warnings` clean; `cargo test --workspace --no-fail-fast`: 90 binaries report
`test result: ok. N` with N ≥ 1 (1,338 passed, 0 failed, 14 ignored); `cargo test --release
--test e2e_village -- --ignored` passes (70 s); in `apps/viewer`, `npm run build`,
`npm run lint` and `npm test` (72 tests in 8 files) pass. Smoke test of `arda-server` on
`out/micro42`: `/v1/tiles/relief/7/55/95.webp` 200 (lossless WebP, 87 KB, 0.46 s cold),
`/v1/tactical/cell/618/689` 200 (1.0 s cold), its scene's first token resolves through
`/v1/npc/{id}` (200) and `/v1/npc/demo` 200.

## Phase E and release 0.2.0

### Merge

`integrate/world` (variant Q, fix/uneroded-bands, feat/world-coast, feat/world-water,
feat/atlas-polish and the full-size fix round, 44 commits over `7f32695`) merged with
`git merge --no-ff` as `4b741df`.

**One textual conflict**, `crates/arda-render/src/atlas/fine.rs`: midzoom had moved the
per-sample land and sea shading into `FineAtlas::land_inputs` and `sea_colour`
(`atlas/fine/relief.rs`), while world added stored shore painting inline. Resolved on
midzoom's structure with world's behaviour: `land_inputs` reads the shore mix into
`SavedMaterial`, `sea_colour` tints through `formed::shore_water`, and `relief_colour` uses
both, so relief tiles and the overview share one shader, palette and shore painting. Inside
the merge commit, for the build and the hand-off: `arda::area_atlas_terrain` takes the shore
layer (`ReliefWorld` reads `World::shore` once, under its build lock);
`AtlasTerrain::relief_colour` passes the shore classes; `formed_river_rgb` calls the
overview's `formed_river_colour`, so relief rivers take atlas polish's palette. Docs
(logic/02, logic/04, changelog.d), `Cargo.toml` and `Cargo.lock` merged cleanly.

### Commits after the merge

| Commit | What |
|---|---|
| `adc2ea6` | Version 0.2.0: workspace package, every workspace path dependency, viewer `package.json` and lock root. The Classic golden world moves only `world.json` (its `arda_version`); that file with `"0.2.0"` put back to `"0.1.0"` hashes to the old value, and every other file keeps its hash. |
| `127e919`, `829ded3` | Relief rivers follow the overview's formed centrelines: arda-render exposes `FormedRiverNetwork`, `formed_river_centreline` and `formed_source_width` (sharing `Network::relaxed` and `bspline_at` with the overview's `curved_strip`); arda-midzoom draws each edge along them. Before, relief used an unrelaxed Catmull-Rom chain, so meanders turned into D8 staircases past the overview's native zoom. |
| `747ebf7` | world_api re-pinned to two variant-Q river cells (below). |
| `76bba89` | The midzoom warm-tile budget takes the best of three tiles (68–82 ms alone; it hit 201 ms twice in the full parallel suite). |

### Fixture world and re-baselines

`out/micro42` regenerated with `arda generate --seed 42 --micro --terrain fine` (55 s,
land 293‰, 8 rivers, `terrain/shore.bin` and eight `areas/*/water.bin`), then `arda settle`
(629 settlements, 648 roads, 3 realms) and `arda society build` (629 plans, 91,966 people,
3,384 stored notables, 7 s). Before: 644 settlements, 90,947 people, 3,454 notables. The
previous world is kept as `out/micro42.pre-e/`.

Re-baselined, recipe-5 only:

- `world_api` `cell_and_point_return_contract_json`: `(512, 1036)` is now dry forest; `(512, 1039)`
  carries an order-4 river (cy 15, point query 300 m south).
- `world_api` `RIVER`: `(530, 810)` has no watercourse; `(529, 812)` lies on an 11.8 m order-4
  river (580 water squares in its block).

Nothing else pinned the world: the e2e test picks its village, crossings and town bridge
dynamically, and the golden world is Classic.

### e2e result

`cargo test --release --test e2e_village -- --ignored` passes (185 s): the village is
**Ylkin** (id 24, riverine, 524 people, cell 570,702), 4 tokens each resolving through
`/v1/npc/{id}` to an NPC of its plan building, overlays `fields,town`; six rural crossing
windows hold 11 deck and 44 ford squares; town 1's first bridge has 17 deck squares over water.
Artefacts: `out/e2e/village_24_*`; looked at `out/release/e2e_village_24_3x3_small.jpg`
(streets, crofts and fields around a brook with two bridges; no building in water).

### Mid-zoom on the new terrain

Relief crops (`arda-midzoom`, 1024 px at z 5/6/7 = 25/12.5/6.25 m/px) in `out/release/`:
`relief_mountain_z{5,6,7}.png` (centre 76.8, 140.8 km) and `relief_coast_z{5,6,7}.png`
(44.16, 78.72 km, the northern estuary), with `cmp_{mountain,coast}_ov_vs_z5.jpg` (the 4k
Atlas overview crop, upscaled, beside the z 5 relief) and `cmp_coast_river_detail.jpg`.
Palette, light, rock, beaches, estuary and shallows match the overview across the switch;
the main river keeps the overview's meander bends; ridges and gullies read crisp at 6.25 m.
Warm 1024 px crops take 165–516 ms.

### Gate

- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast` on `76bba89`: 90 binaries report `test result: ok. N` with
  N ≥ 1 (1,412 passed, 0 failed, 14 ignored).
- `cargo test --workspace --release -- --ignored`: 7 passed (anisotropy, near-rim lakes,
  refine real-world ×2, society scale timing, tactical timing, coast source control with
  `ARDA_COAST_MEASUREMENT_OUT`), plus the e2e test. Skipped as full- or default-size:
  `measure_seed42_fine_macro_against_delivered_profile`, `hypsometry_default_size`,
  `measure_rainfall`, `measure_horton`, `reported_boundary_patch`,
  `measure_accepted_coast_geometry`.
- `apps/viewer`: `npm run build`, `npm run lint`, `npm test` (72 tests in 8 files) pass.
- `arda tactical validate assets/tactical/placeholder`: ok (207 assets).

### Smoke test (`arda-server --world out/micro42`, stopped by PID)

| Request | Result |
|---|---|
| `/v1/world` | 200; `arda_version` 0.2.0, `max_zoom` 4, `relief_max_zoom` 12 |
| `/v1/cell/570/702` | 200 |
| `/v1/tiles/relief/7/55/95.webp` | 200, 82 KB WebP, 0.57 s cold; `12/900/1800` 200 in 20 ms |
| `/v1/tiles/overview/4/5/10.png` | 200, 256 px, 4.7 s cold, 0.3 ms cached; `0/0/0` 200 |
| `/v1/overview.png?quality=1024` | 200 |
| `/v1/tactical/cell/570/702.png` | 200, 8192 px PNG, 2.0 s |
| `/v1/tactical/cell/570/702/scene` | 200, 4 tokens |
| `/v1/npc/6813388508269287032` (first token) | 200, Tirif Rin, with sheet |
| `/v1/settlements?tier=village`, `/v1/settlements/24/plan` | 200 (134 villages) |

Server log: no errors.

## Left open

- **Not done by instruction:** no merge to main, no tag, no push; `JOURNAL-tactical.md`
  untouched (the coordinator strips it). Changelog fragments stay unfolded: the capstone
  convention folds `changelog.d/` on the default branch. Release notes are in `CHANGELOG.md`.
- **I1 generator side** (cell sampling at the centre, which would let `FINE_FRAME_OFFSET_UM`
  become 0) was not part of the world track and stays open; it needs its own re-baseline.
- **Shore classes at close zoom:** the stored shore layer is a 100 m grid; at ≤ 12 m/px its
  beach, estuary and shallow-water edges show as 100 m steps (`relief_coast_z7.png`). The
  overview shows the same steps at its own scale; smoothing them is a formed-shader change.
- **Relief draws more small streams** than the overview (its minimum width is 1.2 px, the
  overview's fades small channels out); a deliberate mid-zoom choice, but visible at the switch.
- **Variant Q sign-off:** the look is approved per the brief; `2026-09-30-world-integration.md`
  still says "Not signed off".
- ~~**A11**: `CellSample.road`, `built_by`, `land_use`, `realm_id` from `society/`~~: closed on
  fix/v4-hardening (cell contract 3, see the last section).
- **A12**: ts-rs mirrors for `TacticalScene`/`Token`, settlement records, `TownPlan`.
- ~~`/v1/npc/{id}` serves stored notables only~~: closed on feat/v3-api (see below);
  commoners resolve by reference `<settlement>.<building>.<index>`.
- ~~Round-2 items still open: #29 (other quadratic scans in society), #30, #31 (settle A* and
  admission), #32 (tactical encode bound), #33–#38 (ways/fields), plus the low ones~~: closed
  on fix/v4-hardening except #30, #41 and parts of #38, #42, #43 (reasons in the last section).
- ~~Files over ~500 lines that came with merged branches~~: split (the v0.4 file-size branch and
  fix/v4-hardening).
- Earlier phase A items still open: `logic/13`/`logic/16` still describe `NpcId` as a composite
  string; `JOURNAL-tactical.md` (I34).
- The e2e test is `#[ignore]` (the plan's release gate): run
  `cargo test --release --test e2e_village -- --ignored`.

## v3 API: goals 51, 57, 67, 68 (feat/v3-api)

| Commit | Change |
|---|---|
| `6da5532` | `/v1/npc/{id}` resolves commoners by reference `<s>.<b>.<i>` (or u64 id with `?settlement=`); `/v1/npcs` paged queries (settlement, building, job, realm, notable; ≤ 200 people and ≤ 16 settlement skeletons a page); `/v1/buildings/{s}.{b}/residents` and `/workers`; lazy skeleton access in arda-npc; `NpcPage`/`NpcEntry` bindings. |
| `b488b22`, `ce2c37d` | `/v1/tiles/overview/{z}/{x}/{y}.webp` (lossless, same pixels as the PNG); the viewer uses it. |
| `0aa4637` | `POST /v1/tactical/prefetch` with a bounded background worker pool and a separate prefetch render lane; cache key confirmed (seed, origin, `library_version`). |
| `fb36b4b` | Viewer: the Cell tab prefetches the 8 neighbours; edge arrows walk to the seamless neighbour. |

### Gate

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 90 binaries report `test result: ok. N` with N ≥ 1
  (1,428 passed, 0 failed, 14 ignored).
- `cargo test --release --test e2e_village -- --ignored`: passes (77 s).
- `apps/viewer`: `npm run build`, `npm run lint`, `npm test` (77 tests in 9 files) pass.

### Smoke test (`arda-server --world out/micro42 --port 8797`, release, stopped by PID)

Fixture: `arda generate --seed 42 --micro --terrain fine`, `arda settle`, `arda society build`
(629 settlements, 91,966 inhabitants, 3,384 stored notables).

| Request | Result |
|---|---|
| `/v1/npcs?settlement=85&notable=false&limit=3` | 200, 0.39 s cold; refs `85.5.1`, `85.6.1`, `85.6.2` |
| `/v1/npc/85.5.1` | 200, 4 ms, an acolyte with its sheet |
| `/v1/npc/{its u64 id}?settlement=85` / without `?settlement=` | 200 / 404 |
| `/v1/npcs?realm=2&job=smith&limit=20` | 200, 2.5 s cold, 19 smiths, 16 settlements scanned, cursor `89.0` |
| `/v1/buildings/85.5/residents`, `/v1/buildings/85.{workplace}/workers` | 200 |
| the village block's first scene token through `/v1/npc/{id}` | 200 |
| `/v1/tiles/overview/2/1/2.webp` | 200, `image/webp`, lossless with alpha; `.png` still 200 |
| `POST /v1/tactical/prefetch {632,707, ppsq 64}` | 202, 8 cells queued with images; each warmed in 0.57–1.1 s by 2 workers; meanwhile the centre's tile answered in 0.73 s and `/v1/cell` in 0.12 s |
| the east neighbour afterwards | JSON `X-Arda-Cache: hit`; a z3 tile in 5 ms |
| `radius: 3` | 400 `bad_request` |

### Left open

- A bare u64 id of a commoner needs `?settlement=`: the id is a hash of (settlement, building,
  index) and resolving it alone would need a world-wide index (goal 56).
- Realm- or world-wide queries cost about 0.15 s per scanned settlement cold (town plan and
  skeleton); the plan cache holds 64 settlements.
- Prefetch is explicit (the viewer posts it); `GET /v1/tactical/cell` does not enqueue its
  neighbours by itself as logic/16 §api-prefetch sketches. At 128 px per square the default
  caches warm images for one neighbour only.
- logic/13 §npc-id and logic/16 still call the composite string the `NpcId`; on the wire it is
  now the NPC *reference*, beside the u64 id.

## v0.3 integration (integrate/v0.3)

Five branches, each off `main` at v0.2.0 (`7996e9c`), merged with `git merge --no-ff` in this
order, with the Rust gate (fmt, clippy, workspace tests) green after each merge:

| # | Branch | Merge | Textual conflicts | Tests passed after merge |
|---|---|---|---|---|
| 1 | feat/v3-recipe6 | `129018f` | none | 1,420 (91 suites) |
| 2 | feat/v3-realms | `62493a5` | none | 1,426 (92) |
| 3 | feat/v3-api | `f30c519` | none (Cargo.lock, API.md, bindings, e2e auto-merged); viewer build, lint, 77 tests pass | 1,442 (92) |
| 4 | feat/v3-tactical | `47ca664` | `arda-server/src/tactical/mod.rs`: v3-api moved the render into `raw.rs` (request and prefetch lanes); v3-tactical's region render for crops now runs inside `raw_in` | 1,453 (94) |
| 5 | feat/v3-town-wfc | `7a9c8a0` | `arda-town/src/block/mod.rs` module list: both `block::yard` and `block::wfc` kept; `generate` defaults to `TownFill::Wfc`, `TownFill::Rules` and every relaxed WFC fill use v3-tactical's varied interiors | 1,475 (97) |

### Commits after the merges

| Commit | Change |
|---|---|
| `e77bc21`, `184de67` | **Varied interiors feed the WFC.** A home's partition, trade, beds, fire and clutter pools (v3-tactical's choices, keyed by the interior salt) become its WFC programme; the WFC keys carry the salt. Trades work in a back room (an open hall keeps its trade), small houses stay one room, clutter uses low-weight pieces, so relaxed homes stay at the v3-town-wfc level. Seed-42 city: 1,665 distinct WFC home interiors of 1,673, 1.0 % repeated, no adjacent pair alike (`wfc::indoor::diversity`). |
| `43afd0b` | **PNG encoder fix (found while rendering).** v3-tactical's parallel encoder could end a strip before its sync flush was complete, corrupting every strip after it; the 2048 × 2432 px window of the city warehouses was served as a broken PNG. A strip now ends only on its `00 00 FF FF` marker. |
| `bb17425` | **Follow-up 1, ordered furniture.** `block::wfc::rows` lays pews (aligned rows either side of a clear central aisle), dormitory beds and reading-room bookshelves (along both long walls), warehouse racks and market-hall tables (free-standing rows with walks and end passages) by rule before pass B; the WFC fills around them. Tests: `arda-town/tests/wfc_rows.rs` (nave pews in aligned rows with a clear central aisle; dormitory beds) and `arda-people/tests/interiors.rs` (the seed-42 city's barracks, warehouses, libraries, market hall and temples). |
| `2a8e4b0` | **Follow-up 2, v0.2.0 worlds.** `read_manifest` reads "recipe 5 and `terrain/shore.bin`" as recipe 6 (logic/02 §fine-formation recipes); world.json is untouched. A v0.2.0 MICRO seed-42 world now reports recipe 6 from `/v1/world`. |
| `b35bcef` | **Follow-up 3, relaxed fills in meta.** The town layer marks the squares of `TownBlock.relaxed` problems; `compose_all` returns them (`Applied.review`) and the pipeline merges them into the block meta, so cell and window `meta.relaxed` and `meta.review_squares` count them (cell 565,651: a relaxed 6 × 5 house, `true`, 30 squares). |
| `18931fd` | **Follow-up 4, `prop.drain`.** Art-free (`outdoor_vocab::ART_FREE`): the WFC still places drains along paved kerbs, but their props stay out of town layouts, so no fallback is logged and nothing unknown reaches a library. |

Relaxed WFC fills on MICRO seed 42: 43 of 44,819 problems (0.096 %; v3-town-wfc alone: 45 of
47,247; realm primacy changes the plans).

### Renders (MICRO seed 42, city Yefborkitre, 64 ppsq; market 32 ppsq)

`out/v3-integrate/before/` (after the five merges) and `out/v3-integrate/after/` (this
branch): `temple.png`, `warehouse.png`, `market.png`, `barracks.png`, `library.png`, each with
a 1600 px `.jpg`. After: temple pews stand in aligned rows with a clear central aisle; barracks
beds line both long walls; library bookshelves line the long walls; warehouse racks stand in
rows; the market hall's tables stand in two rows (the market stalls themselves are plan-fixed
and were already in rows).

### Gate on `18931fd`

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 98 binaries report `test result: ok. N` with N ≥ 1
  (1,483 passed, 0 failed, 17 ignored), including `tests/golden_recipes.rs` (recipe 5 replays
  v0.1 and recipe 6 replays v0.2.0 byte for byte).
- `cargo test --release --test e2e_village -- --ignored`: passes (91 s); goal 50: cold block
  316 ms, cold quarter window 85 ms (best of 3); town WFC relaxed rate 0.096 %.
- `apps/viewer`: `npm run build`, `npm run lint`, `npm test` (77 tests in 9 files) pass.
- `arda tactical validate assets/tactical/placeholder`: ok (207 assets).

### Smoke test (`arda-server --world out/micro42 --port 8931`, release, stopped by PID)

Fixture: `arda generate --seed 42 --micro --terrain fine`, `arda settle`, `arda society build`
(629 settlements, 91,896 inhabitants, 3,397 stored notables, 3 realms).

| Request | Result |
|---|---|
| `/v1/world` | 200; recipe 6 |
| `/v1/npcs?settlement=85&notable=false&limit=3` | 200, 90 ms; first ref `85.5.1` |
| `/v1/npc/85.5.1`, `/v1/buildings/85.5/residents` | 200, 4 ms each |
| `/v1/npcs?realm=2&job=smith&limit=20` | 200, 0.41 s |
| `/v1/tiles/overview/2/1/2.webp` / `.png` | 200 `image/webp` (4.1 s cold) / 200 `image/png` |
| `POST /v1/tactical/prefetch {570,702, radius 1, ppsq 64}` | 202, 8 cells queued with images; `radius: 3` is 400 |
| `/v1/tactical/window.png?gsx=36528&gsy=44976&w=32&h=32&ppsq=64` | 200, 0.25 s, decodes |
| `/v1/tactical/window.png?gsx=36434&gsy=41784&w=32&h=38&ppsq=64` | 200, decodes (was corrupt before `43afd0b`) |
| `/v1/tactical/cell/565/651`, window over it | `meta.relaxed` true, `review_squares` 30 |
| `/v1/tactical/cell/570/702/scene` | 200 |

Server log: no errors.

### Left open

- Not done by instruction: no version bump, no merge to main, no tag, no push.
- The front-workroom partition of the rule programmes is drawn by the WFC as a hall with the
  workroom behind it: a workroom at the street left the WFC no legal fill for the hall.
- Yards and crofts: under `TownFill::Wfc` the outdoor WFC dresses them; v3-tactical's yard and
  croft dressing (`block::yard`) serves the rules fill only.
- The placeholder `prop.cask_rack` and `prop.shelf` art is narrower than the WFC footprints
  (racks read as 1-square sprites on 2-square footprints); new rack art would read better.
- The encoder test does not reproduce the lost flush synthetically; the warehouse window above
  is the reproduction (checked by decoding it).
- Earlier open items above still stand (A11, A12, round-2 items, files over ~500 lines).


## v0.4 integration (integrate/v0.4)

Four merges onto `main` at v0.3.0 (`225db90`), `git merge --no-ff` in this order, with
fmt, clippy and the workspace tests after each. fix/v4-zoom-geometry carries
feat/v4-open-country (it was branched from it).

| # | Branch | Merge | Conflicts and how they were resolved | Workspace tests after merge |
|---|---|---|---|---|
| 1 | refactor/v4-file-size (48 pure-move splits) | `b2de9f7` | none | 1,482 passed, 1 failed: the known `arda-people --test interiors` fixture race (fixed in `7bddafb`) |
| 2 | fix/v4-zoom-geometry (+ open country) | `e784299` | `arda-render/src/atlas.rs`, `atlas/fine.rs`: import lists only; the split's module imports kept, the relief exports become `formed_river_rgb, ReliefGeometry, ReliefSurface` | 1,501 (99 suites) |
| 3 | feat/v4-arid | `8a0ebd3` | Tests inlined on the branch but moved by the split (`formation/drainage.rs`, `water/delta.rs`, `water/oxbow.rs`, `arda-cli/src/main.rs`, `atlas/tests.rs`): the branch's edits (u16 runoff weights, the `runoff` argument, `--latitude` and recipe-7 CLI tests, the `salt` fixture field) re-applied in `*/tests.rs` and `tests/shore.rs`. Arid atlas code re-applied in the split modules: `with_salt` in `atlas/sampling.rs`, `salt_near`/`arid_near` in `atlas/fine/saved.rs`, the saline-lake and pan colours in `atlas/fine/shading.rs`, `salt` in the formed `SavedFields`. `arda-refine`: the playa field joins geo's `Ctx::assemble` (blocks and `WaterRegion` both gather `pans`); `block.rs` keeps open country's `trail` ground first, then salt crust and mudflat, and drops scatter on the playa after open country's new `scatter`. Ground `TYPES` 31 (trail + salt_crust + mudflat), schema `GROUND` 23, `Cargo.lock`/`Cargo.toml` dev-deps (`arda-core`, `arda-midzoom`). `catalog.json` regenerated (`arda tactical placeholders`): every committed PNG of both branches is byte-identical to the regenerated one; library 217 assets (207 + 6 open country + 4 arid), no id clashes. `docs/goal-prompts/vocabulary.md`: the tracked file, plus the open-country additions | 1,518 passed, 1 failed (`world::tests::declared_fine_layer…`, fixed in `70dd879`) |
| 4 | feat/v4-look-options | `651a77b` | `arda-midzoom/src/tile.rs`: geo's `shade_pixel` (water from `WindowWater`) returns look's land flag; the world-grade lattice (`lattice.rs`) gathers a `WindowWater` over its own bounds (`WindowWater::gather_um`), so the grade's land and water are the tactical geometry. `overview.rs` module list (split's `rivers`/`sampling` + `oblique`/`oblique_rows`); the streaming test and CLI style test edits re-applied in the split test files; `compose/mod.rs` keeps `weights` and `world_tint` | 1,533 (100) |

### Commits after the merges

| Commit | Change |
|---|---|
| `7bddafb` | **Interiors fixture race.** Both tests of `arda-people --test interiors` generated `target/arda-people-fixture/micro42` at once (`OutputNotEmpty`); the world is now resolved once per test binary (`OnceLock`). Checked from an empty fixture directory. |
| `2608d11`, `70dd879` | **Recipe 6 stays the default.** `FINE_TERRAIN_RECIPE_VERSION`, `Recipe::DEFAULT`, the new `FineRecipe::DEFAULT` (used by `generate_from_fine_source`, so every fixture world) and the CLI default are 6 again. The newest readable recipe is the new `FINE_TERRAIN_LATEST_RECIPE_VERSION` (7): manifests, `World::load` and the orchestrator accept 1..=7. Recipe 7 stays available with `--recipe 7` and pinned by `recipe_7_is_pinned`; a unit test pins the default at 6 on every path. CLI help, logic/02, `CHANGELOG.md` and the arid changelog fragment say recipe 7 is opt-in pending review. |
| `67656f7` | **Recipe-7 tactical ground through the server.** `arda_people::shared::SharedSource` (the server's block source) did not forward `Source::pan`, whose default answers "no pan", so salt pans showed ordinary ground on served tactical maps although `arda-refine` drew them. Forwarded, with a test. |
| `77547c8` | Close-zoom relief draws recipe-7 saline lakes and salt pans as the overview does (empty `salt` before recipe 7, so relief bytes of recipes 5 and 6 are unchanged). |
| `8cb137d` | **Zoom continuity on an arid world.** `tests/zoom_continuity.rs` adds `relief_and_tactical_water_agree_on_an_arid_world`: MICRO seed 74, `--recipe 7 --latitude 15,35` (or `ARDA_ZOOM_ARID_WORLD`); 8 saline-lake shore cells plus 3 salt-crust and 3 mudflat cells, chosen from the stored lake forms and playa runs. IoU 1.0 over 30,225 water squares; the pan cells are dry `salt_crust` (7,033 squares) and `mudflat` (7,783). |
| `c14c602`, `85f3657`, `0408281` | Files back under ~500 lines: `arda-cli` `generate.rs` out of `main.rs` (511 → 418), `fine_delivery` tests into their module (510 → 410), `area/lakes/tests.rs` (570 → 352, `tests/diagonal.rs`) and `area_objects_v4/tests.rs` (546 → 424, `tests/divergence.rs`). |

### Gate on `0408281` (docs commit aside)

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 100 binaries report `test result: ok. N` with N ≥ 1
  (1,534 passed, 0 failed, 19 ignored), including `tests/golden_recipes.rs` (recipe 5 replays
  v0.1, recipe 6 replays v0.2.0, recipe 7 its pinned first output; the oblique look is opt-in)
  and `tests/arid_recipe7.rs`.
- `cargo test --release --test e2e_village -- --ignored`: passes (90 s); goal 50: cold block
  324 ms, cold quarter window 85 ms (best of 3); town WFC relaxed 43 of 44,819 (0.096 %).
- `cargo test --release --test zoom_continuity -- --ignored`: both pass on freshly generated
  worlds. MICRO 42 (recipe 6): IoU 1.0 over 5,102 water squares in 11 cells; MICRO 74
  recipe 7: IoU 1.0 over 30,225.
- `apps/viewer`: `npm run build`, `npm run lint`, `npm test` (79 tests in 10 files) pass.
- `arda tactical validate assets/tactical/placeholder`: ok (217 assets).

### Smoke test (release `arda-server`, each stopped by PID)

MICRO 42 (recipe 6, settled, society built) on port 8941:

| Request | Result |
|---|---|
| `/v1/world` | 200; `fine_terrain.recipe_version` 6 |
| `/v1/cell/570/702`, `/v1/tactical/cell/570/702/scene`, `/v1/npcs?settlement=85&notable=false&limit=3` | 200 |
| `/v1/tiles/overview/2/1/2.webp`, `?oblique=1` | 200 `image/webp` (4.0 s / 4.2 s cold) |
| `/v1/overview.png?quality=1024&style=atlas-oblique` | 200 |
| `/v1/tiles/relief/7/55/95.webp` | 200, 0.12 s |
| `/v1/tactical/cell/465/1166` (JSON, PNG) | 200; `world_grade=0` is byte-identical to the plain PNG, `world_grade=1` differs |

MICRO 74 recipe 7 at 15–35° on port 8942: `/v1/world` reports recipe 7; the 1024 px overview
shows the saline lake on its white pan; `/v1/tiles/relief/9/76/146.webp` 200;
`/v1/tactical/cell/304/1169` is 3,148 `salt_crust` and 461 `mudflat` squares;
`world_grade=1` PNG 200. Both server logs: no errors.

### Left open

- Not done by instruction: no version bump (still 0.3.0), no merge to main, no tag, no push.
  Changelog fragments stay unfolded.
- **Recipe 7** is the default since v0.4.0 (maintainer approval, 2026-10-01); `--recipe 6` reproduces v0.2–v0.3 worlds.
- ~~`Source::pan` has a default (`Ok(None)`)~~: required on fix/v4-hardening (`24691ec`).
- ~~Test files the split itself left just over 500 lines~~: split by pure moves on
  fix/v4-hardening.
- Earlier open items above still stand: A12 (A11 and the round-2 items are closed on
  fix/v4-hardening).

## v0.4 hardening (fix/v4-hardening)

Branched from `integrate/v0.4` at `7b1d1a0`; 37 commits, no merges. Every fix has a
regression test that cannot pass on the unfixed code; for most items this was checked by
running the test with the fix stashed. Exceptions are noted. Outputs: MICRO 42 `settle` and `society build` files are byte-identical to
integrate/v0.4's apart from the new `roads.bin` and the last digit of the rank-size fit in
`stats.json`; the golden recipes replay unchanged.

### Contract and trait changes

| Commit | Change |
|---|---|
| `24691ec` | `arda_refine::Source::pan` has no default: a wrapper that does not forward it fails to compile (a `compile_fail` doctest pins it; a unit test shows the same wrapper with `pan` compiles). Audit of the other defaulted trait methods: `arda_ways::Terrain` (`slope`, `channels`, `rivers_rasterised`, `river_water`, `wet_ground`) and `arda_blocks::Overlays` (`ways`, `fields`, `town`) default to "feature off" and have no wrapping implementation (each implementor is a leaf); `Source::clamp` is derived from `cells_wide_high`. Left as they are. |
| `f11e283` | `arda settle` writes `society/roads.bin` (`ARDARDS`, u8 `RoadClass` codes; logic/08 listed it, settle never wrote it); `output::read_road_map`. |
| `e9f0a8c` | **Cell contract 3 (A11).** With `society/`, `road` comes from `roads.bin` (gains `footpath`), `built_by` from the `landuse.bin` owners and is a settlement id **string**, and `land_use` and `realm_id` (string) are new. Without `society/` the stored `road`/`built_by` stand and the new fields are `null`; a society without `roads.bin` keeps the stored road. A society raster of another size stops startup. `ARDACOLS` layout 2 adds `land_use` (u8) and `realm_id` (u32) and the `footpath` code. Bindings regenerated (`LandUseDto` new); the viewer accepts contract 3 and layout 2 and shows land use and realm; API.md, logic/16 and the e2e test updated. |

### Review items closed

| Item | Commit | Fix |
|---|---|---|
| R1 #14 | `0cdf2c0` | Catalogue footprints over 16 squares a side are an `image_size` issue (cached sprites stay ≤ 2048² px a variant). |
| R1 #15 | `cfe82c4` | Error bodies shorten absolute paths to `…/<file>`; 5xx errors are logged in full. |
| R1 #16 | `00c3f8b` | Overview qualities: 512 doubling to `--max-overview-px` (plus the limit); others are 400 before any render. |
| R1 #17 | `8687758` | The out-of-world message cannot underflow; a world with no areas is refused at open. |
| R1 #18 | `d245eb9` | The 20k-city wall-clock bound moves to an ignored release test (best of three); storage checks stay. No fail-before test (a flake). |
| R1 #19 | `6645772` | `whileLive` drops responses that land after their request was aborted (cell fetch, tactical PNG, prefetch, tokens, layout renders, `useAsync`). |
| R1 #20 | `51ea42f` | Resistances drop non-adjacent duplicates. |
| R1 #21 | `6e14e3d` | Weighted draws over totals beyond u32 use a 64-bit draw; totals within u32 roll as before. |
| R2 #23, #24 | (earlier) | Verified closed by A8 on integrate/tactical: ways and fields emit a format-2 `RulesSidecar` (round-trip tests), the scene takes parapet `blocks_sight` from the sidecar edges. |
| R2 #25 | `012fb70` | Ways ids are u64; unrecorded crossings set bit 62 (bit 30 hid real ids from toll houses); synthetic channels carry their crossing's id. |
| R2 #29 | `ba3657c` | Society: `HistoryIndex`, `HookLookups`, per-run flow sums and indexed realm lookups replace per-settlement and per-pair scans, keeping record order. 8,000 settlements 1.0 s (5.1 s before); 2,000 → 8,000 settlements costs 5.8× (16.4× before; realm relations stay R² by format). MICRO 42 `society.json`/`notables.json` and the tiled world's JSON digest unchanged. Tests: `lookups.rs` (every index against its scan), ignored release gate `society_time_grows_near_linearly`. |
| R2 #31 | `da7e503`, `5c33283` | `encode` builds every society file before `write` touches the directory; `tests/memory.rs` measures the peak (56–78 B a cell past a 16 MiB fixed allowance; about 90 B a cell for MICRO 42 in all) against the admitted 128 B. No fail-before test for the write order (it needs an allocation failure). |
| R2 #32 | `f566465` | Tactical PNG encodes and pyramid builds share one slot across both lanes; admission counts one transient encode (10 B/px at the limit). |
| R2 #33 | `f0f0365` | Ways leave the layout's lakes and sea unpaved, unrepainted and at their elevation; a crossing over them gets a guide-only synthetic channel. |
| R2 #34 | `130da2b` | A crossing is planned in every window its synthetic channel reaches (`max(1.5w + 150, 150 + max(3w, 60) + w/2 + 1 square)`). |
| R2 #35 | `045342f` | A way is reindexed right after a crossing straightens it. |
| R2 #36 | `a679f44`, `34ce7c4`, `1e77077` | `plan::relevant::relevant_roads`: roads within 500 m of the window plus, to a fixed point, roads within 64 m of a kept road's endpoint; a unit test checks the window draws byte-identically to planning every road. Report counts now near-window only. |
| R2 #37 | `c9edd9c` | Mines and quarries move to the first of eight sites 16 squares around their own that is clear of roads, else give up the road squares. |
| R2 #38 a–c | `6792bfa` | Open fields are `complete` when the window holds all their squares; `LandUseGrid::filled` refuses rasters over 2²⁸ cells; furrows take the edge square's weight past the window edge. |
| R2 #39 | `158ea40` | `squares::derive` checks the square count and sidecar size; a sidecar clearing a canopy's heavy obscurement leaves it light; secret doors are no spawn entrances; the light doc names the lamp's two radii. No fail-before run for `derive` (its old signature cannot compile the test). |
| R2 #40 | `f7be2b3` | The four-corner cover count (DMG grid procedure), the sightline vertex rule and the diagonal corner and squeeze rules are labelled house rules in code and README. Docs only. |
| R2 #42 (part) | `bedc92c` | `fresh_root` keeps drawing (to 4,096) instead of returning a blocked or repeated root; the place fallback never respells into a blocked word. The whole-word blocklist was already fixed (fragments). |
| R2 #43 (part) | `cef5b6e` | The rank-size fit in `stats.json` uses `num::ln` (IEEE basic operations only) instead of libm's `ln`. |
| R2 #44 | `c79f79c` | Every image (overview, relief, tactical) carries a strong ETag and `Cache-Control: no-cache`; `If-None-Match` answers 304. |
| R2 #45 | `669fa48` | `tests/support/fixture_dir.rs` builds shared fixtures in a private sibling and renames them into place (server, settle, people, midzoom fixtures); `tests/fixture_dir.rs` races six builders. |

### Skipped, with reasons

- **R2 #30** (settle A* window and heuristic): the search is already one multi-target A* per
  village over a bounded window; a stronger heuristic or candidate pruning changes heap order
  and therefore road output, which needs a settle re-baseline approved by the maintainer and a
  full-size measurement. MICRO 42 settles in 2.5 s.
- **R2 #38 d** (compounds rebuilt per window): performance only; a cross-window compound
  cache needs shared state in the blocks' fields layer. The e2e goal-50 checks pass and a cold
  village block serves in 0.2 s.
- **R2 #41** (roots keyed by meaning hash behind a language version): renames every existing
  world; a language-version change for the maintainer.
- **R2 #42 rest**: the CV fallback that ignores `no_initial` and harmony runs only after 48
  failed draws and yields a valid-looking name; a head-first toponymic family name keeping only
  the place phonemes in `word` changes every such family name (golden languages and every
  world's NPC names), so it needs a names re-baseline.
- **R2 #43 rest** (`cap_cities` clamps a demoted city to 7,999 after land use and roads are
  sized): the goal-35 model's intent, not a defect; land use stays sized for the pre-cap town.

### Files split (pure moves, same test names and counts)

`1003a47` `hydrology/flow_metrics_tests.rs` 583 → 477 (+`flow_metrics_tests/limits.rs`, 13
tests); `bf84a69` `channels/tests.rs` 580 → 407 (+`tests/atlas.rs`, 14); `bd2272b`
`arda-settle/tests/society.rs` 591 → 450 (+`society/realms.rs`, 19); `bbd658e`
`arda/tests/area_exports.rs` 536 → 439 (+`area_exports/reload.rs`, 12); `08c098d`
`arda-ways/tests/ways.rs` 502 → 470 (+`ways/refusals.rs`, 17); and, for files the fixes grew,
`24aa6f0` (`tactical/admit.rs`), `34ce7c4` (`plan/relevant.rs`), crossing tests into
`plan/crossing_tests.rs`. Each split was checked by diffing the test lists before and after.

### Gate on `5c33283`

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 113 binaries report `test result: ok. N` with
  N ≥ 1 (1,573 passed, 0 failed, 21 ignored; integrate/v0.4: 100, 1,534, 19).
- `cargo test --release --test e2e_village -- --ignored`: passes (93 s);
  `--test zoom_continuity -- --ignored`: both pass (102 s); `--test golden_recipes`: 4 pass
  (it has no ignored tests; recipe 5, 6 and 7 replay byte for byte).
- `apps/viewer`: `npm test` (83 tests in 11 files), `npm run lint`, `npm run build` pass.
- Fixture: `arda generate --seed 42 --micro --terrain fine` (8 areas, land 293‰, 8 rivers),
  `arda settle` (629 settlements, 648 roads, 3 realms), `arda society build` (629 plans,
  91,896 inhabitants, 3,397 stored notables).

### Smoke test (release `arda-server --world out/micro42 --port 8963`, stopped by PID)

| Request | Result |
|---|---|
| `/v1/world` | 200, contract 3, recipe 6 |
| `/v1/cell/432/904` (village 10) | `"road":"road","built_by":"10","land_use":"built","realm_id":"1"` |
| `/v1/area/0/1/cells?format=bin` | 200, 27.0 MB, layout 2, contract 3 |
| `/v1/tiles/overview/2/1/2.webp` with its ETag in `If-None-Match` | 304 |
| `/v1/tiles/relief/7/55/95.webp` | 200, `no-cache`, ETag |
| `/v1/overview.png?quality=600` / `quality=1024` | 400 `bad_request` naming the ladder / 200 |
| `/v1/tactical/cell/432/904`, `/scene`, `.png?ppsq=32` | 200 (0.2 s cold), 4 tokens, PNG `no-cache` |
| `/v1/cell/99999/1` | 400 `out_of_range` |

Server log: no errors.

### Left open

- Not done by instruction: no merge, no push, no tag, no version bump; changelog fragments
  unfolded (`2026-10-01-v4-hardening.md` added).
- **A12**: ts-rs mirrors for `TacticalScene`/`Token`, settlement records, `TownPlan`.
- Review items skipped above: R2 #30, #38 d, #41, the rest of #42 and #43.
- From earlier sections, still standing: I1 generator side (`FINE_FRAME_OFFSET_UM` → 0 needs a
  re-baseline); shore classes step at close zoom; relief draws
  more small streams than the overview; `logic/13`/`logic/16` still call the composite
  reference the `NpcId`; a bare commoner u64 id needs `?settlement=`; realm-wide NPC queries
  cost ~0.15 s per scanned settlement cold; prefetch is explicit; the v0.3 WFC notes;
  `JOURNAL-tactical.md` (I34).
- `arda-fields/src/synthetic.rs` is 507 lines (+9 from the land-use bound).

## v0.4 schemas and sheet mapping (feat/v4-schemas)

Goal 69 preparation and adapter A12, on `integrate/v0.4`:

- **JSON Schemas** (logic/16 §api-schema): 30 draft 2020-12 schemas generated by
  `schemars` (MIT/Apache-2.0) from the server DTOs and, through a new optional `schema`
  feature, from `arda_settle::model::Settlement`, `arda_society::SettlementSociety` and
  `arda_town::TownPlan`. Committed in `bindings/schema/`, served at `GET /v1/schema` and
  `/v1/schema/{Name}.json`, staleness-tested with the TS bindings. `tests/world_api/schemas.rs`
  validates real MICRO seed-42 responses of every route with `jsonschema` (MIT, dev-only).
- **Typed scenes (A12)**: server mirror `SceneDto` (run-length `Runs<T>`, `SqDto`, string
  seed, walls, lights, regions, spawn hints, token slots), round-trip tested against
  `arda_scene::Scene`; `TacticalScene`, `Token`, `TokenKind`, `SceneTime` typed;
  `TacticalBlockDto.scene` is `SceneDto | null` instead of `unknown`. The viewer reads
  scenes through them (`apps/viewer/src/api/scene.ts`) and dropped its hand-written
  `TokenView`, `RulesCellView`, `RulesSidecarView` and `CoverLevel`.
- **Sheet mapping** (logic/16 §api-sheet-mapping): `--sheet-mapping game.json` reshapes
  every served `Npc`; checked and dry-run at startup; `X-Arda-Sheet-Mapping` on mapped
  bodies. Examples `identity.json` and `5e-srd-monster.json` in
  `crates/arda-server/mappings/`; maintainer guide in `API.md` §"Sheet mapping".

Still open:

- The maintainer's own game schema: write `game.json` from it (goal 69 stays 🟡 until then).
- No TS types for `Settlement`, `SettlementSociety` and `TownPlan` (they would need server
  mirrors or ts-rs in the domain crates); their schemas exist.
- The schemas describe Arda's NPC shape only; a mapping has no output schema, so a mapped
  server's `Npc` bodies are not described by `/v1/schema`.
- `Faction.seat_building` in arda-society is written as a JSON number, an exception to the
  "ids as strings" convention (I5); the schema records it faithfully.
- Map keys such as `Settlement.buildings` (`BuildingFunction` → count) are schema'd as
  free `additionalProperties`, without restricting keys to the enum names.

## v0.5 integration (integrate/v0.5)

`integrate/v0.5` starts at `c28e2bf` (integrate/v0.4, whose tree is main's 0.4.0, recipe 7 the
default). The three branches were cut at `7b1d1a0`, before the recipe-7 commit, and are merged
with `--no-ff` in this order, each followed by the full gate:

| Merge | Branch | Conflicts and resolution |
|---|---|---|
| `fa80cbf` | fix/v4-hardening | `API.md` (contract 3 with `arda_version` 0.4.0); this file (recipe 7 is the default, hardening's struck-out open items kept). |
| `0a751b7` | feat/v4-schemas | `TacticalCellView.tsx` (the typed scene fetch kept, wrapped in hardening's `whileLive`); `arda-server/Cargo.toml` (schema features); `lib.rs` and `world_api/main.rs` (both modules); this file (A11 done, A12 done for scenes). Contract: hardening's new `LandUseDto` gains `JsonSchema`; bindings and schemas re-blessed with `ARDA_BLESS_BINDINGS=1`: `CellSample`, `PointSample` and `AreaCells` now describe `land_use`, `realm_id` and `footpath` (the TS bindings were already contract 3). |
| `7a163b2` | feat/v4-art-import | `Cargo.toml` (the 0.4.0 path versions, `arda-art-import` added); `Cargo.lock` regenerated and pinned to the branch's tested versions (`toml` 0.8.2, `toml_edit` 0.20.2, `winnow` 0.5.40, `jpeg-decoder` 0.3.1, `indexmap` 2.14.0, …). |

No test of the three branches was pinned to recipe-6 default fixture values: every gate passed
with fixture worlds generated at the recipe-7 default, and e2e stays pinned to recipe 6.

### Commits after the merges

| Commit | Change |
|---|---|
| `f26915a` | `tests/support/fixture_dir.rs` holds an exclusive lock on `<name>.lock` while building. The first gate failed `racing_builders_publish_one_complete_fixture`: between a builder's `ready` and `exists` checks another could publish, and the late builder moved the complete fixture aside as stale. |
| `81acd79` | The server tests reuse `out/micro42` or the cached fixture only when it is a recipe-7 world, so the schemas are never checked on a stale recipe-6 world. New `contract_3_society_cells_validate_on_the_default_recipe`: the world is recipe 7 and contract 3, and a village cell (`built_by` = the village, `land_use`, `realm_id` set) validates against `CellSample`, as does its area against `AreaCells`. |
| `c6d57bf` | Library stacks against the footprint cap: `Library::layered` refuses to rescale a lower layer's asset past `MAX_RESCALED_SIDE_PX` (16 squares × 128 px = 2,048 px a side). A top layer at 1,024 px/square would otherwise blow the placeholder's 4 × 4 assets up to 4,096 px (each layer and the merge were validated, but the rescale ran before the merged validation). |
| `e6fdb90` | `import.toml` footprints of 0 or over 16 squares a side are refused when the manifest is read, before any image is fitted to `footprint × ppsq`. |
| `17482b0` | `arda-town block --library`, and a `--library` argument for the arda-scene `debug` and arda-ways `crossings` examples, take a `top:…:bottom` stack (`Library::load_stack`); both READMEs say so. |
| `17a2238` | CHANGELOG "Unreleased (v0.5)"; `API.md`'s `/v1/world` example shows recipe 7; the `FINE_TERRAIN_LATEST_RECIPE_VERSION` doc no longer calls recipe 7 opt-in. |

### Gate on `17a2238`

| Gate | After hardening | After schemas | After art import | Final |
|---|---|---|---|---|
| `cargo fmt --check`, clippy `-D warnings` | clean | clean | clean | clean |
| `cargo test --workspace --no-fail-fast`: binaries with `ok. N ≥ 1` | 112 + 1 failed (fixture race, fixed) | 113 | 115 | 115 |
| passed / failed / ignored | 1,572 / 1 / 21 | 1,600 / 0 / 21 | 1,633 / 0 / 21 | 1,635 / 0 / 21 |
| `e2e_village`, `zoom_continuity` (`--release --ignored`) | 1 + 2 pass | 1 + 2 pass | 1 + 2 pass | 1 + 2 pass |
| `golden_recipes` (5, 6, 7 byte for byte) | 4 pass | 4 pass | 4 pass | 4 pass |
| viewer `npm ci`, `build`, `lint`, `test` | 83 tests, 11 files | 86, 12 | (unchanged) | 86, 12 |
| `arda tactical validate assets/tactical/placeholder` | ok, 217 assets | ok | ok | ok |

### Smoke test

Release `arda-server` on a copy of the recipe-7 MICRO 42 society fixture, with
`--library <imported>:assets/tactical/placeholder` and
`--sheet-mapping crates/arda-server/mappings/5e-srd-monster.json`, port 8964, stopped by PID.
The imported library was made with the `synthetic_raw` example and `arda tactical import`
(11 imported, 0 rejected, 2 flagged); `arda tactical validate` passes on the stack (216 assets).

| Request | Result |
|---|---|
| `/v1/world` | 200, contract 3, `arda_version` 0.4.0, recipe 7 |
| `/v1/schema` / `/v1/schema/Npc.json` | 200, draft 2020-12, 30 schemas / `application/schema+json` |
| `/v1/schema/CellSample.json` | lists `land_use` and `realm_id` |
| `/v1/cell/564/778` (village 142) | `"road":"road","built_by":"142","land_use":"built","realm_id":"1"` |
| `/v1/area/1/1/cells?format=bin` | 200, 27.0 MB |
| `/v1/settlements/142/npcs`, `/v1/npc/<notable id>`, `/v1/npc/142.5.0` | 200, `X-Arda-Sheet-Mapping: 5e-srd-monster`, 5e SRD monster bodies with an `arda` block |
| `/v1/tactical/library` | `"library":"synthetic-ai:placeholder"` |
| `/v1/tactical/cell/564/778.png?ppsq=32`, `/scene` | 200 PNG; 4 tokens |
| `/v1/cell/99999/1` | 400 |

Server log: no errors. `arda-town block --site thornby` and both examples also rendered with
the same stack.

### Left open

- Not done by instruction: no version bump (still 0.4.0), no merge to main, no tag, no push.
  Changelog fragments stay unfolded (`2026-10-01-v5-integration.md` added; the art-import
  branch had none, so it describes that branch too).
- A mapped server's `Npc` bodies have no schema (the mapping has no output schema); the smoke
  test checks their shape by hand, and the schema tests run unmapped.
- No TS types for `Settlement`, `SettlementSociety` and `TownPlan` (schemas exist).
- `pixels_per_square` itself has no upper bound in the validator; the stack rescale and the
  footprint cap bound sprites, but one huge single library is still only bounded by its own
  image files.
- Earlier items still standing: the maintainer's own `game.json` (goal 69); review items R2
  #30, #38 d, #41, the rest of #42 and #43; I1 generator side; the other items listed under
  v0.4 hardening.

## v0.6 integration (integrate/v0.6)

`integrate/v0.6` starts at `c63a4b1` (main, release 0.5.0, recipe 7 the default). The finished
branches are merged with `--no-ff`, each followed by the full gate:

| Merge | Branch | Conflicts and resolution |
|---|---|---|
| `c59eefa` | feat/v6-plains | None. `plains_metrics` example, opt-in recipe 8 (alluvial valley floors scaled to discharge), golden for recipe 8; the default stays 7. |
| `c3b0fed` | feat/v6-field-patterns (contains feat/v6-midzoom-landuse) | None. The plains side touched only formation, recipe and golden files, the fields side only arda-fields, arda-blocks, arda-midzoom and logic/17; `CHANGELOG.md`, `Cargo.lock` and the logic docs merged cleanly. |

### Commits after the merges

| Commit | Change |
|---|---|
| `20a4512` | Wedge-shaped fields. A cut other than a road may not meet its piece's boundary at under 38°, nor run within 12 squares of a boundary edge at under 38° (a sliver). The check reads the clipped outline, the block outline and the true lines of the cuts above, since a road's piece is clipped along a straight stand-in and its cut runs on past the road's ends as rays. The last-resort cuts obey it too (square to the frontage, then along it); a piece no cut can split stays one field, so the would-be wedge stays part of its neighbour. The shape statistics now measure corner angles, and `no_field_corner_is_a_wedge` holds every field corner in the five synthetic scenarios to 35° or more, except within 10 squares of a road or river. Without the rule it fails, on village_strips (8.2°) and quarry (19.2°). logic/17 §land-fields records the rule. |

**Wedge measurements on MICRO 42** (partition rasterised square by square over six 4 km
squares, corners under 35° more than 10 squares from a road or river): 98 before, 4 after,
with 8,321 fields before and 7,954 after (−4 %; the refused wedges stay with their neighbours).
Four-sided share 0.85–0.89 before, 0.87–0.91 after; T-junction share 0.99 both. The 4 left
are listed as open below.

**Renders** (`out/v6-int/`, `/v1/tactical/window.png`, 3 × 3 cells at 16 px/square):

- `micro42-tac-farm-3x3-{before,after}.png` and `-cmp.jpg`, at cells 580, 695 (the view from
  feat/v6-field-patterns): **unchanged, pixel for pixel.** Measured on the partition, this
  view has no corner under 35°; its triangles have corners of 41–48°, which a 35° limit keeps.
  They come from the last-resort cuts square to and along a diagonal road's frontage, which
  meet the block's sides at about 45°.
- `micro42-tac-farm-3x3-wedge-{before,after}.png` and `-cmp.jpg`, at cells 551, 636: a view
  with real wedges before (a long meadow sliver, field wedges at the left and a meadow wedge at
  the bottom); after, the fields are four-sided.

**Stair-stepped hedges: documented and skipped.** Hedges and field walls are `WallSegment`s on
square edges. The sidecar's SRD edge rules and the scene's walls come from the same segments.
The compositor draws each segment as a kit sprite turned by quarter turns (`Sprites` caches
`(id, turns, mirror)`, and `Rgba::rotated` turns by quarters only). Drawing them along the true
diagonal would need three things: arbitrary-angle, deterministic sprite resampling in
`arda-tactical`; either a new layout field carrying each boundary's polyline (a schema, binding
and golden change) or compositor-side inference of staircase runs from the square edges; and
smoothing the per-square hedge-verge ground, which stair-steps too. That is too invasive for this
integration. The rules would stay on square edges either way.

### Gate on `20a4512`

| Gate | After plains | After field patterns | Final (wedge fix) |
|---|---|---|---|
| `cargo fmt --check`, clippy `-D warnings` | clean | clean | clean |
| `cargo test --workspace --no-fail-fast`: binaries with `ok. N ≥ 1` | 115 | 117 | 117 |
| passed / failed / ignored | 1,642 / 0 / 21 | 1,664 / 0 / 23 | 1,665 / 0 / 23 |
| `golden_recipes` (5, 6 byte for byte; 7, 8 pinned) | 5 pass | 5 pass | 5 pass |
| `e2e_village`, `zoom_continuity` (`--release --ignored`) | 1 + 2 pass | 1 + 4 pass | 1 + 4 pass |
| viewer `npm ci`, `build`, `lint`, `test` | 86 tests, 12 files | 86, 12 | 86, 12 |
| `arda tactical validate assets/tactical/placeholder` | ok, 217 assets | ok | ok |

The gate after field patterns ran its workspace tests on the merge commit; one binary
(`arda-fields` `partition`) was rebuilt with the wedge fix while that run was in progress. The
merge-only e2e, validate and viewer gates ran on a stash of the fix.

### Smoke test

Release `arda-server` (at `20a4512`) on a copy of the MICRO 42 society world that the
field-pattern renders used, port 8990, stopped by PID:

| Request | Result |
|---|---|
| `/v1/world` | 200, contract 3, `arda_version` 0.5.0 |
| `/v1/schema` | 200 |
| `/v1/cell/581/696` | 200 |
| `/v1/tiles/relief/12/900/1800.webp`, `/11/455/1085.webp` | 200, 38 KB / 20 KB WebP |
| `/v1/tactical/cell/581/696.png?ppsq=32`, `/scene` | 200, 7.6 MB PNG; 200 scene |
| `/v1/tactical/window.png?gx0=551&gy0=636&w=3&h=3&ppsq=16` | 200, 17.6 MB PNG |
| `/v1/settlements?tier=village` | 200 |
| `/v1/cell/99999/1` | 400 |

Server log: no errors.

### Left open

- Not done by instruction: no version bump (still 0.5.0), no merge to main, no tag, no push.
  Changelog fragments stay unfolded (`2026-10-02-v6-integration.md` added). Release notes are in
  `CHANGELOG.md` under "Unreleased (v0.6)".
- Stair-stepped hedges and field walls on the tactical map (see above).
- The triangles of the cells 580, 695 view (41–48° corners) remain. A higher limit for the
  last-resort cuts (about 50°) would remove them, at the cost of larger fields beside diagonal
  roads.
- Four corners under 35° remain in the six MICRO 42 squares, at 19–30°. They are not yet
  explained: the cut that makes the 19° one passes the check at 65°, next to a bend in the block
  outline.
