// The mid-zoom relief → tactical hand-off (goal 49): which relief level the
// World view switches to the tactical map at, and which pixels of it cover
// one world cell, so the Cell view can show the two side by side.

import { metresPerPx, offersTactical, type ReliefGrid } from "./relief.ts";

/** The relief tiles and pixel window that cover one cell at the hand-off level. */
export interface HandoffCrop {
  /** Relief zoom level. */
  z: number;
  /** Cell side in relief pixels. */
  size: number;
  /** The cell's north-west corner in level-`z` pixels. */
  x0: number;
  y0: number;
  /** Tile addresses covering the window. */
  tiles: { x: number; y: number }[];
}

/** The first relief level at which the World view offers the tactical map (or the deepest one). */
export function handoffZoom(world: ReliefGrid): number {
  for (let z = world.tiles.max_zoom + 1; z <= world.tiles.relief_max_zoom; z++) {
    if (offersTactical(world, z)) return z;
  }
  return world.tiles.relief_max_zoom;
}

/** The relief window of cell `(gx, gy)` at the hand-off level. */
export function handoffCrop(world: ReliefGrid, gx: number, gy: number): HandoffCrop {
  const z = handoffZoom(world);
  const m = metresPerPx(world, z);
  const size = world.cell_size_m / m;
  const x0 = (gx * world.cell_size_m) / m;
  const y0 = (gy * world.cell_size_m) / m;
  const t = world.tiles.tile_px;
  const tiles: { x: number; y: number }[] = [];
  for (let y = Math.floor(y0 / t); y <= Math.floor((y0 + size - 1) / t); y++) {
    for (let x = Math.floor(x0 / t); x <= Math.floor((x0 + size - 1) / t); x++) {
      tiles.push({ x, y });
    }
  }
  return { z, size, x0, y0, tiles };
}
