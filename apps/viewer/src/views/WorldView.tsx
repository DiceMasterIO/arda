import L from "leaflet";
import { useEffect, useRef, useState } from "react";
import type { CellSample, WorldInfo } from "@arda";
import { describeError, type ArdaClient } from "../api/client.ts";
import { areasHigh, areasWide, basePxToCell, cellBoundsBasePx, cellToArea, type CellRef } from "../geo/coords.ts";
import { fitOnResize } from "../geo/fitOnResize.ts";
import { pyramidCrs, unitsToLatLng } from "../geo/leafletCrs.ts";
import { deepestZoom, hasRelief, metresPerPx, offersTactical } from "../geo/relief.ts";
import { cellHash } from "../geo/cellWalk.ts";
import { CellInspector } from "./CellInspector.tsx";

type Selected = { cell: CellRef; seq: number } & ({ state: "loading" } | { state: "ok"; sample: CellSample } | { state: "error"; message: string });

export function WorldView({ client, world }: { client: ArdaClient; world: WorldInfo }) {
  const mapEl = useRef<HTMLDivElement>(null);
  const [hover, setHover] = useState<CellRef | null>(null);
  const [selected, setSelected] = useState<Selected | null>(null);
  const [showAreas, setShowAreas] = useState(true);
  // Opt-in looks, off by default: the oblique overview (goal 24) and the
  // world grade carried into the tactical map (goal 49).
  const [oblique, setOblique] = useState(false);
  const [grade, setGrade] = useState(false);
  const overviewRef = useRef<L.TileLayer | null>(null);
  const reliefRef = useRef<L.TileLayer | null>(null);
  const obliqueShownRef = useRef(false);
  const [zoom, setZoom] = useState(0);
  const [centre, setCentre] = useState<CellRef | null>(null);
  const mapRef = useRef<L.Map | null>(null);
  const areasRef = useRef<L.LayerGroup | null>(null);
  const markRef = useRef<L.Rectangle | null>(null);

  // Build the map once per client/world.
  useEffect(() => {
    const el = mapEl.current;
    if (!el) return;
    const { tiles } = world;
    const crs = pyramidCrs(tiles.max_zoom);
    const bounds = L.latLngBounds(unitsToLatLng(0, 0), unitsToLatLng(tiles.image_width_px, tiles.image_height_px));
    const deepest = deepestZoom(world);
    const map = L.map(el, {
      crs,
      minZoom: 0,
      maxZoom: deepest,
      zoomSnap: 0.25,
      attributionControl: false,
      maxBounds: bounds.pad(0.25),
    });
    // The overview pyramid, upscaled past native zoom as a backdrop while
    // relief tiles load.
    obliqueShownRef.current = false;
    overviewRef.current = L.tileLayer(client.overviewTileTemplate(), {
      tileSize: tiles.tile_px,
      minZoom: 0,
      maxNativeZoom: tiles.max_zoom,
      maxZoom: deepest,
      noWrap: true,
      bounds,
    }).addTo(map);
    // Past native zoom: on-demand ~10 m relief (GET /v1/tiles/relief).
    if (hasRelief(world)) {
      reliefRef.current = L.tileLayer(client.reliefTileTemplate(), {
        tileSize: tiles.tile_px,
        minZoom: tiles.max_zoom + 1,
        maxNativeZoom: tiles.relief_max_zoom,
        maxZoom: tiles.relief_max_zoom,
        noWrap: true,
        bounds,
      }).addTo(map);
    }
    map.fitBounds(bounds);
    const stopResize = fitOnResize(map, el, bounds);

    // Area boundaries with labels.
    const areas = L.layerGroup();
    const aw = areasWide(world);
    const ah = areasHigh(world);
    for (let ay = 0; ay < ah; ay++) {
      for (let ax = 0; ax < aw; ax++) {
        const [x0, y0] = cellBoundsBasePx(world, ax * world.area_cells, ay * world.area_cells);
        const [, , x1, y1] = cellBoundsBasePx(
          world,
          Math.min(world.cells_wide, (ax + 1) * world.area_cells) - 1,
          Math.min(world.cells_high, (ay + 1) * world.area_cells) - 1,
        );
        L.rectangle(L.latLngBounds(unitsToLatLng(x0, y0), unitsToLatLng(x1, y1)), {
          color: "#ffffff",
          weight: 1,
          opacity: 0.45,
          fill: false,
          dashArray: "4 4",
          interactive: false,
        })
          .bindTooltip(`area ${ax},${ay}`, { permanent: true, direction: "center", className: "area-label" })
          .addTo(areas);
      }
    }
    areasRef.current = areas;
    areas.addTo(map);

    const toCell = (ll: L.LatLng) => basePxToCell(world, ll.lng, ll.lat);
    const track = () => {
      setZoom(map.getZoom());
      setCentre(toCell(map.getCenter()));
    };
    map.on("zoomend moveend", track);
    track();
    map.on("mousemove", (e: L.LeafletMouseEvent) => {
      setHover(toCell(e.latlng));
    });
    map.on("mouseout", () => {
      setHover(null);
    });
    map.on("click", (e: L.LeafletMouseEvent) => {
      const cell = toCell(e.latlng);
      if (!cell) return;
      setSelected((prev) =>
        prev && prev.state !== "error" && prev.cell.gx === cell.gx && prev.cell.gy === cell.gy
          ? prev
          : { cell, seq: (prev?.seq ?? 0) + 1, state: "loading" },
      );
    });
    mapRef.current = map;
    return () => {
      stopResize();
      map.remove();
      mapRef.current = null;
      areasRef.current = null;
      markRef.current = null;
      overviewRef.current = null;
      reliefRef.current = null;
    };
  }, [client, world]);

  // The oblique pyramid (goal 24) replaces the overview tiles. Relief
  // levels are drawn top-down, so they are hidden while it is on: ground
  // would jump north-south by up to tilt × height at the switch.
  useEffect(() => {
    const map = mapRef.current;
    const old = overviewRef.current;
    if (map && old && obliqueShownRef.current !== oblique) {
      // A fresh layer with the same geometry (setUrl redraws at the
      // fractional view zoom instead of the tile zoom).
      const next = L.tileLayer(client.overviewTileTemplate("webp", { oblique }), old.options).addTo(map);
      next.bringToBack();
      old.remove();
      overviewRef.current = next;
      obliqueShownRef.current = oblique;
    }
    const relief = reliefRef.current;
    if (!map || !relief) return;
    if (oblique) relief.remove();
    else relief.addTo(map);
  }, [client, world, oblique]);

  useEffect(() => {
    const map = mapRef.current;
    const areas = areasRef.current;
    if (!map || !areas) return;
    if (showAreas) areas.addTo(map);
    else areas.remove();
  }, [showAreas]);

  // Fetch the clicked cell.
  const selGx = selected?.cell.gx;
  const selGy = selected?.cell.gy;
  const selSeq = selected?.seq;
  useEffect(() => {
    if (selGx === undefined || selGy === undefined) return;
    const ctl = new AbortController();
    const cell = { gx: selGx, gy: selGy };
    const seq = selSeq ?? 0;
    client.cell(selGx, selGy, { signal: ctl.signal }).then(
      (sample) => {
        setSelected({ cell, seq, state: "ok", sample });
      },
      (e: unknown) => {
        if (!ctl.signal.aborted) setSelected({ cell, seq, state: "error", message: describeError(e) });
      },
    );
    return () => {
      ctl.abort();
    };
  }, [client, selGx, selGy, selSeq]);

  // Outline the selected cell.
  useEffect(() => {
    const map = mapRef.current;
    markRef.current?.remove();
    markRef.current = null;
    if (!map || selGx === undefined || selGy === undefined) return;
    const [x0, y0, x1, y1] = cellBoundsBasePx(world, selGx, selGy);
    markRef.current = L.rectangle(L.latLngBounds(unitsToLatLng(x0, y0), unitsToLatLng(x1, y1)), {
      color: "#ff3b30",
      weight: 2,
      fill: false,
      interactive: false,
    }).addTo(map);
  }, [world, selGx, selGy]);

  const hoverArea = hover ? cellToArea(world, hover.gx, hover.gy) : null;

  return (
    <div className="split">
      <div className="map-wrap">
        <div ref={mapEl} className="map" data-testid="world-map" />
        <div className="readout" data-testid="hover-readout">
          {hover && hoverArea ? (
            <>
              cell <b>{hover.gx}, {hover.gy}</b> · area {hoverArea.ax},{hoverArea.ay} · local {hoverArea.cx},{hoverArea.cy} ·{" "}
              {((hover.gx * world.cell_size_m) / 1000).toFixed(1)} km E, {((hover.gy * world.cell_size_m) / 1000).toFixed(1)} km S
            </>
          ) : (
            <span className="muted">hover the map for cell coordinates · click to inspect</span>
          )}
        </div>
        <div className="map-tools">
          <label>
            <input
              type="checkbox"
              checked={showAreas}
              onChange={(e) => {
                setShowAreas(e.target.checked);
              }}
            />{" "}
            area grid
          </label>
          <label title="?oblique=1 (goal 24, opt-in): the overview seen slightly obliquely; ground shifts north with its height, coasts stay put. Relief levels are hidden while it is on.">
            <input
              type="checkbox"
              aria-label="oblique"
              checked={oblique}
              onChange={(e) => {
                setOblique(e.target.checked);
              }}
            />{" "}
            oblique
          </label>
          <label title="Open tactical maps with ?world_grade=1 (goal 49, opt-in): ground and water pulled toward the world map's colours">
            <input
              type="checkbox"
              aria-label="world grade"
              checked={grade}
              onChange={(e) => {
                setGrade(e.target.checked);
              }}
            />{" "}
            world grade
          </label>
          <span className="muted" data-testid="scale-readout">
            {metresPerPx(world, zoom).toFixed(metresPerPx(world, zoom) < 10 ? 2 : 0)} m/px
            {zoom > world.tiles.max_zoom && hasRelief(world) && !oblique ? " · relief" : ""}
            {oblique ? " · oblique" : ""}
          </span>
          {offersTactical(world, zoom) && centre && (
            <a className="tab active" data-testid="open-tactical" href={cellHash(centre, false, grade)}>
              Open tactical map here
            </a>
          )}
          <span className="muted">
            seed {world.seed} · {world.size_km.width}×{world.size_km.height} km · {world.cells_wide}×{world.cells_high} cells
          </span>
        </div>
      </div>
      {selected && selected.state === "ok" && (
        <CellInspector
          cell={selected.sample}
          onClose={() => {
            setSelected(null);
          }}
        />
      )}
      {selected && selected.state === "loading" && (
        <aside className="inspector">
          <p>
            Loading cell {selected.cell.gx}, {selected.cell.gy}…
          </p>
        </aside>
      )}
      {selected && selected.state === "error" && (
        <aside className="inspector">
          <p className="error">{selected.message}</p>
        </aside>
      )}
    </div>
  );
}
