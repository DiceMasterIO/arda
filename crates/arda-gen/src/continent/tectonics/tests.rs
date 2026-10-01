use super::*;
use crate::continent::plates::{plate_of_warped, seed_plates};

fn sim() -> SimExtent {
    SimExtent {
        width: 125,
        height: 250,
    }
}

#[test]
fn tectonics_is_deterministic() {
    let p = seed_plates(42, sim(), 0);
    assert_eq!(
        run_tectonics(42, &p, sim(), 20),
        run_tectonics(42, &p, sim(), 20)
    );
}

#[test]
fn belts_reach_mountain_scale() {
    // A continent whose highest ground is a few hundred metres has no
    // ranges. The artifact expects mountain streams at 9% slopes.
    let p = seed_plates(42, sim(), 0);
    let u = run_tectonics(42, &p, sim(), 20);
    let peak = u.iter().copied().max().unwrap_or(0);
    assert!(peak > 1_500_000, "highest uplift is only {peak} mm");
}

#[test]
fn fine_profile_breaks_equal_height_crests_without_exceeding_bounded_weight() {
    let mut values = Vec::new();
    for x in 0..64 {
        let weight = crest_weight_q10(42, x, 12);
        assert!((224..=1824).contains(&weight));
        values.push(weight);
    }
    assert!(values.iter().max().unwrap() - values.iter().min().unwrap() > 250);
    assert_eq!(
        values,
        (0..64)
            .map(|x| crest_weight_q10(42, x, 12))
            .collect::<Vec<_>>()
    );
}

#[test]
fn fine_rift_floor_preserves_depth_order_where_legacy_clamps() {
    let input = [-100_000, -500_000, -1_000_000, -2_000_000, 2_600_000];
    let mut legacy = input;
    let mut fine = input;
    normalise(&mut legacy);
    normalise_fine(&mut fine, &input);
    assert_eq!(legacy[1..4], [-220_000; 3]);
    assert!(fine[..4].windows(2).all(|w| w[0] > w[1]));
    assert!(fine[..4].iter().all(|&h| h > -220_000 && h < 0));
    assert_eq!(fine[4], 2_600_000);
    let mut extremes = [i32::MIN, 1];
    normalise_fine(&mut extremes, &[i32::MIN, 1]);
    assert!((-220_000..0).contains(&extremes[0]));
    let mut rift_only = [-2_000_000, -1_000_000, 0];
    let no_peak_calibration = rift_only;
    normalise_fine(&mut rift_only, &no_peak_calibration);
    assert!(rift_only[0] < rift_only[1] && rift_only[1] < 0);
}

#[test]
fn straight_boundary_creates_a_symmetric_belt_at_each_configured_width() {
    let mut kinds = vec![BoundaryMask::default(); 31 * 5];
    for y in 0..5 {
        kinds[y * 31 + 15] =
            BoundaryMask(Boundary::Collision as u8 | Boundary::Arc as u8 | Boundary::Rift as u8);
    }
    for (want, width) in [
        (Boundary::Collision, COLLISION_BELT),
        (Boundary::Arc, ARC_BELT),
        (Boundary::Rift, RIFT_BELT),
    ] {
        let distances = distance_to(&kinds, want, 31, 5, width);
        assert_eq!(belt(distances[2 * 31 + 15], width), 1024);
        let mut previous = 1024;
        for offset in 1..=width {
            let left = usize::try_from(15 - offset).unwrap();
            let right = usize::try_from(15 + offset).unwrap();
            let left_dist = distances[2 * 31 + left];
            let right_dist = distances[2 * 31 + right];
            assert_eq!(left_dist, offset * 1024);
            assert_eq!(left_dist, right_dist);
            let height = belt(left_dist, width);
            assert!(height <= previous);
            if offset < width - 1 {
                assert!(height > 0, "{want:?} vanished at {offset} of {width} cells");
            }
            previous = height;
        }
        assert_eq!(previous, 0, "{want:?} extends past {width} cells");
    }
}

#[test]
fn converging_plates_raise_cells_beyond_boundary_diffusion() {
    let sim = SimExtent {
        width: 64,
        height: 64,
    };
    let plates = [
        Plate {
            id: 0,
            centre_x: 16,
            centre_y: 32,
            crust: CrustType::Continental,
            drift_x: 1,
            drift_y: 0,
        },
        Plate {
            id: 1,
            centre_x: 48,
            centre_y: 32,
            crust: CrustType::Continental,
            drift_x: -1,
            drift_y: 0,
        },
    ];
    let seed = 42;
    let uplift = run_tectonics(seed, &plates, sim, 1);
    let width = usize::try_from(sim.width).unwrap();
    let mut raised_interior = false;
    for y in 3..sim.height - 3 {
        for x in 3..sim.width - 3 {
            let owner = plate_of_warped(seed, &plates, x, y);
            let interior = (-3..=3).all(|dy| {
                (-3..=3).all(|dx| plate_of_warped(seed, &plates, x + dx, y + dy) == owner)
            });
            let i = usize::try_from(y).unwrap() * width + usize::try_from(x).unwrap();
            if interior && uplift[i] > 0 {
                raised_interior = true;
            }
        }
    }
    assert!(
        raised_interior,
        "converging plates raised no cell beyond the immediate boundary and one diffusion pass"
    );
}

#[test]
fn belt_profile_falls_to_zero_at_the_edge() {
    assert_eq!(belt(0, 40), 1024);
    assert_eq!(belt(40 * 1024, 40), 0);
    assert_eq!(belt(41 * 1024, 40), 0);
    assert!(belt(20 * 1024, 40) > 0 && belt(20 * 1024, 40) < 1024);
}
