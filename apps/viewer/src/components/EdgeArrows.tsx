import { ARROWS, DIRECTIONS, type Cell, type Direction } from "../geo/cellWalk.ts";

/** Edge walking for server blocks: which directions lead somewhere, and how to go. */
export interface Walk {
  onWalk: (dir: Direction) => void;
  can: (dir: Direction) => boolean;
}

/** Arrow buttons on the map's edges: each opens the seamless neighbour that way. */
export function EdgeArrows({ walk, at }: { walk: Walk; at: Cell | null }) {
  return (
    <>
      {DIRECTIONS.map((d) => (
        <button
          key={d}
          type="button"
          className={`edge-walk edge-${d}`}
          aria-label={`walk ${d}`}
          title={at ? `open the cell to the ${d} (seamless neighbour of ${at.gx}, ${at.gy})` : `walk ${d}`}
          disabled={!walk.can(d)}
          onClick={() => {
            walk.onWalk(d);
          }}
        >
          {ARROWS[d]}
        </button>
      ))}
    </>
  );
}
