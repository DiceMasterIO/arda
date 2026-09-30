import L from "leaflet";

/**
 * A flat CRS whose map units are pyramid pixels at `maxZoom`, with y growing
 * south. At zoom z one unit is 2^(z - maxZoom) screen pixels, so Leaflet's
 * tile (z, x, y) lines up with the server's `{z}/{x}/{y}` pyramid.
 *
 * LatLng is (lat = y units, lng = x units).
 */
export function pyramidCrs(maxZoom: number): L.CRS {
  const s = 1 / 2 ** maxZoom;
  return L.Util.extend({}, L.CRS.Simple, {
    transformation: new L.Transformation(s, 0, s, 0),
  });
}

/** Leaflet LatLng for a (x, y) position in CRS units. */
export function unitsToLatLng(x: number, y: number): L.LatLng {
  return L.latLng(y, x);
}

/**
 * A flat CRS in tactical square units (one unit = one 5-ft square, y south)
 * for a tile pyramid: at zoom z one square is `ppsq * 2^(z - max_zoom)`
 * screen pixels, so Leaflet's `{z}/{x}/{y}` tiles are the server's.
 */
export function squareCrs(scale: number): L.CRS {
  return L.Util.extend({}, L.CRS.Simple, {
    transformation: new L.Transformation(scale, 0, scale, 0),
  });
}
