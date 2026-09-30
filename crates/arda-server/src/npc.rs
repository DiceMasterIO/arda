//! NPC routes (`logic/16` route table, `logic/13`): populate a posted
//! settlement, and the market-town demo with single-NPC regeneration.
//!
//! Bodies are the `arda-npc` domain types serialised by serde; their
//! TypeScript types come from the mirrors in [`crate::npc_dto`], which the
//! tests hold equal to the domain JSON.

use crate::error::{ServerError, ServerResult};
use arda_npc::{BuildingSpec, Generator, Npc, NpcId, Population, SettlementProfile};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Largest accepted `POST /v1/npc/population` body, bytes.
pub const MAX_BODY_BYTES: usize = 2 << 20;
/// Largest population a request may ask for.
pub const MAX_POPULATION: u32 = 50_000;
/// Most buildings a request may list.
pub const MAX_BUILDINGS: usize = 20_000;

/// Body of `POST /v1/npc/population` (TS: `PopulationRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationRequest {
    /// World seed as a decimal string (u64 does not fit a JS number).
    pub world_seed: String,
    /// The settlement to populate.
    pub settlement: SettlementProfile,
    /// Its buildings.
    pub buildings: Vec<BuildingSpec>,
}

/// Body of `GET /v1/npc/demo` (TS: `NpcDemo`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NpcDemo {
    /// World seed as a decimal string.
    pub world_seed: String,
    /// The example settlement.
    pub settlement: SettlementProfile,
    /// Its buildings.
    pub buildings: Vec<BuildingSpec>,
    /// Its population.
    pub population: Population,
}

/// The demo town, built once on first use.
#[derive(Debug, Default)]
pub struct NpcService {
    demo: OnceLock<Result<NpcDemo, String>>,
}

impl NpcService {
    /// The market-town demo and its population.
    ///
    /// # Errors
    /// [`ServerError::Internal`] if the bundled example cannot be generated.
    pub fn demo(&self) -> ServerResult<&NpcDemo> {
        self.demo
            .get_or_init(|| build_demo().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| ServerError::Internal(format!("npc demo: {e}")))
    }

    /// Regenerates one inhabitant of the demo town from scratch, without
    /// reading the cached population.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for an id that names nobody there.
    pub fn demo_npc(&self, id: NpcId) -> ServerResult<Npc> {
        let (seed, settlement, buildings) = arda_npc::sample::market_town();
        Ok(arda_npc::npc(seed, &settlement, &buildings, id)?)
    }
}

fn build_demo() -> ServerResult<NpcDemo> {
    let (seed, settlement, buildings) = arda_npc::sample::market_town();
    let population = Generator::new(seed, &settlement, &buildings)?.population()?;
    Ok(NpcDemo {
        world_seed: seed.to_string(),
        settlement,
        buildings,
        population,
    })
}

/// Checks the request limits, then generates the population.
///
/// # Errors
/// [`ServerError::BadRequest`] for a bad seed, [`ServerError::PayloadTooLarge`]
/// past the limits, and the generator's refusals.
pub fn populate(request: &PopulationRequest) -> ServerResult<Population> {
    let seed: u64 = request.world_seed.parse().map_err(|_| {
        ServerError::BadRequest(format!(
            "world_seed {:?} is not a decimal u64",
            request.world_seed
        ))
    })?;
    if request.settlement.population > MAX_POPULATION {
        return Err(ServerError::PayloadTooLarge(format!(
            "population {} is above the {MAX_POPULATION} limit",
            request.settlement.population
        )));
    }
    if request.buildings.len() > MAX_BUILDINGS {
        return Err(ServerError::PayloadTooLarge(format!(
            "{} buildings is above the {MAX_BUILDINGS} limit",
            request.buildings.len()
        )));
    }
    Ok(arda_npc::generate_population(
        seed,
        &request.settlement,
        &request.buildings,
    )?)
}

/// Parses an NPC id path segment: the decimal u64 string.
///
/// # Errors
/// [`ServerError::BadRequest`] for anything else.
pub fn parse_npc_id(text: &str) -> ServerResult<NpcId> {
    text.parse()
        .map_err(|_| ServerError::BadRequest(format!("npc id {text:?} is not a decimal u64")))
}
