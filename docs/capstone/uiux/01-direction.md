---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: 9ccd81c01ead
paths_covered:
  - ":(top)apps/viewer/README.md"
  - ":(top)apps/viewer/src/styles.css"
  - ":(top)apps/viewer/src/App.tsx"
  - ":(top)apps/viewer/src/components/**"
  - ":(top)apps/viewer/src/views/**"
---

# Viewer direction (extracted)

Extraction mode: the incumbent direction of `apps/viewer` as found in code. No direction contract (thesis, own-world, story, first viewport as a commitment) was ever chosen for it, so none is written here; this file records the world the code holds consistently across its five surfaces.

## Design read

`apps/viewer` is a developer viewer for the `arda-server` v1 HTTP API, described by its own README as "a demo, a verification tool, and a reference" for game clients (`apps/viewer/README.md:3-5`). It is a single-page React app with five hash-routed views behind one top bar (`apps/viewer/src/App.tsx:12`, `:48-79`). Every view renders server data (map tiles, layouts, cell samples) and exposes the HTTP mechanics behind it: endpoint paths in copy, cache headers, status codes.

## Mode map

| Screen | Route | Mode (observed) | Chapter |
|---|---|---|---|
| World | `#/world` (default) | Operate: map exploration and inspection | `screens/01-world.md` |
| Area | `#/area?ax=&ay=` | Operate: data visualisation of one area | `screens/02-area.md` |
| Tactical | `#/tactical` | Operate: layout browsing and square inspection | `screens/03-tactical.md` |
| Cell | `#/cell?gx=&gy=&demo=&grade=` | Operate: per-cell tactical block viewer | `screens/04-cell.md` |
| Editor | `#/editor?layout=` | Operate: edit-and-render tool | `screens/05-editor.md` |

Routes from `apps/viewer/src/App.tsx:12-20` and the hash readers in each view. No Persuade or Read surface exists.

## The world as found

What holds across all five surfaces:

- One stylesheet, one token set: neutral warm-grey ground (`--bg #f4f3ef` light, `#17181b` dark), white or charcoal panels, a single blue `--accent` for active and primary state, OS-driven light/dark (`apps/viewer/src/styles.css:1-24`). Details in `02-system.md`.
- System UI face at 14px root; monospace for code, endpoints and the cache HUD (`apps/viewer/src/styles.css:12-13`, `:28`, `:108`).
- Map-first split layout: a full-bleed map or canvas column with a bottom tools bar, and a right-hand inspector panel of label/value rows (`apps/viewer/src/styles.css:48-82`). World, Tactical, Cell and Editor use it with a Leaflet map; Area uses it with a canvas stage.
- Dark translucent HUD chips laid over maps for coordinates and cache state, fixed-colour regardless of theme (`apps/viewer/src/styles.css:53-59`, `:105-112`).
- Plain-text status vocabulary: loading, error and unavailable states are bordered `.notice` panels whose copy names the endpoint or the server condition (`apps/viewer/src/App.tsx:81-88`; `apps/viewer/src/views/TacticalView.tsx:13-26`).
- Copy register: lowercase, terse, technical, with `·` as separator and raw API paths inline (`apps/viewer/src/views/WorldView.tsx:208-212`; `apps/viewer/src/views/LayoutEditor.tsx:202-204`).

## Dials (observed)

| Dial | Observed value | Evidence |
|---|---|---|
| Visual density | High: compact controls, 14px root, every bar `flex-wrap`s many controls | `apps/viewer/src/styles.css:13`, `:60`, `:86` |
| Colour | Restrained chrome; colour carried by map content and overlays | `apps/viewer/src/styles.css:1-24`; `apps/viewer/src/render/tacticalOverlay.ts:34-92` |
| Motion | None authored; Leaflet defaults only | no `transition`/`animation` in `apps/viewer/src/styles.css` |

## Theme

Light and dark, chosen by `prefers-color-scheme` with `color-scheme: light dark` (`apps/viewer/src/styles.css:11`, `:15-24`). Map grounds and HUD overlays do not change between themes (`:51-58`, `:105-112`). No use-scene sentence exists in the code or README.

## Not in play

- Direction contract blocks (THESIS, OWN-WORLD, STORY, FIRST VIEWPORT): not invented for an incumbent design (uiux extraction rule).
- Anti-default commitments, alternates and canon declined: none recorded anywhere in `apps/viewer/`.
