import { describe, expect, it, test } from "vitest";
import { rulesOverlay, tokenMarks, waterFill } from "./tacticalOverlay.ts";

describe("rulesOverlay", () => {
  const rules = { format_version: 2, width: 2, height: 1, squares: [{ difficult: true, water_depth_ft: 6 }, { cover: "half" as const }] };
  it("draws only the enabled layers, in square units", () => {
    expect(rulesOverlay(rules, { difficult: true, water: false, cover: false })).toEqual([{ kind: "rect", x: 0, y: 0, w: 1, h: 1, hatch: true }]);
    const all = rulesOverlay(rules, { difficult: true, water: true, cover: true });
    expect(all.map((s) => (s.kind === "rect" ? [s.x, s.hatch === true, s.fill ?? s.stroke] : null))).toEqual([
      [0, false, waterFill(6)],
      [0, true, undefined],
      [1.12, false, "#f5c542"],
    ]);
  });
  it("shades wading and swimming depths differently", () => {
    expect(waterFill(0)).toBeNull();
    expect(waterFill(2)).not.toBe(waterFill(5));
  });
});

test("NPC tokens become coloured square markers inside their squares", () => {
  const marks = tokenMarks([
    { npc_id: "1", name: "A", x: 3, y: 4, building_id: "5", settlement_id: "9", kind: "worker" },
    { npc_id: "2", name: "B", x: 0, y: 0, building_id: "6", settlement_id: "9", kind: "resident" },
  ]);
  expect(marks).toHaveLength(2);
  expect(marks[0]).toMatchObject({ kind: "rect", x: 3.18, y: 4.18, fill: "#e5484d" });
  expect(marks[1]).toMatchObject({ fill: "#3e63dd" });
});
