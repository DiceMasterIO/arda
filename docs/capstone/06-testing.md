---
mode: prescriptive
generated_date: 2026-08-27
paths_covered: ["crates/*/src/**", "crates/*/tests/**", "tests/**"]
generated_at_commit: 8be0a0a
---

> Prescriptive — written from the design interview, not from code.

# Testing

Three layers, decided at `architecture-interview.md §Q5`; style detail
(naming, doubles idiom) pending code-prefs.

## Layout

- Unit tests inline per crate (`crates/*/src/`, `#[cfg(test)]`) on stage rules: fixed inputs → exact outputs, possible because of §Q4 bit-exactness.
- Statistical validation suite in `crates/arda-gen/tests/`: generates small fixture continents and asserts the artifact's gates — Horton ratio 3–5, Hack exponent ~0.55, settlement rank-size, road sinuosity 1.2–1.4, ~1 ha farmland/inhabitant (`logic/02` invariants) — as CI regression gates.
- Golden-world snapshots in `tests/` (workspace level): tiny seed → per-stage output hashes committed; CI compares on Linux/macOS/Windows — the §Q4 determinism gate.
- Run command: `cargo test --workspace` (Developer workflow, `07-operations.md`).

## Doubles

Effectively none: stages are pure functions over value inputs (`04-data-flow.md`), so tests feed literal inputs — no IO to fake. Disk-format round-trip tests use temp dirs. Ambient randomness is forbidden (§Q4), so no seams need faking; time is never read in sim paths.

## Observed — area drainage rewrite (2026-08-26)

- **157 tests** pass across the workspace on the 3-OS CI matrix: unit tests per crate, `crates/arda-gen/tests/drainage_invariants.rs`, `crates/arda/tests/round_trip.rs`, `crates/arda-cli/tests/cli.rs`, and the workspace golden gate `tests/golden_world.rs`.
- **Global drainage invariants** now exist, and are the direct answer to the coverage-shape finding recorded below: (a) every land cell's flow path reaches sea, a lake, or the tile edge; (b) lakes are emitted and meet their thresholds; (c) non-land cells carry no flow; (d) segments partition the channel network exactly once; (e) Strahler order never decreases downstream; (f) the same seed yields identical lakes and reaches; (g) each segment's terminus agrees with its link; (h) tile seams are no rougher than the interior, compared land-to-land.
- **`crates/arda-gen/benches/area_erosion.rs`** enforces the erosion budget: both passes must finish within 30 s per 512×512 tile in release. Measured 2.17 s.
- `[profile.test] opt-level = 2` — unoptimised sim code made the suite roughly eight times slower and dominated CI wall-clock.
- **The statistical validation suite still does not exist.** No Horton, Hack, rank-size, sinuosity, or farmland gate is implemented (step 12). Until it does, the erosion constants are calibrated only against the artifact's two stated equilibrium anchors, not against network statistics.

## Observed — continent climate and hydrology (2026-08-26, feature 02)

- **194 tests** pass across the workspace. New: `crates/arda-gen/tests/continent_hydrology.rs` — the tier's five structural invariants on the real MICRO continent (every land 1 km cell reaches ocean without cycles; catchment and discharge monotone downstream; courses connected and descending on the routing surface with `feeds` acyclic; sea cells zeroed; stage determinism plus the golden byte gate) — and codec refusal/round-trip suites in `arda-core` covering crafted-header overflow, the exact `usize` wrap window (dims 238,795,480 × 4,291,618,565), dims past `i32::MAX`, and both older- and newer-major manifest refusal.
- **`crates/arda-gen/tests/continent_measures.rs`** holds the `#[ignore]`d R8 probes (rainfall land-mean / windward-leeward, Horton): measured, never gated — recorded in the feature's spike report. Statistical gates remain step 12.
- **`crates/arda-gen/benches/continent_stage.rs`** gates the full continent stage at 60 s release for the default 500×1000 km world (criterion group on MICRO, loud assert on default), mirroring `area_erosion.rs`.
- The golden fixture was re-blessed **once** for `FORMAT_VERSION` 3 (feature 02 §Q6 sign-off): only `world.json`, `continent/overview.bin`, and the new `continent/objects.bin` hashes changed; zero area or block drift.

### Coverage-shape finding, and how it was closed

Before the invariants above, every sim test asserted a *local* property —
round trips, determinism, per-cell bounds, edge agreement. A drainage
stage in which **93% of catchment terminated in an interior pit** passed
the entire suite. Determinism gates prove output is stable, never that
it is correct; the two must not be conflated when judging a stage done.

## Coverage shape

Priorities (prescriptive): heaviest on `arda-core::formats`
(round-trip + version-refusal), stage rules in `arda-gen`, and the
export byte-identity property (`logic/04`). Bench suite enforces §Q7's
export/load numbers. Coverage percentage targets: none set — the
statistical + golden gates are the meaningful floor (§Q5).

## Observed — climate-driven refinement (2026-08-27, feature 03)

- **228 tests** pass across the workspace. New: `crates/arda-gen/tests/cross_tile.rs` — seam continuity against an independent re-derivation from the continent tree; inflow effectiveness (a tile's whole-tile max drainage rises with inflow, and every seeded cell's own downstream path rises); rainfall/discharge/initiation rules on composed cells; seam-lake agreement; determinism. `drainage_invariants.rs` gained a lake-outlet geometric-soundness sweep.
- **Calibration is measured, not assumed.** The render band cutoffs were set from a default-world Strahler histogram (a floor of 4 would have kept only 5.58% of channel cells; the shipped floor of 3 keeps 17.94%). The entering-order merge rule is pinned on a seed where the correct and incorrect rules diverge, verified to fail under inversion.
- **Recorded limits of the fixtures.** No natural seam-straddling lake pair exists in 4,280 surveyed (seed, seam) combinations, so that case is covered by a synthetic two-tile test rather than a natural one. Seam crossings align with an upstream outlet 53–94% of the time depending on catchment size (median jog on a mismatch: 8 cells) — an inherent 1 km-vs-100 m tier approximation, pinned as a calibrated test rather than an exact equality.
- The golden fixture was re-blessed twice on this branch: once for the feature's own cell changes, and once for the Marsh→Dry defect the review loop found before merge.
