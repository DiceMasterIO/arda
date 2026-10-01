//! Relations between realms, driven by shared borders, trade and history.
//! One record per unordered pair, so relations are symmetric by
//! construction; [`relation`] looks a pair up in either order.

use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::history::run::{realm_trade, HistoryRun};
use crate::num::i64_of;
use crate::rng::hash;
use crate::text::{count, Slots};
use serde::{Deserialize, Serialize};

/// How two realms stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// Sworn allies.
    Alliance,
    /// A treaty of trade.
    TradePact,
    /// No particular ties.
    Neutral,
    /// Hostile but at peace.
    Rivalry,
    /// At war now.
    War,
}

impl Stance {
    /// Readable name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Alliance => "alliance",
            Self::TradePact => "trade pact",
            Self::Neutral => "neutral",
            Self::Rivalry => "rivalry",
            Self::War => "war",
        }
    }
}

/// Relation between realms `a < b`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    /// Lower realm id.
    #[serde(with = "crate::ids::str")]
    pub a: u64,
    /// Higher realm id.
    #[serde(with = "crate::ids::str")]
    pub b: u64,
    /// Stance.
    pub stance: Stance,
    /// Underlying score (positive friendly).
    pub score: i32,
    /// Roads crossing their shared border.
    #[serde(with = "crate::ids::vec")]
    pub border_roads: Vec<u64>,
    /// Yearly trade between them, silver pieces.
    pub trade_sp: u64,
    /// War event ids between them.
    pub wars: Vec<u32>,
    /// Whether their houses are joined by marriage.
    pub marriage: bool,
    /// Why they stand as they do.
    pub reasons: Vec<String>,
}

/// The relation between `x` and `y` in either order.
#[must_use]
pub fn relation(list: &[Relation], x: u64, y: u64) -> Option<&Relation> {
    let (a, b) = (x.min(y), x.max(y));
    list.iter().find(|r| r.a == a && r.b == b)
}

/// Which realm of an unfriendly pair has closed the border roads; the
/// same answer for both sides, so hooks agree.
#[must_use]
pub fn blockader(seed: u64, rel: &Relation) -> u64 {
    if hash(seed, "blockade", rel.a, rel.b).is_multiple_of(2) {
        rel.a
    } else {
        rel.b
    }
}

/// All pairwise relations.
#[must_use]
pub fn relations(
    ctx: &Ctx<'_>,
    hist: &HistoryRun,
    econ: &EconomyRun,
    hereditary: &[bool],
) -> Vec<Relation> {
    let border = ctx.border_roads();
    let trade = realm_trade(ctx, econ);
    let h = &hist.history;
    let reasons = &ctx.t.politics.reasons;
    let mut out = Vec::new();
    for a in 0..ctx.realms.len() {
        for b in a + 1..ctx.realms.len() {
            let (ra, rb) = (&ctx.realms[a], &ctx.realms[b]);
            let roads = border.get(&(a, b)).cloned().unwrap_or_default();
            let trade_sp = trade.get(&(a, b)).copied().unwrap_or(0);
            // Indexed by pair (review round 2 #29: every war and shift was
            // scanned for each of the R² pairs).
            let wars: Vec<&crate::history::War> = hist
                .index
                .wars_between(h, ra.id, rb.id)
                .filter(|w| w.realms == [ra.id, rb.id] || w.realms == [rb.id, ra.id])
                .collect();
            let ongoing = wars.iter().any(|w| w.to.is_none());
            let last_end = wars.iter().filter_map(|w| w.to).max();
            let taken = hist.index.shifts_between(ra.id, rb.id);
            let same_culture = ctx.s(ra.seat).culture == ctx.s(rb.seat).culture;
            let key = crate::rng::hash(0, "pair", ra.id, rb.id);
            let marriage = hereditary[a]
                && hereditary[b]
                && !ongoing
                && hash(ctx.seed, "marriage", key, 0) % 100 < 30;
            let mut score = i64_of(trade_sp / 900).min(30);
            score += if same_culture { 10 } else { -4 };
            score += if marriage { 20 } else { 0 };
            score -= (i64::try_from(roads.len()).unwrap_or(0) * 3).min(15);
            // Wars within 150 years count fully, older ones half.
            let war_pts: i64 = wars
                .iter()
                .map(|w| {
                    if ctx.present - w.to.unwrap_or(ctx.present) <= 150 {
                        6
                    } else {
                        3
                    }
                })
                .sum();
            score -= war_pts;
            score -= i64::try_from(taken).unwrap_or(0) * 5;
            if let Some(end) = last_end {
                score -= (20 * (50 - i64::from(ctx.present - end)).max(0)) / 50;
            }
            score += i64_of(hash(ctx.seed, "relation", key, 0) % 11) - 5;
            let stance = if ongoing {
                Stance::War
            } else if score >= 22 && !roads.is_empty() {
                Stance::Alliance
            } else if score >= 6 && trade_sp > 0 {
                Stance::TradePact
            } else if score <= -16 {
                Stance::Rivalry
            } else {
                Stance::Neutral
            };
            let sl = Slots::new()
                .with("a", ra.name.clone())
                .with("b", rb.name.clone())
                .with("roads", count(roads.len(), "road", "roads"))
                .with("wars", count(wars.len(), "war", "wars"))
                .with("taken", count(taken, "settlement", "settlements"))
                .with("trade", crate::gazetteer::thousands(trade_sp))
                .with("years", last_end.map_or(0, |e| ctx.present - e).to_string());
            let mut why = Vec::new();
            let mut say = |k: &str, cond: bool| {
                if cond {
                    if let Some(t) = reasons.get(k) {
                        why.push(sl.fill(t));
                    }
                }
            };
            say("war_now", ongoing);
            say("border", !roads.is_empty());
            say("trade", trade_sp > 0);
            say("wars", !wars.is_empty());
            say("taken", taken > 0);
            say(
                "recent_war",
                last_end.is_some_and(|e| ctx.present - e < 60) && !ongoing,
            );
            say("culture", same_culture);
            say("marriage", marriage);
            say("distant", roads.is_empty() && trade_sp == 0);
            out.push(Relation {
                a: ra.id,
                b: rb.id,
                stance,
                score: crate::num::i32_of(score),
                border_roads: roads,
                trade_sp,
                wars: wars.iter().map(|w| w.event).collect(),
                marriage,
                reasons: why,
            });
        }
    }
    out
}
