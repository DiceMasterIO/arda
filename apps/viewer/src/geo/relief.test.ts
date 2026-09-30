import { describe, expect, it } from "vitest";
import { deepestZoom, hasRelief, metresPerPx, offersTactical } from "./relief.ts";

// Seed-42 MICRO: 2 × 4 areas of 512 cells at 100 m, a 4096 px base (max zoom 4).
const micro = {
  areas_wide: 2,
  areas_high: 4,
  area_cells: 512,
  cell_size_m: 100,
  tiles: { tile_px: 256, max_zoom: 4, relief_max_zoom: 12 },
};

describe("relief levels", () => {
  it("halve the metres per pixel past the native 50 m", () => {
    expect(metresPerPx(micro, 4)).toBe(50);
    expect(metresPerPx(micro, 5)).toBe(25);
    expect(metresPerPx(micro, 9)).toBe(1.5625);
  });
  it("offer the tactical map near one pixel per 1.5 m square", () => {
    expect(offersTactical(micro, 8)).toBe(false);
    expect(offersTactical(micro, 9)).toBe(true);
  });
  it("fall back to upscaled overview tiles without relief", () => {
    expect(hasRelief(micro)).toBe(true);
    expect(deepestZoom(micro)).toBe(12);
    const none = { ...micro, tiles: { ...micro.tiles, relief_max_zoom: 4 } };
    expect(hasRelief(none)).toBe(false);
    expect(deepestZoom(none)).toBe(7);
  });
});
