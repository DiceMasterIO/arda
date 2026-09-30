//! The per-block pipeline: gather, physical fields, fixed features, fixed
//! borders, WFC ground, scatter and rules.

use crate::borders::{corner_class, solve_line, LineCorner};
use crate::classes::{Class, Mask, COUNT};
use crate::context::{Ctx, N};
use crate::error::RefineError;
use crate::fixed::{classify, vertex_masks, Fixed, FixedSquare};
use crate::grid::span;
use crate::grid::Grid;
use crate::prior::{Prior, SHARPNESS};
use crate::rules::{CoverRule, SquareRules};
use crate::scatter::{scatter, Item, Kind};
use crate::source::{CellKey, Source};
use crate::terrain::{physical, slopes, Phys, Water};
use crate::tiles::dominant_land;
use crate::wfc::{solve, Problem};

/// Squares per block side.
pub const SIDE: usize = 64;
/// Metres per foot.
const FOOT_M: f64 = 0.3048;
/// Deepest water still waded rather than swum, feet: under 5 ft is
/// shallow, 5 ft or more is deep (canonical convention I15).
pub const WADE_FT: u8 = 4;

/// One refined 64 × 64 block.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The block's cell.
    pub cell: CellKey,
    /// Global square of the block's top-left square.
    pub origin: (i64, i64),
    /// Ground key per square, row-major.
    pub ground: Vec<&'static str>,
    /// Elevation in feet on 5-ft steps.
    pub elevation_ft: Vec<i16>,
    /// Water depth in feet.
    pub depth_ft: Vec<u8>,
    /// Pre-WFC decision per square.
    pub fixed: Vec<Fixed>,
    /// Corner classes, 65 × 65 row-major.
    pub corners: Vec<Class>,
    /// Placements anchored inside the block, global coordinates.
    pub items: Vec<Item>,
    /// Elevation and depth for the one-square ring around the block
    /// (squares -1..=64), as the neighbours will compute them.
    pub halo: Grid<(i16, u8)>,
    /// Rules per square.
    pub rules: Vec<SquareRules>,
    /// Whether the WFC or a border fell back to the relaxed fill.
    pub relaxed: bool,
    /// WFC attempts used.
    pub attempts: u8,
    /// Local WFC repairs.
    pub repairs: u32,
}

/// Quantises metres to feet on 5-ft contour steps.
#[must_use]
pub fn contour_ft(m: f64) -> i16 {
    let steps = (m / FOOT_M / 5.0 + 0.5).floor().clamp(-6_000.0, 6_000.0);
    #[allow(clippy::cast_possible_truncation)]
    let ft = steps as i16 * 5;
    ft
}

/// Water depth in whole feet, at least one for any water.
#[must_use]
pub fn depth_ft(p: &Phys) -> u8 {
    if p.water == Water::Dry {
        return 0;
    }
    let ft = (p.depth_m / FOOT_M).ceil().clamp(1.0, 255.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let d = ft as u8;
    d
}

fn vertex_scores(
    ctx: &Ctx,
    prior: &Prior,
    phys: &Grid<Phys>,
    mask: Mask,
    x: i64,
    y: i64,
) -> [f64; COUNT] {
    let around = [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)].map(|(a, b)| *phys.clamped(a, b));
    let slope = around.iter().map(|p| p.slope_deg).sum::<f64>() / 4.0;
    let north = around.iter().map(|p| p.north).sum::<f64>() / 4.0;
    let river = around
        .iter()
        .map(|p| p.river_d)
        .fold(f64::INFINITY, f64::min);
    let mut s = prior.scores(ctx, x as f64, y as f64, slope, north, river);
    let water = Class::Water.bit();
    if mask & water != 0 && mask != water {
        // A bank corner: favour the bank classes so the margin shows.
        for (i, v) in s.iter_mut().enumerate() {
            if i != Class::Water.index() && mask & (1 << i) != 0 {
                *v = v.max(-2.0) + 1.2;
            }
        }
    }
    let crag = Class::Cliff.bit() | Class::Rock.bit() | Class::Scree.bit();
    if mask & !crag == 0 {
        s[Class::Cliff.index()] += 1.5;
    }
    s
}

struct Lattice {
    masks: Grid<Mask>,
    scores: Grid<[f64; COUNT]>,
}

impl Lattice {
    fn line(&self, pts: &[(i64, i64)], horizontal: bool) -> Vec<LineCorner> {
        pts.iter()
            .map(|&(x, y)| {
                let (a, b) = if horizontal {
                    ((x, y - 1), (x, y + 1))
                } else {
                    ((x - 1, y), (x + 1, y))
                };
                let sc = self.scores.clamped(x, y);
                LineCorner {
                    mask: *self.masks.clamped(x, y),
                    side_a: *self.masks.clamped(a.0, a.1),
                    side_b: *self.masks.clamped(b.0, b.1),
                    cost: sc.map(|v| -SHARPNESS * v),
                }
            })
            .collect()
    }

    fn corner(&self, x: i64, y: i64) -> Class {
        let mut around = [0; 9];
        for (k, slot) in around.iter_mut().enumerate() {
            let (dx, dy) = (span(k) % 3 - 1, span(k) / 3 - 1);
            *slot = *self.masks.clamped(x + dx, y + dy);
        }
        let cost = self.scores.clamped(x, y).map(|v| -SHARPNESS * v);
        corner_class(*self.masks.clamped(x, y), &cost, around)
    }
}

/// Refines the block of `cell`.
///
/// # Errors
/// A world layer failed to read.
pub fn refine(src: &dyn Source, cell: CellKey) -> Result<Block, RefineError> {
    let ctx = Ctx::gather(src, cell)?;
    let (x0, y0) = (cell.x * N, cell.y * N);
    let pieces = crate::rivers::pieces(&ctx);
    let mut phys = physical(&ctx, &pieces, x0 - 3, y0 - 3, SIDE + 6);
    slopes(&mut phys);

    let mut fixed: Grid<FixedSquare> = Grid::new(
        x0 - 2,
        y0 - 2,
        SIDE + 4,
        SIDE + 4,
        FixedSquare {
            fixed: Fixed::Open,
            mask: 0,
        },
    );
    for (i, (x, y)) in fixed.coords().collect::<Vec<_>>().into_iter().enumerate() {
        fixed.data[i] = classify(&ctx, x, y, phys.clamped(x, y));
    }
    let masks = vertex_masks(&fixed);
    let prior = Prior::new(&ctx);
    let mut scores = Grid::new(x0 - 1, y0 - 1, SIDE + 3, SIDE + 3, [0.0; COUNT]);
    for (i, (x, y)) in scores.coords().collect::<Vec<_>>().into_iter().enumerate() {
        scores.data[i] = vertex_scores(&ctx, &prior, &phys, *masks.clamped(x, y), x, y);
    }
    let lat = Lattice { masks, scores };

    // Fixed borders: block corners first, then each edge in increasing order.
    let n = span(SIDE);
    let corner = |dx: i64, dy: i64| lat.corner(x0 + dx * n, y0 + dy * n);
    let (nw, ne, sw, se) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
    let row = |y: i64| (0..=n).map(|i| (x0 + i, y)).collect::<Vec<_>>();
    let col = |x: i64| (0..=n).map(|j| (x, y0 + j)).collect::<Vec<_>>();
    let (north, ok_n) = solve_line(&lat.line(&row(y0), true), nw, ne);
    let (south, ok_s) = solve_line(&lat.line(&row(y0 + n), true), sw, se);
    let (west, ok_w) = solve_line(&lat.line(&col(x0), false), nw, sw);
    let (east, ok_e) = solve_line(&lat.line(&col(x0 + n), false), ne, se);

    let side = SIDE + 1;
    let mut problem = Problem {
        n: side,
        masks: vec![0; side * side],
        weights: vec![[0.0; COUNT]; side * side],
        seed: ctx.seed,
        key: (cell.x, cell.y),
    };
    for j in 0..side {
        for i in 0..side {
            let (x, y) = (x0 + span(i), y0 + span(j));
            let k = j * side + i;
            problem.masks[k] = *lat.masks.clamped(x, y);
            #[allow(clippy::cast_possible_truncation)]
            let w = lat
                .scores
                .clamped(x, y)
                .map(|s| crate::math::exp(SHARPNESS * s) as f32);
            problem.weights[k] = w;
            let border = if j == 0 {
                Some(north[i])
            } else if j == SIDE {
                Some(south[i])
            } else if i == 0 {
                Some(west[j])
            } else if i == SIDE {
                Some(east[j])
            } else {
                None
            };
            if let Some(c) = border {
                problem.masks[k] = c.bit();
            }
        }
    }
    let sol = solve(&problem);

    let mut block = Block {
        cell,
        origin: (x0, y0),
        ground: Vec::with_capacity(SIDE * SIDE),
        elevation_ft: Vec::with_capacity(SIDE * SIDE),
        depth_ft: Vec::with_capacity(SIDE * SIDE),
        fixed: Vec::with_capacity(SIDE * SIDE),
        corners: sol.classes.clone(),
        items: Vec::new(),
        halo: Grid::new(x0 - 1, y0 - 1, SIDE + 2, SIDE + 2, (0, 0)),
        rules: Vec::with_capacity(SIDE * SIDE),
        relaxed: sol.relaxed || !(ok_n && ok_s && ok_w && ok_e),
        attempts: sol.attempts,
        repairs: sol.repairs,
    };
    for (i, (x, y)) in block
        .halo
        .coords()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        let p = phys.clamped(x, y);
        block.halo.data[i] = (contour_ft(p.elev_m), depth_ft(p));
    }
    for j in 0..SIDE {
        for i in 0..SIDE {
            let (x, y) = (x0 + span(i), y0 + span(j));
            let p = phys.clamped(x, y);
            let f = fixed.clamped(x, y).fixed;
            let c = [
                sol.classes[j * side + i],
                sol.classes[j * side + i + 1],
                sol.classes[(j + 1) * side + i + 1],
                sol.classes[(j + 1) * side + i],
            ];
            let d = depth_ft(p);
            let key = match f {
                Fixed::Water if d > WADE_FT => "water_deep",
                Fixed::Water => "water_shallow",
                Fixed::Cliff => Class::Cliff.ground_key(),
                Fixed::Shore(k) => k.ground_key(),
                Fixed::Open => dominant_land(c).unwrap_or(Class::Mud).ground_key(),
            };
            block.ground.push(key);
            block.elevation_ft.push(contour_ft(p.elev_m));
            block.depth_ft.push(d);
            block.fixed.push(f);
        }
    }
    let x0f = x0 as f64;
    let y0f = y0 as f64;
    let all = scatter(&ctx, &phys, x0f - 2.0, y0f - 2.0, x0f + 66.0, y0f + 66.0);
    let illegal: std::collections::BTreeSet<usize> = sol.illegal.iter().copied().collect();
    block.rules = crate::output::square_rules(&block, &all, &illegal);
    block.items = all
        .into_iter()
        .filter(|it| it.x >= x0f && it.x < x0f + 64.0 && it.y >= y0f && it.y < y0f + 64.0)
        .collect();
    Ok(block)
}

/// Whether ground or water makes a square difficult terrain.
#[must_use]
pub fn difficult_ground(key: &str, depth: u8) -> bool {
    (depth > 0 && depth <= WADE_FT)
        || matches!(
            key,
            "scree" | "mud" | "marsh" | "reed_bed" | "snow" | "ice" | "scrub" | "cliff"
        )
}

/// Cover and difficulty an item grants.
#[must_use]
pub const fn item_rules(kind: Kind) -> (CoverRule, bool) {
    match kind {
        Kind::TreeLarge | Kind::Boulder => (CoverRule::ThreeQuarters, false),
        Kind::TreeSmall | Kind::Rock => (CoverRule::Half, false),
        Kind::Log => (CoverRule::Half, true),
        Kind::Undergrowth => (CoverRule::None, true),
        Kind::Reeds | Kind::Lilies | Kind::Low => (CoverRule::None, false),
    }
}
