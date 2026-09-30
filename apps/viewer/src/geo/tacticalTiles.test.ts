import { describe, expect, it } from "vitest";
import {
  levelSize,
  pxPerSquare,
  squareCrsScale,
  squareToTile,
  tacticalPyramid,
  tileBoundsSquares,
  tileExists,
  tileGrid,
  zoomForPxPerSquare,
} from "./tacticalTiles.ts";

// riverside: 30 x 17 squares; the server lists 3840 x 2176 px, max_zoom 3 at 128 ppsq.
const riverside = tacticalPyramid(30, 17, 128);

describe("tacticalPyramid", () => {
  it("matches the server's listing for riverside", () => {
    expect(riverside).toEqual({ tile_px: 512, ppsq: 128, image_width_px: 3840, image_height_px: 2176, max_zoom: 3 });
  });
  it("picks the least zoom whose 512 * 2^z covers the long edge", () => {
    expect(tacticalPyramid(4, 4, 128).max_zoom).toBe(0); // exactly 512
    expect(tacticalPyramid(4, 5, 128).max_zoom).toBe(1); // 640
    expect(tacticalPyramid(30, 17, 64).max_zoom).toBe(2); // 1920
    expect(tacticalPyramid(16, 9, 96).max_zoom).toBe(2); // 1536
  });
});

describe("pyramid levels", () => {
  it("halves each level, rounding up", () => {
    expect(levelSize(riverside, 3)).toEqual({ w: 3840, h: 2176 });
    expect(levelSize(riverside, 2)).toEqual({ w: 1920, h: 1088 });
    expect(levelSize(riverside, 1)).toEqual({ w: 960, h: 544 });
    expect(levelSize(riverside, 0)).toEqual({ w: 480, h: 272 });
    const odd = tacticalPyramid(1, 1, 1000 / 1); // 1000 px, max_zoom 1
    expect(levelSize(odd, 0)).toEqual({ w: 500, h: 500 });
    const odder = { tile_px: 512, ppsq: 1, image_width_px: 1001, image_height_px: 3, max_zoom: 1 };
    expect(levelSize(odder, 0)).toEqual({ w: 501, h: 2 });
  });
  it("counts tiles per level", () => {
    expect(tileGrid(riverside, 3)).toEqual({ cols: 8, rows: 5 });
    expect(tileGrid(riverside, 2)).toEqual({ cols: 4, rows: 3 });
    expect(tileGrid(riverside, 0)).toEqual({ cols: 1, rows: 1 });
  });
  it("knows which tiles exist (the rest are 404)", () => {
    expect(tileExists(riverside, 3, 7, 4)).toBe(true);
    expect(tileExists(riverside, 3, 8, 0)).toBe(false);
    expect(tileExists(riverside, 3, 0, 5)).toBe(false);
    expect(tileExists(riverside, 4, 0, 0)).toBe(false);
    expect(tileExists(riverside, -1, 0, 0)).toBe(false);
    expect(tileExists(riverside, 0, 0, 0)).toBe(true);
  });
});

describe("square units and zoom", () => {
  it("scales the CRS so a square is ppsq px at max_zoom", () => {
    expect(squareCrsScale(riverside)).toBe(16);
    expect(pxPerSquare(riverside, 3)).toBe(128);
    expect(pxPerSquare(riverside, 0)).toBe(16);
    expect(zoomForPxPerSquare(riverside, 128)).toBe(3);
    expect(zoomForPxPerSquare(riverside, 32)).toBe(1);
  });
  it("gives each tile's square-unit bounds", () => {
    // At max_zoom a 512 px tile is 4 squares at 128 ppsq.
    expect(tileBoundsSquares(riverside, 3, 2, 1)).toEqual([8, 4, 12, 8]);
    // Zoom 0 covers 32 squares per tile, the whole layout.
    expect(tileBoundsSquares(riverside, 0, 0, 0)).toEqual([0, 0, 32, 32]);
  });
  it("maps a square position to its tile, or null outside", () => {
    expect(squareToTile(riverside, 3, 29.5, 16.5)).toEqual({ z: 3, x: 7, y: 4 });
    expect(squareToTile(riverside, 1, 17, 9)).toEqual({ z: 1, x: 1, y: 0 }); // 16 squares per tile
    expect(squareToTile(riverside, 1, 17, 16.5)).toEqual({ z: 1, x: 1, y: 1 });
    expect(squareToTile(riverside, 3, 30, 0)).toBeNull();
    expect(squareToTile(riverside, 3, -0.1, 0)).toBeNull();
  });
});
