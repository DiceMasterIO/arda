---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: 850c34085104
paths_covered:
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/geo/coords.ts"
  - ":(top)apps/viewer/src/views/TacticalView.tsx"
  - ":(top)apps/viewer/src/components/TacticalMap.tsx"
  - ":(top)apps/viewer/src/components/SquareInfo.tsx"
  - ":(top)apps/viewer/src/components/CacheCorner.tsx"
  - ":(top)apps/viewer/src/render/tacticalOverlay.ts"
  - ":(top)apps/viewer/src/imageLog.ts"
---

# Tactical (`#/tactical`)

Extraction mode. Component: `TacticalView` (`apps/viewer/src/views/TacticalView.tsx:28`), with `LayoutViewer` (`:77`) and `LayoutMap` (`:118`). Does not wait for `/v1/world` (`apps/viewer/src/App.tsx:15`, `:91`).

## Mode & job

Operate. Lists the server's built-in tactical layouts and shows the chosen one as a Leaflet `CRS.Simple` map in square units, from its WebP tile pyramid or the whole-map PNG, with a browser-side 5-ft grid and a square inspector (`apps/viewer/src/views/TacticalView.tsx:138-153`, `apps/viewer/src/components/TacticalMap.tsx:148-275`). A debug corner shows the latest image's `ETag`, `X-Arda-Cache` and running hit/miss counts (`apps/viewer/src/components/CacheCorner.tsx:8-33`).

Not on this screen: editing (Editor tab, "Edit this layout" link, `apps/viewer/src/views/TacticalView.tsx:243-245`); world-derived blocks (Cell tab).

## Composition

`.split` row, three columns (`apps/viewer/src/views/TacticalView.tsx:44-73`, `:160-247`):

1. Layout list, left, `14rem` (`apps/viewer/src/styles.css:97-104`): heading "Layouts", one button per layout "name W×H" (active accent-filled), footer "world seed … / library …".
2. Map column (focal moment): `TacticalMap` on ground `#2b2b2b` (`apps/viewer/src/styles.css:52`), SVG panes for grid, data and marks (`apps/viewer/src/components/TacticalMap.tsx:22-26`). Hovered square: white outline plus its edge walls in orange (run) or green (`apps/viewer/src/render/tacticalOverlay.ts:63-76`); pinned square: `#ff3b30` outline. `SquareReadout` HUD bottom-left ("square A1 (x, y) · x0–x1 ft E, y0–y1 ft S", `apps/viewer/src/components/SquareInfo.tsx:127-140`); `CacheCorner` HUD top-right (`apps/viewer/src/styles.css:105-112`).
3. Map tools bar (`apps/viewer/src/views/TacticalView.tsx:176-225`): "5-ft grid" checkbox, "source" select (WebP tiles / PNG), "px/square" select from `ppsq_options`, geometry summary text, fallback warning, PNG error.
4. Inspector, right (`:227-246`): layout name, `SquareInfo` for the shown square (Ground, Elevation, Water depth, Walls on edges; `apps/viewer/src/components/SquareInfo.tsx:15-124`), "Unpin" button when pinned, counts "N wall segments · M placements · K lights", "Edit this layout" link.

Pinning: click a square pins it; the inspector shows the pinned square over the hovered one (`apps/viewer/src/views/TacticalView.tsx:155`). Changing source or px/square clears the fallback and resets the cache counts (`:190-209`).

Keyboard: layout buttons, selects, checkbox and Unpin are native and tab-reachable; square hover and pin are pointer-only (`apps/viewer/src/components/TacticalMap.tsx:181-189`).

Responsive: no breakpoints; tools bar wraps; inspector caps at `45vw` (`apps/viewer/src/styles.css:60`, `:69`).

## States

| State | Trigger | Rendered | Site |
|---|---|---|---|
| Listing loading | `/v1/tactical/layouts` pending | notice "Loading /v1/tactical/layouts…" | `apps/viewer/src/views/TacticalView.tsx:32` |
| Not available | listing 404/501 | `NotAvailable`: "Not available on this server", the error in `code`, hint to point the API URL at a newer build | `:13-26`, `:34-35` |
| Listing error | other failure | `.notice.error` | `:37` |
| Empty | listing has no layouts | notice "The server lists no tactical layouts." | `:42` |
| Layout loading | layout JSON pending | notice "Loading layout {name}…" | `:79` |
| Layout not available / error | layout fetch 404/501 / other | `NotAvailable` for "Layout “name”" / `.notice.error` | `:80-86` |
| Idle | nothing hovered | readout "hover for square coordinates · click to pin a square"; inspector "Hover or click a square." | `:174`, `:229` |
| Hover / pinned | pointer, click | marks on map; `SquareInfo` with " · pinned" suffix when pinned | `:155-156`, `apps/viewer/src/components/SquareInfo.tsx:34` |
| Tile fallback | first tile error with source = tiles | switches to PNG; `.warn-text` "tiles failed (…); showing the PNG" | `apps/viewer/src/views/TacticalView.tsx:142`, `:223`; `apps/viewer/src/components/TacticalMap.tsx:224-228` |
| PNG error | PNG fetch failed | `.error` "PNG: …"; map source none | `apps/viewer/src/views/TacticalView.tsx:149-152`, `:224` |
| No images yet | cache log empty | corner "no image requests yet" | `apps/viewer/src/components/CacheCorner.tsx:29` |

## Motion

Leaflet default pan/zoom animation only.

## Copy

Technical, endpoint-forward: "Loading /v1/tactical/layouts…", "source", "px/square", "5-ft grid", "WebP tiles", "PNG", "Unpin", "Edit this layout" (`apps/viewer/src/views/TacticalView.tsx:32`, `:177-245`). Square labels are chess-style letters plus numbers via `squareLabel` (`apps/viewer/src/geo/coords.ts:117`). Water depth reads "dry", "N ft (wading)", "N ft (swimming)" (`apps/viewer/src/components/SquareInfo.tsx:9-12`).

## Not in play

- P3 grid: fixed list, flexible map, fixed inspector; no column grid.
- P5 permission-denied: the API has no auth.
- P9 touch targets: no touch sizing is defined.
