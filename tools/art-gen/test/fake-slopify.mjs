// A stand-in for Slopify's API on 127.0.0.1, shared by the generate.mjs end-to-end tests.

import { createServer } from "node:http";

export const png = Buffer.from(
  "89504e470d0a1a0a0000000d4948445200000001000000010806000000" + "1f15c4890000000d49444154789c6360000002000100" + "0548af0e0000000049454e44ae426082",
  "hex",
);
const jpg = Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0, 0x10, 0x4a, 0x46, 0x49, 0x46]);

export function fakeSlopify() {
  const projects = new Map();
  const prompts = [];
  const batches = new Map();
  const calls = [];
  const running = { max: 0 };
  // Failure switches for the tests: titles whose first POST /api/projects drops the connection,
  // either after the project was created or before; `listStatus` makes GET /api/projects fail.
  const switches = { dropAfterCreate: new Set(), dropBeforeCreate: new Set(), listStatus: 200 };
  let next = 1;
  let clock = 0;
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
        projects.set(id, { title: item.title, polls: 0, prompt: item.values.Subject, createdAt: new Date().toISOString() });
        return { projectId: id, batchId: body.requestId, position: at + 1, state: "queued" };
      });
      batches.set(body.requestId, queue);
      return json(201, { queue });
    }
    if (url.pathname === "/api/projects" && req.method === "GET") {
      if (switches.listStatus !== 200) return json(switches.listStatus, { title: "Server Error", detail: "list unavailable" });
      return json(200, { projects: [...projects].map(([id, p]) => ({ id, title: p.title, createdAt: p.createdAt, status: "running", channelId: "chan-1" })) });
    }
    if (url.pathname === "/api/projects" && req.method === "POST") {
      if (switches.dropBeforeCreate.delete(body.title)) return req.socket.destroy();
      const id = `PRJ${next++}`;
      projects.set(id, { title: body.title, polls: 0, prompt: body.values.Subject, single: true, createdAt: new Date(Date.now() + clock++).toISOString() });
      if (switches.dropAfterCreate.delete(body.title)) return req.socket.destroy();
      return json(201, { project: { id, title: body.title, status: "running" }, stages: [] });
    }
    if ((m = /^\/api\/projects\/([^/]+)\/cancel$/.exec(url.pathname))) {
      projects.get(m[1]).canceled = true;
      return json(200, { canceled: ["images"] });
    }
    if ((m = /^\/api\/projects\/([^/]+)$/.exec(url.pathname))) {
      const p = projects.get(m[1]);
      p.polls += 1;
      const done = p.polls > 1;
      if (!done) running.max = Math.max(running.max, [...projects.values()].filter((q) => q.polls === 1 && q.single).length);
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
    server.listen(0, "127.0.0.1", () => ok({ server, url: `http://127.0.0.1:${server.address().port}`, calls, prompts, running, projects, switches })),
  );
}

export const sink = () => {
  let text = "";
  return { write: (s) => (text += s), get text() { return text; } };
};
