import { useState } from "react";
import type { ArdaClient } from "../api/client.ts";
import { handoffCrop } from "../geo/handoff.ts";
import type { ReliefGrid } from "../geo/relief.ts";

const SIDE = 256;

/** The cell as the World view draws it at the hand-off level: relief tiles clipped to the cell. */
function ReliefCell({ client, world, gx, gy }: { client: ArdaClient; world: ReliefGrid; gx: number; gy: number }) {
  const crop = handoffCrop(world, gx, gy);
  const k = SIDE / crop.size;
  const t = world.tiles.tile_px;
  return (
    <div className="handoff-pane" style={{ width: SIDE, height: SIDE }}>
      {crop.tiles.map(({ x, y }) => (
        <img
          key={`${x},${y}`}
          alt=""
          src={client.reliefTileUrl(crop.z, x, y)}
          style={{ left: (x * t - crop.x0) * k, top: (y * t - crop.y0) * k, width: t * k, height: t * k }}
        />
      ))}
    </div>
  );
}

/**
 * Goal 49: the World view's relief at the switch-over zoom next to the
 * tactical map of the same cell, with a cross-fade between them.
 */
export function HandoffCompare({ client, world, gx, gy, worldGrade }: { client: ArdaClient; world: ReliefGrid; gx: number; gy: number; worldGrade: boolean }) {
  const [mix, setMix] = useState(0.5);
  const tactical = client.tacticalCellPngUrl(gx, gy, { ppsq: 16, worldGrade });
  const z = handoffCrop(world, gx, gy).z;
  return (
    <section className="handoff" data-testid="handoff">
      <div className="handoff-row">
        <figure>
          <ReliefCell client={client} world={world} gx={gx} gy={gy} />
          <figcaption>world relief, z{z}</figcaption>
        </figure>
        <figure>
          <div className="handoff-pane" style={{ width: SIDE, height: SIDE }}>
            <ReliefCell client={client} world={world} gx={gx} gy={gy} />
            <img alt="" src={tactical} style={{ left: 0, top: 0, width: SIDE, height: SIDE, opacity: mix }} />
          </div>
          <figcaption>
            <input
              type="range"
              aria-label="cross-fade"
              min={0}
              max={1}
              step={0.05}
              value={mix}
              onChange={(e) => {
                setMix(Number(e.target.value));
              }}
            />{" "}
            cross-fade
          </figcaption>
        </figure>
        <figure>
          <div className="handoff-pane" style={{ width: SIDE, height: SIDE }}>
            <img alt="" src={tactical} style={{ left: 0, top: 0, width: SIDE, height: SIDE }} />
          </div>
          <figcaption>tactical{worldGrade ? " · world grade" : ""}</figcaption>
        </figure>
      </div>
    </section>
  );
}
