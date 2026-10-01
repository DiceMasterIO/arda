// Scene helpers over the generated `@arda` bindings (adapter A12).
//
// `GET /v1/tactical/cell/{gx}/{gy}/scene` answers a `TacticalScene`: the
// arda-scene `SceneDto` of the block plus its NPC tokens. Per-square layers
// are run-length grids (`Runs<T>`, `[count, value][]`), squares are `[x, y]`
// pairs and the seed is a decimal string. The checks here only confirm the
// shape the views index into; every type comes from ts-rs.

import type { CoverLevelDto, MovementDto, ObscurementDto, Runs, SceneDto, TacticalScene, Token } from "@arda";

export type { SceneDto, TacticalScene, Token };

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isUint(v: unknown): v is number {
  return typeof v === "number" && Number.isInteger(v) && v >= 0;
}

function fail(what: string, why: string): never {
  throw new Error(`${what}: ${why}`);
}

/** Expands a run-length grid to `len` row-major values; throws when the runs do not cover exactly `len` squares. */
export function expandRuns<T>(runs: Runs<T>, len: number, what = "grid"): T[] {
  const out: T[] = [];
  // Runs arrive from the wire, so check them although the type says pairs.
  for (const [i, run] of (runs as unknown[]).entries()) {
    if (!Array.isArray(run) || run.length !== 2 || !isUint(run[0])) fail(what, `run ${i} is not a [count, value] pair`);
    const [count, value] = run as [number, T];
    if (out.length + count > len) fail(what, `runs cover more than ${len} squares`);
    for (let k = 0; k < count; k++) out.push(value);
  }
  if (out.length !== len) fail(what, `runs cover ${out.length} of ${len} squares`);
  return out;
}

const GRIDS = ["movement", "climb", "cover", "obscured", "elevation_ft", "water_depth_ft"] as const;
const LISTS = ["walls", "vision_blockers", "lights", "regions", "tokens"] as const;

/** Checks a `SceneDto`: identity strings, sizes, and grids that cover the map. */
export function parseSceneDto(v: unknown, what = "scene"): SceneDto {
  if (!isRecord(v)) fail(what, "not an object");
  if (!isUint(v["format_version"])) fail(what, "format_version is not an integer");
  if (typeof v["seed"] !== "string") fail(what, "seed is not a decimal string");
  const { width, height } = v;
  if (!isUint(width) || !isUint(height)) fail(what, "width/height are not integers");
  for (const k of GRIDS) {
    const runs = v[k];
    if (!Array.isArray(runs)) fail(what, `${k} is not a run list`);
    expandRuns(runs as Runs<unknown>, width * height, `${what}.${k}`);
  }
  for (const k of LISTS) if (!Array.isArray(v[k])) fail(what, `${k} is not a list`);
  if (!isRecord(v["spawn_hints"])) fail(what, "spawn_hints is not an object");
  return v as SceneDto;
}

/** Checks a `TacticalScene` body (the scene route). */
export function parseScene(body: unknown): TacticalScene {
  const what = "tactical scene";
  if (!isRecord(body)) fail(what, "body is not an object");
  if (!isUint(body["tactical_format"])) fail(what, "tactical_format is not an integer");
  const origin = body["origin_gs"];
  if (!Array.isArray(origin) || origin.length !== 2 || !origin.every((n) => Number.isInteger(n))) fail(what, "origin_gs is not an [x, y] pair");
  if (body["time"] !== "day" && body["time"] !== "night") fail(what, "time is not day or night");
  parseSceneDto(body["scene"], `${what} scene`);
  const tokens = body["tokens"];
  if (!Array.isArray(tokens)) fail(what, "tokens is not a list");
  for (const [i, t] of (tokens as unknown[]).entries()) {
    const ok =
      isRecord(t) &&
      typeof t["npc_id"] === "string" &&
      typeof t["name"] === "string" &&
      isUint(t["x"]) &&
      isUint(t["y"]) &&
      typeof t["building_id"] === "string" &&
      typeof t["settlement_id"] === "string" &&
      (t["kind"] === "worker" || t["kind"] === "resident");
    if (!ok) fail(what, `tokens[${i}] is not a Token`);
  }
  return body as TacticalScene;
}

/** One square of a scene, read from its expanded grids. */
export interface SceneSquare {
  movement: MovementDto;
  cover: CoverLevelDto;
  obscured: ObscurementDto;
  /** Directions (0 N … 7 NW) whose step needs climbing. */
  climb: number[];
  elevation_ft: number;
  water_depth_ft: number;
}

/** The expanded per-square layers of a scene, for square lookups. */
export interface SceneGrids {
  width: number;
  height: number;
  movement: MovementDto[];
  cover: CoverLevelDto[];
  obscured: ObscurementDto[];
  climb: number[];
  elevation_ft: number[];
  water_depth_ft: number[];
}

/** Expands every per-square layer of `scene` once. */
export function sceneGrids(scene: SceneDto): SceneGrids {
  const n = scene.width * scene.height;
  return {
    width: scene.width,
    height: scene.height,
    movement: expandRuns(scene.movement, n, "movement"),
    cover: expandRuns(scene.cover, n, "cover"),
    obscured: expandRuns(scene.obscured, n, "obscured"),
    climb: expandRuns(scene.climb, n, "climb"),
    elevation_ft: expandRuns(scene.elevation_ft, n, "elevation_ft"),
    water_depth_ft: expandRuns(scene.water_depth_ft, n, "water_depth_ft"),
  };
}

/** The scene's view of square `(sx, sy)`, or `null` outside the map. */
export function sceneSquare(g: SceneGrids, sx: number, sy: number): SceneSquare | null {
  if (!Number.isInteger(sx) || !Number.isInteger(sy) || sx < 0 || sy < 0 || sx >= g.width || sy >= g.height) return null;
  const i = sy * g.width + sx;
  const mask = g.climb[i] ?? 0;
  return {
    movement: g.movement[i] ?? "impassable",
    cover: g.cover[i] ?? "none",
    obscured: g.obscured[i] ?? "clear",
    climb: [0, 1, 2, 3, 4, 5, 6, 7].filter((d) => (mask >> d) & 1),
    elevation_ft: g.elevation_ft[i] ?? 0,
    water_depth_ft: g.water_depth_ft[i] ?? 0,
  };
}
