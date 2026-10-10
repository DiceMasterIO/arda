---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 71a844087604
paths_covered:
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/views/AreaView.tsx"
  - ":(top)apps/viewer/src/views/CellInspector.tsx"
  - ":(top)apps/viewer/src/render/heatmap.ts"
---

# Area (`#/area?ax=&ay=`)

Extraction mode. Component: `AreaView` (`apps/viewer/src/views/AreaView.tsx:28`), mounted after `/v1/world` loads (`apps/viewer/src/App.tsx:90`). Initial area comes from `ax`/`ay` in the hash, clamped to the world, defaulting to 0,0 (`apps/viewer/src/views/AreaView.tsx:19-26`).

## Mode & job

Operate. Draws one area as a canvas heatmap of a chosen field (Height, Cover, Soil moisture, Forest density, Temperature, Slope; `apps/viewer/src/render/heatmap.ts:8-15`) from the binary `ARDACOLS` columns, with rivers and lakes on an overlay canvas (`apps/viewer/src/views/AreaView.tsx:39-49`, `:72-104`). Hover reads values; click opens the cell inspector.

Not on this screen: pan/zoom (the stage is a fixed square); the world map (World tab); tactical maps (reached via the inspector link, `apps/viewer/src/views/CellInspector.tsx:31-33`).

## Composition

`.split` row with `.area-main` scrolling column (`apps/viewer/src/views/AreaView.tsx:129-130`, `apps/viewer/src/styles.css:85`):

1. Toolbar (`apps/viewer/src/views/AreaView.tsx:131-209`): an area-picker grid of `x,y` buttons (one per area, active one accent-filled, `apps/viewer/src/styles.css:87-89`); an "Area" select duplicating it; a "Field" select; "rivers" and "lakes" checkboxes; a muted count "N river segments · M lakes".
2. Stage (focal moment): two stacked canvases in a square `.area-stage` sized `min(100%, 720px, calc(100vh - 215px))` (`apps/viewer/src/views/AreaView.tsx:212-233`, `apps/viewer/src/styles.css:90-92`). The base canvas is pixelated; the overlay renders at 3x per cell (`OVERLAY_SCALE`, `apps/viewer/src/views/AreaView.tsx:17`) with a crosshair cursor.
3. Static readout below the stage (`:234-242`).
4. Legend (`:243`, `:257-296`): ramp fields show min, gradient bar, max with unit; Cover shows category swatches; both add "sea" (and ramps "lake") swatches.
5. Inspector, right, after a click succeeds (`:245-252`): the shared `CellInspector`.

Changing area replaces the hash with `#/area?ax&ay` and closes the inspector (`:51-55`).

Keyboard: area buttons, selects and checkboxes are native and tab-reachable in DOM order; cell hover and inspect are pointer-only (canvas mouse events, `:218-231`).

Responsive: the stage shrinks with viewport width and height, floor 256px (`apps/viewer/src/styles.css:90`); toolbar wraps (`:86`).

## States

| State | Trigger | Rendered | Site |
|---|---|---|---|
| World loading / error | shell | as World screen | `apps/viewer/src/App.tsx:81-88` |
| Area loading | cells, rivers, lakes pending | notice "Loading area ax,ay (binary columns, rivers, lakes)…"; stage hidden | `apps/viewer/src/views/AreaView.tsx:210`, `:212` |
| Area error | any of the three fetches failed | `.notice.error` with `describeError` | `:211` |
| Loaded, idle | no hover | readout "hover for values · click a cell to inspect it" | `:240` |
| Hover | pointer on canvas | "local cx, cy · global gx, gy · {Field value unit (terrain)}"; non-finite values read "n/a" | `:113-124`, `:235-238` |
| Inspect | click, `/v1/cell` ok | `CellInspector` | `:245-252` |
| Inspect failed | click, `/v1/cell` failed | browser `window.alert` with `describeError` | `:228-230` |

Sea cells always paint `rgb(38,78,128)` and lake cells `rgb(64,120,176)` (lakes not in Cover mode) (`apps/viewer/src/render/heatmap.ts:29-30`, `:111-113`).

## Motion

None.

## Copy

Field labels from `FIELDS` with units "m", "0–1", "°C", "°" (`apps/viewer/src/render/heatmap.ts:8-15`). Toolbar labels "Area", "Field", "rivers", "lakes" (`apps/viewer/src/views/AreaView.tsx:149-202`). Legend labels "sea", "lake" (`:269`, `:288-292`).

## Not in play

- P5 empty: every area of a loaded world has cells.
- P5 permission-denied: the API has no auth.
- P6 motion: nothing animates.
- P9 touch targets: area-picker buttons are `0.7rem` text with `0.15rem` vertical padding; no touch sizing is defined (`apps/viewer/src/styles.css:88`).
