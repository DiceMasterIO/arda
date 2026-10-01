//! Settlement-level tensions: guilds against the lord, faction feuds, border
//! disputes, blockades, bandit roads, shortages, ruins, disasters, mines,
//! disloyal vassals and smuggling.

use super::{cite_role, select, Candidate, Hook, HookWorld};
use crate::ctx::Ctx;
use crate::history::EventKind;
use crate::input::Function;
use crate::refs::EntityRef;
use crate::text::Slots;

mod fortunes;
mod strife;
use fortunes::{goods, past};
use strife::{borders, feuds, roads};

/// Hooks for node `i`.
#[must_use]
pub fn hooks(ctx: &Ctx<'_>, w: &HookWorld<'_>, i: usize) -> Vec<Hook> {
    let s = ctx.s(i);
    let base = Slots::new()
        .with("settlement", s.name.clone())
        .with("realm", ctx.realms[ctx.realm_of(i)].name.clone());
    let me = EntityRef::settlement(s.id);
    let mut c = Vec::new();
    feuds(w, i, &base, &me, &mut c);
    borders(ctx, w, i, &base, &me, &mut c);
    roads(ctx, w, i, &base, &me, &mut c);
    goods(ctx, w, i, &base, &me, &mut c);
    past(ctx, w, i, &base, &me, &mut c);
    country(ctx, w, i, &base, &me, &mut c);
    fallbacks(ctx, w, i, &base, &me, &mut c);
    select(ctx, &format!("h{}", s.id), s.id, c)
}

fn fallbacks(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let near = ctx.graph.adj[i].first().map(|e| e.to).or_else(|| {
        (0..ctx.n())
            .filter(|&j| j != i)
            .min_by_key(|&j| (ctx.dist(i, j), j))
    });
    let mut sl = base.clone();
    let mut refs = vec![me.clone()];
    cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
    c.push(Candidate::new(
        12,
        "missing_person",
        sl.clone(),
        refs.clone(),
    ));
    let inn = ctx
        .buildings_of(i)
        .iter()
        .find(|b| b.function == "inn" || b.function == "tavern");
    let mut r2 = refs.clone();
    let mut s2 = sl.clone().with(
        "place",
        inn.map_or("the well", |b| {
            if b.function == "inn" {
                "the inn"
            } else {
                "the tavern"
            }
        }),
    );
    if let Some(b) = inn {
        r2.push(EntityRef::Building {
            settlement: s.id,
            id: b.id,
        });
    }
    s2.set("tier", s.tier.key());
    c.push(Candidate::new(11, "stranger", s2, r2));
    c.push(Candidate::new(9, "omen", sl.clone(), refs.clone()));
    c.push(Candidate::new(11, "fair", sl.clone(), refs.clone()));
    c.push(Candidate::new(10, "plough_find", sl.clone(), refs.clone()));
    c.push(Candidate::new(10, "wedding", sl.clone(), refs.clone()));
    if let Some(j) = near {
        let o = ctx.s(j);
        let mut r3 = refs;
        r3.push(EntityRef::settlement(o.id));
        c.push(Candidate::new(
            10,
            "boundary_stone",
            sl.with("other", o.name.clone()),
            r3,
        ));
    }
}

/// Rural tensions from the land and the realm's wars.
fn country(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let upland = ["forest", "hill", "mountain"].iter().find(|t| s.tagged(t));
    if let (true, Some(t)) = (s.has(Function::Pastoral), upland) {
        let terrain = match *t {
            "forest" => "forest edge",
            "hill" => "hills",
            _ => "mountains",
        };
        let mut sl = base.clone().with("terrain", terrain);
        let mut refs = vec![me.clone()];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        c.push(Candidate::new(
            if s.tier.is_urban() { 26 } else { 40 },
            "wolf_winter",
            sl,
            refs,
        ));
    }
    let flood = w
        .index
        .events_of(w.history, s.id)
        .filter(|e| e.kind == EventKind::Flood)
        .max_by_key(|e| (e.year, e.id));
    if let Some(e) = flood {
        let mut sl = base.clone().with("flood", crate::text::event_phrase(e));
        let mut refs = vec![me.clone(), EntityRef::Event { id: e.id }];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        let age = i64::from(ctx.present - e.year);
        c.push(Candidate::new(
            34 + (60 - age).max(0) / 4 + i64::from(e.severity) * 2,
            "rising_water",
            sl,
            refs,
        ));
    }
    let r = &w.realms[ctx.realm_of(i)];
    let war = w
        .index
        .wars_of(w.history, r.id)
        .filter(|x| x.to.is_none_or(|t| ctx.present - t <= 12))
        .max_by_key(|x| (x.from, x.event));
    if let Some(x) = war {
        let men = (s.population / 40).clamp(2, 200);
        let mut sl = base.clone().with("men", men.to_string());
        let mut refs = vec![
            me.clone(),
            EntityRef::realm(r.id),
            EntityRef::Event { id: x.event },
        ];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        c.push(Candidate::new(
            if x.to.is_none() { 58 } else { 36 },
            "press_gang",
            sl,
            refs,
        ));
    }
}
