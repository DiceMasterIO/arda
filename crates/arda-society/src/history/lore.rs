//! History hooks: 2–4 lines of original text per settlement, each drawn
//! from something that actually happened to it in the timeline.

use super::founding::{site_phrase, Founding};
use super::{EventKind, History};
use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::text::{pick_fill, Slots};
use serde::{Deserialize, Serialize};

/// One line of settlement history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryHook {
    /// The line.
    pub text: String,
    /// Year it refers to, if any.
    pub year: Option<i32>,
    /// Entities it mentions.
    pub refs: Vec<EntityRef>,
}

/// History hooks per node.
#[must_use]
pub fn lore(ctx: &Ctx<'_>, f: &Founding, h: &History, econ: &EconomyRun) -> Vec<Vec<HistoryHook>> {
    (0..ctx.n()).map(|i| one(ctx, f, h, econ, i)).collect()
}

fn one(ctx: &Ctx<'_>, f: &Founding, h: &History, econ: &EconomyRun, i: usize) -> Vec<HistoryHook> {
    let s = ctx.s(i);
    let t = &ctx.t.history;
    let mut rng = Stream::new(ctx.seed, "lore", s.id, 0);
    let era = t.era_abbrev.as_str();
    let base = Slots::new()
        .with("settlement", s.name.clone())
        .with("site", site_phrase(ctx, i))
        .with("era", era);
    let mut out: Vec<HistoryHook> = Vec::new();
    let mut add = |out: &mut Vec<HistoryHook>,
                   key: &str,
                   slots: &Slots,
                   year: Option<i32>,
                   refs: Vec<EntityRef>| {
        let text = pick_fill(
            t.lore.get(key).map_or(&[][..], Vec::as_slice),
            &mut rng,
            slots,
        );
        if !text.is_empty() {
            out.push(HistoryHook { text, year, refs });
        }
    };
    if !s.history.is_empty() {
        let sl = Slots::new().with("line", s.history.clone());
        add(
            &mut out,
            "site_line",
            &sl,
            None,
            vec![EntityRef::settlement(s.id)],
        );
    }
    let year = f.year[i];
    let sl = base.clone().with("year", year.to_string());
    match f.parent[i] {
        Some(p) => add(
            &mut out,
            "from",
            &sl.clone().with("parent", ctx.s(p).name.clone()),
            Some(year),
            vec![EntityRef::settlement(ctx.s(p).id)],
        ),
        None => add(
            &mut out,
            "root",
            &sl,
            Some(year),
            vec![EntityRef::settlement(s.id)],
        ),
    }
    if let Some(b) = h.border_shifts.iter().find(|b| b.settlement == s.id) {
        let former = ctx
            .realm_ix(b.from_realm)
            .map_or(String::new(), |r| ctx.realms[r].name.clone());
        let war = h
            .wars
            .iter()
            .find(|w| w.event == b.war_event)
            .map_or(String::new(), |w| w.name.clone());
        let sl = base
            .clone()
            .with("former", former)
            .with("war", war)
            .with("year", b.year.to_string());
        let key = if ctx.present - b.year <= crate::politics::realm::RECENT_CONQUEST {
            "captured_recent"
        } else {
            "captured"
        };
        add(
            &mut out,
            key,
            &sl,
            Some(b.year),
            vec![
                EntityRef::realm(b.from_realm),
                EntityRef::Event { id: b.war_event },
            ],
        );
    }
    let worst = h
        .events
        .iter()
        .filter(|e| e.settlements.contains(&s.id) && e.severity >= 2)
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Plague
                    | EventKind::Flood
                    | EventKind::Fire
                    | EventKind::Famine
                    | EventKind::MineCollapse
            )
        })
        .max_by_key(|e| (e.severity, e.year));
    if let Some(e) = worst {
        let key = match e.kind {
            EventKind::Plague if s.tier.is_urban() => "plague",
            EventKind::Plague => "plague_small",
            EventKind::Flood => "flood",
            EventKind::Fire => "fire",
            EventKind::Famine => "famine",
            _ => "mine_collapse",
        };
        let sl = base
            .clone()
            .with("year", e.year.to_string())
            .with("event", crate::text::event_phrase(e));
        add(
            &mut out,
            key,
            &sl,
            Some(e.year),
            vec![EntityRef::Event { id: e.id }],
        );
    }
    if let Some(r) = ctx.realms.iter().find(|r| r.seat == i) {
        if let Some(d) = h
            .dynasties
            .iter()
            .find(|d| d.realm_id == r.id && d.to.is_none())
        {
            let sl = base
                .clone()
                .with("house", d.name.clone())
                .with("year", d.from.to_string())
                .with("realm", r.name.clone());
            add(
                &mut out,
                "dynasty",
                &sl,
                Some(d.from),
                vec![EntityRef::Dynasty { id: d.id.clone() }],
            );
        }
    }
    if let Some(r) = h.ruins.iter().find(|r| r.near == s.id) {
        let sl = base
            .clone()
            .with("ruin", r.name.clone())
            .with("kind", r.kind.clone())
            .with("year", r.abandoned.to_string());
        add(
            &mut out,
            "ruin",
            &sl,
            Some(r.abandoned),
            vec![EntityRef::Ruin { id: r.id }],
        );
    }
    let top = econ
        .economy
        .flows
        .iter()
        .filter(|fl| fl.from == s.id)
        .max_by(|a, b| a.value_sp.cmp(&b.value_sp).then(b.to.cmp(&a.to)));
    if let Some(fl) = top {
        let dest = ctx
            .node(fl.to)
            .map_or(String::new(), |j| ctx.s(j).name.clone());
        let sl = base
            .clone()
            .with("good", ctx.t.good_label(&fl.good).to_string())
            .with("dest", dest);
        add(
            &mut out,
            "export",
            &sl,
            None,
            vec![EntityRef::good(&fl.good), EntityRef::settlement(fl.to)],
        );
    }
    if out.len() < 2 {
        add(
            &mut out,
            "quiet",
            &base,
            None,
            vec![EntityRef::settlement(s.id)],
        );
    }
    out.truncate(4);
    out
}
