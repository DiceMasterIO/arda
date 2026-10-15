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
#[cfg(test)]
pub(crate) mod compat;
pub mod routes;

pub use bodies::{
    RealmDetail, RealmList, RealmRuler, RealmSummary, SettlementDetail, SettlementList,
    SettlementNpcs, REALM_FORMAT, SETTLEMENT_FORMAT,
};

use crate::error::{ServerError, ServerResult};
use crate::versions::Stability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;
use ts_rs::TS;

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
    /// The version family that covers it (a `GET /v1/world` `versions` key,
    /// or `api_version`).
    pub family: &'static str,
    /// Whether it is frozen (`docs/capstone/versioning.md`).
    pub stability: Stability,
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
    ($( $name:literal => $t:ty, $family:literal, $stability:ident, [$($route:literal),* $(,)?]; )*) => {
        /// Every published schema, in name order.
        ///
        /// # Errors
        /// [`ServerError::Internal`] if schemars cannot render a type.
        pub fn catalog() -> ServerResult<Vec<Entry>> {
            let mut out = vec![$( Entry {
                name: $name,
                family: $family,
                stability: Stability::$stability,
                routes: &[$($route),*],
                schema: doc::<$t>($name)?,
            } ),*];
            out.sort_by(|a, b| a.name.cmp(b.name));
            for e in &mut out {
                // Responses are open (review 3.6-12): a field added in a
                // minor release is additive, so no strict validator may
                // refuse it. Request bodies stay closed: the server refuses
                // unknown fields there.
                if !e.routes.iter().any(|r| r.contains("(request)")) {
                    open_up(&mut e.schema);
                }
            }
            Ok(out)
        }
    };
}

use crate::columnar::AreaCells;
use crate::contract::{CellSample, PointSample};
use crate::dto::{AreaLakes, AreaRivers, Health, WorldInfo};
use crate::error::{ApiError, NotYetError};
use crate::npc_dto::{GenerateNpcRequest, Npc, NpcDemo, Population, PopulationRequest, Sheet};
use crate::npcs::dto::NpcPage;
use crate::tactical::bake::{BakeAccepted, BakeRequest, BakeStatus};
use crate::tactical::dto::{TacticalBlockDto, TacticalLayoutDto, TacticalLayouts};
use crate::tactical::dungeon_dto::DungeonDto;
use crate::tactical::library_dto::TacticalLibraryDto;
use crate::tactical::prefetch::{PrefetchAccepted, PrefetchRequest};
use crate::tactical::rules_dto::RulesSidecarDto;
use crate::tactical::scene::TacticalScene;
use crate::tactical::scene_dto::SceneDto;

catalog! {
    "Health" => Health, "contract_version", Stable, ["GET /v1/health"];
    "WorldInfo" => WorldInfo, "contract_version", Stable, ["GET /v1/world"];
    "CellSample" => CellSample, "contract_version", Stable, ["GET /v1/cell/{gx}/{gy}"];
    "PointSample" => PointSample, "contract_version", Stable, ["GET /v1/point"];
    "AreaCells" => AreaCells, "contract_version", Stable, ["GET /v1/area/{ax}/{ay}/cells?format=json"];
    "AreaRivers" => AreaRivers, "contract_version", Stable, ["GET /v1/area/{ax}/{ay}/rivers"];
    "AreaLakes" => AreaLakes, "contract_version", Stable, ["GET /v1/area/{ax}/{ay}/lakes"];
    "ApiError" => ApiError, "api_version", Stable, ["every 4xx and 5xx response"];
    "NotYetError" => NotYetError, "api_version", Debug, ["501 from /v1/tactical/cell and /window (PendingBlocks test seam only)"];
    "TacticalLayouts" => TacticalLayouts, "tactical_format", Debug, ["GET /v1/tactical/layouts"];
    "TacticalLayoutDto" => TacticalLayoutDto, "layout_schema", Stable,
        ["GET /v1/tactical/layout/{name}", "POST /v1/tactical/render (request)", "DungeonDto.layout"];
    "TacticalBlockDto" => TacticalBlockDto, "tactical_format", Stable,
        ["GET /v1/tactical/cell/{gx}/{gy}", "GET /v1/tactical/window"];
    "RulesSidecarDto" => RulesSidecarDto, "rules_sidecar_format", Stable, ["TacticalBlockDto.rules", "DungeonDto.rules"];
    "SceneDto" => SceneDto, "scene_format", Stable,
        ["GET /v1/tactical/dungeon/scene", "TacticalBlockDto.scene", "TacticalScene.scene"];
    "TacticalScene" => TacticalScene, "tactical_format", Stable, ["GET /v1/tactical/cell/{gx}/{gy}/scene"];
    "TacticalLibraryDto" => TacticalLibraryDto, "tactical_format", Stable, ["GET /v1/tactical/library"];
    "DungeonDto" => DungeonDto, "tactical_format", Stable, ["GET /v1/tactical/dungeon"];
    "PrefetchRequest" => PrefetchRequest, "tactical_format", Stable, ["POST /v1/tactical/prefetch (request)"];
    "PrefetchAccepted" => PrefetchAccepted, "tactical_format", Stable, ["POST /v1/tactical/prefetch"];
    "BakeRequest" => BakeRequest, "bake_format", Stable, ["POST /v1/tactical/bake (request)"];
    "BakeAccepted" => BakeAccepted, "bake_format", Stable, ["POST /v1/tactical/bake"];
    "BakeStatus" => BakeStatus, "bake_format", Stable, ["GET /v1/tactical/bake/{id}"];
    "Npc" => Npc, "npc_format", Stable, ["GET /v1/npc/{id}", "GET /v1/npc/demo/{npc_id}", "POST /v1/npc/generate"];
    "Sheet" => Sheet, "npc_format", Stable, ["Npc.sheet"];
    "NpcPage" => NpcPage, "npc_page_format", Stable,
        ["GET /v1/npcs", "GET /v1/buildings/{building}/residents", "GET /v1/buildings/{building}/workers"];
    "Population" => Population, "npc_format", Stable, ["POST /v1/npc/population"];
    "PopulationRequest" => PopulationRequest, "npc_format", Stable, ["POST /v1/npc/population (request)"];
    "GenerateNpcRequest" => GenerateNpcRequest, "npc_format", Stable, ["POST /v1/npc/generate (request)"];
    "NpcDemo" => NpcDemo, "npc_format", Debug, ["GET /v1/npc/demo"];
    "SettlementList" => SettlementList, "settlement_format", Stable, ["GET /v1/settlements"];
    "SettlementDetail" => SettlementDetail, "settlement_format", Stable, ["GET /v1/settlements/{id}"];
    "Settlement" => arda_settle::model::Settlement, "settlement_format", Stable, ["SettlementList.settlements[]", "SettlementDetail.record"];
    "SettlementSociety" => arda_society::SettlementSociety, "settlement_format", Stable, ["SettlementDetail.society"];
    "TownPlan" => arda_town::TownPlan, "settlement_format", Stable, ["GET /v1/settlements/{id}/plan"];
    "SettlementNpcs" => SettlementNpcs, "settlement_format", Stable, ["GET /v1/settlements/{id}/npcs"];
    "Realm" => arda_settle::realms::Realm, "realm_format", Stable, ["RealmDetail.record"];
    "RealmList" => RealmList, "realm_format", Stable, ["GET /v1/realms"];
    "RealmDetail" => RealmDetail, "realm_format", Stable, ["GET /v1/realms/{id}"];
    "RealmSociety" => arda_society::RealmSociety, "realm_format", Stable, ["RealmDetail.politics"];
    "Relation" => arda_society::politics::relations::Relation, "realm_format", Stable, ["RealmDetail.relations[]"];
    "Event" => arda_society::history::Event, "realm_format", Stable, ["RealmDetail.events[]"];
    "Gazetteer" => arda_settle::gazetteer::GazetteerList, "gazetteer_format", Stable, ["GET /v1/gazetteer"];
    "GazetteerFeature" => arda_settle::gazetteer::GazetteerFeature, "gazetteer_format", Stable,
        ["Gazetteer.features[]", "society/gazetteer.json"];
    "GazetteerPoint" => arda_settle::gazetteer::GazetteerPoint, "gazetteer_format", Stable, ["Gazetteer.features[]"];
    "SchemaIndex" => SchemaIndex, "schema_index_format", Stable, ["GET /v1/schema"];
}

/// Drops every `additionalProperties: false` in `v`.
fn open_up(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if m.get("additionalProperties") == Some(&Value::Bool(false)) {
                m.remove("additionalProperties");
            }
            m.values_mut().for_each(open_up);
        }
        Value::Array(a) => a.iter_mut().for_each(open_up),
        _ => {}
    }
}

fn pretty(v: &Value) -> ServerResult<String> {
    let mut text =
        serde_json::to_string_pretty(v).map_err(|e| ServerError::Internal(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Text of [`SchemaIndex::note`].
const INDEX_NOTE: &str = "Schemas describe Arda's own NPC shape; a server started with --sheet-mapping reshapes every Npc it serves (header X-Arda-Sheet-Mapping) and the Npc parts of these schemas no longer apply to it.";

/// `GET /v1/schema`: the catalogue of published schemas (`index.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaIndex {
    /// Version of this body ([`INDEX_FORMAT`]).
    pub format_version: u32,
    /// The API version (`v1`).
    pub api_version: String,
    /// The JSON Schema dialect every file declares ([`DIALECT`]).
    pub dialect: String,
    /// How `--sheet-mapping` affects the `Npc` schemas.
    pub note: String,
    /// Every schema, in name order.
    pub schemas: Vec<SchemaIndexEntry>,
}

/// One schema in [`SchemaIndex`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaIndexEntry {
    /// File stem and TypeScript type name.
    pub name: String,
    /// Where it is served: `/v1/schema/{name}.json`.
    pub url: String,
    /// Its version family: a `GET /v1/world` `versions` key, or
    /// `api_version` for the error bodies.
    pub family: String,
    /// `stable` (frozen), `unstable` or `debug` (outside the freeze).
    pub stability: Stability,
    /// The routes (or body parts, such as `TacticalBlockDto.scene`) it describes.
    pub routes: Vec<String>,
}

/// The `index.json` body: every schema's name, URL and routes.
#[must_use]
pub fn index(entries: &[Entry]) -> Value {
    let body = SchemaIndex {
        format_version: INDEX_FORMAT,
        api_version: crate::dto::API_VERSION.to_owned(),
        dialect: DIALECT.to_owned(),
        note: INDEX_NOTE.to_owned(),
        schemas: entries
            .iter()
            .map(|e| SchemaIndexEntry {
                name: e.name.to_owned(),
                url: format!("/v1/schema/{}.json", e.name),
                family: e.family.to_owned(),
                stability: e.stability,
                routes: e.routes.iter().map(|r| (*r).to_owned()).collect(),
            })
            .collect(),
    };
    json!(body)
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
