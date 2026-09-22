---
generated_date: 2026-09-22
generated_at_commit: 342d03e55120
content_hash: 25800d0e3de2
paths_covered: [":(top)crates/arda-cli/src/**", ":(top)crates/arda/src/**", ":(top)crates/arda-gen/src/orchestrator.rs", ":(top)crates/arda-gen/src/orchestrator/**", ":(top)crates/arda-core/src/config.rs", ":(top)crates/arda-core/src/formats/manifest.rs", ":(top)crates/arda-render/src/**"]
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08, features/2026-09-22-geographical-rendering-first-pass@2026-09-22
---

# 07 — Preview a seed

Descriptive extraction of the implemented `Preview` command; owns the composed CLI behavior with generation, export and query behavior in the corresponding scenario chapters and `../04-data-flow.md`. The block chapter also retains the deferred richer tactical design.

## Trigger & preconditions

A local user runs `arda preview --seed <u64> --out <path>`. There is no application identity or authorization layer; filesystem permissions govern access (`crates/arda-cli/src/main.rs`). Seed and out are required; size defaults to 500x1000 km, micro defaults false, and overview quality defaults to 8192 (8K) on the long edge. `--quality` accepts any integer 512–32768 pixels or an integer k/K suffix, where 1K is 1024. `--style classic|atlas` selects PNG presentation; omission remains Classic. Legacy `--px` is optional (1–512 pixels per area) and conflicts with explicit quality or any explicit style, including Classic (`crates/arda-cli/src/main.rs:51`). Generation requires an empty `out/world` directory (`crates/arda-gen/src/orchestrator.rs`).

## Steps

1. Choose MICRO (102×204 km) when `--micro` is set; otherwise parse lowercase-x-separated width/height and validate `GenerateConfig` with latitude 35–55°N and density 15. MICRO bypasses parsing the supplied size (`crates/arda-cli/src/main.rs`, `crates/arda-cli/src/main.rs`).
2. Create `out/world`, print seed, configured dimensions and area count (`crates/arda-cli/src/main.rs`). Config uses integer axis/51 counts; default is 9×19 areas (`crates/arda-core/src/config.rs`).
3. Run the full generation pipeline and print generated area count and land permille. This includes canonical terrain preparation, shared annual water calculation, immutable area composition and sampled tactical generation (`crates/arda-cli/src/main.rs`, `crates/arda-gen/src/orchestrator.rs`).
4. Load and validate the format-4 manifest and create empty per-area cache slots. Export the area-derived overview by reading owned areas as needed; this does not populate the persistent area cache or load block archives. The quality renderer can reread an area when it intersects several 256-row bands and receives cells after the facade drops the owned area's objects; the Atlas callback also supplies derived AtlasTerrain context. Atlas additionally reads up to eight saved neighbors per area in fixed order, copying a two-cell halo and dropping each neighbor before the next; missing or corrupt in-bounds neighbors fail. Atlas derives fixed-size palette/light/class samples and filters interpolated output pixels by their saved terrain class. The overview retains its categorical lake threshold and discharge-band rivers. By default, or with `--quality`, the PNG uses the selected long edge and rounds the shorter edge to the nearest pixel from the saved area-grid aspect ratio; bounded bands allow square 32K output. Explicit legacy `--px` instead produces areas_wide×px by areas_high×px through the buffered Classic renderer. Raster validation rejects invalid dimensions before area reads (`crates/arda/src/export_quality.rs:107`, `crates/arda/src/atlas.rs:35`, `crates/arda-render/src/overview/streaming.rs:73`).
5. In the quality path, stream into an exclusive temporary sibling, flush/close it and rename it to `out/overview.png` on success. Explicit `--px` retains the completed-buffer write. Print the PNG path and return success (`crates/arda-cli/src/main.rs`, `crates/arda/src/export_quality.rs`, `crates/arda/src/lib.rs`).

## Branches

`--micro` selects fixed config; otherwise invalid size/latitude/density is a typed constructor error. The generator tries at most five continent candidates with land fraction 250–900‰ and a sea-reaching major river; no accepted candidate yields GenError::Validation (`crates/arda-gen/src/orchestrator.rs`). Preview has no export-format selector: the artifact is PNG (`crates/arda-cli/src/main.rs`).

## Unhappy paths

Clap rejects malformed integer arguments, unknown styles, quality outside 512–32768, legacy px outside 1–512 and explicit `--px` with `--quality` or `--style` before generation (`crates/arda-cli/src/main.rs:51`). Config refusal occurs before directory creation; mkdir, generation, load, render and write errors propagate to main with no outer retry or compensation. Atlas also fails on unavailable in-bounds neighbors or contradictory halo context (`crates/arda/src/atlas.rs:43`, `crates/arda-render/src/atlas.rs:176`). Aggregate raster validation for the legacy buffered path still occurs after generation and manifest loading; its pixel cap can leave a complete world with no PNG. Mid-generation failure can leave a partial world; repeating the command against an occupied `out/world` is refused. An ordinary quality-path render/write failure preserves any previous completed overview and attempts to remove its temporary sibling; abrupt interruption may leave the temporary file. A failed legacy `--px` buffer write can leave truncated destination bytes. A create-new transaction marker reserves an empty target against cooperating generators. Interruption has no resume handler (`crates/arda-gen/src/orchestrator.rs`, `crates/arda/src/lib.rs`).

## State transitions

Empty target → reserved partial world → complete layers and global tables → final manifest rename → overview artifact. Generation stages the manifest, drops scratch owners and removes scratch before its final same-directory rename. This supplies process-interruption completion semantics; neither generation nor quality publication promises power-loss durability. PNG failure does not revoke a completed world (`crates/arda-gen/src/orchestrator/publication.rs`, `crates/arda-cli/src/main.rs`).

## Invariants

On successful return, generation, manifest validation and every requested overview area read succeeded; Atlas also read all required in-bounds neighbors. Preview wrote overview bytes without mutating generated layers or using clock-dependent rendering input (`crates/arda-cli/src/main.rs:209`, `crates/arda/src/export_quality.rs:107`). Style and image quality do not change the generated 100 m terrain. It provides no all-or-nothing guarantee over world plus PNG and no cache/deduplication service for repeated invocations.

## Outcomes & side effects

The caller sees seed/dimensions, generated counts, and the PNG path on stdout; failures return nonzero through anyhow. The filesystem holds the world and PNG indefinitely until managed externally; no separate audit journal, retention cleanup, notifications, financial effects or external-service calls are implemented (`crates/arda-cli/src/main.rs`). The manifest records seed/config/build and counts for reproduction (`crates/arda-gen/src/orchestrator.rs`).

## Dimensions not in play

- Authority roles: no application identity or delegated permissions; local filesystem access only (`crates/arda-cli/src/main.rs`).
- Money: no charge, credit or budget ledger in this command (`crates/arda-cli/src/main.rs`).
- Time: no scheduled execution, expiry or timezone calculation (`crates/arda-cli/src/main.rs`).
- External notifications and cascades: stdout and filesystem writes only; no other actor's application state (`crates/arda-cli/src/main.rs`).
