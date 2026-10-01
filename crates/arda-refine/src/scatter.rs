//! Vegetation and rock scatter (goal 43).
//!
//! Each layer is a Matérn type-II hard-core process on a global jittered
//! lattice: one candidate per lattice cell with a hashed mark, kept when no
//! other candidate within the layer's spacing has a smaller mark. The
//! result has Poisson-disk spacing and depends on nothing but the seed and
//! global position, so layers continue across block edges. Density fields
//! then thin the kept points independently, which leaves the minimum
//! spacing intact and makes the expected count proportional to density.

use crate::context::Ctx;
use crate::density::{choose, density, woods};
use crate::ecology::eco;
use crate::grid::Grid;
use crate::hash::{hash3, unit};
use crate::shape::Shape;
use crate::terrain::Phys;
use crate::trails::Trails;

/// What a placement stands for, independent of art.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A mature tree whose trunk gives three-quarters cover.
    TreeLarge,
    /// A young or slender tree: half cover.
    TreeSmall,
    /// Undergrowth: difficult terrain.
    Undergrowth,
    /// A boulder: three-quarters cover.
    Boulder,
    /// A rock or stones.
    Rock,
    /// A fallen log: half cover, difficult terrain.
    Log,
    /// Reeds or cattails.
    Reeds,
    /// Lily pads on still water.
    Lilies,
    /// Low plants: flowers, tall grass, heather, ferns, mushrooms.
    Low,
    /// A rock outcrop several squares across: total cover, impassable.
    Outcrop,
}

/// One placed item in global square units.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Global x, squares.
    pub x: f64,
    /// Global y, squares.
    pub y: f64,
    /// What it is.
    pub kind: Kind,
    /// Vocabulary asset id, e.g. `veg.tree_oak`.
    pub asset: &'static str,
    /// Semantic tag, e.g. `tree:broadleaf:large`.
    pub tag: &'static str,
    /// Rotation, degrees clockwise.
    pub rotation: u16,
    /// Mirror flag.
    pub mirror: bool,
}

struct Layer {
    kind: Kind,
    tag: u64,
    grid: f64,
    radius: f64,
}

const LAYERS: [Layer; 9] = [
    Layer {
        kind: Kind::Outcrop,
        tag: 0x7009,
        grid: 7.0,
        radius: 7.5,
    },
    Layer {
        kind: Kind::TreeLarge,
        tag: 0x7001,
        grid: 2.6,
        radius: 2.9,
    },
    Layer {
        kind: Kind::TreeSmall,
        tag: 0x7002,
        grid: 2.2,
        radius: 1.8,
    },
    Layer {
        kind: Kind::Undergrowth,
        tag: 0x7003,
        grid: 1.8,
        radius: 1.3,
    },
    Layer {
        kind: Kind::Boulder,
        tag: 0x7004,
        grid: 2.0,
        radius: 1.7,
    },
    Layer {
        kind: Kind::Log,
        tag: 0x7005,
        grid: 5.0,
        radius: 4.5,
    },
    Layer {
        kind: Kind::Reeds,
        tag: 0x7006,
        grid: 1.4,
        radius: 1.1,
    },
    Layer {
        kind: Kind::Lilies,
        tag: 0x7007,
        grid: 3.0,
        radius: 2.4,
    },
    Layer {
        kind: Kind::Low,
        tag: 0x7008,
        grid: 1.9,
        radius: 1.4,
    },
];

fn floor_i(v: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    let i = v.floor() as i64;
    i
}

fn candidate(seed: u64, l: &Layer, i: i64, j: i64) -> (f64, f64, f64) {
    let hx = unit(hash3(seed, l.tag, i, j, 1));
    let hy = unit(hash3(seed, l.tag, i, j, 2));
    let mark = unit(hash3(seed, l.tag, i, j, 3));
    ((i as f64 + hx) * l.grid, (j as f64 + hy) * l.grid, mark)
}

/// Kept (hard-core) points of a layer inside `[x0, x1) × [y0, y1)`.
fn hard_core(seed: u64, l: &Layer, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64, u64)> {
    let reach = floor_i(l.radius / l.grid) + 1;
    let mut out = Vec::new();
    for j in floor_i(y0 / l.grid)..=floor_i(y1 / l.grid) {
        for i in floor_i(x0 / l.grid)..=floor_i(x1 / l.grid) {
            let (x, y, m) = candidate(seed, l, i, j);
            if x < x0 || x >= x1 || y < y0 || y >= y1 {
                continue;
            }
            let mut kept = true;
            'n: for dj in -reach..=reach {
                for di in -reach..=reach {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let (ox, oy, om) = candidate(seed, l, i + di, j + dj);
                    let d2 = (ox - x) * (ox - x) + (oy - y) * (oy - y);
                    if d2 < l.radius * l.radius && (om, i + di, j + dj) < (m, i, j) {
                        kept = false;
                        break 'n;
                    }
                }
            }
            if kept {
                out.push((x, y, hash3(seed, l.tag ^ 0xFF, i, j, 4)));
            }
        }
    }
    out
}

/// Clearance between two solid items, squares.
fn clearance(a: Kind, b: Kind) -> f64 {
    if a == Kind::Outcrop || b == Kind::Outcrop {
        2.4
    } else {
        1.6
    }
}

/// Whether an item of `kind` stands clear of other solid items.
fn solid(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::TreeLarge | Kind::TreeSmall | Kind::Boulder | Kind::Outcrop
    )
}

/// Every item whose anchor lies in `[x0, x1) × [y0, y1)` (global squares).
/// `phys` and `shape` must cover those squares; `trails` must cover them
/// too, and no item stands on or (if solid) beside a trail.
#[must_use]
pub fn scatter(
    ctx: &Ctx,
    phys: &Grid<Phys>,
    shape: &Grid<Shape>,
    trails: &Trails,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for l in &LAYERS {
        for (x, y, h) in hard_core(ctx.seed, l, x0, y0, x1, y1) {
            let (sx, sy) = (floor_i(x), floor_i(y));
            let (Some(p), Some(s)) = (phys.get(sx, sy), shape.get(sx, sy)) else {
                continue;
            };
            let big = solid(l.kind) || l.kind == Kind::Log;
            if if big {
                trails.near(sx, sy)
            } else {
                trails.at(sx, sy) != crate::trails::Way::None
            } {
                continue;
            }
            let e = eco(ctx, p, *s, x, y);
            let w = woods(ctx, &e, x, y);
            let keep = unit(crate::hash::mix(h));
            if keep >= density(l.kind, &e, &w, p) {
                continue;
            }
            // Keep trunks, boulders and logs apart across layers.
            if (big || l.kind == Kind::Undergrowth)
                && items.iter().any(|o| {
                    let c = clearance(o.kind, l.kind);
                    solid(o.kind) && (o.x - x) * (o.x - x) + (o.y - y) * (o.y - y) < c * c
                })
            {
                continue;
            }
            let r = unit(crate::hash::mix(h ^ 0x5EED));
            let q = unit(crate::hash::mix(h ^ 0x0A1D));
            let (kind, (asset, tag)) = choose(l.kind, &e, &w, r, q);
            items.push(Item {
                x,
                y,
                kind,
                asset,
                tag,
                rotation: [0, 90, 180, 270][usize::try_from((h >> 20) & 3).unwrap_or(0)],
                mirror: (h >> 23) & 1 == 1,
            });
        }
    }
    items
}
