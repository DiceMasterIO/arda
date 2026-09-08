//! Isostatic base elevation and the coastline cut (`logic/01` step 3).

use super::plates::{CrustType, Plate};

/// Base elevation of continental crust before tectonics, millimetres.
pub const CONTINENTAL_BASE_MM: i32 = 320_000;
/// Base elevation of oceanic crust before tectonics, millimetres.
pub const OCEANIC_BASE_MM: i32 = -2_100_000;

/// Base elevation for a cell owned by `plate`.
#[must_use]
pub const fn base_elevation_mm(plate: &Plate) -> i32 {
    match plate.crust {
        CrustType::Continental => CONTINENTAL_BASE_MM,
        CrustType::Oceanic => OCEANIC_BASE_MM,
    }
}

/// Continentality scale: 0 is fully oceanic crust, 1000 fully continental.
pub const CONTINENTALITY_FULL: i32 = 1000;

/// Box-blur passes applied to the binary crust field.
///
/// A real continental margin is a shelf and a slope spanning tens of
/// kilometres. Assigning crust per cell gives a 2,420 mm-per-4-km cliff at
/// every plate boundary, and because the sea-level contour then falls inside
/// one bilinear patch the coastline comes out as rectangular steps. Blurring
/// the crust indicator first turns that cliff into a shelf.
pub const SHELF_PASSES: usize = 6;
/// Half-width of each blur pass, in 4 km cells.
pub const SHELF_RADIUS: i32 = 2;

/// Radial mask placing the landmass in the middle of the domain.
///
/// `logic/01` step 1 seeds plates on a domain about twice the visible
/// continent and forces the rim oceanic, which only implies a centred
/// continent when the domain is large enough for the plate mixture to work
/// out. On a small domain it does not: the micro continent came out 18 per
/// mille land. This makes the intent explicit at every scale — full
/// continental weight across the middle, falling to open ocean at the rim.
#[must_use]
pub fn continental_mask(x: i32, y: i32, w: i32, h: i32) -> i32 {
    // logic/01 "Radial coast correction": use Euclidean distance in the
    // normalized domain. A max-axis distance makes square contours whose
    // straight flanks propagate into physical terrain and drainage.
    // The widest i32 inputs produce normalized magnitudes below 2^43, so
    // both squares and their sum are representable in u128.
    let nx = (i128::from(x) * 2 - i128::from(w)).unsigned_abs() * 1024
        / u128::try_from(w.max(1)).unwrap_or(1);
    let ny = (i128::from(y) * 2 - i128::from(h)).unsigned_abs() * 1024
        / u128::try_from(h.max(1)).unwrap_or(1);
    let r = (nx * nx + ny * ny).isqrt();
    const CORE: u128 = 80; // full weight inside this radius
    const EDGE: u128 = 400; // open ocean beyond this
    if r <= CORE {
        return CONTINENTALITY_FULL;
    }
    if r >= EDGE {
        return 0;
    }
    let t = 1024 - (r - CORE) * 1024 / (EDGE - CORE);
    let t2 = t * t / 1024;
    let t3 = t2 * t / 1024;
    i32::try_from((3 * t2 - 2 * t3) * u128::from(CONTINENTALITY_FULL.unsigned_abs()) / 1024)
        .unwrap_or(0)
}

/// Smooths a binary crust field into a continentality gradient in
/// `0..=CONTINENTALITY_FULL`.
///
/// Separable box blur, integer throughout.
#[must_use]
pub fn shelf_gradient(binary: &[i32], w: i32, h: i32) -> Vec<i32> {
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut cur = binary.to_vec();
    let mut next = cur.clone();

    for _ in 0..SHELF_PASSES {
        // Horizontal.
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0i64;
                let mut n = 0i64;
                for d in -SHELF_RADIUS..=SHELF_RADIUS {
                    let sx = (x + d).clamp(0, w - 1);
                    sum += i64::from(cur[idx(sx, y)]);
                    n += 1;
                }
                next[idx(x, y)] = i32::try_from(sum / n.max(1)).unwrap_or(0);
            }
        }
        std::mem::swap(&mut cur, &mut next);
        // Vertical.
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0i64;
                let mut n = 0i64;
                for d in -SHELF_RADIUS..=SHELF_RADIUS {
                    let sy = (y + d).clamp(0, h - 1);
                    sum += i64::from(cur[idx(x, sy)]);
                    n += 1;
                }
                next[idx(x, y)] = i32::try_from(sum / n.max(1)).unwrap_or(0);
            }
        }
        std::mem::swap(&mut cur, &mut next);
    }

    // Blurring pulls the field toward its mean, and on a domain only a few
    // blur-widths across that mean sits near the middle of the crust range —
    // which is below sea level, so the whole continent drowns. The micro
    // continent came out 18 per mille land before this. Stretching the result
    // back to the full range keeps the shelf gradient while preserving how
    // much crust is continental.
    let lo = cur.iter().copied().min().unwrap_or(0);
    let hi = cur.iter().copied().max().unwrap_or(CONTINENTALITY_FULL);
    let span = (hi - lo).max(1);
    for v in &mut cur {
        *v = (*v - lo) * CONTINENTALITY_FULL / span;
    }
    cur
}

/// Continentality at which the base surface crosses sea level.
///
/// A straight line from the oceanic base to the continental one puts this
/// crossing at 0.87, so a cell needed 87% continental crust merely to be dry
/// — which drowned the micro continent to 18 per mille land. Real margins
/// are not linear either: the abyssal plain, the slope, and the shelf occupy
/// very different elevation spans for similar widths of crust.
pub const SEA_LEVEL_AT: i32 = 500;

/// Elevation of freshly continental crust at the shoreline, millimetres.
const COAST_BASE_MM: i32 = 90_000;

/// Depth at the outer edge of the shelf, millimetres.
const SHELF_EDGE_MM: i32 = -180_000;

/// Base elevation across the shelf gradient, piecewise so that sea level
/// falls at [`SEA_LEVEL_AT`].
#[must_use]
pub fn graded_base_mm(continentality: i32) -> i32 {
    let t = continentality.clamp(0, CONTINENTALITY_FULL);
    // i64 throughout: the quadratic shelf term reaches -4.8e11 at the
    // abyssal end and overflows i32.
    let t = t as i64;
    let sea = SEA_LEVEL_AT as i64;
    let full = CONTINENTALITY_FULL as i64;
    let v = if t >= sea {
        // Coastal plain rising inland. It starts at COAST_BASE_MM rather than
        // zero: crust that is continental at all should sit clear of the
        // water, or a modest rift is enough to drown the whole interior.
        COAST_BASE_MM as i64
            + (CONTINENTAL_BASE_MM as i64 - COAST_BASE_MM as i64) * (t - sea) / (full - sea)
    } else {
        // Shelf near the coast, steepening away to the abyssal floor.
        let u = sea - t;
        SHELF_EDGE_MM as i64 * u / sea
            + (OCEANIC_BASE_MM as i64 - SHELF_EDGE_MM as i64) * u * u / (sea * sea)
    };
    i32::try_from(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

/// Forces a cell to ocean when it lies within `margin` cells of the visible
/// map edge (`logic/01` invariant: every map-edge cell is ocean).
#[must_use]
pub const fn rim_forced_ocean(x: i32, y: i32, width: i32, height: i32, margin: i32) -> bool {
    x < margin || y < margin || x >= width - margin || y >= height - margin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_euclidean_radii_have_equal_continental_weight() {
        // The 3-4-5 triangle puts these points on the same contour. A
        // max-axis distance instead produces square contours and fails here.
        let axial = continental_mask(1024 + 250, 1024, 2048, 2048);
        let oblique = continental_mask(1024 + 150, 1024 + 200, 2048, 2048);
        assert_eq!(axial, oblique);
        assert!(axial > 0 && axial < CONTINENTALITY_FULL);
    }

    #[test]
    fn moving_tangentially_off_an_axis_changes_the_coastal_slope() {
        let on_axis = continental_mask(1024 + 200, 1024, 2048, 2048);
        let off_axis = continental_mask(1024 + 200, 1024 + 100, 2048, 2048);
        assert!(off_axis < on_axis);

        let mut previous = on_axis;
        for y in 1..=250 {
            let next = continental_mask(1024 + 200, 1024 + y, 2048, 2048);
            assert!(next <= previous, "continental weight rose outward at {y}");
            assert!(previous - next <= 12, "abrupt contour step at {y}");
            previous = next;
        }
    }

    #[test]
    fn radial_mask_preserves_core_ocean_and_reflection_symmetry() {
        assert_eq!(continental_mask(1024, 1024, 2048, 2048), 1000);
        assert_eq!(continental_mask(1024 + 80, 1024, 2048, 2048), 1000);
        assert_eq!(continental_mask(1024 + 400, 1024, 2048, 2048), 0);
        for x in 0..=2048 {
            assert_eq!(continental_mask(x, 0, 2048, 2048), 0);
            assert_eq!(continental_mask(x, 2048, 2048, 2048), 0);
            assert_eq!(continental_mask(0, x, 2048, 2048), 0);
            assert_eq!(continental_mask(2048, x, 2048, 2048), 0);
        }
        for (dx, dy) in [(100, 200), (300, 50), (60, 70), (280, 290)] {
            let weight = continental_mask(1024 + dx, 1024 + dy, 2048, 2048);
            assert_eq!(weight, continental_mask(1024 - dx, 1024 + dy, 2048, 2048));
            assert_eq!(weight, continental_mask(1024 + dx, 1024 - dy, 2048, 2048));
            assert_eq!(weight, continental_mask(1024 + dy, 1024 + dx, 2048, 2048));
        }
        // Normalized distance remains independent of the map aspect ratio.
        assert_eq!(
            continental_mask(1024 + 200, 512 + 50, 2048, 1024),
            continental_mask(512 + 50, 1024 + 200, 1024, 2048)
        );
    }

    #[test]
    fn radial_mask_is_bounded_for_extreme_integer_inputs() {
        for x in [i32::MIN, -1, 0, 1, i32::MAX] {
            for y in [i32::MIN, -1, 0, 1, i32::MAX] {
                for (w, h) in [(0, 0), (-1, -1), (1, 1), (i32::MAX, i32::MAX)] {
                    assert!((0..=1000).contains(&continental_mask(x, y, w, h)));
                }
            }
        }
    }
}
