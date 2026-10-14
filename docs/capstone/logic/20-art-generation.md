---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 8916834f2f9a
paths_covered:
  - ":(top)tools/art-gen/*.mjs"
  - ":(top)tools/art-gen/checklist.csv"
  - ":(top)tools/art-gen/prompts.json"
  - ":(top)tools/art-gen/test/**"
---

# 20 — Tactical art generation (art-gen prompt pack and Slopify driver)

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. It owns two entry points: `node tools/art-gen/build-prompts.mjs [--check]`, which builds `tools/art-gen/prompts.json` from `checklist.csv` and `expansion.mjs`, and `node tools/art-gen/generate.mjs [options]`, which selects assets from `prompts.json`, submits them to a local Slopify server, waits, downloads the images and writes an `import.toml` manifest. Shared helpers live in `tools/art-gen/lib.mjs`. The output folder is consumed by `arda tactical import` ([19](19-art-import.md)). `README.md` and `plan.md` state intent; every fact below is cited from the `.mjs` source or the tests.

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/20 §<rule>`.

## Trigger & preconditions

| Entry point | Invocation | Preconditions read by the code |
|---|---|---|
| `build-prompts.mjs` | `node tools/art-gen/build-prompts.mjs` writes; `--check` compares only (`tools/art-gen/build-prompts.mjs:1221`) | `tools/art-gen/checklist.csv` (`build-prompts.mjs:22`, read at `:1156`), `expansion.mjs` exports `entries`, `groundTargets`, `newGround`, `newKits`, `wallVariants` (`build-prompts.mjs:18`), and `assets/tactical/placeholder/catalog.json` for catalogue metadata (`build-prompts.mjs:26-27`). |
| `generate.mjs` | `node tools/art-gen/generate.mjs …`; runs `main` only when invoked directly (`tools/art-gen/generate.mjs:944-956`); `main` is also exported for tests (`generate.mjs:621`). | `prompts.json` at `--prompts` (default beside the script, `generate.mjs:172`), loaded with `JSON.parse` (`generate.mjs:245-247`). With `--yes`: a Slopify server reachable at `--server` (default `SLOPIFY_URL` or `http://127.0.0.1:6969`, `generate.mjs:22`), a channel named `--channel` (default `DiceMaster Assets`, `generate.mjs:157`), the provider listed with the model (`generate.mjs:508-523`). |

- D1 authority: one actor, the local operator. `generate.mjs` sends no credentials: requests carry only a `content-type` header (`generate.mjs:449-452`). Provider keys live in Slopify; the script only reads readiness `hasKey` (`generate.mjs:518-521`). Who may call Slopify is Slopify's concern; not inventoried here.
- Runtime: Node standard library only (`generate.mjs:2-3`, `lib.mjs:1`).

## Rules

### §gen-pack

`build-prompts.mjs` assembles the pack in this order (`build-prompts.mjs:1155-1178`):

1. Every row of `checklist.csv` through `describe` (`build-prompts.mjs:882-934`), tier `0` (`:905`); a duplicate id throws (`:1159`). The checklist holds 225 rows (asserted, `tools/art-gen/test/art-gen.test.mjs:33-43`).
2. `groundExpansion` (`:941-968`): for each `groundTargets` key `[target, tier]`, variants from the checklist's count up to `target`; a key absent from the checklist throws (`:946`). Structured keys get `derive_from: <prefix>.<key>.0` (`:962`).
3. `newGroundExpansion` (`:971-1000`): every variant of each `newGround` key, footprint `[2, 2]`, format `1:1`, `structured: false`.
4. `wallExpansion` (`:1024-1080`): every role of each `newKits` kit in `roleOrder` (`run, door, window, gate, post, corner, tee, cross, end`, `:939`), footprint `[1, 1]`, `layer: "wall"`, `height_ft: 10`, `blocking.wall`; then `wallVariants` alts that share the base's metadata (`:1061-1071`).
5. `cutoutExpansion` (`:1082-1128`): each `expansion.mjs` entry; an alt (`variant_of`) copies `sharedMeta` of its base and must have the same class (`:1089-1091`); a new id expands its short metadata and must name a known `blocking` key (`:1094`).

- Ordering: the 225 first in checklist order, then the expansion sorted by the key `[tier, impact, round, source index]` (`:1173-1178`). `impact` (`:1139-1144`): ground/water 0, canopy vegetation 1, wall `run` 2, frequent props 3 (the 20-id `frequent` set, `:1133-1138`), other vegetation 4, other wall roles 5, other props 6. `round` is the alt number, or for ground the variant index past the original count (`:1147-1151`).
- Duplicates across sources throw (`:1168-1172`; `newGround` keys already described throw at `:222`). In `expansion.mjs`, `fam` requires at least three takes per new id (`tools/art-gen/expansion.mjs:44`) and `alts` rejects empty text (`expansion.mjs:37`).
- Catalogue metadata per entry: `layer`, `height_ft`, `tags {biome, culture, wealth, function, free}`, `blocks_movement`, `blocks_sight`, `difficult_terrain`, `cover`. Originals take it from the placeholder catalogue by id or ground key, `extraMeta` winning (`build-prompts.mjs:826-842`); an id with neither throws `no metadata` (`:841`). `blocking` presets (`:778-788`): `solid, tall, total, post, low, rough, clear, canopy, wall`. The placeholder's `fixed_pose` free tag never carries over (asserted, `art-gen.test.mjs:495-498`).
- `pixels` is `footprint × 128` (`build-prompts.mjs:982, 1098`; asserted for every asset, `art-gen.test.mjs:59`).
- Output document (`build-prompts.mjs:1188-1217`): `format_version: 1`, `about`, `checklist`, `expansion`, `keywords`, `keyword_max_chars: 200`, `style`, `classes {cls: {template, rules}}`, `templates`, `tiers {counts, about}`, `notes {structured, walls, variants, sizes}`, `assets {id: entry}`. Each entry carries `class, template, format, footprint, pixels, layer, height_ft, functions, file, status, tier, tags`, the four blocking fields, `values`, `prompt`, and per kind `ground/structured/variant/derive_from`, `kit/role`, or `variant_of`. At this commit the file holds 1516 assets over 5 templates (`arda-ground`, `arda-water`, `arda-wall`, `arda-prop`, `arda-vegetation`) with tier counts `{0: 225, 1: 415, 2: 467, 3: 318, 4: 34, 5: 57}` (observed in `tools/art-gen/prompts.json`; content not reproduced).
- `--check`: compares the generated text byte for byte to the file on disk and exits 1 with `prompts.json is out of date` when they differ (`build-prompts.mjs:1221-1227`); the test suite runs it (`art-gen.test.mjs:45-47`).
- Determinism: no random source in `build-prompts.mjs`; the per-vegetation size is a 31-multiplier string hash of the id (`build-prompts.mjs:744-748`).

### §gen-keywords

- Each class has one Library image template whose body is `style + subject + {{Shape}} {{Detail}}` plus ` {{Variant}}` for textured classes (ground, water), then the class rules (`build-prompts.mjs:90-99`). Assets carry only short keyword values `Subject`, `Shape`, `Detail` (`Variant` for ground and water) (`build-prompts.mjs:10-12, 876, 931`).
- `finish` (`build-prompts.mjs:859-869`) is applied to every entry: each value must be non-empty, one trimmed line, at most 200 characters, else it joins `problems` and the run exits 1 before writing (`:1180-1183`); a template slot with no value throws (`:866`); `prompt` is the template rendered with the values.
- Slot grammar (`lib.mjs:56-64`): `{{ name }}`, one pass, a name with no value is left as written, "the same slot grammar as Slopify's `slices/admission/substitute.ts`". Rendered prompts equal Slopify's own render (asserted against a built Slopify when present, `art-gen.test.mjs:394-425`, skipped otherwise, `:378-379`).
- `Shape` wording: textures state the frame height in feet (`build-prompts.mjs:756-758`); fences, railings and hurdles get the thin-line wording (`:738-740`); square vegetation gets an irregular outline and one of five sizes (`:743-748`); other cut-outs state proportions from the footprint (`:724-735`).
- `generate.mjs` re-checks every selected value (at most `valueMax = 200`, no newline, `generate.mjs:49, 350-357`) and exits 1 listing the problems before anything is printed or sent (`generate.mjs:675-679`).

### §gen-frame

- `formatOf` (`build-prompts.mjs:762-765`): square footprint → `1:1`; taller than wide → `9:16`; wider than tall → `16:9`. Observed split in `prompts.json`: `1:1` 1161, `16:9` 228, `9:16` 127.
- `tools/art-gen/README.md:41-44` states that Slopify makes only 16:9 or 9:16 images; the code emits `1:1` for every square footprint (`build-prompts.mjs:763`) and the test accepts all three (`art-gen.test.mjs:57`).
- The format is the draft's `format` field (`generate.mjs:299`) and one of the two batch grouping keys (§gen-batch).

### §gen-tiers

- Tiers are integers on each asset (`asset.tier ?? 0` when selecting, `generate.mjs:263`). `tiers.about` in the pack and the header of `expansion.mjs` describe every tier (0–7). Tier 5 (57 thin interior partition images: `timber_partition`, `wattle_partition`, `adobe_partition`) is assigned in `newKits` in `expansion.mjs` and counted in `tiers.counts`. **Tier 6 (1.0 task 05) is effect and damage art**: 37 images, the class `effect` with its own Library prompt `arda-effect` (nine sets in `effectSets`, three takes each, ids `effect.<name>.<n>`, 2 × 2 squares, `layer: floor`, nothing blocking, tags `effect:texture` or `effect:decal`) and the damage props `prop.rubble_heap`, `prop.broken_deck` and `prop.charred_floor` (three or four takes each, tag `damage`). Each effect entry carries `alpha`, `luminance` (the prompt asks for bright light on pure black, for frost, fire, acid, web, fog and water) or `source` (a real alpha channel, for ash, grease and the scorch mark), and `importToml` writes it as `alpha = …` so the importer derives the transparency the same way (logic/19 §imp-metadata). The review prompt `arda-asset-review` rule 3 covers effect backdrops. Dry run: `node tools/art-gen/generate.mjs --tier 6 --parallel 5 --price-per-image 0.10 --max-images 37 --max-spend 12 --dry-run` prints 37 images in 3 batches (27 effect, 7 + 3 prop) and an estimate of 37 × $0.10 = $3.70, worst case 111 images with two reviewer remakes each, $11.10, capped at 37 submissions by `--max-images`. **Tier 7 (1.0 task 17) is kitchen-garden art**: 27 plain cut-outs, `veg.vegetable_bed`, `veg.herb_bed`, `veg.bean_poles`, `prop.cold_frame`, `prop.compost_heap`, `prop.rain_barrel` and `prop.garden_tools` (three or four takes each, tag `garden`, no new ground key), added at the end of `expansion.mjs`. Dry run: `node tools/art-gen/generate.mjs --tier 7 --provider codex --model gpt-6-astra --thinking high --parallel 5 --max-images 27 --resume --dry-run` prints 27 images (vegetation 12, prop 15) at $0 (Codex).

### §gen-select

`select` (`generate.mjs:250-267`) returns assets in `prompts.json` order matching all given filters:

| Flag | Parse | Effect |
|---|---|---|
| `--class LIST` | comma list, trimmed (`generate.mjs:186`) | asset `class` in list; an unknown class throws `UsageError` naming the known ones (`generate.mjs:253-257`) |
| `--tier LIST` | each a non-negative integer, else `UsageError` (`generate.mjs:187-191`) | asset tier in list; an unused tier number selects nothing |
| `--only GLOB` | comma alternatives, `*` any run, `?` one character, anchored (`lib.mjs:75-90`) | id matches any alternative |
| `--limit N` | number ≥ 1, floored (`generate.mjs:193`) | applied last, to the assets still to submit (`generate.mjs:673`) |
| `--resume` | boolean | drops assets whose `.png` or `.jpg` exists in `--out` (`generate.mjs:650-652`) |
| `--overwrite` | boolean | keeps them; files are moved aside after `--yes` (§gen-state) |

Because the 225 come first and the expansion is impact-ordered (§gen-pack), `--tier N --limit K` takes the K highest-impact assets of tier N (`build-prompts.mjs:1130-1132`). Asserted: `--tier` selects whole tiers and rejects non-numbers (`art-gen.test.mjs:230-236`).

### §gen-guard

The money guards implemented, in execution order inside `main`, which delegates to `run` (`generate.mjs:621-631`, `631-735`):

1. `--help` prints usage and returns 0 before anything else (`generate.mjs:634-637`).
2. Licence refusal (§gen-licence) returns 2 (`generate.mjs:638-642`).
3. Value check (§gen-keywords) returns 1.
4. The spend plan (`spendPlan` in `tools/art-gen/safety.mjs:25-52`, §gen-estimate) is computed and its refusals applied, before any HTTP client exists and also under `--dry-run`: `--max-spend` below the worst case, `--max-spend` with an unknown price, or `--yes` with an unknown price and neither `--max-images` nor `--price-per-image` print `REFUSED: …` and return 2 (a dry run still prints the plan and sends nothing). Asserted: zero requests reach the fake server (`tools/art-gen/test/safety.test.mjs`).
5. A summary is always printed to stderr: server, provider/model, image count, batch count, per-class and per-tier counts, in-flight and unconfirmed counts, and the estimate (§gen-estimate), plus one line per planned batch (`generate.mjs:689-718`).
6. `--dry-run`: prints `{templates, payloads}` JSON to stdout, each payload a `POST /api/projects/batch` body with the fixed `requestId` `00000000-0000-4000-8000-000000000000`, logs `Dry run: nothing was sent.` and returns 0 (`generate.mjs:720-731`). It never reads the state file (`generate.mjs:647`) and never constructs the HTTP client. It does read `prompts.json`, stat `--out` for existing images (`generate.mjs:649`) and stat `--reference` (`generate.mjs:643-644`). Asserted with `fetch` replaced by a function that throws (`art-gen.test.mjs:298-343`).
7. Without `--yes`: logs `Nothing sent. Re-run with --yes to submit (this spends money on your provider key).` and returns 0 (`generate.mjs:732-735`). Asserted: zero requests reach a fake server (`tools/art-gen/test/mock-slopify.test.mjs:75-87`).
8. With `--yes` the run proceeds. Every billable submission in this run (new project, re-sent batch, re-posted `failed` asset, re-posted cancelled batch item) first takes from a counter (`guard` in `safety.mjs:63-83`, cap from `submissionCap`, `:54-60`) whose cap is the smaller of `--max-images` and `floor(--max-spend / (price × (1 + remakes)))`. When the next submission would exceed it, the run logs `STOPPED: <cap> reached …`, stops submitting, still collects what was already submitted, writes state and `import.toml`, and returns 3. In batch mode the last new batch is cut to the room left; a batch sent before is all or nothing.

Not implemented: an interactive prompt, a spend ledger across runs (the caps are per run), and any check of the provider account balance. Reviewer remakes happen inside Slopify and cannot be counted at run time; `--max-spend` bounds them only through the worst case in §gen-estimate.

### §gen-estimate

- `estimate(count, options)` (`generate.mjs:359`): per-image price is `--price-per-image` if given, else `knownPrices["<provider>/<model>"]`, else `knownPrices["<provider>/*"]` (`priceOf`, `safety.mjs:11-14`); `total = round(each × count × 100) / 100` (round half up to cents via `Math.round`).
- `spendPlan(assets, options)` (`safety.mjs:25`) returns `{images, worstCaseImages, each, worstCaseCost, refusal?}`. `worstCaseImages = min(images, --max-images) × (1 + remakes)` where `remakes` is `--review-retries` when the reviewer is on in `redo` mode and 0 with `--no-review` or `--review-mode flag`. `--max-spend` defaults to `$ART_GEN_MAX_SPEND`. The summary prints `Worst case: N images × $each = $total`. Codex (`each` 0) never trips the money cap but obeys `--max-images`.
- `knownPrices` (`safety.mjs:6-9`): `google-image/gemini-3.1-flash-image` 0.101, `codex-image/*` 0. Every other pair, including the default `openai-image/gpt-image-2`, prints `Estimated cost: unknown … pass --price-per-image to estimate.`; with `--yes` that is a refusal (exit 2) unless `--max-images` or `--price-per-image` is given, and with `--max-spend` it is a refusal unless `--price-per-image` is given.
- `count` is `limited.length` only. The estimate does not count assets collected from earlier runs, unconfirmed batches re-sent (§gen-batch), pending batch items cancelled and re-posted in parallel mode (§gen-parallel) or the reviewer model's own cost; the run-time counter (§gen-guard 8) does count the re-sent and re-posted submissions, and the worst case counts reviewer remakes (§gen-review).
- `--price-per-image` accepts any number ≥ 0 (`generate.mjs:212`).

### §gen-licence

`licenceProblem` (`generate.mjs:232-243`) refuses, with exit 2 and `REFUSED:` on stderr (`generate.mjs:638-642`): a model id matching the FLUX.1 [dev] family patterns, and any provider not in `imageProviders` = `openai-image, google-image, fal, replicate, codex-image` (`generate.mjs:45`) after aliasing `openai, google, gemini, codex` (`generate.mjs:39-44, 184`). Asserted (`art-gen.test.mjs:240-252`).

### §gen-preflight

With `--yes`, before any image request (`generate.mjs:737-776`):

| Step | Call | Failure |
|---|---|---|
| Health | `GET /api/health` (`:738`) | network error after retries → `SlopifyError` "cannot reach Slopify … Is it running?" (`:456-457`) |
| Channel | `GET /api/channels`, match by `name` (`:743-747`) | empty `--channel` → `UsageError` (`:741`); no such channel → plain `Error` (`:745`) |
| Output dir | `mkdirSync(--out, recursive)` (`:749`) | — |
| Model | skipped for `codex-image` (`:509`); `GET /api/providers/<p>/models` must list the model (`:510-515`); with `--reference`, the model's `keywords` must include `reference` when it has a keyword list (`:516-517`); `GET /api/providers` readiness `keyed` with `hasKey === false` refuses (`:518-521`) | `SlopifyError` |
| Library prompts | §gen-library-prompts (`:766-772`) | `SlopifyError` |
| Reference | `POST /api/staging/images` multipart, only when there is something new to submit (`:774-776, 525-530`) | `SlopifyError` |

Model and prompt checks run only when there are new or unconfirmed items (`generate.mjs:764`).

### §gen-library-prompts

`ensureTemplates` (`generate.mjs:488-506`): for each template named by the run (plus the review prompt `arda-asset-review` when review is on, `:491`), `GET /api/prompts`, then create the missing one with `POST /api/prompts`; an existing one whose body differs is replaced with `PUT /api/prompts/<id>` only under `--update-templates`, otherwise the run stops with a `SlopifyError` (`:497-503`). The review prompt's body is `reviewInstruction` (`generate.mjs:56-68`). Asserted: a first run creates `arda-prop` and `arda-asset-review` (`mock-slopify.test.mjs:28`).

### §gen-batch

- `planBatches(assets, size)` (`generate.mjs:279-291`): groups by `template|format` in first-seen order, then slices each group into chunks of at most `size`. `--batch-size` is 1–50 (`batchMax = 50`, `generate.mjs:48, 210`). Asserted (`art-gen.test.mjs:254-262`).
- `batchBody` (`generate.mjs:296-341`): `{requestId, draft, items}`. `draft.sources` has `images: "generate"` and every other stage `off` (`:301-309`); `draft.images = {provider, model, thinking?}`; `channelId`; `reviews` (§gen-review); `imagePrompts: [{name: template, number: 1}]`; with `--reference`, `reference: {source: "provide", thumbnail: false}` and `provided.reference` = staged id (`:332-335`). Each item is `{title: asset id, values}` (`:339`).
- Used when `--parallel 0`, or whenever unconfirmed batches exist (`generate.mjs:794`). Each send records `state.batches[requestId] = {at, ids, body, confirmed: false}` before the POST (`:889-890`); `POST /api/projects/batch` is retried up to 3 attempts, only on "cannot reach", with 2 s × attempt sleeps (`:892-900`); the reply's queue, sorted by `position`, must have one entry per item (`:901-903`); each asset is recorded `submitted` with its `projectId` and `requestId`, and the batch marked `confirmed` (`:904-920`).
- Unconfirmed batches from an earlier run are re-sent first with their stored body and original `requestId` (`generate.mjs:667-669, 864-865`); the code comment states Slopify answers a repeated `requestId` with the queue it already made (`:665-666, 887-888`), and the fake server does the same (`tools/art-gen/test/fake-slopify.mjs:47`).
- After all sends, each submitted project is fetched and its `title` must equal the asset id, else `SlopifyError` (`generate.mjs:930-934`).

### §gen-parallel

- Default `--parallel 5`, range 0–20 (`generate.mjs:35, 211`). With `parallel > 0` and no unconfirmed batches (`generate.mjs:794`), each asset is posted as its own project with `POST /api/projects` and `projectBody` = the batch draft with `title` and `values` of that asset (`generate.mjs:345-348, 825`); the body carries no `requestId` or idempotency key, so the script keeps its own record (below).
- Record before send: `postProject` (`safety.mjs:117-145`) writes `state.assets[id] = {…, status: "posting", at}` before each POST and replaces it with `submitted` once the reply names the project. On a network error it does not throw: it reads `GET /api/projects` (retried by the client) and adopts the newest project titled `id` with `createdAt ≥ at − 5 s` (logged `ADOPTED`); if the list shows none, the POST never arrived and it posts once more (a second drop is checked the same way, then the error propagates). If the list cannot be read the run stops with exit 1 and the intent stays `posting`. An HTTP refusal restores the previous record and rethrows.
- At the start of every run with `--yes`, assets left `posting` are reconciled the same way before anything is submitted: adopted into the collect set, or released for submission when the list proves no project exists; an unreadable list returns 1 and posts nothing. Asserted: `safety.test.mjs` (drop after create, drop before create, leftover intent adopted, leftover intent released, list unreadable).
- `feed` keeps `pending.size < parallel` (`generate.mjs:817-853`). Each reply must carry a project whose `title` is the asset id (`:845-846`) and, when it reports a `channelId`, the chosen channel (`:847-848`); the asset is recorded `submitted` and the state written after the reply (`:849-850`). Asserted: 5 single posts, 0 batch posts, at most 2 running with `--parallel 2` (`mock-slopify.test.mjs:54-73`).
- In-flight assets from an earlier run that came from a batch (`requestId` set) and are still `pending` are cancelled (`POST /api/projects/<id>/cancel` with `baseRevisionId` and a fresh `idempotencyKey`), marked `failed` with `moved out of the batch queue`, and appended to the work list (`generate.mjs:799-813`); all others are only collected.
- `--dry-run` always prints batch payloads, whatever `--parallel` is (`generate.mjs:721-725`), and the summary line counts batches in both modes (`:691`).

### §gen-review

On by default (`generate.mjs:199-207`): `draft.reviews = {provider, model, thinking?, retries, stages: {images: {mode, prompt: "arda-asset-review"}}}` (`generate.mjs:316-325`). `--reviewer` must be `claude-code` or `codex` (`:119-123`), `--review-mode` `redo` or `flag` (`:124-127`), `--review-retries` 0–5 (`:206`); defaults `claude-code`, `opus`, 2 retries, `redo` (`:32-34, 152`). `--no-review` omits `reviews`. The remake loop itself runs inside Slopify; `generate.mjs` only configures it. Asserted (`art-gen.test.mjs:460-478`).

### §gen-state

- `<out>/.art-gen-state.json` = `{assets: {id: record}, batches: {requestId: {at, ids, body, confirmed}}}` (`generate.mjs:543-549`). Written atomically via `<path>.tmp` + rename (`generate.mjs:550-555`), after every submit, every finished poll, and every batch confirmation.
- Asset record: `projectId, requestId?, template, provider, model, prompt, footprint, structured, status, at` (`status: "posting"` with `at` and no `projectId` is a POST intent not yet answered, §gen-parallel) (`generate.mjs:849, 905-916`), plus `file`, or `error` on failure (`:581-583, 597-603`).
- Existing images in `--out`: without `--resume` or `--overwrite` (and not a dry run) the run stops with exit 1 before contacting Slopify (`generate.mjs:653-659`; asserted, zero new requests, `mock-slopify.test.mjs:38-41`). `--resume` takes precedence when both flags are given (`generate.mjs:650`).
- In-flight: assets with a `projectId` whose status is neither `failed` nor `downloaded` are not re-submitted; they are collected (`generate.mjs:660-664`). `--overwrite` and `--dry-run` disable this (`:662`).
- `--overwrite` moves each existing image to `<out>-replaced/<file>.<Date.now()>` before submitting (`generate.mjs:778-785`).

### §gen-collect

`collect` (`generate.mjs:569-617`) polls each pending project with `GET /api/projects/<id>` every `--poll` seconds (default 10, ≥ 1, `generate.mjs:27, 219, 614`) until it reaches one of `done, partial, failed, canceled` (`:567, 578`). Then:

- No output with `role: "image"` → record `failed`, `error` = the images stage's `failureReason`, else the status (`:580-587`).
- Otherwise downloads `GET /files/<projectId>/image-<meta.index ?? 1>` (`:589-590`) for the first image output only, writes the bytes, sets `status: "downloaded"`, `file`, and `prompt`/`provider`/`model` from the output's meta (falling back to the submitted values), writes state and rewrites `import.toml` (`:591-606`).
- Deadline: `--timeout` minutes (default 360, ≥ 1, `generate.mjs:28, 220`) from the start of `collect` (`:570`); on expiry logs `gave up waiting … run again with --resume` and returns false (`:610-613`).

### §gen-files

- File stem: `rawStem(id)` turns a trailing `.altN` into `__altN` (`lib.mjs:96-98`), so the importer's sorted file order numbers alts like the prompt ids (`lib.mjs:92-95`; asserted, `art-gen.test.mjs:221-228`, `mock-slopify.test.mjs:23`).
- Extension by magic bytes (`generate.mjs:532-538`): `.png`, `.jpg`, `.webp`, else `.bin`. `.webp` and `.bin` are saved with a `WARNING … arda tactical import cannot read; convert it to PNG.` (`generate.mjs:595-596`).
- `downloadedFile` recognises only `.png` and `.jpg` (`generate.mjs:269-275`).

### §gen-manifest

- `<out>/import.toml` is rewritten in full from state after every download and at the end (`generate.mjs:559-565, 605, 856, 937`), listing only records whose `file` exists, sorted by file name (`:560-563`).
- Content (`generate.mjs:379-420`): header comments naming `arda tactical import <this folder> --out assets/tactical/ai --manifest <this file>` (`:382-383`); `[library]` with `name = "arda-ai"`, `version = "0.1.0"`, `licence` (`--licence`, default `LicenseRef-AI-generated`, `generate.mjs:29-31`), `tool = "Slopify"`, `model` only when all records share one model, `author = "tools/art-gen"`; per asset a comment with project id, Library prompt and timestamp, then `[[asset]]` with `file, prompt, tool = "Slopify (<provider>)", model, footprint`, `structured = true` when set, and from the current `prompts.json` entry: `layer`, integer `height_ft`, `cover`, boolean `blocks_sight/blocks_movement/difficult_terrain`, and `tags = {biome, culture, wealth, function, free}` (`:404-416`). Strings are TOML-escaped (`:369-377`). Asserted (`art-gen.test.mjs:264-295`, `mock-slopify.test.mjs:29-36`).
- Hand-off: this folder and manifest are the input of `arda tactical import` ([19](19-art-import.md)).

### §gen-retry

HTTP client (`generate.mjs:444-484`):

- Network errors on `GET` are retried after 2, 5, 10, 20, 30, 60 s (`readRetryDelays`, `generate.mjs:442, 454-461`); `POST`/`PUT` network errors are not retried by the client (`:456`); the `SlopifyError` it throws carries `network: true` (`:457-460`), which `postProject` reads to tell a dropped connection from an HTTP refusal (`safety.mjs:127`).
- A `409` whose body says `is updating` is re-sent after 15 s, at most 40 times per client (the counter lives on the `call` function and is never reset, `generate.mjs:468-471`).
- Any other non-2xx becomes a `SlopifyError` with RFC 7807 `title`, `detail` and field list (`generate.mjs:427-438, 476`).

### §gen-exit

| Code | When |
|---|---|
| 0 | help; dry run; no `--yes`; all collected with no `failed` record (`generate.mjs:636, 730, 734, 860, 941`) |
| 1 | value problems; the project list unreadable while an uncertain POST is being reconciled; existing files without `--resume`/`--overwrite`; timeout; any record in the whole state with `status: "failed"` (`generate.mjs:678, 658, 857-860, 938-941`); any thrown `UsageError`, `SlopifyError` or other error (`generate.mjs:951-954`) |
| 2 | licence refusal (`generate.mjs:641`); spend refusal: `--max-spend` below the worst case or uncheckable, or `--yes` with an unknown price and no `--max-images`/`--price-per-image` (§gen-guard 4) |
| 3 | cap reached: `--max-images` or `--max-spend` stopped the submissions; state and `import.toml` were written and already-submitted projects collected (§gen-guard 8). Takes precedence over 1 |

Thrown errors print the message, prefixed `error: ` unless `UsageError`/`SlopifyError`, plus `Run with --help for usage.` for `UsageError` (`generate.mjs:952-953`). `process.exitCode` is set rather than `process.exit()` so a large dry-run payload finishes flushing (`generate.mjs:946`).

## Steps

`build-prompts.mjs`: load inputs → describe the 225 → expand (ground, new ground, walls, cut-outs) → order → exit 1 on `problems` → build pack → `--check` compare or write `prompts.json` and print the asset count by tier (`build-prompts.mjs:1155-1231`).

`generate.mjs` (`main` `generate.mjs:621-629` catches `UnreadableListError` and returns 1; `run` `generate.mjs:631-942`):

1. Parse options strictly; unknown flags throw (`parseArgs` `strict: true`, `generate.mjs:175`).
2. `--help` → usage. Licence check (§gen-licence). `--reference` must exist (`:643-644`).
3. Load pack; read state unless dry run; select (§gen-select); stop on existing files (§gen-state).
4. Split into in-flight, unconfirmed-batch, and to-submit sets; apply `--limit` (`:660-673`).
5. Check values; plan batches; estimate and spend plan; print summary and worst case; refuse over-cap or unpriced `--yes` with exit 2 (§gen-guard 3–5).
6. Dry run → payload JSON, exit 0. No `--yes` → exit 0.
7. Preflight (§gen-preflight); reconcile leftover `posting` intents (§gen-parallel); move old files aside under `--overwrite`.
8. Parallel mode (§gen-parallel) or batch mode (§gen-batch).
9. Collect (§gen-collect), rewriting `import.toml` per download; write final `import.toml`; report failures; exit (§gen-exit).

## Branches

- `--parallel 0` or any unconfirmed batch in state → batch mode; otherwise parallel mode (`generate.mjs:794`).
- Nothing selected with `--yes`: health and channel checks still run, `--out` is created, model and prompt checks are skipped (`generate.mjs:764`), state and `import.toml` are written, exit 0 unless failed records exist (`generate.mjs:814-860`).
- `codex-image` provider: model list and key readiness are not checked (`generate.mjs:509`); estimate 0 (`knownPrices`, `safety.mjs:8`).
- Model without a `keywords` array: the reference-capability check is skipped (`generate.mjs:516`).
- `--reference` with nothing new to submit: not uploaded (`generate.mjs:775`). In dry run, `provided.reference` is the placeholder `<staged id of <basename>>` (`generate.mjs:334`).
- Structured ground variants carry `derive_from` (`build-prompts.mjs:918, 962`; wall alts `:1073`); `generate.mjs` does not read `derive_from` (no reference to it in `generate.mjs`); a reference image is only what `--reference` names.
- Project status `partial` counts as finished and is downloaded if it has an image output (`generate.mjs:567, 580`).

## Unhappy paths

| Situation | Behavior |
|---|---|
| `prompts.json` missing or invalid JSON | uncaught `readFileSync`/`JSON.parse` error, exit 1 with `error:` prefix (`generate.mjs:245-247, 952`) |
| Keyword over 200 chars / multi-line | `build-prompts`: exit 1, no write (`build-prompts.mjs:1180-1183`); `generate`: exit 1, nothing sent (`generate.mjs:675-679`) |
| Slopify unreachable | GETs retried 6 times then `SlopifyError` (`generate.mjs:454-461`) |
| Library prompt text differs | stops unless `--update-templates` (`generate.mjs:497-501`) |
| Provider has no key | stops before submitting (`generate.mjs:520-521`) |
| Batch POST connection dropped | retried up to 3 attempts with the same `requestId`; state already holds the unconfirmed batch, so a later run re-sends it (`generate.mjs:889-900, 667`) |
| Single-project POST connection dropped (parallel mode) | the intent record `posting` was written first; the project list is read and a project created meanwhile is adopted (no second POST); if none exists the POST is retried once; if the list cannot be read the run exits 1 without posting. A later run reconciles a leftover `posting` record the same way (§gen-parallel) |
| Queue length differs from items | `SlopifyError` after the batch is in state as unconfirmed (`generate.mjs:901-903`) |
| Project title or channel mismatch | `SlopifyError`, run stops (`generate.mjs:845-848, 930-934`) |
| Image generation fails | record `failed` with reason; exit 1; the next run submits it again (failed is not in-flight, `generate.mjs:664`) |
| Download GET fails after retries | `SlopifyError` ends the run; the record stays `submitted`, so the next run collects it again (`generate.mjs:590, 664`) |
| Crash while writing an image | the image is written with plain `writeFileSync` (`generate.mjs:594`); a partial `.png`/`.jpg` counts as downloaded for `--resume` (`generate.mjs:269-275`). Integrity check of downloaded bytes: not implemented beyond magic-byte sniffing |
| Image saved as `.webp`/`.bin` | listed in `import.toml` (file exists, `generate.mjs:561`), but not seen by `downloadedFile`; its record is `downloaded`, so it is neither in-flight nor skipped by `--resume`, and a later run submits it again (`generate.mjs:649-669`) |
| Timeout | exit 1; records stay `submitted`; the next run collects them (`generate.mjs:610-613, 660-664`) |
| Ctrl-C / abandonment | state reflects every confirmed submit; projects keep running in Slopify. Cancellation of submitted projects on exit: not implemented (no signal handler in `generate.mjs`) |
| Two `generate.mjs` runs on one `--out` | no lock; both read and rewrite `.art-gen-state.json` and `import.toml` (`generate.mjs:546-565`). Concurrency guard: not implemented |
| Slopify updating (409) | waits 15 s, up to 40 times per run (`generate.mjs:468-471`) |
| `build-prompts.mjs` data error (duplicate id, missing description, unknown kit or block, missing metadata) | uncaught `Error`, no write (`build-prompts.mjs:222, 866, 873, 875, 923, 930, 946, 1061, 1066, 1089-1094, 1123, 1159, 1170`) |

## State transitions

Asset record `status` in `.art-gen-state.json`:

| From | To | Where |
|---|---|---|
| (none) | `submitted` | single POST reply (`generate.mjs:849`) or batch confirmation (`:905-916`) |
| `submitted` | `downloaded` | image output saved (`generate.mjs:597-604`) |
| `submitted` | `failed` | finished with no image output (`generate.mjs:581-586`) |
| `submitted` (batch, Slopify `pending`) | `failed` (`moved out of the batch queue`) then `submitted` as a single project | parallel mode re-plan (`generate.mjs:799-813`) |
| `failed` / `downloaded` | `submitted` | re-selected on a later run: `failed` always; `downloaded` when the file is missing, or under `--overwrite` |

Batch record: `confirmed: false` on write before POST → `true` after a matching queue (`generate.mjs:889, 919`). Batch records are never removed from state (no deletion in `generate.mjs`).

`prompts.json`: rewritten in full by `build-prompts.mjs` (`build-prompts.mjs:1229`), non-atomically.

## Invariants

1. Nothing is sent to Slopify without `--yes`, and `--dry-run` constructs no HTTP client (`generate.mjs:720-737`; asserted, `mock-slopify.test.mjs:75-87`, `art-gen.test.mjs:298-343`).
2. A batch is recorded before it is sent, and re-sent only with its original `requestId` (`generate.mjs:889-890, 864-865`). A single-project post is recorded as a `posting` intent before it is sent and is never repeated unless Slopify's project list proves no project was created (§gen-parallel).
3. Every batch has one template and one format and at most 50 items (`generate.mjs:279-291, 210`).
4. Every keyword value in `prompts.json` is one trimmed line of 1–200 characters and every template slot is filled; `prompt` equals the rendered template (`build-prompts.mjs:859-869`; asserted, `art-gen.test.mjs:49-62`).
5. `prompts.json` is byte-identical to a fresh build (`build-prompts.mjs:1221-1227`; asserted, `art-gen.test.mjs:45-47`).
6. The first 225 assets are the checklist rows in order, tier 0 (asserted, `art-gen.test.mjs:33-43`).
7. A downloaded file's stem is `rawStem(id)`, and Slopify's project title equals the asset id before any file is written under it (`generate.mjs:845, 930-934, 593`).
8. Library prompts are never overwritten without `--update-templates` (`generate.mjs:497-503`).
9. FLUX.1 [dev]-family models and non-image providers are never submitted (`generate.mjs:638-642`).
10. `.art-gen-state.json` is never left half-written (tmp + rename, `generate.mjs:553-554`); `import.toml`, images and `prompts.json` have no such guarantee.

## Outcomes & side effects

| Artefact | Writer | Content |
|---|---|---|
| `tools/art-gen/prompts.json` | `build-prompts.mjs` (`build-prompts.mjs:1229`) | §gen-pack; stdout `wrote … (N assets; by tier {...})` |
| Slopify Library prompts `arda-*` and `arda-asset-review` | `generate.mjs` (`generate.mjs:488-506`) | created or, with `--update-templates`, replaced |
| Slopify projects in the chosen channel | `generate.mjs` | one per asset (single or batch item), image-only runs |
| Staged reference image | `generate.mjs` (`generate.mjs:525-530`) | one per run with `--reference` and new work |
| `<out>/<rawStem(id)>.<ext>` | `generate.mjs` (`generate.mjs:593-594`) | downloaded images |
| `<out>/.art-gen-state.json` | `generate.mjs` | §gen-state |
| `<out>/import.toml` | `generate.mjs` | §gen-manifest |
| `<out>-replaced/<file>.<ms>` | `generate.mjs` under `--overwrite` (`generate.mjs:782-784`) | previous takes |
| stderr log | both | progress lines; `FAILED <id>: <reason>`, `WARNING`, `REFUSED:` |

- D5 money: every submitted project is billed by the provider behind Slopify to the key configured there (`tools/art-gen/README.md:17` states intent; the code's only acknowledgements are the `--yes` gate text `generate.mjs:733` and usage `:113`). The script records no amount spent, but refuses a run whose worst case exceeds `--max-spend` and stops at `--max-images` (§gen-guard). Who absorbs failed or rejected images, reviewer remakes, and cancelled pending items: the provider account; no compensating action exists in `tools/art-gen/`. A duplicate single-project post after a dropped connection is closed by the intent record, except when Slopify lists the project under another title.
- D7 time: `at` stamps are `new Date().toISOString()` (UTC instant, `generate.mjs:832, 915`); the poll interval and timeout use `Date.now()` and `setTimeout` (`generate.mjs:570, 610, 614`). No expiry of state or batch records.
- D11 termination: success ends with `import.toml written to …` (`generate.mjs:858, 939`); failure leaves state resumable; abandonment leaves Slopify projects running and state as of the last write.
- D12 visibility: the full prompt text, keyword values and reference image leave the machine via Slopify to the chosen image provider and, with review on, to the reviewer CLI agent (`generate.mjs:316-325`). Nothing is redacted.
- D13 notification: stderr only; no other channel.
- D14 effects on others: Slopify's Library gains or changes `arda-*` prompts shared by any other Slopify user of that install; projects land in the shared channel; parallel mode cancels batch items left pending by earlier runs (`generate.mjs:806-809`).
- D15 record: provenance per asset (Slopify project id, Library prompt, timestamp, prompt, provider, model) is kept in state and in `import.toml` comments and fields (`generate.mjs:396-401`). `import.toml` is regenerated, not appended; history lives only in the state file, which keeps every record ever written, and the batch bodies sent.

## Dimensions not in play

- D1 Authority: single local operator; the script holds no credentials and performs no authorisation (`generate.mjs:449-452`); Slopify-side access control is outside `tools/art-gen/`.
- D9 Lifecycle beyond the state record: no entity outside `.art-gen-state.json` changes state under this code's control; Slopify project lifecycles are read, not owned (`generate.mjs:567`).
- D2, D3, D4, D5, D6, D7, D8, D10, D11, D12, D13, D14, D15, D16: in play; recorded in the sections above (D6 limits: batch ≤ 50, keyword ≤ 200, parallel ≤ 20, retries ≤ 5, 409 waits ≤ 40; behaviour at exactly the limit is accepted, `generate.mjs:180, 354`).
