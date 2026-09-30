//! Disasters: floods on low river sites, plague spreading along roads from
//! ports and cities, town fires, famines and mine collapses. Each one only
//! touches settlements that already exist in its year.

use super::founding::Founding;
use super::{event, Event, EventKind, Timeline};
use crate::ctx::Ctx;
use crate::input::Function;
use crate::num::{i32_of, i64_of, u64_of};
use crate::rng::Stream;
use crate::tables::HistoryTable;
use crate::text::{join_and, pick_fill, Slots};

/// Site tags that put a riverside settlement on the floodplain.
const LOW_TAGS: [&str; 7] = [
    "confluence",
    "ford",
    "bridge_site",
    "marsh",
    "navigable",
    "estuary",
    "river",
];

fn list<'a>(t: &'a HistoryTable, key: &str) -> &'a [String] {
    t.events.get(key).map_or(&[][..], Vec::as_slice)
}

fn titles<'a>(t: &'a HistoryTable, key: &str) -> &'a [String] {
    t.titles.get(key).map_or(&[][..], Vec::as_slice)
}

fn names(ctx: &Ctx<'_>, nodes: &[usize]) -> String {
    let mut v: Vec<String> = nodes
        .iter()
        .take(3)
        .map(|&i| ctx.s(i).name.clone())
        .collect();
    if nodes.len() > 3 {
        v.push(format!("{} other places", nodes.len() - 3));
    }
    join_and(&v)
}

fn make(
    ctx: &Ctx<'_>,
    kind: EventKind,
    key: &str,
    year: i32,
    nodes: &[usize],
    slots: &Slots,
    rng: &mut Stream,
) -> Event {
    let t = &ctx.t.history;
    let mut slots = slots.clone().with("year", year.to_string());
    if !slots.has("places") {
        slots.set("places", names(ctx, nodes));
    }
    let mut e = event(
        kind,
        year,
        pick_fill(titles(t, key), rng, &slots),
        pick_fill(list(t, key), rng, &slots),
    );
    e.settlements = nodes.iter().map(|&i| ctx.s(i).id).collect();
    let mut realms: Vec<u64> = nodes.iter().map(|&i| ctx.s(i).realm_id).collect();
    realms.sort_unstable();
    realms.dedup();
    e.realms = realms;
    e
}

/// Pushes every disaster event.
pub fn disasters(ctx: &Ctx<'_>, f: &Founding, formed: &[i32], tl: &mut Timeline) {
    floods(ctx, f, tl);
    plagues(ctx, f, tl);
    local(ctx, f, tl);
    famines(ctx, f, formed, tl);
}

fn floods(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) {
    for i in 0..ctx.n() {
        let s = ctx.s(i);
        let low = s.riverine && LOW_TAGS.iter().any(|t| s.tagged(t));
        if !(low || s.biome == "wetland") {
            continue;
        }
        let mut rng = Stream::new(ctx.seed, "flood", s.id, 0);
        let odds = 220
            + if s.tagged("confluence") || s.tagged("marsh") {
                150
            } else {
                0
            };
        let mut c = f.year[i];
        while c < ctx.present {
            let year = c + i32_of(rng.range(0, 99));
            c += 100;
            if year >= ctx.present || !rng.chance(odds) {
                continue;
            }
            let mut hit = vec![i];
            for e in &ctx.graph.adj[i] {
                let o = ctx.s(e.to);
                if o.riverine && e.len_m < 25_000 && f.year[e.to] <= year && rng.chance(450) {
                    hit.push(e.to);
                }
            }
            hit.sort_unstable();
            hit.dedup();
            let slots = Slots::new().with("settlement", s.name.clone());
            let mut e = make(ctx, EventKind::Flood, "flood", year, &hit, &slots, &mut rng);
            e.severity = u8::try_from(1 + rng.below(3)).unwrap_or(1);
            tl.push(e);
        }
    }
}

fn plagues(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) {
    let t = &ctx.t.history;
    let count = 1 + ctx.present / 140;
    for k in 0..count {
        let mut rng = Stream::new(ctx.seed, "plague", u64_of(i64::from(k)), 0);
        let year = i32_of(rng.range(50, i64::from(ctx.present - 15)));
        let hubs: Vec<usize> = (0..ctx.n())
            .filter(|&i| {
                let s = ctx.s(i);
                f.year[i] <= year && (s.has(Function::Port) || s.tier.is_urban())
            })
            .collect();
        let total: u64 = hubs.iter().map(|&i| u64::from(ctx.s(i).population)).sum();
        let mut pick = rng.below(total.max(1));
        let Some(origin) = hubs.iter().copied().find(|&i| {
            let p = u64::from(ctx.s(i).population);
            if pick < p {
                true
            } else {
                pick -= p;
                false
            }
        }) else {
            continue;
        };
        let reach = ctx.graph.reach(origin, 200_000);
        let mut hit: Vec<(u64, usize)> = reach
            .dist
            .iter()
            .filter(|&(&j, &d)| {
                let km = i64_of(d / 1000);
                f.year[j] <= year && (j == origin || rng.chance(u64_of(760 - km * 5)))
            })
            .map(|(&j, &d)| (d, j))
            .collect();
        hit.sort_unstable();
        let nodes: Vec<usize> = hit.iter().map(|h| h.1).collect();
        let others: Vec<usize> = nodes.iter().copied().filter(|&j| j != origin).collect();
        let span = i32_of(i64_of(hit.last().map_or(0, |h| h.0) / 35_000));
        let name = rng.pick(&t.plague_names).cloned().unwrap_or_default();
        let slots = Slots::new()
            .with("plague", name)
            .with("settlement", ctx.s(origin).name.clone());
        let slots = slots.with("places", names(ctx, &others));
        let mut e = make(
            ctx,
            EventKind::Plague,
            "plague",
            year,
            &nodes,
            &slots,
            &mut rng,
        );
        e.end_year = Some(year + span.max(1));
        e.severity = if nodes.len() > 6 { 3 } else { 2 };
        tl.push(e);
    }
}

fn local(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) {
    for i in 0..ctx.n() {
        let s = ctx.s(i);
        let slots = Slots::new().with("settlement", s.name.clone());
        if s.tier.is_urban() {
            let mut rng = Stream::new(ctx.seed, "fire", s.id, 0);
            let mut c = f.year[i] + 10;
            while c < ctx.present {
                let year = c + i32_of(rng.range(0, 99));
                c += 100;
                if year < ctx.present && rng.chance(170) {
                    let mut e = make(ctx, EventKind::Fire, "fire", year, &[i], &slots, &mut rng);
                    e.severity = u8::try_from(1 + rng.below(2)).unwrap_or(1);
                    tl.push(e);
                }
            }
        }
        if s.has(Function::Mining) {
            let mut rng = Stream::new(ctx.seed, "mine", s.id, 0);
            let year = i32_of(rng.range(i64::from(f.year[i] + 15), i64::from(ctx.present - 3)));
            if year > f.year[i] && rng.chance(380) {
                let mut e = make(
                    ctx,
                    EventKind::MineCollapse,
                    "mine_collapse",
                    year,
                    &[i],
                    &slots,
                    &mut rng,
                );
                e.severity = 2;
                tl.push(e);
            }
        }
    }
}

fn famines(ctx: &Ctx<'_>, f: &Founding, formed: &[i32], tl: &mut Timeline) {
    for (ri, r) in ctx.realms.iter().enumerate() {
        let mut rng = Stream::new(ctx.seed, "famine", r.id, 0);
        let mut c = formed[ri] + 10;
        while c < ctx.present {
            let year = c + i32_of(rng.range(0, 119));
            c += 120;
            if year >= ctx.present || !rng.chance(380) {
                continue;
            }
            let nodes: Vec<usize> = r
                .members
                .iter()
                .copied()
                .filter(|&i| {
                    f.year[i] <= year
                        && (ctx.s(i).has(Function::Farming) || ctx.s(i).has(Function::Pastoral))
                })
                .collect();
            if nodes.is_empty() {
                continue;
            }
            let slots = Slots::new()
                .with("realm", r.name.clone())
                .with("settlement", ctx.s(r.seat).name.clone());
            let mut e = make(
                ctx,
                EventKind::Famine,
                "famine",
                year,
                &nodes,
                &slots,
                &mut rng,
            );
            e.severity = if rng.chance(300) { 2 } else { 1 };
            e.realms = vec![r.id];
            tl.push(e);
        }
    }
}
