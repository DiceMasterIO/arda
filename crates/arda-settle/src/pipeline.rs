//! The stage pipeline: each step reads only earlier outputs.

use crate::cost::CostSurface;
use crate::crossings::{self, Crossing, Pass};
use crate::culture::CultureMap;
use crate::error::SettleError;
use crate::grid::Grid;
use crate::landuse::{self, LandUse};
use crate::model::Settlement;
use crate::naming;
use crate::output::{NamedPeak, NamedRegion, NamedRiver};
use crate::place::{self, PlaceParams};
use crate::profile;
use crate::realms::{self, Realms};
use crate::roads::{self, Network};
use crate::stats::{self, Stats};
use crate::suitability;
use crate::tags;

/// Everything the stage produces.
#[derive(Debug, Clone)]
pub struct Society {
    /// Settlements by id.
    pub settlements: Vec<Settlement>,
    /// Land use raster.
    pub landuse: LandUse,
    /// Road network.
    pub network: Network,
    /// Crossing objects.
    pub crossings: Vec<Crossing>,
    /// Pass objects.
    pub passes: Vec<Pass>,
    /// Realm partition.
    pub realms: Realms,
    /// Named rivers.
    pub rivers: Vec<NamedRiver>,
    /// Named mountains.
    pub mountains: Vec<NamedPeak>,
    /// Named regions.
    pub regions: Vec<NamedRegion>,
    /// Statistics.
    pub stats: Stats,
}

/// Runs every stage over a grid.
///
/// # Errors
/// Allocation errors from any stage.
pub fn run(g: &Grid, params: PlaceParams) -> Result<Society, SettleError> {
    let seed = params.seed;
    let sites = tags::compute(g, seed)?;
    let suit = suitability::compute(g, &sites)?;
    let placement = place::place(g, &sites, &suit, params)?;
    let mut settlements = placement.settlements;
    let cultures = CultureMap::build(g, &sites);
    let miners = profile::miners(g, &sites, seed, &settlements);
    for s in &mut settlements {
        let miner = miners.contains(&s.id.get());
        profile::describe(g, &suit, &cultures, seed, miner, s);
    }
    let landuse = landuse::allocate(g, &sites, &settlements)?;
    let cs = CostSurface::build(g, &sites, &landuse, seed)?;
    let network = roads::build(g, &cs, &settlements);
    let (mut crossings, crossing_cells) = crossings::crossings(g, &network, &settlements)?;
    let mut passes = crossings::passes(g, &sites, &network);
    let mut realms = realms::partition(g, &sites, &cs, &network, &mut settlements)?;
    for r in &realms.realms {
        if let Some(s) = settlements
            .iter_mut()
            .find(|s| s.id == arda_ids::SettlementId(r.seat))
        {
            profile::make_capital(s);
        }
    }

    let named = naming::name_world(
        g,
        seed,
        &cultures,
        &mut settlements,
        &mut crossings,
        &crossing_cells,
        &mut passes,
        &mut realms.realms,
    )?;
    let mut stats = stats::gather(
        &settlements,
        placement.target_population,
        landuse.farmland_cells(),
        &network,
        &crossings,
        passes.len(),
        realms.realms.len(),
    );
    (stats.inland_town_share, stats.town_nni, stats.town_nn_km) =
        stats::town_pattern(&settlements, u64::try_from(g.land_cells()).unwrap_or(0));
    stats.border_on_river_or_ridge_pm = realms.natural_border_pm;
    stats.border_on_river_or_ridge_raw_pm = realms.raw_border_pm;
    stats.border_natural_of_all_pm = realms.natural_all_pm;
    stats.land_near_river_or_ridge_pm = realms.natural_land_pm;
    Ok(Society {
        settlements,
        landuse,
        network,
        crossings,
        passes,
        realms,
        rivers: named.rivers,
        mountains: named.mountains,
        regions: named.regions,
        stats,
    })
}
