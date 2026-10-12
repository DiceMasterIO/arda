//! `GET /v1/realms` and `GET /v1/realms/{id}` over the society fixture:
//! the list agrees with `realms.json`, the politics with `society.json`.

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

fn realms_json() -> Vec<Value> {
    let bytes = std::fs::read(society_dir().join("society/realms.json")).unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    v["realms"].as_array().unwrap().clone()
}

#[tokio::test]
async fn list_matches_realms_json() {
    let list = ok("/v1/realms").await;
    assert_eq!(list["format_version"], 1);
    let listed = list["realms"].as_array().unwrap();
    let files = realms_json();
    assert!(!files.is_empty());
    assert_eq!(listed.len(), files.len());
    let ids: Vec<u64> = listed
        .iter()
        .map(|r| r["id"].as_str().unwrap().parse().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "id order");
    for (l, f) in listed.iter().zip(&files) {
        assert!(l.get("borders").is_none());
        assert_eq!(l["id"], f["id"]);
        assert_eq!(l["name"], f["name"]);
        assert_eq!(l["name_gloss"], f["name_gloss"]);
        assert_eq!(l["seat"], f["seat"]);
        assert_eq!(l["settlements"], f["settlements"]);
    }
    let settlements = ok("/v1/settlements").await;
    for s in settlements["settlements"].as_array().unwrap() {
        let realm = &s["realm_id"];
        if realm.is_null() || realm == "0" {
            continue;
        }
        assert!(
            listed.iter().any(|r| &r["id"] == realm),
            "settlement realm {realm} is not listed"
        );
    }
}

#[tokio::test]
async fn politics_agree_with_realms_json() {
    let list = ok("/v1/realms").await;
    for r in list["realms"].as_array().unwrap() {
        let id = r["id"].as_str().unwrap();
        let d = ok(&format!("/v1/realms/{id}")).await;
        let p = &d["politics"];
        assert_eq!(p["name"], r["name"], "realm {id}");
        assert_eq!(p["seat"], r["seat"], "realm {id}");
        assert_eq!(r["style"], p["style"]);
        assert_eq!(r["rank"], p["rank"]);
        assert_eq!(r["government_label"], p["government_label"]);
        assert_eq!(r["ruler"]["title"], p["ruler"]["title"]);
        assert!(r["ruler"]["name"].as_str().is_some());
    }
}

#[tokio::test]
async fn detail_of_every_realm() {
    let list = ok("/v1/realms").await;
    for r in list["realms"].as_array().unwrap() {
        let id = r["id"].as_str().unwrap();
        let d = ok(&format!("/v1/realms/{id}")).await;
        assert_eq!(d["record"]["id"], id);
        assert!(d["record"]["borders"].is_array());
        for rel in d["relations"].as_array().unwrap() {
            assert!(rel["a"] == id || rel["b"] == id, "relation {rel}");
        }
        for e in d["events"].as_array().unwrap() {
            let realms = e["realms"].as_array().unwrap();
            assert!(realms.iter().any(|x| x == id), "event {e}");
        }
        for g in d["reigns"].as_array().unwrap() {
            assert_eq!(g["realm_id"], id);
        }
    }
}

#[tokio::test]
async fn bad_ids() {
    let r = get("/v1/realms/999999").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.error_code(), "not_found");
    let r = get("/v1/realms/abc").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_code(), "bad_request");
}

#[tokio::test]
async fn same_bytes_twice() {
    for uri in ["/v1/realms", "/v1/realms/1"] {
        assert_eq!(get(uri).await.body, get(uri).await.body, "{uri}");
    }
}

#[tokio::test]
async fn world_without_society_is_not_found() {
    for uri in ["/v1/realms", "/v1/realms/1"] {
        let r = get_with(state(), uri, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(r.error_code(), "not_found");
    }
}
