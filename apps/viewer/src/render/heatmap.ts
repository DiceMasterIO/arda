// Pure pixel painting for the area heatmap: field column -> RGBA bytes.

import type { CoverDto } from "@arda";
import type { BinAreaColumns, ColumnArray } from "../api/areaBin.ts";

export type Rgb = readonly [number, number, number];

export const FIELDS = {
  height_m: { label: "Height", unit: "m" },
  cover: { label: "Cover", unit: "" },
  moisture: { label: "Soil moisture", unit: "0–1" },
  forest_density: { label: "Forest density", unit: "0–1" },
  temperature_c: { label: "Temperature", unit: "°C" },
  slope_deg: { label: "Slope", unit: "°" },
} as const;

export type FieldName = keyof typeof FIELDS;

export const COVER_COLOURS: Record<CoverDto, Rgb> = {
  bare: [196, 176, 140],
  grass: [150, 196, 96],
  scrub: [168, 160, 86],
  forest: [44, 110, 52],
  marsh: [96, 140, 124],
  rock: [128, 124, 120],
  ice: [236, 244, 250],
};

const SEA: Rgb = [38, 78, 128];
const LAKE: Rgb = [64, 120, 176];

/** Linear interpolation through evenly spaced colour stops, t in [0, 1]. */
export function ramp(stops: readonly Rgb[], t: number): Rgb {
  const first = stops[0];
  if (!first) return [0, 0, 0];
  if (!(t > 0)) return first;
  if (t >= 1) return stops[stops.length - 1] ?? first;
  const f = t * (stops.length - 1);
  const i = Math.floor(f);
  const a = stops[i] ?? first;
  const b = stops[i + 1] ?? a;
  const u = f - i;
  return [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u];
}

const HYPSO: Rgb[] = [
  [70, 128, 70],
  [150, 180, 96],
  [220, 204, 140],
  [176, 128, 84],
  [140, 110, 100],
  [250, 250, 250],
];
const SEQ: Rgb[] = [
  [247, 244, 232],
  [180, 214, 160],
  [70, 150, 110],
  [16, 76, 72],
];
const THERMAL: Rgb[] = [
  [60, 80, 170],
  [140, 190, 220],
  [245, 240, 200],
  [240, 150, 80],
  [180, 40, 40],
];

/** Min and max of the finite values of a column where `mask` (if any) is true. */
export function finiteRange(col: ColumnArray, mask?: (i: number) => boolean): [number, number] {
  let lo = Infinity;
  let hi = -Infinity;
  for (let i = 0; i < col.length; i++) {
    const v = col[i] ?? NaN;
    if (!Number.isFinite(v) || (mask && !mask(i))) continue;
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return lo <= hi ? [lo, hi] : [0, 1];
}

export interface Legend {
  kind: "ramp" | "categories";
  min?: number;
  max?: number;
  stops?: readonly Rgb[];
  categories?: { label: string; colour: Rgb }[];
}

/** Paints `field` into `out` (RGBA, width*height*4). Water is drawn as water. */
export function paintField(area: BinAreaColumns, field: FieldName, legendCover: readonly CoverDto[], out: Uint8ClampedArray): Legend {
  const col = area.columns[field];
  const terrain = area.columns.terrain;
  const n = area.width * area.height;
  if (!col || !terrain) throw new Error(`area has no ${field} column`);
  const isLand = (i: number) => terrain[i] === 1;

  let legend: Legend;
  let colour: (v: number) => Rgb;
  if (field === "cover") {
    colour = (v) => COVER_COLOURS[legendCover[v] ?? "bare"];
    legend = { kind: "categories", categories: legendCover.map((c) => ({ label: c, colour: COVER_COLOURS[c] })) };
  } else {
    const fixed = field === "moisture" || field === "forest_density";
    const [lo, hi] = fixed ? [0, 1] : finiteRange(col, isLand);
    const stops = field === "height_m" ? HYPSO : field === "temperature_c" ? THERMAL : SEQ;
    const span = hi - lo || 1;
    colour = (v) => ramp(stops, (v - lo) / span);
    legend = { kind: "ramp", min: lo, max: hi, stops };
  }

  for (let i = 0; i < n; i++) {
    const t = terrain[i];
    const c = t === 0 ? SEA : t === 2 && field !== "cover" ? LAKE : colour(col[i] ?? 0);
    const o = i * 4;
    out[o] = c[0];
    out[o + 1] = c[1];
    out[o + 2] = c[2];
    out[o + 3] = 255;
  }
  return legend;
}

export function rgbCss(c: Rgb): string {
  return `rgb(${Math.round(c[0])}, ${Math.round(c[1])}, ${Math.round(c[2])})`;
}
