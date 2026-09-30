import type L from "leaflet";

/**
 * Keeps a Leaflet map sized to its container and fits `bounds` once the
 * container first has a real size (flex layouts settle after mount).
 */
export function fitOnResize(map: L.Map, el: HTMLElement, bounds: L.LatLngBounds): () => void {
  let fitted = false;
  const ro = new ResizeObserver(() => {
    map.invalidateSize();
    if (!fitted && el.clientHeight > 0 && el.clientWidth > 0) {
      fitted = true;
      map.fitBounds(bounds, { animate: false });
    }
  });
  ro.observe(el);
  return () => {
    ro.disconnect();
  };
}
