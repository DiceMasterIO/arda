---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 620a8e17cf7a
paths_covered:
  - ":(top)crates/arda-cli/src/main.rs"
  - ":(top)crates/arda-cli/src/generate.rs"
  - ":(top)crates/arda/src/lib.rs"
  - ":(top)crates/arda/src/export_quality.rs"
  - ":(top)crates/arda/src/atlas.rs"
  - ":(top)crates/arda/src/world.rs"
  - ":(top)crates/arda-render/src/quality.rs"
  - ":(top)crates/arda-render/src/overview.rs"
  - ":(top)crates/arda-core/src/config.rs"
  - ":(top)crates/arda-gen/src/orchestrator.rs"
  - ":(top)crates/arda-gen/src/orchestrator/publication.rs"
  - ":(top)crates/arda-midzoom/src/export.rs"
---

# 07 — Preview a seed

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. Owns the `arda preview` subcommand: argument definition `crates/arda-cli/src/main.rs:70-99`, dispatch `:428-437`, implementation `run_preview` `:252-302`. Generation itself is owned by [01](01-continent-generation.md) and [02](02-area-generation.md); overview rendering and styles by [04](04-export.md); world loading by [05](05-load-query.md).

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/07 §<rule>`.

## Trigger & preconditions

- Trigger: a local user runs `arda preview --seed <u64> --out <dir> [--size WxH] [--micro] [--px N | --quality Q] [--style classic|atlas|atlas-oblique] [--society auto|off]` (`crates/arda-cli/src/main.rs:73-99`).
- Authority: none; filesystem permissions govern access. There is no application identity layer in `crates/arda-cli/src/`.
- Preconditions: `<out>/world` is absent or an empty directory; generation refuses an occupied root (`crates/arda-gen/src/orchestrator/publication.rs:58-64`). Generation's resource admission happens before any output is created (`crates/arda-gen/src/orchestrator.rs:174-179`).

## Rules

### §prev-args

| Flag | Default | Accepted | Site |
|---|---|---|---|
| `--seed` | required | `u64` | `crates/arda-cli/src/main.rs:74-76` |
| `--size` | `500x1000` | `WxH` km, split on lowercase `x`, each a number | `crates/arda-cli/src/main.rs:77-79`, `crates/arda-cli/src/generate.rs:11-19` |
| `--micro` | false | flag; replaces `--size` | `crates/arda-cli/src/main.rs:80-82` |
| `--px` | none | 1–512 pixels per area; conflicts with `--quality` and `--style` | `crates/arda-cli/src/main.rs:83-85`, `:90` |
| `--quality` | 8192 | 512–32768 pixels, or an integer with `k`/`K` (1K = 1024) | `crates/arda-cli/src/main.rs:86-88`, `crates/arda-render/src/quality.rs:9-27`, `:59-82` |
| `--style` | Classic | `classic`, `atlas`, `atlas-oblique` | `crates/arda-cli/src/main.rs:89-91`, `:182-189` |
| `--society` | `auto` | `auto`, `off` (`SocietyArg`; no `on`) | `crates/arda-cli/src/main.rs:92-95`, `:164-171` |
| `--out` | required | directory; holds `world/` and `overview.png` | `crates/arda-cli/src/main.rs:96-98` |

Preview has no `--terrain`, `--recipe`, `--latitude` or resource-ceiling flags; those exist only on `arda generate` (`crates/arda-cli/src/main.rs:36-69`).

### §prev-config

`--micro` selects `GenerateConfig::MICRO` (102 × 204 km, 35–55° N, density 15) without parsing `--size` (`crates/arda-cli/src/main.rs:261-263`, `crates/arda-core/src/config.rs:63-67`). Otherwise `GenerateConfig::new(parse_size(size), LatitudeBand::new(35, 55), 15)` (`crates/arda-cli/src/main.rs:264`), which refuses axes outside 64–4000 km, an inverted or out-of-±80° band, and density outside 1–200 (`crates/arda-core/src/config.rs:10-13`, `:73-111`). Area counts are `size / AREA_SIZE_KM` per axis, truncating (`crates/arda-core/src/config.rs:134-136`).

### §prev-generate

Preview always runs the legacy terrain pipeline `arda::generate` → `arda_gen::generate_world` (`crates/arda-cli/src/main.rs:276`, `crates/arda/src/lib.rs:45-47`), so its manifest carries no fine terrain (`crates/arda-gen/src/orchestrator.rs:429`). The continent passes a land-fraction gate of 250–900 ‰ and a sea-reaching-river gate, with up to `CONTINENT_ATTEMPTS` = 5 deterministic candidates; none accepted is `GenError::Validation` "… after 5 candidate(s)" (`crates/arda-gen/src/orchestrator.rs:66-70`, `:206-264`).

### §prev-render-path

After `World::load` (`crates/arda-cli/src/main.rs:282`):

| Selection | Renderer | Size | Write | Site |
|---|---|---|---|---|
| `--px N` | `export_overview` (Classic buffered raster; no society layer) | `areas_wide·N × areas_high·N`, at most 64,000,000 px | `std::fs::write` to `overview.png` | `crates/arda-cli/src/main.rs:283-284`, `crates/arda/src/lib.rs:201-218`, `crates/arda-render/src/overview.rs:60-75` |
| otherwise | `arda_midzoom::export::export_overview(quality, (style, look), society)`, which delegates to `export_overview_with_overlay` | long edge = quality; short edge `round(quality · short / long)` over the area grid (1–78 areas per axis) | streamed via `publish_png` | `crates/arda-cli/src/main.rs:285-298`, `crates/arda-midzoom/src/export.rs:62-84`, `crates/arda/src/export_quality.rs:178-296`, `crates/arda-render/src/quality.rs:41-56` |

- Style mapping: `classic` → Classic; `atlas` and `atlas-oblique` → Atlas, the latter with `OverviewLook { oblique: true }` (`crates/arda-cli/src/main.rs:196-203`, `:245-249`).
- Atlas renderer choice by the manifest's fine terrain: oblique look, recipe ≥ 5 (formed), recipe ≥ 4, else the plain Atlas renderer (`crates/arda/src/export_quality.rs:192-205`, `:264-292`). Preview worlds have no fine terrain, so `atlas` takes the plain renderer: `AtlasChannelLook::Original` through `write_atlas_overview_png_with_channels_overlay` (`crates/arda/src/export_quality.rs:277-291`, `crates/arda/src/world.rs:137-139`).
- Atlas reads each area's eight saved neighbours for a halo and the optional `shore` layer (`crates/arda/src/atlas.rs:83-101`, `crates/arda/src/export_quality.rs:209-214`, `:225-263`).
- `atlas-oblique` requires a recipe-5 Atlas overview; on a preview world it fails with `RenderError::AtlasContext` "the oblique look needs a recipe-5 Atlas overview" after generation has completed (`crates/arda/src/export_quality.rs:195-201`). Unlike `arda export`, preview does not pre-validate this combination (`crates/arda-cli/src/main.rs:227-229` applies to `export` only).

### §prev-society

`--society auto` (default) paints the society layer onto the quality-path overview only when `<world>/society/settlements.json` exists and the look is not oblique; `off` never paints (`crates/arda-midzoom/src/export.rs:28-47`). A preview world is freshly generated and never contains `society/`, so `auto` writes the plain overview and the flag changes no output on this subcommand; the overlay builder `SocietyBandOverlay::open` is never reached (`crates/arda-midzoom/src/export.rs:68-77`). `SocietyMode::On` exists in the library but `SocietyArg` offers only `auto` and `off` (`crates/arda-cli/src/main.rs:164-171`, `crates/arda-midzoom/src/export.rs:15-22`). `--society` has no `conflicts_with`; with `--px` it is accepted and ignored because `export_overview` takes no overlay (`crates/arda-cli/src/main.rs:283-284`). `export_overview_with_overlay` refuses an oblique look combined with an overlay with `RenderError::AtlasContext` "the oblique look cannot carry an overlay" (`crates/arda/src/export_quality.rs:185-190`).

### §prev-publish

The quality path writes into an exclusive temporary sibling `.overview.png.<pid>-<n>.tmp` (`create_new`), flushes it and renames it onto `overview.png`; a drop guard removes the temporary file on any failure (`crates/arda/src/export_quality.rs:311-348`). The `--px` path writes the finished buffer directly (`crates/arda/src/lib.rs:212-216`).

## Steps

1. Build the configuration (§prev-config); a refusal returns before any directory exists.
2. `create_dir_all(<out>/world)` (`crates/arda-cli/src/main.rs:266-267`).
3. Print `arda <version> — preview (seed …, WxHkm, N areas)` (`crates/arda-cli/src/main.rs:269-275`).
4. Generate the world (§prev-generate): reserve the empty root with a `create_new` marker and a scratch directory (`crates/arda-gen/src/orchestrator/publication.rs:58-77`), run every stage, then commit (`crates/arda-gen/src/orchestrator.rs:444`).
5. Print `generated N areas · land N‰` (`crates/arda-cli/src/main.rs:277-280`).
6. Load the world and render the overview (§prev-render-path, §prev-publish).
7. Print `wrote <path>` and exit 0 (`crates/arda-cli/src/main.rs:300-301`).

## Branches

- `--micro` vs `--size` (§prev-config).
- `--px` vs quality path (§prev-render-path).
- Style: Classic, Atlas, Atlas oblique (§prev-render-path).
- `--society` auto vs off (§prev-society); no effect on a fresh preview world.
- Continent candidate accepted or rerolled, up to 5 (§prev-generate).
- There is no export-format selector: the artifact is always PNG (`crates/arda-cli/src/main.rs:73-99`).

## Unhappy paths

| Failure | When | Effect | Site |
|---|---|---|---|
| malformed integer, unknown style, quality outside 512–32768, `--px` outside 1–512, `--px` with `--quality` or `--style` | argument parsing | clap error, nothing created | `crates/arda-cli/src/main.rs:83-91`, `crates/arda-render/src/quality.rs:65-82` |
| malformed `--size` or config out of range | before mkdir | error, nothing created | `crates/arda-cli/src/generate.rs:11-19`, `crates/arda-core/src/config.rs:73-111` |
| `<out>/world` not empty | generation start | `PublicationError::Occupied`, nothing written into it | `crates/arda-gen/src/orchestrator/publication.rs:58-64` |
| no continent candidate passes | generation | `GenError::Validation`; the reserved root keeps its marker and no manifest | `crates/arda-gen/src/orchestrator.rs:261-264`, `:94-95` |
| stage failure mid-generation | generation | incomplete world without manifest; a rerun needs an empty directory | `crates/arda-gen/src/orchestrator.rs:94-95` |
| `--px` raster above 64,000,000 px | after generation and load | error; complete world, no PNG | `crates/arda-render/src/overview.rs:70-73` |
| `atlas-oblique` | after generation | `AtlasContext` error; complete world, no PNG | `crates/arda/src/export_quality.rs:195-201` |
| missing or corrupt area, neighbour or shore file | render | error; quality path keeps any previous `overview.png` and removes its temporary file | `crates/arda/src/export_quality.rs:225-263`, `:342-348` |
| `--px` write failure | write | truncated `overview.png` possible (direct write) | `crates/arda/src/lib.rs:212-216` |

- Retry: none beyond the five continent candidates; errors propagate to `main` through `anyhow` with nonzero exit (`crates/arda-cli/src/main.rs:399-472`).
- Concurrency: the generation marker is `create_new`, so a cooperating second generator on the same empty root fails (`crates/arda-gen/src/orchestrator/publication.rs:65-70`); temporary PNG names carry pid and a counter, so concurrent renders do not share a temporary file (`crates/arda/src/export_quality.rs:317-323`).
- Interruption: no resume handler; an abrupt stop may leave the marker, scratch or a temporary PNG behind.

## State transitions

Absent `<out>/world` → empty directory → reserved (marker + scratch) → layers written → manifest staged, scratch and marker removed, manifest renamed into place (`crates/arda-gen/src/orchestrator/publication.rs:58-77`, `:122-131`) → `overview.png` published. The commit gives process-interruption completion semantics, not power-loss durability (`crates/arda-gen/src/orchestrator/publication.rs:120-121`). A PNG failure never revokes a committed world.

## Invariants

1. On success, generation committed a manifest, the world loaded, and every area the renderer needed (plus Atlas neighbours) was read without error.
2. Preview never mutates a generated layer after commit: rendering only reads (`crates/arda/src/export_quality.rs:178-296`).
3. Style and quality never change generated terrain: they are applied after `generate` returns (`crates/arda-cli/src/main.rs:276-298`).
4. Same seed and size, same build → same world and overview bytes; no clock or random input reaches rendering (no clock reads in `crates/arda/src/export_quality.rs`).
5. Not guaranteed: atomicity over world plus PNG; a world can exist without its PNG (Unhappy paths).

## Outcomes & side effects

- Success: three stdout lines (Steps 3, 5, 7), `<out>/world/` and `<out>/overview.png`, exit 0.
- Failure: the anyhow error chain on stderr, nonzero exit; partial artifacts as in Unhappy paths.
- The manifest records seed, configuration, build and counts for reproduction (`crates/arda-gen/src/orchestrator.rs:429`, and 01 Outcomes).
- No cache, deduplication, retention cleanup, notification or external call.

## Dimensions not in play

- D1 Authority: one local actor; filesystem permissions only.
- D5 Money: nothing is charged or credited.
- D7 Time: no schedule, expiry or timezone; no clock input to generation or rendering.
- D9 Lifecycle beyond the publication states above: no entity lifecycle.
- D12 Visibility: the caller sees everything written; nothing is hidden.
- D13 Notification: stdout/stderr only.
- D14 Effects on others: no shared state outside `<out>`.
- D15 Record and audit: the manifest is the only record (Outcomes).
