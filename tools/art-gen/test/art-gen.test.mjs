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
import { batchBody, importToml, licenceProblem, planBatches, select } from "../generate.mjs";
import { readChecklist, render, slotsOf } from "../lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const tool = resolve(here, "..");
const repo = resolve(tool, "../..");
const pack = JSON.parse(readFileSync(join(tool, "prompts.json"), "utf8"));
const slopify = resolve(process.env.SLOPIFY_DIR ?? join(repo, "../slopify"));
const dist = join(slopify, "packages/app/dist");

// --- the prompt pack ---------------------------------------------------------------------------

test("prompts.json has exactly one entry per checklist id", () => {
  const ids = readChecklist(join(tool, "checklist.csv")).map((row) => row.id);
  assert.equal(ids.length, 225);
  assert.deepEqual(Object.keys(pack.assets).sort(), [...ids].sort());
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
      assert.match(p, /plain flat white background \(#ffffff\)/, id);
    }
    if (asset.class === "prop" || asset.class === "vegetation") assert.match(p, /about 70% of the frame/, id);
    assert.doesNotMatch(p, /in the style of|Forgotten Adventures|Dungeondraft|Greg Rutkowski|artstation/i, id);
  }
});

test("wall roles describe their canonical arms", () => {
  const arms = {
    run: /left edge to the right edge/,
    door: /left edge to the right edge/,
    window: /left edge to the right edge/,
    gate: /left edge to the right edge/,
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
  assert.equal(batches.flat().length, 225);
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
  assert.equal(all.payloads.flatMap((p) => p.body.items).length, 63);
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
