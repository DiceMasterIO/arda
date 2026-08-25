---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/*/src/**", "crates/*/tests/**", "tests/**"]
generated_at_commit: 8d3c9d9
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

## Observed — steps 0–3 (2026-08-25)

- 113 tests pass: unit tests in all four library crates, `crates/arda/tests/round_trip.rs`, `crates/arda-cli/tests/cli.rs`, and the workspace golden gate `tests/golden_world.rs` with fixture `tests/golden/micro-42.txt`, run on the 3-OS CI matrix.
- **The statistical validation suite does not exist.** No Horton, Hack, rank-size, sinuosity, or farmland gate is implemented (step 12).
- **Coverage-shape gap, recorded as a finding.** Every existing sim test asserts a *local* property — round-trips, determinism, per-cell bounds, edge agreement. None asserts a *global* one such as "every land cell drains to sea or tile edge". A drainage stage in which 93% of catchment terminates in an interior pit passed the whole suite. Determinism gates prove output is stable, never that it is correct; the two must not be conflated when judging a stage done.

## Coverage shape

Priorities (prescriptive): heaviest on `arda-core::formats`
(round-trip + version-refusal), stage rules in `arda-gen`, and the
export byte-identity property (`logic/04`). Bench suite enforces §Q7's
export/load numbers. Coverage percentage targets: none set — the
statistical + golden gates are the meaningful floor (§Q5).
