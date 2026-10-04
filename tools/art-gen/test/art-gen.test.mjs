// Tests for tools/art-gen. Run with:
//
//   node --test tools/art-gen/test/
//
// They never start Slopify and never reach a provider: generate.mjs runs only with --dry-run,
// and with `fetch` replaced by a function that fails the test. The payloads are validated
// with Slopify's own code from its build (`packages/app/dist`): the run-draft schema, the
// admission rules and the keyword substitution, plus a copy of the batch route's body schema.
// Point SLOPIFY_DIR at a Slopify checkout (default: ../slopify beside this repository) that
// has been built with `npm run build`; without it those checks are skipped.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { entries as expansion } from "../expansion.mjs";
import { batchBody, importToml, licenceProblem, parseOptions, planBatches, select } from "../generate.mjs";
import { rawStem, readChecklist, render, slotsOf } from "../lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const tool = resolve(here, "..");
const repo = resolve(tool, "../..");
const pack = JSON.parse(readFileSync(join(tool, "prompts.json"), "utf8"));
const vocabulary = JSON.parse(readFileSync(join(repo, "assets/tactical/placeholder/catalog.json"), "utf8")).vocabulary;
const total = Object.keys(pack.assets).length;
const slopify = resolve(process.env.SLOPIFY_DIR ?? join(repo, "../slopify"));
const dist = join(slopify, "packages/app/dist");

// --- the prompt pack ---------------------------------------------------------------------------

test("prompts.json keeps the checklist's 225 as tier 0 and adds the expansion after them", () => {
  const ids = readChecklist(join(tool, "checklist.csv")).map((row) => row.id);
  assert.equal(ids.length, 225);
  assert.deepEqual(Object.keys(pack.assets).slice(0, 225), ids, "the 225 come first, in checklist order");
  for (const id of ids) assert.equal(pack.assets[id].tier, 0, id);
  assert.ok(total >= 1000 && total <= 1500, `${total} assets`);
  const own = expansion.map((e) => e.id);
  assert.equal(new Set(own).size, own.length, "expansion.mjs ids are unique");
  for (const id of own) assert.ok(!ids.includes(id), `${id} collides with the checklist`);
  for (const id of own) assert.ok(pack.assets[id] !== undefined, `${id} is in prompts.json`);
});

test("prompts.json is what build-prompts.mjs makes", () => {
  execFileSync(process.execPath, [join(tool, "build-prompts.mjs"), "--check"], { stdio: "pipe" });
});

test("every asset fills its template, within Slopify's keyword limits", () => {
  for (const [id, asset] of Object.entries(pack.assets)) {
    const body = pack.templates[asset.template].body;
    assert.deepEqual(slotsOf(body).sort(), Object.keys(asset.values).sort(), id);
    for (const [name, value] of Object.entries(asset.values)) {
      assert.ok(value.length > 0 && value.length <= 200, `${id} ${name} length ${value.length}`);
      assert.ok(!/[\n\r]/.test(value) && value.trim() === value, `${id} ${name} is one trimmed line`);
    }
    assert.equal(render(body, asset.values), asset.prompt, id);
    assert.ok(["16:9", "9:16", "1:1"].includes(asset.format), id);
    assert.deepEqual(asset.pixels, asset.footprint.map((n) => n * 128), id);
  }
});

test("prompts carry the art conventions and no artist names", () => {
  for (const [id, asset] of Object.entries(pack.assets)) {
    const p = asset.prompt;
    assert.match(p, /orthographic/, id);
    assert.match(p, /top-left/, id);
    assert.match(p, /no cast shadow/, id);
    assert.match(p, /no text/i, id);
    assert.match(p, /no frame, no border/, id);
    if (asset.class === "ground" || asset.class === "water") {
      assert.match(p, /Seamless tileable texture, even lighting, no large light or dark patches/, id);
      assert.match(p, /fills the whole frame edge to edge/, id);
    } else {
      assert.match(p, /real alpha channel/, id);
    }
    if (asset.class === "prop" || asset.class === "vegetation") assert.match(p, /about 70% of the frame/, id);
    assert.doesNotMatch(p, /in the style of|Forgotten Adventures|Dungeondraft|Greg Rutkowski|artstation/i, id);
  }
});

test("wall roles describe their canonical arms", () => {
  const arms = {
    run: /left edge to the right edge/,
    door: /left edge to right edge on the centre line/,
    window: /left edge to the right edge/,
    gate: /left edge to right edge on the centre line/,
    post: /left edge to the right edge/,
    corner: /right edge and the other from the centre to the bottom edge; nothing left of or above/,
    tee: /left edge, the right edge and the bottom edge; nothing above/,
    cross: /all four edges/,
    end: /in from the right edge and stops at the centre/,
  };
  for (const [id, asset] of Object.entries(pack.assets)) {
    if (asset.class !== "wall") continue;
    assert.match(asset.values.Shape, arms[asset.role], id);
  }
});

test("structured ground variants say they share variant A's layout", () => {
  for (const [id, asset] of Object.entries(pack.assets)) {
    if (!asset.structured) continue;
    if (asset.variant === 0) assert.match(asset.values.Variant, /master layout/, id);
    else {
      assert.match(asset.values.Variant, /exactly the same/, id);
      assert.equal(asset.derive_from, id.replace(/\.\d+$/, ".0"));
    }
  }
});

// --- the expansion's ids, metadata and variants ----------------------------------------------

const roles = ["run", "door", "window", "gate", "post", "corner", "tee", "cross", "end"];
const idGrammar = {
  ground: /^ground\.[a-z0-9_]+\.\d$/,
  water: /^water\.water_(shallow|deep)\.\d$/,
  wall: new RegExp(`^wall\\.[a-z0-9_]+\\.(${roles.join("|")})(\\.alt[1-9])?$`),
  prop: /^prop\.[a-z0-9_]+(\.alt[1-9])?$/,
  vegetation: /^veg\.[a-z0-9_]+(\.alt[1-9])?$/,
};
const layers = ["ground", "water", "floor", "prop", "wall", "canopy"];
const covers = ["none", "half", "three_quarters", "total"];

test("every id uses a known class and arda-art-import's naming", () => {
  for (const [id, a] of Object.entries(pack.assets)) {
    assert.ok(Object.hasOwn(idGrammar, a.class), `${id}: unknown class ${a.class}`);
    assert.match(id, idGrammar[a.class], id);
    assert.equal(a.template, pack.classes[a.class].template, id);
    if (a.class === "wall") assert.equal(id.split(".").slice(1, 3).join("."), `${a.kit}.${a.role}`, id);
    if (a.ground !== undefined) assert.equal(id, `${a.class}.${a.ground}.${a.variant}`, id);
  }
});

test("every asset carries valid catalogue metadata and tags", () => {
  for (const [id, a] of Object.entries(pack.assets)) {
    assert.ok(layers.includes(a.layer), `${id}: layer ${a.layer}`);
    assert.ok(covers.includes(a.cover), `${id}: cover ${a.cover}`);
    assert.ok(Number.isInteger(a.height_ft) && a.height_ft >= 0, `${id}: height_ft ${a.height_ft}`);
    for (const flag of ["blocks_movement", "blocks_sight", "difficult_terrain"]) assert.equal(typeof a[flag], "boolean", `${id} ${flag}`);
    assert.ok(a.footprint.every((n) => Number.isInteger(n) && n >= 1 && n <= 16), `${id}: footprint`);
    assert.ok([0, 1, 2, 3].includes(a.tier), `${id}: tier ${a.tier}`);
    assert.deepEqual(Object.keys(a.tags).sort(), ["biome", "culture", "free", "function", "wealth"], id);
    for (const list of ["biome", "culture", "wealth", "function"])
      for (const tag of a.tags[list]) assert.ok(vocabulary[list].includes(tag), `${id}: ${list} "${tag}" is not in the catalogue vocabulary`);
    for (const tag of a.tags.free) assert.match(tag, /^[a-z0-9_]+(:[a-z0-9_]+)?$/, `${id}: free tag ${tag}`);
    for (const f of a.functions) assert.ok(vocabulary.function.includes(f), `${id}: function ${f}`);
    if (a.class === "ground" || a.class === "water") assert.ok(["ground", "water"].includes(a.layer), id);
    if (a.class === "wall") assert.equal(a.layer, "wall", id);
    if (a.class === "prop" || a.class === "vegetation") assert.ok(["floor", "prop", "canopy"].includes(a.layer), id);
  }
});

test("alts are interchangeable with their base: same footprint, layer, tags and blocking", () => {
  const shared = ["class", "template", "format", "footprint", "pixels", "layer", "height_ft", "tags", "blocks_movement", "blocks_sight", "difficult_terrain", "cover"];
  const altsOf = {};
  for (const [id, a] of Object.entries(pack.assets)) {
    const alt = /^(.*)\.alt(\d)$/.exec(id);
    assert.equal(a.variant_of, alt?.[1], `${id}: variant_of`);
    if (alt === null) continue;
    const base = pack.assets[a.variant_of];
    assert.ok(base !== undefined, `${id}: no base ${a.variant_of}`);
    for (const key of shared) assert.deepEqual(a[key], base[key], `${id}: ${key} differs from ${a.variant_of}`);
    (altsOf[a.variant_of] ??= []).push(id);
  }
  for (const [base, list] of Object.entries(altsOf)) {
    const numbers = list.map((id) => Number(id.slice(-1))).sort();
    assert.deepEqual(numbers, numbers.map((_, i) => i + 1), `${base}: alts run .alt1 to .alt${numbers.length}`);
    // Each take reads differently, so the model draws visibly different images.
    const texts = [base, ...list].map((id) => `${pack.assets[id].values.Subject}|${pack.assets[id].values.Detail}`);
    assert.equal(new Set(texts).size, texts.length, `${base}: two takes share Subject and Detail`);
  }
});

test("every cut-out has at least three takes, every wall piece two, every ground key three", () => {
  const takes = {};
  for (const [id, a] of Object.entries(pack.assets)) {
    const key = a.ground !== undefined ? `ground:${a.ground}` : (a.variant_of ?? id);
    takes[key] = (takes[key] ?? 0) + 1;
  }
  for (const [key, n] of Object.entries(takes)) {
    const min = key.startsWith("wall.") ? 2 : 3;
    assert.ok(n >= min, `${key} has ${n} takes, want at least ${min}`);
    assert.ok(n <= (key.startsWith("ground:") ? 10 : 9), `${key} has ${n} takes; the importer numbers at most nine alts or ten variants in order`);
  }
  for (const freq of ["prop.barrel", "prop.crate", "prop.table", "prop.chair", "prop.bed", "prop.chest", "veg.tree_oak", "veg.tree_pine", "veg.tree_spruce"])
    assert.ok(takes[freq] >= 5, `${freq} has ${takes[freq]} takes`);
});

test("every wall kit is complete and its alts start from an accepted base piece", () => {
  const kits = {};
  for (const a of Object.values(pack.assets)) if (a.class === "wall") (kits[a.kit] ??= new Set()).add(a.role);
  for (const [kit, have] of Object.entries(kits)) assert.deepEqual([...have].sort(), [...roles].sort(), `kit ${kit}`);
  for (const [id, a] of Object.entries(pack.assets))
    if (a.class === "wall" && a.variant_of !== undefined) assert.equal(a.derive_from, a.variant_of, id);
});

test("variants of the first 225 are tier 1, and each tier runs in impact order", () => {
  for (const [id, a] of Object.entries(pack.assets)) {
    const base = a.variant_of ?? (a.ground !== undefined ? `${a.class}.${a.ground}.0` : undefined);
    if (a.tier !== 0 && base !== undefined && pack.assets[base]?.tier === 0) assert.equal(a.tier, 1, id);
  }
  const tiers = Object.values(pack.assets).map((a) => a.tier);
  assert.deepEqual(tiers, [...tiers].sort((x, y) => x - y), "prompts.json is ordered by tier");
  const counts = pack.tiers.counts;
  assert.equal(counts[0], 225);
  assert.ok(counts[1] >= 200 && counts[2] >= 300 && counts[3] >= 100, JSON.stringify(counts));
});

test("floor tiles are rail-less and fill their footprint; doors and gates are contrasting slabs", () => {
  for (const [id, a] of Object.entries(pack.assets)) {
    if (/^prop\.(bridge_deck|dock_planks|gangplank)(\.alt\d)?$/.test(id) && (a.tier > 0 || id === "prop.bridge_deck")) {
      assert.match(a.prompt, /no rails/, id);
      assert.match(a.prompt, /square-cut|straight cut/, id);
    }
    if (/^prop\.bridge_deck_stone/.test(id)) assert.match(a.prompt, /no parapets/, id);
    if (/^prop\.bridge_deck(\.alt\d)?$/.test(id)) assert.match(a.values.Subject, /five narrow (\w+ )?planks, each about one foot wide/, id);
    if (a.class === "wall" && (a.role === "door" || a.role === "gate")) assert.match(a.values.Shape, /gap between (jambs|posts), wall top stopped, shut by .* along the wall line/, id);
  }
});

test("rawStem saves alts so the importer numbers them like the prompt ids", () => {
  assert.equal(rawStem("prop.barrel"), "prop.barrel");
  assert.equal(rawStem("prop.barrel.alt3"), "prop.barrel__alt3");
  assert.equal(rawStem("wall.stone.run.alt1"), "wall.stone.run__alt1");
  assert.equal(rawStem("ground.grass.3"), "ground.grass.3");
  const files = ["prop.barrel.alt2", "prop.barrel", "prop.barrel.alt1"].map((id) => `${rawStem(id)}.png`).sort();
  assert.deepEqual(files, ["prop.barrel.png", "prop.barrel__alt1.png", "prop.barrel__alt2.png"]);
});

test("--tier selects whole tiers and rejects nonsense", () => {
  const one = select(pack, parseOptions(["--tier", "1"]));
  assert.ok(one.length > 0 && one.every((a) => a.tier === 1));
  const both = select(pack, parseOptions(["--tier", "2,3", "--class", "prop"]));
  assert.ok(both.every((a) => (a.tier === 2 || a.tier === 3) && a.class === "prop"));
  assert.throws(() => parseOptions(["--tier", "one"]), /--tier/);
});

// --- generate.mjs, offline -------------------------------------------------------------------

test("FLUX.1 [dev] models are refused; commercial ones pass", () => {
  for (const model of ["black-forest-labs/flux-dev", "FLUX.1-dev", "fal-ai/flux/dev", "fal-ai/flux-krea-dev"])
    assert.match(licenceProblem("replicate", model) ?? "", /non-commercial/, model);
  for (const [provider, model] of [
    ["openai-image", "gpt-image-2"],
    ["google-image", "gemini-3.1-flash-image"],
    ["replicate", "black-forest-labs/flux-1.1-pro"],
    ["replicate", "black-forest-labs/flux-schnell"],
    ["fal", "fal-ai/flux-2"],
  ])
    assert.equal(licenceProblem(provider, model), undefined, model);
  assert.match(licenceProblem("openai", "gpt-image-2") ?? "", /Unknown image provider/);
});

test("batches group by template and frame, at most 50 items", () => {
  const assets = select(pack, {});
  const batches = planBatches(assets, 50);
  assert.equal(batches.flat().length, total);
  for (const batch of batches) {
    assert.ok(batch.length >= 1 && batch.length <= 50);
    assert.equal(new Set(batch.map((a) => `${a.template}|${a.format}`)).size, 1);
  }
});

test("import.toml escapes prompts and carries provenance", () => {
  const text = importToml(
    [
      {
        file: "prop.anvil.png",
        prompt: 'say "hi"\\ok',
        provider: "openai-image",
        model: "gpt-image-2",
        footprint: [1, 1],
        projectId: "01J",
        template: "arda-prop",
        at: "now",
      },
    ],
    { licence: "CC0-1.0" },
  );
  assert.match(text, /^\[library\]$/m);
  assert.match(text, /^licence = "CC0-1.0"$/m);
  assert.match(text, /^file = "prop\.anvil\.png"$/m);
  assert.match(text, /^prompt = "say \\"hi\\"\\\\ok"$/m);
  assert.match(text, /^model = "gpt-image-2"$/m);
  assert.match(text, /^tool = "Slopify \(openai-image\)"$/m);
  const meta = importToml(
    [{ file: "prop.barrel__alt1.png", prompt: "p", provider: "codex-image", model: "gpt-6-astra", footprint: [1, 1], projectId: "01K", template: "arda-prop", at: "now", meta: pack.assets["prop.barrel.alt1"] }],
    { licence: "CC0-1.0" },
  );
  assert.match(meta, /^layer = "prop"$/m);
  assert.match(meta, /^height_ft = 4$/m);
  assert.match(meta, /^cover = "half"$/m);
  assert.match(meta, /^blocks_movement = true$/m);
  assert.match(meta, /^tags = \{ biome = \["temperate"\], culture = \["human"\], wealth = \["poor", "modest"\], function = \[.*"warehouse".*\], free = \["container"\] \}$/m);
});

// Runs the real CLI with --dry-run, with `fetch` made to fail the run if it is ever called.
const noNetwork = `data:text/javascript,${encodeURIComponent(
  'globalThis.fetch = () => { throw new Error("art-gen dry run used the network"); };',
)}`;
function dryRun(args) {
  const stdout = execFileSync(
    process.execPath,
    ["--import", noNetwork, join(tool, "generate.mjs"), "--dry-run", "--out", join(here, "no-such-dir"), ...args],
    { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  return JSON.parse(stdout);
}

const runs = {
  "ground grass pilot": ["--only", "ground.grass.*", "--limit", "3"],
  "all ground": ["--class", "ground"],
  water: ["--class", "water"],
  "stone wall kit": ["--only", "wall.stone.*"],
  "all walls": ["--class", "wall"],
  props: ["--class", "prop", "--provider", "google", "--model", "gemini-3.1-flash-image"],
  vegetation: ["--class", "vegetation"],
  "with a reference": ["--class", "vegetation,prop", "--limit", "12", "--reference", join(tool, "checklist.csv")],
  "tier 1": ["--tier", "1"],
  "tiers 2 and 3": ["--tier", "2,3", "--provider", "codex", "--model", "gpt-6-astra", "--thinking", "high"],
};

test("dry runs print payloads without touching the network", () => {
  const out = dryRun(runs["ground grass pilot"]);
  assert.equal(out.payloads.length, 1);
  assert.deepEqual(
    out.payloads[0].body.items.map((i) => i.title),
    ["ground.grass.0", "ground.grass.1", "ground.grass.2"],
  );
  assert.equal(out.payloads[0].path, "/api/projects/batch");
  assert.equal(out.templates["arda-ground"].body, pack.templates["arda-ground"].body);
  const all = dryRun(runs["all walls"]);
  const walls = Object.values(pack.assets).filter((a) => a.class === "wall").length;
  assert.equal(all.payloads.flatMap((p) => p.body.items).length, walls);
  const tier = dryRun(runs["tier 1"]);
  const items = tier.payloads.flatMap((p) => p.body.items.map((i) => i.title));
  assert.deepEqual(
    items.sort(),
    Object.keys(pack.assets)
      .filter((id) => pack.assets[id].tier === 1)
      .sort(),
  );
});

// --- against Slopify's own code -------------------------------------------------------------

async function slopifyCode() {
  const zodPkg = join(slopify, "node_modules/zod/package.json");
  if (!existsSync(join(dist, "slices/admission/schema.js")) || !existsSync(zodPkg)) return undefined;
  const zodEntry = JSON.parse(readFileSync(zodPkg, "utf8")).exports["."].import;
  const load = (path) => import(pathToFileURL(path).href);
  const { z } = await load(join(dirname(zodPkg), zodEntry));
  const { runDraftSchema } = await load(join(dist, "slices/admission/schema.js"));
  const { admit } = await load(join(dist, "slices/admission/rules.js"));
  const { collectFields, render: slopifyRender } = await load(join(dist, "slices/admission/substitute.js"));
  // A copy of `edge/http/planning.ts`'s body schema, which the module does not export. The
  // test below checks the built route still says the same.
  const planningBody = z.object({
    draft: runDraftSchema,
    expectedWords: z.number().int().min(1).max(100000).default(1500),
    items: z
      .array(
        z
          .object({
            title: z.string().trim().min(1).max(200),
            values: z.record(z.string().max(200), z.string().max(10000)),
          })
          .strict(),
      )
      .min(1)
      .max(50)
      .optional(),
  });
  const batchSchema = planningBody.extend({ requestId: z.uuid() });
  return { batchSchema, admit, collectFields, slopifyRender };
}

const code = await slopifyCode();
const skip = code === undefined ? `no built Slopify at ${slopify} (set SLOPIFY_DIR, run npm run build there)` : false;

test("the copied batch schema matches Slopify's built route", { skip }, () => {
  const route = readFileSync(join(dist, "edge/http/planning.js"), "utf8").replace(/\s+/g, "");
  for (const fragment of [
    "draft:runDraftSchema",
    "expectedWords:z.number().int().min(1).max(100000).default(1500)",
    "title:z.string().trim().min(1).max(200)",
    "values:z.record(z.string().max(200),z.string().max(10000))",
    ".min(1).max(50).optional()",
    "requestId:z.uuid()",
  ])
    assert.ok(route.includes(fragment), `planning.js no longer has ${fragment}`);
});

for (const [name, args] of Object.entries(runs)) {
  test(`dry-run payloads pass Slopify's schema and admission: ${name}`, { skip }, () => {
    const { payloads, templates } = dryRun(args);
    assert.ok(payloads.length > 0);
    for (const { body } of payloads) {
      // The route's zod validation, defaults applied, as zValidator does.
      const parsed = code.batchSchema.safeParse({ ...body, requestId: crypto.randomUUID() });
      assert.ok(parsed.success, JSON.stringify(parsed.error?.issues?.slice(0, 3)));
      const input = parsed.data;
      const template = templates[input.draft.imagePrompts[0].name].body;
      // What the route's prepare() does per item: merge the item into the draft, read the
      // picked prompt's keywords, admit the run and render the prompt text.
      const staged =
        input.draft.provided.reference === undefined
          ? []
          : [{ id: input.draft.provided.reference, stageKind: "images", state: "staged", path: "x", originalFilename: "x", bytes: 1, createdAt: "" }];
      for (const item of input.items) {
        const draft = { ...input.draft, title: item.title, values: { ...input.draft.values, ...item.values } };
        const requiredSlots = code.collectFields([draft.title], [template]).map((f) => f.name);
        const admitted = code.admit({ draft, staged, requiredSlots });
        assert.ok(admitted.ok, `${item.title}: ${JSON.stringify(admitted.fields)}`);
        const rendered = code.slopifyRender(template, admitted.draft.values);
        assert.equal(rendered, pack.assets[item.title].prompt, item.title);
        assert.deepEqual(
          Object.values(admitted.draft.sources).filter((s) => s !== "off"),
          ["generate"],
          "only the Images stage runs",
        );
      }
    }
  });
}

test("the default and documented models are in Slopify's model list", { skip }, () => {
  const yaml = readFileSync(join(slopify, "packages/app/src/assets/models.yaml"), "utf8");
  const listed = (provider, model) =>
    new RegExp(`- provider: ${provider}\\n\\s+id: ${model.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&")}\\n`).test(yaml);
  for (const [provider, model] of [
    ["openai-image", "gpt-image-2"],
    ["google-image", "gemini-3.1-flash-image"],
    ["replicate", "black-forest-labs/flux-1.1-pro"],
  ])
    assert.ok(listed(provider, model), `${provider} ${model}`);
});

test("batchBody keeps every stage but Images off", () => {
  const body = batchBody(select(pack, { only: "prop.anvil" }), { provider: "openai-image", model: "gpt-image-2" });
  assert.equal(body.draft.sources.article, "off");
  assert.equal(body.draft.sources.images, "generate");
  assert.equal(body.draft.reference, undefined);
  assert.deepEqual(body.draft.imagePrompts, [{ name: "arda-prop", number: 1 }]);
});

test("Slopify's admission refuses what the limits forbid (the harness can fail)", { skip }, () => {
  const { payloads, templates } = dryRun(["--only", "prop.anvil"]);
  const input = code.batchSchema.parse({ ...payloads[0].body, requestId: crypto.randomUUID() });
  const template = templates["arda-prop"].body;
  const requiredSlots = code.collectFields(["prop.anvil"], [template]).map((f) => f.name);
  const long = { ...input.draft, title: "prop.anvil", values: { ...input.items[0].values, Detail: "x".repeat(201) } };
  assert.equal(code.admit({ draft: long, staged: [], requiredSlots }).ok, false);
  const missing = { ...input.draft, title: "prop.anvil", values: { Subject: "a" } };
  assert.equal(code.admit({ draft: missing, staged: [], requiredSlots }).ok, false);
  const article = { ...input.draft, title: "prop.anvil", values: input.items[0].values, sources: { ...input.draft.sources, article: "generate" } };
  assert.equal(code.admit({ draft: article, staged: [], requiredSlots }).ok, false);
});

test("batches carry an image reviewer that remakes failed images, unless turned off", () => {
  const asset = { id: "wall.city_wall.door", template: "arda-wall", format: "1:1", values: { Shape: "x" } };
  const on = batchBody([asset], parseOptions([]));
  assert.deepEqual(
    { ...on.draft.reviews, stages: undefined },
    { provider: "claude-code", model: "opus", retries: 2, stages: undefined },
  );
  assert.equal(on.draft.reviews.stages.images.mode, "redo");
  assert.equal(on.draft.reviews.stages.images.prompt, "arda-asset-review");
  const flag = batchBody([asset], parseOptions(["--review-mode", "flag", "--reviewer", "codex", "--reviewer-model", "gpt-6-astra", "--review-retries", "4"]));
  assert.deepEqual(
    { ...flag.draft.reviews, stages: undefined },
    { provider: "codex", model: "gpt-6-astra", retries: 4, stages: undefined },
  );
  assert.equal(flag.draft.reviews.stages.images.mode, "flag");
  assert.equal(batchBody([asset], parseOptions(["--no-review"])).draft.reviews, undefined);
  assert.throws(() => parseOptions(["--reviewer", "gemini-cli"]), /--reviewer must be/);
  assert.throws(() => parseOptions(["--review-retries", "9"]), /--review-retries/);
});
