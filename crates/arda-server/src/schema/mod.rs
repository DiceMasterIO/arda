//! JSON Schemas (draft 2020-12) of every public body, generated with
//! schemars from the same Rust types the TypeScript bindings come from
//! (goal 66; logic/16 §api-schema).
//!
//! The committed files live in `bindings/schema/` at the workspace root, one
//! `<Name>.json` per body plus `index.json`. They are served unchanged at
//! `GET /v1/schema/{Name}.json` and `GET /v1/schema`. A schema's name is the
//! TypeScript type's name, so `import type { Npc }` and `Npc.json` describe
//! the same JSON. `cargo test -p arda-server schema` fails when the files
//! are stale; rerun it with `ARDA_BLESS_BINDINGS=1` to rewrite them (the same
//! switch rewrites the TypeScript bindings).

mod bodies;
pub mod routes;

pub use bodies::{SettlementDetail, SettlementList, SettlementNpcs};

use crate::error::{ServerError, ServerResult};
use schemars::JsonSchema;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Workspace-relative directory of the committed schemas.
pub const SCHEMA_DIR: &str = "bindings/schema";

/// The JSON Schema dialect every file declares.
pub const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Version of the `index.json` body.
pub const INDEX_FORMAT: u32 = 1;

/// The committed directory, resolved from this crate's manifest.
#[must_use]
pub fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(SCHEMA_DIR)
}

/// One published schema.
#[derive(Debug, Clone)]
pub struct Entry {
    /// File stem and TypeScript type name.
    pub name: &'static str,
    /// The routes whose body (or part of whose body) it describes.
    pub routes: &'static [&'static str],
    /// The schema document.
    pub schema: Value,
}

fn doc<T: JsonSchema>(name: &str) -> ServerResult<Value> {
    let mut v = serde_json::to_value(schemars::schema_for!(T))
        .map_err(|e| ServerError::Internal(format!("schemars: {e}")))?;
    let obj = v
        .as_object_mut()
        .ok_or_else(|| ServerError::Internal(format!("schema {name} is not an object")))?;
    if obj.get("$schema").and_then(Value::as_str) != Some(DIALECT) {
        return Err(ServerError::Internal(format!(
            "schema {name} is not draft 2020-12"
        )));
    }
    obj.insert("title".into(), Value::from(name));
    Ok(v)
}

macro_rules! catalog {
    ($( $name:literal => $t:ty, [$($route:literal),* $(,)?]; )*) => {
        /// Every published schema, in name order.
        ///
        /// # Errors
        /// [`ServerError::Internal`] if schemars cannot render a type.
        pub fn catalog() -> ServerResult<Vec<Entry>> {
            let mut out = vec![$( Entry {
                name: $name,
                routes: &[$($route),*],
                schema: doc::<$t>($name)?,
            } ),*];
            out.sort_by(|a, b| a.name.cmp(b.name));
            Ok(out)
        }
    };
}

use crate::columnar::AreaCells;
use crate::contract::{CellSample, PointSample};
use crate::dto::{AreaLakes, AreaRivers, Health, WorldInfo};
use crate::error::{ApiError, NotYetError};
use crate::npc_dto::{Npc, NpcDemo, Population, PopulationRequest, Sheet};
use crate::npcs::dto::NpcPage;
use crate::tactical::dto::{TacticalBlockDto, TacticalLayoutDto, TacticalLayouts};
use crate::tactical::library_dto::TacticalLibraryDto;
use crate::tactical::prefetch::{PrefetchAccepted, PrefetchRequest};
use crate::tactical::rules_dto::RulesSidecarDto;
use crate::tactical::scene::TacticalScene;
use crate::tactical::scene_dto::SceneDto;

catalog! {
    "Health" => Health, ["GET /v1/health"];
    "WorldInfo" => WorldInfo, ["GET /v1/world"];
    "CellSample" => CellSample, ["GET /v1/cell/{gx}/{gy}"];
    "PointSample" => PointSample, ["GET /v1/point"];
    "AreaCells" => AreaCells, ["GET /v1/area/{ax}/{ay}/cells?format=json"];
    "AreaRivers" => AreaRivers, ["GET /v1/area/{ax}/{ay}/rivers"];
    "AreaLakes" => AreaLakes, ["GET /v1/area/{ax}/{ay}/lakes"];
    "ApiError" => ApiError, ["every 4xx and 5xx response"];
    "NotYetError" => NotYetError, ["501 from /v1/tactical/cell and /window"];
    "TacticalLayouts" => TacticalLayouts, ["GET /v1/tactical/layouts"];
    "TacticalLayoutDto" => TacticalLayoutDto,
        ["GET /v1/tactical/layout/{name}", "POST /v1/tactical/render (request)"];
    "TacticalBlockDto" => TacticalBlockDto,
        ["GET /v1/tactical/cell/{gx}/{gy}", "GET /v1/tactical/window"];
    "RulesSidecarDto" => RulesSidecarDto, ["TacticalBlockDto.rules"];
    "SceneDto" => SceneDto, ["TacticalBlockDto.scene", "TacticalScene.scene"];
    "TacticalScene" => TacticalScene, ["GET /v1/tactical/cell/{gx}/{gy}/scene"];
    "TacticalLibraryDto" => TacticalLibraryDto, ["GET /v1/tactical/library"];
    "PrefetchRequest" => PrefetchRequest, ["POST /v1/tactical/prefetch (request)"];
    "PrefetchAccepted" => PrefetchAccepted, ["POST /v1/tactical/prefetch"];
    "Npc" => Npc, ["GET /v1/npc/{id}", "GET /v1/npc/demo/{npc_id}"];
    "Sheet" => Sheet, ["Npc.sheet"];
    "NpcPage" => NpcPage,
        ["GET /v1/npcs", "GET /v1/buildings/{building}/residents", "GET /v1/buildings/{building}/workers"];
    "Population" => Population, ["POST /v1/npc/population"];
    "PopulationRequest" => PopulationRequest, ["POST /v1/npc/population (request)"];
    "NpcDemo" => NpcDemo, ["GET /v1/npc/demo"];
    "SettlementList" => SettlementList, ["GET /v1/settlements"];
    "SettlementDetail" => SettlementDetail, ["GET /v1/settlements/{id}"];
    "Settlement" => arda_settle::model::Settlement, ["SettlementList.settlements[]", "SettlementDetail.record"];
    "SettlementSociety" => arda_society::SettlementSociety, ["SettlementDetail.society"];
    "TownPlan" => arda_town::TownPlan, ["GET /v1/settlements/{id}/plan"];
    "SettlementNpcs" => SettlementNpcs, ["GET /v1/settlements/{id}/npcs"];
}

fn pretty(v: &Value) -> ServerResult<String> {
    let mut text =
        serde_json::to_string_pretty(v).map_err(|e| ServerError::Internal(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// The `index.json` body: every schema's name, URL and routes.
#[must_use]
pub fn index(entries: &[Entry]) -> Value {
    let schemas: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "url": format!("/v1/schema/{}.json", e.name),
                "routes": e.routes,
            })
        })
        .collect();
    json!({
        "format_version": INDEX_FORMAT,
        "api_version": crate::dto::API_VERSION,
        "dialect": DIALECT,
        "note": "Schemas describe Arda's own NPC shape; a server started with --sheet-mapping reshapes every Npc it serves (header X-Arda-Sheet-Mapping) and the Npc parts of these schemas no longer apply to it.",
        "schemas": schemas,
    })
}

/// Every file as `(file name, contents)`, sorted by name, `index.json` last.
///
/// # Errors
/// [`ServerError::Internal`] if a schema cannot be rendered.
pub fn render() -> ServerResult<Vec<(String, String)>> {
    let entries = catalog()?;
    let mut files = Vec::with_capacity(entries.len() + 1);
    for e in &entries {
        files.push((format!("{}.json", e.name), pretty(&e.schema)?));
    }
    files.push(("index.json".to_owned(), pretty(&index(&entries))?));
    Ok(files)
}

/// The rendered files, built once per process.
///
/// # Errors
/// [`ServerError::Internal`] if a schema cannot be rendered.
pub fn files() -> ServerResult<&'static [(String, String)]> {
    static FILES: OnceLock<Result<Vec<(String, String)>, String>> = OnceLock::new();
    FILES
        .get_or_init(|| render().map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| ServerError::Internal(e.clone()))
}

#[cfg(test)]
mod tests;
