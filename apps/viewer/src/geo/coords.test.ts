import { describe, expect, it } from "vitest";
import {
  areaToCell,
  areasHigh,
  areasWide,
  basePxToCell,
  basePxToTile,
  cellBoundsBasePx,
  cellToArea,
  cellToBasePx,
  cellToMetres,
  inWorld,
  metresToCell,
  squareLabel,
  unitsToSquare,
  zoomScale,
  type WorldGrid,
} from "./coords.ts";

// The seed-42 MICRO world from API.md.
const MICRO: WorldGrid = {
  cells_wide: 1024,
  cells_high: 2048,
  area_cells: 512,
  cell_size_m: 100,
  tiles: { tile_px: 256, max_zoom: 4, base_px: 4096, image_width_px: 2048, image_height_px: 4096 },
};

describe("pyramid pixels and cells", () => {
  it("maps base pixels to the cell footprint they fall in", () => {
    expect(basePxToCell(MICRO, 0, 0)).toEqual({ gx: 0, gy: 0 });
    expect(basePxToCell(MICRO, 1.99, 1.99)).toEqual({ gx: 0, gy: 0 });
    expect(basePxToCell(MICRO, 2, 2)).toEqual({ gx: 1, gy: 1 });
    expect(basePxToCell(MICRO, 1024, 2072)).toEqual({ gx: 512, gy: 1036 });
    expect(basePxToCell(MICRO, 2047.9, 4095.9)).toEqual({ gx: 1023, gy: 2047 });
  });
  it("returns null outside the overview image (the transparent pyramid margin)", () => {
    expect(basePxToCell(MICRO, -0.1, 5)).toBeNull();
    expect(basePxToCell(MICRO, 2048, 5)).toBeNull();
    expect(basePxToCell(MICRO, 5, 4096)).toBeNull();
    expect(basePxToCell(MICRO, NaN, 5)).toBeNull();
  });
  it("round-trips cell centres", () => {
    for (const [gx, gy] of [[0, 0], [512, 1036], [1023, 2047]] as const) {
      const { px, py } = cellToBasePx(MICRO, gx, gy);
      expect(basePxToCell(MICRO, px, py)).toEqual({ gx, gy });
    }
  });
  it("gives cell footprints in base pixels", () => {
    expect(cellBoundsBasePx(MICRO, 3, 5)).toEqual([6, 10, 8, 12]);
  });
  it("finds the slippy tile of a base pixel", () => {
    expect(basePxToTile(MICRO, 4, 300, 10)).toEqual({ z: 4, x: 1, y: 0 });
    expect(basePxToTile(MICRO, 0, 4095, 4095)).toEqual({ z: 0, x: 0, y: 0 });
    expect(basePxToTile(MICRO, 2, 2047, 3000)).toEqual({ z: 2, x: 1, y: 2 });
    expect(basePxToTile(MICRO, 5, 0, 0)).toBeNull();
    expect(basePxToTile(MICRO, 1, 4096, 0)).toBeNull();
  });
  it("scales zoom levels relative to max_zoom", () => {
    expect(zoomScale(4, 4)).toBe(1);
    expect(zoomScale(4, 0)).toBe(1 / 16);
    expect(zoomScale(4, 6)).toBe(4);
  });
});

describe("cells, areas and metres", () => {
  it("splits a global cell into area and local cell (API.md example)", () => {
    expect(cellToArea(MICRO, 512, 1036)).toEqual({ ax: 1, ay: 2, cx: 0, cy: 12 });
    expect(cellToArea(MICRO, 511, 0)).toEqual({ ax: 0, ay: 0, cx: 511, cy: 0 });
  });
  it("keeps local cells non-negative for negative globals", () => {
    expect(cellToArea(MICRO, -1, -513)).toEqual({ ax: -1, ay: -2, cx: 511, cy: 511 });
  });
  it("joins area and local cell back to global", () => {
    expect(areaToCell(MICRO, 1, 2, 0, 12)).toEqual({ gx: 512, gy: 1036 });
  });
  it("converts between nodes and metres", () => {
    expect(cellToMetres(MICRO, 512, 1036)).toEqual({ x_m: 51200, y_m: 103600 });
    expect(metresToCell(MICRO, 51234.5, 103617.25)).toEqual({ gx: 512, gy: 1036 });
    expect(metresToCell(MICRO, 51250.1, 103649)).toEqual({ gx: 513, gy: 1036 });
  });
  it("counts areas and bounds-checks cells", () => {
    expect(areasWide(MICRO)).toBe(2);
    expect(areasHigh(MICRO)).toBe(4);
    expect(inWorld(MICRO, 1023, 2047)).toBe(true);
    expect(inWorld(MICRO, 1024, 0)).toBe(false);
    expect(inWorld(MICRO, 0.5, 0)).toBe(false);
  });
});

describe("tactical squares", () => {
  it("maps square units to square indices", () => {
    expect(unitsToSquare(40, 30, 0, 0)).toEqual({ sx: 0, sy: 0 });
    expect(unitsToSquare(40, 30, 3.5, 7.99)).toEqual({ sx: 3, sy: 7 });
    expect(unitsToSquare(40, 30, 40, 0)).toBeNull();
    expect(unitsToSquare(40, 30, -0.01, 0)).toBeNull();
  });
  it("labels squares chess-style", () => {
    expect(squareLabel(0, 0)).toBe("A1");
    expect(squareLabel(25, 9)).toBe("Z10");
    expect(squareLabel(26, 0)).toBe("AA1");
    expect(squareLabel(51, 0)).toBe("AZ1");
    expect(squareLabel(52, 0)).toBe("BA1");
  });
});
