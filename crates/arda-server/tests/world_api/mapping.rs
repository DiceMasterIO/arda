//! The sheet mapping layer at the HTTP edge (goal 69): a server started with
//! `--sheet-mapping` reshapes every NPC it serves; a bad mapping refuses to
//! start.

use crate::support::{config, get_with, society_dir, society_state, Reply};
use arda_server::{AppState, ServerError};
use axum::http::StatusCode;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn mappings() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mappings")
}

fn mapped_state() -> Arc<AppState> {
    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    Arc::clone(STATE.get_or_init(|| {
        let mut c = config();
        c.world = society_dir().to_path_buf();
        c.sheet_mapping = Some(mappings().join("5e-srd-monster.json"));
        c.prefetch_workers = 0;
        Arc::new(AppState::open(&c).unwrap())
    }))
}

async fn both(uri: &str) -> (Reply, Reply) {
    let plain = get_with(society_state(), uri, None).await;
    let mapped = get_with(mapped_state(), uri, None).await;
    assert_eq!(plain.status, StatusCode::OK, "{uri}");
    assert_eq!(mapped.status, StatusCode::OK, "{uri}");
    assert!(plain.headers.get("x-arda-sheet-mapping").is_none());
    (plain, mapped)
}

fn header(r: &Reply) -> &str {
    r.headers["x-arda-sheet-mapping"].to_str().unwrap()
}

fn same_npc(arda: &Value, srd: &Value) {
    assert_eq!(srd["index"], arda["id"]);
    assert_eq!(srd["strength"], arda["sheet"]["abilities"][0]);
    assert_eq!(srd["hit_points"], arda["sheet"]["hit_points"]);
    assert!(srd.get("sheet").is_none(), "{srd}");
}

#[tokio::test]
async fn every_npc_route_serves_the_mapped_shape() {
    let list = get_with(society_state(), "/v1/settlements?tier=village", None)
        .await
        .json();
    let id = list["settlements"][0]["id"].as_str().unwrap().to_owned();

    let (plain, mapped) = both(&format!("/v1/settlements/{id}/npcs")).await;
    assert_eq!(header(&mapped), "5e-srd-monster");
    let (p, m) = (plain.json(), mapped.json());
    assert_eq!(m["settlement_id"], p["settlement_id"]);
    let notable = p["npcs"][0]["id"].as_str().unwrap().to_owned();
    for (a, b) in p["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .zip(m["npcs"].as_array().unwrap())
    {
        same_npc(a, b);
    }

    let (plain, mapped) = both(&format!("/v1/npc/{notable}")).await;
    same_npc(&plain.json(), &mapped.json());
    assert_eq!(header(&mapped), "5e-srd-monster");

    let (plain, mapped) = both(&format!("/v1/npcs?settlement={id}&notable=false&limit=5")).await;
    let (p, m) = (plain.json(), mapped.json());
    assert_eq!(m["next_cursor"], p["next_cursor"]);
    for (a, b) in p["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .zip(m["npcs"].as_array().unwrap())
    {
        assert_eq!(b["npc_ref"], a["npc_ref"]);
        same_npc(&a["npc"], &b["npc"]);
    }

    let (plain, mapped) = both("/v1/npc/demo").await;
    let (p, m) = (plain.json(), mapped.json());
    same_npc(&p["population"]["npcs"][0], &m["population"]["npcs"][0]);
    let demo = p["population"]["npcs"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (plain, mapped) = both(&format!("/v1/npc/demo/{demo}")).await;
    same_npc(&plain.json(), &mapped.json());

    // Routes without NPCs are untouched, byte for byte.
    let (plain, mapped) = both(&format!("/v1/settlements/{id}")).await;
    assert_eq!(plain.body, mapped.body);
    assert!(mapped.headers.get("x-arda-sheet-mapping").is_none());
}

#[test]
fn a_bad_mapping_refuses_startup() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("arda-server-mapping-test");
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.json");
    std::fs::write(
        &bad,
        r#"{"format":"arda-sheet-mapping","version":1,"name":"bad","base":"empty",
            "rules":[{"to":"/size","from":"/sheet/size","convert":[{"op":"table","table":"sizes"}]}]}"#,
    )
    .unwrap();
    let mut c = config();
    c.sheet_mapping = Some(bad.clone());
    match AppState::open(&c) {
        Err(ServerError::SheetMapping(e)) => {
            assert!(
                e.contains("rules[0].convert[0]") && e.contains("sizes"),
                "{e}"
            );
        }
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
    c.sheet_mapping = Some(dir.join("missing.json"));
    assert!(matches!(
        AppState::open(&c),
        Err(ServerError::SheetMapping(_))
    ));
    c.sheet_mapping = Some(mappings().join("identity.json"));
    let state = AppState::open(&c).unwrap();
    assert_eq!(state.sheet_mapping.as_ref().unwrap().name(), "identity");
}
