//! SRD 5.1 rule maths: ability modifiers, proficiency, hit points, armour
//! class, spell DCs and spell slots. Pure functions, checked by the tests.

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::error::NpcError;

/// One of the six ability scores, in SRD order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ability {
    /// Strength.
    #[serde(rename = "STR")]
    Str,
    /// Dexterity.
    #[serde(rename = "DEX")]
    Dex,
    /// Constitution.
    #[serde(rename = "CON")]
    Con,
    /// Intelligence.
    #[serde(rename = "INT")]
    Int,
    /// Wisdom.
    #[serde(rename = "WIS")]
    Wis,
    /// Charisma.
    #[serde(rename = "CHA")]
    Cha,
}

impl Ability {
    /// All six, in SRD order.
    pub const ALL: [Self; 6] = [
        Self::Str,
        Self::Dex,
        Self::Con,
        Self::Int,
        Self::Wis,
        Self::Cha,
    ];

    /// Position in SRD order (0–5).
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Str => 0,
            Self::Dex => 1,
            Self::Con => 2,
            Self::Int => 3,
            Self::Wis => 4,
            Self::Cha => 5,
        }
    }

    /// Three-letter abbreviation.
    #[must_use]
    pub fn abbr(self) -> &'static str {
        match self {
            Self::Str => "STR",
            Self::Dex => "DEX",
            Self::Con => "CON",
            Self::Int => "INT",
            Self::Wis => "WIS",
            Self::Cha => "CHA",
        }
    }
}

/// The ability each SRD skill uses.
pub const SKILLS: [(&str, Ability); 18] = [
    ("Acrobatics", Ability::Dex),
    ("Animal Handling", Ability::Wis),
    ("Arcana", Ability::Int),
    ("Athletics", Ability::Str),
    ("Deception", Ability::Cha),
    ("History", Ability::Int),
    ("Insight", Ability::Wis),
    ("Intimidation", Ability::Cha),
    ("Investigation", Ability::Int),
    ("Medicine", Ability::Wis),
    ("Nature", Ability::Int),
    ("Perception", Ability::Wis),
    ("Performance", Ability::Cha),
    ("Persuasion", Ability::Cha),
    ("Religion", Ability::Int),
    ("Sleight of Hand", Ability::Dex),
    ("Stealth", Ability::Dex),
    ("Survival", Ability::Wis),
];

/// The ability a skill uses, if `name` is an SRD skill.
#[must_use]
pub fn skill_ability(name: &str) -> Option<Ability> {
    SKILLS.iter().find(|(n, _)| *n == name).map(|&(_, a)| a)
}

/// How a class casts spells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasterKind {
    /// No spellcasting.
    None,
    /// Full caster (bard, cleric, druid, sorcerer, wizard).
    Full,
    /// Half caster (paladin, ranger).
    Half,
    /// Pact Magic (warlock).
    Pact,
}

/// Ability modifier: `floor((score - 10) / 2)`.
#[must_use]
pub fn modifier(score: u8) -> i8 {
    let delta = i16::from(score) - 10;
    i8::try_from(delta.div_euclid(2)).unwrap_or(0)
}

/// Proficiency bonus of a character level (1–20): `2 + (level - 1) / 4`.
#[must_use]
pub fn proficiency_bonus(level: u8) -> i8 {
    let level = level.clamp(1, 20);
    i8::try_from(2 + (level - 1) / 4).unwrap_or(2)
}

/// Proficiency bonus of a monster challenge rating such as "1/8" or "12".
#[must_use]
pub fn cr_proficiency(challenge_rating: &str) -> i8 {
    let cr: u8 = challenge_rating.parse().unwrap_or(0);
    match cr {
        0..=4 => 2,
        5..=8 => 3,
        9..=12 => 4,
        13..=16 => 5,
        17..=20 => 6,
        21..=24 => 7,
        25..=28 => 8,
        _ => 9,
    }
}

/// Hit points of a class build: the hit die maximum plus Con at level 1,
/// then the rounded-up average (`die / 2 + 1`) plus Con for every later
/// level, plus any flat per-level bonus (hill dwarf, draconic bloodline).
/// Each level grants at least 1 hit point.
#[must_use]
pub fn class_hit_points(level: u8, hit_die: u8, con_mod: i8, bonus_per_level: u8) -> u16 {
    let con = i32::from(con_mod);
    let bonus = i32::from(bonus_per_level);
    let first = (i32::from(hit_die) + con).max(1) + bonus;
    let later = (i32::from(hit_die) / 2 + 1 + con).max(1) + bonus;
    let total = first + later * (i32::from(level.max(1)) - 1);
    u16::try_from(total).unwrap_or(u16::MAX)
}

/// How much Dexterity counts toward armour class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexRule {
    /// Light armour or no armour: all of it.
    Full,
    /// Medium armour: up to a cap.
    Capped(u8),
    /// Heavy armour: none.
    Ignored,
}

/// Armour class: `base + Dex (per rule) + 2 for a shield + other bonuses`.
#[must_use]
pub fn armor_class(base: u8, dex_mod: i8, dex: DexRule, shield: bool, bonus: i8) -> u8 {
    let dex_part = match dex {
        DexRule::Full => i16::from(dex_mod),
        DexRule::Capped(cap) => i16::from(dex_mod).min(i16::from(cap)),
        DexRule::Ignored => 0,
    };
    let total = i16::from(base) + dex_part + if shield { 2 } else { 0 } + i16::from(bonus);
    u8::try_from(total.max(0)).unwrap_or(u8::MAX)
}

/// Spell save DC: `8 + proficiency + ability modifier`.
#[must_use]
pub fn spell_save_dc(proficiency: i8, ability_mod: i8) -> u8 {
    u8::try_from((8 + i16::from(proficiency) + i16::from(ability_mod)).max(0)).unwrap_or(0)
}

/// Spell attack bonus: `proficiency + ability modifier`.
#[must_use]
pub fn spell_attack_bonus(proficiency: i8, ability_mod: i8) -> i8 {
    proficiency.saturating_add(ability_mod)
}

/// Spell slots of a single-class caster, indexed by spell level − 1.
/// Pact Magic puts all its slots at the pact slot level.
///
/// # Errors
/// [`NpcError::Data`] when the bundled slot tables fail to load.
pub fn spell_slots(kind: CasterKind, level: u8) -> Result<Vec<u8>, NpcError> {
    let tables = &Data::get()?.srd.slots;
    let row = |caster_level: u8| -> Vec<u8> {
        usize::from(caster_level)
            .checked_sub(1)
            .and_then(|i| tables.full_caster_slots.get(i))
            .cloned()
            .unwrap_or_default()
    };
    Ok(match kind {
        CasterKind::None => Vec::new(),
        CasterKind::Full => row(level.min(20)),
        CasterKind::Half if level >= 2 => row(level.min(20).div_ceil(2)),
        CasterKind::Half => Vec::new(),
        CasterKind::Pact => {
            let Some(pact) = usize::from(level.clamp(1, 20))
                .checked_sub(1)
                .and_then(|i| tables.pact_slots.get(i))
            else {
                return Ok(Vec::new());
            };
            let mut slots = vec![0; usize::from(pact.level)];
            if let Some(last) = slots.last_mut() {
                *last = pact.slots;
            }
            slots
        }
    })
}

/// A parsed dice expression such as `2d6+4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dice {
    /// Number of dice.
    pub count: u8,
    /// Sides per die (0 for a flat value).
    pub sides: u8,
    /// Flat modifier.
    pub bonus: i8,
}

impl Dice {
    /// Parses `NdS`, `NdS+B`, `NdS-B`, `N d S + B` or a flat number.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let (dice, bonus) = match compact.find(['+', '-']) {
            Some(i) => {
                let (d, b) = compact.split_at(i);
                (d.to_string(), b.parse::<i8>().ok()?)
            }
            None => (compact.clone(), 0),
        };
        match dice.split_once('d') {
            Some((n, s)) => Some(Self {
                count: n.parse().ok()?,
                sides: s.parse().ok()?,
                bonus,
            }),
            None => Some(Self {
                count: dice.parse().ok()?,
                sides: 0,
                bonus,
            }),
        }
    }

    /// Average rounded down, as printed in SRD stat blocks (minimum 1 for dice).
    #[must_use]
    pub fn average(self) -> i16 {
        if self.sides == 0 {
            return i16::from(self.count) + i16::from(self.bonus);
        }
        let avg = i16::from(self.count) * (i16::from(self.sides) + 1) / 2 + i16::from(self.bonus);
        avg.max(1)
    }

    /// Formats as `NdS`, `NdS + B` or `NdS - B`.
    #[must_use]
    pub fn format(dice_count: u8, sides: u8, bonus: i8) -> String {
        match bonus {
            0 => format!("{dice_count}d{sides}"),
            b if b > 0 => format!("{dice_count}d{sides} + {b}"),
            b => format!("{dice_count}d{sides} - {}", -i16::from(b)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_round_down() {
        assert_eq!(modifier(1), -5);
        assert_eq!(modifier(9), -1);
        assert_eq!(modifier(10), 0);
        assert_eq!(modifier(11), 0);
        assert_eq!(modifier(20), 5);
    }

    #[test]
    fn dice_parse_and_average() {
        let d = Dice::parse("2d6+4");
        assert_eq!(
            d,
            Some(Dice {
                count: 2,
                sides: 6,
                bonus: 4
            })
        );
        assert_eq!(d.map(Dice::average), Some(11));
        assert_eq!(Dice::parse("1d4 - 1").map(Dice::average), Some(1));
        assert_eq!(Dice::format(1, 8, -1), "1d8 - 1");
    }
}
