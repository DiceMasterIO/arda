import { useEffect, useMemo, useState } from "react";
import { ArdaClient, describeError } from "./api/client.ts";
import { HealthIndicator } from "./components/HealthIndicator.tsx";
import { initialApiBase, saveApiBase } from "./config.ts";
import { AreaView } from "./views/AreaView.tsx";
import { LayoutEditor } from "./views/LayoutEditor.tsx";
import { TacticalCellView } from "./views/TacticalCellView.tsx";
import { TacticalView } from "./views/TacticalView.tsx";
import { WorldView } from "./views/WorldView.tsx";
import { useAsync } from "./hooks.ts";

const VIEWS = ["world", "area", "tactical", "cell", "editor"] as const;
type ViewName = (typeof VIEWS)[number];
/** Views that don't need /v1/world. */
const WORLDLESS: readonly ViewName[] = ["tactical", "cell", "editor"];

function viewFromHash(): ViewName {
  const h = window.location.hash.replace(/^#\/?/, "").split("?")[0];
  return (VIEWS as readonly string[]).includes(h ?? "") ? (h as ViewName) : "world";
}

export function App() {
  const [base, setBase] = useState(initialApiBase);
  const [draft, setDraft] = useState(base);
  const [view, setView] = useState<ViewName>(viewFromHash);
  const client = useMemo(() => new ArdaClient(base), [base]);
  const world = useAsync((signal) => client.world({ signal }), [client]);

  useEffect(() => {
    const onHash = () => {
      setView(viewFromHash());
    };
    window.addEventListener("hashchange", onHash);
    return () => {
      window.removeEventListener("hashchange", onHash);
    };
  }, []);

  const apply = () => {
    const next = new ArdaClient(draft).base;
    saveApiBase(next);
    setBase(next);
    setDraft(next);
  };

  return (
    <div className="app">
      <header className="topbar">
        <strong className="brand">Arda viewer</strong>
        <nav className="tabs" aria-label="Views">
          {VIEWS.map((v) => (
            <a key={v} href={`#/${v}`} className={v === view ? "tab active" : "tab"} aria-current={v === view ? "page" : undefined}>
              {v[0]?.toUpperCase()}
              {v.slice(1)}
            </a>
          ))}
        </nav>
        <form
          className="api-form"
          onSubmit={(e) => {
            e.preventDefault();
            apply();
          }}
        >
          <label htmlFor="api-base">API</label>
          <input
            id="api-base"
            value={draft}
            spellCheck={false}
            onChange={(e) => {
              setDraft(e.target.value);
            }}
          />
          <button type="submit" disabled={draft === base}>
            Connect
          </button>
        </form>
        <HealthIndicator key={client.base} client={client} />
      </header>
      <main className="content">
        {!WORLDLESS.includes(view) && world.state === "loading" && <p className="notice">Loading /v1/world…</p>}
        {!WORLDLESS.includes(view) && world.state === "error" && (
          <div className="notice error">
            <p>Could not load the world from {client.base}.</p>
            <p className="muted">{describeError(world.error)}</p>
            <p className="muted">Start a server with <code>arda-server --world out/micro42 --port 8787</code>, or change the API URL above.</p>
          </div>
        )}
        {world.state === "ok" && view === "world" && <WorldView client={client} world={world.value} />}
        {world.state === "ok" && view === "area" && <AreaView client={client} world={world.value} />}
        {view === "tactical" && <TacticalView client={client} />}
        {view === "cell" && <TacticalCellView client={client} world={world.state === "ok" ? world.value : null} />}
        {view === "editor" && <LayoutEditor client={client} />}
      </main>
    </div>
  );
}
