import { useCallback, useEffect, useMemo, useState } from "react";
import { describeError, type ArdaClient, type ImageMeta } from "../api/client.ts";
import { groundKeys, squareAt, withGround, type TacticalLayoutDto, type TacticalLayouts } from "../api/tactical.ts";
import { CacheCorner } from "../components/CacheCorner.tsx";
import { useImageLog } from "../imageLog.ts";
import { SquareInfo, SquareReadout } from "../components/SquareInfo.tsx";
import { TacticalMap, type SquareRef } from "../components/TacticalMap.tsx";
import { squareLabel } from "../geo/coords.ts";
import { tacticalPyramid } from "../geo/tacticalTiles.ts";
import { useAsync, whileLive } from "../hooks.ts";
import { editMarks, pointerMarks } from "../render/tacticalOverlay.ts";

function layoutFromHash(): string | null {
  return new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("layout");
}

interface Loaded {
  listing: TacticalLayouts;
  layouts: TacticalLayoutDto[];
  /** Every ground key of the loaded library (`/v1/tactical/library`), or null on older servers. */
  libraryGrounds: string[] | null;
}

/**
 * Layout editor lite: load a layout's JSON, change squares' ground, and POST
 * it to /v1/tactical/render to check the compositor quickly.
 */
export function LayoutEditor({ client }: { client: ArdaClient }) {
  const loaded = useAsync(async (signal): Promise<Loaded> => {
    const listing = await client.tacticalLayouts({ signal });
    const layouts = await Promise.all(listing.layouts.map((l) => client.tacticalLayout(l.name, { signal })));
    const libraryGrounds = await client.tacticalLibrary({ signal }).then(
      (lib) => lib.grounds.map((g) => g.key),
      () => null,
    );
    return { listing, layouts, libraryGrounds };
  }, [client]);
  const [name, setName] = useState<string | null>(layoutFromHash);

  if (loaded.state === "loading") return <p className="notice">Loading the built-in layouts…</p>;
  if (loaded.state === "error") return <p className="notice error">{describeError(loaded.error)}</p>;
  const { listing, layouts, libraryGrounds } = loaded.value;
  const original = layouts.find((l) => l.name === name) ?? layouts[0];
  if (!original) return <p className="notice">The server lists no tactical layouts to edit.</p>;
  return (
    <Editor
      key={original.name}
      client={client}
      listing={listing}
      original={original}
      names={layouts.map((l) => l.name)}
      grounds={libraryGrounds ?? groundKeys(layouts)}
      onPick={(n) => {
        window.location.hash = `#/editor?layout=${encodeURIComponent(n)}`;
        setName(n);
      }}
    />
  );
}

type RenderState = { state: "busy" } | { state: "ok"; url: string; meta: ImageMeta; edits: number } | { state: "error"; message: string };

function Editor({
  client,
  listing,
  original,
  names,
  grounds,
  onPick,
}: {
  client: ArdaClient;
  listing: TacticalLayouts;
  original: TacticalLayoutDto;
  names: string[];
  grounds: string[];
  onPick: (name: string) => void;
}) {
  const [layout, setLayout] = useState(original);
  const [ppsq, setPpsq] = useState(() => Math.min(...listing.ppsq_options));
  const [picked, setPicked] = useState<SquareRef | null>(null);
  const [hover, setHover] = useState<SquareRef | null>(null);
  const [ground, setGround] = useState<string>("");
  const [result, setResult] = useState<RenderState>({ state: "busy" });
  const [grid, setGrid] = useState(true);
  const { log, record } = useImageLog();

  const edited = useMemo(() => {
    const out: (SquareRef & { from: string; to: string })[] = [];
    layout.squares.forEach((s, i) => {
      const was = original.squares[i]?.ground;
      if (was !== undefined && was !== s.ground) out.push({ sx: i % layout.width, sy: Math.floor(i / layout.width), from: was, to: s.ground });
    });
    return out;
  }, [layout, original]);

  const render = useCallback(
    (l: TacticalLayoutDto, signal?: AbortSignal) => {
      const edits = l.squares.filter((s, i) => original.squares[i]?.ground !== s.ground).length;
      whileLive(
        signal,
        client.renderLayout(l, { ppsq }, signal ? { signal } : undefined),
        (img) => {
          record(img);
          const url = URL.createObjectURL(img.blob);
          setResult((prev) => {
            if (prev.state === "ok" && prev.url !== url) URL.revokeObjectURL(prev.url);
            return { state: "ok", url, meta: img, edits };
          });
        },
        (e) => { setResult({ state: "error", message: describeError(e) }); },
      );
    },
    [client, ppsq, record, original],
  );

  // Render the unedited layout once on load (and when ppsq changes), so the first round trip is visible.
  const [initial] = useState(original);
  useEffect(() => {
    const ctl = new AbortController();
    render(initial, ctl.signal);
    return () => {
      ctl.abort();
    };
  }, [render, initial]);

  const geometry = useMemo(() => tacticalPyramid(layout.width, layout.height, ppsq), [layout.width, layout.height, ppsq]);
  const shown = picked ?? hover;
  const marks = useMemo(() => pointerMarks(layout, shown, picked), [layout, shown, picked]);
  const overlay = useMemo(() => editMarks(edited), [edited]);
  const current = picked ? squareAt(layout, picked.sx, picked.sy) : null;
  const imageUrl = result.state === "ok" ? result.url : null;

  const apply = () => {
    if (!picked || ground === "") return;
    setLayout((l) => withGround(l, picked.sx, picked.sy, ground));
  };

  return (
    <div className="split grow" data-testid="layout-editor">
      <div className="map-wrap">
        <TacticalMap
          width={layout.width}
          height={layout.height}
          geometry={geometry}
          source={imageUrl ? { kind: "image", key: imageUrl, url: imageUrl } : { kind: "none" }}
          grid={grid}
          overlay={overlay}
          marks={marks}
          onHover={setHover}
          onClick={(sq) => {
            setPicked(sq);
            const s = sq ? squareAt(layout, sq.sx, sq.sy) : null;
            if (s) setGround(s.ground);
          }}
          testId="editor-map"
        />
        <SquareReadout hover={hover} hint="click a square to edit its ground" />
        <CacheCorner log={log} source="POST /render" />
        <div className="map-tools">
          <label>
            layout{" "}
            <select
              value={original.name}
              onChange={(e) => {
                onPick(e.target.value);
              }}
            >
              {names.map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
          <label>
            px/square{" "}
            <select
              value={ppsq}
              onChange={(e) => {
                setResult({ state: "busy" });
                setPpsq(Number(e.target.value));
              }}
            >
              {listing.ppsq_options.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={grid}
              onChange={(e) => {
                setGrid(e.target.checked);
              }}
            />{" "}
            5-ft grid
          </label>
          <span className="muted">
            {result.state === "busy" && "rendering…"}
            {result.state === "ok" &&
              `rendered ${result.edits} edit${result.edits === 1 ? "" : "s"} · ${result.meta.status} · ${(result.meta.bytes / 1024).toFixed(0)} KiB · ${result.meta.ms.toFixed(0)} ms`}
          </span>
          {result.state === "error" && (
            <span className="error" data-testid="render-error">
              {result.message}
            </span>
          )}
        </div>
      </div>
      <aside className="inspector" aria-label="Layout editor">
        <h2>Edit {original.name}</h2>
        {picked && current ? (
          <section className="edit-box">
            <h3>
              Square {squareLabel(picked.sx, picked.sy)} ({picked.sx}, {picked.sy})
            </h3>
            <label>
              ground{" "}
              <select
                aria-label="ground"
                value={ground}
                onChange={(e) => {
                  setGround(e.target.value);
                }}
              >
                {grounds.map((g) => (
                  <option key={g} value={g}>
                    {g}
                  </option>
                ))}
              </select>
            </label>{" "}
            <button type="button" onClick={apply} disabled={ground === current.ground}>
              Set ground
            </button>
          </section>
        ) : (
          <p className="muted">Click a square on the map to change its ground.</p>
        )}
        <div className="toolbar">
          <button
            type="button"
            className="primary"
            disabled={result.state === "busy"}
            onClick={() => {
              setResult({ state: "busy" });
              render(layout);
            }}
          >
            POST /render
          </button>
          <button
            type="button"
            disabled={edited.length === 0}
            onClick={() => {
              setLayout(original);
            }}
          >
            Reset edits
          </button>
        </div>
        <h3>Edits ({edited.length})</h3>
        {edited.length === 0 ? (
          <p className="muted small">none yet</p>
        ) : (
          <ul className="plain small" data-testid="edit-list">
            {edited.map((e) => (
              <li key={`${e.sx},${e.sy}`}>
                {squareLabel(e.sx, e.sy)} ({e.sx}, {e.sy}): <code>{e.from}</code> → <code>{e.to}</code>
              </li>
            ))}
          </ul>
        )}
        {shown && <SquareInfo layout={layout} at={shown} pinned={picked !== null} />}
        <p className="muted small">
          Ground keys are the ones the server's built-in layouts use ({grounds.length}); the server has no library listing route yet. An unknown key
          answers 422 <code>invalid_layout</code>.
        </p>
        <details>
          <summary>Layout JSON ({(JSON.stringify(layout).length / 1024).toFixed(1)} KiB)</summary>
          <pre>{JSON.stringify({ ...layout, squares: `[${layout.squares.length} squares]` }, null, 2)}</pre>
        </details>
      </aside>
    </div>
  );
}
