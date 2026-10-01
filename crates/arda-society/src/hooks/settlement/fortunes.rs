//! Shortages of goods, mines and smuggling, and the weight of the past:
//! ruins, disasters and old wars.

use crate::ctx::Ctx;
use crate::history::EventKind;
use crate::hooks::{cite_role, Candidate, HookWorld};
use crate::input::Function;
use crate::num::i64_of;
use crate::refs::EntityRef;
use crate::text::Slots;

fn shortfall_phrase(good: &str, pct: u64) -> String {
    if pct >= 90 {
        format!("has almost none of the {good} it needs")
    } else if pct >= 50 {
        format!("is short of {good}, getting less than half of what it needs")
    } else {
        format!("is short of {good}, about {pct} loads in every hundred it needs")
    }
}

pub(super) fn goods(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let l = &w.econ.per_node[i].ledger;
    let val = |k: &str, n: u64| n * u64::from(ctx.t.good(k).map_or(0, |g| g.value_sp));
    let merchants = w.factions[i].iter().find(|f| f.kind == "merchants");
    // Towns feel any shortage; villages and hamlets only miss staples.
    // The worst shortage is the largest share of demand unmet.
    let urban = s.tier.is_urban();
    let short = l
        .shortfall
        .iter()
        .filter(|(k, _)| {
            // Pure processing inputs (wool, ore) are the workshops' problem.
            ctx.t
                .good(k)
                .is_some_and(|g| g.demand_per_100 > 0 && (urban || g.staple))
        })
        .map(|(k, &n)| {
            (
                n * 100 / l.demand.get(k).copied().unwrap_or(1).max(1),
                val(k, n),
                k,
                n,
            )
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(b.2.cmp(a.2)))
        .map(|x| (x.2, x.3));
    if let Some((k, n)) = short {
        let want = l.demand.get(k).copied().unwrap_or(1).max(1);
        if n * 100 / want >= 15 && n >= 3 && val(k, n) >= 100 {
            let mut sl = base
                .clone()
                .with("good", ctx.t.good_label(k).to_string())
                .with("pct", (n * 100 / want).to_string())
                .with(
                    "shortfall",
                    shortfall_phrase(ctx.t.good_label(k), n * 100 / want),
                );
            let mut refs = vec![me.clone(), EntityRef::good(k)];
            cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
            if let Some(m) = merchants {
                refs.push(EntityRef::faction(&m.id));
            }
            c.push(Candidate::new(
                (if urban { 32 } else { 20 }) + i64_of(n * 24 / want).min(20),
                "shortage",
                sl,
                refs,
            ));
        }
    }
    if let Some((k, &n)) = l
        .stored
        .iter()
        .max_by_key(|(k, &n)| (val(k, n), std::cmp::Reverse(k.as_str())))
    {
        if val(k, n) >= 600 {
            let mut sl = base.clone().with("good", ctx.t.good_label(k).to_string());
            let mut refs = vec![me.clone(), EntityRef::good(k)];
            cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
            c.push(Candidate::new(
                24 + i64_of(val(k, n) / 600).min(12),
                "glut",
                sl,
                refs,
            ));
        }
    }
    let crooks = w.factions[i].iter().find(|f| f.kind == "criminals");
    if let (true, Some(cr)) = (s.has(Function::Port), crooks) {
        let mut sl = base.clone().with("gang", cr.name.clone());
        let mut refs = vec![
            me.clone(),
            EntityRef::faction(&cr.id),
            EntityRef::role(&cr.leader_role),
        ];
        cite_role(
            &mut refs,
            &mut sl,
            "harbourmaster",
            w.role(i, "harbourmaster").or_else(|| w.head(ctx, i)),
        );
        c.push(Candidate::new(58, "smugglers", sl, refs));
    }
}

pub(super) fn past(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let h = w.history;
    if let Some(r) = w.index.ruin_near(h, s.id) {
        let mut sl = base
            .clone()
            .with("ruin", r.name.clone())
            .with("kind", r.kind.clone())
            .with("year", r.abandoned.to_string());
        let mut refs = vec![me.clone(), EntityRef::Ruin { id: r.id }];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        c.push(Candidate::new(46, "ruin", sl, refs));
    }
    let recent = w
        .index
        .events_of(h, s.id)
        .filter(|e| e.severity >= 2 && ctx.present - e.year <= 45)
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Plague | EventKind::Flood | EventKind::Fire | EventKind::Famine
            )
        })
        .max_by_key(|e| (e.year, e.id));
    if let Some(e) = recent {
        let mut sl = base
            .clone()
            .with("event", crate::text::event_phrase(e))
            .with("years", (ctx.present - e.year).to_string());
        let mut refs = vec![me.clone(), EntityRef::Event { id: e.id }];
        let priest = w
            .role(i, "high_priest")
            .or_else(|| w.role(i, "shrine_keeper"))
            .or_else(|| w.role(i, "abbot"));
        cite_role(
            &mut refs,
            &mut sl,
            "priest",
            priest.or_else(|| w.head(ctx, i)),
        );
        let age = i64::from(ctx.present - e.year);
        c.push(Candidate::new(
            30 + (45 - age) / 2 + i64::from(e.severity) * 4,
            "aftermath",
            sl,
            refs,
        ));
    }
    if s.has(Function::Mining) {
        let collapse = w
            .index
            .events_of(h, s.id)
            .find(|e| e.kind == EventKind::MineCollapse);
        let mut sl = base.clone();
        let mut refs = vec![me.clone()];
        if let Some(e) = collapse {
            refs.push(EntityRef::Event { id: e.id });
            sl.set("year", e.year.to_string());
        } else {
            sl.set("year", "living memory".to_string());
        }
        cite_role(
            &mut refs,
            &mut sl,
            "overseer",
            w.role(i, "mine_overseer").or_else(|| w.head(ctx, i)),
        );
        c.push(Candidate::new(
            if collapse.is_some() { 52 } else { 40 },
            "deep_trouble",
            sl,
            refs,
        ));
    }
    let r = &w.realms[ctx.realm_of(i)];
    if let Some(v) = r
        .vassals
        .iter()
        .find(|v| v.settlement == s.id && v.loyalty < 45)
    {
        let mut sl = base
            .clone()
            .with(
                "grievance",
                v.grievances.first().cloned().unwrap_or_default(),
            )
            .with("house", v.house.clone());
        let mut refs = vec![
            me.clone(),
            EntityRef::role(&v.role),
            EntityRef::role(&r.ruler.role),
            EntityRef::realm(r.id),
        ];
        cite_role(&mut refs, &mut sl, "lord", w.role(i, "lord"));
        c.push(Candidate::new(
            62 + i64::from(45 - v.loyalty),
            "disloyal_vassal",
            sl,
            refs,
        ));
    }
}
