//! Time-stepped tectonics, coupled with erosion (`logic/01` steps 2-3).
//!
//! Skeleton scope: kinematic-lite — real boundary typing and a real coupled
//! response, 20 steps instead of the full 100. Build-order step 4 replaces
//! the loop body after spike S1; the signature stays.

use super::plates::{plate_of, CrustType, Plate, SimExtent};

/// Uplift added per step at a convergent continental collision, millimetres.
const COLLISION_UPLIFT_MM: i32 = 62_000;
/// Uplift added per step at a subduction arc, millimetres.
const ARC_UPLIFT_MM: i32 = 41_000;
/// Subsidence per step at a rift, millimetres.
const RIFT_SUBSIDENCE_MM: i32 = 24_000;

/// Accumulated uplift in millimetres per 4 km cell.
///
/// Boundary type follows the two plates' crust types and relative motion
/// (`logic/01` §Q5); after each step a coarse diffusion pass lets relief
/// respond, which is the artifact's causality rule at continental scale.
#[must_use]
pub fn run_tectonics(plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    let w = sim.width;
    let h = sim.height;
    let mut uplift = vec![0i32; usize::try_from(w * h).unwrap_or(0)];

    let owner: Vec<u8> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| plate_of(plates, x, y))
        .collect();

    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);

    for _ in 0..steps {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = idx(x, y);
                let mine = owner[i];
                let Some(a) = plates.iter().find(|p| p.id == mine) else {
                    continue;
                };

                // Only the four orthogonal neighbours, in a fixed order.
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let theirs = owner[idx(x + dx, y + dy)];
                    if theirs == mine {
                        continue;
                    }
                    let Some(b) = plates.iter().find(|p| p.id == theirs) else {
                        continue;
                    };

                    // Relative motion projected onto the boundary normal:
                    // negative means the plates are approaching.
                    let closing = (b.drift_x - a.drift_x) * dx + (b.drift_y - a.drift_y) * dy;

                    uplift[i] += match (a.crust, b.crust, closing) {
                        (CrustType::Continental, CrustType::Continental, c) if c < 0 => {
                            COLLISION_UPLIFT_MM
                        }
                        (CrustType::Continental, CrustType::Oceanic, c)
                        | (CrustType::Oceanic, CrustType::Continental, c)
                            if c < 0 =>
                        {
                            ARC_UPLIFT_MM
                        }
                        (_, _, c) if c > 0 => -RIFT_SUBSIDENCE_MM,
                        // Transform: no vertical component.
                        _ => 0,
                    };
                }
            }
        }
        diffuse(&mut uplift, w, h);
    }
    uplift
}

/// One coarse erosion/isostasy pass: a fixed-weight five-point stencil.
///
/// Integer arithmetic with a fixed divisor, so it is bit-identical anywhere.
fn diffuse(field: &mut [i32], w: i32, h: i32) {
    let source = field.to_vec();
    let stride = usize::try_from(w).unwrap_or(1);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            let sum = i64::from(source[i]) * 4
                + i64::from(source[i - 1])
                + i64::from(source[i + 1])
                + i64::from(source[i - stride])
                + i64::from(source[i + stride]);
            #[allow(clippy::cast_possible_truncation)]
            {
                field[i] = (sum / 8) as i32;
            }
        }
    }
}
