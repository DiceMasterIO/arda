//! Typed view of the SRD 5.1 JSON files in `data/srd/`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::rules::{Ability, CasterKind};

/// An SRD race (with its SRD subrace folded in).
#[derive(Debug, Clone, Deserialize)]
pub struct RaceData {
    pub key: String,
    pub name: String,
    pub subrace: Option<String>,
    pub size: String,
    pub speed: u8,
    pub darkvision: u16,
    pub ability_bonuses: [i8; 6],
    pub flexible_bonuses: u8,
    pub languages: Vec<String>,
    pub extra_languages: u8,
    pub traits: Vec<String>,
    pub hp_per_level: u8,
    pub skills: Vec<String>,
    pub skill_choices: u8,
    pub resistances: Vec<String>,
    pub resistance_choices: Vec<String>,
    pub adult_age: u16,
    pub max_age: u16,
    pub working_age: u16,
    pub elder_age: u16,
}

/// An SRD subclass.
#[derive(Debug, Clone, Deserialize)]
pub struct SubclassData {
    pub name: String,
    pub level: u8,
    pub features: BTreeMap<u8, Vec<String>>,
    pub hp_per_level: u8,
    pub unarmored_base: Option<u8>,
}

/// Default equipment for a class, by wealth band (poor, modest, rich).
#[derive(Debug, Clone, Deserialize)]
pub struct Loadout {
    pub armor: [Option<String>; 3],
    pub shield: bool,
    pub weapons: Vec<String>,
}

/// An SRD class.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassData {
    pub key: String,
    pub name: String,
    pub hit_die: u8,
    pub priority: Vec<Ability>,
    pub saves: Vec<Ability>,
    pub armor_training: Vec<String>,
    pub weapon_training: Vec<String>,
    pub skill_count: u8,
    pub skills: Vec<String>,
    pub subclass: SubclassData,
    pub features: BTreeMap<u8, Vec<String>>,
    pub caster: CasterKind,
    pub spell_ability: Option<Ability>,
    pub cantrips_known: [u8; 3],
    pub unarmored_bonus: Option<Ability>,
    pub asi_levels: Vec<u8>,
    pub loadout: Loadout,
}

/// One row of the Pact Magic table.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PactRow {
    pub slots: u8,
    pub level: u8,
}

/// SRD spell-slot tables.
#[derive(Debug, Clone, Deserialize)]
pub struct SlotTables {
    pub full_caster_slots: Vec<Vec<u8>>,
    pub pact_slots: Vec<PactRow>,
}

/// An SRD spell, by name and level only.
#[derive(Debug, Clone, Deserialize)]
pub struct SpellData {
    pub name: String,
    pub level: u8,
    pub classes: Vec<String>,
}

/// Additional damage on an SRD stat-block attack.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExtraDamage {
    pub dice: String,
    #[serde(rename = "type")]
    pub damage_type: String,
}

/// An attack action of an SRD stat block.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionData {
    pub name: String,
    pub attack_bonus: i8,
    pub damage: String,
    pub damage_type: String,
    pub extra_damage: Vec<ExtraDamage>,
}

/// Spellcasting of an SRD stat block.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockSpellcasting {
    pub ability: Ability,
    pub save_dc: u8,
    pub attack_bonus: i8,
    pub slots: Vec<u8>,
    pub spells: Vec<(String, u8)>,
}

/// An SRD NPC stat block.
#[derive(Debug, Clone, Deserialize)]
pub struct StatBlockData {
    pub name: String,
    pub armor_class: u8,
    pub armor: Vec<String>,
    pub hit_points: u16,
    pub hit_dice: String,
    pub speed: u8,
    pub abilities: [u8; 6],
    pub saves: BTreeMap<Ability, i8>,
    pub skills: BTreeMap<String, i8>,
    pub passive_perception: u8,
    pub languages: String,
    pub challenge_rating: String,
    pub xp: u32,
    pub damage_resistances: Vec<String>,
    pub traits: Vec<String>,
    pub reactions: Vec<String>,
    pub multiattack: bool,
    pub actions: Vec<ActionData>,
    pub spellcasting: Option<BlockSpellcasting>,
    pub equipment: Vec<String>,
}

/// SRD armour.
#[derive(Debug, Clone, Deserialize)]
pub struct ArmorData {
    pub name: String,
    pub category: String,
    pub base_ac: u8,
    pub dex_bonus: bool,
    pub max_dex: Option<u8>,
    pub str_minimum: u8,
}

/// An SRD weapon.
#[derive(Debug, Clone, Deserialize)]
pub struct WeaponData {
    pub name: String,
    pub category: String,
    pub range_kind: String,
    pub damage_dice: String,
    pub damage_type: String,
    pub range: Option<(u16, Option<u16>)>,
    pub properties: Vec<String>,
}

impl WeaponData {
    /// Whether the weapon has a property such as "Finesse".
    #[must_use]
    pub fn has(&self, property: &str) -> bool {
        self.properties.iter().any(|p| p == property)
    }
}

/// SRD equipment lists.
#[derive(Debug, Clone, Deserialize)]
pub struct EquipmentData {
    pub armor: Vec<ArmorData>,
    pub weapons: Vec<WeaponData>,
    pub gear: Vec<String>,
    pub tools: Vec<String>,
}

/// SRD languages.
#[derive(Debug, Clone, Deserialize)]
pub struct Languages {
    pub standard: Vec<String>,
    pub secret: Vec<String>,
}

/// Everything under `data/srd/`.
#[derive(Debug, Clone)]
pub struct Srd {
    pub races: Vec<RaceData>,
    pub classes: Vec<ClassData>,
    pub slots: SlotTables,
    pub spells: Vec<SpellData>,
    pub stat_blocks: Vec<StatBlockData>,
    pub equipment: EquipmentData,
    pub languages: Languages,
}

impl Srd {
    /// Race by key ("human", "half-elf", …).
    #[must_use]
    pub fn race(&self, key: &str) -> Option<&RaceData> {
        self.races.iter().find(|r| r.key == key)
    }

    /// Class by key ("fighter", …).
    #[must_use]
    pub fn class(&self, key: &str) -> Option<&ClassData> {
        self.classes.iter().find(|c| c.key == key)
    }

    /// Stat block by name ("Guard", …).
    #[must_use]
    pub fn stat_block(&self, name: &str) -> Option<&StatBlockData> {
        self.stat_blocks.iter().find(|b| b.name == name)
    }

    /// Armour by name.
    #[must_use]
    pub fn armor(&self, name: &str) -> Option<&ArmorData> {
        self.equipment.armor.iter().find(|a| a.name == name)
    }

    /// Weapon by name.
    #[must_use]
    pub fn weapon(&self, name: &str) -> Option<&WeaponData> {
        self.equipment.weapons.iter().find(|w| w.name == name)
    }

    /// Whether `name` is any SRD armour, weapon, gear or tool.
    #[must_use]
    pub fn is_item(&self, name: &str) -> bool {
        self.armor(name).is_some()
            || self.weapon(name).is_some()
            || self.equipment.gear.iter().any(|g| g == name)
            || self.equipment.tools.iter().any(|t| t == name)
    }

    /// Spell by name.
    #[must_use]
    pub fn spell(&self, name: &str) -> Option<&SpellData> {
        self.spells.iter().find(|s| s.name == name)
    }
}
