// Mid-zoom relief levels (crates/arda-server/API.md, /v1/tiles/relief):
// zoom levels past the overview pyramid's native zoom, each halving the
// metres per pixel, down to about one pixel per tactical square.

import type { WorldInfo } from "@arda";

/** The subset of WorldInfo the relief geometry needs. */
export type ReliefGrid = Pick<WorldInfo, "areas_wide" | "areas_high" | "area_cells" | "cell_size_m"> & {
  tiles: Pick<WorldInfo["tiles"], "tile_px" | "max_zoom" | "relief_max_zoom">;
};

/** Metres per tactical square (5 ft). */
export const SQUARE_M = 1.524;

/** Pixels this coarse (m/px) or finer offer the tactical map. */
export const TACTICAL_OFFER_M_PER_PX = 2;

/** Metres per pixel at zoom `z`: the pyramid square spans the world's longer side. */
export function metresPerPx(world: ReliefGrid, z: number): number {
  const longest = Math.max(world.areas_wide, world.areas_high) * world.area_cells * world.cell_size_m;
  return longest / (world.tiles.tile_px * 2 ** z);
}

/** Whether the server serves relief levels past the overview. */
export function hasRelief(world: ReliefGrid): boolean {
  return world.tiles.relief_max_zoom > world.tiles.max_zoom;
}

/** Whether zoom `z` is close enough to offer the tactical map. */
export function offersTactical(world: ReliefGrid, z: number): boolean {
  return metresPerPx(world, z) <= TACTICAL_OFFER_M_PER_PX;
}

/** The deepest zoom the World view allows. */
export function deepestZoom(world: ReliefGrid): number {
  return hasRelief(world) ? world.tiles.relief_max_zoom : world.tiles.max_zoom + 3;
}
