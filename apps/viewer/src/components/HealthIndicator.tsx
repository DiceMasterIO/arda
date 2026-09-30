import { useEffect, useState } from "react";
import type { Health } from "@arda";
import { describeError, type ArdaClient } from "../api/client.ts";

type Status = { kind: "checking" } | { kind: "up"; health: Health } | { kind: "down"; reason: string };

const POLL_MS = 5000;

/** Polls /v1/health and shows a coloured dot with the served seed. */
export function HealthIndicator({ client }: { client: ArdaClient }) {
  const [status, setStatus] = useState<Status>({ kind: "checking" });

  useEffect(() => {
    let live = true;
    let timer: number | undefined;
    const check = async () => {
      const ctl = new AbortController();
      const t = window.setTimeout(() => {
        ctl.abort();
      }, 3000);
      try {
        const health = await client.health({ signal: ctl.signal });
        if (live) setStatus({ kind: "up", health });
      } catch (e) {
        if (live) setStatus({ kind: "down", reason: describeError(e) });
      } finally {
        window.clearTimeout(t);
        if (live) timer = window.setTimeout(() => void check(), POLL_MS);
      }
    };
    void check();
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [client]);

  const label =
    status.kind === "up"
      ? `healthy · seed ${status.health.seed} · contract v${status.health.contract_version}`
      : status.kind === "down"
        ? "unreachable"
        : "checking…";
  return (
    <span className={`health health-${status.kind}`} title={status.kind === "down" ? status.reason : label} data-testid="health">
      <span className="dot" aria-hidden="true" />
      {label}
    </span>
  );
}
