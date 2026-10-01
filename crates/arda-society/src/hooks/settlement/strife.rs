//! Feuds between factions and with the lord, border disputes with
//! neighbouring realms, blockades and bandit roads.

use crate::ctx::Ctx;
use crate::hooks::{cite_role, Candidate, HookWorld};
use crate::input::RoadClass;
use crate::num::i64_of;
use crate::politics::factions::FactionStance;
use crate::politics::relations::Stance;
use crate::refs::EntityRef;
use crate::text::Slots;

/// Town guilds: their quarrel with the lord is about charters and dues.
const GUILDS: [&str; 2] = ["merchants", "crafts"];
/// Working folk whose quarrel with the lord is about what they owe.
const TOILERS: [&str; 5] = ["miners", "fishers", "boatmen", "foresters", "commons"];

pub(super) fn feuds(
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
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

pub(super) fn borders(
    ctx: &Ctx<'_>,
    w: &HookWorld<'_>,
    i: usize,
    base: &Slots,
    me: &EntityRef,
    c: &mut Vec<Candidate>,
) {
    let s = ctx.s(i);
    let h = w.history;
    if let Some(b) = w.index.first_shift(h, s.id) {
        let st = w
            .relation(b.from_realm, b.to_realm)
            .map_or(Stance::Neutral, |r| r.stance);
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
        let st = w
            .relation(mine, other.realm_id)
            .map_or(Stance::Neutral, |r| r.stance);
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
                let rel = w.relation(mine, other.realm_id);
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

pub(super) fn roads(
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
    let ruin = w.index.ruin_on_road(w.history, e.road);
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
