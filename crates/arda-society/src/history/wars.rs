//! Wars between neighbouring realms and the border shifts they caused.
//!
//! The present map is the outcome: a war only ever moves a border
//! settlement *into* the realm that holds it today, so the simulated past
//! converges on the input realms. Tension grows with shared border roads
//! and differing cultures and falls with mutual trade.

use super::founding::Founding;
use super::{event, BorderShift, EventKind, Timeline, War};
use crate::ctx::Ctx;
use crate::num::{i32_of, i64_of};
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::text::{join_and, pick_fill, Slots};
use std::collections::{BTreeMap, BTreeSet};

/// Wars and shifts with provisional event ids.
#[derive(Debug, Clone, Default)]
pub struct WarRun {
    /// Wars.
    pub wars: Vec<War>,
    /// Border shifts.
    pub shifts: Vec<BorderShift>,
    /// War tension per realm-index pair, per mille per decade.
    pub tension: BTreeMap<(usize, usize), i64>,
}

const DURATIONS: [i32; 10] = [1, 1, 2, 2, 3, 3, 4, 6, 8, 11];

fn pairs(ctx: &Ctx<'_>) -> BTreeMap<(usize, usize), usize> {
    let mut out: BTreeMap<(usize, usize), usize> = ctx
        .border_roads()
        .into_iter()
        .map(|(k, v)| (k, v.len()))
        .collect();
    for a in 0..ctx.realms.len() {
        for b in a + 1..ctx.realms.len() {
            if ctx.dist(ctx.realms[a].seat, ctx.realms[b].seat) < 200_000 {
                out.entry((a, b)).or_insert(0);
            }
        }
    }
    out
}

/// Simulates wars; `trade` is present trade value per realm-index pair.
#[must_use]
pub fn wars(
    ctx: &Ctx<'_>,
    f: &Founding,
    formed: &[i32],
    trade: &BTreeMap<(usize, usize), u64>,
    tl: &mut Timeline,
) -> WarRun {
    let mut run = WarRun::default();
    let mut shifted: BTreeSet<usize> = BTreeSet::new();
    let present = ctx.present;
    let strength: Vec<i64> = ctx
        .realms
        .iter()
        .map(|r| {
            r.members
                .iter()
                .map(|&i| i64::from(ctx.s(i).population))
                .sum()
        })
        .collect();
    for (&(a, b), &roads) in &pairs(ctx) {
        let (ra, rb) = (&ctx.realms[a], &ctx.realms[b]);
        let culture_gap = ctx.s(ra.seat).culture != ctx.s(rb.seat).culture;
        let t = trade.get(&(a, b)).copied().unwrap_or(0);
        let roads = i64::try_from(roads).unwrap_or(0);
        let tension = (45 + (roads * 15).min(90) + if culture_gap { 30 } else { 0 }
            - i64_of(t / 3000).min(50))
        .clamp(25, 240);
        run.tension.insert((a, b), tension);
        let key = crate::rng::hash(0, "pair", ra.id, rb.id);
        let mut rng = Stream::new(ctx.seed, "wars", key, 0);
        let mut y = formed[a].max(formed[b]) + 5;
        let mut last_end = y;
        while y < present - 10 {
            if !rng.chance(crate::num::u64_of(tension * 11 / 20)) {
                y += 10;
                continue;
            }
            let begin = y + i32_of(rng.range(0, 9));
            let dur = rng.pick(&DURATIONS).copied().unwrap_or(2);
            let end = begin + dur;
            if end >= present - 2 {
                break;
            }
            let sa = strength[a] * rng.range(70, 130);
            let sb = strength[b] * rng.range(70, 130);
            let (win, lose) = if sa >= sb { (a, b) } else { (b, a) };
            let take = 1 + usize::from(dur >= 4);
            let gained = capture(ctx, f, win, lose, begin, take, &mut shifted, &mut rng);
            add_war(
                ctx,
                &mut run,
                tl,
                (a, b),
                (begin, Some(end)),
                Some(win),
                &gained,
                t,
                &mut rng,
            );
            last_end = end;
            y = end + 20 + i32_of(rng.range(0, 10));
        }
        // An unresolved war at present, likelier the tenser the pair.
        let mut now = Stream::new(ctx.seed, "war_now", key, 0);
        let begin = present - i32_of(now.range(1, 5));
        if now.chance(crate::num::u64_of(tension * 3 / 4)) && begin > last_end + 5 {
            add_war(
                ctx,
                &mut run,
                tl,
                (a, b),
                (begin, None),
                None,
                &[],
                t,
                &mut now,
            );
        }
    }
    run
}

#[allow(clippy::too_many_arguments)]
fn capture(
    ctx: &Ctx<'_>,
    f: &Founding,
    win: usize,
    lose: usize,
    begin: i32,
    take: usize,
    shifted: &mut BTreeSet<usize>,
    rng: &mut Stream,
) -> Vec<usize> {
    let mut cand: Vec<(u64, usize)> = ctx.realms[win]
        .members
        .iter()
        .copied()
        .filter(|&i| {
            !ctx.is_seat(i)
                && !shifted.contains(&i)
                && f.year[i] <= begin
                && ctx.graph.adj[i].iter().any(|e| ctx.realm_of(e.to) == lose)
                && f.parent[i].is_none_or(|p| ctx.realm_of(p) == lose)
        })
        .map(|i| (rng.next_u64(), i))
        .collect();
    cand.sort_unstable();
    let got: Vec<usize> = cand.into_iter().take(take).map(|c| c.1).collect();
    shifted.extend(got.iter().copied());
    got
}

#[allow(clippy::too_many_arguments)]
fn add_war(
    ctx: &Ctx<'_>,
    run: &mut WarRun,
    tl: &mut Timeline,
    (a, b): (usize, usize),
    (begin, end): (i32, Option<i32>),
    winner: Option<usize>,
    gained: &[usize],
    trade: u64,
    rng: &mut Stream,
) {
    let t = &ctx.t.history;
    let (ra, rb) = (&ctx.realms[a], &ctx.realms[b]);
    let border: Vec<usize> = ra
        .members
        .iter()
        .chain(rb.members.iter())
        .copied()
        .filter(|&i| ctx.is_border(i) && !ctx.is_seat(i))
        .collect();
    let place = gained
        .first()
        .or_else(|| rng.pick(&border))
        .map_or_else(|| ctx.s(rb.seat).name.clone(), |&i| ctx.s(i).name.clone());
    let good = if trade > 0 { "toll" } else { "border" };
    let years = end.map_or(0, |e| e - begin).max(1);
    let slots = Slots::new()
        .with("realm_a", ra.name.clone())
        .with("realm_b", rb.name.clone())
        .with("place", place)
        .with("years", years.to_string())
        .with("cause", good)
        .with("year", begin.to_string());
    let name = crate::text::pick_name(&t.war_names, rng, &slots);
    let slots = slots.with("war", name.clone());
    let title = crate::text::capitalise_first(&name);
    let mut e = event(
        EventKind::War,
        begin,
        title,
        pick_fill(key_list(t, "war"), rng, &slots),
    );
    e.end_year = end;
    e.ongoing = end.is_none();
    e.realms = vec![ra.id, rb.id];
    e.severity = u8::try_from(years.clamp(1, 9) / 3 + 1).unwrap_or(1);
    let war_id = tl.push(e);
    run.wars.push(War {
        event: war_id,
        name: name.clone(),
        realms: [ra.id, rb.id],
        from: begin,
        to: end,
        winner: winner.map(|w| ctx.realms[w].id),
    });
    let (Some(end), Some(w)) = (end, winner) else {
        return;
    };
    let lose = if w == a { b } else { a };
    let (rw, rl) = (&ctx.realms[w], &ctx.realms[lose]);
    let names: Vec<String> = gained.iter().map(|&i| ctx.s(i).name.clone()).collect();
    let slots = slots
        .with("winner", rw.name.clone())
        .with("loser", rl.name.clone())
        .with("gains", join_and(&names));
    let key = if gained.is_empty() {
        "peace_status_quo"
    } else {
        "peace_gains"
    };
    let mut p = event(
        EventKind::Peace,
        end,
        pick_fill(
            t.titles.get("peace").map_or(&[][..], Vec::as_slice),
            rng,
            &slots,
        ),
        pick_fill(key_list(t, key), rng, &slots),
    );
    p.realms = vec![rw.id, rl.id];
    p.cause = Some(war_id);
    p.refs.push(EntityRef::Event { id: war_id });
    let peace_id = tl.push(p);
    for &i in gained {
        let s = ctx.s(i);
        let sl = slots.clone().with("settlement", s.name.clone());
        let mut e = event(
            EventKind::BorderShift,
            end,
            pick_fill(
                t.titles.get("border_shift").map_or(&[][..], Vec::as_slice),
                rng,
                &sl,
            ),
            pick_fill(key_list(t, "border_shift"), rng, &sl),
        );
        e.settlements.push(s.id);
        e.realms = vec![rl.id, rw.id];
        e.cause = Some(war_id);
        e.refs.push(EntityRef::Event { id: peace_id });
        e.severity = 2;
        tl.push(e);
        run.shifts.push(BorderShift {
            settlement: s.id,
            from_realm: rl.id,
            to_realm: rw.id,
            year: end,
            war_event: war_id,
        });
    }
}

fn key_list<'a>(t: &'a crate::tables::HistoryTable, key: &str) -> &'a [String] {
    t.events.get(key).map_or(&[][..], Vec::as_slice)
}
