import { useEffect, useState } from "react";

export type Async<T> =
  | { state: "loading" }
  | { state: "ok"; value: T }
  | { state: "error"; error: unknown };

function sameDeps(a: readonly unknown[], b: readonly unknown[]): boolean {
  return a.length === b.length && a.every((v, i) => Object.is(v, b[i]));
}

/**
 * Runs `load` whenever `deps` change, aborting the previous run. A result is
 * only reported for the deps it was loaded with, so a change reads as loading.
 */
export function useAsync<T>(load: (signal: AbortSignal) => Promise<T>, deps: readonly unknown[]): Async<T> {
  const [settled, setSettled] = useState<{ deps: readonly unknown[]; result: Async<T> } | null>(null);
  useEffect(() => {
    const ctl = new AbortController();
    load(ctl.signal).then(
      (value) => {
        if (!ctl.signal.aborted) setSettled({ deps, result: { state: "ok", value } });
      },
      (error: unknown) => {
        if (!ctl.signal.aborted) setSettled({ deps, result: { state: "error", error } });
      },
    );
    return () => {
      ctl.abort();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return settled && sameDeps(settled.deps, deps) ? settled.result : { state: "loading" };
}
