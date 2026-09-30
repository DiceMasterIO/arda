//! Class builds for notables: SRD 5.1 class levels on an SRD race.

use crate::data::content::BackgroundData;
use crate::data::srd::{ArmorData, ClassData, RaceData};
use crate::data::Data;
use crate::error::NpcError;
use crate::rng::Rng;
use crate::rules::{
    armor_class, class_hit_points, modifier, proficiency_bonus, skill_ability, spell_attack_bonus,
    spell_save_dc, spell_slots, Ability, CasterKind, DexRule, Dice, SKILLS,
};
use crate::sheet::equipment::{add_item, Wealth};
use crate::sheet::spells::choose_spells;
use crate::sheet::stat_block::{resistances, weapon_range};
use crate::sheet::{
    Attack, AttackKind, Background, Coins, Item, SaveBonus, Sheet, SheetKind, SkillBonus,
    Spellcasting,
};

/// The SRD standard array, assigned in class priority order.
pub const STANDARD_ARRAY: [u8; 6] = [15, 14, 13, 12, 10, 8];

/// Everything a class build needs.
pub(crate) struct ClassRequest<'a> {
    pub class: &'a ClassData,
    pub race: &'a RaceData,
    pub level: u8,
    pub background: &'a BackgroundData,
    pub wealth: Wealth,
    pub items: &'a [Item],
    pub coins: Coins,
}

/// Ability scores: standard array by class priority, racial bonuses
/// (half-elves put their two floating +1s on the class's top non-Charisma
/// abilities), then +2 per Ability Score Improvement on the highest-priority
/// score below 20.
#[must_use]
pub fn class_abilities(class: &ClassData, race: &RaceData, level: u8) -> [u8; 6] {
    let mut scores = [10u8; 6];
    for (ability, &value) in class.priority.iter().zip(STANDARD_ARRAY.iter()) {
        scores[ability.index()] = value;
    }
    for (score, bonus) in scores.iter_mut().zip(race.ability_bonuses) {
        *score = score.saturating_add_signed(bonus);
    }
    let floating = class.priority.iter().filter(|&&a| a != Ability::Cha);
    for ability in floating.take(usize::from(race.flexible_bonuses)) {
        scores[ability.index()] += 1;
    }
    let improvements = class.asi_levels.iter().filter(|&&l| l <= level).count();
    for _ in 0..improvements * 2 {
        if let Some(ability) = class.priority.iter().find(|a| scores[a.index()] < 20) {
            scores[ability.index()] += 1;
        }
    }
    scores
}

/// Class and subclass feature names up to `level`.
#[must_use]
pub fn class_features(class: &ClassData, level: u8) -> Vec<String> {
    let mut features = Vec::new();
    for (&at, names) in class.features.range(..=level) {
        features.extend(names.iter().cloned());
        if at == 1 && class.key == "rogue" {
            features.push(format!("Sneak Attack ({}d6)", level.div_ceil(2)));
        }
    }
    if level >= class.subclass.level {
        for names in class.subclass.features.range(..=level).map(|(_, n)| n) {
            features.extend(names.iter().cloned());
        }
    }
    if class.key == "monk" {
        features.push(format!("Martial Arts (1d{})", martial_arts_die(level)));
    }
    features
}

/// Monk martial arts die by level.
#[must_use]
pub fn martial_arts_die(level: u8) -> u8 {
    match level {
        0..=4 => 4,
        5..=10 => 6,
        11..=16 => 8,
        _ => 10,
    }
}

/// Monk Unarmored Movement bonus by level.
fn unarmored_movement(level: u8) -> u8 {
    match level {
        0..=1 => 0,
        2..=5 => 10,
        6..=9 => 15,
        10..=13 => 20,
        14..=17 => 25,
        _ => 30,
    }
}

/// Armour worn: the loadout's choice for the wealth band, stepping down to
/// cheaper bands while Strength is below the armour's minimum.
pub(crate) fn chosen_armor<'a>(
    data: &'a Data,
    class: &ClassData,
    wealth: Wealth,
    strength: u8,
) -> Option<&'a ArmorData> {
    let options: Vec<&ArmorData> = class.loadout.armor[..=wealth.index()]
        .iter()
        .rev()
        .filter_map(|name| name.as_deref().and_then(|n| data.srd.armor(n)))
        .collect();
    options
        .iter()
        .copied()
        .find(|a| strength >= a.str_minimum)
        .or_else(|| options.last().copied())
}

/// Armour class and walking speed of a class build.
fn defence(
    data: &Data,
    req: &ClassRequest<'_>,
    scores: &[u8; 6],
    features: &[String],
) -> (u8, Vec<String>, u8) {
    let class = req.class;
    let mods = scores.map(modifier);
    let dex = mods[Ability::Dex.index()];
    let armor = chosen_armor(data, class, req.wealth, scores[Ability::Str.index()]);
    let shield = class.loadout.shield;
    let defense_style = features.iter().any(|f| f == "Fighting Style (Defense)");
    let mut worn = Vec::new();
    let mut speed = req.race.speed;
    let ac = match armor {
        Some(a) => {
            worn.push(a.name.clone());
            let rule = match (a.dex_bonus, a.max_dex) {
                (false, _) => DexRule::Ignored,
                (true, Some(cap)) => DexRule::Capped(cap),
                (true, None) => DexRule::Full,
            };
            if scores[Ability::Str.index()] < a.str_minimum {
                speed = speed.saturating_sub(10);
            }
            armor_class(a.base_ac, dex, rule, shield, i8::from(defense_style))
        }
        None => {
            let (base, extra) = match (class.subclass.unarmored_base, class.unarmored_bonus) {
                (Some(base), _) if req.level >= class.subclass.level => (base, 0),
                (_, Some(ability)) => (10, mods[ability.index()]),
                _ => (10, 0),
            };
            armor_class(base, dex, DexRule::Full, shield, extra)
        }
    };
    if shield {
        worn.push("Shield".to_string());
    }
    if class.key == "monk" {
        speed += unarmored_movement(req.level);
    }
    let heavy = armor.is_some_and(|a| a.category == "Heavy");
    if class.key == "barbarian" && req.level >= 5 && !heavy {
        speed += 10;
    }
    (ac, worn, speed)
}

/// Whether the class is trained with a weapon.
pub(crate) fn weapon_proficient(class: &ClassData, name: &str, category: &str) -> bool {
    class
        .weapon_training
        .iter()
        .any(|t| t == name || t.eq_ignore_ascii_case(category))
}

fn attacks(data: &Data, class: &ClassData, level: u8, mods: [i8; 6], prof: i8) -> Vec<Attack> {
    let str_mod = mods[Ability::Str.index()];
    let dex_mod = mods[Ability::Dex.index()];
    let mut out = Vec::new();
    for weapon in class
        .loadout
        .weapons
        .iter()
        .filter_map(|n| data.srd.weapon(n))
    {
        let ranged = weapon.range_kind == "Ranged";
        let monk_weapon = class.key == "monk"
            && (weapon.name == "Shortsword"
                || (weapon.category == "Simple"
                    && !weapon.has("Two-Handed")
                    && !weapon.has("Heavy")));
        let ability_mod = if ranged {
            dex_mod
        } else if weapon.has("Finesse") || monk_weapon {
            str_mod.max(dex_mod)
        } else {
            str_mod
        };
        let proficient = weapon_proficient(class, &weapon.name, &weapon.category);
        let dice = Dice::parse(&weapon.damage_dice).unwrap_or(Dice {
            count: 1,
            sides: 4,
            bonus: 0,
        });
        out.push(Attack {
            name: weapon.name.clone(),
            kind: if ranged {
                AttackKind::Ranged
            } else {
                AttackKind::Melee
            },
            to_hit: ability_mod + if proficient { prof } else { 0 },
            damage: Dice::format(dice.count, dice.sides, ability_mod),
            average: Dice {
                bonus: ability_mod,
                ..dice
            }
            .average(),
            damage_type: weapon.damage_type.clone(),
            range: weapon_range(weapon),
            extra: Vec::new(),
        });
    }
    if class.key == "monk" {
        let ability_mod = str_mod.max(dex_mod);
        let die = martial_arts_die(level);
        out.push(Attack {
            name: "Unarmed Strike".to_string(),
            kind: AttackKind::Melee,
            to_hit: ability_mod + prof,
            damage: Dice::format(1, die, ability_mod),
            average: Dice {
                count: 1,
                sides: die,
                bonus: ability_mod,
            }
            .average(),
            damage_type: "bludgeoning".to_string(),
            range: "5 ft.".to_string(),
            extra: Vec::new(),
        });
    }
    out
}

/// Proficient skills: background, race, half-elf picks, class picks, then
/// expertise for rogues (levels 1 and 6) and bards (levels 3 and 10).
fn skills(req: &ClassRequest<'_>, mods: [i8; 6], prof: i8, rng: &mut Rng) -> Vec<SkillBonus> {
    let mut names: Vec<String> = req.background.skills.clone();
    for s in &req.race.skills {
        if !names.contains(s) {
            names.push(s.clone());
        }
    }
    let mut pick_from = |pool: Vec<&str>, count: u8, names: &mut Vec<String>| {
        let mut pool: Vec<&str> = pool
            .into_iter()
            .filter(|s| !names.iter().any(|n| n == s))
            .collect();
        rng.shuffle(&mut pool);
        names.extend(
            pool.into_iter()
                .take(usize::from(count))
                .map(str::to_string),
        );
    };
    pick_from(
        SKILLS.iter().map(|&(n, _)| n).collect(),
        req.race.skill_choices,
        &mut names,
    );
    let class_pool = req.class.skills.iter().map(String::as_str).collect();
    pick_from(class_pool, req.class.skill_count, &mut names);
    let expertise_count = match (req.class.key.as_str(), req.level) {
        ("rogue", l) if l >= 6 => 4,
        ("rogue", _) => 2,
        ("bard", l) if l >= 10 => 4,
        ("bard", l) if l >= 3 => 2,
        _ => 0,
    };
    names.sort();
    let mut expert: Vec<&String> = names.iter().collect();
    rng.shuffle(&mut expert);
    let expert: Vec<String> = expert.into_iter().take(expertise_count).cloned().collect();
    names
        .iter()
        .filter_map(|name| {
            let ability = skill_ability(name)?;
            let expertise = expert.contains(name);
            let bonus = mods[ability.index()] + if expertise { 2 * prof } else { prof };
            Some(SkillBonus {
                name: name.clone(),
                ability,
                bonus,
                expertise,
            })
        })
        .collect()
}

/// Passive Perception: 10 + the Perception bonus, including Jack of All
/// Trades for bards when not proficient.
#[must_use]
pub fn passive_perception(
    skills: &[SkillBonus],
    wis_mod: i8,
    prof: i8,
    class: &str,
    level: u8,
) -> u8 {
    let bonus = skills.iter().find(|s| s.name == "Perception").map_or_else(
        || {
            wis_mod
                + if class == "bard" && level >= 2 {
                    prof / 2
                } else {
                    0
                }
        },
        |s| s.bonus,
    );
    u8::try_from((10 + i16::from(bonus)).max(0)).unwrap_or(10)
}

/// Builds the sheet of a class build.
pub(crate) fn build_class_sheet(
    data: &Data,
    req: &ClassRequest<'_>,
    rng: &mut Rng,
) -> Result<Sheet, NpcError> {
    let class = req.class;
    let level = req.level.clamp(1, 20);
    let abilities = class_abilities(class, req.race, level);
    let modifiers = abilities.map(modifier);
    let prof = proficiency_bonus(level);
    let saves = Ability::ALL
        .iter()
        .map(|&ability| {
            let proficient = class.saves.contains(&ability);
            let bonus = modifiers[ability.index()] + if proficient { prof } else { 0 };
            SaveBonus {
                ability,
                bonus,
                proficient,
            }
        })
        .collect();
    let skills = skills(req, modifiers, prof, rng);
    let mut features = req.race.traits.clone();
    features.extend(class_features(class, level));
    let (armor_class, armor, speed) = defence(data, req, &abilities, &features);
    let hp_bonus = req.race.hp_per_level
        + if level >= class.subclass.level {
            class.subclass.hp_per_level
        } else {
            0
        };
    let hit_points = class_hit_points(
        level,
        class.hit_die,
        modifiers[Ability::Con.index()],
        hp_bonus,
    );
    let spellcasting = spellcasting(data, class, level, modifiers, prof, rng)?;
    let mut equipment = Vec::new();
    for name in armor.iter().chain(&class.loadout.weapons) {
        add_item(&mut equipment, name, 1);
    }
    for weapon in class
        .loadout
        .weapons
        .iter()
        .filter_map(|n| data.srd.weapon(n))
    {
        let ammo = match weapon.name.as_str() {
            "Longbow" | "Shortbow" => Some("Arrow"),
            "Crossbow, light" | "Crossbow, heavy" | "Crossbow, hand" => Some("Crossbow bolt"),
            "Sling" => Some("Sling bullet"),
            "Dart" | "Javelin" => Some(weapon.name.as_str()),
            _ => None,
        };
        if let Some(ammo) = ammo {
            let count = if ammo == weapon.name { 4 } else { 20 };
            add_item(&mut equipment, ammo, count);
        }
    }
    for item in req.items {
        add_item(&mut equipment, &item.name, item.quantity);
    }
    for focus in class_focus(&class.key) {
        if !equipment.iter().any(|i| i.name == *focus) {
            add_item(&mut equipment, focus, 1);
        }
    }
    let mut languages = req.race.languages.clone();
    let mut pool: Vec<&String> = data
        .srd
        .languages
        .standard
        .iter()
        .filter(|l| !languages.contains(l))
        .collect();
    rng.shuffle(&mut pool);
    languages.extend(
        pool.into_iter()
            .take(usize::from(req.race.extra_languages))
            .cloned(),
    );
    match class.key.as_str() {
        "rogue" => languages.push("Thieves' Cant".to_string()),
        "druid" => languages.push("Druidic".to_string()),
        _ => {}
    }
    let mut senses = Vec::new();
    if req.race.darkvision > 0 {
        senses.push(format!("darkvision {} ft.", req.race.darkvision));
    }
    let wis = modifiers[Ability::Wis.index()];
    Ok(Sheet {
        kind: SheetKind::Class {
            class: class.name.clone(),
            subclass: (level >= class.subclass.level).then(|| class.subclass.name.clone()),
            level,
            background: Background {
                name: req.background.name.clone(),
                text: req.background.text.clone(),
                skills: req.background.skills.clone(),
            },
        },
        size: req.race.size.clone(),
        abilities,
        modifiers,
        proficiency_bonus: prof,
        saves,
        passive_perception: passive_perception(&skills, wis, prof, &class.key, level),
        skills,
        hit_points,
        hit_dice: format!("{level}d{}", class.hit_die),
        armor_class,
        armor,
        speed,
        senses,
        languages,
        damage_resistances: resistances(&[], req.race, rng),
        attacks: attacks(data, class, level, modifiers, prof),
        features,
        equipment,
        coins: req.coins,
        spellcasting,
    })
}

/// Spellcasting focus or book carried by the class.
fn class_focus(class: &str) -> &'static [&'static str] {
    match class {
        "cleric" | "paladin" => &["Amulet"],
        "druid" => &["Sprig of mistletoe"],
        "wizard" => &["Spellbook", "Component pouch"],
        "bard" => &["Lute", "Component pouch"],
        "sorcerer" | "warlock" => &["Component pouch"],
        _ => &[],
    }
}

fn spellcasting(
    data: &Data,
    class: &ClassData,
    level: u8,
    mods: [i8; 6],
    prof: i8,
    rng: &mut Rng,
) -> Result<Option<Spellcasting>, NpcError> {
    let Some(ability) = class.spell_ability else {
        return Ok(None);
    };
    let slots = spell_slots(class.caster, level)?;
    if slots.is_empty() {
        return Ok(None);
    }
    let ability_mod = mods[ability.index()];
    Ok(Some(Spellcasting {
        ability,
        save_dc: spell_save_dc(prof, ability_mod),
        attack_bonus: spell_attack_bonus(prof, ability_mod),
        spells: choose_spells(data, class, level, &slots, rng),
        pact_magic: class.caster == CasterKind::Pact,
        slots,
    }))
}
