//! NPC routes (`logic/16` route table, `logic/13`): populate a posted
//! settlement, generate one NPC from optional inputs, and the market-town
//! demo with single-NPC regeneration.
//!
//! Bodies are the `arda-npc` domain types serialised by serde; their
//! TypeScript types come from the mirrors in [`crate::npc_dto`], which the
//! tests hold equal to the domain JSON.

use crate::error::{ServerError, ServerResult};
use crate::routes::blocking;
use crate::sheet_map::NpcAt;
use crate::AppState;
use arda_npc::{
    BuildingSpec, Generator, Npc, NpcId, OneRequest, Population, SettlementProfile, Sex,
    SocialRank, Tier,
};
use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// Largest accepted `POST /v1/npc/population` body, bytes.
pub const MAX_BODY_BYTES: usize = 2 << 20;
/// Largest population a request may ask for.
pub const MAX_POPULATION: u32 = 50_000;
/// Most buildings a request may list.
pub const MAX_BUILDINGS: usize = 20_000;

/// Largest accepted `POST /v1/npc/generate` body, bytes.
pub const MAX_GENERATE_BODY_BYTES: usize = 16 << 10;

/// Body of `POST /v1/npc/generate` (TS: `GenerateNpcRequest`): Arda's own
/// inputs, every field optional (logic/13 §npc-generate-one).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateNpcRequest {
    /// Caller's salt as a decimal u64 string; default `"0"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    /// Occupation key, e.g. `smith`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// Society office kind, e.g. `reeve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub office: Option<String>,
    /// Required social rank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<SocialRank>,
    /// SRD race key, e.g. `dwarf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ancestry: Option<String>,
    /// Required sex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex: Option<Sex>,
    /// Size class of the synthetic settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<Tier>,
    /// Culture key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub culture: Option<String>,
    /// A stored settlement (decimal id) to borrow context from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
    /// Whether the NPC must (or must not) be a notable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notable: Option<bool>,
}

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

/// Converts the wire body to the generator's input, resolving
/// `settlement_id` through the world's `society/`.
///
/// # Errors
/// [`ServerError::BadRequest`] for a malformed seed or settlement id,
/// [`ServerError::NotFound`] for an unknown settlement or a world without
/// `society/`.
pub fn one_request(state: &AppState, request: &GenerateNpcRequest) -> ServerResult<OneRequest> {
    let decimal = |what: &str, text: &str| {
        text.parse::<u64>()
            .map_err(|_| ServerError::BadRequest(format!("{what} {text:?} is not a decimal u64")))
    };
    let seed = request
        .seed
        .as_deref()
        .map_or(Ok(0), |s| decimal("seed", s))?;
    let settlement = match request.settlement_id.as_deref() {
        Some(text) => {
            let id = decimal("settlement_id", text)?;
            let world = crate::people::people(state)?;
            Some(world.profile(id).map_err(crate::people::people_error)?)
        }
        None => None,
    };
    Ok(OneRequest {
        seed,
        job: request.job.clone(),
        office: request.office.clone(),
        rank: request.rank,
        ancestry: request.ancestry.clone(),
        sex: request.sex,
        tier: request.tier,
        culture: request.culture.clone(),
        settlement,
        notable: request.notable,
    })
}

/// Generates one NPC in the served world: a pure function of the world
/// seed and the body; nothing is stored.
///
/// # Errors
/// As [`one_request`], and the generator's refusals.
pub fn generate(state: &AppState, request: &GenerateNpcRequest) -> ServerResult<Npc> {
    let one = one_request(state, request)?;
    Ok(arda_npc::generate_one(state.query.world().seed(), &one)?)
}

/// `POST /v1/npc/generate`.
pub(crate) async fn generate_route(
    State(state): State<Arc<AppState>>,
    body: Result<Json<GenerateNpcRequest>, JsonRejection>,
) -> ServerResult<Response> {
    let Json(request) = body.map_err(|e| match e.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ServerError::PayloadTooLarge(format!(
            "request body is above {MAX_GENERATE_BODY_BYTES} bytes"
        )),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => ServerError::UnsupportedMediaType(e.body_text()),
        _ => ServerError::BadRequest(e.body_text()),
    })?;
    blocking(state, move |s| {
        s.npc_response(&generate(s, &request)?, NpcAt::Root)
    })
    .await
}

/// Parses an NPC id path segment: the decimal u64 string.
///
/// # Errors
/// [`ServerError::BadRequest`] for anything else.
pub fn parse_npc_id(text: &str) -> ServerResult<NpcId> {
    text.parse()
        .map_err(|_| ServerError::BadRequest(format!("npc id {text:?} is not a decimal u64")))
}
