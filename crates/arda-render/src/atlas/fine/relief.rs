//! Close-zoom relief (logic/17 §render): the recipe-5 formed shader at
//! one point whose finest geometry comes from a refined lattice supplied by
//! the caller, so palette, light, water and climate match the overview at
//! the switch-over. Coast and lake shores stay on the stored field and the
//! stored shore layer.

use super::{
    lake_surface_near, saved_material, saved_temperature, shore_near, FineAtlas, SavedFields,
    SavedMaterial, CELL_UM, KM_UM,
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

    /// Relief colour at an absolute fine-lattice position: water and coast
    /// from the stored field, land lit with the refined geometry. The flag
    /// is true for land.
    pub(in crate::atlas) fn relief_colour(
        &self,
        x: i128,
        y: i128,
        footprint_um: i128,
        geometry: ReliefGeometry,
        saved: &SavedFields<'_>,
    ) -> Result<([u8; 3], bool), RenderError> {
        if !self.formed {
            return Err(invalid("relief shading requires recipe-5 formed terrain"));
        }
        let point = self.clamp_source(x, y)?;
        let (qx, qy) = (i128::from(point.x_um), i128::from(point.y_um));
        let height = self.height(qx, qy)?;
        let surface = lake_surface_near(x, y, (self.origin_x, self.origin_y), saved.lake_surface)?;
        if let Some(s) = surface.filter(|&s| height < s) {
            return Ok((
                crate::atlas::formed::lake(i64::from(s) - i64::from(height), self.v6),
                false,
            ));
        }
        if height <= 0 {
            let shore_mix = shore_near(x, y, (self.origin_x, self.origin_y), saved.shore)?;
            return Ok((self.sea_colour(qx, qy, height, &shore_mix)?, false));
        }
        let (material, temperature) = self.land_inputs(x, y, geometry.height_mm, saved)?;
        Ok((
            self.formed_color(
                qx,
                qy,
                height,
                (material, temperature),
                footprint_um,
                Some(geometry),
            )?,
            true,
        ))
    }
}
