---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 9f29be871f4b
paths_covered:
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/api/client.ts"
  - ":(top)apps/viewer/src/views/WorldView.tsx"
  - ":(top)apps/viewer/src/views/CellInspector.tsx"
  - ":(top)apps/viewer/src/geo/relief.ts"
  - ":(top)apps/viewer/src/geo/cellWalk.ts"
  - ":(top)apps/viewer/src/format.ts"
---

# World (`#/world`)

Extraction mode. Component: `WorldView` (`apps/viewer/src/views/WorldView.tsx:15`), mounted when `/v1/world` has loaded and the hash is `#/world` or unrecognised (`apps/viewer/src/App.tsx:17-20`, `:89`).

## Mode & job

Operate. A pan/zoom slippy map of the whole world: overview WebP tiles, then on-demand relief tiles past native zoom (`apps/viewer/src/views/WorldView.tsx:52-70`). Hover reads out cell coordinates; click inspects one cell's `CellSample`; at close zoom the screen offers the tactical map for the centre cell (`:253-257`, `apps/viewer/src/geo/relief.ts:16`, `:30-32`).

Not on this screen: per-area field heatmaps (Area tab, linked from the inspector, `apps/viewer/src/views/CellInspector.tsx:29`); tactical maps themselves (Cell tab, `apps/viewer/src/views/CellInspector.tsx:31-33`).

## Composition

`.split` row (`apps/viewer/src/views/WorldView.tsx:202`):

1. Map column (`.map-wrap`): the Leaflet map fills it on ground `#0e2438` (`:204`, `apps/viewer/src/styles.css:51`). Focal moment: the map. Area boundaries are dashed white rectangles with permanent centred labels "area ax,ay" (`apps/viewer/src/views/WorldView.tsx:75-99`). The selected cell gets a 2px `#ff3b30` outline (`:185-197`).
2. Hover readout HUD, bottom-left over the map (`:205-214`, `apps/viewer/src/styles.css:53-57`).
3. Map tools bar (`apps/viewer/src/views/WorldView.tsx:215-261`): checkboxes "area grid", "oblique", "world grade"; scale readout "N m/px" with " · relief" / " · oblique" suffixes; the "Open tactical map here" link when `offersTactical` (styled `tab active`); world summary "seed · W×H km · cells".
4. Inspector, right, only while a cell is selected (`:263-282`): `CellInspector` with close `×`, sections from `describeCell` (Location, Height, Terrain and cover, Climate, Rivers and lakes, Coast, Snow (proxy); `apps/viewer/src/format.ts:63-147`), links to the Area and Cell tabs, and a collapsed "Raw CellSample JSON" (`apps/viewer/src/views/CellInspector.tsx:4-41`).

Map behaviour: `zoomSnap: 0.25`, `maxBounds` padded 25%, max zoom `deepestZoom` (`apps/viewer/src/views/WorldView.tsx:41-48`, `apps/viewer/src/geo/relief.ts:35-37`). Turning "oblique" on swaps the overview layer and removes relief levels (`apps/viewer/src/views/WorldView.tsx:138-154`). Clicking the already-selected cell is a no-op unless it errored (`:117-121`).

Keyboard: Leaflet's built-in keyboard pan/zoom; checkboxes and links in DOM order. Cell selection is pointer-only (map `click`, `:114`). The inspector's close button is a native button (`apps/viewer/src/views/CellInspector.tsx:11-13`); no Escape handler and no focus move on open.

Responsive: no breakpoints; the tools bar wraps and the inspector caps at `45vw` (`apps/viewer/src/styles.css:60`, `:69`). Touch: no touch-specific handling beyond Leaflet's.

## States

| State | Trigger | Rendered | Site |
|---|---|---|---|
| World loading | `/v1/world` pending | notice "Loading /v1/world…" (shell) | `apps/viewer/src/App.tsx:81` |
| World error | `/v1/world` failed | `.notice.error`: "Could not load the world from {base}.", the `describeError` text, and the `arda-server --world out/micro42 --port 8787` hint | `apps/viewer/src/App.tsx:82-88` |
| Idle | no hover | readout "hover the map for cell coordinates · click to inspect" | `apps/viewer/src/views/WorldView.tsx:212` |
| Hover | pointer on map | readout "cell gx, gy · area ax,ay · local cx,cy · X km E, Y km S" | `:206-210` |
| Cell loading | click | inspector "Loading cell gx, gy…" | `:271-277` |
| Cell error | `/v1/cell` failed | inspector with `.error` text | `:278-282` |
| Cell loaded | sample arrived | `CellInspector` | `:263-270` |
| Tactical offered | zoom at or below 2 m/px | "Open tactical map here" link to `#/cell?gx&gy[&grade=1]` | `:253-257`; `apps/viewer/src/geo/cellWalk.ts:40-42` |

Tile load failures render nothing (Leaflet default empty tile). Offline and permission states have no distinct rendering; network failure surfaces through `describeError` as "cannot reach …" (`apps/viewer/src/api/client.ts:407`).

## Motion

Leaflet default pan/zoom animation only; no authored motion.

## Copy

Lowercase technical labels: "area grid", "oblique", "world grade", "m/px", "hover the map for cell coordinates · click to inspect" (`apps/viewer/src/views/WorldView.tsx:212-251`). Checkbox `title`s carry the query parameter and goal number, e.g. "?oblique=1 (goal 24, opt-in): …" (`:226`, `:237`). Inspector headings "Cell gx, gy", links "Open area ax,ay" and "Open tactical map here" (`apps/viewer/src/views/CellInspector.tsx:8-33`).

## Not in play

- P3 grid: full-bleed map with a fixed side panel; no column grid.
- P5 empty: a loaded world always has tiles; there is no empty-world state.
- P5 permission-denied: the API has no auth.
- P9 touch targets: no touch sizing is defined; Leaflet defaults apply.
