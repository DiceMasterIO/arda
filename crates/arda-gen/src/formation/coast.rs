//! Post-lowstand estuarine infill (logic/02 §fine-formation drowned coasts).
//!
//! Formation grades valleys to a glacial lowstand; returning the sea to 0
//! drowns them. In nature most drowned valleys were back-filled by sediment
//! during the Holocene, leaving open rias and bays only near the coast and
//! longer ones on steep, sediment-poor coasts. Cells flooded further inland
//! than a relief-dependent limit become a flat alluvial floor just above sea
//! level that grades seaward, so rivers resume there.

use rayon::prelude::*;

use super::lattice::{alloc, Lattice};
use super::FormationError;

/// Open-water reach inland of the macro coast on low coasts, metres.
pub const RIA_LOW_M: i64 = 1_500;
/// Additional reach on fully mountainous coasts, metres.
pub const RIA_RELIEF_M: i64 = 6_000;
/// Infill surface: height at the reach limit and gradient seaward-to-inland.
const INFILL_BASE_MM: i64 = 300;
const INFILL_MM_PER_KM: i64 = 150;

/// Two-pass chamfer distance (metres) from every cell with `macro <= 0`.
fn coast_distance_m(
    macro_mm: &[i32],
    w: usize,
    h: usize,
    d_m: i64,
) -> Result<Vec<i32>, FormationError> {
    let mut dist: Vec<i32> = alloc(w * h)?;
    let far = i32::MAX / 2;
    dist.par_iter_mut()
        .zip(macro_mm.par_iter())
        .for_each(|(v, &m)| *v = if m <= 0 { 0 } else { far });
    let card = i32::try_from(d_m).map_err(|_| FormationError::ArithmeticOverflow)?;
    let diag = i32::try_from(d_m * 181 / 128).map_err(|_| FormationError::ArithmeticOverflow)?;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = dist[i];
            if x > 0 {
                v = v.min(dist[i - 1] + card);
            }
            if y > 0 {
                v = v.min(dist[i - w] + card);
                if x > 0 {
                    v = v.min(dist[i - w - 1] + diag);
                }
                if x + 1 < w {
                    v = v.min(dist[i - w + 1] + diag);
                }
            }
            dist[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = dist[i];
            if x + 1 < w {
                v = v.min(dist[i + 1] + card);
            }
            if y + 1 < h {
                v = v.min(dist[i + w] + card);
                if x + 1 < w {
                    v = v.min(dist[i + w + 1] + diag);
                }
                if x > 0 {
                    v = v.min(dist[i + w - 1] + diag);
                }
            }
            dist[i] = v;
        }
    }
    Ok(dist)
}

/// Fills drowned valleys beyond the relief-dependent ria reach.
/// `macro_mm` is the macro surface and `relief_q8` the coastal relief mask
/// (256 = mountainous), both on `g`'s lattice. Returns cells infilled.
///
/// # Errors
/// Allocation failure.
pub fn infill(
    g: &mut Lattice,
    macro_mm: &[i32],
    relief_q8: &[u8],
) -> Result<usize, FormationError> {
    infill_recipe(g, macro_mm, relief_q8, false)
}

/// Recipe-9 infill gradient, mm per km (review 3.1-11): 3.9 mm per 39 m
/// cardinal step and 5.5 mm per diagonal step, both inside the flats'
/// [`super::flats::FLAT_DROP_MM`]. At 150 mm per km a diagonal step drops
/// 8.3 mm, so on coasts at 45° a quarter of the infill was not flagged as
/// flat and never regraded.
const INFILL_MM_PER_KM_V9: i64 = 100;

/// [`infill`] with the recipe-9 gradient when `v9`.
///
/// # Errors
/// Allocation failure.
pub fn infill_recipe(
    g: &mut Lattice,
    macro_mm: &[i32],
    relief_q8: &[u8],
    v9: bool,
) -> Result<usize, FormationError> {
    let per_km = if v9 {
        INFILL_MM_PER_KM_V9
    } else {
        INFILL_MM_PER_KM
    };
    let (w, h) = (g.width, g.height);
    let d_m = g.spacing_um / 1_000_000;
    let dist = coast_distance_m(macro_mm, w, h, d_m.max(1))?;
    let changed =
        g.z.par_iter_mut()
            .enumerate()
            .map(|(i, z)| {
                if *z > 0 || macro_mm[i] <= 0 {
                    return 0_usize;
                }
                let reach = RIA_LOW_M + RIA_RELIEF_M * i64::from(relief_q8[i]) / 256;
                let d = i64::from(dist[i]);
                if d <= reach {
                    return 0;
                }
                let nz = INFILL_BASE_MM + (d - reach) * per_km / 1000;
                *z = i32::try_from(nz).unwrap_or(i32::MAX);
                1
            })
            .sum();
    Ok(changed)
}

/// Minimum catchment for a delta, km².
pub const DELTA_MIN_KM2: u64 = 2_000;
/// Delta radius: 0.12 km per √km² of catchment, metres per √km².
const DELTA_M_PER_ROOT_KM2: i64 = 120;
/// Deltas build on shelves no deeper than this, millimetres.
const DELTA_SHELF_MM: i32 = 40_000;
/// Delta apex height above sea level, millimetres.
const DELTA_APEX_MM: i64 = 3_000;

/// Recipe-5 river-mouth deltas (logic/02 §fine-formation deltas, goal 10;
/// recipe 6 builds them in [`super::water::delta`]): where a
/// large river reaches the sea on a low, sediment-rich coast, a fan of low
/// alluvial land builds out over the shelf, opening seaward (±75°) along
/// the river's final flow direction. Mountain coasts keep open rias.
/// Returns the number of deltas built.
///
/// # Errors
/// Allocation failure.
pub fn deltas_v5(g: &mut Lattice, relief_q8: &[u8], seed: u64) -> Result<usize, FormationError> {
    use super::drainage::{open_sea_flags, receiver_index, receivers, upstream_order, SELF};
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = g.spacing_um / 1_000_000;
    let cell_m2 = u64::try_from((g.spacing_um / 1000) * (g.spacing_um / 1000) / 1_000_000)
        .map_err(|_| FormationError::ArithmeticOverflow)?
        .max(1);
    let mut flags: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut flags);
    let mut rcv: Vec<u8> = alloc(n)?;
    receivers(&g.z, w, h, &flags, 0, 0, &mut rcv);
    let mut area: Vec<u32> = alloc(n)?;
    {
        let mut order: Vec<u32> = alloc(n)?;
        let mut indeg: Vec<u8> = alloc(n)?;
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
        area.fill(1);
        for &i in &order {
            let i = i as usize;
            let r = receiver_index(i, w, h, rcv[i]);
            if r != i {
                area[r] = area[r].saturating_add(area[i]);
            }
        }
    }
    let min_cells = DELTA_MIN_KM2 * 1_000_000 / cell_m2;
    let mut mouths = Vec::new();
    for i in 0..n {
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i
            && g.z[i] > 0
            && g.z[r] <= 0
            && flags[r] != 0
            && u64::from(area[i]) >= min_cells
            && relief_q8[i] < 100
        {
            mouths.push(i);
        }
    }
    let mut built = 0;
    for &m in &mouths {
        // Seaward direction: from ~2 km upstream along the main donor path.
        let mut up = m;
        for _ in 0..(2_000 / d_m.max(1)) {
            let mut best: Option<usize> = None;
            for k in 0..8 {
                let Some(nb) = super::drainage::neighbour(up, w, h, k) else {
                    continue;
                };
                if rcv[nb] != SELF
                    && receiver_index(nb, w, h, rcv[nb]) == up
                    && best.is_none_or(|b| area[nb] > area[b])
                {
                    best = Some(nb);
                }
            }
            match best {
                Some(b) => up = b,
                None => break,
            }
        }
        let (mx, my) = ((m % w) as i64, (m / w) as i64);
        let (dx, dy) = (mx - (up % w) as i64, my - (up / w) as i64);
        let dlen = i64::try_from((dx * dx + dy * dy).unsigned_abs().isqrt()).unwrap_or(0);
        if dlen == 0 {
            continue;
        }
        let km2 = u64::from(area[m]) * cell_m2 / 1_000_000;
        let radius_m =
            DELTA_M_PER_ROOT_KM2 * i64::try_from(super::incision::isqrt(km2)).unwrap_or(0);
        let radius = (radius_m / d_m.max(1)).max(4);
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let r2 = ox * ox + oy * oy;
                if r2 > radius * radius {
                    continue;
                }
                // Within ±75° of seaward: cos 75° ≈ 0.259 (Q12 1_060).
                let dist = i64::try_from(r2.unsigned_abs().isqrt()).unwrap_or(0);
                if dist > 0 && (ox * dx + oy * dy) * 4096 < 1_060 * dist * dlen {
                    continue;
                }
                let (x, y) = (mx + ox, my + oy);
                if x < 1 || y < 1 || x >= w as i64 - 1 || y >= h as i64 - 1 {
                    continue;
                }
                let j = (y as usize) * w + x as usize;
                if g.z[j] > 0 || g.z[j] < -DELTA_SHELF_MM {
                    continue;
                }
                // Lobate edge: ±40% radius wobble from 1–2 km value noise.
                let (xm, ym) = (
                    i32::try_from(x * d_m).unwrap_or(0),
                    i32::try_from(y * d_m).unwrap_or(0),
                );
                let wobble = i64::from(crate::noise::value_noise(seed, xm, ym, 1_500));
                let reach = radius * (4096 + wobble * 1_638 / 32_768) / 4096;
                if dist >= reach {
                    continue;
                }
                let height = DELTA_APEX_MM * (reach - dist) / reach.max(1) + 200;
                g.z[j] = i32::try_from(height).unwrap_or(g.z[j]);
            }
        }
        built += 1;
    }
    Ok(built)
}

/// Wave-reworked shore band: heights within this distance of sea level are
/// blended towards the local mean.
pub const SHORE_BAND_MM: i64 = 50_000;
/// Radius of the wave-rework mean, metres.
pub const SHORE_RADIUS_M: i64 = 625;

/// Wave reworking (logic/02 §fine-formation shore): cuts back small spurs
/// and fills small coves near sea level so shorelines keep their large bays
/// and headlands without a frill at every gully mouth. Returns cells changed.
///
/// # Errors
/// Allocation failure.
pub fn rework_shore(g: &mut Lattice, fixed_deep_mm: i32) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let r = usize::try_from((SHORE_RADIUS_M * 1_000_000 / g.spacing_um).max(1))
        .map_err(|_| FormationError::ArithmeticOverflow)?;
    let mut tmp: Vec<i32> = alloc(n)?;
    let mut mean: Vec<i32> = alloc(n)?;
    super::lattice::blur_into(&g.z, w, h, r, &mut tmp, &mut mean);
    drop(tmp);
    let changed =
        g.z.par_iter_mut()
            .zip(mean.par_iter())
            .map(|(z, &m)| {
                let zi = i64::from(*z);
                if zi.abs() >= SHORE_BAND_MM || *z <= fixed_deep_mm {
                    return 0_usize;
                }
                let weight = SHORE_BAND_MM - zi.abs();
                let nz = zi + (i64::from(m) - zi) * weight / SHORE_BAND_MM;
                // Never lift open water above sea level or sink land below it by
                // more than the rework itself implies: the sign may change, which
                // is exactly the spur cut-back / cove fill.
                *z = i32::try_from(nz).unwrap_or(*z);
                usize::from(nz != zi)
            })
            .sum();
    Ok(changed)
}

/// Recipe-9 wave rework reaches full strength this close to open sea,
/// metres.
pub const SHORE_REACH_FULL_M: i64 = 250;
/// ...and fades out by this distance, metres.
pub const SHORE_REACH_END_M: i64 = 1_250;

/// Recipe-9 wave reworking (review 3.1-5): [`rework_shore`] weighted by
/// distance to the open sea, full within [`SHORE_REACH_FULL_M`] and fading
/// out by [`SHORE_REACH_END_M`]. Waves rework the shore, not every cell
/// within 50 m of sea level: the unweighted rework also blended inland
/// valleys and lowland basins toward a 625 m mean (an inland valley at
/// 17-25 m lost 57% of its depth). Returns cells changed.
///
/// # Errors
/// Allocation failure.
pub fn rework_shore_near(g: &mut Lattice, fixed_deep_mm: i32) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = (g.spacing_um / 1_000_000).max(1);
    let r = usize::try_from((SHORE_RADIUS_M * 1_000_000 / g.spacing_um).max(1))
        .map_err(|_| FormationError::ArithmeticOverflow)?;
    let dist = {
        let mut flags: Vec<u8> = alloc(n)?;
        super::drainage::open_sea_flags_d8(&g.z, w, h, &mut flags);
        let source: Vec<i32> = flags
            .iter()
            .zip(&g.z)
            .map(|(&f, &z)| i32::from(!(f != 0 && z <= 0)))
            .collect();
        coast_distance_m(&source, w, h, d_m)?
    };
    let mut tmp: Vec<i32> = alloc(n)?;
    let mut mean: Vec<i32> = alloc(n)?;
    super::lattice::blur_into(&g.z, w, h, r, &mut tmp, &mut mean);
    drop(tmp);
    let span = SHORE_REACH_END_M - SHORE_REACH_FULL_M;
    let changed =
        g.z.par_iter_mut()
            .zip(mean.par_iter())
            .zip(dist.par_iter())
            .map(|((z, &m), &dm)| {
                let zi = i64::from(*z);
                if zi.abs() >= SHORE_BAND_MM || *z <= fixed_deep_mm {
                    return 0_usize;
                }
                let reach = (SHORE_REACH_END_M - i64::from(dm)).clamp(0, span) * 1_024 / span;
                if reach == 0 {
                    return 0;
                }
                let weight = (SHORE_BAND_MM - zi.abs()) * reach / 1_024;
                let nz = zi + (i64::from(m) - zi) * weight / SHORE_BAND_MM;
                *z = i32::try_from(nz).unwrap_or(*z);
                usize::from(nz != zi)
            })
            .sum();
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A coast at x = 20 (sea west of it) on a 39 m lattice, rising 3 m per
    /// cell to a 60 m plateau, with an inland valley 300 cells (~12 km)
    /// from the sea whose floor lies at 17 m between 25 m banks.
    fn coast_with_inland_valley() -> Lattice {
        let (w, h) = (512, 64);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let land = (x as i32 - 20) * 3_000;
                let mut z = if x < 20 { -20_000 } else { land.min(60_000) };
                if (300..340).contains(&x) {
                    // Valley: 25 m banks, 17 m floor 8 cells wide.
                    z = if (316..324).contains(&x) {
                        17_000
                    } else {
                        25_000
                    };
                }
                g.z[y * w + x] = z;
            }
        }
        g
    }

    fn valley_depth(g: &Lattice) -> i32 {
        let w = g.width;
        let row = 32 * w;
        g.z[row + 305] - g.z[row + 320]
    }

    #[test]
    fn recipe_9_shore_rework_leaves_inland_valleys_alone() {
        let mut old = coast_with_inland_valley();
        let before = valley_depth(&old);
        assert_eq!(before, 8_000);
        rework_shore(&mut old, -LOWSTAND).unwrap();
        let mut new = coast_with_inland_valley();
        let near_before = new.z[32 * new.width + 22];
        rework_shore_near(&mut new, -LOWSTAND).unwrap();
        let (v7, v9) = (valley_depth(&old), valley_depth(&new));
        eprintln!("inland valley depth: {before} mm, recipe 7 {v7} mm, recipe 9 {v9} mm");
        assert!(v7 * 4 < before * 3, "recipe 7 smooths the valley: {v7}");
        assert_eq!(v9, before, "recipe 9 keeps the inland valley");
        // The shore itself is still reworked.
        assert_ne!(
            new.z[32 * new.width + 22],
            near_before,
            "the shore is reworked"
        );
    }

    const LOWSTAND: i32 = super::super::LOWSTAND_MM;

    /// Review 3.1-11: on a coast at 45° the recipe-7 infill steps 8.3 mm
    /// per diagonal, beyond the flats' 8 mm, so part of it is never
    /// regraded. Recipe 9's gradient keeps every step a flat.
    #[test]
    fn recipe_9_infill_on_a_diagonal_coast_is_all_flat() {
        // Sea where x + y < 40; the drowned valley floor everywhere else.
        let (w, h) = (256, 256);
        let macro_mm: Vec<i32> = (0..w * h)
            .map(|i| if i % w + i / w < 40 { -50_000 } else { 20_000 })
            .collect();
        let flat_share = |v9: bool| {
            let mut g = Lattice::new(w, h, 39_062_500).unwrap();
            for (z, &m) in g.z.iter_mut().zip(&macro_mm) {
                *z = if m < 0 { m } else { -5_000 };
            }
            infill_recipe(&mut g, &macro_mm, &vec![0; w * h], v9).unwrap();
            let (mut flat, mut land) = (0, 0);
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let i = y * w + x;
                    if g.z[i] <= 0 || x + y < 120 {
                        continue;
                    }
                    let drop = (0..8)
                        .filter_map(|k| super::super::drainage::neighbour(i, w, h, k))
                        .map(|j| g.z[i] - g.z[j])
                        .max()
                        .unwrap();
                    land += 1;
                    flat += usize::from(drop > 0 && drop <= super::super::flats::FLAT_DROP_MM);
                }
            }
            flat * 1_000 / land.max(1)
        };
        let (v7, v9) = (flat_share(false), flat_share(true));
        eprintln!("infill flagged flat: recipe 7 {v7}‰, recipe 9 {v9}‰");
        assert!(v7 < 900, "recipe 7 leaves infill unflagged: {v7}‰");
        assert!(v9 >= 990, "recipe 9 flags it all: {v9}‰");
    }

    #[test]
    fn long_drowned_valleys_fill_and_short_rias_stay_open() {
        // Coast at x = 10; a valley drowned from the coast to x = 200.
        let (w, h) = (256, 9);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        let macro_mm: Vec<i32> = (0..w * h)
            .map(|i| if i % w < 10 { -50_000 } else { 20_000 })
            .collect();
        for (i, (z, &m)) in g.z.iter_mut().zip(&macro_mm).enumerate() {
            *z = if i / w == 4 && i % w < 200 { -5_000 } else { m };
        }
        let relief = vec![0_u8; w * h];
        infill(&mut g, &macro_mm, &relief).unwrap();
        let row = |x: usize| g.z[4 * w + x];
        // 1.5 km reach = 38 cells from the coast at x = 10.
        assert!(row(20) < 0 && row(45) < 0, "the ria mouth stays open");
        assert!(
            row(60) > 0 && row(150) > row(60),
            "inland it becomes graded land"
        );
    }
}
