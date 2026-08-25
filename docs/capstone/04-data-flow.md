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

## Observed — area drainage rewrite (2026-08-26)

The area stage now runs `relief → erosion → fill → water → compose`;
climate, vegetation, settlement, land use, and roads remain unbuilt
(step 5).

- **Erosion** implements the artifact's four coupled processes over 40 iterations: uplift scaled by the coarse relief beneath each cell, stream-power incision (`K·sqrt(A)·S`, clamped so a channel never cuts below what it drains into), hillslope creep, and 35-degree talus collapse. The sea is the floor and sea cells never move.
- **Filling** is priority-flood to a *routing* surface; stored `Cell.height` stays raw eroded relief, because the artifact keeps depressions as real terrain that rivers pass through. The heap key is `(height, y, x)` — `BinaryHeap` leaves equal keys unordered, and a bare height key would break the cross-platform golden gate while passing locally.
- **Routing** is D8 by steepest *descent* (drop ÷ distance, compared by integer cross-multiplication), not steepest drop. `water` takes the `TileBundle` so boundary cells route against the neighbouring tile's real heights; a cell whose best descent leaves the tile becomes an **outlet**. Per the artifact the tree is rooted at "the sea and the low edges", so a rim cell with no downhill neighbour is an outlet rather than a sink.
- Both erosion passes hold the pinned rim fixed and ramp their amplitude to zero over 32 cells approaching it, so the frozen edge does not leave a lip. Within that band the repose-angle rule is deliberately not enforced.
- **Cross-tile inflow does not exist.** `logic/01` step 10's entering rivers still have no producer, so each tile drains independently and no river spans tiles. Outflow works; inflow waits on the continent drainage tree.

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
