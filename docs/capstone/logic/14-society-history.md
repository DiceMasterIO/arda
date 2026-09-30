---
generated_date: 2026-09-30
scenario: society-history
status: normative design; implementation on feat/society (crate arda-society)
goals: 36, 38, 40, 53, 56, 57, 70
---

# 14 — Society and history: polities, offices and a consistent past

> Normative design for the layer that gives realms and settlements a political structure and a history a game master can use: who rules, which houses hold which seats, how realms stand toward each other, and what happened where. It builds on the realms and settlements of [08](08-settlements-roads-realms.md) (which already implement [06](06-society-generation.md) steps 1–3), names everything through [15](15-naming.md), and ties offices to buildings of the [town plan](10-town-layout.md) so the ruler is an actual NPC of [13](13-npc-population.md) who lives in an actual keep (goal 45). History is narrative: a deterministic, internally consistent record, not a simulation of growth.

Code cites rules as `// logic/14 §<rule>`.

## Trigger & preconditions

- Trigger: `arda-society generate --world <dir>` after `arda-settle` (a second post-pass), or the library call `society(world, seed)`.
- Preconditions: `society/settlements.json`, `realms.json`, `roads.json` and `features.json` exist with `format_version` 1 (08 Outcomes). The world seed is the manifest's.

## Rules

### §soc-calendar

One calendar for the world: integer years, `0` is the present of the campaign, the past is negative. The present is not tied to any wall clock (standards: no wall-clock time in sim paths).

### §soc-founding

- The world's history spans `T` years, `T ∈ [200, 500]` drawn once from `H(seed, "history-span")` (the span feat/society uses; assumed, tunable). Stages run in causal order: founding → realm formation → wars and border shifts → disasters → reigns → ruins → prosperity → present-day hooks, and the outcome of the last stage **is** the present world of 08.
- Every settlement gets `founded_year ∈ [−T, −5]`. Towns and cities are older than villages, villages older than hamlets, and within a tier a larger settlement is older (rank order); the exact year comes from `H(seed, "founded", id)` within its tier's band (bands assumed, tunable: cities and towns in the oldest 40 % of the span, villages in the oldest 80 %, hamlets anywhere).
- A settlement's founding story is chosen from its site tags and functions (goal 36): a `crossing` settlement "grew at the ford/bridge of <river>", a `port` "was a landing on <coast feature>", a `mining` village "was a miners' camp on <peak>", an `abbey` "grew around the abbey of <name>", a `fortress` "grew under the castle raised to hold <pass or height>". Every named referent must exist in `features.json`, `roads.json` or this file. The 08 `history` hook is replaced by the story's first sentence, so the two cannot disagree.

### §soc-realms

- Each realm has a founding year no later than its seat's `founded_year + 50` and no earlier than the seat's founding (assumed).
- **Ruling house**: a family name from 15-naming (`family` kind, the seat's culture), stored on the realm. The ruler is the master of the seat's `keep`, or of its best `manor` if it has no keep (10 §town-functions), and is resolved as an NPC at query time (§soc-offices). The ruler's household carries the house name: the NPC adapter passes it as the family name of that building's household (see Outcomes and the integration plan adapter list).
- **Lesser houses**: every town and city that is not a seat and every `fortress` village has a lord's office at its keep or manor, held by a lesser house (one family name per office, seat culture or local culture).
- **Allegiance chain**: settlement → lord (if any) → realm ruler. Every settlement's `realm_id` in 08 is authoritative; this layer adds only the lord.

### §soc-relations

Between every pair of realms that share a land border (08 `neighbours`), a relation from {`alliance`, `rivalry`, `truce`, `vassalage`}, chosen by hash with weights shifted by culture (same culture: alliance ×2; different: rivalry ×2) and relative population (a realm with < 1/3 of its neighbour's population may be its vassal) (assumed, tunable). Relations are symmetric, except `vassalage`, which is stored once with `overlord` and `vassal`. Non-neighbouring realms have no relation.

### §soc-events

- Each realm has 4–12 events and each town or city 1–4; villages at most 1; hamlets none (bounded storage, goal 56; counts assumed).
- Event kinds (closed set): `founding`, `succession`, `reign`, `war`, `border_shift`, `peace`, `siege`, `bridge_built`, `abbey_founded`, `charter` (market right), `mine_opened`, `fire`, `flood`, `plague`, `famine`, `mine_collapse`, `abandonment`. Each has a year, participants (realm ids, settlement ids, house names) and a place (a settlement, crossing, pass, river or peak id).
- Consistency rules (all tested): an event's year lies after every participant settlement's `founded_year` and after its realm's founding; a `war` or `siege` involves realms that share a border and happens at a place on or near that border (within 5 km, assumed); a `peace` follows a `war` between the same realms; a `bridge_built` refers to an existing `bridge` crossing; a `flood` to a settlement with `riverine` or `coastal`; a `border_shift` to two neighbouring realms and ends in the 08 partition; `mine_opened` to a `mining` settlement; `charter` to a `market` settlement; `abbey_founded` to an `abbey` settlement; a `succession` names a house that held the office before and one that holds it now (the current one is the stored ruling house).
- Event text is composed from original templates (no copied prose), filled with names from 15-naming.

### §soc-offices

An office is `{kind, settlement_id, building_key, house}` with kind in {`ruler`, `lord`, `abbot`, `high_priest`, `mayor`, `reeve`}. The holder is resolved on demand: the NPC whose job is the office's job at that building (masters of keeps and manors for `ruler`/`lord`, the master of the temple for `high_priest`, the abbey temple for `abbot`, the officials of 13 §npc-jobs for `mayor` and `reeve`). Offices are stored; holders are not (goal 56).

### §soc-present

History may move borders, destroy settlements and change rulers in the past, but the present it ends in is fixed by 08 (constraints flow down only):

- The present realm of every settlement is its 08 `realm_id`; a border shift is an event whose end state is the 08 partition.
- A **ruin** is an abandoned settlement site beside the road network: on a land cell that 08 does not refuse, at least 1.5 km from every living settlement, not on a `built` cell, and emptied by an event of this file near it (plague, flood, war, famine, fire, mine collapse) that falls within its lifetime, otherwise by a stated local cause. Ruins are points of interest for the tactical layer; they are not settlements and have no NPCs.
- **Economy** is narrative and bounded: per market settlement, key goods produced (from functions and land use), and trade flows between markets that follow existing 08 roads or ferries only, with a traffic figure per road for the referee. No prices, stocks or ledgers.
- **Factions** inside a settlement (court, guilds, clergy, watch, criminals, …) each have a goal, a seat building of the town plan, a leader office (§soc-offices) and a stance toward every other faction, stored once per unordered pair and read symmetrically.

### §soc-regions

Named regions for the overview (goal 40): each realm, each culture region of 08 Steps 3 larger than 500 km², and each major river basin and mountain range already in `features.json`. Region records carry an anchor point for labels (goal 41) and a polygon or a reference to the raster that defines them.

## Steps

1. Load the 08 files.
2. Founding years and stories (§soc-founding).
3. Realm founding, ruling houses, lesser houses and offices (§soc-realms, §soc-offices).
4. Relations (§soc-relations).
5. Wars, border shifts, disasters, reigns, ruins and prosperity (§soc-events, §soc-present), then a consistency pass that drops any draft event failing a rule and redraws it up to 8 times, deterministically (attempt in the key), before omitting it (assumed).
6. Regions (§soc-regions).
7. Write `society/history.json`.

## Branches

- One realm: no relations, no wars; events are internal only.
- A seat without a keep or manor (a small town seat): the ruler's office sits at the seat's largest non-dwelling building (market hall, then temple).

## Unhappy paths

- Missing or wrong-version 08 files: refused with the file named; nothing written.
- A referenced feature id missing from 08 files: an internal invariant violation (a bug), reported with the event and id.

## State transitions

`society/history.json` is staged and renamed into `society/`, like 08. It never touches the 08 files. Re-running 08 invalidates it: `history.json` records the BLAKE3 of the 08 files it read and the service refuses a stale one (16 §api-errors, `stale_society`).

## Invariants

1. Determinism: byte-identical `history.json` for the same world and 08 files [goal-prompt §8].
2. Referential integrity: every id and name reference resolves [36, 38].
3. Temporal order: §soc-events rules hold for every event; every settlement's founding precedes its events [36].
4. Relations: symmetric; only between neighbours; vassalage has one overlord [38].
5. Offices: every realm has exactly one `ruler` office at a building that exists in the seat's town plan; the resolved holder is an NPC whose `home_building` or `workplace_building` is that building [45, 53].
6. Bounded storage: file size grows with settlements and realms, not population (≤ 4 KB per town and ≤ 16 KB per realm, assumed) [56].
7. Names are unique within their scope (15 §name-unique) [40].
8. Present: every settlement's present realm equals its 08 `realm_id`; every ruin satisfies §soc-present; every trade flow follows existing roads or ferries; faction stances are symmetric [36, 38].

## Outcomes & side effects

`society/history.json`, format 1: `{format_version, seed (decimal string), inputs_hash, calendar: {present_year: 0, span_years}, realms: [{id, founded_year, ruling_house, relations: [{realm, kind}], events: [event id]}], settlements: [{id, founded_year, story, lord_house, events: [event id]}], offices: [Office], events: [{id, year, kind, place: {kind, id}, participants, text}], regions: [{id, kind, name, anchor_m, source}], ruins: [{id, cell, founded_year, abandoned_year, cause_event}], economy: [{settlement, goods, flows: [{to, road_ids, traffic}]}], factions: [{settlement, kind, goal, seat_building_key, office, stances}]}`.

## Dimensions not in play

- Simulated growth and migration, and battles resolved by numbers: history is a causal narrative constrained to end in the 08 present.
- Religion as a mechanical system: temples carry an original faith key per culture for flavour only; no deity statistics (SRD 5.1 content only).
- Prices, stocks and trade balances (§soc-present keeps economy narrative).
