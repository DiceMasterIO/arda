//! Ruins of abandoned settlements beside the road network. Each ruin was
//! emptied by a real event near it (plague, flood, war, famine, fire, mine
//! collapse) when one fits its lifetime, otherwise by a local cause.

use super::founding::Founding;
use super::{event, EventKind, Ruin, Timeline};
use crate::ctx::Ctx;
use crate::input::Tier;
use crate::names;
use crate::num::{i32_of, i64_of, u64_of_usize, usize_of};
use crate::refs::EntityRef;
use crate::rng::Stream;
use crate::text::{pick_fill, Slots};

const LOCAL_CAUSES: [&str; 3] = ["well_failed", "raiders", "enclosure"];

fn cause_key(kind: EventKind) -> Option<&'static str> {
    Some(match kind {
        EventKind::Plague => "plague",
        EventKind::Flood => "flood",
        EventKind::War | EventKind::BorderShift => "war",
        EventKind::Famine => "famine",
        EventKind::Fire => "fire",
        EventKind::MineCollapse => "mine_collapse",
        _ => return None,
    })
}

/// Places ruins and pushes their abandonment events.
#[must_use]
pub fn ruins(ctx: &Ctx<'_>, f: &Founding, tl: &mut Timeline) -> Vec<Ruin> {
    let n = ctx.n();
    if n == 0 {
        return Vec::new();
    }
    let count = (n / 5).clamp(2, 60);
    let roads: Vec<&crate::input::Road> = ctx
        .world
        .roads
        .iter()
        .filter(|r| r.to.is_some_and(|t| ctx.node(t).is_some()) && ctx.node(r.from).is_some())
        .collect();
    let mut out = Vec::with_capacity(count);
    for k in 0..count {
        let mut rng = Stream::new(ctx.seed, "ruin", u64_of_usize(k), 0);
        let (near, road, x, y) = if let Some(r) = rng.pick(&roads) {
            let a = ctx.node(r.from).unwrap_or(0);
            let b = r.to.and_then(|t| ctx.node(t)).unwrap_or(a);
            let (sa, sb) = (ctx.s(a), ctx.s(b));
            let t = rng.range(30, 70);
            let x = sa.x_m + (sb.x_m - sa.x_m) * t / 100;
            let y = sa.y_m + (sb.y_m - sa.y_m) * t / 100;
            let (dx, dy) = (sb.x_m - sa.x_m, sb.y_m - sa.y_m);
            let len = i64_of(crate::num::isqrt(crate::num::u64_of(dx * dx + dy * dy))).max(1);
            let off = rng.range(-2500, 2500);
            let near = if t < 50 { a } else { b };
            (near, Some(r.id), x - dy * off / len, y + dx * off / len)
        } else {
            let a = usize_of(rng.below(u64_of_usize(n)));
            (
                a,
                None,
                ctx.s(a).x_m + rng.range(-3000, 3000),
                ctx.s(a).y_m + rng.range(-3000, 3000),
            )
        };
        let s = ctx.s(near);
        let founded = i32_of(rng.range(i64::from(f.year[near] - 40), i64::from(f.year[near] + 60)))
            .clamp(1, ctx.present - 40);
        let hit = tl
            .events
            .iter()
            .filter(|e| {
                cause_key(e.kind).is_some()
                    && e.year > founded + 10
                    && e.year < ctx.present - 5
                    && (e.settlements.contains(&s.id)
                        || (e.kind == EventKind::Famine && e.realms.contains(&s.realm_id)))
            })
            .max_by_key(|e| {
                (
                    e.severity,
                    std::cmp::Reverse(e.year),
                    std::cmp::Reverse(e.id),
                )
            })
            .map(|e| (e.id, e.year, e.kind));
        let (cause, cause_event, abandoned) = match hit {
            Some((id, year, kind)) if rng.chance(800) => (
                cause_key(kind).unwrap_or("raiders").to_string(),
                Some(id),
                year + i32_of(rng.range(0, 3)),
            ),
            _ => {
                let c = rng.pick(&LOCAL_CAUSES).copied().unwrap_or("raiders");
                let y = founded + i32_of(rng.range(30, 160));
                (c.to_string(), None, y.min(ctx.present - 8))
            }
        };
        let abandoned = abandoned.max(founded + 5).min(ctx.present - 1);
        let former_tier = if rng.chance(300) {
            Tier::Village
        } else {
            Tier::Hamlet
        };
        let kinds = ctx
            .t
            .history
            .ruin_kinds
            .get(former_tier.key())
            .map_or(&[][..], Vec::as_slice);
        let kind = rng
            .pick(kinds)
            .cloned()
            .unwrap_or_else(|| former_tier.key().to_string());
        let mut name = names::place(&ctx.t.names, &s.culture, &mut rng);
        while out.iter().any(|r: &Ruin| r.name == name)
            || ctx.world.settlements.iter().any(|x| x.name == name)
        {
            name = names::place(&ctx.t.names, &s.culture, &mut rng);
        }
        let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
        let slots = Slots::new()
            .with("ruin", name.clone())
            .with("kind", kind.clone())
            .with("settlement", s.name.clone());
        let t = &ctx.t.history;
        let key = format!("abandon_{cause}");
        let text = pick_fill(
            t.events.get(&key).map_or(&[][..], Vec::as_slice),
            &mut rng,
            &slots,
        );
        let mut e = event(
            EventKind::Abandonment,
            abandoned,
            format!("{name} is abandoned"),
            text,
        );
        e.refs.push(EntityRef::Ruin { id });
        e.realms.push(s.realm_id);
        e.cause = cause_event;
        let ev = tl.push(e);
        out.push(Ruin {
            id,
            name,
            kind,
            former_tier,
            x_m: x,
            y_m: y,
            near: s.id,
            road,
            realm_id: s.realm_id,
            founded,
            abandoned,
            cause,
            cause_event,
            event: ev,
        });
    }
    out
}
