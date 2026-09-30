import type { ImageLog } from "../imageLog.ts";

function kib(n: number): string {
  return n >= 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} MiB` : `${(n / 1024).toFixed(1)} KiB`;
}

/** Debug corner: the latest image's ETag and `X-Arda-Cache`, plus running counts. */
export function CacheCorner({ log, source }: { log: ImageLog; source: string }) {
  const { last } = log;
  return (
    <div className="cache-corner" data-testid="cache-corner">
      <div>
        <b>{source}</b> · hit {log.hits} · miss {log.misses}
        {log.other > 0 ? ` · ? ${log.other}` : ""} · {kib(log.bytes)}
      </div>
      {last ? (
        <>
          <div title={last.url}>
            {last.method} {last.label} → {last.status}
          </div>
          <div>
            X-Arda-Cache: <b className={last.cache === "hit" ? "ok" : last.cache === "miss" ? "warn" : ""}>{last.cache ?? "—"}</b> ·{" "}
            {last.ms.toFixed(0)} ms
            {last.serverTiming ? ` · ${last.serverTiming}` : ""}
          </div>
          <div className="etag">ETag: {last.etag ?? "— (not exposed)"}</div>
        </>
      ) : (
        <div>no image requests yet</div>
      )}
    </div>
  );
}
