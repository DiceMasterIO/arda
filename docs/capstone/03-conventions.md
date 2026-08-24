---
mode: prescriptive
generated_date: 2026-08-24
paths_covered: ["crates/**"]
---

> Prescriptive — written from the design interview, not from code.

# Conventions

Code-style conventions (paradigm detail, typing strictness, lints,
naming) are owned by the `code-prefs` stage (`code-prefs.md`, pending —
`architecture-interview.md §D8`). This chapter records only the
architecture-level conventions already decided.

## Paradigm

Rust; sim stages as pure functions over prior stages' outputs (the
causal rule, `logic/02`). Further paradigm decisions: pending
code-prefs.

## Typing

Rust's static typing; sim-core arithmetic must be integer/fixed-point
or strictly-ordered IEEE — no fast-math, no platform intrinsics in sim
paths (`§Q4`). Strictness/lint configuration: pending code-prefs.

## Error handling

Typed errors, `Result` everywhere at the API surface (`logic/05`,
`mockup/04`): load errors name the manifest problem; version skew
carries both versions; corruption names the file; range errors carry
valid ranges. CLI maps them to non-zero exits with the messages mocked
in `mockup/01`/`mockup/03`. Panics are defects; a panicking tile halts
the batch naming the tile (`logic/02`). Exception-vs-result nuance
beyond this: pending code-prefs.

## Dependency injection

None — no container, no globals holding state (`01-architecture.md`
Composition). The only ambient inputs are (seed, config), passed
explicitly; all randomness flows from the counter-based PRNG keyed by
(seed, tier, stage, coords, attempt) (`§Q4`) — `thread_rng`-style
ambient randomness is forbidden everywhere.
