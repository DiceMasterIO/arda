---
generated_at_commit: 022ce7cb5707
generated_date: 2026-10-10
capstone_version: 7.0.1
content_hash: eb5a1b3e0d2f
paths_covered:
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/views/TacticalCellView.tsx"
  - ":(top)apps/viewer/src/components/**"
  - ":(top)apps/viewer/src/render/tacticalOverlay.ts"
  - ":(top)apps/viewer/src/geo/cellWalk.ts"
---

# Cell (`#/cell?gx=&gy=&demo=&grade=`)

Extraction mode. Component: `TacticalCellView` (`apps/viewer/src/views/TacticalCellView.tsx:21`) with `CellResult` (`:129`), `NotYetPanel` (`:181`) and `BlockView` (`:247`). Renders without `/v1/world`; when the world is loaded it bounds edge walking and enables the hand-off compare (`apps/viewer/src/App.tsx:92`; `apps/viewer/src/views/TacticalCellView.tsx:26`).

## Mode & job

Operate. Shows the world-derived tactical Block for one global cell from `GET /v1/tactical/cell/{gx}/{gy}` at 64 px/square (`RENDER_PPSQ`, `apps/viewer/src/views/TacticalCellView.tsx:18`, `:144`): the cell's seamless WebP tiles, its rules sidecar as toggleable overlays, NPC tokens from the scene endpoint, and arrows to walk to neighbouring cells (`:247-479`). Opening a block prefetches its 8 neighbours (`:275-290`).

Not on this screen: choosing a cell on a map (World tab, "Open tactical map here"; hint text `:107-109`); editing layouts (Editor tab).

## Composition

`.cell-view` column (`apps/viewer/src/views/TacticalCellView.tsx:63`, `apps/viewer/src/styles.css:115-117`):

1. Cell form bar (`apps/viewer/src/views/TacticalCellView.tsx:64-110`): "Tactical block at cell", `gx` and `gy` inputs (6rem, numeric input mode), "demo overlays" checkbox, `CellLookToggles` ("world grade", "world hand-off", the latter disabled without world relief; `apps/viewer/src/components/CellLookToggles.tsx:15-41`), "Load" submit, muted hint with the endpoint.
2. Optional hand-off strip (`apps/viewer/src/views/TacticalCellView.tsx:111`): `HandoffCompare`, three 256px figures: world relief at the hand-off zoom, a cross-fade with a range slider, and the tactical PNG (`apps/viewer/src/components/HandoffCompare.tsx:31-71`).
3. Result, one of: a `BlockView` split, the `NotYetPanel` notice, or a demo banner over a `BlockView`.

`BlockView` split (`apps/viewer/src/views/TacticalCellView.tsx:368-477`):
- Map column (focal moment): `TacticalMap` with rules overlay (water fill, difficult-terrain hatch, cover outlines) and NPC token squares under pointer marks (`:346-351`; `apps/viewer/src/render/tacticalOverlay.ts:43-95`). Four round edge-walk arrows on the map edges (`apps/viewer/src/components/EdgeArrows.tsx:10-30`, `apps/viewer/src/styles.css:127-134`). `SquareReadout` bottom-left, `CacheCorner` top-right.
- Map tools bar: "5-ft grid", "difficult terrain (hatch)", "water depth", "cover" (disabled with no sidecar), "NPC tokens (N)", a legend of wading/swimming/half/three-quarters/total swatches, prefetch status, sidecar and render errors (`apps/viewer/src/views/TacticalCellView.tsx:391-437`).
- Inspector "Block · name" or "Demo block · name": Origin, Render seed, Rules, Size, `meta.*` rows, then `SquareInfo` with rules and scene sections (`:439-476`; `apps/viewer/src/components/SquareInfo.tsx:68-121`).

Edge walking sets the hash to the neighbour and remounts `CellResult` (`apps/viewer/src/views/TacticalCellView.tsx:54-60`, `:113-121`); arrows are disabled past the world edge (`apps/viewer/src/geo/cellWalk.ts:30-37`). Hash changes from outside update cell and grade (`apps/viewer/src/views/TacticalCellView.tsx:29-42`).

Keyboard: the form submits on Enter (`:66-69`); edge arrows are native buttons with `aria-label="walk {dir}"` (`apps/viewer/src/components/EdgeArrows.tsx:18`); no arrow-key shortcuts. Square hover and pin are pointer-only.

Responsive: no breakpoints; bars wrap; hand-off figures wrap (`apps/viewer/src/styles.css:138`).

## States

| State | Trigger | Rendered | Site |
|---|---|---|---|
| No cell | hash lacks valid `gx`/`gy` | notice "Enter a global cell." (inputs prefilled 512, 1036) | `apps/viewer/src/views/TacticalCellView.tsx:27`, `:123` |
| Invalid input | non-integer or negative | submit ignored, no message | `:44-47` |
| Loading | cell request pending | notice "Loading /v1/tactical/cell/gx/gy…" | `:147` |
| Error | request failed | `.notice.error` "Cell gx, gy" plus `describeError` | `:148-156` |
| Not yet (501) | server has no block source | `.notice.not-yet`: "Not yet: no world-derived tactical block for cell gx, gy", status and code, planned source in a mono chip, explanation, demo-block picker and "Preview demo block" | `:211-243`; `apps/viewer/src/styles.css:118-119` |
| Demo busy / failed | preview pending / failed | muted "loading…" (button disabled) / muted error text | `apps/viewer/src/views/TacticalCellView.tsx:198-208`, `:238-241` |
| Demo block | preview built | `.warn-banner` "Demo block built in the browser from the {layout} layout: not world data. …" with "Back", over a `BlockView` rendered via `POST /render` | `:159-176` |
| Block | server block | `BlockView` with cell tiles | `:158` |
| No sidecar | block has no rules | toggles disabled; muted "this block has no rules sidecar"; inspector "No rules sidecar." | `:359`, `:434`; `apps/viewer/src/components/SquareInfo.tsx:72` |
| Sidecar invalid | `parseRules` threw | `.error` with the message | `apps/viewer/src/views/TacticalCellView.tsx:304-310`, `:435` |
| Prefetch | prefetch answered / failed | muted "Q/N neighbours prefetching" / "prefetch: …" | `:281-285`, `:429-433` |
| Scene missing | scene request failed | tokens 0, no Scene section | `:296`, `:302` |
| Walk blocked | neighbour past world edge | arrow disabled (opacity 0.3) | `apps/viewer/src/styles.css:130` |

## Motion

Leaflet default pan/zoom animation only. The hand-off cross-fade is a user-driven opacity, not timed (`apps/viewer/src/components/HandoffCompare.tsx:45-58`).

## Copy

Endpoint-forward and explanatory in the not-yet state: "The server answered 501 … World-derived blocks will come from {planned_source}, which isn't wired into arda-server yet." (`apps/viewer/src/views/TacticalCellView.tsx:213-216`). Controls: "Load", "Preview demo block", "Back", "demo overlays", "world grade", "world hand-off", "cross-fade", "NPC tokens (N)" (`:103-106`, `:171`, `:239`, `:414`). Checkbox titles carry query parameters and goal numbers (`:94`; `apps/viewer/src/components/CellLookToggles.tsx:17`, `:28`).

## Not in play

- P3 grid: flexible map plus fixed inspector; no column grid.
- P5 permission-denied: the API has no auth.
- P9 touch targets: edge arrows are 2rem circles; no other touch sizing is defined (`apps/viewer/src/styles.css:128`).
