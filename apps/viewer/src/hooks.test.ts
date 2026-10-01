import { describe, expect, it, vi } from "vitest";
import { whileLive } from "./hooks.ts";

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("whileLive", () => {
  it("drops a response that resolves after its request was aborted", async () => {
    const ctl = new AbortController();
    const d = deferred<string>();
    const onOk = vi.fn();
    const onError = vi.fn();
    whileLive(ctl.signal, d.promise, onOk, onError);
    // A new click aborts the old request just as its response arrives.
    ctl.abort();
    d.resolve("previous cell");
    await d.promise;
    await Promise.resolve();
    expect(onOk).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("delivers live results and errors", async () => {
    const ok = deferred<number>();
    const onOk = vi.fn();
    whileLive(new AbortController().signal, ok.promise, onOk);
    ok.resolve(7);
    await ok.promise;
    await Promise.resolve();
    expect(onOk).toHaveBeenCalledWith(7);

    const bad = deferred<number>();
    const onError = vi.fn();
    whileLive(undefined, bad.promise, () => undefined, onError);
    bad.reject(new Error("boom"));
    await bad.promise.catch(() => undefined);
    await Promise.resolve();
    expect(onError).toHaveBeenCalledTimes(1);
  });
});
