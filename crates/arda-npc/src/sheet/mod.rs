//! SRD 5.1 character sheets: an NPC stat block or a class build.

mod class_build;
mod equipment;
mod spells;
mod stat_block;

pub(crate) use class_build::{build_class_sheet, ClassRequest};
pub(crate) use equipment::{coins as purse, job_items as equipment_for, Wealth};
pub(crate) use stat_block::build_block_sheet;

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::error::NpcError;
use crate::npc::Lifestyle;
use crate::rng::SeedKey;
use crate::rules::Ability;

/// What the sheet is built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SheetKind {
    /// An SRD NPC stat block, used as printed.
    StatBlock {
        /// Stat block name, e.g. "Guard".
        name: String,
        /// Challenge rating, e.g. "1/8".
        challenge_rating: String,
        /// Experience points.
        xp: u32,
    },
    /// A character built with SRD class levels.
    Class {
        /// Class name, e.g. "Fighter".
        class: String,
        /// SRD subclass, once the level allows it.
        subclass: Option<String>,
        /// Class level, 1–20.
        level: u8,
        /// Original flavour background.
        background: Background,
    },
}

/// Original flavour background of a class build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Background {
    /// Name, e.g. "Wall-Sworn".
    pub name: String,
    /// One-line description.
    pub text: String,
    /// The two skills it grants.
    pub skills: Vec<String>,
}

/// A saving throw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveBonus {
    /// Ability.
    pub ability: Ability,
    /// Total bonus.
    pub bonus: i8,
    /// Whether proficiency applies.
    pub proficient: bool,
}

/// A proficient skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillBonus {
    /// SRD skill name.
    pub name: String,
    /// Ability used.
    pub ability: Ability,
    /// Total bonus.
    pub bonus: i8,
    /// Whether proficiency is doubled.
    pub expertise: bool,
}

/// Melee or ranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackKind {
    /// Melee weapon attack.
    Melee,
    /// Ranged weapon attack.
    Ranged,
}

/// One attack line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attack {
    /// Weapon or attack name.
    pub name: String,
    /// Melee or ranged.
    pub kind: AttackKind,
    /// Attack bonus.
    pub to_hit: i8,
    /// Damage dice, e.g. "1d8 + 3".
    pub damage: String,
    /// Average damage.
    pub average: i16,
    /// Damage type, e.g. "slashing".
    pub damage_type: String,
    /// Reach or range, e.g. "5 ft." or "80/320 ft.".
    pub range: String,
    /// Extra effects, e.g. "plus 7d6 poison".
    pub extra: Vec<String>,
}

/// An item and how many.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// SRD item name.
    pub name: String,
    /// Quantity.
    pub quantity: u16,
}

/// Coins carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Coins {
    /// Gold pieces.
    pub gp: u32,
    /// Silver pieces.
    pub sp: u32,
    /// Copper pieces.
    pub cp: u32,
}

/// A spell known or prepared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellRef {
    /// SRD spell name.
    pub name: String,
    /// Spell level, 0 for cantrips.
    pub level: u8,
}

/// Spellcasting block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spellcasting {
    /// Casting ability.
    pub ability: Ability,
    /// Spell save DC.
    pub save_dc: u8,
    /// Spell attack bonus.
    pub attack_bonus: i8,
    /// Slots per spell level (index 0 = 1st level).
    pub slots: Vec<u8>,
    /// Whether the slots are Pact Magic slots.
    pub pact_magic: bool,
    /// Cantrips and spells.
    pub spells: Vec<SpellRef>,
}

/// A full SRD 5.1 sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sheet {
    /// Stat block or class build.
    pub kind: SheetKind,
    /// Size category.
    pub size: String,
    /// STR, DEX, CON, INT, WIS, CHA.
    pub abilities: [u8; 6],
    /// Their modifiers.
    pub modifiers: [i8; 6],
    /// Proficiency bonus.
    pub proficiency_bonus: i8,
    /// All six saving throws.
    pub saves: Vec<SaveBonus>,
    /// Proficient skills.
    pub skills: Vec<SkillBonus>,
    /// Hit points.
    pub hit_points: u16,
    /// Hit dice, e.g. "5d8".
    pub hit_dice: String,
    /// Armour class.
    pub armor_class: u8,
    /// Worn armour and shield.
    pub armor: Vec<String>,
    /// Walking speed in feet.
    pub speed: u8,
    /// Senses, e.g. "darkvision 60 ft.".
    pub senses: Vec<String>,
    /// Passive Wisdom (Perception).
    pub passive_perception: u8,
    /// Languages.
    pub languages: Vec<String>,
    /// Damage resistances.
    pub damage_resistances: Vec<String>,
    /// Attacks.
    pub attacks: Vec<Attack>,
    /// Class, race and stat-block features by name.
    pub features: Vec<String>,
    /// Equipment carried.
    pub equipment: Vec<Item>,
    /// Coins carried.
    pub coins: Coins,
    /// Spellcasting, for casters.
    pub spellcasting: Option<Spellcasting>,
}

impl Sheet {
    /// Modifier of one ability.
    #[must_use]
    pub fn modifier(&self, ability: Ability) -> i8 {
        self.modifiers[ability.index()]
    }

    /// Class level, or 0 for a stat block.
    #[must_use]
    pub fn level(&self) -> u8 {
        match self.kind {
            SheetKind::Class { level, .. } => level,
            SheetKind::StatBlock { .. } => 0,
        }
    }
}

/// Builds a class-build sheet directly, e.g. for a GM-made notable.
/// `background` is a job category key such as `military`; items and coins
/// follow the lifestyle. Deterministic in `key`.
///
/// # Errors
/// [`NpcError::Data`] for an unknown class, ancestry or background.
pub fn class_sheet(
    class: &str,
    ancestry: &str,
    level: u8,
    background: &str,
    lifestyle: Lifestyle,
    key: SeedKey,
) -> Result<Sheet, NpcError> {
    let data = Data::get()?;
    let class = data
        .srd
        .class(class)
        .ok_or_else(|| NpcError::Data(format!("unknown class {class}")))?;
    let race = data
        .srd
        .race(ancestry)
        .ok_or_else(|| NpcError::Data(format!("unknown ancestry {ancestry}")))?;
    let background = data
        .backgrounds
        .backgrounds
        .get(background)
        .ok_or_else(|| NpcError::Data(format!("unknown background {background}")))?;
    let clothes = if lifestyle >= Lifestyle::Wealthy {
        "Clothes, fine"
    } else {
        "Clothes, common"
    };
    let items = [Item {
        name: clothes.to_string(),
        quantity: 1,
    }];
    let request = ClassRequest {
        class,
        race,
        level,
        background,
        wealth: Wealth::from_lifestyle(lifestyle),
        items: &items,
        coins: purse(lifestyle, &mut key.rng("coins")),
    };
    build_class_sheet(data, &request, &mut key.rng("sheet"))
}
