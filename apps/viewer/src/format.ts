// Plain-language formatting of CellSample fields, with units.

import type { CellSample, CoverDto, TerrainKindDto, TerminusDto } from "@arda";

export function metres(v: number, digits = 1): string {
  if (Math.abs(v) >= 1000) return `${(v / 1000).toFixed(2)} km`;
  return `${v.toFixed(digits)} m`;
}

export function pct(v: number): string {
  return `${Math.round(v * 100)}%`;
}

const TERRAIN: Record<TerrainKindDto, string> = { sea: "Sea", land: "Dry land", lake: "Lake surface" };

const COVER: Record<CoverDto, string> = {
  bare: "Bare ground",
  grass: "Grassland",
  scrub: "Scrub",
  forest: "Forest",
  marsh: "Marsh",
  rock: "Bare rock",
  ice: "Permanent ice",
};

export const TERMINUS: Record<TerminusDto, string> = {
  junction: "joins another river",
  sea: "reaches the sea",
  lake: "enters a lake",
  off_tile: "leaves the area",
  basin: "ends in a closed basin",
  divergence: "splits off",
};

export function terrainLabel(t: TerrainKindDto): string {
  return TERRAIN[t];
}

export function coverLabel(c: CoverDto): string {
  return COVER[c];
}

const COMPASS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"] as const;

/** Downslope compass bearing in words; "flat" when there's no slope. */
export function aspectLabel(aspectDeg: number, slopeDeg: number): string {
  if (slopeDeg <= 0) return "flat";
  const i = Math.round((((aspectDeg % 360) + 360) % 360) / 45) % 8;
  return `faces ${COMPASS[i] ?? "N"} (${Math.round(aspectDeg)}°)`;
}

export interface Row {
  label: string;
  value: string;
}

export interface Section {
  title: string;
  rows: Row[];
}

/** Groups a CellSample into readable sections for the inspector. */
export function describeCell(c: CellSample): Section[] {
  const s: Section[] = [];
  s.push({
    title: "Location",
    rows: [
      { label: "Global cell", value: `${c.gx}, ${c.gy}` },
      { label: "Area / local", value: `area ${c.ax},${c.ay} · cell ${c.cx},${c.cy}` },
      { label: "Position", value: `${(c.x_m / 1000).toFixed(1)} km east, ${(c.y_m / 1000).toFixed(1)} km south` },
    ],
  });
  const height: Row[] = [{ label: "Elevation", value: `${metres(c.height_m)} above sea level` }];
  if (c.fine) {
    height.push({
      label: "Fine relief",
      value: `${metres(c.fine.min_m)} to ${metres(c.fine.max_m)} across the 100 m cell`,
    });
  }
  height.push(
    { label: "Slope", value: `${c.slope_deg.toFixed(1)}°, ${aspectLabel(c.aspect_deg, c.slope_deg)}` },
    { label: "Above nearest channel", value: metres(c.height_above_river_m) },
  );
  s.push({ title: "Height", rows: height });
  s.push({
    title: "Terrain and cover",
    rows: [
      { label: "Terrain", value: terrainLabel(c.terrain) },
      { label: "Cover", value: coverLabel(c.cover) },
      { label: "Forest density", value: `${pct(c.forest_density)} canopy closure` },
      { label: "Road", value: c.road === "none" ? "none" : c.road },
      { label: "Settlement", value: c.built_by === null ? "none" : `#${c.built_by}` },
    ],
  });
  s.push({
    title: "Climate",
    rows: [
      { label: "Temperature", value: `${c.temperature_c.toFixed(1)} °C mean annual` },
      { label: "Rainfall", value: `${Math.round(c.rainfall_mm)} mm per year` },
      { label: "Soil moisture", value: pct(c.moisture) },
      { label: "Wetness", value: `${pct(c.wetness)} standing-water tendency` },
    ],
  });
  const water: Row[] = [
    { label: "Drainage area", value: `${c.drainage_area_km2.toFixed(2)} km² upstream` },
    { label: "Discharge", value: `${c.discharge_m3s.toFixed(3)} m³/s mean` },
  ];
  if (c.watercourse_order > 0) {
    water.push({
      label: "Watercourse",
      value: `Strahler order ${c.watercourse_order}, ${metres(c.watercourse_width_m)} wide`,
    });
  }
  if (c.river) {
    water.push({
      label: "River",
      value: `segment ${c.river.segment_id} (reach ${c.river.global_reach_id}), order ${c.river.order}, ${metres(c.river.width_m)} wide, ${c.river.discharge_m3s.toFixed(3)} m³/s`,
    });
  } else {
    water.push({ label: "River", value: "no saved river here" });
  }
  if (c.lake) {
    water.push({
      label: "Lake",
      value: `lake ${c.lake.lake_id} (basin ${c.lake.global_basin_id}), surface ${metres(c.lake.surface_m)}, ${metres(c.lake.depth_m)} deep here (max ${metres(c.lake.max_depth_m)})`,
    });
  } else {
    water.push({ label: "Lake", value: "none" });
  }
  s.push({ title: "Rivers and lakes", rows: water });
  s.push({
    title: "Coast",
    rows: [
      {
        label: "Coast",
        value: c.coast.is_coast
          ? "on the coast"
          : c.coast.distance_m === null
            ? "more than 5 km from the coast"
            : `${metres(c.coast.distance_m, 0)} from the coast`,
      },
    ],
  });
  s.push({
    title: "Snow (proxy)",
    rows: [
      { label: "Snow cover", value: `${pct(c.snow.fraction)}${c.snow.perennial ? ", perennial" : ""}` },
      {
        label: "At the highest point",
        value: c.snow.peak_fraction === null ? "n/a (no fine terrain)" : pct(c.snow.peak_fraction),
      },
      { label: "Snowline", value: `${metres(c.snow.snowline_m, 0)} altitude` },
    ],
  });
  return s;
}
