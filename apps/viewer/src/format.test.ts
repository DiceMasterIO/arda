import { describe, expect, it } from "vitest";
import type { CellSample } from "@arda";
import { aspectLabel, describeCell, metres } from "./format.ts";

// The real seed-42 MICRO response from API.md.
const CELL: CellSample = {
  contract_version: 1, gx: 512, gy: 1036, ax: 1, ay: 2, cx: 0, cy: 12, x_m: 51200.0, y_m: 103600.0,
  centre_x_m: 51250.0, centre_y_m: 103650.0,
  height_m: 78.586, terrain: "land", cover: "forest", slope_deg: 0.0, aspect_deg: 0.0, temperature_c: 11.56,
  rainfall_mm: 2227.0, moisture: 1.0, wetness: 1.0, forest_density: 0.7529411764705882, drainage_area_km2: 65.95,
  discharge_m3s: 2.865, watercourse_order: 4, watercourse_width_m: 6.7, height_above_river_m: 0.0, road: "none",
  built_by: null, coast: { is_coast: false, distance_m: 4517.7427992306075 },
  snow: { fraction: 0.0, peak_fraction: 0.0, perennial: false, snowline_m: 2703.201384615385 },
  river: { segment_id: 210, global_reach_id: "35596688953344", order: 4, width_m: 6.7, discharge_m3s: 2.865 },
  lake: null, fine: { centre_m: 78.586, min_m: 65.708, max_m: 104.07 },
};

function value(label: string): string | undefined {
  return describeCell(CELL).flatMap((s) => s.rows).find((r) => r.label === label)?.value;
}

describe("describeCell", () => {
  it("covers every section in plain language with units", () => {
    expect(describeCell(CELL).map((s) => s.title)).toEqual([
      "Location", "Height", "Terrain and cover", "Climate", "Rivers and lakes", "Coast", "Snow (proxy)",
    ]);
    expect(value("Elevation")).toBe("78.6 m above sea level");
    expect(value("Cover")).toBe("Forest");
    expect(value("Temperature")).toBe("11.6 °C mean annual");
    expect(value("Rainfall")).toBe("2227 mm per year");
    expect(value("Coast")).toBe("4.52 km from the coast");
    expect(value("Lake")).toBe("none");
    expect(value("River")).toContain("order 4");
    expect(value("Snowline")).toBe("2.70 km altitude");
  });
});

describe("helpers", () => {
  it("formats metres and aspect", () => {
    expect(metres(12.34)).toBe("12.3 m");
    expect(metres(1500)).toBe("1.50 km");
    expect(aspectLabel(0, 0)).toBe("flat");
    expect(aspectLabel(90, 10)).toBe("faces E (90°)");
    expect(aspectLabel(350, 10)).toBe("faces N (350°)");
  });
});
