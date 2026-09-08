---
generated_date: 2026-09-08
scenario: load-query
absorbed_from: features/2026-09-07-area-water-terrain-realism@2026-09-08
---

# 05 — Load & query

The implemented read path in `crates/arda/src/world.rs`, also used by [export](04-export.md). The retained [crate API mockup](../mockup/04-crate-api.md) includes future queries; the methods and lifetimes below describe the current implementation as of 2026-09-08.

## Trigger & preconditions

- Trigger: `World::load(&Path)` followed by manifest, area, cell or block queries.
- The directory must contain a parseable `world.json` with current format major 4. Its configuration must validate, and its area dimensions must match the configuration. Saved data is treated as immutable for the lifetime of a `World`.
- Loading the completion manifest does not eagerly verify the existence or validity of every saved layer.

## Steps

1. Read and version-gate only `world.json`. Revalidate its configuration and area dimensions, derive the fine domain as the union of requested kilometre extent and exported 512-cell areas, and allocate empty, separate area and block cache slots for the exported coordinates. No area, continent layer, global hydrology table or block archive is decoded at this point.
2. Answer `seed()`, `size_km()`, `areas()`, `dir()`, `manifest()` and row-major `area_coords()` from this manifest state.
3. `area(ax, ay)` loads the requested tile's `cells.bin` and `objects.bin` on first successful access, stores the resulting `Area` in its `OnceLock`, and returns a stable borrowed reference. `read_area(ax, ay)` reads the same files into an owned `Area` without consulting or populating the cache; the caller releases its payload by dropping it.
4. Before allocating area input buffers, bound cells by their exact 512²×39-byte format size and objects by the codec's current 128 MiB default limit. Decode the layers, validate the copied annual water context against the manifest-derived domain, and reject channel endpoints outside that domain. Each area carries local rivers/lakes, saved channel geometry and the relevant global hydrology context, so reading it does not open neighboring areas or global tables.
5. `Area::cell(x,y)` returns a checked borrowed `Cell`; `cells()` exposes the grid, `rivers()` and `lakes()` expose local fragments, and `objects()` exposes saved geometry and copied hydrology context. Hydrology model revision 2 describes representative annual support and mean annual flow with stable global identities. Stored `Cell` fields can be read through Rust even where area JSON omits them; the presence of a stored field does not establish a future society or vegetation simulation.
6. `block(ax,ay,cx,cy)` uses an independent `OnceLock` for that area's block archive. The first request reads and decompresses the entire `.tiles.zst` archive, then returns the requested saved block. It does not load area cells/objects. Further block requests in that area reuse the archive. Blocks exist only for materialized cells, currently a 64-cell stride over land. Their 64×64 tile IDs and the existing 24-tile tactical vocabulary are unchanged.

## Branches

- Borrowed area access retains each successfully queried area until `World` is dropped; owned area reads have caller-controlled lifetime and are used for area/overview export passes.
- Block queries retain entire queried area archives, not just individual requested blocks. Area and block caches are independent.
- Cached queries reuse saved in-memory data. Owned reads perform fresh file reads; callers must not modify a world beneath either access mode.

## Unhappy paths

- Missing/partial world: `ManifestMissing`; unreadable/malformed manifest, invalid configuration or mismatched area dimensions: `ManifestUnreadable`.
- Incompatible format major: `VersionSkew` carries found and supported versions. The current exact-major reader rejects both older and newer formats, including format 3. Existing format-3 worlds remain preserved; no in-place migration or rewrite is performed.
- A missing, inaccessible or changed file can produce an ordinary I/O error on a later uncached query even after `World::load` succeeds. Codec failures retain file context for truncation, malformed records, unsupported values, invalid hydrology/domain geometry and exceeded size/resource limits.
- Area/cell coordinates outside their valid ranges return `OutOfRange`. A valid in-range cell without a materialized block also returns `OutOfRange`; CLI export adds the sampled-land-block explanation.
- Failed reads do not populate the corresponding cache. They do not return a fabricated empty area or ocean tile.

## State transitions

On disk, none: queries create no lock files and write no metadata or layers. In memory, an area or block-archive cache slot changes from empty to populated after a successful read and remains populated until the `World` is dropped. Concurrent borrowed accesses converge on a stable cached value; racing first accesses may each read/decode before one value is retained. An owned `read_area` has no cache transition.

## Invariants

- Loading and queries do not recompute terrain, water, annual state or tactical generation.
- With unchanged stored files, decoded fields and identities reproduce generation's saved decisions. Area context supplies its own relevant water authorities without recursive neighbor loads.
- Manifest loading retains empty slots proportional to exported areas. Payload memory additionally includes cached requested areas and whole block archives for requested areas. Owned export reads can release one area's cells/objects before reading the next; repeatedly calling the borrowed accessor intentionally retains those areas.
- Successful manifest loading is not a promise that future reads are infallible. The requested layer must still pass I/O, format, geometry and resource checks.

## Outcomes & side effects

- Success provides manifest facts, typed area/cell views and materialized block access, with the documented cache lifetime. Rendering/serialization is performed by the export functions or renderer crate, not by methods on `Block`.
- Failure returns the relevant typed error without changing saved data. Source authorities are `crates/arda/src/world.rs`, `crates/arda-core/src/formats/{manifest,cells,area_objects_v4,hydrology,blocks}.rs` and `crates/arda-core/src/error.rs`.

## Dimensions not in play

Settlements, roads, road crossings, passes, buildings, population, NPC/society queries, named continent-object accessors, custom artwork/tilesets, collision and movement schemas, and `arda serve` remain deferred. The 2026-09-07 retained design listed these broader object queries and described post-load reads as otherwise infallible; that was planned behavior, superseded for current readers by the explicit methods and lazy failure rules above. The consumer reference does not close pending natural-panel, golden or project-verification gates.
