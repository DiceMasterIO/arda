//! Bodies of the world-people queries (goal 57; logic/16 §api-routes
//! `NpcPage`). [`NpcPage`] and [`NpcEntry`] are the TypeScript mirrors;
//! handlers serialise [`PageOut`], which borrows the domain `arda_npc::Npc`
//! and writes the same JSON (tested below).

use crate::npc_dto;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Version of the [`NpcPage`] body.
pub const NPC_PAGE_FORMAT: u32 = 1;

/// One person of a query page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpcEntry {
    /// `"<settlement>.<building>.<index>"`: resolves with `GET /v1/npc/{ref}`,
    /// commoners included.
    pub npc_ref: String,
    /// The settlement, a decimal string.
    pub settlement_id: String,
    /// Whether the person is a stored notable.
    pub notable: bool,
    /// The person with their SRD 5.1 sheet.
    pub npc: npc_dto::Npc,
}

/// A page of people (TS `NpcPage`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpcPage {
    /// [`NPC_PAGE_FORMAT`].
    pub format_version: u32,
    /// The matches, in settlement-id then skeleton order (home building id,
    /// then resident index).
    pub npcs: Vec<NpcEntry>,
    /// Pass as `?cursor=` for the next page; `null` on the last one.
    pub next_cursor: Option<String>,
    /// Settlements whose skeletons this page scanned (bounded per page).
    pub scanned_settlements: u32,
}

/// Wire form of [`NpcEntry`] over the domain record.
#[derive(Debug, Clone, Serialize)]
pub struct EntryOut {
    /// See [`NpcEntry::npc_ref`].
    pub npc_ref: String,
    /// See [`NpcEntry::settlement_id`].
    pub settlement_id: String,
    /// See [`NpcEntry::notable`].
    pub notable: bool,
    /// The domain record.
    pub npc: arda_npc::Npc,
}

/// Wire form of [`NpcPage`] over the domain records.
#[derive(Debug, Clone, Serialize)]
pub struct PageOut {
    /// See [`NpcPage::format_version`].
    pub format_version: u32,
    /// See [`NpcPage::npcs`].
    pub npcs: Vec<EntryOut>,
    /// See [`NpcPage::next_cursor`].
    pub next_cursor: Option<String>,
    /// See [`NpcPage::scanned_settlements`].
    pub scanned_settlements: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_npc::{sample, Generator};

    #[test]
    fn the_wire_page_reads_as_its_mirror_unchanged() {
        let (seed, settlement, buildings) = sample::market_town();
        let g = Generator::new(seed, &settlement, &buildings).unwrap();
        let npcs = (0..3)
            .map(|p| {
                let r = g.resident(p).unwrap();
                EntryOut {
                    npc_ref: format!("{}.{}.{}", settlement.id, r.home, r.index),
                    settlement_id: settlement.id.to_string(),
                    notable: r.notable,
                    npc: g.npc_at(p).unwrap(),
                }
            })
            .collect();
        let out = PageOut {
            format_version: NPC_PAGE_FORMAT,
            npcs,
            next_cursor: Some("1.3".into()),
            scanned_settlements: 1,
        };
        let json = serde_json::to_value(&out).unwrap();
        let mirror: NpcPage = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&mirror).unwrap(), json);
        assert_eq!(mirror.npcs.len(), 3);
    }
}
