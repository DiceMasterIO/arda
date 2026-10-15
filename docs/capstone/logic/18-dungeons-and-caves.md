---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: b0a36e2c603a
paths_covered:
  - ":(top)crates/arda-dungeon/**"
  - ":(top)crates/arda-cli/src/dungeon.rs"
  - ":(top)crates/arda-server/src/tactical/dungeon.rs"
  - ":(top)crates/arda-server/src/tactical/dungeon_dto.rs"
  - ":(top)crates/arda-server/src/tactical/mod.rs"
  - ":(top)crates/arda-server/src/tactical/routes.rs"
  - ":(top)crates/arda-server/src/tactical/world_routes.rs"
  - ":(top)crates/arda-server/src/error.rs"
  - ":(top)crates/arda-server/tests/world_api/dungeon.rs"
  - ":(top)crates/arda-server/src/schema/**"
---

# 18 — Dungeons and caves: seeded underground battle maps

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. Owns the `arda dungeon` subcommand (`crates/arda-cli/src/main.rs:153-155`, `crates/arda-cli/src/dungeon.rs:82`), the library call `arda_dungeon::generate` (`crates/arda-dungeon/src/lib.rs:229`) and the routes `GET /v1/tactical/dungeon`, `/v1/tactical/dungeon.png` and `/v1/tactical/dungeon/scene` (`crates/arda-server/src/tactical/routes.rs:61-63`). The compositor that paints a level is owned by [11](11-tactical-art-compositor.md), the scene builder by [12](12-scene-data.md), the server conventions by [16](16-service-api.md).

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/18 §<rule>`.

## Trigger & preconditions

| Entry point | Inputs (defaults) | Site |
|---|---|---|
| `arda dungeon` | `--seed` (1), `--kind dungeon\|cave` (dungeon), `--size WxH` (`48x36`), `--out` (`out/dungeon/layout.json`), optional `--rules`, `--meta`, `--scene`, `--render` paths, `--library` (`assets/tactical/placeholder`), `--ppsq` (128), `--grid` | `crates/arda-cli/src/dungeon.rs:18-54` |
| `GET /v1/tactical/dungeon` | `seed`, or `gx` and `gy`; `kind` (`dungeon`), `w` (48), `h` (36) | `crates/arda-server/src/tactical/dungeon.rs:28-70`, `:78` |
| `GET /v1/tactical/dungeon/scene` | as above | `crates/arda-server/src/tactical/dungeon.rs:91` |
| `GET /v1/tactical/dungeon.png` | as above, plus `ppsq` (128) and `grid` (`0`) | `crates/arda-server/src/tactical/dungeon.rs:110-137` |
| `arda_dungeon::generate(&Params)` | `{seed, kind, width, height}` | `crates/arda-dungeon/src/lib.rs:87-99`, `:229` |

- Authority: none. The CLI is a local process; the server routes carry no authentication and share the server's loopback binding and CORS (16 §api-conventions).
- Preconditions: both sides lie in `MIN_SIDE..=MAX_SIDE` = 16..=160 squares (`crates/arda-dungeon/src/lib.rs:40-42`, `:106-115`). The generator reads no world data: a level is a pure function of `Params` (`crates/arda-dungeon/src/lib.rs:20-21`). The server form `?gx=&gy=` requires the cell to lie inside the served world (`crates/arda-server/src/tactical/world_routes.rs:108-119`). `--scene` and `--render` require a loadable library or `top:…:bottom` stack (`crates/arda-cli/src/dungeon.rs:117-118`); the server uses the library it validated at startup.

## Rules

### §dgn-params

- Size parsing (CLI): `WxH` split on `x` or `X`, each side a `u32`; anything else is an error naming the text (`crates/arda-cli/src/dungeon.rs:56-68`). The bounds check is `Params::check` inside `generate` (`crates/arda-dungeon/src/lib.rs:230`).
- Kind: CLI `ValueEnum` `dungeon` or `cave` (`crates/arda-cli/src/dungeon.rs:10-16`); server `Kind::parse`, exactly `dungeon` or `cave` (`crates/arda-dungeon/src/lib.rs:66-74`).
- Server seed selection (`crates/arda-server/src/tactical/dungeon.rs:40-60`): `seed` alone → that `u64`; `gx` and `gy` without `seed` → `site_seed(world_seed, gx, gy, salt)` with salt 0 for `dungeon` and 1 for `cave`; every other combination (none, `seed` with a cell, one coordinate) → `400 bad_request` "give either seed, or gx and gy (a world cell)". The server runs `Params::check` before generating (`:67-68`).
- Query keys other than `kind`, `w`, `h`, `seed`, `gx`, `gy` (and `ppsq`, `grid` on the PNG route) are ignored: `level` reads named keys only (`crates/arda-server/src/tactical/dungeon.rs:32-70`).

### §dgn-site-seed

`site_seed(world_seed, gx, gy, salt) = hash2(hash_str(world_seed ^ salt, "arda-dungeon site"), gx, gy)` (`crates/arda-dungeon/src/lib.rs:241-247`). A world cell therefore always opens onto the same level per kind. Where a world places dungeon entrances is not implemented: no caller chooses sites (`crates/arda-dungeon/README.md` "World tie-in" states it as a follow-up; no other caller of `site_seed` than `crates/arda-server/src/tactical/dungeon.rs:48`).

### §dgn-rng

All draws come from a SplitMix64 stream seeded `mix(seed ^ 0xD06E0B)` (`crates/arda-dungeon/src/rng.rs:9-16`); built levels seed it with `seed ^ 0xB0117` (`crates/arda-dungeon/src/built.rs:258`), caves with `seed ^ 0xCA7E` (`crates/arda-dungeon/src/cave.rs:280`). `range(lo, hi)` is uniform in `lo..=hi` and returns `lo` when `hi <= lo` (`crates/arda-dungeon/src/rng.rs:28-35`); shuffles are Fisher-Yates (`:43-48`). Cave noise uses `fbm(seed ^ 0x57A1 ^ k, …, 3 octaves)` (`crates/arda-dungeon/src/cave.rs:295-298`).

### §dgn-zones

Every square belongs to a zone: `ROCK` 0, `CORRIDOR` 1, rooms, prison cells and the cavern from 2 (`crates/arda-dungeon/src/grid.rs:16-21`); squares outside the map read as rock (`:57-62`). A wall stands on every unit edge between two different zones, except edges in `open` (cave mouth) and openings: rock faces are always `Run`; a floor/floor edge is an arch (no wall), a door or gate from `openings`, or a `Run` (`crates/arda-dungeon/src/grid.rs:110-142`). Corridors share one zone, so crossing corridors merge without walls (`crates/arda-dungeon/src/grid.rs:7-9`). Every cave wall is tagged `natural` (`crates/arda-dungeon/src/cave.rs`, after the walls): the compositor draws the rock faces as smoothed outlines instead of square steps (11 §natural-walls, OI-38); the edges, the sidecar and the scene walls stay on the square grid. Built dungeon walls carry no such tag.

### §dgn-rooms

Built levels (`crates/arda-dungeon/src/built.rs`):

- Binary space partition of the interior `(1, 1, w−2, h−2)` (`:261-270`). A rectangle splits along an axis only if that side is ≥ 2 × `MIN_LEAF` (7); it stops when neither axis can split, or with probability 0.2 once both sides are ≤ `MAX_ROOM` + 4 = 16. A side more than 5/4 the other is split; otherwise a coin decides. The cut position is uniform in `MIN_LEAF..=side−MIN_LEAF` (`:14-17`, `:57-97`).
- One room per leaf with both sides ≥ 5: each side uniform in `max(3, ⌊3m/5⌋)..=m` with `m = min(leaf side − 2, 12)`, offset so one rock square remains on every leaf side (`:100-113`).
- Fewer than two rooms: two fixed rooms side by side, `rw = max(⌊(w−4)/2⌋, 3)` (`:275-292`).

### §dgn-corridors

- Prim's minimum spanning tree over room centres (`centre = (x + ⌊w/2⌋, y + ⌊h/2⌋)`) by Manhattan distance, from room 0, ties to the first pair found (`crates/arda-dungeon/src/built.rs:119-144`, `crates/arda-dungeon/src/dress.rs:44-46`).
- Loops: candidate non-tree pairs with distance ≤ `longest + ⌊longest/3⌋` (longest tree edge), sorted ascending; `loops = max(⌊n/4⌋, 1)`; the first `2 × loops` candidates are each added with probability 0.6 while fewer than `loops` loop links exist (`crates/arda-dungeon/src/built.rs:306-323`).
- Every link is an L-shaped 1-square path between centres, horizontal or vertical leg first by a coin; rock squares on a path become `CORRIDOR` (`:146-171`, `:325-335`).
- An opening is every path step where the zone changes; it records the rooms it enters and whether only loop paths use it (`:337-359`).

### §dgn-purposes

- Entrance: the room whose centre is nearest one of the four map corners, chosen at random; ties to the lower index (`crates/arda-dungeon/src/built.rs:363-366`).
- Stairs-down room: most graph hops from the entrance over all links, then greatest centre distance, then lowest index (`:367-371`).
- Treasure: the smallest-area dead end (degree 1) that is neither entrance nor stairs-down room; with no such dead end and at least 4 rooms, the smallest other room; otherwise no treasure room (`:372-382`).
- Every other room, the stairs-down room included, takes the next purpose from a shuffled cycle of `Crypt, Barracks, Storage, Shrine, Prison, Hall` (`:383-404`).

### §dgn-openings

Per opening, in edge-key order (`crates/arda-dungeon/src/built.rs:405-425`):

| Condition | Result |
|---|---|
| enters the treasure room | door tagged `locked` |
| used only by loop paths, then p = 0.4 | door tagged `secret` |
| else p = 0.2 | arch (no wall) |
| else p = 0.15 | door tagged `locked` |
| else | plain door |

An arch with another opening on the adjacent edge of the same axis becomes a plain door, so no door has a free end (`:426-446`). The level description's `doors` lists every `Door` and `Gate` wall with its `locked` and `secret` tags (`:586-598`).

### §dgn-prison

A prison room of at least 4 × 5 takes a row of cells along its north rows, else its south rows, provided no opening square lies in those two rows: `⌊w/2⌋` cells 2 squares wide (the last takes the remainder) and 2 deep, each its own zone behind a `Gate` tagged `locked` on the cell's first column (`crates/arda-dungeon/src/built.rs:198-254`, `:447-471`). A prison room with no free row becomes `Storage` (`:469`). Each cell gets a cot on a wall (8 tries) and a bucket (6 tries); the square outside its gate is kept clear (`:556-561`).

### §dgn-caverns

Caves (`crates/arda-dungeon/src/cave.rs`):

- Seed: every interior square is floor with probability 0.54 (`FILL` = 0.46 rock); the border is rock (`:28-40`).
- Five passes of the 4-5 rule: a square becomes rock when ≥ 5 of its 3 × 3 block are rock (outside the map counts as rock), the border always rock (`:41-57`).
- Regions are labelled by orthogonal flood fill; the largest (ties to the first) is the cave. A cave under a quarter of the interior rejects the draw (`:58-77`). Up to 16 draws are made from the continuing stream; none usable is `DungeonError::Generation` "no usable cavern" (`:282-289`).
- Regions under 6 squares are filled with rock; every larger region is joined from a random square of it to the nearest already-joined square by a winding tunnel stepping in x or y by coin, carving a 3-square brush that never touches the border, at most `4 × (w + h)` steps (`:78-120`).
- The level has one room, `Cavern`, the bounding box of all floor (`:489-509`), and no doors (`:510`).

### §dgn-mouth

A random map side; among floor squares within 2 of the nearest-to-that-side floor distance and at least 2 squares from the corners, a random start (else the first floor square). A straight 2-wide tunnel runs to the side; its border edges join `open`, so they carry no wall, and the border square is the `Mouth` exit (`crates/arda-dungeon/src/cave.rs:182-245`, `:396-400`).

### §dgn-water

- Pools: floor squares ≥ 6 steps from the mouth, ≥ 2 from rock, with noise(scale 7) > 0.67 are 2 ft deep; above 0.73 and ≥ 3 from rock, 6 ft (`crates/arda-dungeon/src/cave.rs:299-310`).
- Stream: with probability 0.8 when the cave has more than 60 floor squares, the shortest floor path between the two far ends (a double breadth-first sweep from a random floor square) gets 1 ft, or 2 ft with probability 0.3, on dry squares ≥ 3 steps from the mouth (`:311-334`).

### §dgn-ground

| Level | Square | Ground | Site |
|---|---|---|---|
| any | rock | `bedrock` (`ROCK_GROUND`), the layout's default | `crates/arda-dungeon/src/lib.rs:49`, `crates/arda-dungeon/src/built.rs:473`, `crates/arda-dungeon/src/cave.rs:343` |
| dungeon | room | `flagstone` | `crates/arda-dungeon/src/built.rs:19`, `:476-480` |
| dungeon | corridor, prison cell | `stone_floor` | `crates/arda-dungeon/src/built.rs:20`, `:478-479` |
| cave | water | `gravel` | `crates/arda-dungeon/src/cave.rs:352-353` |
| cave | beside water | `mud` (noise scale 3 > 0.5) or `gravel` | `crates/arda-dungeon/src/cave.rs:354-359` |
| cave | ≤ 2 from rock, noise scale 6 > 0.66 | `scree`, difficult | `crates/arda-dungeon/src/cave.rs:360-362` |
| cave | noise scale 5 > 0.72 | `moss` | `crates/arda-dungeon/src/cave.rs:363-364` |
| cave | otherwise | `cave_floor` | `crates/arda-dungeon/src/cave.rs:366` |

Every square has elevation 0 (`crates/arda-dungeon/src/built.rs:483-488`, `crates/arda-dungeon/src/cave.rs:368-375`). Wall kits: `stone` for dungeons, `cave` for caves (`crates/arda-dungeon/src/lib.rs:44-47`, `crates/arda-dungeon/src/built.rs:491`, `crates/arda-dungeon/src/cave.rs:377`).

### §dgn-dress

- `Dresser::place` accepts a footprint only when every covered square is in the region and not occupied, kept or wet; a blocking item is accepted only if the region's unblocked squares stay one orthogonally connected piece (`crates/arda-dungeon/src/dress.rs:136-188`, `:191-209`). Whether an item blocks is fixed per item (`blocks`, "the connectivity guard assumes the worst", `:17-19`). A placement's position is the footprint centre; rotation turns the asset's back to its wall (`:60-71`, `:175-186`).
- Kept squares (never dressed): both sides of every dungeon opening (`crates/arda-dungeon/src/built.rs:493-506`), every stair foot (`:527`, `crates/arda-dungeon/src/cave.rs:418`), cave squares within 2 steps of the mouth (`crates/arda-dungeon/src/cave.rs:381-385`). Water squares are never dressed (`crates/arda-dungeon/src/cave.rs:380`).
- Stairs (dungeon): `prop.stairs` (up) in the entrance room, `prop.stairs_down` in the stairs-down room, 1 × 2 against a wall, sides in shuffled order with up to 12 slot draws per side; the foot square must be in the room and unused, and becomes the exit square. No slot is `DungeonError::Generation` "no wall in room … takes the …" (`crates/arda-dungeon/src/built.rs:507-544`, `:622-645`).
- Descent (cave): floor squares sorted by descending mouth distance (ties by row, then column); the first whose stair back lies on rock and foot on unused floor takes `prop.stairs_down`. None is `DungeonError::Generation` "no slot for the descent" (`crates/arda-dungeon/src/cave.rs:387-430`).
- Room dressing by purpose, with fixed counts and try budgets (`crates/arda-dungeon/src/built.rs:682-800`); 1 torch sconce per room, 2 when the room area exceeds 30, 10 tries each (`:550-555`, `:647-660`); per corridor square a roll < 0.035 places a sconce facing an adjacent rock face, < 0.06 a rubble pile, < 0.07 bones (`:562-584`).
- Cave scatter counts from `fc` floor squares: stalagmites `fc/45 + 1` and large rocks `fc/260` on open floor (≥ 2 from rock); boulders and small rocks `fc/90`, stones `fc/120` by the wall; rubble `fc/70 + 1` anywhere dry; mushroom rings `fc/80 + 1` on moss or beside water; 1–3 bone piles; a skeleton with p = 0.5 and a treasure chest with p = 0.4 by the wall; each with 4 × count tries (`crates/arda-dungeon/src/cave.rs:431-476`). Every even-indexed placed mushroom ring adds a light of radius 10 ft, colour `[120, 220, 200]`; caves have no other light (`:478-488`).

### §dgn-rules-sidecar

The sidecar has `format_version` `SIDECAR_FORMAT_VERSION` (2), the map size, one cell per square and no edge rules: rock squares `blocks_movement` and `blocks_sight`; difficult squares `difficult` (`crates/arda-dungeon/src/grid.rs:144-168`, `crates/arda-scene/src/sidecar.rs:32`). Only cave scree is difficult; built levels pass an all-false mask (`crates/arda-dungeon/src/built.rs:611`, `crates/arda-dungeon/src/cave.rs:344`, `:361`).

### §dgn-serve

- `/dungeon`: the whole `Dungeon` (`params`, `layout`, `rules`, `rooms`, `doors`, `exits`) as JSON with `Cache-Control: no-cache`; no `ETag` and no cache lookup (`crates/arda-server/src/tactical/dungeon.rs:78-88`, `crates/arda-server/src/tactical/routes.rs:30-32`). `params.seed` is serialised as a decimal string and read from a string or a number (`#[serde(with = "arda_scene::serde_str")]` on `Params::seed`, `crates/arda-dungeon/src/lib.rs`), per 16 §api-conventions; the CLI's `--meta` file carries the same `params`. The body is described by the wire mirror `DungeonDto` (TS binding and JSON Schema `DungeonDto`, `crates/arda-server/src/tactical/dungeon_dto.rs`), whose test round-trips dungeons and caves through it; the schema catalogue lists `/dungeon` under `DungeonDto` and `/dungeon/scene` under `SceneDto` (`crates/arda-server/src/schema/mod.rs`).
- `/dungeon/scene`: `arda_scene::build_scene(layout, served library, world seed, Some(rules))` as JSON, with no `Cache-Control` header and no `ETag` (`crates/arda-server/src/tactical/dungeon.rs:91-107`).
- `/dungeon.png`: `ppsq` in `PPSQ_OPTIONS` = {64, 96, 128}, default 128; `grid` `0`/absent or `1` (`crates/arda-server/src/tactical/dungeon.rs:117-137`, `crates/arda-server/src/tactical/mod.rs:53`). The layout passes `admit` (≤ `max_squares` = 204 × 204; render ≤ `max_render_px` = 96 Mi px; `admit_work`; `TacticalLayout::check` against the library) (`crates/arda-server/src/tactical/mod.rs:303-322`, `:82-86`), is keyed by `RenderKey {world_seed, anchor = layout name, BLAKE3(layout JSON), ppsq, grid, library_version}` (`:344-364`) and rendered or served from the PNG cache (`:370-387`). The response carries `ETag`, `Cache-Control: no-cache`, `X-Arda-Cache` and `Server-Timing`, and answers `304` to a matching `If-None-Match` (`crates/arda-server/src/tactical/routes.rs:147-183`).
- At the defaults a 160 × 160 level exceeds `max_render_px` at every allowed `ppsq` (160² × 64² = 104,857,600 > 100,663,296), so its PNG is refused with `413` (`crates/arda-server/src/tactical/mod.rs:311-317`).
- Every request logs one line `arda-server tactical <route> <detail> cache=<hit|miss> bytes=<n> <ms> ms` to stderr (`crates/arda-server/src/tactical/routes.rs:137-145`).

### §dgn-cli-outputs

`arda dungeon` writes, in order (`crates/arda-cli/src/dungeon.rs:82-134`): the layout JSON at `--out`, then the rules sidecar at `--rules`, then `{params, rooms, doors, exits}` at `--meta`; it returns there unless `--scene` or `--render` is given. It then loads the library, writes the scene built with the level seed as render seed (`:119-122`) and renders the PNG with lighting on, the level seed and the default style (`:123-133`). Parent directories are created; each file is written with `std::fs::write`, followed by a newline (`:70-80`).

## Steps

1. Parse inputs (§dgn-params); on the server, derive the seed (§dgn-site-seed) and check the size.
2. `generate`: check the size, then branch by kind (`crates/arda-dungeon/src/lib.rs:229-235`).
3. Dungeon: rooms (§dgn-rooms) → zone grid → spanning tree, loops and corridors (§dgn-corridors) → purposes (§dgn-purposes) → openings (§dgn-openings) → prison cells (§dgn-prison) → squares (§dgn-ground) → walls (§dgn-zones) → stairs, room, cell and corridor dressing (§dgn-dress) → door list, room list, sidecar (§dgn-rules-sidecar) (`crates/arda-dungeon/src/built.rs:257-620`).
4. Cave: caverns (§dgn-caverns) → mouth (§dgn-mouth) → water (§dgn-water) → ground (§dgn-ground) → walls with the mouth open → descent and scatter (§dgn-dress) → lights → cavern room, sidecar (`crates/arda-dungeon/src/cave.rs:278-513`).
5. Emit: CLI files (§dgn-cli-outputs) or the route's body (§dgn-serve).

## Branches

- Kind: `Dungeon` → `built::generate`; `Cave` → `cave::generate` (`crates/arda-dungeon/src/lib.rs:231-234`).
- Seed source on the server: explicit `seed` or world cell (§dgn-params).
- Fewer than two BSP rooms → the two-room fallback (`crates/arda-dungeon/src/built.rs:275-292`).
- No dead end and fewer than 4 rooms → no treasure room, so no forced `locked` door (`crates/arda-dungeon/src/built.rs:372-382`).
- Prison without a free row → `Storage` (`crates/arda-dungeon/src/built.rs:469`).
- Cave draw too small → redraw, up to 16 (`crates/arda-dungeon/src/cave.rs:282-287`); no stream when the stream roll fails or the cave has ≤ 60 floor squares (`:311`).
- CLI without `--scene`/`--render` → no library load (`crates/arda-cli/src/dungeon.rs:114-116`).

## Unhappy paths

| Failure | CLI | Server | Site |
|---|---|---|---|
| malformed size | error "size `…` is not WxH" or a number error, nonzero exit | `w`/`h` not a `u32` → `400 bad_request` | `crates/arda-cli/src/dungeon.rs:56-68`, `crates/arda-server/src/tactical/dungeon.rs:37-39` |
| side outside 16..=160 | `DungeonError::Params`, nonzero exit | `400 bad_request` | `crates/arda-dungeon/src/lib.rs:106-115`, `crates/arda-server/src/tactical/dungeon.rs:67-68` |
| unknown kind | clap rejects | `400 bad_request` | `crates/arda-cli/src/dungeon.rs:24-25`, `crates/arda-server/src/tactical/dungeon.rs:33-36` |
| no seed / seed and cell / one coordinate | n/a (seed defaults to 1) | `400 bad_request` | `crates/arda-server/src/tactical/dungeon.rs:55-59` |
| cell outside the world | n/a | `400 out_of_range` | `crates/arda-server/src/tactical/world_routes.rs:108-119`, `crates/arda-server/src/error.rs:124-126` |
| bad `ppsq` or `grid` | n/a (`--ppsq` any `u32`) | `400 bad_request` | `crates/arda-server/src/tactical/dungeon.rs:117-137` |
| generator cannot meet its own invariants (no cavern in 16 draws, no stair slot, no descent slot) | `DungeonError::Generation`, nonzero exit | `500 internal` | `crates/arda-dungeon/src/cave.rs:288-289`, `:428-430`, `crates/arda-dungeon/src/built.rs:538-543`, `crates/arda-server/src/tactical/dungeon.rs:72-75` |
| render too large / layout invalid for the library | render error, nonzero exit | `413 payload_too_large` / `422 invalid_layout` | `crates/arda-server/src/tactical/mod.rs:303-322` |
| scene build failure | error, nonzero exit | `500 internal` "scene: …" | `crates/arda-cli/src/dungeon.rs:120`, `crates/arda-server/src/tactical/dungeon.rs:96-102` |
| library unreadable | error "loading …", nonzero exit | server refuses to start (16) | `crates/arda-cli/src/dungeon.rs:117-118` |
| write failure midway | earlier files stay written; no rollback, no temp-file rename | n/a | `crates/arda-cli/src/dungeon.rs:77-80`, `:94-133` |

- Retry: not implemented beyond the 16 cavern draws; a `Generation` error is final for that seed and size (`crates/arda-dungeon/src/cave.rs:282`).
- Concurrency: generation is pure and holds no state; concurrent server requests for the same level each regenerate it (`crates/arda-server/src/tactical/dungeon.rs:72-75`); only the PNG goes through the render cache (§dgn-serve).
- Repetition: running the CLI twice overwrites the same files (`std::fs::write`, `crates/arda-cli/src/dungeon.rs:79`).
- Generation success is asserted by the tests for 48 levels only (2 kinds × sizes 16×16, 32×24, 48×36, 72×56 × 6 seeds, `crates/arda-dungeon/tests/levels.rs:34-47`); other seeds and sizes have no tested guarantee against `Generation` errors.

## State transitions

None in the generator: no entity has a lifecycle and nothing is stored (`crates/arda-dungeon/src/lib.rs:20-21`). The CLI goes from nothing to files on disk, layout first (§dgn-cli-outputs). The server's only state change is a PNG cache insertion keyed by `RenderKey` (`crates/arda-server/src/tactical/mod.rs:385`).

## Invariants

Asserted by `crates/arda-dungeon/tests/levels.rs` over its 48-level sweep and by `crates/arda-server/tests/world_api/dungeon.rs`:

1. Determinism: the same `Params` give byte-identical JSON; seeds 42 and 43 differ (`crates/arda-dungeon/tests/levels.rs:49-58`). The same server query gives identical bodies; a world cell gives the same cave and its neighbour a different one (`crates/arda-server/tests/world_api/dungeon.rs:8-48`). Render hashes are pinned in `tests/golden/renders.txt` (`crates/arda-dungeon/tests/levels.rs:210`).
2. Every layout passes `TacticalLayout::check` against the placeholder library; sidecar and layout sizes agree (`crates/arda-dungeon/tests/levels.rs:76-83`).
3. Reachability: from the first exit, every non-impassable square is reachable with all doors, gates and secret doors opened (`check::unreachable`), and at least 80 % of floor squares are walkable (`crates/arda-dungeon/tests/levels.rs:86-108`, `crates/arda-dungeon/src/check.rs:16-63`).
4. No floating door: every door or gate lies between two floor squares and both its ends touch another wall segment (`crates/arda-dungeon/tests/levels.rs:111-116`, `crates/arda-dungeon/src/check.rs:79-108`).
5. Exits: every level has `StairsDown` with `prop.stairs_down` placed; dungeons `StairsUp` with `prop.stairs`; caves a `Mouth`; every exit square is floor (`crates/arda-dungeon/tests/levels.rs:119-140`).
6. Scene rules: rock squares are impassable and heavily obscured; across the sweep locked and secret doors, wading and swimming water and difficult terrain all occur; a dungeon's `doors` count equals the scene's opening walls (`crates/arda-dungeon/tests/levels.rs:144-184`).
7. Out-of-range sizes are refused (`crates/arda-dungeon/tests/levels.rs:202-205`); bad server queries answer 400 (`crates/arda-server/tests/world_api/dungeon.rs:50-62`).
8. Every placed id exists in the committed placeholder library (`crates/arda-dungeon/README.md` "Vocabulary"; enforced by invariant 2).

## Outcomes & side effects

- CLI success: one summary line `<name>: WxH squares, N rooms, N doors, N props -> <out>`, plus `rendered <png> (WxH px)` with `--render`; files as §dgn-cli-outputs; exit 0 (`crates/arda-cli/src/dungeon.rs:95-104`, `:132`). Layout names are `dungeon_<seed>` or `cave_<seed>` (`crates/arda-dungeon/src/built.rs:473`, `crates/arda-dungeon/src/cave.rs:343`).
- Server success: JSON or PNG bodies per §dgn-serve and one stderr log line per request.
- Failure: CLI prints the anyhow chain and exits nonzero; the server answers `{"error": {code, status, message}}` (16 §api-errors).
- Render seed differs by entry point: the CLI renders and builds scenes with the level seed (`crates/arda-cli/src/dungeon.rs:120`, `:129`), the server with the world seed (`crates/arda-server/src/tactical/dungeon.rs:96-100`; `RenderKey.world_seed`, `crates/arda-server/src/tactical/mod.rs:355`), so the same level can paint differently in texture variants.
- Records: nothing is persisted by the server; the CLI files are the only record and carry their `params` in `--meta` (`crates/arda-cli/src/dungeon.rs:108-113`).

## Dimensions not in play

- D1 Authority: one local actor (CLI) or an unauthenticated loopback client (server); no roles (`crates/arda-server/src/main.rs:23-25`).
- D5 Money: nothing is charged or credited.
- D7 Time: no deadlines, expiry or clock input; generation reads no clock (`crates/arda-dungeon/src/lib.rs:20-21`). The server's `Instant` timing affects only the log line and `Server-Timing` (`crates/arda-server/src/tactical/dungeon.rs:79-82`).
- D9 Lifecycle: no entity changes state (State transitions).
- D11 Termination: a single synchronous step per call; an abandoned CLI run leaves whichever files were already written (Unhappy paths).
- D12 Visibility: the full level is returned, secret doors included and marked `secret` (`crates/arda-dungeon/src/lib.rs:160-172`); nothing is hidden from the caller. Hiding secrets from players is not implemented here.
- D13 Notification: nobody is notified; stdout/stderr only.
- D14 Effects on others: generation touches no world, society or other client state; the PNG cache is shared and bounded (16 §api-cache).
- D15 Record and audit: no journal; reproducibility comes from `params` (seed, kind, size) alone.
