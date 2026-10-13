//! `GET /v1/gazetteer`: the file's features plus the settlements as one
//! filtered list.

use crate::support::{get_with, society_dir, society_state, state, Reply};
use axum::http::StatusCode;
use serde_json::Value;

async fn get(uri: &str) -> Reply {
    get_with(society_state(), uri, None).await
}

async fn ok(uri: &str) -> Value {
    let r = get(uri).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{uri}: {}",
        String::from_utf8_lossy(&r.body)
    );
    r.json()
}

fn items(v: &Value) -> &Vec<Value> {
    v["features"].as_array().unwrap()
}

fn kinds(v: &Value) -> Vec<&str> {
    items(v).iter().map(|f| f["kind"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn lists_features_then_settlements_with_the_files_numbers() {
    let all = ok("/v1/gazetteer").await;
    assert_eq!(all["format_version"], 1);
    let file: Value = serde_json::from_slice(
        &std::fs::read(society_dir().join("society/gazetteer.json")).unwrap(),
    )
    .unwrap();
    let from_file = file["features"].as_array().unwrap();
    assert!(!from_file.is_empty());
    let served = items(&all);
    assert_eq!(&served[..from_file.len()], from_file.as_slice());
    let towns = ok("/v1/settlements").await;
    let n = towns["settlements"].as_array().unwrap().len();
    assert_eq!(served.len(), from_file.len() + n);
    let city = served
        .iter()
        .find(|f| f["kind"] == "city")
        .expect("a city point");
    assert_eq!(city["rank"], 2);
    assert!(city["at_m"].is_array() && city.get("anchor_m").is_none());
}

#[tokio::test]
async fn filters_by_kind_rank_and_box() {
    let seas = ok("/v1/gazetteer?kind=sea,range").await;
    assert!(!items(&seas).is_empty());
    assert!(kinds(&seas).iter().all(|k| matches!(*k, "sea" | "mountain")));
    let top = ok("/v1/gazetteer?min_rank=2").await;
    assert!(items(&top).iter().all(|f| f["rank"].as_u64().unwrap() <= 2));
    let kinds_top = kinds(&top);
    assert!(kinds_top.contains(&"sea") && kinds_top.contains(&"city"));
    let all = ok("/v1/gazetteer").await;
    assert!(items(&top).len() < items(&all).len());
    // A box over the north-west corner keeps only what meets it.
    let part = ok("/v1/gazetteer?bbox=0,0,20000,20000").await;
    assert!(items(&part).len() < items(&all).len());
    for f in items(&part) {
        let e = f.get("extent_m").map_or_else(
            || {
                let a = &f["at_m"];
                vec![a[0].as_i64().unwrap(), a[1].as_i64().unwrap(), a[0].as_i64().unwrap(), a[1].as_i64().unwrap()]
            },
            |e| e.as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect(),
        );
        assert!(e[0] <= 20_000 && e[1] <= 20_000, "{f}");
    }
}

#[tokio::test]
async fn bad_queries_are_400() {
    for uri in [
        "/v1/gazetteer?kind=dragon",
        "/v1/gazetteer?bbox=1,2,3",
        "/v1/gazetteer?bbox=5,0,1,9",
        "/v1/gazetteer?min_rank=9",
        "/v1/gazetteer?min_rank=x",
    ] {
        let r = get(uri).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(r.error_code(), "bad_request");
    }
}

#[tokio::test]
async fn same_bytes_twice() {
    for uri in ["/v1/gazetteer", "/v1/gazetteer?kind=city,town&min_rank=3"] {
        assert_eq!(get(uri).await.body, get(uri).await.body, "{uri}");
    }
}

#[tokio::test]
async fn world_without_society_is_not_found() {
    let r = get_with(state(), "/v1/gazetteer", None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.error_code(), "not_found");
}
