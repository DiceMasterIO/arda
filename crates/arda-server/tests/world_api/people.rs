//! World people (goals 51, 57): commoners resolve by reference, queries page
//! deterministically and never materialise a settlement at once.

use crate::support::{get_with, society_state, Reply};
use arda_npc::{BuildingId, NpcId, SettlementId};
use axum::http::StatusCode;
use serde_json::Value;
use std::collections::BTreeSet;

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

/// The most populous village (ties by id), and its record.
async fn village() -> Value {
    let all = ok("/v1/settlements?tier=village").await;
    all["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|s| {
            (
                s["population"].as_u64().unwrap(),
                std::cmp::Reverse(s["id"].as_str().unwrap().parse::<u64>().unwrap()),
            )
        })
        .unwrap()
        .clone()
}

fn by_id(mut v: Vec<&Value>) -> Vec<&Value> {
    v.sort_by_key(|n| n["id"].as_str().unwrap().parse::<u64>().unwrap());
    v
}

fn entries(page: &Value) -> &Vec<Value> {
    page["npcs"].as_array().unwrap()
}

/// Every page of `uri` (which already has a `?`), following cursors.
async fn all_pages(uri: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut next: Option<String> = None;
    loop {
        let u = match &next {
            Some(c) => format!("{uri}&cursor={c}"),
            None => uri.to_owned(),
        };
        let page = ok(&u).await;
        assert!(page["scanned_settlements"].as_u64().unwrap() <= 16);
        out.extend(entries(&page).iter().cloned());
        match page["next_cursor"].as_str() {
            Some(c) => next = Some(c.to_owned()),
            None => return out,
        }
    }
}

#[tokio::test]
async fn a_settlement_pages_through_everyone_once_in_order() {
    let v = village().await;
    let id = v["id"].as_str().unwrap();
    let everyone = all_pages(&format!("/v1/npcs?settlement={id}&limit=200")).await;
    assert_eq!(everyone.len() as u64, v["population"].as_u64().unwrap());
    let refs: Vec<(u64, u32)> = everyone
        .iter()
        .map(|e| {
            let r = e["npc_ref"].as_str().unwrap();
            let parts: Vec<&str> = r.split('.').collect();
            assert_eq!(parts[0], id);
            (parts[1].parse().unwrap(), parts[2].parse().unwrap())
        })
        .collect();
    assert!(refs.windows(2).all(|w| w[0] < w[1]), "skeleton order");
    // Smaller pages give the same sequence.
    let small = all_pages(&format!("/v1/npcs?settlement={id}&limit=97")).await;
    assert_eq!(small, everyone);
    // The notables among them are exactly the stored ones, byte for byte.
    let stored = ok(&format!("/v1/settlements/{id}/npcs")).await;
    let notables = by_id(
        everyone
            .iter()
            .filter(|e| e["notable"] == true)
            .map(|e| &e["npc"])
            .collect(),
    );
    let stored = by_id(stored["npcs"].as_array().unwrap().iter().collect());
    assert!(!stored.is_empty());
    assert_eq!(notables, stored);
    // Pages are deterministic.
    let a = get(&format!("/v1/npcs?settlement={id}&limit=5")).await;
    let b = get(&format!("/v1/npcs?settlement={id}&limit=5")).await;
    assert_eq!(a.body, b.body);
}

#[tokio::test]
async fn commoners_resolve_by_reference_and_by_id_within_their_settlement() {
    let v = village().await;
    let id = v["id"].as_str().unwrap();
    let page = ok(&format!("/v1/npcs?settlement={id}&notable=false&limit=4")).await;
    assert_eq!(entries(&page).len(), 4);
    for e in entries(&page) {
        assert_eq!(e["notable"], false);
        let r = e["npc_ref"].as_str().unwrap();
        let by_ref = ok(&format!("/v1/npc/{r}")).await;
        assert_eq!(by_ref, e["npc"], "{r}");
        let npc_id = e["npc"]["id"].as_str().unwrap();
        let parts: Vec<u64> = r.split('.').map(|p| p.parse().unwrap()).collect();
        assert_eq!(
            npc_id,
            NpcId::from_parts(
                SettlementId(parts[0]),
                BuildingId(parts[1]),
                u32::try_from(parts[2]).unwrap()
            )
            .to_string()
        );
        assert_eq!(
            ok(&format!("/v1/npc/{npc_id}?settlement={id}")).await,
            by_ref
        );
        let bare = get(&format!("/v1/npc/{npc_id}")).await;
        assert_eq!(bare.status, StatusCode::NOT_FOUND);
    }
    // A notable by reference is its stored copy.
    let notable = ok(&format!("/v1/npcs?settlement={id}&notable=true&limit=1")).await;
    let e = &entries(&notable)[0];
    let stored = ok(&format!("/v1/npc/{}", e["npc"]["id"].as_str().unwrap())).await;
    assert_eq!(
        ok(&format!("/v1/npc/{}", e["npc_ref"].as_str().unwrap())).await,
        stored
    );
    for (uri, status) in [
        (format!("/v1/npc/{id}.999999.0"), StatusCode::NOT_FOUND),
        (format!("/v1/npc/{id}.x.0"), StatusCode::BAD_REQUEST),
        ("/v1/npc/1.2".to_owned(), StatusCode::BAD_REQUEST),
        ("/v1/npc/999999999.1.0".to_owned(), StatusCode::NOT_FOUND),
        (
            format!("/v1/npc/{id}.1.0?settlement=1"),
            StatusCode::BAD_REQUEST,
        ),
        (format!("/v1/npc/1?town={id}"), StatusCode::BAD_REQUEST),
    ] {
        assert_eq!(get(&uri).await.status, status, "{uri}");
    }
}

#[tokio::test]
async fn filters_hold_on_every_match() {
    let v = village().await;
    let id = v["id"].as_str().unwrap();
    let realm = v["realm_id"].as_str().unwrap();
    let first = ok(&format!("/v1/npcs?settlement={id}&notable=false&limit=1")).await;
    let job = entries(&first)[0]["npc"]["job"]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let jobs = all_pages(&format!("/v1/npcs?settlement={id}&job={job}&limit=200")).await;
    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|e| e["npc"]["job"]["key"] == job.as_str()));
    let notables = all_pages(&format!("/v1/npcs?settlement={id}&notable=true&limit=200")).await;
    assert!(notables.iter().all(|e| e["notable"] == true));
    // A realm-wide query is bounded per page and stays inside the realm.
    let page = ok(&format!("/v1/npcs?realm={realm}&notable=true&limit=200")).await;
    assert!(page["scanned_settlements"].as_u64().unwrap() <= 16);
    let in_realm: BTreeSet<String> = ok(&format!("/v1/settlements?realm={realm}")).await
        ["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(entries(&page)
        .iter()
        .all(|e| in_realm.contains(e["settlement_id"].as_str().unwrap())));
    let other = ok(&format!("/v1/npcs?settlement={id}&realm=999999")).await;
    assert!(entries(&other).is_empty() && other["next_cursor"].is_null());
    for uri in [
        "/v1/npcs?limit=0".to_owned(),
        "/v1/npcs?limit=201".to_owned(),
        "/v1/npcs?notable=maybe".to_owned(),
        "/v1/npcs?building=5".to_owned(),
        "/v1/npcs?colour=red".to_owned(),
        "/v1/npcs?cursor=zz".to_owned(),
        format!("/v1/npcs?settlement={id}&cursor=1.0"),
    ] {
        assert_eq!(get(&uri).await.status, StatusCode::BAD_REQUEST, "{uri}");
    }
    assert_eq!(
        get("/v1/npcs?settlement=999999999").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn buildings_list_their_residents_and_workers() {
    let v = village().await;
    let id = v["id"].as_str().unwrap();
    // A notable's workplace has workers, and their home has residents.
    let notable = ok(&format!("/v1/npcs?settlement={id}&notable=true&limit=200")).await;
    let npc = entries(&notable)
        .iter()
        .map(|e| &e["npc"])
        .find(|n| n["workplace_building"].is_string())
        .unwrap();
    let home = npc["home_building"].as_str().unwrap();
    let work = npc["workplace_building"].as_str().unwrap();
    let residents = all_pages(&format!("/v1/buildings/{id}.{home}/residents?limit=200")).await;
    assert!(residents.iter().any(|e| e["npc"]["id"] == npc["id"]));
    assert!(residents.iter().all(|e| e["npc"]["home_building"] == home));
    let workers = all_pages(&format!("/v1/buildings/{id}.{work}/workers?limit=200")).await;
    assert!(workers.iter().any(|e| e["npc"]["id"] == npc["id"]));
    assert!(workers
        .iter()
        .all(|e| e["npc"]["workplace_building"] == work));
    let bare = ok(&format!(
        "/v1/buildings/{home}/residents?settlement={id}&limit=200"
    ))
    .await;
    assert_eq!(entries(&bare), &residents);
    // `/v1/npcs?building=` is lives-or-works.
    let either = all_pages(&format!("/v1/npcs?building={id}.{work}&limit=200")).await;
    assert!(workers.iter().all(|w| either.contains(w)));
    assert_eq!(
        get(&format!("/v1/buildings/{id}.999999/residents"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&format!("/v1/buildings/{home}/residents")).await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(&format!("/v1/buildings/{id}.{home}/residents?job=x"))
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
}
