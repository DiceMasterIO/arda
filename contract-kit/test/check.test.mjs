// Tests for contract-kit/check.mjs against a local node:http stub. Run with:
//
//   node --test contract-kit/test/
//
// The stub serves a three-case kit written to a temporary directory: a JSON golden, a hashed
// binary with an ETag and its 304 revalidation, and a POST with a request body.

import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { firstDifference, run, sha256, UnreachableError } from "../check.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const checker = resolve(here, "../check.mjs");
const run$ = promisify(execFile);

const world = Buffer.from('{"seed":"42","tiles":{"max_zoom":4,"tile_px":256},"versions":[1,2,3]}');
const png = Buffer.from("89504e470d0a1a0a0000000d49484452", "hex");
const etag = '"abc123"';
const posted = Buffer.from('{"gx":1,"gy":2}');
const accepted = Buffer.from('{"queue_len":0,"radius":1}');

// What the stub serves; tests swap entries to provoke a mismatch.
let served;

function reset() {
  served = { world, png, accepted, etag };
}

function stub() {
  return createServer((req, res) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      const url = new URL(req.url, "http://x");
      if (req.method === "GET" && url.pathname === "/v1/world") {
        res.writeHead(200, { "content-type": "application/json" }).end(served.world);
      } else if (req.method === "GET" && url.pathname === "/v1/image.png") {
        if (req.headers["if-none-match"] === served.etag) {
          res.writeHead(304, { etag: served.etag, "cache-control": "no-cache" }).end();
        } else {
          res.writeHead(200, { "content-type": "image/png", etag: served.etag, "cache-control": "no-cache" }).end(served.png);
        }
      } else if (req.method === "POST" && url.pathname === "/v1/post") {
        const ok = req.headers["content-type"] === "application/json" && Buffer.concat(chunks).equals(posted);
        res.writeHead(ok ? 202 : 400, { "content-type": "application/json" }).end(ok ? served.accepted : "{}");
      } else {
        res.writeHead(404, { "content-type": "application/json" }).end('{"error":{"code":"not_found"}}');
      }
    });
  });
}

let kit;
let server;
let base;

before(async () => {
  kit = mkdtempSync(join(tmpdir(), "arda-contract-kit-test-"));
  mkdirSync(join(kit, "golden"));
  mkdirSync(join(kit, "requests"));
  writeFileSync(join(kit, "golden/world.json"), world);
  writeFileSync(join(kit, "golden/post.json"), accepted);
  writeFileSync(join(kit, "requests/post.json"), posted);
  const manifest = {
    kit_format: 1,
    fixture: { seed: "42", arda_version: "0.0.0", server_version: "0.0.0", library_version: "test" },
    cases: [
      { id: "world", route: "GET /v1/world", method: "GET", uri: "/v1/world", status: 200, content_type: "application/json", headers: {}, schema: "WorldInfo", golden: "golden/world.json" },
      { id: "image", route: "GET /v1/image.png", method: "GET", uri: "/v1/image.png", status: 200, content_type: "image/png", headers: { etag, "cache-control": "no-cache" }, schema: null, body: { sha256: sha256(png), bytes: png.length } },
      { id: "image-304", route: "GET /v1/image.png", method: "GET", uri: "/v1/image.png", if_none_match_of: "image", status: 304, content_type: null, headers: { etag }, schema: null, body: { sha256: sha256(Buffer.alloc(0)), bytes: 0 } },
      { id: "post", route: "POST /v1/post", method: "POST", uri: "/v1/post", request: { content_type: "application/json", body: "requests/post.json" }, status: 202, content_type: "application/json", headers: {}, schema: null, golden: "golden/post.json" },
    ],
  };
  writeFileSync(join(kit, "manifest.json"), JSON.stringify(manifest, null, 2));
  reset();
  server = stub();
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  base = `http://127.0.0.1:${server.address().port}`;
});

after(() => {
  server?.close();
  rmSync(kit, { recursive: true, force: true });
});

test("every case passes against a server that serves the kit", async () => {
  reset();
  const summary = await run({ base, kitDir: kit });
  assert.deepEqual(
    summary.results.map((r) => [r.id, r.problems]),
    [
      ["world", []],
      ["image", []],
      ["image-304", []],
      ["post", []],
    ],
  );
  assert.equal(summary.failed, 0);
  const cli = await run$(process.execPath, [checker, "--base", base, "--kit", kit]);
  assert.match(cli.stdout, /4 of 4 cases match \(seed 42, arda 0\.0\.0/);
});

test("a changed JSON value fails its case and names the first differing path", async () => {
  reset();
  served.world = Buffer.from(world.toString().replace('"max_zoom":4', '"max_zoom":5'));
  const summary = await run({ base, kitDir: kit });
  const failed = summary.results.filter((r) => !r.ok);
  assert.deepEqual(failed.map((r) => r.id), ["world"]);
  assert.match(failed[0].problems[0], /^body differs at \$\.tiles\.max_zoom: want 4, got 5$/);
  await assert.rejects(run$(process.execPath, [checker, "--base", base, "--kit", kit]), (e) => {
    assert.equal(e.code, 1);
    assert.match(e.stdout, /FAIL world: GET \/v1\/world/);
    assert.match(e.stdout, /3 of 4 cases match/);
    return true;
  });
});

test("a changed image byte or ETag fails the hashed case and its revalidation", async () => {
  reset();
  served.png = Buffer.from(png);
  served.png[15] ^= 1;
  served.etag = '"abc124"';
  const summary = await run({ base, kitDir: kit });
  const failed = Object.fromEntries(summary.results.filter((r) => !r.ok).map((r) => [r.id, r.problems]));
  assert.deepEqual(Object.keys(failed), ["image", "image-304"]);
  assert.ok(failed.image.some((p) => p.startsWith("header etag")));
  assert.ok(failed.image.some((p) => p.startsWith("body sha256")));
  assert.ok(failed["image-304"].some((p) => p.startsWith("status 200, want 304")));
});

test("an unreachable server exits 2 without reporting cases", async () => {
  const closed = createServer();
  await new Promise((ok) => closed.listen(0, "127.0.0.1", ok));
  const dead = `http://127.0.0.1:${closed.address().port}`;
  await new Promise((ok) => closed.close(ok));
  await assert.rejects(run({ base: dead, kitDir: kit }), UnreachableError);
  await assert.rejects(run$(process.execPath, [checker, "--base", dead, "--kit", kit]), (e) => {
    assert.equal(e.code, 2);
    assert.match(e.stderr, /cannot reach/);
    return true;
  });
});

test("firstDifference walks objects and arrays", () => {
  assert.equal(firstDifference({ a: [1, { b: 2 }] }, { a: [1, { b: 2 }] }), null);
  assert.deepEqual(firstDifference({ a: [1, { b: 2 }] }, { a: [1, { b: 3 }] }), { path: "$.a[1].b", want: 2, got: 3 });
  assert.deepEqual(firstDifference([1, 2], [1]), { path: "$.length", want: 2, got: 1 });
  assert.deepEqual(firstDifference({ "x-y": 1 }, {}), { path: '$["x-y"]', want: 1, got: "(missing)" });
});
