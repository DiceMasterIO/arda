//! Close-zoom relief (logic/17 §render): the recipe-5 formed shader at
//! one point whose finest geometry comes from a refined lattice supplied by
//! the caller, so palette, light, water and climate match the overview at
//! the switch-over. Coast and lake shores stay on the stored field and the
//! stored shore layer.

use super::{
    arid_near, lake_surface_near, saved_material, saved_temperature, shore_near, FineAtlas,
    SavedFields, SavedMaterial, CELL_UM, KM_UM,
};
use crate::atlas::invalid;
use crate::RenderError;

/// Local geometry of a refined surface at one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReliefGeometry {
    /// Refined height, millimetres.
    pub height_mm: i32,
    /// Refined gradient `(dz/dx, dz/dy)` as Q12 slopes.
    pub gradient_q12: (i64, i64),
    /// Refined-scale concavity, millimetres (positive in gullies), added to
    /// the stored 156 m ring term.
    pub concavity_mm: i64,
}

/// Which surface a relief pixel shows when the caller supplies the water
/// geometry (logic/17 §water): the tactical layer's own lakes, sea and
/// shores rather than the stored field's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliefSurface {
    /// Dry ground, lit with the refined geometry.
    Land,
    /// Lake water this deep, millimetres.
    Lake {
        /// Depth below the surface, millimetres.
        depth_mm: i64,
    },
    /// Sea water this deep, millimetres.
    Sea {
        /// Depth below sea level, millimetres.
        depth_mm: i64,
    },
}

impl FineAtlas {
    /// Saved land material and temperature at a position: bilinear saved
    /// cells, the 1 km moisture cross average (logic/04 §atlas-formed
    /// vegetation), the stored shore classes (logic/04 §atlas-formed shore)
    /// and the lapse to `height_mm`.
    pub(super) fn land_inputs(
        &self,
        px: i128,
        py: i128,
        height_mm: i32,
        saved: &SavedFields<'_>,
    ) -> Result<(SavedMaterial, i32), RenderError> {
        let origin = (self.origin_x, self.origin_y);
        let read = |x: i128, y: i128| {
            saved_material(
                x,
                y,
                origin,
                saved.classes,
                saved.wetness,
                saved.moisture,
                saved.forest_density,
            )
        };
        let base = read(px, py)?.unwrap_or(SavedMaterial {
            wetness: 0,
            moisture: 160,
            forest_density: 0,
            shore: [0; 8],
        });
        // Saved moisture steps at the 1 km climate grid; a 1 km cross
        // average turns those steps into ramps so canopy does not tile.
        let mut msum = u32::from(base.moisture);
        let mut mcount = 1_u32;
        for (ox, oy) in [(5, 0), (-5, 0), (0, 5), (0, -5)] {
            if let Some(m) = read(px + ox * CELL_UM, py + oy * CELL_UM)? {
                msum += u32::from(m.moisture);
                mcount += 1;
            }
        }
        let material = SavedMaterial {
            moisture: u8::try_from((msum + mcount / 2) / mcount).unwrap_or(255),
            shore: shore_near(px, py, origin, saved.shore)?,
            ..base
        };
        let temperature = saved_temperature(
            px,
            py,
            origin,
            saved.classes,
            (saved.temperature, saved.heights),
            height_mm,
        )?;
        Ok((material, temperature))
    }

    /// Seafloor colour at a clamped source position, with the stored
    /// shore classes' water tint (logic/04 §atlas-formed shore).
    pub(super) fn sea_colour(
        &self,
        qx: i128,
        qy: i128,
        height: i32,
        shore_mix: &[i64; 8],
    ) -> Result<[u8; 3], RenderError> {
        let sky = self
            .sky
            .as_ref()
            .ok_or_else(|| invalid("formed shading requires the 1 km context"))?;
        let r = KM_UM;
        let gx = Self::slope_q12(
            sky.context_height_mm(qx + r, qy)?,
            sky.context_height_mm(qx - r, qy)?,
            r,
        )?;
        let gy = Self::slope_q12(
            sky.context_height_mm(qx, qy + r)?,
            sky.context_height_mm(qx, qy - r)?,
            r,
        )?;
        Ok(crate::atlas::formed::shore_water(
            crate::atlas::formed::sea(-i64::from(height), (gx, gy)),
            -i64::from(height),
            shore_mix,
        ))
    }

    /// Relief colour at an absolute fine-lattice position for a surface
    /// the caller decided: lake and sea tints from the formed palette at
    /// the given depth, land lit with the refined geometry wherever the
    /// stored field would have drawn water or coast.
    pub(in crate::atlas) fn relief_surface_colour(
        &self,
        (x, y): (i128, i128),
        footprint_um: i128,
        geometry: ReliefGeometry,
        surface: ReliefSurface,
        saved: &SavedFields<'_>,
    ) -> Result<[u8; 3], RenderError> {
        if !self.formed {
            return Err(invalid("relief shading requires recipe-5 formed terrain"));
        }
        let point = self.clamp_source(x, y)?;
        let (qx, qy) = (i128::from(point.x_um), i128::from(point.y_um));
        // Depth tints follow the stored field wherever it holds water
        // there, so open water shades as smoothly as the overview's; the
        // caller's depth fills the margins only it calls water.
        let stored = self.height(qx, qy)?;
        match surface {
            ReliefSurface::Lake { depth_mm } => {
                let surface =
                    lake_surface_near(x, y, (self.origin_x, self.origin_y), saved.lake_surface)?;
                let depth = surface
                    .filter(|&s| stored < s)
                    .map_or(depth_mm, |s| i64::from(s) - i64::from(stored));
                // Recipe 7: terminal lakes are saline (logic/04 §atlas-formed
                // arid basins), as on the overview.
                let arid = arid_near(x, y, (self.origin_x, self.origin_y), saved)?;
                Ok(match arid {
                    Some(a) if a.saline > 0 => crate::atlas::formed::saline_lake(depth.max(0)),
                    _ => crate::atlas::formed::lake(depth.max(0), self.v6),
                })
            }
            ReliefSurface::Sea { depth_mm } => {
                let shore_mix = shore_near(x, y, (self.origin_x, self.origin_y), saved.shore)?;
                let height = if stored < 0 {
                    stored
                } else {
                    i32::try_from(-depth_mm.max(0)).unwrap_or(i32::MIN)
                };
                self.sea_colour(qx, qy, height, &shore_mix)
            }
            ReliefSurface::Land => {
                // The tactical shore decides land; its ground stands
                // above the water it borders, so the shading never reads
                // a sub-sea height there.
                let geometry = ReliefGeometry {
                    height_mm: geometry.height_mm.max(1),
                    ..geometry
                };
                let height = stored.max(1);
                let inputs = self.land_inputs(x, y, geometry.height_mm, saved)?;
                let c = self.formed_color(qx, qy, height, inputs, footprint_um, Some(geometry))?;
                // Recipe-7 salt pans: white crust and pale mudflat margins.
                Ok(
                    match arid_near(x, y, (self.origin_x, self.origin_y), saved)? {
                        Some(a) => crate::atlas::formed::pan(c, a.crust, a.mudflat),
                        None => c,
                    },
                )
            }
        }
    }
}
