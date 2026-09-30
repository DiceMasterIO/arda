//! SRD 5.1 maths, checked against hand-computed values and recomputed from
//! the raw JSON for every generated class build.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::rng::SeedKey;
use arda_npc::rules::{
    armor_class, class_hit_points, modifier, proficiency_bonus, spell_attack_bonus, spell_save_dc,
    spell_slots, Ability, CasterKind, DexRule,
};
use arda_npc::sheet::{class_sheet, AttackKind, Sheet, SheetKind};
use arda_npc::Npc;
use arda_npc::{Lifestyle, SettlementId};
use common::{city, everyone, port_town, srd, town};
use serde_json::Value;

#[test]
fn ability_modifiers() {
    let expected = [
        (1, -5),
        (3, -4),
        (8, -1),
        (9, -1),
        (10, 0),
        (11, 0),
        (12, 1),
        (15, 2),
        (18, 4),
        (20, 5),
        (30, 10),
    ];
    for (score, m) in expected {
        assert_eq!(modifier(score), m, "score {score}");
    }
}

#[test]
fn proficiency_by_level() {
    let expected = [2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6];
    for (i, &p) in expected.iter().enumerate() {
        assert_eq!(
            proficiency_bonus(u8::try_from(i + 1).unwrap()),
            p,
            "level {}",
            i + 1
        );
    }
}

#[test]
fn hit_points() {
    // Fighter 1, Con +2: 10 + 2.
    assert_eq!(class_hit_points(1, 10, 2, 0), 12);
    // Wizard 5, Con +1: 6 + 1 + 4 × (4 + 1).
    assert_eq!(class_hit_points(5, 6, 1, 0), 27);
    // Hill dwarf cleric 3, Con +3: (8 + 3 + 1) + 2 × (5 + 3 + 1).
    assert_eq!(class_hit_points(3, 8, 3, 1), 30);
    // Barbarian 4, Con −1: 11 + 3 × 6.
    assert_eq!(class_hit_points(4, 12, -1, 0), 29);
}

#[test]
fn armour_class_for_every_armour_type() {
    let equipment = srd("equipment.json");
    let dex = 3;
    let expected = [
        ("Padded Armor", 14),
        ("Leather Armor", 14),
        ("Studded Leather Armor", 15),
        ("Hide Armor", 14),
        ("Chain Shirt", 15),
        ("Scale Mail", 16),
        ("Breastplate", 16),
        ("Half Plate Armor", 17),
        ("Ring Mail", 14),
        ("Chain Mail", 16),
        ("Splint Armor", 17),
        ("Plate Armor", 18),
    ];
    for (name, ac) in expected {
        let a = equipment["armor"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == name)
            .unwrap();
        let rule = match (a["dex_bonus"].as_bool().unwrap(), a["max_dex"].as_u64()) {
            (false, _) => DexRule::Ignored,
            (true, Some(cap)) => DexRule::Capped(u8::try_from(cap).unwrap()),
            (true, None) => DexRule::Full,
        };
        let base = u8::try_from(a["base_ac"].as_u64().unwrap()).unwrap();
        assert_eq!(armor_class(base, dex, rule, false, 0), ac, "{name}");
        assert_eq!(
            armor_class(base, dex, rule, true, 0),
            ac + 2,
            "{name} with shield"
        );
    }
    assert_eq!(
        armor_class(10, 2, DexRule::Full, false, 3),
        15,
        "monk-style unarmoured"
    );
}

#[test]
fn spell_save_dc_and_attack() {
    assert_eq!(spell_save_dc(2, 3), 13);
    assert_eq!(spell_attack_bonus(2, 3), 5);
    assert_eq!(spell_save_dc(4, 5), 17);
    assert_eq!(spell_attack_bonus(6, 5), 11);
}

#[test]
fn spell_slots_full_half_and_pact() {
    let full = |l| spell_slots(CasterKind::Full, l).unwrap();
    assert_eq!(full(1), vec![2]);
    assert_eq!(full(3), vec![4, 2]);
    assert_eq!(full(5), vec![4, 3, 2]);
    assert_eq!(full(9), vec![4, 3, 3, 3, 1]);
    assert_eq!(full(17), vec![4, 3, 3, 3, 2, 1, 1, 1, 1]);
    assert_eq!(full(20), vec![4, 3, 3, 3, 3, 2, 2, 1, 1]);
    let half = |l| spell_slots(CasterKind::Half, l).unwrap();
    assert!(half(1).is_empty());
    assert_eq!(half(2), vec![2]);
    assert_eq!(half(5), vec![4, 2]);
    assert_eq!(half(9), vec![4, 3, 2]);
    assert_eq!(half(20), vec![4, 3, 3, 3, 2]);
    let pact = |l| spell_slots(CasterKind::Pact, l).unwrap();
    assert_eq!(pact(1), vec![1]);
    assert_eq!(pact(2), vec![2]);
    assert_eq!(pact(5), vec![0, 0, 2]);
    assert_eq!(pact(11), vec![0, 0, 0, 0, 3]);
    assert_eq!(pact(17), vec![0, 0, 0, 0, 4]);
    assert!(spell_slots(CasterKind::None, 10).unwrap().is_empty());
}

fn find<'a>(list: &'a Value, key: &str, value: &str) -> &'a Value {
    list.as_array()
        .unwrap()
        .iter()
        .find(|v| v[key] == value)
        .unwrap_or_else(|| panic!("{value} not in data"))
}

/// Recomputes a class build from the raw JSON and checks every derived number.
fn check_class_build(npc: &Npc, classes: &Value, races: &Value, equipment: &Value) {
    check_sheet(
        &npc.sheet,
        &npc.ancestry,
        &format!("{} ({:?})", npc.name.full(), npc.id),
        classes,
        races,
        equipment,
    );
}

fn check_sheet(
    s: &Sheet,
    ancestry: &str,
    who: &str,
    classes: &Value,
    races: &Value,
    equipment: &Value,
) {
    let SheetKind::Class {
        class,
        subclass,
        level,
        ..
    } = &s.kind
    else {
        return;
    };
    let class = find(classes, "name", class);
    let race = find(races, "key", ancestry);
    let level = *level;
    for i in 0..6 {
        assert_eq!(
            s.modifiers[i],
            common::modifier(s.abilities[i]),
            "{who}: modifier {i}"
        );
        assert!(s.abilities[i] <= 20, "{who}: score above 20");
    }
    let prof = 2 + i8::try_from((level - 1) / 4).unwrap();
    assert_eq!(s.proficiency_bonus, prof, "{who}: proficiency");
    let sub_level = u8::try_from(class["subclass"]["level"].as_u64().unwrap()).unwrap();
    assert_eq!(
        subclass.is_some(),
        level >= sub_level,
        "{who}: subclass gate"
    );
    // Saves.
    let saves: Vec<&str> = class["saves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for save in &s.saves {
        let proficient = saves.contains(&save.ability.abbr());
        assert_eq!(save.proficient, proficient, "{who}: save proficiency");
        assert_eq!(
            save.bonus,
            s.modifier(save.ability) + if proficient { prof } else { 0 },
            "{who}: save"
        );
    }
    // Skills.
    for skill in &s.skills {
        let expected = s.modifier(skill.ability) + if skill.expertise { 2 * prof } else { prof };
        assert_eq!(skill.bonus, expected, "{who}: skill {}", skill.name);
    }
    // Hit points: max die at 1, then die / 2 + 1, plus Con and flat bonuses.
    let die = i32::try_from(class["hit_die"].as_u64().unwrap()).unwrap();
    let con = i32::from(s.modifier(Ability::Con));
    let mut bonus = i32::try_from(race["hp_per_level"].as_u64().unwrap()).unwrap();
    if level >= sub_level {
        bonus += i32::try_from(class["subclass"]["hp_per_level"].as_u64().unwrap()).unwrap();
    }
    let hp =
        (die + con).max(1) + bonus + (i32::from(level) - 1) * ((die / 2 + 1 + con).max(1) + bonus);
    assert_eq!(i32::from(s.hit_points), hp, "{who}: hit points");
    assert_eq!(s.hit_dice, format!("{level}d{die}"), "{who}: hit dice");
    // Armour class from the worn armour.
    let dex = s.modifier(Ability::Dex);
    let shield = s.armor.iter().any(|a| a == "Shield");
    let body = s.armor.iter().find(|a| *a != "Shield");
    let defense = s.features.iter().any(|f| f == "Fighting Style (Defense)");
    let expected_ac = match body {
        Some(name) => {
            let a = find(&equipment["armor"], "name", name);
            let base = i16::try_from(a["base_ac"].as_u64().unwrap()).unwrap();
            let dex_part = match (a["dex_bonus"].as_bool().unwrap(), a["max_dex"].as_i64()) {
                (false, _) => 0,
                (true, Some(cap)) => i16::from(dex).min(i16::try_from(cap).unwrap()),
                (true, None) => i16::from(dex),
            };
            let trained = class["armor_training"].as_array().unwrap();
            let category = a["category"].as_str().unwrap().to_lowercase();
            assert!(
                trained.iter().any(|t| t == category.as_str()),
                "{who}: untrained in {name}"
            );
            base + dex_part + i16::from(defense)
        }
        None => match (
            class["subclass"]["unarmored_base"].as_i64(),
            class["unarmored_bonus"].as_str(),
        ) {
            (Some(b), _) if level >= sub_level => i16::try_from(b).unwrap() + i16::from(dex),
            (_, Some(ab)) => {
                let ability = Ability::ALL.into_iter().find(|a| a.abbr() == ab).unwrap();
                10 + i16::from(dex) + i16::from(s.modifier(ability))
            }
            _ => 10 + i16::from(dex),
        },
    } + if shield { 2 } else { 0 };
    assert_eq!(i16::from(s.armor_class), expected_ac, "{who}: armour class");
    // Weapon attacks.
    let str_mod = s.modifier(Ability::Str);
    for attack in &s.attacks {
        if attack.name == "Unarmed Strike" {
            assert_eq!(attack.to_hit, str_mod.max(dex) + prof, "{who}: unarmed");
            continue;
        }
        let w = find(&equipment["weapons"], "name", &attack.name);
        let props: Vec<&str> = w["properties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        let ability = match attack.kind {
            AttackKind::Ranged => dex,
            AttackKind::Melee if props.contains(&"Finesse") || class["key"] == "monk" => {
                str_mod.max(dex)
            }
            AttackKind::Melee => str_mod,
        };
        let training: Vec<String> = class["weapon_training"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_lowercase())
            .collect();
        let category = w["category"].as_str().unwrap().to_lowercase();
        let proficient =
            training.contains(&category) || training.contains(&attack.name.to_lowercase());
        assert!(proficient, "{who}: carries {} untrained", attack.name);
        assert_eq!(
            attack.to_hit,
            ability + prof,
            "{who}: to-hit with {}",
            attack.name
        );
        assert!(
            attack
                .damage
                .starts_with(w["damage_dice"].as_str().unwrap()),
            "{who}: damage dice"
        );
    }
    // Spellcasting.
    if let Some(casting) = &s.spellcasting {
        assert_eq!(
            class["spell_ability"].as_str(),
            Some(casting.ability.abbr()),
            "{who}: casting ability"
        );
        let m = s.modifier(casting.ability);
        assert_eq!(
            i16::from(casting.save_dc),
            8 + i16::from(prof) + i16::from(m),
            "{who}: save DC"
        );
        assert_eq!(casting.attack_bonus, prof + m, "{who}: spell attack");
        let kind = match class["caster"].as_str().unwrap() {
            "full" => CasterKind::Full,
            "half" => CasterKind::Half,
            "pact" => CasterKind::Pact,
            _ => CasterKind::None,
        };
        assert_eq!(
            casting.slots,
            spell_slots(kind, level).unwrap(),
            "{who}: slots"
        );
        let max = u8::try_from(casting.slots.len()).unwrap();
        assert!(
            casting.spells.iter().all(|sp| sp.level <= max),
            "{who}: spell above slot level"
        );
    } else {
        let has_slots = !spell_slots(
            match class["caster"].as_str().unwrap() {
                "full" => CasterKind::Full,
                "half" => CasterKind::Half,
                "pact" => CasterKind::Pact,
                _ => CasterKind::None,
            },
            level,
        )
        .unwrap()
        .is_empty();
        assert!(!has_slots, "{who}: caster without spellcasting");
    }
}

#[test]
fn every_class_build_is_internally_consistent() {
    let (classes, races, equipment) = (
        srd("classes.json"),
        srd("races.json"),
        srd("equipment.json"),
    );
    let mut builds = 0;
    let mut seen_classes = std::collections::BTreeSet::new();
    for (profile, buildings) in [town(), port_town(), city(9_000)] {
        let population = arda_npc::generate_population(common::SEED, &profile, &buildings).unwrap();
        for npc in &population.npcs {
            if let SheetKind::Class { class, .. } = &npc.sheet.kind {
                seen_classes.insert(class.clone());
            }
            check_class_build(npc, &classes, &races, &equipment);
            builds += 1;
        }
    }
    assert!(builds > 100, "only {builds} class builds checked");
    assert!(seen_classes.len() >= 10, "classes seen: {seen_classes:?}");
}

#[test]
fn every_class_race_and_level_is_consistent() {
    let (classes, races, equipment) = (
        srd("classes.json"),
        srd("races.json"),
        srd("equipment.json"),
    );
    let lifestyles = [
        Lifestyle::Poor,
        Lifestyle::Comfortable,
        Lifestyle::Aristocratic,
    ];
    for class in classes.as_array().unwrap() {
        let class_key = class["key"].as_str().unwrap();
        for race in races.as_array().unwrap() {
            let race_key = race["key"].as_str().unwrap();
            for level in 1..=20u8 {
                let lifestyle = lifestyles[usize::from(level) % 3];
                let key = SeedKey::settlement(u64::from(level), SettlementId(7));
                let sheet =
                    class_sheet(class_key, race_key, level, "military", lifestyle, key).unwrap();
                let who = format!("{class_key} {level} {race_key}");
                check_sheet(&sheet, race_key, &who, &classes, &races, &equipment);
            }
        }
    }
}

#[test]
fn commoner_sheets_are_consistent() {
    let (profile, buildings) = town();
    for npc in everyone(&profile, &buildings) {
        let s = &npc.sheet;
        for i in 0..6 {
            assert_eq!(s.modifiers[i], common::modifier(s.abilities[i]));
        }
        assert_eq!(s.saves.len(), 6);
        assert!(s.hit_points > 0 && s.armor_class >= 10);
    }
}
