//! Route inventory (1.0 task 08): every route the policy page
//! `docs/capstone/versioning.md` marks **stable** answers on the MICRO
//! seed-42 society world with something other than `404`/`405`, so a frozen
//! route cannot vanish silently. The probes are cheap (small renders, bodies
//! the handlers refuse before any work).

use crate::support::{send, society_state, Reply};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::path::PathBuf;

/// The river cell of `cells.rs`, a land cell.
const CELL: &str = "529/812";
const WINDOW: &str = "gx0=529&gy0=812&w=1&h=1&ppsq=16";

/// The router's own answer for a path it does not serve.
const FALLBACK: &str = "no such route";

/// `(route as the policy table writes it, method, probe URI, JSON body)`.
fn probes(ids: &Ids) -> Vec<(&'static str, &'static str, String, Option<&'static str>)> {
    let (s, b, n, r, relief) = (
        &ids.settlement,
        &ids.building,
        &ids.npc,
        &ids.realm,
        ids.relief_z,
    );
    vec![
        ("GET /v1/health", "GET", "/v1/health".into(), None),
        ("GET /v1/world", "GET", "/v1/world".into(), None),
        (
            "GET /v1/cell/{gx}/{gy}",
            "GET",
            format!("/v1/cell/{CELL}"),
            None,
        ),
        (
            "GET /v1/point",
            "GET",
            "/v1/point?x_m=52950&y_m=81250".into(),
            None,
        ),
        (
            "GET /v1/area/{ax}/{ay}/cells",
            "GET",
            "/v1/area/1/1/cells?format=bin".into(),
            None,
        ),
        (
            "GET /v1/area/{ax}/{ay}/rivers",
            "GET",
            "/v1/area/1/1/rivers".into(),
            None,
        ),
        (
            "GET /v1/area/{ax}/{ay}/lakes",
            "GET",
            "/v1/area/1/1/lakes".into(),
            None,
        ),
        (
            "GET /v1/overview.png",
            "GET",
            "/v1/overview.png?quality=512".into(),
            None,
        ),
        (
            "GET /v1/tiles/overview/{z}/{x}/{y}.webp",
            "GET",
            "/v1/tiles/overview/0/0/0.webp".into(),
            None,
        ),
        (
            "GET /v1/tiles/overview/{z}/{x}/{y}.png",
            "GET",
            "/v1/tiles/overview/0/0/0.png".into(),
            None,
        ),
        (
            "GET /v1/tiles/relief/{z}/{x}/{y}.webp",
            "GET",
            format!("/v1/tiles/relief/{relief}/0/0.webp"),
            None,
        ),
        (
            "GET /v1/settlements",
            "GET",
            "/v1/settlements?tier=village".into(),
            None,
        ),
        (
            "GET /v1/settlements/{id}",
            "GET",
            format!("/v1/settlements/{s}"),
            None,
        ),
        (
            "GET /v1/settlements/{id}/plan",
            "GET",
            format!("/v1/settlements/{s}/plan"),
            None,
        ),
        (
            "GET /v1/settlements/{id}/npcs",
            "GET",
            format!("/v1/settlements/{s}/npcs"),
            None,
        ),
        ("GET /v1/npc/{npc_id}", "GET", format!("/v1/npc/{n}"), None),
        (
            "GET /v1/npcs",
            "GET",
            format!("/v1/npcs?settlement={s}&limit=1"),
            None,
        ),
        (
            "GET /v1/buildings/{building}/residents",
            "GET",
            format!("/v1/buildings/{b}/residents?limit=1"),
            None,
        ),
        (
            "GET /v1/buildings/{building}/workers",
            "GET",
            format!("/v1/buildings/{b}/workers?limit=1"),
            None,
        ),
        (
            "POST /v1/npc/population",
            "POST",
            "/v1/npc/population".into(),
            Some("{}"),
        ),
        (
            "POST /v1/npc/generate",
            "POST",
            "/v1/npc/generate".into(),
            Some(r#"{"seed":"7"}"#),
        ),
        (
            "GET /v1/gazetteer",
            "GET",
            "/v1/gazetteer?min_rank=2".into(),
            None,
        ),
        ("GET /v1/realms", "GET", "/v1/realms".into(), None),
        (
            "GET /v1/realms/{id}",
            "GET",
            format!("/v1/realms/{r}"),
            None,
        ),
        ("GET /v1/schema", "GET", "/v1/schema".into(), None),
        (
            "GET /v1/schema/{name}.json",
            "GET",
            "/v1/schema/Npc.json".into(),
            None,
        ),
        (
            "GET /v1/tactical/cell/{gx}/{gy}",
            "GET",
            format!("/v1/tactical/cell/{CELL}?ppsq=64"),
            None,
        ),
        (
            "GET /v1/tactical/cell/{gx}/{gy}.png",
            "GET",
            format!("/v1/tactical/cell/{CELL}.png?ppsq=16"),
            None,
        ),
        (
            "GET /v1/tactical/cell/{gx}/{gy}/scene",
            "GET",
            format!("/v1/tactical/cell/{CELL}/scene"),
            None,
        ),
        (
            "GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp",
            "GET",
            format!("/v1/tactical/cell/{CELL}/tiles/0/0/0.webp?ppsq=16"),
            None,
        ),
        (
            "GET /v1/tactical/window",
            "GET",
            format!("/v1/tactical/window?{WINDOW}"),
            None,
        ),
        (
            "GET /v1/tactical/window.png",
            "GET",
            format!("/v1/tactical/window.png?{WINDOW}"),
            None,
        ),
        (
            "GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp",
            "GET",
            format!("/v1/tactical/window/tiles/0/0/0.webp?{WINDOW}"),
            None,
        ),
        (
            "GET /v1/tactical/library",
            "GET",
            "/v1/tactical/library".into(),
            None,
        ),
        (
            "GET /v1/tactical/library/asset/{id}.png",
            "GET",
            "/v1/tactical/library/asset/effect.frost.0.png".into(),
            None,
        ),
        // Refused before any work: radius 9 is 400.
        (
            "POST /v1/tactical/prefetch",
            "POST",
            "/v1/tactical/prefetch".into(),
            Some(r#"{"gx":529,"gy":812,"radius":9}"#),
        ),
        // Refused before any work: a body without a layout is 400.
        (
            "POST /v1/tactical/bake",
            "POST",
            "/v1/tactical/bake".into(),
            Some("{}"),
        ),
        (
            "GET /v1/tactical/bake/{id}",
            "GET",
            "/v1/tactical/bake/00000000000000000000000000000000".into(),
            None,
        ),
        (
            "GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp",
            "GET",
            "/v1/tactical/bake/0/tiles/z/0/0.webp".into(),
            None,
        ),
        (
            "DELETE /v1/tactical/bake/{id}",
            "DELETE",
            "/v1/tactical/bake/00000000000000000000000000000000".into(),
            None,
        ),
        (
            "GET /v1/tactical/dungeon",
            "GET",
            "/v1/tactical/dungeon?seed=7&w=16&h=16".into(),
            None,
        ),
        (
            "GET /v1/tactical/dungeon.png",
            "GET",
            "/v1/tactical/dungeon.png?seed=7&w=16&h=16&ppsq=64".into(),
            None,
        ),
        (
            "GET /v1/tactical/dungeon/scene",
            "GET",
            "/v1/tactical/dungeon/scene?seed=7&w=16&h=16".into(),
            None,
        ),
    ]
}

/// Routes whose probe names a job no one posted: their `404 not_found`
/// comes from the handler, never from the router's fallback.
const UNKNOWN_ID_OK: [&str; 2] = [
    "GET /v1/tactical/bake/{id}",
    "DELETE /v1/tactical/bake/{id}",
];

struct Ids {
    settlement: String,
    building: String,
    npc: String,
    realm: String,
    relief_z: u64,
}

async fn call(method: &str, uri: &str, body: Option<&str>) -> Reply {
    let mut request = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    let body = body.map_or_else(Body::empty, |b| Body::from(b.to_owned()));
    send(society_state(), request.body(body).unwrap()).await
}

async fn json(uri: &str) -> Value {
    let r = call("GET", uri, None).await;
    assert_eq!(r.status, StatusCode::OK, "{uri}");
    r.json()
}

async fn ids() -> Ids {
    let list = json("/v1/settlements?tier=village").await;
    let settlement = list["settlements"][0]["id"].as_str().unwrap().to_owned();
    let page = json(&format!("/v1/npcs?settlement={settlement}&limit=1")).await;
    let npc = page["npcs"][0]["npc_ref"].as_str().unwrap().to_owned();
    let building = npc.rsplit_once('.').unwrap().0.to_owned();
    let realms = json("/v1/realms").await;
    let realm = realms["realms"][0]["id"].as_str().unwrap().to_owned();
    let world = json("/v1/world").await;
    let relief_z = world["tiles"]["max_zoom"].as_u64().unwrap() + 1;
    Ids {
        settlement,
        building,
        npc,
        realm,
        relief_z,
    }
}

/// The stable rows of the policy table, as `METHOD /v1/...`.
fn stable_routes() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/capstone/versioning.md");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('|').map(str::trim).collect();
            (cols.len() > 3 && cols[1].starts_with('`') && cols[2] == "stable")
                .then(|| cols[1].trim_matches('`').to_owned())
        })
        .collect()
}

#[tokio::test]
async fn every_stable_route_of_the_policy_answers() {
    let stable = stable_routes();
    assert!(stable.len() >= 40, "parsed {} stable rows", stable.len());
    let ids = ids().await;
    let probes = probes(&ids);
    let mut failures = Vec::new();
    for route in &stable {
        let Some((_, method, uri, body)) = probes.iter().find(|p| p.0 == route) else {
            failures.push(format!(
                "{route}: no probe; add one to tests/world_api/inventory.rs"
            ));
            continue;
        };
        let r = call(method, uri, *body).await;
        let fallback = r.status == StatusCode::NOT_FOUND
            && String::from_utf8_lossy(&r.body).contains(FALLBACK);
        let refused = r.status == StatusCode::METHOD_NOT_ALLOWED
            || fallback
            || (r.status == StatusCode::NOT_FOUND && !UNKNOWN_ID_OK.contains(&route.as_str()));
        if refused {
            failures.push(format!(
                "{route}: {method} {uri} answered {} {}",
                r.status,
                String::from_utf8_lossy(&r.body)
            ));
        }
    }
    for (route, ..) in &probes {
        if !stable.iter().any(|s| s == route) {
            failures.push(format!(
                "{route}: probed but not a stable row of versioning.md"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
