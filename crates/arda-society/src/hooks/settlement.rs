//! Settlement-level tensions: guilds against the lord, faction feuds, border
//! disputes, blockades, bandit roads, shortages, ruins, disasters, mines,
//! disloyal vassals and smuggling.

use super::{cite_role, select, Candidate, Hook, HookWorld};
use crate::ctx::Ctx;
use crate::history::EventKind;
use crate::input::{Function, RoadClass};
use crate::num::i64_of;
use crate::politics::factions::FactionStance;
use crate::politics::relations::{relation, Stance};
use crate::refs::EntityRef;
use crate::text::Slots;

/// Town guilds: their quarrel with the lord is about charters and dues.
const GUILDS: [&str; 2] = ["merchants", "crafts"];
/// Working folk whose quarrel with the lord is about what they owe.
const TOILERS: [&str; 5] = ["miners", "fishers", "boatmen", "foresters", "commons"];

fn shortfall_phrase(good: &str, pct: u64) -> String {
    if pct >= 90 {
        format!("has almost none of the {good} it needs")
    } else if pct >= 50 {
        format!("is short of {good}, getting less than half of what it needs")
    } else {
        format!("is short of {good}, about {pct} loads in every hundred it needs")
    }
}

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

fn feuds(w: &HookWorld<'_>, i: usize, base: &Slots, me: &EntityRef, c: &mut Vec<Candidate>) {
    let fs = &w.factions[i];
    for r in &w.faction_rel[i] {
        if !matches!(r.stance, FactionStance::Rival | FactionStance::Hostile) {
            continue;
        }
        let (Some(a), Some(b)) = (
            fs.iter().find(|f| f.id == r.a),
            fs.iter().find(|f| f.id == r.b),
        ) else {
            continue;
        };
        let hostile = i64::from(r.stance == FactionStance::Hostile) * 15;
        let (court, other) = if a.kind == "court" {
            (Some(a), b)
        } else if b.kind == "court" {
            (Some(b), a)
        } else {
            (None, b)
        };
        let mut sl = base.clone().with("reason", r.reason.clone());
        let mut refs = vec![
            me.clone(),
            EntityRef::faction(&a.id),
            EntityRef::faction(&b.id),
        ];
        refs.push(EntityRef::role(&a.leader_role));
        refs.push(EntityRef::role(&b.leader_role));
        let toil = TOILERS.contains(&other.kind.as_str());
        if let (Some(court), true) = (court, toil || GUILDS.contains(&other.kind.as_str())) {
            sl.set("guild", other.name.clone());
            sl.set("court", court.name.clone());
            cite_role(
                &mut refs,
                &mut sl,
                "lord",
                w.roles[i].iter().find(|x| x.id == court.leader_role),
            );
            cite_role(
                &mut refs,
                &mut sl,
                "master",
                w.roles[i].iter().find(|x| x.id == other.leader_role),
            );
            let kind = if toil {
                "dues_dispute"
            } else {
                "guild_vs_lord"
            };
            c.push(Candidate::new(
                70 + hostile - i64::from(toil) * 8,
                kind,
                sl,
                refs,
            ));
        } else {
            let (x, y) = if a.influence >= b.influence {
                (a, b)
            } else {
                (b, a)
            };
            sl.set("a", x.name.clone());
            sl.set("b", y.name.clone());
            cite_role(
                &mut refs,
                &mut sl,
                "a_leader",
                w.roles[i].iter().find(|r| r.id == x.leader_role),
            );
            cite_role(
                &mut refs,
                &mut sl,
                "b_leader",
                w.roles[i].iter().find(|r| r.id == y.leader_role),
            );
            let kind = if [x.kind.as_str(), y.kind.as_str()].contains(&"criminals") {
                "underworld"
            } else {
                "faction_feud"
            };
            c.push(Candidate::new(55 + hostile, kind, sl, refs));
        }
    }
}

fn borders(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let h = w.history;
    if let Some(b) = h.border_shifts.iter().find(|b| b.settlement == s.id) {
        let st =
            relation(w.relations, b.from_realm, b.to_realm).map_or(Stance::Neutral, |r| r.stance);
        let former = ctx
            .realm_ix(b.from_realm)
            .map_or(String::new(), |x| ctx.realms[x].name.clone());
        let mut sl = base
            .clone()
            .with("former", former)
            .with("year", b.year.to_string());
        let mut refs = vec![
            me.clone(),
            EntityRef::realm(b.from_realm),
            EntityRef::Event { id: b.war_event },
        ];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        let hot = matches!(st, Stance::War | Stance::Rivalry);
        let recent = ctx.present - b.year <= crate::politics::realm::RECENT_CONQUEST;
        let (score, kind) = match (recent, hot) {
            (true, true) => (80, "old_banner"),
            (true, false) => (50, "old_banner"),
            (false, true) => (45, "old_claim"),
            (false, false) => (32, "old_claim"),
        };
        c.push(Candidate::new(score, kind, sl, refs));
    }
    let mine = s.realm_id;
    for e in &ctx.graph.adj[i] {
        let other = ctx.s(e.to);
        if other.realm_id == mine {
            continue;
        }
        let st = relation(w.relations, mine, other.realm_id).map_or(Stance::Neutral, |r| r.stance);
        let foe = ctx.realms[ctx.realm_of(e.to)].name.clone();
        let mut sl = base
            .clone()
            .with("foe", foe)
            .with("other", other.name.clone())
            .with("road_class", e.class.label());
        let mut refs = vec![
            me.clone(),
            EntityRef::settlement(other.id),
            EntityRef::realm(other.realm_id),
            EntityRef::road(e.road),
        ];
        cite_role(
            &mut refs,
            &mut sl,
            "captain",
            w.role(i, "captain").or_else(|| w.head(ctx, i)),
        );
        match st {
            Stance::War => c.push(Candidate::new(88, "front_line", sl, refs)),
            Stance::Rivalry if w.traffic(e.road) > 0 => {
                if let Some(m) = w.factions[i].iter().find(|f| f.kind == "merchants") {
                    refs.push(EntityRef::faction(&m.id));
                }
                let rel = relation(w.relations, mine, other.realm_id);
                let ours =
                    rel.is_some_and(|r| crate::politics::relations::blockader(ctx.seed, r) == mine);
                let own = &w.realms[ctx.realm_of(i)];
                sl.set("ruler", format!("{} {}", own.ruler.title, own.ruler.regnal));
                c.push(Candidate::new(
                    74,
                    if ours { "embargo" } else { "blockade" },
                    sl,
                    refs,
                ));
            }
            Stance::Rivalry => c.push(Candidate::new(50, "border_dispute", sl, refs)),
            _ => {}
        }
    }
}

fn roads(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    // Brigands like long, busy roads away from the big garrisons: trunk
    // highways are patrolled, so lesser roads weigh triple.
    let lonely = |e: &crate::graph::Edge| if e.class == RoadClass::Highway { 1 } else { 3 };
    // Each road's bandits belong to its smaller end, so a road is one hook.
    let owns = |e: &&crate::graph::Edge| {
        let (me_s, other) = (ctx.s(i), ctx.s(e.to));
        (me_s.tier, me_s.population, me_s.id) < (other.tier, other.population, other.id)
    };
    let best = ctx.graph.adj[i].iter().filter(owns).max_by_key(|e| {
        (
            (e.len_m / 1000 + 1) * (w.traffic(e.road) / 500 + 1) * lonely(e),
            std::cmp::Reverse(e.road),
        )
    });
    let Some(e) = best else {
        return;
    };
    let other = ctx.s(e.to);
    let km = e.len_m / 1000;
    let mut sl = base
        .clone()
        .with("other", other.name.clone())
        .with("km", km.to_string())
        .with("road_class", e.class.label());
    let mut refs = vec![
        me.clone(),
        EntityRef::settlement(other.id),
        EntityRef::road(e.road),
    ];
    let ruin = w.history.ruins.iter().find(|r| r.road == Some(e.road));
    if let Some(r) = ruin {
        sl.set("hideout", format!("the ruins of {}", r.name));
        refs.push(EntityRef::Ruin { id: r.id });
    } else {
        let mut rng = crate::rng::Stream::new(ctx.seed, "hideout", e.road, 0);
        sl.set(
            "hideout",
            rng.pick(&ctx.t.hooks.hideouts).cloned().unwrap_or_default(),
        );
    }
    cite_role(
        &mut refs,
        &mut sl,
        "captain",
        w.role(i, "captain").or_else(|| w.head(ctx, i)),
    );
    let score = 14
        + i64_of(km).min(25)
        + i64_of(w.traffic(e.road) / 3000).min(20)
        + if ruin.is_some() { 8 } else { 0 }
        - if e.class == RoadClass::Highway { 10 } else { 0 };
    c.push(Candidate::new(score, "bandit_road", sl, refs));
}

fn goods(
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

fn past(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let h = w.history;
    if let Some(r) = h.ruins.iter().find(|r| r.near == s.id) {
        let mut sl = base
            .clone()
            .with("ruin", r.name.clone())
            .with("kind", r.kind.clone())
            .with("year", r.abandoned.to_string());
        let mut refs = vec![me.clone(), EntityRef::Ruin { id: r.id }];
        cite_role(&mut refs, &mut sl, "lord", w.head(ctx, i));
        c.push(Candidate::new(46, "ruin", sl, refs));
    }
    let recent = h
        .events
        .iter()
        .filter(|e| e.settlements.contains(&s.id) && e.severity >= 2 && ctx.present - e.year <= 45)
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
        let collapse = h
            .events
            .iter()
            .find(|e| e.kind == EventKind::MineCollapse && e.settlements.contains(&s.id));
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
        .history
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Flood && e.settlements.contains(&s.id))
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
        .history
        .wars
        .iter()
        .filter(|x| x.realms.contains(&r.id) && x.to.is_none_or(|t| ctx.present - t <= 12))
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
