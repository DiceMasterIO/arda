//! The world as flat 100 m rasters, rotation control and angle bins.

use arda::World;
use arda_core::{CellCoord, TerrainKind};
use std::error::Error;

/// Terrain code: sea.
pub const SEA: u8 = 0;
/// Terrain code: land.
pub const LAND: u8 = 1;
/// Terrain code: lake.
pub const LAKE: u8 = 2;

/// Whole-world rasters on the 100 m cell grid.
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// Height, millimetres.
    pub z: Vec<i32>,
    /// `SEA`, `LAND` or `LAKE`.
    pub kind: Vec<u8>,
    /// Strahler order, 0 when no watercourse.
    pub order: Vec<u8>,
}

impl Grid {
    /// A grid of the given size filled with land at height 0.
    pub fn flat(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            z: vec![0; w * h],
            kind: vec![LAND; w * h],
            order: vec![0; w * h],
        }
    }

    /// Reads every area of a saved world.
    pub fn load(world: &World) -> Result<Self, Box<dyn Error>> {
        let (w, h) = (
            usize::try_from(world.manifest().areas_wide)? * 512,
            usize::try_from(world.manifest().areas_high)? * 512,
        );
        let mut g = Self::flat(w, h);
        for (ax, ay) in world.area_coords() {
            let area = world.read_area(ax, ay)?;
            let cells = area.cells();
            for y in 0..512_u16 {
                for x in 0..512_u16 {
                    let c = cells.get(CellCoord::new(x, y).ok_or("cell coordinate")?);
                    let i = (usize::try_from(ay)? * 512 + usize::from(y)) * w
                        + usize::try_from(ax)? * 512
                        + usize::from(x);
                    g.z[i] = c.height.raw();
                    g.kind[i] = match c.terrain {
                        TerrainKind::Sea => SEA,
                        TerrainKind::Land => LAND,
                        TerrainKind::Lake => LAKE,
                    };
                    g.order[i] = c.watercourse_order;
                }
            }
        }
        Ok(g)
    }

    /// Bilinear height / nearest kind sample of a rotated square crop.
    /// `cx, cy` is the crop centre in this grid, `size` its side in cells.
    pub fn rotated_crop(&self, cx: f64, cy: f64, size: usize, deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        let half = size as f64 / 2.0;
        let mut out = Self::flat(size, size);
        for y in 0..size {
            for x in 0..size {
                let (u, v) = (x as f64 + 0.5 - half, y as f64 + 0.5 - half);
                let (sx, sy) = (cx + u * c - v * s, cy + u * s + v * c);
                let i = y * size + x;
                if sx < 1.0 || sy < 1.0 || sx > (self.w - 2) as f64 || sy > (self.h - 2) as f64 {
                    out.kind[i] = SEA;
                    continue;
                }
                let (fx, fy) = (sx - 0.5, sy - 0.5);
                let (x0, y0) = (fx.floor(), fy.floor());
                let (tx, ty) = (fx - x0, fy - y0);
                let at = |dx: usize, dy: usize| {
                    f64::from(self.z[(y0 as usize + dy) * self.w + x0 as usize + dx])
                };
                let top = at(0, 0) * (1.0 - tx) + at(1, 0) * tx;
                let bot = at(0, 1) * (1.0 - tx) + at(1, 1) * tx;
                out.z[i] = (top * (1.0 - ty) + bot * ty).round() as i32;
                let (nx, ny) = (sx.floor() as usize, sy.floor() as usize);
                out.kind[i] = self.kind[ny * self.w + nx];
            }
        }
        out
    }

    /// Centre of the `size`-cell window (on a half-size stride) with the
    /// most cells for which `f` holds, sampling every 8th cell.
    pub fn pick_crop(&self, size: usize, f: impl Fn(usize) -> bool) -> (f64, f64) {
        let stride = (size / 2).max(1);
        let mut best = (0_usize, size as f64 / 2.0, size as f64 / 2.0);
        let mut y0 = 0;
        while y0 + size <= self.h {
            let mut x0 = 0;
            while x0 + size <= self.w {
                let mut n = 0;
                for y in (y0..y0 + size).step_by(8) {
                    for x in (x0..x0 + size).step_by(8) {
                        n += usize::from(f(y * self.w + x));
                    }
                }
                if n > best.0 {
                    best = (n, (x0 + size / 2) as f64, (y0 + size / 2) as f64);
                }
                x0 += stride;
            }
            y0 += stride;
        }
        (best.1, best.2)
    }
}

/// 24 directed 15-degree bins, bin `i` centred on `15 i` degrees.
pub fn bin24(deg: f64) -> usize {
    ((deg.rem_euclid(360.0) + 7.5) / 15.0) as usize % 24
}

/// 12 undirected 15-degree bins (mod 180), bin `i` centred on `15 i` degrees.
pub fn bin12(deg: f64) -> usize {
    ((deg.rem_euclid(180.0) + 7.5) / 15.0) as usize % 12
}

/// Share of samples in the axis-aligned bins, in the 45-degree bins, and
/// the combined share divided by the share a rotation-invariant (uniform)
/// distribution puts there.
pub struct Shares {
    pub axis: f64,
    pub diag: f64,
    pub ratio: f64,
}

/// Computes [`Shares`] for 24 (directed) or 12 (undirected) bins.
pub fn shares(counts: &[u64]) -> Shares {
    let n = counts.len();
    let step = 360.0 / n as f64;
    let step = if n == 12 { 15.0 } else { step };
    let total: u64 = counts.iter().sum();
    let (mut axis, mut diag, mut nb) = (0_u64, 0_u64, 0_usize);
    for (i, &c) in counts.iter().enumerate() {
        let deg = (i as f64 * step).rem_euclid(90.0);
        if deg < 1e-9 {
            axis += c;
            nb += 1;
        } else if (deg - 45.0).abs() < 1e-9 {
            diag += c;
            nb += 1;
        }
    }
    let t = total.max(1) as f64;
    Shares {
        axis: axis as f64 / t,
        diag: diag as f64 / t,
        ratio: (axis + diag) as f64 / t / (nb as f64 / n as f64),
    }
}
