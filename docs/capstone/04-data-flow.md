---
mode: prescriptive
generated_date: 2026-08-25
paths_covered: ["crates/arda-gen/**", "crates/arda-render/**", "crates/arda-cli/**"]
generated_at_commit: 8d3c9d9
---

> Prescriptive — written from the design interview, not from code.

# Data flow

Three lifecycles, fully specified in `logic/`; hop-level detail there,
this chapter maps them onto the planned crates.

## Lifecycles

1. **generate** (`logic/01→02→03`): `arda-cli` parses → `arda` facade → `arda-gen` orchestrator: continent stage (single-threaded, validate+reroll ≤5) → tile bundles → areas fan out on the thread pool (each: relief→water→climate→vegetation→settlement→land-use→roads, edges pinned) → blocks per land cell (WFC, ≤8 retries then relaxed) → `arda-core::formats` writes layers → manifest written last (completion stamp).
2. **export** (`logic/04`): CLI/facade → `arda-core` loads manifest + needed layers (blocks decompressed on demand) → `arda-render` renders PNG (symbolic or tileset manifest; cartographic for area/continent) and serializes versioned JSON → whole-file writes to `--out`.
3. **load-query** (`logic/05`): `World::load` reads manifest eagerly; areas lazy + cached; blocks lazy; typed errors (missing/partial, version skew, corruption, range).
4. **serve** (`mockup/06`, build §Q1): `arda-cli` binds a synchronous listener → each request maps to a load-query read plus, where the path asks for an artifact, the same `arda-render` call `export` makes → response bytes. No request mutates anything; determinism makes the response a pure function of (seed, path), which is what the ETags are derived from.

## Observed — steps 0–3 (2026-08-25)

- The area stage runs `relief → water → compose` only; the other five stages of lifecycle 1 are unbuilt (step 5).
- **`water` takes `(relief)` and nothing else.** It never sees the `TileBundle`, so `logic/01` step 10's entering rivers have no consumer and no cross-tile flow exists.
- **There is no outflow boundary condition.** Off-tile neighbours are skipped in the D8 search, so a cell on the tile edge routes to the best *inland* downhill neighbour instead of leaving the tile.
- Flow accumulation runs over sea cells as well as land.

## State

The world directory is the only persistent state; immutable once the
manifest is stamped (single writer: the batch). In-process state:
orchestrator progress counters (for `mockup/01`'s progress rows) and
the load-side area cache (`logic/05` step 2), bounded O(accessed).

`serve` adds a process but no state class: it holds one loaded `World`
and reuses that same area cache. There are no sessions, no cookies, and
no server-side mutation — restarting it loses nothing.

## Side-effect boundaries

All disk IO lives in `arda-core::formats` (world layers) and
`arda-render` (export artifacts). `arda-gen` stages are pure —
compute-only over in-memory inputs; the orchestrator alone invokes
writes. The only network IO is `serve`'s inbound listener, confined to
`arda-cli` (`01-architecture.md` Communication); nothing in the
workspace makes an outbound connection.

## Failure paths

- Continent validation failure → deterministic subseed reroll ≤5 → hard error (`logic/01` §Q9).
- WFC contradiction → subseeded refill ≤8 → relaxed fill, marked; never fails the batch (`logic/03` §Q12).
- Tile panic → batch halts naming the tile (`logic/02`).
- Batch interrupt/crash → manifest absent → loaders and export refuse the partial world; re-run restarts identically; partial dir left for inspection (`mockup/01` States).
- Export: out-of-range/unmapped-tile/partial-world → typed refusals, no partial artifact files (`logic/04`).
- Serve: partial world → refuses to start with export's message; unknown coordinates → 404 carrying the valid ranges, mirroring `logic/05`'s range errors (`mockup/06` States).
- Memory budget (default 16 GB, configurable — §Q3) bounds areas in flight; the pool blocks rather than exceeding it.
