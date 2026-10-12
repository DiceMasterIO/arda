// Vite plugin: writes THIRD_PARTY_LICENSES.txt into dist/ with the licence
// text of every runtime package in package-lock.json. A missing licence file
// fails the build.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { runtimePackages } from "./check-licences.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));

/** Render the notices text for `pkgs`, reading licence files under `root`. */
export function renderNotices(pkgs, appRoot = root) {
  const parts = ["Third-party software bundled in this build.\n"];
  for (const p of pkgs) {
    const dir = join(appRoot, p.path);
    const files = existsSync(dir)
      ? readdirSync(dir).filter((f) => /^(licen[cs]e|copying)(\.|$)/i.test(f)).sort()
      : [];
    if (files.length === 0) throw new Error(`no LICENSE file found for ${p.name}@${p.version} in ${p.path}`);
    parts.push(`${"=".repeat(72)}\n${p.name}@${p.version} (${p.license})\n${"=".repeat(72)}\n`);
    for (const f of files) parts.push(`${readFileSync(join(dir, f), "utf8").trim()}\n`);
  }
  return parts.join("\n");
}

export default function licenceNotices() {
  return {
    name: "arda-licence-notices",
    apply: "build",
    generateBundle() {
      const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
      let source;
      try {
        source = renderNotices(runtimePackages(lock));
      } catch (e) {
        this.error(e instanceof Error ? e.message : String(e));
      }
      this.emitFile({ type: "asset", fileName: "THIRD_PARTY_LICENSES.txt", source });
    },
  };
}
