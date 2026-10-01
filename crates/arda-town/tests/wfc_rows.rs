//! Ordered furniture in WFC interiors (logic/10 §town-wfc) on the synthetic
//! sample plans: pews in a nave stand in aligned rows with a clear central
//! aisle, and dormitory beds line the long walls. Warehouses and libraries
//! are checked on the seed-42 city (`arda-people/tests/interiors.rs`).
//! Art-free vocabulary stays out of town layouts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use arda_town::block::wfc::indoor::{self, Plan};
use arda_town::block::wfc::rows::{laid_out as anchors, lined_up};
use arda_town::BuildingFunction as F;
use common::plans;
use std::collections::{BTreeMap, BTreeSet};

/// Canonical width: the room rectangles span it.
fn width(p: &Plan<'_>) -> i64 {
    p.layout.rooms.iter().map(|r| r.rect[2]).max().unwrap_or(0)
}

fn walkable(p: &Plan<'_>, x: i64, y: i64) -> bool {
    let (v, f) = &p.furnished;
    let cols = width(p);
    v.walkable(usize::from(f.tiles[(y * cols + x) as usize]))
}

fn solved(f: F) -> Vec<Plan<'static>> {
    plans()
        .iter()
        .flat_map(|p| p.buildings.iter().map(move |b| (p, b)))
        .filter(|(_, b)| b.function == f)
        .filter_map(|(p, b)| indoor::solve(p, b).ok().flatten())
        .collect()
}

#[test]
fn nave_pews_form_aligned_rows_with_a_clear_central_aisle() {
    let mut naves = 0;
    for p in solved(F::Temple) {
        for (rect, pews) in anchors(&p, "nave", "prop.pew") {
            let [x0, y0, x1, _] = rect;
            if x1 - x0 < 5 || pews.is_empty() {
                continue;
            }
            let mut rows: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
            for &(x, y, _) in &pews {
                rows.entry(y).or_default().insert(x);
            }
            let id = p.b.id.0;
            assert!(rows.len() >= 2, "temple {id}: {} pew rows", rows.len());
            let first = rows.values().next().unwrap().clone();
            for (y, xs) in &rows {
                assert_eq!(xs, &first, "temple {id}: row {y} is out of line");
            }
            let ys: Vec<i64> = rows.keys().copied().collect();
            for pair in ys.windows(2) {
                assert!(pair[1] - pair[0] >= 2, "temple {id}: no legroom {ys:?}");
            }
            // Pews on both sides of the aisle, and the aisle clear.
            let c = x0 + (x1 - x0 - 1) / 2;
            let aisle: Vec<i64> = if (x1 - x0) % 2 == 0 {
                vec![c, c + 1]
            } else {
                vec![c]
            };
            assert!(first.iter().any(|&x| x < aisle[0]), "temple {id}: left");
            assert!(first.iter().any(|&x| x > aisle[0]), "temple {id}: right");
            for y in y0..=*ys.last().unwrap() {
                for &x in &aisle {
                    assert!(walkable(&p, x, y), "temple {id}: aisle blocked at {x},{y}");
                }
            }
            naves += 1;
        }
    }
    assert!(naves >= 2, "{naves} naves with pews");
}

#[test]
fn dormitory_beds_line_the_long_walls() {
    let mut rooms = 0;
    for p in solved(F::Barracks) {
        for (_, beds) in anchors(&p, "dormitory", "prop.bed") {
            assert!(beds.len() >= 3, "barracks {}: {beds:?}", p.b.id.0);
            assert!(lined_up(&beds), "barracks {}: {beds:?}", p.b.id.0);
            rooms += 1;
        }
    }
    assert!(rooms > 0, "no solved dormitory");
}

/// Art-free vocabulary (`prop.drain`) stays out of town layouts, so a
/// library without it takes no fallback for it.
#[test]
fn art_free_props_stay_out_of_town_layouts() {
    use arda_town::block::wfc::outdoor_vocab::ART_FREE;
    use arda_town::block::{self, fallback, Window};
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let lib = arda_tactical::Library::load(&dir).unwrap();
    for p in plans() {
        let (ox, oy) = p.origin();
        let (w, h) = p.size();
        // The town centre, where paved kerbs carry drains in the solve.
        let win = Window {
            x: ox + w / 2 - 64,
            y: oy + h / 2 - 64,
            w: 128,
            h: 128,
        };
        let blk = block::generate(p, win).unwrap();
        let json = blk.to_json().unwrap();
        for id in ART_FREE {
            assert!(!json.contains(id), "{}: {id} in the layout", p.name);
        }
        let res = fallback::resolve(&blk, &lib, p.seed);
        assert!(
            res.fallbacks
                .iter()
                .all(|f| !ART_FREE.contains(&f.wanted.as_str())),
            "{}: {:?}",
            p.name,
            res.fallbacks
        );
    }
}
