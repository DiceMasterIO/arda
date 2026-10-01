//! A readable gazetteer of a [`Society`]: realms, rulers, relations, the
//! timeline, the economy, every settlement with its hooks, and the ruins.

use crate::history::EventKind;
use crate::input::{Tier, WorldSettlements};
use crate::politics::factions::FactionStance;
use crate::Society;
use std::collections::BTreeMap;

/// `12345` → `"12,345"`.
#[must_use]
pub fn thousands(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn push(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

/// Renders the gazetteer. `world` supplies tiers, populations and functions.
#[must_use]
pub fn render(society: &Society, world: &WorldSettlements) -> String {
    let mut o = String::new();
    let h = &society.history;
    let names: BTreeMap<u64, &str> = world
        .settlements
        .iter()
        .map(|s| (s.id, s.name.as_str()))
        .collect();
    let realm_names: BTreeMap<u64, &str> = society
        .realms
        .iter()
        .map(|r| (r.state.id, r.state.name.as_str()))
        .collect();
    let name = |id: u64| names.get(&id).copied().unwrap_or("?");
    let yr = |y: i32| format!("{y} {}", h.era_abbrev);
    push(
        &mut o,
        &format!("GAZETTEER OF THE WORLD, YEAR {}", yr(h.present_year)),
    );
    push(
        &mut o,
        &format!(
            "{} years reckoned {} (seed {}).",
            h.present_year, h.era, society.seed
        ),
    );
    push(&mut o, "");
    push(&mut o, "== REALMS ==");
    for r in &society.realms {
        let st = &r.state;
        push(&mut o, "");
        push(
            &mut o,
            &format!("{} ({}, {})", st.style, st.government_label, st.rank),
        );
        push(
            &mut o,
            &format!(
                "  Seat {}. Formed {}. {} souls in {} settlements; {} culture.",
                name(st.seat),
                yr(st.founded),
                thousands(st.population),
                st.members.len(),
                st.culture
            ),
        );
        push(
            &mut o,
            &format!(
                "  Ruler: {} {} of House {}, reigning since {} [{}].",
                st.ruler.title,
                st.ruler.regnal,
                st.ruler.house,
                yr(st.ruler.since),
                st.ruler.role
            ),
        );
        push(
            &mut o,
            &format!(
                "  Levy: {}% of trade, about {} sp a year.",
                st.tax_pct,
                thousands(st.levy_sp)
            ),
        );
        for v in &st.vassals {
            let g = if v.grievances.is_empty() {
                String::new()
            } else {
                format!(" Grievance: {}", v.grievances.join(" "))
            };
            push(
                &mut o,
                &format!(
                    "  Vassal: {} {} of House {}, holding {} (loyalty {}).{g}",
                    v.title,
                    v.given,
                    v.house,
                    name(v.settlement),
                    v.loyalty
                ),
            );
        }
        for rel in society
            .relations
            .iter()
            .filter(|x| x.a == st.id || x.b == st.id)
        {
            let other = if rel.a == st.id { rel.b } else { rel.a };
            push(
                &mut o,
                &format!(
                    "  With {}: {} (score {}). {}",
                    realm_names.get(&other).copied().unwrap_or("?"),
                    rel.stance.label(),
                    rel.score,
                    rel.reasons.join(" ")
                ),
            );
        }
        push(&mut o, "  Realm hooks:");
        for hk in &r.hooks {
            push(
                &mut o,
                &format!(
                    "    * {} [{}, tension {}] {}",
                    hk.title, hk.kind, hk.tension, hk.text
                ),
            );
        }
    }
    push(&mut o, "");
    push(&mut o, "== TIMELINE ==");
    let tiers: BTreeMap<u64, Tier> = world.settlements.iter().map(|s| (s.id, s.tier)).collect();
    for e in &h.events {
        let keep = match e.kind {
            EventKind::Founding => e
                .settlements
                .first()
                .and_then(|s| tiers.get(s))
                .is_some_and(|t| *t != Tier::Hamlet),
            EventKind::Accession => false,
            EventKind::Flood | EventKind::Fire => e.severity >= 2,
            _ => true,
        };
        if keep {
            let span = e
                .end_year
                .filter(|&y| y != e.year)
                .map_or(String::new(), |y| format!("–{y}"));
            let span = if e.ongoing {
                "–present".to_string()
            } else {
                span
            };
            push(
                &mut o,
                &format!("  {:>4}{span:<6} {}. {}", e.year, e.title, e.text),
            );
        }
    }
    push(&mut o, "");
    push(&mut o, "== ECONOMY (loads a year) ==");
    push(
        &mut o,
        &format!(
            "  {:<14}{:>9}{:>9}{:>9}{:>9}{:>9}",
            "good", "made", "at home", "traded", "stored", "short"
        ),
    );
    for g in &society.economy.goods {
        push(
            &mut o,
            &format!(
                "  {:<14}{:>9}{:>9}{:>9}{:>9}{:>9}",
                g.label, g.produced, g.local_use, g.traded, g.stored, g.shortfall
            ),
        );
    }
    push(&mut o, "  Busiest roads:");
    for t in society.economy.road_traffic.iter().take(6) {
        let road = world.roads.iter().find(|r| r.id == t.road);
        let ends = road.map_or(String::new(), |r| {
            format!("{} – {}", name(r.from), r.to.map_or("?", name))
        });
        push(
            &mut o,
            &format!(
                "    road {} {ends}: {} sp a year ({})",
                t.road,
                thousands(t.value_sp),
                t.goods.join(", ")
            ),
        );
    }
    push(&mut o, "");
    push(&mut o, "== SETTLEMENTS ==");
    let mut order: Vec<&crate::input::Settlement> = world.settlements.iter().collect();
    order.sort_by_key(|s| {
        (
            s.realm_id,
            std::cmp::Reverse(s.tier),
            std::cmp::Reverse(s.population),
            s.id,
        )
    });
    // First record per id, as a scan per settlement found it (review round
    // 2 #29: that scan made the gazetteer quadratic).
    let mut by_id: std::collections::BTreeMap<u64, &crate::SettlementSociety> =
        std::collections::BTreeMap::new();
    for x in &society.settlements {
        by_id.entry(x.id).or_insert(x);
    }
    for s in order {
        let Some(&ss) = by_id.get(&s.id) else {
            continue;
        };
        push(&mut o, "");
        let from = ss
            .founded_from
            .map_or(String::new(), |p| format!(" from {}", name(p)));
        let funcs: Vec<&str> = s.functions.iter().map(|f| f.key()).collect();
        push(
            &mut o,
            &format!(
                "{} — {} of {}, {} ({}). Founded {}{from}. Wealth {}, {:?}.",
                s.name,
                s.tier.key(),
                thousands(u64::from(s.population)),
                realm_names.get(&s.realm_id).copied().unwrap_or("?"),
                funcs.join(", "),
                yr(ss.founded),
                s.wealth,
                ss.prosperity.trend
            ),
        );
        let ec = &ss.economy;
        if !ec.key_goods.is_empty() {
            let kg: Vec<String> = ec
                .key_goods
                .iter()
                .map(|k| format!("{} ({:?})", k.good, k.role).to_lowercase())
                .collect();
            push(
                &mut o,
                &format!(
                    "  Key goods: {}. Trade balance {} sp.",
                    kg.join(", "),
                    ec.trade_balance_sp
                ),
            );
        }
        for f in &ss.factions {
            push(
                &mut o,
                &format!(
                    "  Faction: {} ({}, influence {}): {}",
                    f.name,
                    f.kind_label,
                    f.influence,
                    f.goals.join("; ")
                ),
            );
        }
        let mut rels: Vec<_> = ss.faction_relations.iter().collect();
        rels.sort_by_key(|r| match r.stance {
            FactionStance::Hostile => 0,
            FactionStance::Allied => 1,
            FactionStance::Rival => 2,
            FactionStance::Friendly => 3,
        });
        for r in rels.into_iter().take(6) {
            push(&mut o, &format!("    {:?}: {}", r.stance, r.reason));
        }
        let notables: Vec<String> = ss
            .roles
            .iter()
            .map(|r| {
                let stat = r
                    .stat_block
                    .clone()
                    .or_else(|| r.class.as_ref().map(|c| format!("{} {}", c.class, c.level)))
                    .unwrap_or_default();
                let at = r
                    .building
                    .and_then(|b| ss.buildings.iter().find(|x| x.id == b))
                    .map_or(String::new(), |b| format!(" @{}#{}", b.function, b.id));
                format!("{} [{stat}]{at}", r.title)
            })
            .collect();
        push(&mut o, &format!("  Notables: {}", notables.join("; ")));
        for l in &ss.history_hooks {
            push(&mut o, &format!("  ~ {}", l.text));
        }
        for hk in &ss.hooks {
            push(
                &mut o,
                &format!("  * {} [{}, {}] {}", hk.title, hk.kind, hk.tension, hk.text),
            );
        }
    }
    push(&mut o, "");
    push(&mut o, "== RUINS ==");
    for r in &h.ruins {
        push(
            &mut o,
            &format!(
                "  {} — a ruined {} near {}, lived in {}–{}; emptied by {}.",
                r.name,
                r.kind,
                name(r.near),
                r.founded,
                r.abandoned,
                r.cause.replace('_', " ")
            ),
        );
    }
    o
}
