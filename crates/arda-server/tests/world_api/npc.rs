//! `/v1/npc`: the market-town demo, single-NPC regeneration and posted
//! populations (`logic/13`, `logic/16`).

use crate::support::{get, post_json};
use arda_npc::sample::market_town;
use arda_npc::{Generator, Npc, NpcId, Population};
use axum::http::StatusCode;
use serde_json::{json, Value};

fn demo_population(demo: &Value) -> Population {
    serde_json::from_value(demo["population"].clone()).unwrap()
}

#[tokio::test]
async fn demo_is_the_market_town_example() {
    let reply = get("/v1/npc/demo").await;
    assert_eq!(reply.status, StatusCode::OK);
    let demo = reply.json();
    let (seed, settlement, buildings) = market_town();
    assert_eq!(demo["world_seed"], json!(seed.to_string()));
    assert_eq!(demo["settlement"]["name"], json!("Wendlebrook"));
    let expected = Generator::new(seed, &settlement, &buildings)
        .unwrap()
        .population()
        .unwrap();
    assert_eq!(demo_population(&demo), expected);
    assert_eq!(demo["population"]["population"], json!(1500));
}

#[tokio::test]
async fn every_demo_notable_regenerates_to_its_population_entry() {
    let population = demo_population(&get("/v1/npc/demo").await.json());
    assert!(!population.npcs.is_empty());
    for stored in &population.npcs {
        let reply = get(&format!("/v1/npc/demo/{}", stored.id)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", stored.id);
        let fresh: Npc = serde_json::from_slice(&reply.body).unwrap();
        assert_eq!(&fresh, stored, "npc {} differs", stored.id);
    }
}

#[tokio::test]
async fn demo_commoners_regenerate_to_their_roster_entry() {
    let population = demo_population(&get("/v1/npc/demo").await.json());
    let (seed, settlement, buildings) = market_town();
    let generator = Generator::new(seed, &settlement, &buildings).unwrap();
    let mut checked = 0;
    for entry in population.roster.iter().filter(|r| !r.notable).step_by(97) {
        let reply = get(&format!("/v1/npc/demo/{}", entry.id)).await;
        assert_eq!(reply.status, StatusCode::OK);
        let fresh: Npc = serde_json::from_slice(&reply.body).unwrap();
        assert_eq!(fresh, generator.npc(entry.id).unwrap());
        assert_eq!(fresh.workplace_building, entry.workplace);
        assert_eq!(population.job_of(entry.id), Some(fresh.job.key.as_str()));
        checked += 1;
    }
    assert!(checked >= 10, "only {checked} commoners");
}

#[tokio::test]
async fn unknown_and_malformed_npc_ids_are_refused() {
    let reply = get(&format!("/v1/npc/demo/{}", NpcId(12345))).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.error_code(), "not_found");
    let reply = get("/v1/npc/demo/7.1.0").await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.error_code(), "bad_request");
}

fn request(seed: &str) -> Value {
    let (_, settlement, buildings) = market_town();
    json!({ "world_seed": seed, "settlement": settlement, "buildings": buildings })
}

#[tokio::test]
async fn posted_population_equals_the_library() {
    let (seed, settlement, buildings) = market_town();
    let body = serde_json::to_vec(&request(&seed.to_string())).unwrap();
    let reply = post_json("/v1/npc/population", body).await;
    assert_eq!(reply.status, StatusCode::OK);
    let population: Population = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(
        population,
        arda_npc::generate_population(seed, &settlement, &buildings).unwrap()
    );
}

#[tokio::test]
async fn posted_populations_are_limited_and_validated() {
    let bad_seed = serde_json::to_vec(&request("-1")).unwrap();
    let reply = post_json("/v1/npc/population", bad_seed).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    let mut big = request("1");
    big["settlement"]["population"] = json!(arda_server::npc::MAX_POPULATION + 1);
    let reply = post_json("/v1/npc/population", serde_json::to_vec(&big).unwrap()).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(reply.error_code(), "payload_too_large");

    let mut crowded = request("1");
    crowded["settlement"]["population"] = json!(40_000);
    let reply = post_json("/v1/npc/population", serde_json::to_vec(&crowded).unwrap()).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "not enough housing");

    let mut padded = serde_json::to_vec(&request("1")).unwrap();
    padded.pop();
    padded.extend(std::iter::repeat_n(b' ', arda_server::npc::MAX_BODY_BYTES));
    padded.push(b'}');
    let reply = post_json("/v1/npc/population", padded).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);

    let reply = post_json("/v1/npc/population", b"{\"world_seed\": 1}".to_vec()).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.error_code(), "bad_request");
}
