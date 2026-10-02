# art-gen: Arda's tactical art through Slopify

This folder holds a prompt pack for the 225 assets in Arda's tactical art library, and a script that has a local Slopify generate them. The output is a folder that `arda tactical import` reads directly.

| file | what it is |
|---|---|
| `checklist.csv` | the asset list: id, class, footprint, pixels, layer, functions, notes. It is a copy of `docs/art-library-checklist.csv` from the art planning branch. |
| `prompts.json` | one entry per asset id: its keyword values, the full prompt, the frame, footprint and pixels. Generated; don't edit it by hand. |
| `build-prompts.mjs` | writes `prompts.json` from the checklist and the descriptions in the script. Edit descriptions here. |
| `generate.mjs` | the driver: selects assets, sends Slopify batches, waits, downloads, and writes `import.toml`. |
| `test/` | `node --test tools/art-gen/test/` |

Everything uses Node 26 and its standard library. There's nothing to install.

> **Generation costs money.** Every image is billed to the provider key you give Slopify. The script sends nothing unless you pass `--yes`, and `--dry-run` contacts nothing at all. Start with the pilot in step 2.

## How the prompts reach Slopify

Slopify draws an image from a **Library image prompt**, filling its `{{keywords}}` with per-run values. It caps each value at 200 characters on a single line, which is too short for a whole prompt. The pack is therefore split into two parts:

- **Five Library prompts**, `arda-ground`, `arda-water`, `arda-wall`, `arda-prop` and `arda-vegetation`. Each holds the shared style preamble and its class's rules. Their text is in `prompts.json` under `templates`. The script creates them in Slopify's Library the first time you submit.
- **Per-asset keywords** fill each Library prompt: `Subject`, `Shape` and `Detail`, plus `Variant` for ground and water.

Every asset's fully rendered prompt is also stored in `prompts.json` as `prompt`. The tests check that it matches what Slopify renders.

The preamble asks for the same things in every prompt:

- painterly top-down battle-map art, strictly orthographic;
- light from the top-left;
- no cast or drop shadow;
- no text, no frame or border, no vignette.

Each class then adds its own rules:

- **Ground and water:** "seamless tileable texture, even lighting, no large light or dark patches", filling the whole frame edge to edge.
- **Props and vegetation:** one subject, centred, filling about 70% of the frame's shorter side, on a plain flat white `#ffffff` background.
- **Walls:** a band of even thickness on white. The junction sits at the image centre, and each arm runs straight to the image edge it reaches.

A batch is one Slopify draft plus up to 50 items. Each item has a title, the asset id, and its keyword values. In the draft, Images is set to Generate and every other stage, Article included, is Off, so these are image-only runs. Slopify makes only 16:9 or 9:16 images, so the script groups assets by Library prompt and frame:

- 9:16 for cut-outs whose footprint is taller than it is wide (bed, cart, rowboat, ferry boat and others);
- 16:9 for everything else.

The importer then does the cropping:

- textures and walls are centre-cropped to a square;
- cut-outs are cropped to the object and scaled to fill their footprint.

## Step by step

### 1. Start Slopify and add a provider key

```sh
npx @gentbajko/slopify@latest        # opens http://127.0.0.1:6969
```

In Slopify, open **Settings → Providers** and add a key for the image provider you'll use. The default is **OpenAI** (`openai-image`). Press **Test** there; it's the cheapest check the provider offers.

If Slopify runs elsewhere, pass `--server http://host:port` or set `SLOPIFY_URL`. Slopify's API has no authentication, so keep it on loopback.

### 2. Generate a small pilot

First look at what would be sent. This step is free and offline:

```sh
node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --dry-run
```

The summary and the estimate go to stderr. The exact request bodies go to stdout as JSON.

Then send it:

```sh
node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --yes
```

Here is what happens:

1. The script checks that Slopify is up, that the provider has a key, and that the model is in Slopify's model list.
2. It creates the Library prompts it needs.
3. It queues the batch. Slopify runs batch items one at a time.
4. It polls each project until it finishes.
5. It saves `out/art-gen/raw/<asset-id>.png` (or `.jpg`; see below) and rewrites `out/art-gen/raw/import.toml`. The manifest records each file's prompt, provider and model.

Every image also stays in Slopify under **Projects**. Each project is titled with its asset id, and its files are in `Documents/Slopify/Projects/<id>/images/`.

### 3. Import

```sh
cargo run --release -p arda-cli -- tactical import out/art-gen/raw \
    --out assets/tactical/ai --manifest out/art-gen/raw/import.toml --contact-sheet sheet.png
```

The importer does the following:

- removes the white background;
- strips or flags baked shadows;
- crops and fits each cut-out to its footprint;
- turns wall pieces to their canonical arms;
- makes textures tileable;
- validates the library against the placeholder base.

For the colour grade, add `--reference /path/to/a/target-map.jpg --grade-strength 0.4`.

### 4. Curate

Open `sheet.png` and `assets/tactical/ai/report.md`. The contact sheet frames each asset:

- green: clean;
- amber: carries flags;
- red: rejected, and its slot falls back to the placeholder.

Textures are shown tiled 2 × 2, so seams show.

To redo one asset, delete its file from `out/art-gen/raw` and run again with `--resume`. Alternatively, keep several takes side by side; the importer reads `prop.anvil__take2.png` as a second take, `prop.anvil.alt1`. Render a few layouts to judge the art in context:

```sh
cargo run --release -p arda-cli -- tactical render --layout all \
    --library assets/tactical/ai:assets/tactical/placeholder --out out/art
```

### 5. Generate the rest

Follow the checklist's order, biggest visual impact first:

```sh
node tools/art-gen/generate.mjs --class ground,water --resume --yes
node tools/art-gen/generate.mjs --only "veg.tree_*,veg.bush" --resume --yes
node tools/art-gen/generate.mjs --only "wall.stone.*,wall.timber.*" --resume --yes
node tools/art-gen/generate.mjs --class prop,vegetation --resume --yes
node tools/art-gen/generate.mjs --resume --yes          # whatever is left
```

`--resume` skips assets that are already downloaded. Without it, the script stops if any selected asset already has a file, unless you pass `--overwrite`. `--overwrite` moves the old file to `out/art-gen/raw-replaced/` before making a new one.

The script remembers what it has already submitted in `out/art-gen/raw/.art-gen-state.json`. After a crash, Ctrl-C or `--timeout`, run it again and it does the following:

- collects projects that are still running or done instead of paying for them twice;
- resends any batch it never got an answer for, with the original request id. Slopify answers that with the queue it already made.

Then import again (step 3) and curate (step 4).

### 6. Serve the stacked library

```sh
cargo run --release -p arda-server -- --world <world-dir> \
    --library assets/tactical/ai:assets/tactical/placeholder
```

The leftmost library wins, and the placeholder fills every slot the AI library lacks. The art is in place without a code change.

## Options

```
--class LIST          ground, water, wall, prop, vegetation (comma-separated)
--only GLOB           asset ids; * and ?, comma-separated alternatives
--limit N             at most N assets
--resume              skip assets already downloaded
--overwrite           make downloaded assets again
--provider ID         openai-image (default), google-image, fal, replicate, codex-image;
                      aliases openai, google, codex
--model ID            default gpt-image-2
--reference FILE      a style reference, sent as Slopify's establishing image
--batch-size N        at most 50 (Slopify's limit), default 50
--price-per-image USD for the estimate when Slopify's catalogue has no price
--out DIR             default out/art-gen/raw
--licence SPDX        [library] licence in import.toml, default LicenseRef-AI-generated
--server URL          default http://127.0.0.1:6969 or SLOPIFY_URL
--update-templates    overwrite arda-* Library prompts that differ from prompts.json
--poll SECONDS        default 10
--timeout MINUTES     default 360
--dry-run             print the payloads, contact nothing
--yes                 actually submit
```

The script prints a count and cost estimate before it does anything. Slopify's model list gives a fixed per-image price only for some models. Google's `gemini-3.1-flash-image` is listed at about $0.10 per 2K image. OpenAI's `gpt-image-*` models are billed by tokens and have no list price, so pass `--price-per-image` for a figure. The Codex CLI provider (`codex-image`) runs on your local Codex sign-in and plan rather than an API key, so Slopify counts it as $0.

## Tips

**Use a commercially safe model.** The art may ship with Arda, so pick a model whose terms let you use the output commercially. These models work:

- OpenAI `gpt-image-2` (the default);
- Google `gemini-3.1-flash-image` or `gemini-3-pro-image`;
- Replicate `black-forest-labs/flux-1.1-pro` or `black-forest-labs/flux-schnell`; schnell is Apache-2.0.

The script refuses any FLUX.1 [dev]-family model, such as `black-forest-labs/flux-dev`, because its licence is non-commercial. Set the licence you actually hold with `--licence`, for example `--licence CC0-1.0` if you choose to dedicate the art.

**Keep your outputs.** Slopify doesn't take a seed, and the same prompt gives a different image every time. A file you like can't be regenerated, so keep the raw folder and Slopify's projects. `import.toml` records the prompt and model for provenance, not for reproduction.

**Use a reference image for consistency.** `--reference out/target.jpg` uploads one image and sends it as Slopify's establishing image. Every image in the run is then drawn with it as a reference for style and palette. A crop of one of the target battle maps works well. It needs a model that takes a reference image: OpenAI, Google and the fal models listed with `reference`, but not the Replicate FLUX models.

**Structured ground variants.** Cobbles, flagstone, planks, rug, farmland, stone floor and cliff variants must share one layout, differing only in wear and stains. Otherwise the compositor's cross-fades show doubled grout lines. A model won't reliably repeat a layout. So generate variant `.0` first, curate it, and then do one of the following:

- derive `.1` and `.2` from it by hand, by adjusting tint and wear or painting in stains; or
- generate them with the accepted `.0` as the reference:

  ```sh
  node tools/art-gen/generate.mjs --only "ground.cobbles.0" --yes
  node tools/art-gen/generate.mjs --only "ground.cobbles.1,ground.cobbles.2" \
      --reference out/art-gen/raw/ground.cobbles.0.png --resume --yes
  ```

The importer registers structured variants against the first one and flags any that still differ. `prompts.json` marks these with `"structured": true` and `"derive_from"`.

**Wall arms.** The importer centre-crops a wall image to a square and turns the piece to its canonical arms:

- `run`, `door`, `window`, `gate` and `post` run west to east;
- `corner`: east and south;
- `tee`: east, south and west;
- `end`: east;
- `cross`: all four.

A piece drawn in another orientation is still fine, because the importer rotates it. What breaks a piece is an arm that stops short of the edge, a junction away from the centre, or a perspective view. If a kit keeps coming out wrong:

- Generate the `run` piece first. Then pass it as `--reference` for the rest of the kit, so the material and thickness match.
- Strengthen the Shape wording in `build-prompts.mjs`, for example "the arm touches the right edge of the image", and rebuild.
- For a stubborn corner or tee, it's often quicker to cut and mirror the accepted `run` in an image editor.

**Cut-out backgrounds.** The importer removes a plain border-connected backdrop. White works best. Ask again if a model paints a floor or ground patch under a prop. Interior holes in an object, such as the middle of a mushroom ring, are only cleared when they connect to the border.

## Editing the prompts

1. Edit the descriptions or templates in `build-prompts.mjs`.
2. Run `node tools/art-gen/build-prompts.mjs`. It refuses any keyword over 200 characters.
3. Run the tests.

If a template's text changed, pass `--update-templates` on the next submit. The script won't silently overwrite the Library prompt otherwise.

## Tests

```sh
node --test tools/art-gen/test/
```

The tests never start Slopify or call a provider. They check the following:

- `prompts.json` covers every checklist id, is up to date, and stays within Slopify's limits;
- the dry-run payloads for each class pass Slopify's own code: the run-draft zod schema, a copy of the batch route's body schema, the admission rules and the keyword substitution. The copied schema is checked against the built route;
- the default models are in Slopify's model list;
- the submit, poll, download and `--resume` path works against a stand-in HTTP server.

The checks against Slopify's code need a built Slopify checkout beside this repository (`../slopify`, or set `SLOPIFY_DIR`); build it with `npm run build`. Without one, those checks are skipped.
