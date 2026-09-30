## 2026-09-30 - feat: on-demand mid-zoom relief (arda-midzoom, /v1/tiles/relief)
key: feat/2026-09-30-midzoom-relief

- New crate `arda-midzoom` (logic/17): refines the stored 39.0625 m field to 9.765625 m on demand, for the tiles on screen only — sharpened spline base, drainage-aligned branching gullies and rock ribs scaled by roughness, slope, relief and cover, 39 m means restored, no new pits, stored rivers and lakes unchanged. Deterministic integer maths; tiles join pixel-exactly.
- `arda-render`: `AtlasTerrain::relief_colour`, `river_vertex_um` and `formed_river_rgb`; `arda::area_atlas_terrain`. Overview output is byte-identical.
- `arda-server`: `GET /v1/tiles/relief/{z}/{x}/{y}.webp` for eight levels past the overview's native zoom; `tiles.relief_max_zoom` in `/v1/world`.
- Viewer: World view switches to relief tiles past native zoom, shows m/px, and offers "Open tactical map here" at ≤ 2 m/px.
