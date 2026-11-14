//! `GET /v1/sites…` (logic/18 §site-serve): the listing, one site, and a
//! site's levels as body, image and scene.

use super::dto::{SiteDto, SiteLink, SiteList, SiteTrap, SITE_FORMAT};
use super::Site;
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, parse, segments, Params, Segments};
use crate::tactical::routes::{image, log, revalidated, TACTICAL_CACHE_CONTROL};
use crate::tactical::{Anchor, Hit, DEFAULT_PPSQ, PPSQ_OPTIONS};
use crate::AppState;
use arda_dungeon::{site, traps, Dungeon, Kind, Params as LevelParams};
use arda_scene::Scene;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

type Shared = State<Arc<AppState>>;

fn find(state: &AppState, id: &str) -> ServerResult<Site> {
    id.parse::<super::SiteId>()
        .ok()
        .and_then(|id| state.sites.get(id))
        .cloned()
        .ok_or_else(|| ServerError::NotFound(format!("no site {id:?}")))
}

/// Generates level `n` (1-based) of `site`.
fn generate(site: &Site, n: u32) -> ServerResult<Dungeon> {
    let l = n
        .checked_sub(1)
        .and_then(|i| site.levels.get(i as usize))
        .ok_or_else(|| ServerError::NotFound(format!("site {} has no level {n}", site.id)))?;
    arda_dungeon::generate(&LevelParams {
        seed: l.seed,
        kind: site.kind,
        width: l.width,
        height: l.height,
    })
    .map_err(|e| ServerError::Internal(e.to_string()))
}

/// The scene of a level, drawn with the seed its image is drawn with.
fn scene_of(state: &AppState, d: &Dungeon) -> ServerResult<Scene> {
    let t = &state.tactical;
    let seed =
        crate::tactical::key::render_seed(t.world_seed(), &Anchor::Name(d.layout.name.clone()));
    arda_scene::build_scene(&d.layout, t.library(), seed, Some(&d.rules))
        .map_err(|e| ServerError::Internal(format!("scene: {e}")))
}

fn json(body: &impl Serialize) -> ServerResult<Response> {
    let bytes = serde_json::to_vec(body).map_err(|e| ServerError::Internal(e.to_string()))?;
    Ok(([(CONTENT_TYPE, "application/json")], bytes).into_response())
}

/// `GET /sites[?bbox=x0,y0,x1,y1&kind=dungeon|cave]`.
pub(crate) async fn list(
    State(state): Shared,
    headers: HeaderMap,
    query: Params,
) -> ServerResult<Response> {
    let q = params(query)?;
    let etag = state.tactical.etags().of_request("sites", &[], &q);
    if let Some(r) = revalidated(&headers, &etag, TACTICAL_CACHE_CONTROL) {
        return Ok(r);
    }
    let kind = q
        .get("kind")
        .map(|k| {
            Kind::parse(k).map_err(|_| {
                ServerError::BadRequest(format!("kind must be dungeon or cave, not {k:?}"))
            })
        })
        .transpose()?;
    let bbox = q
        .get("bbox")
        .map(|text| {
            let n: Vec<i64> = text
                .split(',')
                .map(|v| v.trim().parse::<i64>())
                .collect::<Result<_, _>>()
                .map_err(|_| ServerError::BadRequest("`bbox` is x0,y0,x1,y1 in metres".into()))?;
            match n.as_slice() {
                [x0, y0, x1, y1] if x0 <= x1 && y0 <= y1 => Ok([*x0, *y0, *x1, *y1]),
                _ => Err(ServerError::BadRequest(
                    "`bbox` is x0,y0,x1,y1 in metres with x0 <= x1 and y0 <= y1".into(),
                )),
            }
        })
        .transpose()?;
    // Micrometres, so the box test is exact: a square is 1.5625 m.
    let inside = |s: &Site| {
        bbox.is_none_or(|[x0, y0, x1, y1]| {
            let [gsx, gsy] = s.entrance_gs();
            let (x, y) = (
                gsx * arda_core::SQUARE_SIZE_UM,
                gsy * arda_core::SQUARE_SIZE_UM,
            );
            let um = 1_000_000_i64;
            (x0.saturating_mul(um)..=x1.saturating_mul(um)).contains(&x)
                && (y0.saturating_mul(um)..=y1.saturating_mul(um)).contains(&y)
        })
    };
    let sites = state
        .sites
        .all()
        .iter()
        .filter(|s| kind.is_none_or(|k| s.kind == k) && inside(s))
        .map(SiteDto::from)
        .collect();
    Ok(crate::etag::tagged(
        Json(SiteList {
            format_version: SITE_FORMAT,
            sites,
        })
        .into_response(),
        &etag,
        TACTICAL_CACHE_CONTROL,
    ))
}

/// `GET /sites/{id}`.
pub(crate) async fn one(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let etag = state.tactical.etags().of("site", &[&p[0]]);
    if let Some(r) = revalidated(&headers, &etag, TACTICAL_CACHE_CONTROL) {
        return Ok(r);
    }
    let site = find(&state, &p[0])?;
    Ok(crate::etag::tagged(
        Json(SiteDto::from(&site)).into_response(),
        &etag,
        TACTICAL_CACHE_CONTROL,
    ))
}

/// The level body: the domain level as `/tactical/dungeon` serialises it,
/// so the inner bytes agree ([`super::dto::SiteLevel`] is its mirror).
#[derive(Serialize)]
struct LevelBody<'a> {
    site_id: String,
    level: u32,
    danger_tier: u8,
    dungeon: &'a Dungeon,
    links: Vec<SiteLink>,
    traps: Vec<SiteTrap>,
}

/// `GET /sites/{id}/levels/{n}` and `{n}.png[?ppsq=&grid=]`.
pub(crate) async fn level_or_png(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let p = segments(path, 2)?;
    let q = params(query)?;
    match p[1].strip_suffix(".png") {
        Some(n) => level_png(state, &headers, &p[0], n, &q).await,
        None => level(state, &headers, &p[0], &p[1]).await,
    }
}

async fn level(
    state: Arc<AppState>,
    headers: &HeaderMap,
    id: &str,
    n: &str,
) -> ServerResult<Response> {
    let start = Instant::now();
    let etag = state.tactical.etags().of("site.level", &[&id, &n]);
    if let Some(r) = revalidated(headers, &etag, TACTICAL_CACHE_CONTROL) {
        return Ok(r);
    }
    let n: u32 = parse("level", n)?;
    let site = find(&state, id)?;
    let detail = format!("{} level {n}", site.id);
    let at = n
        .checked_sub(1)
        .map(|i| i as usize)
        .filter(|i| *i < site.levels.len())
        .ok_or_else(|| ServerError::NotFound(format!("site {} has no level {n}", site.id)))?;
    let body = blocking(state, move |s| {
        // The chain needs every level's exits (logic/18 §site-links).
        let levels: Vec<Dungeon> = (1..=site.levels.len())
            .map(|k| generate(&site, u32::try_from(k).unwrap_or(u32::MAX)))
            .collect::<ServerResult<_>>()?;
        let d = &levels[at];
        let scene = scene_of(s, d)?;
        json(&LevelBody {
            site_id: site.id.to_string(),
            level: n,
            danger_tier: site.levels[at].danger_tier,
            dungeon: d,
            links: site::links(&levels)[at]
                .iter()
                .map(SiteLink::from)
                .collect(),
            traps: traps::place(d, &scene, d.params.seed)
                .iter()
                .map(SiteTrap::from)
                .collect(),
        })
    })
    .await?;
    log("site_level", &detail, Hit::Miss, 0, start);
    Ok(crate::etag::tagged(body, &etag, TACTICAL_CACHE_CONTROL))
}

async fn level_png(
    state: Arc<AppState>,
    headers: &HeaderMap,
    id: &str,
    n: &str,
    q: &std::collections::BTreeMap<String, String>,
) -> ServerResult<Response> {
    let start = Instant::now();
    let path = [id.to_owned(), n.to_owned()];
    let etag = state
        .tactical
        .etags()
        .of_request("site.level.png", &path, q);
    if let Some(r) = revalidated(headers, &etag, TACTICAL_CACHE_CONTROL) {
        return Ok(r);
    }
    let ppsq = match q.get("ppsq") {
        None => DEFAULT_PPSQ,
        Some(t) => {
            let v = parse::<u32>("ppsq", t)?;
            if !PPSQ_OPTIONS.contains(&v) {
                return Err(ServerError::BadRequest(format!(
                    "ppsq must be one of {PPSQ_OPTIONS:?}, not {v}"
                )));
            }
            v
        }
    };
    let grid = match q.get("grid").map(String::as_str) {
        None | Some("0") => false,
        Some("1") => true,
        Some(other) => {
            return Err(ServerError::BadRequest(format!(
                "grid must be 0 or 1, not {other:?}"
            )))
        }
    };
    let n: u32 = parse("level", n)?;
    let site = find(&state, id)?;
    let (name, (encoded, hit)) = blocking(state, move |s| {
        let d = generate(&site, n)?;
        // logic/18 §site-image: the deepest stairs down lead nowhere.
        let layout = if n as usize == site.levels.len() {
            site::sealed(&d.layout)
        } else {
            d.layout
        };
        let t = &s.tactical;
        t.admit(&layout, ppsq)?;
        let key = t.key(&layout, Anchor::Name(layout.name.clone()), ppsq, grid)?;
        Ok((layout.name.clone(), t.png(&layout, &key)?))
    })
    .await?;
    let ms = log(
        "site_level_png",
        &format!("{name} ppsq={ppsq} grid={}", u8::from(grid)),
        hit,
        encoded.bytes.len(),
        start,
    );
    Ok(image(
        headers,
        &encoded,
        &etag,
        "image/png",
        TACTICAL_CACHE_CONTROL,
        hit,
        ms,
    ))
}

/// `GET /sites/{id}/levels/{n}/scene`.
pub(crate) async fn scene(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 2)?;
    let etag = state
        .tactical
        .etags()
        .of("site.level.scene", &[&p[0], &p[1]]);
    if let Some(r) = revalidated(&headers, &etag, TACTICAL_CACHE_CONTROL) {
        return Ok(r);
    }
    let n: u32 = parse("level", &p[1])?;
    let site = find(&state, &p[0])?;
    let scene = blocking(state, move |s| scene_of(s, &generate(&site, n)?)).await?;
    log("site_level_scene", &scene.name, Hit::Miss, 0, start);
    Ok(crate::etag::tagged(
        Json(scene).into_response(),
        &etag,
        TACTICAL_CACHE_CONTROL,
    ))
}
