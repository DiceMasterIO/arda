//! Timeline causality: nobody acts before they exist, and the simulated
//! past ends in exactly the given present (realm partition and seats).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::history::EventKind;
use arda_society::input::Tier;
use std::collections::BTreeMap;

#[test]
fn span_is_two_to_five_centuries() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        assert!(
            (200..=500).contains(&s.history.present_year),
            "{}",
            s.history.present_year
        );
    }
}

#[test]
fn no_settlement_acts_before_it_is_founded() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        let founded: BTreeMap<u64, i32> = s.settlements.iter().map(|x| (x.id, x.founded)).collect();
        let formed: BTreeMap<u64, i32> = s
            .realms
            .iter()
            .map(|r| (r.state.id, r.state.founded))
            .collect();
        let h = &s.history;
        let mut last = (i32::MIN, 0);
        for e in &h.events {
            assert!(
                e.year >= 1 && e.year <= h.present_year,
                "event {} in year {}",
                e.id,
                e.year
            );
            assert!((e.year, e.id) > last, "events in chronological id order");
            last = (e.year, e.id);
            for sid in &e.settlements {
                assert!(
                    founded[sid] <= e.year,
                    "{} involves settlement {sid} before its founding",
                    e.title
                );
            }
            if !matches!(
                e.kind,
                EventKind::Founding
                    | EventKind::Flood
                    | EventKind::Plague
                    | EventKind::Fire
                    | EventKind::MineCollapse
                    | EventKind::Abandonment
            ) {
                for rid in &e.realms {
                    assert!(
                        formed[rid] <= e.year,
                        "{} involves realm {rid} before it formed",
                        e.title
                    );
                }
            }
            if let Some(c) = e.cause {
                let cause = h.events.iter().find(|x| x.id == c).unwrap();
                assert!(cause.year <= e.year, "cause after effect: {}", e.title);
            }
            if let Some(end) = e.end_year {
                assert!(end >= e.year);
            }
        }
        for st in &s.settlements {
            if let Some(p) = st.founded_from {
                assert!(
                    founded[&p] <= st.founded,
                    "{} founded before its parent",
                    st.name
                );
            }
        }
    }
}

#[test]
fn history_ends_in_the_given_partition_and_seats() {
    for seed in common::SEEDS {
        let (world, s) = common::build(seed);
        let replayed = s
            .history
            .replay_partition()
            .expect("consistent border shifts");
        for w in &world.settlements {
            assert_eq!(
                replayed[&w.id], w.realm_id,
                "{} ends in its given realm",
                w.name
            );
        }
        for r in &world.realms {
            let st = s.realms.iter().find(|x| x.state.id == r.id).unwrap();
            assert_eq!(st.state.seat, r.seat, "seat unchanged");
            let mut members: Vec<u64> = world
                .settlements
                .iter()
                .filter(|x| x.realm_id == r.id)
                .map(|x| x.id)
                .collect();
            members.sort_unstable();
            let mut got = st.state.members.clone();
            got.sort_unstable();
            assert_eq!(got, members, "members unchanged");
            assert!(
                s.history
                    .border_shifts
                    .iter()
                    .all(|b| b.settlement != r.seat),
                "seats never change hands"
            );
        }
        for b in &s.history.border_shifts {
            let st = s.settlements.iter().find(|x| x.id == b.settlement).unwrap();
            assert!(st.founded <= b.year);
            let war = s
                .history
                .wars
                .iter()
                .find(|w| w.event == b.war_event)
                .unwrap();
            assert_eq!(war.to, Some(b.year));
            assert_eq!(war.winner, Some(b.to_realm));
        }
    }
}

#[test]
fn reigns_cover_each_realm_without_gaps() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for r in &s.realms {
            let reigns: Vec<_> = s
                .history
                .reigns
                .iter()
                .filter(|x| x.realm_id == r.state.id)
                .collect();
            assert_eq!(reigns.first().unwrap().from, r.state.founded);
            for pair in reigns.windows(2) {
                assert_eq!(pair[0].to, Some(pair[1].from), "contiguous reigns");
            }
            let last = reigns.last().unwrap();
            assert!(last.to.is_none());
            assert_eq!(last.regnal, r.state.ruler.regnal);
            assert!(s
                .history
                .dynasties
                .iter()
                .any(|d| d.id == last.dynasty_id && d.to.is_none()));
        }
    }
}

#[test]
fn ruins_are_founded_before_abandoned_by_a_prior_cause() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        assert!(!s.history.ruins.is_empty());
        for r in &s.history.ruins {
            assert!(r.founded < r.abandoned && r.abandoned < s.history.present_year);
            if let Some(c) = r.cause_event {
                let e = s.history.events.iter().find(|e| e.id == c).unwrap();
                assert!(e.year <= r.abandoned && e.year > r.founded);
            }
        }
    }
}

#[test]
fn best_sites_are_settled_first() {
    let mut by_tier: BTreeMap<Tier, (i64, i64)> = BTreeMap::new();
    for seed in common::SEEDS {
        let (world, s) = common::build(seed);
        for (w, st) in world.settlements.iter().zip(&s.settlements) {
            let e = by_tier.entry(w.tier).or_default();
            e.0 += i64::from(st.founded);
            e.1 += 1;
        }
    }
    let mean = |t: Tier| by_tier[&t].0 / by_tier[&t].1;
    assert!(mean(Tier::City) < mean(Tier::Town));
    assert!(mean(Tier::Town) < mean(Tier::Village));
    assert!(mean(Tier::Village) < mean(Tier::Hamlet));
}

#[test]
fn prosperity_ends_at_present_wealth() {
    let (world, s) = common::build(42);
    for (w, st) in world.settlements.iter().zip(&s.settlements) {
        let last = st.prosperity.samples.last().unwrap();
        assert_eq!(last[0], s.history.present_year);
        assert_eq!(last[1], i32::from(w.wealth));
        assert_eq!(st.prosperity.samples[0][0], st.founded);
    }
}
