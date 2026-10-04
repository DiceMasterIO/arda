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
import { globMatcher } from "./lib.mjs";

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

const usage = `Usage: node tools/art-gen/generate.mjs [options]

Selection
  --class LIST          ground, water, wall, prop, vegetation (comma-separated)
  --only GLOB           asset ids, e.g. "ground.grass.*" or "wall.stone.*,prop.barrel"
  --limit N             at most N assets (after the other filters)
  --resume              skip assets already downloaded to --out
  --overwrite           regenerate assets already downloaded (the old file moves to <out>/../<out>-replaced/)

Generation
  --provider ID         ${imageProviders.join(", ")} (aliases: openai, google, codex); default ${defaults.provider}
  --model ID            default ${defaults.model}
  --reference FILE      a style reference, sent as Slopify's establishing image
  --batch-size N        assets per Slopify batch, at most ${batchMax}; default ${defaults.batchSize}
  --price-per-image USD for the estimate when Slopify's catalogue has no price

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

export function parseOptions(argv) {
  const { values } = parseArgs({
    args: argv,
    options: {
      class: { type: "string" },
      only: { type: "string" },
      limit: { type: "string" },
      resume: { type: "boolean", default: false },
      overwrite: { type: "boolean", default: false },
      provider: { type: "string", default: defaults.provider },
      model: { type: "string", default: defaults.model },
      thinking: { type: "string" },
      channel: { type: "string", default: "DiceMaster Assets" },
      reference: { type: "string" },
      "batch-size": { type: "string", default: String(defaults.batchSize) },
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
    only: values.only,
    limit: values.limit === undefined ? undefined : Math.floor(number("limit", 1)),
    resume: values.resume,
    overwrite: values.overwrite,
    provider,
    model: values.model,
    thinking: values.thinking,
    channel: values.channel,
    reference: values.reference,
    batchSize: Math.floor(number("batch-size", 1, batchMax)),
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
  return Object.entries(pack.assets)
    .filter(([id, asset]) => (classes === undefined || classes.includes(asset.class)) && match(id))
    .map(([id, asset]) => ({ id, ...asset }));
}

export function downloadedFile(out, id) {
  for (const ext of [".png", ".jpg"]) {
    const path = join(out, `${id}${ext}`);
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

function client(server) {
  async function call(method, path, body, { raw = false, form } = {}) {
    let response;
    try {
      response = await fetch(`${server}${path}`, {
        method,
        headers: form ? undefined : body === undefined ? undefined : { "content-type": "application/json" },
        body: form ?? (body === undefined ? undefined : JSON.stringify(body)),
      });
    } catch (error) {
      throw new SlopifyError(`cannot reach Slopify at ${server} (${error.cause?.code ?? error.message}). Is it running?`);
    }
    if (raw && response.ok) return response;
    const text = await response.text();
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
  for (const name of names) {
    const want = pack.templates[name].body;
    const have = prompts.find((p) => p.kind === "image" && p.name === name);
    if (have === undefined) {
      await api.post("/api/prompts", { kind: "image", name, body: want });
      log(`created Library image prompt "${name}"`);
    } else if (have.body !== want) {
      if (!options.updateTemplates)
        throw new SlopifyError(
          `Library image prompt "${name}" differs from prompts.json. Pass --update-templates to overwrite it, or rename yours.`,
        );
      await api.put(`/api/prompts/${have.id}`, { kind: "image", name, body: want });
      log(`updated Library image prompt "${name}"`);
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

function writeManifest(out, state, options) {
  const records = Object.values(state.assets)
    .filter((r) => r.file !== undefined && existsSync(join(out, r.file)))
    .sort((a, b) => a.file.localeCompare(b.file));
  writeFileSync(join(out, "import.toml"), importToml(records, options));
}

const finished = new Set(["done", "partial", "failed", "canceled"]);

async function collect(api, out, state, ids, options, log) {
  const deadline = Date.now() + options.timeout * 60_000;
  const pending = new Set(ids);
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
      const file = `${id}${ext}`;
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
      writeManifest(out, state, options);
      log(`saved ${file} (${pending.size} still waiting)`);
    }
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
  // rather than paying for them again.
  const inFlight = options.dryRun
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
  for (const a of limited) byClass[a.class] = (byClass[a.class] ?? 0) + 1;
  log(`Slopify ${options.server}, ${options.provider} / ${options.model}${options.reference ? `, reference ${options.reference}` : ""}`);
  log(
    `${limited.length} images in ${batches.length} batch${batches.length === 1 ? "" : "es"} (${Object.entries(byClass)
      .map(([c, n]) => `${c} ${n}`)
      .join(", ") || "nothing"})${inFlight.length ? `; ${inFlight.length} submitted earlier will be collected` : ""}${
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
  if (options.channel) {
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

  const complete = await collect(api, options.out, state, [...inFlight.map((a) => a.id), ...submitted], options, log);
  writeManifest(options.out, state, options);
  const failed = Object.entries(state.assets).filter(([, r]) => r.status === "failed");
  log(`import.toml written to ${join(options.out, "import.toml")}`);
  if (failed.length) log(`${failed.length} failed; run again with --resume to retry them.`);
  return complete && failed.length === 0 ? 0 : 1;
}

const invoked = process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invoked) {
  main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (error) => {
      process.stderr.write(`${error instanceof UsageError || error instanceof SlopifyError ? "" : "error: "}${error.message}\n`);
      if (error instanceof UsageError) process.stderr.write("Run with --help for usage.\n");
      process.exit(1);
    },
  );
}
