// A small typed client for the arda-server v1 HTTP API.
//
// This is the reference for how a game should consume Arda: every response
// type comes from the generated ts-rs bindings (`@arda`), cell-bearing bodies
// are checked against CONTRACT_VERSION, and errors are surfaced by their stable
// `code`, never by the human message.

import {
  CONTRACT_VERSION,
  type ApiError,
  type AreaCells,
  type AreaLakes,
  type AreaRivers,
  type CellSample,
  type Health,
  type PointSample,
  type PrefetchAccepted,
  type PrefetchRequest,
  type WorldInfo,
} from "@arda";
import { parseAreaColumnsBin, type BinAreaColumns } from "./areaBin.ts";
import {
  interpretCellResponse,
  parseLayout,
  parseLibrary,
  parseLayouts,
  parseTokens,
  type TacticalCellResult,
  type TacticalLayoutDto,
  type TacticalLayouts,
  type TacticalLibraryDto,
  type TokenView,
} from "./tactical.ts";

/** Options of world-derived tactical cells: `?ppsq`, `?demo_overlays=1`, `?world_grade=1`. */
export interface CellOptions {
  ppsq?: number;
  /** Compose the synthetic ways, fields and town samples anchored at this cell. */
  demo?: boolean;
  /** Opt-in world grade (goal 49): ground and water pulled toward the world map's colours. */
  worldGrade?: boolean;
}

function cellQuery(opts: CellOptions): string {
  const q = new URLSearchParams();
  if (opts.ppsq !== undefined) q.set("ppsq", String(opts.ppsq));
  if (opts.demo) q.set("demo_overlays", "1");
  if (opts.worldGrade) q.set("world_grade", "1");
  const qs = q.toString();
  return qs ? `?${qs}` : "";
}

export const DEFAULT_API_BASE = "http://localhost:8787";

/** An HTTP or transport failure, carrying the server's stable error code when it sent one. */
export class ApiRequestError extends Error {
  readonly status: number;
  readonly code: string;
  readonly url: string;
  /** The parsed JSON error body, when there was one. */
  readonly body: unknown;

  constructor(url: string, status: number, code: string, message: string, body: unknown = null) {
    super(message);
    this.name = "ApiRequestError";
    this.url = url;
    this.status = status;
    this.code = code;
    this.body = body;
  }

  /**
   * True when the server does not offer this route: 404 (unknown route, or a
   * layout the server doesn't have) or 501 (route exists but isn't implemented).
   */
  get notAvailable(): boolean {
    return this.status === 404 || this.status === 501;
  }
}

/** Thrown when a body carries a contract version this client doesn't understand. */
export class ContractVersionError extends Error {
  readonly got: unknown;
  constructor(url: string, got: unknown) {
    super(`${url}: contract_version ${String(got)} is not ${CONTRACT_VERSION}`);
    this.name = "ContractVersionError";
    this.got = got;
  }
}

/** Removes trailing slashes so paths can be appended as `${base}/v1/...`. */
export function normaliseBase(base: string): string {
  const trimmed = base.trim();
  return (trimmed === "" ? DEFAULT_API_BASE : trimmed).replace(/\/+$/, "");
}

function isApiError(v: unknown): v is ApiError {
  if (typeof v !== "object" || v === null) return false;
  const err = (v as { error?: unknown }).error;
  return typeof err === "object" && err !== null && typeof (err as { code?: unknown }).code === "string";
}

function checkContract<T>(url: string, body: T): T {
  const v = (body as { contract_version?: unknown }).contract_version;
  if (v !== CONTRACT_VERSION) throw new ContractVersionError(url, v);
  return body;
}

type FetchFn = (input: string, init?: RequestInit) => Promise<Response>;

/** Tactical render options shared by the PNG, tile and POST routes. */
export interface TacticalImageOptions {
  /** Output pixels per square: 64, 96 or 128. */
  ppsq?: number;
  /** Burn the server-side grid into the image. */
  grid?: boolean;
}

/** Response metadata of one image request, for the cache/debug readout. */
export interface ImageMeta {
  label: string;
  url: string;
  method: "GET" | "POST";
  status: number;
  /** Strong ETag, or null when the header is absent or not exposed. */
  etag: string | null;
  /** `X-Arda-Cache`: `hit` or `miss`. */
  cache: string | null;
  /** `Server-Timing`, when exposed (it isn't cross-origin by default). */
  serverTiming: string | null;
  bytes: number;
  /** Wall time in the browser, milliseconds. */
  ms: number;
}

export interface ImageFetch extends ImageMeta {
  blob: Blob;
}

function imageQuery(opts: TacticalImageOptions & { origin?: readonly [number, number] }): string {
  const q = new URLSearchParams();
  if (opts.ppsq !== undefined) q.set("ppsq", String(opts.ppsq));
  if (opts.grid !== undefined) q.set("grid", opts.grid ? "1" : "0");
  if (opts.origin !== undefined) q.set("origin", `${opts.origin[0]},${opts.origin[1]}`);
  const qs = q.toString();
  return qs ? `?${qs}` : "";
}

export class ArdaClient {
  readonly base: string;
  private readonly fetchFn: FetchFn;

  constructor(base: string = DEFAULT_API_BASE, fetchFn?: FetchFn) {
    this.base = normaliseBase(base);
    this.fetchFn = fetchFn ?? ((input, init) => fetch(input, init));
  }

  /** Absolute URL of an API path such as `/v1/world`. */
  url(path: string): string {
    return `${this.base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  /** Fetches without judging the status; only transport failures throw. */
  private async raw(path: string, init?: RequestInit): Promise<Response> {
    const url = this.url(path);
    try {
      return await this.fetchFn(url, init);
    } catch (e) {
      throw new ApiRequestError(url, 0, "network", e instanceof Error ? e.message : String(e));
    }
  }

  private async errorFrom(res: Response, url: string): Promise<ApiRequestError> {
    let code = res.status === 404 ? "not_found" : res.status === 501 ? "not_implemented" : "http_error";
    let message = `${res.status} ${res.statusText}`.trim();
    let body: unknown = null;
    try {
      body = await res.json();
      if (isApiError(body)) {
        code = body.error.code;
        message = body.error.message;
      }
    } catch {
      // Non-JSON error body: keep the status-derived code.
    }
    return new ApiRequestError(url, res.status, code, message, body);
  }

  private async request(path: string, init?: RequestInit): Promise<Response> {
    const res = await this.raw(path, init);
    if (res.ok) return res;
    throw await this.errorFrom(res, this.url(path));
  }

  /** Fetches an image and keeps its cache headers. */
  private async image(label: string, path: string, init?: RequestInit): Promise<ImageFetch> {
    const t0 = performance.now();
    const res = await this.request(path, init);
    const blob = await res.blob();
    return {
      label,
      url: this.url(path),
      method: init?.method === "POST" ? "POST" : "GET",
      status: res.status,
      etag: res.headers.get("etag"),
      cache: res.headers.get("x-arda-cache"),
      serverTiming: res.headers.get("server-timing"),
      bytes: blob.size,
      ms: performance.now() - t0,
      blob,
    };
  }

  private async json(path: string, init?: RequestInit): Promise<unknown> {
    const res = await this.request(path, init);
    return (await res.json()) as unknown;
  }

  private async contractJson<T>(path: string, init?: RequestInit): Promise<T> {
    return checkContract(this.url(path), (await this.json(path, init)) as T);
  }

  health(init?: RequestInit): Promise<Health> {
    return this.contractJson<Health>("/v1/health", init);
  }

  world(init?: RequestInit): Promise<WorldInfo> {
    return this.contractJson<WorldInfo>("/v1/world", init);
  }

  cell(gx: number, gy: number, init?: RequestInit): Promise<CellSample> {
    return this.contractJson<CellSample>(`/v1/cell/${gx}/${gy}`, init);
  }

  point(xM: number, yM: number, init?: RequestInit): Promise<PointSample> {
    const q = new URLSearchParams({ x_m: String(xM), y_m: String(yM) });
    return this.contractJson<PointSample>(`/v1/point?${q.toString()}`, init);
  }

  /** The ~38 MB JSON form. Prefer {@link areaCellsBin} in anything interactive. */
  areaCells(ax: number, ay: number, init?: RequestInit): Promise<AreaCells> {
    return this.contractJson<AreaCells>(`/v1/area/${ax}/${ay}/cells`, init);
  }

  /** The ~26 MB `ARDACOLS` binary form, parsed into typed-array views. */
  async areaCellsBin(ax: number, ay: number, init?: RequestInit): Promise<BinAreaColumns> {
    const path = `/v1/area/${ax}/${ay}/cells?format=bin`;
    const res = await this.request(path, init);
    const parsed = parseAreaColumnsBin(await res.arrayBuffer());
    return checkContract(this.url(path), parsed);
  }

  areaRivers(ax: number, ay: number, init?: RequestInit): Promise<AreaRivers> {
    return this.contractJson<AreaRivers>(`/v1/area/${ax}/${ay}/rivers`, init);
  }

  areaLakes(ax: number, ay: number, init?: RequestInit): Promise<AreaLakes> {
    return this.contractJson<AreaLakes>(`/v1/area/${ax}/${ay}/lakes`, init);
  }

  overviewPngUrl(opts: { quality?: number | string; style?: "atlas" | "classic" | "atlas-oblique" } = {}): string {
    const q = new URLSearchParams();
    if (opts.quality !== undefined) q.set("quality", String(opts.quality));
    if (opts.style !== undefined) q.set("style", opts.style);
    const qs = q.toString();
    return this.url(`/v1/overview.png${qs ? `?${qs}` : ""}`);
  }

  /**
   * Leaflet-style template for the overview pyramid: lossless WebP (goal 68); `.png` stays served for older clients.
   * `oblique` selects the opt-in oblique pyramid (goal 24).
   */
  overviewTileTemplate(format: "webp" | "png" = "webp", opts: { oblique?: boolean } = {}): string {
    return this.url(`/v1/tiles/overview/{z}/{x}/{y}.${format}${opts.oblique ? "?oblique=1" : ""}`);
  }

  /** Leaflet-style template for the mid-zoom relief levels past the overview. */
  reliefTileTemplate(): string {
    return this.url("/v1/tiles/relief/{z}/{x}/{y}.webp");
  }

  /** One mid-zoom relief tile. */
  reliefTileUrl(z: number, x: number, y: number): string {
    return this.url(`/v1/tiles/relief/${z}/${x}/${y}.webp`);
  }

  /** The PNG of a world cell's tactical map. */
  tacticalCellPngUrl(gx: number, gy: number, opts: CellOptions = {}): string {
    return this.url(`/v1/tactical/cell/${gx}/${gy}.png${cellQuery(opts)}`);
  }

  // --- Tactical maps (/v1/tactical) ---

  async tacticalLayouts(init?: RequestInit): Promise<TacticalLayouts> {
    return parseLayouts(await this.json("/v1/tactical/layouts", init));
  }

  async tacticalLayout(name: string, init?: RequestInit): Promise<TacticalLayoutDto> {
    return parseLayout(await this.json(`/v1/tactical/layout/${encodeURIComponent(name)}`, init), `layout ${name}`);
  }

  tacticalPngUrl(name: string, opts: TacticalImageOptions = {}): string {
    return this.url(`/v1/tactical/layout/${encodeURIComponent(name)}.png${imageQuery(opts)}`);
  }

  tacticalTileUrl(name: string, z: number, x: number, y: number, opts: TacticalImageOptions = {}): string {
    return this.url(`/v1/tactical/layout/${encodeURIComponent(name)}/tiles/${z}/${x}/${y}.webp${imageQuery(opts)}`);
  }

  /** Leaflet-style template; the query (ppsq, grid) is appended verbatim. */
  tacticalTileTemplate(name: string, opts: TacticalImageOptions = {}): string {
    return this.url(`/v1/tactical/layout/${encodeURIComponent(name)}/tiles/{z}/{x}/{y}.webp${imageQuery(opts)}`);
  }

  tacticalPng(name: string, opts: TacticalImageOptions = {}, init?: RequestInit): Promise<ImageFetch> {
    return this.image(`${name}.png`, `/v1/tactical/layout/${encodeURIComponent(name)}.png${imageQuery(opts)}`, init);
  }

  tacticalTile(name: string, z: number, x: number, y: number, opts: TacticalImageOptions = {}, init?: RequestInit): Promise<ImageFetch> {
    return this.image(
      `${name} tile ${z}/${x}/${y}`,
      `/v1/tactical/layout/${encodeURIComponent(name)}/tiles/${z}/${x}/${y}.webp${imageQuery(opts)}`,
      init,
    );
  }

  /** `GET /v1/tactical/library`: ground keys, wall kits and assets. */
  async tacticalLibrary(init?: RequestInit): Promise<TacticalLibraryDto> {
    return parseLibrary(await this.json("/v1/tactical/library", init));
  }

  /** A WebP tile of a world-derived cell (rendered with its apron, seamless). */
  tacticalCellTile(gx: number, gy: number, z: number, x: number, y: number, opts: CellOptions = {}, init?: RequestInit): Promise<ImageFetch> {
    return this.image(`cell ${gx},${gy} tile ${z}/${x}/${y}`, `/v1/tactical/cell/${gx}/${gy}/tiles/${z}/${x}/${y}.webp${cellQuery(opts)}`, init);
  }

  /**
   * `POST /v1/tactical/prefetch`: asks the server to warm the neighbours of a
   * cell in the background (goal 67). It answers 202 at once; nothing waits.
   */
  async tacticalPrefetch(gx: number, gy: number, opts: CellOptions & { radius?: number } = {}, init?: RequestInit): Promise<PrefetchAccepted> {
    const body: PrefetchRequest = { gx, gy };
    if (opts.radius !== undefined) body.radius = opts.radius;
    if (opts.ppsq !== undefined) body.ppsq = opts.ppsq;
    if (opts.demo) body.demo_overlays = true;
    const out = await this.json("/v1/tactical/prefetch", {
      ...init,
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (typeof out !== "object" || out === null || !Array.isArray((out as { cells?: unknown }).cells)) {
      throw new ApiRequestError(this.url("/v1/tactical/prefetch"), 202, "bad_body", "prefetch: the body has no cells");
    }
    return out as PrefetchAccepted;
  }

  /** `POST /v1/tactical/render`: renders a client-supplied layout to PNG. */
  renderLayout(
    layout: TacticalLayoutDto,
    opts: TacticalImageOptions & { origin?: readonly [number, number] } = {},
    init?: RequestInit,
  ): Promise<ImageFetch> {
    return this.image(`render ${layout.name}`, `/v1/tactical/render${imageQuery(opts)}`, {
      ...init,
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(layout),
    });
  }

  /**
   * `GET /v1/tactical/cell/{gx}/{gy}`: a Block, or the 501 "not yet" state,
   * which is a normal result here, not an error. Other failures throw.
   */
  /** `GET /v1/tactical/cell/{gx}/{gy}/scene`: the NPC tokens of the block (A13). */
  async tacticalTokens(gx: number, gy: number, init?: RequestInit, opts: CellOptions = {}): Promise<TokenView[]> {
    return parseTokens(await this.json(`/v1/tactical/cell/${gx}/${gy}/scene${cellQuery(opts)}`, init));
  }

  async tacticalCell(gx: number, gy: number, init?: RequestInit, opts: CellOptions = {}): Promise<TacticalCellResult> {
    const path = `/v1/tactical/cell/${gx}/${gy}${cellQuery(opts)}`;
    const res = await this.raw(path, init);
    if (res.status === 501) {
      let body: unknown = null;
      try {
        body = await res.json();
      } catch {
        // A bare 501 still means "not yet".
      }
      const out = interpretCellResponse(501, body);
      if (out) return out;
    }
    if (!res.ok) throw await this.errorFrom(res, this.url(path));
    const out = interpretCellResponse(res.status, await res.json());
    if (!out) throw new ApiRequestError(this.url(path), res.status, "http_error", `unexpected status ${res.status}`);
    return out;
  }
}

/** True for errors that mean "this server doesn't offer that". */
export function isNotAvailable(e: unknown): boolean {
  return e instanceof ApiRequestError && e.notAvailable;
}

export function describeError(e: unknown): string {
  if (e instanceof ApiRequestError) {
    if (e.code === "network") return `cannot reach ${e.url} (${e.message})`;
    return `${e.status} ${e.code}: ${e.message}`;
  }
  return e instanceof Error ? e.message : String(e);
}
