//! One level of stream-power landscape formation (logic/02 §fine-formation
//! incision): envelope correction plus relative uplift, depression filling,
//! implicit detachment-limited stream power (m = 1/2, n = 1) with a talus
//! cap, and optional linear hillslope diffusion.

use rayon::prelude::*;

use super::drainage::{
    accumulate, fill_local, receiver_index, receivers, upstream_order, FIXED, NB, SELF,
};
use super::lattice::{alloc, blur_into, Lattice};
use super::FormationError;

/// Minimum land height above the formation base level. Keeps eroding cells
/// from dropping to the base level itself, which would create spurious
/// standing-water specks.
pub const LAND_FLOOR_MM: i32 = 500;

/// Coefficients for one level, all integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelParams {
    /// Stream-power `K dt`, Q16, per iteration, for metre units.
    pub k_q16: i64,
    /// Talus (critical) slope, Q16.
    pub talus_q16: i64,
    /// Envelope correction gain per iteration, Q16.
    pub lambda_q16: i64,
    /// Envelope blur radius in level cells.
    pub blur_cells: usize,
    /// Iterations.
    pub iterations: u32,
    /// Linear diffusion number `D dt / d^2`, Q16 (0 disables).
    pub kappa_q16: i64,
    /// Receiver randomisation probability, Q16.
    pub stoch_q16: u32,
    /// Level seed.
    pub seed: u64,
    /// Relative uplift per iteration at full mask, millimetres.
    pub uplift_mm: i64,
    /// Depression-fill gradient per cell, millimetres.
    pub eps_mm: i32,
    /// Level cell area in finest-cell units.
    pub area_unit: u64,
    /// Lowest height a non-fixed cell may reach, millimetres.
    pub floor_mm: i32,
    /// Lowland soil-creep diffusion number at full lowland, Q16.
    pub creep_kappa_q16: i64,
    /// Hillslope diffusion below the channel-initiation area, Q16 (0 off).
    pub hill_kappa_q16: i64,
    /// Channel-initiation area in Q8 finest-cell units at rock strength 1
    /// (0 disables the threshold).
    pub channel_area_q8: u64,
    /// Accumulate contributing area on steepest-descent receivers.
    pub convergent_area: bool,
}

/// Reusable per-level scratch, sized to the lattice.
pub struct Scratch {
    /// Blurred target envelope.
    pub target_blur: Vec<i32>,
    pub(super) bz: Vec<i32>,
    pub(super) tmp: Vec<i32>,
    /// Receiver codes of the last drainage solve.
    pub rcv: Vec<u8>,
    /// Upstream-first order of the last drainage solve.
    pub order: Vec<u32>,
    /// Contributing area, Q8 finest-cell units.
    pub area: Vec<u64>,
    indeg: Vec<u8>,
}

impl Scratch {
    /// Allocates all scratch for `count` cells.
    ///
    /// # Errors
    /// Allocation failure.
    pub fn new(count: usize) -> Result<Self, FormationError> {
        Ok(Self {
            target_blur: alloc(count)?,
            bz: alloc(count)?,
            tmp: alloc(count)?,
            rcv: alloc(count)?,
            order: alloc(count)?,
            area: alloc(count)?,
            indeg: alloc(count)?,
        })
    }

    /// Bytes owned per cell.
    pub const BYTES_PER_CELL: u128 = 4 * 3 + 1 + 4 + 8 + 1;

    /// Raw target buffer; [`prepare_target`] blurs it into `target_blur`.
    pub fn raw_target(&mut self) -> &mut [i32] {
        &mut self.bz
    }
}

/// Per-cell fields for a level, Q8 unless stated.
pub struct LevelFields<'a> {
    /// `FIXED` flags (ocean and domain rim).
    pub flags: &'a [u8],
    /// Relative-uplift mask, 128 = 1.
    pub uplift: &'a [u8],
    /// Erodibility, 64 = 1.
    pub erodibility: &'a [u8],
    /// Runoff weight, 255 ~ 1 (Q8); `None` is uniform.
    pub runoff: Option<&'a [u8]>,
    /// Lowland factor for soil creep, 255 ~ full lowland (Q8).
    pub creep: &'a [u8],
    /// Rock-strength talus multiplier, 128 = 1 (Q7).
    pub rock: &'a [u8],
    /// Landscape maturity, 0 (fully dissected) ..= 255 (rounded upland).
    /// Scales fine-level hillslope creep, channel initiation and relative
    /// uplift (logic/02 §fine-formation maturity).
    pub soft: &'a [u8],
    /// Macro tectonic relief, 0 (flat) ..= 255 (≥ 400 m local relief).
    /// Low relief lowers the talus slope and spaces channels wider, so low
    /// hills get gentle slopes and shallow valleys (logic/02 §fine-formation
    /// relief scaling).
    pub relief: &'a [u8],
}

/// Runs `params.iterations` formation steps on `g`, pulling its blurred
/// surface towards `target` (already blurred into `scratch.target_blur`
/// by [`prepare_target`]). On return `scratch` holds the final drainage.
///
/// # Errors
/// Allocation failure inside the fill.
pub fn run(
    g: &mut Lattice,
    fields: &LevelFields<'_>,
    params: &LevelParams,
    scratch: &mut Scratch,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let d_um = g.spacing_um;
    let cap_card = params.talus_q16 * d_um / 1000 / 65_536;
    let cap_diag = params.talus_q16 * (d_um * 181 / 128) / 1000 / 65_536;
    for it in 0..params.iterations {
        // Envelope correction plus relative uplift.
        blur_into(
            &g.z,
            w,
            h,
            params.blur_cells,
            &mut scratch.tmp,
            &mut scratch.bz,
        );
        g.z.par_iter_mut().enumerate().for_each(|(i, z)| {
            if fields.flags[i] & FIXED != 0 {
                return;
            }
            let corr =
                params.lambda_q16 * (i64::from(scratch.target_blur[i]) - i64::from(scratch.bz[i]));
            // Mature (rounded) ground is uplifting less at fine levels, so
            // it keeps broad arched crests and rolling uplands.
            let damp = if params.hill_kappa_q16 > 0 {
                256 - i64::from(fields.soft[i]) * 150 / 255
            } else {
                256
            };
            let up = params.uplift_mm * i64::from(fields.uplift[i]) * damp / 256;
            let nz = i64::from(*z) + corr.div_euclid(65_536) + up.div_euclid(128);
            *z = i32::try_from(nz.clamp(i64::from(params.floor_mm), i64::from(i32::MAX)))
                .unwrap_or(i32::MAX);
        });
        let iter_seed = params.seed.wrapping_add(u64::from(it).wrapping_mul(7919));
        // Exact drainage every fifth iteration; local fills in between.
        drain(
            g,
            fields,
            params,
            scratch,
            iter_seed,
            params.stoch_q16,
            it % 5 == 4,
        )?;
        // Stream-power coefficients in parallel; the solve itself is serial.
        {
            let area = &scratch.area;
            let rcv = &scratch.rcv;
            scratch.bz.par_iter_mut().enumerate().for_each(|(i, f)| {
                let code = rcv[i];
                if code == SELF || fields.flags[i] & FIXED != 0 {
                    *f = 0;
                    return;
                }
                // Channel initiation (logic/02 §fine-formation hillslopes):
                // unchannelled hillslopes do not incise. The threshold scales
                // with rock strength squared (0.4-1.9x), so gully density
                // varies with lithology instead of repeating everywhere.
                let dist_um = if code < 4 { d_um } else { d_um * 181 / 128 };
                let r = receiver_index(i, w, h, code);
                let drop = i64::from(g.z[i]) - i64::from(g.z[r]);
                if area[i] < initiation_q8(params, fields, i, drop, dist_um) {
                    *f = 0;
                    return;
                }
                let s_q8 = isqrt(area[i].saturating_mul(390_625));
                let k = u128::from(params.k_q16.unsigned_abs()) * u128::from(fields.erodibility[i]);
                let fac = k * u128::from(s_q8) * 1_000_000
                    / (64 * 256 * u128::from(dist_um.unsigned_abs()));
                *f = i32::try_from(fac.min(i32::MAX as u128)).unwrap_or(i32::MAX);
            });
        }
        for &i in scratch.order.iter().rev() {
            let i = i as usize;
            let code = scratch.rcv[i];
            if code == SELF || fields.flags[i] & FIXED != 0 {
                continue;
            }
            let r = receiver_index(i, w, h, code);
            let f = i64::from(scratch.bz[i]);
            let zr = i64::from(g.z[r]);
            let zi = i64::from(g.z[i]);
            let mut nz = (zi * 65_536 + f * zr).div_euclid(65_536 + f);
            let base_cap = if code < 4 { cap_card } else { cap_diag };
            // Quadratic ramp: hills stay gentle until relief is substantial.
            let rq = i64::from(fields.relief[i]);
            let relief_scale = 25 + 230 * rq * rq * rq / (255 * 255 * 255);
            let cap = zr + base_cap * i64::from(fields.rock[i]) / 128 * relief_scale / 255;
            // Land never erodes to the base level (see LAND_FLOOR_MM).
            nz = nz.min(cap).max(zr).max(i64::from(params.floor_mm));
            g.z[i] = i32::try_from(nz).unwrap_or(g.z[i]);
        }
        if params.kappa_q16 > 0 || params.creep_kappa_q16 > 0 || params.hill_kappa_q16 > 0 {
            diffuse(g, fields, params, &scratch.area, &mut scratch.tmp);
        }
    }
    // Final steepest-descent drainage for the floodplain pass and callers.
    drain(g, fields, params, scratch, params.seed ^ 0xF1A7, 0, true)?;
    Ok(())
}

fn drain(
    g: &mut Lattice,
    fields: &LevelFields<'_>,
    params: &LevelParams,
    scratch: &mut Scratch,
    seed: u64,
    stoch_q16: u32,
    exact: bool,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    fill_local(
        &mut g.z,
        w,
        h,
        fields.flags,
        params.eps_mm,
        (&mut scratch.order, &mut scratch.indeg),
        exact,
    )?;
    // logic/02 §fine-formation hillslopes: with convergent area, water is
    // routed on steepest descent (it converges), while the solve and talus
    // cap below keep randomised receivers so no D8 cap seams form.
    let area_stoch = if params.convergent_area { 0 } else { stoch_q16 };
    receivers(&g.z, w, h, fields.flags, seed, area_stoch, &mut scratch.rcv);
    upstream_order(&scratch.rcv, w, h, &mut scratch.indeg, &mut scratch.order);
    accumulate(
        &scratch.rcv,
        w,
        h,
        &scratch.order,
        params.area_unit,
        fields.runoff,
        &mut scratch.area,
    );
    if area_stoch != stoch_q16 {
        receivers(&g.z, w, h, fields.flags, seed, stoch_q16, &mut scratch.rcv);
        upstream_order(&scratch.rcv, w, h, &mut scratch.indeg, &mut scratch.order);
    }
    Ok(())
}

/// Blurs the raw target (see [`Scratch::raw_target`]) into
/// `scratch.target_blur`.
pub fn prepare_target(width: usize, height: usize, r: usize, scratch: &mut Scratch) {
    blur_into(
        &scratch.bz,
        width,
        height,
        r,
        &mut scratch.tmp,
        &mut scratch.target_blur,
    );
}

/// Linear diffusion: a uniform number at coarse levels (valley spacing) and
/// lowland soil creep at every level, scaled by the lowland factor so
/// mountains keep sharp crests (logic/02 §fine-formation incision).
fn diffuse(
    g: &mut Lattice,
    fields: &LevelFields<'_>,
    params: &LevelParams,
    area: &[u64],
    tmp: &mut [i32],
) {
    let (w, h) = (g.width, g.height);
    let z = &g.z;
    let flags = fields.flags;
    let floor_mm = params.floor_mm;
    tmp.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let i = y * w + x;
            let zi = i64::from(z[i]);
            if flags[i] & FIXED != 0 {
                *out = z[i];
                continue;
            }
            let mut lap = -4 * zi;
            for &(dx, dy) in &NB[..4] {
                let nx = (x as i64 + dx).clamp(0, w as i64 - 1);
                let ny = (y as i64 + dy).clamp(0, h as i64 - 1);
                lap += i64::from(z[usize::try_from(ny * w as i64 + nx).unwrap_or(i)]);
            }
            let mut drop = 0_i64;
            for &(dx, dy) in &NB[..4] {
                let nx = (x as i64 + dx).clamp(0, w as i64 - 1);
                let ny = (y as i64 + dy).clamp(0, h as i64 - 1);
                drop =
                    drop.max(zi - i64::from(z[usize::try_from(ny * w as i64 + nx).unwrap_or(i)]));
            }
            let threshold = initiation_q8(params, fields, i, drop, g.spacing_um);
            // Creep rounds divides into convex, arched crests; its rate
            // follows landscape maturity (0.25x to 4x the base number).
            let hill = if area[i] < threshold {
                params.hill_kappa_q16 * (32 + 4 * i64::from(fields.soft[i])) / 128
            } else {
                0
            };
            let kappa = params
                .kappa_q16
                .max(params.creep_kappa_q16 * i64::from(fields.creep[i]) / 255)
                .max(hill);
            let nz = zi + (kappa * lap).div_euclid(65_536);
            *out = i32::try_from(nz.max(i64::from(floor_mm))).unwrap_or(z[i]);
        }
    });
    g.z.copy_from_slice(tmp);
}

/// Channel-initiation area at cell `i` (Q8 finest-cell units; logic/02
/// §fine-formation hillslopes): the base area scaled by rock strength
/// squared (0.4-1.9x), maturity (1-2x), low relief (up to 24x, cubic in
/// the missing relief) and strong relief (down to 0.35x), and
/// divided on steep ground by `(S / S_ref)²` with `S_ref` = 0.4, at most
/// 4x (Montgomery and Dietrich: the area a channel head needs falls with
/// the square of the slope). `drop_mm` over `dist_um` is the local slope.
/// Steep crest slopes then carry channel heads close to the divide instead
/// of a smooth unchannelled band 3-4 km wide (seed-42 full size, where a
/// mature patch had raised the threshold 3.7x); gentle hills (slope below
/// 22°) keep the base threshold, so low hills are not incised more.
/// Maturity still rounds crests through hillslope creep (see [`diffuse`]).
fn initiation_q8(
    params: &LevelParams,
    fields: &LevelFields<'_>,
    i: usize,
    drop_mm: i64,
    dist_um: i64,
) -> u64 {
    let rock = u64::from(fields.rock[i]);
    let soft = u64::from(fields.soft[i]);
    let flat = 255 - u64::from(fields.relief[i]);
    // Strong relief (mask near 255) starts channels at down to 0.35x the
    // base area, cubically, so only real mountains gain channel heads.
    let rq = 255 - flat;
    let mountain = 256 - 166 * rq * rq * rq / (255 * 255 * 255);
    // Low relief raises the threshold up to 24x, cubically: low hills stay
    // barely incised, while a coastal range of 300 m belt relief (1.4x)
    // is no longer left with 3-4 km smooth crests (the linear ramp gave
    // it 4.9x).
    let low = flat * flat * flat / (255 * 255);
    let base = params.channel_area_q8 * rock * rock / (128 * 128) * (255 + soft) / 255
        * (255 + 23 * low)
        / 255
        * mountain
        / 256;
    // Slope Q12 and the steepness factor (S_ref / S)² in Q12, ≥ 1/4.
    let s_q12 = (drop_mm.max(0) * 4_096 * 1_000 / dist_um.max(1)) as u64;
    const S_REF_Q12: u64 = 1_638;
    if s_q12 <= S_REF_Q12 {
        return base;
    }
    let f_q12 = (S_REF_Q12 * S_REF_Q12 / s_q12 * 4_096 / s_q12).max(1_024);
    base * f_q12 / 4_096
}

/// Integer square root (floor), Newton iteration from above.
#[must_use]
pub fn isqrt(v: u64) -> u64 {
    if v < 2 {
        return v;
    }
    let bits = 64 - v.leading_zeros();
    let mut x = 1_u64 << bits.div_ceil(2);
    loop {
        let y = (x + v / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isqrt_is_exact_floor() {
        for v in [
            0_u64,
            1,
            2,
            3,
            4,
            15,
            16,
            17,
            1 << 40,
            (1 << 62) + 12_345,
            u64::MAX >> 2,
        ] {
            let r = isqrt(v);
            assert!(r * r <= v && (r + 1).checked_mul(r + 1).is_none_or(|s| s > v));
        }
    }
}
