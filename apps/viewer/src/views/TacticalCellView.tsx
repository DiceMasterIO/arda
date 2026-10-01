import { useEffect, useMemo, useState } from "react";
import { describeError, type ArdaClient } from "../api/client.ts";
import { demoBlock, parseRules, type NotYetError, type RulesSidecarDto, type TacticalBlockDto } from "../api/tactical.ts";
import { CacheCorner } from "../components/CacheCorner.tsx";
import { useImageLog } from "../imageLog.ts";
import { SquareInfo, SquareReadout } from "../components/SquareInfo.tsx";
import { TacticalMap, type MapSource, type SquareRef, type TileLoader } from "../components/TacticalMap.tsx";
import { tacticalPyramid } from "../geo/tacticalTiles.ts";
import { useAsync, whileLive } from "../hooks.ts";
import { COVER_COLOURS, pointerMarks, rulesOverlay, tokenMarks, waterFill, type RulesToggles } from "../render/tacticalOverlay.ts";
import { sceneGrids, sceneSquare, type TacticalScene } from "../api/scene.ts";
import { cellFromHash, cellHash, demoFromHash, gradeFromHash, neighbour, type CellExtent, type Direction } from "../geo/cellWalk.ts";
import { CellLookToggles } from "../components/CellLookToggles.tsx";
import { EdgeArrows, type Walk } from "../components/EdgeArrows.tsx";
import { HandoffCompare } from "../components/HandoffCompare.tsx";
import type { ReliefGrid } from "../geo/relief.ts";

const RENDER_PPSQ = 64;

/** The "cell" tab: `/v1/tactical/cell/{gx}/{gy}`, its 501 state, and the Block view. */
export function TacticalCellView({ client, world = null }: { client: ArdaClient; world?: (CellExtent & Partial<ReliefGrid>) | null }) {
  const [cell, setCell] = useState(cellFromHash);
  const [overlays, setOverlays] = useState(demoFromHash);
  const [grade, setGrade] = useState(gradeFromHash);
  const [handoff, setHandoff] = useState(false);
  const relief = world && world.tiles && world.areas_wide !== undefined ? (world as ReliefGrid) : null;
  const [draft, setDraft] = useState(() => ({ gx: String(cell?.gx ?? 512), gy: String(cell?.gy ?? 1036) }));

  useEffect(() => {
    const onHash = () => {
      const c = cellFromHash();
      setGrade(gradeFromHash());
      if (c) {
        setCell(c);
        setDraft({ gx: String(c.gx), gy: String(c.gy) });
      }
    };
    window.addEventListener("hashchange", onHash);
    return () => {
      window.removeEventListener("hashchange", onHash);
    };
  }, []);

  const go = () => {
    const gx = Number(draft.gx);
    const gy = Number(draft.gy);
    if (!Number.isInteger(gx) || !Number.isInteger(gy) || gx < 0 || gy < 0) return;
    window.location.hash = cellHash({ gx, gy }, overlays, grade);
    setCell({ gx, gy });
  };

  // Walking off an edge opens the neighbour, which joins this cell seamlessly
  // (its image is rendered with an apron) and was prefetched when this one opened.
  const walk = (dir: Direction) => {
    const next = cell ? neighbour(cell, dir, world) : null;
    if (!next) return;
    window.location.hash = cellHash(next, overlays, grade);
    setCell(next);
    setDraft({ gx: String(next.gx), gy: String(next.gy) });
  };

  return (
    <div className="cell-view">
      <form
        className="toolbar cell-form"
        onSubmit={(e) => {
          e.preventDefault();
          go();
        }}
      >
        <strong>Tactical block at cell</strong>
        <label>
          gx{" "}
          <input
            aria-label="gx"
            inputMode="numeric"
            value={draft.gx}
            onChange={(e) => {
              setDraft((d) => ({ ...d, gx: e.target.value }));
            }}
          />
        </label>
        <label>
          gy{" "}
          <input
            aria-label="gy"
            inputMode="numeric"
            value={draft.gy}
            onChange={(e) => {
              setDraft((d) => ({ ...d, gy: e.target.value }));
            }}
          />
        </label>
        <label title="?demo_overlays=1: the synthetic ways, fields and town samples, composed onto the refined terrain until settlement data is integrated">
          <input
            type="checkbox"
            aria-label="demo overlays"
            checked={overlays}
            onChange={(e) => {
              setOverlays(e.target.checked);
            }}
          />{" "}
          demo overlays
        </label>
        <CellLookToggles grade={grade} onGrade={setGrade} handoff={handoff} onHandoff={setHandoff} canHandoff={relief !== null} />
        <button type="submit">Load</button>
        <span className="muted small">
          or pick a cell in the World view and use “Open tactical map here”. <code>GET /v1/tactical/cell/&#123;gx&#125;/&#123;gy&#125;</code>
        </span>
      </form>
      {cell && handoff && relief && <HandoffCompare client={client} world={relief} gx={cell.gx} gy={cell.gy} worldGrade={grade} />}
      {cell ? (
        <CellResult
          key={`${cell.gx},${cell.gy},${overlays ? 1 : 0},${grade ? 1 : 0}`}
          client={client}
          gx={cell.gx}
          gy={cell.gy}
          overlays={overlays}
          grade={grade}
          walk={{ onWalk: walk, can: (d) => neighbour(cell, d, world) !== null }}
        />
      ) : (
        <p className="notice">Enter a global cell.</p>
      )}
    </div>
  );
}

function CellResult({
  client,
  gx,
  gy,
  overlays,
  grade,
  walk,
}: {
  client: ArdaClient;
  gx: number;
  gy: number;
  overlays: boolean;
  grade: boolean;
  walk: Walk;
}) {
  const res = useAsync((signal) => client.tacticalCell(gx, gy, { signal }, { ppsq: RENDER_PPSQ, demo: overlays }), [client, gx, gy, overlays]);
  const [demo, setDemo] = useState<TacticalBlockDto | null>(null);

  if (res.state === "loading") return <p className="notice">Loading /v1/tactical/cell/{gx}/{gy}…</p>;
  if (res.state === "error") {
    return (
      <div className="notice error" data-testid="cell-error">
        <h2>
          Cell {gx}, {gy}
        </h2>
        <p>{describeError(res.error)}</p>
      </div>
    );
  }
  if (res.value.kind === "block") return <BlockView client={client} block={res.value.block} demo={false} cell={{ gx, gy, overlays, grade }} walk={walk} />;
  if (demo) {
    return (
      <>
        <p className="banner warn-banner">
          Demo block built in the browser from the <code>{demo.meta["from_layout"]}</code> layout: not world data. The server still answers 501 for
          this cell.{" "}
          <button
            type="button"
            onClick={() => {
              setDemo(null);
            }}
          >
            Back
          </button>
        </p>
        <BlockView client={client} block={demo} demo />
      </>
    );
  }
  return <NotYetPanel client={client} gx={gx} gy={gy} notYet={res.value.notYet} onDemo={setDemo} />;
}

function NotYetPanel({
  client,
  gx,
  gy,
  notYet,
  onDemo,
}: {
  client: ArdaClient;
  gx: number;
  gy: number;
  notYet: NotYetError;
  onDemo: (b: TacticalBlockDto) => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const layouts = useAsync((signal) => client.tacticalLayouts({ signal }), [client]);
  const [pick, setPick] = useState("riverside");
  const names = layouts.state === "ok" ? layouts.value.layouts.map((l) => l.name) : [];
  const preview = () => {
    setBusy("loading…");
    client.tacticalLayout(pick).then(
      (l) => {
        setBusy(null);
        onDemo(demoBlock(l, gx, gy));
      },
      (e: unknown) => {
        setBusy(describeError(e));
      },
    );
  };
  return (
    <div className="notice not-yet" data-testid="cell-not-yet">
      <h2>Not yet: no world-derived tactical block for cell {gx}, {gy}</h2>
      <p>
        The server answered <b>{notYet.error.status}</b> <code>{notYet.error.code}</code>. World-derived blocks will come from{" "}
        <b className="planned-source">{notYet.planned_source}</b>, which isn't wired into arda-server yet.
      </p>
      <p className="muted">{notYet.error.message}</p>
      <p className="muted small">
        When it lands, this tab renders the Block's layout (<code>POST /v1/tactical/render?origin=X,Y</code>, anchored at
        its origin <code>[{64 * gx}, {64 * gy}]</code>) and overlays its rules sidecar: difficult terrain, water depth and cover.
      </p>
      <div className="toolbar">
        <label>
          Try the Block view with a demo block from{" "}
          <select
            value={pick}
            onChange={(e) => {
              setPick(e.target.value);
            }}
          >
            {(names.length > 0 ? names : [pick]).map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <button type="button" onClick={preview} disabled={busy === "loading…"}>
          Preview demo block
        </button>
        {busy && <span className="muted">{busy}</span>}
      </div>
    </div>
  );
}

function BlockView({
  client,
  block,
  demo,
  cell,
  walk,
}: {
  client: ArdaClient;
  block: TacticalBlockDto;
  demo: boolean;
  /** Set for server blocks: images come from the cell's own seamless tiles. */
  cell?: { gx: number; gy: number; overlays: boolean; grade: boolean };
  /** Set for server blocks: the edge arrows open the neighbouring cell. */
  walk?: Walk;
}) {
  const { layout } = block;
  const [toggles, setToggles] = useState<RulesToggles>({ difficult: true, water: true, cover: true });
  const [grid, setGrid] = useState(true);
  const [hover, setHover] = useState<SquareRef | null>(null);
  const [picked, setPicked] = useState<SquareRef | null>(null);
  const [image, setImage] = useState<{ url: string } | { error: string } | null>(null);
  const { log, record } = useImageLog();
  const [showTokens, setShowTokens] = useState(true);
  const [scene, setScene] = useState<TacticalScene | null>(null);
  const [prefetch, setPrefetch] = useState<string | null>(null);

  // Server blocks: warm the 8 neighbours in the background (goal 67), so
  // walking off an edge finds them cached. Fire and forget: nothing waits.
  useEffect(() => {
    if (!cell) return;
    const ctl = new AbortController();
    whileLive(
      ctl.signal,
      client.tacticalPrefetch(cell.gx, cell.gy, { radius: 1, ppsq: RENDER_PPSQ, demo: cell.overlays }, { signal: ctl.signal }),
      (r) => {
        const queued = r.cells.filter((c) => c.status !== "dropped").length;
        setPrefetch(`${queued}/${r.cells.length} neighbours prefetching`);
      },
      (e) => { setPrefetch(`prefetch: ${describeError(e)}`); },
    );
    return () => {
      ctl.abort();
    };
  }, [client, cell]);

  // Server blocks: the block's scene and its NPC tokens (A12, A13).
  useEffect(() => {
    if (!cell) return;
    const ctl = new AbortController();
    whileLive(ctl.signal, client.tacticalScene(cell.gx, cell.gy, { signal: ctl.signal }, { demo: cell.overlays }), setScene, () => { setScene(null); });
    return () => {
      ctl.abort();
    };
  }, [client, cell]);

  const tokens = useMemo(() => scene?.tokens ?? [], [scene]);
  const grids = useMemo(() => (scene ? sceneGrids(scene.scene) : null), [scene]);
  const rules = useMemo((): { ok: RulesSidecarDto | null } | { error: string } => {
    try {
      return { ok: parseRules(block.rules, layout.width, layout.height) };
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) };
    }
  }, [block.rules, layout.width, layout.height]);
  const sidecar = "ok" in rules ? rules.ok : null;

  // Server blocks: the cell's WebP tiles, rendered with an apron from the
  // neighbouring blocks so edges join (logic/11 §seam-art).
  const loadTile = useMemo<TileLoader | null>(
    () =>
      cell ? (z, x, y, signal) => client.tacticalCellTile(cell.gx, cell.gy, z, x, y, { ppsq: RENDER_PPSQ, demo: cell.overlays, worldGrade: cell.grade }, { signal }).then((img) => {
        record(img);
        return img;
      }) : null,
    [client, cell, record],
  );

  // Viewer-built demo blocks: render the layout, anchored at its origin.
  useEffect(() => {
    if (cell) return;
    const ctl = new AbortController();
    let url: string | null = null;
    whileLive(
      ctl.signal,
      client.renderLayout(layout, { ppsq: RENDER_PPSQ, origin: block.origin }, { signal: ctl.signal }),
      (img) => {
        record(img);
        url = URL.createObjectURL(img.blob);
        setImage({ url });
      },
      (e) => { setImage({ error: describeError(e) }); },
    );
    return () => {
      ctl.abort();
      if (url) URL.revokeObjectURL(url);
    };
  }, [client, layout, block.origin, record, cell]);

  const geometry = useMemo(() => tacticalPyramid(layout.width, layout.height, RENDER_PPSQ), [layout.width, layout.height]);
  const overlay = useMemo(
    () => [...(sidecar ? rulesOverlay(sidecar, toggles) : []), ...(showTokens ? tokenMarks(tokens) : [])],
    [sidecar, toggles, showTokens, tokens],
  );
  const shown = picked ?? hover;
  const marks = useMemo(() => pointerMarks(layout, shown, picked), [layout, shown, picked]);
  const rulesCell = shown && sidecar ? (sidecar.squares[shown.sy * layout.width + shown.sx] ?? null) : null;

  const toggle = (k: keyof RulesToggles) => (
    <label key={k}>
      <input
        type="checkbox"
        checked={toggles[k]}
        disabled={!sidecar}
        onChange={(e) => {
          setToggles((t) => ({ ...t, [k]: e.target.checked }));
        }}
      />{" "}
      {k === "difficult" ? "difficult terrain (hatch)" : k === "water" ? "water depth" : "cover"}
    </label>
  );

  return (
    <div className="split grow" data-testid="block-view">
      <div className="map-wrap">
        <TacticalMap
          width={layout.width}
          height={layout.height}
          geometry={geometry}
          source={
            loadTile && cell
              ? ({ kind: "tiles", key: `cell ${cell.gx},${cell.gy},${cell.overlays ? 1 : 0},${cell.grade ? 1 : 0}`, load: loadTile } satisfies MapSource)
              : image && "url" in image
                ? { kind: "image", key: image.url, url: image.url }
                : { kind: "none" }
          }
          grid={grid}
          overlay={overlay}
          marks={marks}
          onHover={setHover}
          onClick={setPicked}
        />
        <SquareReadout hover={hover} hint="hover for square data · click to pin" />
        {walk && <EdgeArrows walk={walk} at={cell ?? null} />}
        <CacheCorner log={log} source={cell ? "cell tiles" : "POST /render"} />
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
          {toggle("difficult")}
          {toggle("water")}
          {toggle("cover")}
          <label title="GET /v1/tactical/cell/{gx}/{gy}/scene: notables at work (red) or at home (blue)">
            <input
              type="checkbox"
              aria-label="NPC tokens"
              checked={showTokens}
              onChange={(e) => {
                setShowTokens(e.target.checked);
              }}
            />{" "}
            NPC tokens ({tokens.length})
          </label>
          <span className="legend">
            <span className="swatch">
              <i style={{ background: waterFill(2) ?? undefined }} /> wading
            </span>
            <span className="swatch">
              <i style={{ background: waterFill(6) ?? undefined }} /> swimming
            </span>
            {(["half", "three_quarters", "total"] as const).map((c) => (
              <span className="swatch" key={c}>
                <i style={{ borderColor: COVER_COLOURS[c], borderWidth: 2 }} /> {c.replace("_", "-")}
              </span>
            ))}
          </span>
          {prefetch && (
            <span className="muted small" data-testid="prefetch-status">
              {prefetch}
            </span>
          )}
          {!sidecar && "ok" in rules && <span className="muted">this block has no rules sidecar</span>}
          {"error" in rules && <span className="error">{rules.error}</span>}
          {image && "error" in image && <span className="error">render: {image.error}</span>}
        </div>
      </div>
      <aside className="inspector" aria-label="Block data">
        <h2>
          {demo ? "Demo block" : "Block"} · {layout.name}
        </h2>
        <dl>
          <div className="row">
            <dt>Origin (squares)</dt>
            <dd>
              {block.origin[0]}, {block.origin[1]}
            </dd>
          </div>
          <div className="row">
            <dt>Render seed</dt>
            <dd>{block.render_seed}</dd>
          </div>
          <div className="row">
            <dt>Rules</dt>
            <dd>{block.rules ? `format ${block.rules.format_version}, ${block.rules.edges.length} edge rules` : "none"}</dd>
          </div>
          <div className="row">
            <dt>Size</dt>
            <dd>
              {layout.width}×{layout.height} squares
            </dd>
          </div>
          {Object.entries(block.meta).map(([k, v]) => (
            <div className="row" key={k}>
              <dt>meta.{k}</dt>
              <dd>{v}</dd>
            </div>
          ))}
        </dl>
        {shown ? (
          <SquareInfo layout={layout} at={shown} pinned={picked !== null} rules={sidecar ? rulesCell : null} scene={grids ? sceneSquare(grids, shown.sx, shown.sy) : null} />
        ) : (
          <p className="muted">Hover or click a square.</p>
        )}
      </aside>
    </div>
  );
}

