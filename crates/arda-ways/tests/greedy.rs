//! Review 3.5-14: crossings on a rasterised river are laid by a rule that
//! does not depend on the window: two windows that overlap lay the same
//! crossings over their shared squares, however many wet stretches lie
//! beyond either window's reach.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod common;

use arda_tactical::TacticalLayout;
use arda_ways::{apply_ways, Feature, RoadClass, Terrain, SQUARE_M};
use common::{flat, road};

/// Narrow channels every 20 m across the road, x in 0..400 m.
struct Braids;

impl Terrain for Braids {
    fn height_m(&self, x: f64, y: f64) -> f64 {
        flat(x, y)
    }

    fn rivers_rasterised(&self) -> bool {
        true
    }

    fn river_water(&self, gx: i64, _gy: i64) -> bool {
        (0..256).contains(&gx) && gx.rem_euclid(13) < 3
    }
}

/// Feature per global square `(gx, gy)` of a window at `x0` squares.
fn features(x0: i64) -> Vec<((i64, i64), Feature)> {
    let roads = vec![road(1, RoadClass::Highway, 170, &[[-600, 50], [1000, 50]])];
    #[allow(clippy::cast_precision_loss)]
    let origin = [x0 as f64 * SQUARE_M, 0.0];
    let mut l = TacticalLayout::new("braids", 64, 64, "grass");
    let out = apply_ways(&mut l, origin, &roads, &[], &Braids, 7).unwrap();
    (0..64_i64)
        .flat_map(|y| (0..64_i64).map(move |x| (x, y)))
        .map(|(x, y)| {
            let f = out
                .sidecar
                .at(u32::try_from(x).unwrap(), u32::try_from(y).unwrap())
                .unwrap()
                .feature;
            ((x0 + x, y), f)
        })
        .collect()
}

#[test]
fn overlapping_windows_lay_the_same_crossings() {
    let mut disagree = Vec::new();
    for x0 in (64..192).step_by(16) {
        let a = features(x0);
        let b = features(x0 + 32);
        let in_b: std::collections::BTreeMap<_, _> = b.into_iter().collect();
        for (sq, f) in a {
            if let Some(g) = in_b.get(&sq) {
                if *g != f {
                    disagree.push(format!("{sq:?}: {f:?} at x0 {x0}, {g:?} at {}", x0 + 32));
                }
            }
        }
    }
    assert!(
        disagree.is_empty(),
        "{} squares differ, e.g. {:?}",
        disagree.len(),
        &disagree[..disagree.len().min(4)]
    );
}
