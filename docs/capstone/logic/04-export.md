---
generated_date: 2026-09-07
scenario: export
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 04 — Export

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

Renders images and serializes JSON from a stored world on demand —
never during the batch. Surface mocked in
`../mockup/03-export.md`; rules here are the confirmed design.

## Trigger & preconditions

- Trigger: `arda export` (CLI/docker) or the crate's render/serialize calls.
- Preconditions: a complete world directory — manifest present and stamped; partial worlds are refused with a pointer at `generate`.

## Steps

1. **Resolve target** from `world.json` ranges: continent scope (no `--area`), area scope (`--area ax_ay`), or cell scope (`--area` + `--cell x,y`).
2. **Load** only what the target needs; block tile-IDs are decompressed on demand from the area archive.
3. **Render**:
   - Continent: cartographic overview PNG — hypsometric relief, water, cover, roads/corridors, settlement glyphs, names — plus regions/settlements/rivers JSON.
   - Area: area-map PNG at 1 px/cell (same cartographic style) + `cells.json` + `objects.json`.
   - Cell: block PNG at 8 px/square (resolution assumed, mockup 03) using the built-in symbolic style (flat colors + glyphs per tile kind, grid lines, contour shading) or a user tileset manifest mapping tile-ID → sprite; plus block JSON (tile-ID grid + tile-name legend).
4. **Serialize**: `cells.json` carries exactly the artifact's "what the finished map knows" fields — snake_case, SI units, one object per cell; `objects.json` lists river segments, lakes, settlements, roads, crossings, passes with courses as cell-coord arrays; every JSON file carries a top-level `schema_version`; evolution is additive-only.
5. **Write** to `--out`, overwriting; identical request → byte-identical files.

## Branches

- Scope selector (step 1); style source — built-in vs tileset manifest (step 3).

## Unhappy paths

- Out-of-range `--area`/`--cell`: exit non-zero naming the valid ranges from the manifest.
- Tileset manifest missing tile-IDs: exit non-zero listing every unmapped ID; no partial image.
- Partial/absent world: refused (precondition).
- Overwrite of existing outputs: silent and safe — outputs are pure functions of (world, request, style).

## State transitions

None — export never mutates the world directory.

## Invariants

- Read-only on the world; deterministic byte-identical outputs.
- JSON schema versioned, additive-only.
- Every tile-ID in a block render resolves to art (built-in always total; manifests validated up front).

## Outcomes & side effects

- Success: listed artifacts with sizes (mockup 03 layout); exit 0.
- Failure: no partial artifacts — each file is written whole or not at all (assumed).

## Amendments (build gate, 2026-08-25)

- Block JSON gains: `legend` entries carrying tile attributes (material, traversable, move_cost, cover, hazard tags), a `poi` list, and `buildings` present in the block.
- `objects.json` gains `buildings` (type, footprint, settlement, occupants) and `npcs` (notables with SRD 5.1-style sheets); continent JSON gains `realms` (name, seat, members, border course).
- All additions are additive under `schema_version` 1 (evolution rule).
- New consumer: `arda serve` (`../mockup/06-serve.md`) reuses these serializers/renderers verbatim — one source of truth per representation.

## Dimensions not in play

The retained design did not record a dimension-by-dimension exclusion list. This provenance repair leaves those exclusions unspecified rather than inventing decisions.
