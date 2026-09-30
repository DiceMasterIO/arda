import { describe, expect, it, vi } from "vitest";
import { CONTRACT_VERSION } from "@arda";
import { ApiRequestError, ArdaClient, ContractVersionError, DEFAULT_API_BASE, describeError, isNotAvailable, normaliseBase } from "./client.ts";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

function clientWith(handler: (url: string, init?: RequestInit) => Response | Promise<Response>) {
  const fetchFn = vi.fn((url: string, init?: RequestInit) => Promise.resolve(handler(url, init)));
  return { client: new ArdaClient("http://api.test:8787/", fetchFn), fetchFn };
}

describe("normaliseBase", () => {
  it("strips trailing slashes and whitespace", () => {
    expect(normaliseBase(" http://localhost:8790/// ")).toBe("http://localhost:8790");
  });
  it("falls back to the default when empty", () => {
    expect(normaliseBase("")).toBe(DEFAULT_API_BASE);
    expect(DEFAULT_API_BASE).toBe("http://localhost:8787");
  });
});

describe("ArdaClient URLs", () => {
  const c = new ArdaClient("http://h:1/");
  it("builds absolute paths", () => {
    expect(c.url("/v1/world")).toBe("http://h:1/v1/world");
    expect(c.url("v1/world")).toBe("http://h:1/v1/world");
  });
  it("builds the overview tile template and PNG URL", () => {
    expect(c.overviewTileTemplate()).toBe("http://h:1/v1/tiles/overview/{z}/{x}/{y}.png");
    expect(c.reliefTileTemplate()).toBe("http://h:1/v1/tiles/relief/{z}/{x}/{y}.webp");
    expect(c.overviewPngUrl()).toBe("http://h:1/v1/overview.png");
    expect(c.overviewPngUrl({ quality: "2K", style: "classic" })).toBe("http://h:1/v1/overview.png?quality=2K&style=classic");
  });
  it("builds tactical URLs with encoded names", () => {
    expect(c.tacticalPngUrl("river side", { ppsq: 50, grid: true })).toBe("http://h:1/v1/tactical/layout/river%20side.png?ppsq=50&grid=1");
    expect(c.tacticalTileTemplate("a/b")).toBe("http://h:1/v1/tactical/layout/a%2Fb/tiles/{z}/{x}/{y}.webp");
  });
});

describe("ArdaClient requests", () => {
  it("fetches a cell from the right path and checks the contract", async () => {
    const { client, fetchFn } = clientWith(() => jsonResponse({ contract_version: CONTRACT_VERSION, gx: 512, gy: 1036 }));
    const cell = await client.cell(512, 1036);
    expect(cell.gx).toBe(512);
    expect(fetchFn).toHaveBeenCalledWith("http://api.test:8787/v1/cell/512/1036", undefined);
  });

  it("encodes point queries", async () => {
    const { client, fetchFn } = clientWith(() => jsonResponse({ contract_version: CONTRACT_VERSION }));
    await client.point(51234.5, 103617.25);
    expect(fetchFn.mock.calls[0]?.[0]).toBe("http://api.test:8787/v1/point?x_m=51234.5&y_m=103617.25");
  });

  it("rejects an unknown contract version", async () => {
    const { client } = clientWith(() => jsonResponse({ contract_version: 99, status: "ok", seed: "1" }));
    await expect(client.health()).rejects.toBeInstanceOf(ContractVersionError);
  });

  it("surfaces the server's stable error code", async () => {
    const { client } = clientWith(() =>
      jsonResponse({ error: { code: "out_of_range", status: 400, message: "cell 5000,1 is outside the world" } }, 400),
    );
    const err = await client.cell(5000, 1).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiRequestError);
    const e = err as ApiRequestError;
    expect(e.status).toBe(400);
    expect(e.code).toBe("out_of_range");
    expect(e.notAvailable).toBe(false);
    expect(describeError(e)).toBe("400 out_of_range: cell 5000,1 is outside the world");
  });

  it("treats 404 and 501 as not available, even with non-JSON bodies", async () => {
    const { client: c404 } = clientWith(() => jsonResponse({ error: { code: "not_found", status: 404, message: "no route" } }, 404));
    const { client: c501 } = clientWith(() => new Response("Not Implemented", { status: 501 }));
    const e404 = await c404.tacticalLayouts().catch((e: unknown) => e);
    const e501 = await c501.tacticalLayouts().catch((e: unknown) => e);
    expect(isNotAvailable(e404)).toBe(true);
    expect(isNotAvailable(e501)).toBe(true);
    expect((e501 as ApiRequestError).code).toBe("not_implemented");
  });

  it("wraps transport failures as network errors", async () => {
    const client = new ArdaClient("http://down.test", () => Promise.reject(new TypeError("Failed to fetch")));
    const err = await client.world().catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiRequestError);
    expect((err as ApiRequestError).code).toBe("network");
    expect((err as ApiRequestError).status).toBe(0);
    expect(isNotAvailable(err)).toBe(false);
  });

  it("parses the tactical layout listing through the bindings", async () => {
    const tiles = { tile_px: 512, ppsq: 128, image_width_px: 3840, image_height_px: 2176, max_zoom: 3, format: "webp" };
    const body = { world_seed: "42", library_version: "0.1.0", ppsq_options: [64, 96, 128], default_ppsq: 128, layouts: [{ name: "riverside", width: 30, height: 17, tiles }] };
    const { client } = clientWith(() => jsonResponse(body));
    const got = await client.tacticalLayouts();
    expect(got.layouts[0]?.tiles.max_zoom).toBe(3);
    expect(got.world_seed).toBe("42");
  });

  it("validates a tactical layout's square count", async () => {
    const { client } = clientWith(() =>
      jsonResponse({ name: "x", width: 2, height: 2, squares: [{ ground: "grass", elevation_ft: 0, water_depth_ft: 0 }], walls: [], placements: [], lights: [] }),
    );
    await expect(client.tacticalLayout("x")).rejects.toThrow(/squares do not match/);
  });

  it("builds tactical tile and PNG URLs with options", () => {
    const c = new ArdaClient("http://h:1");
    expect(c.tacticalTileUrl("riverside", 3, 2, 1, { ppsq: 64 })).toBe("http://h:1/v1/tactical/layout/riverside/tiles/3/2/1.webp?ppsq=64");
    expect(c.tacticalTileTemplate("riverside", { ppsq: 96, grid: false })).toBe(
      "http://h:1/v1/tactical/layout/riverside/tiles/{z}/{x}/{y}.webp?ppsq=96&grid=0",
    );
  });

  it("keeps ETag and X-Arda-Cache from image responses", async () => {
    const { client } = clientWith(
      () => new Response(new Uint8Array([1, 2, 3]), { status: 200, headers: { etag: '"abc"', "x-arda-cache": "hit", "content-type": "image/png" } }),
    );
    const img = await client.tacticalPng("riverside", { ppsq: 64 });
    expect(img.etag).toBe('"abc"');
    expect(img.cache).toBe("hit");
    expect(img.bytes).toBe(3);
    expect(img.method).toBe("GET");
  });

  it("POSTs a layout to /render with the origin in the query", async () => {
    const { client, fetchFn } = clientWith(() => new Response(new Uint8Array([0]), { status: 200, headers: { "x-arda-cache": "miss" } }));
    const layout = { name: "x", width: 1, height: 1, squares: [{ ground: "grass", elevation_ft: 0, water_depth_ft: 0 }], walls: [], placements: [], lights: [] };
    const img = await client.renderLayout(layout, { ppsq: 64, origin: [32768, 66304] });
    const [url, init] = fetchFn.mock.calls[0] ?? [];
    expect(url).toBe("http://api.test:8787/v1/tactical/render?ppsq=64&origin=32768%2C66304");
    expect(init?.method).toBe("POST");
    expect(JSON.parse(init?.body as string)).toEqual(layout);
    expect(img.method).toBe("POST");
    expect(img.cache).toBe("miss");
  });

  it("surfaces 422 invalid_layout from /render", async () => {
    const { client } = clientWith(() => jsonResponse({ error: { code: "invalid_layout", status: 422, message: "ground lava has no texture" } }, 422));
    const layout = { name: "x", width: 1, height: 1, squares: [{ ground: "lava", elevation_ft: 0, water_depth_ft: 0 }], walls: [], placements: [], lights: [] };
    const err = await client.renderLayout(layout).catch((e: unknown) => e);
    expect((err as ApiRequestError).code).toBe("invalid_layout");
  });
});

describe("tacticalLibrary and cell tiles", () => {
  it("reads the library listing and builds cell tile URLs", async () => {
    const lib = { library: "placeholder", library_version: "1", pixels_per_square: 128, grounds: [{ key: "grass", variants: 2, water: false }], wall_kits: [], assets: [] };
    const { client, fetchFn } = clientWith(() => jsonResponse(lib));
    const got = await client.tacticalLibrary();
    expect(fetchFn.mock.calls[0]?.[0]).toBe("http://api.test:8787/v1/tactical/library");
    expect(got.grounds[0]?.key).toBe("grass");
  });
});

describe("tacticalCell", () => {
  it("returns the 501 not-yet state with the planned source instead of throwing", async () => {
    const body = {
      error: { code: "not_implemented", status: 501, message: "world-derived tactical blocks are not implemented yet; planned source: arda-refine" },
      planned_source: "arda-refine",
    };
    const { client, fetchFn } = clientWith(() => jsonResponse(body, 501));
    const got = await client.tacticalCell(512, 1036);
    expect(fetchFn.mock.calls[0]?.[0]).toBe("http://api.test:8787/v1/tactical/cell/512/1036");
    expect(got.kind).toBe("not_yet");
    if (got.kind === "not_yet") expect(got.notYet.planned_source).toBe("arda-refine");
  });

  it("treats a bare 501 as not yet, with an unknown source", async () => {
    const { client } = clientWith(() => new Response("Not Implemented", { status: 501 }));
    const got = await client.tacticalCell(0, 0);
    expect(got).toMatchObject({ kind: "not_yet", notYet: { planned_source: "unknown" } });
  });

  it("parses a 200 Block", async () => {
    const layout = { name: "b", width: 1, height: 1, squares: [{ ground: "grass", elevation_ft: 0, water_depth_ft: 0 }], walls: [], placements: [], lights: [] };
    const tiles = { tile_px: 512, ppsq: 64, image_width_px: 64, image_height_px: 64, max_zoom: 0, format: "webp" };
    const body = { tactical_format: 1, layout_schema: 1, layout, rules: null, origin: [64, 128], size: [1, 1], render_seed: "7", meta: { source: "arda-refine" }, scene: null, tiles };
    const { client, fetchFn } = clientWith(() => jsonResponse(body));
    const got = await client.tacticalCell(1, 2, undefined, { ppsq: 64, demo: true });
    expect(fetchFn.mock.calls[0]?.[0]).toBe("http://api.test:8787/v1/tactical/cell/1/2?ppsq=64&demo_overlays=1");
    expect(got.kind).toBe("block");
    if (got.kind === "block") expect(got.block.origin).toEqual([64, 128]);
  });

  it("still throws other errors such as 400 out_of_range", async () => {
    const { client } = clientWith(() => jsonResponse({ error: { code: "out_of_range", status: 400, message: "outside" } }, 400));
    const err = await client.tacticalCell(99999, 0).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiRequestError);
    expect((err as ApiRequestError).code).toBe("out_of_range");
  });
});
