//! A tiled world many times the synthetic one still simulates, keeps every
//! invariant the small world has, and stores O(settlements) records.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_society::input::{Road, RoadClass, WorldSettlements};
use arda_society::{simulate_society, synthetic};

/// `tiles` copies of the synthetic world side by side, seats joined by highways.
pub fn tiled(tiles: u64) -> WorldSettlements {
    let base = synthetic::world(5).unwrap();
    let mut w = WorldSettlements::default();
    for k in 0..tiles {
        let (dx, sid, rid, road) = (
            i64::try_from(k).unwrap() * 170_000,
            k * 1000,
            k * 10,
            u32::try_from(k).unwrap() * 1000,
        );
        for s in &base.settlements {
            let mut s = s.clone();
            s.id += sid;
            s.realm_id += rid;
            s.x_m += dx;
            w.settlements.push(s);
        }
        for r in &base.roads {
            let mut r = r.clone();
            r.id += u64::from(road);
            r.from += sid;
            r.to = r.to.map(|t| t + sid);
            w.roads.push(r);
        }
        for r in &base.realms {
            let mut r = r.clone();
            r.id += rid;
            r.seat += sid;
            r.members = r.members.iter().map(|m| m + sid).collect();
            w.realms.push(r);
        }
        if k > 0 {
            w.roads.push(Road {
                id: 900_000 + k,
                class: RoadClass::Highway,
                from: (k - 1) * 1000 + 3,
                to: Some(sid + 1),
                to_edge: None,
                length_m: 60_000,
                straight_m: 50_000,
                new_m: 60_000,
                segments: Vec::new(),
            });
        }
    }
    w
}

#[test]
fn tiled_world_keeps_invariants() {
    let world = tiled(8);
    let s = simulate_society(11, &world).unwrap();
    assert_eq!(s.settlements.len(), 320);
    assert_eq!(s.realms.len(), 24);
    assert_eq!(s.relations.len(), 24 * 23 / 2);
    let replay = s.history.replay_partition().unwrap();
    assert!(world
        .settlements
        .iter()
        .all(|x| replay[&x.id] == x.realm_id));
    assert!(s
        .settlements
        .iter()
        .all(|x| (3..=5).contains(&x.hooks.len())));
    let roles: usize = s.settlements.iter().map(|x| x.roles.len()).sum();
    assert!(roles < 320 * 20, "stored notables stay O(settlements)");
}

/// `cargo test --release -p arda-society --test scale -- --ignored --nocapture`
/// (`ARDA_SOCIETY_TILES` sets the tile count, 40 settlements each).
#[test]
#[ignore = "timing run on a 4,000-settlement world"]
fn large_world_timing() {
    let tiles = std::env::var("ARDA_SOCIETY_TILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let world = tiled(tiles);
    let t = std::time::Instant::now();
    let s = simulate_society(3, &world).unwrap();
    let json = arda_society::output::to_json(&s).unwrap();
    let digest = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        json.hash(&mut h);
        h.finish()
    };
    println!(
        "{} settlements, {} realms, {} events, {} flows: {:?}, {} MB JSON, digest {digest:016x}",
        s.settlements.len(),
        s.realms.len(),
        s.history.events.len(),
        s.economy.flows.len(),
        t.elapsed(),
        json.len() / 1_000_000
    );
}

/// Review round 2 #29: per-settlement and per-pair scans made the
/// simulation quadratic (8,000 settlements took 4.5× as long as 4,000).
/// Four times the settlements must now cost well under sixteen times as
/// much. Release gate:
/// `cargo test --release -p arda-society --test scale -- --ignored`.
#[test]
#[ignore = "timing run on 2,000 and 8,000 settlements"]
fn society_time_grows_near_linearly() {
    let best = |tiles: u64| {
        let world = tiled(tiles);
        (0..2)
            .map(|_| {
                let t = std::time::Instant::now();
                simulate_society(3, &world).unwrap();
                t.elapsed()
            })
            .min()
            .unwrap()
    };
    let (small, large) = (best(50), best(200));
    let ratio = large.as_secs_f64() / small.as_secs_f64();
    println!("2,000: {small:?}, 8,000: {large:?}, ratio {ratio:.1}");
    assert!(ratio < 9.0, "4× the settlements took {ratio:.1}× as long");
}
