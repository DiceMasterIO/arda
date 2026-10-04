#!/usr/bin/env node
// Generates Arda's tactical art through a local Slopify (http://127.0.0.1:6969) and writes a
// folder `arda tactical import` reads. Node 26, standard library only.
//
//   node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --dry-run
//   node tools/art-gen/generate.mjs --only "ground.grass.*" --limit 3 --out out/art-gen/raw --yes
//
// Nothing is sent without --yes; --dry-run never contacts Slopify at all. See README.md.

import { randomUUID } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as sleep } from "node:timers/promises";
import { parseArgs } from "node:util";
import { globMatcher, rawStem } from "./lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));

export const defaults = {
  server: process.env.SLOPIFY_URL ?? "http://127.0.0.1:6969",
  provider: "openai-image",
  model: "gpt-image-2",
  out: "out/art-gen/raw",
  batchSize: 50,
  poll: 10,
  timeout: 360,
  licence: "LicenseRef-AI-generated",
  libraryName: "arda-ai",
  libraryVersion: "0.1.0",
  reviewer: "claude-code",
  reviewerModel: "opus",
  reviewRetries: 2,
  parallel: 5,
};

// Slopify's provider ids for image models, and friendlier spellings.
const providerAliases = {
  openai: "openai-image",
  google: "google-image",
  gemini: "google-image",
  codex: "codex-image",
};
export const imageProviders = ["openai-image", "google-image", "fal", "replicate", "codex-image"];

// Slopify's admission limits (`edge/http/planning.ts`, `slices/admission/rules.ts`).
const batchMax = 50;
const valueMax = 200;

// Known per-image prices from Slopify's models.yaml (2026-09-26). Everything else is unknown
// there too; --price-per-image fills the gap.
const knownPrices = {
  "google-image/gemini-3.1-flash-image": 0.101,
  "codex-image/*": 0,
};

// What the image reviewer fails an asset for, kept as the Library review prompt
// reviewPromptName (created like the image templates). Slopify sends it the image and the brief it was
// drawn from (the full prompt), so the rules here are the ones every Arda asset shares; the
// geometry comes from the brief.
export const reviewPromptName = "arda-asset-review";
export const reviewInstruction = [
  "Review this tabletop battle-map asset against the brief it was drawn from. It is placed on a 5-foot square grid, seen from straight above.",
  "Fail it when any of these hold:",
  "1. View: it is not strictly top-down orthographic (any perspective, horizon, tilted, isometric or side-on view; visible vertical faces of walls or objects beyond a thin edge; a fence or railing drawn as an upright panel of pickets or planks instead of a thin line of post tops and rail tops).",
  "2. Geometry: it does not match the layout the brief states. Walls must run exactly where the brief says (which edges they touch, through the centre line, nothing extra). A door or gate must sit in a gap in the wall line, its leaf lying ALONG the wall's length between two jambs, with the wall top, parapet or crenellations stopping at the gap; fail a door drawn across the wall's thickness, a door lying on top of the wall-walk, or a door that cannot be told apart from the wall.",
  "3. Background: for a cut-out (prop, vegetation, wall piece), anything other than the subject is not transparent (a solid, white, checkerboard or coloured backdrop, a ground patch or a frame); for a ground or water tile, it does not fill the whole frame edge to edge as a texture.",
  "4. Shading: it shows a cast shadow or drop shadow on the ground.",
  "5. Clutter: stray text, letters, numbers, grid lines, watermarks, borders, or extra objects the brief did not ask for; or several copies of the subject when the brief asks for one.",
  "6. Scale and build: parts are implausibly sized for the stated footprint (e.g. bridge planks much wider than a foot, a door wider than its wall run), or a structure has nothing holding it together.",
  "7. Malformed: broken, melted or nonsensical shapes; a creature with wrong anatomy.",
  "Do not fail it for painterly style, palette or small detail choices.",
].join("\n");

const usage = `Usage: node tools/art-gen/generate.mjs [options]

Selection
  --class LIST          ground, water, wall, prop, vegetation (comma-separated)
  --tier LIST           generation tiers, e.g. 1 or 1,2 (see plan.md); 0 is the first 225
  --only GLOB           asset ids, e.g. "ground.grass.*" or "wall.stone.*,prop.barrel"
  --limit N             at most N assets (after the other filters)
  --resume              skip assets already downloaded to --out
  --overwrite           regenerate assets already downloaded (the old file moves to <out>/../<out>-replaced/)

Generation
  --provider ID         ${imageProviders.join(", ")} (aliases: openai, google, codex); default ${defaults.provider}
  --model ID            default ${defaults.model}
  --reference FILE      a style reference, sent as Slopify's establishing image
  --parallel N          keep N assets generating at once, each posted as its own project;
                        default ${defaults.parallel}. 0 sends Slopify batches instead, which run one at a time
  --batch-size N        assets per Slopify batch (with --parallel 0), at most ${batchMax}; default ${defaults.batchSize}
  --price-per-image USD for the estimate when Slopify's catalogue has no price

Review (on by default: a failed image is made again)
  --no-review           turn the image reviewer off
  --review-mode MODE    redo (default) or flag
  --reviewer ID         claude-code (default) or codex
  --reviewer-model ID   default ${defaults.reviewerModel}
  --review-retries N    remakes per image, 0 to 5; default ${defaults.reviewRetries}

Output
  --out DIR             default ${defaults.out}
  --licence SPDX        the [library] licence in import.toml; default ${defaults.licence}

Slopify
  --server URL          default ${defaults.server} (or SLOPIFY_URL)
  --update-templates    overwrite Library prompts arda-* whose text differs from prompts.json
  --poll SECONDS        default ${defaults.poll}
  --timeout MINUTES     give up waiting after this long; default ${defaults.timeout}

Safety
  --dry-run             print the batch payloads as JSON on stdout; contacts nothing
  --yes                 actually submit (costs money on your provider keys)
  --prompts FILE        default tools/art-gen/prompts.json
`;

// Slopify reviews pictures only with a CLI agent that can look at them.
const reviewers = ["claude-code", "codex"];
function reviewerProvider(id) {
  if (!reviewers.includes(id)) throw new UsageError(`--reviewer must be ${reviewers.join(" or ")}; got "${id}"`);
  return id;
}
function reviewMode(mode) {
  if (mode !== "redo" && mode !== "flag") throw new UsageError(`--review-mode must be redo or flag; got "${mode}"`);
  return mode;
}

export function parseOptions(argv) {
  const { values } = parseArgs({
    args: argv,
    options: {
      class: { type: "string" },
      tier: { type: "string" },
      only: { type: "string" },
      limit: { type: "string" },
      resume: { type: "boolean", default: false },
      overwrite: { type: "boolean", default: false },
      provider: { type: "string", default: defaults.provider },
      model: { type: "string", default: defaults.model },
      thinking: { type: "string" },
      "no-review": { type: "boolean", default: false },
      "review-mode": { type: "string", default: "redo" },
      reviewer: { type: "string", default: defaults.reviewer },
      "reviewer-model": { type: "string", default: defaults.reviewerModel },
      "review-retries": { type: "string", default: String(defaults.reviewRetries) },
      channel: { type: "string", default: "DiceMaster Assets" },
      reference: { type: "string" },
      "batch-size": { type: "string", default: String(defaults.batchSize) },
      parallel: { type: "string", default: String(defaults.parallel) },
      "price-per-image": { type: "string" },
      out: { type: "string", default: defaults.out },
      licence: { type: "string", default: defaults.licence },
      server: { type: "string", default: defaults.server },
      "update-templates": { type: "boolean", default: false },
      poll: { type: "string", default: String(defaults.poll) },
      timeout: { type: "string", default: String(defaults.timeout) },
      "dry-run": { type: "boolean", default: false },
      yes: { type: "boolean", default: false },
      prompts: { type: "string", default: join(here, "prompts.json") },
      help: { type: "boolean", short: "h", default: false },
    },
    strict: true,
  });
  const number = (name, min, max) => {
    const raw = values[name];
    const n = Number(raw);
    if (!Number.isFinite(n) || n < min || (max !== undefined && n > max))
      throw new UsageError(`--${name} must be a number from ${min}${max === undefined ? "" : ` to ${max}`}`);
    return n;
  };
  const provider = providerAliases[values.provider] ?? values.provider;
  return {
    classes: values.class?.split(",").map((c) => c.trim()).filter(Boolean),
    tiers: values.tier?.split(",").map((t) => {
      const n = Number(t.trim());
      if (!Number.isInteger(n) || n < 0) throw new UsageError(`--tier takes tier numbers such as 1 or 1,2; got "${values.tier}"`);
      return n;
    }),
    only: values.only,
    limit: values.limit === undefined ? undefined : Math.floor(number("limit", 1)),
    resume: values.resume,
    overwrite: values.overwrite,
    provider,
    model: values.model,
    thinking: values.thinking,
    review: values["no-review"]
      ? undefined
      : {
          provider: reviewerProvider(values.reviewer),
          model: values["reviewer-model"],
          mode: reviewMode(values["review-mode"]),
          retries: Math.floor(number("review-retries", 0, 5)),
        },
    channel: values.channel,
    reference: values.reference,
    batchSize: Math.floor(number("batch-size", 1, batchMax)),
    parallel: Math.floor(number("parallel", 0, 20)),
    pricePerImage: values["price-per-image"] === undefined ? undefined : number("price-per-image", 0),
    out: resolve(values.out),
    licence: values.licence,
    server: values.server.replace(/\/+$/, ""),
    updateTemplates: values["update-templates"],
    poll: number("poll", 1),
    timeout: number("timeout", 1),
    dryRun: values["dry-run"],
    yes: values.yes,
    prompts: values.prompts,
    help: values.help,
  };
}

export class UsageError extends Error {}

// FLUX.1 [dev] and its derivatives ship under a non-commercial licence; their outputs are not
// safe for a library that may be redistributed or sold.
export function licenceProblem(provider, model) {
  if (/flux[^/]*dev|flux.*[-_./]dev\b/i.test(model))
    return (
      `${model} is a FLUX.1 [dev]-family model, released under the non-commercial FLUX.1 [dev] ` +
      "licence. Its outputs are not safe for Arda's art library. Pick a commercially usable model " +
      "instead, e.g. --provider openai-image --model gpt-image-2, --provider google-image " +
      "--model gemini-3.1-flash-image, or --provider replicate --model black-forest-labs/flux-1.1-pro."
    );
  if (!imageProviders.includes(provider))
    return `Unknown image provider "${provider}". Slopify's image providers are ${imageProviders.join(", ")}.`;
  return undefined;
}

export function loadPack(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

// The assets to make, in prompts.json order.
export function select(pack, options) {
  const match = options.only === undefined ? () => true : globMatcher(options.only);
  const classes = options.classes;
  if (classes !== undefined) {
    const known = new Set(Object.keys(pack.classes));
    const unknown = classes.filter((c) => !known.has(c));
    if (unknown.length) throw new UsageError(`unknown --class ${unknown.join(", ")}; known: ${[...known].join(", ")}`);
  }
  const tiers = options.tiers;
  return Object.entries(pack.assets)
    .filter(
      ([id, asset]) =>
        (classes === undefined || classes.includes(asset.class)) &&
        (tiers === undefined || tiers.includes(asset.tier ?? 0)) &&
        match(id),
    )
    .map(([id, asset]) => ({ id, ...asset }));
}

export function downloadedFile(out, id) {
  for (const ext of [".png", ".jpg"]) {
    const path = join(out, `${rawStem(id)}${ext}`);
    if (existsSync(path)) return path;
  }
  return undefined;
}

// One Slopify batch per Library prompt and frame, at most `size` items each: a batch is one
// draft, and the draft names the prompt and the aspect.
export function planBatches(assets, size) {
  const groups = new Map();
  for (const asset of assets) {
    const key = `${asset.template}|${asset.format}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(asset);
  }
  const batches = [];
  for (const members of groups.values()) {
    for (let at = 0; at < members.length; at += size) batches.push(members.slice(at, at + size));
  }
  return batches;
}

// The body of POST /api/projects/batch: a run draft with every stage but Images off (Article
// Off makes an image-only run), one Library image prompt drawn once, and one item per asset
// whose keyword values fill that prompt.
export function batchBody(batch, options, { requestId = randomUUID(), referenceId } = {}) {
  const { template, format } = batch[0];
  const draft = {
    title: `Arda art: ${template} ${format}`,
    format,
    sources: {
      research: "off",
      article: "off",
      audio: "off",
      images: "generate",
      thumbnail: "off",
      video: "off",
      document: "off",
    },
    images: {
      provider: options.provider,
      model: options.model,
      ...(options.thinking ? { thinking: options.thinking } : {}),
    },
    ...(options.channelId ? { channelId: options.channelId } : {}),
    ...(options.review
      ? {
          reviews: {
            provider: options.review.provider,
            model: options.review.model,
            retries: options.review.retries,
            stages: { images: { mode: options.review.mode, prompt: reviewPromptName } },
          },
        }
      : {}),
    imagePrompts: [{ name: template, number: 1 }],
    values: {},
    provided: {},
    silenceGapSeconds: 0,
  };
  if (options.reference !== undefined) {
    draft.reference = { source: "provide", thumbnail: false };
    draft.provided = { reference: referenceId ?? `<staged id of ${basename(options.reference)}>` };
  }
  return {
    requestId,
    draft,
    items: batch.map((asset) => ({ title: asset.id, values: { ...asset.values } })),
  };
}

// One asset as its own project. Slopify runs batch items one at a time (`pumpQueue`), while a
// project posted on its own starts at once, limited only by the provider's concurrency.
export function projectBody(asset, options, extra) {
  const { draft } = batchBody([asset], options, extra);
  return { ...draft, title: asset.id, values: { ...asset.values } };
}

export function checkValues(assets) {
  const problems = [];
  for (const asset of assets)
    for (const [name, value] of Object.entries(asset.values))
      if (value.length > valueMax || /[\n\r]/.test(value))
        problems.push(`${asset.id}: keyword ${name} must be one line of at most ${valueMax} characters`);
  return problems;
}

export function estimate(count, options) {
  const exact = knownPrices[`${options.provider}/${options.model}`] ?? knownPrices[`${options.provider}/*`];
  const each = options.pricePerImage ?? exact;
  return each === undefined
    ? { count, each: undefined, total: undefined }
    : { count, each, total: Math.round(each * count * 100) / 100 };
}

// ---------------------------------------------------------------------------------------------
// import.toml

function tomlString(text) {
  return `"${String(text)
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t")
    .replace(/[\u0000-\u001f\u007f]/g, (ch) => `\\u${ch.charCodeAt(0).toString(16).padStart(4, "0")}`)}"`;
}

export function importToml(records, options) {
  const models = [...new Set(records.map((r) => r.model))];
  const lines = [
    "# Written by tools/art-gen/generate.mjs; rewritten on every run.",
    "# Use with: arda tactical import <this folder> --out assets/tactical/ai --manifest <this file>",
    "",
    "[library]",
    `name = ${tomlString(defaults.libraryName)}`,
    `version = ${tomlString(defaults.libraryVersion)}`,
    `licence = ${tomlString(options.licence)}`,
    `tool = ${tomlString("Slopify")}`,
  ];
  if (models.length === 1) lines.push(`model = ${tomlString(models[0])}`);
  lines.push(`author = ${tomlString("tools/art-gen")}`);
  for (const r of records) {
    lines.push(
      "",
      `# Slopify project ${r.projectId}, Library prompt ${r.template}, ${r.at}`,
      "[[asset]]",
      `file = ${tomlString(r.file)}`,
      `prompt = ${tomlString(r.prompt)}`,
      `tool = ${tomlString(`Slopify (${r.provider})`)}`,
      `model = ${tomlString(r.model)}`,
      `footprint = [${r.footprint.join(", ")}]`,
    );
    if (r.structured) lines.push("structured = true");
    const m = r.meta;
    if (m !== undefined) {
      if (m.layer) lines.push(`layer = ${tomlString(m.layer)}`);
      if (Number.isInteger(m.height_ft)) lines.push(`height_ft = ${m.height_ft}`);
      if (m.cover) lines.push(`cover = ${tomlString(m.cover)}`);
      for (const flag of ["blocks_sight", "blocks_movement", "difficult_terrain"])
        if (typeof m[flag] === "boolean") lines.push(`${flag} = ${m[flag]}`);
      if (m.tags) {
        const list = (values) => `[${values.map(tomlString).join(", ")}]`;
        const fields = ["biome", "culture", "wealth", "function", "free"].map((k) => `${k} = ${list(m.tags[k] ?? [])}`);
        lines.push(`tags = { ${fields.join(", ")} }`);
      }
    }
  }
  return `${lines.join("\n")}\n`;
}

// ---------------------------------------------------------------------------------------------
// Slopify client

class SlopifyError extends Error {}

function problemText(status, body) {
  if (body && typeof body === "object") {
    const fields = Array.isArray(body.fields)
      ? body.fields
      : Array.isArray(body.extensions?.fields)
        ? body.extensions.fields
        : [];
    const list = fields.map((f) => `\n  - ${f.field}: ${f.message}`).join("");
    return `${status} ${body.title ?? ""}: ${body.detail ?? JSON.stringify(body)}${list}`;
  }
  return `${status}: ${String(body).slice(0, 500)}`;
}

// Reads are retried through brief network drops (a busy Slopify can reset a connection
// mid-run), waiting a little longer each time; writes are not.
export const readRetryDelays = [2, 5, 10, 20, 30, 60];

function client(server, delays = readRetryDelays) {
  async function call(method, path, body, { raw = false, form } = {}) {
    let response;
    for (let attempt = 0; ; attempt++) {
      try {
        response = await fetch(`${server}${path}`, {
          method,
          headers: form ? undefined : body === undefined ? undefined : { "content-type": "application/json" },
          body: form ?? (body === undefined ? undefined : JSON.stringify(body)),
        });
        break;
      } catch (error) {
        if (method !== "GET" || attempt >= delays.length)
          throw new SlopifyError(`cannot reach Slopify at ${server} (${error.cause?.code ?? error.message}). Is it running?`);
        await new Promise((done) => setTimeout(done, delays[attempt] * 1000));
      }
    }
    if (raw && response.ok) return response;
    const text = await response.text();
    // Slopify refuses everything while it installs an update and then restarts: nothing was
    // done, so the same request is safe to send again once it is back.
    if (response.status === 409 && /is updating/i.test(text) && (call.updateWaits = (call.updateWaits ?? 0) + 1) <= 40) {
      await new Promise((done) => setTimeout(done, 15_000));
      return call(method, path, body, { raw, form });
    }
    let parsed = text;
    try {
      parsed = text === "" ? undefined : JSON.parse(text);
    } catch {}
    if (!response.ok) throw new SlopifyError(`${method} ${path}: ${problemText(response.status, parsed)}`);
    return parsed;
  }
  return {
    get: (path, opts) => call("GET", path, undefined, opts),
    post: (path, body, opts) => call("POST", path, body, opts),
    put: (path, body) => call("PUT", path, body),
  };
}

// The Library prompts the batches name: created when missing; when the text differs, replaced
// only with --update-templates, because the prompt text is the art's provenance.
async function ensureTemplates(api, pack, names, options, log) {
  const { prompts } = await api.get("/api/prompts");
  const wanted = names.map((name) => ({ kind: "image", name, body: pack.templates[name].body }));
  if (options.review) wanted.push({ kind: "review", name: reviewPromptName, body: reviewInstruction });
  for (const { kind, name, body: want } of wanted) {
    const have = prompts.find((p) => p.kind === kind && p.name === name);
    if (have === undefined) {
      await api.post("/api/prompts", { kind, name, body: want });
      log(`created Library ${kind} prompt "${name}"`);
    } else if (have.body !== want) {
      if (!options.updateTemplates)
        throw new SlopifyError(
          `Library ${kind} prompt "${name}" differs from ${kind === "review" ? "generate.mjs" : "prompts.json"}. Pass --update-templates to overwrite it, or rename yours.`,
        );
      await api.put(`/api/prompts/${have.id}`, { kind, name, body: want });
      log(`updated Library ${kind} prompt "${name}"`);
    }
  }
}

async function checkModel(api, options, log) {
  if (options.provider === "codex-image") return;
  const listed = await api.get(`/api/providers/${options.provider}/models`);
  const model = listed.models?.find((m) => m.id === options.model);
  if (model === undefined)
    throw new SlopifyError(
      `Slopify's model list has no ${options.provider} model "${options.model}". It offers: ${(listed.models ?? []).map((m) => m.id).join(", ")}`,
    );
  if (options.reference !== undefined && Array.isArray(model.keywords) && !model.keywords.includes("reference"))
    throw new SlopifyError(`${options.model} cannot take a reference image; drop --reference or pick another model.`);
  const { providers } = await api.get("/api/providers");
  const readiness = providers?.find((p) => p.id === options.provider)?.readiness;
  if (readiness?.kind === "keyed" && readiness.hasKey === false)
    throw new SlopifyError(`${options.provider} has no API key in Slopify. Add one in Settings → Providers.`);
  log(`provider ${options.provider} is ready`);
}

async function uploadReference(api, path) {
  const form = new FormData();
  form.append("file", new Blob([readFileSync(path)]), basename(path));
  const staged = await api.post("/api/staging/images", undefined, { form });
  return staged.id;
}

function sniff(bytes) {
  if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return ".png";
  if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return ".jpg";
  if (bytes.subarray(0, 4).toString("latin1") === "RIFF" && bytes.subarray(8, 12).toString("latin1") === "WEBP")
    return ".webp";
  return ".bin";
}

// ---------------------------------------------------------------------------------------------
// State: what was submitted, so a crash or Ctrl-C never pays for the same image twice.

function stateFile(out) {
  return join(out, ".art-gen-state.json");
}
function readState(out) {
  const path = stateFile(out);
  return existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : { assets: {}, batches: {} };
}
function writeState(out, state) {
  mkdirSync(out, { recursive: true });
  const path = stateFile(out);
  writeFileSync(`${path}.tmp`, `${JSON.stringify(state, null, 2)}\n`);
  renameSync(`${path}.tmp`, path);
}

// The catalogue metadata (layer, height, cover, blocking, tags) comes from prompts.json at write
// time, so a manifest rewritten after a prompts.json update carries the current metadata.
function writeManifest(out, state, options, pack) {
  const records = Object.entries(state.assets)
    .filter(([, r]) => r.file !== undefined && existsSync(join(out, r.file)))
    .map(([id, r]) => ({ ...r, meta: pack?.assets[id] }))
    .sort((a, b) => a.file.localeCompare(b.file));
  writeFileSync(join(out, "import.toml"), importToml(records, options));
}

const finished = new Set(["done", "partial", "failed", "canceled"]);

async function collect(api, out, state, ids, options, log, pack, feed = async () => {}) {
  const deadline = Date.now() + options.timeout * 60_000;
  const pending = new Set(ids);
  await feed(pending);
  while (pending.size > 0) {
    for (const id of [...pending]) {
      const record = state.assets[id];
      const view = await api.get(`/api/projects/${record.projectId}`);
      const status = view.project?.status;
      if (!finished.has(status)) continue;
      pending.delete(id);
      const image = (view.outputs ?? []).find((o) => o.role === "image");
      if (image === undefined) {
        const reason = (view.stages ?? []).find((s) => s.kind === "images")?.failureReason ?? status;
        record.status = "failed";
        record.error = reason;
        log(`FAILED ${id}: ${reason}`);
        writeState(out, state);
        continue;
      }
      const asset = `image-${image.meta?.index ?? 1}`;
      const response = await api.get(`/files/${record.projectId}/${asset}`, { raw: true });
      const bytes = Buffer.from(await response.arrayBuffer());
      const ext = sniff(bytes);
      const file = `${rawStem(id)}${ext}`;
      writeFileSync(join(out, file), bytes);
      if (ext === ".webp" || ext === ".bin")
        log(`WARNING ${id}: saved ${file}, which arda tactical import cannot read; convert it to PNG.`);
      Object.assign(record, {
        status: "downloaded",
        file,
        prompt: image.meta?.prompt ?? record.prompt,
        provider: image.meta?.provider ?? record.provider,
        model: image.meta?.model ?? record.model,
      });
      writeState(out, state);
      writeManifest(out, state, options, pack);
      log(`saved ${file} (${pending.size} still waiting)`);
    }
    await feed(pending);
    if (pending.size === 0) break;
    if (Date.now() > deadline) {
      log(`gave up waiting after ${options.timeout} min; run again with --resume to pick up ${pending.size} still running.`);
      return false;
    }
    await sleep(options.poll * 1000);
  }
  return true;
}

// ---------------------------------------------------------------------------------------------

export async function main(argv, io = { out: process.stdout, err: process.stderr }) {
  const log = (line) => io.err.write(`${line}\n`);
  const options = parseOptions(argv);
  if (options.help) {
    io.out.write(usage);
    return 0;
  }
  const refusal = licenceProblem(options.provider, options.model);
  if (refusal !== undefined) {
    log(`REFUSED: ${refusal}`);
    return 2;
  }
  if (options.reference !== undefined && !existsSync(options.reference))
    throw new UsageError(`--reference ${options.reference} does not exist`);

  const pack = loadPack(options.prompts);
  const state = options.dryRun ? { assets: {}, batches: {} } : readState(options.out);
  let assets = select(pack, options);
  const downloaded = assets.filter((a) => downloadedFile(options.out, a.id) !== undefined);
  if (downloaded.length && options.resume) {
    assets = assets.filter((a) => downloadedFile(options.out, a.id) === undefined);
    log(`--resume: skipping ${downloaded.length} already downloaded`);
  } else if (downloaded.length && !options.overwrite && !options.dryRun) {
    log(
      `${downloaded.length} of these are already in ${options.out} (e.g. ${downloaded[0].id}). ` +
        "Pass --resume to skip them or --overwrite to make them again.",
    );
    return 1;
  }
  // Submitted earlier and still running or done but not yet downloaded: wait for those
  // rather than paying for them again; --overwrite means make them again regardless.
  const inFlight = options.dryRun || options.overwrite
    ? []
    : assets.filter((a) => state.assets[a.id]?.projectId && !["failed", "downloaded"].includes(state.assets[a.id].status));
  // A batch sent but never confirmed (the connection dropped, or the script stopped): it is
  // sent again with its own requestId, which Slopify answers with the queue it already made.
  const unconfirmed = Object.entries(state.batches).filter(([, b]) => !b.confirmed);
  const recovering = new Set(unconfirmed.flatMap(([, b]) => b.ids));
  const toSubmit = assets.filter((a) => !inFlight.includes(a) && !recovering.has(a.id));
  const limited = options.limit === undefined ? toSubmit : toSubmit.slice(0, options.limit);

  const problems = checkValues(limited);
  if (problems.length) {
    log(problems.join("\n"));
    return 1;
  }
  const batches = planBatches(limited, options.batchSize);
  const cost = estimate(limited.length, options);
  const byClass = {};
  const byTier = {};
  for (const a of limited) {
    byClass[a.class] = (byClass[a.class] ?? 0) + 1;
    byTier[a.tier ?? 0] = (byTier[a.tier ?? 0] ?? 0) + 1;
  }
  log(`Slopify ${options.server}, ${options.provider} / ${options.model}${options.reference ? `, reference ${options.reference}` : ""}`);
  log(
    `${limited.length} images in ${batches.length} batch${batches.length === 1 ? "" : "es"} (${Object.entries(byClass)
      .map(([c, n]) => `${c} ${n}`)
      .join(", ") || "nothing"}; tier ${Object.entries(byTier)
      .map(([t, n]) => `${t}: ${n}`)
      .join(", ") || "none"})${inFlight.length ? `; ${inFlight.length} submitted earlier will be collected` : ""}${
      recovering.size ? `; ${recovering.size} in ${unconfirmed.length} unconfirmed earlier batch(es) will be re-sent with their original requestId` : ""
    }`,
  );
  log(
    cost.total === undefined
      ? `Estimated cost: unknown. Slopify's catalogue has no fixed price for ${options.model}; pass --price-per-image to estimate.`
      : `Estimated cost: ${limited.length} × $${cost.each} = $${cost.total} (provider list price; check your account)`,
  );
  for (const batch of batches) log(`  batch: ${batch.length} × ${batch[0].template} ${batch[0].format} (${batch[0].id} …)`);

  if (options.dryRun) {
    const payloads = batches.map((batch) => ({
      method: "POST",
      path: "/api/projects/batch",
      body: batchBody(batch, options, { requestId: "00000000-0000-4000-8000-000000000000" }),
    }));
    io.out.write(
      `${JSON.stringify({ templates: Object.fromEntries([...new Set(limited.map((a) => a.template))].map((n) => [n, pack.templates[n]])), payloads }, null, 2)}\n`,
    );
    log("Dry run: nothing was sent.");
    return 0;
  }
  if (!options.yes) {
    log("Nothing sent. Re-run with --yes to submit (this spends money on your provider key).");
    return 0;
  }

  const api = client(options.server);
  const health = await api.get("/api/health");
  log(`Slopify ${health.version ?? "?"} is up`);
  // Every asset run belongs in a channel; without one Slopify files it under the default.
  if (!options.channel) throw new UsageError("--channel must name a Slopify channel (default DiceMaster Assets)");
  {
    const { channels = [] } = await api.get("/api/channels");
    const found = channels.find((ch) => ch.name === options.channel);
    if (!found) throw new Error(`Slopify has no channel named "${options.channel}"; create it in Channels or pass --channel`);
    options.channelId = found.id;
    log(`runs go to channel "${found.name}"`);
  }
  mkdirSync(options.out, { recursive: true });
  if (limited.length > 0 || unconfirmed.length > 0) {
    await checkModel(api, options, log);
    await ensureTemplates(
      api,
      pack,
      [...new Set([...limited, ...[...recovering].map((id) => pack.assets[id])].map((a) => a.template))],
      options,
      log,
    );
  }
  const referenceId =
    options.reference !== undefined && limited.length > 0 ? await uploadReference(api, options.reference) : undefined;
  if (referenceId !== undefined) log(`uploaded the reference as staged file ${referenceId}`);

  if (options.overwrite)
    for (const a of limited) {
      const old = downloadedFile(options.out, a.id);
      if (old === undefined) continue;
      const aside = `${options.out}-replaced`;
      mkdirSync(aside, { recursive: true });
      renameSync(old, join(aside, `${basename(old)}.${Date.now()}`));
    }

  if (options.parallel > 0 && unconfirmed.length === 0) {
    const waiting = [...limited];
    // Items an earlier run left in Slopify's one-at-a-time batch queue: those not started yet
    // are canceled and started again as their own projects, so they run side by side.
    const collecting = [];
    for (const asset of inFlight) {
      const record = state.assets[asset.id];
      const view = record.requestId === undefined ? undefined : await api.get(`/api/projects/${record.projectId}`);
      if (view?.project?.status !== "pending") {
        collecting.push(asset.id);
        continue;
      }
      await api.post(`/api/projects/${record.projectId}/cancel`, {
        baseRevisionId: view.revisionId,
        idempotencyKey: randomUUID(),
      });
      record.status = "failed";
      record.error = "moved out of the batch queue";
      waiting.push(asset);
    }
    if (waiting.length > limited.length)
      log(`moved ${waiting.length - limited.length} waiting batch items to run ${options.parallel} at a time`);
    writeState(options.out, state);
    const feed = async (pending) => {
      while (pending.size < options.parallel && waiting.length > 0) {
        const asset = waiting.shift();
        const body = projectBody(asset, options, { referenceId });
        const reply = await api.post("/api/projects", body);
        const projectId = reply.project?.id;
        if (projectId === undefined || reply.project.title !== asset.id)
          throw new SlopifyError(`project for ${asset.id} came back as ${JSON.stringify(reply.project ?? reply).slice(0, 200)}`);
        if (options.channelId && reply.project.channelId !== undefined && reply.project.channelId !== options.channelId)
          throw new SlopifyError(`project for ${asset.id} landed in channel ${reply.project.channelId}, not "${options.channel}"; stopping.`);
        state.assets[asset.id] = {
          projectId,
          template: asset.template,
          provider: body.images.provider,
          model: body.images.model,
          prompt: asset.prompt,
          footprint: asset.footprint,
          structured: asset.structured === true,
          status: "submitted",
          at: new Date().toISOString(),
        };
        writeState(options.out, state);
        pending.add(asset.id);
        log(`started ${asset.id} (${waiting.length} not started yet)`);
      }
    };
    const complete = await collect(api, options.out, state, collecting, options, log, pack, feed);
    writeManifest(options.out, state, options, pack);
    const failed = Object.entries(state.assets).filter(([, r]) => r.status === "failed");
    log(`import.toml written to ${join(options.out, "import.toml")}`);
    if (failed.length) log(`${failed.length} failed; run again with --resume to retry them.`);
    return complete && failed.length === 0 ? 0 : 1;
  }

  const submitted = [];
  const sends = [
    ...unconfirmed.map(([, b]) => ({ body: b.body, batch: b.ids.map((id) => ({ id, ...pack.assets[id] })) })),
    ...batches.map((batch) => ({ body: batchBody(batch, options, { referenceId }), batch })),
  ];
  for (const { body, batch } of sends) {
    // Saved before sending: Slopify treats a repeated requestId as the same batch, so a retry
    // after a dropped connection never queues it twice.
    state.batches[body.requestId] = { at: new Date().toISOString(), ids: batch.map((a) => a.id), body, confirmed: false };
    writeState(options.out, state);
    let reply;
    for (let attempt = 1; ; attempt += 1) {
      try {
        reply = await api.post("/api/projects/batch", body);
        break;
      } catch (error) {
        if (attempt >= 3 || !/cannot reach/.test(error.message)) throw error;
        await sleep(2000 * attempt);
      }
    }
    const queue = [...reply.queue].sort((a, b) => a.position - b.position);
    if (queue.length !== batch.length)
      throw new SlopifyError(`batch ${body.requestId}: sent ${batch.length} items, Slopify queued ${queue.length}`);
    batch.forEach((asset, at) => {
      state.assets[asset.id] = {
        projectId: queue[at].projectId,
        requestId: body.requestId,
        template: asset.template,
        provider: body.draft.images.provider,
        model: body.draft.images.model,
        prompt: asset.prompt,
        footprint: asset.footprint,
        structured: asset.structured === true,
        status: "submitted",
        at: new Date().toISOString(),
      };
      submitted.push(asset.id);
    });
    state.batches[body.requestId].confirmed = true;
    writeState(options.out, state);
    log(`queued ${batch.length} × ${batch[0].template} (${body.requestId})`);
  }

  // Slopify names each project after the item's title, which is the asset id: check the
  // mapping before anything is saved under that name.
  for (const id of submitted) {
    const view = await api.get(`/api/projects/${state.assets[id].projectId}`);
    if (view.project?.title !== id)
      throw new SlopifyError(`project ${state.assets[id].projectId} is titled "${view.project?.title}", expected ${id}`);
  }

  const complete = await collect(api, options.out, state, [...inFlight.map((a) => a.id), ...submitted], options, log, pack);
  writeManifest(options.out, state, options, pack);
  const failed = Object.entries(state.assets).filter(([, r]) => r.status === "failed");
  log(`import.toml written to ${join(options.out, "import.toml")}`);
  if (failed.length) log(`${failed.length} failed; run again with --resume to retry them.`);
  return complete && failed.length === 0 ? 0 : 1;
}

const invoked = process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invoked) {
  // exitCode, not exit(): a large --dry-run payload is still flushing to a pipe when main returns.
  main(process.argv.slice(2)).then(
    (code) => {
      process.exitCode = code;
    },
    (error) => {
      process.stderr.write(`${error instanceof UsageError || error instanceof SlopifyError ? "" : "error: "}${error.message}\n`);
      if (error instanceof UsageError) process.stderr.write("Run with --help for usage.\n");
      process.exitCode = 1;
    },
  );
}
