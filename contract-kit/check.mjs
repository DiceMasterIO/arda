#!/usr/bin/env node
// Replays every case of the Arda contract kit against a running arda-server and compares
// status, content type, the listed headers and the body (golden bytes, or SHA-256 and size).
// Node 22 or later, standard library only.
//
//   node contract-kit/check.mjs --base http://127.0.0.1:8787
//   node contract-kit/check.mjs --base http://127.0.0.1:8787 --only tactical-cell
//
// Exit 0 when every case matches, 1 on any difference, 2 when the server cannot be reached
// or the kit cannot be read. See README.md.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));

/** The kit format this checker reads. */
export const KIT_FORMAT = 1;

export class UnreachableError extends Error {}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function readKit(kitDir) {
  const manifest = JSON.parse(readFileSync(join(kitDir, "manifest.json"), "utf8"));
  if (manifest.kit_format !== KIT_FORMAT) {
    throw new Error(`manifest kit_format ${manifest.kit_format}; this checker reads ${KIT_FORMAT}`);
  }
  return manifest;
}

// A JSON value's path segment, `$.a.b[3]` style.
function step(path, key) {
  return typeof key === "number" ? `${path}[${key}]` : /^[A-Za-z_][A-Za-z0-9_]*$/.test(key) ? `${path}.${key}` : `${path}[${JSON.stringify(key)}]`;
}

function kind(v) {
  return v === null ? "null" : Array.isArray(v) ? "array" : typeof v;
}

/** The first path where two JSON values differ, with both sides, or null when equal. */
export function firstDifference(want, got, path = "$") {
  if (kind(want) !== kind(got)) return { path, want, got };
  if (Array.isArray(want)) {
    const n = Math.min(want.length, got.length);
    for (let i = 0; i < n; i += 1) {
      const d = firstDifference(want[i], got[i], step(path, i));
      if (d) return d;
    }
    if (want.length !== got.length) return { path: `${path}.length`, want: want.length, got: got.length };
    return null;
  }
  if (kind(want) === "object") {
    const keys = [...new Set([...Object.keys(want), ...Object.keys(got)])];
    for (const k of keys) {
      if (!(k in got)) return { path: step(path, k), want: want[k], got: "(missing)" };
      if (!(k in want)) return { path: step(path, k), want: "(absent)", got: got[k] };
      const d = firstDifference(want[k], got[k], step(path, k));
      if (d) return d;
    }
    return null;
  }
  return Object.is(want, got) ? null : { path, want, got };
}

function short(v) {
  const text = typeof v === "string" ? JSON.stringify(v) : JSON.stringify(v) ?? String(v);
  return text.length > 120 ? `${text.slice(0, 117)}...` : text;
}

// Why the golden bytes and the served bytes differ, naming the first JSON path when both parse.
function bodyDifference(want, got) {
  try {
    const d = firstDifference(JSON.parse(want.toString("utf8")), JSON.parse(got.toString("utf8")));
    if (d) return `body differs at ${d.path}: want ${short(d.want)}, got ${short(d.got)}`;
    return `body bytes differ (${want.length} vs ${got.length} bytes) but the JSON is equal: key order or number formatting`;
  } catch {
    let i = 0;
    while (i < want.length && i < got.length && want[i] === got[i]) i += 1;
    return `body differs at byte ${i} (${want.length} bytes wanted, ${got.length} got)`;
  }
}

function requestOf(c, kitDir, byId) {
  const headers = {};
  let body;
  if (c.request) {
    headers["content-type"] = c.request.content_type;
    body = readFileSync(join(kitDir, c.request.body));
  }
  // Without a recorded ETag the request goes unconditional, and the case fails on its status.
  const etag = c.if_none_match_of ? byId.get(c.if_none_match_of)?.headers?.etag : null;
  if (etag) headers["if-none-match"] = etag;
  return { method: c.method, headers, body };
}

/** Checks one case's response; returns the list of differences (empty when it matches). */
export function compare(c, kitDir, response) {
  const problems = [];
  if (response.status !== c.status) problems.push(`status ${response.status}, want ${c.status}`);
  const type = response.headers.get("content-type");
  if (type !== (c.content_type ?? null)) problems.push(`content-type ${short(type)}, want ${short(c.content_type ?? null)}`);
  for (const [name, want] of Object.entries(c.headers ?? {})) {
    const got = response.headers.get(name);
    if (got !== want) problems.push(`header ${name} ${short(got)}, want ${short(want)}`);
  }
  const got = response.body;
  if (c.golden) {
    let want;
    try {
      want = readFileSync(join(kitDir, c.golden));
    } catch {
      problems.push(`missing golden ${c.golden}`);
      return problems;
    }
    if (!want.equals(got)) problems.push(bodyDifference(want, got));
  } else if (c.body) {
    const hash = sha256(got);
    if (got.length !== c.body.bytes || hash !== c.body.sha256) {
      problems.push(`body sha256 ${hash.slice(0, 16)}… (${got.length} bytes), want ${String(c.body.sha256).slice(0, 16)}… (${c.body.bytes} bytes)`);
    }
  } else {
    problems.push("case names neither a golden nor a body hash");
  }
  return problems;
}

async function send(base, c, kitDir, byId, timeoutMs) {
  const { method, headers, body } = requestOf(c, kitDir, byId);
  let response;
  try {
    response = await fetch(new URL(c.uri, base), {
      method,
      headers,
      body,
      redirect: "manual",
      signal: AbortSignal.timeout(timeoutMs),
    });
  } catch (e) {
    const reason = e?.cause?.code ?? e?.cause?.message ?? e?.message ?? String(e);
    throw new UnreachableError(`${method} ${c.uri}: ${reason}`);
  }
  const bytes = Buffer.from(await response.arrayBuffer());
  return { status: response.status, headers: response.headers, body: bytes };
}

/**
 * Replays the kit against `base`. Returns `{results: [{id, ok, problems, ms}], passed, failed}`;
 * throws `UnreachableError` when a request cannot be sent at all.
 */
export async function run({ base, kitDir = here, only, timeoutMs = 300_000, log = () => {} }) {
  const manifest = readKit(kitDir);
  const byId = new Map(manifest.cases.map((c) => [c.id, c]));
  const cases = only ? manifest.cases.filter((c) => only.includes(c.id)) : manifest.cases;
  if (only) {
    for (const id of only) if (!byId.has(id)) throw new Error(`no case named ${id}`);
  }
  const results = [];
  for (const c of cases) {
    const start = performance.now();
    const response = await send(base, c, kitDir, byId, timeoutMs);
    const problems = compare(c, kitDir, response);
    const ms = performance.now() - start;
    results.push({ id: c.id, ok: problems.length === 0, problems, ms });
    log(problems.length === 0 ? `ok   ${c.id} (${ms.toFixed(0)} ms)` : `FAIL ${c.id}: ${c.method} ${c.uri}\n${problems.map((p) => `       ${p}`).join("\n")}`);
  }
  const failed = results.filter((r) => !r.ok).length;
  return { results, passed: results.length - failed, failed, fixture: manifest.fixture };
}

async function main() {
  const { values } = parseArgs({
    options: {
      base: { type: "string", default: "http://127.0.0.1:8787" },
      kit: { type: "string", default: here },
      only: { type: "string", multiple: true },
      "timeout-s": { type: "string", default: "300" },
      help: { type: "boolean", short: "h" },
    },
  });
  if (values.help) {
    console.log("usage: node contract-kit/check.mjs [--base URL] [--kit DIR] [--only CASE]... [--timeout-s N]");
    return 0;
  }
  const kitDir = resolve(values.kit);
  let summary;
  try {
    summary = await run({
      base: values.base,
      kitDir,
      only: values.only,
      timeoutMs: Number(values["timeout-s"]) * 1000,
      log: (line) => console.log(line),
    });
  } catch (e) {
    if (e instanceof UnreachableError) {
      console.error(`check.mjs: cannot reach ${values.base}: ${e.message}`);
    } else {
      console.error(`check.mjs: ${e.message}`);
    }
    return 2;
  }
  const f = summary.fixture;
  console.log(
    `${summary.passed} of ${summary.results.length} cases match (seed ${f.seed}, arda ${f.arda_version}, server ${f.server_version}, library ${f.library_version}); ${summary.failed} differ`,
  );
  return summary.failed === 0 ? 0 : 1;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  process.exitCode = await main();
}
