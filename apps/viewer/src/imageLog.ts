import { useCallback, useState } from "react";
import type { ImageMeta } from "./api/client.ts";

export interface ImageLog {
  last: ImageMeta | null;
  hits: number;
  misses: number;
  other: number;
  bytes: number;
}

const EMPTY: ImageLog = { last: null, hits: 0, misses: 0, other: 0, bytes: 0 };

/** Counts image responses by `X-Arda-Cache` and keeps the latest one. */
export function useImageLog(): { log: ImageLog; record: (m: ImageMeta) => void; reset: () => void } {
  const [log, setLog] = useState<ImageLog>(EMPTY);
  const record = useCallback((m: ImageMeta) => {
    setLog((p) => ({
      last: m,
      hits: p.hits + (m.cache === "hit" ? 1 : 0),
      misses: p.misses + (m.cache === "miss" ? 1 : 0),
      other: p.other + (m.cache !== "hit" && m.cache !== "miss" ? 1 : 0),
      bytes: p.bytes + m.bytes,
    }));
  }, []);
  const reset = useCallback(() => {
    setLog(EMPTY);
  }, []);
  return { log, record, reset };
}
