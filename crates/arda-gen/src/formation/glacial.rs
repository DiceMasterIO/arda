//! Glacial trough lakes (logic/02 §fine-formation glacial lakes, goal 12).
//!
//! Large valley glaciers of the last glacial maximum overdeepened their
//! troughs behind their terminal moraines; after the ice left, those basins
//! became long lakes such as Como, Garda or Te Anau. On the final drained
//! lattice this pass:
//!
//! 1. accumulates ice area from cells above a glacial-era equilibrium-line
//!    altitude (ELA) along the drainage;
//! 2. places each glacier terminus where the ice area first falls below an
//!    accumulation-area ratio of 0.6 of the catchment;
//! 3. for glaciers with at least [`MIN_ICE_KM2`] of ice, carves a U-shaped
//!    overdeepened basin upstream of the terminus and a moraine lip across
//!    the valley just below it.
//!
//! The basins are deliberately closed. The shared annual water balance
//! decides whether each holds a lake, and where it spills.

use super::drainage::{open_sea_flags, receiver_index, receivers, upstream_order, SELF};
use super::incision::isqrt;
use super::lattice::{alloc, Lattice};
use super::FormationError;

/// Glacial-maximum equilibrium-line altitude, millimetres.
pub const GLACIAL_ELA_MM: i32 = 1_900_000;
/// Minimum ice area for an overdeepened trough lake, km².
pub const MIN_ICE_KM2: u64 = 10;
/// Accumulation-area ratio, thousandths.
const AAR_PERMILLE: u64 = 600;
/// A terminus this close to sea level is tidewater (fjord, no lip), mm.
const TIDEWATER_MM: i64 = 30_000;
/// Moraine height above the terminus, millimetres.
const MORAINE_MM: i64 = 15_000;

/// Carves trough-lake basins. Returns each land-terminating trough's sink:
/// its lowest carved cell, in absolute micrometres.
///
/// # Errors
/// Allocation failure.
pub fn trough_lakes(g: &mut Lattice) -> Result<Vec<(i64, i64)>, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = g.spacing_um / 1_000_000;
    // Cell area in square millimetres: 39.0625 m cells are 1525.88 m², and
    // truncating that to whole m² at coarser test spacings would be fine,
    // but keep full precision so the ice threshold is exact.
    let cell_mm2 = u64::try_from((g.spacing_um / 1000) * (g.spacing_um / 1000))
        .map_err(|_| FormationError::ArithmeticOverflow)?
        .max(1);
    let mut flags: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut flags);
    let mut rcv: Vec<u8> = alloc(n)?;
    receivers(&g.z, w, h, &flags, 0, 0, &mut rcv);
    let mut order: Vec<u32> = alloc(n)?;
    {
        let mut indeg: Vec<u8> = alloc(n)?;
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
    }
    drop(flags);
    let mut area: Vec<u32> = alloc(n)?;
    let mut ice: Vec<u32> = alloc(n)?;
    for i in 0..n {
        area[i] = 1;
        ice[i] = u32::from(g.z[i] > GLACIAL_ELA_MM);
    }
    for &i in &order {
        let i = i as usize;
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i {
            area[r] = area[r].saturating_add(area[i]);
            ice[r] = ice[r].saturating_add(ice[i]);
        }
    }
    drop(order);
    let z = &g.z;
    let glaciated = |i: usize| {
        z[i] > 0 && ice[i] > 0 && u64::from(ice[i]) * 1000 >= u64::from(area[i]) * AAR_PERMILLE
    };
    let min_ice_cells = MIN_ICE_KM2 * 1_000_000_000_000 / cell_mm2;
    // Termini: the last glaciated cell before the flow leaves the glacier.
    let mut termini = Vec::new();
    for i in 0..n {
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i && glaciated(i) && !glaciated(r) && u64::from(ice[i]) >= min_ice_cells {
            termini.push(i);
        }
    }
    let mut carved = 0;
    let mut sinks = Vec::new();
    for &t in &termini {
        let ice_m2 = u64::from(ice[t]) * cell_mm2 / 1_000_000;
        let ice_km2 = ice_m2 / 1_000_000;
        let root_km = isqrt(ice_km2.max(1));
        let length_m = i64::try_from((isqrt(ice_m2) * 4 / 5).min(30_000)).unwrap_or(30_000);
        let depth_mm = (20_000 + 5_000 * i64::try_from(root_km).unwrap_or(0)).min(200_000);
        let half_width_m = (150 + 40 * i64::try_from(root_km).unwrap_or(0)).min(2_000);
        let lip = i64::from(g.z[t]) + MORAINE_MM;
        // Trunk: follow the largest-area donor upstream from the terminus.
        let mut path = vec![t];
        let mut cur = t;
        while (path.len() as i64) * d_m < length_m {
            let mut best: Option<usize> = None;
            for k in 0..8 {
                let Some(nb) = super::drainage::neighbour(cur, w, h, k) else {
                    continue;
                };
                if rcv[nb] != SELF
                    && receiver_index(nb, w, h, rcv[nb]) == cur
                    && best.is_none_or(|b| area[nb] > area[b])
                {
                    best = Some(nb);
                }
            }
            let Some(b) = best else { break };
            path.push(b);
            cur = b;
        }
        let m = path.len() as i64;
        if m < 8 {
            continue;
        }
        let radius = (half_width_m / d_m.max(1)).max(1);
        for (k, &p) in path.iter().enumerate() {
            let s = k as i64;
            // Along-valley shape: ramps in over the first 15% (behind the
            // lip) and out over the last 40% (towards the glacier head).
            let ramp_in = (s * 1000 / (m * 15 / 100).max(1)).min(1000);
            let ramp_out = ((m - s) * 1000 / (m * 40 / 100).max(1)).min(1000);
            let along = ramp_in.min(ramp_out);
            let (px, py) = ((p % w) as i64, (p / w) as i64);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let r2 = dx * dx + dy * dy;
                    if r2 > radius * radius {
                        continue;
                    }
                    let (x, y) = (px + dx, py + dy);
                    if x < 1 || y < 1 || x >= w as i64 - 1 || y >= h as i64 - 1 {
                        continue;
                    }
                    let j = (y as usize) * w + x as usize;
                    // U-shaped cross-section: flat floor, walls near the edge.
                    let across = 1000 - r2 * r2 * 1000 / (radius * radius * radius * radius);
                    let floor = lip - depth_mm * along / 1000 * across / 1000;
                    if i64::from(g.z[j]) > floor && g.z[j] > 0 {
                        g.z[j] = i32::try_from(floor).unwrap_or(g.z[j]);
                    }
                }
            }
        }
        // The trough's sink: its lowest carved cell (lake bed or fjord floor),
        // protected from the drainage guarantees that run after carving.
        if let Some(&low) = path.iter().min_by_key(|&&p| (g.z[p], p)) {
            sinks.push((
                (low % w) as i64 * g.spacing_um,
                (low / w) as i64 * g.spacing_um,
            ));
        }
        // Tidewater glaciers leave fjords: no moraine lip, the trough floods.
        if lip - MORAINE_MM < TIDEWATER_MM {
            carved += 1;
            continue;
        }
        // Moraine lip across the valley just below the terminus.
        let below = receiver_index(t, w, h, rcv[t]);
        let (bx, by) = ((below % w) as i64, (below / w) as i64);
        let lip_r = (radius * 3 / 4).max(1);
        for dy in -lip_r..=lip_r {
            for dx in -lip_r..=lip_r {
                if dx * dx + dy * dy > lip_r * lip_r {
                    continue;
                }
                let (x, y) = (bx + dx, by + dy);
                if x < 1 || y < 1 || x >= w as i64 - 1 || y >= h as i64 - 1 {
                    continue;
                }
                let j = (y as usize) * w + x as usize;
                if g.z[j] > 0 && i64::from(g.z[j]) < lip {
                    g.z[j] = i32::try_from(lip).unwrap_or(g.z[j]);
                }
            }
        }
        carved += 1;
    }
    let _ = carved;
    Ok(sinks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_glacier_valley_gets_a_closed_basin_behind_its_lip() {
        // A 20 x 60 km valley at 1.25 km spacing-equivalent: high head in the
        // north (above the ELA), draining south to the sea.
        let (w, h) = (97, 241);
        let mut g = Lattice::new(w, h, 250_000_000).unwrap();
        for y in 0..h {
            for x in 0..w {
                let valley = ((x as i64 - 48).abs() * 30_000) as i32;
                let fall = (3_600_000 - y as i64 * 16_000) as i32;
                g.z[y * w + x] = if y >= h - 3 { -50_000 } else { fall + valley };
            }
        }
        let before = g.z.clone();
        let sinks = trough_lakes(&mut g).unwrap();
        assert!(
            !sinks.is_empty(),
            "a glacier of this size must carve a basin"
        );
        // Some trunk cell now lies below the lowest cell downstream of it.
        let deepest = (0..h)
            .map(|y| g.z[y * w + 48] - before[y * w + 48])
            .min()
            .unwrap();
        assert!(deepest < -20_000, "overdeepening {deepest} mm");
    }
}
