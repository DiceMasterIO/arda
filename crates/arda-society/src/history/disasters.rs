//! Disasters: floods on low river sites, town fires and mine collapses; the
//! plagues and famines that chain off other events live in [`super::chains`].
//! Each one only touches settlements that already exist in its year.

use super::founding::Founding;
use super::{event, stories, Event, EventKind, Timeline};
use crate::ctx::Ctx;
use crate::input::Function;
use crate::num::i32_of;
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

pub(super) fn list<'a>(t: &'a HistoryTable, key: &str) -> &'a [String] {
    t.events.get(key).map_or(&[][..], Vec::as_slice)
}

pub(super) fn titles<'a>(t: &'a HistoryTable, key: &str) -> &'a [String] {
    t.titles.get(key).map_or(&[][..], Vec::as_slice)
}

pub(super) fn names(ctx: &Ctx<'_>, nodes: &[usize]) -> String {
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

pub(super) fn make(
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
    let home = nodes.first().map_or(0, |&i| ctx.s(i).realm_id);
    if !slots.has("realm") {
        let r = ctx
            .realm_ix(home)
            .map_or(String::new(), |r| ctx.realms[r].name.clone());
        slots.set("realm", r);
    }
    if !slots.has("river") {
        slots.set("river", stories::river(ctx, home));
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
    local(ctx, f, tl);
    super::chains::famines(ctx, f, formed, tl);
    super::chains::plagues(ctx, f, tl);
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
            let kind = if s.coastal && rng.chance(500) {
                "storm_surge"
            } else if rng.chance(500) {
                "snowmelt"
            } else {
                "river_spate"
            };
            e.cause_kind = Some(kind.to_string());
            let sl = slots_of(ctx, &e, &slots);
            if e.severity >= 3 {
                stories::name_event(ctx, tl, &mut e, "flood", &sl, &mut rng);
            }
            stories::tell(ctx, &mut e, &format!("flood_{kind}"), None, &sl, &mut rng);
            tl.push(e);
        }
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
                    let kind = if rng.chance(500) {
                        "lamp_fire"
                    } else {
                        "bad_timber"
                    };
                    e.cause_kind = Some(kind.to_string());
                    let sl = slots_of(ctx, &e, &slots);
                    if e.severity >= 2 {
                        stories::name_event(ctx, tl, &mut e, "fire", &sl, &mut rng);
                    }
                    stories::tell(ctx, &mut e, &format!("fire_{kind}"), None, &sl, &mut rng);
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
                let kind = if rng.chance(500) {
                    "bad_timber"
                } else {
                    "flooded_gallery"
                };
                e.cause_kind = Some(kind.to_string());
                let sl = slots_of(ctx, &e, &slots);
                stories::tell(
                    ctx,
                    &mut e,
                    &format!("mine_collapse_{kind}"),
                    None,
                    &sl,
                    &mut rng,
                );
                tl.push(e);
            }
        }
    }
}

/// The slots of a disaster event as `make` filled them, for its backstory
/// and name: settlement, places, year, realm and river.
pub(super) fn slots_of(ctx: &Ctx<'_>, e: &Event, base: &Slots) -> Slots {
    let home = e
        .settlements
        .first()
        .and_then(|&id| ctx.node(id))
        .map_or(0, |i| ctx.s(i).realm_id);
    let realm = ctx
        .realm_ix(home)
        .map_or(String::new(), |r| ctx.realms[r].name.clone());
    let places: Vec<usize> = e
        .settlements
        .iter()
        .filter_map(|&id| ctx.node(id))
        .collect();
    base.clone()
        .with("year", e.year.to_string())
        .with("realm", realm)
        .with("river", stories::river(ctx, home))
        .with("places", names(ctx, &places))
}
