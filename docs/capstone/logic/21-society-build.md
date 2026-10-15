---
generated_at_commit: b6cae7350f6b
generated_date: 2026-10-13
capstone_version: 7.0.1
content_hash: a69c698c1c91
paths_covered:
  - ":(top)crates/arda-people/**"
  - ":(top)crates/arda-cli/src/society.rs"
  - ":(top)crates/arda-cli/src/main.rs"
  - ":(top)crates/arda-society/src/output.rs"
  - ":(top)crates/arda-society/src/read.rs"
  - ":(top)crates/arda-society/src/lib.rs"
  - ":(top)crates/arda-society/src/ctx.rs"
  - ":(top)crates/arda-town/src/plan/spec.rs"
  - ":(top)crates/arda-settle/src/output.rs"
  - ":(top)crates/arda-settle/src/lib.rs"
  - ":(top)crates/arda-npc/src/lib.rs"
  - ":(top)crates/arda-npc/src/population.rs"
  - ":(top)crates/arda-server/src/lib.rs"
  - ":(top)crates/arda-server/src/people.rs"
  - ":(top)crates/arda-server/tests/world_api/support.rs"
  - ":(top)tests/e2e_village.rs"
  - ":(top)crates/arda-settle/src/gazetteer.rs"
  - ":(top)crates/arda-server/src/realms.rs"
  - ":(top)crates/arda-society/src/families.rs"
---

# 21 — Society build: planning, simulation and stored notables over a settled world

> Descriptive extraction of current behavior at commit 022ce7cb5707, not a normative design. It owns the entry point `arda society build --world <dir>` (`SocietyCommand::Build`, `crates/arda-cli/src/society.rs:20`) and the library calls `arda_people::build` and `arda_people::compute` (`crates/arda-people/src/build.rs:45`, `:58`): the orchestration that reads the settlement stage's files ([08](08-settlements-roads-realms.md)), draws town plans ([10](10-town-layout.md)), runs `arda_society::simulate_society` ([14](14-society-history.md); the trigger text of 14 names `arda-society generate`, which does not exist; this command is the real trigger), generates notables ([13](13-npc-population.md)) and writes `society/society.json` and `society/notables.json`. The internal rules of 08, 10, 13 and 14 are cited, not restated.

Rule names below are descriptive labels for observed behavior; code may cite them as `// logic/21 §<rule>`.

## Trigger & preconditions

- Trigger: `arda society build --world <dir>`, parsed by clap as `Command::Society { command: SocietyCommand }` (`crates/arda-cli/src/main.rs:144-147`) and dispatched to `society::run` (`crates/arda-cli/src/main.rs:402`), which calls `arda_people::build(&world)` (`crates/arda-cli/src/society.rs:43-45`). `--world` is the only argument (`crates/arda-cli/src/society.rs:21-23`).
- Other callers of `arda_people::build`: the test fixtures `tests/e2e_village.rs:96`, `tests/zoom_continuity.rs:79`, `crates/arda-server/tests/world_api/support.rs:185`. `arda_people::compute` (no writes) is called by `crates/arda-people/tests/notable_mix.rs:69`.
- Preconditions (checked by the first failing read, not up front):
  | Precondition | Checked at |
  |---|---|
  | `<dir>` loads as an `arda::World` (manifest, cells, fine terrain when declared) | `crates/arda-people/src/shared.rs:25-30` via `crates/arda-people/src/build.rs:59` |
  | `<dir>/society/settlements.json` and `roads.json` exist and parse as `arda_settle::output::SettlementsFile` / `RoadsFile` | `crates/arda-people/src/files.rs:96-105` via `crates/arda-people/src/build.rs:61` |
  | `settlements.json` has a top-level `settlements` array | `crates/arda-people/src/files.rs:100-104` |
  | `settlements.json`, `roads.json`, `realms.json` have `format_version` 1, each at most 1 GiB | `crates/arda-society/src/read.rs:20`, `:46-50`, `:56-65`, `:75-86` via `crates/arda-people/src/build.rs:76` (after all planning) |
- The CLI help names `arda settle` as the prerequisite (`crates/arda-cli/src/society.rs:17-19`); the CLI enforces nothing beyond the reads above.
- Authority: none. Any process that can read and write `<dir>/society/` runs it; no role, lock or ownership check exists in `crates/arda-cli/src/society.rs` or `crates/arda-people/src/`.

## Rules

### §build-inputs

`compute` reads, in this order:

| Read | What | Where |
|---|---|---|
| `SharedSource::open(world_dir)` | the world as an `arda_refine::WorldSource` behind an `Arc` | `crates/arda-people/src/build.rs:59`, `crates/arda-people/src/shared.rs:25-30` |
| `SocietyFiles::read(<world>/society)` | `settlements.json` typed (`SettlementsFile`), the same file again as raw JSON records (adapter A2), `roads.json` typed (`RoadsFile`), `realms.json` typed (`RealmsFile`) and `gazetteer.json` typed (`GazetteerFile`) each only when the file exists (`SocietyFiles.realms`, `.gazetteer`: `None` when absent, `crates/arda-people/src/files.rs:58-61`, `:106-117`), and an id → index map | `crates/arda-people/src/build.rs:61`, `crates/arda-people/src/files.rs:96-133` |
| `WorldSettlements::read_dir(<world>/society)` | `settlements.json`, `roads.json`, `realms.json` a third/second time, version-checked | `crates/arda-people/src/build.rs:76`, `crates/arda-society/src/read.rs:75-94` |

`settlements.json` is therefore read three times and `roads.json` twice, and `realms.json` twice when present (optionally by `SocietyFiles::read`, required by `read_dir`); nothing ties the reads to one snapshot of the files. `compute` does not use `SocietyFiles.realms` or `.gazetteer`: they serve `arda_people::World` readers (`SocietyFiles::realm`, `crates/arda-people/src/files.rs:153-157`, used by `crates/arda-server/src/realms.rs:99`; the gazetteer by `crates/arda-server/src/lib.rs:257-263`, label tiles, logic/17 §labels). An existing `realms.json` or `gazetteer.json` that does not parse fails `SocietyFiles::read` with `PeopleError::Format` at step 3, before planning; a missing `gazetteer.json` is not an error. Other `society/` files written by 08 (`names.json`, `landuse.bin`, `realms.bin`, `roads.bin`, `stats.json`) are not read by `crates/arda-people/src/build.rs`.

### §build-seed

The seed for planning, simulation and notables is the world source's seed (`src.seed()`, `crates/arda-people/src/build.rs:62`; `crates/arda-people/src/shared.rs:34-36`), passed to `arda_town::generate` (`crates/arda-people/src/town.rs:136`), `simulate_society` (`crates/arda-people/src/build.rs:78`) and `Generator::with_notables` (`crates/arda-people/src/build.rs:94`). The `seed` field of `settlements.json` (`crates/arda-settle/src/output.rs:34`) is parsed but never compared with it.

### §build-plan-all

`plan_all` (`crates/arda-people/src/build.rs:122-164`) plans every settlement in parallel:

- Thread budget `available_parallelism()`, 4 when unknown (`crates/arda-people/src/build.rs:126`).
- Settlements are processed in chunks of `threads × 4`; each settlement in a chunk gets its own scoped thread (`crates/arda-people/src/build.rs:134-149`). A chunk completes before the next starts.
- Results are collected per chunk in settlement order (`crates/arda-people/src/build.rs:150-161`), so output order equals `settlements.json` order regardless of scheduling (doc comment `crates/arda-people/src/build.rs:119-121`).
- A panicking planner thread becomes `PeopleError::World("a planner thread panicked")` (`crates/arda-people/src/build.rs:153-155`).
- The first `Err` in settlement order within a chunk is returned after the whole chunk has joined; later chunks are not started (`crates/arda-people/src/build.rs:159-161`).
- Per settlement, a successful plan yields two building lists: `town::society_buildings` (for arda-society, adapter A14) and `town::plan_specs(&p, s.population)` (for arda-npc, adapter A4) (`crates/arda-people/src/build.rs:140-145`).

### §build-plan-site

`town::plan` (`crates/arda-people/src/town.rs:124-137`) for one settlement:

1. `population == 0` → `Ok(None)` without sampling terrain (`crates/arda-people/src/town.rs:130-132`).
2. Terrain within `plan_radius_for(settlement)`: the tier's `plan_radius_m` (hamlet 700 m, village 1 200 m, town 2 000 m, city 2 600 m), or `12·√population` m when larger, so a city above about 48,000 sees ground for its people (`crates/arda-people/src/town.rs`, review 3.3-11), sampled by `town::terrain` (`crates/arda-people/src/town.rs:99-117`): world cells and refined river pieces within radius + 300 m (`crates/arda-people/src/town.rs:105-106`; `crates/arda-people/src/terrain.rs:55-70`; `crates/arda-people/src/rivers.rs:22-37`), a 5 m height field (`crates/arda-people/src/town.rs:113`), and road stretches from `roads.json` within the radius plus one vertex of margin (`crates/arda-people/src/town.rs:57-90`, `:115`).
3. The raw record deserialises into `arda_town::TownSite`; a mismatch is `PeopleError::Format` naming the bare path `settlements.json` (`crates/arda-people/src/town.rs:33-36`).
4. `arda_town::generate(site, terrain, seed).ok()`, kept only when `town::usable` holds: the plan's own buildings house at least half the population (`crates/arda-people/src/town.rs`). A planner refusal or an under-housing plan becomes `None` (the error is discarded) and the settlement takes the mix path (review 3.3-11). Planner rules: 10.

`compute` does not call `town::read_history` (`crates/arda-people/src/town.rs:146-164`) on its plans; `World::plan` does (`crates/arda-people/src/people.rs:148-150`). `read_history` changes only `plan.fabric` (`crates/arda-people/src/town.rs:154-163`), which `arda_town::plan::spec::specs` does not read (`crates/arda-town/src/plan/spec.rs:77`; no `fabric` reference in that file), so the building specs are the same on both paths.

### §build-simulate

- The plan buildings of every planned settlement, as `arda_society::input::BuildingSpec` (`crates/arda-people/src/town.rs:220-230`), are concatenated in settlement order (`crates/arda-people/src/build.rs:70-72`) and replace `WorldSettlements::buildings` (`crates/arda-people/src/build.rs:77`; `read_dir` leaves it empty, `crates/arda-society/src/read.rs:92`).
- Settlements without a plan contribute no explicit buildings; arda-society derives theirs from the building mix (`crates/arda-society/src/read.rs:68-70`; rules: 14).
- `arda_society::simulate_society(seed, &world)` (`crates/arda-people/src/build.rs:78`; `crates/arda-society/src/lib.rs:136`) produces `Society`, whose `settlements` follow input settlement order (`crates/arda-society/src/lib.rs:212-223`, `crates/arda-society/src/ctx.rs:152-154`). Simulation rules: 14.

### §build-buildings

Per settlement, the building list handed to the NPC generator (`crates/arda-people/src/build.rs:87-90`):

| Source | Condition | Builder | `StoredNotable.buildings` |
|---|---|---|---|
| `BuildingSource::Plan` | `plan_all` returned a plan | `town::plan_specs` (`crates/arda-people/src/town.rs:175-179`) | `"plan"` |
| `BuildingSource::Mix` | no plan (population 0 or planner refusal) | `town::mix_specs(s, &soc.buildings)` over the building refs arda-society assigned (`crates/arda-people/src/town.rs:237-270`) | `"mix"` |

(`BuildingSource::key`, `crates/arda-people/src/people.rs:32-37`.)

- `plan_specs` keeps plan buildings whose `arda_town::BuildingFunction` key maps to an `arda_ids::BuildingFunction`; others are dropped silently (`crates/arda-people/src/town.rs:200-215`, `:207`).
- `mix_specs` drops refs whose function key is unknown (`crates/arda-people/src/town.rs:264`); a known key with no `arda_town::BuildingFunction` counterpart gets capacity 0 and 0 workplace slots (`crates/arda-people/src/town.rs:243-250`); wealth is the settlement's (`crates/arda-people/src/town.rs:258`).
- Both pad housing (`pad_housing`): while summed capacity < population, append a `Cottage` tagged `off_plan` with the next id after the maximum; padded cottages take the settlement's wealth on both paths (`town::plan_specs(plan, population, wealth)`, `town::mix_specs`; review 3.3-11).

### §build-notables

Per settlement, in settlement order (`crates/arda-people/src/build.rs:80-102`):

1. The raw record deserialises into `arda_npc::SettlementProfile`; a mismatch is `PeopleError::Format` naming `<world>/society/settlements.json` (`crates/arda-people/src/build.rs:91-92`).
2. Society's `roles` of that settlement become `NotableSlot`s via `slot` (`crates/arda-people/src/build.rs:93`; `crates/arda-people/src/people.rs:281-291`).
3. `Generator::with_notables(seed, &profile, &buildings, &slots)?.population()?` (`crates/arda-people/src/build.rs:94`; `crates/arda-npc/src/lib.rs:76-85`). Generation rules: 13.
4. `pop.npcs` (stored notables, `crates/arda-npc/src/population.rs:58-59`) becomes a `StoredNotable { settlement_id, buildings, plan_fingerprint, npcs }` (`crates/arda-people/src/build.rs`; `crates/arda-people/src/files.rs`); `plan_fingerprint` is `town::fingerprint` of the plan building specs (32 hex digits of a digest), `None` on the mix path (review 3.3-5); households, roster and commoners are discarded (goal 56).
5. `society.settle_stamp` and `notables.settle_stamp` take the `id` of `settlements.json`'s generation stamp (08 Outcomes; review 3.3-4), `None` for an unstamped file.

The three sequences (`settlements`, raw `records`, plans × `society.settlements`) are combined with `zip` (`crates/arda-people/src/build.rs:80-85`); no length check exists, so a length mismatch truncates silently to the shortest.

### §build-write

`build` (`crates/arda-people/src/build.rs:45-51`) calls `compute`, then writes two files into `<world>/society/`, in this order:

| File | Writer | Format | Trailing newline |
|---|---|---|---|
| `society.json` (`SOCIETY_FILE`, `crates/arda-people/src/files.rs:16`) | `arda_society::output::write_json` (`crates/arda-people/src/build.rs:48`; `crates/arda-society/src/output.rs:20-30`): `create_dir_all(parent)`, then `std::fs::write` of `serde_json::to_string_pretty` (`crates/arda-society/src/output.rs:12-14`) | `Society`, `format_version` from arda-society (`crates/arda-society/src/lib.rs:114`, `:241`) | no |
| `notables.json` (`NOTABLES_FILE`) | `notables::encode` then `std::fs::write` (`crates/arda-people/src/build.rs`, `crates/arda-people/src/notables.rs`) | `NotablesFile { format_version: 2, seed: <decimal string>, settle_stamp, index, settlements }`, written as one header line (everything but `settlements`, whose array is opened) followed by one compact settlement per line and a closing `]}`: still one JSON document. `index` holds, per settlement, its notable ids, `plan_fingerprint` and the byte range of its line after the header (review 3.3-10) | yes |

- Neither write is atomic: no temp file, no rename, no fsync (`crates/arda-society/src/output.rs:29`, `crates/arda-people/src/files.rs:88`).
- Existing files are overwritten (truncated by `std::fs::write`) without backup or a prior-existence check.
- Nothing is written until `compute` has succeeded for every settlement (`crates/arda-people/src/build.rs:46`).

### §build-report

`BuildReport` (`crates/arda-people/src/build.rs:18-27`):

| Field | Value | Set at |
|---|---|---|
| `settlements` | number of records in `settlements.json` | `crates/arda-people/src/build.rs:64` |
| `planned` | settlements for which `plan_all` returned a plan | `crates/arda-people/src/build.rs:72` |
| `notables` | Σ `pop.npcs.len()` | `crates/arda-people/src/build.rs:95` |
| `population` | Σ `pop.population` (stored and regenerated inhabitants) | `crates/arda-people/src/build.rs:96` |

The CLI prints one line on success: `wrote <world>/society: <settlements> settlements (<planned> with town plans), <population> inhabitants, <notables> stored notables` (`crates/arda-cli/src/society.rs:46-53`). On failure the error carries the context `building the society of <world>` (`crates/arda-cli/src/society.rs:45`) and propagates out of `main` (`crates/arda-cli/src/main.rs:399`), so the process exits non-zero with anyhow's error chain.

### §build-read-path

Pointer only. `arda_people::World` (`crates/arda-people/src/people.rs:43-55`) is the read side of these files:

- `World::with_source` reads `SocietyFiles`, then `society.json` and `notables.json` only if they exist; either may be absent. `notables.json` is opened lazily (`Notables::open`): the header line is parsed, and a settlement's line the first time it is asked for; format-1 files are read whole. `check_stamps` then refuses (`PeopleError::Stale`, naming the file and the command to run) settlement files whose stamp's `world_json` is not the digest of this world's `world.json`, and a `society.json` or `notables.json` whose `settle_stamp` is not the settlement files' stamp id; unstamped files pass only beside unstamped settlement files (review 3.3-4).
- `World::plan` draws plans on demand with `read_history` applied and an LRU-by-tick cache of `PLAN_CACHE` = 64, and compares each plan's `town::fingerprint` with the stored `plan_fingerprint`; a mismatch marks that settlement's notables stale, and `try_notables_of` / `try_notable` then fail with `PeopleError::Stale` (`notables_of` / `notable` return nothing) (review 3.3-5). `World::buildings` and `slots` redo §build-buildings and §build-notables for one settlement; `World::generator` builds the population skeleton once (an owned `arda_npc::Generator`) and keeps the 16 most recently used, so `population` and the server's commoner lookups do not rebuild it per call (review 3.3-10).
- A plan-less settlement without `society.json` fails with `society/society.json is missing: run \`arda society build\`` (`crates/arda-people/src/people.rs:207-215`); `slots` returns empty instead (`crates/arda-people/src/people.rs:240-246`).
- The server opens `World::with_source` when `society/settlements.json` exists, so stale files stop it at startup with the stale message, and warms town and city plans; routes reach it through `people()` and map errors with `people_error` (`PeopleError::Npc` keeps the generator's status); `GET /v1/settlements/{id}/npcs` answers an error rather than stale notables; the NPC resolver reads `World::generator`. Server rules: [16](16-service-api.md). Shallow: server routes and `npcs/` not inventoried here.

## Steps

1. Parse `--world` and call `arda_people::build` (§build-report; `crates/arda-cli/src/society.rs:43-45`).
2. Open the world source (§build-inputs; `crates/arda-people/src/build.rs:59`).
3. Read `settlements.json` (typed and raw) and `roads.json` (§build-inputs; `crates/arda-people/src/build.rs:61`).
4. Take the seed from the world (§build-seed; `crates/arda-people/src/build.rs:62`).
5. Plan every settlement in parallel chunks (§build-plan-all, §build-plan-site; `crates/arda-people/src/build.rs:69-75`).
6. Read `settlements.json`, `roads.json`, `realms.json` for arda-society with version checks; inject the plan buildings (§build-simulate; `crates/arda-people/src/build.rs:76-77`).
7. Simulate society (§build-simulate; `crates/arda-people/src/build.rs:78`).
8. Per settlement: choose plan or mix buildings, build the profile and slots, generate the population, keep its notables (§build-buildings, §build-notables; `crates/arda-people/src/build.rs:80-102`).
9. Assemble `Built { report, society, notables }` (`crates/arda-people/src/build.rs:103-111`).
10. Write `society.json`, then `notables.json` (§build-write; `crates/arda-people/src/build.rs:48-49`).
11. Print the report line (§build-report; `crates/arda-cli/src/society.rs:46-53`).

## Branches

| Branch | Condition | Effect | Where |
|---|---|---|---|
| No plan, empty settlement | `population == 0` | `None`; mix buildings; padding adds nothing for population 0 | `crates/arda-people/src/town.rs:130-132`, `:185` |
| No plan, planner refusal | `arda_town::generate` returns `Err` | `None`; error discarded; mix buildings | `crates/arda-people/src/town.rs` |
| No plan, under-housing plan | the plan's buildings house under half the population (`town::usable`) | `None`; mix buildings | `crates/arda-people/src/town.rs` |
| Plan | `arda_town::generate` returns `Ok` | plan buildings to society and NPC generator; `planned += 1` | `crates/arda-people/src/build.rs:70-73`, `:88` |
| Under-housed list | capacity sum < population | off-plan cottages appended | `crates/arda-people/src/town.rs:181-198` |
| `compute` vs `build` | caller | `compute` writes nothing (`crates/arda-people/src/build.rs:53-54`); `build` writes both files | `crates/arda-people/src/build.rs:45-51` |
| Thread count unknown | `available_parallelism` errors | 4 threads assumed, chunk 16 | `crates/arda-people/src/build.rs:126`, `:134` |

## Unhappy paths

| Failure | Result | Files written | Where |
|---|---|---|---|
| World does not load | `PeopleError::World(<msg>)` | none | `crates/arda-people/src/shared.rs:26-28` |
| `settlements.json` / `roads.json` missing or unreadable | `PeopleError::Io { path }` | none | `crates/arda-people/src/files.rs:67`, `:76` |
| Either file invalid JSON or wrong shape; no `settlements` array | `PeopleError::Format { path }` | none | `crates/arda-people/src/files.rs:68`, `:77`, `:104` |
| A record does not fit `TownSite` | `PeopleError::Format` (path `settlements.json`), raised inside a planner thread | none | `crates/arda-people/src/town.rs:33-36` |
| Terrain or river layer fails during planning | `PeopleError::World` | none | `crates/arda-people/src/terrain.rs:71`, `crates/arda-people/src/rivers.rs:22-37` |
| Planner thread panics | `PeopleError::World("a planner thread panicked")` | none | `crates/arda-people/src/build.rs:153-155` |
| `realms.json` missing (an unparseable one fails earlier, at `SocietyFiles::read`, as `PeopleError::Format`), any input over 1 GiB, or any `format_version` ≠ 1 | `PeopleError::Society(SocietyError::Io/Input)`, detected only after every settlement has been planned | none | `crates/arda-society/src/read.rs:40-65`, `:75-86`; `crates/arda-people/src/lib.rs:63-64` |
| Simulation refuses input | `PeopleError::Society` | none | `crates/arda-people/src/build.rs:78` |
| Record does not fit `SettlementProfile` | `PeopleError::Format` (full `settlements.json` path) | none | `crates/arda-people/src/build.rs:91-92` |
| NPC generator refuses | `PeopleError::Npc` | none | `crates/arda-people/src/build.rs:94`; `crates/arda-people/src/lib.rs:66-67` |
| Writing `society.json` fails | `PeopleError::Society(SocietyError::Io)`; `notables.json` keeps its previous content (or stays absent) | possibly a truncated/partial `society.json` | `crates/arda-society/src/output.rs:22-29` |
| Writing `notables.json` fails | `PeopleError::Io` or `Format` | new `society.json` beside an old, missing or partial `notables.json` | `crates/arda-people/src/files.rs:86-88` |
| Process killed or interrupted mid-write | no handler | partial file possible | not implemented in `crates/arda-cli/src/society.rs` or `crates/arda-people/src/` |

Absent:

- Retry: not implemented anywhere in `crates/arda-people/src/`.
- Rollback or compensation of a half-written pair: not implemented (`crates/arda-people/src/build.rs:48-49`).
- Stale-output detection at build time: `arda society build` does not refuse to run over files it would make stale; detection is on read (§build-read-path, `check_stamps`). 14's `inputs_hash` / `stale_society` is not implemented on this path; the 08 generation stamp stands in for it.
- `NotablesFile.format_version` selects the reader (an indexed header or a whole document) but is not otherwise checked.
- Concurrent runs: no lock file or advisory lock; two builds on one world write the same paths, last writer per file wins (`crates/arda-people/src/build.rs:48-49`).
- Early version check: `format_version` of the 08 files is checked only by `read_dir` at step 6, after planning (`crates/arda-people/src/build.rs:69-76`).
- Planner refusal reason: discarded, not logged or reported (`crates/arda-people/src/town.rs:136`); the report shows only the `planned` count.
- Progress output during planning or simulation: none (`crates/arda-cli/src/society.rs:43-54`).

## State transitions

| Entity | Before | After success | After failure |
|---|---|---|---|
| `<world>/society/society.json` | absent or any prior content | replaced by this run's `Society` | unchanged if failure precedes step 10; possibly partial if its write fails |
| `<world>/society/notables.json` | absent or any prior content | replaced by this run's `NotablesFile` | unchanged unless its own write fails (then possibly partial) |
| 08 files (`settlements.json`, `roads.json`, `realms.json`, others) | written by `arda settle` | untouched (read only) | untouched |
| World directory outside `society/` | — | untouched | untouched |

Reversible only by re-running the command; no history of previous outputs is kept.

## Invariants

Observable in arda-people and the CLI:

1. Settlement order: `notables.json` `settlements[i].settlement_id` equals `settlements.json` `settlements[i].id` (`crates/arda-people/src/build.rs:80-101`); asserted by `crates/arda-people/tests/notable_mix.rs:73-80` over `compute`.
2. Scheduling independence: plan results keep settlement order whatever the thread count (`crates/arda-people/src/build.rs:119-121`, `:150-161`). Plan determinism per settlement is asserted by `crates/arda-people/tests/rivers.rs:96-100` (same plan JSON twice).
3. Determinism for equal inputs: all randomness derives from the world seed (§build-seed); `notables.json` is byte-identical for equal values (`crates/arda-people/src/files.rs:80-85`). No test in `crates/arda-people/tests/` compares two `build` outputs byte for byte.
4. Every inhabitant is housed: the building list given to the NPC generator has capacity ≥ population on both paths (`crates/arda-people/src/town.rs:185`; 10 §town-capacity), except where every appended cottage has capacity 0 (the loop counts `capacity.max(1)`, `crates/arda-people/src/town.rs:195`).
5. Only notables are stored: households, roster and commoner counts are not written (`crates/arda-people/src/build.rs:97-101`).
6. `planned ≤ settlements` and each `StoredNotable.buildings` is `plan` exactly for planned settlements (`crates/arda-people/src/build.rs:70-75`, `:87-90`).
7. All-or-nothing compute: either `compute` returns a complete `Built` for every settlement or an error and no file is written; the pair of written files is not atomic (§build-write).
8. The plan buildings society assigns roles to are the same buildings the NPC generator receives (both from `plan_buildings`, `crates/arda-people/src/town.rs:176`, `:221`), so role slots name existing building ids (adapter A14).

Enforced on read (§build-read-path): `society.json` and `notables.json` match the current 08 files (`settle_stamp` equals the stamp id), the 08 files match the world (`world_json`), and a plan drawn again matches the plan the notables were generated in (`plan_fingerprint`).

## Outcomes & side effects

- Success: two files in `<world>/society/` (§build-write) and one stdout line (§build-report); exit status 0.
- Failure: error chain on stderr via anyhow, non-zero exit (`crates/arda-cli/src/main.rs:399`); files as in Unhappy paths.
- Consumers: `arda_people::World` and through it the server (§build-read-path); `tests/e2e_village.rs:308-317` (an `#[ignore]` wall-clock release gate that also prints `GATE e2e_cold_*` timing lines) checks both files have `format_version` 1; `crates/arda-server/tests/world_api/support.rs:160` treats `society/notables.json` together with `society/gazetteer.json` as the readiness marker of a fixture world (`has_society`, `crates/arda-server/tests/world_api/support.rs:159-163`).
- Termination: the command is a single synchronous process; abandonment equals process exit, leaving files as in State transitions.
- Record and audit: the outputs record `seed`, format versions and `settle_stamp` (the 08 stamp id, which digests the world's `world.json`, the settling code's version and every 08 output), and per settlement a `plan_fingerprint`; no timestamp or run log is recorded.
- Temporary resources: up to `threads × 4` scoped OS threads per chunk, all joined before `plan_all` returns (`crates/arda-people/src/build.rs:134-158`); no temporary files.

## Dimensions not in play

- D1 Authority: a single local operator; no actor model, roles or grants in `crates/arda-cli/src/society.rs` or `crates/arda-people/src/`.
- D5 Money: nothing is charged or credited; arda-society's economy is narrative output (14).
- D6 Limits and thresholds: no caps of this command's own; the 1 GiB input limit is arda-society's (`crates/arda-society/src/read.rs:20`), plan radii are set by tier and, for large cities, by population (`town::plan_radius_for`), and the chunk size is a scheduling unit, not a limit.
- D7 Time: no wall clock, deadline or expiry; no timestamps are written (§Outcomes & side effects). Calendar years inside `society.json` are 14's.
- D9 Lifecycle: no entity state machine; the only transitions are file replacements (State transitions).
- D12 Visibility and disclosure: everything computed for stored notables is written; commoners and households are omitted by design for storage (goal 56), not for disclosure; no actor-specific views exist at this layer.
- D13 Notification: nothing is notified; the running server is not signalled and holds whatever it loaded at startup (`crates/arda-server/src/lib.rs:217-231`).
- D14 Effects on others: no shared state beyond the two output files; their consumers are listed in Outcomes & side effects.
