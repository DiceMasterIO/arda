//! The field partition: historical field systems from a fixed global
//! hierarchy of land blocks (logic/17 §land-fields).
//!
//! The world is tiled by jittered brick-laid land blocks ([`lattice`]),
//! each recursively split into fields ([`split`]): along the roads that
//! cross it, then across long axes or along contours at jittered, slightly
//! kinked lines, until each piece is the size its ground asks for — small
//! closes by the houses, larger fields further out, furlongs of strips by
//! open-field villages, floodplain meadow by rivers and commons on poor
//! grazing ([`context`]). Junctions are T-shaped and most fields have four
//! sides.
//!
//! Every block is a pure function of its key, the seed and the world data
//! around it, so the tactical plan and the relief tiles, built over any
//! windows, agree on every field. [`Partition::locate`] says which field
//! holds a point; [`Partition::locate_edge`] also gives the distance to the
//! field's edge and the field across it, for anti-aliased edges.

pub mod context;
pub mod cut;
pub mod guide;
pub mod lattice;
pub mod poly;
pub mod split;

use crate::fields::{Crop, FieldKind};
use crate::geom::{Grid, Sq};
use crate::input::{LandUse, LandUseMap, RiverLine, Road, Settlement, Terrain};
pub use context::{Ground, Style, INFLUENCE_M};
use lattice::{blocks_meeting, warp, BLOCK_SPAN, TILE, WARP_MAX};
use poly::{closest_on_outline, contains, P};
use rayon::prelude::*;
use split::{Node, Tree, MAX_LEN};
use std::collections::BTreeMap;

/// Largest distance between two squares of one field, squares: the
/// diagonal of a longest field, plus a road cut's bend and the warp.
pub const FIELD_SPAN: i64 = 310;
/// How far beyond a rectangle the partition reads terrain and rivers,
/// metres: the span of a block.
pub const PARTITION_REACH_M: f64 = BLOCK_SPAN * crate::geom::SQUARE_M + 50.0;
/// How far beyond a rectangle the partition needs settlements and roads,
/// metres.
pub const INPUT_REACH_M: f64 = PARTITION_REACH_M + INFLUENCE_M;

const _: () = assert!(MAX_LEN * 1.42 + 40.0 + 2.0 * WARP_MAX <= 310.0);

/// A field of the partition.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    /// Global key: block (pair index, row) and path in its tree.
    pub key: (i64, i64, u32),
    /// Centroid (warped square units, within a few squares of the real).
    pub p: [f64; 2],
    /// Land use (the majority of the field's ground).
    pub class: Option<LandUse>,
    /// Unit direction of the field's long axis (its strips for furlongs).
    pub along: [f64; 2],
    /// Terrain slope at the centroid (rise over run).
    pub slope: f64,
    /// Use.
    pub kind: FieldKind,
    /// Crop.
    pub crop: Crop,
    /// Area, squares.
    pub area: f64,
}

impl Site {
    /// A two-number hash key of a site key, for salted hashes.
    #[must_use]
    pub fn hash_key(key: (i64, i64, u32)) -> (i64, i64) {
        (key.0 * (1 << 26) + i64::from(key.2), key.1)
    }

    /// This field's hash key.
    #[must_use]
    pub fn hkey(&self) -> (i64, i64) {
        Self::hash_key(self.key)
    }
}

/// Which field holds a point, how far inside it, and which lies across.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Site index.
    pub site: usize,
    /// The field across the nearest edge, if built.
    pub other: Option<usize>,
    /// Distance to the edge, squares (positive inside).
    pub edge: f64,
}

/// What the partition reads.
pub struct PartitionInputs<'a> {
    /// Land use per 100 m cell.
    pub landuse: &'a dyn LandUseMap,
    /// Terrain (heights only).
    pub terrain: &'a dyn Terrain,
    /// Settlements within [`INPUT_REACH_M`] of the rectangle.
    pub settlements: &'a [Settlement],
    /// Roads within [`INPUT_REACH_M`] of the rectangle.
    pub roads: &'a [Road],
    /// River channels within [`PARTITION_REACH_M`] of the rectangle.
    pub rivers: &'a [RiverLine],
    /// Culture.
    pub culture: &'a str,
    /// Wealth, 0–255.
    pub wealth: u8,
}

impl<'a> From<&crate::input::FieldInputs<'a>> for PartitionInputs<'a> {
    fn from(i: &crate::input::FieldInputs<'a>) -> Self {
        Self {
            landuse: i.landuse,
            terrain: i.terrain,
            settlements: i.settlements,
            roads: i.roads,
            rivers: i.rivers,
            culture: i.culture,
            wealth: i.wealth,
        }
    }
}

/// The fields of every block meeting a rectangle.
#[derive(Debug, Clone)]
pub struct Partition {
    seed: u64,
    bias: f64,
    trees: Vec<Tree>,
    first_site: Vec<usize>,
    /// Built blocks by the lattice tiles their outlines' boxes cover.
    tiles: BTreeMap<(i64, i64), Vec<usize>>,
    /// Every field, block by block.
    pub sites: Vec<Site>,
}

impl Partition {
    /// The partition over the square rectangle `(x0, y0, x1, y1)`: every
    /// block that may hold one of its points.
    #[must_use]
    pub fn new(inputs: &PartitionInputs<'_>, seed: u64, rect: (i64, i64, i64, i64)) -> Self {
        let style = Style::of(inputs.culture, inputs.wealth);
        let pad = WARP_MAX + 1.0;
        #[allow(clippy::cast_precision_loss)] // map coordinates
        let r = [
            rect.0 as f64 - pad,
            rect.1 as f64 - pad,
            rect.2 as f64 + pad,
            rect.3 as f64 + pad,
        ];
        let reach = BLOCK_SPAN + 64.0;
        let wide = [r[0] - reach, r[1] - reach, r[2] + reach, r[3] + reach];
        let guides = guide::roads(inputs.roads, seed, style.bias, wide);
        // Blocks are independent: built in parallel, collected in order.
        // Each reads the settlements and rivers that can reach it.
        let built: Vec<(Tree, Vec<Site>)> = blocks_meeting(seed, r)
            .into_par_iter()
            .map(|id| {
                let b = poly::bbox(&[lattice::outline(seed, id)]);
                let m = |v: f64| v * crate::geom::SQUARE_M;
                let (x0, y0, x1, y1) = (m(b[0]), m(b[1]), m(b[2]), m(b[3]));
                let pad = INFLUENCE_M + 50.0;
                let settlements: Vec<Settlement> = inputs
                    .settlements
                    .iter()
                    .filter(|s| {
                        s.x_m >= x0 - pad
                            && s.x_m <= x1 + pad
                            && s.y_m >= y0 - pad
                            && s.y_m <= y1 + pad
                    })
                    .cloned()
                    .collect();
                let pad = 250.0;
                let rivers: Vec<RiverLine> = inputs
                    .rivers
                    .iter()
                    .filter(|r| {
                        r.a[0].min(r.b[0]) <= x1 + pad
                            && r.a[0].max(r.b[0]) >= x0 - pad
                            && r.a[1].min(r.b[1]) <= y1 + pad
                            && r.a[1].max(r.b[1]) >= y0 - pad
                    })
                    .copied()
                    .collect();
                let ground = Ground {
                    seed,
                    landuse: inputs.landuse,
                    terrain: inputs.terrain,
                    settlements: &settlements,
                    rivers: &rivers,
                    style,
                };
                split::build(seed, &ground, &guides, id)
            })
            .collect();
        let mut trees = Vec::with_capacity(built.len());
        let mut first_site = Vec::with_capacity(built.len());
        let mut sites = Vec::new();
        let mut tiles: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
        for (t, s) in built {
            let [x0, y0, x1, y1] = poly::bbox(std::slice::from_ref(&t.outline));
            for j in tile(y0)..=tile(y1) {
                for i in tile(x0)..=tile(x1) {
                    tiles.entry((i, j)).or_default().push(trees.len());
                }
            }
            first_site.push(sites.len());
            sites.extend(s);
            trees.push(t);
        }
        Self {
            seed,
            bias: style.bias,
            trees,
            first_site,
            tiles,
            sites,
        }
    }

    /// The warped position of a real point (square units).
    #[must_use]
    pub fn warped(&self, p: P) -> P {
        warp(self.seed, p, self.bias)
    }

    fn tree_at(&self, w: P) -> Option<usize> {
        self.tiles
            .get(&(tile(w[0]), tile(w[1])))?
            .iter()
            .copied()
            .find(|&t| contains(&self.trees[t].outline, w))
    }

    /// The field holding warped point `w`.
    fn locate_warped(&self, w: P) -> Option<usize> {
        let t = self.tree_at(w)?;
        let nodes = &self.trees[t].nodes;
        let mut k = 0_usize;
        loop {
            match nodes.get(k)? {
                Node::Leaf(s) => return Some(self.first_site[t] + *s as usize),
                Node::Split { cut, kids } => k = kids[cut.side(w)] as usize,
            }
        }
    }

    /// The field holding real point `p` (square units), if its block is
    /// built.
    #[must_use]
    pub fn locate(&self, p: P) -> Option<usize> {
        self.locate_warped(self.warped(p))
    }

    /// The field holding `p`, the distance to its edge and the field
    /// across it.
    #[must_use]
    pub fn locate_edge(&self, p: P) -> Option<Hit> {
        let w = self.warped(p);
        let t = self.tree_at(w)?;
        let tree = &self.trees[t];
        let (mut dist, mut at) = closest_on_outline(&tree.outline, w);
        let mut k = 0_usize;
        let site = loop {
            match tree.nodes.get(k)? {
                Node::Leaf(s) => break self.first_site[t] + *s as usize,
                Node::Split { cut, kids } => {
                    let n = cut.near(w);
                    if n.dist < dist {
                        (dist, at) = (n.dist, n.at);
                    }
                    k = kids[usize::from(n.left)] as usize;
                }
            }
        };
        // Step just across the nearest edge.
        let d = poly::unit(poly::sub(at, w));
        let step = dist + 0.05;
        let q = [w[0] + d[0] * step, w[1] + d[1] * step];
        let other = self.locate_warped(q).filter(|&o| o != site);
        Some(Hit {
            site,
            other,
            edge: dist,
        })
    }

    /// The number of blocks built.
    #[must_use]
    pub fn blocks(&self) -> usize {
        self.trees.len()
    }
}

/// The lattice tile index of a warped coordinate.
#[allow(clippy::cast_possible_truncation)] // floors of map positions
fn tile(v: f64) -> i64 {
    (v / TILE).floor() as i64
}

/// Terrain gradient (rise over run) at world point `m` over ±8 m.
#[must_use]
pub fn gradient(terrain: &dyn Terrain, m: [f64; 2]) -> [f64; 2] {
    let d = 8.0;
    let t = |dx: f64, dy: f64| terrain.sample(m[0] + dx, m[1] + dy).height_m;
    [
        (t(d, 0.0) - t(-d, 0.0)) / (2.0 * d),
        (t(0.0, d) - t(0.0, -d)) / (2.0 * d),
    ]
}

/// Labels the 4-connected components of equal values in `raw`, calling
/// `emit(value, squares)` for each in row-major order of first square.
pub fn components<T: Copy + PartialEq>(raw: &Grid<Option<T>>, mut emit: impl FnMut(T, &[Sq])) {
    let mut seen = Grid::new(raw.x0, raw.y0, raw.w, raw.h, false);
    let mut stack = Vec::new();
    let mut members = Vec::new();
    for s in raw.squares() {
        let Some(Some(v)) = raw.get(s).copied() else {
            continue;
        };
        if seen.get(s) == Some(&true) {
            continue;
        }
        members.clear();
        stack.push(s);
        seen.set(s, true);
        while let Some(c) = stack.pop() {
            members.push(c);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = c.offset(dx, dy);
                if seen.get(n) == Some(&false) && raw.get(n) == Some(&Some(v)) {
                    seen.set(n, true);
                    stack.push(n);
                }
            }
        }
        members.sort_unstable_by_key(|q| (q.y, q.x));
        emit(v, &members);
    }
}
