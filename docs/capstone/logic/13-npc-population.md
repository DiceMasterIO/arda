---
generated_date: 2026-09-30
scenario: npc-population
status: normative design; implemented on feat/npc-population (crate arda-npc); inputs come from 08 and 10 through adapters
goals: 51, 52, 53, 54, 55, 56, 57, 69
---

# 13 — NPC population: jobs, personality, SRD sheets and regeneration

> Normative design for the inhabitants of every settlement. It refines [06 — Society generation](06-society-generation.md) steps 5–6 (notables stored, commoners on demand) and adds job titles and personalities. `arda-npc` is a pure library driven by a `SettlementProfile` (from [08](08-settlements-roads-realms.md)) and a list of `BuildingSpec`s (from [10](10-town-layout.md) §town-building-spec). NPCs are placed on tactical maps as tokens by [12](12-scene-data.md) §scene-tokens and served by [16](16-service-api.md). Names come from [15 — Naming](15-naming.md) once `arda-names` merges. SRD 5.1 content only, CC-BY-4.0, attributed in the root `NOTICE` and `crates/arda-npc/data/srd/SOURCE.md`.

Code cites rules as `// logic/13 §<rule>`.

## Trigger & preconditions

- Trigger: `generate_population(world_seed, &profile, &buildings)`; `npc(world_seed, &profile, &buildings, NpcId)`; `commoner(world_seed, &profile, &buildings, building, index)`; `Generator::new(..)?.npc(id)` for many lookups on one skeleton.
- Preconditions: every building's `settlement_id` equals the profile's id; building ids are unique; `Σ capacity ≥ population` (never trimmed: a shortfall is `NpcError::Housing`).

## Rules

### §npc-inputs

`SettlementProfile`: `id` (u64), `name`, `tier`, `population` (u32), `functions` (sorted set, 08 §settle-functions), `wealth` (u8), `culture` (a culture key; unknown keys fall back to `heartland`), `realm_id` (u32), `biome`, `coastal`, `riverine`, optional `ancestry_mix`. The 08 settlement record deserialises into it directly; extra record fields are ignored.

`BuildingSpec`: `id` (u64, unique in the settlement), `settlement_id`, `function` (`BuildingFunction`, serialised adjacently tagged as `{"kind": "<key>"}` or `{"kind": "workshop", "craft": "<craft>"}`), `capacity` (u16), `workplace_slots` (u16), `wealth` (u8). Produced by 10 §town-building-spec; never hand-built outside tests and the `sample` stand-in.

### §npc-id

`NpcId = {settlement, building, index}`: the home building and the person's index among its residents. It is stable for a given world seed and building list. Wire form (16-service-api): the string `"<settlement>.<building>.<index>"` in decimal, for example `"12.34.2"`.

### §npc-seed

Every random draw comes from a stream keyed by BLAKE3 of `"arda-npc/v1"`, world seed, settlement id, building id, index and a purpose string, feeding a hand-rolled xoshiro256**. Buildings are sorted by id before use, so nothing depends on input order (goal 51).

### §npc-regeneration

1. **Skeleton** (rebuilt in full on every call; about 65 ms for 20,000 people in release): population shared over buildings by capacity (largest remainder); households from each building's key; jobs assigned by the deterministic pass of §npc-jobs; friends and rivals paired from a seeded shuffle of all adults, so every tie is mutual by construction.
2. **Expansion**: one person's name, wealth, personality and sheet come only from their own key and their household's key; expanding one person never changes another.
3. Hence `npc(seed, profile, buildings, id)` equals the record inside a full run, and equals the stored copy for notables (goal 51).

### §npc-storage

Only notables are stored (as full `Npc` records inside `Population`); every other inhabitant is derivable (goal 56). The service stores nothing: it recomputes the skeleton from the town plan and caches populations in a bounded cache (16 §api-cache). Stored size is O(notables) = O(settlements) because notable counts are bounded per tier (§npc-notables).

### §npc-households

Household sizes are drawn from the weights `1:6, 2:13, 3:17, 4:20, 5:18, 6:13, 7:8, 8:5` (per cent, `tiers.json`; assumed, tunable). Kinds of tie: spouse, parent, child, sibling, landlord, lodger. Ages are in each ancestry's own years: parents are at least `max(16, adult_age × 3/4)` years older than their children; nobody exceeds the ancestry's maximum age (SRD race lifespans); a human–elf couple has half-elf children. Family names are carried in households (goal 55).

### §npc-jobs

Jobs attach to buildings in this order (goal 52):

1. Residents staff their own building; the first seasoned resident is the master; the master's family takes the building's family job if it has one.
2. The pool of working-age people fills the remaining workplace slots in a seeded order; masters are always seasoned adults.
3. The society layer's offices bind (14 §soc-offices; holder = the master or a worker of the office's building, else a mature resident or household head), then the tier appoints its officials: village `reeve`; town `mayor`, `magistrate`, `tax_collector`; city `lord_mayor`, two `magistrate`s, two `tax_collector`s; a capital adds a `herald` (`tiers.json`). An official is skipped when a held office covers it (a `reeve` office stands in for the reeve, an `elder` office for a headman or elder; `occupations.json` `offices[].covers`) or when the tier's civic cap is reached (hamlet 1, village 3, town 8, city 16; `tiers.json` `civic_cap`; assumed, tunable). Office holders keep their own job: a hamlet's elder is usually also its farmer, woodcutter or fisher.
4. Everyone left takes a land job weighted by `occupations.json`, the settlement's functions and the tier's rural/urban factor (per mille: hamlet 1000/150, village 1000/300, town 220/1000, city 60/1400). Children and elders get the nominal jobs `child` and `retired`.

Target statistics (tests): farming 60–80 % of working adults in hamlets and villages (goal-prompt text "roughly 60–80 %"), 5–35 % in towns, about 1 % in cities; port towns 30–45 % maritime; one master per inn, temple and smithy.

### §npc-personality

Two traits on different axes, one ideal with an alignment lean, one bond, one flaw, one mannerism and a one-line summary, all from original tables in `data/content/personality.json` (never PHB/DMG tables). Coherence rules: an ideal must be allowed for the job category (no atheist priest); traits must not contradict each other (distinct axes); the bond names a real person, place or building of the settlement when one fits (goal 53).

### §npc-relationships

Family, employer and employees, one friend and one rival within the settlement; every tie appears on both people with the inverse kind (spouse ↔ spouse, parent ↔ child, landlord ↔ lodger, employer ↔ employee, friend ↔ friend, rival ↔ rival) (goal 53).

### §npc-sheet

Every NPC has an SRD 5.1 sheet (goal 54):

- **Stat-block sheets** for commoners and staff, exactly as printed in the SRD appendix: Commoner by default; Guard for watch, men-at-arms and soldiers; Veteran for sergeants; Knight; Acolyte and Priest for clergy; Noble for a lord's family; Thug, Scout, Druid or Mage by job. The race adds size, speed, darkvision, languages, resistances and trait names (the SRD appendix's note on racial traits); the block's numbers are unchanged.
- **Class sheets** for notables: class drawn from the job's weights (a smith is usually a fighter, a high priest a cleric); subclass once the level allows (the one SRD subclass per class). Levels by tier: hamlet notables 1–2 (leaders 2–3), village 1–2 (2–3), town 1–4 (leaders 5–8), city 2–6 (elites 8–12); 2 % get +3 levels, capped at 20 (`tiers.json`; the goal-prompt ranges "villages 1–3, towns up to 5–8, cities up to 10–12" are met).
- **Class maths** (SRD): standard array in class priority, racial bonuses, Ability Score Improvements; HP = hit-die maximum at level 1, then `die/2 + 1` per level, plus Con, plus hill-dwarf or draconic bonuses; AC from SRD armour with capped Dex, shield, Defense style or unarmoured defence; attacks from SRD weapons with proficiency, finesse and monk rules; saves and skills from class, an original flavour background, race and expertise; casters get the SRD slot table (full, half or Pact Magic), save DC `8 + prof + mod`, attack `prof + mod`, and a few SRD spells by name and level; equipment by job and wealth band; SRD lifestyle from wealth.
- **Allow-list**: every race, subrace, class, subclass, spell, item and stat block name must appear in `data/srd/*.json`.

### §npc-notables

Notables are the masters of notable workplaces, the holders of society offices and the officials. Counts per tier (hamlet 2–4, village 6–12, town 20–60 from logic/06 step 5; city 60–200 assumed, tunable: logic/06 gives no city figure). Above the range, the least important masters are demoted to their job's stat block (office holders never are); below it, household heads are promoted by the tier's list (hamlet `headman`, `land`, `yeoman`; village `elder`, `yeoman`; town `elder`, `yeoman`; city `magistrate`, `factor`), where `land` keeps the head's own land job (the woodcutter of a logging hamlet stays a woodcutter) and a civic title (a government job) is skipped once the tier's civic cap is reached. A civic figure is a notable with a government job or a civic office (`ruler`, `lord`, `steward`, `reeve`, `elder`, `commons_voice`, `harbourmaster`). Hence a hamlet of 20–80 people has exactly one civic figure — the elder of 14 §soc-offices, or a promoted headman without the society layer — and its other notables are its boatwright or land folk. A hamlet with fewer than two mature heads may fall short; this is recorded, not an error.

Target statistics on the seed-42 MICRO world (tests, `arda-people/tests/notable_mix.rs`): masters ≤ 40 % of notables, tradesfolk ≥ 40 %, labourers 5–30 %, gentry and nobles ≤ 10 %; government jobs ≤ 12 %, religion ≤ 12 %; hamlet notables ≥ 70 % agriculture, extraction, maritime or labour; town and city notables ≥ 45 % craft, trade or service; no settlement above its tier's civic cap.

### §npc-ranks

Social rank (`dependent` 0, `labourer` 1, `tradesfolk` 2, `master` 3, `gentry` 4, `noble` 5) is the job's rank in the settlement's tier (`occupations.json` `rank`, overridden per tier by `rank_by_tier`), raised to the rank of the society office the person holds (`offices[].rank`, `rank_by_tier`). `master` means a guild master or prosperous owner, not "runs a building": a workshop, smithy, bakery, brewery, tannery, inn or boathouse owner is tradesfolk in a hamlet or village and a master in a town or city; a tavern keeper, shrine keeper, headman, elder and reeve are tradesfolk everywhere; a miller, factor or harbourmaster is a master from the village up; a village lord of the manor is gentry, a town or city lord noble. Offices: `ruler` 5; `lord` 5 (4 in hamlets and villages); `steward`, `abbot`, `court_mage`, `spymaster` 4; `high_priest` 4 (3 in hamlets and villages); `harbourmaster` 3 (2 in hamlets); `captain` 3 (4 in cities); `reeve`, `elder`, `shrine_keeper`, `crime_boss`, `boatmaster` 2; `commons_voice`, `fisher_head`, `forester`, `miner_captain` 1 (a voice of the commons is one of the commons, so a farmer who speaks for them stays a labourer). Hence an "Elder of …" who farms ranks with a reeve, never below.

The title "Master …" belongs to the master rank: below it the prefix is dropped (a village "Smith", a town "Master Smith").

Wealth (0–255) is `settlement/4 + home/4 + 30 × rank + d40`, then held inside the rank's lifestyle band for every working person (`rank_lifestyles`): labourer squalid–modest, tradesfolk poor–comfortable, master modest–wealthy, gentry comfortable–aristocratic, noble wealthy–aristocratic. Dependants (children, the retired) are not banded and live as their home does.

### §npc-queries

`Population` answers `residents(building)`, `workers(building)`, `by_job(key)`, `job_of(id)`, `notables()`, `job_counts()`, `category_counts()` (goal 57). Realm-level and world-level queries (by realm, by job across settlements) are the service's, built by iterating settlements (16 §api-routes).

### §npc-game-schema

The game consumes NPC sheets in its own SRD 5.1 creature schema (goal 69). The contract is the `arda-npc` `Npc` and `Sheet` JSON as serialised by serde; the service exposes it through ts-rs DTO mirrors whose JSON is byte-identical to the domain serialisation (16 §api-bindings). `arda-npc` stays free of HTTP and ts-rs dependencies. The contract is also published as JSON Schema (`Npc.json`, `Sheet.json`; 16 §api-schema). Because the game's own schema is not Arda's, the server can reshape every served `Npc` into it through a declarative sheet mapping (16 §api-sheet-mapping); the mapping is data, so the game's schema needs no Arda code change.

## Steps

1. Validate inputs (Preconditions).
2. Sort buildings; build the skeleton (§npc-regeneration 1, §npc-households, §npc-jobs, §npc-relationships).
3. Select notables (§npc-notables); expand them fully (§npc-personality, §npc-sheet).
4. Return `Population { settlement_id, households, notables, commoner_count, roster }`.

## Branches

- `ancestry_mix` given: it replaces the culture's ancestry table.
- Coastal or riverine without a `fishing` function: fishing land jobs are still allowed.
- A building with zero capacity and zero slots (a barn-like spec): ignored with a note; the adapter should not send such specs.

## Unhappy paths

- Housing shortfall: `NpcError::Housing { population, capacity }` — the town layout must fix it (10 §town-capacity), never trimmed.
- Mismatched settlement id or duplicate building id: `NpcError::Input`.
- Unknown `NpcId`: `NpcError::NotFound`.

## State transitions

None: pure functions.

## Invariants

1. Determinism and order independence: identical output across runs and across any permutation of the building list [51].
2. Regeneration: `npc(..)` equals the full-population record for every notable and for sampled commoners [51].
3. SRD maths recomputed from the raw JSON: modifiers, proficiency by level, HP, AC per armour type, save DC, attack bonus, slots for full, half and pact casters [54].
4. Every stat-block sheet matches its data file [54].
5. Allow-list holds [54].
6. Occupation statistics within §npc-jobs tolerances for hamlet, village, town and port town [52]; notable statistics within §npc-notables tolerances on the MICRO world.
6a. Every NPC's rank is its job's tier rank raised to its office's rank, and every working NPC's lifestyle lies in its rank's band (§npc-ranks); a hamlet has one civic figure.
7. Every workplace slot of a staffed building is filled; every NPC's home exists in the building list [52, 55].
8. Household ages consistent; relationships symmetric [53, 55].
9. No personality contradicts its job [53].
10. A 20,000-person city generates in < 1 s in release; stored notables are bounded by the tier range, not the population [56].
11. Integration: for the MICRO seed-42 end-to-end village (integration plan), every NPC's `home_building` and `workplace_building` is a building of its town plan [45].

## Outcomes & side effects

`Population` and `Npc` values (serde). `examples/town.rs` writes `out/npc/town.json`. No other files.

## Dimensions not in play

- Schedules beyond the two token times (`day`, `night`, 12 §scene-tokens).
- Migration, births and deaths over time.
- Non-SRD content of any kind, including feats, fighting styles other than Defense, multiclassing and non-SRD subclasses.
