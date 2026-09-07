---
generated_date: 2026-09-07
scenario: society-generation
generated_at_commit: 8d3c9d9
capstone_version: 6.4
---

# 06 — Society generation (realms, buildings, NPCs)

> Retained design and dated implementation history. Current implementation: `../01-architecture.md`, `../04-data-flow.md`.

Fourth batch stage, added as an approved scope expansion. Runs after every area exists (needs
all towns) and alongside/after blocks (building footprints come from
the block-stage layout function). Supersedes the artifact's "borders …
not produced" non-goal.

## Trigger & preconditions

- Trigger: batch step [4/5], after all areas; building objects require each settlement cell's block layout.
- Preconditions: all `areas/*/objects.bin` (settlements, roads) and the continent cost surface.

## Steps

1. **Realm seats**: the N largest towns, N scaling with continent population (default ≈ population/8,000, min 2 — assumed, tunable).
2. **Allegiance**: every settlement swears to the seat cheapest to reach over the road/terrain cost surface (roads cheap, terrain per the artifact's road-cost table).
3. **Borders**: realm territory = watershed of allegiance over land cells; the boundary snaps to a river or ridge line when one lies within ~2 cells (assumed). Realms are named with the region-naming scheme.
4. **Buildings**: for each settlement, the deterministic building-layout function (block stage, `logic/03`) yields the building list: id, type from a tier-scaled mix (hamlet: houses+barns; village: +church, inn, mill, smithy; town: +market hall, warehouses, keep), footprint squares, settlement id. Written to area objects; the same function constrains the block tiles — objects and drawings cannot disagree.
5. **NPC notables**: per settlement, tier-scaled named notables (hamlet ~2–4; village ~6–12; town ~20–60 — assumed) with SRD 5.1-style sheets: name (settlement naming scheme), ancestry, occupation/class, level, six ability scores, HP, AC, skills, gear; each linked to a building (occupant ids) and to their settlement.
6. **Commoners on demand**: any other inhabitant is derivable — a deterministic commoner sheet keyed (seed, settlement, index); never stored.

## Branches

- Seat count degenerates (<2 towns): whole continent = one realm (assumed).
- A settlement equidistant to two seats: lowest-cost tie broken by seat rank (deterministic).

## Unhappy paths

- None external — pure derivation from prior stages; determinism rules apply (subseeded per entity).

## State transitions

None — appends `Realm` objects (name, seat, member settlements, border course) to continent objects, buildings + notables to area objects.

## Invariants

- Every land cell belongs to exactly one realm; borders form closed partitions.
- Every notable occupies an existing building; every building belongs to an existing settlement.
- Same seed → same realms, buildings, names, sheets.
- Stored NPC count stays O(settlements), never O(population).

## Outcomes & side effects

- `Realm` objects in continent layer; `Building` and `Npc` objects in area layers; export JSON gains them (`04-export.md` amendment); NOTICE file carries SRD 5.1 CC-BY-4.0 attribution.

## Dimensions not in play

The retained design did not record a dimension-by-dimension exclusion list. This provenance repair leaves those exclusions unspecified rather than inventing decisions.
