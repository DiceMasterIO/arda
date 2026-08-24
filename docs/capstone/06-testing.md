---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/*/src/**", "crates/*/tests/**", "tests/**"]
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

## Coverage shape

Priorities (prescriptive): heaviest on `arda-core::formats`
(round-trip + version-refusal), stage rules in `arda-gen`, and the
export byte-identity property (`logic/04`). Bench suite enforces §Q7's
export/load numbers. Coverage percentage targets: none set — the
statistical + golden gates are the meaningful floor (§Q5).
