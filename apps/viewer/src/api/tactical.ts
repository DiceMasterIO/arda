// Tactical map helpers over the generated `@arda` bindings.
//
// Every wire type comes from ts-rs (bindings/ts/arda). The parsers here only
// check the shape a response must have before the views index into it; they
// never redefine a DTO. The rules sidecar is arda-scene's `RulesSidecar`
// format 2 (`RulesSidecarDto`, convention I9); `RulesSidecarView` is the
// viewer's validated reading of it, so the viewer's own demo blocks and the
// server's blocks go through the same checks.

import type {
  NotYetError,
  RulesSidecarDto,
  TacticalLibraryDto,
  SquareDto,
  TacticalBlockDto,
  TacticalLayoutDto,
  TacticalLayouts,
  TacticalLayoutSummary,
  TacticalTilesDto,
  WallSegmentDto,
} from "@arda";

export type { NotYetError, RulesSidecarDto, TacticalLibraryDto, SquareDto, TacticalBlockDto, TacticalLayoutDto, TacticalLayouts, TacticalLayoutSummary, TacticalTilesDto, WallSegmentDto };

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isUint(v: unknown): v is number {
  return typeof v === "number" && Number.isInteger(v) && v >= 0;
}

function fail(what: string, why: string): never {
  throw new Error(`${what}: ${why}`);
}

function checkTiles(what: string, t: unknown): TacticalTilesDto {
  if (!isRecord(t)) fail(what, "missing tiles");
  for (const k of ["tile_px", "ppsq", "image_width_px", "image_height_px", "max_zoom"] as const) {
    if (!isUint(t[k])) fail(what, `tiles.${k} is not a non-negative integer`);
  }
  if (typeof t["format"] !== "string") fail(what, "tiles.format is not a string");
  return t as TacticalTilesDto;
}

/** Checks a `/v1/tactical/library` body against `TacticalLibraryDto`. */
export function parseLibrary(body: unknown): TacticalLibraryDto {
  const what = "/v1/tactical/library";
  if (!isRecord(body)) fail(what, "body is not an object");
  if (typeof body["library_version"] !== "string") fail(what, "library_version is not a string");
  for (const k of ["grounds", "wall_kits", "assets"] as const) {
    if (!Array.isArray(body[k])) fail(what, `${k} is not a list`);
  }
  for (const [i, g] of (body["grounds"] as unknown[]).entries()) {
    if (!isRecord(g) || typeof g["key"] !== "string") fail(what, `grounds[${i}] has no key`);
  }
  return body as TacticalLibraryDto;
}

/** Checks a `/v1/tactical/layouts` body against `TacticalLayouts`. */
export function parseLayouts(body: unknown): TacticalLayouts {
  const what = "/v1/tactical/layouts";
  if (!isRecord(body)) fail(what, "body is not an object");
  if (typeof body["world_seed"] !== "string") fail(what, "world_seed is not a decimal string");
  if (typeof body["library_version"] !== "string") fail(what, "library_version is not a string");
  const opts: unknown = body["ppsq_options"];
  if (!Array.isArray(opts) || !opts.every(isUint)) fail(what, "ppsq_options is not a list of integers");
  if (!isUint(body["default_ppsq"])) fail(what, "default_ppsq is not an integer");
  const layouts: unknown = body["layouts"];
  if (!Array.isArray(layouts)) fail(what, "layouts is not a list");
  for (const [i, l] of (layouts as unknown[]).entries()) {
    if (!isRecord(l) || typeof l["name"] !== "string") fail(what, `layouts[${i}] has no name`);
    if (!isUint(l["width"]) || !isUint(l["height"])) fail(what, `layouts[${i}] has no integer size`);
    checkTiles(`${what} layouts[${i}]`, l["tiles"]);
  }
  return body as TacticalLayouts;
}

/** Checks a layout body against `TacticalLayoutDto`, including the square count. */
export function parseLayout(body: unknown, what = "layout"): TacticalLayoutDto {
  if (!isRecord(body)) fail(what, "body is not an object");
  if (typeof body["name"] !== "string") fail(what, "name is not a string");
  const { width, height } = body;
  if (!isUint(width) || !isUint(height)) fail(what, "width/height are not integers");
  const squares: unknown = body["squares"];
  if (!Array.isArray(squares)) fail(what, "squares is not a list");
  if (squares.length !== width * height) fail(what, `squares do not match ${width}x${height} (${squares.length} given)`);
  for (const [i, s] of (squares as unknown[]).entries()) {
    if (!isRecord(s) || typeof s["ground"] !== "string" || typeof s["elevation_ft"] !== "number" || typeof s["water_depth_ft"] !== "number") {
      fail(what, `squares[${i}] is not a SquareDto`);
    }
  }
  for (const k of ["walls", "placements", "lights"] as const) {
    if (!Array.isArray(body[k])) fail(what, `${k} is not a list`);
  }
  return body as TacticalLayoutDto;
}

/** Checks a `/v1/tactical/cell` 200 body against `TacticalBlockDto`. */
export function parseBlock(body: unknown): TacticalBlockDto {
  const what = "tactical block";
  if (!isRecord(body)) fail(what, "body is not an object");
  parseLayout(body["layout"], `${what} layout`);
  const origin: unknown = body["origin"];
  if (!Array.isArray(origin) || origin.length !== 2 || !origin.every((v) => Number.isInteger(v))) {
    fail(what, "origin is not an [x, y] integer pair");
  }
  if (!isRecord(body["meta"])) fail(what, "meta is not an object");
  if (!("rules" in body)) fail(what, "rules is missing (null is allowed)");
  if (!isUint(body["tactical_format"])) fail(what, "tactical_format is not an integer");
  if (typeof body["render_seed"] !== "string") fail(what, "render_seed is not a decimal string");
  checkTiles(what, body["tiles"]);
  return body as TacticalBlockDto;
}

/** The `NotYetError` inside a 501 body, or `null` when the body is not one. */
export function parseNotYet(body: unknown): NotYetError | null {
  if (!isRecord(body) || typeof body["planned_source"] !== "string") return null;
  const err = body["error"];
  if (!isRecord(err) || typeof err["code"] !== "string" || typeof err["message"] !== "string" || typeof err["status"] !== "number") return null;
  return body as NotYetError;
}

/** What `/v1/tactical/cell/{gx}/{gy}` answered. Anything else is an error. */
export type TacticalCellResult = { kind: "block"; block: TacticalBlockDto } | { kind: "not_yet"; notYet: NotYetError };

/**
 * Interprets a tactical cell response: 200 is a Block, 501 is the planned
 * "not yet" state (with or without a `NotYetError` body). Returns `null` for
 * any other status so the caller raises it as an ordinary API error.
 */
export function interpretCellResponse(status: number, body: unknown): TacticalCellResult | null {
  if (status === 200) return { kind: "block", block: parseBlock(body) };
  if (status === 501) {
    const notYet = parseNotYet(body) ?? {
      error: { code: "not_implemented", status: 501, message: "world-derived tactical blocks are not implemented yet" },
      planned_source: "unknown",
    };
    return { kind: "not_yet", notYet };
  }
  return null;
}

// --- Squares and walls -----------------------------------------------------

/** The square at `(sx, sy)`, or `null` outside the layout. */
export function squareAt(layout: Pick<TacticalLayoutDto, "width" | "height" | "squares">, sx: number, sy: number): SquareDto | null {
  if (!Number.isInteger(sx) || !Number.isInteger(sy)) return null;
  if (sx < 0 || sy < 0 || sx >= layout.width || sy >= layout.height) return null;
  return layout.squares[sy * layout.width + sx] ?? null;
}

export type Side = "N" | "E" | "S" | "W";

/**
 * Wall segments on the four edges of square `(sx, sy)`. A horizontal segment
 * at `(x, y)` is the north edge of that square, a vertical one its west edge
 * (arda_tactical `EdgeAxis`), so the south edge is horizontal `(x, y+1)` and
 * the east edge vertical `(x+1, y)`.
 */
export function wallsAround(layout: Pick<TacticalLayoutDto, "walls">, sx: number, sy: number): { side: Side; wall: WallSegmentDto }[] {
  const out: { side: Side; wall: WallSegmentDto }[] = [];
  const order: Record<Side, number> = { N: 0, E: 1, S: 2, W: 3 };
  for (const w of layout.walls) {
    let side: Side | null = null;
    if (w.axis === "horizontal" && w.x === sx && w.y === sy) side = "N";
    else if (w.axis === "horizontal" && w.x === sx && w.y === sy + 1) side = "S";
    else if (w.axis === "vertical" && w.x === sx && w.y === sy) side = "W";
    else if (w.axis === "vertical" && w.x === sx + 1 && w.y === sy) side = "E";
    if (side) out.push({ side, wall: w });
  }
  return out.sort((a, b) => order[a.side] - order[b.side]);
}

/** Edge line of a wall segment in square units: [x1, y1, x2, y2]. */
export function wallLine(w: Pick<WallSegmentDto, "x" | "y" | "axis">): [number, number, number, number] {
  return w.axis === "horizontal" ? [w.x, w.y, w.x + 1, w.y] : [w.x, w.y, w.x, w.y + 1];
}

/** Sorted, de-duplicated ground keys used by the given layouts. */
export function groundKeys(layouts: readonly Pick<TacticalLayoutDto, "squares">[]): string[] {
  const set = new Set<string>();
  for (const l of layouts) for (const s of l.squares) set.add(s.ground);
  return [...set].sort();
}

/** A copy of `layout` with the ground of square `(sx, sy)` replaced. */
export function withGround(layout: TacticalLayoutDto, sx: number, sy: number, ground: string): TacticalLayoutDto {
  if (!squareAt(layout, sx, sy)) throw new Error(`square ${sx},${sy} is outside ${layout.width}x${layout.height}`);
  const i = sy * layout.width + sx;
  return { ...layout, squares: layout.squares.map((s, j) => (j === i ? { ...s, ground } : s)) };
}

// --- Rules sidecar ---------------------------------------------------------

export type CoverLevel = "none" | "half" | "three_quarters" | "total";

/** One square of the arda-scene rules sidecar; an absent field means "no opinion". */
export interface RulesCellView {
  difficult?: boolean;
  water_depth_ft?: number;
  cover?: CoverLevel;
  blocks_sight?: boolean;
  lightly_obscured?: boolean;
  blocks_movement?: boolean;
  deck?: boolean;
  /** Layer extras (`feature`, `road_class`, `crop`, `building`, …). */
  ext?: Record<string, unknown>;
}

export interface RulesSidecarView {
  format_version: number;
  width: number;
  height: number;
  squares: RulesCellView[];
}

const COVERS: readonly string[] = ["none", "half", "three_quarters", "total", "full"];

/**
 * Validates the block's `rules` against the layout size. Returns `null` for a
 * `null` sidecar and throws on a malformed one.
 */
export function parseRules(rules: unknown, width: number, height: number): RulesSidecarView | null {
  if (rules === null || rules === undefined) return null;
  const what = "rules sidecar";
  if (!isRecord(rules)) fail(what, "not an object");
  const version = rules["format_version"];
  if (!isUint(version)) fail(what, "format_version is not an integer");
  if (rules["width"] !== width || rules["height"] !== height) fail(what, `size is not the layout's ${width}x${height}`);
  const squares: unknown = rules["squares"];
  if (!Array.isArray(squares) || squares.length !== width * height) fail(what, `squares do not match ${width}x${height}`);
  const out: RulesCellView[] = (squares as unknown[]).map((s, i) => {
    if (!isRecord(s)) fail(what, `squares[${i}] is not an object`);
    const cell: RulesCellView = {};
    for (const k of ["difficult", "blocks_sight", "lightly_obscured", "blocks_movement", "deck"] as const) {
      const v = s[k];
      if (typeof v === "boolean") cell[k] = v;
      else if (v !== undefined && v !== null) fail(what, `squares[${i}].${k} is not a boolean`);
    }
    const d = s["water_depth_ft"];
    if (isUint(d)) cell.water_depth_ft = d;
    else if (d !== undefined && d !== null) fail(what, `squares[${i}].water_depth_ft is not an integer`);
    const c = s["cover"];
    if (typeof c === "string" && COVERS.includes(c)) cell.cover = c === "full" ? "total" : (c as CoverLevel);
    else if (c !== undefined && c !== null) fail(what, `squares[${i}].cover is not a cover level`);
    const ext = s["ext"];
    if (isRecord(ext)) cell.ext = ext;
    else if (ext !== undefined && ext !== null) fail(what, `squares[${i}].ext is not an object`);
    return cell;
  });
  return { format_version: version, width, height, squares: out };
}

/**
 * A stand-in Block built from a built-in layout, so the Block path can be
 * exercised while the server still answers 501. Rules come from the layout:
 * water depth from the squares, mud and sand as difficult terrain, and half
 * cover on both sides of every wall edge. Views label it as a demo.
 */
export function demoBlock(layout: TacticalLayoutDto, gx: number, gy: number): TacticalBlockDto {
  const walled = new Set<number>();
  const mark = (x: number, y: number) => {
    if (x >= 0 && y >= 0 && x < layout.width && y < layout.height) walled.add(y * layout.width + x);
  };
  for (const w of layout.walls) {
    mark(w.x, w.y);
    if (w.axis === "horizontal") mark(w.x, w.y - 1);
    else mark(w.x - 1, w.y);
  }
  const squares: RulesCellView[] = layout.squares.map((s, i) => {
    const cell: RulesCellView = {};
    if (s.water_depth_ft > 0) cell.water_depth_ft = s.water_depth_ft;
    if (s.ground === "mud" || s.ground === "sand") cell.difficult = true;
    if (walled.has(i)) cell.cover = "half";
    return cell;
  });
  const rules: RulesSidecarDto = { format_version: 2, width: layout.width, height: layout.height, squares, edges: [] };
  return {
    tactical_format: 1,
    layout_schema: 1,
    origin: [64 * gx, 64 * gy],
    size: [layout.width, layout.height],
    render_seed: "0",
    layout,
    rules,
    meta: { source: "viewer-demo", from_layout: layout.name },
    scene: null,
    tiles: { tile_px: 512, ppsq: 64, image_width_px: layout.width * 64, image_height_px: layout.height * 64, max_zoom: 0, format: "webp" },
  };
}

/** One NPC token of a block's scene (`GET /v1/tactical/cell/{gx}/{gy}/scene`, A13). */
export interface TokenView {
  npc_id: string;
  name: string;
  x: number;
  y: number;
  building_id: string;
  settlement_id: string;
  kind: "worker" | "resident";
}

/** The `tokens` of a scene body; malformed entries are dropped. */
export function parseTokens(body: unknown): TokenView[] {
  const tokens = (body as { tokens?: unknown } | null)?.tokens;
  if (!Array.isArray(tokens)) return [];
  const out: TokenView[] = [];
  for (const t of tokens as Record<string, unknown>[]) {
    const ok =
      typeof t.npc_id === "string" &&
      typeof t.name === "string" &&
      Number.isInteger(t.x) &&
      Number.isInteger(t.y) &&
      typeof t.building_id === "string" &&
      typeof t.settlement_id === "string" &&
      (t.kind === "worker" || t.kind === "resident");
    if (ok) out.push(t as unknown as TokenView);
  }
  return out;
}
