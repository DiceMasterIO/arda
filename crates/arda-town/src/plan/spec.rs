//! `arda-npc` `BuildingSpec`-compatible records, one per plan building, so
//! the NPC generator can populate the town directly (goals 45, 52, 55).
//!
//! Capacity and workplace slots mirror `arda-npc`'s `sample.rs` tables;
//! barns and jetties (ancillary) house and employ nobody.

use super::types::{BuildingId, TownPlan};
use crate::function::BuildingFunction as F;
use crate::site::SettlementId;
use serde::{Deserialize, Serialize};

/// One building as `arda-npc` reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSpec {
    /// Building id (same as the plan and the tactical sidecar).
    pub id: BuildingId,
    /// Owning settlement.
    pub settlement_id: SettlementId,
    /// Function, a plain snake_case key.
    pub function: F,
    /// How many people can live here.
    pub capacity: u16,
    /// How many people work here, master included.
    pub workplace_slots: u16,
    /// Building wealth, 0–255.
    pub wealth: u8,
    /// Free `key:value` tags (`craft:*` on workshops).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Residents a building can house (`arda-npc` `sample::capacity`).
#[must_use]
pub const fn capacity(f: F) -> u16 {
    match f {
        F::House => 6,
        F::Farmhouse => 7,
        F::Cottage => 4,
        F::Inn | F::Manor => 8,
        F::Keep => 14,
        F::Barracks => 20,
        F::Tavern
        | F::Temple
        | F::Guardhouse
        | F::Workshop
        | F::Bakery
        | F::Brewery
        | F::Tannery
        | F::Apothecary
        | F::School => 4,
        F::Smithy | F::Mill => 5,
        F::Shrine | F::Stable | F::Boathouse | F::Library => 3,
        F::Mine | F::LumberCamp => 6,
        F::MarketHall | F::Stall | F::Warehouse | F::Dock | F::Barn => 0,
    }
}

/// Workers a building employs (`arda-npc` `sample::slots`).
#[must_use]
pub const fn workplace_slots(f: F) -> u16 {
    match f {
        F::House | F::Farmhouse | F::Cottage | F::Barn => 0,
        F::Smithy | F::Workshop | F::Stable => 3,
        F::Inn | F::Warehouse | F::Dock => 5,
        F::Tavern | F::Temple | F::MarketHall | F::Guardhouse | F::Library => 4,
        F::Shrine | F::Boathouse | F::Brewery => 3,
        F::Keep => 7,
        F::Barracks => 12,
        F::Manor | F::LumberCamp => 5,
        F::Mine => 8,
        F::Mill | F::Stall | F::Bakery | F::Tannery | F::Apothecary | F::School => 2,
    }
}

/// Every building of the plan as a `BuildingSpec`.
#[must_use]
pub fn specs(plan: &TownPlan) -> Vec<BuildingSpec> {
    plan.buildings
        .iter()
        .map(|b| BuildingSpec {
            id: b.id,
            settlement_id: plan.site,
            function: b.function,
            capacity: if b.ancillary { 0 } else { capacity(b.function) },
            workplace_slots: if b.ancillary {
                0
            } else {
                workplace_slots(b.function)
            },
            wealth: b.wealth,
            tags: b.tags.clone(),
        })
        .collect()
}
