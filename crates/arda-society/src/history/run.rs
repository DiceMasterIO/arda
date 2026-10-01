//! Runs the history stages in causal order and renumbers the timeline.

use super::founding::{self, Founding};
use super::lore::{self, HistoryHook};
use super::prosperity::{self, Prosperity};
use super::{disasters, reigns, remap, ruins, wars, History, Timeline};
use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::politics::government::Regime;
use std::collections::BTreeMap;

/// Everything the history stage produces.
#[derive(Debug, Clone)]
pub struct HistoryRun {
    /// The timeline and its records.
    pub history: History,
    /// Founding per node.
    pub founding: Founding,
    /// Realm formation years, [`Ctx::realms`] order.
    pub formed: Vec<i32>,
    /// Prosperity per node.
    pub prosperity: Vec<Prosperity>,
    /// History hooks per node.
    pub lore: Vec<Vec<HistoryHook>>,
    /// Lookups over `history`.
    pub index: super::index::HistoryIndex,
}

/// Present trade value between each realm-index pair `(low, high)`.
#[must_use]
pub fn realm_trade(ctx: &Ctx<'_>, econ: &EconomyRun) -> BTreeMap<(usize, usize), u64> {
    let mut out = BTreeMap::new();
    for f in &econ.economy.flows {
        let (Some(a), Some(b)) = (ctx.node(f.from), ctx.node(f.to)) else {
            continue;
        };
        let (ra, rb) = (ctx.realm_of(a), ctx.realm_of(b));
        if ra != rb {
            *out.entry((ra.min(rb), ra.max(rb))).or_default() += f.value_sp;
        }
    }
    out
}

/// Simulates the whole history.
#[must_use]
pub fn run(ctx: &Ctx<'_>, econ: &EconomyRun, regimes: &[Regime]) -> HistoryRun {
    let mut tl = Timeline::default();
    let mut founding = founding::found(ctx, &mut tl);
    let formed = reigns::form_realms(ctx, &founding, &mut tl);
    let trade = realm_trade(ctx, econ);
    let mut wr = wars::wars(ctx, &founding, &formed, &trade, &mut tl);
    disasters::disasters(ctx, &founding, &formed, &mut tl);
    let (dynasties, reigns) = reigns::reigns(ctx, regimes, &formed, &wr.wars, &mut tl);
    let mut ruins = ruins::ruins(ctx, &founding, &mut tl);
    let at_war: Vec<u64> = wr
        .wars
        .iter()
        .filter(|w| w.to.is_none())
        .flat_map(|w| w.realms)
        .collect();
    let prosperity = prosperity::prosperity(ctx, &founding, &tl, econ, &at_war);
    let map = tl.renumber();
    for w in &mut wr.wars {
        w.event = remap(&map, w.event);
    }
    for b in &mut wr.shifts {
        b.war_event = remap(&map, b.war_event);
    }
    for r in &mut ruins {
        r.event = remap(&map, r.event);
        r.cause_event = r.cause_event.map(|c| remap(&map, c));
    }
    for e in &mut founding.event {
        *e = remap(&map, *e);
    }
    // The first shift of each settlement, as a scan per settlement found it.
    let mut first_from: std::collections::BTreeMap<u64, u64> = Default::default();
    for b in &wr.shifts {
        first_from.entry(b.settlement).or_insert(b.from_realm);
    }
    let allegiances = (0..ctx.n())
        .map(|i| {
            let s = ctx.s(i);
            let first_realm = first_from.get(&s.id).copied().unwrap_or(s.realm_id);
            super::Allegiance {
                settlement: s.id,
                first_realm,
            }
        })
        .collect();
    let history = History {
        era: ctx.t.history.era.clone(),
        era_abbrev: ctx.t.history.era_abbrev.clone(),
        start_year: 1,
        present_year: ctx.present,
        events: tl.events,
        dynasties,
        reigns,
        wars: wr.wars,
        border_shifts: wr.shifts,
        ruins,
        allegiances,
    };
    let index = super::index::HistoryIndex::new(&history);
    let lore = lore::lore(ctx, &founding, &history, &index, econ);
    HistoryRun {
        history,
        founding,
        formed,
        prosperity,
        lore,
        index,
    }
}
