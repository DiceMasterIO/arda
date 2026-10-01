//! Notable NPC slots per settlement for the NPC generator to fill
//! (deliverable 5). Each slot has a stable id `r<settlement>.<kind>` that
//! depends only on the settlement and the role kind, a building, and a
//! suggested SRD 5.1 stat block or class and level.

use crate::buildings;
use crate::ctx::Ctx;
use crate::input::{Function, Tier};
use crate::politics::factions::Faction;
use crate::politics::realm::RealmState;
use crate::text::Slots;
use serde::{Deserialize, Serialize};

/// An SRD 5.1 class and level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClassLevel {
    /// SRD class name.
    pub class: String,
    /// Level.
    pub level: u8,
}

/// One notable NPC slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NpcRole {
    /// Stable id.
    pub id: String,
    /// Settlement.
    #[serde(with = "crate::ids::str")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub settlement: u64,
    /// Role kind key (`ruler`, `captain`, `high_priest`, …).
    pub kind: String,
    /// Title as the NPC would be addressed.
    pub title: String,
    /// Workplace building.
    #[serde(default, with = "crate::ids::opt")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub building: Option<u64>,
    /// Faction led, if any.
    pub faction: Option<String>,
    /// Suggested SRD 5.1 NPC stat block.
    pub stat_block: Option<String>,
    /// Suggested SRD 5.1 class and level.
    pub class: Option<ClassLevel>,
    /// Given name fixed by history (rulers, vassals).
    pub given_name: Option<String>,
    /// Family name fixed by history.
    pub family_name: Option<String>,
    /// Sex fixed by history.
    pub female: Option<bool>,
    /// What the role does.
    pub duties: String,
}

/// Stable role id.
#[must_use]
pub fn role_id(settlement: u64, kind: &str) -> String {
    format!("r{settlement}.{kind}")
}

/// Head role kind of node `i`: `ruler`, `lord`, `reeve` or `elder`.
#[must_use]
pub fn head_kind(ctx: &Ctx<'_>, i: usize) -> &'static str {
    let s = ctx.s(i);
    let blds = ctx.buildings_of(i);
    let manor = buildings::count(blds, "manor") + buildings::count(blds, "keep") > 0;
    if ctx.is_seat(i) {
        "ruler"
    } else if s.tier.is_urban() || (s.tier == Tier::Village && manor) {
        "lord"
    } else if s.tier == Tier::Village {
        "reeve"
    } else {
        "elder"
    }
}

/// Role kinds a settlement needs regardless of factions.
fn required(ctx: &Ctx<'_>, i: usize) -> Vec<&'static str> {
    let s = ctx.s(i);
    let blds = ctx.buildings_of(i);
    let has = |k: &str| buildings::count(blds, k) > 0;
    let mut v = vec![head_kind(ctx, i)];
    let seat = ctx.is_seat(i);
    let mut add = |cond: bool, k: &'static str| {
        if cond {
            v.push(k);
        }
    };
    add(seat, "steward");
    add(
        has("barracks") || has("guardhouse") || has("keep"),
        "captain",
    );
    add(has("temple"), "high_priest");
    add(s.has(Function::Abbey), "abbot");
    add(has("shrine") && !has("temple"), "shrine_keeper");
    add(has("dock"), "harbourmaster");
    add(has("mine"), "mine_overseer");
    add(seat && has("library"), "court_mage");
    add(seat && s.tier == Tier::City, "spymaster");
    v
}

/// Roles of node `i`, given its factions and its realm.
#[must_use]
pub fn roles(ctx: &Ctx<'_>, i: usize, factions: &[Faction], realm: &RealmState) -> Vec<NpcRole> {
    let s = ctx.s(i);
    let blds = ctx.buildings_of(i);
    let mut kinds: Vec<String> = required(ctx, i).into_iter().map(str::to_string).collect();
    for f in factions {
        let kind = f
            .leader_role
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_string();
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    let vassal = realm.vassals.iter().find(|v| v.settlement == s.id);
    let mut used: Vec<u64> = Vec::new();
    let mut out = Vec::new();
    for kind in &kinds {
        let Some(def) = ctx.t.roles.roles.get(kind) else {
            continue;
        };
        let id = role_id(s.id, kind);
        let faction = factions.iter().find(|f| f.leader_role == id);
        // Prefer a building no other notable has claimed yet.
        let mut cands: Vec<u64> = faction.and_then(|f| f.seat_building).into_iter().collect();
        for pref in &def.buildings {
            cands.extend(blds.iter().filter(|b| &b.function == pref).map(|b| b.id));
        }
        let building = cands
            .iter()
            .copied()
            .find(|b| !used.contains(b))
            .or_else(|| cands.first().copied())
            .or_else(|| blds.first().map(|b| b.id));
        if let Some(b) = building {
            used.push(b);
        }
        let mut sug = def
            .by_tier
            .get(s.tier.key())
            .or_else(|| def.by_tier.values().next())
            .cloned()
            .unwrap_or_default();
        let gov = ctx.t.politics.governments.get(&realm.government);
        if let (true, true, Some(r)) = (
            kind == "ruler",
            s.tier.is_urban(),
            gov.and_then(|g| g.ruler.clone()),
        ) {
            sug = r;
        }
        let mut slots = Slots::new()
            .with("settlement", s.name.clone())
            .with("realm", realm.name.clone());
        let (mut given, mut family, mut female) = (None, None, None);
        if kind == "ruler" {
            slots.set("title", realm.ruler.title.clone());
            given = Some(realm.ruler.given.clone());
            family = Some(realm.ruler.house.clone());
            female = Some(realm.ruler.female);
        } else if let (Some(v), "lord") = (vassal, kind.as_str()) {
            slots.set("title", v.title.clone());
            given = Some(v.given.clone());
            family = Some(v.house.clone());
            female = Some(v.female);
        } else {
            slots.set("title", "Lord".to_string());
        }
        out.push(NpcRole {
            title: slots.fill(&def.title),
            id,
            settlement: s.id,
            kind: kind.clone(),
            building,
            faction: faction.map(|f| f.id.clone()),
            stat_block: sug.block,
            class: sug.class.map(|(class, level)| ClassLevel { class, level }),
            given_name: given,
            family_name: family,
            female,
            duties: def.duties.clone(),
        });
    }
    out
}
