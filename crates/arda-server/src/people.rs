//! Settlement and world-NPC routes over `<world>/society/` (logic/16
//! §api-society; goals 52–57, 69):
//!
//! - `GET /v1/settlements[?tier=&realm=]`: the settlement records;
//! - `GET /v1/settlements/{id}`: the record and its society (history,
//!   economy, factions, offices, hooks);
//! - `GET /v1/settlements/{id}/plan`: its `arda-town` plan;
//! - `GET /v1/settlements/{id}/npcs`: its stored notables.
//!
//! People (`/v1/npc/{id}`, `/v1/npcs`, `/v1/buildings/{id}/…`) are served by
//! [`crate::npcs`]. Only notables are stored (goal 56); a world without
//! `society/` answers 404 `not_found` on every route here.

use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, parse, segments, Params, Segments};
use crate::AppState;
use arda_people::{PeopleError, World};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

type Shared = State<Arc<AppState>>;

/// Most records one listing returns.
pub const MAX_LISTED: usize = 20_000;

pub(crate) fn people(state: &AppState) -> ServerResult<&World> {
    state.people.as_deref().ok_or_else(|| {
        ServerError::NotFound(
            "this world has no society/ directory; run `arda settle` and `arda society build`"
                .into(),
        )
    })
}

pub(crate) fn people_error(e: PeopleError) -> ServerError {
    match e {
        PeopleError::UnknownSettlement(id) => ServerError::NotFound(format!("no settlement {id}")),
        other => ServerError::Internal(other.to_string()),
    }
}

fn settlement_id(text: &str) -> ServerResult<u64> {
    parse::<u64>("settlement id", text)
}

/// `GET /v1/settlements`.
pub(crate) async fn list(State(state): Shared, query: Params) -> ServerResult<Response> {
    let q = params(query)?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let tier = q.get("tier").cloned();
        let realm = q.get("realm").cloned();
        let records: Vec<&Value> = w
            .files
            .records
            .iter()
            .filter(|r| tier.as_deref().is_none_or(|t| r["tier"] == t))
            .filter(|r| realm.as_deref().is_none_or(|t| r["realm_id"] == t))
            .take(MAX_LISTED)
            .collect();
        Ok(json!({ "format_version": 1, "settlements": records }))
    })
    .await?;
    Ok(Json(body).into_response())
}

/// `GET /v1/settlements/{id}` and its `/plan` and `/npcs`.
pub(crate) async fn one(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let id = settlement_id(&p[0])?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let (_, record) = w.files.settlement(id).map_err(people_error)?;
        let society = w
            .society
            .as_ref()
            .and_then(|soc| soc.settlements.iter().find(|x| x.id == id));
        Ok(json!({ "record": record, "society": society }))
    })
    .await?;
    Ok(Json(body).into_response())
}

/// `GET /v1/settlements/{id}/plan`.
pub(crate) async fn plan(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let id = settlement_id(&p[0])?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let plan = w.plan(id).map_err(people_error)?;
        let plan = plan
            .as_ref()
            .as_ref()
            .ok_or_else(|| ServerError::NotFound(format!("settlement {id} has no town plan")))?;
        serde_json::to_value(plan).map_err(|e| ServerError::Internal(e.to_string()))
    })
    .await?;
    Ok(Json(body).into_response())
}

/// `GET /v1/settlements/{id}/npcs`: the stored notables.
pub(crate) async fn npcs(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let id = settlement_id(&p[0])?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        w.files.settlement(id).map_err(people_error)?;
        Ok(json!({ "settlement_id": id.to_string(), "npcs": w.notables_of(id) }))
    })
    .await?;
    Ok(Json(body).into_response())
}
