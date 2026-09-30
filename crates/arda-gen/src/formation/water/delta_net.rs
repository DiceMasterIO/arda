//! Delta plain relief and distributary networks (logic/02 §world-water
//! deltas, goal 10).
//!
//! A delta plain is not a planar cone. Its channels build natural levees,
//! low ridges that stand a metre or two above the plain and fall away over
//! a few channel widths; between them lie interdistributary basins barely
//! above sea level. The channels themselves wander (sinuosity about 1.15,
//! a wavelength of eleven widths) and bifurcate as they advance, each
//! mouth building its own sub-lobe, so the front is lobate.
//!
//! Here the distributaries grow as a tree from the apex: every branch
//! walks down the fan with a sine-generated heading about its own course
//! (bends of about 32° at eleven widths, sinuosity about 1.1) that also wanders ±25° at the
//! scale of the fan,
//! pulled gently back towards the radial direction so the tree spreads
//! rather than curls, and forks into two at ±12-20° while enough reach
//! remains. Channels deepen downstream through the tidal reach: where the
//! plain lies below 1.5 m a channel is cut to 1.5 m below sea level and
//! opens to the sea, so the distal plain is split into delta islands by
//! water, not by a routed line. Levees are raised along every channel
//! before it is cut; the final fills drain the basins between them.

use super::super::lattice::Lattice;
use super::geom::{
    carve_disc, hash3, isqrt_i, m_to_q8, pick, rotate, sin_q14, unit, CELL_Q8, ONE_Q14, TURN,
};

/// Levee crest above the local plain at the apex and at the lobe edge, mm.
const LEVEE_APEX_MM: i64 = 1_400;
const LEVEE_EDGE_MM: i64 = 400;
/// A channel's bed lies 1.5 m below sea level where the plain is lower than
/// this, millimetres (the tidal reach).
const TIDAL_PLAIN_MM: i64 = 1_500;
/// Tidal bed below sea level, millimetres.
const TIDAL_BED_MM: i64 = 1_500;
/// Least carve half-width of a tidal channel, metres: the 100 m grid must
/// see it as continuous water.
const TIDAL_HALF_M: i64 = 90;

/// One distributary: its centreline (Q8), carve half-width (Q8), depth
/// below the plain (mm) and whether it forked into the tidal reach.
#[derive(Debug, Clone)]
pub struct Channel {
    /// Centreline points, Q8.
    pub points: Vec<(i64, i64)>,
    /// Carve half-width, Q8.
    pub half: i64,
    /// Depth below the plain above the tidal reach, millimetres.
    pub depth_mm: i64,
    /// Branch generation (0 = trunk).
    pub generation: u32,
}

/// How many times the tree forks for a lobe of `reach_m` metres.
#[must_use]
pub fn generations(reach_m: i64) -> u32 {
    match reach_m {
        ..3_000 => 0,
        3_000..8_000 => 1,
        8_000..16_000 => 2,
        _ => 3,
    }
}

/// Grows the distributary tree of a lobe with apex `apex` (Q8), axis `dir`
/// (Q14 unit) and radius `reach` (Q8). `inside(p)` says whether a Q8 point
/// still lies on the fan or its front; `width_m` is the trunk's bankfull
/// width. Channels end one wavelength beyond the fan into open water.
#[must_use]
pub fn grow(
    g: &Lattice,
    apex: (i64, i64),
    dir: (i64, i64),
    reach: i64,
    width_m: i64,
    seed: u64,
    inside: &dyn Fn((i64, i64)) -> bool,
) -> Vec<Channel> {
    let reach_m = reach * g.spacing_um / CELL_Q8 / 1_000_000;
    let max_gen = generations(reach_m);
    let ds = CELL_Q8 / 2;
    let mut out = Vec::new();
    // (start, heading, discharge share ‰, generation, travelled so far)
    let mut stack = vec![(apex, dir, 1_000_i64, 0_u32, 0_i64)];
    let mut id = 0_i64;
    while let Some((start, heading, share, gen, done)) = stack.pop() {
        id += 1;
        let h = hash3(seed, id, i64::from(gen));
        // Width scales as the square root of discharge (4 m × Q^0.5).
        let w_m = (width_m * isqrt_i(i128::from(share) * 1_000) / 1_000).max(30);
        let lam = m_to_q8(g, (11 * w_m).max(800));
        // Plan-form wander at the scale of the fan: ±25° over knots a
        // quarter of the reach apart (at least 1.5 km).
        let knot = (reach / 4).max(m_to_q8(g, 1_500));
        let phase0 = pick(h, TURN as u64) as i64;
        // Bends: a heading amplitude of 28-36°, sinuosity about 1.1.
        let omega = TURN * (28 + pick(hash3(h, 1, 0), 9) as i64) / 360;
        // Fork after 30-50% of the reach still ahead of this branch.
        let ahead = (reach - done).max(0);
        let fork_at = if gen < max_gen && ahead > reach * 3 / 10 {
            ahead * (300 + pick(hash3(h, 2, 0), 200) as i64) / 1_000
        } else {
            i64::MAX
        };
        let mut pos = start;
        let mut points = vec![pos];
        let mut s = 0_i64;
        let mut outside = 0_i64;
        let limit = reach * 2;
        while s < limit {
            // Radial direction from the apex keeps the tree spreading.
            let radial = if (pos.0 - apex.0, pos.1 - apex.1) == (0, 0) {
                dir
            } else {
                unit(pos.0 - apex.0, pos.1 - apex.1)
            };
            let base = unit(heading.0 * 3 + radial.0, heading.1 * 3 + radial.1);
            let swing = omega * sin_q14(phase0 + s * TURN / lam.max(1)) / ONE_Q14;
            let drift = TURN * 25 / 360 * wander(h, done + s, knot) / 1_000;
            let hd = rotate(base, swing + drift);
            pos = (pos.0 + hd.0 * ds / ONE_Q14, pos.1 + hd.1 * ds / ONE_Q14);
            s += ds;
            points.push(pos);
            if s >= fork_at {
                let spread = TURN * (18 + pick(hash3(h, 3, 0), 13) as i64) / 360;
                let left = 450 + pick(hash3(h, 4, 0), 200) as i64;
                stack.push((
                    pos,
                    rotate(hd, -spread),
                    share * left / 1_000,
                    gen + 1,
                    done + s,
                ));
                stack.push((
                    pos,
                    rotate(hd, spread),
                    share * (1_000 - left) / 1_000,
                    gen + 1,
                    done + s,
                ));
                break;
            }
            if inside(pos) {
                outside = 0;
            } else if super::geom::height_at(g, pos) > 0 {
                // Off the fan onto older land: the channel ends, it never
                // trenches the valley sides.
                points.pop();
                break;
            } else {
                // Beyond the front in open water: run on half a wavelength
                // so the mouth opens cleanly.
                outside += ds;
                if outside > lam / 2 {
                    break;
                }
            }
        }
        let depth_mm = (1_000 + 1_200 * isqrt_i(i128::from(share) * 1_000) / 1_000).min(2_200);
        out.push(Channel {
            points,
            half: m_to_q8(g, (w_m / 2).max(60)),
            depth_mm,
            generation: gen,
        });
    }
    out
}

/// Smooth 1-D noise in −1000..=1000 over `t` with knots every `period`.
fn wander(seed: u64, t: i64, period: i64) -> i64 {
    let period = period.max(1);
    let (k, f) = (t.div_euclid(period), t.rem_euclid(period) * 1000 / period);
    let at = |k: i64| pick(hash3(seed, k, 5), 2_001) as i64 - 1_000;
    let sf = f * f * (3_000 - 2 * f) / 1_000_000;
    at(k) + (at(k + 1) - at(k)) * sf / 1000
}

/// Raises natural levees along `channels` over the plain nodes `plain`:
/// sorted `(node, rel, base)` with the radial position `rel` (‰ of the
/// reach) and the fan surface `base` (mm) before any levee. The crest
/// stands 1.4 m above the plain at the apex and 0.4 m at the lobe edge and
/// falls to it over four half-widths (at least 250 m): steep toward the
/// channel, a long back slope into the basins. Only raises.
pub fn levees(g: &mut Lattice, channels: &[Channel], plain: &[(usize, i64, i32)]) {
    let w = g.width;
    for ch in channels {
        let reach = (ch.half * 4).max(m_to_q8(g, 250));
        for p in ch.points.iter().step_by(2) {
            let (x0, x1) = (
                ((p.0 - reach) / CELL_Q8).max(0),
                (p.0 + reach) / CELL_Q8 + 1,
            );
            let (y0, y1) = (
                ((p.1 - reach) / CELL_Q8).max(0),
                (p.1 + reach) / CELL_Q8 + 1,
            );
            for y in y0..=y1 {
                let row = (y as usize) * w;
                let lo = plain.partition_point(|&(j, _, _)| j < row + x0 as usize);
                for &(j, rel, base) in plain[lo..]
                    .iter()
                    .take_while(|&&(j, _, _)| j <= row + x1 as usize)
                {
                    let (x, yy) = ((j % w) as i64, (j / w) as i64);
                    let (dx, dy) = (x * CELL_Q8 - p.0, yy * CELL_Q8 - p.1);
                    let d = isqrt_i(i128::from(dx * dx + dy * dy));
                    if d > reach {
                        continue;
                    }
                    let crest =
                        LEVEE_APEX_MM + (LEVEE_EDGE_MM - LEVEE_APEX_MM) * rel.min(1_000) / 1_000;
                    let t = (d - ch.half).max(0) * 1_000 / (reach - ch.half).max(1);
                    let rise = crest * (1_000 - t) * (1_000 - t) / 1_000_000;
                    let target = i64::from(base) + rise;
                    if i64::from(g.z[j]) < target {
                        g.z[j] = i32::try_from(target).unwrap_or(g.z[j]);
                    }
                }
            }
        }
    }
}

/// Cuts every channel: one depth below the plain, and through the tidal
/// reach (plain below 1.5 m) to 1.5 m below sea level at least 90 m
/// half-wide, so the distal channels are open water. Returns whether the
/// trunk and at least one fork reached open water.
pub fn cut(g: &mut Lattice, channels: &[Channel]) -> bool {
    let mut tidal = 0;
    for ch in channels {
        let mut reached = false;
        for &p in &ch.points {
            let surface = i64::from(super::geom::height_at(g, p));
            if surface <= 0 {
                reached = true;
                continue;
            }
            let (bed, half) = if surface < TIDAL_PLAIN_MM {
                (-TIDAL_BED_MM, ch.half.max(m_to_q8(g, TIDAL_HALF_M)))
            } else {
                ((surface - ch.depth_mm).max(-TIDAL_BED_MM), ch.half)
            };
            carve_disc(g, p, half, bed, surface - bed);
            // `carve_disc` keeps land above 0; open the tidal bed itself.
            if bed < 0 {
                let x = ((p.0 + CELL_Q8 / 2) / CELL_Q8).clamp(1, g.width as i64 - 2);
                let y = ((p.1 + CELL_Q8 / 2) / CELL_Q8).clamp(1, g.height as i64 - 2);
                let r = (half * 3 / 5 + CELL_Q8 - 1) / CELL_Q8;
                for yy in (y - r).max(1)..=(y + r).min(g.height as i64 - 2) {
                    for xx in (x - r).max(1)..=(x + r).min(g.width as i64 - 2) {
                        let (dx, dy) = (xx * CELL_Q8 - p.0, yy * CELL_Q8 - p.1);
                        if dx * dx + dy * dy > (half * 3 / 5) * (half * 3 / 5) {
                            continue;
                        }
                        let j = (yy as usize) * g.width + xx as usize;
                        if i64::from(g.z[j]) > bed
                            && g.z[j] <= i32::try_from(TIDAL_PLAIN_MM).unwrap_or(0)
                        {
                            g.z[j] = i32::try_from(bed).unwrap_or(g.z[j]);
                        }
                    }
                }
            }
        }
        tidal += usize::from(reached);
    }
    tidal >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_fans_grow_forking_sinuous_distributaries() {
        // Regression (seed-42 full size: three straight rays from the apex).
        let g = Lattice::new(1_200, 1_200, 39_062_500).unwrap();
        let apex = (600 * CELL_Q8, 100 * CELL_Q8);
        let reach = m_to_q8(&g, 18_000);
        let inside = |p: (i64, i64)| {
            let (dx, dy) = (p.0 - apex.0, p.1 - apex.1);
            dy >= 0 && dx * dx + dy * dy <= reach * reach
        };
        let chans = grow(&g, apex, (0, ONE_Q14), reach, 50, 11, &inside);
        assert!(
            chans.len() >= 7,
            "three forks deep: {} channels",
            chans.len()
        );
        assert!(chans.iter().any(|c| c.generation == 3));
        for c in chans.iter().filter(|c| c.points.len() > 200) {
            let len: i64 = c
                .points
                .windows(2)
                .map(|w| {
                    isqrt_i(i128::from(
                        (w[1].0 - w[0].0).pow(2) + (w[1].1 - w[0].1).pow(2),
                    ))
                })
                .sum();
            let (a, b) = (c.points[0], c.points[c.points.len() - 1]);
            let chord = isqrt_i(i128::from((b.0 - a.0).pow(2) + (b.1 - a.1).pow(2)));
            assert!(
                len * 1_000 >= chord * 1_050,
                "sinuosity {}‰",
                len * 1_000 / chord.max(1)
            );
        }
        // Small fans keep a single channel.
        assert_eq!(generations(2_000), 0);
    }
}
