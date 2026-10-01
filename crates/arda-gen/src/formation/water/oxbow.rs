//! Oxbow lakes (logic/02 §world-water oxbows, goal 12).
//!
//! Meander cutoffs leave abandoned loops beside the belt. They are planned
//! while the meanders are carved and carved only after the channels have
//! been drained again, so each crescent is a local closed basin; its
//! mid-arc is a protected sink and the annual water balance decides whether
//! it holds an oxbow lake.

use super::super::lattice::Lattice;
use super::super::FormationError;
use super::geom::{hash3, m_to_q8, pick, q8_to_um, rotate, CELL_Q8, ONE_Q14, TURN};
use super::meander::{
    carve_depth_mm, channel_radius, Plan, BELT_CUT_MM, FLOODPLAIN_TILT_PERMILLE, FLOOR_RISE_MM,
    OXBOW_MIN_FLOOR_MM,
};
use super::network::Stream;
use super::{Sink, SinkKind, WaterFeatures};

/// A planned cutoff loop: crescent arc points (Q8), carve half-width (Q8)
/// and depth below its lowest floor (mm).
pub struct Cutoff {
    arc: Vec<(i64, i64)>,
    radius: i64,
    depth: i64,
    /// Contributing area of the river it was cut from, lattice cells.
    river_cells: u32,
}

/// Carves planned cutoff crescents that capture no channel to a flat
/// bottom below their lowest floor and protects each mid-arc as an oxbow
/// sink. Runs after the
/// drainage fill that follows channel carving, so each crescent is a local
/// closed basin and never joins a pit left by the channels.
///
/// # Errors
/// Allocation failure.
pub fn carve_cutoffs(
    g: &mut Lattice,
    cutoffs: &[Cutoff],
    features: &mut WaterFeatures,
    runoff: Option<&[u16]>,
) -> Result<(), FormationError> {
    if cutoffs.is_empty() {
        return Ok(());
    }
    // Drainage of the carved, re-drained surface: a loop may not capture a
    // river, including courses moved by other meander spans.
    let net = super::network::build(g, runoff)?;
    let floor_cells = u32::try_from(CAPTURE_MAX_M2 / super::cell_m2(g).max(1)).unwrap_or(u32::MAX);
    for c in cutoffs {
        let small = floor_cells.max(c.river_cells / 5);
        if !clear_of_channels(g, &net.area, c, small) {
            continue;
        }
        let floors: Vec<i64> = c
            .arc
            .iter()
            .map(|&q| i64::from(super::geom::height_at(g, q)))
            .collect();
        let Some(low) = floors.iter().copied().min() else {
            continue;
        };
        let bottom = low - c.depth;
        let radius = c.radius.max(m_to_q8(g, MIN_HALF_WIDTH_M));
        let (x0, x1, y0, y1) = bounds(g, c, radius);
        let before: Vec<i32> = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (y, x)))
            .map(|(y, x)| g.z[y * g.width + x])
            .collect();
        for (&q, &f) in c.arc.iter().zip(&floors) {
            carve_flat(g, q, radius, bottom, f - bottom);
        }
        // The loop must hold one lake the 100 m hydrology sees whole: enough
        // 100 m samples at least 0.3 m below its rim, all in one 8-connected
        // piece. A loop the samples would split or barely see is undone.
        if !whole_at_samples(g, c, radius, low - 300) {
            let mut k = 0;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    g.z[y * g.width + x] = before[k];
                    k += 1;
                }
            }
            continue;
        }
        // The whole crescent is protected, not only its mid-arc: the later
        // fills and the 100 m sampled pass would otherwise lift every part
        // that does not drain to the protected point at 100 m, and leave a
        // one- to three-cell fragment (seed-42 full size: 59 lakes).
        for &q in c.arc.iter().step_by(2) {
            let (x_um, y_um) = q8_to_um(g, q);
            features.sinks.push(Sink {
                x_um,
                y_um,
                radius_um: radius * g.spacing_um / CELL_Q8,
                kind: SinkKind::Oxbow,
            });
        }
        features.stats.oxbows += 1;
    }
    Ok(())
}

/// Lowers nodes within `radius` of `centre` to a flat `bottom` over the
/// inner 85% of the radius, rising as a parabola to `bottom + rise` at the
/// edge: an abandoned channel's flat, silted bed, which the 100 m samples
/// read at one depth. Only lowers, and only land.
fn carve_flat(g: &mut Lattice, centre: (i64, i64), radius: i64, bottom: i64, rise: i64) {
    let (w, h) = (g.width as i64, g.height as i64);
    let r = radius.max(1);
    let inner = r * 17 / 20;
    for y in ((centre.1 - r) / CELL_Q8).max(1)..=((centre.1 + r) / CELL_Q8 + 1).min(h - 2) {
        for x in ((centre.0 - r) / CELL_Q8).max(1)..=((centre.0 + r) / CELL_Q8 + 1).min(w - 2) {
            let (dx, dy) = (x * CELL_Q8 - centre.0, y * CELL_Q8 - centre.1);
            let d = super::geom::isqrt_i(i128::from(dx * dx + dy * dy));
            if d > r {
                continue;
            }
            let j = (y as usize) * g.width + x as usize;
            if g.z[j] <= 0 {
                continue;
            }
            let t = (d - inner).max(0);
            let target = bottom + rise * t * t / ((r - inner).max(1) * (r - inner).max(1));
            if i64::from(g.z[j]) > target {
                g.z[j] = i32::try_from(target.max(1)).unwrap_or(g.z[j]);
            }
        }
    }
}

/// Least carve half-width of a loop, metres: narrower loops fall between
/// the 100 m samples.
const MIN_HALF_WIDTH_M: i64 = 90;
/// Least 100 m samples a loop must hold 0.3 m below its rim: the smallest
/// oxbow lake the world publishes, 0.08 km².
pub const MIN_SAMPLES: usize = 8;

/// Lattice node bounds (inclusive) of a loop and its carve radius.
fn bounds(g: &Lattice, c: &Cutoff, radius: i64) -> (usize, usize, usize, usize) {
    let (w, h) = (g.width as i64, g.height as i64);
    let xs = c.arc.iter().map(|q| q.0);
    let ys = c.arc.iter().map(|q| q.1);
    let lo = |v: i64, n: i64| ((v - radius) / CELL_Q8 - 1).clamp(0, n - 1) as usize;
    let hi = |v: i64, n: i64| ((v + radius) / CELL_Q8 + 1).clamp(0, n - 1) as usize;
    (
        lo(xs.clone().min().unwrap_or(0), w),
        hi(xs.max().unwrap_or(0), w),
        lo(ys.clone().min().unwrap_or(0), h),
        hi(ys.max().unwrap_or(0), h),
    )
}

/// Whether the 100 m point samples (as the prepared bed takes them) on a
/// loop that lie below `level` number at least [`MIN_SAMPLES`], form one
/// 8-connected piece, and are closed by samples at `level` or above.
fn whole_at_samples(g: &Lattice, c: &Cutoff, radius: i64, level: i64) -> bool {
    const SAMPLE_UM: i64 = 100_000_000;
    let (x0, x1, y0, y1) = bounds(g, c, radius);
    let span = |a: usize, b: usize| {
        let a_um = a as i64 * g.spacing_um;
        let b_um = b as i64 * g.spacing_um;
        (a_um + SAMPLE_UM - 1) / SAMPLE_UM..=b_um / SAMPLE_UM
    };
    let at = |p: (i64, i64)| {
        i64::from(super::super::sampled::sample(
            g,
            i128::from(p.0 * SAMPLE_UM),
            i128::from(p.1 * SAMPLE_UM),
        ))
    };
    let mut below = std::collections::BTreeMap::new();
    for sy in span(y0, y1) {
        for sx in span(x0, x1) {
            // Only samples on the loop itself.
            let q = (
                sx * SAMPLE_UM * CELL_Q8 / g.spacing_um,
                sy * SAMPLE_UM * CELL_Q8 / g.spacing_um,
            );
            let on_loop = c
                .arc
                .iter()
                .any(|a| (a.0 - q.0).pow(2) + (a.1 - q.1).pow(2) <= radius * radius);
            if on_loop && at((sx, sy)) < level {
                below.insert((sx, sy), at((sx, sy)));
            }
        }
    }
    if below.len() < MIN_SAMPLES {
        return false;
    }
    // The lake the 100 m hydrology sees when the loop is full: it spills at
    // the lowest sample around the loop. It must be at least 0.3 m deep and
    // cover every loop sample in one 8-connected piece. A loop breached by
    // lower ground (a delta distributary, another channel) would otherwise
    // be a protected, dry, drained flat: pits in the published bed (seed-42
    // full size, on a delta plain), or a fragment.
    let near = |(x, y): (i64, i64)| {
        (-1..=1_i64).flat_map(move |dy| (-1..=1_i64).map(move |dx| (x + dx, y + dy)))
    };
    let spill = below
        .keys()
        .flat_map(|&p| near(p))
        .filter(|p| !below.contains_key(p) && p.0 >= 0 && p.1 >= 0)
        .map(at)
        .min()
        .unwrap_or(i64::MIN);
    let Some((&start, &low)) = below.iter().min_by_key(|&(p, z)| (*z, *p)) else {
        return false;
    };
    if spill - low < 300 {
        return false;
    }
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut stack = vec![start];
    while let Some(p) = stack.pop() {
        for q in near(p) {
            if below.get(&q).is_some_and(|&z| z < spill) && seen.insert(q) {
                stack.push(q);
            }
        }
    }
    seen.len() == below.len()
}

/// Least spacing of cutoff loops along one stream, in wavelengths.
const CUTOFF_SPACING_L: i64 = 2;

/// A cutoff loop may capture floodplain side streams (yazoo streams along
/// the belt edge) below this catchment or a fifth of its river's, m²: they
/// become its inlets. It never captures the river itself.
const CAPTURE_MAX_M2: u64 = 5_000_000;

/// Every node within the carve radius plus 100 m of the arc drains less
/// than `small` cells.
fn clear_of_channels(g: &Lattice, area: &[u32], c: &Cutoff, small: u32) -> bool {
    let reach = c.radius + m_to_q8(g, 100);
    let r = reach / CELL_Q8 + 1;
    c.arc.iter().all(|&q| {
        let (qx, qy) = (q.0 / CELL_Q8, q.1 / CELL_Q8);
        (-r..=r).all(|dy| {
            (-r..=r).all(|dx| {
                let (x, y) = (qx + dx, qy + dy);
                let (ex, ey) = (x * CELL_Q8 - q.0, y * CELL_Q8 - q.1);
                ex * ex + ey * ey > reach * reach
                    || x < 0
                    || y < 0
                    || x >= g.width as i64
                    || y >= g.height as i64
                    || area[(y as usize) * g.width + x as usize] < small
            })
        })
    })
}

/// Lowers nodes within `r` of `c` to a floodplain `floor` rising
/// [`FLOODPLAIN_TILT_PERMILLE`] with distance from the belt axis (a point
/// and its unit normal), cutting at most [`BELT_CUT_MM`] +
/// [`FLOOR_RISE_MM`]; the outer fifth blends back.
fn plane_pad(
    g: &mut Lattice,
    c: (i64, i64),
    r: i64,
    (axis, n): ((i64, i64), (i64, i64)),
    floor: i64,
) {
    let (w, h) = (g.width as i64, g.height as i64);
    let tilt = g.spacing_um / 1000 / CELL_Q8 * FLOODPLAIN_TILT_PERMILLE;
    for y in ((c.1 - r) / CELL_Q8).max(1)..=((c.1 + r) / CELL_Q8 + 1).min(h - 2) {
        for x in ((c.0 - r) / CELL_Q8).max(1)..=((c.0 + r) / CELL_Q8 + 1).min(w - 2) {
            let (px, py) = (x * CELL_Q8, y * CELL_Q8);
            let d = super::geom::isqrt_i(i128::from((px - c.0).pow(2) + (py - c.1).pow(2)));
            if d > r {
                continue;
            }
            let lat = (((px - axis.0) * n.0 + (py - axis.1) * n.1) / ONE_Q14).abs();
            let plain = floor + lat * tilt / 1000;
            let j = (y as usize) * g.width + x as usize;
            let z = i64::from(g.z[j]);
            if z <= plain || z - floor > BELT_CUT_MM + FLOOR_RISE_MM {
                continue;
            }
            let blend = ((d * 1000 / r.max(1) - 800).max(0) * 5).min(1000);
            g.z[j] = i32::try_from(plain + (z - plain) * blend / 1000).unwrap_or(g.z[j]);
        }
    }
}

/// Plans cutoff loops beside the belt. At each bend where the course swings
/// to at least 70% of its amplitude (hashed, more often the more sinuous
/// the reach), a crescent of radius 0.2 λ is placed beyond the bend with
/// its tips towards the course and two carve radii plus 150 m from it, a
/// rim wide enough to survive the 100 m bilinear sample. It needs floor
/// within 5.5 m of the bed, which is planed into a floodplain pad.
#[allow(clippy::too_many_arguments)]
pub(super) fn plan(
    g: &mut Lattice,
    s: &Stream,
    area: &[u32],
    p: &Plan,
    curve: &[((i64, i64), usize)],
    (a, b): (usize, usize),
    seed: u64,
    cutoffs: &mut Vec<Cutoff>,
) {
    // Lateral offset of each curve point from the floor middle, Q8.
    let lateral = |&(q, k): &((i64, i64), usize)| {
        let (t, c) = (s.dir[k], s.centre[k]);
        let n = (-t.1, t.0);
        ((q.0 - c.0) * n.0 + (q.1 - c.1) * n.1) / ONE_Q14 - p.shift[k]
    };
    let mut last_along = i64::MIN / 2;
    for i in 1..curve.len().saturating_sub(1) {
        let (e_prev, e, e_next) = (
            lateral(&curve[i - 1]),
            lateral(&curve[i]),
            lateral(&curve[i + 1]),
        );
        let kc = curve[i].1;
        if kc <= a || kc >= b || !(e.abs() >= e_prev.abs() && e.abs() > e_next.abs()) {
            continue;
        }
        let lam = p.lambda[kc];
        let amp = p.amp[kc];
        if amp == 0 || e.abs() * 10 < amp * 7 || s.along[kc] - last_along < lam * CUTOFF_SPACING_L {
            continue;
        }
        let s_t = p.sinuosity[kc];
        // Cutoffs need tight bends: probability rises with sinuosity, to half
        // of the eligible bends at a sinuosity of 2.2. At most one loop in
        // two wavelengths, so loops never string along a reach like beads.
        if s_t < 1_300 || pick(hash3(seed, i as i64, 0), 1000) as i64 > (s_t - 1_200) / 2 {
            continue;
        }
        let t = s.dir[kc];
        let n = (-t.1, t.0);
        let radius = (lam * 20 / 100).clamp(m_to_q8(g, 150), m_to_q8(g, 1_500));
        let r_c = channel_radius(g, s, area, kc);
        // The abandoned loop lies beyond the bend it was cut from, its tips
        // two carve radii plus 150 m from the present course.
        let side = if e > 0 { 1 } else { -1 };
        let clear = 2 * r_c + m_to_q8(g, 100);
        let off = p.shift[kc] + e + side * (clear + radius / 2);
        let c = (
            s.centre[kc].0 + n.0 * off / ONE_Q14,
            s.centre[kc].1 + n.1 * off / ONE_Q14,
        );
        // Arc from -120° to +120° about the outward normal: tips point back
        // towards the course.
        let out_dir = (n.0 * side, n.1 * side);
        let steps = (radius * 4 / (CELL_Q8 / 2)).max(8);
        let arc: Vec<(i64, i64)> = (0..=steps)
            .map(|k| {
                let ang = -TURN / 3 + (TURN * 2 / 3) * k / steps;
                let d = rotate(out_dir, ang);
                (c.0 + d.0 * radius / ONE_Q14, c.1 + d.1 * radius / ONE_Q14)
            })
            .collect();
        let bed = s.bed(g, kc);
        let on_floor = bed >= OXBOW_MIN_FLOOR_MM
            && arc.iter().all(|&q| {
                let z = i64::from(super::geom::height_at(g, q));
                z > 0 && z <= bed + FLOOR_RISE_MM + BELT_CUT_MM
            });
        let apart = arc.iter().all(|&q| {
            curve.iter().all(|&(cp, _)| {
                let (dx, dy) = (q.0 - cp.0, q.1 - cp.1);
                dx * dx + dy * dy > clear * clear
            })
        });
        if !on_floor || !apart {
            continue;
        }
        last_along = s.along[kc];
        // The migrating belt once planed this floodplain: plane a pad
        // around the loop, rising 3‰ away from the belt axis so it drains
        // to the river rather than into the loop.
        let axis = (
            s.centre[kc].0 + n.0 * p.shift[kc] / ONE_Q14,
            s.centre[kc].1 + n.1 * p.shift[kc] / ONE_Q14,
        );
        plane_pad(g, c, radius + r_c + CELL_Q8, (axis, n), bed + 300);
        cutoffs.push(Cutoff {
            arc,
            radius: r_c,
            depth: carve_depth_mm(g, s, area, kc) + 300,
            river_cells: area[s.cells[kc]],
        });
    }
}

#[cfg(test)]
mod tests;
