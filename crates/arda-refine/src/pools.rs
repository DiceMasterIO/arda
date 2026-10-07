//! Standing pools on land: rare, coherent patches of shallow water in real
//! hollows, on floodplains and in marshes.
//!
//! Each pool is a site on a jittered lattice of [`SPACING`] squares. A site
//! holds water only where the world makes it wet enough (a marsh cell, a
//! low floodplain or a closed hollow in the smooth terrain), decided once at
//! the site, and its outline is a warped disc no smaller than
//! [`MIN_RADIUS`] squares. Scattered speckle cannot arise: every pool is a
//! whole patch around its site, a pure function of global position.

use crate::context::Ctx;
use crate::hash::{hash2, mix, signed, unit};
use crate::noise::{fbm, smoothstep};
use arda::{Cover, TerrainKind};

/// Lattice spacing of pool sites, squares.
pub const SPACING: f64 = 32.0;
/// Smallest pool radius, squares.
pub const MIN_RADIUS: f64 = 3.0;
/// Largest pool radius, squares (kept below the spacing).
const MAX_RADIUS: f64 = 11.0;
/// Outline warp as a fraction of the radius.
const WARP: f64 = 0.4;
/// Distance of the hollow test's rim samples from the site, squares.
const RIM: f64 = 10.0;
/// How far a site must lie below its rim to count as a hollow, metres.
const HOLLOW_M: f64 = 0.25;

/// A candidate site in lattice cell `(i, j)`: position and two draws.
fn site(ctx: &Ctx, i: i64, j: i64) -> (f64, f64, u64) {
    let h = hash2(ctx.seed, 0x9001, i, j);
    let jx = unit(hash2(ctx.seed, 0x9002, i, j));
    let jy = unit(hash2(ctx.seed, 0x9003, i, j));
    #[allow(clippy::cast_precision_loss)] // lattice indices are small
    let (x, y) = (
        (i as f64 + 0.2 + 0.6 * jx) * SPACING,
        (j as f64 + 0.2 + 0.6 * jy) * SPACING,
    );
    (x, y, h)
}

/// Wetness of the site's cell neighbourhood: `(marsh, floodplain)`, each
/// `0..1`.
fn wetness(ctx: &Ctx, u: f64, v: f64) -> (f64, f64) {
    // Marsh cover or saturated ground (`biome::marshy`).
    let marsh = ctx.bilinear(u, v, |c, _| crate::biome::marshy(c));
    let flood = ctx.bilinear(u, v, |c, _| {
        if c.terrain != TerrainKind::Land || c.watercourse_order > 0 {
            return 0.0;
        }
        let wet = f64::from(c.wetness) / 255.0;
        let low = 1.0 - smoothstep(0.3, 1.5, f64::from(c.height_above_river_dm) / 10.0);
        low * smoothstep(0.45, 0.8, wet)
    });
    (marsh, flood)
}

/// Whether the smooth terrain at the site lies in a closed hollow: below
/// every one of eight rim samples.
fn hollow(ctx: &Ctx, u: f64, v: f64) -> bool {
    let here = ctx.base_height(u, v);
    let d = RIM * std::f64::consts::FRAC_1_SQRT_2;
    [
        (RIM, 0.0),
        (-RIM, 0.0),
        (0.0, RIM),
        (0.0, -RIM),
        (d, d),
        (d, -d),
        (-d, d),
        (-d, -d),
    ]
    .iter()
    .all(|&(dx, dy)| ctx.base_height(u + dx, v + dy) - here > HOLLOW_M)
}

/// Whether a cell may hold marsh or floodplain pools.
#[must_use]
pub fn wet_cell(c: &arda::Cell) -> bool {
    if c.terrain != TerrainKind::Land {
        return true;
    }
    let wet = f64::from(c.wetness) / 255.0;
    c.cover == Cover::Marsh
        || crate::biome::marshy(c) > 0.5
        || (c.watercourse_order == 0 && c.height_above_river_dm < 15 && wet > 0.45)
}

/// How far a pool at `(x, y)` may reach before a cell that holds no pools,
/// squares (sides and corners of its own cell).
fn room(ctx: &Ctx, x: f64, y: f64) -> f64 {
    let k = Ctx::cell_of(x, y);
    #[allow(clippy::cast_precision_loss)] // cell indices are small
    let (x0, y0) = (k.x as f64 * 64.0, k.y as f64 * 64.0);
    let mut best = f64::INFINITY;
    for dy in -1..=1_i64 {
        for dx in -1..=1_i64 {
            if (dx, dy) == (0, 0) || wet_cell(ctx.cell_at(k.offset(dx, dy))) {
                continue;
            }
            let gx = match dx {
                -1 => x - x0,
                1 => x0 + 64.0 - x,
                _ => 0.0,
            };
            let gy = match dy {
                -1 => y - y0,
                1 => y0 + 64.0 - y,
                _ => 0.0,
            };
            best = best.min(gx.hypot(gy));
        }
    }
    best
}

/// The pool radius at a site, squares, and its outline's largest reach per
/// unit radius; `None` for a dry site.
fn radius(ctx: &Ctx, x: f64, y: f64, h: u64, spread: f64) -> Option<f64> {
    let draw = unit(h);
    let (marsh, flood) = wetness(ctx, x, y);
    let size = unit(mix(h ^ 0x9004));
    let r = (MIN_RADIUS + (4.0 + 4.0 * marsh) * size).min(MAX_RADIUS);
    let p = 0.55 * marsh + 0.25 * flood;
    if draw < p && wet_cell(ctx.cell_at(Ctx::cell_of(x, y))) {
        // A marsh or floodplain pool stays within the wet cells.
        let fit = room(ctx, x, y) / spread;
        return (fit >= MIN_RADIUS).then(|| r.min(fit));
    }
    // Elsewhere only a closed hollow holds water.
    let wet = ctx.bilinear(x, y, |c, _| f64::from(c.wetness) / 255.0);
    (draw < p + 0.25 + 0.35 * wet && hollow(ctx, x, y)).then_some(r)
}

/// Pool indicator at global square position `(u, v)`: positive inside a
/// pool, by how many squares.
#[must_use]
pub fn pool(ctx: &Ctx, u: f64, v: f64) -> f64 {
    #[allow(clippy::cast_possible_truncation)] // lattice indices are small
    let (i0, j0) = ((u / SPACING).floor() as i64, (v / SPACING).floor() as i64);
    let reach = MAX_RADIUS * (1.0 + WARP) * std::f64::consts::SQRT_2;
    let mut best = f64::NEG_INFINITY;
    for j in j0 - 1..=j0 + 1 {
        for i in i0 - 1..=i0 + 1 {
            let (x, y, h) = site(ctx, i, j);
            if (u - x).hypot(v - y) > reach {
                continue;
            }
            // An ellipse up to twice as long as wide, at a random heading.
            let (c, s) = (signed(mix(h ^ 0x9006)), signed(mix(h ^ 0x9008)));
            let n = c.hypot(s).max(1e-9);
            let (c, s) = (c / n, s / n);
            let stretch = 1.0 + unit(mix(h ^ 0x9007));
            let Some(r) = radius(ctx, x, y, h, (1.0 + WARP) * stretch.sqrt()) else {
                continue;
            };
            let (a, b) = ((u - x) * c + (v - y) * s, (v - y) * c - (u - x) * s);
            let d = (a / stretch.sqrt()).hypot(b * stretch.sqrt());
            let bend = WARP * r * fbm(ctx.seed, 0x9005 ^ h, u, v, 1.5 * r, 2, 0.5);
            best = best.max(r - d - bend);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::CellKey;
    use crate::synthetic;

    /// Connected pools over the marsh of the synthetic world: every whole
    /// pool (one not cut by the sampled area's edge) has at least the
    /// minimum patch size.
    #[test]
    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // small indices
    fn pools_are_whole_patches_never_speckle() {
        let src = synthetic::world(42);
        let (mx, my) = synthetic::MARSH[0];
        let ctx = Ctx::gather(&src, CellKey::new(mx, my)).unwrap();
        let (x0, y0, n) = (mx * 64 - 32, my * 64 - 32, 128_usize);
        let wet: Vec<bool> = (0..n * n)
            .map(|i| {
                let (x, y) = (x0 + (i % n) as i64, y0 + (i / n) as i64);
                pool(&ctx, x as f64 + 0.5, y as f64 + 0.5) > 0.0
            })
            .collect();
        assert!(wet.iter().any(|&w| w), "a marsh holds some pools");
        let mut seen = vec![false; n * n];
        for start in 0..n * n {
            if !wet[start] || seen[start] {
                continue;
            }
            let (mut stack, mut size, mut edge) = (vec![start], 0, false);
            seen[start] = true;
            while let Some(k) = stack.pop() {
                size += 1;
                let (x, y) = (k % n, k / n);
                edge |= x == 0 || y == 0 || x == n - 1 || y == n - 1;
                let nb = [
                    (x > 0).then(|| k - 1),
                    (x + 1 < n).then(|| k + 1),
                    (y > 0).then(|| k - n),
                    (y + 1 < n).then(|| k + n),
                ];
                for j in nb.into_iter().flatten() {
                    if wet[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            assert!(edge || size >= 12, "a pool of {size} squares");
        }
    }
}
