//! River-mouth deltas (logic/02 §world-water deltas, goal 10).
//!
//! Supersedes `coast::deltas`, which built no visible delta. Two causes:
//! 1. Its 2,000 km² catchment threshold exceeds every river of a MICRO
//!    world (seed 42's largest reaches the sea with 565 km²), and deltas
//!    form at far smaller catchments where the coast is low.
//! 2. Formation grades rivers to the glacial lowstand, so a large river
//!    reaches the sea at the head of its drowned valley. Its fan was built
//!    around that ria head, inside the drowned valley: it only shortened
//!    the ria and never built land beyond the coast, and the shore rework
//!    that followed smoothed it away.
//!
//! Here a river of at least [`DELTA_MIN_KM2`] on a low coast (relief mask
//! below 100/255, rising to 160/255 for rivers of 2,000 km², see
//! [`coast_limit_q8`]) builds a delta by filling accommodation with its Holocene
//! sediment ([`TRAPPED_M3_PER_KM2`] per km² of catchment). The fan is a
//! rounded lobe from the mouth, pointing down the river blended with the
//! macro shelf normal, whose reach falls as `√cos θ` off its axis and varies
//! ±30% with bearing, so its front is a chain of rounded sub-lobes. Its
//! radius is bisected until the fill of open water uses the volume, so
//! shallow shelves hold wide deltas and deep water or drowned bays small
//! bayhead deltas. The plain falls, concave, from 3 m at the apex to 0.2 m
//! at the lobe radius, and a delta front shoals the water beyond. A tree of
//! wandering, forking distributaries with natural levees grows over it and
//! is cut into the tidal reach ([`super::delta_net`]), so the distal plain
//! is split into delta islands by open water. Built after the shore rework,
//! so the lobes and channels survive. Steep coasts keep their open rias.

use super::super::drainage::{
    fill, neighbour, open_sea_flags, receiver_index, receivers, upstream_order,
};
use super::super::lattice::{alloc, Lattice};
use super::super::FormationError;
use super::geom::{hash3, isqrt_i, m_to_q8, node_q8, pick, q8_to_um, unit, CELL_Q8, ONE_Q14};
use super::{Delta, WaterFeatures};

/// Least catchment for a delta, km².
pub const DELTA_MIN_KM2: u64 = 150;
/// The catchment threshold of the superseded rule, km².
pub const OLD_DELTA_MIN_KM2: u64 = 2_000;
/// Least catchment whose delta splits into islands, km².
pub const ISLAND_MIN_KM2: u64 = 1_000;
/// Coasts at or above this relief mask keep rias.
pub const LOW_COAST_Q8: u8 = 100;
/// Moderate coasts: rivers of [`LARGE_KM2`] or more still build deltas
/// below this relief mask (the coast rule's limit), their sediment
/// outpacing the steeper nearshore.
pub const MODERATE_COAST_Q8: u8 = 160;
/// Catchment from which the moderate-coast limit applies in full, km².
pub const LARGE_KM2: u64 = 2_000;

/// Relief mask below which a river of `km2` builds a delta: 100/255 up to
/// 1,000 km², rising linearly to 160/255 at 2,000 km².
#[must_use]
pub fn coast_limit_q8(km2: u64) -> u8 {
    let t = km2.saturating_sub(LARGE_KM2 / 2).min(LARGE_KM2 / 2);
    let span = u64::from(MODERATE_COAST_Q8 - LOW_COAST_Q8);
    LOW_COAST_Q8 + u8::try_from(span * t / (LARGE_KM2 / 2)).unwrap_or(0)
}
/// Fan radius per √km², metres.
const M_PER_ROOT_KM2: i64 = 120;
/// Deepest water at the outer edge of the shoaling delta front, mm.
const FRONT_DEPTH_MM: i64 = 8_000;
/// Deepest water a fan front advances into, millimetres (the lowstand
/// base level): deeper water only costs more sediment.
const SHELF_MM: i64 = 130_000;
/// Holocene sediment trapped in a delta per km² of catchment, m³: a yield
/// of 200 t/km²/yr over the 7,000 years since sea level stabilised, at
/// 1.6 t/m³, half of it trapped at the mouth.
const TRAPPED_M3_PER_KM2: i64 = 437_500;
/// Lobes keep this far from the domain rim, metres.
const RIM_M: i64 = 3_000;
/// Fan apex and lobe-edge heights, millimetres.
const APEX_MM: i64 = 3_000;
const EDGE_MM: i64 = 200;

/// Builds deltas. `macro_mm` is the macro surface and `relief_q8` the
/// coastal relief mask, both on `g`'s lattice.
///
/// # Errors
/// Allocation failure.
pub fn build(
    g: &mut Lattice,
    macro_mm: &[i32],
    relief_q8: &[u8],
    seed: u64,
    features: &mut WaterFeatures,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = g.spacing_um / 1_000_000;
    let cell_m2 = super::cell_m2(g);
    // Route on a drained copy so shore pits do not split catchments.
    let mut z: Vec<i32> = alloc(n)?;
    z.copy_from_slice(&g.z);
    let mut flags: Vec<u8> = alloc(n)?;
    open_sea_flags(&z, w, h, &mut flags);
    {
        let mut next: Vec<u32> = alloc(n)?;
        let mut closed: Vec<u8> = alloc(n)?;
        fill(&mut z, w, h, &flags, 1, &mut next, &mut closed)?;
    }
    let mut rcv: Vec<u8> = alloc(n)?;
    receivers(&z, w, h, &flags, 0, 0, &mut rcv);
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
    drop(z);
    let min_cells = DELTA_MIN_KM2 * 1_000_000 / cell_m2;
    let old_cells = OLD_DELTA_MIN_KM2 * 1_000_000 / cell_m2;
    let mut mouths = Vec::new();
    for i in 0..n {
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i && g.z[i] > 0 && flags[r] != 0 && u64::from(area[i]) >= min_cells {
            features.stats.delta_mouths += 1;
            if u64::from(area[i]) >= old_cells {
                features.stats.delta_old_rule_mouths += 1;
            }
            let km2 = u64::from(area[i]) * cell_m2 / 1_000_000;
            if relief_q8[i] < coast_limit_q8(km2) {
                mouths.push(i);
            }
        }
    }
    // Largest rivers first: where several mouths share one ria, the
    // largest builds its delta.
    mouths.sort_by_key(|&i| (std::cmp::Reverse(area[i]), i));
    for &m in &mouths {
        let km2 = u64::from(area[m]) * cell_m2 / 1_000_000;
        let apex = receiver_index(m, w, h, rcv[m]);
        // Several mouths into one bay share one delta.
        let ap_um = q8_to_um(g, node_q8(apex, w));
        if features.deltas.iter().any(|d| {
            let (dx, dy) = (
                (d.apex_um.0 - ap_um.0) / 1_000_000,
                (d.apex_um.1 - ap_um.1) / 1_000_000,
            );
            dx * dx + dy * dy < d.radius_m * d.radius_m
        }) {
            continue;
        }
        // Final flow direction: from ~2 km up the river to the mouth.
        let up = upstream(
            m,
            &rcv,
            &area,
            w,
            h,
            usize::try_from(2_000 / d_m.max(1)).unwrap_or(1),
        );
        let (ap, upp) = (node_q8(apex, w), node_q8(up, w));
        if (ap.0 - upp.0, ap.1 - upp.1) == (0, 0) {
            continue;
        }
        // Lobes build seaward: blend the river's direction with the macro
        // shelf downslope normal over ±1.5 km, the river weighted twice.
        let river = unit(ap.0 - upp.0, ap.1 - upp.1);
        let sea = seaward(
            macro_mm,
            w,
            h,
            apex,
            usize::try_from(1_500 / d_m.max(1)).unwrap_or(1),
        );
        let dir = unit(2 * river.0 + sea.0, 2 * river.1 + sea.1);
        let radius_m =
            M_PER_ROOT_KM2 * i64::try_from(super::super::incision::isqrt(km2)).unwrap_or(0);
        let volume = i64::try_from(km2).unwrap_or(0) * TRAPPED_M3_PER_KM2 * 1_000;
        let Some(built) = fan(g, ap, dir, (radius_m, volume), &flags, seed ^ (m as u64)) else {
            continue;
        };
        let reach = built.reach;
        // Distributaries: a tree of wandering, forking channels with
        // levees, cut into the tidal reach (see `delta_net`).
        let h0 = hash3(seed, m as i64, 3);
        let q = super::nominal_discharge(km2);
        let width_m = i64::from(arda_core::hydrology::channel_width_dm(q).unwrap_or(0)) / 10;
        let w = g.width;
        let inside = |p: (i64, i64)| {
            let x = (p.0 + CELL_Q8 / 2) / CELL_Q8;
            let y = (p.1 + CELL_Q8 / 2) / CELL_Q8;
            x >= 0 && y >= 0 && built.front.contains(&((y as usize) * w + x as usize))
        };
        let channels = super::delta_net::grow(g, ap, dir, reach, width_m, h0, &inside);
        super::delta_net::levees(g, &channels, &built.plain);
        let split = super::delta_net::cut(g, &channels);
        if km2 >= ISLAND_MIN_KM2 && split {
            features.stats.delta_islands += 1;
        }
        let reach_q8 = reach;
        features.deltas.push(Delta {
            apex_um: q8_to_um(g, ap),
            radius_m: reach_q8 * g.spacing_um / CELL_Q8 / 1_000_000,
            catchment_km2: i64::try_from(km2).unwrap_or(i64::MAX),
        });
        features.stats.deltas += 1;
    }
    Ok(())
}

/// Unit vector (Q14) down the macro surface around lattice node `i`.
fn seaward(macro_mm: &[i32], w: usize, h: usize, i: usize, r: usize) -> (i64, i64) {
    let (x, y) = (i % w, i / w);
    let at = |x: usize, y: usize| i64::from(macro_mm[y.min(h - 1) * w + x.min(w - 1)]);
    let gx = at(x + r, y) - at(x.saturating_sub(r), y);
    let gy = at(x, y + r) - at(x, y.saturating_sub(r));
    if (gx, gy) == (0, 0) {
        return (0, 0);
    }
    unit(-gx, -gy)
}

/// The largest-area donor path `steps` cells upstream of `m`.
fn upstream(m: usize, rcv: &[u8], area: &[u32], w: usize, h: usize, steps: usize) -> usize {
    let mut up = m;
    for _ in 0..steps {
        let best = (0..8)
            .filter_map(|k| neighbour(up, w, h, k))
            .filter(|&nb| receiver_index(nb, w, h, rcv[nb]) == up && nb != up)
            .max_by_key(|&nb| (area[nb], std::cmp::Reverse(nb)));
        match best {
            Some(b) => up = b,
            None => break,
        }
    }
    up
}

/// Smooth 1-D noise in −1000..=1000 over the bearing `sin` (Q14), with
/// knots every 0.35 (about five lobes across a fan).
fn lobes(seed: u64, sin: i64) -> i64 {
    let period = 5_734;
    let t = sin + 4 * ONE_Q14;
    let (k, f) = (t.div_euclid(period), t.rem_euclid(period) * 1000 / period);
    let at = |k: i64| pick(hash3(seed, k, 17), 2_001) as i64 - 1_000;
    let sf = f * f * (3_000 - 2 * f) / 1_000_000;
    at(k) + (at(k + 1) - at(k)) * sf / 1000
}

/// Nodes of the lobe at radius `r` (Q8) with their relative position
/// `d / reach` (‰) along their own ray, out to 1.3 × reach (the front).
/// Reach falls as `√cos θ` from the lobe axis, so the fan is a rounded lobe
/// from the mouth, and a smooth function of bearing varies it ±30%.
fn lobe(
    g: &Lattice,
    apex: (i64, i64),
    dir: (i64, i64),
    r: i64,
    seed: u64,
) -> Option<Vec<(usize, i64, i64)>> {
    let (w, h) = (g.width as i64, g.height as i64);
    let cell_m = g.spacing_um / 1_000_000;
    let reach_max = r * 2;
    // A lobe reaching the rim band is refused (`None`), so the radius
    // bisection keeps whole, rounded lobes off the domain rim, which stays
    // open sea: a clipped lobe would end in a straight coast.
    let rim = (RIM_M / cell_m.max(1)).max(1);
    let mut out = Vec::new();
    for y in ((apex.1 - reach_max) / CELL_Q8).max(1)..=((apex.1 + reach_max) / CELL_Q8).min(h - 2) {
        for x in
            ((apex.0 - reach_max) / CELL_Q8).max(1)..=((apex.0 + reach_max) / CELL_Q8).min(w - 2)
        {
            let (dx, dy) = (x * CELL_Q8 - apex.0, y * CELL_Q8 - apex.1);
            let d = isqrt_i(i128::from(dx * dx + dy * dy));
            let cos = if d > 0 {
                (dx * dir.0 + dy * dir.1) / d
            } else {
                ONE_Q14
            };
            if cos <= 0 {
                continue;
            }
            // Lobate front: the reach varies ±30% with the bearing from the
            // apex (a smooth function of angle, about five sub-lobes across
            // the fan), never with position, so the outline is a chain of
            // rounded lobes and the front has no holes.
            let sin = if d > 0 {
                (dy * dir.0 - dx * dir.1) / d
            } else {
                0
            };
            let wob = lobes(seed, sin);
            let reach =
                r * isqrt_i(i128::from(cos * ONE_Q14)) / ONE_Q14 * (1_000 + wob * 3 / 10) / 1_000;
            let rel = d * 1000 / reach.max(1);
            if rel < 1_300 {
                if x < rim || y < rim || x > w - 1 - rim || y > h - 1 - rim {
                    return None;
                }
                out.push(((y as usize) * g.width + x as usize, rel, d));
            }
        }
    }
    Some(out)
}

/// Builds the fan: the lobe radius is the largest (by bisection) whose
/// fill of open water, from the bed to a 1.5 m mean surface, fits the
/// sediment volume. The surface falls radially from 3 m at the apex to
/// 0.2 m at the lobe radius and never lowers land; beyond the edge a delta front shoals
/// open water to at most 8 m deep over 0.3 × reach. Returns the fan, or
/// `None` when fewer than 32 open-water nodes fit.
fn fan(
    g: &mut Lattice,
    apex: (i64, i64),
    dir: (i64, i64),
    (radius_m, volume_mm_m2): (i64, i64),
    flags: &[u8],
    seed: u64,
) -> Option<Fan> {
    let cell_m2 = i64::try_from(super::cell_m2(g)).unwrap_or(1);
    let sea = |j: usize| flags[j] != 0 && g.z[j] <= 0 && i64::from(g.z[j]) >= -SHELF_MM;
    let cost = |cells: &[(usize, i64, i64)]| -> (i64, usize) {
        cells
            .iter()
            .filter(|&&(j, rel, _)| rel < 1_000 && sea(j))
            .fold((0, 0), |(c, n), &(j, _, _)| {
                (c + (1_500 - i64::from(g.z[j])).max(0) * cell_m2, n + 1)
            })
    };
    let (mut lo, mut hi) = (0_i64, m_to_q8(g, radius_m * 3).max(8 * CELL_Q8));
    for _ in 0..20 {
        let mid = (lo + hi + 1) / 2;
        if lobe(g, apex, dir, mid, seed).is_some_and(|c| cost(&c).0 <= volume_mm_m2) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let cells = lobe(g, apex, dir, lo, seed)?;
    if cost(&cells).1 < 32 {
        return None;
    }
    // Only the part connected to the apex builds: the plain through plain
    // nodes, the front through plain or front nodes. Detached noise lobes
    // would otherwise stand offshore as ringed islets.
    let plain = connected(g, &cells, apex, 1_000);
    let front = connected(g, &cells, apex, 1_300);
    let mut surface = Vec::new();
    for &(j, rel, d) in &cells {
        if !front.contains(&j) || (rel < 1_000 && !plain.contains(&j)) {
            continue;
        }
        let z = i64::from(g.z[j]);
        // The plain falls from the apex to the lobe edge, concave: most of
        // it lies low, the basins between the levees barely above the sea.
        let target = if rel < 1_000 {
            let t = 1_000 - d.min(lo) * 1_000 / lo.max(1);
            EDGE_MM + (APEX_MM - EDGE_MM) * t * t / 1_000_000
        } else if flags[j] != 0 && z <= 0 {
            -FRONT_DEPTH_MM * (rel - 1_000) / 300
        } else {
            continue;
        };
        if z < target && (flags[j] != 0 || z > 0) {
            g.z[j] = i32::try_from(target).unwrap_or(g.z[j]);
        }
        if rel < 1_000 {
            surface.push((j, rel, g.z[j]));
        }
    }
    surface.sort_unstable();
    Some(Fan {
        reach: lo,
        plain: surface,
        front,
    })
}

/// A built fan: its radius (Q8), plain nodes `(node, rel ‰, surface mm)`
/// sorted by node, and every node of the plain and front.
struct Fan {
    reach: i64,
    plain: Vec<(usize, i64, i32)>,
    front: std::collections::BTreeSet<usize>,
}

/// Lobe nodes with `rel` below `limit` connected (8-neighbour) to the node
/// nearest the apex.
fn connected(
    g: &Lattice,
    cells: &[(usize, i64, i64)],
    apex: (i64, i64),
    limit: i64,
) -> std::collections::BTreeSet<usize> {
    let inside: std::collections::BTreeSet<usize> = cells
        .iter()
        .filter(|&&(_, rel, _)| rel < limit)
        .map(|&(j, _, _)| j)
        .collect();
    let start = ((apex.1 + CELL_Q8 / 2) / CELL_Q8) as usize * g.width
        + ((apex.0 + CELL_Q8 / 2) / CELL_Q8) as usize;
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![start];
    while let Some(j) = stack.pop() {
        if !seen.insert(j) {
            continue;
        }
        for k in 0..8 {
            if let Some(nb) = neighbour(j, g.width, g.height, k) {
                if inside.contains(&nb) && !seen.contains(&nb) {
                    stack.push(nb);
                }
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_river_in_a_drowned_valley_builds_land_beyond_the_coast() {
        // Low coast at y = 100 (macro), sea shelf 5-30 m deep beyond; a
        // drowned valley reaches 30 cells inland; the river drains a 400 km²
        // equivalent through it (a wide sloping plain funnels to x = 128).
        let (w, h) = (257, 260);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        let mut macro_mm = vec![0; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                macro_mm[i] = if y >= 100 { -20_000 } else { 10_000 };
                let across = (x as i32 - 128).abs() * 400;
                g.z[i] = if y >= 100 {
                    -5_000 - (y as i32 - 100) * 200
                } else if y >= 70 && (x as i32 - 128).abs() <= 2 {
                    -3_000
                } else if y >= 90 {
                    // A coastal ridge: the plain drains only by the valley.
                    60_000
                } else {
                    2_000 + (100 - y as i32) * 300 + across
                };
                if x == 0 || x == w - 1 || y == h - 1 {
                    g.z[i] = -50_000;
                }
            }
        }
        let relief = vec![0_u8; w * h];
        let mut f = WaterFeatures::default();
        // Scale the threshold: this test lattice holds ~8,000 cells above
        // the coast; use a coarse spacing so that is > 150 km².
        g.spacing_um = 156_250_000;
        build(&mut g, &macro_mm, &relief, 9, &mut f).unwrap();
        assert!(f.stats.deltas >= 1, "{:?}", f.stats);
        // The valley head is back-filled and land now stands beyond the
        // macro coast.
        assert_eq!(f.deltas.len(), 1, "one delta per ria: {:?}", f.deltas);
        assert!(g.z[95 * w + 128] > 0, "bayhead plain");
        let beyond = (101 * w..(h - 1) * w).filter(|&i| g.z[i] > 0).count();
        assert!(
            beyond >= 25,
            "{beyond} fan cells beyond the coast: {:?}",
            f.deltas
        );
    }

    #[test]
    fn large_rivers_build_deltas_on_moderate_coasts() {
        assert_eq!(coast_limit_q8(150), LOW_COAST_Q8);
        assert_eq!(coast_limit_q8(1_000), LOW_COAST_Q8);
        assert_eq!(coast_limit_q8(1_500), 130);
        assert_eq!(coast_limit_q8(2_000), MODERATE_COAST_Q8);
        // Seed-7 MICRO: a 4,479 km² river at relief 130/255 built none.
        assert!(130 < coast_limit_q8(4_479));
    }

    #[test]
    fn a_large_river_splits_its_delta_into_islands() {
        // Ported from the superseded coast delta rule: a funnel of 600 x 560
        // nodes (~1,150 km² at 58.6 m) draining to one mouth at (300, 559);
        // a 5 m shelf south of y = 560.
        let (w, h) = (600, 900);
        let mut g = Lattice::new(w, h, 58_593_750).unwrap();
        let mut macro_mm = vec![0; w * h];
        for y in 0..h {
            for x in 0..w {
                let across = (x as i32 - 300).abs();
                let i = y * w + x;
                macro_mm[i] = if y >= 560 { -5_000 } else { 10_000 };
                g.z[i] = if y >= 560 {
                    -5_000
                } else {
                    2_000 + (560 - y as i32) * 20 + across * 60
                };
                if x == 0 || x == w - 1 || y == h - 1 {
                    g.z[i] = -50_000;
                }
            }
        }
        let relief = vec![0_u8; w * h];
        let mut f = WaterFeatures::default();
        build(&mut g, &macro_mm, &relief, 9, &mut f).unwrap();
        assert_eq!(f.stats.deltas, 1, "{:?}", f.stats);
        assert_eq!(f.stats.delta_islands, 1, "{:?}", f.stats);
        // Land components entirely beyond the coast: the island wedge.
        let mut seen = vec![false; w * h];
        let mut islands = 0;
        for s in 561 * w..(h - 1) * w {
            if g.z[s] <= 0 || seen[s] {
                continue;
            }
            let (mut stack, mut touches) = (vec![s], false);
            seen[s] = true;
            while let Some(c) = stack.pop() {
                touches |= c / w <= 560;
                for k in 0..8 {
                    if let Some(nb) = neighbour(c, w, h, k) {
                        if g.z[nb] > 0 && !seen[nb] {
                            seen[nb] = true;
                            stack.push(nb);
                        }
                    }
                }
            }
            islands += usize::from(!touches);
        }
        assert!(islands >= 1, "no delta island");
        // Regression (seed-42 full size: dead-flat, planar delta plains):
        // levees and basins give the plain relief of a metre or so at the
        // 250 m scale, most of it below 3 m, and the front is lobate, so no
        // fan node is more than 1.5 m from the radial cone alone.
        let plain: Vec<usize> = (561 * w..(h - 1) * w)
            .filter(|&i| g.z[i] > 0 && g.z[i] < 6_000)
            .collect();
        assert!(plain.len() > 2_000, "fan land {}", plain.len());
        let rough = plain
            .iter()
            .filter(|&&i| {
                let (x, y) = (i % w, i / w);
                let mut lo = i32::MAX;
                let mut hi = i32::MIN;
                for yy in y - 2..=y + 2 {
                    for xx in x - 2..=x + 2 {
                        let v = g.z[yy * w + xx];
                        if v > 0 {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                }
                hi - lo >= 400
            })
            .count();
        assert!(
            rough * 10 >= plain.len(),
            "levee relief on {rough} of {} plain nodes",
            plain.len()
        );
        // The domain rim stays open sea (3 km, 51 nodes here).
        assert!(
            (0..w).all(|x| (h - 52..h).all(|y| g.z[y * w + x] <= 0)),
            "delta land at the rim"
        );
    }
}
