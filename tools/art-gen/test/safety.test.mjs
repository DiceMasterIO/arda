// The spending caps and the double-billing guards of generate.mjs, against the stand-in server:
// every case here costs nothing.

import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { main, parseOptions } from "../generate.mjs";
import { spendPlan } from "../safety.mjs";
import { fakeSlopify, sink } from "./fake-slopify.mjs";

const five = "prop.barrel,prop.crate,prop.bed,prop.chest,prop.table";
const three = "prop.barrel,prop.crate,prop.bed";

async function withFake(body) {
  const fake = await fakeSlopify();
  const out = mkdtempSync(join(tmpdir(), "art-gen-"));
  try {
    await body(fake, out);
  } finally {
    fake.server.close();
    rmSync(out, { recursive: true, force: true });
  }
}
const base = (fake, out, only, ...more) => ["--server", fake.url, "--out", out, "--only", only, "--poll", "1", ...more];
const posts = (fake) => fake.calls.filter((c) => c === "POST /api/projects").length;
const stateOf = (out) => JSON.parse(readFileSync(join(out, ".art-gen-state.json"), "utf8"));

test("spendPlan: worst case counts reviewer remakes, --no-review lowers it, --max-images caps it", () => {
  const o = (...a) => parseOptions(["--price-per-image", "0.05", ...a], {});
  const assets = Array.from({ length: 5 }, (_, i) => ({ id: `a${i}` }));
  assert.deepEqual(spendPlan(assets, o()), { images: 5, worstCaseImages: 15, each: 0.05, worstCaseCost: 0.75 });
  assert.equal(spendPlan(assets, o("--no-review")).worstCaseCost, 0.25);
  assert.equal(spendPlan(assets, o("--review-mode", "flag")).worstCaseCost, 0.25);
  assert.equal(spendPlan(assets, o("--max-images", "2")).worstCaseImages, 6);
  assert.match(spendPlan(assets, o("--max-spend", "0.5")).refusal, /worst case is \$0\.75/);
  assert.equal(spendPlan(assets, o("--max-spend", "0.75")).refusal, undefined);
  const codex = parseOptions(["--provider", "codex", "--max-spend", "0"], {});
  assert.equal(spendPlan(assets, codex).refusal, undefined, "codex costs $0, so the money cap never refuses it");
});

test("ART_GEN_MAX_SPEND is the default for --max-spend and the flag wins", () => {
  assert.equal(parseOptions([], { ART_GEN_MAX_SPEND: "1.5" }).maxSpend, 1.5);
  assert.equal(parseOptions(["--max-spend", "2"], { ART_GEN_MAX_SPEND: "1.5" }).maxSpend, 2);
  assert.throws(() => parseOptions([], { ART_GEN_MAX_SPEND: "lots" }), /ART_GEN_MAX_SPEND/);
});

test("--max-images 3 with 5 assets posts exactly 3, exits 3, saves state; --resume finishes the rest", async () =>
  withFake(async (fake, out) => {
    const args = base(fake, out, five, "--parallel", "5", "--max-images", "3", "--yes");
    const err = sink();
    assert.equal(await main(args, { out: sink(), err }), 3, err.text);
    assert.equal(posts(fake), 3);
    assert.match(err.text, /STOPPED: --max-images 3 reached/);
    assert.equal(Object.keys(stateOf(out).assets).length, 3);
    assert.ok(existsSync(join(out, "import.toml")));
    assert.equal(fake.projects.size, 3);
    assert.equal(await main([...args, "--resume"], { out: sink(), err: sink() }), 0);
    assert.equal(posts(fake), 5);
    assert.equal(fake.projects.size, 5);
  }));

test("--max-images in batch mode cuts the last batch and stops", async () =>
  withFake(async (fake, out) => {
    const err = sink();
    const code = await main(base(fake, out, five, "--parallel", "0", "--batch-size", "2", "--max-images", "3", "--yes"), { out: sink(), err });
    assert.equal(code, 3, err.text);
    assert.equal(fake.projects.size, 3);
    assert.match(err.text, /STOPPED: --max-images 3/);
  }));

test("--max-spend under the worst case refuses before any request, also with --dry-run", async () =>
  withFake(async (fake, out) => {
    for (const extra of [["--yes"], ["--dry-run"], ["--dry-run", "--yes"]]) {
      const err = sink();
      const code = await main(base(fake, out, five, "--price-per-image", "0.05", "--max-spend", "0.5", ...extra), { out: sink(), err });
      assert.equal(code, 2, err.text);
      assert.match(err.text, /Worst case: 15 images × \$0\.05 = \$0\.75/);
      assert.match(err.text, /REFUSED: .*over --max-spend/);
    }
    assert.deepEqual(fake.calls, []);
    // Reviewer remakes are what push it over: without them 5 x $0.05 fits.
    const err = sink();
    assert.equal(await main(base(fake, out, five, "--price-per-image", "0.05", "--max-spend", "0.5", "--no-review", "--dry-run"), { out: sink(), err }), 0, err.text);
    assert.deepEqual(fake.calls, []);
  }));

test("an unknown price with --yes needs --max-images or --price-per-image", async () =>
  withFake(async (fake, out) => {
    const err = sink();
    assert.equal(await main(base(fake, out, three, "--yes"), { out: sink(), err }), 2);
    assert.match(err.text, /no known price/);
    assert.equal(await main(base(fake, out, three, "--yes", "--dry-run"), { out: sink(), err: sink() }), 2);
    assert.deepEqual(fake.calls, []);
    // Without --yes nothing is sent either way, and that is not a refusal.
    assert.equal(await main(base(fake, out, three), { out: sink(), err: sink() }), 0);
    assert.equal(await main(base(fake, out, "prop.crate", "--yes", "--max-images", "1"), { out: sink(), err: sink() }), 0);
    assert.equal(posts(fake), 1);
    assert.equal(await main(base(fake, out, "prop.bed", "--yes", "--price-per-image", "0.05"), { out: sink(), err: sink() }), 0);
    assert.equal(posts(fake), 2);
  }));

test("a --max-spend with an unknown price is refused: the cap could not be checked", async () =>
  withFake(async (fake, out) => {
    const err = sink();
    assert.equal(await main(base(fake, out, three, "--max-spend", "1"), { out: sink(), err }), 2);
    assert.match(err.text, /cannot be checked/);
    assert.deepEqual(fake.calls, []);
  }));

test("a dropped reply after the project was created is adopted, not posted twice", async () =>
  withFake(async (fake, out) => {
    fake.switches.dropAfterCreate.add("prop.crate");
    const err = sink();
    const code = await main(base(fake, out, three, "--parallel", "2", "--price-per-image", "0.05", "--yes"), { out: sink(), err });
    assert.equal(code, 0, err.text);
    assert.equal([...fake.projects.values()].filter((p) => p.title === "prop.crate").length, 1);
    assert.equal(posts(fake), 3, "one POST per asset");
    const crateId = [...fake.projects].find(([, p]) => p.title === "prop.crate")[0];
    assert.equal(stateOf(out).assets["prop.crate"].projectId, crateId);
    assert.match(err.text, /ADOPTED prop\.crate/);
    for (const id of ["prop.barrel", "prop.crate", "prop.bed"]) assert.equal(stateOf(out).assets[id].status, "downloaded", id);
  }));

test("a drop before the project was created is retried exactly once", async () =>
  withFake(async (fake, out) => {
    fake.switches.dropBeforeCreate.add("prop.crate");
    const err = sink();
    const code = await main(base(fake, out, three, "--parallel", "2", "--price-per-image", "0.05", "--yes"), { out: sink(), err });
    assert.equal(code, 0, err.text);
    assert.equal(posts(fake), 4, "three assets plus the one retry");
    assert.equal(fake.projects.size, 3);
    assert.match(err.text, /posting it once more/);
  }));

test("an intent left by a stopped run is adopted when the project exists", async () =>
  withFake(async (fake, out) => {
    const at = new Date().toISOString();
    writeFileSync(
      join(out, ".art-gen-state.json"),
      JSON.stringify({ assets: { "prop.crate": { status: "posting", at, template: "arda-prop", provider: "openai-image", model: "gpt-image-2", prompt: "p", footprint: [1, 2], structured: false } }, batches: {} }),
    );
    fake.projects.set("PRJ-early", { title: "prop.crate", polls: 0, prompt: "x", createdAt: new Date(Date.now() + 1000).toISOString() });
    const err = sink();
    const code = await main(base(fake, out, "prop.crate,prop.bed", "--parallel", "2", "--price-per-image", "0.05", "--yes"), { out: sink(), err });
    assert.equal(code, 0, err.text);
    assert.equal(posts(fake), 1, "only prop.bed is posted");
    assert.equal(stateOf(out).assets["prop.crate"].projectId, "PRJ-early");
    assert.ok(existsSync(join(out, "prop.crate.png")));
  }));

test("an intent with no project in the list is released and the asset posted once", async () =>
  withFake(async (fake, out) => {
    writeFileSync(join(out, ".art-gen-state.json"), JSON.stringify({ assets: { "prop.crate": { status: "posting", at: new Date().toISOString() } }, batches: {} }));
    const err = sink();
    assert.equal(await main(base(fake, out, "prop.crate", "--price-per-image", "0.05", "--yes"), { out: sink(), err }), 0, err.text);
    assert.equal(posts(fake), 1);
  }));

test("when the project list cannot be read, nothing is posted and the run exits 1", async () =>
  withFake(async (fake, out) => {
    writeFileSync(join(out, ".art-gen-state.json"), JSON.stringify({ assets: { "prop.crate": { status: "posting", at: new Date().toISOString() } }, batches: {} }));
    fake.switches.listStatus = 500;
    const err = sink();
    assert.equal(await main(base(fake, out, "prop.crate,prop.bed", "--price-per-image", "0.05", "--yes"), { out: sink(), err }), 1);
    assert.equal(posts(fake), 0);
    assert.match(err.text, /cannot read Slopify's project list/);
    assert.equal(stateOf(out).assets["prop.crate"].status, "posting", "the intent is kept for the next run");

    // Same during a run: the reply dropped after creation and the list is down.
    fake.switches.listStatus = 200;
    writeFileSync(join(out, ".art-gen-state.json"), JSON.stringify({ assets: {}, batches: {} }));
    fake.switches.dropAfterCreate.add("prop.bed");
    fake.switches.listStatus = 500;
    const before = posts(fake);
    assert.equal(await main(base(fake, out, "prop.bed", "--price-per-image", "0.05", "--yes"), { out: sink(), err: sink() }), 1);
    assert.equal(posts(fake) - before, 1, "no second POST");
    assert.equal(stateOf(out).assets["prop.bed"].status, "posting");
  }));
