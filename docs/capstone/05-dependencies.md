---
mode: prescriptive
generated_date: 2026-08-25
paths_covered: ["Cargo.toml", "crates/*/Cargo.toml"]
generated_at_commit: 8d3c9d9
---

> Prescriptive — written from the design interview, not from code.

# Dependencies

Picks decided in `stack-interview.md` (§Q1–Q8), constrained by
code-prefs §Q2 (sim = build, commodity = adopt; permissive license,
>1 yr maintained, no platform-variant arithmetic; ~40-crate soft cap)
and architecture §Q4 (bit-exact determinism). Version floors are the
latest stable at research time (2026-08); exact pins at build. Zero
paid services.

## Runtime dependencies

| Capability | Pick | Floor | License | Used by | Traces |
|---|---|---|---|---|---|
| zstd compression | `zstd` (C bindings) | 0.13 | MIT | `arda-core::formats` block archives | §Q1 |
| PNG encoding | `png` | 0.17 | MIT/Apache-2.0 | `arda-render` | §Q2 |
| JSON | `serde` + `serde_json` (derive; BTreeMap where maps unavoidable — byte-identity) | 1.0 | MIT/Apache-2.0 | manifest, export JSON | §Q3 |
| Deterministic PRNG | `rand_chacha` ChaCha8, wrapped in `arda-core::rng` keyed (seed, tier, stage, coords, attempt) — no direct use outside the wrapper | 0.3 | MIT/Apache-2.0 | all sim stages | §Q4; arch §Q4 |
| Data-parallel pool | `rayon` (order-independent reductions only) | 1.10 | MIT/Apache-2.0 | `arda-gen` orchestrator | §Q5; arch §Q3 |
| CLI parsing | `clap` v4 derive | 4.5 | MIT/Apache-2.0 | `arda-cli` only | §Q6 |
| HTTP server | `tiny_http` (synchronous, no async runtime) | 0.12 | MIT/Apache-2.0 | `arda-cli::serve` | build §Q1; arch §Q3 |
| Error derives | `thiserror` | 1.0 | MIT/Apache-2.0 | every library crate | code-prefs §Q4 |
| CLI error context | `anyhow` — `arda-cli` only, never a library crate | 1.0 | MIT/Apache-2.0 | `arda-cli` | code-prefs §Q4 |
| Sim arithmetic | hand-rolled i32/i64 fixed-point Q-format newtypes (no dependency) | — | — | `arda-core`, `arda-gen` | §Q7; arch §Q4 |
| Noise, erosion, hydrology, WFC | hand-rolled in `arda-gen` (no dependency) | — | — | sim stages | code-prefs §Q2 |

## Dev and tooling

| Capability | Pick | License | Role | Traces |
|---|---|---|---|---|
| Benches | `criterion` | MIT/Apache-2.0 | enforce arch §Q7 export/load numbers | §Q8 |
| Golden hashing | `blake3` — also a **runtime** dependency of `arda-core::rng`, which derives each ChaCha8 subseed key from `blake3(domain ‖ seed ‖ key)` | CC0/Apache-2.0 | per-stage world hashes, CI determinism gate; subseed derivation | §Q8; arch §Q4 |
| Lint/format | clippy `-D warnings`, rustfmt defaults | toolchain | CI gates | code-prefs §Q7 |
| Supply chain | `cargo-deny` | MIT/Apache-2.0 | license + advisory checks | code-prefs §Q7 |

## External services

None at runtime (fully local — arch §D4). Distribution/dev: GitHub +
Actions (CI), crates.io (publish), GHCR (docker image) — arch §Q6;
exit cost trivial for all three (standard registries), lock-in priced
at §D9.

Replaced from the architecture draft: every "stack stage" placeholder
row above now carries its pick; the `fixed` crate option was rejected
in favor of hand-rolled Q-format newtypes.

Added after the stack stage closed, each traced to the decision that
introduced it rather than to `stack-interview.md`:

- `tiny_http` — the build gate's §Q1 serve decision. Flagged there as
  "at the re-gate"; recorded here. Synchronous by choice: architecture
  §Q3 rules out an async runtime.
- `thiserror` — code-prefs §Q4 mandates thiserror-style derives; the
  stack stage picked no error crate. Alternative considered and
  rejected: hand-written `Display`/`Error` impls, about ten lines per
  enum, which buys nothing over the derive.
- `anyhow` — code-prefs §Q4 permits it in `arda-cli` only. The crate
  graph enforces this: no library crate depends on it.
- `blake3` promoted from dev/tooling to a runtime dependency of
  `arda-core` (see its row above).

All four clear code-prefs §Q2's vetting bar (permissive license,
maintained > 1 year, no platform-variant arithmetic). Whole-tree count
remains well inside the ~40-crate soft cap. Still open: none.
