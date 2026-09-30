// Coordinate conversions between the overview tile pyramid, global cells,
// areas, world metres and tactical squares. All pure functions, unit-tested.
//
// Conventions (crates/arda-server/API.md): x runs east and y runs south from
// the world's north-west corner. Global cell (gx, gy) is the 100 m node at
// (gx*100 m, gy*100 m); area (ax, ay) holds cells ax*512 .. ax*512+511.

import type { WorldInfo } from "@arda";

/** The subset of WorldInfo the conversions need. */
export type WorldGrid = Pick<WorldInfo, "cells_wide" | "cells_high" | "area_cells" | "cell_size_m"> & {
  tiles: Pick<WorldInfo["tiles"], "tile_px" | "max_zoom" | "base_px" | "image_width_px" | "image_height_px">;
};

export interface CellRef {
  gx: number;
  gy: number;
}

export interface AreaRef {
  ax: number;
  ay: number;
  /** Column within the area. */
  cx: number;
  /** Row within the area. */
  cy: number;
}

/** Floor modulo that stays non-negative for negative inputs. */
function mod(a: number, n: number): number {
  return ((a % n) + n) % n;
}

export function inWorld(world: WorldGrid, gx: number, gy: number): boolean {
  return Number.isInteger(gx) && Number.isInteger(gy) && gx >= 0 && gy >= 0 && gx < world.cells_wide && gy < world.cells_high;
}

/**
 * Base-pyramid pixel (the pixel grid at `max_zoom`) to the global cell whose
 * footprint it falls in, or `null` outside the overview image.
 */
export function basePxToCell(world: WorldGrid, px: number, py: number): CellRef | null {
  const { image_width_px: w, image_height_px: h } = world.tiles;
  if (!(px >= 0 && py >= 0 && px < w && py < h)) return null;
  const gx = Math.min(world.cells_wide - 1, Math.floor((px * world.cells_wide) / w));
  const gy = Math.min(world.cells_high - 1, Math.floor((py * world.cells_high) / h));
  return { gx, gy };
}

/** Centre of a cell's footprint in base-pyramid pixels. */
export function cellToBasePx(world: WorldGrid, gx: number, gy: number): { px: number; py: number } {
  return {
    px: ((gx + 0.5) * world.tiles.image_width_px) / world.cells_wide,
    py: ((gy + 0.5) * world.tiles.image_height_px) / world.cells_high,
  };
}

/** Pixel footprint of a cell at the base zoom: [x0, y0, x1, y1]. */
export function cellBoundsBasePx(world: WorldGrid, gx: number, gy: number): [number, number, number, number] {
  const sx = world.tiles.image_width_px / world.cells_wide;
  const sy = world.tiles.image_height_px / world.cells_high;
  return [gx * sx, gy * sy, (gx + 1) * sx, (gy + 1) * sy];
}

export function cellToArea(world: Pick<WorldGrid, "area_cells">, gx: number, gy: number): AreaRef {
  const n = world.area_cells;
  return { ax: Math.floor(gx / n), ay: Math.floor(gy / n), cx: mod(gx, n), cy: mod(gy, n) };
}

export function areaToCell(world: Pick<WorldGrid, "area_cells">, ax: number, ay: number, cx: number, cy: number): CellRef {
  return { gx: ax * world.area_cells + cx, gy: ay * world.area_cells + cy };
}

/** Node position of a cell in world metres. */
export function cellToMetres(world: Pick<WorldGrid, "cell_size_m">, gx: number, gy: number): { x_m: number; y_m: number } {
  return { x_m: gx * world.cell_size_m, y_m: gy * world.cell_size_m };
}

/** The cell whose footprint (centred on its node) contains a world position. */
export function metresToCell(world: Pick<WorldGrid, "cell_size_m">, xM: number, yM: number): CellRef {
  return { gx: Math.round(xM / world.cell_size_m), gy: Math.round(yM / world.cell_size_m) };
}

export function areasWide(world: Pick<WorldGrid, "cells_wide" | "area_cells">): number {
  return Math.ceil(world.cells_wide / world.area_cells);
}

export function areasHigh(world: Pick<WorldGrid, "cells_high" | "area_cells">): number {
  return Math.ceil(world.cells_high / world.area_cells);
}

/** Scale from base pixels to screen pixels at a Leaflet zoom level. */
export function zoomScale(maxZoom: number, zoom: number): number {
  return 2 ** (zoom - maxZoom);
}

/** The slippy tile (at zoom z) containing a base-pyramid pixel. */
export function basePxToTile(world: WorldGrid, z: number, px: number, py: number): { z: number; x: number; y: number } | null {
  const { tile_px, max_zoom, base_px } = world.tiles;
  if (z < 0 || z > max_zoom || px < 0 || py < 0 || px >= base_px || py >= base_px) return null;
  const span = tile_px * 2 ** (max_zoom - z);
  return { z, x: Math.floor(px / span), y: Math.floor(py / span) };
}

// --- Tactical squares -------------------------------------------------------

/** Feet per tactical square. */
export const SQUARE_FT = 5;

/** A position in square units (1 unit = one 5-ft square) to the square index. */
export function unitsToSquare(width: number, height: number, ux: number, uy: number): { sx: number; sy: number } | null {
  if (!(ux >= 0 && uy >= 0 && ux < width && uy < height)) return null;
  return { sx: Math.floor(ux), sy: Math.floor(uy) };
}

/** Chess-style label for a square: column letters A, B, ... Z, AA, ...; rows 1-based. */
export function squareLabel(sx: number, sy: number): string {
  let n = sx + 1;
  let letters = "";
  while (n > 0) {
    const r = (n - 1) % 26;
    letters = String.fromCharCode(65 + r) + letters;
    n = Math.floor((n - 1) / 26);
  }
  return `${letters}${sy + 1}`;
}
