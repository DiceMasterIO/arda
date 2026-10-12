import assert from "node:assert/strict";
import { test } from "node:test";
import { checkLockfile, expressionAllowed } from "./check-licences.mjs";

const policy = {
  runtime: ["MIT", "ISC", "BSD-2-Clause", "Apache-2.0"],
  buildOnly: ["MPL-2.0"],
  exceptions: [],
};
const lock = (pkgs) => ({
  packages: { "": { name: "x" }, ...pkgs },
});
const names = (v) => v.map((x) => x.name);

test("runtime GPL fails", () => {
  const v = checkLockfile(lock({ "node_modules/a": { version: "1.0.0", license: "GPL-3.0" } }), policy);
  assert.deepEqual(names(v), ["a"]);
  assert.equal(v[0].scope, "runtime");
});

test("dev MPL passes, runtime MPL fails", () => {
  const dev = lock({ "node_modules/a": { version: "1", license: "MPL-2.0", dev: true } });
  assert.equal(checkLockfile(dev, policy).length, 0);
  const rt = lock({ "node_modules/a": { version: "1", license: "MPL-2.0" } });
  assert.deepEqual(names(checkLockfile(rt, policy)), ["a"]);
  const devOpt = lock({ "node_modules/a": { version: "1", license: "MPL-2.0", devOptional: true } });
  assert.equal(checkLockfile(devOpt, policy).length, 0);
});

test("missing and unlicensed fail", () => {
  const v = checkLockfile(
    lock({
      "node_modules/a": { version: "1", dev: true },
      "node_modules/b": { version: "1", license: "UNLICENSED", dev: true },
    }),
    policy,
  );
  assert.deepEqual(names(v), ["a", "b"]);
});

test("OR passes on any allowed branch, AND needs all", () => {
  const allowed = new Set(["MIT"]);
  assert.ok(expressionAllowed("MIT OR GPL-3.0", allowed));
  assert.ok(!expressionAllowed("MIT AND GPL-3.0", allowed));
  assert.ok(expressionAllowed("(GPL-3.0 AND MIT) OR MIT", allowed));
  assert.ok(!expressionAllowed("MIT OR", allowed));
  assert.ok(!expressionAllowed("(MIT", allowed));
});

test("exception with a reason passes; without a reason is rejected", () => {
  const l = lock({ "node_modules/a": { version: "1", license: "GPL-3.0" } });
  const ok = { ...policy, exceptions: [{ package: "a", reason: "dev-only codegen, reviewed" }] };
  assert.equal(checkLockfile(l, ok).length, 0);
  const bad = { ...policy, exceptions: [{ package: "a", reason: " " }] };
  assert.throws(() => checkLockfile(l, bad));
});

test("output is sorted by path", () => {
  const l = lock({
    "node_modules/z": { version: "1", license: "GPL-3.0" },
    "node_modules/a": { version: "1", license: "GPL-3.0" },
    "node_modules/m/node_modules/b": { version: "1", license: "AGPL-3.0" },
  });
  const paths = checkLockfile(l, policy).map((x) => x.path);
  assert.deepEqual(paths, [...paths].sort());
});
