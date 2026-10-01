//! World people over HTTP (goals 51, 57; logic/13 §npc-id, §npc-queries;
//! logic/16 §api-routes):
//!
//! - `GET /v1/npc/{id}`: any inhabitant. `id` is a stored notable's u64 id,
//!   or the reference `<settlement>.<building>.<index>` of anyone, commoners
//!   included, regenerated from (seed, settlement, building, index). A u64
//!   id of a commoner resolves with `?settlement=`;
//! - `GET /v1/npcs?settlement=&building=&job=&realm=&notable=&limit=&cursor=`:
//!   a bounded page of matches ([`page`]);
//! - `GET /v1/buildings/{id}/residents` and `/workers`: who lives or works
//!   in a building, `id` being `<settlement>.<building>` (or the bare
//!   building id with `?settlement=`).
//!
//! Only notables are stored (goal 56); everyone else is regenerated per
//! request and never materialised world-wide.

pub mod dto;
pub mod page;
pub mod refs;

use crate::error::{ServerError, ServerResult};
use crate::people::people;
use crate::routes::{blocking, params, parse, segments, Params, Segments};
use crate::sheet_map::NpcAt;
use crate::AppState;
use arda_npc::{BuildingId, Npc, SettlementId};
use axum::extract::State;
use axum::response::Response;
use page::{Filter, Inputs, Link, DEFAULT_LIMIT, MAX_LIMIT};
use refs::{NpcKey, NpcRef};
use std::collections::BTreeMap;
use std::sync::Arc;

type Shared = State<Arc<AppState>>;

fn only(q: &BTreeMap<String, String>, allowed: &[&str]) -> ServerResult<()> {
    match q.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(ServerError::BadRequest(format!(
            "unknown query parameter {k:?}; this route takes {allowed:?}"
        ))),
        None => Ok(()),
    }
}

fn limit(q: &BTreeMap<String, String>) -> ServerResult<usize> {
    let n = q
        .get("limit")
        .map_or(Ok(DEFAULT_LIMIT), |t| parse::<usize>("limit", t))?;
    if (1..=MAX_LIMIT).contains(&n) {
        Ok(n)
    } else {
        Err(ServerError::BadRequest(format!(
            "limit must be 1..={MAX_LIMIT}, not {n}"
        )))
    }
}

fn settlement(q: &BTreeMap<String, String>) -> ServerResult<Option<u64>> {
    q.get("settlement")
        .map(|t| parse::<u64>("settlement", t))
        .transpose()
}

fn cursor(q: &BTreeMap<String, String>) -> ServerResult<Option<refs::Cursor>> {
    q.get("cursor").map(|t| refs::parse_cursor(t)).transpose()
}

/// Parses the `/v1/npcs` query.
///
/// # Errors
/// [`ServerError::BadRequest`] for unknown or malformed parameters.
pub fn filter(q: &BTreeMap<String, String>) -> ServerResult<Filter> {
    only(
        q,
        &[
            "settlement",
            "building",
            "job",
            "realm",
            "notable",
            "limit",
            "cursor",
        ],
    )?;
    let mut settlement = settlement(q)?;
    let building = q
        .get("building")
        .map(|t| refs::parse_building(t, settlement))
        .transpose()?
        .map(|(s, b)| {
            settlement = Some(s);
            (b, Link::Either)
        });
    let notable = match q.get("notable").map(String::as_str) {
        None => None,
        Some("1" | "true") => Some(true),
        Some("0" | "false") => Some(false),
        Some(other) => {
            return Err(ServerError::BadRequest(format!(
                "notable must be true or false, not {other:?}"
            )))
        }
    };
    Ok(Filter {
        settlement,
        building,
        job: q.get("job").cloned(),
        realm: q
            .get("realm")
            .map(|t| parse::<u64>("realm", t))
            .transpose()?,
        notable,
        limit: limit(q)?,
        cursor: cursor(q)?,
    })
}

async fn run(state: Arc<AppState>, f: Filter) -> ServerResult<Response> {
    blocking(state, move |s| {
        let body = page::page(people(s)?, &f)?;
        s.npc_response(&body, NpcAt::List("/npcs", Some("npc")))
    })
    .await
}

/// `GET /v1/npcs`.
pub(crate) async fn list(State(state): Shared, query: Params) -> ServerResult<Response> {
    let f = filter(&params(query)?)?;
    run(state, f).await
}

async fn building(
    state: Arc<AppState>,
    path: Segments,
    query: Params,
    link: Link,
) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let q = params(query)?;
    only(&q, &["settlement", "limit", "cursor"])?;
    let (s, b) = refs::parse_building(&p[0], settlement(&q)?)?;
    let f = Filter {
        settlement: Some(s),
        building: Some((b, link)),
        job: None,
        realm: None,
        notable: None,
        limit: limit(&q)?,
        cursor: cursor(&q)?,
    };
    run(state, f).await
}

/// `GET /v1/buildings/{id}/residents`.
pub(crate) async fn residents(
    State(state): Shared,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    building(state, path, query, Link::Residents).await
}

/// `GET /v1/buildings/{id}/workers`.
pub(crate) async fn workers(
    State(state): Shared,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    building(state, path, query, Link::Workers).await
}

/// Resolves any inhabitant of the world: a stored notable by id, anyone by
/// reference, or anyone by id within `settlement`.
///
/// # Errors
/// 404 for nobody; 400 for a reference whose settlement disagrees.
pub fn resolve(s: &AppState, key: NpcKey, settlement: Option<u64>) -> ServerResult<Npc> {
    let w = people(s)?;
    let id = match key {
        NpcKey::Id(id) => id,
        NpcKey::Ref(r) => r.id(),
    };
    let wanted = match key {
        NpcKey::Ref(r) => {
            if settlement.is_some_and(|q| q != r.settlement) {
                return Err(ServerError::BadRequest(format!(
                    "npc {r} is not in settlement {}",
                    settlement.unwrap_or_default()
                )));
            }
            Some(r.settlement)
        }
        NpcKey::Id(_) => settlement,
    };
    if let Some((npc, home)) = w.notable(id) {
        if wanted.is_none_or(|q| home == SettlementId(q)) {
            return Ok(npc.clone());
        }
    }
    let Some(sid) = wanted else {
        return Err(ServerError::NotFound(format!(
            "no stored notable {id}; address a commoner as \
             <settlement>.<building>.<index> or add ?settlement="
        )));
    };
    let inputs = Inputs::of(w, sid)?;
    let g = inputs.generator(w.seed())?;
    Ok(match key {
        NpcKey::Ref(NpcRef {
            building, index, ..
        }) => g.commoner(BuildingId(building), index)?,
        NpcKey::Id(id) => g.npc(id)?,
    })
}

/// `GET /v1/npc/{id}[?settlement=]`.
pub(crate) async fn one(
    State(state): Shared,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let q = params(query)?;
    only(&q, &["settlement"])?;
    let key = refs::parse_npc(&p[0])?;
    let settlement = settlement(&q)?;
    blocking(state, move |s| {
        s.npc_response(&resolve(s, key, settlement)?, NpcAt::Root)
    })
    .await
}
