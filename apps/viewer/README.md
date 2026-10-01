# Arda viewer

A small developer viewer for the `arda-server` v1 HTTP API
([`crates/arda-server/API.md`](../../crates/arda-server/API.md)). It's a demo, a
verification tool, and a reference for how a game client should consume Arda.

- **World**: pan/zoom slippy map of `/v1/tiles/overview/{z}/{x}/{y}.webp` (lossless WebP, Leaflet, flat CRS).
  Hovering shows the global cell, area, local cell and position in km. Clicking
  opens an inspector with the full `CellSample` in plain language with units.
- **Area**: pick an area and draw a canvas heatmap of height, cover, soil moisture,
  forest density, temperature or slope from `/v1/area/{ax}/{ay}/cells?format=bin`, with
  `/rivers` and `/lakes` overlaid. Hover for values, click a cell to inspect it.
- **Tactical**: lists `/v1/tactical/layouts` and shows the chosen layout from its 512 px
  WebP tile pyramid (`/v1/tactical/layout/{name}/tiles/{z}/{x}/{y}.webp`) in a Leaflet
  `CRS.Simple` map in square units, falling back to the PNG when a tile fails (or on
  request). It has a browser-side 5-ft grid, a square readout (A1 labels, feet), and a
  square inspector (ground, elevation, water depth and the walls on the square's edges).
  A debug corner shows the latest image's `ETag` and `X-Arda-Cache` with hit/miss counts.
- **Cell**: `/v1/tactical/cell/{gx}/{gy}` for a typed or clicked cell (World inspector:
  "Open tactical map here"). The world-derived Block (layout, format-2 rules, origin,
  scene) is drawn from the cell's own seamless WebP tiles
  (`/v1/tactical/cell/{gx}/{gy}/tiles/…?ppsq=64`) with the rules sidecar overlaid as
  toggles (difficult-terrain hatch, water depth, cover). "demo overlays" adds
  `?demo_overlays=1` (the synthetic ways, fields and town samples; `#/cell?…&demo=1`).
  Opening a block posts `POST /v1/tactical/prefetch` for its 8 neighbours at the same
  ppsq, so the server warms them in the background (the map tools show how many were
  queued). The arrows on the map's edges walk one cell north, west, east or south: the
  neighbour's image joins this one seamlessly and loads from the warmed cache.
  A server with the pending block source answers 501, shown as a "not yet" state naming
  the planned source, where "Preview demo block" renders a block built in the browser
  from a built-in layout through `POST /v1/tactical/render?origin=X,Y`.
- **Editor**: loads a layout's JSON, changes a square's ground from a dropdown (the keys
  of `/v1/tactical/library`), and POSTs it to `/v1/tactical/render`, showing the result
  and its cache headers.
- A health dot polls `/v1/health` every 5 s.

## Run

Needs Node 22+ (the repo uses Node 26 through mise) and a running server.

```sh
# from the repository root: build and start a server
cargo build --release -p arda-server -p arda-cli
target/release/arda generate --seed 42 --micro --terrain fine --out out/micro42   # once
target/release/arda-server --world out/micro42 --port 8787

# in another shell
cd apps/viewer
npm ci
npm run dev          # http://localhost:5173
```

The API base URL defaults to `http://localhost:8787`. To change it:

- edit the **API** field in the top bar (the choice is saved in `localStorage`),
- open `http://localhost:5173/?api=http://localhost:8790`, or
- set `VITE_ARDA_API=http://localhost:8790` when you run `npm run dev` or `npm run build`.

The server's CORS policy allows any `localhost` or `127.0.0.1` origin, so the dev server
talks to it directly. Views can be deep-linked: `#/world`, `#/area?ax=1&ay=2`, `#/tactical`,
`#/cell?gx=512&gy=1036`, `#/editor?layout=riverside`.

## Scripts

| Script | What it does |
|---|---|
| `npm run dev` | Vite dev server |
| `npm run build` | Typecheck, then production build into `dist/` |
| `npm run typecheck` | `tsc -b --noEmit` (strict) |
| `npm run lint` | ESLint (typescript-eslint strict, type-checked) |
| `npm test` | Vitest unit tests (API client, ARDACOLS parser, coordinates, formatting) |

## Using Arda from TypeScript

The types come from the ts-rs bindings at `bindings/ts/arda/`, imported through the `@arda`
alias (`tsconfig.app.json` `paths`, `vite.config.ts` `resolve.alias`). Don't copy them.
Regenerate them from Rust with `ARDA_BLESS_BINDINGS=1 cargo test -p arda-server bindings`.

`src/api/client.ts` is the reference client:

- Every cell-bearing body is checked against `CONTRACT_VERSION`, and an unknown version
  throws `ContractVersionError`.
- Errors become `ApiRequestError` with the server's stable `code` (`out_of_range`,
  `not_found`, …). `notAvailable` is true for 404 and 501.
- `areaCellsBin` fetches the ~26 MB `ARDACOLS` binary form. `src/api/areaBin.ts` wraps
  each column as a zero-copy `Float32Array`/`Uint32Array`/`Uint8Array`, so prefer it
  over the ~38 MB JSON form.
- Seeds and reach/basin ids are decimal strings. Keep them as strings.

`src/geo/coords.ts` holds the coordinate conversions: pyramid pixels to cells, cells to
areas and metres, slippy tiles, and tactical squares.

The tactical types all come from the `@arda` bindings; `src/api/tactical.ts` only checks
response shapes and holds square/wall helpers. The rules sidecar is `unknown` in the
bindings (arda-scene `RulesSidecar`), so `parseRules` validates it before use.
`src/geo/tacticalTiles.ts` holds the pyramid geometry (levels, tile grid, zoom to
pixels per square).

The ground dropdown lists the keys the server's built-in layouts use, because the server
has no library listing route.

## Licences

The runtime dependencies are React and React DOM (MIT) and Leaflet (BSD-2-Clause). The
build tooling is MIT/Apache-2.0/ISC/BSD, with a few transitive exceptions:
`lightningcss` (MPL-2.0, Vite's CSS minifier), `caniuse-lite` (CC-BY-4.0 data) and
`minimatch` (BlueOak-1.0.0). They're build-time only and aren't shipped in `dist/`.
