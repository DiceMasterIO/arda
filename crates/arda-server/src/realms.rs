//! Realm routes over `<world>/society/` (logic/16 §api-routes; OI-46):
//!
//! - `GET /v1/realms`: every realm of `realms.json`, without borders, with
//!   the politics headline when `arda society build` has run;
//! - `GET /v1/realms/{id}`: one realm with borders, its politics, the
//!   relations and events that involve it and its reigns.
//!
//! A world without `society/` answers 404 `not_found`, like
//! `/v1/settlements`; a settled world without `society.json` still serves
//! the `realms.json` part and `politics` is `null`.

use crate::error::{ServerError, ServerResult};
use crate::people::{people, MAX_LISTED};
use crate::routes::{blocking, parse, segments, Segments};
use crate::schema::{RealmDetail, RealmList, RealmRuler, RealmSummary};
use crate::AppState;
use arda_settle::realms::Realm;
use arda_society::RealmSociety;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::sync::Arc;

type Shared = State<Arc<AppState>>;

fn ids(v: &[u64]) -> Vec<String> {
    v.iter().map(u64::to_string).collect()
}

fn ruler_name(given: &str, regnal: &str) -> String {
    if regnal.is_empty() {
        given.to_owned()
    } else {
        format!("{given} {regnal}")
    }
}

fn summary(r: &Realm, politics: Option<&RealmSociety>) -> RealmSummary {
    RealmSummary {
        id: r.id.to_string(),
        name: r.name.clone(),
        name_gloss: r.name_gloss.clone(),
        seat: r.seat.to_string(),
        culture: r.culture.clone(),
        settlements: ids(&r.settlements),
        population: r.population,
        land_cells: r.land_cells,
        neighbours: ids(&r.neighbours),
        style: politics.map(|p| p.state.style.clone()),
        rank: politics.map(|p| p.state.rank.clone()),
        government_label: politics.map(|p| p.state.government_label.clone()),
        ruler: politics.map(|p| RealmRuler {
            name: ruler_name(&p.state.ruler.given, &p.state.ruler.regnal),
            title: p.state.ruler.title.clone(),
        }),
    }
}

fn politics_of(w: &arda_people::World, id: u64) -> Option<&RealmSociety> {
    w.society
        .as_ref()
        .and_then(|s| s.realms.iter().find(|r| r.state.id == id))
}

/// `GET /v1/realms`.
pub(crate) async fn list(State(state): Shared) -> ServerResult<Response> {
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let mut records: Vec<&Realm> = w
            .files
            .realms
            .as_ref()
            .map(|f| f.realms.iter().collect())
            .unwrap_or_default();
        records.sort_by_key(|r| r.id);
        let realms = records
            .into_iter()
            .take(MAX_LISTED)
            .map(|r| summary(r, politics_of(w, r.id)))
            .collect();
        Ok(RealmList {
            format_version: 1,
            realms,
        })
    })
    .await?;
    Ok(Json(body).into_response())
}

/// `GET /v1/realms/{id}`.
pub(crate) async fn one(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let id = parse::<u64>("realm id", &p[0])?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let record = w
            .files
            .realm(id)
            .ok_or_else(|| ServerError::NotFound(format!("no realm {id}")))?
            .clone();
        let society = w.society.as_ref();
        let relations = society
            .map(|x| {
                x.relations
                    .iter()
                    .filter(|r| r.a == id || r.b == id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let events = society
            .map(|x| {
                x.history
                    .events
                    .iter()
                    .filter(|e| e.realms.contains(&id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let reigns = society
            .map(|x| {
                x.history
                    .reigns
                    .iter()
                    .filter(|g| g.realm_id == id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(RealmDetail {
            record,
            politics: politics_of(w, id).cloned(),
            relations,
            events,
            reigns,
        })
    })
    .await?;
    Ok(Json(body).into_response())
}
