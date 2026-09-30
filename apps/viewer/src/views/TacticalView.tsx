import { useEffect, useMemo, useState } from "react";
import { describeError, isNotAvailable, type ArdaClient } from "../api/client.ts";
import type { TacticalLayoutDto, TacticalLayouts, TacticalLayoutSummary } from "../api/tactical.ts";
import { CacheCorner } from "../components/CacheCorner.tsx";
import { useImageLog } from "../imageLog.ts";
import { SquareInfo, SquareReadout } from "../components/SquareInfo.tsx";
import { TacticalMap, type MapSource, type SquareRef, type TileLoader } from "../components/TacticalMap.tsx";
import { SQUARE_FT } from "../geo/coords.ts";
import { pxPerSquare, tacticalPyramid, tileGrid } from "../geo/tacticalTiles.ts";
import { useAsync } from "../hooks.ts";
import { pointerMarks } from "../render/tacticalOverlay.ts";

export function NotAvailable({ what, error }: { what: string; error: unknown }) {
  return (
    <div className="notice unavailable" data-testid="tactical-unavailable">
      <h2>Not available on this server</h2>
      <p>
        {what} is not offered by this arda-server (<code>{describeError(error)}</code>).
      </p>
      <p className="muted">
        The tactical endpoints (<code>/v1/tactical/layouts</code>, <code>/v1/tactical/layout/&#123;name&#125;</code>, its <code>.png</code> and
        WebP tile pyramid) ship with a newer server build. Point the API URL at one that has them.
      </p>
    </div>
  );
}

export function TacticalView({ client }: { client: ArdaClient }) {
  const list = useAsync((signal) => client.tacticalLayouts({ signal }), [client]);
  const [chosen, setChosen] = useState<string | null>(null);

  if (list.state === "loading") return <p className="notice">Loading /v1/tactical/layouts…</p>;
  if (list.state === "error") {
    return isNotAvailable(list.error) ? (
      <NotAvailable what="The tactical layout listing" error={list.error} />
    ) : (
      <p className="notice error">{describeError(list.error)}</p>
    );
  }
  const listing = list.value;
  const summary = listing.layouts.find((l) => l.name === chosen) ?? listing.layouts[0];
  if (!summary) return <p className="notice">The server lists no tactical layouts.</p>;
  return (
    <div className="split">
      <nav className="layout-list" aria-label="Tactical layouts">
        <h2>Layouts</h2>
        <ul>
          {listing.layouts.map((l) => (
            <li key={l.name}>
              <button
                type="button"
                className={l.name === summary.name ? "active" : ""}
                onClick={() => {
                  setChosen(l.name);
                }}
              >
                {l.name}
                <span className="muted">
                  {" "}
                  {l.width}×{l.height}
                </span>
              </button>
            </li>
          ))}
        </ul>
        <p className="muted small">
          world seed {listing.world_seed}
          <br />
          library {listing.library_version}
        </p>
      </nav>
      <LayoutViewer key={summary.name} client={client} listing={listing} summary={summary} />
    </div>
  );
}

function LayoutViewer({ client, listing, summary }: { client: ArdaClient; listing: TacticalLayouts; summary: TacticalLayoutSummary }) {
  const layout = useAsync((signal) => client.tacticalLayout(summary.name, { signal }), [client, summary.name]);
  if (layout.state === "loading") return <p className="notice">Loading layout {summary.name}…</p>;
  if (layout.state === "error") {
    return isNotAvailable(layout.error) ? (
      <NotAvailable what={`Layout “${summary.name}”`} error={layout.error} />
    ) : (
      <p className="notice error">{describeError(layout.error)}</p>
    );
  }
  return <LayoutMap client={client} listing={listing} summary={summary} layout={layout.value} />;
}

type SourceChoice = "tiles" | "png";

/** Loads the whole-map PNG as a blob URL, for the PNG source and the tile fallback. */
function usePngUrl(client: ArdaClient, name: string, ppsq: number, enabled: boolean, record: ReturnType<typeof useImageLog>["record"]) {
  const [state, setState] = useState<{ key: string; url: string } | { key: string; error: string } | null>(null);
  const key = `${name}@${ppsq}`;
  useEffect(() => {
    if (!enabled) return;
    const ctl = new AbortController();
    let url: string | null = null;
    client.tacticalPng(name, { ppsq }, { signal: ctl.signal }).then(
      (img) => {
        record(img);
        url = URL.createObjectURL(img.blob);
        setState({ key, url });
      },
      (e: unknown) => {
        if (!ctl.signal.aborted) setState({ key, error: describeError(e) });
      },
    );
    return () => {
      ctl.abort();
      if (url) URL.revokeObjectURL(url);
    };
  }, [client, name, ppsq, enabled, key, record]);
  return enabled && state && state.key === key ? state : null;
}

function LayoutMap({
  client,
  listing,
  summary,
  layout,
}: {
  client: ArdaClient;
  listing: TacticalLayouts;
  summary: TacticalLayoutSummary;
  layout: TacticalLayoutDto;
}) {
  const [grid, setGrid] = useState(true);
  const [ppsq, setPpsq] = useState<number>(listing.default_ppsq);
  const [choice, setChoice] = useState<SourceChoice>("tiles");
  const [fallback, setFallback] = useState<string | null>(null);
  const [hover, setHover] = useState<SquareRef | null>(null);
  const [picked, setPicked] = useState<SquareRef | null>(null);
  const { log, record, reset } = useImageLog();

  // The listing gives the pyramid at the default ppsq; other ppsq values follow the same rule.
  const geometry = useMemo(
    () => (ppsq === summary.tiles.ppsq ? summary.tiles : tacticalPyramid(layout.width, layout.height, ppsq, summary.tiles.tile_px)),
    [ppsq, summary.tiles, layout.width, layout.height],
  );
  const usePng = choice === "png" || fallback !== null;
  const png = usePngUrl(client, layout.name, ppsq, usePng, record);

  const loadTile = useMemo<TileLoader>(
    () => (z, x, y, signal) => client.tacticalTile(layout.name, z, x, y, { ppsq }, { signal }),
    [client, layout.name, ppsq],
  );
  const source: MapSource = usePng
    ? png && "url" in png
      ? { kind: "image", key: png.url, url: png.url }
      : { kind: "none" }
    : { kind: "tiles", key: `${layout.name}@${ppsq}`, load: loadTile };

  const shown = picked ?? hover;
  const marks = useMemo(() => pointerMarks(layout, shown, picked), [layout, shown, picked]);
  const topGrid = tileGrid(geometry, geometry.max_zoom);

  return (
    <div className="split grow">
      <div className="map-wrap">
        <TacticalMap
          width={layout.width}
          height={layout.height}
          geometry={geometry}
          source={source}
          grid={grid}
          marks={marks}
          onHover={setHover}
          onClick={setPicked}
          onImageMeta={record}
          onSourceError={setFallback}
        />
        <SquareReadout hover={hover} hint="hover for square coordinates · click to pin a square" />
        <CacheCorner log={log} source={usePng ? "PNG" : "WebP tiles"} />
        <div className="map-tools">
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
          <label>
            source{" "}
            <select
              value={choice}
              onChange={(e) => {
                setChoice(e.target.value as SourceChoice);
                setFallback(null);
                reset();
              }}
            >
              <option value="tiles">WebP tiles</option>
              <option value="png">PNG</option>
            </select>
          </label>
          <label>
            px/square{" "}
            <select
              value={ppsq}
              onChange={(e) => {
                setPpsq(Number(e.target.value));
                setFallback(null);
                reset();
              }}
            >
              {listing.ppsq_options.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </label>
          <span className="muted">
            {layout.width}×{layout.height} squares ({layout.width * SQUARE_FT}×{layout.height * SQUARE_FT} ft) · {geometry.image_width_px}×
            {geometry.image_height_px} px · zoom 0–{geometry.max_zoom}, {topGrid.cols}×{topGrid.rows} tiles at z{geometry.max_zoom},{" "}
            {pxPerSquare(geometry, 0)} px/square at z0
          </span>
          {fallback !== null && choice === "tiles" && <span className="warn-text">tiles failed ({fallback}); showing the PNG</span>}
          {png && "error" in png && <span className="error">PNG: {png.error}</span>}
        </div>
      </div>
      <aside className="inspector" aria-label="Square data">
        <h2>{layout.name}</h2>
        {shown ? <SquareInfo layout={layout} at={shown} pinned={picked !== null} /> : <p className="muted">Hover or click a square.</p>}
        {picked && (
          <button
            type="button"
            onClick={() => {
              setPicked(null);
            }}
          >
            Unpin
          </button>
        )}
        <p className="muted small">
          {layout.walls.length} wall segments · {layout.placements.length} placements · {layout.lights.length} lights
        </p>
        <p className="inspector-links">
          <a href={`#/editor?layout=${encodeURIComponent(layout.name)}`}>Edit this layout</a>
        </p>
      </aside>
    </div>
  );
}
