---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 066558bafcde
paths_covered:
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/views/LayoutEditor.tsx"
  - ":(top)apps/viewer/src/components/TacticalMap.tsx"
  - ":(top)apps/viewer/src/components/SquareInfo.tsx"
  - ":(top)apps/viewer/src/components/CacheCorner.tsx"
  - ":(top)apps/viewer/src/render/tacticalOverlay.ts"
  - ":(top)apps/viewer/src/imageLog.ts"
---

# Editor (`#/editor?layout=`)

Extraction mode. Component: `LayoutEditor` (`apps/viewer/src/views/LayoutEditor.tsx:28`) and inner `Editor` (`:63`). Does not wait for `/v1/world` (`apps/viewer/src/App.tsx:15`, `:93`).

## Mode & job

Operate. "Layout editor lite": loads every built-in layout's JSON, lets the user change squares' ground from a dropdown, and POSTs the edited layout to `/v1/tactical/render`, showing the returned image and its cache headers (`apps/viewer/src/views/LayoutEditor.tsx:24-27`, `:96-114`). The unedited layout renders once on load and on px/square change (`:116-124`).

Not on this screen: walls, placements, lights and elevation editing (only `ground` changes, via `withGround`, `:133-136`); saving to the server (render only); the tile pyramid (the result is one image).

## Composition

`.split.grow` row (`apps/viewer/src/views/LayoutEditor.tsx:139`):

1. Map column (focal moment): `TacticalMap` showing the rendered image (`source` kind `image`, `:141-156`); edited squares outlined orange (`apps/viewer/src/render/tacticalOverlay.ts:79-81`); hover/pin marks as on the Tactical screen. `SquareReadout` with hint "click a square to edit its ground"; `CacheCorner` labelled "POST /render" (`apps/viewer/src/views/LayoutEditor.tsx:157-158`).
2. Map tools bar (`:159-211`): "layout" select, "px/square" select, "5-ft grid" checkbox, render status text, render error.
3. Inspector "Edit {name}" (`:213-286`): when a square is picked, "Square A1 (x, y)", a "ground" select and "Set ground" button (disabled while unchanged); otherwise the hint. Then a toolbar with the page's only `button.primary`, "POST /render" (disabled while busy), and "Reset edits" (disabled with no edits). Then "Edits (N)" listing `A1 (x, y): from → to`, `SquareInfo`, a note on where ground keys come from, and a collapsed "Layout JSON (N KiB)".

Picking a layout sets `#/editor?layout=name` and remounts `Editor`, discarding edits (`:46-57`). Clicking a square pins it and preloads its current ground into the select (`:150-154`).

Keyboard: all controls are native and tab-reachable; square picking is pointer-only.

Responsive: no breakpoints; tools bar wraps; inspector caps at `45vw`.

## States

| State | Trigger | Rendered | Site |
|---|---|---|---|
| Loading | listing, layouts, library pending | notice "Loading the built-in layouts…" | `apps/viewer/src/views/LayoutEditor.tsx:40` |
| Error | listing or a layout failed | `.notice.error` | `:41` |
| Empty | no layouts | notice "The server lists no tactical layouts to edit." | `:44` |
| Library missing | `/v1/tactical/library` failed | ground list falls back to keys used by loaded layouts; no message | `:32-35`, `:52` |
| Rendering | render pending | muted "rendering…"; primary disabled; map keeps previous image until replaced | `:202`, `:247` |
| Rendered | image arrived | "rendered N edit(s) · status · K KiB · M ms" | `:203-204` |
| Render error | POST failed (e.g. 422 `invalid_layout`) | `.error` with message, `data-testid="render-error"` | `:206-210`, `:279-280` |
| Nothing picked | no pinned square | inspector "Click a square on the map to change its ground." | `:241` |
| No edits | layout equals original | "none yet"; "Reset edits" disabled | `:257`, `:266-267` |

## Motion

Leaflet default pan/zoom animation only.

## Copy

The primary button is labelled with the HTTP call, "POST /render" (`apps/viewer/src/views/LayoutEditor.tsx:253`). Other controls: "Set ground", "Reset edits", "layout", "px/square", "5-ft grid", "ground" (`:161-238`, `:262`). The explanatory note states "the server has no library listing route yet. An unknown key answers 422 invalid_layout." (`:278-281`).

## Not in play

- P3 grid: flexible map plus fixed inspector; no column grid.
- P5 offline / permission-denied: no distinct rendering; network failure shows as the render or load error text; the API has no auth.
- P9 touch targets: no touch sizing is defined.
- Destructive confirmation: "Reset edits" and switching layouts discard edits without confirmation (`apps/viewer/src/views/LayoutEditor.tsx:258-260`, `:53-56`).
