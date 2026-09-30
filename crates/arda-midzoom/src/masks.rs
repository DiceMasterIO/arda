//! Saved-cell factors that scale refinement (logic/17 §amplitude):
//! ground cover, watercourses and standing water, bilinear between cell
//! centres (lattice node 0 is the centre of cell 0, vocabulary I1).

use crate::fixed::ONE;
use crate::source::{Terrain, CELL_UM};
use crate::MidzoomError;
use arda_core::{Cover, TerrainKind};

/// Per-cell factors over a window of saved cells, Q12.
#[derive(Debug, Clone)]
pub struct CellMasks {
    gx0: i64,
    gy0: i64,
    w: usize,
    h: usize,
    /// How much sub-39 m relief the cover shows.
    cover: Vec<i64>,
    /// 1 on cells with a watercourse.
    river: Vec<i64>,
    /// 1 on sea and lake cells.
    water: Vec<i64>,
}

/// Relief a cover shows, Q12: bare rock and ice keep every rib; soil,
/// turf and canopy mantle the ground; marsh stays flat.
const fn cover_factor(cover: Cover) -> i64 {
    match cover {
        Cover::Bare | Cover::Rock | Cover::Ice => ONE,
        Cover::Scrub => 3_686,
        Cover::Grass => 3_277,
        Cover::Forest => 3_072,
        Cover::Marsh => 819,
    }
}

impl CellMasks {
    /// Reads cells covering lattice positions `[x0_um, x1_um] × [y0_um, y1_um]`.
    ///
    /// # Errors
    /// A saved area failed to read.
    pub fn read(
        src: &dyn Terrain,
        x0_um: i64,
        y0_um: i64,
        x1_um: i64,
        y1_um: i64,
    ) -> Result<Self, MidzoomError> {
        let gx0 = x0_um.div_euclid(CELL_UM) - 1;
        let gy0 = y0_um.div_euclid(CELL_UM) - 1;
        let gx1 = x1_um.div_euclid(CELL_UM) + 2;
        let gy1 = y1_um.div_euclid(CELL_UM) + 2;
        let w = usize::try_from(gx1 - gx0 + 1).map_err(|_| MidzoomError::Window("cells".into()))?;
        let h = usize::try_from(gy1 - gy0 + 1).map_err(|_| MidzoomError::Window("cells".into()))?;
        let mut cover = Vec::with_capacity(w * h);
        let mut river = Vec::with_capacity(w * h);
        let mut water = Vec::with_capacity(w * h);
        for gy in gy0..=gy1 {
            for gx in gx0..=gx1 {
                let c = src.cell(gx, gy)?;
                cover.push(cover_factor(c.cover));
                river.push(if c.river_width_dm > 0 { ONE } else { 0 });
                water.push(if c.terrain == TerrainKind::Land {
                    0
                } else {
                    ONE
                });
            }
        }
        Ok(Self {
            gx0,
            gy0,
            w,
            h,
            cover,
            river,
            water,
        })
    }

    fn bilinear(&self, field: &[i64], x_um: i64, y_um: i64) -> i64 {
        let (gx, fx) = (x_um.div_euclid(CELL_UM), x_um.rem_euclid(CELL_UM));
        let (gy, fy) = (y_um.div_euclid(CELL_UM), y_um.rem_euclid(CELL_UM));
        let at = |x: i64, y: i64| -> i128 {
            let xi = (x - self.gx0).clamp(0, i64::try_from(self.w).unwrap_or(1) - 1);
            let yi = (y - self.gy0).clamp(0, i64::try_from(self.h).unwrap_or(1) - 1);
            usize::try_from(yi * i64::try_from(self.w).unwrap_or(0) + xi)
                .ok()
                .and_then(|i| field.get(i))
                .map_or(0, |v| i128::from(*v))
        };
        let (fx, fy, c) = (i128::from(fx), i128::from(fy), i128::from(CELL_UM));
        let top = at(gx, gy) * (c - fx) + at(gx + 1, gy) * fx;
        let bottom = at(gx, gy + 1) * (c - fx) + at(gx + 1, gy + 1) * fx;
        i64::try_from((top * (c - fy) + bottom * fy) / (c * c)).unwrap_or(0)
    }

    /// Cover factor at a lattice position, Q12.
    #[must_use]
    pub fn cover(&self, x_um: i64, y_um: i64) -> i64 {
        self.bilinear(&self.cover, x_um, y_um)
    }

    /// Watercourse presence at a lattice position, Q12.
    #[must_use]
    pub fn river(&self, x_um: i64, y_um: i64) -> i64 {
        self.bilinear(&self.river, x_um, y_um)
    }

    /// Standing-water presence at a lattice position, Q12.
    #[must_use]
    pub fn water(&self, x_um: i64, y_um: i64) -> i64 {
        self.bilinear(&self.water, x_um, y_um)
    }
}
