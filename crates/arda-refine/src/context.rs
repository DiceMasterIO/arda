//! Everything one block reads from the world, gathered once: a 5 × 5 cell
//! neighbourhood, the channel edges of the 3 × 3 cells around the block,
//! and a window of the fine terrain lattice. Every query afterwards is a pure function of global
//! position, so two blocks asking about the same point get the same answer.

use crate::error::RefineError;
use crate::grid::span;
use crate::source::{CellKey, Edge, LakeInfo, Source};
use arda::{Cell, TerrainKind};

/// Squares per cell side.
pub const N: i64 = 64;
/// Cell neighbourhood radius gathered around the block's own cell. The
/// design reads the 3 × 3 neighbours; C1 (Catmull-Rom) interpolation of cell
/// fields at the block's three-square halo needs one more ring, and so do
/// the far ends of the neighbours' channel edges.
pub const R: i64 = 2;
/// Radius of cells whose channel pieces can reach into the block: a piece
/// stays inside its cell plus its bow and half-width (under 30 squares).
pub const EDGE_R: i64 = 1;
/// Extra squares of fine lattice beyond the channel cells.
const FINE_MARGIN: i64 = 24;
/// Fine lattice spacing in squares (39.0625 m / 1.5625 m).
pub const FINE_STEP: f64 = 25.0;
/// World square of fine-lattice node 0 on each axis: the I1 cell frame
/// puts the lattice origin at cell `(0, 0)`'s centre,
/// `arda_core::FINE_FRAME_OFFSET_UM` (50 m) in from the world corner, and
/// a square is 100/64 m (I2). Exactly 32.
#[allow(clippy::cast_precision_loss)] // 50 m in µm is exact in f64
pub const FINE_ORIGIN_SQ: f64 = arda_core::FINE_FRAME_OFFSET_UM as f64 * 64.0 / 100_000_000.0;
/// Metres per square (a 100 m cell is 64 squares).
pub const SQUARE_M: f64 = 100.0 / 64.0;

/// A block's gathered world data.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// World seed.
    pub seed: u64,
    /// The block's cell.
    pub cell: CellKey,
    cells: Vec<Cell>,
    lakes: Vec<Option<LakeInfo>>,
    /// Channel edges touching cells within [`EDGE_R`], sorted, unique.
    pub edges: Vec<Edge>,
    fine: Option<(i64, i64, i64, Vec<f64>)>,
}

/// Catmull-Rom weights for fraction `t`.
fn cr_weights(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        0.5 * (-t3 + 2.0 * t2 - t),
        0.5 * (3.0 * t3 - 5.0 * t2 + 2.0),
        0.5 * (-3.0 * t3 + 4.0 * t2 + t),
        0.5 * (t3 - t2),
    ]
}

/// Splits a lattice coordinate into its integer index and fraction.
fn split(t: f64) -> (i64, f64) {
    let f = t.floor();
    #[allow(clippy::cast_possible_truncation)]
    let i = f as i64;
    (i, t - f)
}

impl Ctx {
    /// Gathers the data for the block of `cell`.
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn gather(src: &dyn Source, cell: CellKey) -> Result<Self, RefineError> {
        let side = 2 * R + 1;
        let mut cells = Vec::with_capacity(usize::try_from(side * side).unwrap_or(0));
        let mut lakes = Vec::with_capacity(cells.capacity());
        for dy in -R..=R {
            for dx in -R..=R {
                let k = src.clamp(cell.offset(dx, dy));
                cells.push(src.cell(k)?);
                lakes.push(src.lake(k)?);
            }
        }
        let mut edges = Vec::new();
        for dy in -EDGE_R..=EDGE_R {
            for dx in -EDGE_R..=EDGE_R {
                edges.extend(src.edges_touching(cell.offset(dx, dy))?);
            }
        }
        edges.sort();
        edges.dedup();
        // Fine lattice covering the block and the channel cells around it
        // (one cell each way plus bows and warps), with the Catmull-Rom
        // support: every point a block evaluates lies inside, so two
        // blocks never see different (clamped) samples for one point.
        let u0 = cell.x * N;
        let v0 = cell.y * N;
        let k = |u: i64| split((u as f64 - FINE_ORIGIN_SQ) / FINE_STEP).0;
        let margin = EDGE_R * N + FINE_MARGIN;
        let (kx0, ky0) = (k(u0 - margin) - 1, k(v0 - margin) - 1);
        let (kx1, ky1) = (k(u0 + N + margin) + 2, k(v0 + N + margin) + 2);
        let w = kx1 - kx0 + 1;
        let mut fine = Vec::new();
        let mut have = true;
        'rows: for ky in ky0..=ky1 {
            for kx in kx0..=kx1 {
                match src.fine_mm(kx, ky)? {
                    Some(mm) => fine.push(f64::from(mm) / 1000.0),
                    None => {
                        have = false;
                        break 'rows;
                    }
                }
            }
        }
        Ok(Self {
            seed: src.seed(),
            cell,
            cells,
            lakes,
            edges,
            fine: have.then_some((kx0, ky0, w, fine)),
        })
    }

    fn slot(&self, c: CellKey) -> usize {
        let dx = (c.x - self.cell.x).clamp(-R, R) + R;
        let dy = (c.y - self.cell.y).clamp(-R, R) + R;
        usize::try_from(dy * (2 * R + 1) + dx).unwrap_or(0)
    }

    /// The gathered cell at global `c` (clamped to the neighbourhood).
    #[must_use]
    pub fn cell_at(&self, c: CellKey) -> &Cell {
        &self.cells[self.slot(c).min(self.cells.len() - 1)]
    }

    /// The lake at global `c`, if any.
    #[must_use]
    pub fn lake_at(&self, c: CellKey) -> Option<LakeInfo> {
        self.lakes.get(self.slot(c)).copied().flatten()
    }

    /// Whether `c` is standing water (sea or lake).
    #[must_use]
    pub fn is_water_cell(&self, c: CellKey) -> bool {
        self.cell_at(c).terrain != TerrainKind::Land
    }

    /// The cell containing global square position `(u, v)`.
    #[must_use]
    pub fn cell_of(u: f64, v: f64) -> CellKey {
        CellKey::new(split(u / 64.0).0, split(v / 64.0).0)
    }

    /// Whether the world had a fine terrain layer.
    #[must_use]
    pub const fn has_fine(&self) -> bool {
        self.fine.is_some()
    }

    /// Bilinear interpolation of a per-cell value between cell centres.
    pub fn bilinear(&self, u: f64, v: f64, f: impl Fn(&Cell, CellKey) -> f64) -> f64 {
        let (ix, fx) = split((u - 32.0) / 64.0);
        let (iy, fy) = split((v - 32.0) / 64.0);
        let at = |dx: i64, dy: i64| {
            let k = CellKey::new(ix + dx, iy + dy);
            f(self.cell_at(k), k)
        };
        let a = at(0, 0) + (at(1, 0) - at(0, 0)) * fx;
        let b = at(0, 1) + (at(1, 1) - at(0, 1)) * fx;
        a + (b - a) * fy
    }

    /// Catmull-Rom interpolation of a per-cell value between cell centres.
    pub fn bicubic(&self, u: f64, v: f64, f: impl Fn(&Cell, CellKey) -> f64) -> f64 {
        let (ix, fx) = split((u - 32.0) / 64.0);
        let (iy, fy) = split((v - 32.0) / 64.0);
        let (wx, wy) = (cr_weights(fx), cr_weights(fy));
        let mut sum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            let mut row = 0.0;
            for (i, wxi) in wx.iter().enumerate() {
                let k = CellKey::new(ix + span(i) - 1, iy + span(j) - 1);
                row += wxi * f(self.cell_at(k), k);
            }
            sum += wyj * row;
        }
        sum
    }

    /// Smooth base terrain in metres: Catmull-Rom over the fine lattice, or
    /// over cell heights when the world has no fine layer.
    #[must_use]
    pub fn base_height(&self, u: f64, v: f64) -> f64 {
        let Some((kx0, ky0, w, vals)) = &self.fine else {
            return self.bicubic(u, v, |c, _| f64::from(c.height.raw()) / 1000.0);
        };
        // World squares to lattice steps through the shared I1 cell frame
        // (`arda_core::FINE_FRAME_OFFSET_UM`), as the server's queries do.
        let (ix, fx) = split((u - FINE_ORIGIN_SQ) / FINE_STEP);
        let (iy, fy) = split((v - FINE_ORIGIN_SQ) / FINE_STEP);
        let (wx, wy) = (cr_weights(fx), cr_weights(fy));
        let h = span(vals.len()) / w.max(&1);
        let mut sum = 0.0;
        for (j, wyj) in wy.iter().enumerate() {
            let y = (iy + span(j) - 1 - ky0).clamp(0, h - 1);
            let mut row = 0.0;
            for (i, wxi) in wx.iter().enumerate() {
                let x = (ix + span(i) - 1 - kx0).clamp(0, w - 1);
                let s = usize::try_from(y * w + x).unwrap_or(0);
                row += wxi * vals.get(s).copied().unwrap_or(0.0);
            }
            sum += wyj * row;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_rom_weights_sum_to_one_and_interpolate() {
        for i in 0..=10 {
            let w = cr_weights(f64::from(i) / 10.0);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        }
        assert_eq!(cr_weights(0.0), [0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn split_handles_negative_coordinates() {
        assert_eq!(split(-0.25), (-1, 0.75));
        assert_eq!(split(3.5), (3, 0.5));
    }
}
