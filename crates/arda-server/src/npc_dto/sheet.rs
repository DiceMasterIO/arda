//! Mirrors of the `arda-npc` SRD 5.1 sheet types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One of the six abilities, in SRD order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
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

/// What the sheet is built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SheetKind {
    /// An SRD NPC stat block.
    StatBlock {
        /// Stat block name.
        name: String,
        /// Challenge rating.
        challenge_rating: String,
        /// Experience points.
        xp: u32,
    },
    /// A class build.
    Class {
        /// Class name.
        class: String,
        /// SRD subclass.
        subclass: Option<String>,
        /// Class level.
        level: u8,
        /// Flavour background.
        background: Background,
    },
}

/// Flavour background of a class build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Background {
    /// Name.
    pub name: String,
    /// Description.
    pub text: String,
    /// Granted skills.
    pub skills: Vec<String>,
}

/// A saving throw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveBonus {
    /// Ability.
    pub ability: Ability,
    /// Total bonus.
    pub bonus: i8,
    /// Whether proficiency applies.
    pub proficient: bool,
}

/// A proficient skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AttackKind {
    /// Melee.
    Melee,
    /// Ranged.
    Ranged,
}

/// One attack line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Attack {
    /// Name.
    pub name: String,
    /// Melee or ranged.
    pub kind: AttackKind,
    /// Attack bonus.
    pub to_hit: i8,
    /// Damage dice.
    pub damage: String,
    /// Average damage.
    pub average: i16,
    /// Damage type.
    pub damage_type: String,
    /// Reach or range.
    pub range: String,
    /// Extra effects.
    pub extra: Vec<String>,
}

/// An item and how many.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// SRD item name.
    pub name: String,
    /// Quantity.
    pub quantity: u16,
}

/// Coins carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Coins {
    /// Gold.
    pub gp: u32,
    /// Silver.
    pub sp: u32,
    /// Copper.
    pub cp: u32,
}

/// A spell known or prepared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SpellRef {
    /// SRD spell name.
    pub name: String,
    /// Spell level, 0 for cantrips.
    pub level: u8,
}

/// Spellcasting block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Spellcasting {
    /// Casting ability.
    pub ability: Ability,
    /// Spell save DC.
    pub save_dc: u8,
    /// Spell attack bonus.
    pub attack_bonus: i8,
    /// Slots per spell level (index 0 = 1st level).
    pub slots: Vec<u8>,
    /// Pact Magic slots.
    pub pact_magic: bool,
    /// Cantrips and spells.
    pub spells: Vec<SpellRef>,
}

/// A full SRD 5.1 sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
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
    /// Hit dice.
    pub hit_dice: String,
    /// Armour class.
    pub armor_class: u8,
    /// Worn armour and shield.
    pub armor: Vec<String>,
    /// Walking speed, feet.
    pub speed: u8,
    /// Senses.
    pub senses: Vec<String>,
    /// Passive Wisdom (Perception).
    pub passive_perception: u8,
    /// Languages.
    pub languages: Vec<String>,
    /// Damage resistances.
    pub damage_resistances: Vec<String>,
    /// Attacks.
    pub attacks: Vec<Attack>,
    /// Features by name.
    pub features: Vec<String>,
    /// Equipment.
    pub equipment: Vec<Item>,
    /// Coins.
    pub coins: Coins,
    /// Spellcasting, for casters.
    pub spellcasting: Option<Spellcasting>,
}
