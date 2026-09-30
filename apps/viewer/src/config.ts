import { DEFAULT_API_BASE, normaliseBase } from "./api/client.ts";

const STORAGE_KEY = "arda-viewer.apiBase";

/**
 * API base: `?api=` in the page URL wins, then the saved choice, then
 * `VITE_ARDA_API`, then http://localhost:8787.
 */
export function initialApiBase(): string {
  const fromQuery = new URLSearchParams(window.location.search).get("api");
  if (fromQuery) return normaliseBase(fromQuery);
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    if (saved) return normaliseBase(saved);
  } catch {
    // Storage blocked: fall through.
  }
  const env: unknown = import.meta.env["VITE_ARDA_API"];
  return normaliseBase(typeof env === "string" && env !== "" ? env : DEFAULT_API_BASE);
}

export function saveApiBase(base: string): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, base);
  } catch {
    // Not persisted; the session still uses it.
  }
}
