---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 0a578d6f5325
paths_covered:
  - ":(top)apps/viewer/index.html"
  - ":(top)apps/viewer/package.json"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/main.tsx"
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/config.ts"
  - ":(top)apps/viewer/src/hooks.ts"
  - ":(top)apps/viewer/src/api/client.ts"
  - ":(top)apps/viewer/src/geo/fitOnResize.ts"
  - ":(top)apps/viewer/src/geo/cellWalk.ts"
  - ":(top)apps/viewer/src/components/**"
  - ":(top)apps/viewer/src/views/**"
  - ":(top)apps/viewer/src/render/**"
  - ":(top)apps/viewer/vite.config.ts"
  - ":(top)apps/viewer/scripts/**"
  - ":(top)apps/viewer/licence-policy.json"
---

# Viewer design system (extracted)

Extraction mode: what `apps/viewer` implements, as found. The whole visual system is one hand-written stylesheet, `apps/viewer/src/styles.css` (143 lines), plus `leaflet/dist/leaflet.css` (`apps/viewer/src/main.tsx:3`) and colour literals in TypeScript for map overlays. No component library, no CSS framework, no CSS modules, no design-token file beyond the `:root` custom properties (`apps/viewer/package.json:17-21` lists only `leaflet`, `react`, `react-dom` as runtime dependencies).

## Typography

| Role | Value | Site |
|---|---|---|
| UI face | `system-ui, -apple-system, "Segoe UI", sans-serif` | `apps/viewer/src/styles.css:12` |
| Mono face | `ui-monospace, "SF Mono", Menlo, monospace` | `apps/viewer/src/styles.css:28`, `:108`, `:119` |
| Root size | `14px` | `apps/viewer/src/styles.css:13` |
| Brand (`.brand`) | `1.05rem` | `apps/viewer/src/styles.css:34` |
| Panel `h2` (`.notice h2`, `.inspector h2`) | `1.1rem` | `apps/viewer/src/styles.css:67`, `:74` |
| Layout list `h2` | `1rem` | `apps/viewer/src/styles.css:98` |
| Inspector `h3` | `0.8rem`, uppercase, `letter-spacing: 0.04em`, `var(--muted)` | `apps/viewer/src/styles.css:75` |
| `code` | `0.92em` | `apps/viewer/src/styles.css:28` |
| `.health` | `0.9rem` | `apps/viewer/src/styles.css:43` |
| `.small`, `figcaption` | `0.85rem` | `apps/viewer/src/styles.css:62`, `:140` |
| Cache corner | `0.75rem/1.35` mono | `apps/viewer/src/styles.css:108` |
| Inspector `pre` | `0.75rem` | `apps/viewer/src/styles.css:80` |
| Area picker buttons | `0.7rem` | `apps/viewer/src/styles.css:88` |

No type scale ratio: sizes are individual literals. Numeric readouts use `font-variant-numeric: tabular-nums` (`apps/viewer/src/styles.css:56`). No web fonts are loaded (`apps/viewer/index.html:1-12` has no font link). No prose measure is set; `.notice` caps at `max-width: 46rem` (`apps/viewer/src/styles.css:64`).

## Color

Strategy: neutral chrome with one blue accent; colour-rich content comes only from server images and data overlays. Two themes follow the OS via `prefers-color-scheme` (`apps/viewer/src/styles.css:11`, `:15-24`); there is no in-app theme switch.

| Token | Light | Dark | Site |
|---|---|---|---|
| `--bg` (ground) | `#f4f3ef` | `#17181b` | `apps/viewer/src/styles.css:2`, `:17` |
| `--panel` (raised surface) | `#ffffff` | `#212327` | `:3`, `:18` |
| `--ink` (primary text) | `#1d1f22` | `#e7e6e2` | `:4`, `:19` |
| `--muted` (secondary text) | `#6b6f76` | `#9a9ea6` | `:5`, `:20` |
| `--line` (borders) | `#d9d7d0` | `#34363b` | `:6`, `:21` |
| `--accent` | `#2d6cdf` | `#6c9cff` | `:7`, `:22` |
| `--ok` | `#1f9d55` | same | `:8` |
| `--bad` | `#d64545` | same | `:9` |
| `--warn` | `#c88a12` | same | `:10` |

Accent use: active tab background (`apps/viewer/src/styles.css:38`), active area-picker cell (`:89`), active layout-list row (`:101`), inspector links (`:82`), `button.primary` (`:124`). Text on accent is hard-coded `#fff` in both themes (`:38`, `:89`, `:101`, `:124`), and muted text inside an active list row is `#e3ebff` (`:104`).

Semantic colours:

| Meaning | Treatment | Site |
|---|---|---|
| Error text | `.error { color: var(--bad) }` | `apps/viewer/src/styles.css:63` |
| Error panel | `.notice.error`: `--bad` border, `--ink` text | `:65` |
| Unavailable / not-yet panel | 4px `--warn` left border | `:66`, `:118` |
| Warning text | `.warn-text { color: var(--warn) }` | `:113` |
| Warning banner | `color-mix(in srgb, var(--warn) 20%, var(--panel))` with `--warn` bottom border | `:121` |
| Health dot | `--warn` checking, `--ok` up, `--bad` down | `:44-46` |
| Cache hit / miss | `#4cd07d` / `#ffb340` on the dark HUD | `:111-112` |

Map surfaces use fixed colours independent of theme: World map ground `#0e2438` (`apps/viewer/src/styles.css:51`), tactical map ground `#2b2b2b` (`:52`), HUD readouts `rgba(20, 22, 26, 0.82)` with `#f1f1f1` text (`:55`) and cache corner `rgba(20, 22, 26, 0.85)` with `#e8e8e8` (`:107`).

Overlay palette (TypeScript literals, not tokens):

| Mark | Colour | Site |
|---|---|---|
| Selected cell / pinned square outline | `#ff3b30` | `apps/viewer/src/views/WorldView.tsx:192`, `apps/viewer/src/render/tacticalOverlay.ts:65` |
| Hovered square outline | `rgba(255,255,255,0.9)` | `apps/viewer/src/render/tacticalOverlay.ts:68` |
| Wall under pointer: run / other | `#ff9f0a` / `#30d158` | `apps/viewer/src/render/tacticalOverlay.ts:72` |
| Edited square outline | `#ff9f0a` | `apps/viewer/src/render/tacticalOverlay.ts:80` |
| Cover: half / three-quarters / total | `#f5c542` / `#f5862f` / `#e5484d` | `apps/viewer/src/render/tacticalOverlay.ts:34` |
| Water: wading / swimming | `rgba(80,170,255,0.38)` / `rgba(20,80,210,0.55)` | `apps/viewer/src/render/tacticalOverlay.ts:39` |
| Difficult-terrain hatch | `rgba(120,72,20,0.18)` fill, `rgba(92,52,8,0.85)` lines | `apps/viewer/src/components/TacticalMap.tsx:100-101` |
| NPC token: worker / resident | `#e5484d` / `#3e63dd`, white stroke | `apps/viewer/src/render/tacticalOverlay.ts:91-92` |
| 5-ft grid | `rgba(0,0,0,0.5)`, non-scaling 1px | `apps/viewer/src/components/TacticalMap.tsx:88` |
| Area boundaries | `#ffffff`, opacity 0.45, dash `4 4` | `apps/viewer/src/views/WorldView.tsx:87-91` |
| Area heatmap ramps (HYPSO, SEQ, THERMAL, cover, sea, lake) | RGB arrays | `apps/viewer/src/render/heatmap.ts:19-66` |
| Area rivers / lakes | `rgba(20,90,220,0.95)` / `rgba(40,110,200,0.85)` | `apps/viewer/src/views/AreaView.tsx:83`, `:89` |

Contrast: no ratio is declared or tested anywhere in `apps/viewer/`. Theme parity: the dark block redefines only the six neutral and accent tokens; `--ok`, `--bad`, `--warn` and all map/HUD literals are shared (`apps/viewer/src/styles.css:15-24`).

## Spacing and shape

No spacing scale: paddings and gaps are per-rule rem literals. Recurring values: gap `1rem` for bars (`apps/viewer/src/styles.css:31`, `:60`, `:86`, `:138`), `0.4rem`/`0.25rem` inside groups (`:35`, `:39`), panel padding `0.75rem 1rem` (`:69`, `:85`), bar padding `0.45rem-0.5rem 0.75rem-1rem` (`:32`, `:60`, `:116`, `:137`).

| Radius | Used on | Site |
|---|---|---|
| `8px` | `.notice` | `apps/viewer/src/styles.css:64` |
| `6px` | `.tab`, `.readout`, `.cache-corner` | `:36`, `:55`, `:106-107` |
| `4px` | inputs, layout-list buttons, `button.primary`, `.planned-source` | `:40`, `:100`, `:117`, `:119`, `:124` |
| `3px` | area-picker buttons, legend gradient | `:88`, `:94` |
| `2px` | legend swatches | `:96` |
| `50%` | health dot, edge-walk arrows | `:44`, `:128` |

Separation is by 1px `--line` borders (top bar bottom `:32`, map tools top `:60`, inspector left `:70`, layout list right `:97`, cell form and hand-off bottom `:116`, `:137`) and dotted row dividers in the inspector (`:77`). No `box-shadow` is declared except `box-shadow: none` on area labels (`:83`); Leaflet's own control shadows come from `leaflet.css`.

Density: compact. Inputs are padded `0.25rem 0.4rem` (`:40`) or `0.2rem 0.35rem` (`:117`); inspector rows are a `9.5rem 1fr` grid with `0.2rem` vertical padding (`:77`). One density only.

## Layout

| Region | Rule | Site |
|---|---|---|
| Shell | full-height flex column: `.topbar` then `.content` | `apps/viewer/src/styles.css:26`, `:29`, `:47`; `apps/viewer/src/App.tsx:47-95` |
| Top bar | brand, view tabs, API form pushed right (`margin-left: auto`), health indicator; `flex-wrap: wrap` | `apps/viewer/src/styles.css:30-39`; `apps/viewer/src/App.tsx:48-79` |
| Split | `.split` flex row: map or main column, then a fixed side panel | `apps/viewer/src/styles.css:48-50` |
| Map column | `.map-wrap` relative flex column: `.map` (min 300px high), floating `.readout`, `.map-tools` bar at the bottom | `:50-60` |
| Inspector | `width: 24rem; max-width: 45vw`, scrolls vertically | `:68-71` |
| Layout list | `width: 14rem`, left of the map | `:97` |

No breakpoints: the only `@media` query is the colour-scheme one (`apps/viewer/src/styles.css:15`). Narrow widths are handled by `flex-wrap` on bars and `max-width: 45vw` on the inspector; nothing collapses or moves. The Area stage sizes with `min(100%, 720px, calc(100vh - 215px))`, square, `min-width: 256px` (`:90`). Leaflet maps refit on container resize until first sized (`apps/viewer/src/geo/fitOnResize.ts:7-15`).

## Components

Hand-built in React; each primitive is a CSS class, not a component, except where noted.

| Primitive | States implemented | Site |
|---|---|---|
| View tab (`a.tab`) | default, hover (`--bg` fill), active (accent fill, `aria-current="page"`) | `apps/viewer/src/styles.css:36-38`; `apps/viewer/src/App.tsx:52` |
| Button (native) | browser default look; `cursor: pointer`; disabled via attribute, no custom style | `apps/viewer/src/styles.css:41-42` |
| `button.primary` | default, disabled (`opacity: 0.6`); no hover/focus/active style | `:124-125`; used once, `apps/viewer/src/views/LayoutEditor.tsx:244-254` |
| Edge-walk arrow | default (`opacity: 0.85`), hover not disabled (1), disabled (0.3, default cursor) | `apps/viewer/src/styles.css:128-134`; `apps/viewer/src/components/EdgeArrows.tsx:10-30` |
| List button (area picker, layout list) | default, active (accent) | `apps/viewer/src/styles.css:88-89`, `:100-101` |
| Text input | bordered, `--bg` fill; no focus, error or disabled styling | `:40`, `:117` |
| Checkbox / select / range | native controls, `font: inherit` for select | `:41` |
| `.notice` | neutral, `.error`, `.unavailable`, `.not-yet` | `:64-67`, `:118` |
| `.banner.warn-banner` | single warning variant | `:120-121` |
| `.inspector` | side panel with `.inspector-head` close button (`×`), `h2`, `h3`, `dl` rows, `pre`, links | `:68-82`; `apps/viewer/src/views/CellInspector.tsx:4-41` |
| `.readout` | floating HUD on maps; `.static` inline variant | `:53-59` |
| `.cache-corner` (`CacheCorner`) | empty ("no image requests yet"), last-request; hit green / miss amber | `apps/viewer/src/components/CacheCorner.tsx:8-33` |
| `.health` (`HealthIndicator`) | checking, up, down | `apps/viewer/src/components/HealthIndicator.tsx:5`, `:38-48` |
| `.legend` / `.swatch` | gradient bar, colour swatches | `apps/viewer/src/styles.css:93-96` |
| `TacticalMap` | Leaflet `CRS.Simple` map with tile/image/none source and three fixed SVG panes (grid 410, data 420, marks 430) | `apps/viewer/src/components/TacticalMap.tsx:22-26`, `:148-275` |

Focus: no `:focus` or `:focus-visible` rule exists in `apps/viewer/src/styles.css`; focus rendering is the browser default. Selection on maps is an outline mark (`#ff3b30`) distinct from the hover outline (white) (`apps/viewer/src/render/tacticalOverlay.ts:65-68`).

Button hierarchy: one `primary` class (accent fill) and native buttons for everything else; no destructive or tertiary variant exists. The "Open tactical map here" link reuses `tab active` styling to look like a button (`apps/viewer/src/views/WorldView.tsx:254`).

## Iconography

No icon library. Glyphs are Unicode text: `×` close (`apps/viewer/src/views/CellInspector.tsx:12`), arrows `↑ → ↓ ←` for edge walking (`apps/viewer/src/geo/cellWalk.ts:16`), `→` inside text. The health dot is a CSS circle (`apps/viewer/src/styles.css:44`).

## Imagery

All imagery is server-rendered map content: overview and relief WebP tiles (`apps/viewer/src/views/WorldView.tsx:52-70`), tactical tiles or PNG (`apps/viewer/src/components/TacticalMap.tsx:202-241`), relief crops in the hand-off compare (`apps/viewer/src/components/HandoffCompare.tsx:9-25`), and canvas heatmaps painted in the browser (`apps/viewer/src/render/heatmap.ts:90-121`). Area heatmaps use `image-rendering: pixelated` (`apps/viewer/src/styles.css:91`). Before an image arrives the map shows its fixed ground colour (`:51-52`). No illustration, photography or decorative imagery.

## Motion

No CSS transitions or animations (no `transition`/`animation` in `apps/viewer/src/styles.css`). Pan and zoom animation is Leaflet's default; the first fit after resize passes `animate: false` (`apps/viewer/src/geo/fitOnResize.ts:13`). The hand-off cross-fade is a user-driven opacity slider, not an animation (`apps/viewer/src/components/HandoffCompare.tsx:45-58`). No `prefers-reduced-motion` handling.

## Loading, empty and error vocabulary

| Kind | Treatment | Site |
|---|---|---|
| Blocking load | `.notice` paragraph naming the endpoint, e.g. "Loading /v1/world…" | `apps/viewer/src/App.tsx:81`; `apps/viewer/src/views/TacticalView.tsx:32` |
| Inline busy | muted text "rendering…", "loading…", "checking…" | `apps/viewer/src/views/LayoutEditor.tsx:202`; `apps/viewer/src/views/TacticalCellView.tsx:199`; `apps/viewer/src/components/HealthIndicator.tsx:43` |
| Request error | `.notice.error` or `.error` text with `describeError` (`<status> <code>: <message>` or `cannot reach <url> (...)`) | `apps/viewer/src/api/client.ts:405-411` |
| Not offered (404/501) | `.notice.unavailable` "Not available on this server" or `.notice.not-yet` | `apps/viewer/src/views/TacticalView.tsx:13-26`; `apps/viewer/src/views/TacticalCellView.tsx:211-243` |
| Empty | `.notice` sentence ("The server lists no tactical layouts.") or muted hint ("Hover or click a square.") | `apps/viewer/src/views/TacticalView.tsx:42`, `:229` |
| Modal alert | `window.alert` on Area cell fetch failure (the only one) | `apps/viewer/src/views/AreaView.tsx:229` |

No spinners, skeletons or progress bars. Loads are aborted on dependency change via `useAsync`/`whileLive` (`apps/viewer/src/hooks.ts:9-54`).

## Form fields

Labels wrap or precede their control inline (`<label>text <select/></label>`), never above it (`apps/viewer/src/views/AreaView.tsx:149-183`; `apps/viewer/src/App.tsx:65-73`). No placeholders, no help text, no required markers, no field-level error text. The Cell form silently ignores non-integer or negative input (`apps/viewer/src/views/TacticalCellView.tsx:44-50`). The API URL "Connect" button is disabled while the draft equals the current base (`apps/viewer/src/App.tsx:74`).

## Design-system pick

Hand-built: React 19 components with one global stylesheet and Leaflet 1.9 for maps (`apps/viewer/package.json:17-21`). No component library.

Build-time licence handling (no runtime or visual effect): the Vite config adds the `licenceNotices()` plugin (`apps/viewer/vite.config.ts:12`), which during `build` emits `THIRD_PARTY_LICENSES.txt` into `dist/` with the licence text of every non-dev package in `package-lock.json` and fails the build when a package has no licence file (`apps/viewer/scripts/licence-notices.mjs:25-38`). `npm run licences` checks the lockfile against `apps/viewer/licence-policy.json`, whose `runtime` allow-list is MIT, ISC, BSD-2-Clause, BSD-3-Clause, Apache-2.0, 0BSD, Zlib, MIT-0, CC0-1.0, and whose `buildOnly` list is MPL-2.0, CC-BY-4.0, BlueOak-1.0.0, Python-2.0 (`apps/viewer/package.json:15`, `apps/viewer/scripts/check-licences.mjs:1-4`); `npm test` also runs `node --test scripts/` (`apps/viewer/package.json:14`).

## Implementation constraints

uiux-craft §8's build-time checklist, each item with what the code observably implements:

| Checklist item | Observed in `apps/viewer` |
|---|---|
| Every interactive component implements default, hover, focus, active, disabled, loading, error | Hover exists on tabs and edge arrows only; disabled styled on `button.primary` and edge arrows; no focus or active rules (`apps/viewer/src/styles.css:36-38`, `:124-130`) |
| Skeletal loading; empty states teach; errors inline with recovery | Text notices; the World load error names the recovery command (`apps/viewer/src/App.tsx:86`); the unavailable notice names the fix (`apps/viewer/src/views/TacticalView.tsx:20-23`) |
| Contrast verified 4.5:1 / 3:1 per theme | Not measured anywhere in the repo |
| One radius system, one icon family, one accent | Six radius values (table above); Unicode glyphs; one `--accent` per theme |
| Animation on transform/opacity only; reduced motion | No authored animation; no reduced-motion rule |
| Overlays escape their containers | HUDs are absolutely positioned inside `.map-wrap` (`apps/viewer/src/styles.css:53-59`, `:105-110`) |
| Real images; no hand-rolled icons or emoji-as-icons | Server-rendered map images; Unicode arrows and `×` as icons |
| Interface copy: no em-dashes, labels above inputs, error text below | `—` appears as an empty-value marker (`apps/viewer/src/components/SquareInfo.tsx:77-95`, `apps/viewer/src/components/CacheCorner.tsx:22`, `:26`); labels inline |
| CTAs: one label per intent | "Open tactical map here" appears in two places with the same label (`apps/viewer/src/views/WorldView.tsx:255`, `apps/viewer/src/views/CellInspector.tsx:32`) |
| `prefers-reduced-motion` and keyboard focus honored | No reduced-motion rule; browser-default focus only |

Project facts a build must keep: the API base resolves `?api=` then `localStorage["arda-viewer.apiBase"]` then `VITE_ARDA_API` then `http://localhost:8787` (`apps/viewer/src/config.ts:3-20`); routes are hash routes (`apps/viewer/src/App.tsx:12-20`).

## Assets

| Asset | File | Source | Status |
|---|---|---|---|
| Logo mark | `docs/capstone/uiux/assets/logo-mark.svg` | placeholder | awaited |
| Logo wordmark | `docs/capstone/uiux/assets/logo-wordmark.svg` | placeholder | awaited |
| Logo mono | `docs/capstone/uiux/assets/logo-mono.svg` | placeholder | awaited |
| Logo on dark | `docs/capstone/uiux/assets/logo-on-dark.svg` | placeholder | awaited |
| Favicon source | `docs/capstone/uiux/assets/favicon.svg` | placeholder | awaited |
| `og.png` source | `docs/capstone/uiux/assets/og.svg` | placeholder | awaited |
| App icon source | `docs/capstone/uiux/assets/app-icon.svg` | placeholder | awaited |
| Email header | `docs/capstone/uiux/assets/email-header.svg` | placeholder | awaited |

The viewer ships none of these: `apps/viewer/index.html:1-12` links no icon, the brand is the text "Arda viewer" (`apps/viewer/src/App.tsx:49`), and no `public/` directory is tracked. No screen calls for an empty-state illustration.
