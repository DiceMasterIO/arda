//! Relations are symmetric: one record per unordered realm pair and per
//! unordered faction pair, looked up the same in either order.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::politics::relations::relation;
use std::collections::BTreeSet;

#[test]
fn one_realm_relation_per_pair() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        let ids: Vec<u64> = s.realms.iter().map(|r| r.state.id).collect();
        let n = ids.len();
        assert_eq!(s.relations.len(), n * (n - 1) / 2);
        let mut seen = BTreeSet::new();
        for r in &s.relations {
            assert!(r.a < r.b);
            assert!(seen.insert((r.a, r.b)));
        }
        for &x in &ids {
            for &y in &ids {
                if x != y {
                    assert_eq!(relation(&s.relations, x, y), relation(&s.relations, y, x));
                    assert!(relation(&s.relations, x, y).is_some());
                }
            }
        }
    }
}

#[test]
fn war_stance_iff_ongoing_war() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for r in &s.relations {
            let ongoing = s
                .history
                .wars
                .iter()
                .any(|w| w.to.is_none() && w.realms == [r.a, r.b]);
            let at_war = r.stance == arda_society::politics::relations::Stance::War;
            assert_eq!(ongoing, at_war, "{} vs {}", r.a, r.b);
        }
    }
}

#[test]
fn faction_relations_are_unique_pairs_within_a_settlement() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for st in &s.settlements {
            let ids: BTreeSet<&str> = st.factions.iter().map(|f| f.id.as_str()).collect();
            let mut seen = BTreeSet::new();
            for r in &st.faction_relations {
                assert!(r.a < r.b, "ordered pair");
                assert!(ids.contains(r.a.as_str()) && ids.contains(r.b.as_str()));
                assert!(seen.insert((r.a.clone(), r.b.clone())), "duplicate pair");
            }
        }
    }
}
