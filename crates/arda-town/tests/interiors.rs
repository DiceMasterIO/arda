//! Goal 64 on the synthetic sample towns: varied home interiors (no two
//! adjacent homes alike, few repeats at all), each with a fire, a bed and
//! clutter, and clutter standing against walls or other furniture.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_town::block::interior::{canon, variety, Canon, Want};
use common::plans;

/// Whether prop `i` touches a wall edge or another prop.
fn supported(c: &Canon, i: usize) -> bool {
    let p = &c.props[i];
    let wall = |x: i64, y: i64, hz: bool| c.walls.contains_key(&(x, y, hz));
    let on_wall = (p.x0..p.x1).any(|x| wall(x, p.y0, true) || wall(x, p.y1, true))
        || (p.y0..p.y1).any(|y| wall(p.x0, y, false) || wall(p.x1, y, false));
    let touches = c
        .props
        .iter()
        .enumerate()
        .any(|(j, q)| j != i && p.x0 <= q.x1 && q.x0 <= p.x1 && p.y0 <= q.y1 && q.y0 <= p.y1);
    on_wall || touches
}

#[test]
fn home_interiors_vary_and_adjacent_homes_differ() {
    for p in plans() {
        let d = variety::diversity(p);
        println!("{}: {d:?}", p.name);
        assert_eq!(d.adjacent_equal, 0, "{}: adjacent homes alike", p.name);
        if d.homes >= 20 {
            assert!(
                d.repeated_fraction() < 0.05,
                "{}: {:.1} % repeated",
                p.name,
                100.0 * d.repeated_fraction()
            );
        }
    }
}

#[test]
fn wfc_home_interiors_vary_and_adjacent_homes_differ() {
    for p in plans() {
        let d = arda_town::block::wfc::indoor::diversity(p);
        println!("{} (wfc): {d:?}", p.name);
        assert_eq!(d.adjacent_equal, 0, "{}: adjacent WFC homes alike", p.name);
        if d.homes >= 20 {
            assert!(
                d.repeated_fraction() < 0.05,
                "{}: {:.1} % of WFC homes repeated",
                p.name,
                100.0 * d.repeated_fraction()
            );
        }
    }
}

#[test]
fn homes_have_a_fire_a_bed_and_clutter_against_walls() {
    const AGAINST: [&str; 8] = [
        "prop.barrel",
        "prop.sacks",
        "prop.crate",
        "prop.chest",
        "prop.shelf",
        "prop.cupboard",
        "prop.bed",
        "prop.hearth",
    ];
    let mut homes = 0;
    for p in plans() {
        for b in p.buildings.iter().filter(|b| variety::is_home(b.function)) {
            let (c, _) = canon(p, b, p.interior_salt(b));
            let ids: Vec<&str> = c
                .props
                .iter()
                .filter_map(|q| match q.want {
                    Want::Id(id) => Some(id),
                    Want::Query(_) => None,
                })
                .collect();
            let fire = ids
                .iter()
                .any(|id| matches!(*id, "prop.hearth" | "prop.brazier" | "prop.oven"));
            assert!(fire, "{} home {:?} has no fire: {ids:?}", p.name, b.id);
            assert!(
                ids.contains(&"prop.bed"),
                "{} home {:?}: {ids:?}",
                p.name,
                b.id
            );
            assert!(
                ids.len() >= 4,
                "{} home {:?} is bare: {ids:?}",
                p.name,
                b.id
            );
            for (i, q) in c.props.iter().enumerate() {
                if let Want::Id(id) = q.want {
                    if AGAINST.contains(&id) {
                        assert!(supported(&c, i), "{} home {:?}: {id} floats", p.name, b.id);
                    }
                }
            }
            homes += 1;
        }
    }
    assert!(homes > 50, "{homes} homes");
}

#[test]
fn interior_salts_are_stable() {
    for p in plans() {
        let again = variety::salts(p);
        for (i, b) in p.buildings.iter().enumerate() {
            assert_eq!(p.interior_salt(b), again[i]);
        }
    }
}
