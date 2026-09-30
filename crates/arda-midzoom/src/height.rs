//! A refined height lattice and its C1 (Catmull-Rom) evaluation.

use crate::source::FINE_UM;

/// Refined heights on a lattice `n` times finer than the stored one.
///
/// Node `(i, j)` sits at fine-lattice micrometres `(i·s, j·s)` with
/// `s = 39.0625 m / n` (world position plus `FINE_FRAME_OFFSET_UM`'s
/// inverse: node 0 is the centre of saved cell 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeightTile {
    /// First node column.
    pub i0: i64,
    /// First node row.
    pub j0: i64,
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Subdivisions of the stored 39.0625 m spacing (1, 2, 4 or 8).
    pub n: i64,
    /// Heights, row-major, millimetres.
    pub heights_mm: Vec<i32>,
}

/// Height and gradient at one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePoint {
    /// Height, mm.
    pub height_mm: i64,
    /// Gradient `(dz/dx, dz/dy)`, Q12 slopes.
    pub gradient_q12: (i64, i64),
}

impl HeightTile {
    /// Lattice spacing in micrometres.
    #[must_use]
    pub const fn spacing_um(&self) -> i64 {
        FINE_UM / self.n
    }

    /// Height of node `(i, j)` (global indices), if inside.
    #[must_use]
    pub fn node(&self, i: i64, j: i64) -> Option<i32> {
        let x = usize::try_from(i - self.i0).ok()?;
        let y = usize::try_from(j - self.j0).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        self.heights_mm.get(y * self.width + x).copied()
    }

    /// Catmull-Rom height and gradient at fine-lattice micrometres. `None`
    /// when the 4×4 support leaves the tile.
    #[must_use]
    pub fn sample(&self, x_um: i64, y_um: i64) -> Option<SurfacePoint> {
        let s = self.spacing_um();
        let (i, tx) = (x_um.div_euclid(s), (x_um.rem_euclid(s) << 16) / s);
        let (j, ty) = (y_um.div_euclid(s), (y_um.rem_euclid(s) << 16) / s);
        let (wx, dwx) = catmull(tx);
        let (wy, dwy) = catmull(ty);
        let (mut v, mut gx, mut gy) = (0_i128, 0_i128, 0_i128);
        for (dy, (wyb, dwyb)) in (-1..=2_i64).zip(wy.iter().zip(dwy)) {
            let (mut row, mut drow) = (0_i128, 0_i128);
            for (dx, (wxa, dwxa)) in (-1..=2_i64).zip(wx.iter().zip(dwx)) {
                let z = i128::from(self.node(i + dx, j + dy)?);
                row += wxa * z;
                drow += dwxa * z;
            }
            v += wyb * row;
            gx += wyb * drow;
            gy += dwyb * row;
        }
        let q32 = 1_i128 << 32;
        let spacing_mm = i128::from(s) / 1_000;
        let slope = |d: i128| i64::try_from(d * 4_096 / q32 / spacing_mm.max(1)).unwrap_or(0);
        Some(SurfacePoint {
            height_mm: i64::try_from(v.div_euclid(q32)).unwrap_or(0),
            gradient_q12: (slope(gx), slope(gy)),
        })
    }
}

/// Catmull-Rom weights and `d/dt` at a Q16 `t`, both Q16.
fn catmull(t: i64) -> ([i128; 4], [i128; 4]) {
    let one = 65_536_i128;
    let t = i128::from(t);
    let t2 = t * t / one;
    let t3 = t2 * t / one;
    (
        [
            (-t3 + 2 * t2 - t) / 2,
            (3 * t3 - 5 * t2 + 2 * one) / 2,
            (-3 * t3 + 4 * t2 + t) / 2,
            (t3 - t2) / 2,
        ],
        [
            (-3 * t2 + 4 * t - one) / 2,
            (9 * t2 - 10 * t) / 2,
            (-9 * t2 + 8 * t + one) / 2,
            (3 * t2 - 2 * t) / 2,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_is_reproduced_with_its_slope() {
        let n = 4;
        let s = FINE_UM / n;
        let (w, h) = (8, 8);
        // z = 0.25·x (Q12 slope 1024).
        let heights_mm = (0..h)
            .flat_map(|_| (0..w).map(move |i: i64| i32::try_from(i * s / 4_000).unwrap()))
            .collect();
        let t = HeightTile {
            i0: 0,
            j0: 0,
            width: 8,
            height: 8,
            n,
            heights_mm,
        };
        let p = t.sample(3 * s + s / 3, 4 * s).unwrap();
        assert!((p.height_mm - (3 * s + s / 3) / 4_000).abs() <= 2, "{p:?}");
        assert!((p.gradient_q12.0 - 1_024).abs() <= 2, "{p:?}");
        assert_eq!(p.gradient_q12.1, 0);
        assert!(t.sample(0, 0).is_none(), "support leaves the tile");
    }
}
