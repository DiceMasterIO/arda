import { describe, expect, it } from "vitest";
import { handoffCrop, handoffZoom } from "./handoff.ts";

// Seed-42 MICRO: 2 × 4 areas of 512 cells at 100 m, a 4096 px base (max zoom 4).
const micro = {
  areas_wide: 2,
  areas_high: 4,
  area_cells: 512,
  cell_size_m: 100,
  tiles: { tile_px: 256, max_zoom: 4, relief_max_zoom: 12 },
};

describe("relief → tactical hand-off", () => {
  it("happens at the first level of about a pixel per square", () => {
    expect(handoffZoom(micro)).toBe(9);
    expect(handoffZoom({ ...micro, tiles: { ...micro.tiles, relief_max_zoom: 6 } })).toBe(6);
  });
  it("covers one cell with 64 relief pixels from one tile", () => {
    const c = handoffCrop(micro, 503, 1292);
    expect(c).toEqual({ z: 9, size: 64, x0: 503 * 64, y0: 1292 * 64, tiles: [{ x: 125, y: 323 }] });
  });
});
