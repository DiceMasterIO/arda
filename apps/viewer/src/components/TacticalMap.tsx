import L from "leaflet";
import { useEffect, useRef } from "react";
import type { ImageFetch, ImageMeta } from "../api/client.ts";
import { unitsToSquare } from "../geo/coords.ts";
import { fitOnResize } from "../geo/fitOnResize.ts";
import { squareCrs, unitsToLatLng } from "../geo/leafletCrs.ts";
import { squareCrsScale, type TileGeometry } from "../geo/tacticalTiles.ts";
import type { OverlayShape, SquareRef } from "../render/tacticalOverlay.ts";

export type { SquareRef } from "../render/tacticalOverlay.ts";

export type TileLoader = (z: number, x: number, y: number, signal: AbortSignal) => Promise<ImageFetch>;

export type MapSource =
  | { kind: "tiles"; key: string; load: TileLoader }
  | { kind: "image"; key: string; url: string }
  | { kind: "none" };

const SVG_NS = "http://www.w3.org/2000/svg";

/** Fixed stacking above the base image (overlayPane is 400). */
const PANES = [
  ["arda-grid", 410],
  ["arda-data", 420],
  ["arda-marks", 430],
] as const;

/**
 * A Leaflet tile layer that loads each tile with `fetch`, so the viewer can
 * read `ETag` and `X-Arda-Cache` per tile, and shows it from a blob URL.
 */
class FetchTileLayer extends L.TileLayer {
  private readonly loader: TileLoader;
  private readonly onMeta: (m: ImageMeta) => void;
  private readonly pending = new WeakMap<HTMLElement, AbortController>();

  constructor(loader: TileLoader, onMeta: (m: ImageMeta) => void, options: L.TileLayerOptions) {
    super("", options);
    this.loader = loader;
    this.onMeta = onMeta;
    this.on("tileunload", (e: L.TileEvent) => {
      this.pending.get(e.tile)?.abort();
      if (e.tile.src.startsWith("blob:")) URL.revokeObjectURL(e.tile.src);
    });
  }

  override createTile(coords: L.Coords, done: L.DoneCallback): HTMLElement {
    const img = document.createElement("img");
    img.alt = "";
    img.setAttribute("role", "presentation");
    const ctl = new AbortController();
    this.pending.set(img, ctl);
    this.loader(coords.z, coords.x, coords.y, ctl.signal).then(
      (f) => {
        this.onMeta(f);
        img.onload = () => {
          done(undefined, img);
        };
        img.onerror = () => {
          done(new Error("tile decode failed"), img);
        };
        img.src = URL.createObjectURL(f.blob);
      },
      (e: unknown) => {
        if (!ctl.signal.aborted) done(e instanceof Error ? e : new Error(String(e)), img);
      },
    );
    return img;
  }
}

function svgEl<K extends keyof SVGElementTagNameMap>(name: K, attrs: Record<string, string | number>): SVGElementTagNameMap[K] {
  const el = document.createElementNS(SVG_NS, name);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

function squareSvg(width: number, height: number): SVGSVGElement {
  const svg = svgEl("svg", { viewBox: `0 0 ${width} ${height}`, preserveAspectRatio: "none" });
  return svg;
}

function gridSvg(width: number, height: number): SVGSVGElement {
  const svg = squareSvg(width, height);
  let d = "";
  for (let x = 0; x <= width; x++) d += `M${x} 0V${height}`;
  for (let y = 0; y <= height; y++) d += `M0 ${y}H${width}`;
  svg.appendChild(svgEl("path", { d, stroke: "rgba(0,0,0,0.5)", "stroke-width": 1, "vector-effect": "non-scaling-stroke", fill: "none" }));
  return svg;
}

let patternSeq = 0;

function shapesSvg(width: number, height: number, shapes: readonly OverlayShape[]): SVGSVGElement {
  const svg = squareSvg(width, height);
  const id = `hatch-${++patternSeq}`;
  if (shapes.some((s) => s.kind === "rect" && s.hatch)) {
    const defs = svgEl("defs", {});
    const pat = svgEl("pattern", { id, patternUnits: "userSpaceOnUse", width: 0.25, height: 0.25, patternTransform: "rotate(45)" });
    pat.appendChild(svgEl("rect", { width: 0.25, height: 0.25, fill: "rgba(120, 72, 20, 0.18)" }));
    pat.appendChild(svgEl("line", { x1: 0, y1: 0, x2: 0, y2: 0.25, stroke: "rgba(92, 52, 8, 0.85)", "stroke-width": 0.06 }));
    defs.appendChild(pat);
    svg.appendChild(defs);
  }
  for (const s of shapes) {
    if (s.kind === "rect") {
      svg.appendChild(
        svgEl("rect", {
          x: s.x,
          y: s.y,
          width: s.w,
          height: s.h,
          fill: s.hatch ? `url(#${id})` : (s.fill ?? "none"),
          stroke: s.stroke ?? "none",
          "stroke-width": s.strokeWidth ?? 0,
          "vector-effect": "non-scaling-stroke",
        }),
      );
    } else {
      svg.appendChild(
        svgEl("line", { x1: s.x1, y1: s.y1, x2: s.x2, y2: s.y2, stroke: s.stroke, "stroke-width": s.strokeWidth, "stroke-linecap": "round", "vector-effect": "non-scaling-stroke" }),
      );
    }
  }
  return svg;
}

export interface TacticalMapProps {
  /** Layout size in squares. */
  width: number;
  height: number;
  /** Pyramid geometry; fixes the CRS so zoom z shows the server's tiles 1:1. */
  geometry: TileGeometry;
  source: MapSource;
  grid: boolean;
  /** Overlays under the pointer shapes (rules layers, edit marks). */
  overlay?: readonly OverlayShape[];
  /** Pointer-level marks: the pinned square and the hovered square's walls. */
  marks?: readonly OverlayShape[];
  onHover?: (sq: SquareRef | null) => void;
  onClick?: (sq: SquareRef | null) => void;
  onImageMeta?: (m: ImageMeta) => void;
  onSourceError?: (message: string) => void;
  testId?: string;
}

/** A Leaflet map in square units (CRS.Simple) showing tiles or one image. */
export function TacticalMap(props: TacticalMapProps) {
  const { width, height, geometry, source, grid, overlay, marks } = props;
  const el = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const cb = useRef(props);
  useEffect(() => {
    cb.current = props;
  });

  const scale = squareCrsScale(geometry);
  const maxZoom = geometry.max_zoom;

  // The map itself: rebuilt when the layout size or the pyramid changes.
  useEffect(() => {
    const node = el.current;
    if (!node) return;
    const bounds = L.latLngBounds(unitsToLatLng(0, 0), unitsToLatLng(width, height));
    const map = L.map(node, {
      crs: squareCrs(scale),
      minZoom: -1,
      maxZoom: maxZoom + 2,
      zoomSnap: 0.25,
      attributionControl: false,
      maxBounds: bounds.pad(0.5),
    });
    for (const [pane, z] of PANES) {
      const p = map.createPane(pane);
      p.style.zIndex = String(z);
      p.style.pointerEvents = "none";
    }
    map.fitBounds(bounds);
    const stopResize = fitOnResize(map, node, bounds);
    const toSquare = (ll: L.LatLng) => unitsToSquare(width, height, ll.lng, ll.lat);
    map.on("mousemove", (e: L.LeafletMouseEvent) => {
      cb.current.onHover?.(toSquare(e.latlng));
    });
    map.on("mouseout", () => {
      cb.current.onHover?.(null);
    });
    map.on("click", (e: L.LeafletMouseEvent) => {
      cb.current.onClick?.(toSquare(e.latlng));
    });
    mapRef.current = map;
    // Dev-only handle for poking at the map from the console or a test driver.
    if (import.meta.env.DEV) (node as HTMLDivElement & { ardaMap?: L.Map }).ardaMap = map;
    return () => {
      stopResize();
      map.remove();
      mapRef.current = null;
    };
  }, [width, height, scale, maxZoom]);

  // Base layer: the WebP pyramid, or one image.
  const sourceKey = source.kind === "none" ? "none" : `${source.kind}:${source.key}`;
  useEffect(() => {
    const map = mapRef.current;
    if (!map || source.kind === "none") return;
    const bounds = L.latLngBounds(unitsToLatLng(0, 0), unitsToLatLng(width, height));
    let layer: L.Layer;
    if (source.kind === "tiles") {
      let failed = false;
      const tiles = new FetchTileLayer(
        source.load,
        (m) => {
          cb.current.onImageMeta?.(m);
        },
        {
          tileSize: geometry.tile_px,
          minZoom: -1,
          minNativeZoom: 0,
          maxNativeZoom: geometry.max_zoom,
          maxZoom: geometry.max_zoom + 2,
          noWrap: true,
          bounds,
        },
      );
      tiles.on("tileerror", (e: L.TileErrorEvent) => {
        if (failed) return;
        failed = true;
        cb.current.onSourceError?.(`tile ${e.coords.z}/${e.coords.x}/${e.coords.y} failed: ${e.error.message}`);
      });
      layer = tiles;
    } else {
      layer = L.imageOverlay(source.url, bounds, { interactive: false }).on("error", () => {
        cb.current.onSourceError?.("image failed to load");
      });
    }
    layer.addTo(map);
    return () => {
      layer.remove();
    };
    // sourceKey stands in for `source`, whose identity changes every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceKey, width, height, geometry.tile_px, geometry.max_zoom, scale, maxZoom]);

  // Browser-side 5-ft grid.
  useEffect(() => {
    const map = mapRef.current;
    if (!map || !grid) return;
    const layer = L.svgOverlay(gridSvg(width, height), L.latLngBounds(unitsToLatLng(0, 0), unitsToLatLng(width, height)), {
      interactive: false,
      pane: "arda-grid",
    });
    layer.addTo(map);
    return () => {
      layer.remove();
    };
  }, [grid, width, height, scale, maxZoom]);

  // Data overlays, then pointer marks on top.
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;
    const bounds = L.latLngBounds(unitsToLatLng(0, 0), unitsToLatLng(width, height));
    const layers: L.Layer[] = [];
    for (const [shapes, pane] of [
      [overlay, "arda-data"],
      [marks, "arda-marks"],
    ] as const) {
      if (shapes && shapes.length > 0) layers.push(L.svgOverlay(shapesSvg(width, height, shapes), bounds, { interactive: false, pane }).addTo(map));
    }
    return () => {
      for (const l of layers) l.remove();
    };
  }, [overlay, marks, width, height, scale, maxZoom]);

  return <div ref={el} className="map tactical-map" data-testid={props.testId ?? "tactical-map"} />;
}
