# arda-names

Deterministic naming languages for Arda. The crate names settlements, regions, rivers, mountains, realms and people, and every name belongs to a consistent language of its culture.

All linguistic material is generated. The phoneme inventories, syllable rules and preset weights were written for Arda, and the lexicon is drawn from them by seed. The crate contains no copied name lists, no real-world name databases and no fictional-language vocabulary, so it is safe to ship in a commercial game. Its only dependencies are `serde` and `thiserror`, both MIT/Apache-2.0.

## Quick start

```rust
use arda_names::{person_name, place_name, FamilyCtx, Language, Meaning, PlaceKind, PlaceSpec, Preset, Sex, SiteTag};

let lang = Language::new(2026, Preset::Heartland);

let spec = PlaceSpec::new(PlaceKind::Village, 1)
    .tags(&[SiteTag::Ford])
    .features(&[Meaning::Oak]);
let town = place_name(&lang, &spec);
// town.native = "Erena", town.gloss = "Oakford", town.literal = "ford of the oaks",
// town.pronunciation = "E-re-na"  (pinned by tests/determinism.rs)

let npc = person_name(&lang, Sex::Female, &FamilyCtx::new(1));
// npc.full = "Teemin Tarnero", npc.gloss = "Teemin, daughter of Tarner"
```

Run the showcase to hear every preset. It prints 21 places and 20 people per preset. The optional argument is a seed:

```sh
cargo run -p arda-names --example showcase [seed]
cargo run -p arda-names --example lexicon  [seed]   # the first 60 words per language
```

## What a language is

`Language::new(seed, preset)` builds the following:

- **Phonology** (`phonology.rs`). The consonants and vowels come from the preset, and the seed drops a few consonants and jitters the weights. The phonology also holds onset clusters, codas and coda clusters, diphthongs, hiatus, vowel harmony, geminates, the longest medial consonant run, and the stress rule (initial, penultimate or final).
- **Orthography** (`ortho.rs`). This is the romanisation: lowercase ASCII plus the apostrophe, with per-language spellings (`k=c`, `x=ch`), a context rule for /k/ before front vowels, and word-final spellings (`-y`).
- **Lexicon** (`meaning.rs`, `language.rs`). The lexicon holds one root per meaning, about 190 in all. They include place heads (river, ford, town, fort, harbour, marsh), descriptive adjectives (holy, new, old, white, black), nouns (oak, stone, wolf, bridge), beings (king, monk), trades, and grammatical affixes (plural, genitive linker, son of, daughter of, kin of). Roots never share a spelling and are never blocked. The lexicon also holds 48 meaningless roots for personal names.
- **Grammar**. Compounds are either head-last ("Oakford") or head-first ("Ford-of-Oaks", with a genitive linker). The grammar also sets the weights of the family-name customs and the byname rate.
- **Substrate**. This is an older `Ancient` tongue. It is shared across a world when you pass the same `Options::substrate_seed` to every language. Major rivers and some mountains take their names from it, and the names are then adapted to the living language's inventory and phonotactics. In the gloss, the original meaning appears as "bright river, in the old tongue".

## Presets

| Human culture (keys match `arda-settle`) | Sound |
|---|---|
| `heartland` | mellow lowland; soft stops, liquids, closed first syllables |
| `highland` | clipped, closed monosyllables, velar fricatives, head-first |
| `sylvan` | breathy fricatives, liquids, diphthongs, penultimate stress |
| `coastal` | harsh northern seafarers; s-clusters, long vowels, patronymics |
| `southern` | flowing open syllables, head-first with a linker |
| `borderland` | steppe marchfolk; vowel harmony, final stress, no clusters |

| SRD ancestry (keys include `dwarf`, `elf`, `half-orc`, `dragonborn`, `tiefling`) | Sound |
|---|---|
| `dwarvish` | heavy stops, back vowels, r-clusters, clan names |
| `elvish` | sibilants and glide clusters, vowel-rich, end-stressed |
| `halfling` | bouncy labials, doubled consonants, homely endings |
| `gnomish` | quick buzzing polysyllables with z |
| `orcish` | gutturals, glottal breaks, closed back-vowelled syllables |
| `draconic` | hissing, rolling r, x-codas, end-stressed |
| `infernal` | dark sonorants, buzzing fricatives |

`Preset::from_key` accepts every preset key and the SRD ancestry names.

## Names

- **Places.** Build a `PlaceSpec` from `kind`, `features`, `site_tags`, `key` and `from`. The head comes from the strongest site tag: estuary gives mouth, harbour gives haven, confluence gives meet, and ford gives ford. When no tag applies, the kind supplies the head (hamlet gives stead, town gives ton). The modifier comes from the caller's features, or else from what grows and lives at such a site, or else from a founder's name. Morphemes meet through sandhi (`sandhi.rs`), which covers elision, cluster simplification and epenthesis. Affixes follow vowel harmony, and names longer than four syllables lose a medial unstressed vowel. Every name carries `native`, the English `gloss` ("Kingsbridge"), a `literal` reading ("the king's bridge") and a stress respelling.
- **Regions and realms.** Pass the capital as `from`. A region becomes "Oakfordshire", and a realm joins the capital's first element to "mark" ("Oakmark").
- **People.** `person_name(lang, sex, &FamilyCtx)` builds a given name from one or two elements plus a sex ending. The family name follows a custom picked by the culture's weights: patronymic, occupational, toponymic, a descriptive compound, or kin of an ancestor. Adults sometimes also get a byname. `FamilyCtx` supplies the father, an inherited family name, a trade and a home, and it generates whatever is missing from `id`.

## Dialects

`DialectMap::new(&lang, seed, width, height)` draws up to ten isoglosses across the realm. Each one is a regular sound change, such as lenition, palatalisation, a vowel shift, apocope or rhotacism, which holds on one side of a straight line. `dialect_at(x, y)` returns the local language. Neighbouring towns differ by the few lines between them, and distant towns differ by many, so similarity decays with distance. The `dialects` test measures that decay.

## Uniqueness and the blocklist

`NameScope::place_name` guarantees that no two names in a scope share a spelling skeleton. The skeleton ignores case, apostrophes and doubled letters. On a clash the scope first redraws, then names the place after a founder, then adds a distinguishing adjective ("Upper Oakford"), and when every candidate is taken returns `NamesError::ScopeExhausted` rather than a duplicate. Name places in a stable order, because the result depends on what the scope already holds.

`src/blocklist.txt` is an original, hand-kept list. It holds offensive fragments, which are blocked anywhere in a name (prefixed with `*`), and whole words, which are blocked only when a name word equals them. The whole words include short profanities, everyday English words that would read as jokes, and well-known fictional and real place names. Maintainers can extend the list in the file or at run time with `Blocklist::extend_from_str`, `add_word` or `add_fragment`, passed through `Options::blocklist`. Any extension changes which names are drawn.

## Guarantees and tests

- **Deterministic.** Generation uses integers only and a hand-rolled SplitMix64 keyed by seed, preset and caller key. Every public type is serde-serialisable, and a round trip reproduces every name (`tests/determinism.rs`, which also pins a golden name).
- **Phonotactically valid, spellable and clean.** Every root and name passes its language's rules and the spellability check, and none is blocked (`tests/phonotactics.rs`).
- **Unique within a scope** (`tests/uniqueness.rs`).
- **Distinct presets.** Phoneme and bigram Jensen-Shannon divergences between presets clear fixed floors and are more than twice the variation between seeds of one preset (`tests/distinctness.rs`).
- **Dialect distance grows with map distance** (`tests/dialects.rs`).
- **Fast.** 100 000 names take about 0.4 s, and the test requires under 1 s (`tests/performance.rs`).

## Adopting it

The simple generators in `arda-settle` and `arda-npc` can switch over like this:

- **Culture.** Use `Preset::from_key(culture.key())` for the culture and `Preset::from_key(ancestry)` for non-human NPCs.
- **Settlements.** Build the tags with `SiteTag::from_keys(site.tags.iter().map(String::as_str))` and the kind with `PlaceKind::from_key(tier)`. Name settlements through one `NameScope` per realm or per world, in id order.
- **Rivers.** Use `PlaceKind::River` for major rivers and `PlaceKind::Stream` for small ones. Build every culture's language with the same `substrate_seed`, so that a river keeps related names across borders.
- **NPCs.** Pass the NPC id as `FamilyCtx::id`. For households, pass the head's family `Name` as `family`, so that members share it. Pass the job as `trade`, using `Meaning::from_key("blacksmith")`.
