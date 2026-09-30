import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The generated ts-rs bindings live at the workspace root and are imported
// through the `@arda` alias. They are never copied into this app.
const bindings = fileURLToPath(new URL("../../bindings/ts/arda", import.meta.url));
const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [{ find: /^@arda(\/.*)?$/, replacement: `${bindings}$1` }],
  },
  server: {
    port: 5173,
    fs: { allow: [repoRoot] },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
