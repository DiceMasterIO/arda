# arda-society

The society layer above individual NPCs: realm politics, economy, factions,
a simulated history whose outcome is the present, campaign hooks, and the
notable NPC slots that `arda-npc` fills. Goal-prompt goals 36, 38 and 51–57;
design in `docs/capstone/logic/06-society-generation.md`.

```rust
let society = arda_society::simulate_society(seed, &world)?;   // deterministic
arda_society::output::write_json(&society, path)?;             // serde JSON
print!("{}", arda_society::gazetteer::render(&society, &world)); // readable text
```

```sh
cargo run -p arda-society --example realm [seed]   # synthetic 3-realm, 40-settlement world
```

The example prints a gazetteer and writes `out/society/realm.json`.

## Input (`input.rs`)

`WorldSettlements { format_version, settlements, roads, realms, buildings }`
mirrors the settlement stage's `society/settlements.json`, `roads.json` and
`realms.json` field for field (goal 04, step 8), so the adapter is a
`serde_json::from_str`.

- **Settlement:** `id`, `name`, `tier`, `population`, `functions`, `wealth`,
  `culture`, `realm_id`, `biome`, `coastal`, `riverine`, `x_m`, `y_m`,
  `cell_x`, `cell_y`, `height_m`, `rank`, `site_tags`, `history`, `buildings`
  (the building mix, function key → count). Enum values match `arda-npc`.
- **Road:** `id`, `class`, `from`, `to`, `to_edge`, `length_m`, `straight_m`,
  `new_m`, `segments`. Roads without a `to` (map-edge exits) are kept but
  carry no trade.
- **Realm:** `id`, `name`, `seat`, `members` (optional). A `realm_id` with no
  realm record gets a derived realm seated at its largest member
  (`logic/06` branch).
- **BuildingSpec** (optional): `id`, `settlement_id`, `function`, `tags`.

These follow the canonical conventions in `docs/goal-prompts/vocabulary.md`:

- settlement, realm and building ids and the seed are `u64`, written as JSON
  strings (numbers are accepted on read);
- road classes are `none | track | road | highway | footpath`, with codes
  0–4 (`RoadClass::code`);
- building functions are plain snake_case keys, and a workshop's craft is the
  tag `craft:<name>`.

**Buildings.** Settlements with explicit `buildings` use them. Every other
settlement derives its buildings from the mix: ids are assigned 1, 2, … in
function-key order, then by index within the key, so the same mix always
yields the same ids. Derived workshops cycle through `workshop_crafts` in
`goods.json`.

Invalid input returns `SocietyError::Input`: duplicate ids, a seat outside
its realm, or a road to an unknown settlement.

## Output (`Society`)

| Field | Contents |
|---|---|
| `realms[]` | style ("the Duchy of …"), government, rank, seat, founding year, members, population, levy, `ruler` (house, regnal name, NPC slot), `vassals[]` (title, house, loyalty 0–100, grievances, claimant flag), `hooks[]` (3–5 region hooks) |
| `relations[]` | one per unordered realm pair (`a < b`): stance `alliance / trade_pact / neutral / rivalry / war`, score, border roads, trade value, war event ids, marriage, reasons |
| `settlements[]` | founding year and parent, buildings, economy ledger and key goods, prosperity curve and trend, factions, faction relations, NPC roles, 2–4 `history_hooks`, 3–5 `hooks` |
| `economy` | per-good world totals, every trade flow (with path and roads), traffic per road |
| `history` | calendar, events, dynasties, reigns, wars, border shifts, ruins, allegiances |

References between entities use the tagged `EntityRef`
(`{"kind":"settlement","id":"12"}`, `role`, `faction`, `road`, `ruin`,
`event`, `dynasty`, `building`, `good`).

**Stable ids.**

| Id | Form | Depends on |
|---|---|---|
| role | `r<settlement>.<kind>` | settlement contents only; the same across seeds |
| faction | `f<settlement>.<kind>` | settlement contents only |
| hook | `h<settlement>.<kind>`, `hr<realm>.<kind>` | settlement or realm, and hook kind |
| dynasty | `d<realm>.<n>` | realm and order of rise |
| event | 1-based, in chronological order | the timeline |
| ruin | 1-based | the timeline |

## Rules

**Economy.**

- *Production.* Land workers (a tier share of the population) are split over
  the settlement's land functions, and yield per 100 workers, adjusted by
  biome. Buildings add fixed yields: smithy, tannery, brewery, mines, lumber
  camps, boats. Workshops yield by craft. Site tags add yields up to a
  1,000-person cap: salt pans, ore, fish, timber.
- *Demand* is per 100 people. It can be urban-only or wealth-scaled, and it
  includes processing inputs: looms want wool, forges want ore, breweries want
  grain, tanners want hides.
- *Trade* is a gravity model over road distance. Each good's surplus is split
  over reachable deficits within that good's `range_km`, weighted
  `deficit × attraction / (km² + 9)`. Markets and ports attract three times,
  towns twice. The split runs for four rounds with a 12-way fan-out.
- *Conservation.* Every load is accounted for:
  `production = local_use + exports + stored` and
  `demand = local_use + imports + shortfall`.
- *Key goods* for each market are the top four goods by value exported,
  imported or carried through.

**History.** The span is 200–500 years, from the seed. Stages run in causal
order:

1. *Founding.* Realm seats and the best site in each road component are the
   roots, founded in the first decades. Every other settlement is founded from
   the road neighbour that reaches it first. The delay shrinks with site
   quality (tier, crossings, confluences, harbours, rivers, coasts) and grows
   with distance. It is a Dijkstra over years, so no settlement predates the
   neighbour that settled it.
2. *Realm formation* follows 12–40 years after the seat's founding.
3. *Wars.* Neighbouring realms go to war with a chance that grows with border
   roads and cultural difference, and falls with mutual trade. Border shifts
   only move a settlement into the realm that holds it today, and only if its
   founders came from the losing realm. The past therefore converges on the
   given partition. `History::replay_partition` replays the shifts from
   `allegiances` and the tests assert that it equals the input exactly, with
   seats never changing hands. A pair may also be at war now.
4. *Disasters.*
   - floods on low river sites (confluence, ford, marsh, and similar);
   - plague spreading along roads from ports and cities;
   - town fires, mine collapses, and realm famines.
5. *Reigns and dynasties.* Reigns end by death, by battle (only while at
   war), by plague (only in a plague year at the seat), or by deposition,
   abdication or a failed line. The last three, and some battle deaths, bring
   a new house to power. Elective governments change house often.
6. *Ruins* lie beside roads. Each is emptied by a real nearby event when one
   fits its lifetime, or else by a local cause (a well failed, raiders,
   enclosure).
7. *Prosperity* climbs from founding towards present wealth, knocked back by
   events. The last sample equals the input wealth.
8. *History hooks:* 2–4 lines per settlement, from its founding, conquest,
   worst disaster, ruling house, nearby ruin and main export.

**Politics.**

- *Government:* hereditary monarchy by default. A trading port seat leans
  merchant republic, abbeys lean theocracy, and four or more towns lean
  elective crown.
- *Rank:* kingdom, duchy or county, from the seat's tier and the member count.
- *Vassals:* every non-seat town, and every village with a manor or keep,
  holds a vassal. Loyalty falls with distance and with a conquest less than a
  century old, and rises with trade with the seat. Half the realms harbour a
  recently ousted house as a claimant vassal.
- *Relations* are scored from trade, culture, marriage, border roads, wars
  (fading after 150 years), captured settlements and recent war.

**Factions** come from buildings and functions: court, merchant and craft
guilds, temple, shrine, abbey, watch, criminals, fishers, miners, woodfolk,
lightermen, commons. Stances start from a pairwise affinity table and shift
with context (tax, conquest, recent hardship). Reasons are written per kind
pair.

**Hooks.** Each tension becomes a scored candidate:

- guild against the lord, dues disputes, feuds, the underworld;
- old banners and old claims, border disputes, front lines, blockades and
  embargoes;
- bandit roads (one per road), shortages, gluts, smugglers;
- ruins, plague aftermath, mines, disloyal vassals, wolves, rising water,
  levies.

The strongest 3–5 distinct kinds are written up. Weak candidates only fill up
to three, in a per-place order.

**NPC slots** (`roles.rs`). Each settlement gets:

- its head (`ruler`, `lord`, `reeve` or `elder`);
- a steward, court mage and spymaster at seats;
- a captain wherever there is a guardhouse, barracks or keep;
- a high priest, prior or shrine keeper;
- a harbourmaster and a mine overseer where there are docks and mines;
- one leader per faction.

Each slot names a building (one not already claimed where possible) and an
SRD 5.1 stat block or class and level, scaled by tier (and by government for
rulers). Rulers and vassals carry their given and family names from the
history.

## Data (`data/*.json`)

`goods.json`, `politics.json`, `factions.json`, `roles.json`,
`history.json`, `hooks.json` and `names.json` are embedded at compile time
and cross-checked by `Tables::load`. All prose, names and syllables are
original to Arda. Stat-block and class names are SRD 5.1 (CC-BY-4.0). The
attribution for them goes in the root `NOTICE`, which the `arda-npc` branch
adds, so the merge must carry it. No other published material is used.

## Determinism and scale

- Every decision is keyed by `(seed, domain, entity)` through blake3, so no
  value depends on generation order.
- Maps are `BTreeMap`s, and the maths is integer.
- The same inputs give byte-identical JSON (tested).
- A 4,000-settlement, 300-realm world simulates in about 1.5 s in release
  (`cargo test --release -p arda-society --test scale -- --ignored`).
- Stored roles stay O(settlements).

## Tests

| File | Covers |
|---|---|
| `determinism` | byte-identical output; JSON round-trip; string ids |
| `economy` | conservation per good and settlement; flows follow real roads within range |
| `relations` | one symmetric record per realm pair; war iff an ongoing war; unique faction pairs |
| `history` | causality; partition replay equals the input; seats fixed; reign continuity; ruins; oldest at the best sites; prosperity endpoint |
| `references` | every `EntityRef` and every role, building, faction, event and dynasty id resolves |
| `hooks` | 3–5 hooks and 2–4 history hooks; no unfilled slots; SRD names only; roles have buildings; role ids independent of seed |
| `input` | settlement-stage JSON loads (string and numeric ids, `none` roads); explicit buildings; refusals |
| `scale` | a tiled 320-settlement world keeps its invariants |

## Open

- The input `Realm` shape mirrors `logic/06`, because `arda-settle` had not
  yet written `realms.json`. Border courses are ignored.
- Rulers' and vassals' names are fixed by history. `arda-npc` should take
  `given_name`, `family_name` and `female` from the slot rather than roll its
  own.
- Relations are all-pairs, O(realms²). That is fine at the continent's
  expected realm count, but large for hundreds of realms.
