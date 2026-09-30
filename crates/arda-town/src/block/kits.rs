//! Wall kits and floors per culture, wealth and function, using the shared
//! vocabulary keys (`stone`, `timber`, `wattle`, `city_wall`; `planks`,
//! `packed_earth`, `stone_floor`, `flagstone`, `rug`).

use crate::function::BuildingFunction as F;
use crate::plan::{Building, WealthLevel};
use crate::rng::hash_str;

/// How a culture builds ordinary houses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tradition {
    /// Timber framing; stone only for the rich and for civic buildings.
    Timber,
    /// Stone throughout; wattle only for the poorest.
    Stone,
    /// Wattle and daub for the poor, timber for the rest.
    Wattle,
}

/// Building tradition of an `arda-settle` culture key; unknown cultures
/// pick one by hash so they stay stable.
#[must_use]
pub fn tradition(culture: &str) -> Tradition {
    match culture {
        "heartland" | "coastal" => Tradition::Timber,
        "highland" | "southern" => Tradition::Stone,
        "sylvan" | "borderland" => Tradition::Wattle,
        other => match hash_str(0x7ad, other) % 3 {
            0 => Tradition::Timber,
            1 => Tradition::Stone,
            _ => Tradition::Wattle,
        },
    }
}

/// Always-stone functions.
fn civic(f: F) -> bool {
    matches!(
        f,
        F::Temple | F::Keep | F::Library | F::School | F::Guardhouse | F::Barracks | F::MarketHall
    )
}

/// Shell kit of a building.
#[must_use]
pub fn shell(b: &Building, t: Tradition) -> &'static str {
    if civic(b.function) || matches!(b.function, F::Manor) {
        return "stone";
    }
    if matches!(b.function, F::Barn | F::Stable | F::Boathouse) {
        return if b.wealth_level == WealthLevel::Poor {
            "wattle"
        } else {
            "timber"
        };
    }
    match (t, b.wealth_level) {
        (_, WealthLevel::Wealthy) | (Tradition::Stone, _) => "stone",
        (Tradition::Timber | Tradition::Wattle, WealthLevel::Modest) => "timber",
        (Tradition::Timber | Tradition::Wattle, WealthLevel::Poor) => "wattle",
    }
}

/// Partition kit of a building.
#[must_use]
pub fn partition(b: &Building) -> &'static str {
    if b.wealth_level == WealthLevel::Poor {
        "wattle"
    } else {
        "timber"
    }
}

/// Main floor of a building.
#[must_use]
pub fn floor(b: &Building) -> &'static str {
    match b.function {
        F::Temple | F::Shrine | F::Keep | F::Library | F::MarketHall => "flagstone",
        F::Warehouse | F::Barracks | F::Guardhouse | F::Mill => "stone_floor",
        F::Smithy | F::Stable | F::Barn | F::Tannery | F::Brewery => "packed_earth",
        F::Bakery => "stone_floor",
        _ => match b.wealth_level {
            WealthLevel::Poor => "packed_earth",
            WealthLevel::Modest => "planks",
            WealthLevel::Wealthy => "planks",
        },
    }
}
