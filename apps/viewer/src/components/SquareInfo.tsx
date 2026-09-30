import { squareAt, wallsAround, type RulesCellView, type TacticalLayoutDto } from "../api/tactical.ts";
import { SQUARE_FT, squareLabel } from "../geo/coords.ts";
import type { SquareRef } from "./TacticalMap.tsx";

const SIDE_NAMES = { N: "north", E: "east", S: "south", W: "west" } as const;

function depth(ft: number): string {
  if (ft <= 0) return "dry";
  return `${ft} ft${ft >= 5 ? " (swimming)" : " (wading)"}`;
}

/** Ground, elevation, water depth and edge walls of one square, plus its rules when given. */
export function SquareInfo({ layout, at, pinned, rules }: { layout: TacticalLayoutDto; at: SquareRef; pinned: boolean; rules?: RulesCellView | null }) {
  const sq = squareAt(layout, at.sx, at.sy);
  if (!sq) return null;
  const walls = wallsAround(layout, at.sx, at.sy);
  return (
    <div data-testid="square-info">
      <h3>
        Square {squareLabel(at.sx, at.sy)} ({at.sx}, {at.sy}){pinned ? " · pinned" : ""}
      </h3>
      <dl>
        <div className="row">
          <dt>Ground</dt>
          <dd>
            <code>{sq.ground}</code>
          </dd>
        </div>
        <div className="row">
          <dt>Elevation</dt>
          <dd>{sq.elevation_ft} ft</dd>
        </div>
        <div className="row">
          <dt>Water depth</dt>
          <dd>{depth(sq.water_depth_ft)}</dd>
        </div>
        <div className="row">
          <dt>Walls on edges</dt>
          <dd>
            {walls.length === 0 ? (
              <span className="muted">none</span>
            ) : (
              <ul className="plain">
                {walls.map(({ side, wall }) => (
                  <li key={`${side}-${wall.axis}-${wall.x}-${wall.y}`}>
                    {SIDE_NAMES[side]}: <b>{wall.kind}</b> <span className="muted">({wall.kit})</span>
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </div>
      </dl>
      {rules !== undefined && (
        <>
          <h3>Rules sidecar</h3>
          {rules === null ? (
            <p className="muted">No rules sidecar.</p>
          ) : (
            <dl>
              <div className="row">
                <dt>Difficult terrain</dt>
                <dd>{rules.difficult === undefined ? "—" : rules.difficult ? "yes" : "no (cleared)"}</dd>
              </div>
              <div className="row">
                <dt>Water depth</dt>
                <dd>{rules.water_depth_ft === undefined ? "—" : depth(rules.water_depth_ft)}</dd>
              </div>
              <div className="row">
                <dt>Cover</dt>
                <dd>{rules.cover?.replace("_", "-") ?? "—"}</dd>
              </div>
              <div className="row">
                <dt>Blocks sight / move</dt>
                <dd>
                  {rules.blocks_sight === undefined ? "—" : String(rules.blocks_sight)} / {rules.blocks_movement === undefined ? "—" : String(rules.blocks_movement)}
                </dd>
              </div>
              <div className="row">
                <dt>Deck</dt>
                <dd>{rules.deck === undefined ? "—" : String(rules.deck)}</dd>
              </div>
            </dl>
          )}
        </>
      )}
    </div>
  );
}

/** Readout text for the hovered square. */
export function SquareReadout({ hover, hint }: { hover: SquareRef | null; hint: string }) {
  return (
    <div className="readout" data-testid="square-readout">
      {hover ? (
        <>
          square <b>{squareLabel(hover.sx, hover.sy)}</b> ({hover.sx}, {hover.sy}) · {hover.sx * SQUARE_FT}–{(hover.sx + 1) * SQUARE_FT} ft E,{" "}
          {hover.sy * SQUARE_FT}–{(hover.sy + 1) * SQUARE_FT} ft S
        </>
      ) : (
        <span className="muted">{hint}</span>
      )}
    </div>
  );
}
