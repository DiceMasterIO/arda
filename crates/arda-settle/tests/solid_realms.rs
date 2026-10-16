//! Realms are territorially solid (logic/08 §realm-seats step 3,
//! Invariant 11): the partition flood crosses the sea at ferry cost, but a
//! landmass that is more than an islet is never ruled from across the sea.
//! Every realm keeps nine tenths of its land in its largest 4-connected
//! piece, or its other pieces are each islets (below a tenth of a fair
//! realm's land).
//!
//! The default-size check is ignored: generate a world, run `arda settle`
//! there and point `ARDA_SOLID_REALMS_WORLD` at it:
//! `ARDA_SOLID_REALMS_WORLD=<world> cargo test --release -p arda-settle
//! --test solid_realms -- --ignored`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::cost::CostSurface;
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::landmass;
use arda_settle::model::{Settlement, Tier};
use arda_settle::{realms, tags};

fn grass(g: &mut Grid, x0: i64, y0: i64, x1: i64, y1: i64) {
    for y in y0..y1 {
        for x in x0..x1 {
            let j = g.at(x, y).unwrap();
            g.terrain[j] = TerrainKind::Land;
            g.height_mm[j] = 50_000;
            g.temp_cc[j] = 1100;
            g.moisture[j] = 150;
            g.har_dm[j] = 40;
            g.slope_md[j] = 1000;
            g.cover[j] = Cover::Grass;
        }
    }
}

fn villages(s: &mut Vec<Settlement>, x0: u32, y0: u32, x1: u32, y1: u32) {
    for y in (y0 + 3..y1).step_by(6) {
        for x in (x0 + 3..x1).step_by(6) {
            let id = u64::try_from(s.len() + 1).unwrap();
            s.push(Settlement::bare(id, x, y, Tier::Village, 300));
        }
    }
}

/// A mainland strip 240 × 60 cells with two seats in its west, a large
/// island (80 × 30 cells from row `top`) with a town at column `town_x`,
/// and a 5 × 5 islet two cells off the coast, east of the island.
fn world(town_x: u32, top: u32) -> (Grid, Vec<Settlement>, usize) {
    let mut g = Grid::sea(240, 120, MEMORY_BUDGET).unwrap();
    grass(&mut g, 0, 0, 240, 60);
    grass(&mut g, 80, i64::from(top), 160, i64::from(top) + 30);
    grass(&mut g, 200, 62, 205, 67);
    let mut s = vec![
        Settlement::bare(1, 20, 30, Tier::Town, 5000),
        Settlement::bare(2, 70, 30, Tier::Town, 5000),
    ];
    villages(&mut s, 0, 0, 240, 60);
    let island = s.len();
    let id = u64::try_from(island + 1).unwrap();
    s.push(Settlement::bare(id, town_x, top + 15, Tier::Town, 1500));
    villages(&mut s, 80, top, 160, top + 30);
    (g, s, island)
}

#[test]
fn a_large_island_gets_its_own_realm_and_islets_join_the_coast() {
    let (g, s, island) = world(120, 75);
    let sites = tags::compute(&g, 1).unwrap();
    let cs = CostSurface::terrain(&g, &sites, 1).unwrap();
    let (seats, t) = realms::settle(&g, &sites, &cs, vec![0, 1], &s, &s).unwrap();
    assert!(
        seats.contains(&island),
        "the island town is a seat: {seats:?}"
    );
    let broken = landmass::broken(g.width, g.height, &t.map, seats.len());
    assert!(broken.is_empty(), "realms {broken:?} are not solid");
    // The island is one realm, the island town's.
    let own = t.map[s[island].index(g.width)];
    let island_cells = (0..g.len())
        .filter(|&i| {
            let (x, y) = g.xy(i);
            (80..160).contains(&x) && (75..105).contains(&y)
        })
        .collect::<Vec<_>>();
    assert!(island_cells.iter().all(|&i| t.map[i] == own));
    // The islet is still governed, from the mainland.
    let islet = t.map[g.at(202, 64).unwrap()];
    assert!(islet != 0 && islet != own, "islet realm {islet}");
}

#[test]
fn no_seat_floods_onto_a_landmass_with_a_seat_of_its_own() {
    // The island town is a seat from the start; the mainland seats must
    // not take any of its island, even where its town stands at the far
    // end and a mainland seat is nearer the island's western shore.
    let (g, s, island) = world(158, 62);
    let sites = tags::compute(&g, 1).unwrap();
    let cs = CostSurface::terrain(&g, &sites, 1).unwrap();
    let t = realms::territory(&g, &sites, &cs, &[0, 1, island], &s).unwrap();
    let own = t.map[s[island].index(g.width)];
    let foreign = (0..g.len())
        .filter(|&i| {
            let (x, y) = g.xy(i);
            (80..160).contains(&x) && (62..92).contains(&y) && t.map[i] != own
        })
        .count();
    assert_eq!(foreign, 0);
}

#[test]
#[ignore = "reads a default-size world named by ARDA_SOLID_REALMS_WORLD"]
fn default_size_realms_are_solid() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("ARDA_SOLID_REALMS_WORLD").expect("ARDA_SOLID_REALMS_WORLD"),
    );
    let (w, h, map) = arda_settle::output::read_realm_map(&dir.join("society/realms.bin")).unwrap();
    let n = usize::from(map.iter().copied().max().unwrap_or(0));
    let all = landmass::pieces(w, h, &map, n);
    let land: u64 = all.iter().flatten().sum();
    let islet = landmass::islet_cells(land, n);
    eprintln!("{n} realms, {land} land cells, islet below {islet} cells");
    for (r, p) in all.iter().enumerate() {
        let total: u64 = p.iter().sum();
        let first = p.first().copied().unwrap_or(0);
        eprintln!(
            "realm {}: {total} cells, {} pieces, largest {:.1} %, next {:?}",
            r + 1,
            p.len(),
            first as f64 * 100.0 / total.max(1) as f64,
            &p[1..p.len().min(4)]
        );
    }
    let broken = landmass::broken(w, h, &map, n);
    assert!(broken.is_empty(), "realms {broken:?} are not solid");
}
