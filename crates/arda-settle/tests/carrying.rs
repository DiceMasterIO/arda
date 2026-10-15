//! The configured density is people per km² of all land, on average over
//! the world (logic/01 "mean density"); where they live follows the
//! land's carrying capacity and the sites' resources (review 3.3-13;
//! logic/08 §settle-tiers, §settle-placement).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::central::town_score;
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::{place, suitability, tags, PlaceParams};

/// 80 × 80 cells of land; `rock(x, y)` marks bare rock, the rest is warm,
/// moist, gentle grass with a stream down the middle.
fn world(rock: impl Fn(i64, i64) -> bool) -> Grid {
    let mut g = Grid::sea(80, 80, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        let (x, y) = g.xy(i);
        g.terrain[i] = TerrainKind::Land;
        g.height_mm[i] = 50_000;
        g.temp_cc[i] = 1100;
        g.moisture[i] = 150;
        g.har_dm[i] = 40;
        g.slope_md[i] = 2000;
        g.cover[i] = if rock(x, y) {
            Cover::Rock
        } else {
            Cover::Grass
        };
        if x == 40 {
            g.order[i] = 2;
            g.width_dm[i] = 40;
            g.har_dm[i] = 0;
            g.cover[i] = Cover::Grass;
        }
    }
    g
}

fn placement(g: &Grid) -> place::Placement {
    let sites = tags::compute(g, 1).unwrap();
    let suit = suitability::compute(g, &sites).unwrap();
    let p = PlaceParams {
        seed: 1,
        density_per_km2: 15,
    };
    place::place(g, &sites, &suit, p).unwrap()
}

#[test]
fn the_density_is_people_per_km2_of_all_land() {
    // 6,400 ha = 64 km² at 15 people/km²: 960 people, fertile or not.
    let farm = world(|_, _| false);
    let rocky = world(|x, _| !(30..=50).contains(&x));
    for g in [&farm, &rocky] {
        assert_eq!(placement(g).target_population, 960);
    }
    // On the rocky land they crowd onto the strip of farmland.
    let p = placement(&rocky);
    assert!(!p.settlements.is_empty());
    assert!(p.settlements.iter().all(|s| (30..=50).contains(&s.cell_x)));
}

#[test]
fn ore_and_salt_draw_towns_and_passes_and_heights_enlarge_them() {
    let base = town_score(600, 300, 0, 0);
    for bit in [tags::ORE, tags::SALT] {
        assert!(
            town_score(600, 300, bit, 0) >= base + 50,
            "tag {bit:#x} adds little to a town site"
        );
    }
    for bit in [tags::PASS, tags::DEFENSIBLE, tags::ORE, tags::SALT] {
        assert!(
            arda_settle::townsize::advantage_pm(bit) >= 1150,
            "tag {bit:#x} adds little to a town's trade"
        );
    }
}
