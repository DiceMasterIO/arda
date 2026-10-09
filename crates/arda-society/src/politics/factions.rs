//! Factions within a settlement: court, guilds, clergy, watch, criminals and
//! the rest, each with goals, a leader slot, a seat building and stances
//! towards the others (symmetric, one record per unordered pair).

use crate::buildings::first_of;
use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::history::{EventKind, History};
use crate::input::{Function, Tier};
use crate::num::{i64_of, u8_of};
use crate::rng::Stream;
use crate::roles::{head_kind, role_id};
use crate::text::{pick_fill, Slots};
use serde::{Deserialize, Serialize};

/// One faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Faction {
    /// Stable id (`f<settlement>.<kind>`).
    pub id: String,
    /// Settlement.
    #[serde(with = "crate::ids::str")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub settlement: u64,
    /// Kind key.
    pub kind: String,
    /// Readable kind.
    pub kind_label: String,
    /// Name.
    pub name: String,
    /// What it wants.
    pub goals: Vec<String>,
    /// Leader NPC slot.
    pub leader_role: String,
    /// Building it meets in.
    pub seat_building: Option<u64>,
    /// Influence, 0–100.
    pub influence: u8,
}

/// How two factions stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FactionStance {
    /// Working together.
    Allied,
    /// On good terms.
    Friendly,
    /// Competing.
    Rival,
    /// Openly hostile.
    Hostile,
}

impl FactionStance {
    /// Key used in `factions.json` reasons.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Allied => "allied",
            Self::Friendly => "friendly",
            Self::Rival => "rival",
            Self::Hostile => "hostile",
        }
    }
}

/// Stance between factions `a < b` (by id) of one settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FactionRelation {
    /// Lower faction id.
    pub a: String,
    /// Higher faction id.
    pub b: String,
    /// Stance.
    pub stance: FactionStance,
    /// Why.
    pub reason: String,
}

/// Context a settlement's factions depend on.
#[derive(Debug, Clone)]
pub struct Local<'h> {
    /// House of the local lord or ruler, if any.
    pub lord_house: Option<String>,
    /// Realm levy on trade, percent.
    pub tax_pct: u32,
    /// History.
    pub history: &'h History,
    /// Lookups over `history`.
    pub index: &'h crate::history::index::HistoryIndex,
}

/// Factions and their relations for node `i`.
#[must_use]
pub fn factions(
    ctx: &Ctx<'_>,
    i: usize,
    econ: &EconomyRun,
    local: &Local<'_>,
    used_names: &mut std::collections::BTreeSet<String>,
) -> (Vec<Faction>, Vec<FactionRelation>) {
    let s = ctx.s(i);
    let t = &ctx.t.factions;
    let blds = ctx.buildings_of(i);
    let mut rng = Stream::new(ctx.seed, "factions", s.id, 0);
    let top_good = econ.per_node[i]
        .ledger
        .exports
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map_or("grain", |(k, _)| k.as_str());
    let craft = blds
        .iter()
        .find_map(|b| b.craft().map(str::to_string))
        .unwrap_or_else(|| "weaver".to_string());
    let recent_woe = local.index.events_of(local.history, s.id).any(|e| {
        matches!(
            e.kind,
            EventKind::Plague | EventKind::Famine | EventKind::Flood
        ) && ctx.present - e.year < 40
    });
    let captured = local.index.first_shift(local.history, s.id).is_some();
    let slots = Slots::new()
        .with("settlement", s.name.clone())
        .with(
            "house",
            local.lord_house.clone().unwrap_or_else(|| s.name.clone()),
        )
        .with("good", ctx.t.good_label(top_good).to_string())
        .with(
            "good_title",
            crate::text::title_case(ctx.t.good_label(top_good)),
        )
        .with("craft", craft)
        .with("patron", rng.pick(&t.patrons).cloned().unwrap_or_default());
    let mut out = Vec::new();
    for k in &t.kinds {
        let by_b = k
            .buildings_any
            .iter()
            .any(|b| blds.iter().any(|x| &x.function == b));
        let by_f = k
            .functions_any
            .iter()
            .any(|f| s.functions.iter().any(|x| x.key() == f));
        let open = k.buildings_any.is_empty() && k.functions_any.is_empty();
        if s.tier < k.min_tier || !(open || by_b || by_f) {
            continue;
        }
        let leader = match (s.tier, k.leader_hamlet.as_deref()) {
            (Tier::Hamlet, Some(small)) => small,
            _ => k.leader.as_str(),
        };
        let leader = if leader == "head" {
            head_kind(ctx, i)
        } else {
            leader
        };
        let mut influence = i64::from(k.power) + rng.range(-5, 5);
        influence += match k.key.as_str() {
            "merchants" => i64_of(
                econ.per_node[i]
                    .key_goods
                    .iter()
                    .map(|g| g.value_sp)
                    .sum::<u64>()
                    / 5000,
            )
            .min(20),
            "temple" if recent_woe => 12,
            "criminals" if s.wealth < 120 || s.has(Function::Port) => 10,
            "watch" if s.has(Function::Fortress) => 10,
            "court" if ctx.is_seat(i) => 15,
            _ => 0,
        };
        let mut goals = Vec::new();
        let rural = !s.tier.is_urban() && !k.goals_rural.is_empty();
        let mut pool = if rural {
            k.goals_rural.clone()
        } else {
            k.goals.clone()
        };
        for _ in 0..2 {
            if pool.is_empty() {
                break;
            }
            let at = crate::num::usize_of(rng.below(crate::num::u64_of_usize(pool.len())));
            goals.push(slots.fill(&pool.remove(at)));
        }
        out.push(Faction {
            id: format!("f{}.{}", s.id, k.key),
            settlement: s.id,
            kind: k.key.clone(),
            kind_label: k.label.clone(),
            name: unique(
                crate::text::pick_name(&k.names, &mut rng, &slots),
                &s.name,
                used_names,
            ),
            goals,
            leader_role: role_id(s.id, leader),
            seat_building: first_of(blds, &k.seat).map(|b| b.id),
            influence: u8_of(influence.clamp(5, 95)),
        });
    }
    let mut rel = Vec::new();
    for x in 0..out.len() {
        for y in x + 1..out.len() {
            let (fa, fb) = (&out[x], &out[y]);
            let mut v = t
                .affinity
                .iter()
                .find(|a| (a.0 == fa.kind && a.1 == fb.kind) || (a.0 == fb.kind && a.1 == fa.kind))
                .map_or(0, |a| a.2);
            let base = v;
            let pair =
                |p: &str, q: &str| (fa.kind == p && fb.kind == q) || (fa.kind == q && fb.kind == p);
            if pair("merchants", "court") && local.tax_pct >= 12 {
                v -= 1;
            }
            if captured && (pair("court", "commons") || pair("court", "merchants")) {
                v -= 1;
            }
            if pair("court", "commons") && (recent_woe || local.tax_pct >= 12 || captured) {
                v -= 1;
            }
            if recent_woe && pair("temple", "commons") {
                v += 1;
            }
            if rng.chance(150) {
                v += if rng.chance(500) { 1 } else { -1 };
            }
            let stance = match v {
                i32::MIN..=-2 => FactionStance::Hostile,
                -1 => FactionStance::Rival,
                0 => continue,
                1 => FactionStance::Friendly,
                _ => FactionStance::Allied,
            };
            let sl = slots
                .clone()
                .with("a", fa.name.clone())
                .with("b", fb.name.clone());
            let mut kinds = [fa.kind.as_str(), fb.kind.as_str()];
            kinds.sort_unstable();
            let pair_key = format!("{}|{}", kinds[0], kinds[1]);
            // Pair reasons name the alphabetically first kind `{a}`.
            let sl = if fa.kind.as_str() == kinds[0] {
                sl
            } else {
                sl.with("a", fb.name.clone()).with("b", fa.name.clone())
            };
            let specific = t
                .pair_reasons
                .get(&pair_key)
                .filter(|_| base != 0 && (base < 0) == (v < 0));
            let generic = t.reasons.get(stance.key()).map_or(&[][..], Vec::as_slice);
            let reason = pick_fill(specific.map_or(generic, Vec::as_slice), &mut rng, &sl);
            let (a, b) = if fa.id < fb.id { (fa, fb) } else { (fb, fa) };
            rel.push(FactionRelation {
                a: a.id.clone(),
                b: b.id.clone(),
                stance,
                reason,
            });
        }
    }
    (out, rel)
}

/// `name`, or `name of <settlement>` when another faction already has it.
fn unique(name: String, settlement: &str, used: &mut std::collections::BTreeSet<String>) -> String {
    let name = if used.contains(&name) {
        format!("{name} of {settlement}")
    } else {
        name
    };
    used.insert(name.clone());
    name
}
