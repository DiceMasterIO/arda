//! Coast derivation (contract §coast).
//!
//! A coast cell is a land cell with a sea cell among its 8 neighbours; lake
//! shores are not coast. Distance is the exact Euclidean distance between
//! cell nodes to the nearest coast cell, found with Meijster's linear-time
//! distance transform over the area plus a [`HALO_CELLS`] neighbour ring,
//! and capped at [`MAX_DISTANCE_M`]. Cells outside the modeled world are
//! unknown: never sea, never coast.

use arda_core::TerrainKind;

/// Search radius in cells.
pub const MAX_DISTANCE_CELLS: u32 = 50;
/// Largest reported coast distance, metres.
pub const MAX_DISTANCE_M: f64 = 5_000.0;
/// Neighbour cells loaded around an area: the radius plus the ring needed to classify its edge.
pub const HALO_CELLS: usize = MAX_DISTANCE_CELLS as usize + 1;

/// A rectangular window of terrain kinds; `None` marks cells outside the world.
#[derive(Debug, Clone)]
pub struct KindWindow {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Row-major kinds.
    pub kinds: Vec<Option<TerrainKind>>,
}

impl KindWindow {
    fn at(&self, x: isize, y: isize) -> Option<TerrainKind> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        if x >= self.width || y >= self.height {
            return None;
        }
        self.kinds.get(y * self.width + x).copied().flatten()
    }

    /// Coast flag per window cell.
    #[must_use]
    pub fn coast_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let (Ok(ix), Ok(iy)) = (isize::try_from(x), isize::try_from(y)) else {
                    continue;
                };
                if self.at(ix, iy) != Some(TerrainKind::Land) {
                    continue;
                }
                let sea_near = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        (dx, dy) != (0, 0) && self.at(ix + dx, iy + dy) == Some(TerrainKind::Sea)
                    })
                });
                mask[y * self.width + x] = sea_near;
            }
        }
        mask
    }
}

/// Exact squared Euclidean distance (cells²) from every cell to the nearest `true` cell.
///
/// Cells with no feature anywhere receive a value above any in-window distance.
#[must_use]
pub fn squared_distance(mask: &[bool], width: usize, height: usize) -> Vec<u64> {
    let infinity = i64::try_from(width + height).unwrap_or(i64::MAX / 4);
    let mut g = vec![infinity; width * height];
    for x in 0..width {
        if height == 0 {
            break;
        }
        g[x] = if mask[x] { 0 } else { infinity };
        for y in 1..height {
            let i = y * width + x;
            g[i] = if mask[i] {
                0
            } else {
                (g[i - width] + 1).min(infinity)
            };
        }
        for y in (0..height.saturating_sub(1)).rev() {
            let i = y * width + x;
            if g[i + width] < g[i] {
                g[i] = g[i + width] + 1;
            }
        }
    }
    let mut out = vec![0_u64; width * height];
    let mut s = vec![0_i64; width];
    let mut t = vec![0_i64; width];
    for y in 0..height {
        let row = &g[y * width..(y + 1) * width];
        let gi = |i: i64| row[usize::try_from(i).unwrap_or(0)];
        let f = |x: i64, i: i64| (x - i) * (x - i) + gi(i) * gi(i);
        let sep = |i: i64, u: i64| {
            (u * u - i * i + gi(u) * gi(u) - gi(i) * gi(i)).div_euclid(2 * (u - i))
        };
        let last = i64::try_from(width).unwrap_or(0);
        // `k` is the envelope length; entries `s[..k]` start at `t[..k]`.
        let mut k = 1_usize;
        s[0] = 0;
        t[0] = 0;
        for u in 1..last {
            while k > 0 && f(t[k - 1], s[k - 1]) > f(t[k - 1], u) {
                k -= 1;
            }
            if k == 0 {
                k = 1;
                s[0] = u;
                t[0] = 0;
            } else {
                let w = 1 + sep(s[k - 1], u);
                if w < last {
                    s[k] = u;
                    t[k] = w;
                    k += 1;
                }
            }
        }
        for u in (0..last).rev() {
            let d = f(u, s[k - 1]);
            out[y * width + usize::try_from(u).unwrap_or(0)] = u64::try_from(d).unwrap_or(u64::MAX);
            if u == t[k - 1] {
                k -= 1;
            }
        }
    }
    out
}

/// Per-cell coast facts for one area, cropped from a haloed window.
#[derive(Debug, Clone)]
pub struct AreaCoast {
    is_coast: Vec<bool>,
    distance_sq: Vec<u32>,
}

impl AreaCoast {
    /// Derives the `side × side` area at offset `halo` inside `window`.
    #[must_use]
    pub fn derive(window: &KindWindow, halo: usize, side: usize) -> Self {
        let mask = window.coast_mask();
        let d2 = squared_distance(&mask, window.width, window.height);
        let cap = u64::from(MAX_DISTANCE_CELLS * MAX_DISTANCE_CELLS);
        let mut is_coast = Vec::with_capacity(side * side);
        let mut distance_sq = Vec::with_capacity(side * side);
        for y in 0..side {
            for x in 0..side {
                let i = (y + halo) * window.width + x + halo;
                is_coast.push(mask.get(i).copied().unwrap_or(false));
                let d = d2.get(i).copied().unwrap_or(u64::MAX);
                distance_sq.push(if d <= cap {
                    u32::try_from(d).unwrap_or(u32::MAX)
                } else {
                    u32::MAX
                });
            }
        }
        Self {
            is_coast,
            distance_sq,
        }
    }

    /// Coast facts for the area-local row-major index `i`.
    #[must_use]
    pub fn sample(&self, i: usize) -> super::CoastSample {
        let d = self.distance_sq.get(i).copied().unwrap_or(u32::MAX);
        super::CoastSample {
            is_coast: self.is_coast.get(i).copied().unwrap_or(false),
            distance_m: (d != u32::MAX).then(|| f64::from(d).sqrt() * super::CELL_M),
        }
    }

    /// Approximate heap footprint, for cache accounting.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.is_coast.len() + self.distance_sq.len() * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(mask: &[bool], w: usize, h: usize) -> Vec<u64> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i64::try_from(i % w).unwrap(), i64::try_from(i / w).unwrap());
                (0..w * h)
                    .filter(|&j| mask[j])
                    .map(|j| {
                        let (jx, jy) =
                            (i64::try_from(j % w).unwrap(), i64::try_from(j / w).unwrap());
                        u64::try_from((x - jx).pow(2) + (y - jy).pow(2)).unwrap()
                    })
                    .min()
                    .unwrap_or(u64::MAX)
            })
            .collect()
    }

    #[test]
    fn transform_matches_brute_force_on_pseudo_random_masks() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        for (w, h) in [(1, 1), (7, 3), (13, 17), (31, 29)] {
            let mask: Vec<bool> = (0..w * h)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state.is_multiple_of(11)
                })
                .collect();
            if !mask.iter().any(|&m| m) {
                continue;
            }
            assert_eq!(squared_distance(&mask, w, h), brute(&mask, w, h), "{w}x{h}");
        }
    }

    #[test]
    fn land_touching_sea_diagonally_is_coast_and_lake_shore_is_not() {
        use TerrainKind::{Lake, Land, Sea};
        let window = KindWindow {
            width: 3,
            height: 3,
            kinds: [Sea, Land, Land, Land, Land, Lake, Land, Land, Land]
                .into_iter()
                .map(Some)
                .collect(),
        };
        let mask = window.coast_mask();
        assert_eq!(
            mask,
            vec![false, true, false, true, true, false, false, false, false]
        );
    }

    #[test]
    fn distances_are_metres_and_capped() {
        let side = 3;
        let mut kinds = vec![Some(TerrainKind::Land); 100 * 60];
        kinds[0] = Some(TerrainKind::Sea);
        let window = KindWindow {
            width: 100,
            height: 60,
            kinds,
        };
        let coast = AreaCoast::derive(&window, 0, side);
        assert!(coast.sample(1).is_coast);
        assert_eq!(coast.sample(1).distance_m, Some(0.0));
        // (2,0) is one cell east of coast (1,0); (2,2) is diagonal to coast (1,1).
        assert_eq!(coast.sample(2).distance_m, Some(100.0));
        assert_eq!(
            coast.sample(2 * side + 2).distance_m,
            Some(100.0 * 2_f64.sqrt())
        );
        let far = AreaCoast::derive(&window, 55, side);
        assert_eq!(far.sample(0).distance_m, None);
    }
}
