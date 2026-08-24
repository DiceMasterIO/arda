# arda

Deterministic fantasy worldgen for tabletop games. One seed produces one
continent, resolved down to the 5-ft squares of a D&D battle map, with
geography that holds up to scrutiny: rivers run where water would go,
villages sit on river terraces, roads follow the terrain that carries
them.

**Status: design finished, implementation not started.** The repository
will grow a Rust workspace; nothing here is usable yet.

## What it generates

Three tiers sharing one coordinate system:

- **Continent** (default 500×1000 km, configurable): plate tectonics
  coupled with erosion, climate from latitude and orography, major
  rivers, settlement density, trunk roads, realms with borders.
- **Areas** (51.2 km tiles, 512×512 cells of ~100 m): relief, hydrology,
  climate, vegetation, settlements, land use and roads, each stage
  computed only from the stages before it. Adjacent tiles agree on
  every shared edge by construction.
- **Blocks** (64×64 five-ft squares per cell): tile grids filled by
  constrained WFC. Rivers and roads cross block boundaries without
  breaking; a standard battle map is a quarter block.

The world also carries named regions, ranges, rivers and seas; towns,
villages and hamlets with populations and site tags; buildings with
type and footprint; and named NPCs with D&D-style stat sheets.

Everything derives from a single 64-bit seed. The same seed and config
produce a byte-identical world on any OS — share the seed, not the
gigabytes.

## Planned usage

```sh
# generate a world (hours; runs on a 16 GB desktop)
arda generate --seed 42 --out worlds/w42

# export maps and data
arda export --world worlds/w42 --area 03_11 --cell 300,128 --format png,json

# serve a world read-only over HTTP (for VTT integration)
arda serve worlds/w42 --port 8080
```

Also usable as a library (`arda` on crates.io, eventually) and as a
docker image with a mounted output volume.

## Output

- Per-cell data: height, cover, climate, hydrology, roads, ownership.
- Objects: rivers, lakes, settlements, roads, crossings, passes,
  realms, buildings, NPCs.
- Renders: cartographic continent/area maps and battle-map PNGs, plus
  versioned JSON for everything, including per-tile attributes
  (traversability, movement cost, cover, hazards) for game engines.

## License

MIT or Apache-2.0, at your option.
