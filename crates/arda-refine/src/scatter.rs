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
use crate::grid::Grid;
use crate::hash::{hash3, unit};
use crate::noise::{fbm, smoothstep};
use crate::terrain::{Phys, Water};
use arda::Cover;

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

const LAYERS: [Layer; 8] = [
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
        grid: 2.6,
        radius: 2.2,
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
        grid: 3.0,
        radius: 2.5,
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

/// Per-point fields a layer's density reads.
struct Here<'a> {
    p: &'a Phys,
    fd: f64,
    scrub: f64,
    marsh: f64,
    meadow: f64,
    temp: f64,
    rocky: f64,
}

fn frac(ctx: &Ctx, u: f64, v: f64, cover: Cover) -> f64 {
    ctx.bilinear(u, v, |c, _| if c.cover == cover { 1.0 } else { 0.0 })
}

fn density(kind: Kind, h: &Here, ctx: &Ctx, x: f64, y: f64) -> f64 {
    let p = h.p;
    let dry = p.water == Water::Dry;
    let firm = dry && p.slope_deg < 38.0 && p.river_d > 0.6 && p.stand_v < -0.34;
    match kind {
        Kind::TreeLarge | Kind::TreeSmall | Kind::Log => {
            if !firm {
                return 0.0;
            }
            // Groves and glades: the clumping factor averages about 0.78
            // and never exceeds 1, so the count stays proportional to density.
            let clump = smoothstep(-0.45, 0.45, fbm(ctx.seed, 0xC1, x, y, 18.0, 2, 0.5));
            let f = h.fd * (0.55 + 0.45 * clump);
            match kind {
                Kind::TreeLarge => f,
                Kind::TreeSmall => (0.3 * f + 0.15 * h.scrub).min(1.0),
                _ => 0.18 * h.fd,
            }
        }
        Kind::Undergrowth => {
            if !firm {
                return 0.0;
            }
            (0.22 * h.fd + 0.45 * h.scrub + 0.05).min(1.0)
        }
        Kind::Boulder | Kind::Rock => {
            if !dry || p.river_d < 0.3 {
                return 0.0;
            }
            (0.04 + 0.5 * h.rocky + 0.6 * smoothstep(20.0, 42.0, p.slope_deg)).min(0.9)
        }
        Kind::Reeds => {
            let shore = if dry {
                1.0 - smoothstep(0.3, 2.0, p.river_d.min(-p.stand_v * 30.0))
            } else if p.depth_m < 0.6 {
                0.8
            } else {
                0.0
            };
            let calm = if p.water == Water::Sea || p.stand_kind == Water::Sea {
                0.1
            } else {
                1.0
            };
            (shore * calm * (0.35 + 0.6 * h.marsh) + 0.5 * h.marsh * f64::from(u8::from(dry)))
                .min(0.95)
                * (1.0 - smoothstep(4.0, 1.0, h.temp + 3.0))
        }
        Kind::Lilies => match p.water {
            Water::Lake | Water::Pool if p.depth_m < 1.8 => 0.35,
            _ => 0.0,
        },
        Kind::Low => {
            if !firm {
                return 0.0;
            }
            0.04 + 0.2 * h.meadow + 0.12 * h.scrub
        }
    }
}

fn choose(kind: Kind, h: &Here, r: f64) -> (&'static str, &'static str) {
    match kind {
        Kind::TreeLarge | Kind::TreeSmall => {
            let cold = smoothstep(7.0, 2.0, h.temp);
            let wet = 1.0 - smoothstep(0.5, 5.0, h.p.river_d.min(-h.p.stand_v * 40.0));
            let large = kind == Kind::TreeLarge;
            if r < 0.03 {
                ("veg.tree_dead", "tree:dead")
            } else if r < 0.03 + 0.5 * wet {
                (
                    "veg.tree_willow",
                    if large {
                        "tree:broadleaf:large"
                    } else {
                        "tree:broadleaf:small"
                    },
                )
            } else if r < 0.2 + 0.75 * cold {
                if r < 0.1 + 0.45 * cold {
                    (
                        "veg.tree_spruce",
                        if large {
                            "tree:conifer:large"
                        } else {
                            "tree:conifer:small"
                        },
                    )
                } else {
                    (
                        "veg.tree_pine",
                        if large {
                            "tree:conifer:large"
                        } else {
                            "tree:conifer:small"
                        },
                    )
                }
            } else if !large || r > 0.85 {
                ("veg.tree_birch", "tree:broadleaf:small")
            } else if r > 0.62 {
                ("veg.tree_elm", "tree:broadleaf:large")
            } else {
                ("veg.tree_oak", "tree:broadleaf:large")
            }
        }
        Kind::Undergrowth => {
            if h.fd > 0.5 && r < 0.45 {
                ("veg.fern", "undergrowth:fern")
            } else if r < 0.15 {
                ("veg.bush_flowering", "undergrowth:bush")
            } else {
                ("veg.bush", "undergrowth:bush")
            }
        }
        Kind::Boulder | Kind::Rock => {
            if r < 0.18 {
                ("veg.boulder", "rock:boulder")
            } else if r < 0.4 {
                ("veg.rock_large", "rock:large")
            } else if h.p.slope_deg > 26.0 && r < 0.65 {
                ("veg.scree_patch", "rock:scree")
            } else if r < 0.8 {
                ("veg.rock_small", "rock:small")
            } else {
                ("veg.stones", "rock:stones")
            }
        }
        Kind::Log => {
            if r < 0.65 {
                ("veg.fallen_log", "deadwood:log")
            } else {
                ("veg.stump", "deadwood:stump")
            }
        }
        Kind::Reeds => {
            if r < 0.35 {
                ("veg.cattail", "water_plant:cattail")
            } else {
                ("veg.reeds", "water_plant:reeds")
            }
        }
        Kind::Lilies => ("veg.lily_pads", "water_plant:lily"),
        Kind::Low => {
            if h.fd > 0.55 {
                if r < 0.08 {
                    ("veg.mushroom_ring", "low:mushrooms")
                } else {
                    ("veg.fern", "low:fern")
                }
            } else if h.scrub > 0.4 {
                ("veg.heather", "low:heather")
            } else if r < 0.5 {
                ("veg.flower_patch", "low:flowers")
            } else {
                ("veg.tall_grass", "low:tall_grass")
            }
        }
    }
}

/// Every item whose anchor lies in `[x0, x1) × [y0, y1)` (global squares).
/// `phys` must cover those squares.
#[must_use]
pub fn scatter(ctx: &Ctx, phys: &Grid<Phys>, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for l in &LAYERS {
        for (x, y, h) in hard_core(ctx.seed, l, x0, y0, x1, y1) {
            let Some(p) = phys.get(floor_i(x), floor_i(y)) else {
                continue;
            };
            let here = Here {
                p,
                fd: ctx.bilinear(x, y, |c, _| f64::from(c.forest_density) / 255.0),
                scrub: frac(ctx, x, y, Cover::Scrub),
                marsh: frac(ctx, x, y, Cover::Marsh),
                meadow: frac(ctx, x, y, Cover::Grass)
                    * ctx.bilinear(x, y, |c, _| f64::from(c.moisture) / 255.0),
                temp: ctx.bilinear(x, y, |c, _| f64::from(c.temperature.raw()) / 100.0),
                rocky: crate::fixed::rocky(ctx, x, y),
            };
            let keep = unit(crate::hash::mix(h));
            if keep >= density(l.kind, &here, ctx, x, y) {
                continue;
            }
            // Keep trunks, boulders and logs apart across layers.
            let solid = matches!(
                l.kind,
                Kind::TreeSmall | Kind::Undergrowth | Kind::Boulder | Kind::Log
            );
            if solid
                && items.iter().any(|o| {
                    matches!(o.kind, Kind::TreeLarge | Kind::TreeSmall | Kind::Boulder)
                        && (o.x - x) * (o.x - x) + (o.y - y) * (o.y - y) < 1.6 * 1.6
                })
            {
                continue;
            }
            let r = unit(crate::hash::mix(h ^ 0x5EED));
            let (asset, tag) = choose(l.kind, &here, r);
            let kind = if l.kind == Kind::Boulder && !asset.contains("boulder") {
                Kind::Rock
            } else {
                l.kind
            };
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
