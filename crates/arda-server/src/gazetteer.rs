//! `GET /v1/gazetteer[?kind=&bbox=&min_rank=]`: the world's named natural
//! features and its settlements as one list of map labels
//! (logic/16 §api-gazetteer).
//!
//! The features come from `society/gazetteer.json`, read at startup with
//! the other society files; settlements are added as points. A world
//! without `society/` answers 404 `not_found`; one settled before the
//! gazetteer existed answers 404 too, naming the command that makes it.

use crate::error::{ServerError, ServerResult};
use crate::people::people;
use crate::routes::{blocking, params, Params};
use crate::AppState;
use arda_settle::gazetteer::{list, Filter, PlaceKind};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Parses the query into a filter.
fn filter(q: &BTreeMap<String, String>) -> ServerResult<Filter> {
    let kinds = q
        .get("kind")
        .map(|text| {
            text.split(',')
                .map(|k| {
                    PlaceKind::from_key(k).ok_or_else(|| {
                        ServerError::BadRequest(format!("unknown kind {k:?} in `kind`"))
                    })
                })
                .collect::<ServerResult<Vec<_>>>()
        })
        .transpose()?;
    let bbox = q
        .get("bbox")
        .map(|text| {
            let n: Vec<i64> = text
                .split(',')
                .map(|v| v.trim().parse::<i64>())
                .collect::<Result<_, _>>()
                .map_err(|_| {
                    ServerError::BadRequest("`bbox` is x0,y0,x1,y1 in metres".into())
                })?;
            match n.as_slice() {
                [x0, y0, x1, y1] if x0 <= x1 && y0 <= y1 => Ok([*x0, *y0, *x1, *y1]),
                _ => Err(ServerError::BadRequest(
                    "`bbox` is x0,y0,x1,y1 in metres with x0 <= x1 and y0 <= y1".into(),
                )),
            }
        })
        .transpose()?;
    let max_rank = q
        .get("min_rank")
        .map(|v| match v.parse::<u8>() {
            Ok(r @ 1..=5) => Ok(r),
            _ => Err(ServerError::BadRequest(
                "`min_rank` is a whole number from 1 to 5".into(),
            )),
        })
        .transpose()?;
    Ok(Filter {
        kinds,
        bbox,
        max_rank,
    })
}

/// `GET /v1/gazetteer`.
pub(crate) async fn get(
    State(state): State<Arc<AppState>>,
    query: Params,
) -> ServerResult<Response> {
    let f = filter(&params(query)?)?;
    let body = blocking(state, move |s| {
        let w = people(s)?;
        let file = w.files.gazetteer.as_ref().ok_or_else(|| {
            ServerError::NotFound(
                "this world has no society/gazetteer.json; run `arda settle` again".into(),
            )
        })?;
        Ok(list(file, &w.files.settlements.settlements, &f))
    })
    .await?;
    Ok(Json(body).into_response())
}
