//! Founding order: the best sites are settled first and settlement spreads
//! along roads (and so along the rivers and coasts roads follow).
//!
//! Roots (each realm seat and the best site of every road component) are
//! founded in the first decades. Every other settlement is founded from its
//! road neighbour that reaches it earliest, after a delay that shrinks with
//! site quality and grows with distance — a Dijkstra over "years", which
//! guarantees nobody is founded before the neighbour that settled it.

use super::{event, EventKind, Timeline};
use crate::ctx::Ctx;
use crate::input::{Function, Tier};
use crate::num::{i32_of, i64_of};
use crate::refs::EntityRef;
use crate::rng::{hash, Stream};
use crate::text::{pick_fill, Slots};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Founding year and origin per node.
#[derive(Debug, Clone)]
pub struct Founding {
    /// Year founded, per node.
    pub year: Vec<i32>,
    /// Neighbour whose settlers founded it; `None` for roots.
    pub parent: Vec<Option<usize>>,
    /// Site score, per node.
    pub score: Vec<i64>,
    /// Founding event id (provisional), per node.
    pub event: Vec<u32>,
}

/// Site quality: bigger, better-connected, better-watered sites score higher.
#[must_use]
pub fn site_score(ctx: &Ctx<'_>, i: usize) -> i64 {
    let s = ctx.s(i);
    let mut v = match s.tier {
        Tier::Hamlet => 0,
        Tier::Village => 120,
        Tier::Town => 260,
        Tier::City => 400,
    };
    let f = |func: Function, pts: i64| if s.has(func) { pts } else { 0 };
    let t = |tag: &str, pts: i64| if s.tagged(tag) { pts } else { 0 };
    v += f(Function::Capital, 120) + f(Function::Crossing, 60) + f(Function::Port, 40);
    v += f(Function::Market, 40) + f(Function::Fortress, 30) + f(Function::Abbey, 25);
    v += t("confluence", 35) + t("ford", 25) + t("bridge_site", 25) + t("harbour", 40);
    v += t("defensible", 20) + t("spring", 15) + t("estuary", 25);
    v += if s.riverine { 50 } else { 0 } + if s.coastal { 30 } else { 0 };
    v + i64_of(hash(ctx.seed, "site_noise", s.id, 0) % 60)
}

/// Computes founding years and origins, and pushes founding events.
#[must_use]
pub fn found(ctx: &Ctx<'_>, tl: &mut Timeline) -> Founding {
    let n = ctx.n();
    let score: Vec<i64> = (0..n).map(|i| site_score(ctx, i)).collect();
    let best = score.iter().copied().max().unwrap_or(0);
    let mut roots: Vec<usize> = ctx.realms.iter().map(|r| r.seat).collect();
    for comp in ctx.graph.components() {
        if let Some(&top) = comp.iter().max_by_key(|&&i| (score[i], Reverse(i))) {
            roots.push(top);
        }
    }
    roots.sort_unstable();
    roots.dedup();
    let mut year = vec![i64::MAX; n];
    let mut parent = vec![None; n];
    let mut heap = BinaryHeap::new();
    for &r in &roots {
        let y = 1
            + (best - score[r]).max(0) / 25
            + i64_of(hash(ctx.seed, "root_year", ctx.s(r).id, 0) % 9);
        year[r] = y;
        heap.push(Reverse((y, r)));
    }
    while let Some(Reverse((y, u))) = heap.pop() {
        if y > year[u] {
            continue;
        }
        for e in &ctx.graph.adj[u] {
            let v = e.to;
            let km = i64_of(e.len_m / 1000);
            let noise = i64_of(hash(ctx.seed, "found_delay", ctx.s(u).id, ctx.s(v).id) % 12);
            let delay = 6 + (700 - score[v]).max(0) / 9 + km / 2 + noise;
            let ny = y + delay;
            if ny < year[v] {
                year[v] = ny;
                parent[v] = Some(u);
                heap.push(Reverse((ny, v)));
            }
        }
    }
    // Rescale so the youngest hamlet appears in the last fifth of the span.
    let max_y = year
        .iter()
        .copied()
        .filter(|&y| y != i64::MAX)
        .max()
        .unwrap_or(1)
        .max(2);
    let target = i64::from(ctx.present) * 4 / 5;
    let years: Vec<i32> = year
        .iter()
        .map(|&y| {
            let y = if y == i64::MAX { max_y } else { y };
            i32_of(1 + (y - 1) * (target - 1) / (max_y - 1))
        })
        .collect();
    let mut events = Vec::with_capacity(n);
    for i in 0..n {
        let s = ctx.s(i);
        let mut rng = Stream::new(ctx.seed, "found_text", s.id, 0);
        let mut slots = Slots::new()
            .with("settlement", s.name.clone())
            .with("site", site_phrase(ctx, i));
        let key = if let Some(p) = parent[i] {
            slots.set("parent", ctx.s(p).name.clone());
            "founding_from"
        } else {
            "founding_root"
        };
        let t = &ctx.t.history;
        let title = pick_fill(
            t.titles.get(key).map_or(&[][..], Vec::as_slice),
            &mut rng,
            &slots,
        );
        let text = pick_fill(
            t.events.get(key).map_or(&[][..], Vec::as_slice),
            &mut rng,
            &slots,
        );
        let mut e = event(EventKind::Founding, years[i], title, text);
        e.settlements.push(s.id);
        if let Some(p) = parent[i] {
            e.settlements.push(ctx.s(p).id);
            e.refs.push(EntityRef::settlement(ctx.s(p).id));
        }
        events.push(tl.push(e));
    }
    Founding {
        year: years,
        parent,
        score,
        event: events,
    }
}

/// The phrase for a settlement's site ("the ford", "the sheltered harbour").
#[must_use]
pub fn site_phrase(ctx: &Ctx<'_>, i: usize) -> String {
    let t = &ctx.t.history;
    let s = ctx.s(i);
    t.site_order
        .iter()
        .find(|tag| s.tagged(tag))
        .and_then(|tag| t.sites.get(tag))
        .cloned()
        .unwrap_or_else(|| t.site_default.clone())
}
