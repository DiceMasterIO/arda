---
generated_date: 2026-09-30
scenario: scene-data
status: normative design; implementation in progress on feat/tactical-scene (crate arda-scene)
goals: 48, 57, 65, 67, 69
---

# 12 — Scene data: SRD movement, cover, line of sight and lights

> Normative design for the game-facing scene JSON that travels with every tactical image. The scene is derived from the same `TacticalLayout`, library and render seed as the painted image ([11](11-tactical-art-compositor.md)), plus the per-square rules sidecar from [09](09-tactical-refinement.md) §rules-sidecar and the NPC tokens of [13](13-npc-population.md). Image and data come from one layout, so they never disagree (goal 48). Rules follow SRD 5.1 (CC-BY-4.0, attributed in `NOTICE`); where the SRD leaves a number open, the value is marked assumed.

Code cites rules as `// logic/12 §<rule>`.

## Trigger & preconditions

- Trigger: `build_scene(layout, library, seed, rules)`, then `attach_tokens(scene, tokens)`; called by the service for `/v1/tactical/.../scene` (16-service-api) and by tests.
- Preconditions: the layout passes `TacticalLayout::check` against the library; `seed` is the seed the image was rendered with (11 §seam-art); the sidecar, when given, has format 1 and the layout's width and height.

## Rules

### §scene-frame

Coordinates are 5-ft squares, origin at the layout's top-left, y south, as in `TacticalLayout`. Grid vertices are integers; square `(x, y)` spans `x..x+1 × y..y+1`. The scene records `origin_gs` (09 §square-frame) so the game can place neighbouring scenes (goal 67). Per-square layers are row-major `Grid`s serialised as run-length `[[count, value], …]`, bounded at 2²² squares when decoded.

### §scene-movement

Per-square `movement` (SRD 5.1 "Movement" and "Special Types of Movement"):

| Value | When | Cost |
|---|---|---|
| `normal` | ordinary ground | 5 ft per square |
| `difficult` | sidecar `difficult = true`, or an asset with `difficult_terrain` covers the square | +1 ft per ft (SRD) |
| `wade` | water depth 1–4 ft | +1 ft per ft; treated as difficult terrain (assumed: SRD gives no depth threshold) |
| `swim` | water depth ≥ 5 ft | +1 ft per ft (SRD swimming) |
| `impassable` | an asset with `blocks_movement` covers the square | cannot be entered |

The sidecar's `water_depth_ft`, when present, overrides the layout's depth. The wade/swim threshold (5 ft) is the one source for depth semantics; the compositor's visual shallow/deep blend (11 §ground-blend) is presentation only.

### §scene-climb

`climb` is a per-square 8-bit mask (bit order N, NE, E, SE, S, SW, W, NW). A bit is set when the neighbour's `elevation_ft` differs by ≥ 10 ft; that step is a climb, +1 ft per ft (SRD climbing). The 10-ft threshold is assumed; it matches the cliff rule of 09 §linear-features. Extra costs add: climbing out of a wade costs 15 ft per square.

### §scene-diagonal

Diagonal steps cost 5 ft by default; the alternating 5/10/5 rule is an option recorded in `rules.diagonal` (`five` or `alternating`). Source: assumed grid convention, to be checked against the SRD 5.1 text before it is described as SRD. A diagonal step may not cut a vertex touched by a movement-blocking wall, nor squeeze between two impassable squares.

### §scene-cover

Per-square `cover` is the strongest of the sidecar's cover and the cover of assets covering the square: `none`, `half` (+2 AC and Dex saves), `three_quarters` (+5), `total` (cannot be targeted directly) (SRD 5.1 "Cover"; only the most protective degree applies). The catalogue value `full` reads as `total`. Walls that do not block sight (windows) grant their `cover` to targets behind them. `cover_between(a, b)` samples lines from the attacker's square corners (inset so they never lie on a grid line) to the target's, and returns the best degree any obstacle gives, as a server-side reference the game may reimplement.

### §scene-sight

- Walls: every `WallSegment` becomes a unit edge; plain walls and windows merge into maximal collinear polylines with identical attributes; doors, gates and secret doors stay one edge each so each can open. Later segments on an edge replace earlier ones.

| Kind | Blocks movement | Blocks sight | Blocks light | Opens | Default |
|---|---|---|---|---|---|
| `wall` | yes | yes | yes | no | — |
| `window` | yes | no | no | no | — |
| `door` | when closed | when closed | when closed | yes | closed |
| `gate` | when closed | when closed (solid) | when closed | yes | closed |
| `secret` | when closed | yes | yes | yes | closed, drawn as wall |

- `obscured` per square: `heavy` where the sidecar says `blocks_sight` or a `blocks_sight` asset covers it; `light` where the sidecar says `lightly_obscured`, or a canopy covers it without blocking sight (SRD "lightly obscured: … moderate foliage"; "heavily obscured: … dense foliage"). Canopy and sight-blocking props also appear as `vision_blockers` polygons for the game's renderer.
- Line of sight runs in integers at 8 units per square (square centre `8x + 4`), walking every grid-line crossing: a sight-blocking wall edge blocks; entering a heavily obscured square other than the target blocks; passing exactly through a vertex blocks only if blockers lie on both sides (a line may graze a wall end but not slip through a closed corner).

### §scene-lights

Free layout lights first, then emissive assets in placement order. Each has `bright_ft` (the catalogue `radius_ft`) and `dim_ft = 2 × bright_ft` (SRD 5.1: candle 5/10, torch 20/40, lamp 15/45 is the exception, hooded lantern 30/60). Assets must therefore carry the SRD bright radius; a lamp-type asset may set its own dim radius once the catalogue grows a `dim_ft` field (not in format 1). Lights block by walls with `blocks_light`.

### §scene-regions

`regions` merge squares of kind `difficult`, `shallow_water` and `deep_water` into closed rings on grid vertices (outer rings clockwise in y-down coordinates, holes anticlockwise, even-odd fill), for the game's overlay.

### §scene-spawn

`spawn_hints`: `open` squares (normal movement, no cover, and 8 identical neighbours), `entrances` (enterable squares beside each door or gate), and `exits` (maximal enterable runs along each map edge, in N, E, S, W order) for walking onto the neighbouring map (goal 67).

### §scene-sidecar

The rules sidecar is the one channel for per-square and per-edge rules that `TacticalLayout` cannot carry. Format 1 (implemented on feat/tactical-scene) has per square: `difficult`, `water_depth_ft`, `cover`, `blocks_sight`, `lightly_obscured`, each optional. feat/tactical-ways currently writes a separate sidecar of its own (also "version 1") with a per-square `feature`, `road_class`, `deck`, `deck_elevation_ft`, `difficult` and per-edge rules for bridge parapets and retaining walls. They are merged into **format 2** of the `arda-scene` sidecar, owned by feat/tactical-scene:

- per square, added: `deck_elevation_ft` (i16, optional): a walkable deck (bridge, landing stage, dock) over the square; creatures on it stand at that elevation and the square's movement is `normal` whatever the water below; `feature` (optional string from the ways `Feature` set, informational for the game's overlay).
- top level, added: `edges: [{x, y, axis, role, blocks_movement, blocks_sight, cover}]` for low edge features that are not wall-kit walls (bridge parapets and retaining walls: block movement, not sight, grant `half` cover; assumed from SRD "low wall").
- Merge of several owners' sidecars: per field, the owner with the higher reservation precedence of 09 §reservations wins; `cover` takes the stronger; `difficult` is true if any owner sets it.

Format 1 sidecars stay readable (the new fields default to absent).

### §scene-tokens

Tokens are not in scene format 1; they are added by the service (format 2 of the tactical response, 16-service-api §api-tactical):

- `tokens: [{npc_id, x, y, building_id, kind}]` where `npc_id` is the wire form of 13 §npc-id, `(x, y)` a square, and `kind` is `resident` or `worker`.
- Placement: for the default time `day`, workers stand in their workplace building and everyone else in their home (the building must be at least partly inside the scene); `night` puts everyone at home. Inside a building, tokens take free floor squares nearest the room centre of the room whose tag suits the job (for example the innkeeper in the `common` room), by hash of the NPC id; never an impassable, wall-adjacent doorway or water square (assumed, tunable).
- Only notables and people whose building lies in the scene get tokens; commoners are enumerated by `(building, index)` without being stored (13 §npc-regeneration).

## Steps

1. Check layout, library and sidecar (Preconditions).
2. Resolve placements with the render seed (so the scene names the assets the image shows).
3. Per-square layers: movement, climb, cover, obscured, elevation, water depth (§scene-movement, §scene-climb, §scene-cover, §scene-sight).
4. Walls, vision blockers, lights, regions, spawn hints.
5. Attach tokens (service, §scene-tokens).

## Branches

- No sidecar: rules come from the layout and assets alone.
- A wall kit whose door piece carries the free tag `secret` makes its doors `secret`.

## Unhappy paths

- Layout or library inconsistency: `SceneError::Layout` (422 at the service).
- Sidecar version or size mismatch: `SceneError::Sidecar` naming the mismatch (422).
- Grid over 2²² squares: refused on decode.

## State transitions

Door, gate and secret-door `open` state is data in the scene; Arda always emits the default (closed). Opening a door is the game's state, not Arda's.

## Invariants

1. Determinism: the same inputs give byte-identical scene JSON [48].
2. Agreement: every placement in the scene resolves to the same asset id the compositor draws for the same seed; every wall edge in the scene is a wall edge in the image [48].
3. Movement and depth: every square with depth ≥ 5 is `swim`, 1–4 `wade` (unless impassable) [48].
4. Walls: every unit edge of the layout appears in exactly one scene wall; door edges are single-edge walls [48].
5. Line of sight is symmetric for points at square centres, and a sightline through a closed corner is blocked [48].
6. Path cost of a straight 6-square move over normal ground is 30 ft; over difficult terrain 60 ft [48].
7. Tokens: every token's `npc_id` resolves through `/v1/npc/{id}` to an NPC whose home or workplace is `building_id`, and the token square lies inside that building [45, 57, 69].
8. Seams: the `exits` of one block's east edge and the neighbouring block's west edge list the same rows [67].

## Outcomes & side effects

`Scene` JSON, format 1: `{format_version, name, width, height, seed, library, library_version, rules, movement, climb, cover, obscured, elevation_ft, water_depth_ft, walls, vision_blockers, lights, regions, spawn_hints}`, documented in `crates/arda-scene/README.md`. The service adds `origin_gs` and `tokens` (§scene-tokens) and writes `seed` as a decimal string on the wire (16-service-api §api-conventions). No files are written.

## Dimensions not in play

- Dynamic state (open doors, fire, fog, darkness spells, creature positions after load).
- Vertical line of sight across elevation (height advantage, cliffs as sight blockers): sight is 2-D; `elevation_ft` is provided so the game may add it.
- Hazards and traps (the logic/03 POI layer is a later scenario).
