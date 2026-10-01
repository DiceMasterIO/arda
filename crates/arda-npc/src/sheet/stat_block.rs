//! Sheets that use an SRD NPC stat block as printed.
//!
//! The block's numbers are kept exactly (abilities, AC, HP, attacks,
//! spellcasting). Following the SRD appendix note that NPC blocks can take
//! racial traits, the race contributes size, a slower walking speed,
//! darkvision, languages, resistances and trait names only.

use crate::data::srd::{RaceData, StatBlockData, WeaponData};
use crate::data::Data;
use crate::rng::Rng;
use crate::rules::{cr_proficiency, modifier, Ability, Dice};
use crate::sheet::equipment::add_item;
use crate::sheet::{
    Attack, AttackKind, Coins, Item, SaveBonus, Sheet, SheetKind, SkillBonus, SpellRef,
    Spellcasting,
};

/// Builds a stat-block sheet for someone of `race` carrying `items`.
pub(crate) fn build_block_sheet(
    data: &Data,
    block: &StatBlockData,
    race: &RaceData,
    items: &[Item],
    coins: Coins,
    rng: &mut Rng,
) -> Sheet {
    let modifiers = block.abilities.map(modifier);
    let proficiency_bonus = cr_proficiency(&block.challenge_rating);
    let saves = Ability::ALL
        .iter()
        .map(|&ability| match block.saves.get(&ability) {
            Some(&bonus) => SaveBonus {
                ability,
                bonus,
                proficient: true,
            },
            None => SaveBonus {
                ability,
                bonus: modifiers[ability.index()],
                proficient: false,
            },
        })
        .collect();
    let skills = block
        .skills
        .iter()
        .filter_map(|(name, &bonus)| {
            let ability = crate::rules::skill_ability(name)?;
            let expertise = bonus - modifiers[ability.index()] == 2 * proficiency_bonus;
            Some(SkillBonus {
                name: name.clone(),
                ability,
                bonus,
                expertise,
            })
        })
        .collect();
    let speed = if race.speed < 30 {
        block.speed.min(race.speed)
    } else {
        block.speed
    };
    let mut senses = Vec::new();
    if race.darkvision > 0 {
        senses.push(format!("darkvision {} ft.", race.darkvision));
    }
    let mut features: Vec<String> = block.traits.clone();
    if block.multiattack {
        features.push("Multiattack".to_string());
    }
    features.extend(block.reactions.iter().map(|r| format!("{r} (reaction)")));
    features.extend(race.traits.iter().cloned());
    let mut equipment = Vec::new();
    for name in &block.equipment {
        add_item(&mut equipment, name, 1);
    }
    for item in items {
        add_item(&mut equipment, &item.name, item.quantity);
    }
    Sheet {
        kind: SheetKind::StatBlock {
            name: block.name.clone(),
            challenge_rating: block.challenge_rating.clone(),
            xp: block.xp,
        },
        size: race.size.clone(),
        abilities: block.abilities,
        modifiers,
        proficiency_bonus,
        saves,
        skills,
        hit_points: block.hit_points,
        hit_dice: block.hit_dice.clone(),
        armor_class: block.armor_class,
        armor: block.armor.clone(),
        speed,
        senses,
        passive_perception: block.passive_perception,
        languages: block_languages(data, &block.languages, race, rng),
        damage_resistances: resistances(&block.damage_resistances, race, rng),
        attacks: block
            .actions
            .iter()
            .map(|a| block_attack(data, a))
            .collect(),
        features,
        equipment,
        coins,
        spellcasting: block.spellcasting.as_ref().map(|s| Spellcasting {
            ability: s.ability,
            save_dc: s.save_dc,
            attack_bonus: s.attack_bonus,
            slots: s.slots.clone(),
            pact_magic: false,
            spells: s
                .spells
                .iter()
                .map(|(n, l)| SpellRef {
                    name: n.clone(),
                    level: *l,
                })
                .collect(),
        }),
    }
}

fn block_attack(data: &Data, action: &crate::data::srd::ActionData) -> Attack {
    let weapon = weapon_for(data, &action.name);
    let ranged = weapon.is_some_and(|w| w.range_kind == "Ranged");
    let dice = Dice::parse(&action.damage);
    Attack {
        name: action.name.clone(),
        kind: if ranged {
            AttackKind::Ranged
        } else {
            AttackKind::Melee
        },
        to_hit: action.attack_bonus,
        damage: dice.map_or_else(
            || action.damage.clone(),
            |d| Dice::format(d.count, d.sides, d.bonus),
        ),
        average: dice.map_or(0, Dice::average),
        damage_type: action.damage_type.clone(),
        range: weapon.map_or_else(|| "5 ft.".to_string(), weapon_range),
        extra: action
            .extra_damage
            .iter()
            .map(|e| format!("plus {} {}", e.dice, e.damage_type))
            .collect(),
    }
}

/// The SRD weapon behind a stat-block action name ("Light Crossbow" →
/// "Crossbow, light").
pub(crate) fn weapon_for<'a>(data: &'a Data, action: &str) -> Option<&'a WeaponData> {
    data.srd.weapon(action).or_else(|| {
        let (kind, base) = action.split_once(' ')?;
        data.srd.weapon(&format!("{base}, {}", kind.to_lowercase()))
    })
}

/// "5 ft.", "80/320 ft." or "5 ft. or 20/60 ft." for thrown weapons.
pub(crate) fn weapon_range(weapon: &WeaponData) -> String {
    let reach = if weapon.has("Reach") { 10 } else { 5 };
    match (weapon.range_kind.as_str(), weapon.range) {
        ("Ranged", Some((normal, Some(long)))) => format!("{normal}/{long} ft."),
        ("Ranged", Some((normal, None))) => format!("{normal} ft."),
        (_, Some((normal, Some(long)))) => format!("{reach} ft. or {normal}/{long} ft."),
        _ => format!("{reach} ft."),
    }
}

/// Race languages first, topped up from the SRD standard languages to the
/// count the block allows ("any two languages").
fn block_languages(data: &Data, text: &str, race: &RaceData, rng: &mut Rng) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut languages: Vec<String> = race.languages.clone();
    for secret in &data.srd.languages.secret {
        if lower.contains(&secret.to_lowercase()) {
            languages.push(secret.clone());
        }
    }
    let words: [(&str, usize); 5] = [
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
    ];
    let wanted = words
        .iter()
        .find(|(w, _)| lower.contains(w))
        .map_or(1, |&(_, n)| n);
    let spoken = race.languages.len();
    let mut pool: Vec<&String> = data
        .srd
        .languages
        .standard
        .iter()
        .filter(|l| !languages.contains(l))
        .collect();
    rng.shuffle(&mut pool);
    for extra in pool.into_iter().take(wanted.saturating_sub(spoken)) {
        languages.push(extra.clone());
    }
    languages
}

/// Stat-block resistances plus racial ones.
pub(crate) fn resistances(block: &[String], race: &RaceData, rng: &mut Rng) -> Vec<String> {
    let mut out: Vec<String> = block.to_vec();
    out.extend(race.resistances.iter().cloned());
    if let Some(choice) = rng.pick(&race.resistance_choices) {
        out.push(choice.clone());
    }
    dedup_keep_first(&mut out);
    out
}

/// Drops every repeat, adjacent or not, keeping first occurrences in order
/// (`Vec::dedup` only drops adjacent ones; review round 1 #20).
fn dedup_keep_first(list: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    list.retain(|r| seen.insert(r.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resistance_listed_twice_apart_is_kept_once() {
        let mut list: Vec<String> = ["poison", "fire", "poison", "fire", "cold"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        dedup_keep_first(&mut list);
        assert_eq!(list, ["poison", "fire", "cold"]);
    }
}
