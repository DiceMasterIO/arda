---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/arda-gen/**", "crates/arda-render/**", "crates/arda-cli/**"]
---

> Prescriptive — written from the design interview, not from code.

# Data flow

Three lifecycles, fully specified in `logic/`; hop-level detail there,
this chapter maps them onto the planned crates.

## Lifecycles

1. **generate** (`logic/01→02→03`): `arda-cli` parses → `arda` facade → `arda-gen` orchestrator: continent stage (single-threaded, validate+reroll ≤5) → tile bundles → areas fan out on the thread pool (each: relief→water→climate→vegetation→settlement→land-use→roads, edges pinned) → blocks per land cell (WFC, ≤8 retries then relaxed) → `arda-core::formats` writes layers → manifest written last (completion stamp).
2. **export** (`logic/04`): CLI/facade → `arda-core` loads manifest + needed layers (blocks decompressed on demand) → `arda-render` renders PNG (symbolic or tileset manifest; cartographic for area/continent) and serializes versioned JSON → whole-file writes to `--out`.
3. **load-query** (`logic/05`): `World::load` reads manifest eagerly; areas lazy + cached; blocks lazy; typed errors (missing/partial, version skew, corruption, range).

## State

The world directory is the only persistent state; immutable once the
manifest is stamped (single writer: the batch). In-process state:
orchestrator progress counters (for `mockup/01`'s progress rows) and
the load-side area cache (`logic/05` step 2), bounded O(accessed).
Nothing else holds state; no sessions, no server.

## Side-effect boundaries

All disk IO lives in `arda-core::formats` (world layers) and
`arda-render` (export artifacts). `arda-gen` stages are pure —
compute-only over in-memory inputs; the orchestrator alone invokes
writes. No network IO exists anywhere (`01-architecture.md`
Communication).

## Failure paths

- Continent validation failure → deterministic subseed reroll ≤5 → hard error (`logic/01` §Q9).
- WFC contradiction → subseeded refill ≤8 → relaxed fill, marked; never fails the batch (`logic/03` §Q12).
- Tile panic → batch halts naming the tile (`logic/02`).
- Batch interrupt/crash → manifest absent → loaders and export refuse the partial world; re-run restarts identically; partial dir left for inspection (`mockup/01` States).
- Export: out-of-range/unmapped-tile/partial-world → typed refusals, no partial artifact files (`logic/04`).
- Memory budget (default 16 GB, configurable — §Q3) bounds areas in flight; the pool blocks rather than exceeding it.
