import { useEffect, useState } from "react";

/**
 * Hands `request`'s outcome to `onOk` or `onError` only while `signal` is
 * live. A response that resolves just before its request is aborted would
 * otherwise show the previous selection until the next one lands (review
 * round 1 #19).
 */
export function whileLive<T>(
  signal: AbortSignal | undefined,
  request: Promise<T>,
  onOk: (value: T) => void,
  onError: (error: unknown) => void = () => undefined,
): void {
  request.then(
    (value) => {
      if (!signal?.aborted) onOk(value);
    },
    (error: unknown) => {
      if (!signal?.aborted) onError(error);
    },
  );
}

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
    whileLive(
      ctl.signal,
      load(ctl.signal),
      (value) => { setSettled({ deps, result: { state: "ok", value } }); },
      (error) => { setSettled({ deps, result: { state: "error", error } }); },
    );
    return () => {
      ctl.abort();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return settled && sameDeps(settled.deps, deps) ? settled.result : { state: "loading" };
}
