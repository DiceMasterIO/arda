import { describe, expect, it } from "vitest";
import { cellHash, DIRECTIONS, neighbour } from "./cellWalk.ts";

describe("walking between cells", () => {
  const extent = { cells_wide: 1024, cells_high: 2048 };
  it("steps one cell in each direction", () => {
    const c = { gx: 618, gy: 689 };
    expect(neighbour(c, "north", extent)).toEqual({ gx: 618, gy: 688 });
    expect(neighbour(c, "south", extent)).toEqual({ gx: 618, gy: 690 });
    expect(neighbour(c, "east", extent)).toEqual({ gx: 619, gy: 689 });
    expect(neighbour(c, "west", extent)).toEqual({ gx: 617, gy: 689 });
    expect(DIRECTIONS).toHaveLength(4);
  });
  it("stops at the world's edges", () => {
    expect(neighbour({ gx: 0, gy: 5 }, "west", extent)).toBeNull();
    expect(neighbour({ gx: 5, gy: 0 }, "north", null)).toBeNull();
    expect(neighbour({ gx: 1023, gy: 5 }, "east", extent)).toBeNull();
    expect(neighbour({ gx: 5, gy: 2047 }, "south", extent)).toBeNull();
    expect(neighbour({ gx: 1023, gy: 5 }, "east", null)).toEqual({ gx: 1024, gy: 5 });
  });
  it("builds the tab's hash", () => {
    expect(cellHash({ gx: 3, gy: 4 }, false)).toBe("#/cell?gx=3&gy=4");
    expect(cellHash({ gx: 3, gy: 4 }, true)).toBe("#/cell?gx=3&gy=4&demo=1");
    expect(cellHash({ gx: 3, gy: 4 }, false, true)).toBe("#/cell?gx=3&gy=4&grade=1");
  });
});
