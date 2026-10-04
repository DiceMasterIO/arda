// generate.mjs end to end against a stand-in for Slopify's API on 127.0.0.1: the submit,
// poll, download, resume and import.toml paths, without Slopify or any provider.

import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { main } from "../generate.mjs";

const png = Buffer.from(
  "89504e470d0a1a0a0000000d4948445200000001000000010806000000" + "1f15c4890000000d49444154789c6360000002000100" + "0548af0e0000000049454e44ae426082",
  "hex",
);
const jpg = Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0, 0x10, 0x4a, 0x46, 0x49, 0x46]);

function fakeSlopify() {
  const projects = new Map();
  const prompts = [];
  const batches = new Map();
  const calls = [];
  let next = 1;
  const server = createServer(async (req, res) => {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    const raw = Buffer.concat(chunks);
    const body = req.headers["content-type"]?.startsWith("application/json") ? JSON.parse(raw.toString()) : undefined;
    const url = new URL(req.url, "http://x");
    calls.push(`${req.method} ${url.pathname}`);
    const json = (status, value) => {
      res.writeHead(status, { "content-type": "application/json" });
      res.end(JSON.stringify(value));
    };
    let m;
    if (url.pathname === "/api/health") return json(200, { status: "ok", version: "test" });
    if (url.pathname === "/api/channels") return json(200, { channels: [{ id: "chan-1", name: "DiceMaster Assets" }] });
    if (url.pathname === "/api/providers") return json(200, { providers: [{ id: "openai-image", readiness: { kind: "keyed", hasKey: true } }] });
    if ((m = /^\/api\/providers\/([^/]+)\/models$/.exec(url.pathname)))
      return json(200, { models: [{ id: "gpt-image-2", keywords: ["image", "reference"] }] });
    if (url.pathname === "/api/prompts" && req.method === "GET") return json(200, { prompts });
    if (url.pathname === "/api/prompts" && req.method === "POST") {
      const p = { id: `p${next++}`, ...body };
      prompts.push(p);
      return json(201, p);
    }
    if (url.pathname === "/api/staging/images") return json(201, { id: "staged1", stageKind: "images", state: "staged" });
    if (url.pathname === "/api/projects/batch") {
      if (batches.has(body.requestId)) return json(200, { queue: batches.get(body.requestId) });
      const queue = body.items.map((item, at) => {
        const id = `PRJ${next++}`;
        projects.set(id, { title: item.title, polls: 0, prompt: item.values.Subject });
        return { projectId: id, batchId: body.requestId, position: at + 1, state: "queued" };
      });
      batches.set(body.requestId, queue);
      return json(201, { queue });
    }
    if ((m = /^\/api\/projects\/([^/]+)$/.exec(url.pathname))) {
      const p = projects.get(m[1]);
      p.polls += 1;
      const done = p.polls > 1;
      return json(200, {
        project: { id: m[1], title: p.title, status: done ? (p.title.includes("anvil") ? "failed" : "done") : "running" },
        stages: [{ kind: "images", state: done ? "done" : "running", failureReason: p.title.includes("anvil") ? "provider refused" : null }],
        outputs:
          done && !p.title.includes("anvil")
            ? [{ role: "image", path: "images/001.png", meta: { index: 1, prompt: `rendered ${p.title}`, provider: "openai-image", model: "gpt-image-2" } }]
            : [],
      });
    }
    if ((m = /^\/files\/([^/]+)\/image-1$/.exec(url.pathname))) {
      const p = projects.get(m[1]);
      res.writeHead(200, { "content-type": "application/octet-stream" });
      return res.end(p.title.includes("barrel") ? jpg : png);
    }
    json(404, { title: "Not Found", detail: url.pathname });
  });
  return new Promise((ok) =>
    server.listen(0, "127.0.0.1", () => ok({ server, url: `http://127.0.0.1:${server.address().port}`, calls, prompts })),
  );
}

const sink = () => {
  let text = "";
  return { write: (s) => (text += s), get text() { return text; } };
};

test("submits, polls, downloads and writes import.toml; --resume skips what is done", async () => {
  const fake = await fakeSlopify();
  const out = mkdtempSync(join(tmpdir(), "art-gen-"));
  try {
    const args = ["--server", fake.url, "--out", out, "--only", "prop.barrel,prop.barrel.alt1,prop.crate,prop.anvil,prop.bed", "--poll", "1", "--yes"];
    const err = sink();
    const code = await main(args, { out: sink(), err });
    assert.equal(code, 1, err.text); // the anvil fails on purpose
    assert.ok(existsSync(join(out, "prop.crate.png")));
    assert.ok(existsSync(join(out, "prop.bed.png")));
    assert.ok(existsSync(join(out, "prop.barrel.jpg")), "JPEG kept as .jpg");
    assert.ok(existsSync(join(out, "prop.barrel__alt1.jpg")), "an alt is saved as <id>__altN for the importer");
    assert.ok(!existsSync(join(out, "prop.anvil.png")));
    assert.match(err.text, /FAILED prop\.anvil: provider refused/);
    // Two frames, so two batches; one Library prompt created.
    assert.equal(fake.calls.filter((c) => c === "POST /api/projects/batch").length, 2);
    assert.deepEqual(fake.prompts.map((p) => p.name), ["arda-prop"]);
    const toml = readFileSync(join(out, "import.toml"), "utf8");
    assert.match(toml, /file = "prop\.barrel\.jpg"/);
    assert.match(toml, /prompt = "rendered prop\.crate"/);
    assert.match(toml, /footprint = \[1, 2\]/);
    assert.match(toml, /file = "prop\.barrel__alt1\.jpg"/);
    assert.match(toml, /^layer = "prop"$/m);
    assert.match(toml, /^tags = \{ biome = \["temperate"\]/m);
    assert.doesNotMatch(toml, /prop\.anvil/);

    // Without --resume, existing files stop the run before anything is sent.
    const before = fake.calls.length;
    assert.equal(await main(args, { out: sink(), err: sink() }), 1);
    assert.equal(fake.calls.length, before);

    // With --resume only the failed anvil is sent again.
    const again = sink();
    await main([...args, "--resume"], { out: sink(), err: again });
    assert.match(again.text, /skipping 4 already downloaded/);
    assert.match(again.text, /1 images in 1 batch/);
  } finally {
    fake.server.close();
    rmSync(out, { recursive: true, force: true });
  }
});

test("without --yes nothing is sent", async () => {
  const fake = await fakeSlopify();
  const out = mkdtempSync(join(tmpdir(), "art-gen-"));
  try {
    const err = sink();
    assert.equal(await main(["--server", fake.url, "--out", out, "--class", "water"], { out: sink(), err }), 0);
    assert.match(err.text, /Nothing sent/);
    assert.deepEqual(fake.calls, []);
  } finally {
    fake.server.close();
    rmSync(out, { recursive: true, force: true });
  }
});
