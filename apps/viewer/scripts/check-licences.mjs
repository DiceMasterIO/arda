// Licence policy check for apps/viewer. Node standard library only; reads
// package-lock.json and licence-policy.json, so no install is needed.
// Usage: node scripts/check-licences.mjs [lockfile] [policy]
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));

/** Package name from a lockfile path such as `node_modules/@a/b`. */
export function packageName(path) {
  const i = path.lastIndexOf("node_modules/");
  return i < 0 ? path : path.slice(i + "node_modules/".length);
}

/** Lockfile packages that reach the built bundle: not dev, not devOptional. */
export function runtimePackages(lock) {
  return Object.entries(lock.packages ?? {})
    .filter(([path, p]) => path !== "" && !p.link && !p.dev && !p.devOptional)
    .map(([path, p]) => ({ path, name: packageName(path), version: p.version ?? "", license: p.license }))
    .sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

function tokenise(expr) {
  return expr.match(/\(|\)|[^\s()]+/g) ?? [];
}

/**
 * Does an SPDX expression pass `allowed` (a Set)? OR passes if any branch
 * passes, AND needs every part. `X WITH Y` is one atom and must be listed whole.
 * Anything unparsable fails.
 */
export function expressionAllowed(expr, allowed) {
  const t = tokenise(expr);
  let i = 0;
  const parseOr = () => {
    let ok = parseAnd();
    while (t[i] === "OR") {
      i += 1;
      const rhs = parseAnd();
      ok = ok || rhs;
    }
    return ok;
  };
  const parseAnd = () => {
    let ok = parseAtom();
    while (t[i] === "AND") {
      i += 1;
      const rhs = parseAtom();
      ok = ok && rhs;
    }
    return ok;
  };
  const parseAtom = () => {
    if (t[i] === "(") {
      i += 1;
      const ok = parseOr();
      if (t[i] !== ")") throw new Error("unbalanced parenthesis");
      i += 1;
      return ok;
    }
    const id = t[i];
    if (id === undefined || id === ")" || id === "OR" || id === "AND" || id === "WITH") {
      throw new Error("unexpected token");
    }
    i += 1;
    if (t[i] === "WITH") {
      const exc = t[i + 1];
      if (exc === undefined) throw new Error("WITH without exception");
      i += 2;
      return allowed.has(`${id} WITH ${exc}`);
    }
    return allowed.has(id);
  };
  try {
    const ok = parseOr();
    return i === t.length && ok;
  } catch {
    return false;
  }
}

/** Validate the policy shape; throws on a malformed file. */
export function validatePolicy(policy) {
  for (const key of ["runtime", "buildOnly", "exceptions"]) {
    if (!Array.isArray(policy[key])) throw new Error(`policy.${key} must be an array`);
  }
  for (const e of policy.exceptions) {
    if (typeof e.package !== "string" || typeof e.reason !== "string" || e.reason.trim() === "") {
      throw new Error("every exception needs a package and a non-empty reason");
    }
  }
}

/**
 * Returns violations sorted by lockfile path:
 * `{ path, name, version, scope, license, problem }`.
 */
export function checkLockfile(lock, policy) {
  validatePolicy(policy);
  const runtimeSet = new Set(policy.runtime);
  const buildSet = new Set([...policy.runtime, ...policy.buildOnly]);
  const excepted = new Map(policy.exceptions.map((e) => [e.package, e]));
  const out = [];
  for (const [path, p] of Object.entries(lock.packages ?? {})) {
    if (path === "" || p.link) continue;
    const name = packageName(path);
    if (excepted.has(name)) continue;
    const scope = p.dev || p.devOptional ? "build" : "runtime";
    const license = typeof p.license === "string" ? p.license.trim() : "";
    let problem = null;
    if (license === "") problem = "missing licence";
    else if (/^(UNLICENSED|SEE LICENSE IN.*|UNKNOWN)$/i.test(license)) problem = "unlicensed";
    else if (!expressionAllowed(license, scope === "runtime" ? runtimeSet : buildSet)) {
      problem = `not allowed for ${scope} use`;
    }
    if (problem) out.push({ path, name, version: p.version ?? "", scope, license: license || "-", problem });
  }
  return out.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

function main() {
  const lockPath = process.argv[2] ?? here("../package-lock.json");
  const policyPath = process.argv[3] ?? here("../licence-policy.json");
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  const policy = JSON.parse(readFileSync(policyPath, "utf8"));
  const bad = checkLockfile(lock, policy);
  if (bad.length > 0) {
    console.error("Licence policy violations:");
    for (const v of bad) {
      console.error(`${v.path}@${v.version}\t${v.scope}\t${v.license}\t${v.problem}`);
    }
    console.error(`${bad.length} violation(s). Fix the dependency or add a reasoned exception to licence-policy.json.`);
    process.exit(1);
  }
  const n = Object.keys(lock.packages ?? {}).length - 1;
  console.log(`licences ok: ${n} packages, ${runtimePackages(lock).length} runtime`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
