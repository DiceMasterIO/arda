//! `WorldQuery`: the contract over a stored world, with bounded caches.
//!
//! Every sample for an area goes through the same cached area, derived layer
//! and fine window, so `/cell`, `/area/.../cells` and `/point` always agree.

use crate::cache::ByteLru;
use crate::contract::convert::{self, CellPlace, Derived};
use crate::contract::{CellSample, HeightSource, PointSample, CELL_M, CONTRACT_VERSION};
use crate::derived::{self, AreaDerived, Neighbourhood};
use crate::error::{lock, ServerError, ServerResult};
use crate::fine;
use arda::{Area, World};
use arda_core::{AreaCoord, TerrainField, TerrainFileReader, AREA_CELLS};
use rayon::prelude::*;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

const SIDE: u32 = AREA_CELLS as u32;

/// Byte budgets for the query caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLimits {
    /// Decoded areas (about 13 MiB each).
    pub area_bytes: usize,
    /// Derived coast/river/lake layers (about 3.3 MiB each).
    pub derived_bytes: usize,
    /// Fine-terrain windows (about 7 MiB each).
    pub fine_bytes: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            area_bytes: 1 << 30,
            derived_bytes: 256 << 20,
            fine_bytes: 512 << 20,
        }
    }
}

impl QueryLimits {
    /// Sum of all budgets.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.area_bytes + self.derived_bytes + self.fine_bytes
    }
}

/// Read-only contract queries over one stored world.
pub struct WorldQuery {
    world: World,
    cells_wide: u32,
    cells_high: u32,
    fine: Option<Mutex<TerrainFileReader<File>>>,
    areas: Mutex<ByteLru<AreaCoord, Area>>,
    derived: Mutex<ByteLru<AreaCoord, AreaDerived>>,
    windows: Mutex<ByteLru<AreaCoord, TerrainField>>,
}

fn area_bytes(area: &Area) -> usize {
    let cells =
        usize::from(AREA_CELLS) * usize::from(AREA_CELLS) * std::mem::size_of::<arda::Cell>();
    let rivers: usize = area.rivers().iter().map(|r| 64 + r.course.len() * 4).sum();
    let lakes: usize = area.lakes().iter().map(|l| 64 + l.cells.len() * 4).sum();
    let edges = std::mem::size_of_val(area.channel_edges());
    // Copied global hydrology context is bounded by the objects codec; count a flat MiB.
    cells + rivers + lakes + edges + (1 << 20)
}

impl WorldQuery {
    /// Opens the world at `dir` and verifies its fine terrain, if declared.
    ///
    /// # Errors
    /// Manifest, layer or fine-terrain verification failures.
    pub fn open(dir: &Path, limits: QueryLimits) -> ServerResult<Self> {
        let world = World::load(dir)?;
        let fine = world.fine_terrain(fine::FINE_READER_BYTES)?.map(Mutex::new);
        let m = world.manifest();
        let span = |areas: i32| {
            u32::try_from(areas)
                .ok()
                .and_then(|a| a.checked_mul(SIDE))
                .ok_or_else(|| ServerError::Internal("area grid overflow".into()))
        };
        let (cells_wide, cells_high) = (span(m.areas_wide)?, span(m.areas_high)?);
        Ok(Self {
            world,
            cells_wide,
            cells_high,
            fine,
            areas: Mutex::new(ByteLru::new(limits.area_bytes)),
            derived: Mutex::new(ByteLru::new(limits.derived_bytes)),
            windows: Mutex::new(ByteLru::new(limits.fine_bytes)),
        })
    }

    /// The stored world.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Global cell grid size `(columns, rows)`.
    #[must_use]
    pub const fn cells(&self) -> (u32, u32) {
        (self.cells_wide, self.cells_high)
    }

    /// Whether the world has canonical fine terrain.
    #[must_use]
    pub const fn has_fine(&self) -> bool {
        self.fine.is_some()
    }

    /// Validates a global cell and names its area and local coordinates.
    ///
    /// # Errors
    /// [`ServerError::OutOfRange`] outside the global grid.
    pub fn place(&self, gx: u32, gy: u32) -> ServerResult<CellPlace> {
        if gx >= self.cells_wide || gy >= self.cells_high {
            return Err(ServerError::OutOfRange(format!(
                "cell {gx},{gy} is outside the world ({},{} is the last)",
                self.cells_wide - 1,
                self.cells_high - 1
            )));
        }
        let int = |v: u32| i32::try_from(v).map_err(|_| ServerError::Internal("coordinate".into()));
        let local = |v: u32| {
            u16::try_from(v % SIDE).map_err(|_| ServerError::Internal("coordinate".into()))
        };
        Ok(CellPlace {
            gx,
            gy,
            ax: int(gx / SIDE)?,
            ay: int(gy / SIDE)?,
            cx: local(gx)?,
            cy: local(gy)?,
        })
    }

    /// The cached decoded area.
    ///
    /// # Errors
    /// Out-of-world coordinates or layer read failures.
    pub fn area(&self, ax: i32, ay: i32) -> ServerResult<Arc<Area>> {
        let at = AreaCoord::new(ax, ay);
        if let Some(hit) = lock(&self.areas)?.get(&at) {
            return Ok(hit);
        }
        let area = Arc::new(self.world.read_area(ax, ay)?);
        let bytes = area_bytes(&area);
        Ok(lock(&self.areas)?.insert(at, area, bytes))
    }

    fn inside(&self, ax: i32, ay: i32) -> bool {
        let m = self.world.manifest();
        (0..m.areas_wide).contains(&ax) && (0..m.areas_high).contains(&ay)
    }

    /// The cached derived layer of an area (loads its 8 neighbours on a miss).
    ///
    /// # Errors
    /// Layer read failures.
    pub fn derived(&self, ax: i32, ay: i32) -> ServerResult<Arc<AreaDerived>> {
        let at = AreaCoord::new(ax, ay);
        if let Some(hit) = lock(&self.derived)?.get(&at) {
            return Ok(hit);
        }
        let mut hood: Neighbourhood = Default::default();
        for (dy, row) in hood.iter_mut().enumerate() {
            for (dx, slot) in row.iter_mut().enumerate() {
                let (nx, ny) = (
                    ax + i32::try_from(dx).unwrap_or(1) - 1,
                    ay + i32::try_from(dy).unwrap_or(1) - 1,
                );
                if self.inside(nx, ny) {
                    *slot = Some(self.area(nx, ny)?);
                }
            }
        }
        let centre = hood[1][1].clone().ok_or_else(|| {
            ServerError::OutOfRange(format!("area {ax},{ay} is outside the world"))
        })?;
        let layer = Arc::new(derived::derive(&hood, &centre));
        let bytes = layer.bytes();
        Ok(lock(&self.derived)?.insert(at, layer, bytes))
    }

    /// The cached fine window of an area; `None` for worlds without fine terrain.
    ///
    /// # Errors
    /// Fine-terrain read failures or the window resource limit.
    pub fn fine_window(&self, ax: i32, ay: i32) -> ServerResult<Option<Arc<TerrainField>>> {
        let Some(reader) = &self.fine else {
            return Ok(None);
        };
        let at = AreaCoord::new(ax, ay);
        if let Some(hit) = lock(&self.windows)?.get(&at) {
            return Ok(Some(hit));
        }
        let field = Arc::new(fine::read_window(&mut *lock(reader)?, at)?);
        let bytes = fine::window_bytes(&field);
        Ok(Some(lock(&self.windows)?.insert(at, field, bytes)))
    }

    fn assemble(
        place: CellPlace,
        area: &Area,
        layer: &AreaDerived,
        window: Option<&TerrainField>,
    ) -> ServerResult<CellSample> {
        let cell = area.cell(place.cx, place.cy)?;
        let i = usize::from(place.cy) * usize::from(AREA_CELLS) + usize::from(place.cx);
        let pick = |of: &[u32]| {
            of.get(i)
                .and_then(|&n| n.checked_sub(1))
                .and_then(|n| usize::try_from(n).ok())
        };
        let derived = Derived {
            coast: Some(layer.coast.sample(i)),
            river: pick(&layer.river_of)
                .and_then(|n| area.rivers().get(n))
                .map(convert::river),
            lake: pick(&layer.lake_of)
                .and_then(|n| area.lakes().get(n))
                .map(|l| convert::lake(l, cell.height)),
            fine: window.and_then(|w| fine::footprint(w, place.gx, place.gy)),
        };
        Ok(convert::cell_sample(cell, place, derived))
    }

    /// The contract sample of global cell `(gx, gy)`.
    ///
    /// # Errors
    /// Out-of-world coordinates or layer failures.
    pub fn cell(&self, gx: u32, gy: u32) -> ServerResult<CellSample> {
        let place = self.place(gx, gy)?;
        let area = self.area(place.ax, place.ay)?;
        let layer = self.derived(place.ax, place.ay)?;
        let window = self.fine_window(place.ax, place.ay)?;
        Self::assemble(place, &area, &layer, window.as_deref())
    }

    /// All 512 × 512 samples of an area, row-major.
    ///
    /// # Errors
    /// Out-of-world coordinates, layer failures or allocation refusal.
    pub fn area_samples(&self, ax: i32, ay: i32) -> ServerResult<Vec<CellSample>> {
        if !self.inside(ax, ay) {
            return Err(ServerError::OutOfRange(format!(
                "area {ax},{ay} is outside the world"
            )));
        }
        let area = self.area(ax, ay)?;
        let layer = self.derived(ax, ay)?;
        let window = self.fine_window(ax, ay)?;
        let origin = |a: i32| u32::try_from(a).map_err(|_| ServerError::Internal("area".into()));
        let (gx0, gy0) = (origin(ax)? * SIDE, origin(ay)? * SIDE);
        let rows: Vec<ServerResult<Vec<CellSample>>> = (0..SIDE)
            .into_par_iter()
            .map(|cy| {
                (0..SIDE)
                    .map(|cx| {
                        let place = self.place(gx0 + cx, gy0 + cy)?;
                        Self::assemble(place, &area, &layer, window.as_deref())
                    })
                    .collect()
            })
            .collect();
        let mut out = Vec::new();
        out.try_reserve_exact(usize::from(AREA_CELLS) * usize::from(AREA_CELLS))
            .map_err(|_| ServerError::ResourceLimit("area samples allocation".into()))?;
        for row in rows {
            out.extend(row?);
        }
        Ok(out)
    }

    fn cell_height_m(&self, gx: u32, gy: u32) -> ServerResult<f64> {
        let place = self.place(gx, gy)?;
        let area = self.area(place.ax, place.ay)?;
        Ok(convert::height_m(area.cell(place.cx, place.cy)?.height))
    }

    /// Height at arbitrary world metres plus the sample of the cell that
    /// contains the point (`floor(x/100)`, `floor(y/100)`; vocabulary I1).
    ///
    /// Fine worlds interpolate the canonical fine terrain bilinearly (clamped
    /// to its coverage); older worlds interpolate the four surrounding cell
    /// centres.
    ///
    /// # Errors
    /// Non-finite or out-of-world positions, or layer failures.
    pub fn sample_point(&self, x_m: f64, y_m: f64) -> ServerResult<PointSample> {
        let end_x = f64::from(self.cells_wide) * CELL_M;
        let end_y = f64::from(self.cells_high) * CELL_M;
        if !(x_m.is_finite() && y_m.is_finite()) {
            return Err(ServerError::BadRequest(
                "x_m and y_m must be finite numbers".into(),
            ));
        }
        if !(0.0..end_x).contains(&x_m) || !(0.0..end_y).contains(&y_m) {
            return Err(ServerError::OutOfRange(format!(
                "point {x_m},{y_m} m is outside the world [0, {end_x}) × [0, {end_y})"
            )));
        }
        // In range and non-negative, so the floored casts cannot truncate or lose sign.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let containing = |v: f64| (v / CELL_M).floor() as u32;
        let cell = self.cell(containing(x_m), containing(y_m))?;
        let window = self.fine_window(cell.ax, cell.ay)?;
        #[allow(clippy::cast_possible_truncation)]
        let um = |v: f64| (v * 1e6).round() as i64;
        let (height_m, height_source) = match window {
            Some(w) => {
                let h = fine::point_clamped(&w, um(x_m), um(y_m));
                (convert::height_m(h), HeightSource::Fine)
            }
            None => (self.cell_bilinear(x_m, y_m)?, HeightSource::Cells),
        };
        Ok(PointSample {
            contract_version: CONTRACT_VERSION,
            x_m,
            y_m,
            height_m,
            height_source,
            cell,
        })
    }

    /// Bilinear height between the four cell centres around a point; the
    /// outermost half cell holds the edge centres' values.
    fn cell_bilinear(&self, x_m: f64, y_m: f64) -> ServerResult<f64> {
        let axis = |v: f64, count: u32| -> (u32, u32, f64) {
            let u = v / CELL_M - 0.5;
            let last = count.saturating_sub(1);
            // Callers validated 0 <= v < count·100, so the floor is in range.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i0 = (u.floor().max(0.0) as u32).min(last);
            let i1 = (i0 + 1).min(last);
            (i0, i1, (u - f64::from(i0)).clamp(0.0, 1.0))
        };
        let (x0, x1, fx) = axis(x_m, self.cells_wide);
        let (y0, y1, fy) = axis(y_m, self.cells_high);
        let top = self.cell_height_m(x0, y0)? * (1.0 - fx) + self.cell_height_m(x1, y0)? * fx;
        let bottom = self.cell_height_m(x0, y1)? * (1.0 - fx) + self.cell_height_m(x1, y1)? * fx;
        Ok(top * (1.0 - fy) + bottom * fy)
    }
}

impl std::fmt::Debug for WorldQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldQuery")
            .field("world", &self.world)
            .field("fine", &self.fine.is_some())
            .finish_non_exhaustive()
    }
}
