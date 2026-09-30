//! Fine land material from one immutable, absolute-coordinate terrain window.

use arda_core::{AreaCoord, TerrainField, TerrainKind, TerrainPoint};

mod relief;
pub use relief::ReliefGeometry;

use super::{
    invalid, land_material_ecology, modulate, relief_light, sample_index, AxisKernel, RenderError,
    SAMPLE_SIDE,
};

const CELL_UM: i128 = 100_000_000;
const AREA_UM: i128 = 512 * CELL_UM;
const KM_UM: i128 = 1_000_000_000;
const SKY_RADIUS_KM: i64 = 8;
const SKY_RADII_KM: [i64; 4] = [1, 2, 4, 8];
const SKY_DIRECTIONS: [(i64, i64, i64); 8] = [
    (1, 0, 65_536),
    (-1, 0, 65_536),
    (0, 1, 65_536),
    (0, -1, 65_536),
    (1, 1, 92_682),
    (-1, 1, 92_682),
    (1, -1, 92_682),
    (-1, -1, 92_682),
];
const LIGHT_ONE_Q12: i128 = 4_096;
/// Recipe-5 mid-scale gradient baseline in fine steps (8 x 39.0625 = 312.5 m).
pub(super) const BROAD_STEPS: i128 = 8;
/// Recipe-5 concavity ring radius in fine steps (156.25 m).
const RING_STEPS: i128 = 4;
const MAX_SKY_DARK_Q12: i128 = 737; // 18% ceiling

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
    field: TerrainField,
    origin_x: i128,
    origin_y: i128,
    bounds: AtlasFineWorldBounds,
    // North, east, south, west are actual saved-world edges from AtlasHalo.
    edges: [bool; 4],
    sky: Option<SkyShade>,
}

/// A globally aligned 1 km field of broad horizon exposure. Its source
/// heights come from the same immutable fine terrain as local slope lighting.
#[derive(Debug)]
struct SkyShade {
    first_x_km: i64,
    first_y_km: i64,
    width: usize,
    height: usize,
    light_q12: Vec<u16>,
    // The 1 km horizon context itself, for broad recipe-5 gradients.
    context_x_km: i64,
    context_y_km: i64,
    context_width: usize,
    context_height: usize,
    context_mm: Vec<i32>,
}

fn sky_visible_axis(area: i32) -> Result<(i64, i64), RenderError> {
    let origin = i128::from(area) * AREA_UM;
    let low = (origin - CELL_UM / 2).div_euclid(KM_UM);
    let last = origin + AREA_UM - CELL_UM / 2;
    let high = -(-last).div_euclid(KM_UM);
    Ok((
        i64::try_from(low).map_err(|_| invalid("sky x/y node underflow"))?,
        i64::try_from(high).map_err(|_| invalid("sky x/y node overflow"))?,
    ))
}

impl SkyShade {
    fn new(area: AreaCoord, context: &TerrainField) -> Result<Self, RenderError> {
        if i128::from(context.spacing_um()) != KM_UM
            || i128::from(context.origin().x_um).rem_euclid(KM_UM) != 0
            || i128::from(context.origin().y_um).rem_euclid(KM_UM) != 0
        {
            return Err(invalid("sky context is not on the global 1 km lattice"));
        }
        let (x0, x1) = sky_visible_axis(area.x)?;
        let (y0, y1) = sky_visible_axis(area.y)?;
        let (hx0, hx1) = (x0 - SKY_RADIUS_KM, x1 + SKY_RADIUS_KM);
        let (hy0, hy1) = (y0 - SKY_RADIUS_KM, y1 + SKY_RADIUS_KM);
        let hw = usize::try_from(hx1 - hx0 + 1).map_err(|_| invalid("sky width overflow"))?;
        let hh = usize::try_from(hy1 - hy0 + 1).map_err(|_| invalid("sky height overflow"))?;
        if hw > 72 || hh > 72 {
            return Err(invalid("sky context exceeds the per-area bound"));
        }
        let count = hw
            .checked_mul(hh)
            .ok_or_else(|| invalid("sky context count overflow"))?;
        let mut heights = Vec::new();
        heights
            .try_reserve_exact(count)
            .map_err(|_| invalid("sky context allocation failed"))?;
        for y in hy0..=hy1 {
            for x in hx0..=hx1 {
                let point = TerrainPoint {
                    x_um: i64::try_from(i128::from(x) * KM_UM)
                        .map_err(|_| invalid("sky context x overflow"))?,
                    y_um: i64::try_from(i128::from(y) * KM_UM)
                        .map_err(|_| invalid("sky context y overflow"))?,
                };
                heights.push(
                    context
                        .sample(point)
                        .ok_or_else(|| invalid("sky context omits a horizon node"))?
                        .raw(),
                );
                // Retained for recipe-5 broad gradients.
            }
        }
        let w = usize::try_from(x1 - x0 + 1).map_err(|_| invalid("sky width overflow"))?;
        let h = usize::try_from(y1 - y0 + 1).map_err(|_| invalid("sky height overflow"))?;
        let mut light_q12 = Vec::new();
        light_q12
            .try_reserve_exact(w * h)
            .map_err(|_| invalid("sky light allocation failed"))?;
        let at = |x: i64, y: i64| -> Result<i64, RenderError> {
            let ix = usize::try_from(x - hx0).map_err(|_| invalid("sky node west of context"))?;
            let iy = usize::try_from(y - hy0).map_err(|_| invalid("sky node north of context"))?;
            if ix >= hw || iy >= hh {
                return Err(invalid("sky node outside context"));
            }
            heights
                .get(iy * hw + ix)
                .copied()
                .map(i64::from)
                .ok_or_else(|| invalid("sky node missing from context"))
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let center = at(x, y)?;
                let mut sum = 0_i128;
                for (dx, dy, length_q16) in SKY_DIRECTIONS {
                    let mut horizon_q16 = 0_i128;
                    for radius in SKY_RADII_KM {
                        let rise = (at(x + dx * radius, y + dy * radius)? - center).max(0);
                        let slope_q16 = i128::from(rise) * 65_536 * 65_536
                            / (i128::from(radius) * 1_000_000 * i128::from(length_q16));
                        horizon_q16 = horizon_q16.max(slope_q16);
                    }
                    sum += horizon_q16;
                }
                let mean_q16 = (sum + 4) / 8;
                // Eighteen percent max darkening at 0.25 mean horizon slope.
                // A broad, symmetric exposure term avoids hard cast shadows
                // and does not amplify the 39 m grooves used by local light.
                let dark_q12 = ((mean_q16 * LIGHT_ONE_Q12 * 18 + 25 * 65_536 / 2) / (25 * 65_536))
                    .min(MAX_SKY_DARK_Q12);
                light_q12.push(
                    u16::try_from(LIGHT_ONE_Q12 - dark_q12)
                        .map_err(|_| invalid("sky light outside Q12"))?,
                );
            }
        }
        Ok(Self {
            first_x_km: x0,
            first_y_km: y0,
            width: w,
            height: h,
            light_q12,
            context_x_km: hx0,
            context_y_km: hy0,
            context_width: hw,
            context_height: hh,
            context_mm: heights,
        })
    }

    /// Bilinear 1 km context height at an absolute position.
    fn context_height_mm(&self, x_um: i128, y_um: i128) -> Result<i64, RenderError> {
        let fx = x_um - i128::from(self.context_x_km) * KM_UM;
        let fy = y_um - i128::from(self.context_y_km) * KM_UM;
        let ix = usize::try_from(fx.div_euclid(KM_UM)).map_err(|_| invalid("broad x west"))?;
        let iy = usize::try_from(fy.div_euclid(KM_UM)).map_err(|_| invalid("broad y north"))?;
        if ix + 1 >= self.context_width || iy + 1 >= self.context_height {
            return Err(invalid("broad gradient outside 1 km context"));
        }
        let tx = fx.rem_euclid(KM_UM);
        let ty = fy.rem_euclid(KM_UM);
        let at = |x: usize, y: usize| i128::from(self.context_mm[y * self.context_width + x]);
        let top = at(ix, iy) * (KM_UM - tx) + at(ix + 1, iy) * tx;
        let bottom = at(ix, iy + 1) * (KM_UM - tx) + at(ix + 1, iy + 1) * tx;
        i64::try_from((top * (KM_UM - ty) + bottom * ty) / (KM_UM * KM_UM))
            .map_err(|_| invalid("broad height overflow"))
    }

    fn sample_q12(&self, x_um: i128, y_um: i128) -> Result<u16, RenderError> {
        let x_km =
            i64::try_from(x_um.div_euclid(KM_UM)).map_err(|_| invalid("sky sample x overflow"))?;
        let y_km =
            i64::try_from(y_um.div_euclid(KM_UM)).map_err(|_| invalid("sky sample y overflow"))?;
        let ix = usize::try_from(x_km - self.first_x_km)
            .map_err(|_| invalid("sky sample west of area"))?;
        let iy = usize::try_from(y_km - self.first_y_km)
            .map_err(|_| invalid("sky sample north of area"))?;
        if ix >= self.width || iy >= self.height {
            return Err(invalid("sky sample outside area nodes"));
        }
        let fx = x_um.rem_euclid(KM_UM);
        let fy = y_um.rem_euclid(KM_UM);
        if (fx > 0 && ix + 1 >= self.width) || (fy > 0 && iy + 1 >= self.height) {
            return Err(invalid("sky sample lacks interpolation neighbor"));
        }
        let x1 = (ix + 1).min(self.width - 1);
        let y1 = (iy + 1).min(self.height - 1);
        let at = |x: usize, y: usize| i128::from(self.light_q12[y * self.width + x]);
        let top = (at(ix, iy) * (KM_UM - fx) + at(x1, iy) * fx + KM_UM / 2) / KM_UM;
        let bottom = (at(ix, y1) * (KM_UM - fx) + at(x1, y1) * fx + KM_UM / 2) / KM_UM;
        u16::try_from((top * (KM_UM - fy) + bottom * fy + KM_UM / 2) / KM_UM)
            .map_err(|_| invalid("sky interpolation outside Q12"))
    }
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

    /// Cubic B-spline height (mm) and its exact gradient (Q12 slope) at an
    /// absolute position (logic/04 §atlas-formed close zoom). Piecewise-
    /// bilinear heights give piecewise-constant slopes whose facets, and the
    /// field's own 39 m channel staircases, show at large exports; the
    /// smoothing spline is C2 and stays within the local node range.
    fn cubic(&self, x: i128, y: i128) -> Result<(i64, i64, i64), RenderError> {
        let p = self.clamp_source(x, y)?;
        let o = self.field.origin();
        let s = i128::from(self.field.spacing_um());
        let (w, h) = (
            i128::from(self.field.width()),
            i128::from(self.field.height()),
        );
        let dx = i128::from(p.x_um) - i128::from(o.x_um);
        let dy = i128::from(p.y_um) - i128::from(o.y_um);
        let (ix, iy) = (dx.div_euclid(s), dy.div_euclid(s));
        let tx = (dx.rem_euclid(s) << 16) / s;
        let ty = (dy.rem_euclid(s) << 16) / s;
        // Uniform cubic B-spline: a smoothing (not interpolating) patch whose
        // low-pass removes 39 m D8 staircases that only show below ~12 m/px.
        let weights = |t: i128| {
            let one = 65_536_i128;
            let u = one - t;
            let t2 = (t * t) >> 16;
            let t3 = (t2 * t) >> 16;
            let u2 = (u * u) >> 16;
            let u3 = (u2 * u) >> 16;
            (
                [
                    u3 / 6,
                    (3 * t3 - 6 * t2 + 4 * one) / 6,
                    (-3 * t3 + 3 * t2 + 3 * t + one) / 6,
                    t3 / 6,
                ],
                [
                    -u2 / 2,
                    (3 * t2 - 4 * t) / 2,
                    (-3 * t2 + 2 * t + one) / 2,
                    t2 / 2,
                ],
            )
        };
        let (wx, dwx) = weights(tx);
        let (wy, dwy) = weights(ty);
        let heights = self.field.heights();
        let at = |cx: i128, cy: i128| -> i128 {
            let cx = cx.clamp(0, w - 1);
            let cy = cy.clamp(0, h - 1);
            usize::try_from(cy * w + cx)
                .ok()
                .and_then(|i| heights.get(i))
                .map_or(0, |v| i128::from(v.raw()))
        };
        let (mut hv, mut gx, mut gy) = (0_i128, 0_i128, 0_i128);
        for j in 0..4 {
            let (mut row, mut drow) = (0_i128, 0_i128);
            for i in 0..4 {
                let z = at(ix - 1 + i as i128, iy - 1 + j as i128);
                row += wx[i] * z;
                drow += dwx[i] * z;
            }
            hv += wy[j] * row;
            gx += wy[j] * drow;
            gy += dwy[j] * row;
        }
        let q32 = 1_i128 << 32;
        // Slopes: d(height mm)/d(t) over the spacing in mm, Q12.
        let spacing_mm = s / 1_000;
        let slope = |v: i128| i64::try_from(v * 4_096 / q32 / spacing_mm.max(1)).unwrap_or(0);
        Ok((
            i64::try_from(hv.div_euclid(q32)).map_err(|_| invalid("cubic height overflow"))?,
            slope(gx),
            slope(gy),
        ))
    }

    /// Q12 slope from two heights `2 * half_um` apart.
    fn slope_q12(high: i64, low: i64, half_um: i128) -> Result<i64, RenderError> {
        let dz = i128::from(high - low) * 4_096 * 1_000;
        i64::try_from(dz / (2 * half_um)).map_err(|_| invalid("formed slope overflow"))
    }

    fn formed_color(
        &self,
        x: i128,
        y: i128,
        height: i32,
        (saved, temperature_centi): (SavedMaterial, i32),
        footprint_um: i128,
        refined: Option<relief::ReliefGeometry>,
    ) -> Result<[u8; 3], RenderError> {
        let sky = self
            .sky
            .as_ref()
            .ok_or_else(|| invalid("formed shading requires the 1 km context"))?;
        let s = i128::from(self.field.spacing_um());
        let h = |dx: i128, dy: i128| -> Result<i64, RenderError> {
            let p = self.clamp_source(x + dx, y + dy)?;
            Ok(i64::from(
                self.height(i128::from(p.x_um), i128::from(p.y_um))?,
            ))
        };
        let fine = |r: i128| -> Result<(i64, i64), RenderError> {
            Ok((
                Self::slope_q12(h(r * s, 0)?, h(-r * s, 0)?, r * s)?,
                Self::slope_q12(h(0, r * s)?, h(0, -r * s)?, r * s)?,
            ))
        };
        let broad = |r_km: i128| -> Result<(i64, i64), RenderError> {
            let r = r_km * KM_UM;
            Ok((
                Self::slope_q12(
                    sky.context_height_mm(x + r, y)?,
                    sky.context_height_mm(x - r, y)?,
                    r,
                )?,
                Self::slope_q12(
                    sky.context_height_mm(x, y + r)?,
                    sky.context_height_mm(x, y - r)?,
                    r,
                )?,
            ))
        };
        // Concavity at the drawn scale: never finer than the pixel footprint.
        let ring = (RING_STEPS * s).max(footprint_um);
        let ring_samples = [h(ring, 0)?, h(-ring, 0)?, h(0, ring)?, h(0, -ring)?];
        let ring_mean = ring_samples.iter().sum::<i64>() / 4;
        let ring_min = ring_samples.iter().copied().min().unwrap_or(0);
        // Close zoom (< 12 m/px) blends to the smoothing spline so the
        // 39 m grid never shows; crisp central differences elsewhere.
        let near = super::formed::close_zoom_q12(footprint_um);
        let (g0x, g0y) = fine(1)?;
        // Close-zoom relief (logic/17 §render): a refined lattice
        // supplies the finest light and its own sub-39 m concavity; every
        // broader term still reads the stored field.
        let (gx, gy, height, concavity) = if let Some(r) = refined {
            (
                r.gradient_q12.0,
                r.gradient_q12.1,
                r.height_mm,
                ring_mean - i64::from(height) + r.concavity_mm,
            )
        } else if near > 0 {
            let (bh, bgx, bgy) = self.cubic(x, y)?;
            let mix = |a: i64, b: i64| a + (b - a) * near / 4_096;
            let height =
                i32::try_from(mix(i64::from(height), bh)).map_err(|_| invalid("spline height"))?;
            (
                mix(g0x, bgx),
                mix(g0y, bgy),
                height,
                ring_mean - i64::from(height),
            )
        } else {
            (g0x, g0y, height, ring_mean - i64::from(height))
        };
        Ok(super::formed::shade(&super::formed::FormedInputs {
            height_mm: height,
            position_m: (
                i64::try_from(x / 1_000_000).unwrap_or(0),
                i64::try_from(y / 1_000_000).unwrap_or(0),
            ),
            position_dm: (
                i64::try_from(x / 100_000).unwrap_or(0),
                i64::try_from(y / 100_000).unwrap_or(0),
            ),
            footprint_um,
            gradients: [(gx, gy), fine(BROAD_STEPS)?, broad(1)?, broad(3)?],
            concavity_mm: concavity,
            sky_q12: i64::from(sky.sample_q12(x, y)?),
            ring_min_mm: ring_min,
            temperature_centi,
            moisture: saved.moisture,
            wetness: saved.wetness,
            shore: saved.shore,
        }))
    }

    /// Recipe-5 land and sea: each fine sub-sample is classified by the
    /// canonical field itself, so averaging over the pixel footprint yields
    /// an anti-aliased coastline on the 39 m contour (goal 31). Saved
    /// ownership still decides lakes, which never reach this path.
    pub(super) fn sample_formed(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        saved: &SavedFields<'_>,
    ) -> Result<[u8; 3], RenderError> {
        let (lake_surface, shore) = (saved.lake_surface, saved.shore);
        let (xs, xe, nx) = axis_positions(x, self.origin_x, self.field.spacing_um())?;
        let (ys, ye, ny) = axis_positions(y, self.origin_y, self.field.spacing_um())?;
        // Physical pixel size, for detail that must fade before it aliases.
        let footprint = match x {
            AxisKernel::Linear { denominator, .. } => {
                CELL_UM * 1_024 / i128::from(denominator.max(1))
            }
            // A box kernel spans the whole pixel footprint.
            AxisKernel::Box { .. } => (xe - xs).max(CELL_UM),
        };
        if self.sky.is_none() {
            return Err(invalid("formed shading requires the 1 km context"));
        }
        let mut color_sum = [0_u128; 3];
        let mut count = 0_u128;
        for iy in 0..ny {
            let py = position(ys, ye - ys, iy, ny);
            for ix in 0..nx {
                let px = position(xs, xe - xs, ix, nx);
                let point = self.clamp_source(px, py)?;
                let (qx, qy) = (i128::from(point.x_um), i128::from(point.y_um));
                let height = self.height(qx, qy)?;
                let surface =
                    lake_surface_near(px, py, (self.origin_x, self.origin_y), lake_surface)?;
                let color = if surface.is_some_and(|s| height < s) {
                    let depth = i64::from(surface.unwrap_or(height)) - i64::from(height);
                    super::formed::lake(depth)
                } else if height > 0 {
                    let (saved, temperature) = self.land_inputs(px, py, height, saved)?;
                    self.formed_color(qx, qy, height, (saved, temperature), footprint, None)?
                } else {
                    let shore_mix = shore_near(px, py, (self.origin_x, self.origin_y), shore)?;
                    self.sea_colour(qx, qy, height, &shore_mix)?
                };
                for (sum, c) in color_sum.iter_mut().zip(color) {
                    *sum += u128::from(c);
                }
                count += 1;
            }
        }
        if count == 0 {
            return Err(invalid("formed footprint is empty"));
        }
        let mut color = [0_u8; 3];
        for (channel, sum) in color.iter_mut().zip(color_sum) {
            *channel = u8::try_from((sum + count / 2) / count)
                .map_err(|_| invalid("formed averaged RGB outside channel range"))?;
        }
        Ok(color)
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

fn equivalent_delta(high: i32, low: i32, separation_um: i128) -> Result<i64, RenderError> {
    if separation_um <= 0 {
        return Err(invalid("fine derivative has zero physical baseline"));
    }
    let numerator = i128::from(high) - i128::from(low);
    let equivalent = round_div(numerator * 2 * CELL_UM, separation_um);
    i64::try_from(equivalent).map_err(|_| invalid("fine derivative overflow"))
}

fn round_div(n: i128, d: i128) -> i128 {
    if n < 0 {
        -((-n + d / 2) / d)
    } else {
        (n + d / 2) / d
    }
}

fn axis_positions(
    kernel: AxisKernel,
    origin: i128,
    spacing: u32,
) -> Result<(i128, i128, u32), RenderError> {
    match kernel {
        AxisKernel::Linear {
            low,
            high_weight,
            denominator,
        } => {
            let numerator =
                (i128::from(low) * i128::from(denominator) + i128::from(high_weight)) * CELL_UM;
            let p = origin + round_div(numerator, i128::from(denominator));
            Ok((p, p, 1))
        }
        AxisKernel::Box { start, end } => {
            let a = origin + (i128::from(start) * CELL_UM - CELL_UM / 2);
            let b = origin + (i128::from(end) * CELL_UM - CELL_UM / 2);
            let count = u32::try_from(((b - a) + i128::from(spacing) - 1) / i128::from(spacing))
                .map_err(|_| invalid("fine box quadrature count overflow"))?;
            Ok((a, b, count.max(1)))
        }
    }
}

fn position(start: i128, span: i128, index: u32, count: u32) -> i128 {
    if count == 1 && span == 0 {
        start
    } else {
        start + round_div((i128::from(index) * 2 + 1) * span, i128::from(count) * 2)
    }
}

/// Bilinear weights (Q12) of the stored shore classes around a point,
/// indexed by class; all zero without a shore layer.
fn shore_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    shore: &[u8],
) -> Result<[i64; 8], RenderError> {
    let mut mix = [0_i64; 8];
    if shore.len() != SAMPLE_SIDE * SAMPLE_SIDE {
        return Ok(mix);
    }
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("shore x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("shore y conversion"))?,
            )?;
            let class = usize::from(shore[i]).min(7);
            let w = i64::try_from(wx * wy * 4_096 / (CELL_UM * CELL_UM)).unwrap_or(0);
            mix[class] += w;
        }
    }
    Ok(mix)
}

/// Highest saved lake surface among the (up to) four cells around a point.
/// A sub-sample is lake water where the fine ground lies below it, so lake
/// shorelines follow the 39 m field (logic/04 §atlas-formed lakes).
fn lake_surface_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    lake_surface: &[i32],
) -> Result<Option<i32>, RenderError> {
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let mut best: Option<i32> = None;
    for cx in [qx, qx + 1] {
        for cy in [qy, qy + 1] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("lake x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("lake y conversion"))?,
            )?;
            let s = lake_surface[i];
            if s != i32::MIN {
                best = Some(best.map_or(s, |b| b.max(s)));
            }
        }
    }
    Ok(best)
}

/// Saved mean annual temperature interpolated over land cells and lapsed at
/// 6.5 C/km from the interpolated saved cell height to the fine height
/// (logic/04 §atlas-formed snow).
fn saved_temperature(
    x: i128,
    y: i128,
    origin: (i128, i128),
    classes: &[TerrainKind],
    climate: (&[i16], &[i32]),
    fine_height_mm: i32,
) -> Result<i32, RenderError> {
    let (temperature, heights) = climate;
    if temperature.len() != classes.len() || heights.len() != classes.len() {
        return Err(invalid("fine saved climate has wrong dimensions"));
    }
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let (mut t_sum, mut h_sum, mut w_sum) = (0_i128, 0_i128, 0_i128);
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("climate x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("climate y conversion"))?,
            )?;
            if classes[i] == TerrainKind::Land {
                let w = wx * wy;
                t_sum += i128::from(temperature[i]) * w;
                h_sum += i128::from(heights[i]) * w;
                w_sum += w;
            }
        }
    }
    if w_sum == 0 {
        return Ok(1_500 - i32::try_from(i64::from(fine_height_mm) * 65 / 100_000).unwrap_or(0));
    }
    let t = t_sum / w_sum;
    let h = h_sum / w_sum;
    let lapse = (i128::from(fine_height_mm) - h) * 65 / 100_000;
    i32::try_from(t - lapse).map_err(|_| invalid("temperature overflow"))
}

fn saved_material(
    x: i128,
    y: i128,
    origin: (i128, i128),
    classes: &[TerrainKind],
    wetness: &[u8],
    moisture: &[u8],
    forest_density: &[u8],
) -> Result<Option<SavedMaterial>, RenderError> {
    let (ox, oy) = origin;
    if classes.len() != SAMPLE_SIDE * SAMPLE_SIDE
        || wetness.len() != classes.len()
        || moisture.len() != classes.len()
        || forest_density.len() != classes.len()
    {
        return Err(invalid("fine saved context has wrong dimensions"));
    }
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let mut sum = [0_i128; 3];
    let mut weight_sum = 0_i128;
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("wetness x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("wetness y conversion"))?,
            )?;
            if classes[i] == TerrainKind::Land {
                let w = wx * wy;
                sum[0] += i128::from(wetness[i]) * w;
                sum[1] += i128::from(moisture[i]) * w;
                sum[2] += i128::from(forest_density[i]) * w;
                weight_sum += w;
            }
        }
    }
    if weight_sum == 0 {
        Ok(None)
    } else {
        let read = |v: i128| {
            u8::try_from((v + weight_sum / 2) / weight_sum)
                .map_err(|_| invalid("material interpolation overflow"))
        };
        Ok(Some(SavedMaterial {
            wetness: read(sum[0])?,
            moisture: read(sum[1])?,
            forest_density: read(sum[2])?,
            shore: [0; 8],
        }))
    }
}

#[cfg(test)]
mod tests {
    use arda_core::HeightMm;

    use super::*;

    fn field(
        origin: TerrainPoint,
        spacing: u32,
        width: u32,
        height: u32,
        mut height_at: impl FnMut(u32, u32) -> i32,
    ) -> TerrainField {
        let heights = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| HeightMm::new(height_at(x, y)))
            .collect();
        TerrainField::new(origin, spacing, width, height, heights).unwrap()
    }

    fn all_land() -> (Vec<TerrainKind>, Vec<u8>, Vec<u8>, Vec<u8>) {
        (
            vec![TerrainKind::Land; SAMPLE_SIDE * SAMPLE_SIDE],
            vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        )
    }

    #[test]
    fn physical_plane_has_exact_height_and_equivalent_gradient() {
        let s = 39_062_500;
        let field = field(
            TerrainPoint {
                x_um: -2 * i64::from(s),
                y_um: -2 * i64::from(s),
            },
            s,
            5,
            5,
            |x, y| {
                1_500_000 + (i32::try_from(x).unwrap() - 2) * 5_000
                    - (i32::try_from(y).unwrap() - 2) * 2_500
            },
        );
        let atlas = FineAtlas {
            formed: false,
            field,
            origin_x: 0,
            origin_y: 0,
            bounds: AtlasFineWorldBounds {
                min: TerrainPoint {
                    x_um: -2 * i64::from(s),
                    y_um: -2 * i64::from(s),
                },
                max: TerrainPoint {
                    x_um: 2 * i64::from(s),
                    y_um: 2 * i64::from(s),
                },
            },
            edges: [false; 4],
            sky: None,
        };
        assert_eq!(atlas.height(0, 0).unwrap(), 1_500_000);
        let expected = (
            land_material_ecology(1_500_000, 25_600, -12_800, 0, 0, 0),
            relief_light(25_600, -12_800),
        );
        assert_eq!(
            atlas.material(0, 0, SavedMaterial::default()).unwrap(),
            expected
        );
        let (classes, wetness, moisture, canopy) = all_land();
        assert_eq!(
            atlas
                .sample(
                    AxisKernel::Linear {
                        low: 0,
                        high_weight: 0,
                        denominator: 2
                    },
                    AxisKernel::Linear {
                        low: 0,
                        high_weight: 0,
                        denominator: 2
                    },
                    &classes,
                    &wetness,
                    &moisture,
                    &canopy,
                )
                .unwrap(),
            modulate(expected.0, expected.1)
        );
    }

    #[test]
    fn overlapping_area_queries_share_exact_seam_radiance() {
        let s = 100_000_000;
        let source = field(
            TerrainPoint { x_um: -s, y_um: -s },
            u32::try_from(s).unwrap(),
            1_027,
            3,
            |x, y| {
                1_000_000 + i32::try_from(x).unwrap() * 1_000 + i32::try_from(y).unwrap() * 2_000
            },
        );
        let bounds = AtlasFineWorldBounds {
            min: source.origin(),
            max: TerrainPoint {
                x_um: -s + 1_026 * s,
                y_um: s,
            },
        };
        let left = FineAtlas {
            formed: false,
            field: source.clone(),
            origin_x: 0,
            origin_y: 0,
            bounds,
            edges: [false; 4],
            sky: None,
        };
        let right = FineAtlas {
            formed: false,
            field: source,
            origin_x: AREA_UM,
            origin_y: 0,
            bounds,
            edges: [false; 4],
            sky: None,
        };
        let (classes, wetness, moisture, canopy) = all_land();
        let x_left = AxisKernel::Linear {
            low: 511,
            high_weight: 1,
            denominator: 2,
        };
        let x_right = AxisKernel::Linear {
            low: -1,
            high_weight: 1,
            denominator: 2,
        };
        let y = AxisKernel::Linear {
            low: 0,
            high_weight: 0,
            denominator: 2,
        };
        assert_eq!(
            left.sample(x_left, y, &classes, &wetness, &moisture, &canopy)
                .unwrap(),
            right
                .sample(x_right, y, &classes, &wetness, &moisture, &canopy)
                .unwrap()
        );
    }

    #[test]
    fn fine_high_frequency_changes_box_and_linear_samples() {
        let s = 50_000_000;
        let source = field(
            TerrainPoint {
                x_um: -2 * s,
                y_um: -2 * s,
            },
            u32::try_from(s).unwrap(),
            8,
            8,
            |x, _| if x % 2 == 0 { 1_600_000 } else { 1_000_000 },
        );
        let atlas = FineAtlas {
            formed: false,
            field: source,
            origin_x: 0,
            origin_y: 0,
            bounds: AtlasFineWorldBounds {
                min: TerrainPoint {
                    x_um: -2 * s,
                    y_um: -2 * s,
                },
                max: TerrainPoint {
                    x_um: 5 * s,
                    y_um: 5 * s,
                },
            },
            edges: [false; 4],
            sky: None,
        };
        let (classes, wetness, moisture, canopy) = all_land();
        let y = AxisKernel::Linear {
            low: 0,
            high_weight: 0,
            denominator: 2,
        };
        let center = atlas
            .sample(
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 0,
                    denominator: 2,
                },
                y,
                &classes,
                &wetness,
                &moisture,
                &canopy,
            )
            .unwrap();
        let half = atlas
            .sample(
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 1,
                    denominator: 2,
                },
                y,
                &classes,
                &wetness,
                &moisture,
                &canopy,
            )
            .unwrap();
        assert_ne!(center, half);
        let box_color = atlas
            .sample(
                AxisKernel::Box { start: 0, end: 1 },
                y,
                &classes,
                &wetness,
                &moisture,
                &canopy,
            )
            .unwrap();
        assert_ne!(box_color, center);
        assert_ne!(box_color, half);
    }

    #[test]
    fn only_a_true_world_edge_clamps_outside_source_queries() {
        let source = field(
            TerrainPoint { x_um: 0, y_um: 0 },
            50_000_000,
            3,
            3,
            |x, y| 1_000_000 + i32::try_from(x + y).unwrap() * 1_000,
        );
        let bounds = AtlasFineWorldBounds {
            min: TerrainPoint { x_um: 0, y_um: 0 },
            max: TerrainPoint {
                x_um: 100_000_000,
                y_um: 100_000_000,
            },
        };
        let mut atlas = FineAtlas {
            formed: false,
            field: source,
            origin_x: 0,
            origin_y: 0,
            bounds,
            edges: [false; 4],
            sky: None,
        };
        assert!(atlas.height(-1, 50_000_000).is_err());
        atlas.edges[3] = true;
        assert_eq!(
            atlas.height(-1, 50_000_000).unwrap(),
            atlas.height(0, 50_000_000).unwrap()
        );
        assert!(atlas.height(50_000_000, -1).is_err());
    }

    #[test]
    fn flat_world_has_exactly_unity_sky_light() {
        let context = field(
            TerrainPoint {
                x_um: -10_000_000_000,
                y_um: -10_000_000_000,
            },
            1_000_000_000,
            80,
            80,
            |_, _| 1_000_000,
        );
        let sky = SkyShade::new(AreaCoord::new(0, 0), &context).unwrap();
        assert!(sky.light_q12.iter().all(|&light| light == 4_096));
        assert_eq!(
            sky.sample_q12(25_200_000_000, 25_200_000_000).unwrap(),
            4_096
        );
        let bounds = AtlasFineWorldBounds {
            min: context.origin(),
            max: TerrainPoint {
                x_um: 69_000_000_000,
                y_um: 69_000_000_000,
            },
        };
        let unshaded = FineAtlas::new(
            AreaCoord::new(0, 0),
            bounds,
            context.clone(),
            [false; 4],
            None,
            false,
        )
        .unwrap();
        let shaded = FineAtlas::new(
            AreaCoord::new(0, 0),
            bounds,
            context.clone(),
            [false; 4],
            Some(context),
            false,
        )
        .unwrap();
        assert_eq!(
            shaded
                .material(25_200_000_000, 25_200_000_000, SavedMaterial::default())
                .unwrap(),
            unshaded
                .material(25_200_000_000, 25_200_000_000, SavedMaterial::default())
                .unwrap(),
        );
    }

    #[test]
    fn floodplain_meanders_swing_across_the_valley_and_stay_bounded() {
        // A floodplain falling 1 m per km to the north: down-valley is -y.
        let source = field(
            TerrainPoint {
                x_um: -10_000_000_000,
                y_um: -10_000_000_000,
            },
            1_000_000_000,
            80,
            80,
            |_, y| 100_000 + i32::try_from(y).unwrap() * 1_000,
        );
        let bounds = AtlasFineWorldBounds {
            min: source.origin(),
            max: TerrainPoint {
                x_um: 69_000_000_000,
                y_um: 69_000_000_000,
            },
        };
        let atlas = FineAtlas::new(
            AreaCoord::new(0, 0),
            bounds,
            source.clone(),
            [false; 4],
            Some(source),
            true,
        )
        .unwrap();
        let width_dm = 1_000; // 100 m river: wavelength 1.1 km
        let lambda = 1_100_000_000_i128;
        let at = |y: i128| {
            atlas
                .meander_offset_um(25_000_000_000, y, width_dm)
                .unwrap()
        };
        let a = at(25_000_000_000 + lambda / 4);
        let b = at(25_000_000_000 + 3 * lambda / 4);
        // Offsets lie across the valley (x) and flip sign half a wave on.
        assert!(a.0.signum() == -b.0.signum() && a.0 != 0, "{a:?} {b:?}");
        assert!(
            a.1.abs() < 1_000_000 && b.1.abs() < 1_000_000,
            "across, not along"
        );
        let amp = i64::try_from(lambda / 4).unwrap();
        assert!(a.0.abs() <= amp && b.0.abs() <= amp);
        // Narrow streams do not meander at 100 m vertex spacing.
        assert!(atlas
            .meander_offset_um(25_000_000_000, 25_000_000_000, 300)
            .is_none());
    }

    #[test]
    fn kilometer_horizon_darks_a_valley_without_darking_its_surrounding_ridge() {
        let context = field(
            TerrainPoint {
                x_um: -10_000_000_000,
                y_um: -10_000_000_000,
            },
            1_000_000_000,
            80,
            80,
            |x, y| {
                let gx = i64::from(x) - 10;
                let gy = i64::from(y) - 10;
                let r2 = (gx - 25).pow(2) + (gy - 25).pow(2);
                if (9..=36).contains(&r2) {
                    2_000_000
                } else {
                    1_000_000
                }
            },
        );
        let sky = SkyShade::new(AreaCoord::new(0, 0), &context).unwrap();
        let valley = sky.sample_q12(25_000_000_000, 25_000_000_000).unwrap();
        let ridge = sky.sample_q12(29_000_000_000, 25_000_000_000).unwrap();
        assert!(
            valley < 3_700,
            "valley received too little broad shade: {valley}"
        );
        assert_eq!(ridge, 4_096);
    }

    #[test]
    fn globally_aligned_sky_context_has_equal_radiance_at_an_area_seam() {
        let source = field(
            TerrainPoint {
                x_um: -10_000_000_000,
                y_um: -10_000_000_000,
            },
            1_000_000_000,
            130,
            80,
            |x, y| {
                let gx = i64::from(x) - 10;
                let gy = i64::from(y) - 10;
                let distance = (gx - 54).abs() + (gy - 25).abs();
                1_000_000 + i32::try_from((1_500_000 - distance * 250_000).max(0)).unwrap()
            },
        );
        let bounds = AtlasFineWorldBounds {
            min: source.origin(),
            max: TerrainPoint {
                x_um: 119_000_000_000,
                y_um: 69_000_000_000,
            },
        };
        let left = FineAtlas::new(
            AreaCoord::new(0, 0),
            bounds,
            source.clone(),
            [false; 4],
            Some(source.clone()),
            false,
        )
        .unwrap();
        let right = FineAtlas::new(
            AreaCoord::new(1, 0),
            bounds,
            source.clone(),
            [false; 4],
            Some(source),
            false,
        )
        .unwrap();
        let seam = (AREA_UM, 25_000_000_000_i128);
        let left_sky = left
            .sky
            .as_ref()
            .unwrap()
            .sample_q12(seam.0, seam.1)
            .unwrap();
        let right_sky = right
            .sky
            .as_ref()
            .unwrap()
            .sample_q12(seam.0, seam.1)
            .unwrap();
        assert!(left_sky < 4_096);
        assert_eq!(left_sky, right_sky);
        assert_eq!(
            left.material(seam.0, seam.1, SavedMaterial::default())
                .unwrap(),
            right
                .material(seam.0, seam.1, SavedMaterial::default())
                .unwrap(),
        );
    }
}
