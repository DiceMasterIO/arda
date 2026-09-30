//! A settlement's or window's surroundings sampled from the world's cells
//! (vocabulary I1: cell `(gx, gy)` covers `[100·gx, 100·gx + 100)` m, its
//! values sit at the centre `100·gx + 50`).
//!
//! Every quantity is a pure function of world metres and the cells around
//! it, so two windows or plans that sample the same point agree (goal 46).

use crate::PeopleError;
use arda::TerrainKind;
use arda_refine::source::Edge;
use arda_refine::{CellKey, Source};

/// Cell size, metres.
pub const CELL_M: f64 = 100.0;

/// A straight river piece between two cell centres: `(from, to, width_m)`.
pub type RiverPiece = ((f64, f64), (f64, f64), f64);

/// World cells around a point, with the river channels among them.
#[derive(Debug, Clone)]
pub struct Surroundings {
    /// First cell column and row.
    pub origin: (i64, i64),
    /// Cells across and down.
    pub size: (i64, i64),
    /// Ground or water-surface height per cell, metres.
    pub height_m: Vec<f64>,
    /// Open water (sea or lake) per cell.
    pub open_water: Vec<bool>,
    /// River channels touching the cells, sorted and deduplicated.
    pub edges: Vec<Edge>,
    /// The channels as straight pieces between cell centres.
    pub pieces: Vec<RiverPiece>,
    /// Per cell, the pieces whose channel (half its width, at least 1 m)
    /// reaches into it.
    pub bins: Vec<Vec<u32>>,
}

/// Narrowest channel drawn, metres.
pub const MIN_CHANNEL_M: f64 = 2.0;

/// World metres of a cell's centre.
#[must_use]
pub fn centre_m(c: CellKey) -> (f64, f64) {
    #[allow(clippy::cast_precision_loss)] // cell indices are far below 2^52
    let (x, y) = (c.x as f64, c.y as f64);
    (x * CELL_M + CELL_M / 2.0, y * CELL_M + CELL_M / 2.0)
}

impl Surroundings {
    /// The cells within `radius_m` of `(x_m, y_m)` (clamped into the world).
    ///
    /// # Errors
    /// [`PeopleError::World`] when a layer fails to load.
    pub fn around(
        src: &dyn Source,
        (x_m, y_m): (f64, f64),
        radius_m: f64,
    ) -> Result<Self, PeopleError> {
        let (w, h) = src.cells_wide_high();
        #[allow(clippy::cast_possible_truncation)] // world metres fit i64
        let cell = |m: f64| (m / CELL_M).floor() as i64;
        let (x0, x1) = (
            cell(x_m - radius_m).clamp(0, w - 1),
            cell(x_m + radius_m).clamp(0, w - 1),
        );
        let (y0, y1) = (
            cell(y_m - radius_m).clamp(0, h - 1),
            cell(y_m + radius_m).clamp(0, h - 1),
        );
        let world = |e: arda_refine::RefineError| PeopleError::World(e.to_string());
        let mut height_m = Vec::new();
        let mut open_water = Vec::new();
        let mut edges = Vec::new();
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let k = CellKey::new(gx, gy);
                let c = src.cell(k).map_err(world)?;
                let lake = src.lake(k).map_err(world)?;
                let ground = f64::from(c.height.raw()) / 1000.0;
                height_m.push(lake.map_or(ground, |l| f64::from(l.surface_mm) / 1000.0));
                open_water.push(matches!(c.terrain, TerrainKind::Sea | TerrainKind::Lake));
                edges.extend(src.edges_touching(k).map_err(world)?);
            }
        }
        edges.sort();
        edges.dedup();
        let pieces = edges
            .iter()
            .map(|e| {
                let width_dm = e.from_width_dm.max(e.to_width_dm);
                (centre_m(e.from), centre_m(e.to), f64::from(width_dm) / 10.0)
            })
            .collect();
        let mut s = Self {
            origin: (x0, y0),
            size: (x1 - x0 + 1, y1 - y0 + 1),
            height_m,
            open_water,
            edges,
            pieces,
            bins: Vec::new(),
        };
        s.bin_pieces();
        Ok(s)
    }

    fn bin_pieces(&mut self) {
        let n = usize::try_from(self.size.0 * self.size.1).unwrap_or(0);
        let mut bins = vec![Vec::new(); n];
        #[allow(clippy::cast_possible_truncation)]
        let cell = |m: f64| (m / CELL_M).floor() as i64;
        for (k, &(a, b, w)) in self.pieces.iter().enumerate() {
            let r = w.max(MIN_CHANNEL_M) / 2.0 + 1.0;
            let (xa, xb) = (cell(a.0.min(b.0) - r), cell(a.0.max(b.0) + r));
            let (ya, yb) = (cell(a.1.min(b.1) - r), cell(a.1.max(b.1) + r));
            for gy in ya.max(self.origin.1)..=yb.min(self.origin.1 + self.size.1 - 1) {
                for gx in xa.max(self.origin.0)..=xb.min(self.origin.0 + self.size.0 - 1) {
                    if let Some(bin) = bins.get_mut(self.index(gx, gy)) {
                        bin.push(u32::try_from(k).unwrap_or(u32::MAX));
                    }
                }
            }
        }
        self.bins = bins;
    }

    /// The width of the river channel covering the point (within half its
    /// width, at least [`MIN_CHANNEL_M`], of a centreline), if any.
    #[must_use]
    pub fn river_at(&self, x_m: f64, y_m: f64) -> Option<f64> {
        #[allow(clippy::cast_possible_truncation)]
        let (gx, gy) = ((x_m / CELL_M).floor() as i64, (y_m / CELL_M).floor() as i64);
        let inside = (self.origin.0..self.origin.0 + self.size.0).contains(&gx)
            && (self.origin.1..self.origin.1 + self.size.1).contains(&gy);
        if !inside {
            return None;
        }
        self.bins.get(self.index(gx, gy))?.iter().find_map(|&k| {
            let &(a, b, w) = self.pieces.get(usize::try_from(k).ok()?)?;
            let w = w.max(MIN_CHANNEL_M);
            (segment_distance((x_m, y_m), a, b) <= w / 2.0).then_some(w)
        })
    }

    fn index(&self, gx: i64, gy: i64) -> usize {
        let x = (gx - self.origin.0).clamp(0, self.size.0 - 1);
        let y = (gy - self.origin.1).clamp(0, self.size.1 - 1);
        usize::try_from(y * self.size.0 + x).unwrap_or(0)
    }

    /// Height in metres: bilinear between cell centres.
    #[must_use]
    pub fn height_m(&self, x_m: f64, y_m: f64) -> f64 {
        let u = (x_m - CELL_M / 2.0) / CELL_M;
        let v = (y_m - CELL_M / 2.0) / CELL_M;
        let (fx, fy) = (u - u.floor(), v - v.floor());
        #[allow(clippy::cast_possible_truncation)]
        let (gx, gy) = (u.floor() as i64, v.floor() as i64);
        let at = |dx: i64, dy: i64| {
            self.height_m
                .get(self.index(gx + dx, gy + dy))
                .copied()
                .unwrap_or(0.0)
        };
        let top = at(0, 0) + (at(1, 0) - at(0, 0)) * fx;
        let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * fx;
        top + (bottom - top) * fy
    }

    /// Whether the cell containing the point is sea or lake.
    #[must_use]
    pub fn open_water_at(&self, x_m: f64, y_m: f64) -> bool {
        #[allow(clippy::cast_possible_truncation)]
        let (gx, gy) = ((x_m / CELL_M).floor() as i64, (y_m / CELL_M).floor() as i64);
        self.open_water
            .get(self.index(gx, gy))
            .copied()
            .unwrap_or(false)
    }

    /// River channels as straight pieces between cell centres: `(from, to,
    /// width_m)`.
    #[must_use]
    pub fn river_pieces(&self) -> &[RiverPiece] {
        &self.pieces
    }

    /// Distance from the point to the nearest river centreline, and that
    /// river's width; `None` without rivers.
    #[must_use]
    pub fn nearest_river(&self, x_m: f64, y_m: f64) -> Option<(f64, f64)> {
        self.pieces
            .iter()
            .map(|&(a, b, w)| (segment_distance((x_m, y_m), a, b), w))
            .min_by(|p, q| p.0.total_cmp(&q.0))
    }
}

/// Distance from `p` to the segment `a`–`b`.
#[must_use]
pub fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distance_clamps_to_the_ends() {
        assert!((segment_distance((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)) - 3.0).abs() < 1e-12);
        assert!((segment_distance((-4.0, 3.0), (0.0, 0.0), (10.0, 0.0)) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn heights_interpolate_between_cell_centres() {
        let s = Surroundings {
            origin: (0, 0),
            size: (2, 1),
            height_m: vec![10.0, 20.0],
            open_water: vec![false, true],
            edges: Vec::new(),
            pieces: Vec::new(),
            bins: vec![Vec::new(); 2],
        };
        assert!((s.height_m(50.0, 50.0) - 10.0).abs() < 1e-12);
        assert!((s.height_m(100.0, 50.0) - 15.0).abs() < 1e-12);
        assert!(s.open_water_at(150.0, 10.0) && !s.open_water_at(99.0, 10.0));
    }
}
