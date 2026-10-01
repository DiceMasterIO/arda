//! Schema-only descriptions of the settlement bodies (`crate::people`),
//! which the handlers assemble from the stored records as JSON. The record
//! types themselves are the domain types (`arda-settle`, `arda-society`,
//! `arda-town` with their `schema` feature), so the schemas follow them.

use crate::npc_dto::Npc;
use schemars::JsonSchema;

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
