//! Schema-only descriptions of the settlement bodies (`crate::people`),
//! which the handlers assemble from the stored records as JSON. The record
//! types themselves are the domain types (`arda-settle`, `arda-society`,
//! `arda-town` with their `schema` feature), so the schemas follow them.

use crate::npc_dto::Npc;
use schemars::JsonSchema;
use serde::Serialize;

/// `GET /v1/settlements[?tier=&realm=]`.
#[derive(Debug, JsonSchema)]
pub struct SettlementList {
    /// Version of this body (1).
    pub format_version: u32,
    /// The `arda-settle` records, filtered.
    pub settlements: Vec<arda_settle::model::Settlement>,
}

/// `GET /v1/settlements/{id}`.
#[derive(Debug, JsonSchema)]
pub struct SettlementDetail {
    /// The `arda-settle` record.
    pub record: arda_settle::model::Settlement,
    /// Its `arda-society` record, or `null` when the society layer has none.
    pub society: Option<arda_society::SettlementSociety>,
}

/// `GET /v1/settlements/{id}/npcs`: the stored notables.
#[derive(Debug, JsonSchema)]
pub struct SettlementNpcs {
    /// The settlement, a decimal string.
    pub settlement_id: String,
    /// Its stored notables with their sheets.
    pub npcs: Vec<Npc>,
}

/// The present ruler of a realm in a [`RealmSummary`].
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RealmRuler {
    /// Given name and regnal number (when the reign has one).
    pub name: String,
    /// Title ("Duke", "Queen").
    pub title: String,
}

/// One entry of `GET /v1/realms`: the `realms.json` record without
/// `borders`, plus the politics headline when the society layer exists.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RealmSummary {
    /// Realm id, a decimal string.
    pub id: String,
    /// Name.
    pub name: String,
    /// English gloss of the name.
    pub name_gloss: String,
    /// Seat settlement id, a decimal string.
    pub seat: String,
    /// Culture of the seat.
    pub culture: String,
    /// Member settlement ids, decimal strings.
    pub settlements: Vec<String>,
    /// Total population of members.
    pub population: u64,
    /// Land cells (hectares).
    pub land_cells: u64,
    /// Realms sharing a land border, decimal strings.
    pub neighbours: Vec<String>,
    /// Formal style ("the Duchy of Veld"); absent before `arda society build`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// `kingdom`, `duchy` or `county`; absent before `arda society build`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
    /// Readable government; absent before `arda society build`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub government_label: Option<String>,
    /// Present ruler; absent before `arda society build`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruler: Option<RealmRuler>,
}

/// `GET /v1/realms`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RealmList {
    /// Version of this body (1).
    pub format_version: u32,
    /// Realms in id order.
    pub realms: Vec<RealmSummary>,
}

/// `GET /v1/realms/{id}`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RealmDetail {
    /// The `realms.json` record, with borders.
    pub record: arda_settle::realms::Realm,
    /// The realm today with its hooks, or `null` before `arda society build`.
    pub politics: Option<arda_society::RealmSociety>,
    /// Relations involving this realm.
    pub relations: Vec<arda_society::politics::relations::Relation>,
    /// History events that list this realm.
    pub events: Vec<arda_society::history::Event>,
    /// Reigns of this realm, oldest first.
    pub reigns: Vec<arda_society::history::Reign>,
}
