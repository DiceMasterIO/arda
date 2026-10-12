//! Famines and plagues, and the causal chains between disasters: a flood or
//! war may bring a famine within three years, a famine or war camp a plague
//! within six, and plague or famine then empties ruins (`ruins.rs`). A
//! chain is made by moving an already-planned event to just after its cause,
//! never by adding events, so the per-tier event counts stay as they were.

use super::disasters::{make, slots_of};
use super::founding::Founding;
use super::{stories, Event, EventKind, Timeline};
use crate::ctx::Ctx;
use crate::input::Function;
use crate::num::{i32_of, i64_of, u64_of};
use crate::rng::Stream;
use crate::text::{lower_the, Slots};

fn shares(a: &[u64], b: &[u64]) -> bool {
    a.iter().any(|x| b.contains(x))
}

/// The last event of `kind` matching `ok`, latest year first.
fn latest(tl: &Timeline, kind: EventKind, ok: impl Fn(&Event) -> bool) -> Option<&Event> {
    tl.events
        .iter()
        .filter(|e| e.kind == kind && ok(e))
        .max_by_key(|e| (e.year, e.id))
}

/// What brought a famine to `realm` in `year`: a flood in the realm within
/// three years before, else a war it fought (within three years of the war's
/// end).
fn famine_cause(
    tl: &Timeline,
    ctx: &Ctx<'_>,
    realm: u64,
    year: i32,
) -> Option<(u32, &'static str)> {
    let flood = latest(tl, EventKind::Flood, |e| {
        e.realms.contains(&realm) && e.year <= year && year <= e.year + 3
    });
    if let Some(e) = flood {
        return Some((e.id, "river_spate"));
    }
    latest(tl, EventKind::War, |e| {
        e.realms.contains(&realm) && e.year <= year && year <= e.end_year.unwrap_or(ctx.present) + 3
    })
    .map(|e| (e.id, "war_levy"))
}

/// Pushes the famines. A planned famine that has a flood or war of its realm
/// before it sometimes moves to just after it.
pub fn famines(ctx: &Ctx<'_>, f: &Founding, formed: &[i32], tl: &mut Timeline) {
    for (ri, r) in ctx.realms.iter().enumerate() {
        let mut rng = Stream::new(ctx.seed, "famine", r.id, 0);
        let mut c = formed[ri] + 10;
        while c < ctx.present {
            let mut year = c + i32_of(rng.range(0, 119));
            c += 120;
            if year >= ctx.present || !rng.chance(380) {
                continue;
            }
            let anchors: Vec<i32> = tl
                .events
                .iter()
                .filter(|e| {
                    matches!(e.kind, EventKind::Flood | EventKind::War) && e.realms.contains(&r.id)
                })
                .map(|e| e.year)
                .collect();
            if rng.chance(1000) {
                if let Some(&a) = rng.pick(&anchors) {
                    let moved = a + i32_of(rng.range(0, 3));
                    if moved >= formed[ri] + 10 && moved < ctx.present {
                        year = moved;
                    }
                }
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
                .with("settlement", ctx.s(r.seat).name.clone())
                .with("river", stories::river(ctx, r.id));
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
            let cause = famine_cause(tl, ctx, r.id, year);
            let kind = cause.map_or_else(
                || {
                    if rng.chance(600) {
                        "rain_failure"
                    } else {
                        "blight"
                    }
                },
                |c| c.1,
            );
            e.cause_kind = Some(kind.to_string());
            let sl = slots_of(ctx, &e, &slots).with("realm", r.name.clone());
            stories::name_event(ctx, tl, &mut e, "famine", &sl, &mut rng);
            let cause_event = cause.and_then(|c| tl.get(c.0)).cloned();
            stories::tell(
                ctx,
                &mut e,
                &format!("famine_{kind}"),
                cause_event.as_ref(),
                &sl,
                &mut rng,
            );
            tl.push(e);
        }
    }
}

/// What started a plague in `year`: a famine in the realms it reached in the
/// six years before, else a war the origin's realm was fighting, else a flood
/// within three years. Returns the event id and the cause kind.
fn plague_cause(
    tl: &Timeline,
    ctx: &Ctx<'_>,
    year: i32,
    origin_realm: u64,
    realms: &[u64],
) -> Option<(u32, &'static str)> {
    if let Some(e) = latest(tl, EventKind::Famine, |e| {
        e.year < year && year <= e.year + 6 && shares(&e.realms, realms)
    }) {
        return Some((e.id, "famine_weakened"));
    }
    if let Some(e) = latest(tl, EventKind::War, |e| {
        e.year <= year
            && year <= e.end_year.unwrap_or(ctx.present)
            && e.realms.contains(&origin_realm)
    }) {
        return Some((e.id, "war_camp"));
    }
    latest(tl, EventKind::Flood, |e| {
        e.year < year && year <= e.year + 3 && shares(&e.realms, realms)
    })
    .map(|e| (e.id, "foul_wells"))
}

/// Pushes the plagues, spreading from a port or city along the roads. A
/// planned plague sometimes moves to just after a famine, war or great flood
/// and starts in that event's realm.
pub fn plagues(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) {
    let t = &ctx.t.history;
    let count = 1 + ctx.present / 140;
    for k in 0..count {
        let mut rng = Stream::new(ctx.seed, "plague", u64_of(i64::from(k)), 0);
        let mut year = i32_of(rng.range(50, i64::from(ctx.present - 15)));
        let mut within: Option<Vec<u64>> = None;
        if rng.chance(850) {
            let anchors: Vec<&Event> = tl
                .events
                .iter()
                .filter(|e| match e.kind {
                    EventKind::Famine | EventKind::War => true,
                    EventKind::Flood => e.severity >= 2,
                    _ => false,
                })
                .collect();
            // A famine is the likeliest cause of a plague, so it is tried first.
            let famines: Vec<&Event> = anchors
                .iter()
                .copied()
                .filter(|e| e.kind == EventKind::Famine)
                .collect();
            let pool = if !famines.is_empty() && rng.chance(700) {
                &famines
            } else {
                &anchors
            };
            if let Some(a) = rng.pick(pool) {
                let moved = match a.kind {
                    EventKind::Famine => a.year + i32_of(rng.range(1, 6)),
                    EventKind::War => {
                        let dur = a.end_year.map_or(3, |e| e - a.year).clamp(0, 6);
                        a.year + i32_of(rng.range(0, i64::from(dur)))
                    }
                    _ => a.year + i32_of(rng.range(1, 3)),
                };
                if (50..=ctx.present - 15).contains(&moved) {
                    year = moved;
                    within = Some(a.realms.clone());
                }
            }
        }
        let hub_ok = |i: usize, y: i32, within: &Option<Vec<u64>>| {
            let s = ctx.s(i);
            f.year[i] <= y
                && (s.has(Function::Port) || s.tier.is_urban())
                && within.as_ref().is_none_or(|w| w.contains(&s.realm_id))
        };
        let mut hubs: Vec<usize> = (0..ctx.n()).filter(|&i| hub_ok(i, year, &within)).collect();
        if hubs.is_empty() && within.is_some() {
            within = None;
            hubs = (0..ctx.n()).filter(|&i| hub_ok(i, year, &within)).collect();
        }
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
        let raw = rng.pick(&t.plague_names).cloned().unwrap_or_default();
        let name = stories::unique_name(tl, &raw, year);
        let slots = Slots::new()
            .with("plague", lower_the(&name))
            .with("settlement", ctx.s(origin).name.clone())
            .with("places", super::disasters::names(ctx, &others));
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
        e.name = Some(name);
        let origin_realm = ctx.s(origin).realm_id;
        let cause = plague_cause(tl, ctx, year, origin_realm, &e.realms);
        let kind = cause.map_or_else(
            || {
                if ctx.s(origin).has(Function::Port) {
                    "ship_borne"
                } else {
                    "crowded_lanes"
                }
            },
            |c| c.1,
        );
        e.cause_kind = Some(kind.to_string());
        let sl = slots_of(ctx, &e, &slots)
            .with("settlement", ctx.s(origin).name.clone())
            .with("places", super::disasters::names(ctx, &others));
        let cause_event = cause.and_then(|c| tl.get(c.0)).cloned();
        stories::tell(
            ctx,
            &mut e,
            &format!("plague_{kind}"),
            cause_event.as_ref(),
            &sl,
            &mut rng,
        );
        tl.push(e);
    }
}
