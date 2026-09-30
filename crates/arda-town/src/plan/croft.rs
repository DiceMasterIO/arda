//! The built-up footprint: the open ground a settlement's plots enclose
//! becomes crofts (paddocks, orchards, kitchen gardens, small greens and
//! work yards), so the village is continuous from its core to its rim and
//! the countryside begins at its edge rather than between its houses.
//!
//! The footprint is a morphological closing of the plots, buildings, squares
//! and walls: every open cell within a noisy reach `R₁` of them, less a rim
//! of `R₁ − R₂`, so gaps up to `2·R₁` between plots fill while the outline
//! follows the outermost plots a few squares out. Crofts are split into
//! parcels by a jittered Voronoi lattice of global squares; a parcel's use,
//! hedges and dressing are pure functions of the plan seed and global
//! coordinates, so blocks join without seams.

use super::grid::{Kind, PlanGrid};
use crate::num::floor_i;
use crate::rng::{hash_i, mix, noise2, unit};
use crate::site::Tier;

/// Rows in a band of croft parcels.
const BAND: i64 = 12;
/// Columns in a croft parcel.
const RUN: i64 = 17;

/// What a croft parcel is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// A grazing paddock.
    Paddock,
    /// Fruit trees in grass.
    Orchard,
    /// Vegetable beds.
    Kitchen,
    /// A small hay or flower meadow.
    Meadow,
    /// Beaten earth with stacks and carts.
    Yard,
}

/// Closing radii `(R₁, R₂)` in squares by tier.
const fn radii(tier: Tier) -> (f64, f64) {
    match tier {
        Tier::Hamlet => (12.0, 7.0),
        Tier::Village => (20.0, 14.0),
        Tier::Town | Tier::City => (18.0, 13.0),
    }
}

/// Whether a cell anchors the footprint.
fn anchors(g: &PlanGrid, k: usize) -> bool {
    g.building[k] != 0
        || matches!(
            g.kind[k],
            Kind::Front
                | Kind::Yard
                | Kind::Garden
                | Kind::Building
                | Kind::Churchyard
                | Kind::Square
                | Kind::Green
                | Kind::Bailey
                | Kind::Wall
                | Kind::Gate
                | Kind::Plot
        )
}

/// Chamfer (3-4) distance in squares to the nearest `true` cell.
fn distance(w: usize, h: usize, src: &[bool]) -> Vec<f64> {
    const FAR: u32 = u32::MAX / 4;
    let mut d: Vec<u32> = src.iter().map(|&s| if s { 0 } else { FAR }).collect();
    let at = |x: usize, y: usize| y * w + x;
    for y in 0..h {
        for x in 0..w {
            let mut v = d[at(x, y)];
            if x > 0 {
                v = v.min(d[at(x - 1, y)] + 3);
            }
            if y > 0 {
                v = v.min(d[at(x, y - 1)] + 3);
                if x > 0 {
                    v = v.min(d[at(x - 1, y - 1)] + 4);
                }
                if x + 1 < w {
                    v = v.min(d[at(x + 1, y - 1)] + 4);
                }
            }
            d[at(x, y)] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut v = d[at(x, y)];
            if x + 1 < w {
                v = v.min(d[at(x + 1, y)] + 3);
            }
            if y + 1 < h {
                v = v.min(d[at(x, y + 1)] + 3);
                if x + 1 < w {
                    v = v.min(d[at(x + 1, y + 1)] + 4);
                }
                if x > 0 {
                    v = v.min(d[at(x - 1, y + 1)] + 4);
                }
            }
            d[at(x, y)] = v;
        }
    }
    d.into_iter().map(|v| f64::from(v) / 3.0).collect()
}

/// Turns the open cells of the built-up footprint into crofts.
pub fn fill(g: &mut PlanGrid, tier: Tier, seed: u64) {
    let (w, h) = (
        usize::try_from(g.w).unwrap_or(0),
        usize::try_from(g.h).unwrap_or(0),
    );
    if w == 0 || h == 0 {
        return;
    }
    let (r1, r2) = radii(tier);
    let src: Vec<bool> = (0..g.kind.len()).map(|k| anchors(g, k)).collect();
    let near = distance(w, h, &src);
    let wobble = |k: usize, tag: u64| {
        let (i, j) = g.ij(k);
        #[allow(clippy::cast_precision_loss)] // plan grids are small
        let (x, y) = ((g.gx0 + i) as f64, (g.gy0 + j) as f64);
        3.0 * noise2(seed ^ tag, x / 11.0, y / 11.0) + noise2(seed ^ tag ^ 1, x / 4.0, y / 4.0)
    };
    let grown: Vec<bool> = (0..near.len())
        .map(|k| near[k] <= r1 + wobble(k, 0xC20F))
        .collect();
    let outside: Vec<bool> = grown.iter().map(|&x| !x).collect();
    let inner = distance(w, h, &outside);
    for (k, &inside) in inner.iter().enumerate() {
        let (i, j) = g.ij(k);
        // The grid border counts as outside, so a footprint never runs off
        // the plan's edge.
        let border =
            f64::from(u32::try_from(i.min(j).min(g.w - 1 - i).min(g.h - 1 - j)).unwrap_or(0));
        let depth = inside.min(border + 1.0);
        if g.kind[k] == Kind::Open && depth > r2 {
            g.kind[k] = Kind::Croft;
        }
    }
}

/// The parcel of a global square: bands of [`BAND`] rows, each cut into
/// runs of [`RUN`] columns at its own offset, so parcels are oblong closes
/// with straight hedges, laid like bricks.
#[must_use]
pub fn parcel(seed: u64, gx: i64, gy: i64) -> u64 {
    let band = gy.div_euclid(BAND);
    let shift = i64::try_from(hash_i(seed ^ 0xC20F_7A11, band, 0) % 1024).unwrap_or(0);
    let run = (gx + shift).div_euclid(RUN);
    hash_i(seed ^ 0xC20F_7A12, run, band)
}

/// The use of a parcel.
#[must_use]
pub fn use_of(parcel: u64) -> Use {
    match unit(mix(parcel ^ 0x05E)) {
        u if u < 0.30 => Use::Paddock,
        u if u < 0.52 => Use::Orchard,
        u if u < 0.72 => Use::Kitchen,
        u if u < 0.90 => Use::Meadow,
        _ => Use::Yard,
    }
}

/// Whether a croft square lies on the footprint's rough outer rim: within
/// one to three squares (by noise) of open country.
#[must_use]
pub fn rim(seed: u64, open: impl Fn(i64, i64) -> bool, gx: i64, gy: i64) -> bool {
    #[allow(clippy::cast_precision_loss)] // squares are far below 2^52
    let n = noise2(seed ^ 0x51A1, gx as f64 / 6.0, gy as f64 / 6.0);
    let r = 1 + floor_i((n + 1.0) * 1.2).clamp(0, 2);
    (-r..=r).any(|dy| (-r..=r).any(|dx| dx * dx + dy * dy <= r * r && open(gx + dx, gy + dy)))
}
