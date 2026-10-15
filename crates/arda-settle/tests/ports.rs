//! Port tags are real tests, not proxies (review 3.3-1; logic/08
//! §settle-sites): a reach is navigable by its channel's width and depth,
//! a harbour is a low, gentle landing on a sheltered deep bay, and neither
//! tag climbs onto ground far above the water it names.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::tags::{self, HARBOUR, NAVIGABLE};

fn land(g: &mut Grid, i: usize, h_mm: i32) {
    g.terrain[i] = TerrainKind::Land;
    g.cover[i] = Cover::Grass;
    g.height_mm[i] = h_mm;
    g.temp_cc[i] = 1100;
    g.moisture[i] = 150;
    g.har_dm[i] = 50;
}

/// Flat land 60 × 40 at 20 m with a channel along row 20 of the given width
/// (decimetres) and discharge (thousandths of m³/s), Strahler order 5.
fn river(width_dm: u32, discharge_milli: u32) -> Grid {
    let mut g = Grid::sea(60, 40, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        land(&mut g, i, 20_000);
        let (_, y) = g.xy(i);
        if y == 20 {
            g.order[i] = 5;
            g.width_dm[i] = width_dm;
            g.discharge_milli[i] = discharge_milli;
            g.drainage[i] = 50_000;
            g.height_mm[i] = 18_000;
            g.har_dm[i] = 0;
        }
    }
    g
}

#[test]
fn a_wadeable_fifth_order_stream_is_not_navigable() {
    // 8 m wide, 4 m³/s: fordable, whatever its Strahler order.
    let g = river(80, 4_000);
    let sites = tags::compute(&g, 42).unwrap();
    assert!(
        sites.tags.iter().all(|t| t & NAVIGABLE == 0),
        "an 8 m stream is not navigable"
    );
}

#[test]
fn a_wide_deep_river_is_navigable_only_on_its_low_banks() {
    // 40 m wide, 100 m³/s (about 1.6 m deep).
    let mut g = river(400, 100_000);
    let bank = g.at(30, 21).unwrap();
    // A bluff two cells off the river, 30 m above it.
    let bluff = g.at(30, 23).unwrap();
    for x in 0..60 {
        for y in 23..40 {
            let i = g.at(x, y).unwrap();
            g.height_mm[i] = 48_000;
        }
    }
    let sites = tags::compute(&g, 42).unwrap();
    assert!(
        sites.tags[bank] & NAVIGABLE != 0,
        "the bank beside the river"
    );
    assert!(
        sites.tags[bluff] & NAVIGABLE == 0,
        "30 m above the river is not a quay"
    );
}

/// A sheltered inlet: open sea in the west five columns, an inlet 5 cells
/// wide and 5 m deep running east to x = 30; land elsewhere at `shore_mm`.
fn inlet(shore_mm: i32, slope_md: u16) -> (Grid, usize) {
    let mut g = Grid::sea(60, 45, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        let (x, y) = g.xy(i);
        let sea = x < 5 || ((20..25).contains(&y) && x <= 30);
        if sea {
            g.height_mm[i] = -5_000;
        } else {
            land(&mut g, i, shore_mm);
            g.slope_md[i] = slope_md;
        }
    }
    let head = g.at(31, 22).unwrap();
    (g, head)
}

#[test]
fn a_low_landing_on_a_sheltered_deep_inlet_is_a_harbour() {
    let (g, head) = inlet(2_000, 2_000);
    assert!(g.is_coast(head));
    let sites = tags::compute(&g, 42).unwrap();
    assert!(
        sites.tags[head] & HARBOUR != 0,
        "the inlet head is a harbour"
    );
}

#[test]
fn a_cliff_above_a_sheltered_deep_inlet_is_not_a_harbour() {
    let (g, head) = inlet(40_000, 2_000);
    let sites = tags::compute(&g, 42).unwrap();
    assert!(
        sites.tags.iter().all(|t| t & HARBOUR == 0),
        "no landing 40 m above the water"
    );
    let _ = head;
    let (g, _) = inlet(2_000, 20_000);
    let sites = tags::compute(&g, 42).unwrap();
    assert!(
        sites.tags.iter().all(|t| t & HARBOUR == 0),
        "no landing on a 20° shore"
    );
}

#[test]
fn the_harbour_tag_stays_on_ground_near_the_water_level() {
    let (mut g, head) = inlet(2_000, 2_000);
    // Two cells inland the ground rises 25 m.
    let (hx, hy) = g.xy(head);
    let high = g.at(hx + 2, hy).unwrap();
    for x in (hx + 2)..60 {
        for y in 0..45 {
            let i = g.at(x, y).unwrap();
            if g.is_land(i) {
                g.height_mm[i] = 27_000;
            }
        }
    }
    let sites = tags::compute(&g, 42).unwrap();
    assert!(sites.tags[head] & HARBOUR != 0);
    assert!(sites.tags[high] & HARBOUR == 0, "25 m above the quay");
}
