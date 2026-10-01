//! Margin-aware bathymetry on the 1 km macro lattice (logic/02
//! §fine-formation bathymetry, goal 18).
//!
//! Open ocean follows a profile of distance to the coast and that margin's
//! activity: a narrow, steep shelf off active margins (convergent plate
//! boundaries nearby, or mountainous coasts), a wide one off passive
//! margins, a continental slope, then an abyssal plain.
//! - [`apply`] runs on the smooth macro lattice before any formation level
//!   and may only deepen, so every level, the lowstand base level and the
//!   coast infill see one consistent seafloor.
//! - [`shelf_fill`] runs on the formed lattice and may also shallow: on
//!   passive margins the shelf is a sediment prism built out over the
//!   margin, so macro ocean that is too deep near the coast is raised to
//!   the profile (it never reaches sea level).
//!
//! Enclosed basins are not open ocean and are untouched.

use super::drainage::{open_sea_flags, FIXED};
use super::incision::isqrt;
use super::lattice::{alloc, blur_into, Lattice};
use super::FormationError;

/// Shelf width off low (passive) coasts, metres.
pub const SHELF_PASSIVE_M: i64 = 60_000;
/// Recipe-5 shelf width off low (passive) coasts, metres.
pub const SHELF_PASSIVE_V5_M: i64 = 80_000;
/// Shelf width off mountainous (active) coasts, metres.
pub const SHELF_ACTIVE_M: i64 = 8_000;
/// Depth at the shelf break, millimetres.
pub const SHELF_BREAK_MM: i64 = 130_000;
/// Continental slope width, metres.
pub const SLOPE_M: i64 = 40_000;
/// Abyssal plain depth, millimetres.
pub const ABYSS_MM: i64 = 4_000_000;
/// Coastal relief (m) at which a margin counts as fully active.
const ACTIVE_RELIEF_M: i64 = 400;
/// Smoothing radius of the margin label, lattice cells.
const LABEL_BLUR_CELLS: usize = 15;

/// Profile depth (mm, positive down) at `dist_m` from a coast whose shelf
/// is `shelf_m` wide: roughly linear shelf (t^0.75), smoothstep slope.
#[must_use]
pub fn profile_mm(dist_m: i64, shelf_m: i64) -> i64 {
    if dist_m <= shelf_m {
        let t = (dist_m * 4_096 / shelf_m.max(1)).clamp(0, 4_096);
        let s1 = i64::try_from(isqrt(u64::try_from(t << 12).unwrap_or(0))).unwrap_or(0);
        let s2 = i64::try_from(isqrt(u64::try_from(s1 << 12).unwrap_or(0))).unwrap_or(0);
        let t075 = (s1 * s2) >> 12;
        return 2_000 + (SHELF_BREAK_MM - 2_000) * t075 / 4_096;
    }
    let t = ((dist_m - shelf_m) * 4_096 / SLOPE_M).clamp(0, 4_096);
    let s = t * t / 4_096 * (3 * 4_096 - 2 * t) / 4_096;
    SHELF_BREAK_MM + (ABYSS_MM - SHELF_BREAK_MM) * s / 4_096
}

/// Shelf width for a coastal relief in metres.
fn shelf_m(relief_m: i64) -> i64 {
    let a = relief_m.clamp(0, ACTIVE_RELIEF_M);
    SHELF_PASSIVE_M - (SHELF_PASSIVE_M - SHELF_ACTIVE_M) * a / ACTIVE_RELIEF_M
}

/// Shelf width for a margin activity (0 passive ..= 255 active), metres.
#[must_use]
pub fn shelf_for_activity_m(activity: u8) -> i64 {
    shelf_m(i64::from(activity) * ACTIVE_RELIEF_M / 255)
}

/// Combined margin activity of a coastal relief label (m) and the
/// tectonic activity (0..=255): whichever is more active wins.
fn combined_activity(relief_m: i64, tectonic: u8) -> u8 {
    let from_relief = relief_m.clamp(0, ACTIVE_RELIEF_M) * 255 / ACTIVE_RELIEF_M;
    u8::try_from(from_relief.max(i64::from(tectonic))).unwrap_or(255)
}

/// Labelled Euclidean distance from non-ocean cells: distance in metres
/// and the macro relief (m) of the nearest source cell.
///
/// A two-pass vector propagation (8SSEDT): every cell carries the offset to
/// its nearest source, so distance is the true Euclidean length rather than
/// a chamfer sum. A chamfer's octagonal isolines showed through the shelf
/// prism as straight 45° and axis-aligned steps across the open ocean.
fn distance_and_label(
    open: &[u8],
    relief_m: &[i32],
    w: usize,
    h: usize,
    d_m: i32,
) -> Result<(Vec<i32>, Vec<i32>), FormationError> {
    const FAR: i32 = i32::MAX / 4;
    let n = w * h;
    let mut off: Vec<(i32, i32)> = alloc(n)?;
    let mut label: Vec<i32> = alloc(n)?;
    for i in 0..n {
        if open[i] & FIXED == 0 {
            label[i] = relief_m[i];
        } else {
            off[i] = (FAR, FAR);
        }
    }
    let len2 = |o: (i32, i32)| i64::from(o.0) * i64::from(o.0) + i64::from(o.1) * i64::from(o.1);
    let mut relax = |i: usize, j: usize, dx: i32, dy: i32, off: &mut [(i32, i32)]| {
        let o = off[j];
        if o.0 >= FAR {
            return;
        }
        let c = (o.0 + dx, o.1 + dy);
        if len2(c) < len2(off[i]) {
            off[i] = c;
            label[i] = label[j];
        }
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if y > 0 {
                relax(i, i - w, 0, 1, &mut off);
                if x > 0 {
                    relax(i, i - w - 1, 1, 1, &mut off);
                }
                if x + 1 < w {
                    relax(i, i - w + 1, -1, 1, &mut off);
                }
            }
            if x > 0 {
                relax(i, i - 1, 1, 0, &mut off);
            }
        }
        for x in (0..w.saturating_sub(1)).rev() {
            let i = y * w + x;
            relax(i, i + 1, -1, 0, &mut off);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if y + 1 < h {
                relax(i, i + w, 0, -1, &mut off);
                if x + 1 < w {
                    relax(i, i + w + 1, -1, -1, &mut off);
                }
                if x > 0 {
                    relax(i, i + w - 1, 1, -1, &mut off);
                }
            }
            if x + 1 < w {
                relax(i, i + 1, -1, 0, &mut off);
            }
        }
        for x in 1..w {
            let i = y * w + x;
            relax(i, i - 1, 1, 0, &mut off);
        }
    }
    let mut dist: Vec<i32> = alloc(n)?;
    for (d, &o) in dist.iter_mut().zip(&off) {
        *d = if o.0 >= FAR {
            i32::MAX / 2
        } else {
            let d2 = u64::try_from(len2(o) * i64::from(d_m) * i64::from(d_m)).unwrap_or(u64::MAX);
            i32::try_from(isqrt(d2)).unwrap_or(i32::MAX / 2)
        };
    }
    Ok((dist, label))
}

/// The margin profile on the macro lattice, kept for the formed lattice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shelf {
    /// Profile depth (mm, negative) of open-ocean cells; 0 elsewhere.
    pub target: Lattice,
    /// Combined margin activity 0 (passive) ..= 255 (active) per cell.
    pub activity: Lattice,
}

/// Replaces open-ocean macro depths with the margin profile. Beyond the
/// margin, deeper macro ocean (trenches, rifts) is kept. `activity` is the
/// tectonic margin activity per macro cell (0..=255), or empty for none.
/// Returns the profile for [`shelf_fill`].
///
/// # Errors
/// Allocation failure.
pub fn apply(
    macro_mm: &mut Lattice,
    relief_m: &Lattice,
    activity: &[u8],
) -> Result<Shelf, FormationError> {
    let (w, h) = (macro_mm.width, macro_mm.height);
    let n = w * h;
    let d_m = i32::try_from(macro_mm.spacing_um / 1_000_000)
        .map_err(|_| FormationError::ArithmeticOverflow)?
        .max(1);
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags(&macro_mm.z, w, h, &mut open);
    // The domain rim is flagged even where it is land; only open water
    // below sea level counts as ocean here.
    for (f, &z) in open.iter_mut().zip(&macro_mm.z) {
        if z > 0 {
            *f = 0;
        }
    }
    let (dist, label) = distance_and_label(&open, &relief_m.z, w, h, d_m)?;
    let mut tmp: Vec<i32> = alloc(n)?;
    let mut smooth: Vec<i32> = alloc(n)?;
    blur_into(&label, w, h, LABEL_BLUR_CELLS, &mut tmp, &mut smooth);
    // The tectonic label is margin-scale too: smooth it like the relief.
    let tect: Vec<i32> = (0..n)
        .map(|i| i32::from(activity.get(i).copied().unwrap_or(0)))
        .collect();
    let mut tect_smooth: Vec<i32> = alloc(n)?;
    blur_into(&tect, w, h, LABEL_BLUR_CELLS, &mut tmp, &mut tect_smooth);
    let mut shelf = Shelf {
        target: Lattice::new(w, h, macro_mm.spacing_um)?,
        activity: Lattice::new(w, h, macro_mm.spacing_um)?,
    };
    for i in 0..n {
        let tect_i = u8::try_from(tect_smooth[i].clamp(0, 255)).unwrap_or(255);
        let act = combined_activity(i64::from(smooth[i]), tect_i);
        shelf.activity.z[i] = i32::from(act);
        if open[i] & FIXED == 0 || dist[i] >= i32::MAX / 2 {
            continue;
        }
        let dist_m = i64::from(dist[i]);
        let target = -profile_mm(dist_m, shelf_for_activity_m(act));
        shelf.target.z[i] = i32::try_from(target).unwrap_or(i32::MIN);
        let z = i64::from(macro_mm.z[i]);
        // Only deepen: coasts, land fraction and shallow macro shelves are
        // unchanged; active margins become narrow and steep.
        let nz = z.min(target);
        macro_mm.z[i] = i32::try_from(nz).unwrap_or(macro_mm.z[i]);
    }
    Ok(shelf)
}

/// Shelf-fill ceiling: a raised seafloor stays at least this deep, mm.
const SHELF_FILL_TOP_MM: i32 = 600;

/// Passive-margin sediment prism on the formed lattice (logic/02
/// §fine-formation bathymetry, goal 18). Open sea deeper than the margin
/// profile `target` (per cell, from [`apply`] through the macro warp) is
/// raised toward it, fully on passive margins and not at all on fully
/// active ones (`activity` per cell, 0..=255). Nothing is lowered, no cell
/// reaches sea level, and enclosed water is untouched. Returns cells raised.
///
/// # Errors
/// Allocation failure.
pub fn shelf_fill(
    g: &mut Lattice,
    target: &[i32],
    activity: &[u8],
) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut open);
    let mut raised = 0;
    for (i, (z, &f)) in g.z.iter_mut().zip(&open).enumerate() {
        let t = target.get(i).copied().unwrap_or(0).min(-SHELF_FILL_TOP_MM);
        if f & FIXED == 0 || *z > 0 || *z >= t {
            continue;
        }
        let act = i64::from(activity.get(i).copied().unwrap_or(255));
        let zi = i64::from(*z);
        let nz = zi + (i64::from(t) - zi) * (255 - act) / 255;
        if nz != zi {
            *z = i32::try_from(nz).unwrap_or(*z);
            raised += 1;
        }
    }
    Ok(raised)
}

/// Recipe-5 shelf width for a coastal relief in metres.
fn shelf_m_v5(relief_m: i64) -> i64 {
    let a = relief_m.clamp(0, ACTIVE_RELIEF_M);
    SHELF_PASSIVE_V5_M - (SHELF_PASSIVE_V5_M - SHELF_ACTIVE_M) * a / ACTIVE_RELIEF_M
}

/// Recipe 5: labelled two-pass chamfer from non-ocean cells, distance in
/// metres and the macro relief (m) of the source cell.
fn distance_and_label_chamfer(
    open: &[u8],
    relief_m: &[i32],
    w: usize,
    h: usize,
    d_m: i32,
) -> Result<(Vec<i32>, Vec<i32>), FormationError> {
    let n = w * h;
    let mut dist: Vec<i32> = alloc(n)?;
    let mut label: Vec<i32> = alloc(n)?;
    for i in 0..n {
        if open[i] & FIXED == 0 {
            label[i] = relief_m[i];
        } else {
            dist[i] = i32::MAX / 2;
        }
    }
    let diag = d_m * 181 / 128;
    let mut relax = |i: usize, j: usize, step: i32, dist: &mut [i32]| {
        let c = dist[j].saturating_add(step);
        if c < dist[i] {
            dist[i] = c;
            label[i] = label[j];
        }
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                relax(i, i - 1, d_m, &mut dist);
            }
            if y > 0 {
                relax(i, i - w, d_m, &mut dist);
                if x > 0 {
                    relax(i, i - w - 1, diag, &mut dist);
                }
                if x + 1 < w {
                    relax(i, i - w + 1, diag, &mut dist);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                relax(i, i + 1, d_m, &mut dist);
            }
            if y + 1 < h {
                relax(i, i + w, d_m, &mut dist);
                if x + 1 < w {
                    relax(i, i + w + 1, diag, &mut dist);
                }
                if x > 0 {
                    relax(i, i + w - 1, diag, &mut dist);
                }
            }
        }
    }
    Ok((dist, label))
}

/// Recipe 5 (v0.1): replaces open-ocean macro depths with the relief-only
/// margin profile ([`SHELF_PASSIVE_V5_M`] passive shelves, chamfer
/// distance). Beyond the margin, deeper macro ocean is kept.
///
/// # Errors
/// Allocation failure.
pub fn apply_v5(macro_mm: &mut Lattice, relief_m: &Lattice) -> Result<(), FormationError> {
    let (w, h) = (macro_mm.width, macro_mm.height);
    let n = w * h;
    let d_m = i32::try_from(macro_mm.spacing_um / 1_000_000)
        .map_err(|_| FormationError::ArithmeticOverflow)?
        .max(1);
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags(&macro_mm.z, w, h, &mut open);
    // The domain rim is flagged even where it is land; only open water
    // below sea level counts as ocean here.
    for (f, &z) in open.iter_mut().zip(&macro_mm.z) {
        if z > 0 {
            *f = 0;
        }
    }
    let (dist, label) = distance_and_label_chamfer(&open, &relief_m.z, w, h, d_m)?;
    let mut tmp: Vec<i32> = alloc(n)?;
    let mut smooth: Vec<i32> = alloc(n)?;
    blur_into(&label, w, h, LABEL_BLUR_CELLS, &mut tmp, &mut smooth);
    for i in 0..n {
        if open[i] & FIXED == 0 || dist[i] >= i32::MAX / 2 {
            continue;
        }
        let dist_m = i64::from(dist[i]);
        let shelf = shelf_m_v5(i64::from(smooth[i]));
        let target = -profile_mm(dist_m, shelf);
        let z = i64::from(macro_mm.z[i]);
        // Only deepen: coasts, land fraction and shallow macro shelves are
        // unchanged; active margins become narrow and steep.
        let nz = z.min(target);
        macro_mm.z[i] = i32::try_from(nz).unwrap_or(macro_mm.z[i]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coast_distance_is_euclidean_not_octagonal() {
        // One land cell at the centre of open ocean: distance isolines are
        // circles, so a diagonal and an axis cell at the same true distance
        // agree (a chamfer overestimates off-axis directions by up to 8%).
        let (w, h) = (201, 201);
        let mut open = vec![FIXED; w * h];
        open[100 * w + 100] = 0;
        let relief = vec![7; w * h];
        let (dist, label) = distance_and_label(&open, &relief, w, h, 1_000).unwrap();
        assert_eq!(dist[100 * w + 190], 90_000);
        // (100 + 60, 100 + 67) is 89.94 cells away, (100+80, 100+41) 89.89.
        assert_eq!(dist[167 * w + 160], 89_938);
        assert_eq!(dist[141 * w + 180], 89_894);
        assert!(label.iter().all(|&l| l == 7));
    }

    #[test]
    fn passive_shelves_are_wide_and_active_ones_narrow() {
        assert!(profile_mm(20_000, shelf_m(0)) < SHELF_BREAK_MM);
        assert!(profile_mm(20_000, shelf_m(ACTIVE_RELIEF_M)) > SHELF_BREAK_MM);
        assert!(profile_mm(200_000, shelf_m(0)) > 3_000_000, "abyss far out");
        assert!(profile_mm(0, shelf_m(100)) <= 2_000, "shore is shallow");
        // Roughly linear, not a pale near-shore flat: 25% out is > 25 m.
        assert!(profile_mm(20_000, 80_000) > 25_000);
    }

    #[test]
    fn enclosed_basins_keep_their_macro_floor() {
        // 40 x 40 km: ocean on the west quarter, a closed -200 m basin inside.
        let (w, h) = (40, 40);
        let mut m = Lattice::new(w, h, 1_000_000_000).unwrap();
        for y in 0..h {
            for x in 0..w {
                m.z[y * w + x] = if x < 10 { -3_000_000 } else { 300_000 };
            }
        }
        m.z[20 * w + 30] = -200_000;
        let relief = Lattice::new(w, h, 1_000_000_000).unwrap();
        apply(&mut m, &relief, &[]).unwrap();
        assert_eq!(m.z[20 * w + 30], -200_000, "basin untouched");
        assert!(m.z[20 * w + 9] >= -3_000_000, "never shallower than macro");
    }

    #[test]
    fn passive_shelves_shallow_and_active_ones_keep_their_depth() {
        // Land on the west 10 cells (39 m), then a 2 km-deep cliff of sea.
        let (w, h) = (400, 6);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for (i, z) in g.z.iter_mut().enumerate() {
            *z = if i % w < 10 { 5_000 } else { -2_000_000 };
        }
        let mut active = g.clone();
        let target: Vec<i32> = (0..w * h)
            .map(|i| -profile_mm((i % w) as i64 * 39 - 390, shelf_m(0)) as i32)
            .collect();
        shelf_fill(&mut g, &target, &vec![0; w * h]).unwrap();
        shelf_fill(&mut active, &target, &vec![255; w * h]).unwrap();
        let x = 3 * w + 60; // 2 km offshore
        assert!(g.z[x] > -60_000 && g.z[x] < 0, "passive shelf {}", g.z[x]);
        assert_eq!(active.z[x], -2_000_000, "active margin unchanged");
        assert!(g.z.iter().all(|&z| z < 0 || z == 5_000), "no new land");
    }
}
