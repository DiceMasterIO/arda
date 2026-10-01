//! Realm-level tensions: wars, rivalries, succession, vassal unrest,
//! blockades of border roads, pacts, returning plague, lost ruins and tax.

use super::{select, Candidate, Hook, HookWorld};
use crate::ctx::Ctx;
use crate::history::EventKind;
use crate::num::i64_of;
use crate::politics::relations::Stance;
use crate::refs::EntityRef;
use crate::text::Slots;

/// Hooks for realm index `ri`.
#[must_use]
pub fn hooks(ctx: &Ctx<'_>, w: &HookWorld<'_>, ri: usize) -> Vec<Hook> {
    let r = &ctx.realms[ri];
    let st = &w.realms[ri];
    let h = w.history;
    let seat = ctx.s(r.seat);
    let me = EntityRef::realm(r.id);
    let ruler = EntityRef::role(&st.ruler.role);
    let ruler_name = format!("{} {}", st.ruler.title, st.ruler.regnal);
    let base = Slots::new()
        .with("realm", st.name.clone())
        .with("style", st.style.clone())
        .with("seat", seat.name.clone())
        .with("ruler", ruler_name)
        .with("house", st.ruler.house.clone());
    let mut c = Vec::new();
    for rel in w.relations_of(r.id) {
        let other_id = if rel.a == r.id { rel.b } else { rel.a };
        let Some(oi) = ctx.realm_ix(other_id) else {
            continue;
        };
        let other = &w.realms[oi];
        let sl = base.clone().with("other", other.name.clone()).with(
            "other_ruler",
            format!("{} {}", other.ruler.title, other.ruler.regnal),
        );
        let mut refs = vec![
            me.clone(),
            EntityRef::realm(other_id),
            ruler.clone(),
            EntityRef::role(&other.ruler.role),
        ];
        let busiest = rel
            .border_roads
            .iter()
            .copied()
            .max_by_key(|&id| (w.traffic(id), std::cmp::Reverse(id)));
        if let Some(road) = busiest {
            refs.push(EntityRef::road(road));
        }
        match rel.stance {
            Stance::War => {
                if let Some(&war) = rel.wars.last() {
                    refs.push(EntityRef::Event { id: war });
                }
                c.push(Candidate::new(95, "war_front", sl, refs));
            }
            Stance::Rivalry => {
                // One side of a pair is the blockader, the same for both.
                let closer =
                    if crate::rng::hash(ctx.seed, "blockade", rel.a, rel.b).is_multiple_of(2) {
                        rel.a
                    } else {
                        rel.b
                    };
                let kind = match (busiest.is_some_and(|id| w.traffic(id) > 0), closer == r.id) {
                    (true, true) => "trade_war",
                    (true, false) => "realm_blockade",
                    _ => "rival_realm",
                };
                c.push(Candidate::new(70, kind, sl, refs));
            }
            Stance::Alliance | Stance::TradePact => {
                let sl = sl
                    .with("pact", rel.stance.label())
                    .with("pact_title", crate::text::title_case(rel.stance.label()));
                c.push(Candidate::new(44, "envoy", sl, refs));
            }
            Stance::Neutral => c.push(Candidate::new(30, "courtship", sl, refs)),
        }
    }
    let reign_years = ctx.present - st.ruler.since;
    let claimant = st
        .vassals
        .iter()
        .find(|v| v.claimant && v.house != st.ruler.house);
    if reign_years >= 20 || claimant.is_some() {
        let mut refs = vec![
            me.clone(),
            ruler.clone(),
            EntityRef::Dynasty {
                id: st.ruler.dynasty.clone(),
            },
        ];
        let mut sl = base.clone().with("years", reign_years.to_string());
        if let Some(v) = claimant {
            refs.push(EntityRef::role(&v.role));
            refs.push(EntityRef::settlement(v.settlement));
            sl.set(
                "claimant",
                format!("{} {} of House {}", v.title, v.given, v.house),
            );
        } else {
            sl.set("claimant", "a cousin nobody has seen in years".to_string());
        }
        let kind = if reign_years >= 20 {
            "succession"
        } else {
            "claimant"
        };
        c.push(Candidate::new(
            58 + i64::from(claimant.is_some()) * 10,
            kind,
            sl,
            refs,
        ));
    }
    if let Some(v) = st
        .vassals
        .iter()
        .min_by_key(|v| (v.loyalty, v.settlement))
        .filter(|v| v.loyalty < 50)
    {
        let name = ctx
            .node(v.settlement)
            .map_or(String::new(), |j| ctx.s(j).name.clone());
        let sl = base
            .clone()
            .with("vassal", format!("{} {} of {}", v.title, v.given, name))
            .with(
                "grievance",
                v.grievances
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "old slights".to_string()),
            );
        let refs = vec![
            me.clone(),
            ruler.clone(),
            EntityRef::role(&v.role),
            EntityRef::settlement(v.settlement),
        ];
        c.push(Candidate::new(
            60 + i64::from(50 - v.loyalty),
            "vassal_revolt",
            sl,
            refs,
        ));
    }
    let plague = w
        .index
        .realm_events(h, r.id)
        .filter(|e| e.kind == EventKind::Plague)
        .max_by_key(|e| (e.year, e.id));
    if let Some(e) = plague {
        let sl = base
            .clone()
            .with("plague", crate::text::event_phrase(e))
            .with("years", (ctx.present - e.year).to_string());
        let mut refs = vec![me.clone(), EntityRef::Event { id: e.id }];
        refs.extend(
            e.settlements
                .iter()
                .take(2)
                .map(|&id| EntityRef::settlement(id)),
        );
        c.push(Candidate::new(
            30 + (30 - i64::from(ctx.present - e.year) / 3).max(0),
            "plague_return",
            sl,
            refs,
        ));
    }
    if let Some(ruin) = h
        .ruins
        .iter()
        .filter(|x| x.realm_id == r.id)
        .min_by_key(|x| (x.founded, x.id))
    {
        let near = ctx
            .node(ruin.near)
            .map_or(String::new(), |j| ctx.s(j).name.clone());
        let sl = base
            .clone()
            .with("ruin", ruin.name.clone())
            .with("kind", ruin.kind.clone())
            .with("near", near)
            .with("year", ruin.founded.to_string());
        let refs = vec![
            me.clone(),
            EntityRef::Ruin { id: ruin.id },
            EntityRef::settlement(ruin.near),
        ];
        c.push(Candidate::new(42, "lost_ruin", sl, refs));
    }
    if st.tax_pct >= 14 {
        let sl = base
            .clone()
            .with("tax", st.tax_pct.to_string())
            .with("levy", st.levy_sp.to_string());
        c.push(Candidate::new(
            34 + i64_of(u64::from(st.tax_pct)),
            "tax_revolt",
            sl,
            vec![me.clone(), ruler.clone()],
        ));
    }
    c.push(Candidate::new(
        12,
        "royal_progress",
        base.clone(),
        vec![me.clone(), ruler.clone(), EntityRef::settlement(seat.id)],
    ));
    c.push(Candidate::new(
        11,
        "census",
        base.clone(),
        vec![me.clone(), EntityRef::settlement(seat.id)],
    ));
    c.push(Candidate::new(
        10,
        "tourney",
        base,
        vec![me, ruler, EntityRef::settlement(seat.id)],
    ));
    select(ctx, &format!("hr{}", r.id), r.id | 1 << 40, c)
}
