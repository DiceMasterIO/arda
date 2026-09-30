---
generated_date: 2026-09-30
scenario: naming
artifact: ../mockup-artifact.md
status: normative design; implementation on feat/names (crate arda-names), replacing the local generators of arda-settle and arda-npc
goals: 40, 41, 53
---

# 15 — Naming: deterministic, culture-consistent, original names

> Normative design for every proper name Arda produces: settlements, rivers, lakes, peaks, ranges, passes, forests, regions, realms, noble houses, people, and named buildings such as inns. One crate, `arda-names`, owns the inventories and rules so that the names of a place, its river and its people sound like one culture. It replaces two local generators that exist today: `arda-settle`'s phonology module (code constants) and `arda-npc`'s `names.rs` with `data/content/names.json` (see the integration plan's adapter list). The site-suffix scheme is the artifact's ("Where people settle": "a village called Dermouth sits at a river mouth because that is what it was named for").

Code cites rules as `// logic/15 §<rule>`.

## Trigger & preconditions

- Trigger: library calls from 08 (settlements, rivers, peaks, passes, regions, realms), 14 (houses, regions, event text fills), 13 (people) and 10 (named buildings).
- Preconditions: the culture key is one of the canonical keys `heartland`, `highland`, `sylvan`, `coastal`, `southern`, `borderland` (08 Steps 3; `arda-npc` `cultures.json`); an unknown key falls back to `heartland`, as in `arda-npc`.

## Rules

### §name-key

Every name is a pure function of `(kind, culture, style, NameKey, context)`:

- `NameKey` is a 64-bit key supplied by the caller (the `key: u64` of `arda-names` `PlaceSpec`): for places, the canonical tactical hash of 09 §hash with kind `"name"` and the entity's `PlaceKind` and stable id; for people, `SeedKey::hash("name")` of their `arda-npc` seed key (13 §npc-seed), so a person's name still depends only on their own key (13 §npc-regeneration 2).
- `style` selects an inventory: the culture's for places and humans; an ancestry style (`dwarf`, `elf`, `halfling`, `gnome`, `dragonborn`, `tiefling`, `half-orc`) for people of that ancestry, as `arda-npc` does today (humans and half-elves use the culture's).
- `context` carries site tags for settlements (§name-site-suffix), the sex for given names, and the referent's name for derived names ("<river> Bridge").
- `arda-names` holds no global state and draws from no caller RNG object; it expands the key with its own deterministic stream.
- Place kinds are the closed `arda-names` `PlaceKind` set (hamlet, village, town, city, fort, abbey, port, mine, river, stream, lake, mountain, hill, forest, marsh, pass, bay, island, cape, vale, region, realm); a settlement's kind comes from its tier, or from its leading function (`fortress` → fort, `abbey`, `port`, `mining` → mine) when it has one. Major rivers are named in an older substrate tongue of the culture, streams in the living tongue (the `arda-names` design).

### §name-inventories

Inventories are data or data-like presets (`crates/arda-names/src/preset/`), original to Arda, never copied from a published name list or from PHB/DMG tables (goal-prompt §8 licensing). Per style: onsets, nuclei, codas, stem syllable-count weights, given-name parts per sex, family-name parts, byname patterns, site suffixes (§name-site-suffix), plain suffixes, and patterns for realm, region, river, mountain, range, pass, forest, lake, house and inn names. The existing `arda-settle` phonology and `arda-npc` name sets are the starting data and are migrated, not reinvented, so the maintainer's accepted look carries over.

### §name-site-suffix

A settlement's suffix encodes its site (artifact). The site is the first matching tag in this priority order (08 §settle-sites): `ford` or `bridge` → Ford; `estuary` → Mouth; `harbour` → Haven; `lake` → Mere; `confluence` → Meet; `pass` → Gate; `ore` with a `mining` function → Delve; `spring` → Well; `defensible` or `hill` → Hill; `marsh` → Fen; `forest` → Wood; `coast` → Strand; otherwise Plain. Each culture spells each site with one or more suffix variants of its own (for example the heartland Ford variants), chosen by key. The history hook of 14 §soc-founding uses the same site, so name and story agree.

### §name-form

- Capitalised; letters from the culture's alphabet only (ASCII letters plus the apostrophe and hyphen where the style allows them; assumed).
- Joins collapse doubled letters and forbid three consonants or three vowels in a row (the `arda-npc` rule).
- Length: settlements, rivers, realms 4–14 letters; given names 3–10; family names 3–12 (assumed, tunable).
- Denylist: a generated name (case-folded) must not equal or contain any entry of `data/denylist.json`: offensive words, and proper nouns that are product identity of the SRD publisher's settings (not in SRD 5.1, so not licensed). A hit redraws with the next attempt (§name-unique). This keeps names safe for a commercial product.

### §name-unique

Scopes (goal 40, goal 04 "unique within scope"):

| Kind | Unique within |
|---|---|
| settlement, realm, region, named river, lake, range, noble house | the world |
| peak, pass | its range, or the world if it has none |
| inn, tavern and other named buildings | the settlement |
| given and family names of people | not unique (households share family names) |

Resolution is deterministic: entities are named in ascending id order within a scope; on a collision the attempt counter in the key increments, up to 16 attempts; after that the culture's qualifier (`Upper`, `Nether`, `Great`, `Little` in their cultural spellings) is prefixed. A name therefore depends on the entity's own key and on the names of lower-id entities in its scope only.

### §name-what-is-named

- Settlements: all.
- Rivers: every river with order ≥ 3 and ≥ 30 cells of main stem, named by the culture at its mouth (thresholds from `arda-settle`; assumed, tunable).
- Peaks: summits rising ≥ 150 m above everything within 2.5 km, at most 60 per world (from `arda-settle`; assumed); ranges from 14 §soc-regions.
- Passes and crossings: passes get their own name; crossings are named after their river ("<river> Ford").
- Realms and regions: 08 §realm-seats, 14 §soc-regions, in the seat's culture.
- People: given, family and optional byname (12 % of adults, `arda-npc` value; assumed).
- Inns and taverns: an original two-part pattern ("The <adjective> <noun>") from culture data.

## Steps

1. Caller derives the `NameKey` (§name-key).
2. Pick the inventory by culture and style; build a stem; apply the site suffix or pattern.
3. Check form and denylist (§name-form); redraw on failure.
4. Check uniqueness in the caller's scope (§name-unique); the caller passes its scope set, `arda-names` returns the resolved name and the attempt used.

## Branches

- Unknown culture: `heartland`.
- Unknown ancestry style: the culture style.

## Unhappy paths

- A malformed inventory file is a load error naming the file and field; the crate embeds its data at build time (`include_str!`), so this is caught by a test, not at run time.
- Exhausted attempts with the qualifier also colliding: append the lowest free Roman numeral (II, III, …); never an error.

## State transitions

None: pure functions over embedded data.

## Invariants

1. Determinism: the same key and context always give the same name [40].
2. Uniqueness per §name-unique on MICRO seed 42 and on the default world's settlements file [40].
3. Culture consistency: a classifier trained on nothing but the inventories (syllable membership) attributes ≥ 95 % of generated settlement names to their own culture (assumed test design) [40].
4. Site suffix: every settlement whose top site is not Plain carries one of its culture's variants for that site [40; artifact].
5. Form and denylist rules hold for 100,000 generated names per style [40].
6. People's names depend only on their own key: permuting buildings does not change any name (13 invariant 1) [51, 53].
7. Originality: every inventory file carries `"_note": "Original to Arda"`, and no entry of a list of known published fantasy name lists (a test fixture of their distinctive names) is generated verbatim in 100,000 draws [goal-prompt §8].

## Outcomes & side effects

Strings. `arda-settle` and `arda-society` write them into `society/*.json`; `arda-npc` puts them into `Npc.name`. Labels on the world overlay (goal 41) use the same strings.

## Dimensions not in play

- Translation, etymology chains and name change over history (a renamed town).
- Fonts and label placement (08 overlay, 16 tiles).
