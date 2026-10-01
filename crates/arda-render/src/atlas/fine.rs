//! Fine land material from one immutable, absolute-coordinate terrain window.

use arda_core::{AreaCoord, TerrainField, TerrainKind, TerrainPoint};

mod relief;
mod saved;
mod shading;
mod sky;
pub use relief::{ReliefGeometry, ReliefSurface};
use saved::{
    arid_near, axis_positions, equivalent_delta, lake_surface_near, position, saved_material,
    saved_temperature, shore_near,
};
use sky::SkyShade;

use super::{invalid, land_material_ecology, modulate, relief_light, AxisKernel, RenderError};

const CELL_UM: i128 = 100_000_000;
const AREA_UM: i128 = 512 * CELL_UM;
const KM_UM: i128 = 1_000_000_000;
/// Recipe-5 mid-scale gradient baseline in fine steps (8 x 39.0625 = 312.5 m).
pub(super) const BROAD_STEPS: i128 = 8;
/// Recipe-5 concavity ring radius in fine steps (156.25 m).
const RING_STEPS: i128 = 4;

/// Saved per-cell fields of the 514² sample grid used by formed shading.
#[derive(Clone, Copy)]
pub(super) struct SavedFields<'a> {
    pub classes: &'a [TerrainKind],
    pub wetness: &'a [u8],
    pub moisture: &'a [u8],
    pub forest_density: &'a [u8],
    pub temperature: &'a [i16],
    pub heights: &'a [i32],
    pub lake_surface: &'a [i32],
    pub shore: &'a [u8],
    /// Recipe-7 arid water codes per sample; empty for earlier recipes.
    pub salt: &'a [u8],
}

#[derive(Clone, Copy, Default)]
struct SavedMaterial {
    wetness: u8,
    moisture: u8,
    forest_density: u8,
    /// Stored shore-class weights (Q12) by class; zero without a layer.
    shore: [i64; 8],
}

/// Closed physical coverage of the canonical fine source, in micrometres.
///
/// These are the first and last fine source nodes, which may extend past the
/// saved-world edge. They are not the bounds of an area window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasFineWorldBounds {
    /// Northwest source node.
    pub min: TerrainPoint,
    /// Southeast source node.
    pub max: TerrainPoint,
}

#[derive(Debug)]
pub(super) struct FineAtlas {
    /// Recipe-5 multi-scale shading (logic/04 §atlas-formed).
    formed: bool,
    /// The recipe-6 look of formed shading; false draws a recipe-5 world
    /// exactly as v0.1 did (logic/04 §atlas-formed recipes).
    v6: bool,
    field: TerrainField,
    origin_x: i128,
    origin_y: i128,
    bounds: AtlasFineWorldBounds,
    // North, east, south, west are actual saved-world edges from AtlasHalo.
    edges: [bool; 4],
    sky: Option<SkyShade>,
}

impl FineAtlas {
    pub(super) fn new(
        area: AreaCoord,
        bounds: AtlasFineWorldBounds,
        field: TerrainField,
        edges: [bool; 4],
        sky_context: Option<TerrainField>,
        formed: bool,
    ) -> Result<Self, RenderError> {
        if formed && sky_context.is_none() {
            return Err(invalid("formed shading requires the 1 km context"));
        }
        if bounds.min.x_um > bounds.max.x_um || bounds.min.y_um > bounds.max.y_um {
            return Err(invalid("fine source bounds are reversed"));
        }
        let origin_x = i128::from(area.x) * AREA_UM;
        let origin_y = i128::from(area.y) * AREA_UM;
        let last_x = origin_x + 511 * CELL_UM;
        let last_y = origin_y + 511 * CELL_UM;
        if origin_x < i128::from(bounds.min.x_um)
            || origin_y < i128::from(bounds.min.y_um)
            || last_x > i128::from(bounds.max.x_um)
            || last_y > i128::from(bounds.max.y_um)
        {
            return Err(invalid("saved area centers lie outside fine source"));
        }
        let this = Self {
            formed,
            v6: true,
            field,
            origin_x,
            origin_y,
            bounds,
            edges,
            sky: sky_context
                .map(|context| SkyShade::new(area, &context))
                .transpose()?,
        };
        this.validate_coverage()?;
        Ok(this)
    }

    /// The possible 32K Linear centers extend by almost half a saved cell;
    /// a fine-spacing derivative extends the required window once more.
    fn validate_coverage(&self) -> Result<(), RenderError> {
        let spacing =
            i128::from(self.field.spacing_um()) * if self.formed { BROAD_STEPS } else { 1 };
        let extremes_x = [
            self.origin_x - CELL_UM / 2 - spacing,
            self.origin_x + 511 * CELL_UM + CELL_UM / 2 + spacing,
        ];
        let extremes_y = [
            self.origin_y - CELL_UM / 2 - spacing,
            self.origin_y + 511 * CELL_UM + CELL_UM / 2 + spacing,
        ];
        for &x in &extremes_x {
            for &y in &extremes_y {
                let point = self.clamp_source(x, y)?;
                if self.field.sample(point).is_none() {
                    return Err(invalid("fine window omits pixel or derivative halo"));
                }
            }
        }
        Ok(())
    }

    fn clamp_source(&self, x: i128, y: i128) -> Result<TerrainPoint, RenderError> {
        let x = if x < i128::from(self.bounds.min.x_um) {
            if !self.edges[3] {
                return Err(invalid("fine query west of in-world source"));
            }
            i128::from(self.bounds.min.x_um)
        } else if x > i128::from(self.bounds.max.x_um) {
            if !self.edges[1] {
                return Err(invalid("fine query east of in-world source"));
            }
            i128::from(self.bounds.max.x_um)
        } else {
            x
        };
        let y = if y < i128::from(self.bounds.min.y_um) {
            if !self.edges[0] {
                return Err(invalid("fine query north of in-world source"));
            }
            i128::from(self.bounds.min.y_um)
        } else if y > i128::from(self.bounds.max.y_um) {
            if !self.edges[2] {
                return Err(invalid("fine query south of in-world source"));
            }
            i128::from(self.bounds.max.y_um)
        } else {
            y
        };
        Ok(TerrainPoint {
            x_um: i64::try_from(x).map_err(|_| invalid("fine x coordinate overflow"))?,
            y_um: i64::try_from(y).map_err(|_| invalid("fine y coordinate overflow"))?,
        })
    }

    fn height(&self, x: i128, y: i128) -> Result<i32, RenderError> {
        let point = self.clamp_source(x, y)?;
        self.field
            .sample(point)
            .map(|h| h.raw())
            .ok_or_else(|| invalid("fine window omits requested height"))
    }

    fn material(
        &self,
        x: i128,
        y: i128,
        saved: SavedMaterial,
    ) -> Result<([u8; 3], u16), RenderError> {
        let point = self.clamp_source(x, y)?;
        let x = i128::from(point.x_um);
        let y = i128::from(point.y_um);
        let s = i128::from(self.field.spacing_um());
        let height = self.height(x, y)?;
        if self.formed {
            // Only reached without saved climate; lapse from 15 C at sea level.
            let temperature = 1_500 - i32::try_from(i64::from(height) * 65 / 100_000).unwrap_or(0);
            return Ok((
                self.formed_color(x, y, height, (saved, temperature), CELL_UM, None)?,
                4_096,
            ));
        }
        let left = self.clamp_source(x - s, y)?;
        let right = self.clamp_source(x + s, y)?;
        let up = self.clamp_source(x, y - s)?;
        let down = self.clamp_source(x, y + s)?;
        let dx = equivalent_delta(
            self.height(i128::from(right.x_um), y)?,
            self.height(i128::from(left.x_um), y)?,
            i128::from(right.x_um) - i128::from(left.x_um),
        )?;
        let dy = equivalent_delta(
            self.height(x, i128::from(down.y_um))?,
            self.height(x, i128::from(up.y_um))?,
            i128::from(down.y_um) - i128::from(up.y_um),
        )?;
        let mut light = relief_light(dx, dy);
        if let Some(sky) = &self.sky {
            light = u16::try_from(
                (u32::from(light) * u32::from(sky.sample_q12(x, y)?) + 2_048) / 4_096,
            )
            .map_err(|_| invalid("sky-modulated light overflow"))?;
        }
        Ok((
            land_material_ecology(
                height,
                dx,
                dy,
                saved.wetness,
                saved.moisture,
                saved.forest_density,
            ),
            light,
        ))
    }

    /// Area-local saved cell of an absolute position (for sample lookups).
    pub(super) fn local_cell(&self, x_um: i128, y_um: i128) -> Option<(i16, i16)> {
        let cx = (x_um - self.origin_x).div_euclid(CELL_UM);
        let cy = (y_um - self.origin_y).div_euclid(CELL_UM);
        if !(-1..=512).contains(&cx) || !(-1..=512).contains(&cy) {
            return None;
        }
        Some((i16::try_from(cx).ok()?, i16::try_from(cy).ok()?))
    }

    /// Recipe-5 floodplain meander offset for a channel vertex (logic/04
    /// §atlas-formed rivers, goal 9): a sinusoid across the down-valley
    /// direction with wavelength 11 × channel width and amplitude up to a
    /// quarter wavelength, only on flat floodplains and only for rivers wide
    /// enough (≥ 55 m) that 100 m vertices resolve the bends. Direction and
    /// phase come from the 3 km relief and absolute position, so every
    /// render of a vertex agrees.
    pub(super) fn meander_offset_um(&self, x: i128, y: i128, width_dm: u32) -> Option<(i64, i64)> {
        if !self.formed || width_dm < 550 {
            return None;
        }
        let sky = self.sky.as_ref()?;
        let r = 3 * KM_UM;
        let gx = sky.context_height_mm(x + r, y).ok()? - sky.context_height_mm(x - r, y).ok()?;
        let gy = sky.context_height_mm(x, y + r).ok()? - sky.context_height_mm(x, y - r).ok()?;
        let glen = i128::try_from(
            (i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy))
                .unsigned_abs()
                .isqrt(),
        )
        .ok()?;
        // Need a defined down-valley direction: at least 3 m over 6 km.
        if glen < 3_000 {
            return None;
        }
        // Floodplain flatness from the 312 m fine gradient (slope < 3%).
        let s = i128::from(self.field.spacing_um()) * BROAD_STEPS;
        let hx = i128::from(self.height(x + s, y).ok()?) - i128::from(self.height(x - s, y).ok()?);
        let hy = i128::from(self.height(x, y + s).ok()?) - i128::from(self.height(x, y - s).ok()?);
        let local = (hx * hx + hy * hy).unsigned_abs().isqrt();
        let limit = u128::try_from(2 * s / 1_000 * 3 / 100).ok()?; // mm over 2s
        if local >= limit {
            return None;
        }
        let flat_q12 = i128::try_from((limit - local) * 4_096 / limit.max(1)).ok()?;
        // Down-valley unit vector (Q12) and its normal.
        let (dx, dy) = (
            -i128::from(gx) * 4_096 / glen,
            -i128::from(gy) * 4_096 / glen,
        );
        let lambda_um = i128::from(width_dm) * 11 * 100_000;
        let amp_um = lambda_um / 4 * flat_q12 / 4_096;
        let along = (x * dx + y * dy) / 4_096;
        let phase = along.rem_euclid(lambda_um) * 4_096 / lambda_um;
        // Bhaskara sine on the half-period, Q12.
        let half = |t: i128| 16 * t * (4_096 - t) / (5 * 4_096 - 4 * t * (4_096 - t) / 4_096);
        let sine = if phase < 2_048 {
            half(phase * 2)
        } else {
            -half((phase - 2_048) * 2)
        };
        let off = amp_um * sine / 4_096;
        Some((
            i64::try_from(-dy * off / 4_096).ok()?,
            i64::try_from(dx * off / 4_096).ok()?,
        ))
    }

    pub(super) fn is_formed(&self) -> bool {
        self.formed
    }

    /// Whether formed shading uses the recipe-6 look.
    pub(super) const fn is_v6(&self) -> bool {
        self.v6
    }

    /// Selects the formed look for a world's recipe (6 and later: v0.2).
    pub(super) fn set_recipe(&mut self, recipe_version: u16) {
        self.v6 = recipe_version >= 6;
    }

    /// Recipe-5 channel vertex: the lowest fine node inside the saved 100 m
    /// cell centred at `(x_um, y_um)`, as an offset from that centre. Every
    /// renderer derives the same offset from the canonical field, so rivers
    /// follow the 39 m valley floor without seams (logic/04 §atlas-formed
    /// rivers). `None` for older recipes or outside this window.
    pub(super) fn thalweg_offset_um(&self, x_um: i128, y_um: i128) -> Option<(i64, i64)> {
        if !self.formed {
            return None;
        }
        let s = i128::from(self.field.spacing_um());
        let half = CELL_UM / 2;
        let lo = |c: i128| -(-(c - half + 1)).div_euclid(s);
        let hi = |c: i128| (c + half).div_euclid(s);
        let mut nodes: Vec<(i32, i128, i128)> = Vec::with_capacity(16);
        for ky in lo(y_um)..=hi(y_um) {
            for kx in lo(x_um)..=hi(x_um) {
                let (px, py) = (kx * s, ky * s);
                let point = TerrainPoint {
                    x_um: i64::try_from(px).ok()?,
                    y_um: i64::try_from(py).ok()?,
                };
                nodes.push((self.field.sample(point)?.raw(), px, py));
            }
        }
        // Centroid of the floor: every node within 1 m of the cell minimum,
        // so flat valley floors do not alternate between equal lows.
        let min = nodes.iter().map(|n| n.0).min()?;
        let floor: Vec<_> = nodes.iter().filter(|n| n.0 <= min + 1_000).collect();
        let count = i128::try_from(floor.len()).ok()?;
        let cx = floor.iter().map(|n| n.1).sum::<i128>() / count;
        let cy = floor.iter().map(|n| n.2).sum::<i128>() / count;
        Some((
            i64::try_from(cx - x_um).ok()?,
            i64::try_from(cy - y_um).ok()?,
        ))
    }

    pub(super) fn sample(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        classes: &[TerrainKind],
        wetness: &[u8],
        moisture: &[u8],
        forest_density: &[u8],
    ) -> Result<[u8; 3], RenderError> {
        let (xs, xe, nx) = axis_positions(x, self.origin_x, self.field.spacing_um())?;
        let (ys, ye, ny) = axis_positions(y, self.origin_y, self.field.spacing_um())?;
        let mut color_sum = [0_u128; 3];
        let mut count = 0_u128;
        for iy in 0..ny {
            let py = position(ys, ye - ys, iy, ny);
            for ix in 0..nx {
                let px = position(xs, xe - xs, ix, nx);
                let Some(saved) = saved_material(
                    px,
                    py,
                    (self.origin_x, self.origin_y),
                    classes,
                    wetness,
                    moisture,
                    forest_density,
                )?
                else {
                    continue;
                };
                let (palette, light) = self.material(px, py, saved)?;
                let color = modulate(palette, light);
                for (sum, c) in color_sum.iter_mut().zip(color) {
                    *sum += u128::from(c);
                }
                count += 1;
            }
        }
        if count == 0 {
            return Err(invalid("fine footprint has no saved land ownership"));
        }
        let mut color = [0_u8; 3];
        for (channel, sum) in color.iter_mut().zip(color_sum) {
            *channel = u8::try_from((sum + count / 2) / count)
                .map_err(|_| invalid("fine averaged RGB outside channel range"))?;
        }
        Ok(color)
    }
}

#[cfg(test)]
mod tests;
