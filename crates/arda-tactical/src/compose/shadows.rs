//! The swept drop shadow of the lighting pass (goal 63): light from the
//! top-left at 45°, cast by the height map.

use super::lighting::{Buffers, Lighting, FADE};
use rayon::prelude::*;

/// Graded shadow mask: light from the top-left at 45°. `reach` carries the
/// tallest occluder height seen along each diagonal, falling by the sun
/// slope each step; a pixel is shadowed where it lies below that line, the
/// more the deeper. Each diagonal is an independent chain, so groups of
/// [`DIAGONALS`] diagonals run in parallel and are scattered back to rows.
pub(super) fn drop_shadows(buf: &Buffers, ppsq: u32, l: &Lighting) -> Vec<u8> {
    let (w, h) = (buf.width as usize, buf.height as usize);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    // Height lost per diagonal pixel step, in 1/256 ft: a step covers √2 px
    // (≈ 1448/1024) and one foot casts `shadow_len·ppsq/1024` px.
    let per_ft = u64::from(l.shadow_len) * u64::from(ppsq);
    let drop = i32::try_from((256 * 1448 / per_ft.max(1)).max(1)).unwrap_or(i32::MAX);
    // Diagonal `e` (0..w + h - 1) holds the pixels with x - y = e - (h - 1).
    let groups: Vec<Vec<u8>> = (0..(w + h - 1).div_ceil(DIAGONALS))
        .into_par_iter()
        .map(|g| {
            let mut out = vec![0u8; h * DIAGONALS];
            let mut carry = [i32::MIN / 2; DIAGONALS];
            for y in 0..h {
                for (k, c) in carry.iter_mut().enumerate() {
                    let Some(x) = (g * DIAGONALS + k + y).checked_sub(h - 1) else {
                        continue;
                    };
                    if x >= w {
                        continue;
                    }
                    let own = buf.height_map[y * w + x];
                    // The chain starts on the top row or the left column.
                    let carried = if x > 0 && y > 0 {
                        *c - drop
                    } else {
                        i32::MIN / 2
                    };
                    let below = carried - own - 128;
                    out[y * DIAGONALS + k] = if below > 0 {
                        u8::try_from((below * 255 / FADE).min(255)).unwrap_or(255)
                    } else {
                        0
                    };
                    *c = own.max(carried);
                }
            }
            out
        })
        .collect();
    let mut mask = vec![0u8; w * h];
    mask.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, m) in row.iter_mut().enumerate() {
            let e = x + h - 1 - y;
            *m = groups[e / DIAGONALS][y * DIAGONALS + e % DIAGONALS];
        }
    });
    mask
}

/// Diagonals per parallel group of [`drop_shadows`].
const DIAGONALS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tall_block_shadows_its_bottom_right_only() {
        let (w, h) = (40u32, 40u32);
        let mut buf = Buffers::new(w, h);
        for y in 10..20 {
            for x in 10..20 {
                buf.height_map[y * 40 + x] = 256 * 10;
            }
        }
        let m = drop_shadows(&buf, 32, &Lighting::default());
        assert_eq!(m[22 * 40 + 22], 255, "below-right is shadowed");
        assert_eq!(m[8 * 40 + 8], 0, "above-left is lit");
        assert_eq!(m[15 * 40 + 15], 0, "the block's top is lit");
        assert_eq!(m[39 * 40 + 39], 0, "shadow length is finite");
    }

    /// The row-by-row sweep the diagonal groups replace (goal 50).
    fn serial_shadows(buf: &Buffers, ppsq: u32, l: &Lighting) -> Vec<u8> {
        let (w, h) = (buf.width as usize, buf.height as usize);
        let per_ft = u64::from(l.shadow_len) * u64::from(ppsq);
        let drop = i32::try_from((256 * 1448 / per_ft.max(1)).max(1)).unwrap();
        let mut mask = vec![0u8; w * h];
        let mut prev = vec![i32::MIN / 2; w];
        for y in 0..h {
            let mut cur = vec![i32::MIN / 2; w];
            for x in 0..w {
                let own = buf.height_map[y * w + x];
                let carried = if x > 0 && y > 0 {
                    prev[x - 1] - drop
                } else {
                    i32::MIN / 2
                };
                let below = carried - own - 128;
                mask[y * w + x] = if below > 0 {
                    u8::try_from((below * 255 / FADE).min(255)).unwrap()
                } else {
                    0
                };
                cur[x] = own.max(carried);
            }
            prev = cur;
        }
        mask
    }

    #[test]
    fn diagonal_shadows_equal_the_row_sweep() {
        for (w, h) in [(1u32, 1u32), (7, 130), (150, 9), (133, 71)] {
            let mut buf = Buffers::new(w, h);
            for (i, v) in buf.height_map.iter_mut().enumerate() {
                let k = i32::try_from(i).unwrap();
                *v = (k * 7919 % 4096) * ((k / 13) % 3) - 512;
            }
            for ppsq in [16, 64] {
                let l = Lighting::default();
                assert_eq!(
                    drop_shadows(&buf, ppsq, &l),
                    serial_shadows(&buf, ppsq, &l),
                    "{w}x{h} at {ppsq}"
                );
            }
        }
    }
}
