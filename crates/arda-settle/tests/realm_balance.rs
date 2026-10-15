//! The realm partition balances the realms' people and land (logic/08
//! §realm-seats step 3, Invariants 5a; OI-53): seats are towns and cannot
//! stand where an even split would put them, so the territory gives each
//! seat a head start (or a handicap) on the cost flood until the shares
//! are within bounds.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::cost::CostSurface;
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::model::{Settlement, Tier};
use arda_settle::{realms, tags};

/// A flat strip of grassland 240 × 60 cells, a village every 6 cells.
fn strip() -> (Grid, Vec<Settlement>) {
    let mut g = Grid::sea(240, 60, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        g.terrain[i] = TerrainKind::Land;
        g.height_mm[i] = 50_000;
        g.temp_cc[i] = 1100;
        g.moisture[i] = 150;
        g.har_dm[i] = 40;
        g.slope_md[i] = 1000;
        g.cover[i] = Cover::Grass;
    }
    let mut s = vec![
        Settlement::bare(1, 20, 30, Tier::Town, 5000),
        Settlement::bare(2, 70, 30, Tier::Town, 5000),
    ];
    for y in (3..60).step_by(6) {
        for x in (3..240).step_by(6) {
            let id = u64::try_from(s.len() + 1).unwrap();
            s.push(Settlement::bare(id, x, y, Tier::Village, 300));
        }
    }
    (g, s)
}

fn shares(map: &[u16], g: &Grid, s: &[Settlement]) -> ([u64; 2], [u64; 2]) {
    let (mut land, mut people) = ([0_u64; 2], [0_u64; 2]);
    for &m in map.iter().filter(|&&m| m > 0) {
        land[usize::from(m) - 1] += 1;
    }
    for x in s {
        let m = map[x.index(g.width)];
        people[usize::from(m) - 1] += u64::from(x.population);
    }
    (land, people)
}

#[test]
fn seats_bunched_at_one_end_still_share_the_land() {
    // The plain flood puts the border midway between the seats, at x = 45:
    // the eastern realm holds four times the western one's land and people.
    let (g, s) = strip();
    let sites = tags::compute(&g, 1).unwrap();
    let cs = CostSurface::terrain(&g, &sites, 1).unwrap();
    let t = realms::territory(&g, &sites, &cs, &[0, 1], &s).unwrap();
    let (land, people) = shares(&t.map, &g, &s);
    let ratio = |v: [u64; 2]| v[0].max(v[1]) as f64 / v[0].min(v[1]).max(1) as f64;
    assert!(ratio(people) <= 2.5, "people {people:?}");
    assert!(ratio(land) <= 4.5, "land {land:?}");
    assert_eq!(t.map[s[0].index(g.width)], 1, "each seat keeps its cell");
    assert_eq!(t.map[s[1].index(g.width)], 2);
}
