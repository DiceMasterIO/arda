// Walking between world cells in the Cell tab (goal 67). A cell's image is
// rendered with an apron from its neighbours, so the neighbour one step
// away joins it seamlessly; these helpers only pick which cell that is.

export type Direction = "north" | "east" | "south" | "west";

export const DIRECTIONS: readonly Direction[] = ["north", "west", "east", "south"];

const STEPS: Record<Direction, readonly [number, number]> = {
  north: [0, -1],
  east: [1, 0],
  south: [0, 1],
  west: [-1, 0],
};

export const ARROWS: Record<Direction, string> = { north: "↑", east: "→", south: "↓", west: "←" };

export interface Cell {
  gx: number;
  gy: number;
}

/** World extent in cells, when known (`/v1/world`). */
export interface CellExtent {
  cells_wide: number;
  cells_high: number;
}

/** The cell one step from `cell`, or null past the world's edge. */
export function neighbour(cell: Cell, dir: Direction, extent: CellExtent | null): Cell | null {
  const [dx, dy] = STEPS[dir];
  const gx = cell.gx + dx;
  const gy = cell.gy + dy;
  if (gx < 0 || gy < 0) return null;
  if (extent && (gx >= extent.cells_wide || gy >= extent.cells_high)) return null;
  return { gx, gy };
}

/** The Cell tab's location hash for `cell`. */
export function cellHash(cell: Cell, overlays: boolean): string {
  return `#/cell?gx=${cell.gx}&gy=${cell.gy}${overlays ? "&demo=1" : ""}`;
}
