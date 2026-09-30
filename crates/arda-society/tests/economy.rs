//! Trade conserves goods: every load produced is used at home, exported or
//! stored; every load demanded is met at home, imported or short; exports
//! equal imports, and flows follow real roads.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

fn get(b: &BTreeMap<String, u64>, k: &str) -> u64 {
    b.get(k).copied().unwrap_or(0)
}

#[test]
fn every_load_is_accounted_for() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for g in &s.economy.goods {
            let k = g.good.as_str();
            let (mut ex, mut im) = (0, 0);
            for st in &s.settlements {
                let l = &st.economy.ledger;
                assert_eq!(
                    get(&l.production, k),
                    get(&l.local_use, k) + get(&l.exports, k) + get(&l.stored, k),
                    "{k} supply at {}",
                    st.name
                );
                assert_eq!(
                    get(&l.demand, k),
                    get(&l.local_use, k) + get(&l.imports, k) + get(&l.shortfall, k),
                    "{k} demand at {}",
                    st.name
                );
                ex += get(&l.exports, k);
                im += get(&l.imports, k);
            }
            assert_eq!(ex, im, "{k} exports vs imports");
            let flowed: u64 = s
                .economy
                .flows
                .iter()
                .filter(|f| f.good == k)
                .map(|f| f.amount)
                .sum();
            assert_eq!(flowed, ex, "{k} flows");
            assert_eq!(
                g.produced,
                g.local_use + g.traded + g.stored,
                "{k} world totals"
            );
        }
    }
}

#[test]
fn flows_follow_roads_and_stay_in_range() {
    let (world, s) = common::build(42);
    let roads: BTreeMap<u64, (u64, Option<u64>)> =
        world.roads.iter().map(|r| (r.id, (r.from, r.to))).collect();
    for f in &s.economy.flows {
        assert!(f.amount > 0 && f.from != f.to);
        assert_eq!(f.path.first(), Some(&f.from));
        assert_eq!(f.path.last(), Some(&f.to));
        assert_eq!(f.roads.len() + 1, f.path.len());
        for (k, r) in f.roads.iter().enumerate() {
            let (a, b) = roads[r];
            let hop: BTreeSet<u64> = [f.path[k], f.path[k + 1]].into();
            assert_eq!(hop, [a, b.unwrap()].into(), "flow hop on road {r}");
        }
        let range = world_range(&s, &f.good);
        assert!(
            f.distance_m <= range,
            "{} carried {} m",
            f.good,
            f.distance_m
        );
    }
}

fn world_range(_s: &arda_society::Society, good: &str) -> u64 {
    let t = arda_society::tables::Tables::load().unwrap();
    u64::from(t.good(good).unwrap().range_km) * 1000
}

#[test]
fn markets_name_key_goods_and_villages_feed_towns() {
    let (world, s) = common::build(42);
    for (w, st) in world.settlements.iter().zip(&s.settlements) {
        if st.economy.market {
            assert!(
                !st.economy.key_goods.is_empty(),
                "{} has no key goods",
                st.name
            );
        }
        if w.tier == arda_society::input::Tier::City {
            assert!(
                get(&st.economy.ledger.imports, "grain") > 0,
                "the city imports grain"
            );
        }
    }
}
