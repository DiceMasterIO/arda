//! `GET /v1/npc/occupations` (logic/13 §npc-occupations): the job and
//! office keys `POST /v1/npc/generate` takes, the same on every world.

use crate::support::{get, get_with, society_state};
use axum::http::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn lists_the_library_catalogue_on_every_world() {
    let plain = get("/v1/npc/occupations").await;
    assert_eq!(plain.status, StatusCode::OK);
    assert_eq!(
        plain.headers.get("content-type").unwrap(),
        "application/json"
    );
    let want = json!({ "occupations": arda_npc::occupations().unwrap() });
    assert_eq!(plain.json(), want);
    let society = get_with(society_state(), "/v1/npc/occupations", None).await;
    assert_eq!(society.body, plain.body, "bundled data: the same bytes");
}

#[tokio::test]
async fn every_entry_names_its_kind_label_and_tier_ranks() {
    let body = get("/v1/npc/occupations").await.json();
    let list = body["occupations"].as_array().unwrap();
    let kinds = |k: &str| list.iter().filter(|o| o["kind"] == k).count();
    assert_eq!((kinds("job"), kinds("office")), (96, 21));
    let reeve: Vec<&Value> = list.iter().filter(|o| o["key"] == "reeve").collect();
    assert_eq!(reeve.len(), 2, "reeve is a job and an office");
    let smith = list.iter().find(|o| o["key"] == "smith").unwrap();
    assert_eq!(smith["label"], "Master Smith");
    assert_eq!(smith["category"], "craft");
    assert_eq!(smith["tiers"][0]["tier"], "hamlet");
    assert!(smith["tiers"][0]["rank"].is_string());
}
