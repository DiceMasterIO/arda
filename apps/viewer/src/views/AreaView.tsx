import { useEffect, useRef, useState } from "react";
import type { AreaLakes, AreaRivers, CellSample, WorldInfo } from "@arda";
import { BIN_LEGEND, type BinAreaColumns } from "../api/areaBin.ts";
import { describeError, type ArdaClient } from "../api/client.ts";
import { areaToCell, areasHigh, areasWide } from "../geo/coords.ts";
import { useAsync } from "../hooks.ts";
import { FIELDS, paintField, rgbCss, type FieldName, type Legend } from "../render/heatmap.ts";
import { CellInspector } from "./CellInspector.tsx";

interface AreaData {
  cells: BinAreaColumns;
  rivers: AreaRivers;
  lakes: AreaLakes;
}

/** Overlay resolution per cell, so river lines stay crisp when the canvas is scaled up. */
const OVERLAY_SCALE = 3;

function areaFromHash(world: WorldInfo): { ax: number; ay: number } {
  const q = new URLSearchParams(window.location.hash.split("?")[1] ?? "");
  const clamp = (v: string | null, max: number) => {
    const n = Number(v);
    return Number.isInteger(n) && n >= 0 && n < max ? n : 0;
  };
  return { ax: clamp(q.get("ax"), areasWide(world)), ay: clamp(q.get("ay"), areasHigh(world)) };
}

export function AreaView({ client, world }: { client: ArdaClient; world: WorldInfo }) {
  const [{ ax, ay }, setAreaState] = useState(() => areaFromHash(world));
  const [field, setField] = useState<FieldName>("height_m");
  const [showRivers, setShowRivers] = useState(true);
  const [showLakes, setShowLakes] = useState(true);
  const [hover, setHover] = useState<{ cx: number; cy: number } | null>(null);
  const [inspect, setInspect] = useState<CellSample | null>(null);
  const [legend, setLegend] = useState<Legend | null>(null);
  const base = useRef<HTMLCanvasElement>(null);
  const overlay = useRef<HTMLCanvasElement>(null);

  const data = useAsync<AreaData>(
    async (signal) => {
      const [cells, rivers, lakes] = await Promise.all([
        client.areaCellsBin(ax, ay, { signal }),
        client.areaRivers(ax, ay, { signal }),
        client.areaLakes(ax, ay, { signal }),
      ]);
      return { cells, rivers, lakes };
    },
    [client, ax, ay],
  );

  const setArea = (next: { ax: number; ay: number }) => {
    setAreaState(next);
    setInspect(null);
    window.history.replaceState(null, "", `#/area?ax=${next.ax}&ay=${next.ay}`);
  };

  // Heatmap.
  useEffect(() => {
    const canvas = base.current;
    if (data.state !== "ok" || !canvas) return;
    const { cells } = data.value;
    canvas.width = cells.width;
    canvas.height = cells.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const img = ctx.createImageData(cells.width, cells.height);
    setLegend(paintField(cells, field, BIN_LEGEND.cover, img.data));
    ctx.putImageData(img, 0, 0);
  }, [data, field]);

  // Rivers and lakes.
  useEffect(() => {
    const canvas = overlay.current;
    if (data.state !== "ok" || !canvas) return;
    const { cells, rivers, lakes } = data.value;
    const s = OVERLAY_SCALE;
    canvas.width = cells.width * s;
    canvas.height = cells.height * s;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    if (showLakes) {
      ctx.fillStyle = "rgba(40, 110, 200, 0.85)";
      for (const lake of lakes.lakes) {
        for (const [gx, gy] of lake.cells) ctx.fillRect((gx - cells.gx0) * s, (gy - cells.gy0) * s, s, s);
      }
    }
    if (showRivers) {
      ctx.strokeStyle = "rgba(20, 90, 220, 0.95)";
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      for (const r of rivers.rivers) {
        const pts = r.course;
        const first = pts[0];
        if (!first) continue;
        ctx.lineWidth = (0.7 + 0.3 * Math.min(8, r.order)) * s;
        ctx.beginPath();
        ctx.moveTo((first[0] - cells.gx0 + 0.5) * s, (first[1] - cells.gy0 + 0.5) * s);
        if (pts.length === 1) ctx.lineTo((first[0] - cells.gx0 + 0.5) * s + 0.1, (first[1] - cells.gy0 + 0.5) * s);
        for (const [gx, gy] of pts.slice(1)) ctx.lineTo((gx - cells.gx0 + 0.5) * s, (gy - cells.gy0 + 0.5) * s);
        ctx.stroke();
      }
    }
  }, [data, showRivers, showLakes]);

  const cellAt = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const cx = Math.floor(((e.clientX - r.left) / r.width) * world.area_cells);
    const cy = Math.floor(((e.clientY - r.top) / r.height) * world.area_cells);
    return cx >= 0 && cy >= 0 && cx < world.area_cells && cy < world.area_cells ? { cx, cy } : null;
  };

  const hoverValue = (() => {
    if (!hover || data.state !== "ok") return null;
    const { cells } = data.value;
    const i = hover.cy * cells.width + hover.cx;
    const v = cells.columns[field]?.[i];
    const t = cells.columns.terrain?.[i];
    const terrain = t === undefined ? "" : (BIN_LEGEND.terrain[t] ?? "");
    if (v === undefined) return terrain;
    if (field === "cover") return `${BIN_LEGEND.cover[v] ?? "?"} (${terrain})`;
    const f = FIELDS[field];
    return `${f.label} ${Number.isFinite(v) ? v.toFixed(field === "height_m" ? 1 : 2) : "n/a"} ${f.unit} (${terrain})`;
  })();

  const g = hover ? areaToCell(world, ax, ay, hover.cx, hover.cy) : null;

  return (
    <div className="split">
      <div className="area-main">
        <div className="toolbar">
          <div className="area-picker" aria-label="Area picker" style={{ gridTemplateColumns: `repeat(${areasWide(world)}, 1.6rem)` }}>
            {Array.from({ length: areasHigh(world) }, (_, y) =>
              Array.from({ length: areasWide(world) }, (_, x) => (
                <button
                  key={`${x},${y}`}
                  type="button"
                  className={x === ax && y === ay ? "active" : ""}
                  title={`area ${x},${y}`}
                  onClick={() => {
                    setArea({ ax: x, ay: y });
                  }}
                >
                  {x},{y}
                </button>
              )),
            )}
          </div>
          <label>
            Area{" "}
            <select
              value={`${ax},${ay}`}
              onChange={(e) => {
                const [x, y] = e.target.value.split(",").map(Number);
                setArea({ ax: x ?? 0, ay: y ?? 0 });
              }}
              data-testid="area-select"
            >
              {Array.from({ length: areasHigh(world) }, (_, y) =>
                Array.from({ length: areasWide(world) }, (_, x) => (
                  <option key={`${x},${y}`} value={`${x},${y}`}>
                    {x},{y}
                  </option>
                )),
              )}
            </select>
          </label>
          <label>
            Field{" "}
            <select
              value={field}
              onChange={(e) => {
                setField(e.target.value as FieldName);
              }}
              data-testid="field-select"
            >
              {(Object.keys(FIELDS) as FieldName[]).map((f) => (
                <option key={f} value={f}>
                  {FIELDS[f].label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={showRivers}
              onChange={(e) => {
                setShowRivers(e.target.checked);
              }}
            />{" "}
            rivers
          </label>
          <label>
            <input
              type="checkbox"
              checked={showLakes}
              onChange={(e) => {
                setShowLakes(e.target.checked);
              }}
            />{" "}
            lakes
          </label>
          {data.state === "ok" && (
            <span className="muted">
              {data.value.rivers.rivers.length} river segments · {data.value.lakes.lakes.length} lakes
            </span>
          )}
        </div>
        {data.state === "loading" && <p className="notice">Loading area {ax},{ay} (binary columns, rivers, lakes)…</p>}
        {data.state === "error" && <p className="notice error">{describeError(data.error)}</p>}
        <div className="area-stage" style={{ display: data.state === "ok" ? undefined : "none" }}>
          <canvas ref={base} className="heat" />
          <canvas
            ref={overlay}
            className="heat overlay"
            data-testid="area-canvas"
            onMouseMove={(e) => {
              setHover(cellAt(e));
            }}
            onMouseLeave={() => {
              setHover(null);
            }}
            onClick={(e) => {
              const c = cellAt(e);
              if (!c) return;
              const { gx, gy } = areaToCell(world, ax, ay, c.cx, c.cy);
              client.cell(gx, gy).then(setInspect, (err: unknown) => {
                window.alert(describeError(err));
              });
            }}
          />
        </div>
        <div className="readout static" data-testid="area-readout">
          {hover && g ? (
            <>
              local <b>{hover.cx}, {hover.cy}</b> · global {g.gx}, {g.gy} · {hoverValue}
            </>
          ) : (
            <span className="muted">hover for values · click a cell to inspect it</span>
          )}
        </div>
        {legend && <LegendBar legend={legend} field={field} />}
      </div>
      {inspect && (
        <CellInspector
          cell={inspect}
          onClose={() => {
            setInspect(null);
          }}
        />
      )}
    </div>
  );
}

function LegendBar({ legend, field }: { legend: Legend; field: FieldName }) {
  if (legend.kind === "categories") {
    return (
      <div className="legend">
        {legend.categories?.map((c) => (
          <span key={c.label} className="swatch">
            <i style={{ background: rgbCss(c.colour) }} />
            {c.label}
          </span>
        ))}
        <span className="swatch">
          <i style={{ background: "rgb(38,78,128)" }} />
          sea
        </span>
      </div>
    );
  }
  const stops = legend.stops ?? [];
  const gradient = `linear-gradient(to right, ${stops.map(rgbCss).join(", ")})`;
  const unit = FIELDS[field].unit;
  return (
    <div className="legend">
      <span>
        {legend.min?.toFixed(1)} {unit}
      </span>
      <span className="gradient" style={{ background: gradient }} />
      <span>
        {legend.max?.toFixed(1)} {unit}
      </span>
      <span className="swatch">
        <i style={{ background: "rgb(38,78,128)" }} />
        sea
      </span>
      <span className="swatch">
        <i style={{ background: "rgb(64,120,176)" }} />
        lake
      </span>
    </div>
  );
}
