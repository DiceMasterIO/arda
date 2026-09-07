---
generated_date: 2026-09-07
generated_at_commit: 987aeca04c77
content_hash: a91f72ded9de
paths_covered: [":(top)crates/arda-cli/src/**", ":(top)crates/arda/src/**", ":(top)crates/arda-gen/src/orchestrator.rs", ":(top)crates/arda-core/src/config.rs", ":(top)crates/arda-core/src/formats/manifest.rs", ":(top)crates/arda-render/src/carto.rs"]
capstone_version: 6.4
---

# 07 — Preview a seed

Descriptive extraction of the implemented `Preview` command; owns the composed CLI behavior while `01-continent-generation.md`, `02-area-generation.md`, `03-block-generation.md`, `04-export.md`, and `05-load-query.md` retain their design rules. Current subordinate behavior is in `../04-data-flow.md`.

## Trigger & preconditions

A local user runs `arda preview --seed <u64> --out <path>`. There is no application identity or authorization layer; filesystem permissions govern access (`crates/arda-cli/src/main.rs:43`). Seed and out are required; size defaults to 500x1000 km, micro defaults false, px defaults 48. Generation requires an empty `out/world` directory (`crates/arda-cli/src/main.rs:129`, `crates/arda-gen/src/orchestrator.rs:138`).

## Steps

1. Choose MICRO (102×204 km) when `--micro` is set; otherwise parse lowercase-x-separated width/height and validate `GenerateConfig` with latitude 35–55°N and density 15. MICRO bypasses parsing the supplied size (`crates/arda-cli/src/main.rs:90`, `crates/arda-cli/src/main.rs:129`).
2. Create `out/world`, print seed, configured dimensions and area count (`crates/arda-cli/src/main.rs:135`). Config uses integer axis/51 counts; default is 9×19 areas (`crates/arda-core/src/config.rs:133`).
3. Run the full generation pipeline and print generated area count and land permille. This includes area and sampled tactical generation (`crates/arda-cli/src/main.rs:145`, `crates/arda-gen/src/orchestrator.rs:199`).
4. Eagerly load every area and block archive, then render the area-derived overview using px pixels per area. PNG dimensions are areas_wide×px by areas_high×px; the render rejects zero dimensions and negative area dimensions (`crates/arda/src/lib.rs:223`, `crates/arda/src/lib.rs:93`, `crates/arda-render/src/carto.rs:230`).
5. Write `out/overview.png` and print its path; return success (`crates/arda-cli/src/main.rs:152`, `crates/arda/src/lib.rs:104`).

## Branches

`--micro` selects fixed config; otherwise invalid size/latitude/density is a typed constructor error. The generator tries at most five continent candidates with land fraction 250–900‰ and a sea-reaching major river; no accepted candidate yields GenError::Validation (`crates/arda-gen/src/orchestrator.rs:61`). Preview has no export-format selector: the artifact is PNG (`crates/arda-cli/src/main.rs:43`).

## Unhappy paths

Clap rejects malformed integer arguments. Config refusal occurs before directory creation; mkdir, generation, load, render and write errors propagate to main with no outer retry or compensation (`crates/arda-cli/src/main.rs:129`, `crates/arda-cli/src/main.rs:215`). Pixel validation occurs after generation/loading, so failure can leave a complete world with no PNG. Mid-generation failure can leave a partial world; repeating the command against an occupied `out/world` is refused. A failed PNG write can leave truncated bytes. Concurrent invocations at the same path have no lock, and interruption has no cleanup/resume handler (`crates/arda-gen/src/orchestrator.rs:137`, `crates/arda/src/lib.rs:105`).

## State transitions

Empty target → partially written world → manifest-stamped world → overview artifact. Manifest is written last, directly, so an interrupted manifest write can leave malformed JSON. PNG failure does not revoke a completed world (`crates/arda-core/src/formats/manifest.rs:59`, `crates/arda-cli/src/main.rs:145`).

## Invariants

On successful return, generation and eager load both succeeded and overview bytes were written. Preview does not mutate generated layers during export, and uses no clock-dependent rendering input (`crates/arda-cli/src/main.rs:145`, `crates/arda/src/lib.rs:93`). It provides no all-or-nothing guarantee over world plus PNG and no deduplication guarantee for simultaneous writers.

## Outcomes & side effects

The caller sees seed/dimensions, generated counts, and the PNG path on stdout; failures return nonzero through anyhow. The filesystem holds the world and PNG indefinitely until managed externally; no separate audit journal, retention cleanup, notifications, financial effects or external-service calls are implemented (`crates/arda-cli/src/main.rs:129`). The manifest records seed/config/build and counts for reproduction (`crates/arda-gen/src/orchestrator.rs:241`).

## Dimensions not in play

- Authority roles: no application identity or delegated permissions; local filesystem access only (`crates/arda-cli/src/main.rs:43`).
- Money: no charge, credit or budget ledger in this command (`crates/arda-cli/src/main.rs:129`).
- Time: no scheduled execution, expiry or timezone calculation (`crates/arda-cli/src/main.rs:129`).
- External notifications and cascades: stdout and filesystem writes only; no other actor's application state (`crates/arda-cli/src/main.rs:129`).
