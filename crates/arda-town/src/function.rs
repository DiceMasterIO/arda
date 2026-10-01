//! Building functions from the shared vocabulary
//! (`docs/goal-prompts/vocabulary.md`, "Building functions", plus the
//! canonical additions `market_hall`, `mine`, `lumber_camp` and `school`)
//! and the footprint, storey and placement rules the planner uses for each.
//! Serialised as plain snake_case strings (convention I6, I7).

use crate::site::Tier;
use serde::{Deserialize, Serialize};

/// A building function key (vocabulary `function:*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BuildingFunction {
    /// Town house.
    House,
    /// Farmhouse (long house with byre).
    Farmhouse,
    /// One-room cottage.
    Cottage,
    /// Manor house.
    Manor,
    /// Inn with rooms.
    Inn,
    /// Tavern.
    Tavern,
    /// Bakery.
    Bakery,
    /// Brewery.
    Brewery,
    /// Mill.
    Mill,
    /// Smithy.
    Smithy,
    /// Workshop.
    Workshop,
    /// Tannery.
    Tannery,
    /// Apothecary.
    Apothecary,
    /// Temple.
    Temple,
    /// Shrine or chapel.
    Shrine,
    /// Library or archive.
    Library,
    /// School.
    School,
    /// Warehouse.
    Warehouse,
    /// Market hall (the vocabulary's `market` building).
    MarketHall,
    /// Market stall.
    Stall,
    /// Dock or jetty.
    Dock,
    /// Boathouse.
    Boathouse,
    /// Keep.
    Keep,
    /// Barracks.
    Barracks,
    /// Guardhouse.
    Guardhouse,
    /// Stable.
    Stable,
    /// Barn.
    Barn,
    /// Mine or quarry head (outside the street plan).
    Mine,
    /// Lumber camp (outside the street plan).
    LumberCamp,
}

/// Every building function, in vocabulary order.
pub const ALL: [BuildingFunction; 29] = {
    use BuildingFunction as F;
    [
        F::House,
        F::Farmhouse,
        F::Cottage,
        F::Manor,
        F::Inn,
        F::Tavern,
        F::Bakery,
        F::Brewery,
        F::Mill,
        F::Smithy,
        F::Workshop,
        F::Tannery,
        F::Apothecary,
        F::Temple,
        F::Shrine,
        F::Library,
        F::School,
        F::Warehouse,
        F::MarketHall,
        F::Stall,
        F::Dock,
        F::Boathouse,
        F::Keep,
        F::Barracks,
        F::Guardhouse,
        F::Stable,
        F::Barn,
        F::Mine,
        F::LumberCamp,
    ]
};

/// How a building-mix key maps onto the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixKey {
    /// A vocabulary function (possibly through an alias).
    Building(BuildingFunction),
    /// A function that lies outside the street plan (mines, lumber camps).
    OffPlan(BuildingFunction),
    /// Not recognised.
    Unknown,
}

impl BuildingFunction {
    /// The vocabulary key.
    #[must_use]
    pub fn key(self) -> &'static str {
        use BuildingFunction as F;
        match self {
            F::House => "house",
            F::Farmhouse => "farmhouse",
            F::Cottage => "cottage",
            F::Manor => "manor",
            F::Inn => "inn",
            F::Tavern => "tavern",
            F::Bakery => "bakery",
            F::Brewery => "brewery",
            F::Mill => "mill",
            F::Smithy => "smithy",
            F::Workshop => "workshop",
            F::Tannery => "tannery",
            F::Apothecary => "apothecary",
            F::Temple => "temple",
            F::Shrine => "shrine",
            F::Library => "library",
            F::School => "school",
            F::Warehouse => "warehouse",
            F::MarketHall => "market_hall",
            F::Stall => "stall",
            F::Dock => "dock",
            F::Boathouse => "boathouse",
            F::Keep => "keep",
            F::Barracks => "barracks",
            F::Guardhouse => "guardhouse",
            F::Stable => "stable",
            F::Barn => "barn",
            F::Mine => "mine",
            F::LumberCamp => "lumber_camp",
        }
    }

    /// Maps a building-mix key; `market` is an alias of `market_hall`.
    #[must_use]
    pub fn parse_mix(key: &str) -> MixKey {
        let key = if key == "market" { "market_hall" } else { key };
        match ALL.iter().find(|f| f.key() == key) {
            Some(&f) if matches!(f, Self::Mine | Self::LumberCamp) => MixKey::OffPlan(f),
            Some(&f) => MixKey::Building(f),
            None => MixKey::Unknown,
        }
    }

    /// Frontage × depth ranges in squares `((w0, w1), (d0, d1))`.
    #[must_use]
    pub fn size(self, tier: Tier) -> ((i32, i32), (i32, i32)) {
        use BuildingFunction as F;
        let city = matches!(tier, Tier::City);
        match self {
            F::House => ((4, 7), (6, 9)),
            F::Cottage => ((4, 6), (4, 6)),
            F::Farmhouse => ((8, 11), (5, 7)),
            F::Manor => ((12, 16), (9, 12)),
            F::Inn => ((10, 13), (9, 12)),
            F::Tavern => ((7, 9), (7, 9)),
            F::Bakery | F::Apothecary | F::Workshop => ((5, 7), (6, 9)),
            F::Brewery | F::Tannery => ((7, 10), (7, 10)),
            F::Mill => ((7, 9), (7, 9)),
            F::Smithy => ((6, 8), (6, 8)),
            F::Temple if city => ((14, 18), (26, 32)),
            F::Temple => ((11, 14), (20, 26)),
            F::Shrine => ((6, 8), (9, 12)),
            F::Library => ((10, 12), (10, 13)),
            F::School => ((8, 10), (8, 10)),
            F::Mine => ((6, 8), (6, 8)),
            F::LumberCamp => ((8, 10), (6, 8)),
            F::Warehouse => ((10, 14), (10, 13)),
            F::MarketHall => ((12, 18), (8, 10)),
            F::Stall => ((2, 2), (2, 2)),
            F::Dock => ((3, 4), (8, 12)),
            F::Boathouse => ((5, 6), (8, 10)),
            F::Keep => ((14, 18), (14, 18)),
            F::Barracks => ((14, 18), (7, 8)),
            F::Guardhouse => ((6, 7), (6, 7)),
            F::Stable => ((8, 10), (5, 6)),
            F::Barn => ((8, 10), (5, 7)),
        }
    }

    /// How many standard plots the building needs side by side.
    #[must_use]
    pub fn plots_needed(self, plot_w: i32) -> i32 {
        let ((w0, _), _) = self.size(Tier::Town);
        ((w0 + plot_w - 1) / plot_w.max(1)).max(1)
    }

    /// Storeys by tier and wealth (0–255).
    #[must_use]
    pub fn storeys(self, tier: Tier, wealth: u8) -> u8 {
        use BuildingFunction as F;
        let base = match tier {
            Tier::City => 3,
            Tier::Town => 2,
            _ => 1,
        };
        match self {
            F::Keep => 4,
            F::Temple | F::Shrine | F::Stall | F::Dock | F::Barn | F::Stable => 1,
            F::Cottage | F::Farmhouse | F::Boathouse | F::Smithy | F::Guardhouse => 1,
            F::Inn | F::Manor => base.max(2),
            F::Warehouse | F::MarketHall | F::Library | F::Barracks | F::Mill => 2,
            F::Mine | F::LumberCamp => 1,
            _ => base + u8::from(wealth > 200 && base > 1),
        }
    }

    /// Buildings that stand in the market square rather than on plots.
    #[must_use]
    pub fn in_square(self) -> bool {
        matches!(self, Self::Stall | Self::MarketHall)
    }

    /// Buildings that stand on the shore rather than on plots.
    #[must_use]
    pub fn on_shore(self) -> bool {
        matches!(self, Self::Dock)
    }

    /// Buildings drawn with a wall shell (stalls and docks are open).
    #[must_use]
    pub fn walled(self) -> bool {
        !matches!(self, Self::Stall | Self::Dock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_keys_map_through_aliases() {
        assert_eq!(
            BuildingFunction::parse_mix("market"),
            MixKey::Building(BuildingFunction::MarketHall)
        );
        assert_eq!(
            BuildingFunction::parse_mix("mine"),
            MixKey::OffPlan(BuildingFunction::Mine)
        );
        assert_eq!(BuildingFunction::parse_mix("zeppelin"), MixKey::Unknown);
        for f in ALL {
            let k = BuildingFunction::parse_mix(f.key());
            assert!(k == MixKey::Building(f) || k == MixKey::OffPlan(f));
        }
        let json = serde_json::to_string(&BuildingFunction::MarketHall).unwrap();
        assert_eq!(json, "\"market_hall\"");
    }
}
