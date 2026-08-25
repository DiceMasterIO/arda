---
generated_date: 2026-08-24
scenario: load-query
traces: [Q16]
generated_at_commit: 8d3c9d9
---

# 05 — Load & query

The crate's read path (`../mockup/04-crate-api.md`); the CLI's `export`
consumes the same path. Confirmed at §Q16.

## Trigger & preconditions

- Trigger: `World::load(dir)` (identifier assumed, mockup 04).
- Preconditions: complete world directory — manifest present, stamped, version-compatible.

## Steps

1. **Load**: read `world.json` eagerly — seed, config echo, coordinate ranges, validation stats. Nothing else is touched.
2. **Lazy area access**: a tile's cells/objects load on first access and cache; memory stays O(accessed areas), never O(world).
3. **Lazy blocks**: a cell's tile-IDs decompress on demand from the area archive.
4. **Queries**: manifest facts; per-cell properties (the artifact's field list); typed per-area object lists (river segments, lakes, settlements, roads, crossings, passes) and continent objects (named rivers, ranges, regions, seas); block square lookup and render (same renderer as export).

## Branches

None beyond target scope — all reads are pure lookups.

## Unhappy paths

- Missing/partial world: `load` returns a typed error naming the manifest problem.
- Version skew (world written by an incompatible arda): typed error carrying both versions; remedy is regeneration from the seed (mockup Q9 — worlds are disposable).
- Corrupt/truncated archive discovered lazily: typed error naming the file.
- Out-of-range coordinates: typed range error carrying the valid ranges.

## State transitions

None — strictly read-only; no lock files, no metadata writes, ever. Concurrent readers are safe because the world is immutable.

## Invariants

- Loading never mutates the directory.
- Queries return exactly what `generate` wrote — no recomputation, no drift.
- After a successful `load`, only range and corruption errors exist; every other read is infallible.
- Memory bound: O(accessed areas + accessed blocks).

## Outcomes & side effects

- Success: typed world/area/cell/block views for the consumer; no side effects.
- Failure: typed errors as above; the directory is untouched.
