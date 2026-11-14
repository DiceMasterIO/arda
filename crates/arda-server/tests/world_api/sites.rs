//! Site routes and invariants (logic/18 §site-serve) on MICRO seed 42.

use crate::support::{get_with, society_state};
use axum::http::StatusCode;
use serde_json::Value;

async fn get(uri: &str) -> crate::support::Reply {
    get_with(society_state(), uri, None).await
}

async fn sites() -> Vec<Value> {
    get("/v1/sites").await.json()["sites"]
        .as_array()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn the_listing_is_deterministic_dense_enough_and_ordered() {
    let a = get("/v1/sites").await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(a.body, get("/v1/sites").await.body);
    let list = sites().await;
    assert!((35..=70).contains(&list.len()), "{} sites", list.len());
    let count = |k: &str| list.iter().filter(|s| s["kind"] == k).count();
    assert!(count("dungeon") >= 15 && count("cave") >= 15);
    let ids: Vec<u64> = list
        .iter()
        .map(|s| s["id"].as_str().unwrap().parse().unwrap())
        .collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    let mut cells: Vec<_> = list.iter().map(|s| s["cell"].to_string()).collect();
    cells.sort();
    cells.dedup();
    assert_eq!(cells.len(), list.len());
    assert_eq!(a.json()["format_version"], 1);
}

#[tokio::test]
async fn every_site_has_rising_tiers_allowed_sizes_and_a_settlement_only_for_dungeons() {
    for s in sites().await {
        let levels = s["levels"].as_array().unwrap();
        assert!((1..=3).contains(&levels.len()));
        let mut last = (0, 0);
        for (i, l) in levels.iter().enumerate() {
            assert_eq!(l["level"], i as u64 + 1);
            let size = (l["width"].as_u64().unwrap(), l["height"].as_u64().unwrap());
            assert!([(32, 24), (48, 36), (72, 56)].contains(&size));
            assert!(size.0 * size.1 >= last.0 * last.1);
            last = size;
            let tier = l["danger_tier"].as_u64().unwrap();
            assert_eq!(tier, levels[0]["danger_tier"].as_u64().unwrap() + i as u64);
            assert!((1..=4).contains(&tier));
        }
        assert_eq!(s["settlement_id"].is_null(), s["kind"] == "cave");
        let cell = s["cell"].as_array().unwrap();
        let gs = s["entrance_gs"].as_array().unwrap();
        assert_eq!(gs[0].as_i64().unwrap(), cell[0].as_i64().unwrap() * 64 + 32);
        assert_eq!(gs[1].as_i64().unwrap(), cell[1].as_i64().unwrap() * 64 + 32);
    }
}

#[tokio::test]
async fn every_level_serves_its_body_scene_links_and_traps() {
    for s in sites().await {
        let id = s["id"].as_str().unwrap();
        assert_eq!(get(&format!("/v1/sites/{id}")).await.json(), s);
        let levels = s["levels"].as_array().unwrap();
        let mut sealed = 0;
        for l in levels {
            let n = l["level"].as_u64().unwrap();
            let body = get(&format!("/v1/sites/{id}/levels/{n}")).await;
            assert_eq!(body.status, StatusCode::OK, "{id} level {n}");
            let v = body.json();
            // The published mirror reads the served body.
            serde_json::from_slice::<arda_server::sites::dto::SiteLevel>(&body.body).unwrap();
            assert_eq!(v["danger_tier"], l["danger_tier"]);
            let q = format!(
                "/v1/tactical/dungeon?seed={}&kind={}&w={}&h={}",
                l["seed"].as_str().unwrap(),
                s["kind"].as_str().unwrap(),
                l["width"],
                l["height"]
            );
            assert_eq!(v["dungeon"], get(&q).await.json(), "{id} level {n}");
            let links = v["links"].as_array().unwrap();
            assert_eq!(links.len(), v["dungeon"]["exits"].as_array().unwrap().len());
            sealed += links.iter().filter(|k| k["to"] == "sealed").count();
            if n == 1 {
                assert_eq!(links.iter().filter(|k| k["to"] == "surface").count(), 1);
            }
            for t in v["traps"].as_array().unwrap() {
                assert!(t["id"].as_str().unwrap().starts_with('t'));
                assert!(!t["squares"].as_array().unwrap().is_empty());
            }
            let scene = get(&format!("/v1/sites/{id}/levels/{n}/scene")).await;
            assert_eq!(scene.status, StatusCode::OK, "{id} level {n} scene");
        }
        assert_eq!(sealed, 1, "{id}");
    }
}

#[tokio::test]
async fn the_level_image_is_sealed_only_on_the_deepest_level() {
    let list = sites().await;
    let s = list
        .iter()
        .find(|s| s["levels"].as_array().unwrap().len() >= 2)
        .unwrap();
    let id = s["id"].as_str().unwrap();
    let levels = s["levels"].as_array().unwrap();
    for l in levels {
        let n = l["level"].as_u64().unwrap();
        let png = get(&format!("/v1/sites/{id}/levels/{n}.png?ppsq=64")).await;
        assert_eq!(png.status, StatusCode::OK);
        let plain = get(&format!(
            "/v1/tactical/dungeon.png?seed={}&kind={}&w={}&h={}&ppsq=64",
            l["seed"].as_str().unwrap(),
            s["kind"].as_str().unwrap(),
            l["width"],
            l["height"]
        ))
        .await;
        let deepest = n == levels.len() as u64;
        assert_eq!(png.body != plain.body, deepest, "{id} level {n}");
    }
}

#[tokio::test]
async fn filters_and_bad_requests() {
    let all = sites().await;
    let caves = get("/v1/sites?kind=cave").await.json()["sites"]
        .as_array()
        .unwrap()
        .len();
    let dungeons = get("/v1/sites?kind=dungeon").await.json()["sites"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(caves + dungeons, all.len());
    let s = &all[0];
    let gs = s["entrance_gs"].as_array().unwrap();
    let (x, y) = (
        gs[0].as_f64().unwrap() * 1.5625,
        gs[1].as_f64().unwrap() * 1.5625,
    );
    let near = get(&format!(
        "/v1/sites?bbox={},{},{},{}",
        x - 10.0,
        y - 10.0,
        x + 10.0,
        y + 10.0
    ))
    .await;
    assert_eq!(near.json()["sites"].as_array().unwrap(), &vec![s.clone()]);
    assert!(get("/v1/sites?bbox=-9,-9,-1,-1").await.json()["sites"]
        .as_array()
        .unwrap()
        .is_empty());
    let id = s["id"].as_str().unwrap();
    for (uri, status) in [
        ("/v1/sites?bbox=1,2,3".to_owned(), StatusCode::BAD_REQUEST),
        ("/v1/sites?bbox=5,5,1,1".to_owned(), StatusCode::BAD_REQUEST),
        ("/v1/sites?kind=crypt".to_owned(), StatusCode::BAD_REQUEST),
        ("/v1/sites/1".to_owned(), StatusCode::NOT_FOUND),
        ("/v1/sites/abc".to_owned(), StatusCode::NOT_FOUND),
        (format!("/v1/sites/{id}/levels/0"), StatusCode::NOT_FOUND),
        (format!("/v1/sites/{id}/levels/9"), StatusCode::NOT_FOUND),
        (format!("/v1/sites/{id}/levels/x"), StatusCode::BAD_REQUEST),
        (
            format!("/v1/sites/{id}/levels/1.png?ppsq=50"),
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("/v1/sites/{id}/levels/1.png?grid=2"),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(get(&uri).await.status, status, "{uri}");
    }
}

/// logic/18 §site-place: without `society/` there are no settlements to
/// hang a dungeon on, and no town to lower a tier.
#[test]
fn a_world_without_society_has_caves_only() {
    let state = society_state();
    let placed =
        arda_server::sites::Sites::place(&state.query, None, state.tactical.library()).unwrap();
    assert!(!placed.all().is_empty());
    for s in placed.all() {
        assert_eq!(s.kind, arda_dungeon::Kind::Cave);
        assert_eq!(s.settlement, None);
        assert_eq!(s.levels[0].danger_tier, 2);
    }
}
