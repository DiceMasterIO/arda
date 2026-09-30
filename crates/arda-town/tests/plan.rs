//! Plan invariants for the four synthetic sites: determinism, no overlaps,
//! reachability, frontage statistics, wall and gate continuity, the
//! building mix and `BuildingSpec` ids.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_town::function::{BuildingFunction as F, MixKey};
use arda_town::plan::{check, grid::Kind, spec, StreetClass};
use arda_town::samples;
use arda_town::site::Tier;
use common::{plan, plans, SEED};
use std::collections::BTreeMap;

#[test]
fn plans_are_deterministic() {
    for name in samples::NAMES {
        let (site, terrain) = samples::by_name(name).unwrap();
        let a = arda_town::generate(&site, &terrain, SEED).unwrap();
        let b = plan(name);
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap(), "{name}");
        assert_eq!(a.grid.kind, b.grid.kind, "{name} grid");
        assert_eq!(a.grid.building, b.grid.building, "{name} grid ids");
    }
}

#[test]
fn a_different_seed_gives_a_different_town() {
    let (site, terrain) = samples::by_name("thornby").unwrap();
    let other = arda_town::generate(&site, &terrain, SEED + 1).unwrap();
    assert_ne!(other.to_json().unwrap(), plan("thornby").to_json().unwrap());
}

#[test]
fn no_buildings_overlap() {
    for p in plans() {
        assert!(
            check::overlaps(p).is_empty(),
            "{}: {:?}",
            p.name,
            &check::overlaps(p)[..3.min(check::overlaps(p).len())]
        );
        // The grid agrees: every building square carries exactly its id.
        for b in &p.buildings {
            for y in b.rect.y0..b.rect.y1 {
                for x in b.rect.x0..b.rect.x1 {
                    let k = p.grid.gidx(x, y).unwrap();
                    assert_eq!(
                        u64::from(p.grid.building[k]),
                        b.id.0,
                        "{} {:?}",
                        p.name,
                        b.id
                    );
                }
            }
        }
    }
}

#[test]
fn every_building_is_reachable_from_a_street_through_a_door() {
    for p in plans() {
        assert!(
            check::unreachable(p).is_empty(),
            "{}: {:?}",
            p.name,
            check::unreachable(p)
        );
        for b in p.buildings.iter().filter(|b| b.function.walled()) {
            let d = b.doors.first().expect("walled buildings have a door");
            assert_eq!(
                d.side, b.front,
                "{} {:?}: first door faces the street",
                p.name, b.id
            );
            assert!(b.rect.contains(d.x, d.y));
            assert!(!b.rect.contains(d.outside().0, d.outside().1));
        }
    }
}

#[test]
fn plot_frontages_are_realistic() {
    for p in plans() {
        let f = check::frontages(p);
        assert!(f.len() >= 10, "{}: {} plots", p.name, f.len());
        let median = f[f.len() / 2];
        let (lo, hi) = match p.tier {
            Tier::City => (4.5, 8.5),
            Tier::Town => (5.5, 10.5),
            Tier::Village => (10.0, 22.0),
            Tier::Hamlet => (12.0, 28.0),
        };
        assert!(
            (lo..=hi).contains(&median),
            "{}: median frontage {median:.1} m",
            p.name
        );
        // Burgages are long and narrow: deeper than wide, in towns.
        if matches!(p.tier, Tier::Town | Tier::City) {
            let narrow = p
                .plots
                .iter()
                .filter(|pl| pl.depth_m > pl.frontage_m)
                .count();
            assert!(
                narrow * 10 >= p.plots.len() * 7,
                "{}: {narrow} of {} plots are deeper than wide",
                p.name,
                p.plots.len()
            );
        }
    }
}

#[test]
fn walled_towns_have_a_closed_wall_and_working_gates() {
    for p in plans() {
        if !p.tier.is_walled() {
            assert!(p.wall.is_none(), "{}", p.name);
            continue;
        }
        let wall = p.wall.as_ref().expect("walled tier");
        assert!(!check::escapes(p, false), "{}: the wall has a gap", p.name);
        assert!(check::escapes(p, true), "{}: the gates do not open", p.name);
        let road_gates = wall.gates.iter().filter(|g| g.street.is_some()).count();
        assert!(road_gates >= 2, "{}: {road_gates} gates", p.name);
        // Every gate carries a main street, and the passage is on the grid.
        for g in wall.gates.iter().filter(|g| g.street.is_some()) {
            let s = &p.streets[usize::from(g.street.unwrap().0)];
            assert_eq!(s.class, StreetClass::Main);
            let (i, j) = p.grid.cell_of(g.point);
            let near_gate =
                (-3..=3).any(|dy| (-3..=3).any(|dx| p.grid.kind_at(i + dx, j + dy) == Kind::Gate));
            assert!(near_gate, "{}: no passage at gate {:?}", p.name, g.point);
        }
    }
}

#[test]
fn the_building_mix_is_built() {
    for name in samples::NAMES {
        let (site, _) = samples::by_name(name).unwrap();
        let p = plan(name);
        let mut want: BTreeMap<F, u32> = BTreeMap::new();
        for (k, &n) in &site.buildings {
            if let MixKey::Building(f) = F::parse_mix(k) {
                *want.entry(f).or_default() += n;
            }
        }
        assert_eq!(check::built_mix(p), want, "{name}: notes {:?}", p.notes);
    }
}

#[test]
fn ids_are_one_based_and_match_building_specs() {
    for p in plans() {
        for (i, b) in p.buildings.iter().enumerate() {
            assert_eq!(b.id.0, i as u64 + 1);
            assert_eq!(p.building(b.id).map(|x| x.id), Some(b.id));
        }
        let specs = spec::specs(p);
        assert_eq!(specs.len(), p.buildings.len());
        for (s, b) in specs.iter().zip(&p.buildings) {
            assert_eq!(
                (s.id, s.function, s.settlement_id),
                (b.id, b.function, p.site)
            );
        }
        let homes: u32 = specs.iter().map(|s| u32::from(s.capacity)).sum();
        assert!(homes > 0, "{}", p.name);
    }
    let json = serde_json::to_string(&spec::specs(plan("saltwick"))[0]).unwrap();
    assert!(
        json.contains("\"id\":\"1\"") && json.contains("\"settlement_id\":\"404\""),
        "{json}"
    );
}

#[test]
fn functions_sit_where_they_belong() {
    let a = plan("aldermere");
    let temple = a
        .buildings
        .iter()
        .find(|b| b.function == F::Temple)
        .unwrap();
    let c = arda_town::plan::grid::SquareRect::polygon(&temple.rect);
    let d = arda_town::geom::centroid(&c).dist(a.focal.market);
    assert!(d < 120.0, "temple {d:.0} m from the market");
    let stalls = a.buildings.iter().filter(|b| b.function == F::Stall);
    for s in stalls {
        let k = a.grid.gidx(s.rect.x0, s.rect.y0).unwrap();
        assert!(matches!(
            a.grid.kind[k],
            Kind::Square | Kind::Green | Kind::Street
        ));
    }
    let h = plan("highcrag");
    let keep = h.buildings.iter().find(|b| b.function == F::Keep).unwrap();
    let ward = h.castle.as_ref().unwrap();
    assert!(ward.bailey.contains(keep.rect.x0, keep.rect.y0));
    let s = plan("saltwick");
    assert!(s.buildings.iter().any(|b| b.function == F::Boathouse));
}

#[test]
fn crofts_fill_the_footprint_and_stay_near_the_plots() {
    let anchor = |k: Kind| {
        matches!(
            k,
            Kind::Front
                | Kind::Yard
                | Kind::Garden
                | Kind::Building
                | Kind::Square
                | Kind::Green
                | Kind::Churchyard
                | Kind::Bailey
                | Kind::Wall
                | Kind::Gate
        )
    };
    for p in plans() {
        let g = &p.grid;
        let crofts: Vec<usize> = (0..g.kind.len())
            .filter(|&k| g.kind[k] == Kind::Croft)
            .collect();
        assert!(!crofts.is_empty(), "{}: no crofts", p.name);
        for &k in &crofts {
            let (i, j) = g.ij(k);
            let near = (-24..=24).any(|dy| (-24..=24).any(|dx| anchor(g.kind_at(i + dx, j + dy))));
            assert!(near, "{}: croft ({i}, {j}) far from any plot", p.name);
        }
        // A gap of a few squares between two plots along a row never stays
        // open country.
        for j in 0..g.h {
            for i in 0..g.w {
                if g.kind_at(i, j) != Kind::Open {
                    continue;
                }
                let hit = |d: i64| (1..=4).any(|n| anchor(g.kind_at(i + d * n, j)));
                assert!(!(hit(-1) && hit(1)), "{}: open gap at ({i}, {j})", p.name);
            }
        }
    }
}
