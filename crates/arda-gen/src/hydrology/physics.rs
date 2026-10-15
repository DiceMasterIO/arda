//! The shared solve's recipe-dependent rules (logic/02 §fine-formation
//! recipe 9). Every field defaults to the rule recipes up to 8 published.

use super::budyko::RunoffModel;

/// Recipe-dependent rules of the shared annual solve.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SolvePhysics {
    /// How a cell's land loss is taken from rain and evaporation (review
    /// 3.2-1).
    pub runoff: RunoffModel,
    /// Height above the nearest drainage also measures to a lake surface or
    /// the sea, not only to a channel (review 3.2-6).
    pub hand_to_standing_water: bool,
    /// Equal-distance ties across exact flats break by a cell hash rather
    /// than toward the lowest index (review 3.2-13).
    pub hashed_flat_ties: bool,
}

impl SolvePhysics {
    /// The rules of fine recipe `recipe_version` (0 for legacy terrain).
    #[must_use]
    pub const fn for_recipe(recipe_version: u16) -> Self {
        if recipe_version >= 9 {
            Self {
                runoff: RunoffModel::Budyko,
                hand_to_standing_water: true,
                hashed_flat_ties: true,
            }
        } else {
            Self {
                runoff: RunoffModel::HalfRain,
                hand_to_standing_water: false,
                hashed_flat_ties: false,
            }
        }
    }
}
