import { describe, expect, it } from "vitest";
import type { SceneDto } from "@arda";
import { expandRuns, parseScene, sceneGrids, sceneSquare } from "./scene.ts";

const scene: SceneDto = {
  format_version: 1,
  name: "cell_1_2",
  width: 2,
  height: 2,
  seed: "18446744073709551615",
  library: "placeholder",
  library_version: "0.1.0",
  movement: [
    [3, "normal"],
    [1, "swim"],
  ],
  climb: [
    [1, 0b101],
    [3, 0],
  ],
  cover: [[4, "none"]],
  obscured: [
    [2, "clear"],
    [2, "light"],
  ],
  elevation_ft: [[4, 10]],
  water_depth_ft: [
    [3, 0],
    [1, 6],
  ],
  walls: [],
  vision_blockers: [],
  lights: [],
  regions: [],
  spawn_hints: { open: [[0, 0]], entrances: [], exits: [{ edge: "N", from: [0, 0], to: [1, 0] }] },
  tokens: [],
};

const token = { npc_id: "10166616894566488805", name: "Brelil Fyler", x: 1, y: 0, building_id: "5", settlement_id: "118", kind: "worker" as const };
const body = { tactical_format: 1, origin_gs: [64, 128], time: "day", scene, tokens: [token] };

describe("expandRuns", () => {
  it("expands [count, value] runs row-major and checks the square count", () => {
    expect(expandRuns(scene.movement, 4)).toEqual(["normal", "normal", "normal", "swim"]);
    expect(() => expandRuns(scene.movement, 5)).toThrow(/4 of 5/);
    expect(() => expandRuns(scene.movement, 3)).toThrow(/more than 3/);
    expect(() => expandRuns([[-1, "x"]] as unknown as [number, string][], 1)).toThrow(/pair/);
  });
});

describe("parseScene", () => {
  it("accepts a scene body with tokens and string seeds", () => {
    const got = parseScene(body);
    expect(got.scene.seed).toBe("18446744073709551615");
    expect(got.tokens).toEqual([token]);
  });
  it("rejects malformed bodies", () => {
    expect(() => parseScene({ ...body, time: "noon" })).toThrow(/time/);
    expect(() => parseScene({ ...body, tokens: [{ ...token, kind: "ghost" }] })).toThrow(/tokens\[0\]/);
    expect(() => parseScene({ ...body, scene: { ...scene, seed: 42 } })).toThrow(/seed/);
    expect(() => parseScene({ ...body, scene: { ...scene, cover: [[3, "none"]] } })).toThrow(/cover/);
  });
});

describe("sceneSquare", () => {
  it("reads one square from the expanded grids", () => {
    const g = sceneGrids(scene);
    expect(sceneSquare(g, 0, 0)).toEqual({ movement: "normal", cover: "none", obscured: "clear", climb: [0, 2], elevation_ft: 10, water_depth_ft: 0 });
    expect(sceneSquare(g, 1, 1)).toMatchObject({ movement: "swim", obscured: "light", water_depth_ft: 6, climb: [] });
    expect(sceneSquare(g, 2, 0)).toBeNull();
  });
});
