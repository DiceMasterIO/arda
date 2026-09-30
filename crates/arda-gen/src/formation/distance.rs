//! Integer chamfer distances on formation lattices (logic/02
//! §fine-formation littoral). Shared by the littoral pass and its tests.

use super::lattice::alloc;
use super::FormationError;

/// Two-pass 3-4 style chamfer distance in metres (cardinal `d_m`, diagonal
/// `d_m * 181 / 128`) from every cell where `source` holds. Cells beyond
/// reach keep `i32::MAX / 2`.
///
/// # Errors
/// Allocation failure or a spacing that does not fit `i32`.
pub fn chamfer_m(
    source: impl Fn(usize) -> bool,
    w: usize,
    h: usize,
    d_m: i64,
) -> Result<Vec<i32>, FormationError> {
    let mut dist: Vec<i32> = alloc(w * h)?;
    let far = i32::MAX / 2;
    for (i, v) in dist.iter_mut().enumerate() {
        *v = if source(i) { 0 } else { far };
    }
    let card = i32::try_from(d_m.max(1)).map_err(|_| FormationError::ArithmeticOverflow)?;
    let diag =
        i32::try_from(d_m.max(1) * 181 / 128).map_err(|_| FormationError::ArithmeticOverflow)?;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = dist[i];
            if x > 0 {
                v = v.min(dist[i - 1].saturating_add(card));
            }
            if y > 0 {
                v = v.min(dist[i - w].saturating_add(card));
                if x > 0 {
                    v = v.min(dist[i - w - 1].saturating_add(diag));
                }
                if x + 1 < w {
                    v = v.min(dist[i - w + 1].saturating_add(diag));
                }
            }
            dist[i] = v.min(far);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = dist[i];
            if x + 1 < w {
                v = v.min(dist[i + 1].saturating_add(card));
            }
            if y + 1 < h {
                v = v.min(dist[i + w].saturating_add(card));
                if x + 1 < w {
                    v = v.min(dist[i + w + 1].saturating_add(diag));
                }
                if x > 0 {
                    v = v.min(dist[i + w - 1].saturating_add(diag));
                }
            }
            dist[i] = v.min(far);
        }
    }
    Ok(dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_grow_away_from_the_source() {
        let (w, h) = (20, 5);
        let d = chamfer_m(|i| i % w == 0, w, h, 100).unwrap();
        assert_eq!(d[2 * w], 0);
        assert_eq!(d[2 * w + 7], 700);
        assert!(d[2 * w + 19] == 1_900);
    }
}
