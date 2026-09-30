import { describe, expect, it } from "vitest";
import type { TacticalLayoutDto } from "@arda";
import {
  demoBlock,
  groundKeys,
  interpretCellResponse,
  parseBlock,
  parseLayout,
  parseLayouts,
  parseNotYet,
  parseRules,
  parseTokens,
  squareAt,
  wallLine,
  wallsAround,
  withGround,
} from "./tactical.ts";

const sq = (ground: string, elevation_ft = 0, water_depth_ft = 0) => ({ ground, elevation_ft, water_depth_ft });

const layout: TacticalLayoutDto = {
  name: "t",
  width: 2,
  height: 2,
  squares: [sq("grass"), sq("stone_floor", 5), sq("water_shallow", 0, 3), sq("mud")],
  walls: [
    { x: 1, y: 0, axis: "horizontal", kind: "run", kit: "timber" },
    { x: 1, y: 1, axis: "horizontal", kind: "door", kit: "timber" },
    { x: 1, y: 0, axis: "vertical", kind: "window", kit: "timber" },
    { x: 2, y: 0, axis: "vertical", kind: "run", kit: "timber" },
    { x: 0, y: 1, axis: "vertical", kind: "run", kit: "timber" },
  ],
  placements: [],
  lights: [],
};

describe("parseLayouts", () => {
  const tiles = { tile_px: 512, ppsq: 128, image_width_px: 3840, image_height_px: 2176, max_zoom: 3, format: "webp" };
  const body = { world_seed: "42", library_version: "0.1.0+seed58", ppsq_options: [64, 96, 128], default_ppsq: 128, layouts: [{ name: "riverside", width: 30, height: 17, tiles }] };
  it("accepts the documented TacticalLayouts shape", () => {
    const got = parseLayouts(body);
    expect(got.layouts[0]?.name).toBe("riverside");
    expect(got.layouts[0]?.tiles.format).toBe("webp");
  });
  it("rejects the old guessed shapes and broken bodies", () => {
    expect(() => parseLayouts(["riverside"])).toThrow(/not an object/);
    expect(() => parseLayouts({ layouts: [] })).toThrow(/world_seed/);
    expect(() => parseLayouts({ ...body, world_seed: 42 })).toThrow(/decimal string/);
    expect(() => parseLayouts({ ...body, layouts: [{ name: "x", width: 1, height: 1 }] })).toThrow(/tiles/);
  });
});

describe("parseLayout", () => {
  it("accepts a TacticalLayoutDto", () => {
    expect(parseLayout(layout)).toBe(layout);
  });
  it("checks the square count and square fields", () => {
    expect(() => parseLayout({ ...layout, squares: [sq("grass")] })).toThrow(/squares do not match 2x2/);
    expect(() => parseLayout({ ...layout, squares: [{ ground: "grass" }, sq("a"), sq("b"), sq("c")] })).toThrow(/squares\[0\]/);
    expect(() => parseLayout({ ...layout, walls: undefined })).toThrow(/walls/);
  });
});

describe("squareAt", () => {
  it("indexes row-major", () => {
    expect(squareAt(layout, 1, 0)?.ground).toBe("stone_floor");
    expect(squareAt(layout, 0, 1)?.water_depth_ft).toBe(3);
  });
  it("returns null outside the layout", () => {
    expect(squareAt(layout, 2, 0)).toBeNull();
    expect(squareAt(layout, 0, -1)).toBeNull();
    expect(squareAt(layout, 0.5, 0)).toBeNull();
  });
});

describe("wallsAround", () => {
  it("finds walls on the N, E, S and W edges of a square", () => {
    const got = wallsAround(layout, 1, 0).map((w) => `${w.side}:${w.wall.kind}`);
    expect(got).toEqual(["N:run", "E:run", "S:door", "W:window"]);
  });
  it("shares an edge between neighbours", () => {
    // vertical (1, 0) is the east edge of (0, 0) and the west edge of (1, 0).
    expect(wallsAround(layout, 0, 0).map((w) => w.side)).toEqual(["E"]);
    expect(wallsAround(layout, 0, 1).map((w) => w.side)).toEqual(["W"]);
  });
  it("maps segments to edge lines", () => {
    expect(wallLine({ x: 1, y: 2, axis: "horizontal" })).toEqual([1, 2, 2, 2]);
    expect(wallLine({ x: 1, y: 2, axis: "vertical" })).toEqual([1, 2, 1, 3]);
  });
});

describe("editing helpers", () => {
  it("collects sorted, unique ground keys", () => {
    expect(groundKeys([layout, { squares: [sq("grass"), sq("cobbles")] }])).toEqual(["cobbles", "grass", "mud", "stone_floor", "water_shallow"]);
  });
  it("replaces one square's ground without mutating", () => {
    const next = withGround(layout, 1, 1, "cobbles");
    expect(next.squares[3]?.ground).toBe("cobbles");
    expect(layout.squares[3]?.ground).toBe("mud");
    expect(next.squares[0]).toBe(layout.squares[0]);
    expect(() => withGround(layout, 2, 0, "x")).toThrow(/outside/);
  });
});

describe("tactical cell responses", () => {
  const notYet = {
    error: { code: "not_implemented", status: 501, message: "world-derived tactical blocks are not implemented yet; planned source: arda-refine" },
    planned_source: "arda-refine",
  };
  it("reads a NotYetError body", () => {
    expect(parseNotYet(notYet)?.planned_source).toBe("arda-refine");
    expect(parseNotYet({ error: notYet.error })).toBeNull();
    expect(parseNotYet(null)).toBeNull();
  });
  it("maps 501 to not_yet and other non-200 statuses to null", () => {
    expect(interpretCellResponse(501, notYet)).toEqual({ kind: "not_yet", notYet });
    expect(interpretCellResponse(501, "garbage")).toMatchObject({ kind: "not_yet", notYet: { planned_source: "unknown", error: { status: 501 } } });
    expect(interpretCellResponse(404, null)).toBeNull();
  });
  it("parses a 200 Block and rejects a malformed one", () => {
    const tiles = { tile_px: 512, ppsq: 64, image_width_px: 128, image_height_px: 128, max_zoom: 0, format: "webp" };
    const block = {
      tactical_format: 1,
      layout_schema: 1,
      layout,
      rules: null,
      origin: [32768, 66304],
      size: [2, 2],
      render_seed: "42",
      meta: { source: "arda-refine" },
      scene: null,
      tiles,
    };
    expect(interpretCellResponse(200, block)).toEqual({ kind: "block", block });
    expect(() => parseBlock({ ...block, origin: [1] })).toThrow(/origin/);
    expect(() => parseBlock({ layout, origin: [0, 0], meta: {} })).toThrow(/rules/);
    expect(() => parseBlock({ ...block, render_seed: 42 })).toThrow(/render_seed/);
  });
});

describe("parseRules", () => {
  it("accepts null and a matching sidecar", () => {
    expect(parseRules(null, 2, 2)).toBeNull();
    const got = parseRules(
      { format_version: 2, width: 1, height: 2, squares: [{ difficult: true, cover: "full", lightly_obscured: true }, { water_depth_ft: 5, deck: true, ext: { feature: "bridge" } }] },
      1,
      2,
    );
    expect(got?.squares).toEqual([
      { difficult: true, cover: "total", lightly_obscured: true },
      { water_depth_ft: 5, deck: true, ext: { feature: "bridge" } },
    ]);
  });
  it("rejects size mismatches and bad fields", () => {
    expect(() => parseRules({ format_version: 2, width: 3, height: 2, squares: [] }, 2, 2)).toThrow(/size/);
    expect(() => parseRules({ format_version: 2, width: 1, height: 1, squares: [{ cover: "lots" }] }, 1, 1)).toThrow(/cover/);
  });
  it("round-trips the demo block", () => {
    const block = demoBlock(layout, 512, 1036);
    expect(block.origin).toEqual([32768, 66304]);
    expect(parseBlock(block)).toBe(block);
    const rules = parseRules(block.rules, 2, 2);
    expect(rules?.squares[2]).toMatchObject({ water_depth_ft: 3 });
    expect(rules?.squares[3]).toMatchObject({ difficult: true });
    expect(rules?.squares[0]?.cover).toBe("half");
  });
});

describe("parseTokens", () => {
  it("keeps well-formed tokens and drops the rest", () => {
    const good = { npc_id: "10166616894566488805", name: "Brelil Fyler", x: 24, y: 9, building_id: "5", settlement_id: "118", kind: "worker" };
    expect(parseTokens({ tokens: [good, { ...good, x: "1" }, { ...good, kind: "ghost" }] })).toEqual([good]);
    expect(parseTokens(null)).toEqual([]);
    expect(parseTokens({ scene: {} })).toEqual([]);
  });
});
