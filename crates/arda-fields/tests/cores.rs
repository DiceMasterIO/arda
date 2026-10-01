//! Fields run up to the settlements' own footprints when the caller knows
//! them, and a noise-shaped fringe marks the wild ground beside worked land.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    missing_docs
)]

use arda_fields::fringe::MAX_WIDTH;
use arda_fields::generate;
use arda_fields::geom::SQUARE_M;
use arda_fields::synthetic;

const SEED: u64 = 42;
const SIDE: u32 = 160;

/// The window's squares with their distance from the village centre.
fn around(origin: [f64; 2]) -> impl Iterator<Item = (usize, f64)> {
    let c = (5050.0 / SQUARE_M, 5050.0 / SQUARE_M);
    let (x0, y0) = (
        (origin[0] / SQUARE_M).floor(),
        (origin[1] / SQUARE_M).floor(),
    );
    (0..(SIDE * SIDE) as usize).map(move |i| {
        let (x, y) = (
            x0 + (i % SIDE as usize) as f64,
            y0 + (i / SIDE as usize) as f64,
        );
        (i, (x + 0.5 - c.0).hypot(y + 0.5 - c.1))
    })
}

#[test]
fn fields_reach_the_plans_footprint_instead_of_the_density_disc() {
    let s = synthetic::village_strips();
    let origin = [5050.0 - 125.0, 5050.0 - 125.0];
    let disc = generate(&s.inputs(), origin, SIDE, SIDE, SEED).unwrap();
    let core = |gx: i64, gy: i64| {
        let (cx, cy) = ((5050.0 / SQUARE_M) as i64, (5050.0 / SQUARE_M) as i64);
        (gx - cx).abs() <= 12 && (gy - cy).abs() <= 12
    };
    let mut inputs = s.inputs();
    inputs.cores = Some(&core);
    let plan = generate(&inputs, origin, SIDE, SIDE, SEED).unwrap();
    let ring: Vec<usize> = around(origin)
        .filter(|&(_, d)| (25.0..60.0).contains(&d))
        .map(|(i, _)| i)
        .collect();
    let owned = |w: &arda_fields::FieldsWindow| ring.iter().filter(|&&i| w.owned[i]).count();
    assert_eq!(owned(&disc), 0, "the density disc keeps fields out");
    // The plan's fields reach the footprint. Field sites with no land use
    // (the village's own cells here) are planned but left unclaimed, so the
    // refiner's natural ground shows there (logic/09 §reservations).
    let win = arda_fields::window::window_rect(origin, SIDE, SIDE).unwrap();
    let planned = arda_fields::plan::Plan::build(&inputs, win, SEED);
    let fields = ring
        .iter()
        .filter(|&&i| {
            let s = arda_fields::geom::Sq::new(
                win.0 + (i % SIDE as usize) as i64,
                win.1 + (i / SIDE as usize) as i64,
            );
            matches!(planned.at(s), arda_fields::plan::Cover::Field(_))
        })
        .count();
    assert!(
        fields * 10 > ring.len() * 8,
        "fields fill the ring outside the footprint: {fields} of {}",
        ring.len()
    );
    for &i in &ring {
        let s = arda_fields::geom::Sq::new(
            win.0 + (i % SIDE as usize) as i64,
            win.1 + (i / SIDE as usize) as i64,
        );
        let wild = planned
            .field_at(s)
            .is_some_and(|(_, f)| f.kind == arda_fields::fields::FieldKind::Wild);
        assert!(!(wild && plan.owned[i]), "unused land {i} is claimed");
    }
    for (i, d) in around(origin) {
        if d < 8.0 {
            assert!(!plan.owned[i], "square {i} inside the footprint is farmed");
        }
    }
}

#[test]
fn the_fringe_lies_on_wild_ground_near_worked_land() {
    let s = synthetic::village_strips();
    let w = generate(&s.inputs(), s.origin_m, 256, 256, SEED).unwrap();
    let side = 256_usize;
    let fringe: Vec<usize> = (0..w.fringe.len())
        .filter(|&i| w.fringe[i].is_some())
        .collect();
    assert!(!fringe.is_empty());
    let reach = MAX_WIDTH.ceil() as i64 + 1;
    for &i in &fringe {
        assert!(!w.owned[i]);
        let (x, y) = ((i % side) as i64, (i / side) as i64);
        let near = (-reach..=reach).any(|dy| {
            (-reach..=reach).any(|dx| {
                let (a, b) = (x + dx, y + dy);
                (0..256).contains(&a)
                    && (0..256).contains(&b)
                    && w.owned[b as usize * side + a as usize]
            })
        });
        let edge = x < reach || y < reach || x >= 256 - reach || y >= 256 - reach;
        assert!(
            near || edge,
            "fringe square ({x}, {y}) far from worked land"
        );
    }
}
