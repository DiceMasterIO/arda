# SRD 5.1 data

Every file in this directory holds game mechanics from the **System Reference
Document 5.1** (SRD 5.1) and nothing else:

| File | Content |
| --- | --- |
| `races.json` | the nine SRD races, one SRD subrace each where the SRD has one (hill dwarf, high elf, lightfoot halfling, rock gnome) |
| `classes.json` | the twelve SRD classes, each with its one SRD subclass, hit die, saves, proficiencies, class skill lists and feature names |
| `spellcasting.json` | the full-caster and Pact Magic slot tables |
| `spells.json` | every SRD spell by name, level and class list (no descriptions) |
| `stat_blocks.json` | the 21 NPC stat blocks from the SRD appendix "Nonplayer Characters" |
| `languages.json` | the SRD standard, exotic and secret languages |
| `equipment.json` | SRD armour, weapons, adventuring gear and tools (names, costs and rules values) |

- **Source:** System Reference Document 5.1, Wizards of the Coast, released
  under CC-BY-4.0 on 2023-01-23:
  <https://dnd.wizards.com/resources/systems-reference-document>
  (PDF: <https://media.wizards.com/2023/downloads/dnd/SRD_CC_v5.1.pdf>).
- **Transcription:** the values were extracted from the SRD-only dataset
  served by <https://www.dnd5eapi.co> (API version `2014`) and then checked
  against the SRD 5.1 PDF: hit points, armour class, saves, challenge
  ratings and every attack line of the 21 NPC stat blocks. One value in that
  dataset differs from the PDF and was corrected to the PDF: Cult Fanatic
  hit points 33 (6d8 + 6). Only names, numbers and rule values are kept. No
  rules prose or descriptions are copied.
- **Arda additions:** `working_age` and `elder_age` in `races.json` are
  Arda's own tuning values for jobs and retirement; they are not SRD text.
  `adult_age` and `max_age` restate the SRD's race age descriptions as
  numbers. `loadout`, `priority`, `unarmored_bonus`, `asi_levels` and
  `extra_attack_level` in `classes.json` are Arda's encodings of SRD class
  rules and of a default equipment choice.

Personality, occupation, background and name material is **not** SRD
content. It is original to Arda and lives in `../content/`.

## Attribution (CC-BY-4.0)

This work includes material taken from the System Reference Document 5.1
("SRD 5.1") by Wizards of the Coast LLC and available at
<https://dnd.wizards.com/resources/systems-reference-document>. The SRD 5.1
is licensed under the Creative Commons Attribution 4.0 International License
available at <https://creativecommons.org/licenses/by/4.0/legalcode>.
