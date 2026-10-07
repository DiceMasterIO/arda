# arda-dungeon

Deterministic dungeon and cave battle maps. `generate` turns a seed, a kind
and a size into an ordinary `arda_tactical::TacticalLayout` plus a rules
sidecar, so the compositor paints it and `arda-scene` derives movement,
cover and sight with no special cases.

```rust
use arda_dungeon::{generate, Kind, Params};
let d = generate(&Params { seed: 7, kind: Kind::Dungeon, width: 48, height: 36 })?;
let scene = arda_scene::build_scene(&d.layout, &library, render_seed, Some(&d.rules))?;
```

```sh
arda dungeon --seed 7 --kind dungeon --size 48x36 --out out/d7.json \
    --rules out/d7.rules.json --meta out/d7.meta.json --scene out/d7.scene.json \
    --render out/d7.png --library assets/tactical/placeholder
arda tactical render --layout out/d7.json --out out/d7.png   # the layout renders like any other
```

The server serves the same levels at `GET /v1/tactical/dungeon`, `/dungeon.png` and
`/dungeon/scene` (see `crates/arda-server/API.md`).

## What comes out

`Dungeon { params, layout, rules, rooms, doors, exits }`:

- `layout`: squares, walls on square edges, props and lights. Unexcavated
  rock is ground `bedrock`; a wall stands on every rock/floor edge.
- `rules`: a format-2 `RulesSidecar`. Rock squares block movement and
  sight (so no spawn hint or path ever lands in the rock); cave scree is
  difficult terrain.
- `rooms` (purpose and rectangle), `doors` (edge, `locked`, `secret`) and
  `exits` (`stairs_up`, `stairs_down`, `mouth`, each with the floor square a
  token arrives on).

Sizes run from 16 to 160 squares a side. The same `Params` always give the
same bytes; the generator uses its own SplitMix64 stream, never a
third-party RNG.

## Dungeons (`Kind::Dungeon`)

1. **Rooms.** Binary space partition of the map into leaves of at least
   7 squares; one room per leaf, 3 to 12 squares a side, with rock around it.
2. **Corridors.** Prim's minimum spanning tree over room centres, plus a
   few extra links between near rooms for loops; each link is an L-shaped
   1-square corridor. All corridors share one zone, so they merge freely.
3. **Walls and doors.** A wall stands on every edge between rock and floor
   and between two different zones; where a corridor path crosses into a
   room, the edge is an opening instead. Openings into the treasure room are
   `locked` doors; openings used only by loop corridors are often `secret`;
   about a fifth are open arches (never next to another opening, so no door
   hangs free), and some others are `locked`.
4. **Purposes.** The room nearest a random corner is the entrance (stairs
   up); the room most hops away holds the stairs down; a dead-end room holds
   the treasure. The rest cycle through crypt, barracks, storage, shrine,
   prison and hall. A prison gets a row of 2 × 2 cells along a wall no
   corridor enters, each its own zone behind a locked gate, with a cot and
   a bucket.
5. **Dressing.** Props by purpose (tombs, coffins, bones, skeletons; beds,
   chests, weapon racks; barrels, crates, sacks, cask racks; altar, candles,
   statues, braziers; cages; treasure chests; tables, benches, banners),
   one or two torch sconces per room and the odd torch, rubble pile or bone
   pile in corridors. Back-to-wall props face out of their wall.
   Doorways and stair feet are kept clear, and a blocking prop is only
   placed when the room's open squares stay connected.

Floors are `flagstone` in rooms and `stone_floor` in corridors and cells;
walls use the `stone` kit.

## Caves (`Kind::Cave`)

1. **Caverns.** 46 % random rock, five passes of the 4-5 cellular-automaton
   rule, map border solid. The largest region is the cave; every other
   region of 6 squares or more is joined to it by a winding 2-wide tunnel,
   smaller pockets are filled. Maps whose cave covers under a quarter of the
   interior are redrawn with the next draw of the stream.
2. **Mouth.** A 2-wide tunnel from the cavern to a random map edge; its
   border edges carry no wall, so the scene's edge exits find it.
3. **Water.** Pools where low-frequency noise peaks away from the walls
   (2 ft wading, 6 ft swimming at the core), and in most caves a 1–2 ft
   stream along the shortest path between the cave's two far ends.
4. **Ground.** `cave_floor`, with `gravel` under and `mud` or `gravel`
   beside water, patches of `moss`, and `scree` near the walls (difficult
   in the sidecar).
5. **Dressing.** Stalagmites and large rocks in open ground, boulders,
   small rocks and stones along the walls, rubble piles, mushroom rings in
   damp spots (every other one with a faint teal light), a lair's bones,
   sometimes a skeleton or a treasure chest, and stairs down in the
   farthest dry wall slot from the mouth. The same connectivity guard keeps
   every walkable square reachable (water counts as walkable: wade or swim).

Walls use the `cave` kit.

## Vocabulary

Everything placed exists in the committed placeholder library, so every
level renders with `assets/tactical/placeholder` alone. Ids new with this
crate (placeholder art now, AI prompts in `tools/art-gen`, tier 4):
`ground.cave_floor`, `ground.bedrock`, wall kit `cave`, `prop.stairs_down`,
`prop.torch_sconce`, `veg.stalagmite`. Placeholder art was also added for
ids the AI library already had: `prop.tomb`, `prop.coffin`,
`prop.bone_pile`, `prop.skeleton`, `prop.cage`, `prop.gaol_cot`,
`prop.rubble_pile`, `prop.chest_treasure` and `prop.stairs`.

Dungeon props carry only free tags (`dungeon`, `crypt`, `prison`, …) that no
town or field tag query uses, so no query picks them. Town blocks resolved
against the placeholder library alone now draw `prop.stairs` instead of its
ladder fallback.

## Checks

`arda_dungeon::check` holds the invariants the tests assert, for callers
that want to verify a level: `unreachable(scene, start)` (squares that are
walkable yet unreachable with every door opened) and `floating_doors(layout)`
(doors not between two floors, or with an end no wall touches).

## World tie-in

`site_seed(world_seed, gx, gy, salt)` gives a stable seed per world cell;
the server's `?gx=&gy=` form uses it. Choosing *where* a world has dungeon
entrances (crypt entrances near settlements, cave mouths on steep slopes)
is a follow-up.

## Limits

- One level per map; stairs mark the links to other levels, which the
  caller generates with its own seeds.
- Walls follow square edges, so cave walls are stepped rather than curved.
- The compositor's light pools ignore walls, so torchlight glows over the
  surrounding rock in the image (the scene's lights are exact).
