# Light pools stop at walls

- **Request**: OI-39, torchlight and lamp glow shone through walls onto rock and neighbouring rooms in the composited PNG.
- **Change**: `crates/arda-tactical/src/compose/light_los.rs` computes a per-pool visibility mask in layout coordinates (8 samples per square, integer segment-versus-edge test, scene vertex rule, two blurs) and `add_pool_row` scales the pool by it. Pools that reach no blocking edge are byte-identical to before.
- **Shared rule**: `WallSegment::blocks_light(lib)` is used by the compositor and by `arda-scene`; windows pass light, closed doors, gates and secret doors block. Walls only; props and canopies do not occlude.
- **Render identity**: lit maps with walls near lights render differently; no cache key changed (server caches are in memory).
- **Docs**: logic/11 §lighting; OI-39 removed; the torchlight limit removed from `crates/arda-dungeon/README.md`.
- **Tests**: `compose/tests_light.rs` (wall, window, door and gate, light on an edge, open-ground golden, determinism, clip invariance) and `crates/arda-scene/tests/light_parity.rs`.
