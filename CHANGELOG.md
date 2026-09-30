# Changelog

Release notes for Arda. The design ledger (decisions, stage records and per-change detail)
lives in `docs/capstone/changelog.md` and `docs/capstone/changelog.d/`.

## 0.2.0 — 2026-09-30

Arda grows from a world generator into a product: a physically formed world, the people who
live in it, tactical battle maps of any 100 m cell, an HTTP API and a browser viewer. The same
seed still gives the same bytes. Legacy (Classic) worlds are unchanged but for the version
string in `world.json`.

### World physics (recipe-5 "fine" terrain)

- **Variant Q mountains:** stream-power formation with drifting plates, convergent routing,
  glacial troughs and talus seams gives ranges with real crests, valleys and flanks. The
  uneroded-bands fix removes the flat strips that used to cross belts.
- **Water** (`areas/*/water.bin`): hydraulic geometry for every reach, irregular meanders
  with oxbow lakes that hold water, braided reaches, and river-led deltas with levees, forking
  distributaries and delta islands. Closed basins drain or become proper lakes.
- **Coasts** (`terrain/shore.bin`): beaches in bays and at river mouths, shingle, cliffs,
  rocky shores, marsh and tidal flats; islands, volcanic arcs standing above the sea, a
  continental shelf with wandering submarine canyons, and coastal plains.
- **Atlas look:** sub-grid relief detail, tapered smooth river centrelines, a warmer formed
  palette and stored shores painted along the coast.
- A full-size world (500 × 1000 km, seed 42) generates within 13.4 GiB peak memory.

### Settlements, names and society

- `arda settle`: settlements of every tier sited on the terrain, roads, crossings, passes,
  realms, land use and names (`<world>/society/`).
- `arda-names`: languages and dialects per culture, used for places, houses and people.
- `arda society build`: a town plan for every settlement (streets, plots, buildings, walls,
  crofts, river bridges and docks), politics, trade, economy and history, and the stored
  notables. MICRO seed 42 holds 629 settlements and about 92,000 people.

### People

- `arda-npc`: households, jobs, personalities and relationships for every settlement, with
  notables bound to the offices and houses of the society layer.
- Every NPC has an SRD 5.1 stat sheet (see `NOTICE` for the attribution).

### Tactical maps

- 5 ft battle maps of any 100 m cell: WFC-driven terrain refined from the world, towns from
  the settlement plans, roads, bridges and fords on the refined rivers, fields, pastures and
  crofts around villages, and organic forest edges and pools.
- Scene rules (movement, cover, sight, decks, exits) in a format-2 sidecar, and NPC tokens
  placed in their homes and workplaces.
- The art compositor renders layouts to PNG with global seeding, so neighbouring cells meet
  seamlessly; a placeholder art library ships in `assets/tactical/placeholder`
  (`arda tactical validate` checks it).

### Mid-zoom relief

- `arda-midzoom` refines the stored 39 m field to about 10 m with drainage-aligned gullies
  and ribs. Relief tiles shade with the overview's own shader, palette, shores and rivers, so
  zooming past the overview is continuous.

### Server and viewer

- `arda-server`: the HTTP API (`/v1/world`, cells, points, overview and relief tiles,
  settlements and plans, tactical cells, windows, scenes and PNGs, `/v1/npc/{id}`), with
  TypeScript bindings generated from the Rust types in `bindings/ts/`.
- `apps/viewer`: a browser viewer that pans and zooms the world, switches to relief tiles past
  the overview, and opens the tactical map of a cell with its tokens.

### Compatibility

- World format 4 is unchanged; recipe-5 worlds gain the optional `shore.bin` and `water.bin`
  layers. Worlds generated with 0.1.0 fine terrain should be regenerated to get the new terrain,
  water and coasts.
