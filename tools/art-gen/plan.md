# Art library expansion: batching plan

The prompt pack now holds **1,425 assets**: the first 225 (tier 0, already generated on Codex) and 1,200 new ones in three tiers. This plan says what to generate in which order, how long it takes, and the exact commands. Nothing here has been submitted.

The new assets come from `expansion.mjs`. `build-prompts.mjs` merges them with the checklist and writes `prompts.json`, where every entry carries a `tier` and its catalogue metadata (footprint, layer, height, cover, blocking and tags).

## What is in each tier

| class | tier 0 (done) | tier 1 | tier 2 | tier 3 | total |
|---|---:|---:|---:|---:|---:|
| ground | 67 | 49 | – | – | 116 |
| water | 4 | 4 | – | – | 8 |
| wall | 63 | 70 | – | 38 | 171 |
| prop | 62 | 185 | 358 | 249 | 854 |
| vegetation | 29 | 107 | 109 | 31 | 276 |
| **all** | **225** | **415** | **467** | **318** | **1,425** |

- **Tier 1 (415): every variant of the first 225.** Every ground key gets 3–4 variants and every wall piece gets 2–3 takes (two extra for `run`). Every prop and plant gets at least 3 takes; the frequent ones (barrel, crate, table, chair, bed, chest, trees) get 5–7. These fix the copy-paste look on every map at once, so they come first. Tier 1 is about twice the 200 first planned. The pack puts its highest-impact 200 first, so `--limit 200` gives that core (step 1c).
- **Tier 2 (467): building-function and biome sets.** These are the props each vocabulary function lacks:
  - tavern: round tables, bottle shelves;
  - bakery: bread racks, kneading troughs;
  - brewery: vats, kettles, cask stacks;
  - tannery: tanning racks and vats, hides;
  - apothecary: jar shelves, alchemy tables, herb racks, cauldrons;
  - library and school: desks, lecterns, scroll racks, map tables, pupils' desks;
  - barracks and guardhouse: bunks, footlockers, training dummies, targets, shield racks;
  - stable and barn: saddle racks, hay racks, horses, grain bins, ploughs, tool racks, coops, animals;
  - mine: carts, rails, ore, supports;
  - lumber camp: log and timber piles, sawhorses, chopping blocks;
  - toll house and waystation: hitching posts, toll bars, notice boards, campfires, camp gear;
  - market: display tables;
  - shrine and temple: shrine stones, offering bowls, fonts, votive racks;
  - manor and keep: wealthy table, bed and chest, grand hearth, wardrobes, armchairs, settees, large rugs, feast tables;
  - workshop crafts: potter's wheel, kiln, cobbler, jeweller, tinker, mason, dyer.

  The biome sets add beech, ash, maple, larch, fir, palm, acacia, olive and swamp cypress, snow-laden spruce and pine, cacti, agave, sagebrush and dune grass. Coastal and wetland pieces are seaweed, driftwood, sea rocks, sedge, tussocks and duckweed. Outdoor dressing covers picket and wattle fences, wagons, handcarts, haystacks, stooks, scarecrows, fountains, tombs, rubble, broken columns, standing stones and cairns.
- **Tier 3 (318): culture sets, rare dressing and two new wall kits.** Dwarf, elf and orc pieces (8 or 9 ids each, 3 takes each), crypts, coffins, skeletons, ruins and dolmens. Also siege engines, carriages, barges, rare crafts (glass, carving, painting) and rare animals. The adobe and log wall kits come with their alts.

Within a tier, `prompts.json` lists assets by visual impact:

1. ground;
2. tree canopies;
3. wall runs;
4. frequent props;
5. low plants and rocks;
6. other wall pieces;
7. other props.

Within each group the order goes by round: every first alt comes before any second alt. So `--limit N` always takes the N assets that change maps the most.

## Time

Codex (GPT-6 Astra, thinking high) takes about **2.2 minutes per image**, and Slopify runs batch items one at a time.

| step | images | time |
|---|---:|---:|
| 1a structured ground variants (with references) | 11 | 24 min |
| 1b wall alts, per kit (with references) | 70 | 2.6 h |
| 1c tier 1, the next 200 by impact | 200 | 7.3 h |
| 1d rest of tier 1 | 134 | 4.9 h |
| **tier 1** | **415** | **15.2 h** |
| 2 tier 2, in runs of 200 | 467 | 17.1 h |
| 3a new kits' run pieces, then the kits (with references) | 38 | 1.4 h |
| 3b rest of tier 3 | 280 | 10.3 h |
| **tier 3** | **318** | **11.7 h** |
| **all new** | **1,200** | **44 h** |

The script stops waiting after `--timeout` minutes, which defaults to 6 hours. The commands below raise it to 24 hours. If a run still times out, rerun the same command: `--resume` collects what Slopify has finished and pays for nothing twice.

## Commands

Every command below runs from the repository root, with Slopify up and the Codex provider signed in. Runs go to the default Slopify channel, `DiceMaster Assets`. Every step uses the same settings, so set them once as an alias (the first line is for fish, the second for bash):

```sh
alias gen 'node tools/art-gen/generate.mjs --provider codex --model gpt-6-astra --thinking high --timeout 1440 --resume'   # fish
alias gen='node tools/art-gen/generate.mjs --provider codex --model gpt-6-astra --thinking high --timeout 1440 --resume'   # bash
```

Look before each step with `--dry-run` instead of `--yes` (free and offline). The raw folder is `out/art-gen/raw`. A reference below names a `.png`; if Codex returned a `.jpg` for that asset, use that file instead.

### Tier 1

**1a. Structured ground variants.** These must share the accepted variant 0's layout, so each key runs with its `.0` as the reference. They differ only in wear and stains.

```sh
gen --only "ground.cobbles.3"                          --reference out/art-gen/raw/ground.cobbles.0.png --yes
gen --only "ground.flagstone.2,ground.flagstone.3"     --reference out/art-gen/raw/ground.flagstone.0.png --yes
gen --only "ground.planks.2,ground.planks.3"           --reference out/art-gen/raw/ground.planks.0.png --yes
gen --only "ground.stone_floor.2,ground.stone_floor.3" --reference out/art-gen/raw/ground.stone_floor.0.png --yes
gen --only "ground.farmland.2,ground.farmland.3"       --reference out/art-gen/raw/ground.farmland.0.png --yes
gen --only "ground.cliff.2"                            --reference out/art-gen/raw/ground.cliff.0.png --yes
gen --only "ground.rug.2"                              --reference out/art-gen/raw/ground.rug.0.png --yes
```

**1b. Wall alts.** Each kit runs with its accepted plain `run` as the reference, so material, colour and band thickness match and the pieces still join. Each kit has 10 alts: two for `run`, one for each other role.

```sh
gen --only "wall.stone.*.alt*"     --reference out/art-gen/raw/wall.stone.run.png --yes
gen --only "wall.timber.*.alt*"    --reference out/art-gen/raw/wall.timber.run.png --yes
gen --only "wall.wattle.*.alt*"    --reference out/art-gen/raw/wall.wattle.run.png --yes
gen --only "wall.drystone.*.alt*"  --reference out/art-gen/raw/wall.drystone.run.png --yes
gen --only "wall.hedge.*.alt*"     --reference out/art-gen/raw/wall.hedge.run.png --yes
gen --only "wall.palisade.*.alt*"  --reference out/art-gen/raw/wall.palisade.run.png --yes
gen --only "wall.city_wall.*.alt*" --reference out/art-gen/raw/wall.city_wall.run.png --yes
```

**1c. The next 200 by impact.** These are 38 ground and 4 water variants, all tree alts and the first plant and rock alts (76 vegetation), and every alt of the frequent props (82). The `--resume` flag skips what 1a already made.

```sh
gen --tier 1 --class ground,water,prop,vegetation --limit 200 --yes
```

**1d. The rest of tier 1** (134 images).

```sh
gen --tier 1 --yes
```

Import and curate after tier 1 (see the README, steps 3 and 4) before spending another day on tier 2.

### Tier 2

Run tier 2 in chunks of 200. With `--resume`, each run takes the next 200 by impact; three runs cover all 467. Import and curate between chunks.

```sh
gen --tier 2 --limit 200 --yes
gen --tier 2 --limit 200 --yes
gen --tier 2 --yes
```

### Tier 3

**3a. The new wall kits.** Make each kit's run first, accept it, then make the rest of the kit with that run as the reference.

```sh
gen --only "wall.adobe.run,wall.log.run" --yes
gen --only "wall.adobe.*" --reference out/art-gen/raw/wall.adobe.run.png --yes
gen --only "wall.log.*"   --reference out/art-gen/raw/wall.log.run.png --yes
```

**3b. Everything else in tier 3** (280 images).

```sh
gen --tier 3 --limit 200 --yes
gen --tier 3 --yes
```

## How variants reach the library

- **Cut-outs and wall pieces:** `<id>.altN` in `prompts.json`. `generate.mjs` saves it as `<id>__altN.png`. `arda tactical import` ignores what follows `__` and numbers the takes of one id in sorted file order. Because `.` sorts before `_`, `<id>.png` stays `<id>`, and `__alt1`, `__alt2`, … become `<id>.alt1`, `<id>.alt2`, …, the same ids as the prompts. The exception is a missing take: the importer numbers the files it has, so after a gap the later takes shift down.
- **Ground:** `ground.<key>.<n>` continues the series (`.0` to `.3`); the importer numbers the variants in file order.
- **Alts are interchangeable.** They share their base's footprint, layer, height, cover, blocking and tags exactly (a test checks this). So the compositor can pick any take wherever the base fits. A piece that suits only one wealth level, culture or biome has its own id instead (`prop.table_wealthy`, `prop.bed_dwarf`, `prop.well_desert`).
- **Metadata:** `import.toml` now carries each asset's `layer`, `height_ft`, `cover`, blocking flags and `tags` from `prompts.json`, besides the footprint. New ids therefore import with their real metadata, not the importer's class defaults. For the first 225, the values equal the placeholder library's records.

## Tags

- **Controlled tags** use only the catalogue vocabulary, because the importer copies the vocabulary from the placeholder library:
  - `function`: all 34 functions, every one with specific props now;
  - `biome`: temperate, boreal, alpine, wetland;
  - `wealth`: poor, modest, wealthy;
  - `culture`: human.
- **Free tags** carry what the vocabulary lacks:
  - arid and coastal pieces: `arid` and `coastal`, with an empty `biome` list for arid;
  - ancestry pieces: `dwarf`, `elf` and `orc`, with an empty `culture` list;
  - animals: `livestock:*`;
  - crafts: `craft:<key>`, using arda-ids' craft keys (carpentry, weaving, pottery, cobbling, masonry, jewellery, glassblowing, leatherwork, woodcarving, tinkering, painting, cartography).

  A bare tag query such as `dwarf` finds them today, and `culture:human` or `biome:temperate` leaves them out.

## Engine follow-ups (not done here)

1. **Alts by id.** `AssetRef::Id("prop.barrel")` resolves only that exact id today (`compose/mod.rs`), so alts appear only through tag queries. The compositor change that picks among `<id>` and its `.altN` siblings by seed is being made separately.
2. **Vocabulary for the new names.** The new prop and vegetation ids are not yet in `docs/goal-prompts/vocabulary.md`, or in `crates/arda-art-import/src/vocab.rs` (which mirrors it). The importer imports them, flagged "no layout asks for it", until both lists gain them and arda-town or arda-fields place them. The new names are the 258 base ids without an `.altN` suffix in `prompts.json`. Two adjacent changes:
   - add the `adobe` and `log` wall kits to `WALL_KITS`;
   - promote `arid`, `coastal` (biome) and `dwarf`, `elf`, `orc` (culture) to the controlled vocabulary of the placeholder catalogue.
3. **Lights.** `import.toml` has no `light` key, so new fire and lamp pieces import without a light pool: campfire, cauldron, brew kettle, kiln, glass furnace, grand hearth, clay oven, votive rack, the dwarf forge and braziers, the orc brazier and the elf lantern. Alts of lit pieces keep their base's light. The manifest needs a `light` override, or the catalogue needs a pass.
4. **Placement rules.** The manifest cannot set `placement` either. Boats are `on_water` only when their name contains "boat" (the importer's default), so `canoe` and `barge` need it set.
5. **New wall kits.** `arda-town`'s `kits::shell` never picks `adobe` or `log`. A tradition hook is needed, for example a southern or arid culture for adobe and boreal settlements for log.
6. **Craft tags disagree.** The placeholder library tags workshop props `craft:smith`, `craft:carpenter`, `craft:weaver` and `craft:cutler`. arda-ids emits `craft:carpentry`, `craft:weaving` and the rest. The new pieces use arda-ids' keys. The placeholder's four should follow when it is next regenerated.
7. **Footprints.** `arda-town`'s `interior::footprint` hard-codes `prop.oven` as 2×1 and `prop.millstone` as 2×2, but the catalogue has both at 1×1. The alts keep the catalogue's footprints.

## Future ideas (no class for them yet)

- **Roofs.** The compositor has no roof class. The `canopy` layer's doc comment mentions roofs, but `arda-town` and arda-tactical never draw them; tactical maps show interiors. Roof art (thatch, shingle, slate and tile sheets per footprint) would need two things: a roof layer that the renderer can hide over the party's building, and a roof footprint from arda-town's building plan. So none are in the pack.
- **Seasonal sets.** Autumn and snow pieces are separate ids (`veg.tree_autumn`, `veg.tree_spruce_snow`, `veg.rock_snow`, …), so they never mix into summer maps. A `season:*` tag and a layout-level season would let the compositor choose them.
- **More wall kits** for dwarven ashlar, elven living wood and orcish hide-and-stake walls, once cultures choose kits.
