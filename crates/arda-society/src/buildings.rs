//! Buildings per settlement: explicit specs when given, otherwise derived
//! from the settlement's building mix with stable ids.
//!
//! Derived ids are 1-based and assigned in building-function key order, then
//! by index within the key, so the same mix always yields the same ids.

use crate::input::{Settlement, WorldSettlements};
use crate::tables::GoodsTable;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One building as society sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingRef {
    /// Building id, unique within the settlement.
    #[serde(with = "crate::ids::str")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub id: u64,
    /// Building-function key (plain snake_case).
    pub function: String,
    /// Free tags; a workshop carries `craft:<name>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl BuildingRef {
    /// The craft of a workshop, from its `craft:` tag.
    #[must_use]
    pub fn craft(&self) -> Option<&str> {
        self.tags.iter().find_map(|t| t.strip_prefix("craft:"))
    }
}

/// Buildings per settlement id, sorted by building id.
#[must_use]
pub fn resolve(world: &WorldSettlements, goods: &GoodsTable) -> BTreeMap<u64, Vec<BuildingRef>> {
    let mut explicit: BTreeMap<u64, Vec<BuildingRef>> = BTreeMap::new();
    for b in &world.buildings {
        explicit
            .entry(b.settlement_id)
            .or_default()
            .push(BuildingRef {
                id: b.id,
                function: b.function.clone(),
                tags: b.tags.clone(),
            });
    }
    let mut out = BTreeMap::new();
    for s in &world.settlements {
        let mut list = explicit
            .remove(&s.id)
            .unwrap_or_else(|| derive(s, &goods.workshop_crafts));
        list.sort_by_key(|b| b.id);
        list.dedup_by_key(|b| b.id);
        out.insert(s.id, list);
    }
    out
}

fn derive(s: &Settlement, crafts: &[String]) -> Vec<BuildingRef> {
    let mut list = Vec::new();
    let mut next = 1_u64;
    for (kind, &count) in &s.buildings {
        for i in 0..count {
            let craft = (kind == "workshop")
                .then(|| crafts.get(crate::num::usize_of(u64::from(i)) % crafts.len().max(1)))
                .flatten();
            list.push(BuildingRef {
                id: next,
                function: kind.clone(),
                tags: craft
                    .map(|c| vec![format!("craft:{c}")])
                    .unwrap_or_default(),
            });
            next += 1;
        }
    }
    list
}

/// First building whose function is in `prefs` (in preference order).
#[must_use]
pub fn first_of<'a>(list: &'a [BuildingRef], prefs: &[String]) -> Option<&'a BuildingRef> {
    prefs
        .iter()
        .find_map(|p| list.iter().find(|b| &b.function == p))
}

/// Count of buildings with function `kind`.
#[must_use]
pub fn count(list: &[BuildingRef], kind: &str) -> usize {
    list.iter().filter(|b| b.function == kind).count()
}
