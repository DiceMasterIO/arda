use super::*;

fn relief(surface: impl Fn(usize, usize) -> i32) -> ObliqueRelief {
    let nodes = (64, 64);
    let s = (0..nodes.1)
        .flat_map(|y| (0..nodes.0).map(move |x| (x, y)))
        .map(|(x, y)| surface(x, y))
        .collect();
    ObliqueRelief::from_nodes((256, 256), nodes, s, DEFAULT_TILT_Q12)
}

const fn q16(cells: i64) -> i64 {
    cells << 16
}

#[test]
fn sea_level_never_moves_and_shift_follows_height() {
    // A 2 km cone in the middle of a sea.
    let r = relief(|x, y| {
        let d = i32::try_from(x.abs_diff(32).pow(2) + y.abs_diff(32).pow(2)).unwrap();
        (2_000_000 - d * 40_000).max(0)
    });
    assert_eq!(r.shift_mm(q16(10), q16(10)), 0, "open sea");
    let peak = r.shift_mm(q16(130), q16(130));
    let surface = r.surface_mm(q16(130), q16(130));
    assert_eq!(peak, surface * DEFAULT_TILT_Q12 / ONE);
    assert!(
        peak > 400_000 && peak <= 2_000_000 * DEFAULT_TILT_Q12 / ONE,
        "{peak}"
    );
}

#[test]
fn shift_is_bounded_by_tilt_times_the_highest_node() {
    let r = relief(|x, y| i32::try_from((x * 7 + y * 13) % 41).unwrap() * 100_000);
    let bound = r.max_surface_mm() * r.tilt_q12() / ONE;
    for y in (0..256).step_by(5) {
        for x in (0..256).step_by(5) {
            let s = r.shift_mm(q16(x), q16(y));
            assert!((0..=bound).contains(&s), "{x},{y}: {s} > {bound}");
        }
    }
}

#[test]
fn shading_leaves_sea_alone_and_stays_gentle_on_land() {
    let sea = Sample {
        surface: 0,
        hollow: 0,
        broad: 0,
        gradient: (0, 0),
    };
    assert_eq!(shade([20, 60, 120], &sea), [20, 60, 120]);
    for (surface, hollow, broad, g) in [
        (50_000, 300_000, 900_000, (-2_000, -2_000)),
        (2_500_000, 2_000_000, 1_500_000, (1_000, 500)),
        (400_000, 400_000, 400_000, (0, 0)),
    ] {
        let s = Sample {
            surface,
            hollow,
            broad,
            gradient: g,
        };
        let c = [120, 130, 70];
        let out = shade(c, &s);
        for ch in 0..3 {
            let d = (out[ch] - c[ch]).abs();
            assert!(d <= 30, "{s:?}: {c:?} -> {out:?}");
        }
    }
}

#[test]
fn valleys_take_a_cool_veil_and_crests_stay_warm() {
    let valley = shade(
        [120, 130, 70],
        &Sample {
            surface: 100_000,
            hollow: 350_000,
            broad: 1_200_000,
            gradient: (0, 0),
        },
    );
    let crest = shade(
        [120, 130, 70],
        &Sample {
            surface: 2_400_000,
            hollow: 2_100_000,
            broad: 1_600_000,
            gradient: (0, 0),
        },
    );
    // Bluer relative to red in the valley than on the crest.
    assert!(
        valley[2] * crest[0] > crest[2] * valley[0],
        "{valley:?} {crest:?}"
    );
}
