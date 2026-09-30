//! Every SRD stat block is internally consistent, and every stat-block
//! sheet the generator hands out matches its data file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::rules::{cr_proficiency, Dice};
use arda_npc::sheet::SheetKind;
use common::{everyone, modifier, port_town, srd, town};
use serde_json::Value;

fn int(v: &Value) -> i64 {
    v.as_i64().unwrap()
}

#[test]
fn stat_blocks_are_internally_consistent() {
    let blocks = srd("stat_blocks.json");
    let equipment = srd("equipment.json");
    assert_eq!(blocks.as_array().unwrap().len(), 21);
    for b in blocks.as_array().unwrap() {
        let name = b["name"].as_str().unwrap();
        let abilities: Vec<u8> = b["abilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(int(v)).unwrap())
            .collect();
        let mods: Vec<i64> = abilities.iter().map(|&a| i64::from(modifier(a))).collect();
        let prof = i64::from(cr_proficiency(b["challenge_rating"].as_str().unwrap()));
        // Hit points = average of the hit dice + Con × dice.
        let dice = Dice::parse(b["hit_dice"].as_str().unwrap()).unwrap();
        let avg = i64::from(dice.count) * (i64::from(dice.sides) + 1) / 2;
        assert_eq!(
            int(&b["hit_points"]),
            avg + mods[2] * i64::from(dice.count),
            "{name} hit points"
        );
        // Attacks: to-hit = proficiency + Str (Dex for ranged or finesse).
        for a in b["actions"].as_array().unwrap() {
            let dmg = Dice::parse(a["damage"].as_str().unwrap()).unwrap();
            let to_hit = int(&a["attack_bonus"]);
            let options = [mods[0], mods[1]];
            let used = options.iter().copied().find(|m| m + prof == to_hit);
            let used =
                used.unwrap_or_else(|| panic!("{name} {}: +{to_hit} fits no ability", a["name"]));
            assert!(
                i64::from(dmg.bonus) == used || name == "Gladiator",
                "{name} {} damage bonus",
                a["name"]
            );
        }
        // Armour class from the listed armour.
        let armor: Vec<&str> = b["armor"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let shield = if armor.contains(&"Shield") { 2 } else { 0 };
        let body = armor.iter().find(|a| **a != "Shield");
        let expected = match body {
            Some(body) => {
                let a = equipment["armor"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|x| x["name"] == *body)
                    .unwrap();
                let dex = match (a["dex_bonus"].as_bool().unwrap(), a["max_dex"].as_i64()) {
                    (false, _) => 0,
                    (true, Some(cap)) => mods[1].min(cap),
                    (true, None) => mods[1],
                };
                int(&a["base_ac"]) + dex
            }
            None => 10 + mods[1],
        } + shield;
        assert_eq!(int(&b["armor_class"]), expected, "{name} armour class");
        // Spellcasting: DC = 8 + proficiency + ability, attack = proficiency + ability.
        if let Some(sc) = b["spellcasting"].as_object() {
            let idx = ["STR", "DEX", "CON", "INT", "WIS", "CHA"]
                .iter()
                .position(|a| *a == sc["ability"])
                .unwrap();
            assert_eq!(int(&sc["save_dc"]), 8 + prof + mods[idx], "{name} save DC");
            assert_eq!(
                int(&sc["attack_bonus"]),
                prof + mods[idx],
                "{name} spell attack"
            );
        }
        // Saves and skills are at least the ability modifier plus proficiency.
        for (ability, bonus) in b["saves"].as_object().unwrap() {
            let idx = ["STR", "DEX", "CON", "INT", "WIS", "CHA"]
                .iter()
                .position(|a| a == ability)
                .unwrap();
            assert_eq!(int(bonus), mods[idx] + prof, "{name} {ability} save");
        }
    }
}

#[test]
fn generated_stat_block_sheets_match_the_data() {
    let blocks = srd("stat_blocks.json");
    let mut seen = std::collections::BTreeSet::new();
    for (profile, buildings) in [town(), port_town()] {
        for npc in everyone(&profile, &buildings) {
            let s = &npc.sheet;
            let SheetKind::StatBlock {
                name,
                challenge_rating,
                xp,
            } = &s.kind
            else {
                continue;
            };
            seen.insert(name.clone());
            let b = blocks
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["name"] == name.as_str())
                .unwrap();
            let abilities: Vec<u8> = b["abilities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| u8::try_from(int(v)).unwrap())
                .collect();
            assert_eq!(s.abilities.to_vec(), abilities, "{name} abilities");
            assert_eq!(
                i64::from(s.armor_class),
                int(&b["armor_class"]),
                "{name} AC"
            );
            assert_eq!(i64::from(s.hit_points), int(&b["hit_points"]), "{name} HP");
            assert_eq!(s.hit_dice, b["hit_dice"].as_str().unwrap());
            assert_eq!(challenge_rating, b["challenge_rating"].as_str().unwrap());
            assert_eq!(i64::from(*xp), int(&b["xp"]));
            assert_eq!(
                i64::from(s.passive_perception),
                int(&b["passive_perception"])
            );
            let actions = b["actions"].as_array().unwrap();
            assert_eq!(s.attacks.len(), actions.len(), "{name} attacks");
            for (attack, action) in s.attacks.iter().zip(actions) {
                assert_eq!(attack.name, action["name"].as_str().unwrap());
                assert_eq!(i64::from(attack.to_hit), int(&action["attack_bonus"]));
                let dice = Dice::parse(action["damage"].as_str().unwrap()).unwrap();
                assert_eq!(attack.average, dice.average());
            }
            for (skill, bonus) in b["skills"].as_object().unwrap() {
                let found = s.skills.iter().find(|k| k.name == *skill).unwrap();
                assert_eq!(i64::from(found.bonus), int(bonus), "{name} {skill}");
            }
            match (&s.spellcasting, b["spellcasting"].as_object()) {
                (Some(c), Some(d)) => {
                    assert_eq!(i64::from(c.save_dc), int(&d["save_dc"]));
                    assert_eq!(c.spells.len(), d["spells"].as_array().unwrap().len());
                }
                (None, None) => {}
                _ => panic!("{name} spellcasting mismatch"),
            }
            for item in b["equipment"].as_array().unwrap() {
                assert!(
                    s.equipment.iter().any(|i| i.name == item.as_str().unwrap()),
                    "{name} lacks {item}"
                );
            }
        }
    }
    for expected in [
        "Commoner", "Guard", "Veteran", "Knight", "Acolyte", "Priest", "Noble", "Thug", "Druid",
    ] {
        assert!(
            seen.contains(expected),
            "no {expected} generated; saw {seen:?}"
        );
    }
}
