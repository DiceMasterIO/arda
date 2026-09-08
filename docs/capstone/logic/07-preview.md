---
generated_date: 2026-09-08
generated_at_commit: b0f93f22b969
content_hash: de2675f24dc3
paths_covered: [":(top)crates/arda-cli/src/**", ":(top)crates/arda/src/**", ":(top)crates/arda-gen/src/orchestrator.rs", ":(top)crates/arda-gen/src/orchestrator/**", ":(top)crates/arda-core/src/config.rs", ":(top)crates/arda-core/src/formats/manifest.rs", ":(top)crates/arda-render/src/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# 07 — Preview a seed

Descriptive extraction of the implemented `Preview` command; owns the composed CLI behavior with generation, export and query behavior in the corresponding scenario chapters and `../04-data-flow.md`. The block chapter also retains the deferred richer tactical design.

## Trigger & preconditions

A local user runs `arda preview --seed <u64> --out <path>`. There is no application identity or authorization layer; filesystem permissions govern access (`crates/arda-cli/src/main.rs`). Seed and out are required; size defaults to 500x1000 km, micro defaults false, px defaults 48. Generation requires an empty `out/world` directory (`crates/arda-cli/src/main.rs`, `crates/arda-gen/src/orchestrator.rs`).

## Steps

1. Choose MICRO (102×204 km) when `--micro` is set; otherwise parse lowercase-x-separated width/height and validate `GenerateConfig` with latitude 35–55°N and density 15. MICRO bypasses parsing the supplied size (`crates/arda-cli/src/main.rs`, `crates/arda-cli/src/main.rs`).
2. Create `out/world`, print seed, configured dimensions and area count (`crates/arda-cli/src/main.rs`). Config uses integer axis/51 counts; default is 9×19 areas (`crates/arda-core/src/config.rs`).
3. Run the full generation pipeline and print generated area count and land permille. This includes canonical terrain preparation, shared annual water calculation, immutable area composition and sampled tactical generation (`crates/arda-cli/src/main.rs`, `crates/arda-gen/src/orchestrator.rs`).
4. Load and validate the format-4 manifest and create empty per-area cache slots. Export the area-derived overview by reading one owned area at a time; this does not populate the persistent area cache or load block archives. PNG dimensions are areas_wide×px by areas_high×px. Raster validation rejects invalid dimensions before area reads (`crates/arda/src/world.rs`, `crates/arda/src/lib.rs`, `crates/arda-render/src/overview.rs`).
5. Write `out/overview.png` and print its path; return success (`crates/arda-cli/src/main.rs`, `crates/arda/src/lib.rs`).

## Branches

`--micro` selects fixed config; otherwise invalid size/latitude/density is a typed constructor error. The generator tries at most five continent candidates with land fraction 250–900‰ and a sea-reaching major river; no accepted candidate yields GenError::Validation (`crates/arda-gen/src/orchestrator.rs`). Preview has no export-format selector: the artifact is PNG (`crates/arda-cli/src/main.rs`).

## Unhappy paths

Clap rejects malformed integer arguments. Config refusal occurs before directory creation; mkdir, generation, load, render and write errors propagate to main with no outer retry or compensation (`crates/arda-cli/src/main.rs`, `crates/arda-cli/src/main.rs`). Pixel validation occurs after generation and manifest loading, so failure can leave a complete world with no PNG. Mid-generation failure can leave a partial world; repeating the command against an occupied `out/world` is refused. A failed PNG write can leave truncated bytes. A create-new transaction marker reserves an empty target against cooperating generators. Interruption has no resume handler (`crates/arda-gen/src/orchestrator.rs`, `crates/arda/src/lib.rs`).

## State transitions

Empty target → reserved partial world → complete layers and global tables → final manifest rename → overview artifact. Generation stages the manifest, drops scratch owners and removes scratch before its final same-directory rename. This supplies process-interruption completion semantics; it is not a power-loss durability promise. PNG failure does not revoke a completed world (`crates/arda-gen/src/orchestrator/publication.rs`, `crates/arda-cli/src/main.rs`).

## Invariants

On successful return, generation, manifest validation and every overview area read succeeded and overview bytes were written. Preview does not mutate generated layers during export, and uses no clock-dependent rendering input (`crates/arda-cli/src/main.rs`, `crates/arda/src/lib.rs`). It provides no all-or-nothing guarantee over world plus PNG and no cache/deduplication service for repeated invocations.

## Outcomes & side effects

The caller sees seed/dimensions, generated counts, and the PNG path on stdout; failures return nonzero through anyhow. The filesystem holds the world and PNG indefinitely until managed externally; no separate audit journal, retention cleanup, notifications, financial effects or external-service calls are implemented (`crates/arda-cli/src/main.rs`). The manifest records seed/config/build and counts for reproduction (`crates/arda-gen/src/orchestrator.rs`).

## Dimensions not in play

- Authority roles: no application identity or delegated permissions; local filesystem access only (`crates/arda-cli/src/main.rs`).
- Money: no charge, credit or budget ledger in this command (`crates/arda-cli/src/main.rs`).
- Time: no scheduled execution, expiry or timezone calculation (`crates/arda-cli/src/main.rs`).
- External notifications and cascades: stdout and filesystem writes only; no other actor's application state (`crates/arda-cli/src/main.rs`).
