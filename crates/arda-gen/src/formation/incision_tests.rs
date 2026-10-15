//! Numerical checks of the level step (logic/02 §fine-formation hillslopes;
//! review 3.1-1, 3.1-3).

use super::*;
use crate::formation::drainage::FIXED;
use crate::formation::{CHANNEL_AREA_Q8, FINE_SPACING_UM, HILL_KAPPA_Q16};

/// A level-5 style 129² lattice: a gentle plane with ±3 mm cell-scale
/// roughness, every cell on a mature (soft = 255), low-relief hillslope
/// below channel initiation, so only hillslope creep acts.
fn mature_hillslope(v9: bool) -> (Lattice, Vec<u8>, Vec<u8>, Vec<u8>, LevelParams) {
    let (w, h) = (129, 129);
    let mut g = Lattice::new(w, h, FINE_SPACING_UM).unwrap();
    let mut flags = vec![0_u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let rough = if (x + y) % 2 == 0 { 3 } else { -3 };
            g.z[i] = 100_000 + 20 * x as i32 + rough;
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                flags[i] = FIXED;
            }
        }
    }
    let params = LevelParams {
        k_q16: 0,
        talus_q16: 65_536,
        lambda_q16: 0,
        blur_cells: 0,
        iterations: 0,
        kappa_q16: 0,
        stoch_q16: 0,
        seed: 1,
        uplift_mm: 0,
        eps_mm: 2,
        area_unit: 1,
        floor_mm: 0,
        creep_kappa_q16: 0,
        hill_kappa_q16: HILL_KAPPA_Q16,
        channel_area_q8: CHANNEL_AREA_Q8,
        convergent_area: true,
        v9,
    };
    (g, flags, vec![255; w * h], vec![0; w * h], params)
}

/// Largest cell-scale (checkerboard) roughness of the interior, mm.
fn roughness(g: &Lattice) -> i64 {
    let w = g.width;
    let mut worst = 0;
    for y in 2..g.height - 2 {
        for x in 2..w - 2 {
            let i = y * w + x;
            let mean = (i64::from(g.z[i - 1])
                + i64::from(g.z[i + 1])
                + i64::from(g.z[i - w])
                + i64::from(g.z[i + w]))
                / 4;
            worst = worst.max((i64::from(g.z[i]) - mean).abs());
        }
    }
    worst
}

fn creep(v9: bool, steps: usize) -> i64 {
    let (mut g, flags, soft, relief, params) = mature_hillslope(v9);
    let n = g.z.len();
    let (zero, rock) = (vec![0_u8; n], vec![128_u8; n]);
    let fields = LevelFields {
        flags: &flags,
        uplift: &zero,
        erodibility: &zero,
        runoff: None,
        creep: &zero,
        rock: &rock,
        shape: Some(ShapeFields {
            soft: &soft,
            relief: &relief,
        }),
    };
    let area = vec![0_u64; n];
    let mut tmp = vec![0_i32; n];
    for _ in 0..steps {
        diffuse(&mut g, &fields, &params, &area, &mut tmp);
    }
    roughness(&g)
}

/// Review 3.1-1: on mature ground the creep number reached 0.33, beyond
/// the explicit five-point scheme's stability limit of 0.25, so cell-scale
/// roughness grew instead of decaying. Recipe 9 keeps every creep number
/// within the limit; recipes 6-8 keep their published (unstable) number.
#[test]
fn recipe_9_hillslope_creep_is_stable_on_mature_ground() {
    let before = creep(true, 0);
    assert!((3..=6).contains(&before), "initial roughness {before} mm");
    let v9 = creep(true, 30);
    assert!(
        v9 <= before,
        "recipe 9 creep smooths: {v9} mm vs {before} mm"
    );
    let v7 = creep(false, 30);
    assert!(v7 > 10 * before, "recipe 7 creep amplifies: {v7} mm");
}

/// Review 3.1-3: the floor of the implicit update turned a small drop on a
/// large channel into an exact tie with the receiver. Recipe 9 keeps 1 mm.
#[test]
fn recipe_9_implicit_incision_keeps_a_millimetre_of_drop() {
    let (zr, f) = (50_000, 2_000_000);
    for drop in [1, 5, 20] {
        assert_eq!(
            implicit_step(zr + drop, zr, f, false),
            zr,
            "recipe 7 ties at {drop} mm"
        );
        assert_eq!(
            implicit_step(zr + drop, zr, f, true),
            zr + 1,
            "recipe 9 at {drop} mm"
        );
    }
    // Large drops are solved exactly as before.
    let big = implicit_step(zr + 100_000, zr, f, true);
    assert_eq!(big, implicit_step(zr + 100_000, zr, f, false));
    assert!(big > zr + 1);
}
