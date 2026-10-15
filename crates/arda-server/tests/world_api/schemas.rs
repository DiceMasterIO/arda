//! Published JSON Schemas (goal 66; logic/16 §api-schema): every public body
//! of the MICRO seed-42 world validates against the schema the server
//! publishes for it, and the published files are the committed ones.

use crate::support::{get_with, send, society_state, Reply};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::path::PathBuf;

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

/// Validates `body` against the served schema `name`.
async fn check(name: &str, what: &str, body: &Value) {
    let schema = ok(&format!("/v1/schema/{name}.json")).await;
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("{name}.json does not compile: {e}"));
    let errors: Vec<String> = validator
        .iter_errors(body)
        .take(5)
        .map(|e| {
            let text = e.to_string();
            let short: String = text
                .chars()
                .rev()
                .take(160)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!(
                "…{short} at {} (schema {})",
                e.instance_path(),
                e.schema_path()
            )
        })
        .collect();
    assert!(errors.is_empty(), "{what} against {name}: {errors:#?}");
}

async fn checked(name: &str, uri: &str) -> Value {
    let body = ok(uri).await;
    check(name, uri, &body).await;
    body
}

#[tokio::test]
async fn served_schemas_are_the_committed_files() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bindings/schema");
    let r = get("/v1/schema").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body, std::fs::read(dir.join("index.json")).unwrap());
    let index = r.json();
    check("SchemaIndex", "/v1/schema", &index).await;
    assert_eq!(
        index["dialect"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    let schemas = index["schemas"].as_array().unwrap();
    assert!(schemas.len() >= 25, "{}", schemas.len());
    for s in schemas {
        let url = s["url"].as_str().unwrap();
        let r = get(url).await;
        assert_eq!(r.status, StatusCode::OK, "{url}");
        assert_eq!(
            r.headers["content-type"].to_str().unwrap(),
            "application/schema+json"
        );
        let file = format!("{}.json", s["name"].as_str().unwrap());
        assert_eq!(r.body, std::fs::read(dir.join(&file)).unwrap(), "{file}");
    }
    assert_eq!(
        get("/v1/schema/Nope.json").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get("/v1/schema/Npc").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn world_and_contract_bodies_validate() {
    checked("Health", "/v1/health").await;
    checked("WorldInfo", "/v1/world").await;
    checked("CellSample", "/v1/cell/529/812").await;
    checked("PointSample", "/v1/point?x_m=52950&y_m=81250").await;
    checked("AreaRivers", "/v1/area/1/1/rivers").await;
    checked("AreaLakes", "/v1/area/1/1/lakes").await;
    let missing = get("/v1/cell/999999/0").await;
    assert_ne!(missing.status, StatusCode::OK);
    check("ApiError", "an error body", &missing.json()).await;
}

#[tokio::test]
async fn area_cells_validate() {
    checked("AreaCells", "/v1/area/1/1/cells").await;
}

#[tokio::test]
async fn tactical_bodies_validate() {
    checked("TacticalLayouts", "/v1/tactical/layouts").await;
    checked("TacticalLayoutDto", "/v1/tactical/layout/riverside").await;
    checked("TacticalLibraryDto", "/v1/tactical/library").await;
    let block = ok("/v1/tactical/cell/529/812").await;
    check("RulesSidecarDto", "block rules", &block["rules"]).await;
    check("SceneDto", "block scene", &block["scene"]).await;
    check("TacticalBlockDto", "a block", &block).await;
    let window = ok("/v1/tactical/window?gx0=529&gy0=812&w=2&h=1").await;
    check("TacticalBlockDto", "a window", &window).await;
    let dungeon = checked("DungeonDto", "/v1/tactical/dungeon?seed=9&w=32&h=24").await;
    assert!(
        dungeon["params"]["seed"].is_string(),
        "{}",
        dungeon["params"]
    );
    checked(
        "DungeonDto",
        "/v1/tactical/dungeon?gx=529&gy=812&kind=cave&w=24&h=24",
    )
    .await;
    checked("SceneDto", "/v1/tactical/dungeon/scene?seed=9&w=32&h=24").await;
    let request = Request::post("/v1/tactical/prefetch")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"gx":529,"gy":812,"radius":1,"ppsq":64}"#))
        .unwrap();
    let r = send(society_state(), request).await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    check("PrefetchAccepted", "prefetch", &r.json()).await;
}

/// The most populous village (ties by lowest id).
async fn village() -> Value {
    let all = checked("SettlementList", "/v1/settlements").await;
    all["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["tier"] == "village")
        .max_by_key(|s| {
            (
                s["population"].as_u64().unwrap(),
                std::cmp::Reverse(s["id"].as_str().unwrap().parse::<u64>().unwrap()),
            )
        })
        .unwrap()
        .clone()
}

#[tokio::test]
async fn realm_bodies_validate() {
    let list = checked("RealmList", "/v1/realms").await;
    let realms = list["realms"].as_array().unwrap();
    assert!(!realms.is_empty());
    for r in realms {
        let id = r["id"].as_str().unwrap();
        checked("RealmDetail", &format!("/v1/realms/{id}")).await;
    }
}

#[tokio::test]
async fn settlement_people_and_scene_bodies_validate() {
    let v = village().await;
    let id = v["id"].as_str().unwrap();
    check("Settlement", "a settlement record", &v).await;
    let detail = checked("SettlementDetail", &format!("/v1/settlements/{id}")).await;
    assert!(detail["society"].is_object());
    let plan = checked("TownPlan", &format!("/v1/settlements/{id}/plan")).await;
    let notables = checked("SettlementNpcs", &format!("/v1/settlements/{id}/npcs")).await;
    let page = checked("NpcPage", &format!("/v1/npcs?settlement={id}&limit=20")).await;
    let first = page["npcs"][0]["npc_ref"].as_str().unwrap();
    checked("Npc", &format!("/v1/npc/{first}")).await;
    let notable = notables["npcs"][0]["id"].as_str().unwrap();
    let npc = checked("Npc", &format!("/v1/npc/{notable}")).await;
    check("Sheet", "a notable's sheet", &npc["sheet"]).await;
    // The scene of the village's market square, where its people are: a
    // plan can sit a few cells from the settlement's own cell (the recipe-9
    // fixture's largest village, 116: about 290 m), whose scene is empty.
    let square = plan["square"]["polygon"].as_array().unwrap();
    let mean = |axis: &str| {
        square
            .iter()
            .map(|p| p[axis].as_f64().unwrap())
            .sum::<f64>()
            / square.len() as f64
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (gx, gy) = ((mean("x") / 100.0) as u64, (mean("y") / 100.0) as u64);
    for time in ["day", "night"] {
        let uri = format!("/v1/tactical/cell/{gx}/{gy}/scene?time={time}");
        let scene = checked("TacticalScene", &uri).await;
        assert!(!scene["tokens"].as_array().unwrap().is_empty(), "{uri}");
        assert!(scene["scene"]["movement"][0][0].is_u64());
    }
}

#[tokio::test]
async fn npc_service_bodies_validate() {
    let demo = checked("NpcDemo", "/v1/npc/demo").await;
    let id = demo["population"]["npcs"][0]["id"].as_str().unwrap();
    checked("Npc", &format!("/v1/npc/demo/{id}")).await;
    let request = serde_json::json!({
        "world_seed": demo["world_seed"],
        "settlement": demo["settlement"],
        "buildings": demo["buildings"],
    });
    check("PopulationRequest", "the demo request", &request).await;
    let r = send(
        society_state(),
        Request::post("/v1/npc/population")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&request).unwrap()))
            .unwrap(),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    check("Population", "a posted population", &r.json()).await;
}

/// Contract 3 on the default recipe: the fixture is a recipe-7 world (unless
/// `ARDA_SERVER_TEST_WORLD` names another), and a village cell carrying the
/// society fields validates, alone and in its area's columns.
#[tokio::test]
async fn contract_3_society_cells_validate_on_the_default_recipe() {
    let world = checked("WorldInfo", "/v1/world").await;
    assert_eq!(world["contract_version"], 3);
    if std::env::var_os("ARDA_SERVER_TEST_WORLD").is_none() {
        assert_eq!(
            world["fine_terrain"]["recipe_version"],
            arda::FINE_TERRAIN_RECIPE_VERSION
        );
    }
    let v = village().await;
    let (gx, gy) = (v["cell_x"].as_u64().unwrap(), v["cell_y"].as_u64().unwrap());
    let cell = checked("CellSample", &format!("/v1/cell/{gx}/{gy}")).await;
    assert_eq!(cell["built_by"], v["id"], "{cell}");
    assert!(cell["land_use"].is_string(), "{cell}");
    assert!(cell["realm_id"].is_string(), "{cell}");
    checked(
        "AreaCells",
        &format!("/v1/area/{}/{}/cells", gx / 512, gy / 512),
    )
    .await;
}

#[tokio::test]
async fn generated_npc_bodies_validate() {
    let request = serde_json::json!({
        "seed": "7", "job": "smith", "ancestry": "dwarf", "rank": "tradesfolk",
        "sex": "female", "tier": "village", "culture": "highland", "notable": true,
    });
    check("GenerateNpcRequest", "a generate request", &request).await;
    for body in [serde_json::json!({}), request] {
        let r = send(
            society_state(),
            Request::post("/v1/npc/generate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&r.body)
        );
        check("Npc", "a generated npc", &r.json()).await;
    }
}
