# arda-npc

Deterministic NPC populations for Arda settlements. Every inhabitant is a
stable individual with a name, ancestry, age, home, job tied to a building,
personality, relationships and a full SRD 5.1 sheet. Given the same world
seed and inputs, the same id is always the same person.

The crate is a pure library. It doesn't read the world pipeline: another
stage will describe settlements and buildings, and this crate turns those
descriptions into people. Until that stage exists, `sample` builds plausible
buildings for any tier.

```sh
cargo run -p arda-npc --example town   # ~1,500-person market town → out/npc/town.json
cargo test -p arda-npc
```

## Inputs

`SettlementProfile`

| Field | Meaning |
| --- | --- |
| `id`, `name` | Settlement id (an `arda-ids` u64, a JSON string; part of every seed key) and display name |
| `tier` | `hamlet`, `village`, `town` or `city` |
| `population` | Inhabitants to generate. It must fit the buildings' capacity; too little housing is an error and never trimmed |
| `functions` | Any of farming, pastoral, fishing, port, market, mining, logging, crafting, fortress, abbey, crossing, capital |
| `wealth` | 0–255 |
| `culture` | Key into `data/content/cultures.json` (names and ancestry mix). Unknown keys fall back to `heartland`, which is human-majority |
| `realm_id`, `biome`, `coastal`, `riverine` | Context: a u64 realm id (JSON string) and one of the nine `arda-settle` biomes (`temperate`, `warm_temperate`, `temperate_forest`, `boreal_forest`, `highland`, `alpine`, `wetland`, `steppe`, `coastal`). `coastal`/`riverine` allow fishing where no fishing function is set. An `arda-settle` settlement record deserialises into a profile directly |
| `ancestry_mix` | Optional per-settlement ancestry weights that replace the culture's table |

`BuildingSpec`

| Field | Meaning |
| --- | --- |
| `id`, `settlement_id` | Building id (u64, JSON string), unique in the settlement, and its owner |
| `function` | A plain snake_case `arda_ids::BuildingFunction`: house, farmhouse, cottage, inn, tavern, smithy, temple, shrine, mill, market_hall, stall, warehouse, dock, boathouse, keep, barracks, guardhouse, manor, workshop, mine, lumber_camp, stable, bakery, brewery, tannery, apothecary, library, school (market, barn, street and farm are accepted and have no staff) |
| `tags` | Optional free tags. A workshop's craft is `craft:<key>` (carpentry, weaving, pottery, cobbling, masonry, jewellery, glassblowing, leatherwork, woodcarving, tinkering, painting, cartography); an untagged workshop draws one from its building seed key |
| `capacity` | Residents it can house |
| `workplace_slots` | People who work there, master included |
| `wealth` | 0–255 |

## Outputs

```rust
generate_population(world_seed, &profile, &buildings) -> Result<Population, NpcError>
npc(world_seed, &profile, &buildings, NpcId) -> Result<Npc, NpcError>      // anyone, identically
commoner(world_seed, &profile, &buildings, building_id, index) -> Result<Npc, NpcError>
Generator::locate(id) -> Option<(BuildingId, u32)>                           // id -> home and index
Generator::new(world_seed, &profile, &buildings)?.npc(id)                   // many people, one build
```

`Population` holds the settlement id, the head count, every `Household`
(home, family name, members, spouse, parent/child, sibling and lodger ties),
the stored notables as full `Npc` records, `commoner_count`, and a compact
roster (id, job, workplace; a few bytes a person, in home-building then
resident order) behind the queries
`residents(building)`, `workers(building)`, `by_job(key)`, `job_of(id)`,
`notables()`, `job_counts()` and `category_counts()`.

`NpcId` is the `arda-ids` u64 `NpcId::from_parts(settlement, home building,
resident index)` (a BLAKE3 subseed), serialised as a JSON string.

`Npc` has:

- an id (`settlement`, home `building`, `index` in that building);
- a name (given, family, optional byname);
- the SRD race and subrace, age and sex;
- a job (key, title, category);
- a workplace building, home building, household and employer;
- social rank, wealth and SRD lifestyle;
- a personality: two traits on different axes, an ideal with an alignment
  lean, a bond, a flaw, a mannerism and a one-line summary;
- relationships: family, employer and employees, one friend and one rival.
  They are always symmetric: every tie appears on both people, with the
  inverse kind;
- a `Sheet`;
- `notable`.

Everything derives serde `Serialize` and `Deserialize`. `text::sheet_text`
renders a readable sheet.

## Sheets

- **Commoners and staff** use an SRD NPC stat block exactly as printed:
  Commoner, Guard (watch, men-at-arms, soldiers), Veteran (sergeants),
  Knight, Acolyte, Priest, Noble (a lord's family), Thug, Scout, Druid or
  Mage, depending on the job. Following the SRD appendix's note on racial
  traits, the race adds size, a slower speed, darkvision, languages,
  resistances and trait names. The block's numbers are not changed.
- **Notables** get SRD class levels and a subclass once the level allows it.
  The class is drawn from the job's weights; a smith is usually a fighter and
  a high priest a cleric. Levels scale with tier: hamlet notables 1–2
  (leaders 2–3), villages 1–2 (2–3), towns 1–4 with leaders 5–8, and cities
  2–6 with elites 8–12. About 2% get three extra levels.
- **Class builds:**
  - Abilities use the standard array in class priority order, plus racial
    bonuses (half-elves put their floating +1s on the class's best
    non-Charisma scores), plus Ability Score Improvements.
  - Hit points are the hit die maximum at level 1, then `die / 2 + 1` per
    level, plus Con, plus hill-dwarf or draconic bonuses.
  - Armour class is computed from SRD armour, capped Dex, a shield, the
    Defense fighting style, or unarmoured defence.
  - Attacks come from SRD weapons with the proficiency and ability rules,
    including finesse and monk weapons.
  - Saves and skills come from the class, an original background, the race
    and expertise.
  - Casters get the SRD slot table (full, half or Pact Magic), a save DC, an
    attack bonus and a few SRD spells from their class list.
  - Equipment comes from the job and the wealth band.

The tests recompute every one of these numbers from the raw JSON.

## Occupations

Jobs attach to buildings, in this order:

1. **Residents staff their own building.** The first seasoned resident
   becomes the master, the master's family takes the building's family job
   if it has one (noble kin at a manor or keep), and other residents fill
   the remaining slots.
2. **The pool fills the other slots.** Everyone else of working age, in a
   seeded order, fills the remaining slots. Masters are always drawn from
   seasoned adults.
3. **The tier appoints its officials:** a reeve in a village; a mayor,
   magistrate and tax collector in a town; more in a city; a herald in a
   capital.
4. **Everyone left takes a land job.** The weights come from
   `occupations.json`, from the settlement's functions and from the tier's
   rural or urban factor. Family members often share the head's work.
   Children and elders get the nominal jobs "Child" and "Retired".

With the sample buildings, farming is about 68% of working adults in hamlets
and villages, 10–30% in towns (the tests allow 5–35%) and about 1% in a
city. Port towns are 30–45% maritime. The samples place one smithy per
~450 people, and every inn, temple and smithy gets its master, so there is
one innkeeper per inn and one high priest per temple.

**Notables** are the masters of notable workplaces plus the officials.
`logic/06` step 5 gives each tier a range: hamlet 2–4, village 6–12,
town 20–60, city 60–200. Above the range, the least important masters are
demoted to their job's stat block. Below it, household heads are promoted to
headman, elder or yeoman.

## Regeneration scheme

Only notables are stored; any commoner is rebuilt on demand. Every random
draw comes from a stream keyed by
`blake3("arda-npc/v1", world_seed, settlement_id, building_id, index, purpose)`,
fed into a hand-rolled xoshiro256**. Nothing depends on input order: the
buildings are sorted by id first.

1. **Skeleton** (`plan/`). Rebuilt in full every time; it is cheap, about
   65 ms for 20,000 people in release:
   - the population is shared out over buildings by capacity (largest
     remainder);
   - households come from each building's own key;
   - jobs are assigned by the global but deterministic pass above;
   - friends and rivals are paired from a seeded shuffle of all adults, so
     every tie is mutual by construction.
2. **Expansion** (`plan/person.rs`). One person's name, wealth,
   personality and sheet come only from their own key and their household's
   key. Expanding person A never changes person B.

So `npc(seed, settlement, buildings, id)` rebuilds the skeleton and expands
one person. It returns exactly the record `Generator::npc` gives inside a
full run, and exactly the stored copy for notables. The tests assert this
for every notable and for samples of commoners.

Ages are in each ancestry's own years. Parents are at least
`max(16, adult_age × 3/4)` years older than their children, nobody passes
the ancestry's maximum age, and a human–elf couple has half-elf children.

## Data and licensing

- `data/srd/` holds SRD 5.1 content only: races with their SRD subraces,
  the 12 classes with their SRD subclasses, slot tables, spells (name, level
  and classes), the 21 NPC stat blocks, equipment and languages. It is used
  under CC-BY-4.0; `data/srd/SOURCE.md` and the root `NOTICE` carry the
  attribution.
- `data/content/` is original to Arda: personality tables, occupations,
  cultures, tiers and flavour backgrounds. None of it comes from the PHB,
  the DMG or published name lists.
- Names come from `arda-names` (I18): people speak the settlement's
  `tongue` (the culture's language with the dialect at the settlement,
  recorded by `arda-settle`), or the culture's standard language when the
  profile has none; dwarves, elves, halflings, gnomes, dragonborn,
  half-orcs and tieflings are named in their own ancestry's tongue.
- Notable slots: `Generator::with_notables` binds `arda-society`'s role
  slots (`NotableSlot`) to inhabitants. Each holder is a stored notable
  with the slot's title, and takes the given name, family name and sex
  history fixed; a household headed by a holder shares the family name.
- The dependencies are arda-ids and arda-names (workspace crates), blake3 (CC0-1.0 or Apache-2.0), serde and serde_json
  (MIT or Apache-2.0), and thiserror (MIT or Apache-2.0). All of them allow
  commercial use.

## Limits and open points

- Commoner stat blocks keep the SRD numbers, so a halfling Guard still has
  Str 13. Only speed, senses, languages and resistances follow the race.
- Dragonborn breath weapons and draconic ancestry colours are listed as
  traits, not modelled as attacks.
- Feats, fighting styles other than Defense, and multiclassing aren't
  modelled.
- The notable minimum is met by promotion only when there are enough mature
  household heads. A tiny hamlet with fewer than two can fall short.
- The sample building generator is a stand-in for the future building stage.
