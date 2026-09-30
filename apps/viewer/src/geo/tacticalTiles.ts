// Geometry of the tactical WebP tile pyramid (crates/arda-server/API.md,
// "GET /v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp"). Pure functions.
//
// - The full render is W x H = width*ppsq x height*ppsq pixels.
// - max_zoom is the least z with tile_px * 2^z >= max(W, H); it is the render
//   at full resolution, and each lower level halves the one above, rounding up.
// - Level z is ceil(W / 2^(max_zoom - z)) x ceil(H / 2^(max_zoom - z)) pixels
//   and holds ceil(w_z / tile_px) x ceil(h_z / tile_px) tiles, anchored top-left.
//
// The viewer's Leaflet map uses square units (one unit = one 5-ft square), so
// at zoom z one square is ppsq * 2^(z - max_zoom) screen pixels, and Leaflet's
// tile (z, x, y) of `tile_px` screen pixels is exactly the server's tile.

import type { TacticalTilesDto } from "@arda";

export type TileGeometry = Pick<TacticalTilesDto, "tile_px" | "ppsq" | "image_width_px" | "image_height_px" | "max_zoom">;

export const TACTICAL_TILE_PX = 512;

/** The pyramid the server cuts for a width x height layout at `ppsq`. */
export function tacticalPyramid(width: number, height: number, ppsq: number, tilePx = TACTICAL_TILE_PX): TileGeometry {
  const w = width * ppsq;
  const h = height * ppsq;
  const longest = Math.max(w, h);
  let z = 0;
  while (tilePx * 2 ** z < longest) z++;
  return { tile_px: tilePx, ppsq, image_width_px: w, image_height_px: h, max_zoom: z };
}

/** Pixel size of pyramid level `z`. */
export function levelSize(g: TileGeometry, z: number): { w: number; h: number } {
  const div = 2 ** (g.max_zoom - z);
  return { w: Math.ceil(g.image_width_px / div), h: Math.ceil(g.image_height_px / div) };
}

/** Tile grid of level `z`: columns and rows. */
export function tileGrid(g: TileGeometry, z: number): { cols: number; rows: number } {
  const { w, h } = levelSize(g, z);
  return { cols: Math.ceil(w / g.tile_px), rows: Math.ceil(h / g.tile_px) };
}

/** True when the server has tile `(z, x, y)`; everything else is a 404. */
export function tileExists(g: TileGeometry, z: number, x: number, y: number): boolean {
  if (!Number.isInteger(z) || !Number.isInteger(x) || !Number.isInteger(y)) return false;
  if (z < 0 || z > g.max_zoom || x < 0 || y < 0) return false;
  const { cols, rows } = tileGrid(g, z);
  return x < cols && y < rows;
}

/** Screen pixels per square at Leaflet zoom `z`. */
export function pxPerSquare(g: Pick<TileGeometry, "ppsq" | "max_zoom">, z: number): number {
  return g.ppsq * 2 ** (z - g.max_zoom);
}

/** The Leaflet zoom at which one square is `px` screen pixels. */
export function zoomForPxPerSquare(g: Pick<TileGeometry, "ppsq" | "max_zoom">, px: number): number {
  return g.max_zoom + Math.log2(px / g.ppsq);
}

/**
 * CRS scale factor for square units: one square is `ppsq` pixels at
 * max_zoom, and Leaflet multiplies by 2^z, so scale = ppsq / 2^max_zoom.
 */
export function squareCrsScale(g: Pick<TileGeometry, "ppsq" | "max_zoom">): number {
  return g.ppsq / 2 ** g.max_zoom;
}

/** Area covered by tile `(z, x, y)` in square units: [x0, y0, x1, y1]. */
export function tileBoundsSquares(g: TileGeometry, z: number, x: number, y: number): [number, number, number, number] {
  const span = (g.tile_px * 2 ** (g.max_zoom - z)) / g.ppsq;
  return [x * span, y * span, (x + 1) * span, (y + 1) * span];
}

/** The tile of level `z` holding square-unit position `(ux, uy)`, or `null` outside the pyramid. */
export function squareToTile(g: TileGeometry, z: number, ux: number, uy: number): { z: number; x: number; y: number } | null {
  const span = (g.tile_px * 2 ** (g.max_zoom - z)) / g.ppsq;
  const x = Math.floor(ux / span);
  const y = Math.floor(uy / span);
  return tileExists(g, z, x, y) && ux < g.image_width_px / g.ppsq && uy < g.image_height_px / g.ppsq ? { z, x, y } : null;
}
