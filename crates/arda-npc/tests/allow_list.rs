//! Every race, class, subclass, spell, item and stat block a sheet names
//! comes from the SRD 5.1 data files.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeSet;

use arda_npc::sheet::SheetKind;
use common::{city, everyone, hamlet, port_town, srd, village};
use serde_json::Value;

fn names(v: &Value, key: &str) -> BTreeSet<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x[key].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn every_name_is_srd_content() {
    let races = srd("races.json");
    let classes = srd("classes.json");
    let spells = srd("spells.json");
    let blocks = srd("stat_blocks.json");
    let equipment = srd("equipment.json");
    let languages = srd("languages.json");
    let race_keys = names(&races, "key");
    let race_names = names(&races, "name");
    let subraces: BTreeSet<String> = races
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["subrace"].as_str().map(str::to_string))
        .collect();
    let class_names = names(&classes, "name");
    let subclasses: BTreeSet<String> = classes
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["subclass"]["name"].as_str().unwrap().to_string())
        .collect();
    let block_names = names(&blocks, "name");
    let mut items = names(&equipment["armor"], "name");
    items.extend(names(&equipment["weapons"], "name"));
    for list in ["gear", "tools"] {
        items.extend(
            equipment[list]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g.as_str().unwrap().to_string()),
        );
    }
    let mut spoken: BTreeSet<String> = BTreeSet::new();
    for list in ["standard", "exotic", "secret"] {
        spoken.extend(
            languages[list]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g.as_str().unwrap().to_string()),
        );
    }
    let spell_level = |name: &str| {
        spells
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == name)
            .map(|s| s["level"].as_u64().unwrap())
    };
    let spell_classes = |name: &str| -> Vec<String> {
        spells
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == name)
            .map_or_else(Vec::new, |s| {
                s["classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c.as_str().unwrap().to_string())
                    .collect()
            })
    };
    let mut checked = 0;
    for (profile, buildings) in [hamlet(), village(), port_town(), city(8_000)] {
        for npc in everyone(&profile, &buildings) {
            let who = npc.name.full();
            assert!(
                race_keys.contains(&npc.ancestry),
                "{who}: race {}",
                npc.ancestry
            );
            assert!(race_names.contains(&npc.ancestry_name));
            if let Some(sub) = &npc.subrace {
                assert!(subraces.contains(sub), "{who}: subrace {sub}");
            }
            let s = &npc.sheet;
            match &s.kind {
                SheetKind::StatBlock { name, .. } => {
                    assert!(block_names.contains(name), "{who}: block {name}");
                }
                SheetKind::Class {
                    class, subclass, ..
                } => {
                    assert!(class_names.contains(class), "{who}: class {class}");
                    if let Some(sub) = subclass {
                        assert!(subclasses.contains(sub), "{who}: subclass {sub}");
                    }
                    if let Some(casting) = &s.spellcasting {
                        for spell in &casting.spells {
                            assert!(
                                spell_classes(&spell.name).contains(class),
                                "{who}: {} not a {class} spell",
                                spell.name
                            );
                        }
                    }
                }
            }
            if let Some(casting) = &s.spellcasting {
                for spell in &casting.spells {
                    assert_eq!(
                        spell_level(&spell.name),
                        Some(u64::from(spell.level)),
                        "{who}: spell {}",
                        spell.name
                    );
                }
            }
            for item in s.equipment.iter().map(|i| &i.name).chain(&s.armor) {
                assert!(items.contains(item), "{who}: item {item}");
            }
            for attack in &s.attacks {
                let weaponish = items.contains(&attack.name) || block_names.contains(&attack.name);
                let srd_action = [
                    "Unarmed Strike",
                    "Shield Bash",
                    "Light Crossbow",
                    "Heavy Crossbow",
                    "Hand Crossbow",
                ];
                assert!(
                    weaponish || srd_action.contains(&attack.name.as_str()),
                    "{who}: attack {}",
                    attack.name
                );
            }
            for language in &s.languages {
                assert!(spoken.contains(language), "{who}: language {language}");
            }
            checked += 1;
        }
    }
    assert!(checked > 9_000);
}

/// Well-known Player's Handbook and later content that is *not* in SRD 5.1.
/// The check above compares sheets with the bundled data files, so it cannot
/// see a non-SRD entry that slipped into those files; this list is an
/// independent tripwire over every data file and every generated sheet.
const NOT_SRD: &[&str] = &[
    // Subraces.
    "Mountain Dwarf",
    "Wood Elf",
    "Dark Elf",
    "Drow",
    "Stout Halfling",
    "Forest Gnome",
    // Subclasses.
    "Path of the Totem Warrior",
    "College of Valor",
    "Knowledge Domain",
    "Light Domain",
    "Nature Domain",
    "Tempest Domain",
    "Trickery Domain",
    "War Domain",
    "Circle of the Moon",
    "Battle Master",
    "Eldritch Knight",
    "Way of Shadow",
    "Way of the Four Elements",
    "Oath of the Ancients",
    "Oath of Vengeance",
    "Beast Master",
    "Arcane Trickster",
    "Wild Magic",
    "The Archfey",
    "The Great Old One",
    "School of Abjuration",
    "School of Conjuration",
    "School of Divination",
    "School of Enchantment",
    "School of Illusion",
    "School of Necromancy",
    "School of Transmutation",
    // Spells.
    "Hex",
    "Chromatic Orb",
    "Witch Bolt",
    "Armor of Agathys",
    "Arms of Hadar",
    "Hunger of Hadar",
    "Ensnaring Strike",
    "Hail of Thorns",
    "Searing Smite",
    "Thunderous Smite",
    "Wrathful Smite",
    "Blinding Smite",
    "Compelled Duel",
    "Dissonant Whispers",
    "Blade Ward",
    "Friends",
    "Thorn Whip",
    "Crown of Madness",
    "Cloud of Daggers",
    "Lightning Arrow",
    "Conjure Barrage",
    "Booming Blade",
    "Green-Flame Blade",
    "Toll the Dead",
    // Stat blocks outside the SRD appendix.
    "Apprentice Wizard",
    "Swashbuckler",
    "War Priest",
    "Evoker",
    "Master Thief",
];

fn quoted_strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| quoted_strings(x, out)),
        Value::Object(o) => {
            for (k, x) in o {
                out.push(k.clone());
                quoted_strings(x, out);
            }
        }
        _ => {}
    }
}

#[test]
fn no_known_non_srd_name_appears_in_data_or_sheets() {
    let mut strings = Vec::new();
    for file in [
        "races.json",
        "classes.json",
        "spells.json",
        "stat_blocks.json",
        "equipment.json",
        "languages.json",
        "spellcasting.json",
    ] {
        quoted_strings(&srd(file), &mut strings);
    }
    let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/content");
    for entry in std::fs::read_dir(content).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        quoted_strings(&serde_json::from_str(&text).unwrap(), &mut strings);
    }
    for (profile, buildings) in [village(), port_town()] {
        for npc in everyone(&profile, &buildings) {
            quoted_strings(&serde_json::to_value(&npc).unwrap(), &mut strings);
        }
    }
    for s in &strings {
        assert!(
            !NOT_SRD.contains(&s.as_str()),
            "non-SRD content {s:?} in the data or a sheet"
        );
    }
    assert!(strings.len() > 10_000, "{} strings scanned", strings.len());
}
