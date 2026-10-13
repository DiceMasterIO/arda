//! `POST /v1/npc/generate` (logic/13 §npc-generate-one, logic/16 route
//! table): one NPC from optional inputs, deterministic, stored nowhere.

use crate::support::{config, send, society_dir, society_state, state, Reply};
use arda_npc::{generate_one, Npc, OneRequest};
use arda_server::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

async fn generate_on(state: Arc<AppState>, body: &Value) -> Reply {
    let request = Request::post("/v1/npc/generate")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    send(state, request).await
}

async fn generate(body: &Value) -> Reply {
    generate_on(society_state(), body).await
}

fn npc(reply: &Reply) -> Npc {
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    serde_json::from_slice(&reply.body).unwrap()
}

fn world_seed() -> u64 {
    society_state().query.world().seed()
}

#[tokio::test]
async fn an_empty_body_is_one_working_adult_and_equals_the_library() {
    let reply = generate(&json!({})).await;
    let served = npc(&reply);
    assert_ne!(served.job.key, "child");
    assert_eq!(
        served,
        generate_one(world_seed(), &OneRequest::default()).unwrap()
    );
    let again = generate(&json!({})).await;
    assert_eq!(reply.body, again.body, "same body, same bytes");
    assert!(reply.headers.get("x-arda-sheet-mapping").is_none());
}

#[tokio::test]
async fn filters_hold_and_salts_differ() {
    let body = json!({"job": "smith", "ancestry": "dwarf", "seed": "7"});
    let a = npc(&generate(&body).await);
    assert_eq!(
        (a.job.key.as_str(), a.ancestry.as_str()),
        ("smith", "dwarf")
    );
    let b = npc(&generate(&json!({"job": "smith", "ancestry": "dwarf", "seed": "8"})).await);
    assert_ne!(a.id, b.id);
    let c = npc(&generate(&json!({"rank": "gentry", "sex": "female", "tier": "town"})).await);
    assert_eq!(c.social_rank, arda_npc::SocialRank::Gentry);
    assert_eq!(c.sex, arda_npc::Sex::Female);
    let d = npc(&generate(&json!({"office": "reeve", "notable": true})).await);
    assert!(d.notable);
    assert_eq!(d.job.title, "Reeve");
}

#[tokio::test]
async fn a_generated_id_resolves_nowhere() {
    let served = npc(&generate(&json!({"job": "farmer"})).await);
    assert!(served.relationships.is_empty());
    let reply = send(
        society_state(),
        Request::get(format!("/v1/npc/{}", served.id))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_stored_settlement_lends_its_culture() {
    let list = send(
        society_state(),
        Request::get("/v1/settlements").body(Body::empty()).unwrap(),
    )
    .await
    .json();
    let record = &list["settlements"][0];
    let id = record["id"].as_str().unwrap();
    let world = society_state().people.clone().unwrap();
    let profile = world.profile(id.parse().unwrap()).unwrap();
    let served = npc(&generate(&json!({"settlement_id": id, "job": "farmer"})).await);
    let expected = generate_one(
        world_seed(),
        &OneRequest {
            job: Some("farmer".into()),
            settlement: Some(profile.clone()),
            ..OneRequest::default()
        },
    )
    .unwrap();
    assert_eq!(served, expected);
    let other = generate_one(
        world_seed(),
        &OneRequest {
            job: Some("farmer".into()),
            ..OneRequest::default()
        },
    )
    .unwrap();
    assert!(
        profile.culture == "heartland" || served != other,
        "borrowing culture {} changed nothing",
        profile.culture
    );

    let reply = generate(&json!({"settlement_id": "999999999"})).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.error_code(), "not_found");
}

#[tokio::test]
async fn a_world_without_society_has_no_settlements_to_borrow() {
    let bare = state();
    if bare.people.is_some() {
        eprintln!("skipped: the fixture world has society/");
        return;
    }
    let reply = generate_on(bare.clone(), &json!({"settlement_id": "1"})).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.error_code(), "not_found");
    npc(&generate_on(bare, &json!({"job": "miller"})).await);
}

#[tokio::test]
async fn bad_bodies_are_refused() {
    for (body, status, code) in [
        (json!({"seed": "-1"}), 400, "bad_request"),
        (json!({"seed": 7}), 400, "bad_request"),
        (json!({"settlement_id": "x"}), 400, "bad_request"),
        (json!({"colour": "red"}), 400, "bad_request"),
        (json!({"job": "astronaut"}), 400, "bad_request"),
        (json!({"rank": "emperor"}), 400, "bad_request"),
        (json!({"job": "smith", "rank": "noble"}), 400, "bad_request"),
        (
            json!({"job": "farmhand", "notable": true}),
            422,
            "unsatisfiable",
        ),
    ] {
        let reply = generate(&body).await;
        assert_eq!(reply.status.as_u16(), status, "{body}");
        assert_eq!(reply.error_code(), code, "{body}");
    }
    let plain = Request::post("/v1/npc/generate")
        .header("content-type", "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    let reply = send(society_state(), plain).await;
    assert_eq!(reply.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(reply.error_code(), "unsupported_media_type");
    let mut padded = b"{".to_vec();
    padded.extend(std::iter::repeat_n(
        b' ',
        arda_server::npc::MAX_GENERATE_BODY_BYTES,
    ));
    padded.push(b'}');
    let big = Request::post("/v1/npc/generate")
        .header("content-type", "application/json")
        .body(Body::from(padded))
        .unwrap();
    let reply = send(society_state(), big).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
}

fn identity_state() -> Arc<AppState> {
    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    Arc::clone(STATE.get_or_init(|| {
        let mut c = config();
        c.world = society_dir().to_path_buf();
        c.sheet_mapping =
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mappings/identity.json"));
        c.prefetch_workers = 0;
        Arc::new(AppState::open(&c).unwrap())
    }))
}

#[tokio::test]
async fn a_sheet_mapping_applies() {
    let reply = generate_on(identity_state(), &json!({"job": "priest"})).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["x-arda-sheet-mapping"], "identity");
}
