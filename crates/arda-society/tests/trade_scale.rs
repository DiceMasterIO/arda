//! Trade at world scale (review round 2, #28): 5,000 settlements on a
//! 500 × 1,000 km road lattice (about 98,500 km of road, a full world's
//! extent). The per-supplier reach is bounded by `MARKETS`, so memory is
//! O(settlements), not O(settlements × network within 300 km).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_society::ctx::Ctx;
use arda_society::economy::{self, trade::MARKETS};
use arda_society::input::{Realm, Road, RoadClass, Settlement, Tier, WorldSettlements};
use arda_society::tables::Tables;

const WIDE: u64 = 50;
const HIGH: u64 = 100;
const SPACING_M: i64 = 10_000;

fn lattice() -> WorldSettlements {
    let base = arda_society::synthetic::world(5).unwrap();
    let template: &Settlement = base
        .settlements
        .iter()
        .find(|s| s.tier == Tier::Village)
        .unwrap();
    let id = |x: u64, y: u64| y * WIDE + x + 1;
    let mut w = WorldSettlements::default();
    for y in 0..HIGH {
        for x in 0..WIDE {
            let mut s = template.clone();
            s.id = id(x, y);
            s.name = format!("Place {}", s.id);
            s.realm_id = 1 + (y / 10) * 5 + x / 10;
            s.x_m = i64::try_from(x).unwrap() * SPACING_M + 5_000;
            s.y_m = i64::try_from(y).unwrap() * SPACING_M + 5_000;
            // Every tenth place is a market town, so goods move.
            if (x + y) % 10 == 0 {
                s.tier = Tier::Town;
                s.population = 2_000;
            }
            w.settlements.push(s);
        }
    }
    let mut road = 0;
    for y in 0..HIGH {
        for x in 0..WIDE {
            for (dx, dy) in [(1, 0), (0, 1)] {
                if x + dx < WIDE && y + dy < HIGH {
                    road += 1;
                    w.roads.push(Road {
                        id: road,
                        class: RoadClass::Road,
                        from: id(x, y),
                        to: Some(id(x + dx, y + dy)),
                        to_edge: None,
                        length_m: 10_000,
                        straight_m: 10_000,
                        new_m: 10_000,
                        segments: Vec::new(),
                    });
                }
            }
        }
    }
    for r in 0..50 {
        let (rx, ry) = (r % 5, r / 5);
        w.realms.push(Realm {
            id: r + 1,
            name: format!("Realm {}", r + 1),
            seat: id(rx * 10, ry * 10),
            members: Vec::new(),
        });
    }
    w
}

#[test]
fn trade_reach_is_bounded_on_a_full_world_road_network() {
    let world = lattice();
    assert_eq!(world.settlements.len(), 5_000);
    let km: u64 = world.roads.iter().map(|r| r.length_m).sum::<u64>() / 1000;
    assert!(km >= 98_000, "{km} km of road");
    let tables = Tables::load().unwrap();
    let ctx = Ctx::new(7, &world, &tables).unwrap();
    // The longest good range covers hundreds of lattice nodes; the reach
    // keeps only the nearest markets.
    let entries: usize = (0..ctx.n())
        .map(|i| {
            let r = ctx.graph.reach_nearest(i, 300_000, MARKETS);
            assert!(r.dist.len() <= MARKETS + 1 && r.prev.len() <= MARKETS);
            r.dist.len()
        })
        .sum();
    assert!(entries <= 5_000 * (MARKETS + 1), "{entries}");
    let start = std::time::Instant::now();
    let run = economy::run(&ctx);
    let elapsed = start.elapsed();
    assert!(!run.economy.flows.is_empty());
    assert!(
        elapsed.as_secs() < 60,
        "trade over 5,000 settlements took {elapsed:?}"
    );
}
