---
generated_date: 2026-09-07
generated_at_commit: 987aeca04c77
capstone_version: 6.4
---

# Standards — Arda

> User-stated requirements, retained separately from observed code. Unspecified domains have not been decided by this refresh.

## Typing

- `#![deny(unsafe_code)]` workspace-wide.
- `unwrap`/`expect` banned outside `#[cfg(test)]`.
- Domain newtypes for every coordinate and identifier kind (`CellCoord`, `AreaCoord`, `TileId`, …) — never bare `u32`s that can cross grids.
- Enums over raw strings/ints for every closed set.

## Libraries vs reinventing

- Determinism-critical code — erosion, hydrology, WFC, PRNG usage, noise — is hand-rolled or vendored under repo control. Never delegate sim arithmetic to a third-party crate.
- Commodity is adopted: zstd, PNG, serde, CLI parsing, rayon-class parallelism.
- Vetting bar per new dependency: permissive license (MIT/Apache-compatible), maintained > 1 year, no platform-variant arithmetic.
- Whole-tree soft cap ~40 crates; justify any growth past it.

## Paradigm

- Data-oriented functional core: stages are pure functions over plain data; struct-of-arrays for hot grids.
- Mutation stays local to a function; handoffs between stages are immutable.
- Introduce a trait only when two implementations exist today (e.g. render styles); no speculative traits, no trait hierarchies.
- Dependency injection = plain function arguments; no containers, no globals.

## Error handling

- Per-crate error enums, thiserror-style derives; no `anyhow` in library crates — `arda-cli` only.
- Panics are bugs; invariant-backed indexing may panic with a message naming the invariant.
- Never swallow an error: propagate, or log at the CLI boundary. No logging framework inside library crates.

## Organization

- Package by stage/feature, mirroring the pipeline (`continent/`, `area/relief.rs`, `area/water.rs`, …, `block/`); no `utils/` or `types.rs` dumping grounds — types live with their domain.
- Files ≤ ~500 lines, soft.

## Documentation

- `#![deny(missing_docs)]` on published crates; document the public API fully.
- Comments state the why and cite the rule implemented (`// logic/02 §floodplain`); never restate signatures.

## Testing

- Test-first for stage rules and formats: write the failing unit test from the logic doc before the code.
- No mocks, ever — pure functions and temp dirs suffice.
- Every bugfix lands with its regression test.
- Property-based tests welcome for format round-trips.
- The statistical and golden-world suites (`06-testing.md`) are maintained gates, never deleted to go green.

## Tooling

- rustfmt defaults — no custom config.
- clippy at `-D warnings` plus selected pedantic lints, enforced in CI.
- Stable toolchain; MSRV = latest-stable-minus-2, checked in CI.
- cargo-deny for license and advisory checks.

## CI gates

The required gates are workspace tests, the statistical and golden-world suites, formatter, Clippy with warnings denied, the MSRV check and cargo-deny. These are retained requirements from Testing and Tooling; observed CI coverage is in `06-testing.md` and `07-operations.md`.

## Security

No separate rule was recorded in this standard. Existing design constraints remain in `implementation.md` and the topic chapters; this migration adds no policy.

## Logging and privacy

Errors propagate or are logged at the CLI boundary; no logging framework in library crates (Error handling). No additional privacy policy was recorded.

## API conventions

No separate rule was recorded in this standard. Existing design constraints remain in `implementation.md` and the topic chapters; this migration adds no policy.

## Accessibility

No separate rule was recorded in this standard. Existing design constraints remain in `implementation.md` and the topic chapters; this migration adds no policy.

## Performance budgets

No separate rule was recorded in this standard. Existing design constraints remain in `implementation.md` and the topic chapters; this migration adds no policy.

## Versioning and release

- `CHANGELOG.md` maintained per release; 0.x uses honest semantic versioning.

## Process

- Conventional commits (`feat:`, `fix:`, `docs:`, `refactor:` …), imperative subject ≤ 72 chars.
- Trunk-based; direct pushes acceptable while solo.

## Agent rules

- Never introduce ambient randomness, wall-clock time, or hash-iteration-order dependence into sim paths.
- Never modify golden hashes unless explicitly asked.
- Any sim-rule change must cite the `logic/` doc rule it implements.

## Not in play

No dimension-by-dimension exclusion list was recorded in the original standard. Unspecified domains above remain unspecified.
