---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: b5108f625f42
paths_covered:
  - ":(top)crates/arda-art-import/**"
  - ":(top)crates/arda-cli/src/tactical_import.rs"
  - ":(top)crates/arda-cli/src/tactical.rs"
---

# 19 — Art import: raw AI images to a validated tactical library

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. It owns the entry point `arda tactical import <raw_dir> --out <dir> [--manifest] [--base] [--no-base] [--reference] [--grade-strength] [--contact-sheet]` (`crates/arda-cli/src/tactical_import.rs:41`, wired at `crates/arda-cli/src/tactical.rs:21` and `crates/arda-cli/src/tactical.rs:68`) and the library call `arda_art_import::import` (`crates/arda-art-import/src/lib.rs:97`). The tactical library format and validator are owned by [11](11-tactical-art-compositor.md); this file cites them only where import calls them.

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/19 §<rule>`.

## Trigger & preconditions

- Trigger: `arda tactical import` (`crates/arda-cli/src/tactical.rs:21`), dispatched to `tactical_import::run` (`crates/arda-cli/src/tactical.rs:68`), which builds `ImportOptions` (`crates/arda-art-import/src/lib.rs:64`) and calls `import` (`crates/arda-cli/src/tactical_import.rs:42-51`).
- Authority: one actor, the local CLI user; no roles, grants or checks (`crates/arda-cli/src/tactical_import.rs:41-86`).
- Preconditions checked by code:
  - `raw_dir` must be a readable directory; `read_dir` failure is an `ImportError::Io` (`crates/arda-art-import/src/lib.rs:147-148`).
  - The manifest, when given, must read and parse (`crates/arda-art-import/src/manifest.rs:161-172`).
  - The base (default `assets/tactical/placeholder`, relative to the working directory, `crates/arda-cli/src/tactical_import.rs:25`) must be a library or `top:…:bottom` stack whose catalogues read: `read_stack_catalogs` (`crates/arda-tactical/src/library_stack.rs:192-194`) is called at `crates/arda-art-import/src/lib.rs:102-105`. `--no-base` skips it (`crates/arda-cli/src/tactical_import.rs:46`).
  - `pixels_per_square` must be non-zero (`crates/arda-art-import/src/lib.rs:112-116`).
- Not checked: that `--out` differs from or lies outside `raw_dir`; that `--out` is empty; that `--grade-strength` lies in 0–1 or is finite (`crates/arda-cli/src/tactical_import.rs:35`, clamped later, §imp-grade).

## Rules

### §imp-cli

CLI arguments map one to one onto `ImportOptions` (`crates/arda-cli/src/tactical_import.rs:42-50`):

| Argument | Field | Default |
|---|---|---|
| `raw_dir` (positional) | `raw_dir` | required (`:15`) |
| `--out` | `out_dir` | required (`:18`) |
| `--manifest` | `manifest` | none (`:22`) |
| `--base` | `base` | `assets/tactical/placeholder` (`:25`) |
| `--no-base` | `base = None` | off (`:29`, `:46`) |
| `--reference` | `reference` | none (`:32`) |
| `--grade-strength` | `grade_strength` | manifest's, else 0.5 (`:35`) |
| `--contact-sheet` | `contact_sheet` | none (`:38`) |

Line numbers in the table are in `crates/arda-cli/src/tactical_import.rs`. `--no-base` wins over `--base` (`:46`); clap does not declare them conflicting.

### §imp-scan

`list_images` walks `raw_dir` recursively, sorts each directory's entries by path, descends into directories and keeps files whose extension is `png`, `jpg` or `jpeg` case-insensitively (`crates/arda-art-import/src/lib.rs:146-164`). Unreadable directory entries are dropped silently (`crates/arda-art-import/src/lib.rs:149`). The file's report name is its path relative to `raw_dir` with `/` separators (`crates/arda-art-import/src/lib.rs:184-188`).

### §imp-naming

The asset spec is the manifest `id` of the entry whose `file` equals the file name or the relative path, else the file stem (`crates/arda-art-import/src/lib.rs:194-198`). `naming::parse` (`crates/arda-art-import/src/naming.rs:76-109`) keeps the text before the first `__` (`:77`), splits on `.`, and maps:

| First part | Target | Base id |
|---|---|---|
| `ground` or `water` | `Texture { key }` | `water.<key>` if key starts `water_`, else `ground.<key>` (`:49-52`, `:68-70`) |
| `prop` | `Prop { name }` | `prop.<name>` |
| `veg` | `Vegetation { name }` | `veg.<name>` |
| `effect` | `Effect { name }` | `effect.<name>` (variants are numbered `effect.<name>.<n>` in file order, like textures) |
| `wall` | `Wall { kit, role }`, role in `run, door, window, gate, corner, tee, cross, end, post` | `wall.<kit>.<role>` (`:93-104`, `:119-147`) |

Line numbers are in `crates/arda-art-import/src/naming.rs`. A word must be non-empty `[a-z0-9_]` (`crates/arda-art-import/src/naming.rs:111-115`). Parts after the needed ones are ignored (`ground.water_deep` is `water.water_deep`; `veg.tree_oak.v3` is `veg.tree_oak`) (test `crates/arda-art-import/src/naming.rs:153-177`). Any other first part is an error naming the convention (`crates/arda-art-import/src/naming.rs:105-107`); the file goes to `skipped` (§imp-ids).

### §imp-ids

Per base id, in scan order, a counter assigns (`crates/arda-art-import/src/lib.rs:206-213`):

| Target | k-th file (k from 0) |
|---|---|
| Texture | `<base>.<k>` (`ground.grass.0`, `ground.grass.1`) |
| Other, k = 0 | `<base>` |
| Other, k ≥ 1 | `<base>.alt<k>` |

An unparseable spec becomes `Skipped { file, reason }` (`crates/arda-art-import/src/lib.rs:199-205`), observed for `ComfyUI_00001_.png` (test `crates/arda-art-import/tests/import.rs:210`).

### §imp-manifest

`import.toml` (`crates/arda-art-import/src/manifest.rs:36-46`) has `[library]` (`name`, `version`, `pixels_per_square`, `licence`, `tool`, `model`, `author`, `:51-66`), `[grade]` (`reference`, `palette`, `strength`, `:71-79`) and `[[asset]]` entries (`file`, `id`, `prompt`, `seed`, `tool`, `model`, `licence`, `footprint`, `height_ft`, `cover`, `layer`, `blocks_sight`, `blocks_movement`, `difficult_terrain`, `tags`, `structured`, `rot_free`, `fixed_pose`, `shadow`, `holes`, `:85-133`). Every struct is `deny_unknown_fields` (`:35`, `:50`, `:70`, `:84`); `holes` accepts `keep|clear` (`:136-143`), `shadow` accepts `strip|keep`, default `strip` (`:146-154`). `check` refuses any entry footprint side of 0 or above `MAX_FOOTPRINT_SQUARES` (16, `crates/arda-tactical/src/validate.rs:117`) (`crates/arda-art-import/src/manifest.rs:180-193`; test `:271-285`).

Entry lookup for processing: `for_file(file name)`, then `for_file(relative path)`, then `for_id(base id)` which matches only entries without `file` (`crates/arda-art-import/src/lib.rs:253-256`, `crates/arda-art-import/src/manifest.rs:197-207`). The first matching entry wins (`Iterator::find`). Id remapping (§imp-naming) uses only `file`-keyed entries.

### §imp-vocab

`vocab.rs` mirrors `docs/goal-prompts/vocabulary.md`: `GROUND_KEYS` (35, `crates/arda-art-import/src/vocab.rs:6-42`), `WALL_KITS` (8, `:45-54`), `PROPS` (`:57-130`), `VEGETATION` (`:133-164`), `FUNCTIONS` (`:167-202`). A target whose word is not in its list gets a flag "… is not in docs/goal-prompts/vocabulary.md, so no layout asks for it", with "did you mean `x`?" for the closest known word within Levenshtein distance 3 (ties broken by the lexicographically smaller word, `min` over `(distance, word)`) (`crates/arda-art-import/src/lib.rs:227-243`, `crates/arda-art-import/src/vocab.rs:217-240`). Unknown words are flagged, not rejected (test `crates/arda-art-import/tests/import.rs:201-209`).

### §imp-metadata

The catalogue record starts from the base record standing for the target, else `class_default` (`crates/arda-art-import/src/lib.rs:257-260`). `Base::find` matches textures by `ground` key, walls by kit and role, others by base id, searching base catalogues top first (`crates/arda-art-import/src/meta.rs:42-58`). The id is then overwritten with the planned id (`crates/arda-art-import/src/lib.rs:261`) and manifest overrides applied (`crates/arda-art-import/src/meta.rs:268-305`): scalar fields replace, `tags` replaces wholesale (`:290-292`), and `structured`/`rot_free`/`fixed_pose` add or remove that free tag (`:293-304`).

`class_default` (`crates/arda-art-import/src/meta.rs:78-128`; all records start with biome `temperate`, culture `human`, `:139-143`):

| Target | Defaults |
|---|---|
| Effect | class `Effect`, layer `Floor`, footprint 2×2, 4 rotations, mirror, no biome or culture tags, blocks nothing, no cover, no shadow; free tag `effect:texture` and tileable, or `effect:decal` and not tileable for the names in `vocab::EFFECT_DECALS` (`scorch_mark`). `Base::find` matches an effect by its name (`effect_base`), so a placeholder record is inherited |
| Texture | class `Water`/`Ground` and layer by `water_` prefix, footprint 2×2, rotations 0/90/180/270, mirror, tileable; keys in `STRUCTURED_KEYS` (`cobbles, flagstone, planks, stone_floor, rug, farmland, cliff`) get free tag `structured`, and `cobbles`/`flagstone` footprint 4×4 (`:14-22`, `:81-99`) |
| Wall | class/layer `Wall`, 4 rotations, blocks movement, casts shadow; `drystone`: no sight block, `ThreeQuarters` cover, 4 ft; `hedge`/`wattle` 6 ft; `city_wall` 20 ft; else 10 ft, `Full` cover (`:101-123`) |
| Prop | 4 rotations, mirror; name containing `rug, deck, planks, bridge, grave, drain` → layer `Floor`, nothing else; else blocks movement, `Half` cover, shadow, 3 ft; name containing `boat` → on water, no block, no cover, 1×2 (`:162-181`) |
| Vegetation | free tag `plant`; `tree_*` → `Canopy`, 3×3, `Half`, 30 ft, clearance 1, tag `tree`; `rock/boulder/stone` → blocks, `Half`, 3 ft, tag `rock`; `lily` → on water; `bush/juniper/heather/reed/cattail/fern/grass` → difficult terrain, `Half` cover for bush/juniper else none, 4 ft for bush/juniper/reed else 2 ft (`:183-221`) |

### §imp-provenance

`provenance` composes `"<tool> / <model>"` (entry over `[library]`; model falls back to PNG metadata), `prompt: "…"` (entry, else PNG), `seed N` (entry, else PNG), `curated by <author>`, and `raw file <file>; arda tactical import`, joined by `; ` (`crates/arda-art-import/src/meta.rs:238-265`). With neither tool nor model, the first part is `AI-generated (tool unknown)` and the asset is flagged "generator unknown" (`crates/arda-art-import/src/meta.rs:252`, `crates/arda-art-import/src/lib.rs:293-296`). PNG metadata comes from ComfyUI `prompt` graphs (nodes visited in sorted id order; `seed`/`noise_seed`, `ckpt_name`/`unet_name`, longest `CLIPTextEncode` text) and A1111 `parameters` text (`crates/arda-art-import/src/image_io.rs:118-175`); JPEGs carry none (`crates/arda-art-import/src/image_io.rs:87`). Licence: entry, else `[library].licence`, else the string `unspecified` plus a flag "no licence given" (`crates/arda-art-import/src/lib.rs:272-275`, `:289-292`). Observed: `seed 77` from a manifest id entry, `seed 4242` and `flux1-schnell` from a ComfyUI graph (test `crates/arda-art-import/tests/import.rs:83-87`, `:215-224`).

### §imp-ppsq

Output pixels per square: `[library].pixels_per_square`, else the top base catalogue's, else `DEFAULT_PPSQ` 128 (`crates/arda-art-import/src/lib.rs:60`, `:107-111`). Zero is `ImportError::Options` (`:112-116`). No upper bound.

### §imp-texture

Textures (`crates/arda-art-import/src/pipeline.rs:50-57`, `crates/arda-art-import/src/texture.rs:37-99`), target `footprint × ppsq`:

1. Any transparency is composited over the integer mean colour, alpha set to 255 (`texture.rs:102-120`).
2. Centre-crop to the tile aspect, then Mitchell resize without wrap (`texture.rs:43-60`).
3. Flatten broad luminance (no wrap) (`texture.rs:64`): gain `1 + 0.8·(mean/low − 1)` clamped to 0.6–1.67, blur radius `max(2, min(w,h)/12)` (`texture.rs:22`, `:205-239`).
4. Seam fix: structured → `edge_blend` (keeps layout); else `offset_blend` (half-tile roll, variance-preserving cross-fade) (`texture.rs:65-77`, `:131-200`).
5. Flatten again with wrap (`texture.rs:78-83`).
6. If `seam_ratio` > `SEAM_TARGET` 1.5, a second edge blend (`texture.rs:20`, `:84-89`).
7. Flags: seam ratio still > 2.0 (`:90-92`); structured source with initial seam ratio > 3.0 (`:93-97`).

Structured-ness is the free tag `structured` (`crates/arda-art-import/src/meta.rs:309-311`). Hole policy for textures is `Off` (`crates/arda-art-import/src/holes.rs:65`). Observed: grass variants 256×256 with seam ratio < 2.0, cobbles 512×512 (test `crates/arda-art-import/tests/import.rs:158-184`).

**Effects** (`crates/arda-art-import/src/effect.rs`, 1.0 task 05) are the one class that keeps its alpha end to end: no background matte, no shadow strip, no hole clearing, no haze removal and no opaque flattening. Steps, target `footprint × ppsq`:

1. Alpha source. The manifest key `alpha` is `source` (the file's alpha) or `luminance` (light on black: alpha is the brightest of R, G and B, at most 6 of 255 is fully clear, and the colour is divided by that alpha, so drawing the result over black gives the source back). Without the key, a file with alpha uses it, and an opaque file whose outer 2 % border averages under 30 of 255 is read as luminance (fix "opaque file with a black border"). Any other opaque file is flagged "no transparency" and stays opaque (the validator then rejects it as `effect_alpha`).
2. Centre-crop to the tile aspect and resize with the premultiplied Mitchell filter (`ops::resize`; wrap for textures).
3. Textures: an alpha-aware offset blend (half-tile roll, cross-fade in premultiplied alpha) makes opposite edges meet in colour and opacity; flag if the opacity-weighted seam ratio (`validate::seam_ratio_alpha`) stays over 2.0. Decals: alpha is multiplied by a smooth ramp over the outer 8 % of each side, so a decal never clips at the frame.
4. Flag a fully transparent or fully opaque result.

The importer vocabulary has `EFFECT_TEXTURES` and `EFFECT_DECALS` (`vocab.rs`) and the slots `effect.<name>` for coverage; the damage prop names are in `PROPS`. Tests: `crates/arda-art-import/tests/effects.rs`, and `effect.rs` unit tests.

### §imp-matte

Cut-outs (props, vegetation, walls) without any pixel of alpha < 255 (`has_alpha` false, `crates/arda-art-import/src/image_io.rs:53`) go through `cut_out` (`crates/arda-art-import/src/pipeline.rs:111-116`, `crates/arda-art-import/src/matte.rs:123-155`):

- Background colour = per-channel median of a 2-px border ring; tolerance `clamp(3·MAD + 12, 12, 48)` (`matte.rs:48-69`).
- Flood from border pixels within tolerance; a step is also allowed within `3·tol` of the backdrop and `tol/2` of the pixel it came from (`matte.rs:76-105`).
- If the flood removed fewer than 2 % of pixels (`bg_px·50 < w·h`), the image is returned unchanged with the flag "no uniform background found … not cut out" (`matte.rs:130-140`).
- Then shadow (§imp-shadow), pockets (§imp-holes), then `soft_matte`: pixels within `RIM_PX` 2 of the removed mask, or within `HALO_PX` 8 and within `max(4·tol, 64)` of the nearest removed colour, are unmixed `C = a·F + (1−a)·B`; when `|F−B|² < 300` the pixel stays solid (`matte.rs:21-24`, `:160-204`).

Sources with transparency keep their alpha (fix "kept the source alpha") and only get shadow stripping (`crates/arda-art-import/src/pipeline.rs:117-127`); pockets are not cleared for them (test `crates/arda-art-import/tests/holes.rs:207-237`).

### §imp-shadow

- On a backdrop (`crates/arda-art-import/src/shadow.rs:42-90`): candidates are unremoved pixels with luma 0.2–0.97 of the backdrop's and chromaticity within 0.03 per channel; growth from the removed mask through pixels with neighbour roughness ≤ 0.035·Lb, each step within 0.06·Lb and brightening ≤ 0.03·Lb; regions smaller than `max(64, w·h/500)` dropped (`:55-59`, `:75`, `:81`, `:87`, `:94`). Confident when share ≥ 0.03 of the object and centroid offset ≥ 0.1 object radii (`:89`, `:180`).
- In alpha (`crates/arda-art-import/src/shadow.rs:127-159`): soft (0 < a < 250), dark (luma < 110), grey (saturation < 40) pixels grown from fully transparent ones, at Chebyshev distance ≥ 2 from solid; confident needs share ≥ 0.02, offset ≥ 0.1, and pixels at distance ≥ 3 at least 1 % of solid pixels.
- `apply` (`crates/arda-art-import/src/shadow.rs:220-251`): confident + `strip` → removed, fix "stripped a baked drop shadow (N px, falling DIR)"; confident + `keep` → flag only; not confident with ≥ 64 px and share ≥ 0.015 → flag "possible baked shadow". Direction is a compass label from the offset (`:189-216`).
- After fitting, `residual_flag` flags a south-east rim (within 4 px of the edge) whose dark/grey share exceeds 0.3 and `2·NW + 0.1` (`crates/arda-art-import/src/shadow.rs:256-320`, called at `crates/arda-art-import/src/pipeline.rs:93-95`).

Observed: the synthetic barrel's SE shadow and the bush's alpha shadow are stripped (tests `crates/arda-art-import/tests/import.rs:96-135`, `:137-156`; unit test `crates/arda-art-import/src/matte.rs:237-254`).

### §imp-holes

Enclosed backdrop-coloured pockets (4-connected unremoved regions within `tol` of the backdrop) are judged per `HolePolicy` (`crates/arda-art-import/src/holes.rs:29-42`, `:118-167`):

| Policy | Chosen when | worst | mean | luma spread | share of subject | thin allowance |
|---|---|---|---|---|---|---|
| `Off` | texture, or `holes = "keep"` | — | — | — | never cleared | — |
| `Strict` | props, walls (default) | `min(tol, 6)` | 3.0 | 2.5 | ≤ 0.005 | none |
| `Foliage` | vegetation (default) | `tol` | `max(tol/2, 6)` | 6.0 | ≤ 0.015 | ≤ 4× share if thickness ≤ 3 px |
| `Forced` | `holes = "clear"` | `tol` | `max(tol/2, 6)` | 6.0 | any | — |

Sources: `crates/arda-art-import/src/holes.rs:63-95`, `:201-230`. Regions failing worst/mean/spread are object; passing but too large are kept and flagged "set holes = \"clear\" if they are gaps"; cleared ones produce one fix listing up to 8 pockets (`crates/arda-art-import/src/holes.rs:244-273`). Observed: six leaf gaps cleared under `Foliage`; a large white sheet kept under `Strict`/`Foliage`, cleared under `Forced` (tests `crates/arda-art-import/tests/holes.rs:117-166`, `:184-205`).

### §imp-cleanup

After the matte (`crates/arda-art-import/src/pipeline.rs:60-75`):

- `remove_specks`: 8-connected components of alpha ≥ 128 smaller than 1/50 of the largest are cleared with soft pixels > 2 px from kept solid (`crates/arda-art-import/src/cleanup.rs:22`, `:26-78`).
- `kill_haze`: alpha in 1..240 is zeroed when < 8 or > `FRINGE_PX` 2 from a pixel of alpha ≥ 240; fully transparent pixels become `[0,0,0,0]` (`crates/arda-art-import/src/cleanup.rs:16-20`, `:83-107`). Runs again after fitting as "resampling haze" (`crates/arda-art-import/src/pipeline.rs:88-92`).
- `defringe`: only when the source had alpha; soft pixels are pulled toward the nearest solid colour with weight `1 − (a/255)²` (`crates/arda-art-import/src/cleanup.rs:112-145`).

### §imp-fit

Fit selection (`crates/arda-art-import/src/pipeline.rs:76-81`), in order:

| Condition | Fit | Behavior |
|---|---|---|
| Wall | `fit_canvas` | centre-crop to square, resize to `ppsq × ppsq` regardless of footprint (`crates/arda-art-import/src/fit.rs:236-259`) |
| layer `Floor` | `fit_fill` | crop to bbox, turn 90° if aspect matches better, stretch to the full footprint; flag if aspect mismatch > 1.4 (`fit.rs:189-230`) |
| Vegetation | `fit_vegetation` | square footprint: scale the whole source frame onto the footprint, capped at the fill scale ("shrunk to fit"); flag "tiny" when the object spans < 40 % of the footprint; non-square footprint delegates to `fit_cutout` (`fit.rs:119`, `:129-182`) |
| else | `fit_cutout` | crop to bbox, turn 90° on non-square footprints when that improves the aspect mismatch by more than ×1.25, flag mismatch > 1.6 ("letterboxed"), scale to fill footprint less a margin `max(3, 3 % of min side)`, centre (`fit.rs:40-105`) |

All fits flag upscale > 1.5 (`fit.rs:81-86`, `:167-172`; not `fit_fill`/`fit_canvas`) and return an empty `footprint × ppsq` canvas with flag "nothing left after background removal" when no pixel has alpha > 0 (`fit.rs:44-51`, `:136-143`, `:193-200`). Resampling is Mitchell–Netravali (B = C = 1/3) in premultiplied alpha with 4 px filter padding (`crates/arda-art-import/src/ops.rs:163-180`, `:219-281`, `crates/arda-art-import/src/fit.rs:18`, `:109-115`). Unit tests: `crates/arda-art-import/src/fit.rs:319-393`.

### §imp-wall

Walls are oriented after fitting (`crates/arda-art-import/src/pipeline.rs:85-87`, `:132-151`). Canonical arms: edge roles (`run, door, window, gate`) and `post` E+W, `corner` E+S, `tee` E+S+W, `cross` all, `end` E (`crates/arda-art-import/src/wall.rs:23-33`). `check` (`wall.rs:149-184`): edge roles turn once when vertical band coverage exceeds 1.3× horizontal; `cross` is always canonical; a joint with no distinct arms (reach spread < `max(s/20, 2)`) is canonical; otherwise the clockwise quarter-turn count matching the canonical mask is applied (fix "turned N° clockwise…"), or a `Mismatch` flag. Edge pieces not solid at both west and east outer 4 % are flagged (`wall.rs:122-132`, `pipeline.rs:146-150`). Observed: a corner drawn the wrong way is turned 180° (test `crates/arda-art-import/tests/import.rs:186-196`).

### §imp-structured

After all assets are processed, each structured texture variant is registered against the first variant of its key in scan order (`crates/arda-art-import/src/output.rs:54-89`): high-pass luma NCC, coarse search on a 4-px grid, fine ±3 px search; a roll is accepted only when non-zero and ≥ 0.1 better than none (`crates/arda-art-import/src/structured.rs:99-150`). Different sizes or images under 16 px skip registration (`structured.rs:108-113`). A rolled variant is edge-blended again (`output.rs:68-81`). Correlation after < `MIN_LAYOUT_NCC` 0.35 flags "structured layout differs" (`structured.rs:19`, `output.rs:82-87`). Observed: `ground.cobbles.1` rolled into register, no flags (test `crates/arda-art-import/tests/import.rs:174-183`).

### §imp-grade

`grade_all` (`crates/arda-art-import/src/output.rs:95-153`):

- Target: `--reference`, else `[grade].reference` (used as given if it exists, else resolved against the manifest's directory), else `[grade].palette`; none → no grade (`output.rs:101-130`).
- Strength: `--grade-strength`, else `[grade].strength`, else `DEFAULT_STRENGTH` 0.5 (`output.rs:24`, `:131-134`), clamped to 0–1 (`crates/arda-art-import/src/grade.rs:96`).
- One affine transform in opponent space (`Y=(r+g+b)/3`, `A=r−g`, `B=(r+g)/2−b`) from the alpha-weighted statistics of all processed images (rejected-to-be included, since validation runs later) toward the target; spread ratio clamped 0.6–1.6 and only used when source std > 1 (`grade.rs:95-110`); fully transparent pixels untouched (`grade.rs:113-123`).
- No pixels with weight → no grade (`output.rs:139-141`). The report string names the target, strength and library mean before/after (`output.rs:149-152`).

### §imp-validate

Three validations, all with `Thresholds::default()`:

1. **Per asset** (`crates/arda-art-import/src/output.rs:183-198`): a one-asset catalogue with the import's vocabulary and ppsq is run through `arda_tactical::validate::validate`; `Rule::WallKit` issues are dropped. Any issue → `Status::Rejected`, image written to `rejected/<id>.png`, issues appended as flags "rejected by the validator: [rule] message", and the asset is left out of the catalogue (`output.rs:223-240`).
2. **Library** (`output.rs:261-280`): `library::check_dir(out)` (`crates/arda-tactical/src/library.rs:89-93`) on the written directory. `WallKit` issues go to `partial_kits` with "(the base library supplies them)" when any base asset has that kit, else to `validation`; all other issues go to `validation`.
3. **Stack** (`output.rs:281-283`, `:305-314`): only with a base; `Library::load_stack("<out>:<base>")` (`crates/arda-tactical/src/library_stack.rs:95`); `TacticalError::Invalid` issues or any other error string become `stack_validation`.

Vocabulary for validation is the union of base catalogue vocabularies (first-seen order); with no base, a built-in set: biomes `temperate, boreal, alpine, wetland`, culture `human`, wealth `poor, modest, wealthy`, functions `vocab::FUNCTIONS` (`output.rs:155-180`). Observed: the synthetic set validates alone and stacked, with the one-piece stone kit listed as partial (test `crates/arda-art-import/tests/import.rs:67-94`).

### §imp-coverage

Every vocabulary slot (`ground:<key>`, `wall:<kit>`, `prop.<name>`, `veg.<name>`, `crates/arda-art-import/src/vocab.rs:207-213`) is classified `imported` if the written catalogue covers it, else `fallback` if the base covers it, else `missing` (`crates/arda-art-import/src/output.rs:316-328`). Coverage matches ground keys and wall kits by field and props/vegetation by exact id (`crates/arda-art-import/src/meta.rs:62-73`), so an `.alt<k>` id alone does not cover its slot.

### §imp-output

Files under `--out` (`crates/arda-art-import/src/output.rs:209-303`):

| File | Content | Source |
|---|---|---|
| `ground/`, `walls/`, `props/`, `vegetation/` `<id>.png` | imported images (`asset.image`) | `crates/arda-art-import/src/lib.rs:276`, `output.rs:43-50`, `:226` |
| `rejected/<id>.png` | rejected images | `output.rs:230-233` |
| `catalog.json` | `FORMAT_VERSION`, library `[library].name` or `imported`, version or `0.1.0`, ppsq, vocabulary, imported assets in scan order | `output.rs:219-220`, `:250-260` |
| `report.md` | Markdown report (§imp-report) | `output.rs:285-286` |
| `report.json` | the `Report` serialised | `output.rs:287-289` |
| `--contact-sheet` path | contact sheet PNG | `output.rs:290-301` |

PNG writes create parent directories (`crates/arda-tactical/src/raster.rs:290-292`).

### §imp-report

`Report` (`crates/arda-art-import/src/report.rs:58-76`) holds every processed asset (id, file, class, status, fixes, flags), skipped files, grade text, `validation`, `partial_kits`, `stack_validation` and `coverage`. `flagged()` counts assets with at least one flag, rejected ones included (`report.rs:81-83`). `valid()` is `validation` empty and `stack_validation` absent or empty (`report.rs:87-89`); rejections, flags, skips and partial kits do not affect it. `to_markdown` writes a summary line with "Validator: pass|FAIL", grade, "Needs review", "Skipped files", "Validation", "Vocabulary coverage" and "Fixes applied" sections (`report.rs:93-168`).

### §imp-contact

The contact sheet lays out every processed asset (imported and rejected, in scan order) as a 192 px thumbnail, 8 columns, 10 px padding, 3 px frame: red rejected, amber flagged, green clean (`crates/arda-art-import/src/contact.rs:14-19`, `:32-38`, `:104-138`). Textures are tiled 2×2, cut-outs drawn on a 12 px checkerboard (`contact.rs:40-63`); the id is drawn in at most two label lines (`contact.rs:87-100`).

### §imp-exit

After `import` returns, the CLI prints to stdout: `<out>: N imported, N rejected, N skipped, N flagged for review`, `coverage: …`, `contact sheet: <path>` when given, `report: <out>/report.md` (`crates/arda-cli/src/tactical_import.rs:52-74`). If `!report.valid()`, each `validation` then `stack_validation` line goes to stderr and the command fails with "the imported library failed validation" (`tactical_import.rs:75-84`). An `ImportError` from `import` fails with context "importing <raw_dir>" (`tactical_import.rs:51`). `main` returns `anyhow::Result` (`crates/arda-cli/src/main.rs:399-400`), so a failure exits non-zero with the error on stderr; success exits 0.

### §imp-determinism

Files are processed in sorted order; per-asset work runs in parallel with rayon, and results are zipped back in plan order (`crates/arda-art-import/src/lib.rs:119-132`). Raster maths is single-threaded `f32`/integer (`crates/arda-art-import/src/ops.rs:1-5`); ComfyUI graph nodes are visited in sorted order (`crates/arda-art-import/src/image_io.rs:146-147`). Observed: two runs give byte-identical library files and contact sheets (test `crates/arda-art-import/tests/import.rs:233-254`; `crates/arda-art-import/tests/holes.rs:256-269`).

## Steps

1. Parse CLI args into `ImportOptions` (§imp-cli).
2. Read the manifest or use `Manifest::default()` (`crates/arda-art-import/src/lib.rs:98-101`, §imp-manifest).
3. Read base catalogues unless `--no-base` (`lib.rs:102-106`).
4. Resolve ppsq (§imp-ppsq).
5. Plan: list images, map names, assign ids, attach vocabulary flags; unmapped files → `skipped` (§imp-scan, §imp-naming, §imp-ids, §imp-vocab).
6. Per planned file in parallel (`lib.rs:119-122`, `:245-298`): decode, build metadata and provenance, then `pipeline::process`: texture path (§imp-texture) or cut-out path: matte/shadow/holes (§imp-matte, §imp-shadow, §imp-holes), specks, haze, defringe (§imp-cleanup), fit (§imp-fit), wall orientation (§imp-wall), late haze, residual-shadow flag, and the flag "no transparent pixels: the background was not removed" when every pixel is opaque (`crates/arda-art-import/src/pipeline.rs:96-99`). Decode errors → `skipped` (`lib.rs:124-132`).
7. Register structured variants (§imp-structured).
8. Grade (§imp-grade).
9. Create `--out`, validate each asset, write images and `catalog.json` (§imp-validate 1, §imp-output).
10. Validate the written library and the stack; compute coverage (§imp-validate 2–3, §imp-coverage).
11. Write `report.md`, `report.json`, the optional contact sheet (§imp-report, §imp-contact).
12. Print summary; fail when invalid (§imp-exit).

## Branches

| Branch | Behavior | Pointer |
|---|---|---|
| `--no-base` | no base metadata (class defaults only), built-in vocabulary, no stack validation, every partial kit is a `validation` issue (base covers nothing) | `crates/arda-cli/src/tactical_import.rs:46`, `crates/arda-art-import/src/output.rs:172-178`, `:273`, `:281` |
| Manifest absent | defaults: name `imported`, version `0.1.0`, licence `unspecified` + flag, tool unknown + flag, no grade | `crates/arda-art-import/src/lib.rs:100`, `output.rs:219-220`, `lib.rs:289-296` |
| Source has transparency | keep alpha, alpha-shadow only, defringe, no pockets | `crates/arda-art-import/src/pipeline.rs:69-75`, `:117-127` |
| JPEG | always `has_alpha` false, no PNG metadata; formats other than RGB24/L8 are a decode error | `crates/arda-art-import/src/image_io.rs:62-89` |
| Non-`png`/`jpg` extension inside planning | not reached: listing filters extensions; `read` treats every non-JPEG extension as PNG | `crates/arda-art-import/src/lib.rs:155-158`, `image_io.rs:38-41` |
| Flood finds < 2 % background | image kept as-is, flagged; later "no transparent pixels" flag | `crates/arda-art-import/src/matte.rs:130-140`, `pipeline.rs:96-99` |
| `shadow = "keep"` | confident shadow flagged, not removed | `crates/arda-art-import/src/shadow.rs:241-244` |
| Grade target absent | `grade` is `None`, images untouched | `crates/arda-art-import/src/output.rs:128-130` |
| Strength 0 | grade computed and reported, transform is identity | `crates/arda-art-import/src/grade.rs:106-107` |
| Wall kit only partly imported | `partial_kits` if the base has that kit, else validation failure | `crates/arda-art-import/src/output.rs:271-280` |
| Second take of one cut-out | id `<base>.alt<k>`, same base record | `crates/arda-art-import/src/lib.rs:210-211` |

## Unhappy paths

| Case | Outcome | Pointer |
|---|---|---|
| `raw_dir` missing or unreadable | `ImportError::Io`, nothing written | `crates/arda-art-import/src/lib.rs:147-148` |
| Unreadable entry inside `raw_dir` | dropped silently, not reported | `lib.rs:149` |
| Symlinked directory loop | not guarded: `is_dir` follows links and recursion has no visited set | `lib.rs:153-154` |
| Manifest unreadable, bad TOML, unknown key, bad enum value | `ImportError::Io` / `ImportError::Manifest`, nothing written | `crates/arda-art-import/src/manifest.rs:161-172`; test `:266-268` |
| Manifest footprint 0 or > 16 | `ImportError::Manifest` naming the entry | `manifest.rs:180-193` |
| Base missing or catalogue unreadable | `ImportError::Tactical`, nothing written | `lib.rs:102-105` |
| `pixels_per_square = 0` | `ImportError::Options` | `lib.rs:112-116` |
| File name off-convention, unmapped | `skipped` with reason, run continues | `lib.rs:199-205` |
| Image fails to decode | `skipped` with the error text | `lib.rs:124-132` |
| Unknown vocabulary word | flagged, imported | `lib.rs:227-243` |
| Reference image unreadable | `ImportError` after processing, before any write | `crates/arda-art-import/src/output.rs:111` |
| Malformed palette colour | `ImportError::Options("bad palette colour …")` before any write | `output.rs:116-123` |
| `--grade-strength` outside 0–1 | clamped, no error | `crates/arda-art-import/src/grade.rs:96` |
| `--grade-strength` NaN | not validated; `f32::clamp` returns NaN unchanged | `grade.rs:96` |
| Asset fails per-asset validation | `Rejected`, image under `rejected/`, flags; import still succeeds if library validates | `output.rs:229-240` |
| Library or stack validation issue | report written; CLI prints issues to stderr and exits non-zero | `output.rs:261-283`, `crates/arda-cli/src/tactical_import.rs:75-84` |
| Unwritable `--out` or contact-sheet path | `ImportError::Io`/`Tactical`; files written before the failure remain | `output.rs:216`, `:226`, `:260`, `:286`, `:289`, `:300` |
| Empty raw set (no images) | catalogue with zero assets written and validated; no import-side check | `lib.rs:174-224`, `output.rs:250-261` |
| Very large source or ppsq | no size limit on decoded images or `footprint × ppsq` | `lib.rs:107-116`, `crates/arda-art-import/src/image_io.rs:51-89` |
| `--out` inside `raw_dir` | not guarded; a later run's scan lists written PNGs, whose stems (`prop.barrel`) parse as specs | `lib.rs:146-164` |
| Stale files from an earlier run in `--out` | not removed; old images and `rejected/` files remain beside the new catalogue | `output.rs:209-303` |
| Concurrent imports to one `--out` | no lock; writes interleave | `output.rs:209-303` |
| Retry of any step | not implemented in `crates/arda-art-import/src/` | — |

## State transitions

No entity lifecycle. The only state is the filesystem under `--out` and the contact-sheet path, written in this order: directories and asset PNGs (imported or `rejected/`), `catalog.json`, `report.md`, `report.json`, contact sheet (`crates/arda-art-import/src/output.rs:216-301`). Writes are direct, not staged or renamed; a failure mid-write leaves the files already written. Raw inputs, the manifest and the base are only read.

## Invariants

1. Every catalogue asset passed per-asset validation; rejected assets never enter `catalog.json` (`crates/arda-art-import/src/output.rs:225-240`).
2. Every processed image is exactly `footprint × ppsq`, or `ppsq × ppsq` for walls, including empty fits (`crates/arda-art-import/src/fit.rs:41`, `:47`, `:133`, `:220`, `:255`; `crates/arda-art-import/src/texture.rs:38`, `:54-60`).
3. Ids are unique within a run: texture variants `.k`, further cut-out takes `.alt<k>`, counted per base id whether the spec came from a file stem or a manifest `id` (`crates/arda-art-import/src/lib.rs:194-213`).
4. Every asset has non-empty provenance and licence (`unspecified` fallback) (`crates/arda-art-import/src/meta.rs:263`, `crates/arda-art-import/src/lib.rs:275`).
5. Report order equals plan order; the contact sheet zips processed assets with report assets in that order (`crates/arda-art-import/src/output.rs:223-249`, `:291-299`).
6. Same inputs give byte-identical outputs (§imp-determinism).
7. The CLI exits zero only when `report.valid()` (`crates/arda-cli/src/tactical_import.rs:75-85`).
8. Not held on failure paths: atomicity of the output directory (State transitions).

## Outcomes & side effects

- Success: a library directory (§imp-output) that validates alone and, with a base, stacked over it; stdout summary; exit 0 (§imp-exit).
- Validation failure: the same files, `report.md` "Validator: FAIL", issues on stderr, non-zero exit.
- Import error: no files when the error precedes `output::write` (manifest, base, ppsq, scan, grade target); partial files otherwise.
- Visibility: everything computed is disclosed in `report.md`/`report.json` (fixes, flags, skips, issues, coverage); the console shows counts and validation issues only, not per-asset flags or the `report.json` path (`crates/arda-cli/src/tactical_import.rs:57-84`).
- Record: provenance and licence per asset in `catalog.json` (§imp-provenance); per-run fixes and flags in the reports. Earlier runs' reports are overwritten (`std::fs::write`, `crates/arda-art-import/src/output.rs:286`, `:288`).
- Effects on others: the base library is read, never written; render or validate runs that stack the output over the base (11) see the new assets.

## Dimensions not in play

- D1 Authority: single local CLI actor; no roles or grants (`crates/arda-cli/src/tactical_import.rs:41`).
- D5 Money: nothing is charged or credited.
- D7 Time: no deadlines, expiry or clock reads; outputs carry no timestamps (`crates/arda-art-import/src/output.rs:250-257`, §imp-determinism).
- D9 Lifecycle: no entity states; only files (State transitions).
- D11 Termination (cancellation/abandonment): no cancellation handling; an interrupted run leaves whatever files were already written (State transitions).
- D13 Notification: nobody is notified; output is stdout/stderr and files only (`crates/arda-cli/src/tactical_import.rs:57-84`).
- D14 Effects on others: inapplicable beyond reading the base; no shared state, queues or other users.
