// generate.mjs end to end against a stand-in for Slopify's API on 127.0.0.1: the submit,
// poll, download, resume and import.toml paths, without Slopify or any provider.

import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { main } from "../generate.mjs";
import { fakeSlopify, sink } from "./fake-slopify.mjs";

test("submits, polls, downloads and writes import.toml; --resume skips what is done", async () => {
  const fake = await fakeSlopify();
  const out = mkdtempSync(join(tmpdir(), "art-gen-"));
  try {
    const args = ["--server", fake.url, "--out", out, "--only", "prop.barrel,prop.barrel.alt1,prop.crate,prop.anvil,prop.bed", "--poll", "1", "--parallel", "0", "--price-per-image", "0.05", "--yes"];
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
    assert.deepEqual(fake.prompts.map((p) => [p.kind, p.name]), [["image", "arda-prop"], ["review", "arda-asset-review"]]);
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

test("--parallel posts each asset as its own project, keeping N going at once", async () => {
  const fake = await fakeSlopify();
  const out = mkdtempSync(join(tmpdir(), "art-gen-"));
  try {
    const err = sink();
    const code = await main(
      ["--server", fake.url, "--out", out, "--only", "prop.barrel,prop.crate,prop.bed,prop.chest,prop.table", "--poll", "1", "--parallel", "2", "--price-per-image", "0.05", "--yes"],
      { out: sink(), err },
    );
    assert.equal(code, 0, err.text);
    for (const id of ["prop.crate", "prop.bed", "prop.chest", "prop.table"]) assert.ok(existsSync(join(out, `${id}.png`)), id);
    assert.equal(fake.calls.filter((c) => c === "POST /api/projects").length, 5);
    assert.equal(fake.calls.filter((c) => c === "POST /api/projects/batch").length, 0);
    assert.ok(fake.running.max <= 2, `at most 2 at once, saw ${fake.running.max}`);
    assert.match(err.text, /started prop\.\w+ \(3 not started yet\)/);
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
